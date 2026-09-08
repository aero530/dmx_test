//! Watchdog and health monitoring.
//!
//! A fixture-class lighting device has to recover from a fault on its own: a
//! panic that halts the core, or a task that wedges, must end in a reset rather
//! than strips frozen until someone finds the power switch.
//!
//! # Two cores, one watchdog
//!
//! The RP2350 watchdog is fed from core 0 only, but a core-1 fault (UI, buttons,
//! DMX, LED render) would otherwise go unnoticed while core 0 happily keeps
//! feeding. So core 1 has to check in: the button task sets [`CORE1_ALIVE`] on
//! every 20 ms poll, and the feeder here only feeds if that flag was set since
//! the last check. Either core stalling therefore ends in a reset within
//! [`WATCHDOG_TIMEOUT`].
//!
//! # Panics
//!
//! `panic-probe` prints the panic over defmt and then halts in a HardFault.
//! Nothing special is needed to turn that into a reset: the halted core stops
//! feeding (or stops checking in), and the watchdog fires. The boot flag in the
//! EEPROM records that the boot before was incomplete, and the reset reason is
//! logged at the next start (`main.rs`).
//!
//! # VSYS monitor
//!
//! The module exposes VSYS/3 on its internal ADC (GP29). Reading it costs
//! nothing and answers two bench questions at once: is the main supply actually
//! present (a board running from a USB port through R35 sits around 4.2 V and
//! must not be asked to drive strips), and did a brown-out just happen.

use core::sync::atomic::{AtomicBool, Ordering};

use defmt::*;
use embassy_rp::adc::{Adc, Blocking, Channel};
use embassy_rp::watchdog::Watchdog;
use embassy_time::{Duration, Timer};

/// Set by core 1 on every button poll; cleared by the feeder when consumed.
pub static CORE1_ALIVE: AtomicBool = AtomicBool::new(false);

/// Reset if neither core has checked in for this long. Long enough to cover a
/// full TFT redraw and an EEPROM page-write burst with margin; short enough
/// that a wedged unit is back within a few seconds.
pub const WATCHDOG_TIMEOUT: Duration = Duration::from_millis(4000);

/// How often core 1's check-in is examined and the watchdog fed.
const FEED_INTERVAL: Duration = Duration::from_millis(1000);

/// VSYS below this means the main supply is absent or sagging (a USB-only
/// bench feed lands around 4.2 V; the 5 V rail through D2 is ~4.6 V).
const VSYS_LOW_MV: u32 = 4400;

const VSYS_POLL: Duration = Duration::from_secs(5);

/// Feed the watchdog while both cores are alive.
///
/// `wdt` must already be started (`main.rs` does that before spawning, so the
/// window between start and first feed is covered by the initial timeout).
#[embassy_executor::task]
pub async fn watchdog_task(mut wdt: Watchdog) -> ! {
    loop {
        Timer::after(FEED_INTERVAL).await;
        if CORE1_ALIVE.swap(false, Ordering::AcqRel) {
            wdt.feed(WATCHDOG_TIMEOUT);
        } else {
            // Not feeding is the action: say why the reset that follows happened.
            warn!("supervisor: core 1 has not checked in - withholding the watchdog feed");
        }
    }
}

/// Log VSYS, and warn once when it drops below [`VSYS_LOW_MV`].
#[embassy_executor::task]
pub async fn vsys_monitor(mut adc: Adc<'static, Blocking>, mut vsys: Channel<'static>) -> ! {
    let mut warned = false;
    loop {
        // 12-bit result against the 3.3 V ADC reference, through the module's
        // 1:3 divider.
        match adc.blocking_read(&mut vsys) {
            Ok(raw) => {
                let mv = raw as u32 * 3300 * 3 / 4096;
                if mv < VSYS_LOW_MV {
                    if !warned {
                        warned = true;
                        warn!(
                            "VSYS {} mV - main supply absent or sagging (USB-only bench feed?); do not expect strips to light",
                            mv
                        );
                    }
                } else {
                    if warned {
                        info!("VSYS {} mV - supply restored", mv);
                    }
                    warned = false;
                    debug!("VSYS {} mV", mv);
                }
            }
            Err(_) => warn!("VSYS: ADC read failed"),
        }
        Timer::after(VSYS_POLL).await;
    }
}
