//! Tests for the settings-field metadata table (`nucleo/src/ui/fields.rs`).

use common::ui::{all_fields, FieldId, InputMode, MenuData, ModuleSettings, PAGES};

fn defaults() -> MenuData {
    MenuData::default()
}

fn group_sizes(data: &MenuData) -> [u16; common::SMARTLED_PORT_COUNT] {
    match &data.module {
        ModuleSettings::SmartLed(s) => s.dmx_group_size.0,
        _ => panic!("default module should be SmartLed"),
    }
}

#[test]
fn pages_have_expected_shape() {
    let titles: Vec<&str> = PAGES.iter().map(|p| p.title).collect();
    assert_eq!(
        titles,
        ["Main", "LED", "Group 1-4", "Group 5-8", "LEDs 1-4", "LEDs 5-8", "System"]
    );
    // Every page fits the 11-row TFT layout (tabs + footer take 2 rows)
    for page in PAGES {
        assert!(
            page.fields.len() <= common::ui::fields::MAX_FIELDS_PER_PAGE,
            "page {} has too many rows",
            page.title
        );
    }
}

#[test]
fn console_keys_are_unique() {
    let keys: Vec<&str> = all_fields().map(|f| f.key()).collect();
    let mut deduped = keys.clone();
    deduped.sort_unstable();
    deduped.dedup();
    assert_eq!(keys.len(), deduped.len(), "duplicate console key");
}

#[test]
fn numeric_fields_expose_values_on_default_settings() {
    let data = defaults();
    for field in all_fields().filter(|f| f.digits() > 0) {
        // display() zero-pads numerics to the digit count used by the editor
        assert_eq!(
            field.display(&data).len(),
            field.digits() as usize,
            "field {} display width mismatch",
            field.key()
        );
    }
}

#[test]
fn dmx_address_display_is_zero_padded() {
    let data = defaults(); // dmx_address = 1
    assert_eq!(FieldId::DmxAddress.display(&data), "001");
}

#[test]
fn digit_editing_wraps_without_carry() {
    let mut data = defaults();
    FieldId::DmxAddress.set_from_str(&mut data, "199").unwrap();
    // Incrementing the ones digit of 199 wraps 9 -> 0 without carrying: 190
    FieldId::DmxAddress.adjust(&mut data, 2, true);
    assert_eq!(FieldId::DmxAddress.display(&data), "190");
}

#[test]
fn dmx_address_cannot_leave_valid_range() {
    let mut data = defaults();

    // 1 is the minimum: decrementing the ones digit (-> 0) must be rejected
    FieldId::DmxAddress.adjust(&mut data, 2, false);
    assert_eq!(data.dmx_address, 1);

    // 512 is the maximum: +100 and +1 must both be rejected
    FieldId::DmxAddress.set_from_str(&mut data, "512").unwrap();
    FieldId::DmxAddress.adjust(&mut data, 0, true); // hundreds: 612
    assert_eq!(data.dmx_address, 512);
    FieldId::DmxAddress.adjust(&mut data, 2, true); // ones: 513
    assert_eq!(data.dmx_address, 512);
}

#[test]
fn group_size_cannot_reach_zero() {
    let mut data = defaults(); // group sizes default to 1
    FieldId::GroupSize(0).adjust(&mut data, 2, false); // ones digit down -> 0
    assert_eq!(group_sizes(&data)[0], 1, "group size 0 must be rejected");
}

#[test]
fn artnet_fields_respect_wire_format_limits() {
    let mut data = defaults();

    FieldId::ArtNetNet.set_from_str(&mut data, "127").unwrap();
    FieldId::ArtNetNet.adjust(&mut data, 0, true); // 227 > 127
    assert_eq!(data.artnet_address.0[0], 127);

    assert!(FieldId::ArtNetSubNet.set_from_str(&mut data, "16").is_err());
    assert!(FieldId::ArtNetUniverse.set_from_str(&mut data, "16").is_err());
    FieldId::ArtNetUniverse.set_from_str(&mut data, "15").unwrap();
    assert_eq!(data.artnet_address.0[2], 15);

    // The sub-net is capped below the Art-Net wire maximum: DMX_BUFFER holds
    // DMX_UNIVERSE_COUNT (64) universes = sub-nets 0..=3, and a base past
    // that cannot be buffered — so it must not be configurable.
    let max_subnet = (common::DMX_UNIVERSE_COUNT / 16 - 1) as u16;
    assert!(FieldId::ArtNetSubNet
        .set_from_str(&mut data, "3")
        .is_ok_and(|_| data.artnet_address.0[1] == 3));
    assert!(FieldId::ArtNetSubNet.set_from_str(&mut data, "4").is_err());
    assert_eq!(max_subnet, 3);
}

