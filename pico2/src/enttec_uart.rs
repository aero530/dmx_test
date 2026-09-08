//! Enttec DMX USB Pro widget on the carrier's FT232RNL USB-C port (UART0).
//!
//! PC lighting software that discovers Enttec hardware through the FTDI
//! driver stack (QLC+, D2XX/libftdi applications) cannot see a CDC serial
//! port, however faithfully it speaks the protocol. The Rev 2 carrier therefore
//! carries a real FT232RNL behind its own USB-C connector — genuine FTDI
//! silicon, so VID 0403 is legitimate — wired to UART0. No hardware flow
//! control: the FT232RNL's CTS#/RTS# are left open on the board, so hosts must
//! be configured for none (every DMX application defaults to that).
//!
//! | RP2350 | | FT232RNL |
//! |---|---|---|
//! | GP28 (UART0 TX) | → | RXD |
//! | GP13 (UART0 RX) | ← | TXD |
//!
//! GP28 used to be the TCA9555 `INT`; the buttons are polled now (`buttons.rs`).
//!
//! # Baud hunting
//!
//! The FT232R applies whatever baud the *host* configures to its UART, and
//! Enttec host software disagrees about that number: QLC+ opens the Pro at
//! 250 000, pyenttec and DmxPy at 57 600, other libraries at 115 200. A real
//! Pro never noticed because it is built on an FT245 (parallel FIFO, no baud).
//! We cannot read the host's setting from this side, so the task hunts:
//! it listens at each candidate rate in turn and locks on the first one that
//! yields a valid `0x7E … 0xE7` frame. Wrong rates announce themselves quickly
//! — framing errors, or a stream of bytes that never frames — and a host that
//! reopens the port at a different rate is caught the same way.
//!
//! The link runs 8N2 on our side: an 8N1 host accepts the extra stop bit, and
//! an 8N2 host (QLC+) would flag framing errors on 8N1 transmissions.

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{info, warn};
    } else {
        use defmt::{info, warn};
    }
}

use common::channels::{DmxChannelTx, DmxFeedbackChannelRx};
use common::enttec_protocol::{EnttecParser, MAX_PAYLOAD};
use common::event_router::DmxFeedbackEvent;
use common::ui::InputMode;
use embassy_futures::select::{select, Either};
use embassy_rp::uart::BufferedUart;
use embassy_time::{Duration, Timer};
use embedded_io_async::{Read, Write};

use crate::enttec_widget::{handle_message, ChangeForwarder, MSG_MAX};

/// Baud rates tried, most likely first. The list wraps.
pub const BAUD_CANDIDATES: [u32; 8] = [250_000, 115_200, 57_600, 38_400, 230_400, 460_800, 19_200, 9_600];

/// Framing / overrun / break errors before moving to the next candidate.
const HUNT_ERRORS: u8 = 3;

/// Bytes received without a single valid message before moving on — about two
/// full DMX frames' worth, so a host that is streaming gets a fair hearing.
const HUNT_GARBAGE: u32 = 1200;

/// How often the widget->host change forwarder looks at the buffer.
const FORWARD_POLL: Duration = Duration::from_millis(30);

/// Which candidate we are on and how confident we are in it.
struct BaudHunt {
    idx: usize,
    locked: bool,
    errors: u8,
    garbage: u32,
}

impl BaudHunt {
    const fn new() -> Self {
        Self {
            idx: 0,
            locked: false,
            errors: 0,
            garbage: 0,
        }
    }

    fn current(&self) -> u32 {
        BAUD_CANDIDATES[self.idx]
    }

    /// A valid message arrived: this is the right rate.
    fn confirm(&mut self) {
        if !self.locked {
            info!("FTDI widget: locked at {} baud", self.current());
        }
        self.locked = true;
        self.errors = 0;
        self.garbage = 0;
    }

    /// A byte that completed nothing. Returns true when it is time to move on.
    fn garbage(&mut self) -> bool {
        if self.locked {
            return false;
        }
        self.garbage += 1;
        self.garbage >= HUNT_GARBAGE
    }

    /// A line error. Returns true when it is time to move on.
    fn error(&mut self) -> bool {
        self.errors += 1;
        self.errors >= HUNT_ERRORS
    }

    /// Advance to the next candidate and return it.
    fn next(&mut self) -> u32 {
        self.idx = (self.idx + 1) % BAUD_CANDIDATES.len();
        self.locked = false;
        self.errors = 0;
        self.garbage = 0;
        self.current()
    }
}

/// Serve the Enttec protocol on the FT232RNL port.
#[embassy_executor::task]
pub async fn enttec_uart_task(mut uart: BufferedUart, tx: DmxChannelTx, mut rx: DmxFeedbackChannelRx) {
    let mut parser = EnttecParser::new();
    let mut forwarder = ChangeForwarder::new();
    let mut hunt = BaudHunt::new();
    let mut msg = [0_u8; MSG_MAX];
    let mut payload = [0_u8; MAX_PAYLOAD];
    let mut chunk = [0_u8; 64];
    let mut input_mode = InputMode::default();

    uart.set_baudrate(hunt.current());
    info!("FTDI widget: listening on UART0, trying {} baud first", hunt.current());

    loop {
        if let Some(DmxFeedbackEvent::Mode(new_mode, artnet_addr, _, _)) = rx.try_changed() {
            input_mode = new_mode;
            forwarder.set_artnet_base(artnet_addr.buffer_base());
        }

        match select(uart.read(&mut chunk), Timer::after(FORWARD_POLL)).await {
            Either::First(Ok(n)) => {
                for &byte in &chunk[..n] {
                    if !parser.feed(byte) {
                        if hunt.garbage() {
                            let baud = hunt.next();
                            uart.set_baudrate(baud);
                            parser = EnttecParser::new();
                            info!("FTDI widget: nothing framed, trying {} baud", baud);
                            break;
                        }
                        continue;
                    }
                    hunt.confirm();
                    // Copy the payload out so `parser` isn't borrowed across
                    // the await below.
                    let len = parser.payload().len();
                    payload[..len].copy_from_slice(parser.payload());
                    if let Some(reply) = handle_message(parser.label(), &payload[..len], input_mode, &tx, &mut msg).await {
                        if uart.write_all(&msg[..reply]).await.is_err() {
                            warn!("FTDI widget: reply write failed");
                        }
                    }
                }
            }
            Either::First(Err(e)) => {
                // Wrong baud shows up here as framing errors; so does a host
                // that reopened the port at a new rate after we had locked.
                if hunt.error() {
                    let baud = hunt.next();
                    uart.set_baudrate(baud);
                    parser = EnttecParser::new();
                    info!("FTDI widget: line errors ({:?}), trying {} baud", e, baud);
                }
            }
            Either::Second(()) => {
                // Forward received DMX to the host (label 5) on change — only
                // once we know the rate, or we would be shouting into noise.
                if hunt.locked {
                    if let Some(len) = forwarder.poll(input_mode, &mut msg).await {
                        if uart.write_all(&msg[..len]).await.is_err() {
                            warn!("FTDI widget: forward write failed");
                        }
                    }
                }
            }
        }
    }
}
