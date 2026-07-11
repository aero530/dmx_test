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
readme instructs.
