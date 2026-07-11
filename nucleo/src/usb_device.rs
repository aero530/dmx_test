//! Enttec DMX USB Pro widget emulation over USB CDC-ACM.
//!
//! Presents the board to a PC as a serial device speaking the Enttec
//! DMX USB Pro protocol, so standard lighting software can use it as a
//! USB-DMX interface:
//!
//! * Label 6 (Output Only Send DMX Packet): the frame from the PC is stored
//!   in `DMX_BUFFER` (DMX-style: start code at index 0). When the operating
//!   mode is USB>DMX, the `dmx_i2c` task pushes it to the RP2040 bridge for
//!   transmission on the wired DMX port, and the router renders it on the
//!   local LED outputs.
//! * Label 5 (Received DMX Packet): in DMX / ArtNet input modes, received
//!   frames are forwarded to the PC whenever they change.
//! * Labels 3 / 4 / 8 / 10 (widget parameters, receive-on-change, serial
//!   number) are answered so host software recognizes the widget. Output
//!   timing values are fixed by the bridge's PIO program, so Set Widget
//!   Parameters is accepted but ignored.
//!
//! Message framing: 0x7E, label, length LSB, length MSB, payload, 0xE7.
//!
//! This task owns the USB peripheral and builds a composite device:
//! interface 1 is always the Enttec widget, interface 2 is always the
//! console line protocol (see `console_usb.rs`), and with the `usb` logging
//! feature enabled a third CDC-ACM interface carries the `log` output. The
//! host sees the serial ports in that order.

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{info, warn};
    } else {
        use defmt::{info, warn};
    }
}

use embassy_futures::select::{Either, select};
use embassy_stm32::peripherals;
use embassy_stm32::usb::Driver;
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;

use crate::artnet::PortAddress;
use crate::channels::{DmxChannelTx, DmxFeedbackChannelRx, GlobalDataChannelRx, RouterChannelTx};
use crate::event_router::{DmxEvent, DmxFeedbackEvent, PacketAddress};
use crate::ui::InputMode;
use crate::{DMX_BUFFER, DMX_BUFF_SIZE, DMX_UNIVERSE_SIZE};

const START_DELIMITER: u8 = 0x7E;
const END_DELIMITER: u8 = 0xE7;

const LABEL_GET_PARAMS: u8 = 3;
const LABEL_SET_PARAMS: u8 = 4;
const LABEL_RECEIVED_DMX: u8 = 5;
const LABEL_OUTPUT_DMX: u8 = 6;
const LABEL_RECEIVE_ON_CHANGE: u8 = 8;
const LABEL_GET_SERIAL: u8 = 10;

/// Largest meaningful payload: start code + 512 channels (label 6).
const MAX_PAYLOAD: usize = DMX_BUFF_SIZE;

/// Widget parameters reported to the host. Break/MAB are in 10.67us units
/// and describe the actual timing of the bridge's PIO output program
/// (176us break, 16us MAB, ~43 packet/s refresh capped at the 40 the
/// protocol can express).
const FIRMWARE_VERSION_LSB: u8 = 44; // v1.44
const FIRMWARE_VERSION_MSB: u8 = 1;
const BREAK_TIME: u8 = 16;
const MAB_TIME: u8 = 2;
const REFRESH_RATE: u8 = 40;
const SERIAL_NUMBER: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

enum ParseState {
    WaitForStart,
    Label,
    LengthLsb,
    LengthMsb,
    Data,
    WaitForEnd,
}

/// Incremental parser for the Enttec message framing. Oversized messages are
/// consumed and dropped so the stream stays in sync.
struct EnttecParser {
    state: ParseState,
    label: u8,
    length: usize,
    pos: usize,
    oversize: bool,
    payload: [u8; MAX_PAYLOAD],
}

impl EnttecParser {
    const fn new() -> Self {
        Self {
            state: ParseState::WaitForStart,
            label: 0,
            length: 0,
            pos: 0,
            oversize: false,
            payload: [0; MAX_PAYLOAD],
        }
    }

