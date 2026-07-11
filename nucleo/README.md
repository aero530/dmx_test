# nucleo — main firmware (STM32H563ZI)

Embassy-based firmware for the DMX/Art-Net lighting interface. Receives DMX
(via the RP2040 bridge), Art-Net (Ethernet) or Enttec-protocol DMX (USB),
drives WS2812 output modules, can retransmit on the wired DMX port, and is
configured through a Ratatui TFT menu, a USB console, or the `dmx_console`
desktop app.

See the [repository readme](../readme.md) for the overall architecture and
task/data-flow description, and [BUGS.md](../BUGS.md) for review history.

## Source map

| Path | Purpose |
|---|---|
| `src/main.rs` | Clock/peripheral setup, heap init, task spawning, boot sequence |
| `src/event_router.rs` | Central event hub; maps DMX data to LED colors per settings |
| `src/channels.rs` | All inter-task channels (documented per channel) |
| `src/statics.rs` | Global buffers: `DMX_BUFFER`, `LED_COLORS`, shared I2C buses |
| `src/constants.rs` | Addresses, buffer sizes, display geometry |
| `src/dmx_i2c.rs` | RP2040 bridge master: poll received DMX / push outgoing DMX |
| `src/artnet/` | Art-Net receiver + vendored `tiny_artnet` parser |
| `src/usb_device.rs` | Composite USB device (Enttec widget + console + logger) |
| `src/enttec_protocol.rs` | Enttec message framing (pure logic, host-tested) |
| `src/console_usb.rs` | USB console line protocol (`get`/`set`/`dmx`/`info`) |
| `src/ui/mod.rs` | Ratatui menu app + mousefood/ST7789 rendering |
| `src/ui/fields.rs` | Settings-field metadata table (drives UI *and* console) |
| `src/ui/types.rs` | Settings model (`MenuData`), enums, EEPROM encoding |
| `src/eeprom/` | Settings/MAC/boot-flag storage on the module M24C02 |
| `src/smart_led/` | WS2812-over-SPI output driver (4 ports, DMA) |
| `src/button.rs`, `src/button_array.rs` | User button + 4-button menu input |
| `src/pwm_i2c.rs` | PWM output module (unfinished, task not spawned) |

## Build / flash

Requires the `thumbv8m.main-none-eabihf` target and
[probe-rs](https://probe.rs) with the Nucleo's ST-Link attached:

```
cargo build --release
cargo run --release      # flash + stream defmt logs
```

## Features

Defaults: `clock_stlink`, `ethernet` (see `Cargo.toml`).

| Feature | Effect |
|---|---|
| `clock_stlink` | Clock tree fed by the ST-Link 8 MHz output (default) |
| `clock_25MHz_osc` | Clock tree fed by the on-board 25 MHz oscillator |
| `ethernet` | Embassy-net stack + Art-Net task |
| `usb` | Route logs to a third USB CDC interface instead of defmt/RTT |
| `pwm`, `pwm_native`, `display` | Reserved / unused |

## USB ports

The device enumerates as one composite USB device (VID:PID `0xc0de:0xdcaf`)
with, in order:

1. **Enttec DMX USB Pro** compatible serial port (lighting software)
2. **Console** serial port (line protocol; used by `../dmx_console`)
3. **Logger** serial port (only with the `usb` feature)

## Memory notes

- A 160 KB `embedded-alloc` heap backs the Ratatui UI (the mousefood
  framebuffer alone is ~108 KB). Sized in `main.rs`.
- `DMX_BUFFER` holds 256 flat universes (128 KB); static RAM totals ~473 KB
  of the part's 640 KB.

## Testing

The hardware-independent logic (settings model, field metadata, Enttec
framing) is unit tested on the host — the test crate includes these source
files directly:

```
cd ../host_tests
cargo test
```
