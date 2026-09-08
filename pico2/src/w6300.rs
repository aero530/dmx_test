//! W6300 Ethernet transport.
//!
//! The W6300 is a hardwired TCP/IP controller, but it is driven here in
//! **MACRAW** mode — raw Ethernet frames in and out — so smoltcp sits on top and
//! `embassy-net` presents the same API the STM32 build had against its built-in
//! MAC. That is what lets the Art-Net task in `common` port unchanged.
//!
//! # Why the link is PIO and not hardware SPI
//!
//! On the W6300-EVB-Pico2 the W6300's `SCLK` lands on **GP17**, which is a
//! `CSn` mux position on the RP2350, not `SCK`. Hardware SPI0 physically cannot
//! reach it. So the transport is a PIO SPI on PIO0 SM0 — mandatory even in
//! single-SPI mode, which is why the PIO budget reserves that state machine.
//!
//! # Single SPI now, QSPI later
//!
//! `embassy-net-wiznet` 0.3 drives the W6300 in single SPI. QSPI is embassy
//! PR #5809, still a draft. When it lands, only [`SpiBus`] below changes — the
//! `embassy-net` device above it, and everything above that, stays as it is.
//!
//! # Diagnosing a dead chip
//!
//! [`InitError`] distinguishes the two cases that used to look identical on the
//! Nucleo, where a dead PHY could only be found with a scope:
//!
//! * `SpiError` — the chip is not answering on the bus at all.
//! * `InvalidChipVersion` — it answers, but the version register is wrong.
//!
//! Both are logged distinctly. Since the Ethernet failure that prompted this
//! redesign was never diagnosed and now never can be, this is the board's own
//! account of what went wrong. See `docs/ARCHITECTURE.md` §10.

use defmt::*;
use embassy_net_wiznet::chip::W6300;
use embassy_net_wiznet::{Device, InitError, Runner, State};
use embassy_rp::gpio::{Input, Output};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio_programs::spi::Spi as PioSpi;
use embassy_rp::spi::Async;
use embassy_time::Delay;
use embedded_hal_bus::spi::ExclusiveDevice;

/// PIO SPI bus to the W6300 — PIO0 state machine 0.
pub type SpiBus = PioSpi<'static, PIO0, 0, Async>;

/// `embedded-hal-async` SPI device: the bus plus a software chip select on
/// GP16. The W6300 is alone on this bus, so exclusive access is exactly right.
pub type SpiDev = ExclusiveDevice<SpiBus, Output<'static>, Delay>;

/// Driver task handle. `INT` is GP15, `RST` is GP22.
pub type W6300Runner = Runner<'static, W6300, SpiDev, Input<'static>, Output<'static>>;

/// SPI clock for the transport.
///
/// **A Phase 0 tuning parameter.** The byte budget the firmware enforces comes
/// out of measuring achievable Art-Net throughput on real hardware, and this is
/// the main lever on it. Start conservative; raise it while watching for
/// CRC/framing errors on the link.
pub const SPI_FREQ_HZ: u32 = 20_000_000;

/// Receive queue depth, in MACRAW frames.
///
/// Sized for the burst shape rather than the average: a console pushes all its
/// universes back-to-back at the top of each refresh, so ~32 frames can arrive
/// in a clump at 44 Hz. The W6300's own 32 KB RX buffer absorbs most of that;
/// this queue only has to keep the pipeline from stalling.
pub const N_RX: usize = 8;
/// Transmit queue depth. Only ArtPollReply and DHCP go out, so this is small.
pub const N_TX: usize = 4;

/// Shared state for the driver. Must outlive the stack, so it is a `StaticCell`
/// in `main`.
pub type W6300State = State<N_RX, N_TX>;

/// Bring the chip up and return the `embassy-net` device plus its runner.
///
/// Logs the failure mode rather than just returning it, because *which* way
/// this fails is the diagnostic signal.
pub async fn init(
    mac_addr: [u8; 6],
    state: &'static mut W6300State,
    spi_dev: SpiDev,
    int: Input<'static>,
    reset: Output<'static>,
) -> Option<(Device<'static>, W6300Runner)> {
    match embassy_net_wiznet::new(mac_addr, state, spi_dev, int, reset).await {
        Ok((device, runner)) => {
            info!("W6300: up, MAC {:02x}", mac_addr);
            Some((device, runner))
        }
        Err(InitError::SpiError(_)) => {
            // Nothing answered on the bus. Chip dead, unpowered, or the PIO SPI
            // is misconfigured. Distinct from "link down" on purpose.
            error!("W6300: ETH_CHIP_NOT_RESPONDING - no reply over SPI");
            None
        }
        Err(InitError::InvalidChipVersion { expected, actual }) => {
            // It answered, so the bus works, but it is not a healthy W6300.
            error!(
                "W6300: ETH_CHIP_BAD_VERSION - expected {=u8:#04x}, got {=u8:#04x}",
                expected, actual
            );
            None
        }
    }
}

/// Pump the W6300 driver. Must run for the stack to move any traffic.
#[embassy_executor::task]
pub async fn w6300_task(runner: W6300Runner) -> ! {
    runner.run().await
}

/// Pump the `embassy-net` stack.
#[embassy_executor::task]
pub async fn net_task(mut runner: embassy_net::Runner<'static, Device<'static>>) -> ! {
    runner.run().await
}
