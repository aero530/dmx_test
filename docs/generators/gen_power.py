import io

W, H = 1360, 1010
o = []
o.append('<?xml version="1.0" encoding="UTF-8"?>')
o.append('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d" '
         'font-family="ui-sans-serif, Segoe UI, system-ui, sans-serif">' % (W, H, W, H))
o.append('<defs>'
         '<marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#b45309"/></marker>'
         '<marker id="ag" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#059669"/></marker>'
         '<marker id="ak" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#33383f"/></marker>'
         '<style>.t{fill:#1f2328}.mut{fill:#5b6570}'
         '.mono{font-family:ui-monospace,Consolas,monospace}'
         '.w5{stroke:#b45309;stroke-width:2.6;fill:none}'
         '.w5t{stroke:#b45309;stroke-width:1.8;fill:none}'
         '.w3{stroke:#059669;stroke-width:2.2;fill:none}'
         '.wk{stroke:#33383f;stroke-width:1.5;fill:none}</style></defs>')
o.append('<rect width="%d" height="%d" fill="#ffffff"/>' % (W, H))
o.append('<text x="36" y="40" font-size="23" font-weight="700" class="t">Rev 2 — Power Tree</text>')
o.append('<text x="36" y="63" font-size="13.5" class="mut">One carrier 3V3 rail from an LDO on VSYS. '
         '3V3(OUT) feeds the module only, so the RT6150 budget stops being a design risk.</text>')


def box(x, y, w, h, cls, title, lines, tsize=12.5):
    fill, stroke = {'p': ('#fffbeb', '#b45309'), 'g': ('#d1fae5', '#059669'),
                    'm': ('#e8f1fc', '#1d6fd6'), 'n': ('#ffffff', '#33383f'),
                    'x': ('#f3f4f6', '#6b7280')}[cls]
    o.append('<rect x="%d" y="%d" width="%d" height="%d" rx="6" fill="%s" stroke="%s" stroke-width="1.7"/>'
             % (x, y, w, h, fill, stroke))
    o.append('<text x="%d" y="%d" font-size="%s" font-weight="700" text-anchor="middle" class="t">%s</text>'
             % (x + w / 2, y + 22, tsize, title))
    for i, ln in enumerate(lines):
        o.append('<text x="%d" y="%d" font-size="10.5" text-anchor="middle" class="mut">%s</text>'
                 % (x + w / 2, y + 40 + i * 15, ln))


# ---------- input chain ----------
o.append('<text x="36" y="106" font-size="12" font-weight="700" class="mut">INPUT PROTECTION</text>')
box(36, 120, 150, 70, 'p', 'J1 — DC in', ['5 V brick', 'size to LED load'])
box(216, 120, 140, 70, 'p', 'F1 fuse', ['ATO holder', 'sized per install'])
box(386, 120, 140, 70, 'p', 'TVS SMAJ5.0A', ['surge clamp; conducts', 'fwd on reverse - blows F1'])
box(556, 120, 140, 70, 'p', 'Bulk', ['low-ESR', 'at the split'])
for x in (186, 356, 526):
    o.append('<path class="w5" d="M%d 155 H %d" marker-end="url(#a)"/>' % (x, x + 30))

o.append('<path class="w5" d="M696 155 H 740 V 210"/>')
o.append('<circle cx="740" cy="210" r="5" fill="#b45309"/>')
o.append('<text x="752" y="205" font-size="11.5" font-weight="600" class="t">5 V split</text>')

# ---------- LED branch ----------
o.append('<path class="w5" d="M740 210 H 900 V 246" marker-end="url(#a)"/>')
box(800, 246, 250, 70, 'p', 'LED 5 V rail', ['high current, wide pour', '4800 LEDs x 60 mA = 288 A theoretical'])
o.append('<path class="w5" d="M1050 281 H 1090" marker-end="url(#a)"/>')
box(1090, 246, 236, 70, 'p', '8x polyfuse', ['one per output — review §3.7', 'J_LED1 … J_LED8'])

o.append('<path class="w5t" d="M860 316 V 356" marker-end="url(#a)"/>')
box(800, 356, 250, 82, 'p', '5V_LVL_SHIFT', ['from the LED rail, not VSYS —', 'so the shifter output swing tracks',
                                             'the same rail WS2812 VIH references'])
o.append('<text x="925" y="456" font-size="10.5" text-anchor="middle" class="mut">→ SN74ACT245 VCC · 100 nF per pin + local bulk</text>')

