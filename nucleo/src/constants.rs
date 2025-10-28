pub const PWM_ADDRESS: u8 = 0x60;
pub const EEPROM_ADDRESS: u8 = 0x56;
pub const DMX_ADDRESS: u8 = 0x33;
pub const DMX_BUFF_SIZE: usize = 513;
pub const DISPLAY_WIDTH: u16 = 172;
pub const DISPLAY_HEIGHT: u16 = 320;
pub const DISPLAY_OFFSET: u16 = 34; // (240 buffer size - 172 lcd size) / 2 = 34
pub const SMARTLED_PORT_COUNT: usize = 4;

pub const ARTNET_OEM: u16 = 0x7FFF;