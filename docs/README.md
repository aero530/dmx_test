# Rev 2 hardware diagrams

Diagrams for the Rev 2 redesign described in [../REV2_PLAN.md](../REV2_PLAN.md).

| Diagram | What it covers |
|---|---|
| [rev2-block-diagram.svg](rev2-block-diagram.svg) | System overview — how the module, DMX front end, output stage, UI and power fit together |
| [rev2-pinmap.svg](rev2-pinmap.svg) | All 40 pins of the W6300-EVB-Pico2, with the mux checks that make the allocation legal |
| [rev2-power-tree.svg](rev2-power-tree.svg) | DC input to VSYS to 3V3(OUT), the diode-OR, and what gets deleted from Rev 1 |
| [rev2-ws2812-output.svg](rev2-ws2812-output.svg) | 8-channel level shifter, series resistors, per-output fusing |
| [rev2-dmx-isolated.svg](rev2-dmx-isolated.svg) | Opto-isolated RS-485 front end, bias, termination, grounding |
| [rev2-ui-i2c.svg](rev2-ui-i2c.svg) | OLED on SPI1, TCA9555 expander, EEPROM, I²C address map |

## Status

These are **design intent**, not a captured schematic. They record the decisions
in `REV2_PLAN.md` at a level you can check against before drawing the real
schematic in Altium. Anything marked **VERIFY** in a diagram is an open question,
not a settled value.

Blocks are colour-coded as new/changed in Rev 2 vs. carried over from Rev 1, so
the actual scope of the redesign is visible at a glance.

## Regenerating

Four of the six are generated, because they contain repeated rows that are
tedious and error-prone to hand-edit — changing a pin allocation in a Python
table beats editing a 27 KB SVG by hand.

```
cd docs
python generators/gen_pinmap.py     # rev2-pinmap.svg
python generators/gen_ws2812.py     # rev2-ws2812-output.svg
python generators/gen_dmx.py        # rev2-dmx-isolated.svg
python generators/gen_ui.py         # rev2-ui-i2c.svg
```

Scripts write into the current directory, so run them from `docs/`. No
dependencies beyond the standard library.

`rev2-block-diagram.svg` and `rev2-power-tree.svg` are hand-authored — their
layouts are bespoke enough that a generator would not help.

## Conventions

- Plain SVG, no external fonts or scripts, so they render in the IDE, on GitHub
  and in a browser.
- Explicit white background and dark text rather than theme-aware colours —
  a schematic that inverts in dark mode is harder to read, not easier.
- Monospace for signal and part names, proportional for prose.
