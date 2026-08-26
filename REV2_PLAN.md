# DMX Interface Rev 2 — Plan & Progress

Replacing the NUCLEO-H563ZI with a WIZnet **W6300-EVB-Pico2** (RP2350 + W6300
hardwired TCP/IP), collapsing the carrier onto one module.

**Last updated:** 2026-08-24

---

## Status at a glance

| Phase | What | State | Blocked on |
|---|---|---|---|
| Pre-work | Verify the two planning gotchas; fix what they turned up | Done | — |
| **0** | **Bench-gate: measure the real single-SPI universe ceiling** | Not started | **Hardware in hand** |
| **1** | Extract target-agnostic code into a `common/` crate | Done | — |
| **2** | PIO WS2812 ×8 + PIO DMX in/out; dual-core split; retarget HAL code | **Done** (OLED folded into Phase 3) | — |
| **3** | 4→8 ports, buffer resize, OLED UI, editing, EEPROM task, sACN | **Done** | — |
| **4** | QSPI transport | Not started | embassy PR #5809 merging |
| PCB | Rev 2 carrier layout | **Schematic started** — `../dmx_interface_rev2_kicad/` (0 ERC errors); layout still gated on Phase 0 |

**Do not commit the PCB until Phase 0 produces a number.** It sets the byte
budget per port; the design works either way, but the limit the firmware enforces
depends on it.

