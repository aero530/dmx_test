//! FT232R emulation bench test: make the Rev 1 board look like a *genuine*
//! Enttec DMX USB Pro to host software.
//!
//! A real Enttec Pro is an FT232R USB-UART bridge with the widget protocol
//! behind it, and lighting software finds it through the FTDI driver stack
//! (ftdibus/D2XX on Windows, `ftdi_sio`/libftdi elsewhere). That stack speaks
//! FTDI's vendor-specific USB protocol, not CDC, so a compatible device has
//! to look like the chip itself:
//!
//! * device `0403:6001`, bcdDevice `0x0600` (that is how drivers tell an
//!   FT232R from the other FTDI parts), strings `ENTTEC` / `DMX USB PRO` /
//!   `EN0000001`;
//! * one vendor-class interface (`FF/FF/FF`) with bulk **EP1 IN** and
//!   **EP2 OUT** — libftdi hardcodes those numbers;
//! * the FTDI vendor control requests (reset, modem control, flow control,
//!   baud rate, line coding, event/error characters, latency timer, bitmode,
//!   read pins, poll modem status, EEPROM read/write/erase);
//! * a 2-byte modem/line-status header at the start of **every** bulk IN
//!   packet, and a status-only packet each latency-timer interval when there
//!   is nothing to send, exactly as the silicon does;
//! * a 128-byte EEPROM image with a valid FTDI checksum so EEPROM reads look
//!   normal. Writes land in RAM only.
//!
//! Behind that, the Enttec widget core is the same as `enttec_test`
//! (`src/enttec_widget.rs`): label 6 frames are retransmitted continuously
//! on the wired DMX port, labels 3 and 10 are answered.
//!
//! There is no room for a second USB interface here — a composite device
//! would no longer match the FTDI driver's `VID_0403&PID_6001` binding — so
//! the log goes out **UART0 TX on GP28 = J19 pin 4** (GND on J19 5/6),
//! 115200 8N1. The onboard LED still toggles on every host frame.
//!
//! Bench experiment only: FTDI's Windows driver polices non-genuine silicon
//! with undocumented EEPROM checks and may inject `NON GENUINE DEVICE FOUND!`
//! into the data stream, and the FTDI VID may not ship on non-FTDI parts.
//! Linux has no such check.

#![no_std]
#![no_main]

use core::fmt::Write as _;

use embassy_executor::Spawner;
use embassy_futures::join::join3;
use embassy_futures::select::{Either, select};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{PIO0, USB};
use embassy_rp::pio::Pio;
use embassy_rp::uart::{self, UartTx};
use embassy_rp::usb::Driver;
use embassy_rp::{bind_interrupts, dma, pio, usb};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pipe::Pipe;
use embassy_time::Timer;
use embassy_usb::Handler;
use embassy_usb::control::{InResponse, OutResponse, Recipient, Request, RequestType};
use embassy_usb::driver::{Direction, EndpointAddress, EndpointIn, EndpointOut};
use portable_atomic::{AtomicU8, Ordering};
use {defmt_rtt as _, panic_probe as _};

#[allow(dead_code)] // the receiver half is unused here
#[path = "../dmx_pio.rs"]
mod dmx_pio;
use dmx_pio::{DMX_FRAME_SIZE, PioDmxTx, PioDmxTxProgram};

/// `enttec_protocol.rs` sizes its payload buffer from `crate::DMX_BUFF_SIZE`.
pub const DMX_BUFF_SIZE: usize = DMX_FRAME_SIZE;

#[allow(dead_code)] // labels only the widget->host direction uses
#[path = "../../../common/src/enttec_protocol.rs"]
mod enttec_protocol;

#[path = "../enttec_widget.rs"]
mod enttec_widget;
use enttec_widget::{Action, REPLY_MAX, Widget, dmx_task};

// ---------------------------------------------------------------- identity

const FTDI_VID: u16 = 0x0403;
const FT232R_PID: u16 = 0x6001;
/// bcdDevice: 0x0600 = FT232R (0x0400 = FT232B, 0x0900 = FT232H, 0x1000 = FT-X).
const FT232R_BCD_DEVICE: u16 = 0x0600;
const MANUFACTURER: &str = "ENTTEC";
const PRODUCT: &str = "DMX USB PRO";
const SERIAL: &str = "EN0000001";

// ---------------------------------------------------------------- FTDI vendor requests

