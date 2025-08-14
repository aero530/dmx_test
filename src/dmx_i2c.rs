//! LED & Button interaction
use defmt::error;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

use crate::channels::DmxChannelTx;
use crate::event_router::DmxEvent;
use crate::I2c1Bus;

#[embassy_executor::task]
pub async fn dmx_task(i2c_bus_manager: &'static I2c1Bus, address: u8, tx: DmxChannelTx) {
    let mut i2c_bus_dev = I2cDevice::new(i2c_bus_manager);

    let mut data_buffer = [0_u8; 513];
    loop {
        match i2c_bus_dev.read(address, &mut data_buffer).await {
            Ok(()) => tx.send(DmxEvent::DmxPacket(data_buffer)).await,
            Err(e) => error!("Error {}",e)
        };
        Timer::after_millis(10).await;
    }
}
