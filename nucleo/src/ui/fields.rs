//! Declarative settings-field metadata.
//!
//! Every editable (or displayable) setting is a [`FieldId`]. All behavior —
//! label, valid range, digit count, rendering, editing, console get/set — is
//! defined here once, so navigation and clamping are generic code and a new
//! setting is added in exactly one place. Both the on-device TFT UI and the
//! USB console protocol are driven from this table.

use alloc::format;
use alloc::string::String;

use crate::ui::{
    EthernetIPMode, IncDec, InputMode, MenuData, ModuleSettings, SmartLedColorMode,
    SmartLedPortMode, SmartLedSettings,
};
use crate::DMX_UNIVERSE_SIZE;

/// One setting shown in the UI / exposed on the console.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FieldId {
    DmxAddress,
    InputMode,
    IpMode,
    IpAddr,
    ArtNetNet,
    ArtNetSubNet,
    ArtNetUniverse,
    PortMode,
    ColorMode,
    GroupSize(usize),
    LedsPerPort(usize),
    UniverseOffsets,
    EthernetEnabled,
}

/// A page (tab) of the on-device menu.
pub struct Page {
    pub title: &'static str,
    pub fields: &'static [FieldId],
}

pub const PAGES: &[Page] = &[
    Page {
        title: "Main",
        fields: &[
            FieldId::DmxAddress,
            FieldId::InputMode,
            FieldId::IpMode,
            FieldId::IpAddr,
            FieldId::ArtNetNet,
            FieldId::ArtNetSubNet,
            FieldId::ArtNetUniverse,
        ],
    },
    Page {
        title: "LED 1",
        fields: &[
            FieldId::PortMode,
            FieldId::ColorMode,
            FieldId::GroupSize(0),
            FieldId::GroupSize(1),
            FieldId::GroupSize(2),
            FieldId::GroupSize(3),
        ],
    },
    Page {
        title: "LED 2",
        fields: &[
            FieldId::LedsPerPort(0),
            FieldId::LedsPerPort(1),
            FieldId::LedsPerPort(2),
            FieldId::LedsPerPort(3),
            FieldId::UniverseOffsets,
        ],
    },
    Page {
        title: "System",
        fields: &[FieldId::EthernetEnabled],
    },
];

/// Iterate every field, in page order (used by the console `get` command).
pub fn all_fields() -> impl Iterator<Item = FieldId> {
    PAGES.iter().flat_map(|page| page.fields.iter().copied())
}

fn smart_led(data: &MenuData) -> Option<&SmartLedSettings> {
    match &data.module {
        ModuleSettings::SmartLed(settings) => Some(settings),
        _ => None,
    }
}

fn smart_led_mut(data: &mut MenuData) -> Option<&mut SmartLedSettings> {
    match &mut data.module {
        ModuleSettings::SmartLed(settings) => Some(settings),
        _ => None,
    }
}