const SIO_RESET: u8 = 0x00;
const SIO_SET_MODEM_CTRL: u8 = 0x01;
const SIO_SET_FLOW_CTRL: u8 = 0x02;
const SIO_SET_BAUD_RATE: u8 = 0x03;
const SIO_SET_DATA: u8 = 0x04;
const SIO_POLL_MODEM_STATUS: u8 = 0x05;
const SIO_SET_EVENT_CHAR: u8 = 0x06;
const SIO_SET_ERROR_CHAR: u8 = 0x07;
const SIO_SET_LATENCY_TIMER: u8 = 0x09;
const SIO_GET_LATENCY_TIMER: u8 = 0x0A;
const SIO_SET_BITMODE: u8 = 0x0B;
const SIO_READ_PINS: u8 = 0x0C;
const SIO_READ_EEPROM: u8 = 0x90;
const SIO_WRITE_EEPROM: u8 = 0x91;
const SIO_ERASE_EEPROM: u8 = 0x92;

/// Modem status byte: bit 0 is a reserved "always 1"; CTS/DSR/RI/DCD (bits
/// 4–7) clear, i.e. no modem lines connected — what an Enttec Pro reports.
const MODEM_STATUS: u8 = 0x01;
/// Line status byte: THRE | TEMT — transmitter empty, no errors.
const LINE_STATUS: u8 = 0x60;

/// Latency timer in ms (FT232R default 16). Set by the host, read by the
/// IN-endpoint task to pace status-only packets.
static LATENCY_MS: AtomicU8 = AtomicU8::new(16);

// ---------------------------------------------------------------- EEPROM image

/// FT232R internal EEPROM: 1024 bits = 128 bytes = 64 words.
const EEPROM_SIZE: usize = 128;

/// Write a USB string descriptor (len, 0x03, UTF-16LE) at `pos`; returns the
/// image and the position after it.
const fn put_string(mut e: [u8; EEPROM_SIZE], pos: usize, s: &str) -> ([u8; EEPROM_SIZE], usize) {
    let bytes = s.as_bytes();
    let len = 2 + 2 * bytes.len();
    e[pos] = len as u8;
    e[pos + 1] = 0x03;
    let mut i = 0;
    while i < bytes.len() {
        e[pos + 2 + 2 * i] = bytes[i];
        e[pos + 3 + 2 * i] = 0;
        i += 1;
    }
    (e, pos + len)
}

/// Default FT232R EEPROM layout (as ftdi_eeprom / FT_Prog lay it out):
/// VID/PID/bcdDevice, config attributes, string pointers (offset | 0x80),
/// CBUS defaults (TXLED, RXLED, TXDEN, PWREN#, SLEEP#), strings from 0x18,
/// FTDI checksum in the last word.
const fn eeprom_image() -> [u8; EEPROM_SIZE] {
    let mut e = [0u8; EEPROM_SIZE];
    e[0x00] = 0x00; // no high-current drive
    e[0x01] = 0x40;
    e[0x02] = (FTDI_VID & 0xFF) as u8;
    e[0x03] = (FTDI_VID >> 8) as u8;
    e[0x04] = (FT232R_PID & 0xFF) as u8;
    e[0x05] = (FT232R_PID >> 8) as u8;
    e[0x06] = (FT232R_BCD_DEVICE & 0xFF) as u8;
    e[0x07] = (FT232R_BCD_DEVICE >> 8) as u8;
    e[0x08] = 0xA0; // bus powered, remote wakeup
    e[0x09] = 0x2D; // 90 mA / 2
    e[0x0A] = 0x08; // use serial number
    e[0x0B] = 0x00; // no signal inversion
    e[0x0C] = 0x00; // USB 2.00
    e[0x0D] = 0x02;

    let start = 0x18;
    let (e, p1) = put_string(e, start, MANUFACTURER);
    let (e, p2) = put_string(e, p1, PRODUCT);
    let (mut e, _end) = put_string(e, p2, SERIAL);
    e[0x0E] = 0x80 | start as u8;
    e[0x0F] = (2 + 2 * MANUFACTURER.len()) as u8;
    e[0x10] = 0x80 | p1 as u8;
    e[0x11] = (2 + 2 * PRODUCT.len()) as u8;
    e[0x12] = 0x80 | p2 as u8;
    e[0x13] = (2 + 2 * SERIAL.len()) as u8;

    e[0x14] = 0x23; // CBUS0 = TXLED#, CBUS1 = RXLED#
    e[0x15] = 0x10; // CBUS2 = TXDEN,  CBUS3 = PWREN#
    e[0x16] = 0x05; // CBUS4 = SLEEP#
    e[0x17] = 0x00;

    // FTDI checksum: XOR each word into 0xAAAA, rotate left 1, store last.
    let mut checksum: u16 = 0xAAAA;
    let mut i = 0;
    while i < EEPROM_SIZE / 2 - 1 {
        let word = e[2 * i] as u16 | ((e[2 * i + 1] as u16) << 8);
        checksum ^= word;
        checksum = checksum.rotate_left(1);
        i += 1;
    }
    e[EEPROM_SIZE - 2] = checksum as u8;
    e[EEPROM_SIZE - 1] = (checksum >> 8) as u8;
    e
}

