# DMX Interface Firmware

Firmware for a modular DMX-512 / Art-Net lighting interface. An STM32 Nucleo board
receives lighting data — wired DMX (via an RP2040 acting as a DMX-to-I2C bridge),
Art-Net over Ethernet, or USB (Enttec DMX USB Pro protocol) — and drives output
modules (WS2812 "smart LED" strings today; PWM planned). The wired DMX port is
bidirectional: in the `ArtNet>DMX` and `USB>DMX` modes the device becomes an
Art-Net node or USB widget *transmitting* DMX. Settings are edited on a TFT menu
UI and persisted to an EEPROM on the output module.

See [BUGS.md](BUGS.md) for the full findings and fixes of the 2026-07 code
reviews (the 2026-07-11 review and the 2026-07-16 follow-up), and
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
| `common/` | any (`no_std` lib) | Target-agnostic core: event router, channels, shared buffers, settings model, menu field metadata, Art-Net parsing, Enttec framing, constants |
| `nucleo/` | STM32H563ZI (thumbv8m.main-none-eabihf) | Main firmware: inputs, routing, UI, outputs |
| `rp2040_dmx/` | RP2040 (thumbv6m-none-eabi) | DMX-512 receiver + I2C slave bridge (Rust/Embassy) |
| `DMX_on_pico/` | RP2040 (Arduino) | Original Arduino sketch that `rp2040_dmx` reimplements |
| `rp2040/` | — | Empty placeholder crate (excluded from the workspace until it has code) |
| `dmx_console/` | host PC | Ratatui console: edit settings + live DMX monitor over the USB console port |
| `host_tests/` | host PC | Unit tests that run `common` on the PC |
| `reference/` | — | DMX / app-note PDFs |

### What goes in `common/` vs a target crate

`common/` holds everything that does not name a peripheral: the event router and
its message types, the inter-task channels, the shared `DMX_BUFFER` /
`LED_COLORS`, the settings model, and the protocol parsers. It has **no HAL
dependency** — not `embassy-stm32`, not `embassy-rp`, not even
`embassy-executor`.

The dividing line is a concrete peripheral type or an executor attribute. A task
signature naming `Spi<'static, Async>` or carrying `#[embassy_executor::task]`
stays in the target crate; the logic it drives and the messages it passes do not.
`event_router.rs` is the worked example — 458 lines of routing in `common`, and a
22-line task wrapper in `nucleo/`.

That split is what makes the RP2350 port tractable, and keeping `nucleo/`
building against `common` is what proves the extraction stayed faithful.

The embedded crates share one cargo workspace; **build from inside each crate's
folder** so its `.cargo/config.toml` (target + runner) applies. `cargo build
--workspace` is not supported: nucleo (single-core) and rp2040_dmx (multicore)
need conflicting `critical-section` features, which workspace builds unify. Build
profiles live in the workspace root `Cargo.toml` (cargo ignores profiles in member
manifests).

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
  data into `DMX_BUFFER` (indexed by the Port-Address sub-net:universe byte — the
  buffer covers one full net, and the router filters on Net), answers ArtPoll with
  an ArtPollReply.
- **`usb_device_task`** (`usb_device.rs`) — composite USB device. Interface 1
  emulates an Enttec DMX USB Pro widget: PC lighting software can send DMX (label 6)
  which drives the LEDs and, in `USB>DMX` mode, the wired DMX output; received
  DMX/Art-Net frames are forwarded to the PC as label 5 packets on change; widget
  parameter/serial queries (labels 3/4/8/10) are answered. Interface 2 is a console
  line protocol (`console_usb.rs`: `get`/`set`/`dmx`/`info`) used by the
  `dmx_console` host app. With the `usb` logging feature a third interface carries
  the `log` output. Port order on the host: Enttec, console, logger.
- **`event_router`** (`event_router.rs`) — central hub. Routes settings/EEPROM/UI/
  button events, and on each DMX/ArtNet packet maps `DMX_BUFFER` → `LED_COLORS`
  according to the settings (start address, group size, RGB/RGBW, Individual/Mirror
  port mode, Art-Net universe offsets), then pings the LED task.
