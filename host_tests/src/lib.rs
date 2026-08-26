//! Host-runnable tests for the target-agnostic firmware core.
//!
//! This crate used to recreate the firmware's module tree and pull its sources
//! in by `#[path]`, because that code was locked inside a `no_std` binary crate
//! that `cargo test` could not build. It now simply depends on `common`, which
//! is a normal library — the tests in `tests/` exercise the real crate directly.
//!
//! The package is kept separate from the embedded workspace so its dependency
//! tree (std, dev-dependencies) stays out of the firmware's `Cargo.lock`.
//!
//! Run with `cargo test` from this directory.

pub use common::*;
