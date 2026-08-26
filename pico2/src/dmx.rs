//! Wired DMX-512 task.
//!
//! The RP2040 bridge is gone: DMX now runs in-process on PIO2, which deletes
//! the I²C bridge protocol, its 30 ms poll, and the whole class of bugs the Rev 1
//! review found in that link (§1.1, §1.2). The PIO driver in `dmx_pio.rs` is the
//! `rp2040_dmx` one, unchanged — it derives its clock divider from
//! `clk_sys_freq()`, so it adapts from the RP2040's 125 MHz to the RP2350's
//! 150 MHz on its own.

use common::channels::{DmxFeedbackChannelRx, CHANNEL_DMX};
use common::event_router::{DmxEvent, DmxFeedbackEvent, PacketAddress};
use common::artnet::PortAddress;
use common::ui::InputMode;
use common::DMX_BUFFER;
use defmt::*;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::PIO2;
use embassy_time::{with_timeout, Duration};

use crate::dmx_pio::{PioDmxRx, PioDmxTx, DMX_FRAME_SIZE};

/// How long without a frame before saying so. DMX refreshes at ~44 Hz, so a
/// second of silence means the cable is out or the console stopped.
const SILENCE: Duration = Duration::from_millis(1000);

/// Receive wired DMX into `DMX_BUFFER` and notify the router.
///
/// The frame is written at offset 0 with the start code at index 0, matching
/// what the STM32 build did — so universe 0's channels live at `[1..513]` and
/// the router's existing indexing is unchanged.
///
/// Transmit is not driven yet: it belongs to the `ArtNet>DMX` and `USB>DMX`
/// modes, which need the settings from the router. `de` is held low (receive)
/// until then.
#[embassy_executor::task]
pub async fn dmx_task(
    mut rx: PioDmxRx<'static, PIO2, 0>,
    _tx: PioDmxTx<'static, PIO2, 1>,
    mut de: Output<'static>,
    mut feedback: DmxFeedbackChannelRx,
) {
    // Rev 2 direction drive is fail-safe: R46 biases the U14 opto LED on
    // (receive) whenever GP10 is undriven; GP10 only drives Q1's gate
    // (2N7002 via R36, R47 hold-down), so high = LED off = transmit. Gate
    // current is negligible — default drive strength is fine.
    de.set_low(); // Q1 off, opto LED on: RS-485 driver disabled, receiver enabled
    info!("DMX: receiving on PIO2 SM0");

    let mut frame = [0u8; DMX_FRAME_SIZE];
    let mut quiet = false;
    let mut input_mode = InputMode::default();

    loop {
        if let Some(DmxFeedbackEvent::Mode(new_mode, _addr)) = feedback.try_changed() {
            input_mode = new_mode;
        }

        match with_timeout(SILENCE, rx.read(&mut frame)).await {
            Ok(n) => {
                if quiet {
                    info!("DMX: signal restored");
                    quiet = false;
                }
                // Same gate every other input has: only the active source may
                // write the buffer. A console left plugged into the DMX port
                // while in an Art-Net mode must not stomp universe 0.
                if input_mode != InputMode::Dmx {
                    continue;
                }
                // Only NULL start code frames are dimmer data; RDM and other
                // alternate start codes must not be rendered as channels.
                if n == 0 || frame[0] != 0x00 {
                    continue;
                }
                {
                    let mut buf = DMX_BUFFER.lock().await;
                    buf[0..n].copy_from_slice(&frame[..n]);
                    // A short packet defines the rest of the universe as
                    // unchanged per DMX; here the simpler and safer contract
                    // (matching the USB input path) is: what the console
                    // doesn't send goes dark.
                    buf[n..DMX_FRAME_SIZE].fill(0);
                }
                // Wired DMX is always Port-Address 0:0:0. A full channel just
                // means the router has not drained the last frame yet, and the
                // newest frame is the one that matters, so dropping is correct.
                let _ = CHANNEL_DMX
                    .sender()
                    .try_send(DmxEvent::DmxPacket(PacketAddress::new(
                        PortAddress::new(0, 0, 0),
                        0,
                    )));
            }
            Err(_) => {
                if !quiet {
                    warn!("DMX: no data");
                    quiet = true;
                }
            }
        }
    }
}