const EEPROM_DEFAULT: [u8; EEPROM_SIZE] = eeprom_image();
const _: () = assert!(
    0x18 + (2 + 2 * MANUFACTURER.len()) + (2 + 2 * PRODUCT.len()) + (2 + 2 * SERIAL.len()) <= EEPROM_SIZE - 2,
    "EEPROM strings overrun the checksum word"
);

// ---------------------------------------------------------------- control-request handler

struct Ft232r {
    eeprom: [u8; EEPROM_SIZE],
    dtr: bool,
    rts: bool,
    break_on: bool,
}

impl Ft232r {
    const fn new() -> Self {
        Self {
            eeprom: EEPROM_DEFAULT,
            dtr: false,
            rts: false,
            break_on: false,
        }
    }
}

impl Handler for Ft232r {
    fn reset(&mut self) {
        log::info!("USB: bus reset");
    }

    fn configured(&mut self, configured: bool) {
        log::info!("USB: configured = {configured}");
    }

    fn control_out(&mut self, req: Request, _data: &[u8]) -> Option<OutResponse> {
        if req.request_type != RequestType::Vendor || req.recipient != Recipient::Device {
            return None;
        }
        Some(match req.request {
            SIO_RESET => {
                let what = match req.value {
                    0 => "all",
                    1 => "purge rx",
                    2 => "purge tx",
                    _ => "?",
                };
                log::info!("FTDI: reset {what}");
                OutResponse::Accepted
            }
            SIO_SET_MODEM_CTRL => {
                // Low byte = new DTR (bit 0) / RTS (bit 1); high byte = which to apply.
                let mask = (req.value >> 8) as u8;
                let bits = req.value as u8;
                if mask & 0x01 != 0 {
                    self.dtr = bits & 0x01 != 0;
                }
                if mask & 0x02 != 0 {
                    self.rts = bits & 0x02 != 0;
                }
                log::info!("FTDI: DTR={} RTS={}", self.dtr as u8, self.rts as u8);
                OutResponse::Accepted
            }
            SIO_SET_FLOW_CTRL => {
                log::info!("FTDI: flow control {:#04x} (ignored)", req.index >> 8);
                OutResponse::Accepted
            }
            SIO_SET_BAUD_RATE => {
                log::info!("FTDI: baud divisor {:#06x}/{:#06x} (ignored, link is USB-speed)", req.value, req.index);
                OutResponse::Accepted
            }
            SIO_SET_DATA => {
                // Bits 0-7 data bits, 8-10 parity, 11-13 stop bits, 14 = BREAK.
                let brk = req.value & 0x4000 != 0;
                if brk != self.break_on {
                    self.break_on = brk;
                    log::info!("FTDI: break {}", if brk { "on" } else { "off" });
                }
                OutResponse::Accepted
            }
            SIO_SET_LATENCY_TIMER => {
                let ms = (req.value as u8).max(1);
                LATENCY_MS.store(ms, Ordering::Relaxed);
                log::info!("FTDI: latency timer {ms} ms");
                OutResponse::Accepted
            }
            SIO_SET_EVENT_CHAR | SIO_SET_ERROR_CHAR | SIO_SET_BITMODE => OutResponse::Accepted,
            SIO_WRITE_EEPROM => {
                let addr = (req.index as usize) * 2;
                if addr + 1 < EEPROM_SIZE {
                    self.eeprom[addr] = req.value as u8;
                    self.eeprom[addr + 1] = (req.value >> 8) as u8;
                }
                log::warn!("FTDI: EEPROM write word {:#04x} = {:#06x} (RAM only)", req.index, req.value);
                OutResponse::Accepted
            }
            SIO_ERASE_EEPROM => {
                self.eeprom.fill(0xFF);
                log::warn!("FTDI: EEPROM erase (RAM only)");
                OutResponse::Accepted
            }
            other => {
                log::warn!("FTDI: unknown OUT request {other:#04x} value {:#06x} index {:#06x}", req.value, req.index);
                OutResponse::Rejected
            }
        })
    }

    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        if req.request_type != RequestType::Vendor || req.recipient != Recipient::Device {
            return None;
        }
        let n = match req.request {
            SIO_POLL_MODEM_STATUS => {
                buf[0] = MODEM_STATUS;
                buf[1] = LINE_STATUS;
                2
            }
            SIO_GET_LATENCY_TIMER => {
                buf[0] = LATENCY_MS.load(Ordering::Relaxed);
                1
            }
            SIO_READ_PINS => {
                buf[0] = 0x00; // nothing driven on the (non-existent) CBUS pins
                1
            }
            SIO_READ_EEPROM => {
                // Word address in wIndex; beyond the part's EEPROM reads as erased.
                let addr = (req.index as usize) * 2;
                if addr + 1 < EEPROM_SIZE {
                    buf[..2].copy_from_slice(&self.eeprom[addr..addr + 2]);
                } else {
                    buf[..2].fill(0xFF);
                }
                2
            }
            other => {
                log::warn!("FTDI: unknown IN request {other:#04x} value {:#06x} index {:#06x}", req.value, req.index);
                return Some(InResponse::Rejected);
            }
        };
        Some(InResponse::Accepted(&buf[..n.min(req.length as usize)]))
    }
}