impl FieldId {
    /// Label shown on the TFT menu.
    pub fn label(&self) -> &'static str {
        match self {
            FieldId::DmxAddress => "DMX Address",
            FieldId::InputMode => "Mode",
            FieldId::IpMode => "IP Mode",
            FieldId::IpAddr => "IP",
            FieldId::ArtNetNet => "ArtNet Net",
            FieldId::ArtNetSubNet => "ArtNet Sub",
            FieldId::ArtNetUniverse => "ArtNet Univ",
            FieldId::PortMode => "Port Mode",
            FieldId::ColorMode => "Color Mode",
            FieldId::GroupSize(0) => "Group Sz P1",
            FieldId::GroupSize(1) => "Group Sz P2",
            FieldId::GroupSize(2) => "Group Sz P3",
            FieldId::GroupSize(_) => "Group Sz P4",
            FieldId::LedsPerPort(0) => "LEDs Port 1",
            FieldId::LedsPerPort(1) => "LEDs Port 2",
            FieldId::LedsPerPort(2) => "LEDs Port 3",
            FieldId::LedsPerPort(_) => "LEDs Port 4",
            FieldId::UniverseOffsets => "Univ Offset",
            FieldId::EthernetEnabled => "Ethernet (reboot)",
        }
    }

    /// Key used by the USB console protocol (`get` / `set <key> <value>`).
    pub fn key(&self) -> &'static str {
        match self {
            FieldId::DmxAddress => "dmx_address",
            FieldId::InputMode => "input_mode",
            FieldId::IpMode => "ip_mode",
            FieldId::IpAddr => "ip",
            FieldId::ArtNetNet => "artnet_net",
            FieldId::ArtNetSubNet => "artnet_subnet",
            FieldId::ArtNetUniverse => "artnet_universe",
            FieldId::PortMode => "port_mode",
            FieldId::ColorMode => "color_mode",
            FieldId::GroupSize(0) => "group_size_1",
            FieldId::GroupSize(1) => "group_size_2",
            FieldId::GroupSize(2) => "group_size_3",
            FieldId::GroupSize(_) => "group_size_4",
            FieldId::LedsPerPort(0) => "leds_1",
            FieldId::LedsPerPort(1) => "leds_2",
            FieldId::LedsPerPort(2) => "leds_3",
            FieldId::LedsPerPort(_) => "leds_4",
            FieldId::UniverseOffsets => "universe_offsets",
            FieldId::EthernetEnabled => "ethernet_enabled",
        }
    }

    pub fn editable(&self) -> bool {
        !matches!(self, FieldId::IpAddr | FieldId::UniverseOffsets)
    }

    /// Number of digit sub-positions for per-digit editing; 0 for values
    /// edited as a whole (enums, bool).
    pub fn digits(&self) -> u8 {
        match self {
            FieldId::DmxAddress | FieldId::ArtNetNet => 3,
            FieldId::GroupSize(_) | FieldId::LedsPerPort(_) => 3,
            FieldId::ArtNetSubNet | FieldId::ArtNetUniverse => 2,
            _ => 0,
        }
    }

    /// Valid range for numeric fields.
    fn range(&self) -> (u16, u16) {
        match self {
            FieldId::DmxAddress => (1, DMX_UNIVERSE_SIZE as u16),
            FieldId::ArtNetNet => (0, 127),
            FieldId::ArtNetSubNet | FieldId::ArtNetUniverse => (0, 15),
            FieldId::GroupSize(_) => (1, 999),
            FieldId::LedsPerPort(_) => (0, 999),
            _ => (0, 0),
        }
    }

    /// Numeric value of the field, if it is numeric.
    fn get_u16(&self, data: &MenuData) -> Option<u16> {
        match self {
            FieldId::DmxAddress => Some(data.dmx_address),
            FieldId::ArtNetNet => Some(data.artnet_address.0[0] as u16),
            FieldId::ArtNetSubNet => Some(data.artnet_address.0[1] as u16),
            FieldId::ArtNetUniverse => Some(data.artnet_address.0[2] as u16),
            FieldId::GroupSize(i) => smart_led(data).map(|s| s.dmx_group_size.0[*i]),
            FieldId::LedsPerPort(i) => smart_led(data).map(|s| s.leds_per_port[*i]),
            _ => None,
        }
    }

    fn set_u16(&self, data: &mut MenuData, value: u16) {
        match self {
            FieldId::DmxAddress => data.dmx_address = value,
            FieldId::ArtNetNet => data.artnet_address.0[0] = value as u8,
            FieldId::ArtNetSubNet => data.artnet_address.0[1] = value as u8,
            FieldId::ArtNetUniverse => data.artnet_address.0[2] = value as u8,
            FieldId::GroupSize(i) => {
                if let Some(s) = smart_led_mut(data) {
                    s.dmx_group_size.0[*i] = value;
                }
            }
            FieldId::LedsPerPort(i) => {
                if let Some(s) = smart_led_mut(data) {
                    s.leds_per_port[*i] = value;
                }
            }
            _ => {}
        }
    }

    /// Value rendered as text. Numeric fields are zero-padded to their digit
    /// count so the per-digit edit cursor lines up.
    pub fn display(&self, data: &MenuData) -> String {
        if let Some(v) = self.get_u16(data) {
            return format!("{:0width$}", v, width = self.digits() as usize);
        }
        match self {
            FieldId::InputMode => format!("{}", data.input_mode),
            FieldId::IpMode => format!("{}", data.ethernet_ip_mode),
            FieldId::IpAddr => {
                let b = data.ip_addr.octets();
                format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
            }
            FieldId::PortMode => smart_led(data).map_or(String::from("-"), |s| format!("{}", s.port_mode)),
            FieldId::ColorMode => smart_led(data).map_or(String::from("-"), |s| format!("{}", s.color_mode)),
            FieldId::UniverseOffsets => smart_led(data).map_or(String::from("-"), |s| {
                let o = s.universe_offset();
                format!("{} {} {} {}", o[0], o[1], o[2], o[3])
            }),
            FieldId::EthernetEnabled => String::from(if data.ethernet_enabled { "On" } else { "Off" }),
            _ => String::new(),
        }
    }

    /// Apply one Up/Down press while editing. For numeric fields `digit`
    /// selects the digit (0 = most significant); the digit wraps 0..=9
    /// without carrying, and a result outside the valid range is rejected.
    pub fn adjust(&self, data: &mut MenuData, digit: u8, up: bool) {
        if let Some(v) = self.get_u16(data) {
            let (min, max) = self.range();
            let pow = 10u16.pow((self.digits() - 1 - digit) as u32);
            let current = (v / pow) % 10;
            let new_digit = if up { (current + 1) % 10 } else { (current + 9) % 10 };
            let new_value = v - current * pow + new_digit * pow;
            if new_value >= min && new_value <= max {
                self.set_u16(data, new_value);
            }
            return;
        }
        match self {
            FieldId::InputMode => {
                data.input_mode = if up { data.input_mode.increment(0) } else { data.input_mode.decrement(0) };
            }
            FieldId::IpMode => {
                data.ethernet_ip_mode = if up { data.ethernet_ip_mode.increment(0) } else { data.ethernet_ip_mode.decrement(0) };
            }
            FieldId::PortMode => {
                if let Some(s) = smart_led_mut(data) {
                    s.port_mode = if up { s.port_mode.increment(0) } else { s.port_mode.decrement(0) };
                }
            }
            FieldId::ColorMode => {
                if let Some(s) = smart_led_mut(data) {
                    s.color_mode = if up { s.color_mode.increment(0) } else { s.color_mode.decrement(0) };
                }
            }
            FieldId::EthernetEnabled => data.ethernet_enabled = !data.ethernet_enabled,
            _ => {}
        }
    }

    /// Copy this field's value from one settings struct to another. Used to
    /// merge a committed edit without clobbering fields that changed
    /// elsewhere (e.g. an IP update from DHCP) during the edit.
    pub fn transfer(&self, src: &MenuData, dst: &mut MenuData) {
        if let Some(v) = self.get_u16(src) {
            self.set_u16(dst, v);
            return;
        }
        match self {
            FieldId::InputMode => dst.input_mode = src.input_mode,
            FieldId::IpMode => dst.ethernet_ip_mode = src.ethernet_ip_mode,
            FieldId::PortMode => {
                if let (Some(s), Some(d)) = (smart_led(src), smart_led_mut(dst)) {
                    d.port_mode = s.port_mode;
                }
            }
            FieldId::ColorMode => {
                if let (Some(s), Some(d)) = (smart_led(src), smart_led_mut(dst)) {
                    d.color_mode = s.color_mode;
                }
            }
            FieldId::EthernetEnabled => dst.ethernet_enabled = src.ethernet_enabled,
            _ => {}
        }
    }

    /// Parse and apply a console-supplied value.
    pub fn set_from_str(&self, data: &mut MenuData, value: &str) -> Result<(), &'static str> {
        if !self.editable() {
            return Err("read-only field");
        }

        if self.digits() > 0 {
            let v: u16 = value.parse().map_err(|_| "not a number")?;
            let (min, max) = self.range();
            if v < min || v > max {
                return Err("value out of range");
            }
            self.set_u16(data, v);
            return Ok(());
        }

        let lower = value.to_ascii_lowercase();
        match self {
            FieldId::InputMode => {
                data.input_mode = match lower.as_str() {
                    "dmx" => InputMode::Dmx,
                    "artnet" => InputMode::ArtNet,
                    "artnet>dmx" => InputMode::ArtNetToDmx,
                    "usb>dmx" => InputMode::UsbToDmx,
                    _ => return Err("expected dmx|artnet|artnet>dmx|usb>dmx"),
                };
            }
            FieldId::IpMode => {
                data.ethernet_ip_mode = match lower.as_str() {
                    "dhcp" => EthernetIPMode::Dhcp,
                    "static" => EthernetIPMode::Static,
                    _ => return Err("expected dhcp|static"),
                };
            }
            FieldId::PortMode => {
                let mode = match lower.as_str() {
                    "individual" => SmartLedPortMode::Individual,
                    "mirror" => SmartLedPortMode::Mirror,
                    _ => return Err("expected individual|mirror"),
                };
                smart_led_mut(data).ok_or("module is not SmartLED")?.port_mode = mode;
            }
            FieldId::ColorMode => {
                let mode = match lower.as_str() {
                    "rgb" => SmartLedColorMode::Rgb,
                    "rgbw" => SmartLedColorMode::Rgbw,
                    _ => return Err("expected rgb|rgbw"),
                };
                smart_led_mut(data).ok_or("module is not SmartLED")?.color_mode = mode;
            }
            FieldId::EthernetEnabled => {
                data.ethernet_enabled = match lower.as_str() {
                    "on" | "true" | "1" => true,
                    "off" | "false" | "0" => false,
                    _ => return Err("expected on|off"),
                };
            }
            _ => return Err("cannot set this field"),
        }
        Ok(())
    }
}
