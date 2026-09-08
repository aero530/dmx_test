# DMX Interface Rev 2 — Board Bring-Up Checklist

Everything that has to be **verified on the bench** before a Rev 2 unit is trusted.
Only checks — the reasoning behind each lives in [REV2_ALTIUM_REVIEW.md](../../dmx_interface_dev_board_v2/docs/REV2_ALTIUM_REVIEW.md)
(board), [ARCHITECTURE.md](ARCHITECTURE.md) (firmware) and
[rev2-power-bringup.svg](../../dmx_interface_dev_board_v2/docs/rev2-power-bringup.svg) (sequencing). Work top
to bottom: each stage assumes the one before passed.

Log everything with `DEFMT_LOG=debug cargo run --release` (probe-rs on the module's
SWD pads) until stage 6; the USB console (`dmx_console`) is enough afterwards.

Legend: **Do** what to apply · **Expect** what a good board shows · **Fail →** what it means.

---

## Stage 0 — Before first power (both boards, no module fitted)

- [ ] **Visual**: every IC leaded and oriented; no bridges on the SSOP-28 (U2), TSSOP-24 (U12), TSSOP-20 (U10); electrolytic polarity on all 470 µF and C37 (stripe = negative, away from the `+` silk).
- [ ] **Build variant stuffing** matches the intended rail voltage (one voltage per board):
  - 5 V: L2 fitted, U6 **not**; D1 = SMDJ5.0A; F10–17 = 0ZCF0300BF2C; C2 is a **50 V** part.
  - 12 V: U6 fitted, L2 **not**; D1 = SMDJ12A; C2 50 V.
  - 24 V: U6 fitted, L2 not; D1 = SMDJ26A; F10–17 = the 24 V-variant PTC (≥ 30 V rating confirmed on the datasheet); C2 50 V.
- [ ] **USB power path** (R3-D1 direction, 2026-09-03): U22 TPS2553-1 + F19 2 A PTC fitted on **5 V builds only** (F19 not fitted on 12/24 V — U22 is a ±7 V part); R83 = 18k on ILIM (≈ 1.45 A), R82 FAULT pull-up to 3V3, R76 EN pull-down fitted; D5 anode on `V_BUS_FTDI` via F20 (**never** on the U22 output node); present-detect dividers 10k top / 20k bottom. On boards still carrying the old circuit: **R35 DNP** (fitted = the USB ports power the strips — bench only).
- [ ] **Ohmmeter, unpowered**: `V_LED`–GND > 1 kΩ (D1 not shorted, no bridge across the plane); `5V_LOGIC`–GND and `3V3`–GND not shorted; `CHGND`–GND ≈ 0 Ω (R4 fitted); `3V3ISO`–GND **open** (isolation barrier intact); each XLR shell–GND ≈ 0 Ω through PTH3/R4.
- [ ] **Power board**: `V_LED`–GND open at J3/J6 with no fuses fitted; each 3568 holder's fused pin isolated from the plane with the insert out; polarity silk at J3/J6 unambiguous.
- [ ] **Protection present on the power board** (per the review): TVS on the input plane, MAXI main-fuse holder in series with J3 (or the harness fuse documented), inter-board feed to main-board J26 fused.

## Stage 1 — Power, no module (main board)

Do: main supply on at the build voltage, nothing else connected, **no module in the socket**.

- [ ] `V_LED` = supply voltage; `V_LOGIC_IN` ≈ `V_LED`; `5V_LOGIC` = 5.0 V ± 0.25 (5 V build: through L2; 12/24 V: U6 output).
- [ ] `VSYS` ≈ 5V_LOGIC − 0.3…0.45 V (B340A drop).
- [ ] **3V3 = 0 V.** The LDO is enable-gated by the module's 3V3_OUT; with no module it must stay off. Fail → JP3 bridged 2-3 (bench override) or R26/R31 wrong.
- [ ] Idle current < 5 mA on the logic branch.
- [ ] Reverse-polarity sanity (**power board only, current-limited supply ≤ 1 A**): reversed input trips the TVS/fuse path, no cap venting. Skip on the main board (J26 is keyed).

