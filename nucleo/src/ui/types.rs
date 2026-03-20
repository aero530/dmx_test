//! Types used in the user interface
use cfg_if::cfg_if;
use core::net::Ipv4Addr;

use crate::{ui::ValueType, DMX_UNIVERSE_SIZE, SMARTLED_PORT_COUNT};
use bincode::{Decode, Encode};
// use embassy_net::IpAddress;
use heapless::Vec;
use micromath::F32Ext;

use smart_leds::RGB8;

use super::IncDec;

use defmt::Format;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{error};
    } else {
        use defmt::{error};
    }
}
use enum_ordinalize::Ordinalize;

/// Output module type (defines which kind of module is connected)
#[derive(Clone, Copy, Default, PartialEq, Format, Debug, Decode, Encode)]
pub enum BootStatus {
    #[default]
    Failed,
    Success,
}

/// Flag to monitor a values selection mode
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// Not selected
    Normal,
    /// Highlighted but not editing
    Selected,
    /// Currently editing the value
    Editing,
}

/// Output module type (defines which kind of module is connected)
#[derive(Clone, Copy, Default, PartialEq, Format, Debug, Decode, Encode)]
pub enum ModuleType {
    #[default]
    Unknown,
    SmartLed,
    Pwm,
}

/// Action to take based on user interaction and current menu state.
pub enum MenuMovement {
    /// Go to next tab
    NextTab,
    /// Go to previous tab
    PreviousTab,
    /// Go to next item on the tab
    NextItem,
    /// Go to previous item on the tab
    PreviousItem,
    /// Go to the first item on the tab
    FirstItem,
    /// Go to the last item on the tab
    LastItem,
    /// Update the item value
    UpdateValue((usize, usize, ValueType)),
    /// Take no action
    None,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Format, Debug, Decode, Encode)]
pub struct IpAddrMenu([u8; 4]);

impl IpAddrMenu {
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        IpAddrMenu([a, b, c, d])
    }

    pub fn octets(&self) -> [u8; 4] {
        self.0
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
    pub ethernet_enabled: bool,
}

impl Default for MenuData {
    fn default() -> Self {
        Self {
            dmx_address: 1,
            input_mode: InputMode::Dmx,
            ethernet_ip_mode: EthernetIPMode::Dhcp,
            artnet_address: ArtNetAddr::default(),
            module: ModuleSettings::default(),
            ip_addr: IpAddrMenu::new(0, 0, 0, 0),
            ethernet_enabled: false,
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
    pub leds_per_port: [u16; SMARTLED_PORT_COUNT],
    // pub universe_offset: [u16; SMARTLED_PORT_COUNT],
    // pub virtual_leds_per_port: [u16; SMARTLED_PORT_COUNT],
    pub color_mode: SmartLedColorMode,
    pub port_mode: SmartLedPortMode,
    pub dmx_group_size: SmartLedDmxGroupSize,
}

impl SmartLedSettings {
    /// Return the number of virtual LEDs on each port.
    /// The number of virtual LEDs is calcuated based the number of physical LEDs
    /// and the grouping size for each port.
    pub fn virtual_leds_per_port(&self) -> [u16; SMARTLED_PORT_COUNT] {
        self.leds_per_port
            .iter()
            .zip(self.dmx_group_size.0.iter())
            .map(|(led_count, grouping)| (*led_count as f32 / *grouping as f32).ceil() as u16)
            .collect::<Vec<u16, SMARTLED_PORT_COUNT>>()
            .as_slice()
            .try_into()
            .unwrap_or_default()
    }

    pub fn universe_offset(&self) -> [u16; SMARTLED_PORT_COUNT] {
        let universe_count: [u16; SMARTLED_PORT_COUNT] = self
            .virtual_leds_per_port()
            .iter()
            .map(|num_virtual_leds| (*num_virtual_leds as f32 * self.color_mode.addr_size() as f32 / DMX_UNIVERSE_SIZE as f32).ceil() as u16)
            .collect::<Vec<u16, SMARTLED_PORT_COUNT>>()
            .as_slice()
            .try_into()
            .unwrap_or_default();

        universe_count
            .iter()
            .enumerate()
            .map(|(i, _c)| universe_count[0..i].iter().sum())
            .collect::<Vec<u16, SMARTLED_PORT_COUNT>>()
            .as_slice()
            .try_into()
            .unwrap_or_default()
    }
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
    Dmx,
    ArtNet,
}

impl InputMode {
    #[allow(unused)]
    pub fn dmx_addr_limit(&self) -> usize {
        match self {
            InputMode::Dmx => DMX_UNIVERSE_SIZE,
            InputMode::ArtNet => DMX_UNIVERSE_SIZE, // ArtNet still uses a 512 byte universe
        }
    }
}
impl IncDec for InputMode {
    fn increment(&self, _index: usize) -> Self {
        let next = self.ordinal().saturating_add(1);
        Self::from_ordinal(next).unwrap_or(InputMode::Dmx)
    }

    fn decrement(&self, _index: usize) -> Self {
        let prev = self.ordinal().saturating_sub(1);
        Self::from_ordinal(prev).unwrap_or(InputMode::ArtNet)
    }
}

impl core::fmt::Display for InputMode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            InputMode::Dmx => write!(f, "DMX"),
            InputMode::ArtNet => write!(f, "ArtNet"),
        }
    }
}

