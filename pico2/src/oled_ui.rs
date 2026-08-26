//! SSD1306 128x64 menu.
//!
//! Ratatui drawn through mousefood's embedded-graphics backend onto a mono
//! panel. The plan flagged "verify mousefood renders to a `BinaryColor` target"
//! as a risk to check early — it does: `impl From<TermColor<'_>> for BinaryColor`
//! in mousefood's `colors.rs`. This module is that verification made concrete.
//!
//! # Grid
//!
//! `FONT_5X8` on 128x64 gives **25 columns x 8 rows** — one row for the page
//! title and seven for fields, which is exactly `MAX_FIELDS_PER_PAGE`. Change
//! the font and that budget changes with it; the test in `host_tests` is what
//! keeps the page table honest about it.
//!
//! # Why blocking SPI
//!
//! `display-interface-spi` 0.5 has no async variant, so ssd1306's `async`
//! feature has nothing to sit on. A full redraw is ~1 ms at 8 MHz, which briefly
//! stalls the other core-1 tasks. LED and DMX timing is carried by PIO and DMA
//! in hardware, so a late poll shifts scheduling rather than output — at a redraw
//! rate in the tens of hertz that is a few percent of one core. Worth revisiting
//! only if jitter shows up on the bench.

use common::channels::{RouterChannelTx, UiChannelRx};
use common::event_router::RouterEvent;
use common::events::UiEvent;
use common::ui::{MenuData, PAGES};
use defmt::*;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use display_interface_spi::SPIInterface;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::SPI1;
use embassy_rp::spi::{Blocking, Spi};
use embedded_graphics::mono_font::ascii::FONT_5X8;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_hal_bus::spi::ExclusiveDevice;
use mousefood::{EmbeddedBackend, EmbeddedBackendConfig};
use ratatui_core::style::{Modifier, Style};
use ratatui_core::terminal::Terminal;
use ssd1306::mode::{BufferedGraphicsMode, DisplayConfig};
use ssd1306::prelude::*;
use ssd1306::{size::DisplaySize128x64, Ssd1306};

/// Chip select is tied low on the carrier — the OLED is the only device on
/// SPI1, which is what keeps GP13 free as the board's one spare GPIO. The bus
/// abstraction still wants a pin, so give it one that does nothing.
pub struct NoCs;

impl embedded_hal::digital::ErrorType for NoCs {
    type Error = core::convert::Infallible;
}

