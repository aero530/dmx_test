//! On board LED for diagnostics

use embassy_stm32::gpio::Output;
use embassy_time::Timer;

/// Toggle on board LED to indicate application is running
#[embassy_executor::task]
pub async fn led_task(mut pin: Output<'static>) {
    loop {
        Timer::after_millis(250).await;
        pin.set_high();

        Timer::after_millis(250).await;
        pin.set_low();
    }
}
