# Rev 2 firmware documentation

| Document | What it covers |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Why the firmware is shaped the way it is: the core split, data path, budgets, power, the firmware↔board contract and the Ethernet history |
| [BRINGUP.md](BRINGUP.md) | Bench bring-up checklist, stages 0–10, covering both boards |
| [ft232rnl-eeprom.md](ft232rnl-eeprom.md) | FT_Prog field values for the per-unit FT232RNL EEPROM |

Board design documents — schematic, BOM and layout reviews for both PCBs — live
in the hardware repo under
[../../dmx_interface_dev_board_v2/docs/](../../dmx_interface_dev_board_v2/docs/).

## Diagrams

The Rev 2 diagrams moved to the hardware repo when the Altium design became the
source of truth — they document the as-built boards and carry Altium reference
designators:
[../../dmx_interface_dev_board_v2/docs/](../../dmx_interface_dev_board_v2/docs/).

The Python generators that produced them were retired at the same time; the SVGs
are now hand-maintained alongside the board they describe.