impl embedded_hal::digital::OutputPin for NoCs {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

type OledDev = ExclusiveDevice<Spi<'static, SPI1, Blocking>, NoCs, embedded_hal_bus::spi::NoDelay>;
type Oled = Ssd1306<
    SPIInterface<OledDev, Output<'static>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

/// Build the display from an already-configured SPI1 and the DC pin.
pub fn build(spi: Spi<'static, SPI1, Blocking>, dc: Output<'static>) -> Oled {
    let dev = ExclusiveDevice::new_no_delay(spi, NoCs).expect("OLED chip select");
    Ssd1306::new(
        SPIInterface::new(dev, dc),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode()
}

/// Whether the menu is navigating or mid-edit.
///
/// Editing is a **copy-on-edit transaction**: adjustments land on a private
/// draft, and only a completed edit is sent to the router, which merges that one
/// field into the live settings. Esc discards the draft, so a half-finished edit
/// cannot leak into stored settings — the same contract the STM32 build had.
enum Mode {
    Navigate,
    Edit { digit: u8, draft: MenuData },
}

/// Menu cursor: which page, and which field within it.
struct Cursor {
    page: usize,
    field: usize,
}

impl Cursor {
    /// Move by one, rolling onto the next or previous page at the edges — the
    /// same traversal the STM32 build used, so the button feel is unchanged.
    fn step(&mut self, down: bool) {
        let len = PAGES[self.page].fields.len();
        if down {
            if self.field + 1 < len {
                self.field += 1;
            } else {
                self.page = (self.page + 1) % PAGES.len();
                self.field = 0;
            }
        } else if self.field > 0 {
            self.field -= 1;
        } else {
            self.page = (self.page + PAGES.len() - 1) % PAGES.len();
            self.field = PAGES[self.page].fields.len().saturating_sub(1);
        }
    }
}

/// Draw the menu and react to button events.
#[embassy_executor::task]
pub async fn ui_task(
    mut display: Oled,
    rx: UiChannelRx,
    tx: RouterChannelTx,
    oled_ready: &'static Signal<CriticalSectionRawMutex, ()>,
) {
    // The panel's RES pin lives on the TCA9555, released by the button task
    // over I²C. `init()` below is blocking SPI: without this handshake it
    // would run before the release (the display, still in reset, ignores
    // every command on the write-only bus and stays dark forever). Wait for
    // the release, then give the controller a moment to come out of reset.
    oled_ready.wait().await;
    Timer::after_millis(10).await;

    if display.init().is_err() {
        error!("SSD1306: init failed - no display");
        return;
    }
    info!("SSD1306: up, 25x8 grid");

    let config = EmbeddedBackendConfig {
        // Buffered graphics mode draws into RAM; this is what pushes it out.
        flush_callback: alloc::boxed::Box::new(|d: &mut Oled| {
            let _ = d.flush();
        }),
        font_regular: FONT_5X8,
        ..Default::default()
    };
    let backend = EmbeddedBackend::<Oled, BinaryColor>::new(&mut display, config);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => {
            error!("SSD1306: terminal init failed");
            return;
        }
    };

    let mut data = MenuData::default();
    let mut cursor = Cursor { page: 0, field: 0 };
    let mut mode = Mode::Navigate;
    loop {
        {
            let page = &PAGES[cursor.page];
            let _ = terminal.draw(|frame| {
                let buf = frame.buffer_mut();
                buf.set_string(0, 0, page.title, Style::new().add_modifier(Modifier::REVERSED));

                for (row, field) in page.fields.iter().enumerate() {
                    let selected = row == cursor.field;
                    let style = if selected {
                        Style::new().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::new()
                    };
                    let y = row as u16 + 1;
                    // Label left, value from column 13. 25 columns total; labels
                    // are capped at 11 characters in the field table.
                    buf.set_string(0, y, field.label(), style);

                    match &mode {
                        // Show the draft, not the live value, and mark the digit
                        // being changed so it is obvious which one Up/Down moves.
                        Mode::Edit { digit, draft } if selected => {
                            let text = field.display(draft);
                            buf.set_string(13, y, &text, Style::new());
                            let d = *digit as usize;
                            if let Some(ch) = text.chars().nth(d) {
                                let mut one = alloc::string::String::new();
                                one.push(ch);
                                buf.set_string(
                                    13 + d as u16,
                                    y,
                                    &one,
                                    Style::new().add_modifier(Modifier::REVERSED),
                                );
                            }
                        }
                        _ => buf.set_string(13, y, field.display(&data), style),
                    }
                }
            });
        }

        let event = rx.receive().await;
        let field = PAGES[cursor.page].fields[cursor.field];

        match (&mut mode, event) {
            // A settings update from the router always wins; an in-flight edit
            // would be working from a stale base.
            (m, UiEvent::Load(new_data)) => {
                data = new_data;
                *m = Mode::Navigate;
            }

            (Mode::Navigate, UiEvent::Up) => cursor.step(false),
            (Mode::Navigate, UiEvent::Down) => cursor.step(true),
            // Esc jumps to the next page, matching the STM32 build.
            (Mode::Navigate, UiEvent::Esc) => {
                cursor.page = (cursor.page + 1) % PAGES.len();
                cursor.field = 0;
            }
            (Mode::Navigate, UiEvent::Select) => {
                if field.editable() {
                    mode = Mode::Edit { digit: 0, draft: data };
                }
            }

            (Mode::Edit { digit, draft }, UiEvent::Up) => field.adjust(draft, *digit, true),
            (Mode::Edit { digit, draft }, UiEvent::Down) => field.adjust(draft, *digit, false),
            // Esc discards the draft outright.
            (m @ Mode::Edit { .. }, UiEvent::Esc) => *m = Mode::Navigate,
            // Select walks to the next digit and commits past the last one.
            (Mode::Edit { digit, draft }, UiEvent::Select) => {
                if *digit + 1 < field.digits() {
                    *digit += 1;
                } else {
                    let committed = *draft;
                    if tx
                        .try_send(RouterEvent::WriteFieldToEeprom(field, committed))
                        .is_err()
                    {
                        error!("UI: commit dropped, router channel full");
                    }
                    mode = Mode::Navigate;
                }
            }
        }
    }
}
