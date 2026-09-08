//! The DMX-address convention, shared by every input mode.
//!
//! Regression cover for N18 from the 2026-07 review: wired DMX and USB store a
//! frame DMX-style (start code at index 0, channel N at index N) while Art-Net
//! and sACN store channel 1 at slot offset 0. Indexing both with the raw
//! configured address made the same "DMX Address" select a different channel
//! depending on where the data came from.

use common::event_router::buffer_index;
use common::ui::InputMode;

/// Index of the byte holding channel `ch` for a given mode, as the router
/// computes it: the address maps to the first channel the fixture consumes.
fn channel_byte(mode: InputMode, address: u16, ch_offset: usize) -> usize {
    buffer_index(address, mode.is_network()) + ch_offset
}

#[test]
fn wired_dmx_indexes_past_the_start_code() {
    // DMX-style storage: index 0 is the start code, so address 1 is index 1.
    assert_eq!(buffer_index(1, false), 1);
    assert_eq!(buffer_index(512, false), 512);
}

#[test]
fn network_modes_are_zero_based() {
    // Art-Net and sACN carry no start code: address 1 is the first slot.
    assert_eq!(buffer_index(1, true), 0);
    assert_eq!(buffer_index(512, true), 511);
}

#[test]
fn the_same_address_selects_the_same_channel_in_every_mode() {
    // The point of N18: a fixture patched at a given address must light from
    // the same channel whichever input is feeding the box. The two families
    // land on different *buffer* indices precisely so they refer to the same
    // *channel*.
    for address in [1u16, 2, 17, 200, 512] {
        let wired = channel_byte(InputMode::Dmx, address, 0);
        let usb = channel_byte(InputMode::UsbToDmx, address, 0);
        let artnet = channel_byte(InputMode::ArtNet, address, 0);
        let sacn = channel_byte(InputMode::Sacn, address, 0);

        assert_eq!(wired, usb, "wired and USB must agree at address {address}");
        assert_eq!(artnet, sacn, "Art-Net and sACN must agree at address {address}");
        assert_eq!(
            wired,
            artnet + 1,
            "network storage is one lower than DMX storage at address {address}"
        );
    }
}

#[test]
fn a_three_channel_fixture_reads_consecutive_bytes() {
    // An RGB pixel at address 10 takes the 10th, 11th and 12th channels.
    let base = channel_byte(InputMode::ArtNet, 10, 0);
    assert_eq!(
        [base, base + 1, base + 2],
        [9, 10, 11],
        "Art-Net RGB at address 10 should occupy slots 9..=11"
    );
    let wired = channel_byte(InputMode::Dmx, 10, 0);
    assert_eq!([wired, wired + 1, wired + 2], [10, 11, 12]);
}

#[test]
fn address_zero_does_not_underflow() {
    // The menu clamps to 1, but an old EEPROM or a console `set` can still
    // present 0 — it must pin to the start of the slot, not wrap.
    assert_eq!(buffer_index(0, true), 0);
    assert_eq!(buffer_index(0, false), 0);
}

#[test]
fn every_network_mode_is_classified_as_network() {
    assert!(InputMode::ArtNet.is_network());
    assert!(InputMode::Sacn.is_network());
    assert!(InputMode::ArtNetToDmx.is_network());
    assert!(!InputMode::Dmx.is_network());
    assert!(!InputMode::UsbToDmx.is_network());
}
