# rp2040_dmx — DMX-512 to I2C bridge

Rust/Embassy reimplementation of the `DMX_on_pico` Arduino sketch, extended with DMX
output. Receives (or transmits) a full DMX-512 universe on the RS-485 port and talks
to the STM32 (`nucleo` firmware) as an I2C slave at address `0x33`.

## How it works

- **Core 0 (`dmx_task`)** — runs the DMX port in the direction selected over I2C:
  - **Input** (default): a PIO state machine (port of the
    [Pico-DMX](https://github.com/jostlowe/Pico-DMX) `DmxInput` program) waits for a
    valid BREAK (≥ 88 µs low) + Mark-After-Break, then shifts in one byte per DMX
    slot at a 1 MHz PIO clock. DMA drains 513 bytes (start code + 512 channels) per
    packet. Before each packet the state machine is restarted at the BREAK detector —
    the same thing the C library does in its DMA interrupt — so frames are always
    aligned even after signal dropouts. If no packet completes within 100 ms,
    `no data!` is logged and the status pixel turns red.
  - **Output**: a second PIO state machine (port of the Pico-DMX `DmxOutput`
    program) generates a 176 µs BREAK + 16 µs MAB, then streams the 513-slot frame
    at 250 kbaud 8N2 via DMA. The latest frame written by the STM32 is retransmitted
    continuously (~43 packets/s), as DMX fixtures expect. The RS-485 driver-enable
    (GPIO3) is raised in output mode and dropped in input mode.
- **Core 1 (`i2c_task`)** — I2C slave protocol (frames are chunked because the
  STM32's transfer buffer is smaller than a frame; slot 0 is the start code):

  | Command | Direction | Bytes | Frame slots |
  |---|---|---|---|
  | `0x01` | read  | 200 | 0–199 |
  | `0x02` | read  | 200 | 200–399 |
  | `0x03` | read  | 113 | 400–512 |
  | `0x11` + data | write | ≤200 | 0–199 |
  | `0x12` + data | write | ≤200 | 200–399 |
  | `0x13` + data | write | ≤113 | 400–512 |
  | `0x20` + mode | write | 1 | port direction: `0x00` input, `0x01` output |

  Reads work as `write_read` (repeated-start) or as separate write-then-read
  transactions. Unknown read commands are answered with zeroes so the bus is never
  left clock-stretched.
- Received frames cross cores through a 1-deep channel with `try_send` (latest frame
  wins); output frames cross the other way through an `embassy_sync::Watch` (same
  latest-wins semantics). The port direction is a plain atomic polled between
  packets, so a direction change takes effect within ~100 ms.
- `log` messages go to a **USB CDC serial port** (equivalent of the sketch's
  `Serial.println`). defmt/RTT remains available when a debug probe is attached.

## Pins

| Function | Pin |
|---|---|
| DMX RX (RS-485 receiver output) | GPIO2 |
| RS-485 driver enable (low = receive, high = transmit) | GPIO3 |
| DMX TX (RS-485 driver input) | GPIO4 |
| I2C1 SDA / SCL | GPIO6 / GPIO7 |
| WS2812 status pixel (green = receiving, blue = transmitting, red = timeout) | GPIO23 |
| Onboard LED (blinks with traffic) | GPIO25 |

## Build / flash

```
cargo build --release

# with the board in BOOTSEL mode (runner uses picotool):
cargo run --release

# or create a UF2 to drag-and-drop:
picotool uf2 convert -t elf ../target/thumbv6m-none-eabi/release/rp2040_dmx rp2040_dmx.uf2
```

## Known limitations

- Input reads complete only after 513 slots, so the transmitter must send full
  512-channel universes (as virtually all controllers do). Short universes would
  require per-BREAK framing in the PIO program (same limitation as the C library
  configured for 513 slots).
- The port is half-duplex (one RS-485 transceiver): input and output are mutually
  exclusive, selected with command `0x20`. The `nucleo` firmware drives this from
  its operating mode: `ArtNet>DMX` and `USB>DMX` switch the bridge to output and
  stream frames to it (see `nucleo/src/dmx_i2c.rs`).
