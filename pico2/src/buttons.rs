//! Front-panel buttons, read through the TCA9555.
//!
//! Interrupt-driven rather than polled: the task blocks on the expander's `INT`
//! line and only touches the bus when something actually changed. The wait
//! still carries a timeout, because a *held* button produces no further edges —
//! the periodic wake is what advances Pressed → Held and catches the release if
//! an edge is ever missed.
//!
//! The press/release state machine and the emit-on-release behaviour are
//! deliberately identical to the STM32 build's `button_row_task`, so the router
//! and UI see exactly the events they saw before.

use common::channels::RouterChannelTx;
use common::event_router::RouterEvent;
use common::events::{KeyPadButton, KeyPadEvent};
use defmt::*;
use embassy_rp::gpio::Input;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal_async::i2c::I2c;

use crate::tca9555::{Tca9555, BTN_DOWN, BTN_ESC, BTN_SELECT, BTN_UP};

/// Wake at least this often even with no edge, so Held and Released still
/// progress. Matches the STM32 build's poll interval.
const TICK: Duration = Duration::from_millis(125);

/// Settle time between an `INT` edge and the input read. The STM32 build
/// polled at 125 ms and was debounced by construction; `INT` fires on the
/// FIRST bounce edge, and reading mid-bounce can register a phantom
/// press-and-release. Tactile switches bounce for 1–10 ms.
const DEBOUNCE: Duration = Duration::from_millis(8);

/// Port 0 bit → logical button.
///
/// The names are the ones the event router already switches on:
/// `D` → Down, `Pound` → Up, `N0` → Select, `Star` → Esc. Keeping them means
/// the router's mapping needs no change.
const BUTTONS: [(u8, KeyPadButton); 4] = [
    (BTN_DOWN, KeyPadButton::D),
    (BTN_UP, KeyPadButton::Pound),
    (BTN_SELECT, KeyPadButton::N0),
    (BTN_ESC, KeyPadButton::Star),
];

fn next(state: KeyPadEvent, pressed: bool) -> KeyPadEvent {
    match (state, pressed) {
        (KeyPadEvent::None, true) => KeyPadEvent::Pressed,
        (KeyPadEvent::Pressed, true) => KeyPadEvent::Held,
        (KeyPadEvent::Released, true) => KeyPadEvent::Pressed,
        (KeyPadEvent::Held, true) => KeyPadEvent::Held,
        (KeyPadEvent::Pressed, false) => KeyPadEvent::Released,
        (KeyPadEvent::Held, false) => KeyPadEvent::Released,
        (_, false) => KeyPadEvent::None,
    }
}

/// Drive the expander: configure it, release the OLED reset (signalling
/// `oled_ready` so the UI task can safely init the display), then serve
/// buttons.
pub async fn run<I2C: I2c>(
    mut expander: Tca9555<I2C>,
    mut int: Input<'static>,
    tx: RouterChannelTx,
    oled_ready: &'static Signal<CriticalSectionRawMutex, ()>,
) -> ! {
    if expander.init().await.is_err() {
        // Not fatal on its own, but the panel is dead and the OLED will never
        // come out of reset, so say which of the two I²C parts failed.
        error!("TCA9555: init failed - no panel, no display");
    }
    if expander.release_oled_reset().await.is_err() {
        error!("TCA9555: could not release OLED reset");
    }
    // Signalled even on failure so the UI task logs its init result instead of
    // waiting forever; if RES really is stuck low the expander error above is
    // the diagnostic.
    oled_ready.signal(());

    let mut state = [KeyPadEvent::None; BUTTONS.len()];

    loop {
        // Either something changed, or the tick expired and a held button needs
        // its state advanced. On a real edge, wait out contact bounce before
        // reading — INT asserts on the first bounce, and a mid-bounce read can
        // register a phantom release.
        if with_timeout(TICK, int.wait_for_falling_edge()).await.is_ok() {
            Timer::after(DEBOUNCE).await;
        }

        // Reads both ports in one transfer, which is what actually clears INT.
        let ports = match expander.inputs().await {
            Ok(p) => p,
            Err(_) => {
                error!("TCA9555: read failed");
                continue;
            }
        };

        for (i, (mask, button)) in BUTTONS.iter().enumerate() {
            // Active low: a pressed button pulls its input to ground.
            let pressed = ports[0] & mask == 0;
            state[i] = next(state[i], pressed);

            if state[i] == KeyPadEvent::Released {
                if let Err(e) = tx.try_send(RouterEvent::ButtonArray((*button, KeyPadEvent::Released)))
                {
                    error!("button dropped, router channel full: {:?}", e);
                }
            }
        }
    }
}
