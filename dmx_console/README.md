# dmx_console — desktop console for the DMX interface

A Ratatui terminal app that connects to the device's USB **console** serial
port (the second of the serial ports exposed by the composite USB device)
for commissioning and debugging: edit every firmware setting and watch live
DMX channel data.

```
cargo run --release              # list serial ports (device ports are marked)
cargo run --release -- COM5      # connect (use the 2nd of the device's ports)
```

## Views

- **Settings** — every firmware setting as a key/value table. `Enter` starts
  an edit, type the new value, `Enter` applies it (the firmware validates and
  persists to EEPROM; errors are shown in the status line). Auto-refreshes
  every 2 s.
- **DMX Monitor** — all 512 channels of the active universe, 16 per row,
  color-shaded by level, refreshed 4×/s.

Keys: `Tab` switch view, `↑`/`↓` select or scroll, `Enter` edit/apply,
`Esc` cancel edit, `r` refresh, `q` quit.

## Wire protocol

Line-oriented; also usable from any serial terminal. Every reply ends with
`ok` or `err <reason>`.

| Command | Reply |
|---|---|
| `get` | one `key=value` line per setting |
| `set <key> <value>` | applies + persists one setting |
| `dmx <start> <count>` | `dmx <ch> <v> <v> ...` lines, 16 values per line |
| `info` | `mode=`, `module=`, `ip=`, `boot=` summary |
| `help` | command list |

Keys and accepted values come from the firmware's field metadata table
(`common/src/ui/fields.rs`), which also drives the on-device TFT menu — the
three interfaces cannot drift apart.

## Tests

`cargo test` — covers the protocol line parser.
