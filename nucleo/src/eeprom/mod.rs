// use cfg_if::cfg_if;

// use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_time::{with_timeout, Duration};
use embedded_hal_1::i2c::I2c as I2CTRAIT;

// use embassy_futures::block_on;
// use embassy_sync::blocking_mutex::raw::NoopRawMutex;
// use core::cell::RefCell;
// use embassy_stm32::i2c::{I2c, Master};

use crate::{
    channels::RouterChannelTx,
    event_router::RouterEvent,
    ui::{MenuData, ModuleType},
    EepromChannelRx, I2cSharedDev,
};

use defmt::{error, info, Format};

mod m24x02;
use m24x02::{M24x02, PAGE_SIZE};

const MODULE_TYPE_MEMLOC: u8 = 0x01; // must be a low enough address such that module type data does not interfere with settings at SETTINGS_ADDRESS
const MAC_ADDRESS_MEMLOC: u8 = 0x02; // MAC is 6 bytes long
const SETTINGS_MEMLOC: u8 = 0x20; // 0x20 = 32 which is the start of the third page of memory
const SETTINGS_SIZE: usize = 32; // number of bytes to reserve for menu settings

#[allow(unused)]
#[derive(Clone, Copy, Format, Debug)]
pub enum EepromEvent {
    ReadModuleType,
    WriteModuleType(ModuleType),
    ReadSettings,
    WriteSettings(MenuData),
    ReadMacAddress,
    WriteMacAddress([u8; 6]),
}

pub struct Eeprom<I2C: I2CTRAIT> {
    dev: M24x02<I2C>,
    rx: EepromChannelRx,
    tx: RouterChannelTx,
}

// impl<I2C: embedded_hal_1::i2c::I2c> Eeprom<I2C> {
impl<I2C: I2CTRAIT> Eeprom<I2C> {
    pub fn new(i2c: I2C, address: u8, rx: EepromChannelRx, tx: RouterChannelTx) -> Self {
        info!("Define eeprom dev");
        let dev = M24x02::new(i2c, address);
        Self { dev, rx, tx }
    }

    pub async fn show(&mut self) -> Result<(), ()> {
        if let Ok(new_message) = with_timeout(Duration::from_millis(200), self.rx.receive()).await {
            self.process_event(new_message).await
        } else {
            Ok(())
        }
    }

    pub async fn refresh(&mut self) -> Result<(), ()> {
        // self.dev.write_page_wait(0x), data).await
        self.dev.refresh_bus().await.map_err(|_e| ())
    }

