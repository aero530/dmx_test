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

## DMX transmit bench test (`src/bin/dmx_tx_test.rs`)

Stand-alone transmit test for the Rev 1 carrier with **only the Pico fitted** — no
Nucleo, no I2C master. It streams a full universe at ~43 packets/s, treating the
channels as consecutive 3-channel RGB fixtures (group 1 = ch 1–3, group 2 = ch 4–6,
… group 170 = ch 508–510). Every group cycles through the full hue wheel once every
6 s at a 50 % peak (128), and the groups are hue-offset from one another so the
universe as a whole is one rainbow scrolling along the addresses. The Pico's LED
(GP25) toggles once per cycle and a status line goes out over USB serial each second.
`GROUP_COUNT`, `PEAK_LEVEL` and `HUE_PERIOD_MS` at the top of the file are the knobs.

Why it works without the Nucleo: on the Rev 1 schematic the DMX front end
(`DMX.SchDoc`) hangs directly off the Pico — GP4 → R9 → U3 (TLP2368) → THVD1400 `DI`,
GP3 → R12 → U7 (TLP2368) → THVD1400 `DE`/`RE` (high = transmit), GP2 ← U8 ← `RO`. The
STM32 only ever talked to the Pico over I2C, which this binary does not use. The
carrier's 3V3 (U11) and the isolated 3V3ISO (PS1) can be sourced from the Pico's own
USB via `5V_RP2040 → F2 → JP22 → V_LED → L2 → U11`.

### Jumper / hardware checklist

| Item | Setting | Why |
|---|---|---|
| **JP23** (3V3 → RP_VSYS bypass) | **REMOVE** | With USB on the Pico, RP_VSYS sits at ~4.7 V. A shunt here back-drives the carrier 3V3 rail (U11, PS1, optos) to 4.7 V. |
| **JP22** (5V_RP2040_FUSED → V_LED bypass) | **FIT** | Lets the Pico's USB feed V_LED without depending on the U12/Q2 ideal diode (the schematic's own note for USB-only powering). Remove again before feeding V_LED externally, or it back-feeds the PC's USB port. |
| **JP1** (5V_STM32 → V_LED) | remove | Nucleo absent; nothing to bypass. |
| **JP28** (120 Ω termination) | open | Transmitter end — the fixture terminates. |
| **J14** (RUN header) | no shunt | A shunt holds the Pico in reset. |
| External V_LED supply | none | USB is the only source in this configuration. |
| LED strips / LP / HP modules | unplug | Anything on V_LED now draws from the PC's USB port; the SmartLED module itself may stay if its strips are disconnected. |
| **J15 output XLR** (female) | check it is fitted | The schematic PDF shows J5/J15 with Altium's red *not-fitted* cross. If they are absent, pick up **pin 3 = DMX_p (A)**, **pin 2 = DMX_n (B)**, **pin 1 = GNDISO** from the J15 pads. |

Without the Nucleo, RP2040 `RUN` hangs on its ~50 kΩ internal pull-up plus the
floating trace to CN9-30; if the Pico resets spuriously, add a 10 k pull-up at J14.

### Flash and run

```
cd rp2040_dmx
cargo build --release --bin dmx_tx_test

# Pico in BOOTSEL mode (hold BOOTSEL while plugging in USB):
cargo run --release --bin dmx_tx_test        # picotool runner from .cargo/config.toml
# or drag-and-drop:
picotool uf2 convert -t elf ../target/thumbv6m-none-eabi/release/dmx_tx_test dmx_tx_test.uf2
```

Open the USB serial port (any baud). Expected:

```
DMX TX test: 170 RGB groups (ch1-510) cycling the hue wheel every 6000 ms, peak 128 (50%)
GP4 = DMX.TX, GP3 = DMX.EN (held high = transmit), GP25 LED toggles once per cycle
43 pkt/s | group 1 (ch1-3) = R128 G21 B0
43 pkt/s | group 1 (ch1-3) = R128 G64 B0
```

### What to check

