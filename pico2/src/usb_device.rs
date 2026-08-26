//! Enttec DMX USB Pro widget emulation over USB CDC-ACM.
//!
//! Presents the board to a PC as a serial device speaking the Enttec
//! DMX USB Pro protocol, so standard lighting software can use it as a
//! USB-DMX interface:
//!
//! * Label 6 (Output Only Send DMX Packet): the frame from the PC is stored
//!   in `DMX_BUFFER` (DMX-style: start code at index 0). The router renders
//!   it on the local LED outputs; in USB>DMX mode the (future) DMX TX task
//!   will also transmit it on the wired port via PIO2 SM1.
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
use embassy_rp::peripherals;
use embassy_rp::usb::Driver;
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;

use crate::artnet::PortAddress;
use crate::channels::{DmxChannelTx, DmxFeedbackChannelRx, GlobalDataChannelRx, RouterChannelTx};
use crate::event_router::{DmxEvent, DmxFeedbackEvent, PacketAddress};
use crate::ui::InputMode;
use crate::{DMX_BUFFER, DMX_BUFF_SIZE, DMX_UNIVERSE_SIZE};

use crate::enttec_protocol::{
    BREAK_TIME, END_DELIMITER, EnttecParser, FIRMWARE_VERSION_LSB, FIRMWARE_VERSION_MSB,
    LABEL_GET_PARAMS, LABEL_GET_SERIAL, LABEL_OUTPUT_DMX, LABEL_RECEIVE_ON_CHANGE,
    LABEL_RECEIVED_DMX, LABEL_SET_PARAMS, MAB_TIME, MAX_PAYLOAD, REFRESH_RATE, SERIAL_NUMBER,
    START_DELIMITER,
};

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
    // Placeholder VID/PID — fine on the bench (lighting software opens the
    // serial port by protocol, not VID), but allocate a real pair (pid.codes)
    // before anything ships.
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
        // Buffer index (sub-net:universe) of the configured Art-Net address
        let mut artnet_sub_uni: usize = 0;
        // Last frame forwarded to the host, DMX-style (start code at 0)
        let mut forwarded = [0_u8; DMX_BUFF_SIZE];

        loop {
            class.wait_connection().await;
            info!("Enttec: USB host connected");

            'connected: loop {
                if let Some(DmxFeedbackEvent::Mode(new_mode, artnet_addr)) = rx.try_changed() {
                    input_mode = new_mode;
                    // buffer_base, NOT sub_uni: the raw value can reach 255
                    // while the buffer holds 64 universes — the slice below
                    // would panic on an out-of-range configured address.
                    artnet_sub_uni = artnet_addr.buffer_base();
                }

                match select(class.read_packet(&mut packet), Timer::after_millis(30)).await {
                    Either::First(Ok(n)) => {
                        for &byte in &packet[..n] {
                            if parser.feed(byte) {
                                let length = parser.payload().len();
                                // The payload is copied out so `parser` isn't
                                // borrowed across the await point below.
                                let mut payload = [0_u8; MAX_PAYLOAD];
                                payload[..length].copy_from_slice(parser.payload());
                                if handle_message(&mut class, parser.label(), &payload[..length], input_mode, &tx).await.is_err() {
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
                                let start = artnet_sub_uni * DMX_UNIVERSE_SIZE;
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
