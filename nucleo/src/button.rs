//! Button interaction

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::error;
    } else {
        use defmt::error;
    }
}
use embassy_stm32::gpio::Input;
use embassy_time::Timer;

use crate::channels::RouterChannelTx;
use crate::event_router::RouterEvent;

pub use common::events::ButtonEvent;

/// Task to monitor a single button
#[embassy_executor::task]
pub async fn button_task(button: Input<'static>, tx: RouterChannelTx) {
    let mut event: ButtonEvent;
    event = ButtonEvent::None;

    loop {
        if button.is_low() {
            // button is pressed
            match event {
                ButtonEvent::None => event = ButtonEvent::Pressed,
                ButtonEvent::Pressed => event = ButtonEvent::Held,
                ButtonEvent::Released => event = ButtonEvent::Pressed,
                ButtonEvent::Held => event = ButtonEvent::Held,
            }
        } else {
            // button not pressed
            match event {
                ButtonEvent::None => event = ButtonEvent::None,
                ButtonEvent::Pressed => event = ButtonEvent::Released,
                ButtonEvent::Released => event = ButtonEvent::None,
                ButtonEvent::Held => event = ButtonEvent::Released,
            }
        }

        // send pressed buttons to router
        if event == ButtonEvent::Released {
            match tx.try_send(RouterEvent::Button(ButtonEvent::Released)) {
                Ok(_) => {}
                Err(e) => error!("Message dropped. Channel full. {:?}", e),
            };
        }

        // wait to check again
        Timer::after_millis(125).await;
    }
}