/// Ethernet IP mode
#[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format, Debug, Decode, Encode)]
pub enum EthernetIPMode {
    #[default]
    Dhcp,
    Static,
}

impl IncDec for EthernetIPMode {
    fn increment(&self, _index: usize) -> Self {
        let next = self.ordinal().saturating_add(1);
        Self::from_ordinal(next).unwrap_or(EthernetIPMode::Dhcp)
    }

    fn decrement(&self, _index: usize) -> Self {
        let prev = self.ordinal().saturating_sub(1);
        Self::from_ordinal(prev).unwrap_or(EthernetIPMode::Static)
    }
}

impl core::fmt::Display for EthernetIPMode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            EthernetIPMode::Dhcp => write!(f, "DHCP"),
            EthernetIPMode::Static => write!(f, "Static"),
        }
    }
}

/// Smart LED group address type
#[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format, Debug, Decode, Encode)]
pub enum SmartLedColorMode {
    #[default]
    Rgb,
    Rgbw,
}

impl SmartLedColorMode {
    pub fn addr_size(&self) -> usize {
        match self {
            SmartLedColorMode::Rgb => 3,
            SmartLedColorMode::Rgbw => 4,
        }
    }

    pub fn rgb(&self, data: &[u8]) -> RGB8 {
        match self {
            SmartLedColorMode::Rgb => {
                if data.len() >= 3 {
                    RGB8::new(data[0], data[1], data[2])
                } else {
                    RGB8::new(0, 0, 0)
                }
            }
            SmartLedColorMode::Rgbw => {
                error!("Using RGBW color space but that it not implimented yet.");
                if data.len() >= 4 {
                    RGB8::new(data[0], data[1], data[2])
                } else {
                    RGB8::new(0, 0, 0)
                }
            }
        }
    }
}

impl IncDec for SmartLedColorMode {
    fn increment(&self, _index: usize) -> Self {
        let next = self.ordinal().saturating_add(1);
        match Self::from_ordinal(next) {
            Some(n) => n,
            None => SmartLedColorMode::Rgb,
        }
    }

    fn decrement(&self, _index: usize) -> Self {
        let prev = self.ordinal().saturating_sub(1);
        match Self::from_ordinal(prev) {
            Some(n) => n,
            None => SmartLedColorMode::Rgbw,
        }
    }
}

impl core::fmt::Display for SmartLedColorMode {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            SmartLedColorMode::Rgb => write!(f, "RGB"),
            SmartLedColorMode::Rgbw => write!(f, "RGBW"),
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
pub struct SmartLedDmxGroupSize(pub [u16; 4]);

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
pub struct ArtNetAddr(pub [u8; 3]);
