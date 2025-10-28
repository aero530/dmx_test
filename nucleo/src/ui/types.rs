use core::net::Ipv4Addr;

use crate::ui::menu_value::ValueType;
use bincode::{Decode, Encode};
use embassy_net::IpAddress;

use smart_leds::{RGB, RGB8};

use super::IncDec;
use defmt::{Format, error};
use enum_ordinalize::Ordinalize;

#[derive(Clone, Copy, Default, PartialEq, Format, Debug, Decode, Encode)]
pub enum ModuleType {
    #[default]
    Unknown,
    SmartLed,
    Pwm,
}

pub enum MenuMovement {
    NextTab,
    PreviousTab,
    NextItem,
    PreviousItem,
    FirstItem,
    LastItem,
    UpdateValue((usize, usize, ValueType)),
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Format, Debug, Decode, Encode)]
pub struct IpAddrMenu([u8;4]);

impl IpAddrMenu {
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        IpAddrMenu([a, b, c, d])
    }

    pub fn octets(&self) -> [u8; 4] {
        self.0
    }
    
}

impl Default for IpAddrMenu {
    fn default() -> Self {
        IpAddrMenu([0,0,0,0])
    }
}

impl From<Ipv4Addr> for IpAddrMenu {
    fn from(value: Ipv4Addr) -> Self {
        let b = value.octets();
        IpAddrMenu(b)
    }
}


/// Data displayed / configured in the menu system
#[derive(Clone, Copy, PartialEq, Format, Debug, Decode, Encode)]
pub struct MenuData {
    pub dmx_address: u16,
    pub input_mode: InputMode,
    pub ethernet_ip_mode: EthernetIPMode,
    pub artnet_address: ArtNetAddr,
    pub module: ModuleSettings,
    pub ip_addr: IpAddrMenu,
}

impl Default for MenuData {
    fn default() -> Self {
        Self {
            dmx_address: 1,
            input_mode: InputMode::DMX,
            ethernet_ip_mode: EthernetIPMode::DHCP,
            artnet_address: ArtNetAddr::default(),
            module: ModuleSettings::default(),
            ip_addr: IpAddrMenu::new(0,0,0,0)
        }
    }
}

/// Module settings types
#[derive(Clone, Copy, PartialEq, Format, Debug, Decode, Encode)]
pub enum ModuleSettings {
    Pwm(PwmSettings),
    SmartLed(SmartLedSettings),
}

impl Default for ModuleSettings {
    fn default() -> Self {
        ModuleSettings::SmartLed(SmartLedSettings::default())
    }
}

/// PWM Settings
#[derive(Clone, Copy, Format, PartialEq, Default, Debug, Decode, Encode)]
pub struct PwmSettings {
    pub freq: u8,
}

/// Smart LED module settings
#[derive(Default, Clone, Copy, PartialEq, Format, Debug, Decode, Encode)]
pub struct SmartLedSettings {
    pub leds_per_port: [u16; 4],
    pub color_mode: SmartLedColorMode,
    pub port_mode: SmartLedPortMode,
    pub dmx_group_size: SmartLedDmxGroupSize,
}

/// Impl increment and decrement for u8
impl IncDec for u8 {
    fn increment(&self, _index: usize) -> u8 {
        if *self == 255 {
            0
        } else {
            self + 1
        }
    }

    fn decrement(&self, _index: usize) -> u8 {
        if *self == 0 {
            255
        } else {
            self - 1
        }
    }
}








/// Input mode
#[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format, Debug, Decode, Encode)]
pub enum InputMode {
    #[default]
    DMX,
    ArtNet,
}

impl IncDec for InputMode {
    fn increment(&self, _index: usize) -> Self {
        let next = self.ordinal().saturating_add(1);
        match Self::from_ordinal(next) {
            Some(n) => n,
            None => InputMode::DMX,
        }
    }

    fn decrement(&self, _index: usize) -> Self {
        let prev = self.ordinal().saturating_sub(1);
        match Self::from_ordinal(prev) {
            Some(n) => n,
            None => InputMode::ArtNet,
        }
    }
}

impl core::fmt::Display for InputMode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            InputMode::DMX => write!(f, "DMX"),
            InputMode::ArtNet => write!(f, "ArtNet"),
        }
    }
}



/// Ethernet IP mode
#[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format, Debug, Decode, Encode)]
pub enum EthernetIPMode {
    #[default]
    DHCP,
    Static,
}

