//! Transport-agnostic half of the Enttec DMX USB Pro widget.
//!
//! Two ports speak the Enttec protocol and share this code:
//!
//! * the module's native USB, as a CDC-ACM serial port (`usb_device.rs`);
//! * the carrier's FT232RNL USB-C port over UART0 (`enttec_uart.rs`) — the one
//!   that FTDI-only lighting software (QLC+, D2XX apps) recognises as a
//!   genuine Pro, because it *is* FTDI silicon.
//!
//! Both hand complete host messages to [`handle_message`] and run a
//! [`ChangeForwarder`] for the widget-to-host "Received DMX" stream; they differ
//! only in how bytes move.
//!
//! Message framing, both directions: `0x7E, label, length LSB, length MSB,
//! payload, 0xE7` (see `common::enttec_protocol`).

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "usb")] {
        use log::{info, warn};
    } else {
        use defmt::{info, warn};
    }
}

use common::artnet::PortAddress;
use common::channels::DmxChannelTx;
use common::enttec_protocol::{
    BREAK_TIME, END_DELIMITER, FIRMWARE_VERSION_LSB, FIRMWARE_VERSION_MSB, LABEL_GET_PARAMS,
    LABEL_GET_SERIAL, LABEL_OUTPUT_DMX, LABEL_RECEIVED_DMX, LABEL_RECEIVE_ON_CHANGE,
    LABEL_SET_PARAMS, MAB_TIME, MAX_PAYLOAD, REFRESH_RATE, SERIAL_NUMBER, START_DELIMITER,
};
use common::event_router::{DmxEvent, PacketAddress};
use common::ui::InputMode;
use common::{DMX_BUFFER, DMX_BUFF_SIZE, DMX_UNIVERSE_SIZE};

/// Largest framed message in either direction. Label 5 is the biggest: status
/// byte + start code + 512 channels inside the 4-byte header and end delimiter.
pub const MSG_MAX: usize = 4 + 1 + DMX_BUFF_SIZE + 1;

/// Frame `payload` as an Enttec message into `out`; returns the byte count.
pub fn frame_message(label: u8, payload: &[u8], out: &mut [u8]) -> usize {
    let n = payload.len();
    out[0] = START_DELIMITER;
    out[1] = label;
    out[2] = (n & 0xFF) as u8;
    out[3] = (n >> 8) as u8;
    out[4..4 + n].copy_from_slice(payload);
    out[4 + n] = END_DELIMITER;
    5 + n
}

/// Handle one complete message from the host.
///
/// Returns `Some(n)` when `reply[..n]` holds a framed message the transport
/// must send back; `None` when there is nothing to answer.
pub async fn handle_message(
    label: u8,
    payload: &[u8],
    input_mode: InputMode,
    tx: &DmxChannelTx,
    reply: &mut [u8; MSG_MAX],
) -> Option<usize> {
    match label {
        LABEL_OUTPUT_DMX => {
            // Only accept host DMX in USB>DMX mode so a connected PC can't
            // clobber live wired-DMX or Art-Net data in the other modes.
            if input_mode == InputMode::UsbToDmx && !payload.is_empty() {
                let n = payload.len().min(DMX_BUFF_SIZE);
                {
                    let mut dmx_buffer = DMX_BUFFER.lock().await;
                    dmx_buffer[0..n].copy_from_slice(&payload[..n]);
                    // Channels beyond what the host sent go dark, matching a
                    // widget that transmits exactly the received universe.
                    dmx_buffer[n..DMX_BUFF_SIZE].fill(0);
                }
                let _ = tx.try_send(DmxEvent::DmxPacket(PacketAddress::new(PortAddress::new(0, 0, 0), 0)));
            }
            None
        }
        LABEL_GET_PARAMS => {
            // Reply: firmware version, break, MAB, refresh rate, then the
            // requested amount of user configuration data (all zeroes).
            let user_size = if payload.len() >= 2 {
                u16::from_le_bytes([payload[0], payload[1]]) as usize
            } else {
                0
            }
            .min(MAX_PAYLOAD - 5);

            let mut body = [0_u8; MAX_PAYLOAD];
            body[..5].copy_from_slice(&[FIRMWARE_VERSION_LSB, FIRMWARE_VERSION_MSB, BREAK_TIME, MAB_TIME, REFRESH_RATE]);
            Some(frame_message(LABEL_GET_PARAMS, &body[..5 + user_size], reply))
        }
        LABEL_GET_SERIAL => Some(frame_message(LABEL_GET_SERIAL, &SERIAL_NUMBER, reply)),
        LABEL_SET_PARAMS => {
            // Output timing is fixed by the PIO program.
            info!("Enttec: set-params accepted (timing is fixed in the PIO program)");
            None
        }
        LABEL_RECEIVE_ON_CHANGE => {
            // Received DMX is always forwarded on change.
            info!("Enttec: receive-on-change request accepted");
            None
        }
        other => {
            warn!("Enttec: unsupported label {}", other);
            None
        }
    }
}

/// The widget-to-host "Received DMX Packet" (label 5) stream: forwards the
/// active input source's universe whenever it changes. One instance per port,
/// since each host sees its own change history.
pub struct ChangeForwarder {
    /// Last frame forwarded, DMX-style (start code at 0).
    forwarded: [u8; DMX_BUFF_SIZE],
    /// Buffer index (clamped sub-net:universe) of the configured Art-Net address.
    artnet_base: usize,
}

impl ChangeForwarder {
    pub const fn new() -> Self {
        Self {
            forwarded: [0; DMX_BUFF_SIZE],
            artnet_base: 0,
        }
    }

    /// Track the configured Art-Net address. Pass `ArtNetAddr::buffer_base()`,
    /// never the raw sub-net:universe — the raw value can reach 255 while the
    /// buffer holds 64 universes, and the slice below would panic.
    pub fn set_artnet_base(&mut self, base: usize) {
        self.artnet_base = base;
    }

    /// Snapshot the active source; if it differs from what the host last got,
    /// frame a label-5 message into `out` and return its length.
    pub async fn poll(&mut self, input_mode: InputMode, out: &mut [u8; MSG_MAX]) -> Option<usize> {
        let mut current = [0_u8; DMX_BUFF_SIZE];
        match input_mode {
            InputMode::Dmx => {
                let dmx_buffer = DMX_BUFFER.lock().await;
                current.copy_from_slice(&dmx_buffer[0..DMX_BUFF_SIZE]);
            }
            // The host is the data source; nothing to forward.
            InputMode::UsbToDmx => return None,
            // Network modes: the configured Art-Net universe, or sACN's base
            // universe (which the receive task rebases onto slot 0).
            network => {
                let start = if network.is_artnet() { self.artnet_base * DMX_UNIVERSE_SIZE } else { 0 };
                let dmx_buffer = DMX_BUFFER.lock().await;
                current[0] = 0x00;
                current[1..].copy_from_slice(&dmx_buffer[start..start + DMX_UNIVERSE_SIZE]);
            }
        }

        if current == self.forwarded {
            return None;
        }
        self.forwarded = current;
        // Payload: status (0 = no errors) + start code + channels.
        let mut payload = [0_u8; 1 + DMX_BUFF_SIZE];
        payload[1..].copy_from_slice(&current);
        Some(frame_message(LABEL_RECEIVED_DMX, &payload, out))
    }
}

impl Default for ChangeForwarder {
    fn default() -> Self {
        Self::new()
    }
}
