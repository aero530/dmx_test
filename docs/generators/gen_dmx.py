import io

W, H = 1340, 1184
o = []
o.append('<?xml version="1.0" encoding="UTF-8"?>')
o.append('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d" '
         'font-family="ui-sans-serif, Segoe UI, system-ui, sans-serif">' % (W, H, W, H))
o.append('<defs>'
         '<marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#33383f"/></marker>'
         '<marker id="av" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" '
         'orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#7c3aed"/></marker>'
         '<style>.t{fill:#1f2328}.mut{fill:#5b6570}'
         '.mono{font-family:ui-monospace,Consolas,monospace}'
         '.w{stroke:#33383f;stroke-width:1.5;fill:none}'
         '.wv{stroke:#7c3aed;stroke-width:1.8;fill:none}</style></defs>')
o.append('<rect width="%d" height="%d" fill="#ffffff"/>' % (W, H))
o.append('<text x="36" y="40" font-size="23" font-weight="700" class="t">Rev 2 — Isolated DMX Front End</text>')
o.append('<text x="36" y="63" font-size="13.5" class="mut">Carried over from Rev 1 essentially unchanged. '
         'Only the MCU pins move (STM32 → RP2350 PIO), the opto LED resistors are re-valued, '
         'and the EN drive is reworked fail-safe (drawn at the bottom).</text>')

# --- isolation barrier ---
o.append('<rect x="540" y="96" width="4" height="560" fill="none"/>')
o.append('<line x1="542" y1="100" x2="542" y2="640" stroke="#7c3aed" stroke-width="2.5" stroke-dasharray="9 6"/>')
o.append('<text x="552" y="118" font-size="12" font-weight="700" fill="#7c3aed">ISOLATION BARRIER</text>')
o.append('<text x="552" y="134" font-size="10.5" fill="#7c3aed">logic GND  |  GNDISO</text>')

# --- MCU side ---
o.append('<rect x="36" y="150" width="170" height="200" rx="7" fill="#e8f1fc" stroke="#1d6fd6" stroke-width="1.8"/>')
o.append('<text x="121" y="176" font-size="13" font-weight="700" text-anchor="middle" class="t">RP2350</text>')
o.append('<text x="121" y="193" font-size="10.5" text-anchor="middle" class="mut">PIO2 — DmxInput.pio</text>')
o.append('<text x="121" y="207" font-size="10.5" text-anchor="middle" class="mut">and DmxOutput.pio</text>')
o.append('<text x="121" y="228" font-size="10" text-anchor="middle" class="mut">ported verbatim from</text>')
o.append('<text x="121" y="241" font-size="10" text-anchor="middle" class="mut">rp2040_dmx</text>')

pins = [('GP9', 'DMX TX', 250, 'out'), ('GP10', 'DE // RE', 300, 'out'), ('GP8', 'DMX RX', 350, 'in')]
for name, sig, y, d in pins:
    o.append('<text x="196" y="%d" font-size="11" text-anchor="end" class="mono t">%s</text>' % (y + 4, name))

# --- optos ---
optos = [('U15', 'TX', 250, 'fwd'), ('U14', 'DE // RE', 300, 'fwd'), ('U16', 'RX', 350, 'rev')]
for ref, lbl, y, d in optos:
    o.append('<rect x="420" y="%d" width="244" height="36" rx="5" fill="#ede9fe" stroke="#7c3aed" stroke-width="1.6"/>' % (y - 18))
    o.append('<text x="432" y="%d" font-size="11.5" font-weight="700" class="mono t">%s</text>' % (y + 4, ref))
    o.append('<text x="542" y="%d" font-size="10.5" text-anchor="middle" class="t">%s</text>' % (y - 3, lbl))
    o.append('<text x="542" y="%d" font-size="9.5" text-anchor="middle" class="mut">TLP2368</text>' % (y + 10))
    if d == 'fwd':
        o.append('<path class="w" d="M206 %d H 420" marker-end="url(#a)"/>' % y)
        o.append('<path class="wv" d="M664 %d H 730" marker-end="url(#av)"/>' % y)
    else:
        o.append('<path class="w" d="M420 %d H 214" marker-end="url(#a)"/>' % y)
        o.append('<path class="wv" d="M730 %d H 664" marker-end="url(#av)"/>' % y)