    async fn process_event(&mut self, event: EepromEvent) -> Result<(), ()> {
        match event {
            EepromEvent::ReadModuleType => {
                // let _ = self.dev.read_byte(MODULE_TYPE_MEMLOC); //.await;
                let data = self.dev.read_byte(MODULE_TYPE_MEMLOC);

                match data {
                    //.await {
                    Ok(data) => {
                        let decoded: ModuleType = bincode::decode_from_slice(&[data], bincode::config::standard()).unwrap_or_default().0;
                        let _ = self.tx.try_send(RouterEvent::StoreModuleType(Some(decoded)));
                    }
                    Err(e) => {
                        let _ = self.tx.try_send(RouterEvent::StoreModuleType(None));
                        error!("Eeprom - error {}", e);
                        return Err(());
                    }
                }
            }
            EepromEvent::WriteModuleType(module) => {
                let mut slice = [0u8; 1];

                let length = bincode::encode_into_slice(module, &mut slice, bincode::config::standard()).unwrap_or_else(|e| {
                    match e {
                        bincode::error::EncodeError::UnexpectedEnd => error!("Error encoding module type - UnexpectedEnd"),
                        _ => error!("Error encoding module type"),
                    }
                    0
                });

                if length > 0 {
                    #[allow(clippy::let_underscore_future)]
                    let _ = self.dev.write_byte_wait(MODULE_TYPE_MEMLOC, slice[0]); // throw away write due to shared bus issues
                    match self.dev.write_byte_wait(MODULE_TYPE_MEMLOC, slice[0]).await {
                        Ok(_) => {
                            let _ = self.tx.try_send(RouterEvent::StoreModuleType(Some(module)));
                            info!("Wrote module type as {}", module);
                        }
                        Err(e) => {
                            error!("Eeprom - error {}", e);
                            return Err(());
                        }
                    }
                }
            }
            EepromEvent::ReadMacAddress => {
                let mut buf = [0u8; 6];
                // let _ = self.dev.read_byte(MAC_ADDRESS_MEMLOC); // .await; // throw away read due to shared bus issues

                match self.dev.read_data(MAC_ADDRESS_MEMLOC, &mut buf) {
                    // .await {
                    Ok(_) => {
                        info!("EEPROM Read mac {:#X}", buf);
                        let _ = self.tx.try_send(RouterEvent::StoreMacAddress(Some(buf)));
                    }
                    Err(e) => {
                        error!("Eeprom - error {}", e);
                        let _ = self.tx.try_send(RouterEvent::StoreMacAddress(None));
                        return Err(());
                    }
                }
                // if let Some(data) = self.read_mac_address() {
                //     let _ = self.tx.try_send(RouterEvent::StoreMacAddress(data));
                // }
            }
            EepromEvent::WriteMacAddress(mac) => {
                info!("EEPROM Store mac {:#X}", mac);

                // There is something going on that makes it so page write only works if you write a byte to the device first then
                // do the page write...something with the address not being transmitted. Not sure if this is something the eeprom
                // is doing or something in the microcontroller or code.
                //
                // This problem appears to have something to do with using the shared bus...like the bus isn't properly cleared on other
                // read / write attempts.
                #[allow(clippy::let_underscore_future)]
                let _ = self.dev.write_byte_wait(MAC_ADDRESS_MEMLOC, mac[0]); //.await;
                match self.dev.write_page_wait(MAC_ADDRESS_MEMLOC, &mac).await {
                    Ok(_) => {
                        let _ = self.tx.try_send(RouterEvent::StoreMacAddress(Some(mac)));
                        info!("EEPROM MAC stored to {:#X} -- {:#X}", MAC_ADDRESS_MEMLOC, mac);
                    }
                    Err(e) => {
                        error!("MAC store {:#X} error {:#X} -- {:#X}", MAC_ADDRESS_MEMLOC, e, mac);
                        return Err(());
                    }
                }
            }
            EepromEvent::ReadSettings => {
                // if let Some(data) = self.read_settings() {
                //     let _ = self.tx.try_send(RouterEvent::StoreSettings(data));
                // }

                let mut buf = [0u8; SETTINGS_SIZE];
                // let _ = self.dev.read_byte(SETTINGS_MEMLOC); //.await; // throw away read due to shared bus issues

                match self.dev.read_data(SETTINGS_MEMLOC, &mut buf) {
                    // .await {
                    Ok(_) => {
                        let decoded: MenuData = bincode::decode_from_slice(&buf, bincode::config::standard()).unwrap_or_default().0;
                        info!("Read eeprom");
                        let _ = self.tx.try_send(RouterEvent::StoreSettings(Some(decoded)));
                    }
                    Err(e) => {
                        error!("Eeprom - error {}", e);
                        let _ = self.tx.try_send(RouterEvent::StoreSettings(None));
                        return Err(());
                    }
                }
            }
            EepromEvent::WriteSettings(menu_settings) => {
                let mut slice = [0u8; SETTINGS_SIZE];

                let length = bincode::encode_into_slice(menu_settings, &mut slice, bincode::config::standard()).unwrap_or_else(|e| {
                    match e {
                        bincode::error::EncodeError::UnexpectedEnd => error!("Error encoding menu settings - UnexpectedEnd"),
                        _ => error!("Error encoding menu settings"),
                    }
                    0
                });

                if length > 0 {
                    info!("Store settings {}. Data is {} bytes long", menu_settings, length);

                    let chunks = slice.chunks(PAGE_SIZE as usize).enumerate();

                    // There is something going on that makes it so page write only works if you write a byte to the device first then
                    // do the page write...something with the address not being transmitted. Not sure if this is something the eeprom
                    // is doing or something in the microcontroller or code.
                    //
                    // This problem appears to have something to do with using the shared bus...like the bus isn't properly cleared on other
                    // read / write attempts.
                    #[allow(clippy::let_underscore_future)]
                    let _ = self.dev.write_byte_wait(SETTINGS_MEMLOC, slice[0]); //.await;

                    for (num, chunk) in chunks {
                        let mem_addr = SETTINGS_MEMLOC + (num as u8) * PAGE_SIZE;
                        // self.dev.write_page_wait(mem_addr, chunk).await;
                        match self.dev.write_page_wait(mem_addr, chunk).await {
                            Ok(_) => info!("Data chunk {}/{:#X} stored -- {:#X}", num, mem_addr, chunk),
                            Err(e) => {
                                error!("Data chunk {}/{:#X} error {:#X} -- {:#X}", num, mem_addr, e, chunk);
                                return Err(());
                            }
                        }
                    }
                    let _ = self.tx.try_send(RouterEvent::StoreSettings(Some(menu_settings)));
                }
            }
        }
        Ok(())
    }
}

#[embassy_executor::task]
pub async fn eeprom_i2c_task(i2c: I2cSharedDev, address: u8, rx: EepromChannelRx, tx: RouterChannelTx) {
    let mut eeprom = Eeprom::new(i2c, address, rx, tx);
    loop {
        match eeprom.show().await {
            Ok(_) => {}
            Err(_) => {
                error!("EEPROM Error trying to restart");
                let _ = eeprom.refresh().await;
            }
        }
    }
}