#[test]
fn enum_fields_cycle_with_up_down() {
    let mut data = defaults();
    assert_eq!(data.input_mode, InputMode::Dmx);

    for expected in [InputMode::ArtNet, InputMode::ArtNetToDmx, InputMode::UsbToDmx, InputMode::Dmx] {
        FieldId::InputMode.adjust(&mut data, 0, true);
        assert_eq!(data.input_mode, expected);
    }
    // and wrap the other way from the first variant
    FieldId::InputMode.adjust(&mut data, 0, false);
    assert_eq!(data.input_mode, InputMode::UsbToDmx);
}

#[test]
fn transfer_merges_only_the_edited_field() {
    let mut committed = defaults();
    let mut draft = committed;

    // Edit the DMX address on the draft while something else changes the
    // committed group size (e.g. a Load event during the edit)
    FieldId::DmxAddress.set_from_str(&mut draft, "100").unwrap();
    FieldId::GroupSize(1).set_from_str(&mut committed, "50").unwrap();

    FieldId::DmxAddress.transfer(&draft, &mut committed);

    assert_eq!(committed.dmx_address, 100, "edited field must transfer");
    assert_eq!(group_sizes(&committed)[1], 50, "concurrent change must survive");
}

#[test]
fn set_from_str_parses_every_editable_kind() {
    let mut data = defaults();

    FieldId::DmxAddress.set_from_str(&mut data, "512").unwrap();
    assert_eq!(data.dmx_address, 512);

    // Enum values are case-insensitive
    FieldId::InputMode.set_from_str(&mut data, "ArtNet>DMX").unwrap();
    assert_eq!(data.input_mode, InputMode::ArtNetToDmx);
    FieldId::InputMode.set_from_str(&mut data, "usb>dmx").unwrap();
    assert_eq!(data.input_mode, InputMode::UsbToDmx);

    FieldId::IpMode.set_from_str(&mut data, "static").unwrap();
    FieldId::PortMode.set_from_str(&mut data, "mirror").unwrap();
    FieldId::ColorMode.set_from_str(&mut data, "rgbw").unwrap();

    FieldId::EthernetEnabled.set_from_str(&mut data, "on").unwrap();
    assert!(data.ethernet_enabled);
    FieldId::EthernetEnabled.set_from_str(&mut data, "0").unwrap();
    assert!(!data.ethernet_enabled);
}

#[test]
fn set_from_str_rejects_invalid_input() {
    let mut data = defaults();

    assert!(FieldId::DmxAddress.set_from_str(&mut data, "0").is_err());
    assert!(FieldId::DmxAddress.set_from_str(&mut data, "513").is_err());
    assert!(FieldId::DmxAddress.set_from_str(&mut data, "abc").is_err());
    assert!(FieldId::InputMode.set_from_str(&mut data, "bogus").is_err());

    // Read-only fields cannot be set from the console
    assert!(FieldId::IpAddr.set_from_str(&mut data, "1.2.3.4").is_err());
    assert!(FieldId::UniverseOffsets.set_from_str(&mut data, "0").is_err());

    // Nothing above may have modified the settings
    assert_eq!(data, defaults());
}


#[test]
fn every_page_fits_the_display() {
    // The menu is paged, not scrolling, so a page that overflows puts fields
    // somewhere the user cannot reach them. 128x64 at 6x8 is 21x8 characters,
    // less one row for the page title.
    for page in PAGES {
        assert!(
            page.fields.len() <= common::ui::fields::MAX_FIELDS_PER_PAGE,
            "page {:?} has {} fields, over the {} the display can show",
            page.title,
            page.fields.len(),
            common::ui::fields::MAX_FIELDS_PER_PAGE
        );
    }
}

#[test]
fn every_port_is_reachable_from_the_menu() {
    // Guards the failure this replaced: raising SMARTLED_PORT_COUNT without
    // extending PAGES left the extra ports configurable over the console but
    // invisible - and worse, sharing a label with port 4.
    let all: Vec<FieldId> = all_fields().collect();
    for port in 0..common::SMARTLED_PORT_COUNT {
        assert!(
            all.contains(&FieldId::GroupSize(port)),
            "port {port} has no group-size field"
        );
        assert!(
            all.contains(&FieldId::LedsPerPort(port)),
            "port {port} has no LED-count field"
        );
    }
}

#[test]
fn per_port_labels_are_distinct() {
    // The old wildcard match arm gave every port past the third the same label.
    let labels: Vec<&str> = (0..common::SMARTLED_PORT_COUNT)
        .map(|p| FieldId::GroupSize(p).label())
        .collect();
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), labels.len(), "duplicate labels: {labels:?}");
}
