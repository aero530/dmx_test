//! Mirror of the firmware's `ui` module shape: the real `types.rs` and
//! `fields.rs` sources are included by path, and re-exported the same way
//! `nucleo/src/ui/mod.rs` re-exports them, so their `crate::ui::...` imports
//! resolve identically here.

#[path = "../../nucleo/src/ui/types.rs"]
pub mod types;
pub use types::*;

#[path = "../../nucleo/src/ui/fields.rs"]
pub mod fields;
pub use fields::*;
