//! Button interaction
use defmt::Format;
use embassy_time::Timer;

use embassy_stm32::gpio::{Input, OutputOpenDrain};

use crate::channels::RouterChannelTx;
use crate::event_router::RouterEvent;

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
    fn from(row: usize, col: usize) -> Self {
        match (row, col) {
            (0,0) => Self::N1,
            (1,0) => Self::N2,
            (3,0) => Self::N3,
            (4,0) => Self::A,
            (0,1) => Self::N4,
            (1,1) => Self::N5,
            (3,1) => Self::N6,
            (4,1) => Self::B,
            (0,2) => Self::N7,
            (1,2) => Self::N8,
            (3,2) => Self::N9,
            (4,2) => Self::C,
            (0,3) => Self::Star,
            (1,3) => Self::N0,
            (3,3) => Self::Pound,
            (4,3) => Self::D,
            _ => Self::None,
        }
    }
}

#[embassy_executor::task]
pub async fn button_task(
    cols: [Input<'static>; 4],
    mut rows: [OutputOpenDrain<'static>; 4],
    tx: RouterChannelTx
) {
    let mut pressed : [KeyPadButton; 16];
    let mut count : usize;

    loop {
        // set values to default
        pressed = [KeyPadButton::None; 16];
        count = 0;

        // iterate through rows / cols to check if button is pressed
        rows.iter_mut().enumerate().for_each(|(r_index, r)| {
            r.set_low(); // enable this row drain
            cols.iter().enumerate().for_each(|(c_index, c)| {
                if c.is_low() { // check each col for low value (button pressed)
                    pressed[count] = KeyPadButton::from(r_index, c_index);
                    count += 1;
                }
            });
            r.set_high();
        });

        // send pressed buttons to router
        for p_index in 0..count {
            tx.send(RouterEvent::Button(pressed[p_index])).await;
        }

        // wait to check again
        Timer::after_millis(50).await;
    }
}