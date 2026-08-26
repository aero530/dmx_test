//! Application constants
//!
//! I2C device address, hardware settings, etc.

/// I2C address of output module PWM chip
pub const PWM_ADDRESS: u8 = 0x60;

/// I2C address of the output module EEPROM
pub const EEPROM_ADDRESS: u8 = 0x56;

/// I2C address used to communicate with RPI
pub const DMX_ADDRESS: u8 = 0x33;

/// Buffer size for incoming DMX data (512 of data + 0 for 'all call' byte)
pub const DMX_BUFF_SIZE: usize = 513;

/// Number of bytes in a single DMX universe
pub const DMX_UNIVERSE_SIZE: usize = 512;

/// Maximum number of universes buffered.
///
/// 64 = sub-nets 0..=3 of the configured Net. The buffer is indexed by the
/// Port-Address "SubUni" byte, so multi-universe port spans still cross sub-net
/// boundaries freely within that range.
///
/// Sized from the actual payload: 8 ports x 600 RGB LEDs is 8 x ceil(1800/512)
/// = **32 universes**, so 64 leaves 2x headroom for the configurable base
/// offset. It was 256 (a full net, 128 KB); at 64 the buffer is 32 KB, which is
/// most of the RAM Phase 3 reclaims.
///
/// Packets addressed above this are dropped by the Art-Net task rather than
/// indexed — see the guard there.
pub const DMX_UNIVERSE_COUNT: usize = 64;

/// Width of the display in pixels (display hardware is portrait so this is the short dimension)
pub const DISPLAY_WIDTH: u16 = 172;

/// Height of the display in pixels (display hardware is portrait so this is the long dimension)
pub const DISPLAY_HEIGHT: u16 = 320;

/// Offset needed because the display hardware buffer differs in size from the physical display.
///
/// (240 buffer size - 172 lcd size) / 2 = 34
pub const DISPLAY_OFFSET: u16 = 34;

/// Number of ports on the SmartLED output module
pub const SMARTLED_PORT_COUNT: usize = 8;

/// Maximum number of colors (bytes) used to describe the color of a single LED
///
/// default to 4 for the ability to use RGBW or RGBA color
pub const COLORS_PER_LED_MAX: usize = 4; // such as RGBW

/// ArtNet OEM constant
pub const ARTNET_OEM: u16 = 0x7FFF;

/// Byte budget per output port per frame.
///
/// 1800 B = 600 RGB LEDs or 450 RGBW. Colour mode and LED count are two ways of
/// spending the same budget, which is why the limit is expressed in bytes and
/// not as a LED count: RGBW does not need a different plan, just a shorter
/// maximum string.
///
/// Two ceilings sit behind this number. The *strip* takes 30 us/LED (24 bits) or
/// 40 us/LED for SK6812 RGBW, so 1800 B is an 18 ms frame — a 55 Hz ceiling,
/// above the 44 Hz Art-Net rate. The *network* limit is whatever Phase 0
/// measures for single-SPI MACRAW; revise this once that number exists.
pub const MAX_BYTES_PER_PORT: u16 = 1800;

/// Maximum number of LEDs allowed on a single SmartLED string
///
/// in theory this could be up to floor(2^16 / 12) = 5461
/// 2^16 = MAX DMA size, 12 bytes per LED needed
pub const SMARTLED_NUM_LEDS_MAX: usize = 1024;