## Stage 2 — Module fitted

Do: W6300-EVB-Pico2 in the socket, supply on. Nothing on the strips or USB yet.

- [ ] Rise order on a scope: `3V3_OUT` (module) up before carrier `3V3`; carrier 3V3 ≥ 0.2 ms behind. Fail → sequencing gate not working; check JP3 1-2.
- [ ] `3V3` = 3.30 ± 0.05 V; `3V3ISO` = 2.97–3.63 V (PS1).
- [ ] Supply current 250–450 mA at 5 V with Ethernet linked (module + TFT backlight).
- [ ] Heartbeat: module user LED blinks once per second.
- [ ] Log shows `DMX interface starting on RP2350`, no `previous reset: WATCHDOG TIMEOUT`, `TCA9555` and `PCA9633` init without error, `TFT: up, 35x11 grid`.
- [ ] **D1 temperature** after 10 min: warm, not hot (< 40 °C rise) — it carries the full logic+TFT current.
- [ ] **VSYS monitor** log line ≈ 4.55–4.7 V (5 V build) and no low-VSYS warning.
- [ ] TCA9555 P04 reads back as an input (hi-Z) after init, P05 = 0 and P06 = 0 with nothing on either USB port; `USB_LED_EN` (J21) = 0 V (R76) and `USB_LED_FLT` = 3.3 V with nothing on the FTDI port; P07 reads 1.

## Stage 3 — Display and front panel

- [ ] **Straps first**: JP4 bridged **1-2 (GND)** so the TCA9555 answers at 0x20 (A1/A2 are R29/R30 to GND); firmware probes 0x20 only. 2-3 would put it at 0x21 and the panel stays dark.

- [ ] Backlight comes on **after** the menu is drawn (no visible power-on noise).
- [ ] Orientation: landscape, "Main" top-left, `ETH …` status top-right, no cut-off column — if mirrored or offset, adjust `Rotation`/`display_offset`/`ColorInversion` in `tft_ui.rs`.
- [ ] Colours: white text on black; REVERSED highlight readable. Inverted colours → toggle `ColorInversion`.
- [ ] Buttons: Down/Up move the highlight, Esc changes page, Select enters edit (digit highlight). Each button wired to the right expander bit (J6 pin order: Down, Up, Select, Esc, GND).
- [ ] Digit edit on `Static IP`: the cursor sits on digits, skips the dots.
- [ ] Backlight setting (System page) changes brightness live; value 1 is dim but visible.
- [ ] Full-screen redraw (page change) does not disturb a running LED pattern or DMX reception (core-1 jitter check).

## Stage 4 — EEPROM and provisioning

- [ ] **Strap**: JP5 bridged **1-2 (GND)** so E0 = 0 → M24C02 at 0x56 (E1/E2 are R33/R34 to 3V3). 2-3 gives 0x57 and every read reports `no chip`.

- [ ] Fresh EEPROM boot log: `EEPROM: blank, settings default`, `previous boot Failed`, `1 consecutive incomplete boot(s)` — and **Ethernet still comes up** (guard needs two).
- [ ] Second boot: `previous boot Success`, counter cleared, no guard message.
- [ ] Change a setting on the panel, power-cycle: it persists. Change one over the console (`set dmx_address 7`): persists, and the panel updates without a reboot.
- [ ] Console `mac 02:44:4d:58:xx:xx` → `ok`; reboot → `info` shows it and the log says `EEPROM: MAC …`, not `using fallback MAC`.
- [ ] `provision` → `ok`; reboot log shows module type read and schema 3.
- [ ] **Priming writes**: temporarily comment out the two `write_byte_wait` priming calls in `eeprom.rs`, repeat the save test. If it still persists, delete them for good.
- [ ] Pull the EEPROM's SDA (or lift U13) and boot: output **still renders** after the `flag could not be stored` error (local-authority fallback), display shows the menu with defaults.

