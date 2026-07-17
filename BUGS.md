# Bug Review — Findings and Fixes

Full-repo review performed 2026-07-11. Every finding below was verified against the
source before being fixed. **Fixed** items are corrected in this commit; **Documented**
items are real issues left alone deliberately (reason given) — mostly ones needing a
hardware decision or bench test.

## Build-blocking

### B1. FIXED — Corrupted `critical-section` crate in the local cargo registry
The extracted copy of `critical-section-1.2.0` in `~/.cargo/registry/src/...` had been
hand-edited: `type RawRestoreStateInner = u8;` (line 90) and a `compile_error!` (line 74)
were commented out. This made **every** embedded build on this machine fail with
`E0412: cannot find type RawRestoreStateInner`. Cargo does not checksum extracted
sources, so the edit persisted invisibly. Fix: deleted the extracted directory; cargo
re-extracted the pristine crate from the `.crate` archive. Nothing in the repo was wrong.

### B2. FIXED — Workspace member `[profile.*]` sections silently ignored
`nucleo/Cargo.toml` defined dev/release/test/bench profiles, but cargo ignores profiles
in workspace *member* manifests (it warned on every build). Consequences: `nucleo` was
not built with the intended flags, and `overflow-checks = true` did apply in dev builds
via cargo's default — which several arithmetic bugs below turn into boot-time panics.
Fix: profiles moved to the workspace root [Cargo.toml](Cargo.toml); note left in
[nucleo/Cargo.toml](nucleo/Cargo.toml).

## nucleo — HIGH severity

### N1. FIXED — I2C DMX read drops channels 199 and 399 (off-by-one)
[dmx_i2c.rs](nucleo/src/dmx_i2c.rs) read the three blocks with `&mut data_buffer[0..199]`
and `[200..399]` — Rust ranges are end-exclusive, so only 199 of the 200 bytes the pico
sends were read. `data_buffer[199]` and `[399]` (DMX channels 199 and 399) were never
written and stayed 0 forever. Fixed to `[0..200]` / `[200..400]`.

### N2. FIXED — EEPROM layout: boot-status flag overlapped the MAC address
[eeprom/mod.rs](nucleo/src/eeprom/mod.rs): MAC occupies 0x02..=0x07 but
`BOOT_SUCCESSFUL_MEMLOC` was 0x03. Every boot-status write corrupted `mac[1]`, and a MAC
write clobbered the boot flag (a value ≥ 2 decodes as `Failed`, falsely tripping the
ethernet-lockout logic). Moved the flag to 0x10. *Side effect: the first boot after
flashing reads an unwritten byte at 0x10 → decodes as `Failed` → ethernet is disabled
for that one boot, then recovers.*

### N3. FIXED — M24x02 page write always transmits 16 data bytes
[eeprom/m24x02.rs](nucleo/src/eeprom/m24x02.rs): `write_page()` sent the whole
fixed-size 17-byte payload regardless of `data.len()`. Writing the 6-byte MAC at 0x02
actually wrote 16 bytes; the M24C02 wraps within the page, so addresses 0x00–0x01
(including the **module type** at 0x01) were zeroed on every MAC write. Now only
`ADDR_BYTES + data_len` bytes are transmitted.

### N4. FIXED — Art-Net: panic on packets shorter than 512 channels
[artnet/mod.rs](nucleo/src/artnet/mod.rs) did a fixed 512-byte `copy_from_slice` from
`dmx.data`, which is `Length` bytes (the spec allows 2–512). One truncated ArtDmx packet
= slice-length-mismatch panic = reboot loop while the controller keeps transmitting.
Now copies `min(len, 512)` bytes.

### N5. FIXED — Art-Net parser: remotely triggerable out-of-bounds panic
[tiny_artnet/mod.rs](nucleo/src/artnet/tiny_artnet/mod.rs): `parse_dmx` and
`parse_command` sliced `&s[..length]` with the wire-supplied length, unchecked. A single
malformed UDP packet on port 6454 (claimed length > payload) crashed the firmware. Both
sites now bounds-check and return `ParseIncomplete`.

