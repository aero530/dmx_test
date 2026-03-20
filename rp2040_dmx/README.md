# This codes does not work yet. Use DMX_on_pico

```
> cargo build --release
> picotool uf2 convert -t elf target/thumbv6m-none-eabi/release/rp2040_dmx rp2040_dmx.uf2
```