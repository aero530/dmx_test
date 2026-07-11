//! Enum to wrap different value data types
use cfg_if::cfg_if;
use crate::ui::{digits_to_u16, split_digits_no_std, ArtNetAddr, EthernetIPMode, IncDec, InputMode, IpAddrMenu, SmartLedColorMode, SmartLedDmxGroupSize, SmartLedPortMode};
use crate::SMARTLED_PORT_COUNT;
use bincode::{Decode, Encode};
use defmt::Format;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{error, info};
    } else {
        use defmt::{error, info};
    }
}


#[allow(unused)]
/// Enum wrapper for the data in a MenuValue
#[derive(Clone, Copy, PartialEq, Eq, Format, Decode, Encode)]
pub enum ValueType {
    None,
    U8(u8),
    SmartLedPortMode(SmartLedPortMode),
    SmartLedColorMode(SmartLedColorMode),
    InputMode(InputMode),
    EthernetIPMode(EthernetIPMode),
    EthernetEnabled(bool),
    Uint3(u16),
    Ip(IpAddrMenu),
    ArtNetAddr(ArtNetAddr),
    SmartLedDmxGroupSize(SmartLedDmxGroupSize),
    PortUniverseOffsets([u16; SMARTLED_PORT_COUNT]),
}

impl ValueType {
    pub fn size(&self) -> usize {
        match self {
            ValueType::None => 0,
            ValueType::U8(_x) => 1,
            ValueType::SmartLedPortMode(_x) => 1,
            ValueType::SmartLedColorMode(_x) => 1,
            ValueType::SmartLedDmxGroupSize(_x) => 12,
            ValueType::Uint3(_x) => 3,
            ValueType::InputMode(_x) => 1,
            ValueType::EthernetIPMode(_x) => 1,
            ValueType::Ip(_x) => 1,
            ValueType::ArtNetAddr(_x) => 9,
            ValueType::PortUniverseOffsets(_x) => 1,
            ValueType::EthernetEnabled(_x) => 1,
        }
    }

    #[allow(unused)]
    pub fn extract_u8(&self) -> u8 {
        match self {
            ValueType::U8(x) => *x,
            _ => {
                error!("Unable to extract u8 - setting to default value");
                0
            }
        }
    }

    #[allow(unused)]
    pub fn extract_port_mode(&self) -> SmartLedPortMode {
        info!("Extract grouping {}", self);
        match self {
            ValueType::SmartLedPortMode(x) => *x,
            _ => {
                error!("Unable to extract port mode - setting to default value");
                SmartLedPortMode::default()
            }
        }
    }

    #[allow(unused)]
    pub fn extract_led_color_mode(&self) -> SmartLedColorMode {
        info!("Extract color mode {}", self);
        match self {
            ValueType::SmartLedColorMode(x) => *x,
            _ => {
                error!("Unable to extract LED color mode - setting to default value");
                SmartLedColorMode::default()
            }
        }
    }

    #[allow(unused)]
    pub fn extract_dmx_group_size(&self) -> SmartLedDmxGroupSize {
        info!("Extract DMX group size {}", self);
        match self {
            ValueType::SmartLedDmxGroupSize(x) => *x,
            _ => {
                error!("Unable to extract DMX group size - setting to default value");
                SmartLedDmxGroupSize::default()
            }
        }
    }

    #[allow(unused)]
    pub fn extract_uint3(&self) -> u16 {
        match self {
            ValueType::Uint3(x) => *x,
            _ => {
                error!("Unable to extract UINT3 - setting to default value");
                0
            }
        }
    }

    #[allow(unused)]
    pub fn extract_input_mode(&self) -> InputMode {
        info!("Extract input mode {}", self);
        match self {
            ValueType::InputMode(x) => *x,
            _ => {
                error!("Unable to extract input mode - setting to default value");
                InputMode::default()
            }
        }
    }

    #[allow(unused)]
    pub fn extract_ethernet_ip_mode(&self) -> EthernetIPMode {
        info!("Extract ethernet ip mode {}", self);
        match self {
            ValueType::EthernetIPMode(x) => *x,
            _ => {
                error!("Unable to extract ethernet ip mode - setting to default value");
                EthernetIPMode::default()
            }
        }
    }

    #[allow(unused)]
    pub fn extract_ethernet_enabled(&self) -> bool {
        info!("Extract ethernet enabled mode {}", self);
        match self {
            ValueType::EthernetEnabled(x) => *x,
            _ => {
                error!("Unable to extract ethernet enabled - setting to default value");
                false
            }
        }
    }

    #[allow(unused)]
    pub fn extract_ip(&self) -> IpAddrMenu {
        match self {
            ValueType::Ip(x) => *x,
            _ => {
                error!("Unable to extract ip - setting to default value");
                IpAddrMenu::new(0, 0, 0, 0)
            }
        }
    }