# ---------- logic branch ----------
o.append('<path class="w5" d="M740 210 H 250 V 318"/>')
o.append('<path d="M236 332 L 236 360 L 264 346 Z" fill="#fffbeb" stroke="#b45309" stroke-width="1.7"/>')
o.append('<line x1="264" y1="332" x2="264" y2="360" stroke="#b45309" stroke-width="2.6"/>')
o.append('<text x="180" y="342" font-size="11.5" font-weight="700" class="t">D_OR</text>')
o.append('<text x="180" y="356" font-size="10" class="mut">PMEG2010AEH</text>')
o.append('<path class="w5" d="M250 360 V 404 H 625 V 452" marker-end="url(#a)"/>')

# ---------- module ----------
o.append('<rect x="120" y="452" width="620" height="286" rx="9" fill="#e8f1fc" stroke="#1d6fd6" stroke-width="1.8"/>')
o.append('<text x="430" y="478" font-size="14" font-weight="700" text-anchor="middle" class="t">W6300-EVB-Pico2 — internal</text>')

box(150, 494, 130, 44, 'n', 'VBUS  (40)', ['USB 5 V'], 11.5)
o.append('<path d="M316 500 L 316 528 L 340 514 Z" fill="#ffffff" stroke="#6b7280" stroke-width="1.5"/>')
o.append('<line x1="340" y1="500" x2="340" y2="528" stroke="#6b7280" stroke-width="2.2"/>')
o.append('<text x="328" y="546" font-size="10" text-anchor="middle" class="mut">D1 internal</text>')
o.append('<path class="wk" d="M280 514 H 316"/>')
o.append('<path class="wk" d="M340 514 H 560" marker-end="url(#ak)"/>')
box(560, 494, 150, 44, 'p', 'VSYS  (39)', ['1.8–5.5 V'], 11.5)

box(330, 578, 220, 56, 'g', 'RT6150 buck-boost', ['holds 3V3 even as VSYS sags'])
o.append('<path class="w5" d="M635 538 V 558 H 440 V 578" marker-end="url(#a)"/>')
box(330, 660, 220, 40, 'g', '3V3(OUT)  (36)', [], 12)
o.append('<path class="w3" d="M440 634 V 660" marker-end="url(#ag)"/>')
o.append('<text x="440" y="720" font-size="11" text-anchor="middle" font-weight="600" class="t">module only — RP2350 + W6300 ≈ 200 mA</text>')
o.append('<text x="150" y="686" font-size="10.5" class="mut">3V3_EN (37) n/c</text>')
o.append('<text x="150" y="702" font-size="10.5" class="mut">VBUS (40) n/c on carrier</text>')

# ---------- LDO ----------
o.append('<path class="w5" d="M710 516 H 790" marker-end="url(#a)"/>')
o.append('<rect x="790" y="486" width="250" height="92" rx="6" fill="#d1fae5" stroke="#059669" stroke-width="2"/>')
o.append('<text x="915" y="510" font-size="13.5" font-weight="700" text-anchor="middle" class="t">LDO  5 V → 3V3_CARRIER</text>')
o.append('<text x="915" y="529" font-size="11.5" text-anchor="middle" font-weight="600" class="mono t">TLV75733PDYDR</text>')
o.append('<text x="915" y="546" font-size="10.5" text-anchor="middle" class="mut">SOT-23-5 + thermal pad · 60.3 °C/W · 1 A</text>')
o.append('<text x="915" y="562" font-size="10.5" text-anchor="middle" class="mut">EN ← module 3V3_OUT (R61, sequencing) · pad to GND pour</text>')
o.append('<text x="1054" y="498" font-size="10.5" class="mut">fed from VSYS, not the brick —</text>')
o.append('<text x="1054" y="513" font-size="10.5" class="mut">VSYS is already the diode-OR, so the</text>')
o.append('<text x="1054" y="528" font-size="10.5" class="mut">UI, EEPROM and DMX all still work</text>')
o.append('<text x="1054" y="543" font-size="10.5" class="mut">on USB-only bench power</text>')
o.append('<text x="1054" y="563" font-size="10.5" class="t">Headroom at worst case 4.40 V in:</text>')
o.append('<text x="1054" y="577" font-size="10.5" class="mut">4.40 − 0.45 = 3.95 V, and 0.45 V is the</text>')
o.append('<text x="1054" y="591" font-size="10.5" class="mut">1 A figure — at 117 mA it is ~50 mV</text>')

