//! WS2812 output — eight strings, one PIO state machine each.
//!
//! Replaces the STM32 build's SPI-4-bits-per-bit waveform encoder. PIO does the
//! 800 kHz one-wire timing directly and DMA feeds it, so the CPU cost per frame
//! is a few register writes.
//!
//! # State machine allocation
//!
//! Fixed here so the other PIO users do not have to negotiate for slots:
//!
//! | Block | SM0 | SM1 | SM2 | SM3 |
//! |---|---|---|---|---|
//! | PIO0 | *W6300 SPI transport* | string 1 | string 2 | string 3 |
//! | PIO1 | string 4 | string 5 | string 6 | string 7 |
//! | PIO2 | *DMX in* | *DMX out* | string 8 | spare |
//!
//! All three blocks hold the same 4-instruction WS2812 program; the state
//! machines within a block share it, which is why eight strings fit at all.
//!
//! # Why the strings are joined rather than awaited in turn
//!
//! Each `write` is DMA-backed and yields until its transfer completes. Awaiting
//! them sequentially would make the frame time the *sum* of eight strings
//! instead of the longest one — 144 ms rather than 18 ms at 600 LEDs. They are
//! joined so all eight clock out concurrently.

use common::channels::SmartLedChannelRx;
use common::events::SmartLedEvent;
use common::{LED_COLORS, SMARTLED_PORT_COUNT};
use defmt::*;
use embassy_futures::join::{join, join4};
use embassy_rp::peripherals::{PIO0, PIO1, PIO2};
use embassy_rp::pio_programs::ws2812::{Grb, PioWs2812};
use smart_leds::RGB8;

/// Number of physical WS2812 outputs on the carrier.
pub const STRINGS: usize = 8;

/// LEDs per string. Compile-time because the PIO driver takes it as a const
/// generic and sizes its DMA buffer from it.
///
/// This is the decided byte budget (1800 B/port = 600 RGB), not
/// `common::SMARTLED_NUM_LEDS_MAX` — which is still 1024 until Phase 3 narrows
/// it. The distinction matters here because `PioWs2812::write` only takes a
/// **fixed-size** array: every frame costs `MAX_LEDS` worth of wire time
/// whatever the configured string length. At 1024 that would be 30.7 ms and cap
/// refresh at 32 Hz; at 600 it is 18.0 ms, comfortably above the 44 Hz Art-Net
/// rate. Short strings still pay full frame time — acceptable at 600, which is
/// why the number is pinned here rather than inherited.
pub const MAX_LEDS: usize = 600;

/// All eight outputs. The types differ per string (each names its own PIO block
/// and state machine index), so they cannot live in an array — hence the
/// explicit fields.
pub struct Ws2812Outputs {
    pub s1: PioWs2812<'static, PIO0, 1, MAX_LEDS, Grb>,
    pub s2: PioWs2812<'static, PIO0, 2, MAX_LEDS, Grb>,
    pub s3: PioWs2812<'static, PIO0, 3, MAX_LEDS, Grb>,
    pub s4: PioWs2812<'static, PIO1, 0, MAX_LEDS, Grb>,
    pub s5: PioWs2812<'static, PIO1, 1, MAX_LEDS, Grb>,
    pub s6: PioWs2812<'static, PIO1, 2, MAX_LEDS, Grb>,
    pub s7: PioWs2812<'static, PIO1, 3, MAX_LEDS, Grb>,
    pub s8: PioWs2812<'static, PIO2, 2, MAX_LEDS, Grb>,
}

/// A dark string, for outputs with no data behind them yet.
pub static BLANK: [RGB8; MAX_LEDS] = [RGB8 { r: 0, g: 0, b: 0 }; MAX_LEDS];

impl Ws2812Outputs {
    /// Clock one frame out of every string concurrently.
    ///
    /// Returns once the slowest string has finished, so the caller can treat
    /// this as "the frame is on the wire".
    pub async fn render(&mut self, frames: [&[RGB8; MAX_LEDS]; STRINGS]) {
        join(
            join4(
                self.s1.write(frames[0]),
                self.s2.write(frames[1]),
                self.s3.write(frames[2]),
                self.s4.write(frames[3]),
            ),
            join4(
                self.s5.write(frames[4]),
                self.s6.write(frames[5]),
                self.s7.write(frames[6]),
                self.s8.write(frames[7]),
            ),
        )
        .await;
    }

    /// Drive every string dark. Used at boot so a reset does not leave the
    /// strips holding whatever the last frame was.
    pub async fn blank(&mut self) {
        self.render([&BLANK; STRINGS]).await;
    }
}


/// Render `LED_COLORS` to the strings whenever the router says it changed.
///
/// The frame is **staged into a local buffer before rendering**, not written
/// straight from `LED_COLORS`. Holding that lock across the render would block
/// the router for the whole 18 ms a frame takes to clock out, and the router is
/// on the other core feeding Art-Net. Staging costs one 14.4 KB copy per frame —
/// a few hundred microseconds at 150 MHz.
///
/// Staging also handles blanking: pixels past a port's configured count are
/// zeroed rather than left holding whatever the buffer had, which is what stops
/// a shortened string leaving stale LEDs lit.
#[embassy_executor::task]
pub async fn smart_led_task(mut out: Ws2812Outputs, rx: SmartLedChannelRx) {
    out.blank().await;
    info!("WS2812: {} strings ready, {} LEDs max each", STRINGS, MAX_LEDS);

    let mut stage = [[RGB8 { r: 0, g: 0, b: 0 }; MAX_LEDS]; STRINGS];

    loop {
        let SmartLedEvent::UpdateLEDs(counts) = rx.receive().await;

        {
            let colors = LED_COLORS.lock().await;
            for (i, dst) in stage.iter_mut().enumerate() {
                // `counts` is sized by SMARTLED_PORT_COUNT (8, matching the
                // eight physical strings since the Phase 3 settings widening).
                let n = if i < SMARTLED_PORT_COUNT {
                    (counts[i] as usize).min(MAX_LEDS).min(colors[i].len())
                } else {
                    0
                };
                dst[..n].copy_from_slice(&colors[i][..n]);
                dst[n..].fill(RGB8 { r: 0, g: 0, b: 0 });
            }
        }

        let (a, b) = stage.split_at(4);
        out.render([&a[0], &a[1], &a[2], &a[3], &b[0], &b[1], &b[2], &b[3]])
            .await;
    }
}