### N6. FIXED — Guaranteed u8 overflow computing the Art-Net static IP
[main.rs](nucleo/src/main.rs): `mac_addr[3] + oem[0] + oem[1]` with
`ARTNET_OEM = 0x7FFF` → `127 + 255` overflows for every possible MAC. Dev builds
(overflow checks on) panic at boot when ethernet + static IP is enabled; release builds
wrapped, which is coincidentally the correct Art-Net mod-256 rule. Now `wrapping_add`.

### N7. FIXED — LED color buffer overrun when group size doesn't divide LED count
[event_router.rs](nucleo/src/event_router.rs): `place = i + group * vled_index` reaches
`group * ceil(leds/group) - 1`, which exceeds the 1024-entry per-port color array for
legal settings (e.g. 999 LEDs, group 400 → index 1199) → panic on the first DMX frame.
Both Individual and Mirror paths now bounds-guard `place`.

## nucleo — MEDIUM severity

### N8. FIXED — Ethernet lockout-prevention didn't apply to the current boot
[main.rs](nucleo/src/main.rs): after a failed boot it wrote `ethernet_enabled: false` to
EEPROM but still brought ethernet up from the stale local copy, so a hang caused by
ethernet init repeated one extra reboot. The local settings are now cleared too.

### N9. FIXED — Art-Net task wrote DMX_BUFFER regardless of input mode
[artnet/mod.rs](nucleo/src/artnet/mod.rs): every ArtDmx packet overwrote the buffer even
in wired-DMX mode, so a stray universe-0 packet corrupted live DMX data between I2C
polls (visible glitches). The write is now gated on `input_mode == ArtNet`.
*Remaining gap (documented, not fixed): packets whose net/sub-net don't match still
land in the buffer before the router filters the notification; filtering fully would
mean plumbing the configured Art-Net address into `artnet_task`.*

### N10. FIXED — Mirror port mode ignored the Art-Net universe offset
[event_router.rs](nucleo/src/event_router.rs): Individual mode offsets reads by
`artnet universe * 512`; Mirror mode didn't, so Mirror + Art-Net universe > 0 rendered
the wrong (usually empty) region. Mirror now applies the same offset, and also gained
the same buffer-bounds clamps Individual mode already had (it previously had none —
group-size 0 in Mirror mode was a guaranteed slice panic, see N14).

### N11. FIXED — "Throw-away" EEPROM writes were never awaited
[eeprom/mod.rs](nucleo/src/eeprom/mod.rs) (4 sites): `let _ = self.dev.write_byte_wait(...);`
without `.await` builds a future and drops it — a no-op. The adjacent comments say this
priming write is required for page writes on the shared bus, so the documented
workaround never actually ran. The calls are now awaited. **Worth a bench retest** of
settings/MAC writes; if they work without the priming write, these lines can simply be
deleted instead.

### N12. FIXED — Settings could outgrow their 32-byte EEPROM slot and silently not save
[eeprom/mod.rs](nucleo/src/eeprom/mod.rs): bincode varint encoding costs 3 bytes per
u16 ≥ 251; a legal worst-case `MenuData` encodes to ~40 bytes. Encoding then failed and
the save was skipped with only a log line — settings lost on reboot. `SETTINGS_SIZE`
raised 32 → 64 (fits easily in the 256-byte part; nothing else lives above 0x20).

### N13. FIXED — UI allowed DMX address 0/513–999 and Art-Net fields beyond spec
[value_type.rs](nucleo/src/ui/value_type.rs) / [ui/mod.rs](nucleo/src/ui/mod.rs): the
digit editor accepted any 0–999 DMX address (valid range is 1–512; 0 shifts all
channels by one, >512 goes dark silently) and any 0–255 Art-Net net/sub-net/universe
(wire fields are 7/4/4 bits, so e.g. net 200 can never match a packet). DMX address is
now clamped to 1..=512 on commit; Art-Net digits clamp to 127/15/15.

