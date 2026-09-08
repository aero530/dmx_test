# DMX Interface Firmware

Firmware for the **DMX Interface Rev 2**: a W6300-EVB-Pico2 (RP2350 + W6300)
on a custom carrier that takes lighting data in — wired DMX-512, Art-Net or
sACN/E1.31 over Ethernet, or USB (Enttec DMX USB Pro protocol, on either the
module's USB-C or the carrier's genuine-FTDI USB-C) — and drives **eight WS2812
strings** straight from PIO. The wired DMX port is bidirectional: in the
`ArtNet>DMX` and `USB>DMX` modes the box becomes an Art-Net node or USB widget
*transmitting* DMX. Settings live on a 172×320 TFT menu and in an on-board EEPROM.

Design record: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (why the firmware is shaped
this way — core split, data path, budgets, power, the firmware↔board contract),
[docs/BRINGUP.md](docs/BRINGUP.md) (bench checklist). Board design documentation — schematic/BOM/layout reviews for both
PCBs — lives in the hardware repo: [../dmx_interface_dev_board_v2/docs/REV2_ALTIUM_REVIEW.md](../dmx_interface_dev_board_v2/docs/REV2_ALTIUM_REVIEW.md).

## System architecture

```
 DMX-512 in/out ──RS-485 (isolated)──┐        Art-Net / sACN controller
                                     │              │ UDP 6454 / 5568
 PC (QLC+ etc.) ──USB-C──► FT232RNL ─┤ UART0        │ Ethernet
 PC (any)       ──USB-C──► module USB (CDC: Enttec widget + console)
                                     ▼              ▼
                  ┌──────────────────────────────────────────────┐
                  │  W6300-EVB-Pico2   core 0: W6300/smoltcp,    │
                  │                    Art-Net, sACN, USB, FTDI  │
                  │                    widget, router, EEPROM,   │
                  │                    watchdog                  │
                  │                    core 1: TFT UI, buttons,  │
                  │                    wired DMX (PIO), LED      │
                  │                    render (PIO ×8)           │
                  │  DMX_BUFFER 64 universes · LED_COLORS 8×600 RGBW │
                  └───────┬──────────────────────────────────────┘
                          │ 8× WS2812 data (SN74ACT245 level shift)
                          ▼
                 8 strips, up to 600 RGB / 450 RGBW LEDs each
```

### Power sources

Three ways to power a unit (see [rev2-power-tree.svg](../dmx_interface_dev_board_v2/docs/rev2-power-tree.svg)):
the LED Power Board through J26 (full power — strip current stays on the power
board), a plain 5 V/3 A USB-C brick on the **FTDI** connector (low-power installs;
a TPS2553-1 current-limited USB switch (≈ 1.45 A, reverse-voltage cut-off, FAULT on **P07**) and a 2 A PTC feed `V_LED` on 5 V builds, gated by TCA9555 **P04** (driven high to enable), VBUS
present-detect on **P05/P06**), or the module's own USB (0.5 A bench feed). The
brick path is off until firmware enables it; the `LED Power: External / USB brick`
setting, the ≈ 1.3 A brightness budget, the FAULT/EN re-arm and the `PWR usb` title-row flag are the
behaviour is described in [ARCHITECTURE.md](docs/ARCHITECTURE.md) §7.

### Crates / folders

| Folder | Target | Purpose |
|---|---|---|
| `pico2/` | RP2350 (thumbv8m.main-none-eabihf) | **The firmware.** Peripheral setup, PIO programs, executor wiring for both cores |
| `common/` | any (`no_std` lib) | Target-agnostic core: event router, channels, shared buffers, settings model, menu field metadata, Art-Net/sACN parsing, Enttec framing, constants |
| `host_tests/` | host PC | Unit tests that run `common` on the PC |
| `dmx_console/` | host PC | Ratatui console: edit settings + live DMX monitor over the USB console port |
| `rp2040_dmx/` | RP2040 | Bench firmware for the Rev 1 board: DMX TX test, Enttec CDC test, **FT232R emulator** (`ftdi_test`) |
| `nucleo/` | STM32H563ZI | **Frozen.** The previous generation; kept as the record of the `common` extraction |
| `docs/` | — | Design record, firmware review, bring-up checklist, FT232RNL provisioning notes |
| `reference/` | — | DMX / app-note PDFs |

