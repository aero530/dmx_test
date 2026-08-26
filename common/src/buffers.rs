//! Shared data buffers.
//!
//! The two buffers every input writes into and every output renders from.
//! They are plain `embassy-sync` statics with no HAL involvement, so they live
//! here rather than beside the peripheral setup.
//!
//! # Why `CriticalSectionRawMutex`
//!
//! The STM32 build previously used `ThreadModeRawMutex`, which elides real
//! locking by assuming every accessor runs in thread mode on a single core.
//! Two things make that the wrong choice here:
//!
//! 1. It is `cortex-m`-only, so it cannot appear in a crate that also compiles
//!    for the host — which is how `host_tests` exercises this code.
//! 2. It is **unsound across two cores**. The RP2350 plan puts the network
//!    stack and router on core 0 and the LED render and UI on core 1 — a split
//!    that runs straight through both of these buffers.
//!
//! `CriticalSectionRawMutex` is correct on one core or two, and costs a masked
//! interrupt per lock rather than nothing. At the ~44 Hz these buffers turn
//! over, that is not measurable.
//!
//! Consumers must link a `critical-section` implementation — and on a
//! multi-core target it must be a CROSS-CORE one. The pico2 build gets the
//! hardware-spinlock implementation from embassy-rp's `critical-section-impl`
//! feature (single-core `cortex-m` impls would be unsound there); the nucleo
//! build uses `cortex-m/critical-section-single-core`; the host uses
//! `critical-section/std`.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use smart_leds::RGB8;

use crate::{DMX_UNIVERSE_COUNT, DMX_UNIVERSE_SIZE, SMARTLED_NUM_LEDS_MAX, SMARTLED_PORT_COUNT};

/// Raw mutex backing the shared buffers. See the module docs.
pub type BufferRawMutex = CriticalSectionRawMutex;

pub type LedBuffer = Mutex<BufferRawMutex, [[RGB8; SMARTLED_NUM_LEDS_MAX]; SMARTLED_PORT_COUNT]>;

/// Buffer of current LED colors
pub static LED_COLORS: LedBuffer = Mutex::new([[RGB8::new(0, 0, 0); SMARTLED_NUM_LEDS_MAX]; SMARTLED_PORT_COUNT]);

pub type DmxBuffer = Mutex<BufferRawMutex, [u8; DMX_UNIVERSE_COUNT * DMX_UNIVERSE_SIZE]>;

/// Buffer to hold incoming data (DMX or ArtNet)
///
/// Data is stored flat, covering one full Art-Net net (256 universes):
/// location = sub_uni * 512 + channel offset, where sub_uni is the
/// Port-Address "SubUni" byte (sub-net in the high nibble, universe in the
/// low nibble). Wired DMX / USB use offset 0 (start code at index 0).
pub static DMX_BUFFER: DmxBuffer = Mutex::new([0_u8; DMX_UNIVERSE_COUNT * DMX_UNIVERSE_SIZE]);