# ---------- carrier 3V3 loads ----------
o.append('<path class="w3" d="M915 578 V 612" marker-end="url(#ag)"/>')
o.append('<rect x="770" y="612" width="556" height="196" rx="7" fill="#ecfdf5" stroke="#059669" stroke-width="1.7"/>')
o.append('<text x="1048" y="636" font-size="13.5" font-weight="700" text-anchor="middle" class="t">3V3_CARRIER — 117 mA total</text>')
rows = [('PS1 1S7BE input (3V3 in → 3V3ISO)', '≈ 90 mA', 'largest load; derived, measure it'),
        ('SSD1306 OLED', '20 mA', 'SPI, ~1 ms redraw'),
        ('I²C pull-ups 2 × 2k2', '3 mA', ''),
        ('TCA9555 + INT pull-up', '2 mA', ''),
        ('M24C02 EEPROM', '2 mA', 'active')]
for i, (n, c, note) in enumerate(rows):
    y = 664 + i * 22
    o.append('<text x="792" y="%d" font-size="11.5" class="t">%s</text>' % (y, n))
    o.append('<text x="1108" y="%d" font-size="11.5" text-anchor="end" class="mono t">%s</text>' % (y, c))
    if note:
        o.append('<text x="1124" y="%d" font-size="10" class="mut">%s</text>' % (y, note))
o.append('<line x1="792" y1="782" x2="1304" y2="782" stroke="#059669" stroke-width="1"/>')
o.append('<text x="792" y="800" font-size="11.5" font-weight="700" class="t">3V3(OUT) carrier load</text>')
o.append('<text x="1108" y="800" font-size="11.5" text-anchor="end" font-weight="700" class="mono t">0 mA</text>')

# ---------- notes ----------
o.append('<rect x="36" y="766" width="700" height="212" rx="7" fill="#fbfbfa" stroke="#d4d8dd"/>')
o.append('<text x="52" y="790" font-size="13" font-weight="700" class="t">Why the LDO sits on VSYS - and where reverse protection went</text>')
for i, ln in enumerate([
        'VSYS is the diode-OR node: brick through D_OR, USB through the module\'s D1.',
        'Tapping it means one 3V3 rail that is alive on brick power, USB power or both.',
        'Feeding the LDO from the brick instead would leave the OLED, buttons, EEPROM',
        'and the whole DMX side dead whenever you are running on USB at the bench.',
        '',
        'USB-only budget: module ≈165 mA from VSYS + carrier 117 mA ≈ 282 mA from VBUS.',
        'Inside the 500 mA USB allowance. The LED strings obviously do not run on USB,',
        'but everything you need to debug does.',
        '',
        'Reverse protection: no series P-FET. A FET rated for the LED rail is expensive',
        'and thermally awkward at tens of amps; instead the TVS conducts forward on a',
        'reversed brick and blows F1 (crowbar), and the D_OR Schottky already blocks',
        'reverse into VSYS. Strips see about -1 V for the clearing time of the fuse.']):
    o.append('<text x="52" y="%d" font-size="11" class="mut">%s</text>' % (812 + i * 15, ln))

o.append('<rect x="36" y="214" width="170" height="196" rx="7" fill="#fef2f2" stroke="#dc2626" '
         'stroke-width="1.5" stroke-dasharray="6 4"/>')
o.append('<text x="121" y="238" font-size="12.5" font-weight="700" text-anchor="middle" fill="#dc2626">Deleted</text>')
for i, ln in enumerate(['U11', '  carrier 3V3 reg', '  (R-78K3.3-1.0)', '', 'U14',
                        '  AP74700 ideal', '  diode — never', '  worked, needs', '  ~4 V anode', '',
                        'R?', '  0 Ω tie to the', '  Nucleo 3V3 pin']):
    o.append('<text x="52" y="%d" font-size="10.5" class="t">%s</text>' % (258 + i * 13, ln))

o.append('<text x="1324" y="996" font-size="10.5" text-anchor="end" class="mut">dmx_interface_firmware/docs/rev2-power-tree.svg</text>')
o.append('</svg>')

io.open('rev2-power-tree.svg', 'w', encoding='utf-8', newline='').write('\n'.join(o))
print('wrote rev2-power-tree.svg')