### What goes in `common/` vs `pico2/`

`common/` holds everything that does not name a peripheral: the event router and
its message types, the inter-task channels, the shared `DMX_BUFFER` /
`LED_COLORS`, the settings model, and the protocol parsers. It has **no HAL
dependency**. A task signature naming `Spi<'static, …>` or carrying
`#[embassy_executor::task]` stays in `pico2/`; the logic it drives does not.
`event_router.rs` is the worked example — the routing lives in `common`, a
20-line task wrapper in `pico2/`.

**Build from inside each crate's folder** so its `.cargo/config.toml` (target +
runner) applies; `cargo build --workspace` is not supported (conflicting
`critical-section` features). Build profiles live in the workspace root
`Cargo.toml`.

## pico2 firmware

Embassy async tasks on two cores, communicating through `embassy_sync` channels
and two mutex-protected buffers:

- **`DMX_BUFFER`** — flat 64 × 512-byte universe buffer every input writes into.
  Wired DMX and USB store DMX-style (start code at index 0); the network modes
  store channel 1 at slot offset 0.
- **`LED_COLORS`** — per-port RGBW arrays (8 × 600) the output renders from; in
  RGB mode the white byte is zero and never transmitted.

Core 0:

- **`event_router`** — the hub. Routes settings/EEPROM/UI/button events and, on
  every DMX/Art-Net/sACN packet, maps `DMX_BUFFER` → `LED_COLORS` (start address,
  group size, Individual/Mirror, universe offsets), then pings the LED task.
- **`artnet_task`** — ArtDmx on UDP 6454 into the buffer, indexed by
  sub-net:universe and filtered on the configured Net; answers ArtPoll with
  **Art-Net 4 BindIndex replies** — one ArtPollReply per four bound universes,
  so controllers auto-patch everything the node consumes (the count shown as
  `Univ Bound` on the Main page).
- **`sacn_task`** — E1.31 multicast. Universe *base* (the `sACN Universe` setting)
  lands in slot 0; the 32-group window is joined (and re-joined on change) so an
  IGMP-snooping switch drops everything else upstream.
- **`usb_device_task`** — composite CDC device on the module's USB: Enttec widget
  (label 6 in, label 5 out on change, labels 3/4/8/10 answered) + the console line
  protocol (+ a `log` interface with the `usb` feature).
- **`enttec_uart_task`** — the same widget on the carrier's FT232RNL (UART0, 8N2,
  **baud hunting** across the rates host software uses). This is the port QLC+
  and other FTDI-stack software recognise as a DMX USB Pro.
- **`eeprom_task`**, **`boot_task`** — settings persistence and the boot ladder.
- **`supervisor`** — feeds the 4 s watchdog only while core 1 keeps checking in;
  logs VSYS (module ADC) and warns when the main supply is absent.

Core 1:

- **`dmx_task`** — PIO2 DMX receiver (start-code-0 frames only) and, in the
  `*>DMX` modes, PIO2 transmitter with GP10 driving the RS-485 direction
  (fail-safe: undriven = receive).
- **`smart_led_task`** — eight PIO state machines (`ws2812.rs`), all strings
  clocked out concurrently; each frame is packed as 24-bit GRB (WS2812) or
  32-bit GRBW (SK6812) per the colour-mode setting, and only the configured LED
  count is sent. The frame is packed off the shared buffer so the router is
  never blocked by the render.
