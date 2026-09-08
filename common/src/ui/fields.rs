//! Declarative settings-field metadata.
//!
//! Every editable (or displayable) setting is a [`FieldId`]. All behavior —
//! label, valid range, digit count, rendering, editing, console get/set — is
//! defined here once, so navigation and clamping are generic code and a new
//! setting is added in exactly one place. Both the on-device TFT UI and the
//! USB console protocol are driven from this table.

use alloc::format;
use alloc::string::String;

use defmt::Format;

use crate::ui::{
    LedPower,
    EthernetIPMode, IncDec, InputMode, IpAddrMenu, MenuData, ModuleSettings, SmartLedColorMode,
    SmartLedPortMode, SmartLedSettings,
};
use crate::{DMX_UNIVERSE_SIZE, MAX_LEDS_PER_PORT};

/// One setting shown in the UI / exposed on the console.
#[derive(Clone, Copy, PartialEq, Eq, Format, Debug)]
pub enum FieldId {
    DmxAddress,
    InputMode,
    /// DHCP or the static address below. Applies at the next boot.
    IpMode,
    /// The address in use (read-only; DHCP result or the static address).
    IpAddr,
    StaticIp,
    StaticPrefix,
    StaticGateway,
    ArtNetNet,
    ArtNetSubNet,
    ArtNetUniverse,
    /// First sACN universe (1-based, per E1.31).
    SacnUniverse,
    /// Read-only: how many universes the active mode binds from its base —
    /// the same number the Art-Net BindIndex replies advertise.
    UniversesBound,
    PortMode,
    ColorMode,
    GroupSize(usize),
    LedsPerPort(usize),
    UniverseOffsets,
    /// Read-only: universes consumed vs. what the buffer covers, and whether
    /// the configuration is inside its byte budget.
    TotalUniverses,
    EthernetEnabled,
    LedPower,
    /// TFT backlight duty, 1..=255.
    Backlight,
}

/// Per-port field labels.
///
/// Kept to 14 characters so a label plus its value fits the 35-column grid of
/// the 320x172 TFT — values start at column 16 (`pico2/src/tft_ui.rs`). Extend
/// both tables if `SMARTLED_PORT_COUNT` grows past 8 — the `.min()` guard keeps
/// it safe but would repeat the last label.
const GROUP_LABELS: [&str; 8] = [
    "Group Size P1", "Group Size P2", "Group Size P3", "Group Size P4",
    "Group Size P5", "Group Size P6", "Group Size P7", "Group Size P8",
];
/// Console protocol keys, one per port. `dmx_console` discovers these by
/// iterating `all_fields`, so extending the tables is all that is needed for it
/// to drive the new ports.
const GROUP_KEYS: [&str; 8] = [
    "group_size_1", "group_size_2", "group_size_3", "group_size_4",
    "group_size_5", "group_size_6", "group_size_7", "group_size_8",
];
const LED_KEYS: [&str; 8] = [
    "leds_1", "leds_2", "leds_3", "leds_4", "leds_5", "leds_6", "leds_7", "leds_8",
];

const LED_LABELS: [&str; 8] = [
    "LEDs Port 1", "LEDs Port 2", "LEDs Port 3", "LEDs Port 4",
    "LEDs Port 5", "LEDs Port 6", "LEDs Port 7", "LEDs Port 8",
];

/// Most fields any one page may carry.
///
/// The 320x172 TFT is **35x11 characters** with `FONT_9X15` (see
/// `pico2/src/tft_ui.rs`), and one row goes to the page title / status line.
/// Exceeding this means fields fall off the bottom of the display with no
/// scroll to reach them — the menu is paged, not scrolling. Enforced by a test
/// in `host_tests`.
pub const MAX_FIELDS_PER_PAGE: usize = 10;