# LED drive + fail-safe callout
o.append('<rect x="676" y="374" width="238" height="88" rx="5" fill="#fffbeb" stroke="#b45309" stroke-width="1.4"/>')
o.append('<text x="688" y="392" font-size="11" font-weight="700" fill="#b45309">LED drive to spec + fail-safe EN</text>')
o.append('<text x="688" y="406" font-size="10" class="mut">R36/R37/R39 → 220 Ω (TLP2368 ≥6.5 mA); R40/R41</text>')
o.append('<text x="688" y="418" font-size="10" class="mut">output pull-ups unchanged (470 Ω, 3V3ISO).</text>')
o.append('<text x="688" y="430" font-size="10" class="mut">U14 EN reworked fail-safe: LED biased ON from 3V3</text>')
o.append('<text x="688" y="442" font-size="10" class="mut">via R46; Q1 (GP10 via R36, R47 hold-down) grounds</text>')
o.append('<text x="688" y="454" font-size="10" class="mut">the anode to TX. Undriven GP10 = RECEIVE.</text>')
o.append('<path class="w" d="M760 374 V 358" marker-end="url(#a)"/>')

# --- THVD1400 ---
o.append('<rect x="730" y="150" width="200" height="220" rx="7" fill="#f3f4f6" stroke="#33383f" stroke-width="1.8"/>')
o.append('<text x="830" y="176" font-size="13" font-weight="700" text-anchor="middle" class="mono t">THVD1400</text>')
o.append('<text x="830" y="192" font-size="10.5" text-anchor="middle" class="mut">RS-485 transceiver</text>')
o.append('<text x="830" y="206" font-size="10.5" text-anchor="middle" class="mut">on 3V3ISO</text>')
o.append('<text x="742" y="254" font-size="10.5" class="mono mut">DI</text>')
o.append('<text x="742" y="304" font-size="10.5" class="mono mut">DE, RE</text>')
o.append('<text x="742" y="354" font-size="10.5" class="mono mut">RO</text>')
o.append('<text x="918" y="264" font-size="10.5" text-anchor="end" class="mono mut">A</text>')
o.append('<text x="918" y="294" font-size="10.5" text-anchor="end" class="mono mut">B</text>')
o.append('<text x="830" y="336" font-size="9.5" text-anchor="middle" class="mut">internal fail-safe</text>')

# --- bias + termination ---
o.append('<rect x="972" y="150" width="196" height="220" rx="7" fill="#ede9fe" stroke="#7c3aed" stroke-width="1.6"/>')
o.append('<text x="1070" y="174" font-size="12.5" font-weight="700" text-anchor="middle" class="t">Bias + termination</text>')
o.append('<text x="1070" y="198" font-size="11" text-anchor="middle" class="mono t">R42  680 Ω → 3V3ISO</text>')
o.append('<text x="1070" y="216" font-size="11" text-anchor="middle" class="mono t">R43  680 Ω → GNDISO</text>')
o.append('<text x="1070" y="234" font-size="10.5" text-anchor="middle" class="mut">bias pair, always fitted —</text>')
o.append('<text x="1070" y="248" font-size="10.5" text-anchor="middle" class="mut">holds A&gt;B when the bus idles</text>')
o.append('<line x1="990" y1="264" x2="1150" y2="264" stroke="#7c3aed" stroke-width="1" stroke-dasharray="4 3"/>')
o.append('<text x="1070" y="284" font-size="11" text-anchor="middle" font-weight="600" class="t">R45 120 Ω via JP38 (N.O.)</text>')
o.append('<text x="1070" y="300" font-size="10.5" text-anchor="middle" class="mut">the ONLY termination — close</text>')
o.append('<text x="1070" y="314" font-size="10.5" text-anchor="middle" class="mut">only when last on the line</text>')
o.append('<text x="1070" y="336" font-size="10" text-anchor="middle" class="mut">with far-end term: ≈270 mV idle</text>')
o.append('<text x="1070" y="348" font-size="10" text-anchor="middle" class="mut">differential, above the 200 mV</text>')
o.append('<text x="1070" y="360" font-size="10" text-anchor="middle" class="mut">threshold; open bus: full bias</text>')
o.append('<path class="wv" d="M930 264 H 972"/>')
o.append('<path class="wv" d="M930 294 H 972"/>')

# --- XLRs ---
o.append('<rect x="1196" y="150" width="112" height="96" rx="6" fill="#ffffff" stroke="#33383f" stroke-width="1.8"/>')
o.append('<text x="1252" y="174" font-size="12" font-weight="700" text-anchor="middle" class="t">J23</text>')
o.append('<text x="1252" y="190" font-size="10.5" text-anchor="middle" class="mut">XLR-5 male</text>')
o.append('<text x="1252" y="205" font-size="10.5" text-anchor="middle" class="mut">DMX IN</text>')
o.append('<text x="1252" y="226" font-size="9.5" text-anchor="middle" class="mut">1 = GNDISO</text>')
o.append('<text x="1252" y="238" font-size="9.5" text-anchor="middle" class="mut">2 = D− · 3 = D+</text>')