### N14. FIXED — Group size 0 accepted by UI → division by zero downstream
[value_type.rs](nucleo/src/ui/value_type.rs): group-size editing only rejected values
> 999, allowing 0. `virtual_leds_per_port()` then computes `leds / 0.0 = inf` →
saturates to 65535 virtual LEDs → effective lockup (Individual) or slice panic
(Mirror). Group size now clamps to 1..=999, plus the defensive bounds fixes of N7/N10.

### N15. FIXED — Menu navigation skipped digits and could dead-end
[menu_tab.rs](nucleo/src/ui/menu_tab.rs): moving between items reused each item's stale
`value_index`, so navigating downward skipped a multi-digit item's other digits (and a
stale cursor could jump tabs unexpectedly). Also `num_selectable_items - 1` and
`size() - 1` underflow on a tab with no selectable items (the PWM tabs) — a panic in
dev builds, an inescapable navigation loop in release. Item changes now reset the
sub-cursor, and empty tabs pass straight through to the next/previous tab.

### N16. FIXED — `todo!()` panics in settings commit path
[ui/mod.rs](nucleo/src/ui/mod.rs): `to_menu_data()` hit `todo!()` for the Pwm and
Unknown module types — an instant panic on Select the moment `module_type` stops being
hardcoded to SmartLed. Replaced with defaults + an error log. *The larger design gap —
`Ui.module_type` is hardcoded and never synced with the EEPROM-detected module — is
documented in [UI_PROPOSALS.md](UI_PROPOSALS.md).*

### N17. FIXED — `unwrap()` on UDP receive
[artnet/mod.rs](nucleo/src/artnet/mod.rs): `socket.recv_from(...).await.unwrap()` — a
socket error (e.g. truncation) panicked the task. Now logged and skipped.

## nucleo — Documented only (not fixed)

### N18. DMX vs Art-Net one-channel addressing inconsistency
DMX mode stores the start code at buffer index 0 (channel N at index N); Art-Net data
has no start code (channel N at index N−1). The router indexes both with `dmx_address`,
so the same configured address is off by one between the two input modes. **Not fixed**
because recent commits (`16a67b9`) intentionally reworked this addressing and the right
convention is a design decision — recommend: store Art-Net data at offset +1 per
universe slot or subtract 1 in Art-Net mode, then re-test both modes on hardware.

### N19. `pwm_i2c_task` is never spawned
The router sends `PwmEvent`s into a channel nothing drains; the PWM module path is
marked "not programmed" throughout. Spawning it blind against real hardware isn't safe
from a code review; left as-is.

### N20. Static-IP mode uses a /24 prefix and an off-subnet gateway
Art-Net convention for 2.x.x.x addresses is /8, and the hardcoded gateway
`192.168.86.1` is not inside the configured subnet. Also `PollReply` advertises
`oem: 0` instead of `ARTNET_OEM`. Functional decisions for the network design — flagged
for review.

### N21. `UiEvent::Load` clobbers in-progress edits
A DHCP renewal mid-edit resets every widget to stored settings while the tab stays in
editing mode; the next Select commits values the user didn't enter. Fix belongs in the
UI redesign ([UI_PROPOSALS.md](UI_PROPOSALS.md)).

### N22. User-button polarity worth a bench check
PC13 is configured `Pull::Up` + active-low, but the Nucleo-H563ZI user button is
typically active-high with an external pull-down. If misread, factory reset triggers on
every boot. Hardware-dependent; verify on the bench.

## nucleo — fixed as part of other work

- Art-Net protocol-version check was inverted (`> 14` rejected instead of `< 14`);
  spec says accept ≥ 14. Fixed in [tiny_artnet/mod.rs](nucleo/src/artnet/tiny_artnet/mod.rs).
- `PortAddress::as_index()` used `>>` where `<<` was intended (always returned just the
  universe). Currently unused; fixed anyway.
- ArtPoll target-port range was built `top..=bottom` (always empty). Currently unused;
  fixed to `bottom..=top`.
- M24x02 page-boundary check overflowed `u8` for writes on the last EEPROM page
  (0xF0–0xFF). Latent with the current layout; computed in `usize` now.

