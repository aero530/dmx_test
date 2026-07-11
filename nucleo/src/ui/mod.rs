//! User interface — Ratatui on the ST7789 TFT via the mousefood
//! embedded-graphics backend.
//!
//! The menu is a set of tabbed pages of settings fields. All field behavior
//! (labels, ranges, rendering, editing) lives in the [`fields`] metadata
//! table; this module only holds the cursor/edit state machine and the
//! Ratatui rendering.
//!
//! Interaction model (4 buttons):
//! * Navigating: Up/Down move the selection (crossing a page edge moves to
//!   the previous/next tab), Select starts editing the highlighted field,
//!   Esc jumps to the next tab.
//! * Editing: the field is edited on a scratch copy of the settings.
//!   Up/Down change the highlighted digit (numeric fields) or cycle the
//!   value (enums). Select advances to the next digit and, after the last
//!   one, commits: the edited field is merged into the live settings and
//!   written to the EEPROM. Esc cancels the edit, discarding the change.
//!
//! `UiEvent::Load` (settings read back from the EEPROM, DHCP address
//! updates, ...) refreshes the live settings at any time; an in-progress
//! edit keeps its scratch copy, and committing merges only the edited field
//! so a concurrent update can't be clobbered.
use cfg_if::cfg_if;
use defmt::Format;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{error, info};
    } else {
        use defmt::{error, info};
    }
}

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use embassy_stm32::gpio::{Output, OutputOpenDrain};
use embassy_stm32::mode::Async;
use embassy_stm32::spi::Spi;
use embassy_time::Delay;

use embedded_graphics::mono_font::ascii::{FONT_9X15, FONT_9X15_BOLD};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_hal_bus::spi::ExclusiveDevice;

use mipidsi::interface::SpiInterface;
use mipidsi::options::{ColorInversion, Orientation, Rotation};
use mipidsi::{models::ST7789, Builder};

use mousefood::{EmbeddedBackend, EmbeddedBackendConfig};
use ratatui_core::layout::{Constraint, Layout};
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::terminal::{Frame, Terminal};
use ratatui_core::text::{Line, Span};
use ratatui_widgets::paragraph::Paragraph;
use ratatui_widgets::tabs::Tabs;

use static_cell::StaticCell;

use crate::channels::{RouterChannelTx, UiChannelRx};
use crate::event_router::RouterEvent;
use crate::{DISPLAY_HEIGHT, DISPLAY_OFFSET, DISPLAY_WIDTH};

mod types;
pub use types::*;

mod fields;
pub use fields::{all_fields, FieldId, Page, PAGES};

type SpiDisplay = mipidsi::Display<SpiInterface<'static, ExclusiveDevice<Spi<'static, Async>, Output<'static>, embedded_hal_bus::spi::NoDelay>, Output<'static>>, ST7789, Output<'static>>;

/// Buffer to write data to SPI based display
static SPI_DISP_BUFFER: StaticCell<[u8; 512]> = StaticCell::new();

/// Action to be processed by the UI (often user interaction)
#[derive(Format)]
pub enum UiEvent {
    Up,
    Down,
    Select,
    Esc,
    Load(MenuData),
}

const LABEL_WIDTH: usize = 18;

/// UI state machine: current settings, cursor and edit transaction.
pub struct UiApp {
    /// Live settings, kept in sync with the router / EEPROM
    settings: MenuData,
    /// Scratch copy edited in place while `editing`
    draft: MenuData,
    editing: bool,
    tab: usize,
    row: usize,
    /// Digit sub-cursor for numeric fields (0 = most significant)
    digit: u8,
    tx: RouterChannelTx,
}

impl UiApp {
    pub fn new(tx: RouterChannelTx) -> Self {
        let settings = MenuData::default();
        Self {
            settings,
            draft: settings,
            editing: false,
            tab: 0,
            row: 0,
            digit: 0,
            tx,
        }
    }

