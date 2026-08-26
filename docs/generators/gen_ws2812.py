import io

W, H = 1320, 960
o = []
o.append('<?xml version="1.0" encoding="UTF-8"?>')
o.append('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d" '
         'font-family="ui-sans-serif, Segoe UI, system-ui, sans-serif">' % (W, H, W, H))
o.append('<defs>'
         '<marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#33383f"/></marker>'
         '<marker id="ap" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#b45309"/></marker>'
         '<style>.t{fill:#1f2328}.mut{fill:#5b6570}'
         '.mono{font-family:ui-monospace,Consolas,monospace}'
         '.w{stroke:#33383f;stroke-width:1.5;fill:none}'
         '.wp{stroke:#b45309;stroke-width:2.2;fill:none}</style></defs>')
o.append('<rect width="%d" height="%d" fill="#ffffff"/>' % (W, H))
o.append('<text x="36" y="40" font-size="23" font-weight="700" class="t">Rev 2 — WS2812 Output Stage (8 channels)</text>')
o.append('<text x="36" y="63" font-size="13.5" class="mut">Single-wire WS2812 only. Rev 1\'s per-channel CLK line is dropped, '
         'which is what makes 8 strings fit the pin budget.</text>')

# ---- MCU stub ----
o.append('<rect x="36" y="120" width="150" height="330" rx="7" fill="#e8f1fc" stroke="#1d6fd6" stroke-width="1.8"/>')
o.append('<text x="111" y="146" font-size="13" font-weight="700" text-anchor="middle" class="t">RP2350</text>')
o.append('<text x="111" y="163" font-size="10.5" text-anchor="middle" class="mut">PIO0 ×3 · PIO1 ×4</text>')
o.append('<text x="111" y="177" font-size="10.5" text-anchor="middle" class="mut">PIO2 ×1</text>')
o.append('<text x="111" y="198" font-size="10.5" text-anchor="middle" class="mut">3V3 CMOS out</text>')

# ---- U10 body ----
UX, UY, UW, UH = 430, 120, 250, 330
o.append('<rect x="%d" y="%d" width="%d" height="%d" rx="7" fill="#f3f4f6" stroke="#33383f" stroke-width="1.8"/>' % (UX, UY, UW, UH))
o.append('<text x="%d" y="146" font-size="14" font-weight="700" text-anchor="middle" class="t">U10  SN74ACT245</text>' % (UX + UW / 2))
o.append('<text x="%d" y="163" font-size="10.5" text-anchor="middle" class="mut">octal transceiver, single 5 V supply</text>' % (UX + UW / 2))
o.append('<text x="%d" y="177" font-size="10.5" text-anchor="middle" class="mut">TTL input thresholds accept 3V3 drive</text>' % (UX + UW / 2))

TOP, STEP = 210, 29
for i in range(8):
    y = TOP + i * STEP
    o.append('<text x="176" y="%d" font-size="11" text-anchor="end" class="mono t">GP%d</text>' % (y + 4, i))
    o.append('<path class="w" d="M186 %d H %d" marker-end="url(#a)"/>' % (y, UX))
    o.append('<text x="%d" y="%d" font-size="10.5" class="mono mut">A%d</text>' % (UX + 8, y + 4, i + 1))
    o.append('<text x="%d" y="%d" font-size="10.5" text-anchor="end" class="mono mut">B%d</text>' % (UX + UW - 8, y + 4, i + 1))
    o.append('<path class="w" d="M%d %d H 760" marker-end="url(#a)"/>' % (UX + UW, y))
    # series resistor
    o.append('<rect x="762" y="%d" width="40" height="14" rx="2" fill="#ffffff" stroke="#33383f" stroke-width="1.3"/>' % (y - 7))
    o.append('<text x="782" y="%d" font-size="8.5" text-anchor="middle" class="mono t">330R</text>' % (y + 3))
    o.append('<path class="w" d="M802 %d H 880" marker-end="url(#a)"/>' % y)
    # connector
    o.append('<rect x="880" y="%d" width="150" height="22" rx="3" fill="#ffffff" stroke="#33383f" stroke-width="1.4"/>' % (y - 11))
    o.append('<text x="890" y="%d" font-size="10.5" class="mono t">J_LED%d</text>' % (y + 4, i + 1))
    o.append('<text x="1022" y="%d" font-size="9.5" text-anchor="end" class="mut">DATA · 5V · GND</text>' % (y + 4))
    # polyfuse on 5V per output
    o.append('<rect x="1064" y="%d" width="46" height="16" rx="8" fill="#fef3c7" stroke="#b45309" stroke-width="1.3"/>' % (y - 8))
    o.append('<text x="1087" y="%d" font-size="8.5" text-anchor="middle" class="mono t">PF%d</text>' % (y + 3, i + 1))
    o.append('<path class="wp" d="M1064 %d H 1030" marker-end="url(#ap)"/>' % y)
    o.append('<text x="1122" y="%d" font-size="10" class="mut">string %d — up to 600 LEDs</text>' % (y + 4, i + 1))