o.append('<rect x="1196" y="274" width="112" height="96" rx="6" fill="#ffffff" stroke="#33383f" stroke-width="1.8"/>')
o.append('<text x="1252" y="298" font-size="12" font-weight="700" text-anchor="middle" class="t">J24</text>')
o.append('<text x="1252" y="314" font-size="10.5" text-anchor="middle" class="mut">XLR-5 female</text>')
o.append('<text x="1252" y="329" font-size="10.5" text-anchor="middle" class="mut">DMX THRU</text>')
o.append('<text x="1252" y="350" font-size="9.5" text-anchor="middle" class="mut">pins 1,2,3 (+4,5)</text>')
o.append('<text x="1252" y="362" font-size="9.5" text-anchor="middle" class="mut">bussed from J23</text>')

o.append('<path class="wv" d="M1168 200 H 1196"/>')
o.append('<path class="wv" d="M1182 200 V 320 H 1196"/>')
o.append('<circle cx="1182" cy="200" r="3" fill="#7c3aed"/>')

# --- isolated supply ---
o.append('<rect x="730" y="470" width="438" height="96" rx="7" fill="#ede9fe" stroke="#7c3aed" stroke-width="1.6"/>')
o.append('<text x="949" y="494" font-size="13" font-weight="700" text-anchor="middle" class="t">PS1 — 1S7BE isolated DC-DC, 1 W</text>')
o.append('<text x="949" y="514" font-size="11" text-anchor="middle" class="t">→ 3V3ISO / GNDISO   ·   budget ≈ 50–60 mA worst case</text>')
o.append('<text x="949" y="534" font-size="11" text-anchor="middle" fill="#047857" font-weight="600">'
         'Confirmed 3V3 input — fed from 3V3_CARRIER (the LDO on VSYS), NOT the module 3V3(OUT)</text>')
o.append('<text x="949" y="552" font-size="10.5" text-anchor="middle" class="mut">C12 tantalum on 3V3ISO (10 V rating, fine at 3.3 V)</text>')
o.append('<path stroke="#059669" stroke-width="2.2" fill="none" d="M420 518 H 730" marker-end="url(#a)"/>')
o.append('<text x="566" y="508" font-size="11.5" text-anchor="middle" font-weight="600" class="t">3V3_CARRIER</text>')
o.append('<text x="566" y="534" font-size="10.5" text-anchor="middle" class="mut">≈90 mA in — the largest single load</text>')
o.append('<text x="566" y="548" font-size="10.5" text-anchor="middle" class="mut">on the carrier 3V3 rail</text>')

# --- grounding ---
o.append('<rect x="36" y="600" width="620" height="204" rx="7" fill="#fbfbfa" stroke="#d4d8dd"/>')
o.append('<text x="52" y="624" font-size="13" font-weight="700" class="t">Grounding scheme (unchanged)</text>')
for i, line in enumerate([
        'XLR shells  →  CHGND  →  R1 (0 Ω)  →  logic GND',
        'XLR pin 1   →  GNDISO  (the isolated side)',
        '',
        'R1 stays the tuning point: fit it for a solid chassis reference, or',
        'lift it if a ground loop through the shield causes trouble in an install.',
        '',
        'This is the reason the DMX side survives ground-potential differences',
        'that would otherwise take out the transceiver. Ethernet has no',
        'equivalent protection — see the notes in REV2_PLAN.md.']):
    o.append('<text x="52" y="%d" font-size="11.5" class="mut">%s</text>' % (648 + i * 17, line))

o.append('<rect x="680" y="600" width="628" height="204" rx="7" fill="#f0f9ff" stroke="#0284c7" stroke-width="1.4"/>')
o.append('<text x="696" y="624" font-size="13" font-weight="700" fill="#0369a1">What changes in Rev 2</text>')
for i, line in enumerate([
        'The RP2040 DMX bridge is deleted. DMX moves in-process onto PIO2 of',
        'the RP2350, so the I²C bridge protocol, its 30 ms poll and the whole',
        'class of bugs review §1.1/§1.2 found in that link all disappear.',
        '',
        'MCU pins: GP8 = RX, GP9 = TX, GP10 = direction — all PIO-timed,',
        'so the existing DmxInput.pio and DmxOutput.pio move over verbatim.',
        '',
        'GP10 drives Q1, not the opto LED: undriven = RECEIVE (fail-safe),',
        'HIGH = TRANSMIT. Direction stays on a real GPIO (per-frame timing).']):
    o.append('<text x="696" y="%d" font-size="11.5" class="t">%s</text>' % (648 + i * 17, line))