## Stage 5 — Ethernet

- [ ] DHCP: `ETH dhcp…` then the address on the title row; `info` → `net=Up(…)`. Link LED on the module RJ45.
- [ ] Static: Network page → `IP Mode = Static`, set `Static IP`/`Prefix`; reboot; the title row shows that address, a PC on the same subnet pings it.
- [ ] Boot guard: pull power twice during the first second after power-on; third boot shows `ETH guard` and the log's guard message; a clean boot afterwards restores Ethernet.
- [ ] **W6300 diagnostic**: with the RJ45 unplugged nothing changes; with the chip genuinely dead the log says `ETH_CHIP_NOT_RESPONDING` or `ETH_CHIP_BAD_VERSION` and `ETH no chip` shows — not a hang.
- [ ] **Art-Net discovery**: in QLC+ / a node scanner, the node appears once per 4 bound universes (BindIndex 1…N); `Univ Bound` on the Main page equals the number of universes the controller lists. Change `LEDs Port 1` from 150 to 600 → bound count and reply count rise together.
- [ ] Art-Net data: controller drives universe `net:sub:uni` → port 1 follows; universe +1 → port 2 (Individual mode, ≤ 170 RGB LEDs/port). A span crossing a sub-net boundary (base 0:0:14, 4-universe port) renders correctly.
- [ ] Other-Net traffic (controller on Net 1, node on Net 0): ignored, one `ArtNet: ignoring traffic on Net` line, no storm.
- [ ] **sACN**: mode `sACN`, `sACN Universe = 1`, controller on universe 1 → port 1; change the base to 100 → log shows the re-join, universe 100 → port 1.
- [ ] **Throughput ceiling** (Phase 0 measurement — record the number): flood 32 universes at 44 Hz; watch for dropped frames / `ArtNet socket receive error`; raise `w6300::SPI_FREQ_HZ` while CRC-clean.

## Stage 6 — Wired DMX

- [ ] Receive: console into the male XLR, DMX mode → `dmx 1 16` on the console tracks the faders; log `DMX: receiving`. Unplug → `DMX: no data` within 1 s; replug → `signal restored`.
- [ ] Alternate start codes (RDM traffic from the console) do not disturb channel values.
- [ ] Scope GP9/GP10 in `USB>DMX` with QLC+ on the FTDI port: GP10 **high** while transmitting, low in every other mode; DMX out on the female XLR drives a fixture; BREAK 176 µs, MAB 16 µs, ~43 packets/s.
- [ ] `ArtNet>DMX`: the configured Art-Net universe appears on the XLR output.
- [ ] Undriven state: with the module removed, the RS-485 driver is **disabled** (fail-safe receive; opto LED on).
- [ ] Isolation: `3V3ISO` to `GND` still open with the bus connected; no ground current through the XLR shield.

- [ ] **Short-frame consoles**: with a console configured for fewer than 512 slots (e.g. 24), press buttons on the TFT while a fixture above the console's slot count is patched. Any flicker there is the known short-frame/redraw interaction (ARCHITECTURE §11), not a wiring fault; a 512-slot console must show none.

## Stage 7 — LED outputs

- [ ] Boot: all eight strings dark (blank frame), no flash of stale colour at power-up.
- [ ] Scope one data line: 800 kHz bit rate, 1.25 µs/bit, ≥ 100 µs latch gap; frame time ≈ 30 µs × LED count (18 ms at 600).
- [ ] RGB (WS2812/WS2815 strip): DMX channels 1-3 = R,G,B of LED 1 — **colour order correct** (GRB on the wire).
- [ ] **RGBW (SK6812 strip)**: `Color Mode = RGBW`, channels 1-4 → R,G,B,W of LED 1; white channel lights the white die only; a WS2812 strip in RGBW mode shows the expected garbage (proves the mode switch reaches the wire).
- [ ] Mixed length: `LEDs Port 1 = 10` — the strip beyond LED 10 keeps its last latched value only until the next blank; frame rate rises (shorter DMA).
- [ ] Group size 3 on a port: three physical LEDs per DMX pixel. Mirror mode: all ports identical.
- [ ] 600 LEDs on all 8 ports at 44 Hz for 10 min: no dropped frames, ACT245 cool, `5V_LVL_SHIFT` = 5.0 V.
- [ ] Per-port PTC/blade fuse: short a strip's V+ to GND through a 5 A load → the aux PTC trips (and the blade fuse on the power board opens) with no effect on other ports; PTC recovers when cleared.

