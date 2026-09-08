//! Application constants
//!
//! I2C device address, hardware settings, etc.

/// I2C address of the settings EEPROM (M24C02, E2 = E1 = 1, E0 = 0 on the Rev 2 carrier)
pub const EEPROM_ADDRESS: u8 = 0x56;

/// I2C address of the TCA9555 front-panel expander (A0 = A1 = A2 = 0)
pub const EXPANDER_ADDRESS: u8 = 0x20;

/// I2C address of the PCA9633DP1 backlight driver (fixed by the DP1 package)
pub const BACKLIGHT_ADDRESS: u8 = 0x62;

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

/// LEDs the hardware will actually clock out per port per frame.
///
/// This is `MAX_BYTES_PER_PORT` in RGB pixels, and it is what the PIO output
/// driver's fixed-size frame buffer is sized to. The UI clamps the per-port LED
/// count to it so a configured value can never be silently truncated on the
/// wire — before this, the menu accepted 999 and the output quietly stopped at
/// 600.
pub const MAX_LEDS_PER_PORT: usize = MAX_BYTES_PER_PORT as usize / 3;

/// Per-port size of `LED_COLORS`. Equal to the transmit cap: holding pixels
/// that can never be output only cost RAM.
pub const SMARTLED_NUM_LEDS_MAX: usize = MAX_LEDS_PER_PORT;