## rp2040_dmx (old code — replaced by the reimplementation)

The previous `rp2040_dmx` was a non-working prototype (its README said so). Defects
found in it, all resolved by the rewrite:

1. **Buffer was 512 bytes, protocol needs 513** (start code + 512 channels; the STM32
   reads 200+200+113 = 513). All I2C block offsets were therefore wrong.
2. **`read()` treated any 0x00 byte as a reset** — it wiped the whole buffer and didn't
   advance the index. DMX data is full of zero values (channel at 0 = off), so the
   buffer could never fill; there was also no packet framing at all (reading could
   start mid-packet).
3. **No BREAK re-synchronization** — the PIO program only passes the break detector at
   program entry; without restarting the state machine per packet, every inter-packet
   BREAK injects a spurious 0x00 byte and framing is lost. (This is why the Pico-DMX C
   library restarts the SM in its DMA-complete interrupt — the rewrite does the same.)
4. **I2C protocol didn't match the STM32 master** — it served the whole buffer on any
   read and used unrelated 0xC2/0xC8 commands instead of the 0x01/0x02/0x03 block
   protocol in `nucleo/src/dmx_i2c.rs`.
5. **`PioWs2812` missing the color-order generic** required by embassy-rp 0.9 — did not
   compile.
6. **DMX_EN pin (GPIO3) never driven** — the RS-485 transceiver's direction pin was
   left floating instead of held low (receive).
7. Byte-at-a-time FIFO polling instead of DMA.

## Arduino sketches

- `DMX_on_pico.ino`: `Wire1.setClock(4000000)` — 4 MHz is not a valid I2C clock (fast
  mode is 400 kHz). Harmless in slave mode (the master drives SCL) but misleading;
  fixed to 400 kHz. Also note the `onRequest` handler defers the response to `loop1()`,
  which only works because the RP2040 I2C slave clock-stretches — the Rust
  implementation responds synchronously instead.
- `pico_stepper/pico_stepper.ino` is a **whitespace-only duplicate** of
  `DMX_on_pico.ino` — it contains DMX code, not stepper code. Left in place; consider
  deleting or replacing with the real stepper sketch.

## rp2040 (crate)

`rp2040/src/main.rs` is a placeholder (`panic!("no code")`) without an entry attribute
or panic handler — it does not build. Left as-is since it's clearly a stub; note that
`cargo build --workspace` from the root will fail on it, so build per-crate as the
readme instructs. *(2026-07-17: removed from the workspace members list, see R13 below.)*

---

# 2026-07 follow-up review — findings and fixes

Full-repo follow-up review performed 2026-07-16 on the `dev` branch (HEAD
`3da4070`), covering all four crates (`nucleo`, `rp2040_dmx`, `dmx_console`,
`host_tests`), the Arduino sketches, and all documentation. Every fix from the
2026-07-11 review above (N1–N17) was re-verified as present. Findings R1–R13
below were fixed 2026-07-17 (except R7, deferred to a bench session).

**State of the tree at review time (before these fixes):**

| Check | Result |
|---|---|
| `nucleo` / `rp2040_dmx` `cargo build --release` | clean |
| `host_tests` + `dmx_console` `cargo test` | 34/34 pass |
| `cargo clippy` (both firmware crates) | 4 style nits (nucleo), 0 (rp2040_dmx) |
| Docs vs. code (readme, crate READMEs, this file, UI_PROPOSALS.md) | accurate |

Logic verified by hand during the review: the PIO RX/TX programs against
Pico-DMX timing (mid-bit sampling; 176 µs BREAK / 16 µs MAB / 4 µs bits /
2 stop bits), the ArtPollReply wire layout (exactly 239 bytes, spec field
order), the Enttec parser's resync behavior on oversize/corrupt framing, and
the router's virtual-LED / universe-offset math. No reachable panic, overflow,
or out-of-bounds path was found on any input surface (Art-Net UDP, Enttec USB,
console lines, I2C blocks) — all are length-checked and bounds-guarded.

