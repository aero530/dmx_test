//! USB <-> Serial / CDC Interface

use embassy_stm32::peripherals;
use embassy_stm32::usb::Driver;

#[embassy_executor::task(pool_size = 1)]
pub async fn usb_task(driver: Driver<'static, peripherals::USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}