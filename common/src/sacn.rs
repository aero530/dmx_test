//! sACN / E1.31 data packet parsing.
//!
//! The streaming-ACN alternative to Art-Net. Two things make it worth carrying
//! alongside Art-Net rather than instead of it:
//!
//! * It is **multicast**, one group per universe (`239.255.<hi>.<lo>`). With an
//!   IGMP-snooping switch the filtering happens *upstream* — the switch never
//!   forwards universes this node did not join, so they never reach the W6300's
//!   SPI link at all. That is a better answer to the ingest problem than
//!   unicast Art-Net, which still has to arrive before it can be discarded.
//! * It carries an explicit **priority** and a stream-termination flag, so
//!   multiple sources and clean hand-off are part of the protocol rather than
//!   convention.
//!
//! Only the *data* packet (`VECTOR_E131_DATA_PACKET`) is parsed. Synchronisation
//! and discovery packets are recognised well enough to be ignored.

/// UDP port for sACN, fixed by E1.31.
pub const PORT: u16 = 5568;

/// ACN packet identifier: "ASC-E1.17" followed by three NULs.
const ACN_PID: [u8; 12] = [
    0x41, 0x53, 0x43, 0x2d, 0x45, 0x31, 0x2e, 0x31, 0x37, 0x00, 0x00, 0x00,
];

const VECTOR_ROOT_DATA: u32 = 0x0000_0004;
const VECTOR_FRAMING_DATA: u32 = 0x0000_0002;
const VECTOR_DMP_SET_PROPERTY: u8 = 0x02;

/// Byte offsets into an E1.31 data packet. Laid out explicitly because the
/// three nested layers make it easy to be off by a field.
mod offset {
    pub const PREAMBLE_SIZE: usize = 0; // u16, must be 0x0010
    pub const ACN_PID: usize = 4; // 12 bytes
    pub const ROOT_VECTOR: usize = 18; // u32
    pub const FRAMING_VECTOR: usize = 40; // u32
    pub const PRIORITY: usize = 108;
    pub const SEQUENCE: usize = 111;
    pub const OPTIONS: usize = 112;
    pub const UNIVERSE: usize = 113; // u16
    pub const DMP_VECTOR: usize = 117;
    pub const PROPERTY_COUNT: usize = 123; // u16, start code + slots
    pub const START_CODE: usize = 125;
    pub const VALUES: usize = 126;
}

/// Options bit 7 — the packet is a preview, not for live output.
const OPTION_PREVIEW: u8 = 0b1000_0000;
/// Options bit 6 — the source is ending this stream.
const OPTION_TERMINATED: u8 = 0b0100_0000;

/// A parsed E1.31 DMX data packet.
pub struct DataPacket<'a> {
    pub universe: u16,
    pub sequence: u8,
    /// 0..=200, higher wins when several sources drive one universe.
    pub priority: u8,
    /// Preview data — should not drive real output.
    pub preview: bool,
    /// The source is ending this stream; hold or release, do not treat as data.
    pub terminated: bool,
    /// DMX slots, **excluding** the start code.
    pub values: &'a [u8],
}

/// Multicast group for a universe: `239.255.<hi>.<lo>`.
pub fn multicast_group(universe: u16) -> [u8; 4] {
    [239, 255, (universe >> 8) as u8, universe as u8]
}

/// Parse a datagram as an E1.31 data packet.
///
/// Returns `None` for anything that is not one — a short buffer, a foreign
/// protocol on the port, a synchronisation or discovery packet, or a data
/// packet whose start code is not DMX. Every field read is bounds-checked
/// against the actual datagram length rather than the length the packet claims,
/// because the claimed length is attacker-controlled.
pub fn parse(buf: &[u8]) -> Option<DataPacket<'_>> {
    if buf.len() < offset::VALUES {
        return None;
    }
    if u16::from_be_bytes([buf[offset::PREAMBLE_SIZE], buf[offset::PREAMBLE_SIZE + 1]]) != 0x0010 {
        return None;
    }
    if buf[offset::ACN_PID..offset::ACN_PID + 12] != ACN_PID {
        return None;
    }
    if be32(buf, offset::ROOT_VECTOR) != VECTOR_ROOT_DATA {
        return None;
    }
    if be32(buf, offset::FRAMING_VECTOR) != VECTOR_FRAMING_DATA {
        return None;
    }
    if buf[offset::DMP_VECTOR] != VECTOR_DMP_SET_PROPERTY {
        return None;
    }
    // Start code 0 is DMX; anything else is a different slot type (RDM, text)
    // and must not be written into the DMX buffer.
    if buf[offset::START_CODE] != 0x00 {
        return None;
    }

    // Property count includes the start code, so slots is one fewer. Clamp to
    // what actually arrived: a packet may claim more than it carries.
    let claimed = u16::from_be_bytes([buf[offset::PROPERTY_COUNT], buf[offset::PROPERTY_COUNT + 1]]);
    let slots = (claimed.saturating_sub(1) as usize).min(buf.len() - offset::VALUES);

    let options = buf[offset::OPTIONS];
    Some(DataPacket {
        universe: u16::from_be_bytes([buf[offset::UNIVERSE], buf[offset::UNIVERSE + 1]]),
        sequence: buf[offset::SEQUENCE],
        priority: buf[offset::PRIORITY],
        preview: options & OPTION_PREVIEW != 0,
        terminated: options & OPTION_TERMINATED != 0,
        values: &buf[offset::VALUES..offset::VALUES + slots],
    })
}

fn be32(buf: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]])
}