/// Column where field values start on the display; labels must fit before it.
pub const VALUE_COLUMN: u16 = 16;

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
            FieldId::ArtNetNet,
            FieldId::ArtNetSubNet,
            FieldId::ArtNetUniverse,
            FieldId::SacnUniverse,
            FieldId::UniversesBound,
        ],
    },
    Page {
        title: "Network",
        fields: &[
            FieldId::IpMode,
            FieldId::IpAddr,
            FieldId::StaticIp,
            FieldId::StaticPrefix,
            FieldId::StaticGateway,
        ],
    },
    Page {
        title: "LED",
        fields: &[
            FieldId::PortMode,
            FieldId::ColorMode,
            FieldId::TotalUniverses,
            FieldId::UniverseOffsets,
        ],
    },
    Page {
        title: "Groups",
        fields: &[
            FieldId::GroupSize(0),
            FieldId::GroupSize(1),
            FieldId::GroupSize(2),
            FieldId::GroupSize(3),
            FieldId::GroupSize(4),
            FieldId::GroupSize(5),
            FieldId::GroupSize(6),
            FieldId::GroupSize(7),
        ],
    },
    Page {
        title: "LEDs",
        fields: &[
            FieldId::LedsPerPort(0),
            FieldId::LedsPerPort(1),
            FieldId::LedsPerPort(2),
            FieldId::LedsPerPort(3),
            FieldId::LedsPerPort(4),
            FieldId::LedsPerPort(5),
            FieldId::LedsPerPort(6),
            FieldId::LedsPerPort(7),
        ],
    },
    Page {
        title: "System",
        fields: &[FieldId::EthernetEnabled, FieldId::Backlight, FieldId::LedPower],
    },
];

/// Iterate every field, in page order (used by the console `get` command).
pub fn all_fields() -> impl Iterator<Item = FieldId> {
    PAGES.iter().flat_map(|page| page.fields.iter().copied())
}

/// SmartLED is the only module Rev 2 has, so these never fail — they stay
/// `Option` because every call site already handles `None` and the shape
/// leaves room for a second module type without touching them.
fn smart_led(data: &MenuData) -> Option<&SmartLedSettings> {
    let ModuleSettings::SmartLed(settings) = &data.module;
    Some(settings)
}

fn smart_led_mut(data: &mut MenuData) -> Option<&mut SmartLedSettings> {
    let ModuleSettings::SmartLed(settings) = &mut data.module;
    Some(settings)
}

/// Dotted quad, each octet zero-padded to three digits so the per-digit editor
/// has a fixed layout: `192.168.001.050`.
fn ip_text(ip: &IpAddrMenu) -> String {
    let b = ip.octets();
    format!("{:03}.{:03}.{:03}.{:03}", b[0], b[1], b[2], b[3])
}

fn parse_ip(text: &str) -> Result<IpAddrMenu, &'static str> {
    let mut b = [0u8; 4];
    let mut n = 0;
    for part in text.split('.') {
        if n == 4 {
            return Err("expected a.b.c.d");
        }
        b[n] = part.parse().map_err(|_| "octets must be 0-255")?;
        n += 1;
    }
    if n != 4 {
        return Err("expected a.b.c.d");
    }
    Ok(IpAddrMenu::new(b[0], b[1], b[2], b[3]))
}

/// One Up/Down on digit `digit` (0..=11) of a dotted quad: three digits per
/// octet, wrapping 0..=9 without carry; a result over 255 is rejected.
fn adjust_ip(ip: &mut IpAddrMenu, digit: u8, up: bool) {
    let mut b = ip.octets();
    let octet = (digit / 3) as usize;
    let pow = 10u16.pow((2 - digit % 3) as u32);
    let v = b[octet] as u16;
    let current = (v / pow) % 10;
    let new_digit = if up { (current + 1) % 10 } else { (current + 9) % 10 };
    let new_value = v - current * pow + new_digit * pow;
    if new_value <= 255 {
        b[octet] = new_value as u8;
        *ip = IpAddrMenu::new(b[0], b[1], b[2], b[3]);
    }
}