    /// Feed one byte; returns true when a complete valid message is available
    /// in `self.label` / `self.payload[..self.length]`.
    fn feed(&mut self, byte: u8) -> bool {
        match self.state {
            ParseState::WaitForStart => {
                if byte == START_DELIMITER {
                    self.state = ParseState::Label;
                }
            }
            ParseState::Label => {
                self.label = byte;
                self.state = ParseState::LengthLsb;
            }
            ParseState::LengthLsb => {
                self.length = byte as usize;
                self.state = ParseState::LengthMsb;
            }
            ParseState::LengthMsb => {
                self.length |= (byte as usize) << 8;
                self.pos = 0;
                self.oversize = self.length > MAX_PAYLOAD;
                self.state = if self.length == 0 { ParseState::WaitForEnd } else { ParseState::Data };
            }
            ParseState::Data => {
                if !self.oversize {
                    self.payload[self.pos] = byte;
                }
                self.pos += 1;
                if self.pos >= self.length {
                    self.state = ParseState::WaitForEnd;
                }
            }
            ParseState::WaitForEnd => {
                self.state = ParseState::WaitForStart;
                if byte == END_DELIMITER && !self.oversize {
                    return true;
                }
                // Framing error or dropped oversize message: hunt for the
                // next start delimiter.
            }
        }
        false
    }
}

type UsbClass<'d> = CdcAcmClass<'d, Driver<'d, peripherals::USB>>;

/// Frame a payload in the Enttec message format and send it, chunked into
/// full-speed USB packets (with a ZLP when the total lands on a packet
/// boundary, so the host flushes the transfer).
async fn send_message(class: &mut UsbClass<'_>, label: u8, payload: &[u8]) -> Result<(), EndpointError> {
    let header = [START_DELIMITER, label, (payload.len() & 0xFF) as u8, ((payload.len() >> 8) & 0xFF) as u8];

    let mut chunk = [0_u8; 64];
    let mut fill = 0_usize;
    for &byte in header.iter().chain(payload.iter()).chain(core::iter::once(&END_DELIMITER)) {
        chunk[fill] = byte;
        fill += 1;
        if fill == chunk.len() {
            class.write_packet(&chunk).await?;
            fill = 0;
        }
    }
    if fill > 0 {
        class.write_packet(&chunk[..fill]).await?;
    } else {
        class.write_packet(&[]).await?;
    }
    Ok(())
}

/// Handle one complete message from the host.
async fn handle_message(
    class: &mut UsbClass<'_>,
    label: u8,
    payload: &[u8],
    input_mode: InputMode,
    tx: &DmxChannelTx,
) -> Result<(), EndpointError> {
    match label {
        LABEL_OUTPUT_DMX => {
            // Only accept host DMX in USB>DMX mode so a connected PC can't
            // clobber live wired-DMX or Art-Net data in the other modes.
            if input_mode == InputMode::UsbToDmx && !payload.is_empty() {
                let n = payload.len().min(DMX_BUFF_SIZE);
                {
                    let mut dmx_buffer = DMX_BUFFER.lock().await;
                    dmx_buffer[0..n].copy_from_slice(&payload[..n]);
                    // Channels beyond what the host sent go dark, matching a
                    // widget that transmits exactly the received universe.
                    dmx_buffer[n..DMX_BUFF_SIZE].fill(0);
                }
                let _ = tx.try_send(DmxEvent::DmxPacket(PacketAddress::new(PortAddress::new(0, 0, 0), 0)));
            }
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

            let mut reply = [0_u8; MAX_PAYLOAD];
            reply[0] = FIRMWARE_VERSION_LSB;
            reply[1] = FIRMWARE_VERSION_MSB;
            reply[2] = BREAK_TIME;
            reply[3] = MAB_TIME;
            reply[4] = REFRESH_RATE;
            send_message(class, LABEL_GET_PARAMS, &reply[..5 + user_size]).await?;
        }
        LABEL_GET_SERIAL => {
            send_message(class, LABEL_GET_SERIAL, &SERIAL_NUMBER).await?;
        }
        LABEL_SET_PARAMS => {
            // Output timing is fixed by the bridge's PIO program.
            info!("Enttec: set-params accepted (timing is fixed in the bridge)");
        }
        LABEL_RECEIVE_ON_CHANGE => {
            // Received DMX is always forwarded on change.
            info!("Enttec: receive-on-change request accepted");
        }
        other => {
            warn!("Enttec: unsupported label {}", other);
        }
    }
    Ok(())
}