impl IncDec for EthernetIPMode {
    fn increment(&self, _index: usize) -> Self {
        let next = self.ordinal().saturating_add(1);
        match Self::from_ordinal(next) {
            Some(n) => n,
            None => EthernetIPMode::DHCP,
        }
    }

    fn decrement(&self, _index: usize) -> Self {
        let prev = self.ordinal().saturating_sub(1);
        match Self::from_ordinal(prev) {
            Some(n) => n,
            None => EthernetIPMode::Static,
        }
    }
}

impl core::fmt::Display for EthernetIPMode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            EthernetIPMode::DHCP => write!(f, "DHCP"),
            EthernetIPMode::Static => write!(f, "Static"),
        }
    }
}



/// Smart LED group address type
#[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format, Debug, Decode, Encode)]
pub enum SmartLedColorMode {
    #[default]
    RGB,
    RGBW,
}

impl SmartLedColorMode {
    pub fn addr_size(&self) -> usize {
        match self {
            SmartLedColorMode::RGB => 3,
            SmartLedColorMode::RGBW => 4,
        }
    }

    pub fn rgb(&self, data: &[u8]) -> RGB8 {
        match self {
            SmartLedColorMode::RGB => {
                if data.len() >= 3 {
                    RGB8::new(data[0], data[1], data[2])
                } else {
                    RGB8::new(0,0,0)
                }
            },
            SmartLedColorMode::RGBW => {
                error!("Using RGBW color space but that it not implimented yet.");
                if data.len() >= 4 {
                    RGB8::new(data[0], data[1], data[2])
                } else {
                    RGB8::new(0,0,0)
                }
            },
        }
    }
}

impl IncDec for SmartLedColorMode {
    fn increment(&self, _index: usize) -> Self {
        let next = self.ordinal().saturating_add(1);
        match Self::from_ordinal(next) {
            Some(n) => n,
            None => SmartLedColorMode::RGB,
        }
    }

    fn decrement(&self, _index: usize) -> Self {
        let prev = self.ordinal().saturating_sub(1);
        match Self::from_ordinal(prev) {
            Some(n) => n,
            None => SmartLedColorMode::RGBW,
        }
    }
}

impl core::fmt::Display for SmartLedColorMode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            SmartLedColorMode::RGB => write!(f, "RGB"),
            SmartLedColorMode::RGBW => write!(f, "RGBW"),
        }
    }
}

/// Smart LED port mode
#[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format, Debug, Decode, Encode)]
pub enum SmartLedPortMode {
    #[default]
    /// Individual - DMX size = SUM(#leds/port) / DmxGroupSize * color width
    Individual,
    /// Mirror - DMX size = MAX(#leds/port) / DmxGroupSize * color width
    Mirror,
}

impl IncDec for SmartLedPortMode {
    fn increment(&self, _index: usize) -> Self {
        let next = self.ordinal().saturating_add(1);
        match Self::from_ordinal(next) {
            Some(n) => n,
            None => SmartLedPortMode::Individual,
        }
    }

    fn decrement(&self, _index: usize) -> Self {
        let prev = self.ordinal().saturating_sub(1);
        match Self::from_ordinal(prev) {
            Some(n) => n,
            None => SmartLedPortMode::Mirror,
        }
    }
}

impl core::fmt::Display for SmartLedPortMode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            SmartLedPortMode::Individual => write!(f, "Individual"),
            SmartLedPortMode::Mirror => write!(f, "Mirror"),
        }
    }
}


// LED DMX Group Size (1 to #PHYLEDs)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Format, Decode, Encode)]
pub struct SmartLedDmxGroupSize(pub [u16;4]);

impl Default for SmartLedDmxGroupSize {
    fn default() -> Self {
        Self([1, 1, 1, 1])
    }
}

/// Digit
#[derive(Default, Clone, Copy, PartialEq, Eq, Format)]
pub struct Udigit(pub u8);

impl IncDec for Udigit {
    fn increment(&self, _index: usize) -> Self {
        if *self == Udigit(9) {
            Udigit(0)
        } else {
            Udigit(self.0 + 1)
        }
    }

    fn decrement(&self, _index: usize) -> Self {
        if *self == Udigit(0) {
            Udigit(9)
        } else {
            Udigit(self.0 - 1)
        }
    }
}


#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Format, Decode, Encode)]
pub struct ArtNetAddr(pub [u8;3]);