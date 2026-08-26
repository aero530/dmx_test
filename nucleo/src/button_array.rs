//! Button row and array interaction

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{debug, error};
    } else {
        use defmt::{debug, error};
    }
}

use embassy_time::Timer;

use embassy_stm32::gpio::{Input, OutputOpenDrain};

use crate::channels::RouterChannelTx;
use crate::event_router::RouterEvent;

pub use common::events::{KeyPadButton, KeyPadEvent};

#[embassy_executor::task]
pub async fn button_array_task(cols: [Input<'static>; 4], mut rows: [OutputOpenDrain<'static>; 4], tx: RouterChannelTx) {
    // let mut pressed : [KeyPadButton; 16];
    let mut pressed: [bool; 16];
    let mut found: bool;
    let mut location: usize;
    let mut events: [KeyPadEvent; 16];
    events = [KeyPadEvent::None; 16];

    loop {
        // set values to default
        // pressed = [KeyPadButton::None; 16];
        pressed = [false; 16];
        found = false;
        location = 0;

        // iterate through rows / cols to check if button is pressed
        rows.iter_mut().for_each(|r| {
            r.set_low(); // enable this row drain
            cols.iter().for_each(|c| {
                if c.is_low() && !found {
                    // check each col for low value (button pressed)
                    pressed[location] = true;
                    found = true;
                    match events[location] {
                        KeyPadEvent::None => events[location] = KeyPadEvent::Pressed,
                        KeyPadEvent::Pressed => events[location] = KeyPadEvent::Held,
                        KeyPadEvent::Released => events[location] = KeyPadEvent::Pressed,
                        KeyPadEvent::Held => events[location] = KeyPadEvent::Held,
                    }
                } else {
                    // button not pressed
                    match events[location] {
                        KeyPadEvent::None => events[location] = KeyPadEvent::None,
                        KeyPadEvent::Pressed => events[location] = KeyPadEvent::Released,
                        KeyPadEvent::Released => events[location] = KeyPadEvent::None,
                        KeyPadEvent::Held => events[location] = KeyPadEvent::Released,
                    }
                    pressed[location] = false;
                }
                location += 1;
            });
            r.set_high();
        });

        // send pressed buttons to router
        for (l, e) in events.iter().enumerate() {
            if *e == KeyPadEvent::Released {
                match tx.try_send(RouterEvent::ButtonArray((KeyPadButton::at(l), KeyPadEvent::Released))) {
                    Ok(_) => {}
                    Err(e) => error!("Message dropped. Channel full. {:?}", e),
                };
            }
        }

        // wait to check again
        Timer::after_millis(125).await;
    }
}

#[embassy_executor::task]
pub async fn button_row_task(row: [Input<'static>; 4], tx: RouterChannelTx) {
    // let mut pressed : [KeyPadButton; 16];
    let mut pressed: [bool; 4];
    let mut events: [KeyPadEvent; 16];
    events = [KeyPadEvent::None; 16];

    let map = [15, 14, 13, 12];

    loop {
        // set values to default
        // pressed = [KeyPadButton::None; 16];
        pressed = [false; 4];
        let mut location = 0;

        // iterate through rows / cols to check if button is pressed
        row.iter().for_each(|r| {
            if r.is_low() {
                // check each col for low value (button pressed)
                pressed[location] = true;
                match events[map[location]] {
                    KeyPadEvent::None => events[map[location]] = KeyPadEvent::Pressed,
                    KeyPadEvent::Pressed => events[map[location]] = KeyPadEvent::Held,
                    KeyPadEvent::Released => events[map[location]] = KeyPadEvent::Pressed,
                    KeyPadEvent::Held => events[map[location]] = KeyPadEvent::Held,
                }
                debug!("{} is pressed", location);
            } else {
                // button not pressed
                match events[map[location]] {
                    KeyPadEvent::None => events[map[location]] = KeyPadEvent::None,
                    KeyPadEvent::Pressed => events[map[location]] = KeyPadEvent::Released,
                    KeyPadEvent::Released => events[map[location]] = KeyPadEvent::None,
                    KeyPadEvent::Held => events[map[location]] = KeyPadEvent::Released,
                }
                pressed[location] = false;
            }
            location += 1;
        });

        // send pressed buttons to router
        for (l, e) in events.iter().enumerate() {
            if *e == KeyPadEvent::Released {
                match tx.try_send(RouterEvent::ButtonArray((KeyPadButton::at(l), KeyPadEvent::Released))) {
                    Ok(_) => {}
                    Err(e) => error!("Message dropped. Channel full. {:?}", e),
                };
            }
        }

        // wait to check again
        Timer::after_millis(125).await;
    }
}
