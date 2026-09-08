//! Transport-agnostic Enttec DMX USB Pro widget core, shared by the bench
//! binaries: `enttec_test` (CDC-ACM serial) and `ftdi_test` (emulated
//! FT232R). Each transport hands bytes from the host to [`Widget::feed`],
//! sends back whatever it returns, and runs [`dmx_task`] to keep the wire
//! refreshed.
//!
//! The binary that includes this module must also declare `mod dmx_pio;`
//! and `mod enttec_protocol;` at its crate root.

use embassy_rp::gpio::Output;
use embassy_rp::peripherals::PIO0;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;
use embassy_time::{Duration, Instant};
use portable_atomic::{AtomicU32, Ordering};

use crate::dmx_pio::{DMX_FRAME_SIZE, PioDmxTx};
use crate::enttec_protocol::{
    BREAK_TIME, END_DELIMITER, EnttecParser, FIRMWARE_VERSION_LSB, FIRMWARE_VERSION_MSB,
    LABEL_GET_PARAMS, LABEL_GET_SERIAL, LABEL_OUTPUT_DMX, LABEL_RECEIVE_ON_CHANGE,
    LABEL_SET_PARAMS, MAB_TIME, MAX_PAYLOAD, REFRESH_RATE, SERIAL_NUMBER, START_DELIMITER,
};

/// One DMX packet: slot 0 is the start code, slots 1..=512 the channels.
pub type DmxFrame = [u8; DMX_FRAME_SIZE];

/// Largest framed reply: 4-byte header + payload + end delimiter.
pub const REPLY_MAX: usize = 4 + MAX_PAYLOAD + 1;

/// Latest frame from the host, transport -> [`dmx_task`] (latest wins).
pub static HOST_FRAME: Watch<CriticalSectionRawMutex, DmxFrame, 1> = Watch::new();

/// DMX packets put on the wire since the last status line.
pub static DMX_PACKETS: AtomicU32 = AtomicU32::new(0);

/// What a completed host message asks the transport to do.
pub enum Action {
    /// A label-6 frame was accepted and published to the DMX task.
    Frame,
    /// `reply[..n]` holds a complete framed message to send to the host.
    Reply(usize),
    /// Accepted with nothing to send back (labels 4 and 8).
    Ack,
    /// Unknown label, ignored.
    Unsupported,
}

/// Parser state plus the counters behind the once-a-second status line.
pub struct Widget {
    parser: EnttecParser,
    frame: DmxFrame,
    host_frames: u32,
    last_report: Instant,
}

impl Widget {
    pub fn new() -> Self {
        Self {
            parser: EnttecParser::new(),
            frame: [0; DMX_FRAME_SIZE],
            host_frames: 0,
            last_report: Instant::now(),
        }
    }

    /// Feed one byte from the host. Returns `Some` when it completed a
    /// message; for [`Action::Reply`] the framed reply is in `reply`.
    pub fn feed(&mut self, byte: u8, reply: &mut [u8; REPLY_MAX]) -> Option<Action> {
        if !self.parser.feed(byte) {
            return None;
        }
        let payload = self.parser.payload();
        Some(match self.parser.label() {
            LABEL_OUTPUT_DMX => {
                // Payload = start code + channels. Channels the host did not
                // send go dark, like a widget that transmits exactly the
                // received universe.
                let n = payload.len().min(DMX_FRAME_SIZE);
                self.frame[..n].copy_from_slice(&payload[..n]);
                self.frame[n..].fill(0);
                HOST_FRAME.sender().send(self.frame);
                self.host_frames += 1;
                Action::Frame
            }
            LABEL_GET_PARAMS => {
                // Reply: firmware version, break, MAB, refresh rate, then the
                // requested amount of user configuration data (all zeroes).
                let user_size = if payload.len() >= 2 {
                    u16::from_le_bytes([payload[0], payload[1]]) as usize
                } else {
                    0
                }
                .min(MAX_PAYLOAD - 5);
                let mut body = [0_u8; MAX_PAYLOAD];
                body[..5].copy_from_slice(&[FIRMWARE_VERSION_LSB, FIRMWARE_VERSION_MSB, BREAK_TIME, MAB_TIME, REFRESH_RATE]);
                log::info!("Enttec: get-params ({user_size} user bytes)");
                Action::Reply(frame_message(LABEL_GET_PARAMS, &body[..5 + user_size], reply))
            }
            LABEL_GET_SERIAL => {
                log::info!("Enttec: get-serial");
                Action::Reply(frame_message(LABEL_GET_SERIAL, &SERIAL_NUMBER, reply))
            }
            LABEL_SET_PARAMS => {
                log::info!("Enttec: set-params ({} bytes) accepted; timing is fixed by the PIO program", payload.len());
                Action::Ack
            }
            LABEL_RECEIVE_ON_CHANGE => {
                log::info!("Enttec: receive-on-change accepted (this test never receives)");
                Action::Ack
            }
            other => {
                log::warn!("Enttec: unsupported label {other} ({} bytes)", payload.len());
                Action::Unsupported
            }
        })
    }

    /// Log the status line once a second; call from the transport loop.
    pub fn report(&mut self) {
        if self.last_report.elapsed() < Duration::from_secs(1) {
            return;
        }
        let dmx = DMX_PACKETS.swap(0, Ordering::Relaxed);
        log::info!(
            "host {} frames/s | DMX out {dmx} pkt/s | SC {:#04x} ch1-3 = {} {} {}",
            self.host_frames,
            self.frame[0],
            self.frame[1],
            self.frame[2],
            self.frame[3]
        );
        self.host_frames = 0;
        self.last_report = Instant::now();
    }
}

impl Default for Widget {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame `payload` as an Enttec message into `out`; returns the byte count.
pub fn frame_message(label: u8, payload: &[u8], out: &mut [u8; REPLY_MAX]) -> usize {
    let n = payload.len();
    out[0] = START_DELIMITER;
    out[1] = label;
    out[2] = (n & 0xFF) as u8;
    out[3] = (n >> 8) as u8;
    out[4..4 + n].copy_from_slice(payload);
    out[4 + n] = END_DELIMITER;
    5 + n
}

/// Retransmit the latest host frame forever (zeros until the first arrives),
/// ~43 packets/s regardless of the host rate. Holds the RS-485 driver enabled.
#[embassy_executor::task]
pub async fn dmx_task(mut dmx_tx: PioDmxTx<'static, PIO0, 0>, mut dmx_en: Output<'static>) {
    let mut host_rx = defmt::unwrap!(HOST_FRAME.receiver());
    let mut frame: DmxFrame = [0; DMX_FRAME_SIZE];

    loop {
        if let Some(new_frame) = host_rx.try_changed() {
            frame = new_frame;
        }
        dmx_en.set_high(); // belt and braces: never let the driver drop out
        // ~23 ms per packet: BREAK + MAB + 513 slots at 250 kbaud.
        dmx_tx.write(&frame).await;
        DMX_PACKETS.fetch_add(1, Ordering::Relaxed);
    }
}