### LED Power = USB brick (5 V build only)

- [ ] Menu **System → LED Power** offers `External` / `USB brick`; console `set led_power usb` and `get` agree with the display.
- [ ] With `External`: `USB_LED_EN` = 0 V whatever is plugged into J2. This is the safe default and a PC must never be loaded.
- [ ] With `USB brick` **and no brick attached**: EN stays low (the firmware also requires VBUS present on P05).
- [ ] With `USB brick` and a 5 V brick on J2: EN goes high, `V_LED` ≈ 4.9 V, title row shows `PWR usb`.
- [ ] **Budget**: drive all eight ports to full white. The frame is scaled so the draw stays near 1.3 A — measure it; the switch must not latch. Note the budget only covers ~21 full-white RGB pixels unscaled, so heavy content will visibly dim. That is the intended behaviour.
- [ ] **Fault latch**: short an output briefly (current-limited bench supply). `FAULT` pulls low, firmware logs `USB LED power: FAULT`, EN drops, and it retries ~2 s later. After three failures it logs `staying off until LED Power is changed`, the title row shows `PWR flt`, and toggling the setting clears it.
- [ ] Unplug the brick while enabled: EN drops, no glitch on `VSYS`, logic keeps running from D4/F1.

## Stage 8 — USB

- [ ] **Identity**: `info` on the console shows a MAC of `02:44:4D:xx:xx:xx` when none is programmed, and the USB serial (Device Manager / `lsusb -v`) is 16 hex digits — **different on every unit**. Two units on one PC must enumerate as two COM ports.
- [ ] **FTDI straps**: JP1 and JP2 both bridged **1-2** = straight (U2 TXD ← FTDI.TX net → GP13; U2 RXD ← FTDI.RX net ← GP28). 2-3 on both swaps TX/RX if a board is wired the other way round — never mix.

- [ ] Module USB-C on a PC: two (three with the `usb` feature) CDC ports enumerate; port order = Enttec widget, console (, logger). `dmx_console` connects on the second.
- [ ] Enttec widget over CDC: QLC+ **cannot** see it (expected — FTDI-only stack); xLights/OLA `usbpro` on the serial port can.
- [ ] **FTDI port**: after programming the FT232RNL per [docs/ft232rnl-eeprom.md](ft232rnl-eeprom.md), Windows shows `ENTTEC` / `DMX USB PRO`; QLC+ lists a DMX USB Pro; log `FTDI widget: locked at 250000 baud`; faders drive the LEDs in `USB>DMX` mode.
- [ ] With the box **unpowered**, plugging the FTDI USB-C into a PC does nothing (`RESET#` gate: no enumeration, no back-drive). With the box powered, it enumerates.
- [ ] Baud hunting: open the port at 57600 with a Python Enttec script → `locked at 57600 baud`.
- [ ] **Module USB only** (PSU off, nothing on the FTDI port): `VSYS` ≈ 4.4–4.7 V, `V_LED` ≈ 4.3–4.6 V through D3/F18, P06 = 1, P05 = 0, strips stay dark (F18 is 0.5 A — firmware must not drive them). Fail → strips lit or F18 cycling = the brightness budget / power-mode logic is not honouring the source.
- [ ] **Brick on the FTDI port only** (5 V/3 A, no PD, 3 A-rated cable), `LED Power` still `External`: `USB_LED_EN` = 0 V (R76), `V_LED` = 0 V (switch off — a disabled USB port switch passes nothing), `5V_LOGIC` ≈ 4.65 V via F20/D5, P05 = 1, FAULT high, the unit boots and the display works. Fail → EN high with the setting off = R76 missing / P04 driven high; V_LED at ≈ 4.3 V = something with a body diode was fitted in U22's place.
- [ ] Set `LED Power = USB brick`: `USB_LED_EN` = 3.3 V (P04 driven high), `V_LED` ramps up (soft start) to ≈ 4.9 V, drop across U22 ≤ 150 mV at 1.3 A full-white on two short strips, FAULT stays high, U22 under 40 °C rise; F19 never trips. Push the load to ≈ 1.6 A: U22 latches off within 10 ms and FAULT goes low (P07 = 0); firmware must lower the brightness and toggle EN to recover. Then plug the PSU in (trimmed ≥ 5.2 V) while the brick runs: U22 opens once V_LED is ≈ 135 mV above the brick (FAULT low, `V_LED` follows the PSU, no VSYS glitch on the scope); unplug the brick: nothing changes.
- [ ] **PC on the FTDI port** with the PSU off: enumerates as DMX USB Pro, logic runs from D5 (≈ 0.35 A from the port), `V_LED` = 0 V, strips dark. Note: P05 is 1 for a PC and a brick alike — the board cannot tell them apart, so the `USB brick` setting is the user's declaration; the manual must say a PC port is not a brick.

