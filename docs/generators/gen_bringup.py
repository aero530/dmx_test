import io

W, H = 1400, 1060
o = []
o.append('<?xml version="1.0" encoding="UTF-8"?>')
o.append('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d" '
         'font-family="ui-sans-serif, Segoe UI, system-ui, sans-serif">' % (W, H, W, H))
o.append('<defs>'
         '<marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#33383f"/></marker>'
         '<marker id="ao" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#b45309"/></marker>'
         '<marker id="ag" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#059669"/></marker>'
         '<style>.t{fill:#1f2328}.mut{fill:#5b6570}'
         '.mono{font-family:ui-monospace,Consolas,monospace}'
         '.w{stroke:#33383f;stroke-width:1.6;fill:none}'
         '.wo{stroke:#b45309;stroke-width:2.4;fill:none}'
         '.wg{stroke:#059669;stroke-width:2.2;fill:none}'
         '.wd{stroke:#7c3aed;stroke-width:1.8;fill:none;stroke-dasharray:6 4}</style></defs>')
o.append('<rect width="%d" height="%d" fill="#ffffff"/>' % (W, H))
o.append('<text x="36" y="40" font-size="23" font-weight="700" class="t">Rev 2 — Power Bring-Up and Sequencing</text>')
o.append('<text x="36" y="63" font-size="13.5" class="mut">Rise order is enforced by construction: every rail follows '
         'passively except carrier 3V3, which is gated by the module IOVDD through the LDO enable (R61).</text>')


def box(x, y, w, h, fill, stroke, title, lines, badge=None, tsize=12.5):
    o.append('<rect x="%d" y="%d" width="%d" height="%d" rx="7" fill="%s" stroke="%s" stroke-width="1.8"/>'
             % (x, y, w, h, fill, stroke))
    o.append('<text x="%d" y="%d" font-size="%s" font-weight="700" text-anchor="middle" class="t">%s</text>'
             % (x + w / 2, y + 21, tsize, title))
    for i, ln in enumerate(lines):
        o.append('<text x="%d" y="%d" font-size="10.5" text-anchor="middle" class="mut">%s</text>'
                 % (x + w / 2, y + 39 + i * 14, ln))
    if badge:
        o.append('<circle cx="%d" cy="%d" r="11" fill="#1f2328"/>' % (x + 14, y))
        o.append('<text x="%d" y="%d" font-size="12" font-weight="700" text-anchor="middle" fill="#ffffff">%s</text>'
                 % (x + 14, y + 4, badge))


# ---------------- sequencing chain ----------------
o.append('<text x="36" y="100" font-size="13" font-weight="700" class="mut">RISE ORDER (brick power)</text>')

box(36, 118, 150, 72, '#fef3c7', '#b45309', '5 V input', ['J1 → F1 → D2 TVS', 'rail: +5V_LED', 't ≈ 0 (brick slew)'], '1')
box(36, 214, 150, 72, '#fef3c7', '#b45309', '+5V_SHIFT', ['FB2, passive follow', 'U10 level shifter', 'rises with 1'], '1')
box(232, 118, 150, 72, '#fef3c7', '#b45309', '+5V_LOGIC', ['FB1 + C3', 'passive follow', 'rises with 1'], '2')
box(428, 118, 150, 72, '#fef3c7', '#b45309', 'VSYS', ['D1 PMEG2010 −0.35 V', '≈ 4.65 V (brick)', 'also fed by USB D1'], '3')
box(624, 118, 170, 72, '#d1fae5', '#059669', '3V3_OUT (module)', ['RT6150 buck-boost', 'soft-start, ms scale', 'IOVDD of the RP2350'], '4')
box(864, 118, 170, 86, '#d1fae5', '#059669', '3V3 (carrier)', ['TLV75733, start ≈ 0.2 ms', 'EN gated: waits for 4', 'OLED · TCA9555 · EEPROM', 'pull-ups · opto anodes'], '5')
box(1104, 118, 170, 72, '#ede9fe', '#7c3aed', '3V3ISO', ['PS1 GAPTEC 1S7BE', 'in-range 2.97–3.63 V', 'THVD1400 + RX opto'], '6')

