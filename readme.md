# DMX Interface Firmware

Firmware for a modular DMX-512 / Art-Net lighting interface. An STM32 Nucleo board
receives lighting data — wired DMX (via an RP2040 acting as a DMX-to-I2C bridge),
Art-Net over Ethernet, or USB (Enttec DMX USB Pro protocol) — and drives output
modules (WS2812 "smart LED" strings today; PWM planned). The wired DMX port is
bidirectional: in the `ArtNet>DMX` and `USB>DMX` modes the device becomes an
Art-Net node or USB widget *transmitting* DMX. Settings are edited on a TFT menu
UI and persisted to an EEPROM on the output module.

See [BUGS.md](BUGS.md) for the full findings of the 2026-07 code review and
[UI_PROPOSALS.md](UI_PROPOSALS.md) for UI redesign proposals.

## System architecture

```
 DMX-512 bus ──RS-485──► RP2040 (rp2040_dmx)          Art-Net controller
                          PIO+DMA DMX receiver              │ UDP :6454
                          I2C slave @0x33                   │
                              │ I2C1 (3 block reads)        │ Ethernet
                              ▼                             ▼
                        ┌─────────────────────────────────────────┐
                        │  STM32H563ZI Nucleo (nucleo)            │
                        │  DMX_BUFFER (256 universes × 512 B)     │
                        │  event_router → LED_COLORS              │
                        │  ST7789 TFT menu UI · buttons           │
                        │  M24C02 EEPROM (module id + settings)   │
                        └───────┬─────────────────────────────────┘
                                │ 4× SPI (WS2812 waveform)
                                ▼
                         Smart LED output module (4 ports)
```

### Crates / folders

| Folder | Target | Purpose |
|---|---|---|
| `nucleo/` | STM32H563ZI (thumbv8m.main-none-eabihf) | Main firmware: inputs, routing, UI, outputs |
| `rp2040_dmx/` | RP2040 (thumbv6m-none-eabi) | DMX-512 receiver + I2C slave bridge (Rust/Embassy) |
| `DMX_on_pico/` | RP2040 (Arduino) | Original Arduino sketch that `rp2040_dmx` reimplements |
| `pico_stepper/` | — | Currently a duplicate of `DMX_on_pico.ino` (misnamed; no stepper code) |
| `rp2040/` | — | Empty placeholder crate (does not build) |
| `reference/` | — | DMX / app-note PDFs |

All three Rust crates share one cargo workspace; **build from inside each crate's
folder** so its `.cargo/config.toml` (target + runner) applies. Build profiles live in
the workspace root `Cargo.toml` (cargo ignores profiles in member manifests).

## nucleo firmware (STM32)

Embassy async tasks, communicating through `embassy_sync` channels and two global
mutex-protected buffers:

- **`DMX_BUFFER`** — flat 256 × 512-byte universe buffer all inputs write into.
- **`LED_COLORS`** — per-port RGB arrays (4 × 1024) the outputs render from.

Tasks (spawned from `main`):

- **`dmx_task`** (`dmx_i2c.rs`) — the RP2040 bridge interface. In DMX mode it polls
  the bridge every 30 ms over I2C1 (addr 0x33) with three `write_read` block commands
  (0x01 → bytes 0–199, 0x02 → 200–399, 0x03 → 400–512), reassembling the 513-byte
  frame (start code + 512 channels) into `DMX_BUFFER` and notifying the router. In
  the `ArtNet>DMX` / `USB>DMX` modes it instead switches the bridge's port direction
  to output (command 0x20) and pushes the outgoing frame with write blocks
  0x11/0x12/0x13 every 30 ms.
- **`artnet_task`** (`artnet/`) — in the ArtNet modes, receives ArtDmx/ArtPoll/
  ArtSync/ArtCommand on UDP 6454 via a vendored `tiny_artnet` parser, stores universe
  data into `DMX_BUFFER`, answers ArtPoll with an ArtPollReply.
- **`enttec_usb_task`** (`enttec_usb.rs`) — USB CDC device emulating an Enttec DMX
  USB Pro widget (default builds; the `usb` logging feature claims USB instead). PC
  lighting software can send DMX (label 6) which drives the LEDs and, in `USB>DMX`
  mode, the wired DMX output; received DMX/Art-Net frames are forwarded to the PC as
  label 5 packets on change; widget parameter/serial queries (labels 3/4/8/10) are
  answered.
- **`event_router`** (`event_router.rs`) — central hub. Routes settings/EEPROM/UI/
  button events, and on each DMX/ArtNet packet maps `DMX_BUFFER` → `LED_COLORS`
  according to the settings (start address, group size, RGB/RGBW, Individual/Mirror
  port mode, Art-Net universe offsets), then pings the LED task.