/// Composite USB device task: Enttec widget + console (+ logger).
#[embassy_executor::task]
pub async fn usb_device_task(
    driver: Driver<'static, peripherals::USB>,
    tx: DmxChannelTx,
    mut rx: DmxFeedbackChannelRx,
    router_tx: RouterChannelTx,
    global_rx: GlobalDataChannelRx,
) {
    let mut config = embassy_usb::Config::new(0xc0de, 0xdcaf);
    config.manufacturer = Some("EQUUS");
    config.product = Some("DMX USB Pro compatible");
    config.serial_number = Some("00000001");
    config.max_packet_size_0 = 64;

    let mut config_descriptor = [0; 320];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut state = State::new();
    // Declared alongside the other USB state so they outlive everything
    // sharing the builder's lifetime.
    let mut console_state = State::new();
    #[cfg(feature = "usb")]
    let mut logger_state = State::new();

    let mut builder = embassy_usb::Builder::new(driver, config, &mut config_descriptor, &mut bos_descriptor, &mut [], &mut control_buf);
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

    // Second CDC-ACM interface: console line protocol.
    let mut console_class = CdcAcmClass::new(&mut builder, &mut console_state, 64);

    // Third CDC-ACM interface carrying the `log` output.
    #[cfg(feature = "usb")]
    let logger_class = CdcAcmClass::new(&mut builder, &mut logger_state, 64);

    let mut usb = builder.build();

    let protocol = async {
        let mut parser = EnttecParser::new();
        let mut packet = [0_u8; 64];
        let mut input_mode = InputMode::default();
        let mut artnet_universe: u8 = 0;
        // Last frame forwarded to the host, DMX-style (start code at 0)
        let mut forwarded = [0_u8; DMX_BUFF_SIZE];

        loop {
            class.wait_connection().await;
            info!("Enttec: USB host connected");

            'connected: loop {
                if let Some(DmxFeedbackEvent::Mode(new_mode, universe)) = rx.try_changed() {
                    input_mode = new_mode;
                    artnet_universe = universe;
                }

                match select(class.read_packet(&mut packet), Timer::after_millis(30)).await {
                    Either::First(Ok(n)) => {
                        for i in 0..n {
                            if parser.feed(packet[i]) {
                                let length = parser.length.min(MAX_PAYLOAD);
                                // The payload is copied out so `parser` isn't
                                // borrowed across the await point below.
                                let mut payload = [0_u8; MAX_PAYLOAD];
                                payload[..length].copy_from_slice(&parser.payload[..length]);
                                if handle_message(&mut class, parser.label, &payload[..length], input_mode, &tx).await.is_err() {
                                    break 'connected;
                                }
                            }
                        }
                    }
                    Either::First(Err(_)) => break 'connected,
                    Either::Second(()) => {
                        // Forward received DMX to the host (label 5) on change.
                        let mut current = [0_u8; DMX_BUFF_SIZE];
                        match input_mode {
                            InputMode::Dmx => {
                                let dmx_buffer = DMX_BUFFER.lock().await;
                                current.copy_from_slice(&dmx_buffer[0..DMX_BUFF_SIZE]);
                            }
                            InputMode::ArtNet | InputMode::ArtNetToDmx => {
                                let start = (artnet_universe as usize) * DMX_UNIVERSE_SIZE;
                                let dmx_buffer = DMX_BUFFER.lock().await;
                                current[0] = 0x00;
                                current[1..].copy_from_slice(&dmx_buffer[start..start + DMX_UNIVERSE_SIZE]);
                            }
                            // The host is the data source; nothing to forward.
                            InputMode::UsbToDmx => continue,
                        }

                        if current != forwarded {
                            forwarded = current;
                            // Payload: status (0 = no errors) + start code + channels
                            let mut msg = [0_u8; 1 + DMX_BUFF_SIZE];
                            msg[1..].copy_from_slice(&current);
                            if send_message(&mut class, LABEL_RECEIVED_DMX, &msg).await.is_err() {
                                break 'connected;
                            }
                        }
                    }
                }
            }
            info!("Enttec: USB host disconnected");
        }
    };

    let console = crate::console_usb::run(&mut console_class, router_tx, global_rx);

    cfg_if! {
        if #[cfg(feature = "usb")] {
            let logger = embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, logger_class);
            embassy_futures::join::join4(usb.run(), protocol, console, logger).await;
        } else {
            embassy_futures::join::join3(usb.run(), protocol, console).await;
        }
    }
}