// ---------------------------------------------------------------- data path

/// Widget replies waiting for the host to poll the IN endpoint.
static TO_HOST: Pipe<CriticalSectionRawMutex, 1024> = Pipe::new();

/// Bulk OUT: raw serial bytes from the host, straight into the widget parser.
async fn host_to_widget(mut ep: impl EndpointOut, mut led: Output<'_>) {
    let mut widget = Widget::new();
    let mut packet = [0_u8; 64];
    let mut reply = [0_u8; REPLY_MAX];

    loop {
        ep.wait_enabled().await;
        log::info!("FTDI: host configured the device");

        'connected: loop {
            match select(ep.read(&mut packet), Timer::after_millis(200)).await {
                Either::First(Ok(n)) => {
                    for &byte in &packet[..n] {
                        match widget.feed(byte, &mut reply) {
                            Some(Action::Frame) => led.toggle(),
                            Some(Action::Reply(len)) => match TO_HOST.try_write(&reply[..len]) {
                                Ok(written) if written == len => {}
                                Ok(written) => log::warn!("FTDI: dropped {} reply bytes (host not reading)", len - written),
                                Err(_) => log::warn!("FTDI: dropped a {len}-byte reply (host not reading)"),
                            },
                            Some(Action::Ack | Action::Unsupported) | None => {}
                        }
                    }
                }
                Either::First(Err(_)) => break 'connected,
                Either::Second(()) => {}
            }
            widget.report();
        }
        log::info!("FTDI: host disconnected; DMX keeps repeating the last frame");
    }
}

/// Bulk IN: every packet carries the 2-byte status header, then up to 62
/// data bytes. With nothing queued, a status-only packet goes out each
/// latency-timer interval — the driver relies on that cadence.
async fn widget_to_host(mut ep: impl EndpointIn) {
    let mut packet = [0_u8; 64];

    loop {
        ep.wait_enabled().await;
        loop {
            packet[0] = MODEM_STATUS;
            packet[1] = LINE_STATUS;
            let latency = LATENCY_MS.load(Ordering::Relaxed) as u64;
            let n = match select(TO_HOST.read(&mut packet[2..]), Timer::after_millis(latency)).await {
                Either::First(n) => n,
                Either::Second(()) => 0,
            };
            if ep.write(&packet[..2 + n]).await.is_err() {
                break;
            }
        }
    }
}

// ---------------------------------------------------------------- UART log

/// Formatted log lines waiting for the UART; `try_write` so logging can
/// never stall the USB or DMX tasks (overflow just drops text).
static LOG_PIPE: Pipe<CriticalSectionRawMutex, 2048> = Pipe::new();

struct LineBuf {
    buf: [u8; 160],
    len: usize,
}

impl core::fmt::Write for LineBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let n = s.len().min(self.buf.len() - self.len);
        self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        Ok(())
    }
}

