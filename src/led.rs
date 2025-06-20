//! LED & Button interaction
use defmt::{info, Format};
use embassy_stm32::{exti::ExtiInput, gpio::Output};
use embassy_time::{with_timeout, Duration, Timer};

use crate::channels::{LedChannelRx, RouterChannelTx};
use crate::event_router::RouterEvent;

#[derive(Format)]
pub enum LedEvent {
    On,
    Off,
    Blink,
}

pub struct Led<'a> {
    pin: Output<'a>,
    rx: LedChannelRx,
}

impl<'a> Led<'a> {
    pub fn new(pin: Output<'a>, rx: LedChannelRx) -> Self {
        Self { pin, rx }
    }

    pub async fn show(&mut self) {
        if let Ok(new_message) = with_timeout(Duration::from_millis(100), self.rx.receive()).await {
            info!("led message {:?}", new_message);
            self.process_event(new_message).await;
        }
    }

    async fn flash(&mut self) {
        for _ in 0..6 {
            self.pin.toggle();
            Timer::after_millis(200).await;
        }
    }

    async fn process_event(&mut self, event: LedEvent) {
        match event {
            LedEvent::On => {
                self.pin.set_high();
            }
            LedEvent::Off => {
                self.pin.set_low();
            }
            LedEvent::Blink => {
                self.flash().await;
            }
        }
    }
}

/// Monitor button interrupt to single press, double press, and hold
#[embassy_executor::task]
pub async fn button_task(mut button: ExtiInput<'static>, tx: RouterChannelTx) {
    const DOUBLE_CLICK_DELAY: u64 = 250;
    const HOLD_DELAY: u64 = 1000;

    button.wait_for_rising_edge().await;
    loop {
        if with_timeout(
            Duration::from_millis(HOLD_DELAY),
            button.wait_for_falling_edge(),
        )
        .await
        .is_err()
        {
            info!("Hold");
            let _ = tx.try_send(RouterEvent::ButtonHold);
            button.wait_for_falling_edge().await;
        } else if with_timeout(
            Duration::from_millis(DOUBLE_CLICK_DELAY),
            button.wait_for_rising_edge(),
        )
        .await
        .is_err()
        {
            info!("Single click");
            let _ = tx.try_send(RouterEvent::ButtonPressed);
        } else {
            info!("Double click");
            let _ = tx.try_send(RouterEvent::ButtonDouble);
            button.wait_for_falling_edge().await;
        }
        button.wait_for_rising_edge().await;
    }
}

#[embassy_executor::task]
pub async fn led_task(pin: Output<'static>, rx: LedChannelRx) {
    let mut led = Led::new(pin, rx);
    loop {
        led.show().await;
    }
}
