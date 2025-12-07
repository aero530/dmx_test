//! LED blink

use embassy_stm32::gpio::Output;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn led_task(mut pin: Output<'static>) {
    loop {
        Timer::after_millis(250).await;
        pin.set_high();

        Timer::after_millis(250).await;
        pin.set_low();
    }
}
