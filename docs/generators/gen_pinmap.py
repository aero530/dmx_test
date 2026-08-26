import io

# physical pin -> (name, signal, category)
L = [(1, 'GP0', 'WS2812 DATA 1', 'ws'), (2, 'GP1', 'WS2812 DATA 2', 'ws'), (3, 'GND', '', 'gnd'),
     (4, 'GP2', 'WS2812 DATA 3', 'ws'), (5, 'GP3', 'WS2812 DATA 4', 'ws'), (6, 'GP4', 'WS2812 DATA 5', 'ws'),
     (7, 'GP5', 'WS2812 DATA 6', 'ws'), (8, 'GND', '', 'gnd'), (9, 'GP6', 'WS2812 DATA 7', 'ws'),
     (10, 'GP7', 'WS2812 DATA 8', 'ws'), (11, 'GP8', 'DMX RX  (PIO)', 'dmx'), (12, 'GP9', 'DMX TX  (PIO)', 'dmx'),
     (13, 'GND', '', 'gnd'), (14, 'GP10', 'DMX DE / RE', 'dmx'), (15, 'GP11', 'OLED MOSI  SPI1 TX', 'oled'),
     (16, 'GP12', 'OLED DC', 'oled'), (17, 'GP13', 'SPARE → test point', 'spare'), (18, 'GND', '', 'gnd'),
     (19, 'GP14', 'OLED SCK  SPI1 SCK', 'oled'), (20, 'GP15', 'W6300 INT', 'w6300')]

R = [(21, 'GP16', 'W6300 CSn', 'w6300'), (22, 'GP17', 'W6300 SCLK', 'w6300'), (23, 'GND', '', 'gnd'),
     (24, 'GP18', 'W6300 IO0 / MOSI', 'w6300'), (25, 'GP19', 'W6300 IO1 / MISO', 'w6300'),
     (26, 'GP20', 'W6300 IO2', 'w6300'), (27, 'GP21', 'W6300 IO3', 'w6300'), (28, 'GND', '', 'gnd'),
     (29, 'GP22', 'W6300 RSTn', 'w6300'), (30, 'RUN', 'reset - button optional', 'sys'),
     (31, 'GP26', 'I2C1 SDA', 'i2c'), (32, 'GP27', 'I2C1 SCL', 'i2c'), (33, 'AGND', '', 'gnd'),
     (34, 'GP28', 'TCA9555 INT', 'i2c'), (35, 'ADC_VREF', 'not used', 'nc'),
     (36, '3V3(OUT)', 'OUTPUT ONLY -> carrier logic', 'pwr'), (37, '3V3_EN', 'leave n/c', 'nc'),
     (38, 'GND', '', 'gnd'), (39, 'VSYS', '5 V in via Schottky D_OR', 'pwr'), (40, 'VBUS', 'leave n/c', 'nc')]

C = {'ws': ('#1d6fd6', '#e8f1fc'), 'dmx': ('#7c3aed', '#ede9fe'), 'oled': ('#0891b2', '#cffafe'),
     'i2c': ('#059669', '#d1fae5'), 'w6300': ('#dc2626', '#fee2e2'), 'pwr': ('#b45309', '#fef3c7'),
     'gnd': ('#6b7280', '#f3f4f6'), 'spare': ('#16a34a', '#dcfce7'), 'nc': ('#9ca3af', '#fafafa'),
     'sys': ('#6b7280', '#f9fafb')}

W, H = 1300, 1120
BX, BY, BW = 470, 150, 300
STEP, TOP = 34, 190
o = []
o.append('<?xml version="1.0" encoding="UTF-8"?>')
o.append('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d" '
         'font-family="ui-sans-serif, Segoe UI, system-ui, sans-serif">' % (W, H, W, H))
o.append('<style>.t{fill:#1f2328}.mut{fill:#5b6570}'
         '.mono{font-family:ui-monospace,Consolas,monospace}</style>')
o.append('<rect width="%d" height="%d" fill="#ffffff"/>' % (W, H))
o.append('<text x="36" y="40" font-size="23" font-weight="700" class="t">'
         'Rev 2 — W6300-EVB-Pico2 Pin Allocation</text>')
o.append('<text x="36" y="63" font-size="13.5" class="mut">All 40 pins. GP15–22 are committed to the W6300 '
         'inside the module and cannot be reused. One spare GPIO (GP13) remains.</text>')

body_h = TOP - BY + 19 * STEP + 26
o.append('<rect x="%d" y="%d" width="%d" height="%d" rx="10" fill="#f6f7f9" stroke="#33383f" stroke-width="2"/>'
         % (BX, BY, BW, body_h))
o.append('<text x="%d" y="%d" font-size="14" font-weight="700" text-anchor="middle" class="t">W6300-EVB-Pico2</text>'
         % (BX + BW / 2, BY + 30))
o.append('<text x="%d" y="%d" font-size="11" text-anchor="middle" class="mut">RP2350 · A4 stepping · socketed 2×20</text>'
         % (BX + BW / 2, BY + 48))
o.append('<rect x="%d" y="%d" width="%d" height="46" rx="5" fill="#fee2e2" stroke="#dc2626" stroke-width="1.5"/>'
         % (BX + 60, BY + 64, BW - 120))
o.append('<text x="%d" y="%d" font-size="11.5" font-weight="600" text-anchor="middle" class="t">W6300 + RJ45</text>'
         % (BX + BW / 2, BY + 84))
