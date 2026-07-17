//! Tests for the settings model (`nucleo/src/ui/types.rs`): enum cycling,
//! EEPROM encoding size, and the virtual-LED / universe-offset math the
//! event router relies on.

use host_tests::ui::{
    ArtNetAddr, BootStatus, EthernetIPMode, IncDec, InputMode, IpAddrMenu, MenuData,
    ModuleSettings, SmartLedColorMode, SmartLedDmxGroupSize, SmartLedPortMode, SmartLedSettings,
};

/// Must match SETTINGS_SIZE in nucleo/src/eeprom/mod.rs.
const SETTINGS_SIZE: usize = 64;

fn smart_led(leds: [u16; 4], groups: [u16; 4], color: SmartLedColorMode) -> SmartLedSettings {
    SmartLedSettings {
        leds_per_port: leds,
        color_mode: color,
        port_mode: SmartLedPortMode::Individual,
        dmx_group_size: SmartLedDmxGroupSize(groups),
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
    assert!(length <= SETTINGS_SIZE);
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
    // ceil(10/3)=4, 9/3=3, no LEDs -> 0, ceil(999/999)=1
    assert_eq!(s.virtual_leds_per_port(), [4, 3, 0, 1]);
}

#[test]
fn universe_offsets_accumulate_prior_ports() {
    // 170 RGB LEDs = 510 bytes -> exactly one universe per active port
    let s = smart_led([170, 170, 0, 0], [1, 1, 1, 1], SmartLedColorMode::Rgb);
    assert_eq!(s.universe_offset(), [0, 1, 2, 2]);

    // 171 RGB LEDs = 513 bytes -> spills into a second universe
    let s = smart_led([171, 1, 0, 0], [1, 1, 1, 1], SmartLedColorMode::Rgb);
    assert_eq!(s.universe_offset(), [0, 2, 3, 3]);
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
fn boot_status_defaults_to_failed() {
    // The ethernet-lockout logic depends on unknown boot flags reading as
    // Failed (fail safe)
    assert_eq!(BootStatus::default(), BootStatus::Failed);
}
