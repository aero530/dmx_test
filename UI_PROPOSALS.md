# UI Redesign Proposals

The menu UI works, but the 2026-07 review showed the current design is where most of
the firmware's bugs live (BUGS.md N13–N16, N21). This document describes why, then
lays out three redesign options plus one companion idea, with a recommendation.

## Current architecture (and why it breeds bugs)

- **Stack**: ST7789 172×320 TFT over SPI (mipidsi), embedded-graphics + embedded-text
  + u8g2-fonts, a hand-rolled `View` trait, 4 fixed tabs × 5 fixed items.
- **Triple data representation**: every setting exists as (1) `MenuData` (bincode/
  EEPROM), (2) `MenuTabData` (a 4×5 grid addressed by hardcoded coordinates like
  `self.0[1][1]` in `to_menu_data`), and (3) live widget state inside `MenuItem`s.
  All three are synced by hand on every keypress. Adding one setting touches ~6
  places; getting a coordinate wrong silently swaps two settings.
- **No per-value metadata**: min/max/step live as ad-hoc `if` clauses inside each
  `ValueType` inc/dec arm. That's exactly where the group-size-0, DMX-address-0/999
  and Art-Net-field-255 bugs came from — each new value type re-implements clamping,
  and some forgot.
- **Navigation state is distributed**: each item remembers its own sub-cursor
  (`value_index`), which caused the skipped-digit and dead-end-tab bugs.
- **Rendering is clear-and-redraw**: every event does `display.clear(BLACK)` + full
  tab redraw — visible flicker and ~110 KB of SPI traffic per keypress.
- **Design holes**: `module_type` is hardcoded to SmartLed (the EEPROM-detected type
  never reaches the UI); `UiEvent::Load` stomps in-progress edits; there is no
  edit-cancel — Esc is ignored while editing, so every edit session ends in an
  EEPROM write.

Any redesign should fix the *architecture* problems regardless of widget library:

1. **Single source of truth** — one `Settings` struct; widgets are views into it.
2. **Declarative field metadata** — a table of `Field { label, min, max, step,
   get/set accessors, read_only }`; navigation and clamping become generic code that
   cannot be forgotten per-field.
3. **Explicit edit transaction** — copy-on-edit, Select = commit (EEPROM write),
   Esc = cancel; external `Load` events defer until the transaction ends.
4. **Module-type awareness** — menu pages built from the module type reported by the
   EEPROM at boot, not hardcoded.

---

## Proposal A — Ratatui on the TFT via `mousefood` (recommended)

