DMX Test

```
cargo build --bin nucleo --target thumbv8m.main-none-eabi
```

== UI

* Basic Config
    * DMX starting address
    * Addresses used
    * Lighting mode
* ArtNet Config
    * IP address
    * Misc settings / info
* SmartLED
    * LED Type
    * LED Count
    * Group size (between 1 and LED Count)
    * Interleaving (on / off)
    * Addresses used (output)
* PWM
    * Mode 3 channel / 4 channel


https://github.com/embedded-graphics/embedded-graphics/
https://github.com/bugadani/embedded-layout
https://github.com/bugadani/embedded-menu
https://github.com/embedded-graphics/embedded-text


== Menu

* Overview
    - DMX address range
    - IP address
    - LED mode
* DMX Settings
    - Set DMX start address
* Artnet Settings
    - Set IP / netmask / etc
* LED Settings (module specific)

* Smart LED module settings
    - Set number of LEDs per port (4 ports)
    - Set LED grouping scheme
        - Individual
        - Combined by port
        - Combined by module
    - Set address mode
        - RGB
        - RGBW



Item:
    - Selectable
    - Editable
    - Action