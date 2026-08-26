//! E1.31 parsing. The header is three nested layers deep, so these mostly guard
//! against being off by a field — and against trusting a length that came off
//! the wire.

use common::sacn::{self, multicast_group, parse};

/// Build a well-formed data packet carrying `slots` DMX channels.
fn packet(universe: u16, slots: &[u8]) -> Vec<u8> {
    let mut p = vec![0u8; 126 + slots.len()];
    p[0..2].copy_from_slice(&0x0010u16.to_be_bytes()); // preamble
    p[4..16].copy_from_slice(b"ASC-E1.17\0\0\0"); // ACN PID
    p[18..22].copy_from_slice(&4u32.to_be_bytes()); // root vector
    p[40..44].copy_from_slice(&2u32.to_be_bytes()); // framing vector
    p[108] = 100; // priority
    p[111] = 7; // sequence
    p[113..115].copy_from_slice(&universe.to_be_bytes());
    p[117] = 0x02; // DMP vector
    p[123..125].copy_from_slice(&((slots.len() as u16) + 1).to_be_bytes());
    p[125] = 0x00; // start code = DMX
    p[126..].copy_from_slice(slots);
    p
}

#[test]
fn parses_a_data_packet() {
    let p = packet(42, &[1, 2, 3, 4]);
    let d = parse(&p).expect("valid packet");
    assert_eq!(d.universe, 42);
    assert_eq!(d.sequence, 7);
    assert_eq!(d.priority, 100);
    assert_eq!(d.values, &[1, 2, 3, 4]);
    assert!(!d.preview);
    assert!(!d.terminated);
}

#[test]
fn rejects_foreign_traffic() {
    assert!(parse(&[]).is_none());
    assert!(parse(&[0u8; 125]).is_none(), "short of the header");
    let mut p = packet(1, &[0; 4]);
    p[4] = b'X'; // corrupt the ACN identifier
    assert!(parse(&p).is_none());
}

#[test]
fn rejects_non_dmx_start_codes() {
    // Start code 0xCC is RDM; writing it into the DMX buffer would corrupt a
    // universe with data that is not channel values.
    let mut p = packet(1, &[9; 4]);
    p[125] = 0xCC;
    assert!(parse(&p).is_none());
}

#[test]
fn a_lying_length_cannot_read_past_the_datagram() {
    // The property count is attacker-controlled. Claim 512 slots in a packet
    // that carries 4; the parser must clamp to what arrived rather than slice
    // past the end.
    let mut p = packet(1, &[1, 2, 3, 4]);
    p[123..125].copy_from_slice(&513u16.to_be_bytes());
    let d = parse(&p).expect("still a valid packet, just an overstated length");
    assert_eq!(d.values.len(), 4);
}

#[test]
fn options_flags_are_decoded() {
    let mut p = packet(1, &[0; 4]);
    p[112] = 0b1100_0000;
    let d = parse(&p).unwrap();
    assert!(d.preview);
    assert!(d.terminated);
}

#[test]
fn multicast_groups_follow_the_universe() {
    assert_eq!(multicast_group(1), [239, 255, 0, 1]);
    assert_eq!(multicast_group(256), [239, 255, 1, 0]);
    assert_eq!(multicast_group(0x1234), [239, 255, 0x12, 0x34]);
    assert_eq!(sacn::PORT, 5568);
}