# --- drawn fail-safe direction circuit (matches DMX.kicad_sch) ------------


def zz_h(x1, x2, y):
    """Horizontal zigzag resistor body between x1..x2 at height y."""
    lead, n = 8, 6
    step = (x2 - x1 - 2 * lead) / n
    pts = [(x1, y), (x1 + lead, y)]
    for i in range(n):
        pts.append((x1 + lead + step * (i + 0.5), y - 7 if i % 2 == 0 else y + 7))
    pts += [(x2 - lead, y), (x2, y)]
    return '<path class="w" d="M ' + ' L '.join('%g %g' % p for p in pts) + '"/>'


def zz_v(x, y1, y2):
    """Vertical zigzag resistor body between y1..y2 at x."""
    lead, n = 6, 6
    step = (y2 - y1 - 2 * lead) / n
    pts = [(x, y1), (x, y1 + lead)]
    for i in range(n):
        pts.append((x - 7 if i % 2 == 0 else x + 7, y1 + lead + step * (i + 0.5)))
    pts += [(x, y2 - lead), (x, y2)]
    return '<path class="w" d="M ' + ' L '.join('%g %g' % p for p in pts) + '"/>'


def gnd(x, y):
    return ('<path class="w" d="M %g %g V %g M %g %g H %g M %g %g H %g M %g %g H %g"/>'
            % (x, y, y + 8, x - 10, y + 8, x + 10, x - 6, y + 12, x + 6, x - 2.5, y + 16, x + 2.5))


def dot(x, y):
    return '<circle cx="%g" cy="%g" r="3" fill="#33383f"/>' % (x, y)


PX, PY = 36, 834                      # panel origin
o.append('<rect x="%d" y="%d" width="1272" height="310" rx="7" fill="#fffdf5" stroke="#b45309" stroke-width="1.6"/>' % (PX, PY))
o.append('<text x="52" y="%d" font-size="13.5" font-weight="700" fill="#b45309">Fail-safe direction drive — '
         'as drawn on the Rev 2 DMX sheet (R46 / R47 / Q1 new, R36 reused as gate resistor)</text>' % (PY + 28))

RY = PY + 80                          # LED chain row
o.append('<text x="100" y="%d" font-size="12" font-weight="700" text-anchor="end" class="mono t">3V3</text>' % (RY + 4))
o.append('<path class="w" d="M104 %d H 150"/>' % RY)
o.append(zz_h(150, 210, RY))
o.append('<text x="180" y="%d" font-size="11" text-anchor="middle" class="mono t">R46</text>' % (RY - 14))
o.append('<text x="180" y="%d" font-size="10.5" text-anchor="middle" class="mut">220R</text>' % (RY + 22))
o.append('<path class="w" d="M210 %d H 390"/>' % RY)
o.append(dot(330, RY))
o.append('<text x="330" y="%d" font-size="11" text-anchor="middle" class="mono t">EN_LED</text>' % (RY - 14))
# opto LED: anode triangle -> cathode bar, emission arrows up-right
o.append('<polygon points="390,%d 390,%d 408,%d" fill="#ffffff" stroke="#33383f" stroke-width="1.5"/>'
         % (RY - 9, RY + 9, RY))
o.append('<path class="w" d="M408 %d V %d M408 %d H 452"/>' % (RY - 9, RY + 9, RY))
o.append('<path class="w" d="M404 %d L 416 %d M412 %d L 424 %d" marker-end="url(#a)"/>'
         % (RY - 12, RY - 26, RY - 10, RY - 24))
o.append('<text x="399" y="%d" font-size="11" text-anchor="middle" class="mono t">U14 LED</text>' % (RY + 26))
o.append('<text x="399" y="%d" font-size="10" text-anchor="middle" class="mut">EN opto, 1-3</text>' % (RY + 40))
o.append(gnd(452, RY))