- **`ui_task`** (`tft_ui.rs`) — Ratatui on the ST7789 through mousefood, 35×11
  characters; RES/CS on the TCA9555, backlight PWM from the PCA9633.
- **`button_task`** — polls the expander at 50 Hz (debounce by construction) and
  doubles as core 1's watchdog check-in.

### Boot sequence

Read boot flag / module type / settings → count consecutive incomplete boots
(EEPROM `0xA0`) → write flag = `Failed` → bring up Ethernet (DHCP, or the
Network page's static address) unless disabled or **two boots in a row were
incomplete** (the lockout guard; shown as `ETH guard` on the display) → spawn the network tasks → write flag = `Success` (retried; if
the EEPROM will not confirm it, the local result is authoritative so a dead
settings chip cannot black out the lights). A blank EEPROM decodes to the
product defaults: Ethernet **on**, DMX mode, 150 LEDs per port, backlight 200.

### Build / flash

```
cd pico2
cargo build --release
cargo run --release          # probe-rs over SWD (defmt logs at DEFMT_LOG=info)
DEFMT_LOG=debug cargo run --release   # verbose bench session
# USB instead of SWD: hold BOOTSEL, then
picotool load -u -v -x -t elf ../target/thumbv8m.main-none-eabihf/release/pico2
```

### Provisioning a new unit

1. Flash the firmware. First boot: blank EEPROM → defaults, `ETH` comes up on DHCP.
2. Over the USB console (second CDC port): `provision`, then
   `mac 02:xx:xx:xx:xx:xx` (a unique unicast address per unit; applies at the
   next boot). `info` shows `mac=` and `net=`.
3. Program the FT232RNL's EEPROM on the bench — [docs/ft232rnl-eeprom.md](docs/ft232rnl-eeprom.md).
4. Reboot; check QLC+ sees a DMX USB Pro and the display shows the IP.

## UI menu

Six pages on the TFT (title row shows the page and the network state):

- **Main** — DMX start address, mode (`DMX` / `ArtNet` / `ArtNet>DMX` /
  `USB>DMX` / `sACN`), Art-Net net / sub-net / universe, sACN universe, and
  **Univ Bound** (read-only: how many universes this configuration binds, from
  which base — e.g. `32 @ 0:0:0`; the same count the Art-Net replies advertise)
- **Network** — IP mode (DHCP/Static, applies at boot), IP in use (read-only),
  static IP, prefix length, gateway (`0.0.0.0` = none; defaults 2.0.0.1/8)
- **LED** — port mode (Individual/Mirror), colour mode (RGB/RGBW), universes
  used, offsets
- **Groups** — DMX group size per port (×8)
- **LEDs** — LEDs per port (×8, up to 600)
- **System** — Ethernet enable (applies at boot), backlight

Buttons: Up/Down move the selection (crossing a page edge changes page), Select
starts editing, Esc jumps to the next page. While editing, Up/Down change the
highlighted digit (or cycle an enum), Select moves to the next digit and commits
after the last one (written to EEPROM), Esc cancels.

## Host console (dmx_console)

```
cd dmx_console
cargo run --release            # lists serial ports (device ports are marked)
cargo run --release -- COM5    # connect (the console is the 2nd device port)
```

Protocol (one line per command, `ok` / `err <reason>` terminated): `get`,
`set <key> <value>`, `dmx <start> <count>`, `info`, `mac [xx:xx:xx:xx:xx:xx]`,
`provision`, `help`. The same field metadata drives the TFT menu, the console
protocol and this app, so they cannot drift apart.

## Testing

```
cd host_tests && cargo test     # settings model, field metadata/editing, sACN + Enttec framing
cd dmx_console && cargo test    # console protocol line parser
cd pico2 && cargo clippy --release --bins
```

Hardware-facing code (PIO, SPI, I²C, USB, Ethernet) is verified on the bench
against [docs/BRINGUP.md](docs/BRINGUP.md), the stage-by-stage bring-up checklist.
