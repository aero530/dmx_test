# nucleo/ is frozen

This crate no longer builds, deliberately.

It stopped when `common` moved to **embassy-sync 0.8** (needed by embassy-rp
0.10 for the RP2350 target). Bringing it forward means embassy-stm32 0.4 → 0.6
and embassy-net 0.7 → 0.9 — about 72 errors, most mechanical (DMA interrupt
bindings, constructor arity) but including a reworked Ethernet PHY API, on the
exact code the W6300 replaces.

**Its job is done.** It existed as the reference that proved the Phase 1
extraction of `common` stayed faithful, and it passed. `common` is now covered
by `host_tests` (30 tests, on the host) and by `pico2` compiling against it.

**Keep the source** as the reference implementation for the port — `pico2` is
still pulling logic and structure out of it.

Delete it once `pico2` ships. Migrate it instead if a deployed STM32 unit ever
needs patching before then; nothing here is lost, it is only not built.

Removed from workspace `members` in the root `Cargo.toml` on 2026-08-24.