    #[allow(unused)]
    pub fn extract_artnet(&self) -> ArtNetAddr {
        match self {
            ValueType::ArtNetAddr(x) => *x,
            _ => {
                error!("Unable to extract ArtNetAddr - setting to default value");
                ArtNetAddr::default()
            }
        }
    }
}
impl IncDec for ValueType {
    fn increment(&self, index: usize) -> Self {
        match self {
            ValueType::None => ValueType::None,
            ValueType::U8(x) => ValueType::U8(x.increment(index)),
            ValueType::SmartLedPortMode(x) => ValueType::SmartLedPortMode(x.increment(index)),
            ValueType::SmartLedColorMode(x) => ValueType::SmartLedColorMode(x.increment(index)),
            ValueType::SmartLedDmxGroupSize(x) => {
                let mut new = *x;
                let b_index = index / 3;
                let e = new.0[b_index];
                let mut v = split_digits_no_std(e, 3);

                let i = (v.len() - 1) - (index - b_index * 3);
                v[i] = v[i].increment(i);
                let j = digits_to_u16(v);
                // Group size must be at least 1 (a group of 0 LEDs is meaningless
                // and causes a divide-by-zero when computing virtual LEDs)
                let out = if j > 999 || j == 0 { new.0[b_index] } else { j };

                new.0[b_index] = out;
                ValueType::SmartLedDmxGroupSize(new)
            }
            ValueType::InputMode(x) => ValueType::InputMode(x.increment(index)),
            ValueType::EthernetIPMode(x) => ValueType::EthernetIPMode(x.increment(index)),
            ValueType::Uint3(x) => {
                let new = *x;
                let mut v = split_digits_no_std(new, 3);
                let i = (v.len() - 1) - index;
                v[i] = v[i].increment(i);
                let out = digits_to_u16(v);
                ValueType::Uint3(out)
            }
            ValueType::Ip(x) => {
                //todo!();
                ValueType::Ip(*x)
            }
            ValueType::ArtNetAddr(x) => {
                let mut new = *x;
                let b_index = index / 3;
                let e = new.0[b_index] as u16;
                let mut v = split_digits_no_std(e, 3);

                let i = (v.len() - 1) - (index - b_index * 3);
                v[i] = v[i].increment(i);
                let j = digits_to_u16(v);
                // On the wire Net is 7 bits and Sub-Net/Universe are 4 bits each
                let max: u16 = [127, 15, 15][b_index];
                let out = if j > max { new.0[b_index] } else { j as u8 };

                new.0[b_index] = out;
                ValueType::ArtNetAddr(new)
            }
            ValueType::PortUniverseOffsets(x) => ValueType::PortUniverseOffsets(*x),
            ValueType::EthernetEnabled(x) => ValueType::EthernetEnabled(!x),
        }
    }

    fn decrement(&self, index: usize) -> Self {
        match self {
            ValueType::None => ValueType::None,
            ValueType::U8(x) => ValueType::U8(x.decrement(index)),
            ValueType::SmartLedPortMode(x) => ValueType::SmartLedPortMode(x.decrement(index)),
            ValueType::SmartLedColorMode(x) => ValueType::SmartLedColorMode(x.decrement(index)),
            ValueType::SmartLedDmxGroupSize(x) => {
                let mut new = *x;
                let b_index = index / 3;

                let e = new.0[b_index];
                let mut v = split_digits_no_std(e, 3);

                let i = (v.len() - 1) - (index - b_index * 3);
                v[i] = v[i].decrement(i);
                let j = digits_to_u16(v);
                // Group size must be at least 1 (a group of 0 LEDs is meaningless
                // and causes a divide-by-zero when computing virtual LEDs)
                let out = if j > 999 || j == 0 { new.0[b_index] } else { j };
                new.0[b_index] = out;
                ValueType::SmartLedDmxGroupSize(new)
            }
            ValueType::InputMode(x) => ValueType::InputMode(x.decrement(index)),
            ValueType::EthernetIPMode(x) => ValueType::EthernetIPMode(x.decrement(index)),
            ValueType::Uint3(x) => {
                let new = *x;
                let mut v = split_digits_no_std(new, 3);
                let i = (v.len() - 1) - index;
                v[i] = v[i].decrement(i);
                let out = digits_to_u16(v);
                ValueType::Uint3(out)
            }
            ValueType::Ip(x) => {
                //todo!();
                ValueType::Ip(*x)
            }
            ValueType::ArtNetAddr(x) => {
                let mut new: ArtNetAddr = *x;
                let b_index = index / 3;
                let e = new.0[b_index] as u16;
                let mut v = split_digits_no_std(e, 3);

                let i = (v.len() - 1) - (index - b_index * 3);
                v[i] = v[i].decrement(i);
                let j = digits_to_u16(v);
                // On the wire Net is 7 bits and Sub-Net/Universe are 4 bits each
                let max: u16 = [127, 15, 15][b_index];
                let out = if j > max { new.0[b_index] } else { j as u8 };
                new.0[b_index] = out;
                ValueType::ArtNetAddr(new)
            }
            ValueType::PortUniverseOffsets(x) => ValueType::PortUniverseOffsets(*x),
            ValueType::EthernetEnabled(x) => ValueType::EthernetEnabled(!x),
        }
    }
}

// How the value type is displayed in the UI
impl core::fmt::Display for ValueType {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            ValueType::None => write!(f, ""),
            ValueType::U8(x) => write!(f, "{}", x),
            ValueType::SmartLedPortMode(x) => write!(f, "{}", x),
            ValueType::SmartLedColorMode(x) => write!(f, "{}", x),
            ValueType::SmartLedDmxGroupSize(x) => write!(f, "{} {} {} {}", x.0[0], x.0[1], x.0[2], x.0[3]),
            ValueType::InputMode(x) => write!(f, "{}", x),
            ValueType::EthernetIPMode(x) => write!(f, "{}", x),
            ValueType::Uint3(x) => write!(f, "{}", x),
            ValueType::Ip(x) => {
                let b = x.octets();
                write!(f, "{}.{}.{}.{}", b[0], b[1], b[2], b[3])
            }
            ValueType::ArtNetAddr(x) => write!(f, "{} {} {}", x.0[0], x.0[1], x.0[2]),
            ValueType::PortUniverseOffsets(x) => write!(f, "{} {} {} {}", x[0], x[1], x[2], x[3]),
            ValueType::EthernetEnabled(x) => write!(f, "{}", x),
        }
    }
}
