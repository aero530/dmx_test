import io

W, H = 1320, 880
o = []
o.append('<?xml version="1.0" encoding="UTF-8"?>')
o.append('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d" '
         'font-family="ui-sans-serif, Segoe UI, system-ui, sans-serif">' % (W, H, W, H))
o.append('<defs>'
         '<marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#33383f"/></marker>'
         '<marker id="ag" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#059669"/></marker>'
         '<style>.t{fill:#1f2328}.mut{fill:#5b6570}'
         '.mono{font-family:ui-monospace,Consolas,monospace}'
         '.w{stroke:#33383f;stroke-width:1.5;fill:none}'
         '.wg{stroke:#059669;stroke-width:2;fill:none}'
         '.wc{stroke:#0891b2;stroke-width:1.8;fill:none}</style></defs>')
o.append('<rect width="%d" height="%d" fill="#ffffff"/>' % (W, H))
o.append('<text x="36" y="40" font-size="23" font-weight="700" class="t">Rev 2 — User Interface, I²C1 Bus and Expander</text>')
o.append('<text x="36" y="63" font-size="13.5" class="mut">Four buttons and the OLED reset move onto a TCA9555, '
         'costing one GPIO (INT) and freeing GP13 as the board\'s only spare.</text>')

# --- MCU ---
o.append('<rect x="36" y="120" width="164" height="300" rx="7" fill="#e8f1fc" stroke="#1d6fd6" stroke-width="1.8"/>')
o.append('<text x="118" y="146" font-size="13" font-weight="700" text-anchor="middle" class="t">RP2350</text>')
o.append('<text x="118" y="163" font-size="10.5" text-anchor="middle" class="mut">core 1 owns the UI</text>')
for name, y in [('GP14  SPI1 SCK', 200), ('GP11  SPI1 TX', 226), ('GP12  DC', 252),
                ('GP26  I2C1 SDA', 306), ('GP27  I2C1 SCL', 332), ('GP28  INT in', 386)]:
    o.append('<text x="190" y="%d" font-size="10.5" text-anchor="end" class="mono t">%s</text>' % (y + 4, name))

# --- 3V3 rail ---
o.append('<path class="wg" d="M240 108 H 1270"/>')
o.append('<text x="248" y="102" font-size="11.5" font-weight="700" fill="#047857">3V3 — carrier rail (TLV75733 on VSYS; module 3V3(OUT) feeds nothing here)</text>')

# --- SPI to OLED ---
o.append('<path class="wc" d="M200 200 H 470" marker-end="url(#a)"/>')
o.append('<path class="wc" d="M200 226 H 470" marker-end="url(#a)"/>')
o.append('<path class="wc" d="M200 252 H 470" marker-end="url(#a)"/>')
o.append('<rect x="470" y="164" width="230" height="126" rx="7" fill="#cffafe" stroke="#0891b2" stroke-width="1.8"/>')
o.append('<text x="585" y="188" font-size="13" font-weight="700" text-anchor="middle" class="t">SSD1306 OLED</text>')
o.append('<text x="585" y="205" font-size="11" text-anchor="middle" class="mut">128×64 mono, 4-wire SPI</text>')
o.append('<text x="585" y="228" font-size="10.5" text-anchor="middle" class="mono t">SCK · MOSI · DC</text>')
o.append('<text x="585" y="246" font-size="10.5" text-anchor="middle" class="t">CS tied LOW — sole SPI1 device</text>')
o.append('<text x="585" y="264" font-size="10.5" text-anchor="middle" class="t">RES from expander P10</text>')
o.append('<text x="585" y="282" font-size="10" text-anchor="middle" class="mut">~1 ms full redraw at 8 MHz</text>')
o.append('<path class="wg" d="M585 108 V 164" marker-end="url(#ag)"/>')

# --- I2C bus rails ---
SDA_Y, SCL_Y = 306, 332
o.append('<path class="w" d="M200 %d H 1240"/>' % SDA_Y)
o.append('<path class="w" d="M200 %d H 1240"/>' % SCL_Y)
o.append('<text x="1248" y="%d" font-size="10.5" class="mono mut">SDA</text>' % (SDA_Y + 4))
o.append('<text x="1248" y="%d" font-size="10.5" class="mono mut">SCL</text>' % (SCL_Y + 4))