**State after the fixes:** both firmware crates build clean in release,
`cargo clippy` is warning-free on both, and all 35 host tests pass
(host_tests 30 + dmx_console 5).

### R1. FIXED — DMX bridge I2C bus dropped from ~4 MHz to 400 kHz
[main.rs](nucleo/src/main.rs): I2C1 was configured at `Hertz(4_000_000)`, and
embassy-stm32 0.4.0's `Timings::new` — traced with the configured 100 MHz
PLL3_R kernel clock — computes PRESC=1, SCLH=7, SCLL=15 with **no clamping or
assertion**, so the STM32 really drove SCL at ~4 MHz: 4× the I2C Fm+ maximum
(1 MHz) and 4× the RP2040 slave's rating. (The 2026-07-11 review caught the
same constant in the Arduino sketch and called it harmless because a slave
doesn't drive SCL — this was the master, where it isn't.) Now 400 kHz fast
mode; the three 201-byte block transfers take ~14 ms, well inside the 30 ms
poll cycle. **Bench check:** confirm clean block reads in DMX mode at the new
speed.

### R2. FIXED — `pico_stepper/` deleted
It was a stale copy of `DMX_on_pico.ino` (no stepper code) still carrying the
invalid `Wire1.setClock(4000000)`. Removed entirely; `DMX_on_pico/` remains
the reference sketch.

