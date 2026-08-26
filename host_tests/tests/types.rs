//! Tests for the settings model (`nucleo/src/ui/types.rs`): enum cycling,
//! EEPROM encoding size, and the virtual-LED / universe-offset math the
//! event router relies on.

use common::SMARTLED_PORT_COUNT;
use common::ui::{
    ArtNetAddr, BootStatus, EthernetIPMode, IncDec, InputMode, IpAddrMenu, MenuData,
    ModuleSettings, SmartLedColorMode, SmartLedDmxGroupSize, SmartLedPortMode, SmartLedSettings,
};

/// Must match SETTINGS_SIZE in nucleo/src/eeprom/mod.rs.
const SETTINGS_SIZE: usize = 128;

/// Headroom the worst case must leave inside the slot, so that adding a
/// settings field fails here rather than on a customer's EEPROM.
const REQUIRED_MARGIN: usize = 32;

/// Widen a four-port pattern to whatever `SMARTLED_PORT_COUNT` currently is.
///
/// The interesting cases all live in the first four ports; the rest are padded
/// so these tests do not have to be rewritten every time the port count moves.
/// Group sizes pad with 1, not 0 — a zero divisor would make
/// `virtual_leds_per_port` produce a NaN cast rather than a meaningful result.
fn widen(v: [u16; 4], pad: u16) -> [u16; SMARTLED_PORT_COUNT] {
    core::array::from_fn(|i| if i < 4 { v[i] } else { pad })
}

fn smart_led(leds: [u16; 4], groups: [u16; 4], color: SmartLedColorMode) -> SmartLedSettings {
    SmartLedSettings {
        leds_per_port: widen(leds, 0),
        color_mode: color,
        port_mode: SmartLedPortMode::Individual,
        dmx_group_size: SmartLedDmxGroupSize(widen(groups, 1)),
    }
}