    fn page(&self) -> &'static Page {
        &PAGES[self.tab]
    }

    fn field(&self) -> FieldId {
        self.page().fields[self.row]
    }

    pub fn handle(&mut self, event: UiEvent) {
        match event {
            UiEvent::Load(data) => {
                // Refresh the live settings. A running edit keeps its draft;
                // commit merges only the edited field (see FieldId::transfer).
                self.settings = data;
            }
            UiEvent::Up => {
                if self.editing {
                    self.field().adjust(&mut self.draft, self.digit, true);
                } else {
                    self.move_up();
                }
            }
            UiEvent::Down => {
                if self.editing {
                    self.field().adjust(&mut self.draft, self.digit, false);
                } else {
                    self.move_down();
                }
            }
            UiEvent::Select => {
                if self.editing {
                    let digits = self.field().digits();
                    if digits == 0 || self.digit + 1 >= digits {
                        self.commit();
                    } else {
                        self.digit += 1;
                    }
                } else if self.field().editable() {
                    self.draft = self.settings;
                    self.digit = 0;
                    self.editing = true;
                }
            }
            UiEvent::Esc => {
                if self.editing {
                    self.editing = false; // cancel, draft is discarded
                } else {
                    self.tab = (self.tab + 1) % PAGES.len();
                    self.row = 0;
                }
            }
        }
    }

    fn move_up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
        } else {
            self.tab = self.tab.checked_sub(1).unwrap_or(PAGES.len() - 1);
            self.row = self.page().fields.len() - 1;
        }
    }

    fn move_down(&mut self) {
        if self.row + 1 < self.page().fields.len() {
            self.row += 1;
        } else {
            self.tab = (self.tab + 1) % PAGES.len();
            self.row = 0;
        }
    }

    /// Merge the edited field into the live settings and persist everything.
    fn commit(&mut self) {
        self.field().transfer(&self.draft, &mut self.settings);
        self.editing = false;
        info!("UI commit settings");
        match self.tx.try_send(RouterEvent::WriteSettingsToEeprom(self.settings)) {
            Ok(_) => {}
            Err(_) => error!("UI commit dropped - router channel full"),
        }
    }

    /// Render the whole UI into a Ratatui frame.
    pub fn render(&self, frame: &mut Frame) {
        let [tab_area, body_area, footer_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        let tabs = Tabs::new(PAGES.iter().map(|p| p.title))
            .select(self.tab)
            .style(Style::new().fg(Color::Gray))
            .highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD));
        frame.render_widget(tabs, tab_area);

        let data = if self.editing { &self.draft } else { &self.settings };
        let mut lines: Vec<Line> = Vec::new();
        for (row, field) in self.page().fields.iter().enumerate() {
            lines.push(self.render_row(row, *field, data));
        }
        frame.render_widget(Paragraph::new(lines), body_area);

        let hint = if self.editing {
            " \u{2191}\u{2193} change  SEL next/save  ESC cancel"
        } else {
            " \u{2191}\u{2193} move  SEL edit  ESC tab"
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::new().fg(Color::Black).bg(Color::Gray)),
            footer_area,
        );
    }

    fn render_row(&self, row: usize, field: FieldId, data: &MenuData) -> Line<'static> {
        let selected = row == self.row;
        let label = format!(" {:<width$}", field.label(), width = LABEL_WIDTH);
        let value = field.display(data);

        let label_style = if !field.editable() {
            Style::new().fg(Color::DarkGray)
        } else if selected && !self.editing {
            Style::new().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::new().fg(Color::Gray)
        };

        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::styled(label, label_style));

        if selected && self.editing {
            let edit_style = Style::new().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD);
            if field.digits() > 0 {
                // Highlight only the digit under the cursor
                let i = self.digit as usize;
                let mut chars = value.chars();
                let before: String = chars.by_ref().take(i).collect();
                let digit: String = chars.by_ref().take(1).collect();
                let after: String = chars.collect();
                spans.push(Span::styled(before, Style::new().fg(Color::Yellow)));
                spans.push(Span::styled(digit, edit_style));
                spans.push(Span::styled(after, Style::new().fg(Color::Yellow)));
            } else {
                spans.push(Span::styled(value, edit_style));
            }
        } else {
            let value_style = if !field.editable() {
                Style::new().fg(Color::DarkGray)
            } else if selected {
                Style::new().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::White)
            };
            spans.push(Span::styled(value, value_style));
        }

        Line::from(spans)
    }
}

/// Task to manage and display the UI
#[embassy_executor::task]
pub async fn ui_task_spi(bus: Spi<'static, Async>, cs: Output<'static>, dc: Output<'static>, reset: Output<'static>, mut backlight: OutputOpenDrain<'static>, rx: UiChannelRx, tx: RouterChannelTx) {
    let spi_device = ExclusiveDevice::new_no_delay(bus, cs).unwrap();

    let buffer = SPI_DISP_BUFFER.init([0_u8; 512]);

    // Define the display interface with no chip select
    let di = SpiInterface::new(spi_device, dc, buffer);

    let mut delay = Delay;

    // Define the display from the display interface and initialize it
    let mut display = Builder::new(ST7789, di)
        .reset_pin(reset)
        .display_size(DISPLAY_WIDTH, DISPLAY_HEIGHT)
        .display_offset(DISPLAY_OFFSET, 0)
        .orientation(Orientation::new().rotate(Rotation::Deg90))
        .invert_colors(ColorInversion::Inverted)
        .init(&mut delay)
        .unwrap();

    // Turn on backlight
    backlight.set_high();

    // Ratatui terminal rendering through mousefood onto the TFT.
    // FONT_9X15 on the 320x172 logical display gives a 35x11 character grid.
    let config: EmbeddedBackendConfig<SpiDisplay, Rgb565> = EmbeddedBackendConfig {
        font_regular: FONT_9X15,
        font_bold: Some(FONT_9X15_BOLD),
        ..Default::default()
    };
    let backend = EmbeddedBackend::new(&mut display, config);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => {
            error!("UI: failed to create terminal");
            return;
        }
    };

    let mut app = UiApp::new(tx);

    loop {
        if terminal.draw(|frame| app.render(frame)).is_err() {
            error!("UI: draw error");
        }
        let event = rx.receive().await;
        app.handle(event);
    }
}