[Ratatui](https://ratatui.rs) is a mature immediate-mode TUI framework;
[`mousefood`](https://crates.io/crates/mousefood) is an embedded-graphics backend for
`ratatui-core` that renders the character-cell buffer onto any `DrawTarget` — i.e.
directly onto the existing mipidsi/ST7789 display. `no_std` + alloc (the project
already ships `embedded-alloc`).

**What the screen becomes**: a character grid. With an 8×12 font on the 320×172
logical display you get roughly 40 cols × 14 rows — comfortably enough for a tabbed
settings menu, and it instantly buys:

- `Tabs`, `List`/`Table`, `Paragraph`, `Gauge`, `Block` borders, styling — no more
  hand-computed pixel layout or per-digit rectangle math.
- **Diffed rendering for free**: Ratatui double-buffers cells and the backend flushes
  only changed cells → the flicker and the 110 KB-per-keypress SPI storm disappear
  without any dirty-flag bookkeeping.
- A huge widget/pattern ecosystem and testability: the UI state machine can be unit
  tested on the host against a `TestBackend`, no hardware needed.

**Sketch**:

```rust
// ui state = Settings + Cursor + EditTransaction
let backend = mousefood::EmbeddedBackend::new(&mut display, config);
let mut term = ratatui_core::Terminal::new(backend)?;
loop {
    let ev = select(buttons.next(), router_events.next()).await;
    app.handle(ev);                      // pure, host-testable
    term.draw(|f| app.render(f))?;       // Tabs + Table of Fields
}
```

**Costs / risks**: adds `ratatui-core` + `mousefood` + alloc (~tens of KB flash on a
2 MB part — fine); the look changes from pixel-styled to character-cell (a font choice
softens this); mousefood is a young crate — pin the version and smoke-test SPI flush
performance early (a full-screen worst-case flush is the same order as today's full
redraw, and typical keypresses touch a handful of cells).

**Effort**: ~1–2 weeks including the field-metadata refactor, which is 80 % of the
value anyway.

## Proposal B — `embedded-menu` (purpose-built, smallest code)

[`embedded-menu`](https://github.com/bugadani/embedded-menu) (already pencilled into
`Cargo.toml` as a comment) is an interactive menu library for embedded-graphics with
built-in selection, value editing (`SelectValue`), scrolling and styling. The menu
definition collapses to a declarative builder over the settings struct; navigation,
highlight and redraw logic are the library's problem.

- **Pros**: smallest diff and binary; stays pixel-native (keeps the current look);
  no alloc requirement; designed for exactly this 4-button interaction model.
- **Cons**: less flexible than Ratatui for non-menu screens (status dashboard,
  live DMX channel monitor); multi-digit-in-place editing would become
  increment/decrement of whole values (arguably an improvement); still needs the
  field-metadata refactor for clamping, and per-item custom rendering is limited.

**Effort**: ~1 week.

## Proposal C — Keep the stack, refactor in place (lowest risk)

No new dependencies. Keep embedded-graphics rendering but:

1. Introduce the `Field` metadata table and a single generic cursor
   (`tab, row, digit`) — deletes `MenuTabData`, the hardcoded coordinates, and all
   per-ValueType inc/dec arms (bugs N13–N15 become structurally impossible).
2. Add the edit transaction (fixes N21, adds edit-cancel).
3. Replace clear-and-redraw with per-row dirty tracking (row rectangles are already
   known from `arrange()`).

- **Pros**: zero new-crate risk, keeps exact current look, incremental/shippable in
  pieces.
- **Cons**: you keep maintaining a bespoke UI toolkit; the ~600 lines of layout and
  draw code stay yours forever.

**Effort**: ~3–5 days.

## Companion idea D — Host-side Ratatui console over USB

Independent of A/B/C: the firmware already has a `usb` logging feature. Extending it
to a tiny line protocol (`get/set/subscribe`) enables a **desktop Ratatui app** that
mirrors the on-device menu, streams live DMX channel values, and edits settings — a
real keyboard for commissioning and debugging, with zero display constraints. The
same protocol doubles as a scripted test interface for CI. (~2–4 days, incremental.)

## Comparison

| | A: Ratatui/mousefood | B: embedded-menu | C: refactor |
|---|---|---|---|
| Fixes flicker | ✅ cell diffing | ✅ library redraw | ⚠️ manual dirty rects |
| Kills boilerplate | ✅ | ✅✅ | ✅ (metadata only) |
| Visual flexibility (dashboards, monitors) | ✅✅ | ⚠️ | ⚠️ |
| New-dependency risk | medium (young backend) | low-medium | none |
| Binary/RAM cost | +alloc, ~tens of KB | minimal | none |
| Host-side unit testing of UI | ✅ TestBackend | partial | manual |
| Effort | 1–2 wk | ~1 wk | 3–5 d |

## Recommendation

Do the **field-metadata + edit-transaction refactor first** (the core of Proposal C —
it's required by every option and removes the bug class). Then adopt **Proposal A
(Ratatui + mousefood)** for the presentation layer: this project wants more than a
settings menu — a live DMX/universe monitor and status dashboard are natural next
screens, and Ratatui is the only option that makes those cheap. If the mousefood
smoke-test disappoints on flush performance or maturity, fall back to **Proposal B**,
which slots into the same refactored core.

Suggested sequence:

1. `Field` metadata table + single cursor + edit transaction (ship on current renderer).
2. Wire EEPROM-detected module type into menu construction (closes the N16 design gap).
3. Spike: mousefood on the ST7789 — measure a full-frame and a per-keypress flush.
4. Port tabs to Ratatui widgets (`Tabs` + `Table`), delete the hand-rolled `View`
   layout code.
5. Add the fun stuff: live channel monitor page, Art-Net status page.
6. (Optional) Companion D console once the set/get protocol exists.
