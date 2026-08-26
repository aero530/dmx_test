//! Static hardware allocations
//!
//! The shared I2C buses. The data buffers (`DMX_BUFFER`, `LED_COLORS`) are
//! target-agnostic and live in `common::buffers`; they are re-exported here so
//! the existing `crate::DMX_BUFFER` paths keep resolving.

use core::cell::RefCell;
use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_stm32::i2c::{I2c, Master};
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::blocking_mutex::NoopMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

pub use common::buffers::{DmxBuffer, LedBuffer, DMX_BUFFER, LED_COLORS};

pub type I2c1Bus = Mutex<ThreadModeRawMutex, I2c<'static, embassy_stm32::mode::Async, Master>>;
pub type I2cSharedDev = I2cDevice<'static, NoopRawMutex, I2c<'static, embassy_stm32::mode::Async, Master>>;

/// Display I2C / Smbus - I2C2
/// SCL: PF1
/// SDA: PF0
/// Alert#: PF2
/// Reset: PF3
pub static _I2C_BUS_DISPLAY: StaticCell<I2c1Bus> = StaticCell::new();

/// DMX I2C / Smbus - I2C1
/// SCL: PB8
/// SDA: PB9
/// Alert#: PB5
/// Reset: PA3
pub static I2C_BUS_DMX: StaticCell<I2c1Bus> = StaticCell::new();

/// LED (output) I2C / Smbus - I2C4
pub static I2C_BUS_LED: StaticCell<NoopMutex<RefCell<I2c<'static, embassy_stm32::mode::Async, Master>>>> = StaticCell::new();
