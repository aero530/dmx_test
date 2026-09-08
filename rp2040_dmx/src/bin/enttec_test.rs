//! Enttec DMX USB Pro emulation bench test for the Rev 1 carrier with only
//! the RP2040 Pico fitted (no Nucleo) — CDC-ACM transport.
//!
//! The Pico enumerates as a composite USB device with two CDC-ACM serial
//! ports — the same layout the `pico2` firmware uses:
//!
//! * **port 1 — Enttec widget.** Speaks the Enttec DMX USB Pro protocol
//!   (`0x7E, label, len LSB, len MSB, payload, 0xE7`). Label 6 frames from
//!   the host are retransmitted continuously on the wired DMX port; labels 3
//!   (Get Widget Parameters) and 10 (Get Serial) are answered so host
//!   software recognises the widget; 4 and 8 are accepted and ignored.
//! * **port 2 — log output.** One status line per second: host frame rate,
//!   DMX packet rate, and the start code + first three channels of the last
//!   frame received.
//!
//! The message parser is the exact source that ships in the real firmware
//! (`common/src/enttec_protocol.rs`), included by path rather than as a
//! crate dependency because `common` also drags in `alloc`. The widget
//! logic itself lives in `src/enttec_widget.rs`, shared with `ftdi_test`.
//!
//! DMX output starts at boot with all channels at 0 and follows whatever the
//! host last sent (latest frame wins, ~43 packets/s regardless of the host
//! rate). The onboard LED (GP25) toggles on every frame received from the
//! host, so a steady flicker means USB data is flowing.
//!
//! Pins are the Rev 1 wiring, identical to `dmx_tx_test.rs`: GP4 = DMX.TX,
//! GP3 = DMX.EN (held high = transmit). Same power/jumper checklist as that
//! test — JP22 in, JP23 out, USB into the Pico.
//!
//! Host side: `tools/enttec_host.py` finds the widget port, queries labels
//! 3 and 10, then streams frames with a selectable pattern.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_futures::join::join3;
use embassy_futures::select::{Either, select};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{PIO0, USB};
use embassy_rp::pio::Pio;
use embassy_rp::usb::Driver;
use embassy_rp::{bind_interrupts, dma, pio, usb};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use {defmt_rtt as _, panic_probe as _};

#[allow(dead_code)] // the receiver half is unused here
#[path = "../dmx_pio.rs"]
mod dmx_pio;
use dmx_pio::{DMX_FRAME_SIZE, PioDmxTx, PioDmxTxProgram};

/// `enttec_protocol.rs` sizes its payload buffer from `crate::DMX_BUFF_SIZE`
/// (start code + 512 channels), the same constant `common` defines.
pub const DMX_BUFF_SIZE: usize = DMX_FRAME_SIZE;

#[allow(dead_code)] // labels only the widget->host direction uses
#[path = "../../../common/src/enttec_protocol.rs"]
mod enttec_protocol;

#[path = "../enttec_widget.rs"]
mod enttec_widget;
use enttec_widget::{Action, REPLY_MAX, Widget, dmx_task};

type UsbClass<'d> = CdcAcmClass<'d, Driver<'d, USB>>;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<embassy_rp::peripherals::DMA_CH0>;
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Onboard LED toggles on every frame received from the host.
    let led = Output::new(p.PIN_25, Level::Low);

    // RS-485 direction. High = U7 LED off = THVD1400 DE/RE high = transmit.
    let dmx_en = Output::new(p.PIN_3, Level::High);

    // DMX transmitter on GP4, owned by its own task so the wire keeps
    // refreshing at ~43 packets/s no matter what the host is doing.
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let tx_program = PioDmxTxProgram::new(&mut common);
    let dmx_tx = PioDmxTx::new(&mut common, sm0, p.DMA_CH0, Irqs, p.PIN_4, &tx_program);
    spawner.spawn(defmt::unwrap!(dmx_task(dmx_tx, dmx_en)));

    usb_device(Driver::new(p.USB, Irqs), led).await;
}

/// Build the composite CDC device and run the widget + logger on it.
async fn usb_device(driver: Driver<'static, USB>, mut led: Output<'static>) {
    // Same placeholder VID/PID as the pico2 firmware; the host script looks
    // for this pair first. Allocate a real one (pid.codes) before shipping.
    let mut config = embassy_usb::Config::new(0xc0de, 0xdcaf);
    config.manufacturer = Some("EQUUS");
    config.product = Some("DMX USB Pro compatible (bench)");
    config.serial_number = Some("00000001");
    config.max_packet_size_0 = 64;
    // Composite device with Interface Association Descriptors: required for
    // Windows to bind its CDC driver to each serial function separately.
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut enttec_state = State::new();
    let mut logger_state = State::new();

    let mut builder = embassy_usb::Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [],
        &mut control_buf,
    );
    // Interface order fixes the port order the host sees: widget first.
    let mut enttec_class = CdcAcmClass::new(&mut builder, &mut enttec_state, 64);
    let logger_class = CdcAcmClass::new(&mut builder, &mut logger_state, 64);
    let mut usb = builder.build();

    let logger = embassy_usb_logger::with_class!(1024, log::LevelFilter::Info, logger_class);
    join3(usb.run(), enttec_widget(&mut enttec_class, &mut led), logger).await;
}

/// Send raw bytes chunked into full-speed USB packets, with a ZLP when the
/// total lands on a packet boundary so the host flushes the transfer.
async fn send_bytes(class: &mut UsbClass<'_>, bytes: &[u8]) -> Result<(), EndpointError> {
    for chunk in bytes.chunks(64) {
        class.write_packet(chunk).await?;
    }
    if bytes.len().is_multiple_of(64) {
        class.write_packet(&[]).await?;
    }
    Ok(())
}

/// The widget over CDC: parse host bytes, send replies, print status.
async fn enttec_widget(class: &mut UsbClass<'_>, led: &mut Output<'_>) {
    let mut widget = Widget::new();
    let mut packet = [0_u8; 64];
    let mut reply = [0_u8; REPLY_MAX];

    loop {
        class.wait_connection().await;
        log::info!("Enttec: host configured the device");

        'connected: loop {
            match select(class.read_packet(&mut packet), Timer::after_millis(200)).await {
                Either::First(Ok(n)) => {
                    for &byte in &packet[..n] {
                        match widget.feed(byte, &mut reply) {
                            Some(Action::Frame) => led.toggle(),
                            Some(Action::Reply(len)) => {
                                if send_bytes(class, &reply[..len]).await.is_err() {
                                    break 'connected;
                                }
                            }
                            Some(Action::Ack | Action::Unsupported) | None => {}
                        }
                    }
                }
                Either::First(Err(_)) => break 'connected,
                Either::Second(()) => {}
            }
            widget.report();
        }
        log::info!("Enttec: host disconnected; DMX keeps repeating the last frame");
    }
}
