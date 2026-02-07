//! DMX receiver
//!
//! Get DMX data over I2C from RPI
// use defmt::error;
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::error;
    } else {
        use defmt::error;
    }
}

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

use crate::artnet::PortAddress;
use crate::channels::{DmxChannelTx, DmxFeedbackChannelRx};
use crate::event_router::{DmxEvent, DmxFeedbackEvent, PacketAddress};
use crate::ui::InputMode;
use crate::I2c1Bus;
use crate::{DMX_BUFFER, DMX_BUFF_SIZE};

/// Pull DMX data from RPI via I2C.
///
/// This only runs if the mode is set to DMX from the main menu.
#[embassy_executor::task]
pub async fn dmx_task(i2c_bus_manager: &'static I2c1Bus, address: u8, tx: DmxChannelTx, mut rx: DmxFeedbackChannelRx) {
    let mut i2c_bus_dev = I2cDevice::new(i2c_bus_manager);

    let mut data_buffer = [0_u8; DMX_BUFF_SIZE];
    let mut input_mode = InputMode::default();

    // DMX data is split between multiple ranges due to limited hardware buffer sizes
    loop {
        // Try to update current mode
        if let Some(input_data) = rx.try_changed() {
            match input_data {
                DmxFeedbackEvent::Mode(new_mode) => {
                    input_mode = new_mode;
                }
            }
        }

        // Only try to get data if we are in DMX mode (vs ArtNet mode)
        if input_mode == InputMode::Dmx {
            match i2c_bus_dev.write_read(address, &[0x01], &mut data_buffer[0..199]).await {
                Ok(()) => {
                    match i2c_bus_dev.write_read(address, &[0x02], &mut data_buffer[200..299]).await {
                        Ok(()) => {
                            match i2c_bus_dev.write_read(address, &[0x03], &mut data_buffer[400..513]).await {
                                Ok(()) => {
                                    let mut dmx_buffer = DMX_BUFFER.lock().await;
                                    dmx_buffer[0..513].copy_from_slice(&data_buffer);
                                    let _ = tx.try_send(DmxEvent::DmxPacket(PacketAddress::new(PortAddress::new(0, 0, 0), 0)));
                                    // match tx.try_send(DmxEvent::DmxPacket((PortAddress::new(0, 0, 0), 0))) {
                                    //     Ok(_) => {}
                                    //     Err(_) => error!("Unable to send DMX Event packet"),
                                    // };
                                }
                                Err(e) => {
                                    error!("DMX Error {:?}", e);
                                    Timer::after_millis(250).await;
                                }
                            };
                        }
                        Err(e) => {
                            error!("DMX Error {:?}", e);
                            Timer::after_millis(250).await;
                        }
                    };
                }
                Err(e) => {
                    error!("DMX Error {:?}", e);
                    Timer::after_millis(250).await;
                }
            };
        }

        Timer::after_millis(30).await;
    }
}