### Carried over from the design record

- [ ] **The core-split claim**: saturate core 0 with Art-Net while editing on the TFT — the UI must stay responsive. This is the specific claim the two-core architecture rests on.
- [ ] **8 strings × 600 LEDs at 44 Hz sustained**; scope one WS2812 line and confirm the ~18 ms frame.
- [ ] **PS1 input current**: measure the actual 3V3 draw. The 90 mA in the power budget is derived from its output rating and an assumed efficiency, not from a datasheet.
- [ ] **W6300 diagnostics**: version register reads correctly, and the three failure states (chip dead / link down / no DHCP) are distinguishable in the log — the only diagnostic path left for the undiagnosed Rev 1 Ethernet failure.
- [ ] **Buttons**: chord detection (two at once); a quick tap registers at the 20 ms poll; no phantom presses while the EEPROM task is writing on the shared bus.
- [ ] **Power transients**: scope `VSYS` and `3V3` through power-on, power-off and a full-brightness LED step, with and without USB attached. This is the behaviour that may have been killing Nucleos, so it gets measured rather than assumed.

## Stage 9 — Robustness

- [ ] Watchdog: no `WATCHDOG TIMEOUT` reset across 1 h of normal operation (Art-Net + TFT edits + EEPROM saves).
- [ ] Forced fault: hold core 1 (breakpoint in `buttons::run` with `pause_on_debug` temporarily false) → reset within 4 s and `previous reset: WATCHDOG TIMEOUT` in the next boot log.
- [ ] Brown-out: dip the supply to 3.5 V for 100 ms → clean reboot, settings intact, no corrupted EEPROM page.
- [ ] Hot-plug a strip while running: no reset, other ports unaffected.
- [ ] 8 h soak at full load: no reset, no drift in `VSYS`, D1/U9/U10 temperatures stable.

## Stage 10 — Power board at load

- [ ] 40 A through J3/J6 (both screws per pole wired): input band and GND return < 20 °C rise after 30 min (2 oz copper check).
- [ ] Each blade output at 5 A: holder and cap cool; blade opens at a 10 A overload.
- [ ] Hold-up: after power-off the caps bleed down (bleed resistor fitted) — no spark on re-plugging a pigtail.
- [ ] Inter-board feed to J26: 4 × 18 AWG, fused, keyed plug seated; main-board `V_LED` within 0.15 V of the power board's.

---

**Record when done:** measured supply currents (stages 2, 7), the Art-Net throughput ceiling and `SPI_FREQ_HZ` chosen (stage 5), TFT orientation/inversion settings that proved right (stage 3), whether the EEPROM priming writes were removed (stage 4).
