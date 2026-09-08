//! Enttec DMX USB Pro widget emulation over the module's native USB (CDC-ACM).
//!
//! Presents the board to a PC as a serial device speaking the Enttec DMX USB
//! Pro protocol, so serial-port-agnostic lighting software (xLights, Vixen,
//! OLA `usbpro`, scripts) can use it as a USB-DMX interface. The protocol
//! handling itself is shared with the FT232RNL port in `enttec_widget.rs`; this
//! file is only the CDC transport and the composite device.
//!
//! Software that discovers Enttec hardware through the FTDI driver (QLC+,
//! D2XX applications) cannot see a CDC port at all — that is what the
//! carrier's FT232RNL USB-C port (`enttec_uart.rs`) exists for.
//!
//! This task owns the USB peripheral and builds a composite device: interface 1
//! is always the Enttec widget, interface 2 is always the console line protocol
//! (see `console_usb.rs`), and with the `usb` logging feature enabled a third
//! CDC-ACM interface carries the `log` output. The host sees the serial ports
//! in that order.

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::info;
    } else {
        use defmt::info;
    }
}

use embassy_futures::select::{Either, select};
use embassy_rp::peripherals;
use embassy_rp::usb::Driver;
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;

use crate::channels::{DmxChannelTx, DmxFeedbackChannelRx, GlobalDataChannelRx, RouterChannelTx};
use crate::enttec_protocol::{EnttecParser, MAX_PAYLOAD};
use crate::enttec_widget::{handle_message, ChangeForwarder, MSG_MAX};
use crate::event_router::DmxFeedbackEvent;
use crate::ui::InputMode;

type UsbClass<'d> = CdcAcmClass<'d, Driver<'d, peripherals::USB>>;

/// Send a framed message chunked into full-speed USB packets, with a ZLP when
/// the total lands on a packet boundary so the host flushes the transfer.
async fn send_bytes(class: &mut UsbClass<'_>, bytes: &[u8]) -> Result<(), EndpointError> {
    for chunk in bytes.chunks(64) {
        class.write_packet(chunk).await?;
    }
    if bytes.len().is_multiple_of(64) {
        class.write_packet(&[]).await?;
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
    // Composite device with Interface Association Descriptors: without them
    // Windows will not bind its CDC driver to each serial function separately.
    // Verified on the rp2040_dmx bench firmware, which uses the same layout.
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

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
        let mut forwarder = ChangeForwarder::new();
        let mut packet = [0_u8; 64];
        let mut payload = [0_u8; MAX_PAYLOAD];
        let mut msg = [0_u8; MSG_MAX];
        let mut input_mode = InputMode::default();

        loop {
            class.wait_connection().await;
            info!("Enttec: USB host connected");

            'connected: loop {
                if let Some(DmxFeedbackEvent::Mode(new_mode, artnet_addr, _, _)) = rx.try_changed() {
                    input_mode = new_mode;
                    forwarder.set_artnet_base(artnet_addr.buffer_base());
                }

                match select(class.read_packet(&mut packet), Timer::after_millis(30)).await {
                    Either::First(Ok(n)) => {
                        for &byte in &packet[..n] {
                            if parser.feed(byte) {
                                // The payload is copied out so `parser` isn't
                                // borrowed across the await point below.
                                let length = parser.payload().len();
                                payload[..length].copy_from_slice(parser.payload());
                                if let Some(len) = handle_message(parser.label(), &payload[..length], input_mode, &tx, &mut msg).await {
                                    if send_bytes(&mut class, &msg[..len]).await.is_err() {
                                        break 'connected;
                                    }
                                }
                            }
                        }
                    }
                    Either::First(Err(_)) => break 'connected,
                    Either::Second(()) => {
                        // Forward received DMX to the host (label 5) on change.
                        if let Some(len) = forwarder.poll(input_mode, &mut msg).await {
                            if send_bytes(&mut class, &msg[..len]).await.is_err() {
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