- **`smart_led_task`** (`smart_led/`) — renders `LED_COLORS` to 4 WS2812 ports by
  encoding each bit as 4 SPI bits (DMA).
- **`ui_task_spi`** (`ui/`) — Ratatui UI on the ST7789 172×320 TFT, rendered
  through the `mousefood` embedded-graphics backend (35×11 character grid,
  heap-backed framebuffer). All settings are described once in the `ui/fields.rs`
  metadata table (labels, ranges, digit editing, console keys); the app state
  machine in `ui/mod.rs` handles cursor movement and copy-on-edit transactions
  (Select commits to EEPROM, Esc cancels).
- **`eeprom_i2c_task`** (`eeprom/`) — M24C02 (256 B) on the shared module I2C bus.
  Layout: 0x01 module type, 0x02–0x07 MAC, 0x10 boot-success flag, 0x20–0x5F settings
  (bincode-encoded `MenuData`).
- **`button_task` / `button_row_task`, `led_task`** — inputs and heartbeat.

Boot sequence: read boot flag / module type / settings from EEPROM → write flag =
Failed → if the previous boot failed, disable Ethernet (lockout prevention) → bring up
Ethernet (DHCP, or the Art-Net 2.x.y.z static scheme derived from MAC+OEM) → spawn
tasks → write flag = Success (retried up to 3×; if the EEPROM won't confirm, output
is enabled from local state and the failure is logged — the next boot then reads
Failed and disables Ethernet). DMX→LED rendering only runs after the boot reaches
Success: a working module EEPROM is required for output, since it identifies the
output module.

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
logging from defmt/RTT to USB serial, adding a second CDC interface to the composite
USB device alongside the Enttec widget.

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

## UI menu

Four tabs rendered by Ratatui on the TFT:

- **Main** — DMX start address, mode (DMX / ArtNet / ArtNet>DMX / USB>DMX),
  IP mode (DHCP/static), IP (read-only), Art-Net net/sub-net/universe
- **LED 1** — port mode (Individual/Mirror), color mode (RGB/RGBW), LED group
  size per port (×4)
- **LED 2** — LEDs per port (×4), computed universe offsets (read-only)
- **System** — Ethernet enable (takes effect after reboot)

Buttons: while navigating, Up/Down move the selection (crossing a page edge changes
tab), Select starts editing, Esc jumps to the next tab. While editing, Up/Down
change the highlighted digit (or cycle an enum), Select moves to the next digit and
commits after the last one (written to EEPROM), Esc cancels the edit.

## Host console (dmx_console)

A desktop Ratatui app that connects to the device's USB console port:

```
cd dmx_console
cargo run --release            # lists serial ports (device ports are marked)
cargo run --release -- COM5    # connect (the console is the 2nd device port)
```

Two views (Tab to switch): **Settings** — every firmware setting, Enter to edit and
apply (persisted to EEPROM); **DMX Monitor** — live view of all 512 channels,
refreshed 4×/s. The same field metadata drives the TFT menu, the console protocol,
and this app, so they can't drift apart. Protocol reference:
[dmx_console/README.md](dmx_console/README.md).

## Testing

The firmware itself targets bare-metal ARM, so its hardware-independent logic
lives in the `common` library crate and is tested on the host with a normal
`cargo test`. (`host_tests` used to pull the firmware sources in by `#[path]`
because they were locked inside a `no_std` *binary* crate; since the extraction
of `common` it simply depends on it, so the tests exercise the real crate.)

```
cd host_tests && cargo test     # settings model, field metadata/editing, Enttec framing
cd dmx_console && cargo test    # console protocol line parser
```

Covered: settings-range enforcement and per-digit editing, EEPROM encode size
(regression test for the silently-dropped-saves bug), enum cycling, the virtual-LED
and universe-offset math the router relies on, Enttec message framing including
malformed/oversized input recovery, and console line parsing. Hardware-facing code
(PIO, SPI, I2C, USB transport) is verified on the bench — see the checklists in
[BUGS.md](BUGS.md).