# Q1 vertical branch off EN_LED
FGY = RY + 105                        # gate row
o.append('<path class="w" d="M330 %d V %d"/>' % (RY, FGY - 16))            # drop to drain tap
o.append('<path class="w" d="M318 %d H 330"/>' % (FGY - 16))               # drain lead
o.append('<path class="w" d="M318 %d H 330 M330 %d V %d"/>' % (FGY + 16, FGY + 16, FGY + 30))
o.append(gnd(330, FGY + 30))                                               # source to GND
o.append('<path class="w" d="M318 %d V %d M318 %d V %d M318 %d V %d"/>'    # channel dashes
         % (FGY - 22, FGY - 8, FGY - 6, FGY + 6, FGY + 8, FGY + 22))
o.append('<path class="w" d="M310 %d V %d"/>' % (FGY - 22, FGY + 22))      # gate plate
o.append('<polygon points="328,%d 320,%d 328,%d" fill="#33383f"/>' % (FGY + 12, FGY + 16, FGY + 20))
o.append('<text x="345" y="%d" font-size="11.5" font-weight="700" class="mono t">Q1</text>' % (FGY - 4))
o.append('<text x="345" y="%d" font-size="10.5" class="mut">2N7002</text>' % (FGY + 11))

# gate network: GP10 -> R36 -> gate, R47 hold-down
o.append('<text x="100" y="%d" font-size="12" font-weight="700" text-anchor="end" class="mono t">GP10</text>' % (FGY + 4))
o.append('<path class="w" d="M104 %d H 150"/>' % FGY)
o.append(zz_h(150, 210, FGY))
o.append('<text x="180" y="%d" font-size="11" text-anchor="middle" class="mono t">R36</text>' % (FGY - 14))
o.append('<text x="180" y="%d" font-size="10.5" text-anchor="middle" class="mut">220R (Rev 1 part, reused)</text>' % (FGY + 24))
o.append('<path class="w" d="M210 %d H 310"/>' % FGY)
o.append(dot(260, FGY))
o.append('<text x="260" y="%d" font-size="11" text-anchor="middle" class="mono t">EN_GATE</text>' % (FGY - 14))
o.append('<path class="w" d="M260 %d V %d"/>' % (FGY, FGY + 20))
o.append(zz_v(260, FGY + 20, FGY + 65))
o.append('<text x="243" y="%d" font-size="11" text-anchor="end" class="mono t">R47</text>' % (FGY + 40))
o.append('<text x="243" y="%d" font-size="10.5" text-anchor="end" class="mut">100k</text>' % (FGY + 54))
o.append(gnd(260, FGY + 65))

# behaviour annotations
AX = 560
for dy, cls, fw, line in [
        (46, 't', '700', 'GP10 undriven or LOW → Q1 off → 3V3 drives 8 mA through R46 + LED → opto ON → DE/RE low = RECEIVE'),
        (64, 'mut', '400', 'the fail-safe state: boot, reset, BOOTSEL/reflash, firmware crash, module absent — all land here'),
        (94, 't', '700', 'GP10 HIGH → Q1 on → EN_LED node grounded → LED off → R40 (3V3ISO) pulls DE/RE high = TRANSMIT'),
        (112, 'mut', '400', '15 mA flows 3V3 → R46 → Q1 only while transmitting; GP10 itself sources only Q1 gate leakage'),
        (142, 'mut', '400', 'R36 keeps its Rev 1 position (GP10 → R36) and becomes the gate series resistor; R47 defines the'),
        (160, 'mut', '400', 'gate while GP10 floats. Firmware convention unchanged: GP10 HIGH = TX, LOW/Hi-Z = RX.'),
        (190, 'mut', '400', 'The isolated side is untouched: U14 open-collector output + R40 470R pull-up → THVD1400 DE and /RE,'),
        (208, 'mut', '400', 'exactly as Rev 1 drew it. All new parts live on the logic side of the barrier.'),
        (238, 'mut', '400', 'Replaces the Rev 1 topology (GP10 sinks the LED directly), which was fail-safe-TRANSMIT — every'),
        (256, 'mut', '400', 'boot had a ms-scale window where a floating GP10 let R40 enable the RS-485 driver onto the bus.')]:
    o.append('<text x="%d" y="%d" font-size="11.5" font-weight="%s" class="%s">%s</text>' % (AX, PY + dy, fw, cls, line))

o.append('<text x="1304" y="%d" font-size="10.5" text-anchor="end" class="mut">dmx_interface_firmware/docs/rev2-dmx-isolated.svg</text>' % (H - 14))
o.append('</svg>')

io.open('rev2-dmx-isolated.svg', 'w', encoding='utf-8', newline='').write('\n'.join(o))
print('wrote rev2-dmx-isolated.svg')