1. **Rails**: 3V3 ≈ 3.3 V at U11 pin 3 (USB → U11 has little headroom; a long cable
   can pull it under regulation), 3V3ISO ≈ 3.3 V at PS1 pin 7.
2. **Direction**: THVD1400 pin 3 (`DE`) reads high against **GNDISO** — confirms the
   EN opto polarity (GP3 high → U7 LED off → `DE` high).
3. **Data**: scope across XLR pins 3 and 2 (differential, or single-ended with the
   probe ground on pin 1 / GNDISO — *never* on carrier GND): 250 kbaud, 176 µs BREAK,
   ~44 frames/s, ≥ ±1.5 V differential idle.
4. **Fixture**: an RGB fixture at any address 1–508 fades red → yellow → green → cyan
   → blue → magenta → red over 6 s at half brightness; fixtures at adjacent addresses
   sit at different points on the wheel. A USB DMX receiver (Enttec or similar) should
   show slots 1–510 moving and 511–512 at 0.

Only one XLR is needed; J5 and J15 are wired in parallel on the same transceiver.

The bridge firmware's WS2812 status pixel on GP23 is deliberately not used here:
on the SC0915 Pico, GP23 is the internal `SMPS_PS` pin and is not on the header.

## Enttec USB Pro emulation test (`src/bin/enttec_test.rs`)

Same hardware setup and jumper checklist as the transmit test, but the DMX content
comes from the PC: the Pico enumerates as an Enttec DMX USB Pro-compatible widget on
a USB serial port and retransmits every frame the host sends on the wired DMX port.
It uses the message parser that ships in the real firmware
(`common/src/enttec_protocol.rs`) and the same composite-CDC USB layout as `pico2`.

The device shows up as **two** serial ports:

| Port | Interface | Purpose |
|---|---|---|
| 1st | CDC-ACM | Enttec widget — labels 6 (send DMX), 3 (get parameters), 10 (get serial); 4 and 8 accepted |
| 2nd | CDC-ACM | Log — one status line per second (host frames/s, DMX pkt/s, start code + ch 1–3) |

The Pico LED toggles on every frame received from the host. DMX output runs from
boot (all channels 0) and follows the latest host frame at ~43 pkt/s regardless of
the host's rate; on host disconnect the last frame keeps repeating.

### Flash and run

```
cd rp2040_dmx
cargo run --release --bin enttec_test          # Pico in BOOTSEL mode
# or: picotool uf2 convert -t elf ../target/thumbv6m-none-eabi/release/enttec_test enttec_test.uf2

pip install pyserial                            # once
python tools/enttec_host.py --query             # identify the widget
python tools/enttec_host.py                     # rainbow at 40 fps until Ctrl-C
python tools/enttec_host.py --pattern chase
python tools/enttec_host.py --pattern static --set 1=255,2=128,3=0
```

