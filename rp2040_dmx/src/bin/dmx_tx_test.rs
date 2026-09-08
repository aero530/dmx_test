//! DMX transmit bench test for the Rev 1 carrier with only the RP2040 Pico
//! fitted (no Nucleo, no I2C master).
//!
//! Continuously transmits a full 512-slot universe (~43 packets/s) treating
//! the channels as consecutive 3-channel RGB fixtures: group 1 = ch 1-3,
//! group 2 = ch 4-6, ... Every group cycles through the full hue wheel once
//! per `HUE_PERIOD_MS`, and the groups are offset from one another so the
//! universe as a whole shows one rainbow that scrolls along the addresses.
//! Peak level is `PEAK_LEVEL` (50 % by default). The onboard LED (GP25)
//! toggles once per hue cycle and a status line goes out over USB serial
//! each second.
//!
//! Pins are the Rev 1 `DMX_RP2040.SchDoc` / `DMX.SchDoc` wiring, identical to
//! the bridge firmware in `main.rs`:
//!
//! | Pico | Net     | Path                                              |
//! |------|---------|---------------------------------------------------|
//! | GP4  | DMX.TX  | R9 330R -> U3 TLP2368 -> U4 THVD1400 DI           |
//! | GP3  | DMX.EN  | R12 330R -> U7 TLP2368 -> U4 DE / RE (high = TX)  |
//! | GP2  | DMX.RX  | left as input; receiver is disabled while DE=high |
//!
//! The Pico's own USB powers the carrier's DMX front end through
//! 5V_RP2040 -> F2 -> JP22 -> V_LED -> U11 -> 3V3 -> PS1 -> 3V3ISO
//! (see README "DMX transmit bench test" for the jumper checklist).

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{PIO0, USB};
use embassy_rp::pio::Pio;
use embassy_rp::usb::Driver;
use embassy_rp::{bind_interrupts, dma, pio, usb};
use embassy_time::{Instant, Timer};
use {defmt_rtt as _, panic_probe as _};

#[allow(dead_code)] // the receiver half is unused here
#[path = "../dmx_pio.rs"]
mod dmx_pio;
use dmx_pio::{DMX_FRAME_SIZE, PioDmxTx, PioDmxTxProgram};

/// Channels per fixture: R, G, B.
const GROUP_SIZE: usize = 3;

/// Number of RGB groups driven, starting at channel 1. 170 fills the whole
/// universe (channels 1..=510); channels 511 and 512 stay at 0.
const GROUP_COUNT: usize = 170;
const _: () = assert!(GROUP_COUNT * GROUP_SIZE <= 512, "groups overflow the universe");

/// Brightness of the fully saturated colour at the top of the wheel: 50 %.
const PEAK_LEVEL: u8 = 128;

/// Time for one group to go once around the hue wheel.
const HUE_PERIOD_MS: u64 = 6_000;

/// Hue wheel resolution: 6 sectors x 256 steps.
const HUE_STEPS: u32 = 6 * 256;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<embassy_rp::peripherals::DMA_CH0>;
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Onboard LED toggles once per hue cycle.
    let mut led = Output::new(p.PIN_25, Level::Low);

    // RS-485 direction. High = U7 LED off = THVD1400 DE/RE high = transmit.
    // Driven high before the transmitter is even configured so the line
    // direction is settled by the time the first BREAK goes out.
    let mut dmx_en = Output::new(p.PIN_3, Level::High);

    // USB serial for the status line (same logger as the bridge firmware).
    spawner.spawn(defmt::unwrap!(logger_task(Driver::new(p.USB, Irqs))));

    // DMX transmitter on GP4.
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);
    let tx_program = PioDmxTxProgram::new(&mut common);
    let mut dmx_tx = PioDmxTx::new(&mut common, sm0, p.DMA_CH0, Irqs, p.PIN_4, &tx_program);

    // Give the USB host a moment to enumerate so the banner is not lost.
    Timer::after_millis(1500).await;
    log::info!(
        "DMX TX test: {GROUP_COUNT} RGB groups (ch1-{}) cycling the hue wheel every {} ms, peak {PEAK_LEVEL} ({}%)",
        GROUP_COUNT * GROUP_SIZE,
        HUE_PERIOD_MS,
        (PEAK_LEVEL as u32 * 100 + 127) / 255,
    );
    log::info!("GP4 = DMX.TX, GP3 = DMX.EN (held high = transmit), GP25 LED toggles once per cycle");

    // Start code 0x00 (dimmer data) + 512 channels, all dark.
    let mut frame = [0u8; DMX_FRAME_SIZE];
    let mut packets: u32 = 0;
    let mut last_report = Instant::now();

    loop {
        // Derive the phase from wall time so the cycle period holds
        // regardless of how long each packet takes to go out.
        let now_ms = Instant::now().as_millis();
        let base_hue = ((now_ms % HUE_PERIOD_MS) as u32 * HUE_STEPS / HUE_PERIOD_MS as u32) as u16;

        for group in 0..GROUP_COUNT {
            // Spread one full rainbow across the groups; every group still
            // visits every hue once per period.
            let offset = (group as u32 * HUE_STEPS / GROUP_COUNT as u32) as u16;
            let rgb = hue_to_rgb(base_hue.wrapping_add(offset));
            let first = 1 + group * GROUP_SIZE;
            frame[first..first + GROUP_SIZE].copy_from_slice(&rgb);
        }

        led.set_level(if base_hue < (HUE_STEPS / 2) as u16 { Level::High } else { Level::Low });
        dmx_en.set_high(); // belt and braces: never let the driver drop out

        // ~23 ms per packet: BREAK + MAB + 513 slots at 250 kbaud.
        dmx_tx.write(&frame).await;
        packets += 1;

        if last_report.elapsed().as_millis() >= 1000 {
            log::info!(
                "{packets} pkt/s | group 1 (ch1-3) = R{} G{} B{}",
                frame[1],
                frame[2],
                frame[3]
            );
            packets = 0;
            last_report = Instant::now();
        }
    }
}

/// Fully saturated colour at `hue` (0..HUE_STEPS around the wheel:
/// red -> yellow -> green -> cyan -> blue -> magenta -> red), scaled so the
/// brightest channel is `PEAK_LEVEL`.
fn hue_to_rgb(hue: u16) -> [u8; GROUP_SIZE] {
    let hue = hue as u32 % HUE_STEPS;
    let ramp = (hue % 256) as u8;
    let (max, falling, rising, off) = (255u8, 255 - ramp, ramp, 0u8);

    let rgb = match hue / 256 {
        0 => [max, rising, off],     // red -> yellow
        1 => [falling, max, off],    // yellow -> green
        2 => [off, max, rising],     // green -> cyan
        3 => [off, falling, max],    // cyan -> blue
        4 => [rising, off, max],     // blue -> magenta
        _ => [max, off, falling],    // magenta -> red
    };
    rgb.map(|v| ((v as u32 * PEAK_LEVEL as u32 + 127) / 255) as u8)
}

/// Route `log` messages to a USB CDC-ACM serial port.
#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}