impl FieldId {
    /// Label shown on the menu (at most `VALUE_COLUMN - 1` characters).
    pub fn label(&self) -> &'static str {
        match self {
            FieldId::DmxAddress => "DMX Address",
            FieldId::InputMode => "Mode",
            // Applies at the next boot; the status line shows what the
            // network is doing right now.
            FieldId::IpMode => "IP Mode (boot)",
            FieldId::IpAddr => "IP",
            FieldId::StaticIp => "Static IP",
            FieldId::StaticPrefix => "Prefix /bits",
            FieldId::StaticGateway => "Gateway",
            FieldId::ArtNetNet => "ArtNet Net",
            FieldId::ArtNetSubNet => "ArtNet Sub-Net",
            FieldId::ArtNetUniverse => "ArtNet Univ",
            FieldId::SacnUniverse => "sACN Universe",
            FieldId::UniversesBound => "Univ Bound",
            FieldId::PortMode => "Port Mode",
            FieldId::ColorMode => "Color Mode",
            // Table lookup rather than a match with a wildcard: the old
            // `GroupSize(_) => "...P4"` arm silently mislabelled every port past
            // the third once the count grew.
            FieldId::GroupSize(i) => GROUP_LABELS[(*i).min(GROUP_LABELS.len() - 1)],
            FieldId::LedsPerPort(i) => LED_LABELS[(*i).min(LED_LABELS.len() - 1)],
            FieldId::UniverseOffsets => "Univ Offsets",
            FieldId::TotalUniverses => "Universes",
            FieldId::EthernetEnabled => "Ethernet(boot)",
            FieldId::LedPower => "LED Power",
            FieldId::Backlight => "Backlight",
        }
    }

    /// Key used by the USB console protocol (`get` / `set <key> <value>`).
    pub fn key(&self) -> &'static str {
        match self {
            FieldId::DmxAddress => "dmx_address",
            FieldId::InputMode => "input_mode",
            FieldId::IpMode => "ip_mode",
            FieldId::IpAddr => "ip",
            FieldId::StaticIp => "static_ip",
            FieldId::StaticPrefix => "static_prefix",
            FieldId::StaticGateway => "static_gateway",
            FieldId::ArtNetNet => "artnet_net",
            FieldId::ArtNetSubNet => "artnet_subnet",
            FieldId::ArtNetUniverse => "artnet_universe",
            FieldId::SacnUniverse => "sacn_universe",
            FieldId::UniversesBound => "universes_bound",
            FieldId::PortMode => "port_mode",
            FieldId::ColorMode => "color_mode",
            // Table lookup, not a wildcard: the old arms gave every port past
            // the third the key `group_size_4` / `leds_4`, so the console could
            // neither address the extra ports nor unambiguously address port 4.
            FieldId::GroupSize(i) => GROUP_KEYS[(*i).min(GROUP_KEYS.len() - 1)],
            FieldId::LedsPerPort(i) => LED_KEYS[(*i).min(LED_KEYS.len() - 1)],
            FieldId::UniverseOffsets => "universe_offsets",
            FieldId::TotalUniverses => "total_universes",
            FieldId::EthernetEnabled => "ethernet_enabled",
            FieldId::LedPower => "led_power",
            FieldId::Backlight => "backlight",
        }
    }

    pub fn editable(&self) -> bool {
        !matches!(
            self,
            FieldId::IpAddr | FieldId::UniverseOffsets | FieldId::TotalUniverses | FieldId::UniversesBound
        )
    }

    /// Number of digit sub-positions for per-digit editing; 0 for values
    /// edited as a whole (enums, bool).
    pub fn digits(&self) -> u8 {
        match self {
            FieldId::DmxAddress | FieldId::ArtNetNet => 3,
            FieldId::GroupSize(_) | FieldId::LedsPerPort(_) => 3,
            FieldId::ArtNetSubNet | FieldId::ArtNetUniverse => 2,
            FieldId::SacnUniverse => 5,
            FieldId::Backlight => 3,
            FieldId::StaticPrefix => 2,
            // Four octets of three digits; the dots are not digits.
            FieldId::StaticIp | FieldId::StaticGateway => 12,
            _ => 0,
        }
    }

    /// Characters the rendered value occupies for a digit-edited field — the
    /// digits plus, for dotted quads, the three dots.
    pub fn display_width(&self) -> usize {
        match self {
            FieldId::StaticIp | FieldId::StaticGateway => 15,
            other => other.digits() as usize,
        }
    }

    /// Display column (relative to the value start) of digit `digit`, so the
    /// edit cursor lands on the digit and not on a dot.
    pub fn cursor_column(&self, digit: u8) -> u16 {
        match self {
            FieldId::StaticIp | FieldId::StaticGateway => (digit + digit / 3) as u16,
            _ => digit as u16,
        }
    }

    fn is_ip(&self) -> bool {
        matches!(self, FieldId::StaticIp | FieldId::StaticGateway)
    }

    fn ip_field<'a>(&self, data: &'a mut MenuData) -> Option<&'a mut IpAddrMenu> {
        match self {
            FieldId::StaticIp => Some(&mut data.static_ip),
            FieldId::StaticGateway => Some(&mut data.static_gateway),
            _ => None,
        }
    }

    /// Valid range for numeric fields.
    fn range(&self) -> (u16, u16) {
        match self {
            FieldId::DmxAddress => (1, DMX_UNIVERSE_SIZE as u16),
            FieldId::ArtNetNet => (0, 127),
            // Art-Net allows sub-net 0..=15, but DMX_BUFFER holds
            // DMX_UNIVERSE_COUNT (64) universes = sub-nets 0..=3. A base
            // beyond that cannot be buffered, so don't let it be configured.
            FieldId::ArtNetSubNet => (0, (crate::DMX_UNIVERSE_COUNT / 16 - 1) as u16),
            FieldId::ArtNetUniverse => (0, 15),
            // E1.31 universes are 1..=63999.
            FieldId::SacnUniverse => (1, 63999),
            // Capped at what the output driver transmits, so a configured
            // count is never silently truncated on the wire.
            FieldId::GroupSize(_) => (1, MAX_LEDS_PER_PORT as u16),
            FieldId::LedsPerPort(_) => (0, MAX_LEDS_PER_PORT as u16),
            FieldId::Backlight => (1, 255),
            FieldId::StaticPrefix => (1, 30),
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
            FieldId::SacnUniverse => Some(data.sacn_universe),
            FieldId::GroupSize(i) => smart_led(data).map(|s| s.dmx_group_size.0[*i]),
            FieldId::LedsPerPort(i) => smart_led(data).map(|s| s.leds_per_port[*i]),
            FieldId::Backlight => Some(data.backlight as u16),
            FieldId::StaticPrefix => Some(data.static_prefix as u16),
            _ => None,
        }
    }

    fn set_u16(&self, data: &mut MenuData, value: u16) {
        match self {
            FieldId::DmxAddress => data.dmx_address = value,
            FieldId::ArtNetNet => data.artnet_address.0[0] = value as u8,
            FieldId::ArtNetSubNet => data.artnet_address.0[1] = value as u8,
            FieldId::ArtNetUniverse => data.artnet_address.0[2] = value as u8,
            FieldId::SacnUniverse => data.sacn_universe = value,
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
            FieldId::Backlight => data.backlight = value.min(255) as u8,
            FieldId::StaticPrefix => data.static_prefix = value.min(30) as u8,
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
            FieldId::StaticIp => ip_text(&data.static_ip),
            FieldId::StaticGateway => ip_text(&data.static_gateway),
            // "<count> @ <base>" in the active mode's own addressing, so what
            // the console operator sees here matches what they patch.
            FieldId::UniversesBound => {
                let n = data.bound_universes();
                match data.input_mode {
                    InputMode::Dmx => String::from("1 (wired DMX)"),
                    InputMode::UsbToDmx => String::from("1 (USB)"),
                    InputMode::ArtNet | InputMode::ArtNetToDmx => {
                        let a = data.artnet_address.0;
                        format!("{} @ {}:{}:{}", n, a[0], a[1], a[2])
                    }
                    InputMode::Sacn => format!("{} @ {}", n, data.sacn_universe),
                }
            }
            FieldId::PortMode => smart_led(data).map_or(String::from("-"), |s| format!("{}", s.port_mode)),
            FieldId::ColorMode => smart_led(data).map_or(String::from("-"), |s| format!("{}", s.color_mode)),
            // Every port, not the first four — the old fixed format silently
            // hid half of them once the port count doubled.
            FieldId::UniverseOffsets => smart_led(data).map_or(String::from("-"), |s| {
                let mut out = String::new();
                for (i, o) in s.universe_offset().iter().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    out.push_str(&format!("{o}"));
                }
                out
            }),
            // Over budget is not an error, it is a silently lower refresh rate,
            // so say so rather than letting someone wonder why 44 Hz became 30.
            FieldId::TotalUniverses => smart_led(data).map_or(String::from("-"), |s| {
                if s.over_budget() {
                    format!("{} OVER", s.total_universes())
                } else {
                    format!("{}/{}", s.total_universes(), crate::DMX_UNIVERSE_COUNT)
                }
            }),
            FieldId::EthernetEnabled => String::from(if data.ethernet_enabled { "On" } else { "Off" }),
            FieldId::LedPower => String::from(match data.led_power {
                LedPower::External => "External",
                LedPower::UsbBrick => "USB brick",
            }),
            _ => String::new(),
        }
    }

    /// Apply one Up/Down press while editing. For numeric fields `digit`
    /// selects the digit (0 = most significant); the digit wraps 0..=9
    /// without carrying, and a result outside the valid range is rejected.
    pub fn adjust(&self, data: &mut MenuData, digit: u8, up: bool) {
        if self.is_ip() {
            if let Some(ip) = self.ip_field(data) {
                adjust_ip(ip, digit, up);
            }
            return;
        }
        if let Some(v) = self.get_u16(data) {
            let (min, max) = self.range();
            let pow = 10u16.pow((self.digits() - 1 - digit) as u32);
            let current = (v / pow) % 10;
            let new_digit = if up { (current + 1) % 10 } else { (current + 9) % 10 };
            // Widened so a 5-digit field's ten-thousands digit cannot overflow
            // u16 on the way to being rejected.
            let new_value = v as u32 - current as u32 * pow as u32 + new_digit as u32 * pow as u32;
            if new_value >= min as u32 && new_value <= max as u32 {
                self.set_u16(data, new_value as u16);
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
            FieldId::LedPower => data.led_power = data.led_power.increment(0),
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
            FieldId::StaticIp => dst.static_ip = src.static_ip,
            FieldId::StaticGateway => dst.static_gateway = src.static_gateway,
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
            FieldId::LedPower => dst.led_power = src.led_power,
            _ => {}
        }
    }

    /// Parse and apply a console-supplied value.
    pub fn set_from_str(&self, data: &mut MenuData, value: &str) -> Result<(), &'static str> {
        if !self.editable() {
            return Err("read-only field");
        }

        if self.is_ip() {
            let ip = parse_ip(value)?;
            if let Some(field) = self.ip_field(data) {
                *field = ip;
            }
            return Ok(());
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
                    "sacn" => InputMode::Sacn,
                    _ => return Err("expected dmx|artnet|artnet>dmx|usb>dmx|sacn"),
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
                if !mode.is_supported() {
                    return Err("colour mode not supported by this firmware");
                }
                smart_led_mut(data).ok_or("module is not SmartLED")?.color_mode = mode;
            }
            FieldId::EthernetEnabled => {
                data.ethernet_enabled = match lower.as_str() {
                    "on" | "true" | "1" => true,
                    "off" | "false" | "0" => false,
                    _ => return Err("expected on|off"),
                };
            }
            FieldId::LedPower => {
                data.led_power = match lower.as_str() {
                    "external" | "ext" | "j26" => LedPower::External,
                    "usb" | "brick" | "usb_brick" | "usbbrick" => LedPower::UsbBrick,
                    _ => return Err("expected external|usb"),
                };
            }
            _ => return Err("cannot set this field"),
        }
        Ok(())
    }
}