- **`smart_led_task`** (`smart_led/`) — renders `LED_COLORS` to 4 WS2812 ports by
  encoding each bit as 4 SPI bits (DMA).
- **`ui_task_spi`** (`ui/`) — ST7789 172×320 TFT (SPI, mipidsi + embedded-graphics),
  4-tab menu; 4 buttons navigate/edit (see `ui/` and [UI_PROPOSALS.md](UI_PROPOSALS.md)).
- **`eeprom_i2c_task`** (`eeprom/`) — M24C02 (256 B) on the shared module I2C bus.
  Layout: 0x01 module type, 0x02–0x07 MAC, 0x10 boot-success flag, 0x20–0x5F settings
  (bincode-encoded `MenuData`).
- **`button_task` / `button_row_task`, `led_task`** — inputs and heartbeat.

Boot sequence: read boot flag / module type / settings from EEPROM → write flag =
Failed → if the previous boot failed, disable Ethernet (lockout prevention) → bring up
Ethernet (DHCP, or the Art-Net 2.x.y.z static scheme derived from MAC+OEM) → spawn
tasks → write flag = Success.

Settings flow: UI Select → `MenuData` → router → EEPROM write → router `StoreSettings`
→ broadcast back to UI and input tasks (via a `Watch`), so every consumer sees the
same committed state.

### Build / flash (nucleo)

```
cd nucleo
cargo build --release
cargo run --release        # flashes + streams defmt logs via probe-rs (ST-Link)
```

Features (see `nucleo/Cargo.toml`): default = `clock_stlink, ethernet`. `usb` switches
logging from defmt/RTT to USB serial.

## rp2040_dmx firmware (DMX ↔ I2C bridge)

Rust/Embassy reimplementation of the `DMX_on_pico` Arduino sketch, extended with DMX
output (see [rp2040_dmx/README.md](rp2040_dmx/README.md) for details):

- **Core 0** — runs the DMX port in the direction selected over I2C. Input: a PIO
  state machine (port of the Pico-DMX BREAK-detect program) receives DMX on GPIO2 at
  1 MHz PIO clock; each packet is DMA'd into a 513-byte frame, and the state machine
  is restarted at the BREAK detector per packet so frames are always aligned; a
  100 ms timeout logs `no data!` like the sketch. Output: a second PIO state machine
  (port of the Pico-DMX output program) continuously retransmits the latest frame
  written by the STM32 on GPIO4 (~43 packets/s), with the RS-485 driver enable
  (GPIO3) raised. Onboard LED (GPIO25) blinks with traffic; a WS2812 pixel (GPIO23)
  shows green = receiving / blue = transmitting / red = no signal.
- **Core 1** — I2C slave (addr 0x33, SDA GPIO6 / SCL GPIO7): read commands
  0x01/0x02/0x03 serve the received frame in three blocks (`write_read` or split
  write-then-read framings both work); write commands 0x11/0x12/0x13 fill the output
  frame in the same block layout; 0x20 + mode selects the port direction
  (0 = input, 1 = output).
- Frames cross cores through a 1-deep Embassy channel / `Watch` (latest-wins) —
  the async equivalent of the sketch's `newDataReady` + inter-core FIFO handshake.
- `log` output goes to a USB CDC serial port (the sketch's `Serial.begin(115200)`).

### Build / flash (rp2040_dmx)

```
cd rp2040_dmx
cargo build --release
# BOOTSEL mode, then:
cargo run --release        # runner = picotool load --update --verify --execute
# or produce a UF2:
picotool uf2 convert -t elf ../target/thumbv6m-none-eabi/release/rp2040_dmx rp2040_dmx.uf2
```

## Hardware pin map (RP2040 bridge)

| Function | Pin |
|---|---|
| DMX RX (from RS-485 receiver) | GPIO2 |
| RS-485 driver enable (low = receive, high = transmit) | GPIO3 |
| DMX TX (to RS-485 driver) | GPIO4 |
| I2C1 SDA / SCL (to STM32) | GPIO6 / GPIO7 |
| WS2812 status pixel | GPIO23 |
| Onboard LED | GPIO25 |

## UI menu (current)

- **Main Menu** — DMX start address, input mode (DMX / ArtNet / ArtNet>DMX /
  USB>DMX), IP mode (DHCP/static), IP (read-only), Art-Net net/sub-net/universe
- **LED Settings 1** — port mode (Individual/Mirror), LED group size per port,
  color mode (RGB/RGBW)
- **LED Settings 2** — LEDs per port (×4), computed universe offsets (read-only)
- **System** — Ethernet enable (takes effect after reboot)

Buttons: Up/Down move the cursor (per digit for numeric fields) or, in edit mode,
change the highlighted digit; Select toggles edit mode and commits to EEPROM; Esc
changes tab.
