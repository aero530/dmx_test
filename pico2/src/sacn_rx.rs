//! sACN / E1.31 receiver.
//!
//! Runs alongside the Art-Net task and writes into the same `DMX_BUFFER`, so
//! the router and everything downstream are unchanged.
//!
//! # Why multicast is the point
//!
//! Art-Net's ingest problem is that broadcast traffic arrives whether it is
//! wanted or not — a console pushing 32 universes costs SPI bandwidth even if
//! only four are ours. sACN puts each universe in its own multicast group, so
//! joining only the ones in use lets an **IGMP-snooping switch drop the rest
//! upstream**. They never reach the W6300 at all. That is a better answer than
//! unicast Art-Net, which still has to arrive before it can be discarded.
//!
//! # Universe numbering
//!
//! sACN universes are 1-based; `DMX_BUFFER` is indexed from 0. Universe *U*
//! therefore lands at slot *U − 1*, and anything past the buffer is dropped —
//! the same bounds discipline the Art-Net path needs, for the same reason: the
//! universe number comes off the wire.
//!
//! # Source selection
//!
//! Gated on the same network input modes as Art-Net. A dedicated
//! `InputMode::Sacn`, and the source arbitration that sACN's `priority` field
//! makes possible, would change the encoded shape of `MenuData` — a schema
//! change, and the first real exercise of the version byte at EEPROM `0x11`.
//! Deferred deliberately rather than bolted on here.

use common::channels::{DmxChannelTx, DmxFeedbackChannelRx, RouterChannelTx};
use common::event_router::{DmxEvent, DmxFeedbackEvent, PacketAddress};
use common::artnet::PortAddress;
use common::sacn::{multicast_group, parse, PORT};
use common::ui::InputMode;
use common::{DMX_BUFFER, DMX_UNIVERSE_COUNT, DMX_UNIVERSE_SIZE};
use defmt::*;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpAddress, Stack};

/// Universes joined at startup.
///
/// Sized to what eight ports can actually consume — 8 x 1800 B is 32 universes
/// — with headroom to the buffer. Joining every possible group would defeat the
/// purpose, since the switch would then forward everything.
const JOIN_COUNT: u16 = 32;

#[embassy_executor::task]
pub async fn sacn_task(
    stack: Stack<'static>,
    tx: DmxChannelTx,
    _tx_router: RouterChannelTx,
    mut rx: DmxFeedbackChannelRx,
) {
    stack.wait_config_up().await;

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buf = [0u8; 1536];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_buf = [0u8; 64];
    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buf,
        &mut tx_meta,
        &mut tx_buf,
    );

    if socket.bind(PORT).is_err() {
        error!("sACN: could not bind port {}", PORT);
        return;
    }

    // Joining is what tells an IGMP-snooping switch to start forwarding these
    // groups — and, just as importantly, not the others.
    let mut joined = 0u16;
    for universe in 1..=JOIN_COUNT {
        let g = multicast_group(universe);
        if stack
            .join_multicast_group(IpAddress::v4(g[0], g[1], g[2], g[3]))
            .is_ok()
        {
            joined += 1;
        }
    }
    info!("sACN: listening on {}, joined {} groups", PORT, joined);

    let mut input_mode = InputMode::default();
    let mut buf = [0u8; 1536];
    let mut warned_out_of_range = false;

    loop {
        if let Some(DmxFeedbackEvent::Mode(new_mode, _addr)) = rx.try_changed() {
            input_mode = new_mode;
        }

        let Ok((n, _from)) = socket.recv_from(&mut buf).await else {
            continue;
        };

        // Same guard as the Art-Net path: only store when a network mode is
        // selected, or stray traffic overwrites wired DMX / USB data.
        if !matches!(input_mode, InputMode::ArtNet | InputMode::ArtNetToDmx) {
            continue;
        }

        let Some(packet) = parse(&buf[..n]) else {
            continue;
        };
        // Preview data is explicitly not for live output; a terminated stream
        // is an announcement, not channel values.
        if packet.preview || packet.terminated {
            continue;
        }

        let Some(slot) = packet.universe.checked_sub(1) else {
            continue; // universe 0 is not valid in E1.31
        };
        if slot as usize >= DMX_UNIVERSE_COUNT {
            if !warned_out_of_range {
                warn!(
                    "sACN: ignoring universe {} - beyond the {} buffered",
                    packet.universe, DMX_UNIVERSE_COUNT
                );
                warned_out_of_range = true;
            }
            continue;
        }

        let start = slot as usize * DMX_UNIVERSE_SIZE;
        let len = packet.values.len().min(DMX_UNIVERSE_SIZE);
        {
            let mut dmx_buffer = DMX_BUFFER.lock().await;
            dmx_buffer[start..start + len].copy_from_slice(&packet.values[..len]);
        }

        let _ = tx.try_send(DmxEvent::ArtNetPacket(PacketAddress::new(
            PortAddress::new(0, 0, slot as u8),
            packet.sequence,
        )));
    }
}