# pull-ups
for x, y, lbl in [(300, SDA_Y, '2k2'), (340, SCL_Y, '2k2')]:
    o.append('<path class="wg" d="M%d 108 V %d"/>' % (x, y - 40))
    o.append('<rect x="%d" y="%d" width="16" height="34" rx="2" fill="#ffffff" stroke="#059669" stroke-width="1.4"/>' % (x - 8, y - 40))
    o.append('<text x="%d" y="%d" font-size="8.5" text-anchor="middle" class="mono t">%s</text>' % (x, y - 19, lbl))
    o.append('<path class="w" d="M%d %d V %d"/>' % (x, y - 6, y))
    o.append('<circle cx="%d" cy="%d" r="3" fill="#33383f"/>' % (x, y))
o.append('<text x="322" y="150" font-size="10.5" text-anchor="middle" class="mut">one pull-up set</text>')
o.append('<text x="322" y="163" font-size="10.5" text-anchor="middle" class="mut">for the whole segment</text>')

# --- devices on the bus ---
def dev(x, w, title, addr, lines, fill, stroke):
    top = 420
    o.append('<rect x="%d" y="%d" width="%d" height="150" rx="7" fill="%s" stroke="%s" stroke-width="1.8"/>' % (x, top, w, fill, stroke))
    o.append('<text x="%d" y="%d" font-size="13" font-weight="700" text-anchor="middle" class="t">%s</text>' % (x + w / 2, top + 24, title))
    o.append('<text x="%d" y="%d" font-size="11.5" text-anchor="middle" class="mono t">%s</text>' % (x + w / 2, top + 42, addr))
    for i, ln in enumerate(lines):
        o.append('<text x="%d" y="%d" font-size="10.5" text-anchor="middle" class="mut">%s</text>' % (x + w / 2, top + 64 + i * 16, ln))
    cx = x + w / 2
    o.append('<path class="w" d="M%d %d V %d"/>' % (cx - 16, SDA_Y, top))
    o.append('<circle cx="%d" cy="%d" r="3" fill="#33383f"/>' % (cx - 16, SDA_Y))
    o.append('<path class="w" d="M%d %d V %d"/>' % (cx + 16, SCL_Y, top))
    o.append('<circle cx="%d" cy="%d" r="3" fill="#33383f"/>' % (cx + 16, SCL_Y))
    o.append('<path class="wg" d="M%d 108 V %d" marker-end="url(#ag)"/>' % (x + w - 20, top))

dev(430, 250, 'TCA9555PWR', '0x20  (A0/A1/A2 → GND)',
    ['16 I/O · internal pull-ups on inputs', 'so buttons wire straight to GND',
     'INT is open-drain → 10 kΩ to 3V3', 'VCC 1.65–5.5 V'],
    '#d1fae5', '#059669')

dev(760, 250, 'M24C02 EEPROM', '0x56  — now on the carrier',
    ['settings 0x20–0x9F (128 B)', 'liveness 0x01 · MAC 0x02–07',
     'boot flag 0x10 · schema 0x11', 'WC strapped LOW — runtime writes are core;',
     'bridge to 3V3 only to write-protect'],
    '#f3f4f6', '#6b7280')

# INT line
o.append('<path class="w" d="M200 386 H 400 V 470 H 430" marker-end="url(#a)"/>')
o.append('<rect x="352" y="404" width="16" height="34" rx="2" fill="#ffffff" stroke="#059669" stroke-width="1.4"/>')
o.append('<text x="360" y="425" font-size="8.5" text-anchor="middle" class="mono t">10k</text>')
o.append('<path class="wg" d="M360 108 V 404"/>')
o.append('<circle cx="360" cy="386" r="3" fill="#33383f"/>')
o.append('<text x="270" y="378" font-size="10" class="mut">open-drain INT</text>')

