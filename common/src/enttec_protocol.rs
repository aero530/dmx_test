//! Enttec DMX USB Pro protocol: message framing, labels and widget constants.
//!
//! Pure logic with no hardware or executor dependencies so it can be unit
//! tested on the host (see `host_tests/`). The USB transport that uses this
//! parser lives in `usb_device.rs`.
//!
//! Wire format of every message, in both directions:
//!
//! ```text
//! 0x7E | label | length LSB | length MSB | payload (length bytes) | 0xE7
//! ```

use crate::DMX_BUFF_SIZE;

/// First byte of every message.
pub const START_DELIMITER: u8 = 0x7E;
/// Last byte of every message.
pub const END_DELIMITER: u8 = 0xE7;

/// Get Widget Parameters request / reply.
pub const LABEL_GET_PARAMS: u8 = 3;
/// Set Widget Parameters (accepted but ignored; output timing is fixed).
pub const LABEL_SET_PARAMS: u8 = 4;
/// Received DMX Packet (widget -> host).
pub const LABEL_RECEIVED_DMX: u8 = 5;
/// Output Only Send DMX Packet (host -> widget).
pub const LABEL_OUTPUT_DMX: u8 = 6;
/// Receive DMX on Change mode select (accepted; we always send on change).
pub const LABEL_RECEIVE_ON_CHANGE: u8 = 8;
/// Get Widget Serial Number request / reply.
pub const LABEL_GET_SERIAL: u8 = 10;

/// Largest meaningful payload: start code + 512 channels (label 6).
pub const MAX_PAYLOAD: usize = DMX_BUFF_SIZE;

/// Widget parameters reported to the host. Break/MAB are in 10.67us units
/// and describe the actual timing of the bridge's PIO output program
/// (176us break, 16us MAB, ~43 packet/s refresh capped at the 40 the
/// protocol can express).
pub const FIRMWARE_VERSION_LSB: u8 = 44; // v1.44
pub const FIRMWARE_VERSION_MSB: u8 = 1;
pub const BREAK_TIME: u8 = 16;
pub const MAB_TIME: u8 = 2;
pub const REFRESH_RATE: u8 = 40;
/// BCD-coded widget serial number returned for label 10.
pub const SERIAL_NUMBER: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

enum ParseState {
    WaitForStart,
    Label,
    LengthLsb,
    LengthMsb,
    Data,
    WaitForEnd,
}

/// Incremental parser for the Enttec message framing.
///
/// Feed the byte stream in with [`feed`](Self::feed); when it returns `true`
/// a complete message is available through [`label`](Self::label) and
/// [`payload`](Self::payload). Messages whose declared length exceeds
/// [`MAX_PAYLOAD`] are consumed and dropped so the stream stays in sync, and
/// a missing end delimiter makes the parser hunt for the next start
/// delimiter instead of returning a corrupt message.
pub struct EnttecParser {
    state: ParseState,
    label: u8,
    length: usize,
    pos: usize,
    oversize: bool,
    payload: [u8; MAX_PAYLOAD],
}

impl EnttecParser {
    pub const fn new() -> Self {
        Self {
            state: ParseState::WaitForStart,
            label: 0,
            length: 0,
            pos: 0,
            oversize: false,
            payload: [0; MAX_PAYLOAD],
        }
    }

    /// Label of the most recently completed message.
    pub fn label(&self) -> u8 {
        self.label
    }

    /// Payload of the most recently completed message.
    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.length.min(MAX_PAYLOAD)]
    }

    /// Feed one byte; returns true when a complete valid message is available.
    pub fn feed(&mut self, byte: u8) -> bool {
        match self.state {
            ParseState::WaitForStart => {
                if byte == START_DELIMITER {
                    self.state = ParseState::Label;
                }
            }
            ParseState::Label => {
                self.label = byte;
                self.state = ParseState::LengthLsb;
            }
            ParseState::LengthLsb => {
                self.length = byte as usize;
                self.state = ParseState::LengthMsb;
            }
            ParseState::LengthMsb => {
                self.length |= (byte as usize) << 8;
                self.pos = 0;
                self.oversize = self.length > MAX_PAYLOAD;
                self.state = if self.length == 0 { ParseState::WaitForEnd } else { ParseState::Data };
            }
            ParseState::Data => {
                if !self.oversize {
                    self.payload[self.pos] = byte;
                }
                self.pos += 1;
                if self.pos >= self.length {
                    self.state = ParseState::WaitForEnd;
                }
            }
            ParseState::WaitForEnd => {
                self.state = ParseState::WaitForStart;
                if byte == END_DELIMITER && !self.oversize {
                    return true;
                }
                // Framing error or dropped oversize message: hunt for the
                // next start delimiter.
            }
        }
        false
    }
}

impl Default for EnttecParser {
    fn default() -> Self {
        Self::new()
    }
}