o.append('<text x="%d" y="%d" font-size="10" text-anchor="middle" class="mut">on-module, consumes GP15–22</text>'
         % (BX + BW / 2, BY + 100))


def row(px, name, sig, cat, y, side):
    st, fl = C[cat]
    pw, ph = 118, 26
    if side == 'L':
        x = BX - pw - 100
        o.append('<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="%s" stroke-width="2"/>' % (BX, y, BX - 14, y, st))
        o.append('<rect x="%d" y="%d" width="26" height="18" rx="3" fill="#ffffff" stroke="#33383f" stroke-width="1.1"/>'
                 % (BX - 40, y - 9))
        o.append('<text x="%d" y="%d" font-size="10" text-anchor="middle" class="mono t">%d</text>' % (BX - 27, y + 4, px))
        o.append('<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="%s" stroke-width="1.4"/>' % (x + pw, y, BX - 40, y, st))
        o.append('<rect x="%d" y="%d" width="%d" height="%d" rx="4" fill="%s" stroke="%s" stroke-width="1.4"/>'
                 % (x, y - ph / 2, pw, ph, fl, st))
        o.append('<text x="%d" y="%d" font-size="11" class="mono t">%s</text>' % (x + 8, y + 4, name))
        if sig:
            o.append('<text x="%d" y="%d" font-size="11" text-anchor="end" class="t">%s</text>' % (x - 10, y + 4, sig))
    else:
        x = BX + BW + 100
        o.append('<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="%s" stroke-width="2"/>' % (BX + BW, y, BX + BW + 14, y, st))
        o.append('<rect x="%d" y="%d" width="26" height="18" rx="3" fill="#ffffff" stroke="#33383f" stroke-width="1.1"/>'
                 % (BX + BW + 14, y - 9))
        o.append('<text x="%d" y="%d" font-size="10" text-anchor="middle" class="mono t">%d</text>' % (BX + BW + 27, y + 4, px))
        o.append('<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="%s" stroke-width="1.4"/>' % (BX + BW + 40, y, x, y, st))
        o.append('<rect x="%d" y="%d" width="%d" height="%d" rx="4" fill="%s" stroke="%s" stroke-width="1.4"/>'
                 % (x, y - ph / 2, pw, ph, fl, st))
        o.append('<text x="%d" y="%d" font-size="11" class="mono t">%s</text>' % (x + 8, y + 4, name))
        if sig:
            o.append('<text x="%d" y="%d" font-size="11" class="t">%s</text>' % (x + pw + 10, y + 4, sig))


for i, (px, name, sig, cat) in enumerate(L):
    row(px, name, sig, cat, TOP + i * STEP, 'L')
for j, (px, name, sig, cat) in enumerate(R):
    row(px, name, sig, cat, TOP + (19 - j) * STEP, 'R')

ly = TOP + 19 * STEP + 60
o.append('<rect x="36" y="%d" width="1228" height="52" rx="7" fill="#fbfbfa" stroke="#d4d8dd"/>' % ly)
items = [('ws', 'WS2812 out (8)'), ('dmx', 'Wired DMX (3)'), ('oled', 'OLED SPI1 (3)'), ('i2c', 'I²C1 + INT (3)'),
         ('w6300', 'W6300 — unavailable (8)'), ('pwr', 'Power'), ('spare', 'Spare → test point (1)'), ('nc', 'Not connected')]
for k, (cat, label) in enumerate(items):
    st, fl = C[cat]
    x = 52 + k * 152
    o.append('<rect x="%d" y="%d" width="15" height="12" rx="2" fill="%s" stroke="%s" stroke-width="1.3"/>'
             % (x, ly + 19, fl, st))
    o.append('<text x="%d" y="%d" font-size="11" class="t">%s</text>' % (x + 22, ly + 29, label))

ny = ly + 80
o.append('<text x="36" y="%d" font-size="12" font-weight="700" class="t">Mux checks</text>' % ny)
notes = [
    'GP26 = I2C1 SDA and GP27 = I2C1 SCL — a valid hardware I²C1 pair.',
    'GP11 = SPI1 TX, GP14 = SPI1 SCK, GP13 = SPI1 CSn. The OLED is alone on SPI1, so CS ties low and GP13 stays spare; GP12 is a plain GPIO for DC.',
    'WS2812 and DMX are all PIO, so any pin works — GP0–7 and GP8–10 were chosen to keep them contiguous for the level shifter and the RS-485 header.',
    'GP17 (W6300 SCLK) sits on a CSn mux position, not SCK. That is exactly why the transport must be PIO even in single-SPI mode.',
    'GP23/24/25/29 are absent by design — the Pico 2 keeps four GPIOs internal (SMPS PS, VBUS sense, user LED, VSYS/3 ADC), so GP13 is the only spare.',
]
for i, line in enumerate(notes):
    o.append('<text x="36" y="%d" font-size="11.5" class="mut">•  %s</text>' % (ny + 20 + i * 18, line))
o.append('<text x="1264" y="%d" font-size="10.5" text-anchor="end" class="mut">dmx_interface_firmware/docs/rev2-pinmap.svg</text>' % (ny + 100))
o.append('</svg>')

io.open('rev2-pinmap.svg', 'w', encoding='utf-8', newline='').write('\n'.join(o))
print('wrote rev2-pinmap.svg')
