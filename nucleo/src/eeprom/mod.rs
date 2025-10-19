
use defmt::Format;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::{with_timeout, Duration, Delay};
use embassy_futures::block_on;

use embedded_hal_1::delay::DelayNs;

use bincode::{config, Decode, Encode};

use crate::{channels::RouterChannelTx, event_router::RouterEvent, ui::{MenuData, ModuleType}, EepromChannelRx, I2c1Bus};

use defmt::{info, error};

mod m24x02;
use m24x02::{M24x02, PAGE_SIZE};

const MODULE_TYPE_ADDRESS: u8 = 0x05; // must be a low enough address such that module type data does not interfere with settings at SETTINGS_ADDRESS
const SETTINGS_ADDRESS: u8 = 0x20; // 0x20 = 32 which is the start of the third page of memory
const SETTINGS_SIZE: usize = 16; // number of bytes to reserve for menu settings

#[derive(Clone, Copy, Format, Debug)]
pub enum EepromEvent {
    ReadModuleType,
    StoreModuleType(ModuleType),
    ReadSettings,
    StoreSettings(MenuData),
}

pub struct Eeprom<I2C: embedded_hal_async::i2c::I2c> {
    dev: M24x02<I2C>,
    rx: EepromChannelRx,
    tx: RouterChannelTx,
}

impl<I2C: embedded_hal_async::i2c::I2c> Eeprom<I2C> {
    pub fn new(i2c: I2C, address: u8, rx: EepromChannelRx, tx: RouterChannelTx) -> Self {
        let dev = M24x02::new(i2c, address);
        Self { dev, rx, tx }
    }

    pub async fn show(&mut self) {
        if let Ok(new_message) = with_timeout(Duration::from_millis(200), self.rx.receive()).await {
            self.process_event(new_message).await;
        }
    }

    async fn process_event(&mut self, event: EepromEvent) {
        match event {
            EepromEvent::ReadModuleType => {
                let _ = self.dev.read_byte(MODULE_TYPE_ADDRESS).await;

                    match self.dev.read_byte(MODULE_TYPE_ADDRESS).await {
                        Ok(data) => {
                            let decoded: ModuleType = bincode::decode_from_slice(&[data], bincode::config::standard()).unwrap_or_default().0;
                            let _ = self.tx.try_send(RouterEvent::UpdateModuleType(decoded));
                        },
                        Err(e) => {
                            error!("Eeprom - error {}", e);
                        },
                    }
                },
            EepromEvent::StoreModuleType(module) => {
                let mut slice = [0u8; 1];

                let length = bincode::encode_into_slice(module, &mut slice, bincode::config::standard()).unwrap_or_else(|e| {
                    match e {
                        bincode::error::EncodeError::UnexpectedEnd => error!("Error encoding menu settings - UnexpectedEnd"),
                        _ => error!("Error encoding menu settings"),
                    }
                    0
                });

                if length > 0 {
                    let _ = self.dev.write_byte_wait(MODULE_TYPE_ADDRESS, slice[0]).await; // throw away write due to shared bus issues
                    match self.dev.write_byte_wait(MODULE_TYPE_ADDRESS, slice[0]).await {
                        Ok(_) => info!("Wrote module type as {}", module),
                        Err(e) =>error!("Eeprom - error {}", e),
                    }
                }
                },
            EepromEvent::ReadSettings => {
                    let mut buf = [0u8; SETTINGS_SIZE];
                    let _ = self.dev.read_byte(SETTINGS_ADDRESS).await; // throw away read due to shared bus issues

                    match self.dev.read_data(SETTINGS_ADDRESS, &mut buf).await {
                        Ok(_) => {
                            let decoded: MenuData = bincode::decode_from_slice(&buf, bincode::config::standard()).unwrap_or_default().0;
                            let _ = self.tx.try_send(RouterEvent::UpdateSettings(decoded));
                        },
                        Err(e) => {
                            error!("Eeprom - error {}", e);
                        },
                    }
                },
            EepromEvent::StoreSettings(menu_settings) => {
                let mut slice = [0u8; SETTINGS_SIZE];
                
                
                let length = bincode::encode_into_slice(menu_settings, &mut slice, bincode::config::standard()).unwrap_or_else(|e| {
                    match e {
                        bincode::error::EncodeError::UnexpectedEnd => error!("Error encoding menu settings - UnexpectedEnd"),
                        _ => error!("Error encoding menu settings"),
                    }
                    0
                });

                if length > 0 {

                    info!("Storing menu settings {}. Data is {} bytes long", menu_settings, length);

                    let chunks = slice.chunks(PAGE_SIZE as usize).enumerate();
                    let mem_addr = SETTINGS_ADDRESS + (0 as u8)*PAGE_SIZE;
                    
                    // There is something going on that makes it so page write only works if you write a byte to the device first then
                    // do the page write...something with the address not being transmitted. Not sure if this is something the eeprom
                    // is doing or something in the microcontroller or code. 
                    //
                    // This problem appears to have something to do with using the shared bus...like the bus isn't properly cleared on other
                    // read / write attempts. 
                    let _ = self.dev.write_byte_wait(SETTINGS_ADDRESS, slice[0]).await;
                    
                    for (num, chunk) in chunks {
                        let mem_addr = SETTINGS_ADDRESS + (num as u8)*PAGE_SIZE;
                        // self.dev.write_page_wait(mem_addr, chunk).await;
                        match self.dev.write_page_wait(mem_addr, chunk).await {
                            Ok(_) => info!("Data chunk {}/{:#X} stored -- {:#X}", num, mem_addr, chunk),
                            Err(e) => error!("Data chunk {}/{:#X} error {:#X} -- {:#X}", num, mem_addr, e, chunk),
                        }
                    }
                }
            },
        }
    }
}

#[embassy_executor::task]
pub async fn eeprom_i2c_task(i2c_bus_manager: &'static I2c1Bus, address: u8, rx: EepromChannelRx, tx: RouterChannelTx) {
    let i2c_bus_dev = I2cDevice::new(i2c_bus_manager);
    let mut eeprom = Eeprom::new(i2c_bus_dev, address, rx, tx);
    loop {
        eeprom.show().await;
    }
}
