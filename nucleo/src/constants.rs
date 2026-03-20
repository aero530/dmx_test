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

/// Maximum number of universes supported (how much memory is allocated for data buffers)
pub const DMX_UNIVERSE_COUNT: usize = 256;

/// Width of the display in pixels (display hardware is portrait so this is the short dimension)
pub const DISPLAY_WIDTH: u16 = 172;

/// Height of the display in pixels (display hardware is portrait so this is the long dimension)
pub const DISPLAY_HEIGHT: u16 = 320;

/// Offset needed because the display hardware buffer differs in size from the physical display.
///
/// (240 buffer size - 172 lcd size) / 2 = 34
pub const DISPLAY_OFFSET: u16 = 34;

/// Number of menu items (rows) each tab in the UI can display
pub const MENU_ITEMS_PER_TAB: usize = 5;

/// Number of tabs in the UI
pub const MENU_NUM_TABS: usize = 4;

/// Number of ports on the SmartLED output module
pub const SMARTLED_PORT_COUNT: usize = 4;

/// Maximum number of colors (bytes) used to describe the color of a single LED
///
/// default to 4 for the ability to use RGBW or RGBA color
pub const COLORS_PER_LED_MAX: usize = 4; // such as RGBW

/// ArtNet OEM constant
pub const ARTNET_OEM: u16 = 0x7FFF;

/// Maximum number of LEDs allowed on a single SmartLED string
///
/// in theory this could be up to floor(2^16 / 12) = 5461
/// 2^16 = MAX DMA size, 12 bytes per LED needed
pub const SMARTLED_NUM_LEDS_MAX: usize = 1024;