#[test]
fn menu_data_round_trips_through_bincode() {
    let original = MenuData {
        dmx_address: 300,
        input_mode: InputMode::ArtNetToDmx,
        ethernet_ip_mode: EthernetIPMode::Static,
        artnet_address: ArtNetAddr([12, 3, 4]),
        module: ModuleSettings::SmartLed(smart_led([170, 0, 999, 1], [1, 1, 3, 999], SmartLedColorMode::Rgbw)),
        ip_addr: IpAddrMenu::new(192, 168, 1, 50),
        ethernet_enabled: true,
    };

    let mut buffer = [0_u8; SETTINGS_SIZE];
    let length = bincode::encode_into_slice(original, &mut buffer, bincode::config::standard()).unwrap();
    let (decoded, _): (MenuData, usize) = bincode::decode_from_slice(&buffer[..length], bincode::config::standard()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn worst_case_settings_fit_the_eeprom_slot() {
    // bincode varint encoding costs 3 bytes per u16 >= 251, so maximal legal
    // settings are the largest encoding. This is the regression test for the
    // bug where SETTINGS_SIZE = 32 silently dropped saves (BUGS.md N12).
    let worst = MenuData {
        dmx_address: 512,
        input_mode: InputMode::UsbToDmx,
        ethernet_ip_mode: EthernetIPMode::Static,
        artnet_address: ArtNetAddr([127, 15, 15]),
        module: ModuleSettings::SmartLed(smart_led([999; 4], [999; 4], SmartLedColorMode::Rgbw)),
        ip_addr: IpAddrMenu::new(255, 255, 255, 255),
        ethernet_enabled: true,
    };

    let mut buffer = [0_u8; SETTINGS_SIZE];
    let length = bincode::encode_into_slice(worst, &mut buffer, bincode::config::standard())
        .expect("worst-case settings must fit SETTINGS_SIZE");

    // Measured worst case: 40 bytes at SMARTLED_PORT_COUNT = 4, and 64 bytes at
    // 8 ports — which landed exactly on the old 64-byte slot, passing a
    // `<= SETTINGS_SIZE` check with zero room to spare. Assert real margin so
    // the next field added, or the next port-count bump, fails here first.
    assert!(
        length + REQUIRED_MARGIN <= SETTINGS_SIZE,
        "worst-case settings encode to {length} bytes, leaving less than \
         {REQUIRED_MARGIN} bytes of the {SETTINGS_SIZE}-byte slot free; \
         raise SETTINGS_SIZE in nucleo/src/eeprom/mod.rs"
    );
    assert!(length > 32, "if this fails the old 32-byte slot would have been fine");
}

#[test]
fn default_settings_decode_from_blank_eeprom_data() {
    // A fresh EEPROM reads as 0xFF; the firmware falls back to defaults via
    // unwrap_or_default, so decoding failure (not garbage) is the contract.
    let blank = [0xFF_u8; SETTINGS_SIZE];
    let result: Result<(MenuData, usize), _> = bincode::decode_from_slice(&blank, bincode::config::standard());
    assert!(result.is_err());
}

#[test]
fn input_mode_cycles_through_all_variants() {
    let mut mode = InputMode::Dmx;
    let forward = [InputMode::ArtNet, InputMode::ArtNetToDmx, InputMode::UsbToDmx, InputMode::Dmx];
    for expected in forward {
        mode = mode.increment(0);
        assert_eq!(mode, expected);
    }
    // Wraps backwards from the first to the last variant
    assert_eq!(InputMode::Dmx.decrement(0), InputMode::UsbToDmx);
}

#[test]
fn two_state_enums_toggle_both_directions() {
    assert_eq!(EthernetIPMode::Dhcp.increment(0), EthernetIPMode::Static);
    assert_eq!(EthernetIPMode::Static.increment(0), EthernetIPMode::Dhcp);
    assert_eq!(EthernetIPMode::Dhcp.decrement(0), EthernetIPMode::Static);

    assert_eq!(SmartLedPortMode::Individual.increment(0), SmartLedPortMode::Mirror);
    assert_eq!(SmartLedPortMode::Mirror.increment(0), SmartLedPortMode::Individual);

    assert_eq!(SmartLedColorMode::Rgb.increment(0), SmartLedColorMode::Rgbw);
    assert_eq!(SmartLedColorMode::Rgbw.increment(0), SmartLedColorMode::Rgb);
}

#[test]
fn is_dmx_output_matches_the_transmit_modes() {
    assert!(!InputMode::Dmx.is_dmx_output());
    assert!(!InputMode::ArtNet.is_dmx_output());
    assert!(InputMode::ArtNetToDmx.is_dmx_output());
    assert!(InputMode::UsbToDmx.is_dmx_output());
}

#[test]
fn mode_display_strings_match_console_parsing() {
    // The console accepts (lowercased) display strings; keep them in sync
    assert_eq!(format!("{}", InputMode::Dmx), "DMX");
    assert_eq!(format!("{}", InputMode::ArtNet), "ArtNet");
    assert_eq!(format!("{}", InputMode::ArtNetToDmx), "ArtNet>DMX");
    assert_eq!(format!("{}", InputMode::UsbToDmx), "USB>DMX");
}

#[test]
fn virtual_leds_round_up_per_group() {
    let s = smart_led([10, 9, 0, 999], [3, 3, 1, 999], SmartLedColorMode::Rgb);
    // ceil(10/3)=4, 9/3=3, no LEDs -> 0, ceil(999/999)=1; padded ports have no
    // LEDs so they contribute nothing.
    assert_eq!(s.virtual_leds_per_port(), widen([4, 3, 0, 1], 0));
}

#[test]
fn universe_offsets_accumulate_prior_ports() {
    // 170 RGB LEDs = 510 bytes -> exactly one universe per active port
    let s = smart_led([170, 170, 0, 0], [1, 1, 1, 1], SmartLedColorMode::Rgb);
    // Empty ports consume no universes, so the offset stays flat past port 1.
    assert_eq!(s.universe_offset(), widen([0, 1, 2, 2], 2));

    // 171 RGB LEDs = 513 bytes -> spills into a second universe
    let s = smart_led([171, 1, 0, 0], [1, 1, 1, 1], SmartLedColorMode::Rgb);
    assert_eq!(s.universe_offset(), widen([0, 2, 3, 3], 3));
}

#[test]
fn artnet_addr_sub_uni_indexes_one_full_net() {
    // Buffer index = sub-net (high nibble) : universe (low nibble)
    assert_eq!(ArtNetAddr([0, 0, 0]).sub_uni(), 0);
    assert_eq!(ArtNetAddr([7, 3, 5]).sub_uni(), 0x35);
    assert_eq!(ArtNetAddr([127, 15, 15]).sub_uni(), 255);
    // The net byte plays no part, and corrupt (out-of-range) stored values
    // are masked to 4 bits so the index can never exceed 255
    assert_eq!(ArtNetAddr([200, 255, 255]).sub_uni(), 255);
}

#[test]
fn artnet_addr_buffer_base_is_clamped_to_the_buffer() {
    // sub_uni() can reach 255, but DMX_BUFFER holds DMX_UNIVERSE_COUNT
    // universes. buffer_base() is what buffer indexing must use — the raw
    // value sliced the USB forwarder out of bounds (a panic) on a configured
    // sub-net past the buffer.
    use common::DMX_UNIVERSE_COUNT;
    assert_eq!(ArtNetAddr([0, 0, 0]).buffer_base(), 0);
    assert_eq!(ArtNetAddr([0, 3, 15]).buffer_base(), DMX_UNIVERSE_COUNT - 1);
    assert_eq!(ArtNetAddr([0, 15, 15]).buffer_base(), DMX_UNIVERSE_COUNT - 1);
}

#[test]
fn boot_status_defaults_to_failed() {
    // The ethernet-lockout logic depends on unknown boot flags reading as
    // Failed (fail safe)
    assert_eq!(BootStatus::default(), BootStatus::Failed);
}

/// Every port set to the same value, for whole-board budget cases.
fn all(leds: u16, group: u16, color: SmartLedColorMode) -> SmartLedSettings {
    SmartLedSettings {
        leds_per_port: [leds; SMARTLED_PORT_COUNT],
        color_mode: color,
        port_mode: SmartLedPortMode::Individual,
        dmx_group_size: SmartLedDmxGroupSize([group; SMARTLED_PORT_COUNT]),
    }
}

#[test]
fn rgb_600_is_the_budget() {
    let s = all(600, 1, SmartLedColorMode::Rgb);
    assert_eq!(s.bytes_per_port()[0], common::MAX_BYTES_PER_PORT);
    // Ports start on universe boundaries: ceil(1800/512) = 4 each, not 29 total.
    assert_eq!(s.universes_per_port()[0], 4);
    assert_eq!(s.total_universes(), 4 * SMARTLED_PORT_COUNT as u16);
    assert!(!s.over_budget());
}

#[test]
fn rgbw_450_matches_rgb_600_exactly() {
    // The claim the whole budget framing rests on: colour mode and LED count
    // are two ways of spending one budget, so RGBW needs a shorter string, not
    // a different design.
    let rgb = all(600, 1, SmartLedColorMode::Rgb);
    let rgbw = all(450, 1, SmartLedColorMode::Rgbw);
    assert_eq!(rgb.bytes_per_port(), rgbw.bytes_per_port());
    assert_eq!(rgb.universes_per_port(), rgbw.universes_per_port());
    assert_eq!(rgb.total_universes(), rgbw.total_universes());
    assert!(!rgbw.over_budget());
}

#[test]
fn rgbw_at_full_rgb_length_is_over_budget() {
    // 600 RGBW is 2400 B/port - a 24 ms frame, which is a 42 Hz ceiling and
    // already below the 44 Hz Art-Net rate.
    let s = all(600, 1, SmartLedColorMode::Rgbw);
    assert!(s.bytes_per_port()[0] > common::MAX_BYTES_PER_PORT);
    assert!(s.ports_over_budget());
    assert!(s.over_budget());
}

#[test]
fn grouping_reduces_the_budget_cost() {
    // Grouping collapses physical LEDs into virtual ones, so the wire cost
    // falls even though the string is the same length.
    let ungrouped = all(600, 1, SmartLedColorMode::Rgb);
    let grouped = all(600, 4, SmartLedColorMode::Rgb);
    assert_eq!(grouped.bytes_per_port()[0], ungrouped.bytes_per_port()[0] / 4);
    assert!(!grouped.over_budget());
}

#[test]
fn total_universes_stays_inside_the_buffer() {
    // DMX_BUFFER only covers DMX_UNIVERSE_COUNT universes; the Art-Net task
    // drops anything above it, so a configuration that needs more is flagged.
    let s = all(600, 1, SmartLedColorMode::Rgb);
    assert!(s.total_universes() as usize <= common::DMX_UNIVERSE_COUNT);
}