# --- buttons ---
o.append('<rect x="430" y="606" width="250" height="150" rx="7" fill="#ffffff" stroke="#33383f" stroke-width="1.6"/>')
o.append('<text x="555" y="630" font-size="12.5" font-weight="700" text-anchor="middle" class="t">4 buttons → P00–P03</text>')
for i, lbl in enumerate(['Up', 'Down', 'Select', 'Esc']):
    y = 652 + i * 24
    o.append('<text x="452" y="%d" font-size="10.5" class="mono t">P0%d</text>' % (y + 4, i))
    o.append('<path class="w" d="M492 %d H 520"/>' % y)
    o.append('<path class="w" d="M520 %d l 0 -6 M528 %d l 8 -6" stroke-linecap="round"/>' % (y, y))
    o.append('<path class="w" d="M536 %d H 560 V %d" />' % (y, y + 8))
    o.append('<path class="w" d="M552 %d H 568 M556 %d H 564" />' % (y + 8, y + 12))
    o.append('<text x="584" y="%d" font-size="10.5" class="t">%s</text>' % (y + 4, lbl))
o.append('<path class="w" d="M555 606 V 570"/>')

o.append('<rect x="760" y="606" width="250" height="150" rx="7" fill="#f6f7f9" stroke="#33383f" stroke-width="1.4"/>')
o.append('<text x="885" y="630" font-size="12.5" font-weight="700" text-anchor="middle" class="t">Remaining expander lines</text>')
o.append('<text x="885" y="654" font-size="11" text-anchor="middle" class="mono t">P10 → OLED RES</text>')
o.append('<text x="885" y="674" font-size="10.5" text-anchor="middle" class="mut">boot-time only, so I²C latency is fine</text>')
o.append('<text x="885" y="700" font-size="11" text-anchor="middle" font-weight="600" class="t">11 lines spare → test points</text>')
o.append('<text x="885" y="716" font-size="10.5" text-anchor="middle" class="mono mut">P04–P07, P11–P17</text>')
o.append('<text x="885" y="733" font-size="10.5" text-anchor="middle" class="mut">status LEDs · encoder · address DIPs</text>')
o.append('<path class="w" d="M885 606 V 570"/>')

# --- rules panel ---
o.append('<rect x="1040" y="420" width="244" height="336" rx="7" fill="#fef2f2" stroke="#dc2626" stroke-width="1.4"/>')
o.append('<text x="1162" y="446" font-size="13" font-weight="700" text-anchor="middle" fill="#dc2626">Do not move to the expander</text>')
for i, line in enumerate([
        'DMX DE // RE',
        '  switches per frame against PIO',
        '  output timing — an I²C round trip',
        '  is orders of magnitude too slow.',
        '',
        'OLED DC',
        '  toggles per transaction at SPI',
        '  speed, between command and data',
        '  bytes of every redraw.',
        '',
        'WS2812 data',
        '  800 kHz one-wire, PIO only.',
        '',
        'The expander is for things that',
        'change at human speed: buttons,',
        'resets, mode straps, indicators.']):
    o.append('<text x="1058" y="%d" font-size="10.5" class="t">%s</text>' % (472 + i * 17, line))

# --- address table ---
o.append('<rect x="36" y="606" width="360" height="150" rx="7" fill="#fbfbfa" stroke="#d4d8dd"/>')
o.append('<text x="52" y="630" font-size="12.5" font-weight="700" class="t">I²C1 address map</text>')
o.append('<text x="52" y="654" font-size="11" class="mono t">0x20   TCA9555     A0/A1/A2 = GND</text>')
o.append('<text x="52" y="674" font-size="11" class="mono t">0x56   M24C02      E1/E2 high, E0 low</text>')
o.append('<text x="52" y="700" font-size="10.5" class="mut">No collision. TCA9555 can take 0x20–0x27,</text>')
o.append('<text x="52" y="715" font-size="10.5" class="mut">so up to eight expanders share the bus if</text>')
o.append('<text x="52" y="730" font-size="10.5" class="mut">the panel ever grows.</text>')
o.append('<text x="52" y="748" font-size="10.5" class="mut">Run at 400 kHz; the OLED is on SPI, not here.</text>')

o.append('<text x="1284" y="866" font-size="10.5" text-anchor="end" class="mut">dmx_interface_firmware/docs/rev2-ui-i2c.svg</text>')
o.append('</svg>')

io.open('rev2-ui-i2c.svg', 'w', encoding='utf-8', newline='').write('\n'.join(o))
print('wrote rev2-ui-i2c.svg')
