//! Host-runnable unit tests for the pure-logic parts of the nucleo firmware.
//!
//! The firmware is a `no_std` binary crate built for `thumbv8m.main`, so its
//! code cannot be executed by `cargo test` directly. Instead, this crate
//! includes the hardware-independent source files *by path*, recreating just
//! enough of the firmware's module structure (`crate::ui::...` and the
//! crate-root constants) for them to compile unchanged on the host. The
//! tests in `tests/` therefore always run against the real firmware sources.
//!
//! Covered here:
//! * `ui/types.rs` — settings model, enum cycling, EEPROM encoding
//! * `ui/fields.rs` — the field metadata table (ranges, digit editing,
//!   display, console parsing)
//! * `enttec_protocol.rs` — Enttec DMX USB Pro message framing
//!
//! Run with `cargo test` from this directory.

// The included sources use `alloc` types (the firmware runs a heap allocator);
// on the host `alloc` is part of `std`.
extern crate alloc;

// Crate-root constants the included sources expect (values must match
// nucleo/src/constants.rs).

/// Number of bytes in a single DMX universe
pub const DMX_UNIVERSE_SIZE: usize = 512;
/// Number of ports on the SmartLED output module
pub const SMARTLED_PORT_COUNT: usize = 4;
/// Buffer size for incoming DMX data (512 channels + start code)
pub const DMX_BUFF_SIZE: usize = 513;

pub mod ui;

#[path = "../../nucleo/src/enttec_protocol.rs"]
pub mod enttec_protocol;
