//! Static memory allocations
//!
//! Data buffers and shared hardware allocations

use core::cell::RefCell;
use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_stm32::i2c::{I2c, Master};
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::blocking_mutex::NoopMutex;
use embassy_sync::mutex::Mutex;
use smart_leds::RGB8;
use static_cell::StaticCell;

use crate::{DMX_UNIVERSE_COUNT, DMX_UNIVERSE_SIZE, SMARTLED_NUM_LEDS_MAX, SMARTLED_PORT_COUNT};

pub type LedBuffer = Mutex<ThreadModeRawMutex, [[RGB8; SMARTLED_NUM_LEDS_MAX]; SMARTLED_PORT_COUNT]>;

/// Buffer of current LED colors
pub static LED_COLORS: LedBuffer = Mutex::new([[RGB8::new(0, 0, 0); SMARTLED_NUM_LEDS_MAX]; SMARTLED_PORT_COUNT]);

type DmxBuffer = Mutex<ThreadModeRawMutex, [u8; DMX_UNIVERSE_COUNT * DMX_UNIVERSE_SIZE]>;

/// Buffer to hold incoming data (DMX or ArtNet)
///
/// Data is stored flat, covering one full Art-Net net (256 universes):
/// location = sub_uni * 512 + channel offset, where sub_uni is the
/// Port-Address "SubUni" byte (sub-net in the high nibble, universe in the
/// low nibble). Wired DMX / USB use offset 0 (start code at index 0).
pub static DMX_BUFFER: DmxBuffer = Mutex::new([0_u8; DMX_UNIVERSE_COUNT * DMX_UNIVERSE_SIZE]);

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
