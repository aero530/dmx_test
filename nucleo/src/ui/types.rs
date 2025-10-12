use super::IncDec;
use defmt::{info, Format};
use enum_ordinalize::Ordinalize;

/// Data displayed / configured in the menu system
#[derive(Clone, Copy)]
pub struct MenuData {
    dmx_address: u16,
    ip: (u8, u8, u8, u8),
    module: ModuleSettings,
}

impl Default for MenuData {
    fn default() -> Self {
        Self {
            dmx_address: 0,
            ip: (0, 0, 0, 0),
            module: ModuleSettings::SmartLed(SmartLedSettings::default()),
        }
    }
}

/// Module settings types
#[derive(Clone, Copy)]
pub enum ModuleSettings {
    Pwm(PwmSettings),
    SmartLed(SmartLedSettings),
}

/// PWM Settings
#[derive(Clone, Copy)]
pub struct PwmSettings {
    freq: u8,
}

/// Smart LED module settings
#[derive(Default, Clone, Copy)]
pub struct SmartLedSettings {
    leds_per_port: (u16, u16, u16, u16),
    address_mode: SmartLedGroupingAddressing,
    grouping: SmartLedGrouping,
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

    // fn size(&self) -> usize {
    //     1
    // }
}

/// Smart LED group address type
#[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format)]
pub enum SmartLedGroupingAddressing {
    #[default]
    RGB,
    RGBW,
}

impl IncDec for SmartLedGroupingAddressing {
    fn increment(&self, _index: usize) -> Self {
        let next = self.ordinal().saturating_add(1);
        match Self::from_ordinal(next) {
            Some(n) => n,
            None => SmartLedGroupingAddressing::RGB,
        }
    }

    fn decrement(&self, _index: usize) -> Self {
        let prev = self.ordinal().saturating_sub(1);
        match Self::from_ordinal(prev) {
            Some(n) => n,
            None => SmartLedGroupingAddressing::RGBW,
        }
    }
    // fn size(&self) -> usize {
    //     1
    // }
}

impl core::fmt::Display for SmartLedGroupingAddressing {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            SmartLedGroupingAddressing::RGB => write!(f, "RGB"),
            SmartLedGroupingAddressing::RGBW => write!(f, "RGBW"),
        }
    }
}

/// Smart LED grouping mode
#[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format)]
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
    // fn size(&self) -> usize {
    //     1
    // }
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

// /// Digit
// #[derive(Default, Clone, Copy, PartialEq, Eq, Ordinalize, Format)]
// pub enum Digit {
//     #[default]
//     _0,
//     _1,
//     _2,
//     _3,
//     _4,
//     _5,
//     _6,
//     _7,
//     _8,
//     _9,
// }

// impl IncDec for Digit {
//     fn increment(&self, _index: usize) -> Self {
//         info!("types.rs old val {:#?}", self);

//         let next = self.ordinal().saturating_add(1);
//         let new = match Self::from_ordinal(next) {
//             Some(n) => n,
//             None => Digit::_0,
//         };

//         info!("types.rs new val {:#?}", new);

//         new
//     }

//     fn decrement(&self, _index: usize) -> Self {
//         let prev = self.ordinal().saturating_sub(1);
//         match Self::from_ordinal(prev) {
//             Some(n) => n,
//             None => Digit::_9,
//         }
//     }
//     //     fn size(&self) -> usize {
//     //         1
//     //     }
// }

// impl core::fmt::Display for Digit {
//     fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
//         match self {
//             Digit::_0 => write!(f, "0"),
//             Digit::_1 => write!(f, "1"),
//             Digit::_2 => write!(f, "2"),
//             Digit::_3 => write!(f, "3"),
//             Digit::_4 => write!(f, "4"),
//             Digit::_5 => write!(f, "5"),
//             Digit::_6 => write!(f, "6"),
//             Digit::_7 => write!(f, "7"),
//             Digit::_8 => write!(f, "8"),
//             Digit::_9 => write!(f, "9"),
//         }
//     }
// }

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