The script finds the widget by VID/PID (`c0de:dcaf`, or FTDI's `0403:6001` for the
emulator below), then confirms it by sending Get Widget Parameters and taking the port
that answers (the log port doesn't). Expected:

```
widget on COM7: firmware v1.44, break 171 us, MAB 21 us, refresh 40 Hz, 0 user bytes
serial number: 00000001
sending rainbow at 40 fps, 512 channels — Ctrl-C to stop
 40.0 frames/s   ch1-3 = 128  21   0
```

and on the log port:

```
Enttec: host configured the device
Enttec: get-params (0 user bytes)
Enttec: get-serial
host 40 frames/s | DMX out 43 pkt/s | SC 0x00 ch1-3 = 128 21 0
```

Windows sleep granularity means the script may show ~35–40 fps; the widget re-times
the wire at ~43 pkt/s regardless, so the fixture doesn't care.

### Using real lighting software

Most packages recognise the *original* Enttec Pro by its FTDI chip (VID `0403`), so
they will not auto-detect a CDC serial widget — QLC+'s DMX USB plugin is in this camp.
Packages that let you pick any COM port and speak "Enttec Pro" / "DMX Pro" framing
over it do work: xLights (serial controller, protocol *DMX*), Vixen (Enttec Pro on a
chosen COM port), OLA (`usbpro` plugin with `device_prefix = ttyACM`). Pick the
*first* of the two ports; if in doubt, run `enttec_host.py --query` to see which it is.
For software that insists on the real thing, see the FT232R emulator below.

## FT232R emulation test (`src/bin/ftdi_test.rs`)

Same widget core as `enttec_test`, but the USB side impersonates the chip inside a
genuine Enttec DMX USB Pro: an **FT232R** (`0403:6001`, bcdDevice `0x0600`, one
vendor-class interface, bulk EP1 IN / EP2 OUT, manufacturer `ENTTEC`, product
`DMX USB PRO`, serial `EN0000001`). The host's FTDI driver binds to it, so software
that enumerates FTDI devices (QLC+, OLA `ftdidmx`, D2XX/libftdi apps) finds a Pro.

What is emulated:

| FTDI mechanism | Emulation |
|---|---|
| Vendor control requests 0x00–0x0C (reset, modem ctrl, flow ctrl, baud, line coding, event/error char, latency timer, bitmode, read pins) | Accepted; baud/flow/bitmode ignored (the "UART" is virtual), DTR/RTS/break and latency tracked and logged |
| Poll modem status (0x05) and the 2-byte status header on **every** bulk IN packet | `0x01 0x60` — no modem lines, transmitter idle |
| Latency timer | Status-only packet every *N* ms (default 16) when there is nothing to send, like the real chip |
| EEPROM read / write / erase (0x90–0x92) | 128-byte FT232R image with valid checksum, VID/PID/strings/CBUS defaults; writes land in RAM only and are logged |

**Logging** cannot ride on a second USB interface here — a composite device would no
longer match the FTDI driver's `VID_0403&PID_6001` binding — so the log goes out
**UART0 TX on GP28 = J19 pin 4** (GND on J19 pins 5/6), 115200 8N1, to any 3.3 V
USB-UART dongle. defmt/RTT works too with a probe.

### Flash and run

```
cd rp2040_dmx
cargo run --release --bin ftdi_test            # Pico in BOOTSEL mode
python tools/enttec_host.py --query            # via the FTDI VCP COM port
python tools/enttec_host.py                    # rainbow, as before
```

On the UART you should see the driver bind (`FTDI: reset all`, `FTDI: baud divisor …`,
`FTDI: latency timer 16 ms`, `FTDI: DTR=1 RTS=1`) followed by the usual widget lines.

**Result (2026-08-26):** enumerates on Windows through FTDI's own driver and
**QLC+ detects it as an Enttec DMX USB Pro** — the identity (PID `6001`,
bcdDevice `0600`, product string `DMX USB PRO`) is exactly what the Rev 2
carrier's real FT232RL will present once its EEPROM is programmed.

**Windows caveat (repeat of the earlier warning):** FTDI's CDM driver polices
non-genuine silicon and its EEPROM checks are undocumented and change between
releases. Expected outcomes, in order of likelihood: it just works; it works but the
driver injects `NON GENUINE DEVICE FOUND!` into the data stream (you will see garbage
in `enttec_host.py` and unsupported-label warnings on the UART); or the device shows
a yellow bang. Linux (`ftdi_sio`, libftdi) has no such check. Nothing here is
persistent — EEPROM "writes" from the driver vanish at the next reset. This is a bench
experiment only: the FTDI VID may not ship on non-FTDI silicon.

## Known limitations

- Input reads complete only after 513 slots, so the transmitter must send full
  512-channel universes (as virtually all controllers do). Short universes would
  require per-BREAK framing in the PIO program (same limitation as the C library
  configured for 513 slots).
- The port is half-duplex (one RS-485 transceiver): input and output are mutually
  exclusive, selected with command `0x20`. The `nucleo` firmware drives this from
  its operating mode: `ArtNet>DMX` and `USB>DMX` switch the bridge to output and
  stream frames to it (see `nucleo/src/dmx_i2c.rs`).