o.append('<path class="wo" d="M186 154 H 232" marker-end="url(#ao)"/>')
o.append('<path class="wo" d="M382 154 H 428" marker-end="url(#ao)"/>')
o.append('<path class="wo" d="M111 190 V 214"/>')
o.append('<path class="wo" d="M578 154 H 624" marker-end="url(#ao)"/>')
o.append('<path class="wg" d="M794 154 H 864" marker-end="url(#ag)"/>')
o.append('<text x="828" y="146" font-size="10.5" text-anchor="middle" font-weight="700" fill="#047857">EN ≥ 1.0 V</text>')
o.append('<text x="828" y="160" font-size="9.5" text-anchor="middle" class="mut">via R61</text>')
o.append('<path class="wg" d="M1034 161 H 1104" marker-end="url(#ag)"/>')

o.append('<text x="500" y="238" font-size="11.5" class="t">USB-only: VBUS → module D1 → VSYS. Chain 4→5→6 unchanged;'
         ' rails 1/2 stay dark — strips off, logic and DMX fully alive.</text>')
o.append('<text x="500" y="256" font-size="11.5" class="t">Power-down: RT6150 is a buck-boost and holds IOVDD to VSYS ≈ 1.8 V,'
         ' so the module outlives the carrier rail — injection-free in both directions.</text>')

# ---------------- the hazard the gate removes ----------------
o.append('<rect x="36" y="292" width="640" height="140" rx="7" fill="#fef2f2" stroke="#dc2626" stroke-width="1.5"/>')
o.append('<text x="52" y="316" font-size="13" font-weight="700" fill="#dc2626">Why the EN gate exists (fixed this review)</text>')
for i, ln in enumerate([
        'Ungated, the TLV75733 (0.2 ms start) beats the RT6150 (ms soft-start) at every power-up.',
        'During the gap the carrier rail back-injects into unpowered, non-failsafe RP2350 pads:',
        '   I2C pull-ups 2 x 2k2 ≈ 1.5 mA each · INT 10k ≈ 0.3 mA · opto anodes via LED + 220R ≈ 6 mA each',
        '≈ 15 mA total through pad ESD structures, every boot. The gate makes the carrier rail rise',
        'strictly after IOVDD. R62 (DNP) restores an ungated LDO for module-less power-tree bench work.']):
    o.append('<text x="52" y="%d" font-size="11.5" class="t">%s</text>' % (340 + i * 17, ln))

o.append('<rect x="712" y="292" width="562" height="140" rx="7" fill="#fbfbfa" stroke="#d4d8dd"/>')
o.append('<text x="728" y="316" font-size="13" font-weight="700" class="t">Datasheet numbers this diagram relies on</text>')
for i, ln in enumerate([
        'TLV75733 (P): EN guaranteed on ≥ 1.0 V / off ≤ 0.3 V, INTERNAL EN pull-down — module out = LDO off',
        'GAPTEC 1S7BE-0303S3U: input window 2.97–3.63 V, 303 mA out, 76% efficient (1 W)',
        'TLP2368: open-collector, threshold IF 5 mA → 220R/270R give 8.0/6.5 mA',
        'TCA9555: internal 100k pull-ups confirmed, IOL 25 mA, power-on = all inputs',
        'SN74AHCT245: inputs rated to 7 V independent of VCC — USB-only drive into dead U10 is legal']):
    o.append('<text x="728" y="%d" font-size="11.5" class="mut">%s</text>' % (340 + i * 17, ln))

# ---------------- bring-up stage table ----------------
TY = 470
o.append('<text x="36" y="%d" font-size="13" font-weight="700" class="mut">BRING-UP PROCEDURE — measure at the test points, in this order</text>' % TY)
cols = [(36, 'Stage'), (96, 'Fit / apply'), (390, 'Expect'), (900, 'Pass criteria / notes')]
o.append('<rect x="36" y="%d" width="1338" height="24" rx="4" fill="#f3f4f6"/>' % (TY + 12))
for x, name in cols:
    o.append('<text x="%d" y="%d" font-size="11.5" font-weight="700" class="t">%s</text>' % (x + 6, TY + 29, name))
