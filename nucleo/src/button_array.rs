//! Button interaction
use defmt::{error, Format};
use embassy_time::Timer;

use embassy_stm32::gpio::{Input, OutputOpenDrain};

use crate::channels::RouterChannelTx;
use crate::event_router::RouterEvent;

#[derive(Copy, Clone, Format, PartialEq)]
pub enum KeyPadEvent {
    None,
    Pressed,
    Released,
    Held,
}

// Format
// 1 2 3 A
// 4 5 6 B
// 7 8 9 C
// * 0 # D
#[derive(Copy, Clone, Format)]
pub enum KeyPadButton {
    A,
    B,
    C,
    D,
    N0,
    N1,
    N2,
    N3,
    N4,
    N5,
    N6,
    N7,
    N8,
    N9,
    Star,
    Pound,
    None,
}

impl KeyPadButton {
//     fn from(row: usize, col: usize) -> Self {
//         match (row, col) {
// // 1 2 3 A
// // 4 5 6 B
// // 7 8 9 C
// // * 0 # D
//             (0,0) => Self::N1,
//             (1,0) => Self::N4,
//             (3,0) => Self::N7,
//             (4,0) => Self::Star,
//             (0,1) => Self::N2,
//             (1,1) => Self::N5,
//             (3,1) => Self::N8,
//             (4,1) => Self::N0,
//             (0,2) => Self::N3,
//             (1,2) => Self::N6,
//             (3,2) => Self::N9,
//             (4,2) => Self::Pound,
//             (0,3) => Self::A,
//             (1,3) => Self::B,
//             (3,3) => Self::C,
//             (4,3) => Self::D,
//             _ => Self::None,
//         }
//     }
    fn at(location: usize) -> Self {
        match location {
            0 => Self::N1,
            1 => Self::N2,
            2 => Self::N3,
            3 => Self::A,
            4 => Self::N4,
            5 => Self::N5,
            6 => Self::N6,
            7 => Self::B,
            8 => Self::N7,
            9 => Self::N8,
            10 => Self::N9,
            11 => Self::C,
            12 => Self::Star,
            13 => Self::N0,
            14 => Self::Pound,
            15 => Self::D,
            _ => Self::None,
        }
    }
}

#[embassy_executor::task]
pub async fn button_array_task(
    cols: [Input<'static>; 4],
    mut rows: [OutputOpenDrain<'static>; 4],
    tx: RouterChannelTx
) {
    // let mut pressed : [KeyPadButton; 16];
    let mut pressed : [bool; 16];
    let mut found : bool;
    let mut location : usize;
    let mut events : [KeyPadEvent; 16];
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
                if c.is_low() && found == false { // check each col for low value (button pressed)
                    pressed[location] = true;
                    found = true;
                    match events[location] {
                        KeyPadEvent::None => events[location] = KeyPadEvent::Pressed,
                        KeyPadEvent::Pressed => events[location] = KeyPadEvent::Held,
                        KeyPadEvent::Released => events[location] = KeyPadEvent::Pressed,
                        KeyPadEvent::Held => events[location] = KeyPadEvent::Held,
                    }
                } else { // button not pressed
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
                    Ok(_) => {},
                    Err(e) => error!("Message dropped. Channel full. {}",e)
                };

            }
        }

        // wait to check again
        Timer::after_millis(125).await;
    }
}