# DIR / OE straps
o.append('<path class="w" d="M%d 462 H %d V 450" marker-end="url(#a)"/>' % (UX - 70, UX + 40))
o.append('<text x="%d" y="478" font-size="10.5" text-anchor="middle" class="mono t">DIR = 1 (A→B)</text>' % (UX - 10))
o.append('<text x="%d" y="492" font-size="10" text-anchor="middle" class="mut">strapped — JP21 deleted</text>' % (UX - 10))
o.append('<path class="w" d="M%d 462 H %d V 450" marker-end="url(#a)"/>' % (UX + UW + 70, UX + UW - 40))
o.append('<text x="%d" y="478" font-size="10.5" text-anchor="middle" class="mono t">OE = 0</text>' % (UX + UW + 20))
o.append('<text x="%d" y="492" font-size="10" text-anchor="middle" class="mut">always enabled</text>' % (UX + UW + 20))

# VCC feed
o.append('<path class="wp" d="M%d 100 V %d" marker-end="url(#ap)"/>' % (UX + UW / 2, UY))
o.append('<text x="%d" y="94" font-size="11" text-anchor="middle" font-weight="600" class="t">5V_LVL_SHIFT</text>' % (UX + UW / 2))
o.append('<text x="%d" y="80" font-size="10" text-anchor="middle" class="mut">100 nF per VCC pin + bulk</text>' % (UX + UW / 2))

# 5V rail bus to polyfuses
o.append('<path class="wp" d="M1180 100 V %d" />' % (TOP + 7 * STEP + 20))
o.append('<text x="1188" y="94" font-size="11" font-weight="600" class="t">LED 5 V rail</text>')
for i in range(8):
    y = TOP + i * STEP
    o.append('<path class="wp" d="M1180 %d H 1110"/>' % y)
    o.append('<circle cx="1180" cy="%d" r="3" fill="#b45309"/>' % y)

# ---- notes ----
NY = 530
o.append('<rect x="36" y="%d" width="620" height="196" rx="7" fill="#fbfbfa" stroke="#d4d8dd"/>' % NY)
o.append('<text x="52" y="%d" font-size="13" font-weight="700" class="t">Why the level shifter is needed</text>' % (NY + 24))
for i, line in enumerate([
        'WS2812B needs VIH = 0.7 × VDD, i.e. 3.5 V on a 5 V supply.',
        'A 3V3 GPIO does not reliably meet that — it works on the bench and',
        'fails on a cold morning at the end of a long cable.',
        '',
        'The SN74ACT245 is a single-supply part on 5 V, not a dual-rail',
        'translator. It works here because ACT has TTL input thresholds',
        '(VIH = 2.0 V), so 3V3 CMOS drive is comfortably a valid logic 1.',
        'Do not substitute an AC/HC part — those have CMOS thresholds',
        'at 0.7 × VCC = 3.5 V and would not accept 3V3 in.']):
    o.append('<text x="52" y="%d" font-size="11.5" class="mut">%s</text>' % (NY + 48 + i * 17, line))

o.append('<rect x="680" y="%d" width="604" height="196" rx="7" fill="#fef2f2" stroke="#dc2626" stroke-width="1.4"/>' % NY)
o.append('<text x="696" y="%d" font-size="13" font-weight="700" fill="#dc2626">Rev 1 issues fixed here</text>' % (NY + 24))
for i, line in enumerate([
        'Channel numbering was reversed — U10 A1/A2 carried LED4 data, so',
        'connector "LEDn" actually drove channel (5 − n). Fix the A-side order',
        'in Rev 2 rather than patching the silkscreen.',
        '',
        'JP21 let DIR be pulled low, turning the A side into 5 V outputs driving',
        'straight into the MCU (review §2.2). DIR is now strapped; JP21 is gone.',
        '',
        'No per-output fusing (§3.7) — a shorted strip cable made the wiring the',
        'fuse. PF1–PF8 now sit in each output\'s 5 V leg.']):
    o.append('<text x="696" y="%d" font-size="11.5" class="t">%s</text>' % (NY + 48 + i * 17, line))

o.append('<rect x="36" y="%d" width="1248" height="52" rx="7" fill="#fef2f2" stroke="#dc2626" stroke-width="1.4"/>' % (NY + 206))
o.append('<text x="52" y="%d" font-size="12" font-weight="700" fill="#dc2626">Do NOT use an auto-direction translator here</text>' % (NY + 226))
o.append('<text x="52" y="%d" font-size="11" class="t">TXS0108E and friends sense direction with one-shot edge accelerators and internal 10 kΩ pull-ups. They are built for bidirectional '
         'open-drain buses (I²C), drive weakly, and</text>' % (NY + 244))
o.append('<text x="52" y="%d" font-size="11" class="t">false-trigger on the reflections a strip cable produces — a documented failure mode with WS2812. This path is unidirectional and needs a real push-pull driver.</text>' % (NY + 258))
o.append('<text x="36" y="%d" font-size="11.5" class="mut">Long runs: inject 5 V at the far end of each string and tie grounds back — '
         'the 330 Ω series R damps edges at the source but does nothing for volt-drop along the strip.</text>' % (NY + 282))
o.append('<text x="36" y="%d" font-size="11.5" class="mut">Refresh: 600 LEDs × 24 bits × 1.25 µs = 18.0 ms per frame. All eight strings clock out in parallel on separate PIO state machines, '
         'so the frame time is set by the longest string, not the total.</text>' % (NY + 304))
o.append('<text x="1284" y="880" font-size="10.5" text-anchor="end" class="mut">dmx_interface_firmware/docs/rev2-ws2812-output.svg</text>')
o.append('</svg>')

io.open('rev2-ws2812-output.svg', 'w', encoding='utf-8', newline='').write('\n'.join(o))
print('wrote rev2-ws2812-output.svg')