**The Ethernet failure is now permanently undiagnosed** — the dead Nucleos are
gone, so the post-mortem is cancelled. Rev 2 removes the most likely carrier-side
cause by construction and instruments itself so a recurrence yields data. See
[the Ethernet section](#the-ethernet-failure--undiagnosed-and-now-permanently-so).

---

## Diagrams

Hardware diagrams live in [docs/](docs/) — design intent for checking against
before the schematic is drawn, not a capture of one. Blocks are colour-coded
new-in-Rev-2 vs. carried over, and anything marked VERIFY is still open.

| Diagram | Covers |
|---|---|
| [System block diagram](docs/rev2-block-diagram.svg) | How the module, DMX front end, output stage, UI and power fit together |
| [Pin allocation](docs/rev2-pinmap.svg) | All 40 pins, with the mux checks that make the allocation legal |
| [Power tree](docs/rev2-power-tree.svg) | DC in to VSYS to 3V3(OUT), the diode-OR, and what Rev 1 parts get deleted |
| [WS2812 output stage](docs/rev2-ws2812-output.svg) | 8-channel level shifter, series R, per-output fusing |
| [Isolated DMX front end](docs/rev2-dmx-isolated.svg) | Opto-isolated RS-485, bias, termination, grounding |
| [UI and I²C](docs/rev2-ui-i2c.svg) | OLED on SPI1, TCA9555 expander, EEPROM, address map |

---

## Context

The NUCLEO-H563ZI boards keep losing Ethernet permanently — unrecoverable by
power cycle, only fixed by swapping the board. **The failure mechanism is still
undiagnosed** (see [Unfinished business](#unfinished-business--the-actual-ethernet-failure)).

Independently, the Rev 1 design review (`../dmx_interface_dev_board/review.md`)
found a cluster of hard errors concentrated in the STM32-to-RP2040 bridge and the
drive-select matrix, plus abs-max hazards in the PWM output modules.

Rev 2 puts Ethernet, wired DMX in/out, USB, the UI and **8 strings of WS2812**
on one module. The RP2040 DMX bridge, the drive-select matrix and the PWM output
modules all go away; scope narrows to WS2812 only.

The existing firmware is not thrown away — it is Embassy throughout, and Phase 1
proved 46% of it is target-agnostic.

---

## Decisions taken

| Question | Decision |
|---|---|
| Module | W6300-EVB-Pico2 (not W5500) — keeps the QSPI upgrade path open |
| Topology | Single module, dual-core |
| WS2812 strings | 8, level-shifted by **SN74ACT245** (or AHCT245) |
| Max bytes/port | 1800 — i.e. 600 RGB or 450 RGBW per string |
| Transport | PIO single-SPI now; QSPI in Phase 4 when embassy PR #5809 merges |
| Colour mode | RGB default, RGBW selectable (shortens the max string, nothing else) |
| Display | SSD1306 128×64 on SPI |
| Buttons / aux I/O | **TCA9555PWR** (TSSOP-24) on the existing I²C bus, interrupt-driven |
| Power | 5 V into VSYS through a Schottky; 3V3(OUT) feeds the module only |
| Carrier 3V3 | **TLV75733PDYDR** on VSYS → 3V3_CARRIER, 117 mA (PS1 is ~90 mA) |
| Config storage | **M24C02-WMN6TP** on the carrier; SO8N footprint also takes M24256 |
| Spare I/O | GP13 and the 11 free TCA9555 lines broken out to test points |
| Protocols | Art-Net (broadcast + unicast), sACN/E1.31, USB Enttec widget, wired DMX in/out |

## Open questions

*None — all hardware questions are closed. See the change log for what was
settled and when.*

**Settled during the hardware close-out:** GP23 (not available — internal SMPS
`PS` pin); PS1's supply and the RT6150 budget (the LDO removes it); colour mode
(RGB default, RGBW selectable, handled as a byte budget); config storage (EEPROM
stays, moves to the carrier); and the LDO part.

---

## Hardware

### Why W6300 over W5500, and what it costs

The W6300-EVB-Pico2 wires SCLK to GPIO17, which is a CSn mux position — hardware
SPI0 cannot reach it in **any** mode, so PIO is mandatory even for single SPI.
That costs 1 state machine and 8 GPIOs (GP15–22) versus the W5500 board's 0 SMs
and 6 GPIOs. Net cost: **two LED strings** (8 instead of 10). Worth it for the
QSPI door.

Transport, using released crates only:

```
embassy_rp::pio_programs::spi::Spi   (async, DMA-backed, impl SpiBus<u8>, 1 SM)
  └─ embedded_hal_bus::spi::ExclusiveDevice   (already a dependency)
       └─ embassy_net_wiznet::Device  (W6300, MACRAW mode)
            └─ embassy-net / smoltcp   <- unchanged API from today
```

Because MACRAW puts smoltcp underneath `embassy-net`, the networking API is
**identical** to the STM32 build, so `artnet_task` ports as-is.

### Budgets

**Bandwidth — it is a byte budget, not a colour mode.**

The load is set by *bytes per port per frame*. Colour mode and LED count are two
ways of spending the same budget, so RGBW does not need a different plan — just a
shorter maximum string.

`universe_offset()` allocates on **per-port** universe boundaries, so the figure
is `8 × ceil(bytes_per_port / 512)`, not a flat total.

| Bytes/port | RGB LEDs | RGBW LEDs | Universes | Mbps @ 44 Hz | Strip frame | Strip ceiling |
|---|---|---|---|---|---|---|
| 900 | 300 | 225 | 16 | 3.2 | 9.0 ms | 111 Hz |
| 1350 | 450 | 338 | 24 | 4.9 | 13.5 ms | 74 Hz |
| **1800** | **600** | **450** | **32** | **6.5** | **18.0 ms** | **56 Hz** |
| 2400 | 800 | 600 | 40 | 8.1 | 24.0 ms | 42 Hz |

**RGBW at 450/string is byte-for-byte identical to RGB at 600/string** — same
1800 B/port, same 32 universes, same 6.5 Mbps, same 18.0 ms frame. Nothing about
the hardware changes; only a firmware limit does.

Two separate ceilings apply, and the strip one bites first at the top end:

- *Network* — single-SPI MACRAW realistically delivers 8–15 Mbps.
- *Strip* — WS2812 is 30 µs/LED (24 bits), SK6812 RGBW is 40 µs/LED (32 bits).
  At 2400 B/port the strip alone takes 24 ms, a **42 Hz ceiling that is already
  below the 44 Hz Art-Net rate**.

So above roughly 2400 B/port the WS2812 protocol is the wall, not the SPI link.
**That makes QSPI insurance rather than a plan dependency** — it buys headroom
for more strings or a higher refresh, not for this configuration. Keeping the
door open still costs nothing, so the decision stands.

Note the ingest risk is *broadcast* Art-Net, not your own universes: a console
broadcasting 32 universes is 6.5 Mbps you must ingest and discard. Art-Net 4
directs controllers to unicast where possible, driven by ArtPollReply — which is
why that packet being correct matters operationally, not just for spec
compliance.

Phase 0 still gates the PCB, but what it establishes is *where the byte budget
sits*, not whether the design works.

**Memory** (RP2350 has 520 KB):

| Item | Size |
|---|---|
| `DMX_BUFFER` resized 256 to 64 universes | 32 KB (was 128 KB) |
| `LED_COLORS` 8 × 600 × 4 | 19.2 KB |
| WS2812 PIO/DMA buffers | 19.2 KB |
| OLED framebuffer (was ~110 KB TFT) | 1 KB |
| smoltcp + MACRAW RX | ~48 KB |
| **Total** | **~120 KB** |

**PIO** (3 blocks × 4 SM, 32 instr/block):

| Block | Contents | Strings |
|---|---|---|
| PIO0 | PIO SPI transport (1 SM, ~12 instr) + WS2812 program | 3 |
| PIO1 | WS2812 | 4 |
| PIO2 | DMX in + DMX out (2 SM) + WS2812 | 1 (+1 SM spare) |

**Pins** — 18 free (GP0–14, GP26–28 after the W6300 takes GP15–22):

| Function | Pins |
|---|---|
| WS2812 ×8 | GP0–GP7 |
| DMX RX / TX / DE | GP8 / GP9 / GP10 |
| OLED SPI1 — SCK / MOSI / DC | GP14 / GP11 / GP12 |
| EEPROM + TCA9555, I2C1 — SDA / SCL | GP26 / GP27 |
| TCA9555 `INT` | GP28 |
| **Spare** | **GP13** |

`INT` is open-drain — 10 kΩ pull-up to 3V3.

Two things keep GP13 free: the OLED `CS` ties low (it is alone on SPI1) and OLED
`RES` moves onto the expander, since it only asserts at boot. **Do not** move DMX
`DE` or OLED `DC` onto the expander — `DE` switches per-frame against PIO output
timing and `DC` toggles per-transaction at SPI speed. Both must stay on GPIO.

### Button I/O — TCA9555PWR

16-bit expander on the existing I²C1 bus, address 0x20–0x27 (A0/A1/A2 to GND =
0x20; no conflict with the M24C02 at 0x56). Net GPIO cost is 1 — the same as the
resistor ladder it replaces — but it buys:

- **Internal pull-ups on inputs**, so buttons wire straight to ground with no
  external resistors.
- Real digital inputs instead of ADC thresholds, so **chords work** (a resistor
  ladder cannot distinguish simultaneous presses).
- Interrupt-driven via `INT` rather than polled.
- 16 lines total. 4 buttons + OLED `RES` = 5 used, **11 spare** for status LEDs,
  a rotary encoder, Art-Net address DIP switches, or module-present detects.

### Configuration storage — EEPROM on the carrier

Settings made in the UI persist to an **M24C02** (256 B) at **0x56**, keeping the
existing `EEPROM_ADDRESS` so no firmware churn. No strap conflict with the
TCA9555 at 0x20.

The part itself is unchanged; what changes is where it lives. In Rev 1 it sat on
the *output module* and doubled as module-type identification. With the module
system gone it moves onto the carrier and is purely a settings store.

| Address | Rev 1 | Rev 2 |
|---|---|---|
| `0x01` | module type | **settings schema version** — see below |
| `0x02–0x07` | MAC address | unchanged, still needed for Ethernet |
| `0x10` | boot-success flag | unchanged |
| `0x20–0x9F` | settings, 128 B | unchanged (raised from 64 B in pre-work) |
| `0xA0–0xFF` | free | free |

**Repurpose `0x01` as a schema version.** `ModuleType` loses its meaning with one
board, but the byte is worth keeping: `MenuData` is bincode-encoded with no
self-describing header, so a future field change would silently mis-decode an old
EEPROM into plausible-looking garbage. A version byte turns that into a detected
mismatch and a clean fall back to defaults.

Keep the WC pin pulled high with a ground bridge to enable writes, as in Rev 1.

**Part: M24C02-WMN6TP.** `W` = 2.5–5.5 V, `MN` = SO8N, and it matches the
existing `m24x02.rs` driver exactly — 1-byte internal addressing, 16-byte pages.
Zero firmware change, and 256 B is comfortably enough (settings end at 0x9F with
0xA0–0xFF spare).

**The SO8N footprint also takes M24256-BRMN6TP** (32 KB), so the upgrade path
costs no PCB change — populate the bigger part whenever presets, per-port names
or a fault log appear. Two firmware changes are needed when that happens:

- **16-bit internal addressing.** M24256 sends two address bytes before data;
  `m24x02.rs` sends one. This is the substantive change.
- **`PAGE_SIZE` 16 → 64.**

Both parts sit at 0x50–0x57 via E0/E1/E2, so **0x56 works for either** and
`EEPROM_ADDRESS` does not move. Worth staying on the M24C02 for Rev 2: the
existing driver carries a tested workaround for a shared-bus page-write quirk,
and swapping the part means re-validating that path for capacity nothing needs
yet.

### Test points

Break out everything spare rather than leaving it stranded:

| Signal | Source |
|---|---|
| `GP13` | the **only** spare GPIO — also SPI1 CSn if a second SPI device ever appears |
| `P04–P07`, `P11–P17` | 11 unused TCA9555 lines |
| `VSYS`, `3V3_CARRIER`, `3V3(OUT)`, LED 5 V, GND | rail probe points |
| SWD | 3-pin debug header (already specified) |
| DMX `DE`, one WS2812 data line | scope points for the two timing-critical signals |

The rail probes matter more than usual here: the Phase 0 and power verification
steps both call for scoping VSYS and 3V3 through power-on and an LED step, and
the transient behaviour on those rails is the leading suspect for the dead Nucleo
Ethernet. Make them easy to reach with a probe ground spring.

### Power — feed VSYS, never back-drive 3V3

Rev 1 ties carrier 3V3 (U11) to the Nucleo's 3V3 pin through the unannotated 0 Ω
`R?` — two regulators fighting on one node with no ORing. The Pico2 has a
sanctioned answer: VBUS reaches VSYS through an internal Schottky (D1)
specifically so supplies can be power-ORed, and VSYS feeds an **RT6150
buck-boost** that makes 3.3 V for the module.

| Pin | Connection |
|---|---|
| **VSYS** (39) | carrier logic 5 V **through a Schottky** (PMEG2010AEH / SS14); also the LDO's input |
| **VBUS** (40) | **leave unconnected** |
| **3V3(OUT)** (36) | **module only** — no carrier load at all |
| **3V3_EN** (37) | leave unconnected (internally pulled to VSYS); optional test point |

The two Schottkys form a diode-OR: USB and the brick can both be present, neither
back-feeds the other, and USB debugging with the carrier powered just works.

**Delete U11** (carrier 3V3 regulator), **U14** (the non-functional AP74700 ideal
diode) and **`R?`**.

#### Carrier 3V3 comes from an LDO on VSYS

`PS1` (1S7BE) is confirmed **3V3-in**, so it cannot hang off the 5 V rail. At the
review's 50–60 mA on 3V3ISO and ~70% efficiency for a small unregulated isolated
converter, its input draw is **≈90 mA** — far too much to add to 3V3(OUT).

So all carrier 3V3 moves to one LDO:

| Load | Current |
|---|---|
| PS1 1S7BE input (3V3 in → 3V3ISO) | ≈ 79 mA (datasheet: 76% eff, GAPTEC 1S7BE-0303S3U) |
| SSD1306 OLED | 20 mA |
| I²C pull-ups (2 × 2k2) | 3 mA |
| TCA9555 + INT pull-up | 2 mA |
| M24C02 EEPROM | 2 mA |
| **3V3_CARRIER total** | **≈ 106 mA** |
| **3V3(OUT) carrier load** | **0 mA** |

**Selected part: TLV75733PDYDR** (TI, SBVS322C).

| | |
|---|---|
| Package | `DYD` — SOT-23-5 **with exposed thermal pad**, 2.9 × 2.8 mm |
| RθJA | **60.3 °C/W** (the plain DBV SOT-23-5 is 100.8) |
| Pinout | 1 `IN`, 2 `GND`, 3 `EN`, 4 `NC`, 5 `OUT`, + thermal pad |
| Vin | 1.45–5.5 V recommended, 6.0 V absolute max |
| Dropout | 300 mV typ / 450 mV max **at 1 A** → ~35–55 mV at our 117 mA |
| Output cap | stable with 1 µF ceramic, **no ESR floor** |
| Iout | 1 A |
| PSRR | 45 dB at 100 kHz |
| Extras | active output discharge, foldback current limit, thermal shutdown, UVLO |

Headroom, using the pessimistic full-load *max* dropout of 450 mV rather than the
~50 mV we would actually see: 4.65 − 0.45 = **4.20 V** on brick, 4.40 − 0.45 =
**3.95 V** on USB. Both comfortably above 3.3 V, so it regulates in every case
the NCP1117LP failed.

Thermals: 163 mW → Tj ≈ 50 °C at 40 °C ambient; 257 mW worst case → 55 °C.

Four things the layout must get right:

- **`EN` is gated by the module's 3V3(OUT) via R61 (0R).** Not tied to IN: if the
  carrier rail rises before the module's IOVDD (TLV starts in ~0.2 ms, the RT6150
  soft-starts over ms), the I²C pull-ups and opto anodes back-inject ~15 mA into
  unpowered, non-failsafe RP2350 pads on every boot. Gating EN (on ≥0.6 V)
  makes the carrier rail rise strictly after IOVDD. R62 (0R, DNP) is the bench
  override for testing the power tree with no module fitted.
- **Solder the thermal pad to a GND pour with a via array.** That is what buys
  the 60.3 °C/W; unsoldered it degrades toward ~100 °C/W (still fine at this
  power, but there is no reason to give it away).
- **≥1 µF ceramic on both `IN` and `OUT`.** Use 10 µF on the output — PS1 is a
  switching converter with real inrush, and the extra bulk costs nothing.
- **Place it close to the module's VSYS pin.**

The 45 dB PSRR earns its keep here: the LDO's input is VSYS, which shares a node
with the module's own switcher, and its output feeds PS1 across the isolation
barrier.

Two consequences worth stating plainly:

1. **The LDO's input is VSYS, not the brick.** VSYS is already the diode-OR node,
   so the UI, EEPROM and the whole DMX side stay alive on USB-only bench power.
   Feeding the LDO from the brick instead would leave all of it dead whenever you
   plugged in to debug. USB-only budget: module ≈165 mA from VSYS + 117 mA
   carrier ≈ **282 mA from VBUS**, inside the 500 mA allowance. The LED strings
   obviously will not run on USB, but everything you need to debug will.
2. **There are now two 3V3 rails** — the module's 3V3(OUT) and 3V3_CARRIER — with
   I²C and SPI crossing between them. Both derive from VSYS and rise within
   milliseconds of each other, so no explicit sequencing is needed.

#### Rail separation

**Keep the logic branch off the LED rail.** 8 × 600 WS2812 is a ~288 A worst-case
5 V node — a theoretical ceiling, not a design target, but the rail will still
sag hard. Star-feed the logic branch from the input, with its own ferrite and
bulk cap.

The RP2350 is largely immune to sag anyway, because the RT6150 is a *buck-boost*
and holds 3.3 V down to a VSYS of about 1.8 V. The LDO is the part that sets the
real floor: it needs roughly 3.6 V in to stay in regulation, so that is the point
at which DMX drops out.

**`5V_LVL_SHIFT` taps the LED 5 V rail, not VSYS.** That is deliberate — the
SN74ACT245's output swing should reference the same rail the WS2812 VIH threshold
references, so the two track together as the rail moves.

### Carrier board (Rev 2)

**Deleted:** RP2040 Pico bridge, drive-select matrix, Low-Power and High-Power
modules, module connector system (pending the open question above).

**Rev 1 review items this resolves for free:** 1.1, 1.2 (RP2040 I²C/SPI pin
mapping — no bridge), 1.3 (drive-select crossover — no matrix), **1.5 and 3.6
(ideal diode / dual-3V3 contention — deleted outright by the VSYS scheme, not
merely mitigated)**, 2.1, 2.4, 2.5 (TLC59116 — module gone), 3.4 (STM32 PWM AF
availability — module gone).

**Still applies:** 2.3 and 3.7 — TVS uprate and fusing, now on the 5 V LED rail.

**Kept and simplified:** the SN74ACT245 level shifter is exactly 8 channels for 8
strings. Strap `DIR` permanently and delete JP21 — that resolves 2.2 (5 V into
non-5V-tolerant GPIO on reverse direction) by construction.

**New:** socket the W6300-EVB-Pico2 on 2x20 headers rather than soldering it, and
break out the 3-pin SWD debug header. Specify **RP2350 A4 stepping** (E9 erratum
fixed); use active-low buttons with pull-ups regardless.

---

## Firmware

### Crate layout (as of Phase 1)

| Crate | Target | LOC | Role |
|---|---|---|---|
| `common/` | any (`no_std` lib) | 2,442 | Everything that names no peripheral |
| `pico2/` | RP2350 (thumbv8m.main-none-eabihf) | — | **New target crate.** Peripheral setup, PIO programs, executor wiring |
| `nucleo/` | STM32H563ZI | 2,919 | **FROZEN** — see `nucleo/FROZEN.md`. Out of the workspace; source kept as the port reference |
| `rp2040_dmx/` | RP2040 | 560 | DMX bridge; folds into the target crate in Phase 2 |
| `host_tests/` | host | — | Tests `common` directly |
| `dmx_console/` | host | — | Desktop settings/monitor app |

**The dividing line is a concrete peripheral type or `#[embassy_executor::task]`.**
A task signature naming `Spi<'static, Async>` stays in the target crate; the logic
it drives and the messages it passes do not. `event_router` is the worked example:
458 lines of routing in `common`, a 22-line task wrapper in `nucleo/`.

`common/` has **no HAL dependency** — not `embassy-stm32`, not `embassy-rp`, not
`embassy-executor`. Keeping `nucleo/` building against it is what proves the
extraction stayed faithful.

### Phase 2 worklist — what is left in `nucleo/`

| File | LOC | Action |
|---|---|---|
| `main.rs` | 750 | **Rewrite** — clocks, pins, `memory.x`, RP2350 image def, `multicore::spawn_core1` |
| `ui/mod.rs` | 312 | Retarget SPI/GPIO, then **redesign for 128x64 mono** (Phase 3) |
| `eeprom/mod.rs` | 284 | Retarget I²C type only |
| `usb_device.rs` | 258 | Retarget `embassy-stm32` to `embassy-rp` USB driver |
| `artnet/mod.rs` | 241 | Swap `Ethernet<ETH, GenericPhy>` for the `embassy-net-wiznet` device |
| `console_usb.rs` | 186 | Retarget USB driver type |
| `dmx_i2c.rs` | 176 | **Mostly delete** — DMX moves in-process from `rp2040_dmx` |
| `eeprom/m24x02.rs` | 174 | Retarget I²C type only |
| `button_array.rs` | 134 | **Rewrite** for TCA9555 reads on the `INT` edge |
| `smart_led/ws2812_async.rs` | 97 | **Replace** with `pio_programs::ws2812::PioWs2812` |
| `smart_led/mod.rs` | 94 | Retarget; 4 to 8 ports |
| `pwm_i2c.rs` | 85 | **Delete** — PWM modules are out of scope |
| `button.rs` | 55 | Rewrite for TCA9555 |
| `statics.rs` | 35 | Replace I²C aliases with `embassy-rp` equivalents |
| `event_router.rs` | 22 | Task wrapper — trivial |
| `led.rs` | 16 | Retarget `Output` |

Plus **new**: a TCA9555 driver — four register pairs (input / output / polarity /
config), roughly 60 lines of async I²C against the shared bus, rather than
pulling in the blocking `port-expander` crate. The debounce and event logic above
it is unchanged.

And **from `rp2040_dmx/`**: `dmx_pio.rs` plus `pio/DmxInput.pio` and
`pio/DmxOutput.pio` move in verbatim.

### Phase 2 progress

Building `pico2/` alongside `nucleo/` rather than converting it, so the working
STM32 reference stays intact until the new target is proven.

**Done and compiling:**

- **Crate scaffold.** RP2350 `memory.x`, `build.rs`, `.cargo/config.toml`.
  `picotool info` reports a valid *ARM Secure* image with `.start_block`
  (IMAGE_DEF), `.bi_entries` and `.end_block` in the right places — the boot
  metadata is correct, which is the part that silently fails to boot if wrong.
- **WS2812 ×8 on PIO** (`pico2/src/smart_led.rs`). All eight outputs
  instantiated across the three PIO blocks in the exact allocation the plan
  claimed, with SM0 of PIO0 and SM0/SM1 of PIO2 left reserved for the W6300
  transport and wired DMX. **This turns the PIO budget from an assertion into a
  compiled fact.** The eight writes are `join`ed rather than awaited in turn, so
  the frame time is the longest string, not the sum.
- A bring-up colour-walk task, so first power-up immediately exercises all eight
  outputs, the level shifter and the connectors. Explicitly temporary.

**Two decisions worth recording:**

- **embassy-rp 0.9.0, not 0.10.** 0.10 pulls `embassy-sync` 0.8 while `common`
  and nucleo are on 0.7.2 — two incompatible copies of the channel and mutex
  types in one graph. 0.9.0 has everything needed (`rp235xa`, `binary-info`,
  `pio_programs::{spi, ws2812}`) and pins 0.7.2. Revisit when nucleo retires.
- **`link.x` must be passed explicitly.** The embassy rp235x example ships no
  rustflags, but this toolchain does not apply `link.x` on its own, so `memory.x`
  never gets included and the `__bi_entries_*` symbols go undefined. `rp2040_dmx`
  already had the right incantation; `pico2` uses the same, plus `flip-link`.

**Footprint so far:** 51 KB flash (2.4%), 54 KB RAM (10.4%). `DMX_BUFFER` and
`LED_COLORS` are not linked in yet — they add ~140 KB at the current 256-universe
sizing, ~44 KB after the Phase 3 resize to 64 universes.

**Dependency update (2026-08-24).** Whole workspace moved to current embassy:
`embassy-rp` 0.10, `embassy-executor` 0.10, `embassy-sync` 0.8, `embassy-net`
0.9.1, `embassy-net-wiznet` 0.3, plus defmt 1.1.1 and friends. Three API changes
worth remembering:

- `arch-cortex-m` → **`platform-cortex-m`** on embassy-executor.
- **`spawn()` no longer returns `Result`** — the *task function* does, so the
  unwrap moved inside: `spawner.spawn(unwrap!(task(..)))`.
- **Every DMA channel in use must bind a handler** on `DMA_IRQ_0`, and the PIO
  drivers take that binding as an explicit argument. `AnyChannel` became an
  owned `dma::Channel<'d>`, and the `Channel` trait became `ChannelInstance`.

Two versions held back deliberately: **bincode stays at 2.x** (3.0 is a
semver-major on the crate that encodes the EEPROM blob — moving it means
re-measuring the encode budget and re-validating the on-EEPROM format for no
current benefit), and `fixed` stays at 1.x (2.0 is a pre-release alpha).

**`nucleo/` is now frozen** rather than migrated — 72 errors, including a
reworked Ethernet PHY API on the exact code the W6300 replaces. It is out of the
workspace `members`; see `nucleo/FROZEN.md`. Its job — proving the Phase 1
extraction of `common` stayed faithful — is done and passed, and `common` is now
covered by `host_tests` plus `pico2` compiling against it. `rp2040_dmx` *was*
migrated (~10 mechanical errors) since its PIO code is about to be ported across.

**W6300 transport — compiling** (`pico2/src/w6300.rs`). PIO SPI on PIO0 SM0 →
`ExclusiveDevice` → `embassy-net-wiznet` in MACRAW → `embassy-net`. Because
MACRAW puts smoltcp underneath, the stack API is identical to the STM32 build,
which is what lets the Art-Net task port unchanged. DHCP is wired and reports its
address; handing the stack to the router comes with the router.

Two things fell out of it:

- **The self-diagnosis hook is free.** `InitError` already separates
  `SpiError` (chip not answering on the bus) from `InvalidChipVersion`
  (answers, wrong version register). Those are logged as distinct causes —
  exactly the distinction that was impossible on the Nucleo without a scope, and
  the reason the undiagnosed failure is now instrumentable.
- **`SPI_FREQ_HZ` is the Phase 0 lever.** Set to a conservative 20 MHz. Raising
  it while watching for link errors is how the byte budget gets established.

Ethernet failure is non-fatal on purpose: DMX, USB and the LEDs all still run
without a network, and the boot-flag/lockout logic needs to reach that point.

**Wired DMX — compiling** (`pico2/src/dmx_pio.rs`, `dmx.rs`). The PIO driver
came over from `rp2040_dmx` unchanged: it derives its divider from
`clk_sys_freq()`, so it adapted from the RP2040's 125 MHz to the RP2350's
150 MHz on its own. **The RP2040 bridge and its I²C protocol are gone** — DMX
runs in-process on PIO2 SM0/SM1, which deletes the 30 ms poll and the bug class
review §1.1/§1.2 found in that link.

The task writes the 513-byte frame at `DMX_BUFFER[0..513]` with the start code at
index 0, matching the STM32 build exactly, and raises `DmxEvent::DmxPacket` on
Port-Address 0:0:0 — so the router's indexing needs no change. Transmit is
constructed but not driven: it belongs to the `ArtNet>DMX` / `USB>DMX` modes,
which need settings from the router.

**TCA9555 + buttons — compiling** (`tca9555.rs`, `buttons.rs`). A ~110-line
async driver over four register pairs, rather than a dependency. Two details
that matter:

- **`inputs()` always reads both ports in one auto-incrementing transfer.**
  `INT` only clears for the port actually read, so polling one port leaves it
  latched and the panel dies after a single press. That is the failure mode the
  verification checklist calls out, handled at the source.
- **Buttons are interrupt-driven, not polled** — the task blocks on `INT` and
  only touches the bus on a change. The wait keeps a 125 ms timeout because a
  *held* button produces no further edges, and that tick is what advances
  Pressed → Held. The state machine and emit-on-release behaviour are identical
  to the STM32 `button_row_task`, and the `D`/`Pound`/`N0`/`Star` identities are
  preserved, so the router's existing mapping is untouched.

⚠️ **One assumption needs bench confirmation:** buttons are wired straight to
ground on the basis that the TCA9555 pulls its inputs high internally. If that
does not hold, they need external pull-ups — a board change, not a firmware one.

**PIO budget, as built:** 11 of 12 state machines in use — PIO0 SM0 transport +
SM1-3 strings, PIO1 SM0-3 strings, PIO2 SM0/SM1 DMX + SM2 string. One spare, as
predicted.

**M24C02 EEPROM — compiling** (`m24x02.rs`, `eeprom.rs`). The driver came from
the STM32 build with its I²C operations made async, because the bus here is
shared with the TCA9555 behind an async mutex. The logic is unchanged — page
boundary checks, the post-write acknowledge poll, and the bus-refresh workaround
for the shared-bus page-write quirk all carried over intact.

**Boot order changed as a result:** I²C1 now comes up *first*, because the MAC
the W6300 needs lives in the EEPROM. Ethernet cannot start until the bus is
alive.

The MAC read validates rather than trusting: all-`0xFF` (blank part), all-zero
(half-programmed), or the multicast bit set (the classic symptom of reading the
wrong offset) each fall back to a locally-administered default, with a warning
that the part needs programming. Two *unprogrammed* boards on a bench would
still collide — that is the point of the warning.

`0x01` is now the **schema version**, replacing module type. `MenuData` is
bincode-encoded with no self-describing header, so a field change would
otherwise mis-decode an old EEPROM into plausible-looking garbage; the version
byte turns that into a detected mismatch and a clean fall back to defaults.

**Footprint now:** 177 KB flash (8.4%), **211 KB RAM (40.2%)**. The RAM is
dominated by `DMX_BUFFER` at its current 256-universe sizing; the Phase 3 resize
to 64 universes reclaims ~96 KB. `pico2/` is 1,397 lines across nine modules.

**Art-Net, event router and LED render — wired.** `artnet.rs` moved over
unchanged apart from dropping a vestigial `net_task`/`Device` alias that named
the STM32 MAC; the task itself only ever touched `embassy_net::Stack`, so it is
transport-agnostic — which is exactly why MACRAW was chosen over the W6300's
hardwired socket API. The router is constructed against the same eight channel
statics as before, and the WS2812 bring-up pattern is replaced by the real
`LED_COLORS` render.

One constraint shaped that render: **`PioWs2812::write` only takes a
fixed-size array**, so every frame costs `MAX_LEDS` of wire time whatever the
configured string length. `MAX_LEDS` is therefore pinned at **600** here (the
decided byte budget) rather than inherited from `common`'s 1024 — at 1024 a
frame would be 30.7 ms and cap refresh at 32 Hz; at 600 it is 18.0 ms,
comfortably above the 44 Hz Art-Net rate. Frames are also **staged into a local
buffer before rendering** rather than written from `LED_COLORS` directly:
holding that lock across an 18 ms render would stall the router on the other
core. Staging doubles as the blanking path, so a shortened string does not leave
stale pixels lit.

**Dual-core split — done.** `main` is now `#[cortex_m_rt::entry]` with two
executors. Core 1 runs buttons, wired DMX and the LED render; core 0 runs the
W6300, smoltcp, Art-Net, USB and the router. Everything is constructed in `main`
before the split, because peripherals cannot be divided after the cores start,
and the async half of boot (the EEPROM MAC read and the W6300 reset sequence)
moved into a `net_bringup` task since neither can be awaited from `entry`.

This is where the Phase 1 concurrency fix earns itself: `LED_COLORS` and
`DMX_BUFFER` are written by the router on core 0 and read by the LED task on
core 1. Under the original `ThreadModeRawMutex` that would have been unsound.

**USB composite — ported.** `usb_device.rs` and `console_usb.rs` came across
with a two-line change each (`embassy_stm32::usb::Driver` →
`embassy_rp::usb::Driver`); everything above the driver type is `embassy-usb`
and target-agnostic. The Enttec DMX USB Pro widget and the `dmx_console` line
protocol both survive intact. It sits on core 0 as an input alongside the
network.

**The OLED is deliberately deferred to Phase 3.** Phase 2 called for
"retarget SPI/GPIO", but the STM32 UI is a colour-TFT Ratatui/mousefood stack
and the 128×64 mono redesign *is* the Phase 3 work item — there is nothing
meaningful to draw until that redesign exists, so bringing up the bus early
would be motion without progress.

**Footprint:** 242 KB flash (11.5%), **311 KB RAM (59.4%)**. `DMX_BUFFER` at its
current 256-universe sizing is the bulk of that; the Phase 3 resize to 64
universes reclaims ~96 KB and brings it back to roughly 41%. `pico2/` is 2,153
lines across thirteen modules.

**Phase 2 is complete.** What is still stubbed rather than missing: DMX transmit
(needs the mode setting), the EEPROM write path (needs router events), and the
8-port settings model — all Phase 3.

### Phase 3 progress

**8-port settings model — done.** `SMARTLED_PORT_COUNT` is now 8 and `common`
compiled unchanged, because the pre-work fix had already made every port array
derive from the constant. The drift guard built in Phase 1 did its job: the
moment the constant moved, `host_tests` failed to compile rather than silently
testing the wrong shape.

**It caught two real bugs that would have shipped**, both the same shape — a
match arm with a wildcard fallback:

- `FieldId::label()` gave *every* port past the third the label `"Group Sz P4"` /
  `"LEDs Port 4"`, so ports 5–8 would have been indistinguishable on the display.
- `FieldId::key()` did the same to the console protocol: ports 5–8 were
  unaddressable and `set leds_4` was ambiguous. This one was caught by the
  pre-existing `console_keys_are_unique` test — 27 fields, 19 unique keys.

Both are now table lookups sized to the port count. Three new tests guard the
class: every port reachable from the menu, per-port labels distinct, and every
page inside the display's row budget.

**Menu pages restructured for the OLED.** Eight ports do not fit the old
two-page layout on a 21×8 character grid, so group size and LED count each split
across two pages of four, giving seven pages of ≤7 fields. `MAX_FIELDS_PER_PAGE`
is now a constant with a test behind it — the menu is *paged, not scrolling*, so
an overflowing page puts fields somewhere the user cannot reach.

**`DMX_BUFFER` resized 256 → 64 universes**, reclaiming the projected ~96 KB.
Sized from the real payload: 8 ports × 600 RGB is 32 universes, so 64 leaves 2×
headroom for the configurable base offset.

⚠️ **This needed a bounds guard, not just a smaller number.** `SubUni` is a full
byte off the wire (0..=255) and the Art-Net task indexed the buffer with it
directly. At 256 universes that happened to be in range; at 64 an out-of-range
packet would have panicked the firmware — **remotely, from the network**. The
task now drops those packets and logs once. The router's read path already
clamped, so it needed no change.

**RAM: 61.8% → 43.0%**, and that is *with* eight ports doubling `LED_COLORS`.

**OLED menu — rendering, and the flagged risk is resolved.** The plan called out
"verify early that `mousefood` renders to a `BinaryColor` DrawTarget" as
something that could invalidate the UI approach. **It does** —
`impl From<TermColor<'_>> for BinaryColor` in mousefood's `colors.rs`, and the
whole stack now typechecks: ssd1306 → display-interface-spi → mousefood →
Ratatui → `BinaryColor`.

Two things settled while building it:

- **Grid is 25×8, not 21×8.** `FONT_5X8` on 128×64 gives 25 columns and 8 rows —
  one for the page title and seven for fields, which is exactly
  `MAX_FIELDS_PER_PAGE`. The page restructure was sized correctly by luck as
  much as judgement; the constant and its test now say *why*, and the extra four
  columns give labels and values more room than assumed.
- **Blocking SPI, deliberately.** `display-interface-spi` 0.5 has no async
  variant, so ssd1306's `async` feature has nothing to sit on. A full redraw is
  ~1 ms at 8 MHz. LED and DMX timing is carried by PIO and DMA in hardware, so a
  late poll shifts scheduling rather than output — a few percent of one core at
  realistic redraw rates. Revisit only if jitter shows on the bench.

The menu navigates and displays live settings. **On-device editing is not ported
yet** — digit entry and the copy-on-edit commit transaction still live in
`nucleo/src/ui/mod.rs`; until they move, settings change over the console only.

**Byte-budget enforcement — done.** `bytes_per_port`, `universes_per_port`,
`total_universes` and `over_budget` on `SmartLedSettings`, surfaced as a
read-only `Universes` field showing `32/64` or `32 OVER`. Over budget is not an
error — it is a silently lower refresh rate, which is exactly the kind of thing
worth saying out loud. Five tests encode the budget arithmetic, including the
claim the whole framing rests on: **RGB 600 and RGBW 450 are byte-for-byte
identical** on ports, universes and frame time.

Fixing this turned up a third instance of the 4-port wildcard pattern:
`UniverseOffsets` formatted `o[0]..o[3]` with a fixed string, so it displayed
half the ports and silently hid the rest.

**EEPROM event task — ported.** Settings now persist and reload; the task came
from the STM32 build with its I²C operations made async. It runs on core 0
beside the router that feeds it, sharing I²C1 with the expander on core 1 —
which is what the `CriticalSectionRawMutex` on that bus was always for.

**Boot gate simplified, and the schema byte moved.** The plan called for
repurposing `0x01`, but the router still uses `ReadModuleType` as its EEPROM
liveness check — and in a one-board design *that is* the simplified gate: the
read now means "the EEPROM is present and readable" rather than "which module is
fitted". So `0x01` keeps its job and the schema version moved to **`0x11`**.
Overloading one byte with two meanings is how this sort of thing goes wrong
later.

**On-device editing — done.** A copy-on-edit transaction: adjustments land on a
private draft and only a completed edit reaches the router, which merges that one
field. Esc discards, so a half-finished edit cannot leak into stored settings.
The digit arithmetic and enum cycling were already in `common::ui::fields`, so
this is the state machine around them.

**sACN / E1.31 — done.** Parser in `common::sacn` (host-tested, 6 tests) plus a
receiver task joining 32 multicast groups. The multicast angle is the point:
with an IGMP-snooping switch the unwanted universes are dropped *upstream* and
never reach the W6300's SPI link — a better answer to the ingest problem than
unicast Art-Net, which still has to arrive before it can be discarded.

Two parser details worth keeping: the property count is attacker-controlled, so
slots are clamped to the datagram that actually arrived rather than the length it
claims; and a non-DMX start code (RDM, text) is rejected rather than written into
a universe as if it were channel data.

**Deferred deliberately:** a dedicated `InputMode::Sacn` and the source
arbitration sACN's `priority` field enables. Both change the encoded shape of
`MenuData` — a schema change, and the first real exercise of the version byte.
sACN is currently gated on the same network modes as Art-Net.

**Phase 3 is complete.** Footprint: 357 KB flash (17.0%), 233 KB RAM (44.5%).
`pico2/` is 2,829 lines; 44 host tests passing.

### Phase 1 outcome (done)

Extracted `common/`: `event_router` (458), `ui/types` + `ui/fields` (751),
`tiny_artnet` (547), `ansi` (162), `enttec_protocol` (144), `events` (138),
`channels` (81), `constants` (53), `buffers` (49).

- **Event enums were the blocker.** `ButtonEvent`, `UiEvent`, `EepromEvent` and
  friends were defined inside the HAL-bound files that send them, which is why
  `event_router` could not move despite having no HAL reference itself. They are
  now in `common::events`, with each original file re-exporting so
  `crate::button::ButtonEvent` still resolves and no call sites changed.
- **Concurrency landmine defused.** The shared buffers and channels used
  `ThreadModeRawMutex`, which is `cortex-m`-only (so it cannot cross into a crate
  that also builds for the host) *and* **unsound across two cores** — and the
  planned core-0 / core-1 split runs straight through `LED_COLORS` and
  `DMX_BUFFER`. Switched to `CriticalSectionRawMutex`, correct either way; the
  cost is a masked interrupt per lock, immaterial at 44 Hz.
- **`host_tests` is no longer a workaround.** It existed only because the code
  was trapped in a `no_std` *binary* crate, so it recreated the module tree and
  pulled sources in by `#[path]`. It now depends on `common` like any other
  crate, and its duplicated `SMARTLED_PORT_COUNT` is gone.

Verified: `common`, `nucleo` (thumbv8m), `rp2040_dmx` (thumbv6m) all check clean;
`host_tests` 30/30 and `dmx_console` 5/5 green; zero warnings.

### Pre-work outcome (done)

Both planning gotchas were measured rather than assumed, by copying `host_tests`
into a scratch crate and bumping `SMARTLED_PORT_COUNT`.

1. **ArtPollReply — already correct.** `artnet/mod.rs:187` sends to `from_addr`,
   the endpoint the ArtPoll arrived from, so it is already unicast per Art-Net 4.
   No change needed — but it must stay that way.
2. **EEPROM — wrong guess, interesting result.** Measured worst-case `MenuData`:
   **40 bytes at 4 ports, 64 bytes at 8**. It does not overflow the 64-byte slot,
   it lands on it *exactly* — worse than an overflow, because
   `assert!(length <= SETTINGS_SIZE)` passes at 64 <= 64 and keeps passing until
   the next field pushes it over on a real EEPROM. Fixed:
   - `SETTINGS_SIZE` 64 to 128 (ends at 0x9F, clear of the 256-byte part).
   - `WriteSettings` now writes `slice[..length]` instead of the whole reserved
     buffer, so the bump costs nothing — it previously spent a ~5 ms EEPROM write
     cycle per page whether or not the page held data.
     **This touches device write behaviour and wants bench confirmation.**
   - The regression test now asserts 32 bytes of *margin*, not just a fit.
3. **Latent bug found on the way.** `SmartLedDmxGroupSize` was declared
   `[u16; 4]` with a `[1, 1, 1, 1]` default — the only port array in the codebase
   not driven by `SMARTLED_PORT_COUNT`. Genericised; behaviour identical at 4
   ports, landmine removed before the 4-to-8 migration.

Deliberately **not** done yet: bumping `SMARTLED_PORT_COUNT` to 8. That breaks
the 4-port SPI output path and the UI field table — it belongs with Phase 3.

---

## Phase detail

- **Phase 0 — gate.** Bare W6300-EVB-Pico2, flying leads, no PCB. Bring up PIO
  SPI, then `embassy-net-wiznet` MACRAW, then DHCP, then Art-Net RX. Flood it and
  **measure the actual universe ceiling**. Record the number here. Commit the PCB
  only after it is known.
- **Phase 1 — done.** See above.
- **Phase 2.** PIO WS2812 (8 ports) and PIO DMX in/out; dual-core split
  (**core 0** = W6300 + smoltcp + Art-Net/sACN + router; **core 1** = OLED UI +
  buttons + DMX + LED render); retarget the HAL-bound files per the worklist.
- **Phase 3.** 4-to-8 port settings model, EEPROM resize, OLED UI redesign
  (128x64 mono is a 21x8 char grid vs. today's 35x11 — see `UI_PROPOSALS.md`;
  verify early that `mousefood` renders to a `BinaryColor` DrawTarget), sACN with
  smoltcp IGMP. Plus three items that fall out of the decisions above:
  - **Enforce the byte budget, not a LED-count constant.** `SMARTLED_NUM_LEDS_MAX`
    alone cannot express "600 RGB or 450 RGBW"; the UI should compute total bytes
    per port and flag when the configuration exceeds the budget Phase 0 measures.
    The LED 2 tab already shows computed universe offsets, so this is one more
    derived read-only field.
  - **Schema version byte** at EEPROM `0x01`, replacing `ModuleType`.
  - **Simplify the boot gate.** Today output is blocked until a working *module*
    EEPROM is read, because it identified the output module. With one board that
    reasoning is gone — the gate should now mean "settings are readable", and the
    Ethernet-lockout-on-failed-boot logic stays as-is.
- **Phase 4.** QSPI when PR #5809 merges — swap the transport behind the same
  `embassy-net-wiznet` device; nothing above it changes.

---

## The Ethernet failure — undiagnosed, and now permanently so

The dead Nucleos have been disposed of, so **the post-mortem cannot happen and
the failure mechanism will never be established.** That is worth stating plainly
rather than quietly dropping, because it changes what Rev 2 has to do.

### What we know and what we do not

Multiple boards failed the same way — permanent Ethernet loss, unrecoverable by
power cycle, MCU otherwise fine. Repeated failures across boards points at a
systematic cause, not bad luck. Two candidates were never separated:

| Candidate | Rev 2 status |
|---|---|
| **Carrier-side 3V3 rail contention** — two regulators fighting on one node through `R?`, with no ORing | **Removed by construction.** The VSYS scheme deletes U11, U14 and `R?` outright; nothing back-drives anything. |
| **Cable-borne surge / ESD / ground-potential difference through the RJ45** | **Not addressed.** The W6300 has an integrated PHY behind a magjack with no surge protection — the same exposure the LAN8742A had. |

### Mitigations, since diagnosis is off the table

1. **Socket the module** (already specified). A failure becomes a ~$15,
   30-second swap instead of scrapping a carrier. This is the main insurance and
   it costs nothing extra.
2. **Inline Ethernet surge protector on installs.** External, cheap, no PCB
   change. Worth it in any venue where the switch is on a different circuit from
   the DC brick — which in stage lighting is most of them.
3. **A chassis-ground provision near the Ethernet end**, mirroring the `R1`
   tuning point the DMX side already has: pad for 0 Ω / cap / not-fitted, so a
   suspected ground loop can be broken or bonded without cutting traces.
   Cheap now, impossible to retrofit.
4. **Make Rev 2 self-diagnosing** — see below. If it fails again, we want data
   this time.

### Instrumenting for the next failure

This is the one genuine advantage of the W6300 over the LAN8742A: **a dead
Ethernet chip is detectable in firmware.** The W6300 is a hardwired TCP/IP
controller on the far side of an SPI link, so the driver can read its version
register at boot and periodically. With the Nucleo's PHY you had to reach for a
scope; here the board can say so itself.

- Read the W6300 version/ID register at boot. On mismatch, log
  `ETH_CHIP_NOT_RESPONDING` rather than silently failing to get DHCP.
- Record Ethernet init failures to the EEPROM alongside the existing boot flag,
  and surface them on the OLED and the USB console. The 0xA0–0xFF region is free.
- Distinguish *chip dead* (SPI reads garbage) from *link down* (chip responds,
  PHY status says no link) from *no DHCP* (link up, no address). Those three
  point at completely different causes, and today they all look the same.

The rail test points and the power-transient scope check in the verification
list matter more now, not less — they are the only remaining way to catch a
carrier-side cause before it does damage.

### Rev 2 is itself the experiment

If the rebuilt design runs without failures, the 3V3 contention was almost
certainly the cause and the VSYS scheme fixed it. If Ethernet dies again, it is
cable-borne, and the answer is protection at the RJ45 — which realistically
means taking the connector off-module onto a carrier-mounted magjack with TVS on
the MDI pairs, i.e. a bare-W6300 design. That is a much bigger layout job
(differential pairs, magnetics), so it is the right thing to defer until there is
evidence it is needed.

**If a failure does happen, record the circumstances:** what was plugged in and
in what order, whether Ethernet was hot-plugged, whether the switch and the brick
shared a circuit, cable length and route, and whether anything else on the run
failed at the same time. That context is now the only diagnostic input available.

## Verification checklist

**Host (runnable now)**

- [x] `cd host_tests && cargo test` — settings model, field metadata, EEPROM
      encode margin, Enttec framing, universe-offset math
- [x] `cd dmx_console && cargo test` — console line parser
- [ ] Re-run both after the 4-to-8 port bump (Phase 3)

**Bench (needs hardware)**

- [ ] Phase 0: flood with Art-Net (broadcast *and* unicast) to find the
      single-SPI universe ceiling — **record the number in this file**
- [ ] 8 strings x 600 LEDs at 44 Hz sustained; scope one WS2812 line and confirm
      the 18 ms frame
- [ ] **UI stays responsive on core 1 while core 0 is saturated with Art-Net** —
      the specific claim this architecture rests on
- [ ] DMX in and out against a real console and fixture; direction switching in
      `ArtNet>DMX` and `USB>DMX`
- [ ] USB: Enttec widget enumerates and drives from PC lighting software, with
      console and logger interfaces in the expected port order
- [ ] EEPROM settings save/load after the partial-page write change (pre-work #2)
- [ ] W6300 version-register read reports correctly, and the three Ethernet
      failure states (chip dead / link down / no DHCP) are distinguishable in the
      log — the only diagnostic path left for the undiagnosed failure
- [ ] **Measure PS1's actual 3V3 input current** — the 90 mA figure is derived
      from its output budget and an assumed efficiency, not from a datasheet
- [ ] Confirm the UI, EEPROM and DMX all come up on USB-only power (no brick)
- [ ] Power-sequencing: scope 3V3 (carrier) vs 3V3_OUT at power-on — carrier must
      lag the module (the R61 EN gate working); repeat on power-down
- [ ] Power: scope VSYS and 3V3(OUT) through power-on, power-off and a
      full-brightness LED step, with and without USB attached — this transient
      behaviour is what may have been killing Nucleos, so it gets measured
- [ ] Buttons: chord detection, and `INT` re-arms after a read (the classic
      expander bug is a latched interrupt that never clears)

---

## Reference links

- [W6300-EVB-Pico2 pinout](https://docs.wiznet.io/Product/Chip/Ethernet/W6300/w6300-evb-pico2)
- [embassy-net-wiznet docs](https://docs.embassy.dev/embassy-net-wiznet/0.3.0/default/index.html)
  (note: "W6300 (Single SPI only)")
- [embassy issue #4662 / PR #5809 — W6300 QSPI driver](https://github.com/embassy-rs/embassy/issues/4662)
- [`embassy_rp::pio_programs::spi`](https://docs.embassy.dev/embassy-rp/git/rp235xa/pio_programs/spi/index.html)
- [Raspberry Pi Pico 2 datasheet](https://datasheets.raspberrypi.com/pico/pico-2-datasheet.pdf) (VSYS / RT6150)
- [TCA9555 datasheet](https://www.ti.com/lit/ds/symlink/tca9555.pdf)
- [Art-Net 4 specification](https://art-net.org.uk/downloads/art-net.pdf)
- [UM3115 — NUCLEO-H563ZI user manual](https://www.st.com/resource/en/user_manual/um3115-stm32h5-nucleo144-board-mb1404-stmicroelectronics.pdf)

---

## Change log

| Date | Change |
|---|---|
| 2026-08-24 | Plan created; pre-work and Phase 1 completed |
| 2026-08-24 | PS1 confirmed 3V3-in; carrier 3V3 moved to an LDO on VSYS; diagrams added |
| 2026-08-24 | EEPROM kept, moves to carrier; RGB/RGBW reframed as a byte budget; spare I/O to test points; per-port universe math corrected |
| 2026-08-24 | GP23 confirmed unavailable (internal SMPS PS pin) — GP13 is the only spare GPIO |
| 2026-08-24 | LDO selected: TLV75733PDYDR. NCP1117LP rejected — ~1.25 V dropout even at 10 mA, and needs ESR ≥ 20 mΩ |
| 2026-08-24 | Parts fixed: TCA9555PWR, M24C02-WMN6TP (SO8N also takes M24256), SN74ACT245 |
| 2026-08-24 | TXS0108E and ICL3245 evaluated as level shifters and rejected — see the WS2812 section |
| 2026-08-24 | Dead Nucleos disposed of — post-mortem cancelled, failure mechanism permanently unknown; shifted to mitigation + firmware instrumentation |
| 2026-08-24 | Workspace on current embassy (rp 0.10 / sync 0.8 / net 0.9.1); nucleo frozen, rp2040_dmx migrated |
| 2026-08-24 | **Phase 3 complete** — byte-budget enforcement, EEPROM task, boot gate simplified (schema byte moved to 0x11), on-device editing, sACN with IGMP |
| 2026-08-24 | Phase 3: OLED menu rendering — mousefood/BinaryColor risk resolved, grid confirmed 25x8 with FONT_5X8; navigation works, editing still to port |
| 2026-08-24 | Phase 3: 8-port model in (caught label + console-key wildcard bugs); pages restructured for 21x8; DMX_BUFFER 256→64 with a bounds guard on SubUni; RAM 62%→43% |
| 2026-08-25 | Diagram review: fixed stale 3V3 rail label + EEPROM map in ui-i2c; DMX bias/term corrected to Rev 1 ground truth (one 120R behind JP38); THRU pin-1 note; all three opto resistors 470→270; power chain now fuse→TVS crowbar, series P-FET dropped |
| 2026-08-25 | **Firmware review (board-informed) — 16 findings fixed.** CRITICAL: boot ladder ported from nucleo (new `boot_task`: EEPROM reads → boot-flag arm → net gated on lockout guard + `ethernet_enabled` → Success write w/ retry+confirm — without it the router never saw Success and ALL LED output was blocked, and settings never loaded); OLED init raced the expander reset (blocking SSD1306 init ran while RES held low → dead display; fixed with an `OLED_READY` Signal handshake). HIGH: wired-DMX task now mode-gated like every other input (was stomping universe 0 in Art-Net mode; feedback Watch 3→4 receivers); Art-Net/sACN one-channel address skew fixed (network universes are 0-based in the buffer, wired frames start-code-aligned — router now uses per-mode index); EEPROM schema byte now written after every settings save and enforced on load (was defined but disconnected). MEDIUM: DMX RX accepts short universes via DMA stall detection + start-code filter + tail zeroing (fixed 513-slot read never completed on short frames and misaligned on BREAKs); RGBW warns once instead of per-vLED-per-frame error spam; Art-Net RX buffer 64 KB→1.5 KB; configured sub-net bounded to 0..=3 in the UI + `buffer_base()` clamp (raw value could slice-panic the USB forwarder); button debounce 8 ms after INT edge (phantom press-bounce releases); ArtPollReply reports current DHCP address. LOW: `refresh_bus` no longer writes reserved I2C address 0xFE; stale comments fixed (critical-section note, port-count, RP2040-bridge refs, main footer); VID/PID flagged as placeholder. pico2 + common + host green, 45 host tests (+2 new: buffer_base clamp, sub-net UI bound); nucleo crate has pre-existing embassy-stm32 drift (frozen, boards discarded) |
| 2026-08-25 | **PCB started**: tools/make_pcb.py generates DMX Interface Rev 2.kicad_pcb — 116 footprints on a spaced non-overlapping grid grouped by sheet, 103 nets bound to pads, schematic UUID paths for Update-from-Schematic. Found & fixed on the way: Altium-import pad axes transposed on TLP2368 SO-6 + THVD1400 SOIC-8 footprints (self-overlapping pads, defect also present in Rev 1's board file); render_cache absolute-coordinate junk and User.N layers cleaned from all extracted footprints. DRC clean except expected pre-layout items + 3 warnings inherited from Rev 1's L2 inductor footprint |
| 2026-08-25 | Footprint audit: 116/116 components footprinted, all 33 unique footprints verified to resolve on disk; no passives under 0603; every IC in a leaded package (SOIC/TSSOP/SOT — no QFN/DFN/SON). Polyfuses stay 1812 chips (leaded preference clarified as ICs-only; two-pin SMT parts are fine — a brief radial-polyfuse swap was reverted). Netlist bit-identical, ERC 0 |
| 2026-08-25 | rev2-dmx-isolated.svg: fail-safe direction drive now drawn as a schematic panel (R46/LED chain, Q1 with drain off EN_LED, R36→gate, R47 hold-down, behavior notes) instead of text-only; bottom panels resized to fit their text. KiCad re-verified: generator re-run byte-identical, ERC 0 errors, 25-point netlist checklist green (fail-safe nets, all sixth-review fixes, opto drive, CHGND bond, EN gate, diode-OR, I2C addressing) |
| 2026-08-25 | **DMX fail-safe BLOCKER closed** (Phil's circuit, refined): Rev 1 confirmed to have never tested TX (JLC SMT BOM lacks TLP2368) — no heritage constraint on the EN chain. Rework: U14 LED biased ON from 3V3 via new R46 220R (cathode to GND) = RECEIVE whenever GP10 is undriven (boot/reset/unflashed/crash); GP10 drives Q1 2N7002 (gate via R36 — reused as gate series R — plus R47 100k hold-down) which grounds the anode node `EN_LED` = LED off = TRANSMIT (15 mA from 3V3 only while transmitting). Firmware convention unchanged (HIGH = TX); GP10 12 mA drive-strength requirement dropped in dmx.rs. Implemented as generator surgery on the Altium sheet (drops #PWR0143 + 2 wires by exact match, asserts on drift); ERC 0; netlist diff = exactly {Net-(U14-K) removed; EN_LED, EN_GATE added; 3V3 −U14.1 +R46.1; GND +U14.3 +Q1.2 +R47.2}; DMX.EN net untouched. Diagrams + bring-up stage 4 + READMEs updated |
| 2026-08-25 | Sixth review (from scratch, 3 independent reviewers + full net-walk): FIXED — U10 A-input pull-downs R63-R70 (floating inputs w/ module absent), R61/R62 0R→10k (bench-jumper misassembly shorted VSYS into module 3V3), D2→SMDJ5.0A/SMC (crowbar sized to blow the fuse), F1 value MINI-blade 10A (holder mismatch), PF9 logic-branch polyfuse, R39→220R (RX opto worst-case VF margin), TP8-10, C17 DNP bulk, R71 1M GNDISO bleed, stray Rev 1 DNP annotation reworded. OPEN DECISION — DMX fail-safe is transmit when MCU Hi-Z (all 3 reviewers; fix = flip U14 LED sense + iso-side inverter, firmware contract preserved, hand-edit in KiCad). Open question: were Rev 1 TX/EN optos ever fitted (JLC SMT BOM lacks TLP2368)? |
| 2026-08-25 | Fifth review (pre-layout close-out): TLV757P internal EN pull-down confirmed — module-out leaves LDO deterministically off (no extra resistor needed); EN thresholds corrected to guaranteed 1.0 V/0.3 V; module confirmed 21x51 with low-profile RJ45 inside the outline (socket footprint end-labels fixed: USB at pin 1, RJ45 at pin 20/21); D1 thermal note; BOM swept — 61 lines, all footprinted; layout-notes section added to project README |
| 2026-08-25 | Fourth review (pre-layout, datasheet-verified): EN-sequencing gate added (R61/R62) fixing ~15 mA back-injection into unpowered RP2350 pads at every boot; TLP2368 confirmed open-collector/5 mA threshold; TCA9555 100k pull-ups confirmed; AHCT245 inputs 7 V VCC-independent; 1S7BE identified (GAPTEC, 2.97–3.63 V in, 303 mA); new rev2-power-bringup.svg with 5-stage bring-up procedure |
| 2026-08-25 | Third review (equivalence + integration): DMX subcircuit formally verified IDENTICAL to the as-built Rev 1 board netlist (component-for-component, modulo the three intended value changes; R60 exactly recreates board R1). Cross-domain flaw fixed: 220R opto LEDs need ~8 mA sink but RP2350 pads default to 4 mA — GP9/GP10 now set to 12 mA drive in firmware. rev2.kicad_sym shipped (77 lib warnings cleared); ferrite MPN pinned (MPZ2012S601AT000) |
| 2026-08-25 | Second review (footprint layer): J2/J3 socket merged to one 40-pin part with custom DIP-numbered footprint (old J3 pins 21-40 had no pads — board-killer); 11 Rev 1 footprints recovered from the Rev 1 PCB (the .pretty was empty); DYD thermal-pad footprint created (verify vs DYD0005A); all IC pinouts verified against KiCad std symbols; footprint-pad audit ALL CLEAN, ERC 0 |
| 2026-08-25 | Schematic design review: fixed TX/EN opto under-drive (R36/R37 330→220R, inherited from Rev 1 — §3.2 only caught RX), reverted my mistaken R40/R41 change (output pull-ups, not LED resistors), bonded floating CHGND (R60 0R + C16 DNP — the shell tie lived on a Rev 1 sheet not carried over), fixed unbuildable cap footprints. Netlist-diff verified; ERC 0 |
| 2026-08-25 | KiCad project created in dmx_interface_rev2_kicad/ — 5 new sheets + Rev 1 DMX sheet carried over verbatim; kicad-cli: 0 ERC errors, netlist audited against the firmware pin map |
| 2026-08-24 | **Phase 2 complete** — Art-Net + router + LED render wired, dual-core split done, USB ported. OLED folded into Phase 3 |
| 2026-08-24 | M24C02 ported to async and wired; boot reordered so the EEPROM MAC precedes Ethernet; 0x01 repurposed as schema version |
| 2026-08-24 | Wired DMX on PIO2 (bridge deleted) and TCA9555 buttons compiling; 11/12 PIO SMs in use as predicted |
| 2026-08-24 | W6300 transport compiling: PIO SPI → MACRAW → embassy-net, with chip-dead vs bad-version logging |
| 2026-08-24 | Phase 2 started: `pico2/` crate scaffolded and booting-image-valid; 8× WS2812 on PIO compiles, confirming the PIO budget |
| 2026-08-24 | **Hardware design questions all closed.** Remaining hardware work is the dead-Nucleo post-mortem, Phase 0 bench gate, then layout |
