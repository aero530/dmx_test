//! Art-Net packet parsing and ArtPollReply serialisation.
//!
//! Pure parsing over byte slices — the socket, the network stack and the
//! receive task live in the target crate. `tiny_artnet` is vendored (see its
//! module docs) rather than taken from crates.io.

pub mod tiny_artnet;
pub use tiny_artnet::{Art, PollReply, PortAddress};
