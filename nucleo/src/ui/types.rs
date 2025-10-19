use crate::ui::menu_value::ValueType;
use bincode::{Decode, Encode};

use super::IncDec;
use defmt::{info, Format};
use enum_ordinalize::Ordinalize;

#[derive(Clone, Copy, Default, PartialEq, Format, Debug, Decode, Encode)]
pub enum ModuleType {
    Pwm,
    SmartLed,
    #[default]
    Unknown,
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

/// Data displayed / configured in the menu system
#[derive(Clone, Copy, PartialEq, Format, Debug, Decode, Encode)]
pub struct MenuData {
    pub dmx_address: u16,
    pub module: ModuleSettings,
}

impl Default for MenuData {
    fn default() -> Self {
        Self {
            dmx_address: 0,
            module: ModuleSettings::default(),
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
    pub grouping: SmartLedGrouping,
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

/// Smart LED grouping mode
#[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format, Debug, Decode, Encode)]
pub enum SmartLedGrouping {
    #[default]
    Individual,
    CombineByPort,
    CombineByModule,
}

impl IncDec for SmartLedGrouping {
    fn increment(&self, _index: usize) -> Self {
        let next = self.ordinal().saturating_add(1);
        match Self::from_ordinal(next) {
            Some(n) => n,
            None => SmartLedGrouping::Individual,
        }
    }

    fn decrement(&self, _index: usize) -> Self {
        let prev = self.ordinal().saturating_sub(1);
        match Self::from_ordinal(prev) {
            Some(n) => n,
            None => SmartLedGrouping::CombineByModule,
        }
    }
}

impl core::fmt::Display for SmartLedGrouping {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            SmartLedGrouping::Individual => write!(f, "Individual"),
            SmartLedGrouping::CombineByPort => write!(f, "By Port"),
            SmartLedGrouping::CombineByModule => write!(f, "Combined"),
        }
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