### R3. FIXED — rp2040_dmx: I2C `listen()` no longer raced against frame delivery
[rp2040_dmx/src/main.rs](rp2040_dmx/src/main.rs): `i2c_task` used
`select(FRAMES.receive(), dev.listen(&mut buf))`; embassy-rp 0.9.0's
`listen()` keeps its byte-count progress in a local variable, so cancelling
it mid-transaction (a frame arrives ~43×/s in input mode, against roughly
45% bus duty from the STM32's polling) could garble that transaction —
one corrupted poll, which the STM32 logs as `DMX Error` and retries after
250 ms. The failure was partially self-healing (the command byte survives in
the reused `buf`; multi-byte block writes only occur in output mode, when
`FRAMES` is silent), so the exposure was mode transitions and input-mode
traffic. Now the frame channel is drained with `try_receive()` between
transactions and `listen()` is awaited uncancelled. `listen()` completes on
every master transaction (~100/s while polling), so the served frame stays
≤ ~30 ms stale.

### R4. FIXED — LED output no longer silently dead when the EEPROM misbehaves
Two changes:
1. [main.rs](nucleo/src/main.rs): the final `WriteBootStatus(Success)` is now
   retried up to 3× with confirmation via the router; if it still fails, the
   local boot result is treated as authoritative
   (`StoreBootStatus(Some(Success))` is sent directly) so a healthy system
   with a flaky EEPROM still produces output. The unwritten flag means the
   *next* boot reads Failed and disables ethernet — the documented lockout
   behavior, unchanged.
2. [event_router.rs](nucleo/src/event_router.rs): when DMX data is dropped
   because boot never reached `Success` (e.g. no module EEPROM), that is now
   logged at `error!` level once per boot instead of silently ignored.
The "no module EEPROM = no output module" design itself is now documented in
[nucleo/README.md](nucleo/README.md).

### R5. FIXED — ArtPollReply advertises the real device, not the vendored example
[artnet/mod.rs](nucleo/src/artnet/mod.rs): now reports
`short_name: "DMX LED Interface"`, `long_name: "DMX/Art-Net LED Interface"`,
`oem: ARTNET_OEM`, and the configured Port-Address
(`net_switch`/`sub_switch`/`swout[0]` from settings). To make the address
available, `DmxFeedbackEvent::Mode` now carries the full `ArtNetAddr`
(net, sub-net, universe) instead of just the universe byte; the dmx_i2c and
usb_device receivers were updated accordingly. This also closes the
`oem: 0` half of N20. [poll_reply.rs](nucleo/src/artnet/tiny_artnet/poll_reply.rs)
`put_str` also now guarantees NUL termination (see R11).

### R6. FIXED — buffer indexed by sub-net:universe, filtered on Net (option b)
Implemented 2026-07-17 per the analysis at the end of this section.

### R7. DEFERRED — EEPROM `refresh_bus` writes to invalid I2C address 0xFE
[m24x02.rs](nucleo/src/eeprom/m24x02.rs): left as-is per review disposition —
it is an error-recovery path that needs hardware on the bench to retest. When
hardware is available: drop the bogus 0xFE write (keep the ACK-poll `wait()`),
and if a true bus-clear is needed, implement it as 9 manual SCL clocks.

### R8. FIXED — event router is event-driven
[event_router.rs](nucleo/src/event_router.rs): the task loop polled both
channels with 5 ms `with_timeout`s (up to ~10 ms added latency per event,
constant wakeups). Now a single `embassy_futures::select` over the two
receivers.

### R9. FIXED — WS2812 output transmits only the configured LED count
`SmartLedEvent::UpdateLEDs` now carries `leds_per_port` from the router, and
[ws2812_async.rs](nucleo/src/smart_led/ws2812_async.rs) transmits only the
encoded prefix of the pattern buffer (plus the latch gap). Previously every
update pushed all 1024 LEDs per port (~33 ms at 3 MHz SPI) regardless of
configuration; short strings now update much faster. The startup blank in
`enable()` still writes the full 1024 so a longer physical strip can't keep
stale colors across a reboot.

### R10. FIXED — console `set` merges one field instead of writing the whole struct
New `RouterEvent::WriteFieldToEeprom(FieldId, MenuData)`: the router merges
only the named field into its authoritative settings via `FieldId::transfer`
(the same semantics as a TFT commit) and persists. Previously
[console_usb.rs](nucleo/src/console_usb.rs) rebuilt the whole `MenuData` from
the last broadcast, so a console `set` racing a TFT edit could clobber it.

### R11. FIXED — ArtPollReply strings always NUL-terminated
[poll_reply.rs](nucleo/src/artnet/tiny_artnet/poll_reply.rs): `put_str` now
truncates to N−1 bytes so the final byte of the field is always 0x00, as the
Art-Net spec requires. Was latent (names were short); matters now that R5
sets real names.

### R12. FIXED — UI channel deepened so `Load` events aren't dropped
[channels.rs](nucleo/src/channels.rs): `CHANNEL_UI` depth 1 → 4. A `Load`
(e.g. a DHCP address update) arriving while the UI task was mid-draw was
silently dropped and the TFT showed stale settings until the next keypress.

### R13. FIXED — housekeeping (all except the Enttec label-5 note)
- `panic-reset` removed from [nucleo/Cargo.toml](nucleo/Cargo.toml) (never
  linked; would conflict with `panic-probe` if it were).
- `rp2040` stub moved from workspace `members` to `exclude` in
  [Cargo.toml](Cargo.toml). Note discovered while verifying:
  `cargo build --workspace` is *still* not supported even without the stub —
  workspace builds feature-unify `critical-section`, and nucleo (single-core,
  `restore-state-bool`) conflicts with rp2040_dmx (multicore,
  `restore-state-u8`). This is almost certainly how the hand-edited registry
  copy of `critical-section` (B1) came to exist. Per-crate builds remain the
  workflow; the root manifest now documents why.
- Stale RCC comments on the fallback (CSI) clock path in
  [main.rs](nucleo/src/main.rs) corrected (4 MHz CSI math, not 8 MHz HSI).
- All 4 clippy warnings fixed (two indexed loops over `packet`, one
  collapsible match, one manual `% == 0`). Both crates are clippy-clean.
- The Enttec label-5 forwarding starvation note was intentionally **not**
  changed. For the record: in [usb_device.rs](nucleo/src/usb_device.rs) the
  `select` always wins on `read_packet`, so a host that streams label-6
  packets faster than one per 30 ms *while the device is in a receive mode*
  would starve the received-DMX (label 5) forwarding. Only matters with a
  misbehaving host — in USB>DMX mode (the mode where hosts actually stream)
  forwarding is skipped anyway.

## R6 — Art-Net universe addressing analysis (decision needed)

Reviewed against the Art-Net 4 specification (Protocol Release V1.4, document
revision 1.4dp, downloaded 2026-07-17 from art-net.org.uk). The relevant spec
facts:

- **Port-Address is a 15-bit number**: bit 15 = 0, bits 14–8 = **Net**
  (128 nets), bits 7–4 = **Sub-Net**, bits 3–0 = **Universe**. "A group of 16
  consecutive universes is referred to as a sub-net."
- **One ArtPollReply encodes 1–4 ports that share a single Net+Sub-Net**
  (`NetSwitch`/`SubSwitch` are per-reply; only the `SwOut` universe nibble
  varies per port). Classic nodes are therefore "limited to universes from a
  consecutive block of 16."
- **Art-Net 4's BindIndex scheme** is the spec's way past that block: a device
  sends multiple ArtPollReplys with different `BindIndex`, letting every DMX
  port advertise a fully independent Port-Address ("support over 1000 DMX
  ports").

How the firmware behaves today: the router accepts ArtDmx whose Net and
Sub-Net equal the configured address, and the buffer is indexed by the 4-bit
Universe nibble only. So regardless of `DMX_UNIVERSE_COUNT = 256`, **only the
16 universes of the configured sub-net are addressable** — the other 240
universe slots (120 KB of the 128 KB buffer) can never be written.

The catch is that legal LED settings can *span more than those 16 universes*:
`universe_offset()` gives each port a consecutive block, and the worst case
(4 ports × 999 LEDs × RGBW, group 1 → 8 universes per port) needs 32. A
controller transmitting the next consecutive Port-Address crosses the sub-net
boundary (universe 15 → sub-net+1, universe 0); the firmware's sub-net filter
rejects those packets, and worse, the nibble-only buffer indexing would alias
them onto low universes (they are stored before the router filters — the N9
residual gap). Such configurations silently can't work today.

**Answer to "should the UI be updated or is `DMX_UNIVERSE_COUNT = 16` the real
solution?": neither alone — it depends on whether >16-universe configurations
are in scope.** The options:

- **(a) Small node (16-universe block is enough).** Set
  `DMX_UNIVERSE_COUNT = 16` (frees 120 KB of SRAM — the UI already clamps
  universe to 0–15 so no UI change is *required* for correctness), and add a
  validation/warning when `universe_offset() + configured universe` would
  exceed 15 (UI and console), so impossible configurations are visible
  instead of silently dark. This matches the classic node model the firmware
  already implements and what the single ArtPollReply advertises.
- **(b) Recommended — index by Sub-Net:Universe (8 bits), filter on Net only.**
  Change the artnet task to store packets at
  `((sub_net << 4) | universe) * 512` and the router to filter only on Net;
  derive the dmx_i2c output-mode offset the same way. The existing 256 × 512
  buffer is then *exactly* the right size (a full net = 256 universes), spans
  crossing a sub-net boundary work the way controllers actually transmit
  them, and no UI change is needed. This is a ~4-line addressing change plus
  a controller bench test.
- **(c) Full Art-Net 4 BindIndex node.** Advertise each port in its own
  ArtPollReply with independent Port-Addresses. The "proper" modern gateway
  model, but a larger redesign than the current feature set needs.

Recommendation was **(b)** — it converts the currently-wasted 120 KB into
exactly the coverage multi-universe ports need, requires no UI change, and
leaves (c) as a future enhancement.

**Implemented (b) on 2026-07-17.** The changes:

- `PortAddress::sub_uni()` ([tiny_artnet/mod.rs](nucleo/src/artnet/tiny_artnet/mod.rs))
  and `ArtNetAddr::sub_uni()` ([ui/types.rs](nucleo/src/ui/types.rs)) return the
  Port-Address "SubUni" byte (sub-net high nibble : universe low nibble,
  0..=255). Both mask the nibbles to 4 bits so corrupt stored settings can
  never index past the buffer.
- [artnet/mod.rs](nucleo/src/artnet/mod.rs): incoming ArtDmx is stored at
  `sub_uni * 512` instead of `universe * 512`, so consecutive Port-Addresses
  map to consecutive buffer slots across sub-net boundaries.
- [event_router.rs](nucleo/src/event_router.rs): packets are filtered on
  **Net only** (the sub-net equality check is gone), and rendering reads from
  the configured `sub_uni` base in both Individual and Mirror modes. The
  existing bounds clamps still guard spans that would run past the end of the
  net.
- [dmx_i2c.rs](nucleo/src/dmx_i2c.rs) (ArtNet>DMX output) and
  [usb_device.rs](nucleo/src/usb_device.rs) (label-5 forwarding) read the
  universe from the same `sub_uni` base.
- Buffer documentation updated ([statics.rs](nucleo/src/statics.rs),
  [constants.rs](nucleo/src/constants.rs)); `sub_uni` indexing is covered by a
  new host test (`artnet_addr_sub_uni_indexes_one_full_net`).

The full 256 × 512 B buffer is now exactly one addressable net. **Bench
check:** verify with a real controller that a multi-universe configuration
spanning a sub-net boundary (e.g. base universe 14 with a 4-universe port)
renders correctly, and that DMX/USB modes are unaffected.

## Status of items from the 2026-07-11 review (as of the follow-up)

- **N18 — DMX vs Art-Net one-channel addressing offset.** Confirmed still
  present and now the biggest functional decision outstanding: wired DMX and
  USB store channel N at buffer index N (start code at index 0); Art-Net
  stores it at N−1 within its universe slot; the router indexes both with
  `dmx_address`, so the same configured address is off by one between input
  modes (the console `dmx` monitor shows the shift too). Needs the convention
  decision, then retest both modes on hardware.
- **N19 — `pwm_i2c_task` never spawned** (PWM module unfinished). Unchanged.
- **N20 — static-IP /24 prefix + off-subnet gateway (`192.168.86.1`).** The
  `oem: 0` half of N20 was closed by R5; the network-design half remains a
  functional decision.
- **N21 — `UiEvent::Load` vs in-progress edits.** Largely mitigated by the
  Ratatui rewrite's copy-on-edit + per-field merge; the remaining exposure
  (dropped `Load` events) was closed by R12.
- **N22 — PC13 user-button polarity.** Still the top bench-check priority: a
  misread means factory reset + MAC wipe on every boot.
- **First boot after reflashing** still reads an unwritten boot flag → one
  boot with ethernet disabled, then recovers (side effect documented at N2).
- **EEPROM priming writes** (made effective by N11): bench-retest
  settings/MAC page writes; delete the priming writes if unneeded.

## Bench checklist (next hardware session)

1. **R1**: scope SCL on the bridge bus (should now be 400 kHz); confirm clean
   block reads in DMX mode.
2. **N22**: PC13 button polarity — check first, since a misread wipes
   settings/MAC on every boot.
3. **R6**: multi-universe Art-Net configuration crossing a sub-net boundary
   with a real controller; confirm DMX/USB modes unaffected.
4. **N18**: make the addressing-convention decision, then verify a fixture at
   address 1 responds identically in DMX and Art-Net modes.
5. **N11 follow-up**: retest EEPROM settings/MAC page writes without the
   priming write; delete it if unneeded. Then revisit **R7** (the 0xFE
   "bus wake" in [m24x02.rs](nucleo/src/eeprom/m24x02.rs) is not a valid 7-bit
   address and likely just NAKs; drop it and keep the ACK-poll `wait()`, or
   implement a true bus-clear as 9 manual SCL clocks).
6. **R4**: observe the one-boot ethernet-disable after reflashing and confirm
   recovery; optionally test with the module EEPROM disconnected to see the
   new once-per-boot diagnostic and local-state fallback.
7. **Display SPI**: the requested 100 MHz resolves to ~50 MHz actual (SPI
   kernel clock ÷2 floor) — well beyond ST7789 datasheet write-cycle timing.
   It evidently works, but it is overclocked; drop toward ~33 MHz if the
   display ever glitches.
