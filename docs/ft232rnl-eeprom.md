# FT232RNL EEPROM — production programming

The carrier's FT232RNL (U2) is what makes FTDI-stack lighting software (QLC+,
D2XX applications) recognise the box as an Enttec DMX USB Pro. The chip ships
with a generic FTDI identity; **every unit must have its EEPROM programmed on the
bench** with FT_Prog (Windows, from ftdichip.com) or `ftdi_eeprom`/`ft232r_prog`
(Linux) before it leaves.

The firmware cannot do this — the FT232RNL's EEPROM is on the USB side of the
chip, reachable only from a host.

## Why these values

QLC+ keys widget type off the **product string** containing `DMX USB PRO`
(verified on the bench with the `ftdi_test` emulator, 2026-08). The chip is
**self-powered** from the carrier's 3V3 (its `RESET#` follows VBUS through
R24/R25, per FTDI's self-powered configuration), so the descriptor must say so
and must not claim more than the 100 mA a self-powered device may draw before
configuration.

## FT_Prog template

| Field | Value | Notes |
|---|---|---|
| USB Device Descriptor → VID / PID | `0x0403` / `0x6001` | FTDI defaults; **do not change** — the driver match depends on them |
| USB Config Descriptor → Bus Powered / Self Powered | **Self Powered** | VCC is the board's 3V3, not VBUS |
| USB Config Descriptor → Max Bus Power | **90 mA** | FTDI's default for self-powered; the chip itself draws ~15 mA |
| USB Config Descriptor → USB Remote Wakeup | off | |
| USB String Descriptors → Manufacturer | `ENTTEC` | matches what host software expects to see |
| USB String Descriptors → Product Description | `DMX USB PRO` | **the string QLC+ matches on** |
| USB String Descriptors → Serial Number | per unit, `EN` + 7 digits | e.g. `EN0000001`; "Auto Generate Serial No" **off** so units are traceable |
| Hardware Specific → Port A → Driver | **D2XX Direct + VCP** (both) | QLC+ on Windows uses D2XX; other tools use the COM port |
| Hardware Specific → Invert RS232 signals | all **off** | UART0 on the RP2350 is true-polarity |
| Hardware Specific → IO Controls (CBUS0–4) | leave defaults (TXLED/RXLED/TXDEN/PWREN/SLEEP) | the CBUS pins are unconnected on the carrier |
| Hardware Specific → High Current I/O | off | |
| Hardware Specific → Load D2XX driver | on | |

Program, then **cycle the USB cable** so the host re-reads the descriptors.

## Verify

1. Windows Device Manager → the port should enumerate as `USB Serial Port (COMx)`
   with **Manufacturer ENTTEC**, product `DMX USB PRO`.
2. QLC+ → Inputs/Outputs → the device appears as a **DMX USB Pro** output. Patch a
   universe, run a fixture through the console's `dmx 1 16` command: the values
   should follow the QLC+ fader.
3. The firmware log shows `FTDI widget: locked at 250000 baud` (QLC+) — a different
   host will lock at its own rate; the firmware hunts.

## Notes

- The **RN** die reports a different `bcdDevice` from the old FT232RL. QLC+ does
  not check it; the FTDI driver knows the RN. Verified on the first article.
- The board leaves `CTS#`/`RTS#` open — configure host software for **no** hardware
  flow control (every DMX application defaults to that).
- Keep the FT_Prog `.xml` template you save from a programmed unit next to this file
  (`ft232rnl-template.xml`) so production re-uses exactly one configuration.