struct PipeLogger;

impl log::Log for PipeLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let mut line = LineBuf { buf: [0; 160], len: 0 };
        let _ = write!(line, "[{}] {}", record.level(), record.args());
        let end = line.len.min(line.buf.len() - 2);
        line.buf[end..end + 2].copy_from_slice(b"\r\n");
        let _ = LOG_PIPE.try_write(&line.buf[..end + 2]);
    }

    fn flush(&self) {}
}

static LOGGER: PipeLogger = PipeLogger;

#[embassy_executor::task]
async fn uart_log_task(mut tx: UartTx<'static, uart::Async>) {
    let mut buf = [0_u8; 64];
    loop {
        let n = LOG_PIPE.read(&mut buf).await;
        let _ = tx.write(&buf[..n]).await;
    }
}

// ---------------------------------------------------------------- main

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<embassy_rp::peripherals::DMA_CH0>,
                 dma::InterruptHandler<embassy_rp::peripherals::DMA_CH1>;
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Log UART: GP28 = UART0 TX = J19 pin 4.
    let uart_tx = UartTx::new(p.UART0, p.PIN_28, p.DMA_CH1, Irqs, uart::Config::default());
    // SAFETY: called once, before any task logs; the racy variants are the
    // only ones available without atomic CAS on thumbv6m.
    unsafe {
        let _ = log::set_logger_racy(&LOGGER);
        log::set_max_level_racy(log::LevelFilter::Info);
    }
    spawner.spawn(defmt::unwrap!(uart_log_task(uart_tx)));
    log::info!("FT232R / Enttec DMX USB Pro emulation starting");

    // Onboard LED toggles on every frame received from the host.
    let led = Output::new(p.PIN_25, Level::Low);

    // RS-485 direction. High = U7 LED off = THVD1400 DE/RE high = transmit.
    let dmx_en = Output::new(p.PIN_3, Level::High);

    // DMX transmitter on GP4, in its own task.
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let tx_program = PioDmxTxProgram::new(&mut common);
    let dmx_tx = PioDmxTx::new(&mut common, sm0, p.DMA_CH0, Irqs, p.PIN_4, &tx_program);
    spawner.spawn(defmt::unwrap!(dmx_task(dmx_tx, dmx_en)));

    usb_device(Driver::new(p.USB, Irqs), led).await;
}

/// Build the FT232R look-alike and run its three halves.
async fn usb_device(driver: Driver<'static, USB>, led: Output<'static>) {
    let mut config = embassy_usb::Config::new(FTDI_VID, FT232R_PID);
    config.device_release = FT232R_BCD_DEVICE;
    config.manufacturer = Some(MANUFACTURER);
    config.product = Some(PRODUCT);
    config.serial_number = Some(SERIAL);
    // The real FT232R advertises an 8-byte EP0, but embassy-rp's control pipe
    // hardcodes 64 (usb.rs `ControlPipe::max_packet_size`) regardless of this
    // setting. Advertising 8 while sending 64-byte packets makes Windows fail
    // enumeration with "Device Descriptor Request Failed"; drivers don't care
    // about EP0 size, so 64 it is.
    config.max_packet_size_0 = 64;
    config.max_power = 90;
    config.supports_remote_wakeup = true;
    // device_class/subclass/protocol stay 0 and there are no IADs: a plain
    // single-function device, exactly like the real chip.

    let mut config_descriptor = [0; 64];
    let mut bos_descriptor = [0; 32];
    let mut control_buf = [0; 64];
    let mut ft232r = Ft232r::new();

    let mut builder = embassy_usb::Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [],
        &mut control_buf,
    );
    builder.handler(&mut ft232r);

    let (ep_in, ep_out) = {
        let mut func = builder.function(0xFF, 0xFF, 0xFF);
        let mut iface = func.interface();
        let mut alt = iface.alt_setting(0xFF, 0xFF, 0xFF, None);
        // Descriptor order and numbers as on the FT232R: 0x81 IN, then 0x02 OUT.
        let ep_in = alt.endpoint_bulk_in(Some(EndpointAddress::from_parts(1, Direction::In)), 64);
        let ep_out = alt.endpoint_bulk_out(Some(EndpointAddress::from_parts(2, Direction::Out)), 64);
        (ep_in, ep_out)
    };
    let mut usb = builder.build();

    join3(usb.run(), host_to_widget(ep_out, led), widget_to_host(ep_in)).await;
}