rows = [
    ('0', 'Bare board, NO module, brick on. (Fit R62 only if the LDO itself must be tested now.)',
     '+5V_LED 5.0 V · VSYS ≈ 4.65 V · 3V3 = 0 V by design (EN gated) · idle under 5 mA',
     'No smoke test for the input chain. 3V3 present here = R61/R62 error. U10 inputs are held'
     ' low by R63-R70, so the string outputs idle safely even with no module.'),
    ('1', 'Module fitted, brick on. Nothing else connected.',
     'VSYS 4.65 V · 3V3_OUT 3.30 V · 3V3 3.30 V · 3V3ISO ≈ 3.3 V · ~250–350 mA',
     'Heartbeat LED blinks. Scope 3V3 vs 3V3_OUT rise: carrier must lag module.'),
    ('2', 'USB only — brick disconnected.',
     'VSYS ≈ 4.4–4.7 V · +5V_LED = 0 · 3V3 up · ≈ 280 mA from VBUS',
     'UI, EEPROM, DMX all alive; strips dark. This is the bench-debug mode.'),
    ('3', 'Brick + one short strip on J10, then all eight.',
     'Per-string current per configuration; PF cold-resistance drop under 50 mV',
     'Scope +5V_LED during full-white step and power-on: transients are the old'
     ' Nucleo-killer suspect — measure, do not assume.'),
    ('4', 'DMX loop: console into J23, fixture on J24.',
     '3V3ISO under TX load ≥ 3.15 V · U14 LED ≈ 8 mA idle, 0 with GP10 high · EN_LED node 1.55 V idle, under 0.2 V in TX',
     'Direction is fail-safe: R46 biases U14 on (RECEIVE) with GP10 undriven — verify DE'
     ' stays low through reset and reflash; GP10 high (Q1 on) must flip DE high.'),
]
y = TY + 48
for stage, fit, expect, notes in rows:
    o.append('<circle cx="%d" cy="%d" r="11" fill="#1f2328"/>' % (52, y + 14))
    o.append('<text x="%d" y="%d" font-size="12" font-weight="700" text-anchor="middle" fill="#ffffff">%s</text>' % (52, y + 18, stage))
    import textwrap
    for col_x, wchars, text, cls in ((96, 46, fit, 't'), (390, 78, expect, 'mono t'), (900, 74, notes, 'mut')):
        for i, ln in enumerate(textwrap.wrap(text, wchars)):
            fs = '10.5' if 'mono' in cls else '11'
            o.append('<text x="%d" y="%d" font-size="%s" class="%s">%s</text>' % (col_x + 6, y + 10 + i * 14, fs, cls, ln))
    y += 78
    o.append('<line x1="36" y1="%d" x2="1374" y2="%d" stroke="#e5e7eb" stroke-width="1"/>' % (y - 10, y - 10))

o.append('<text x="36" y="%d" font-size="11.5" class="mut">Rails and their owners:  +5V_LED / +5V_SHIFT — fuse+TVS crowbar, strips and U10 ·'
         ' +5V_LOGIC — FB1+C3, exists only to feed D1 · VSYS — diode-OR of brick and USB ·'
         ' 3V3_OUT — module RT6150, feeds R61 EN gate + TP7 only</text>' % (y + 18))
o.append('<text x="36" y="%d" font-size="11.5" class="mut">3V3 — TLV75733, all carrier logic incl. PS1 ·'
         ' 3V3ISO / GNDISO — PS1 isolated island, THVD1400 side · CHGND — XLR shells, bonded via R60 (0R) / C16 (DNP).</text>' % (y + 36))
o.append('<text x="%d" y="%d" font-size="10.5" text-anchor="end" class="mut">dmx_interface_firmware/docs/rev2-power-bringup.svg</text>' % (W - 20, H - 16))
o.append('</svg>')

io.open('rev2-power-bringup.svg', 'w', encoding='utf-8', newline='\n').write('\n'.join(o))
print('wrote rev2-power-bringup.svg')
