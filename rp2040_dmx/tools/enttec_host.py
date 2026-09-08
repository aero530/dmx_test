#!/usr/bin/env python3
"""Host-side exerciser for the Enttec DMX USB Pro emulation (`enttec_test` firmware).

Finds the widget's serial port, queries Get Widget Parameters (label 3) and
Get Serial (label 10), then streams Output Only Send DMX Packet (label 6)
frames at a fixed rate with a selectable pattern.

Examples:
  python enttec_host.py                          # auto-detect, rainbow, 40 fps, until Ctrl-C
  python enttec_host.py --pattern chase --fps 30
  python enttec_host.py --pattern static --set 1=255,2=128,3=0
  python enttec_host.py --pattern blackout --duration 2
  python enttec_host.py --port COM7 --query      # identify only, send nothing

Requires pyserial:  pip install pyserial
"""

import argparse
import sys
import time

import serial
from serial.tools import list_ports

# USB identities the widget may present: the placeholder pair used by the
# CDC bench firmware (and pico2), and FTDI's FT232R pair used by a real Enttec
# Pro and by the `ftdi_test` emulator (reached through the FTDI VCP driver).
KNOWN_IDS = {(0xC0DE, 0xDCAF): "CDC widget", (0x0403, 0x6001): "FT232R"}

START, END = 0x7E, 0xE7
LABEL_GET_PARAMS = 3
LABEL_OUTPUT_DMX = 6
LABEL_GET_SERIAL = 10

UNIVERSE = 512
HUE_STEPS = 6 * 256  # same wheel resolution as the firmware tests


# --------------------------------------------------------------------------- framing


def message(label, payload=b""):
    n = len(payload)
    return bytes((START, label, n & 0xFF, n >> 8)) + bytes(payload) + bytes((END,))


class Parser:
    """Incremental parser mirroring common/src/enttec_protocol.rs."""

    def __init__(self):
        self.state = "start"
        self.label = 0
        self.length = 0
        self.payload = bytearray()

    def feed(self, byte):
        """Feed one byte; returns (label, payload) when a message completes."""
        if self.state == "start":
            if byte == START:
                self.state = "label"
        elif self.state == "label":
            self.label = byte
            self.state = "len_lsb"
        elif self.state == "len_lsb":
            self.length = byte
            self.state = "len_msb"
        elif self.state == "len_msb":
            self.length |= byte << 8
            self.payload = bytearray()
            self.state = "data" if self.length else "end"
        elif self.state == "data":
            self.payload.append(byte)
            if len(self.payload) >= self.length:
                self.state = "end"
        elif self.state == "end":
            self.state = "start"
            if byte == END:
                return self.label, bytes(self.payload)
        return None


def read_message(port, timeout):
    parser = Parser()
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        data = port.read(port.in_waiting or 1)
        for byte in data:
            msg = parser.feed(byte)
            if msg:
                return msg
    return None


def query(port, label, payload=b"", timeout=1.0):
    """Send a request and return the first reply carrying the same label."""
    port.reset_input_buffer()
    port.write(message(label, payload))
    port.flush()
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        msg = read_message(port, deadline - time.monotonic())
        if msg is None:
            return None
        if msg[0] == label:
            return msg
    return None


# --------------------------------------------------------------------------- discovery


def candidate_ports(explicit):
    if explicit:
        return [explicit]
    ports = list(list_ports.comports())
    matching = sorted(p.device for p in ports if (p.vid, p.pid) in KNOWN_IDS)
    if matching:
        for p in ports:
            if p.device in matching:
                print(f"  {p.device}: {KNOWN_IDS[(p.vid, p.pid)]} ({p.vid:04x}:{p.pid:04x})")
        return matching
    ids = ", ".join(f"{v:04x}:{p:04x}" for v, p in KNOWN_IDS)
    print(f"no port with a known VID:PID ({ids}); probing every serial port")
    return sorted(p.device for p in ports)


def open_widget(explicit):
    """Return (port, params) for the first port that answers Get Widget Parameters."""
    for dev in candidate_ports(explicit):
        try:
            port = serial.Serial(dev, 115200, timeout=0.05)
        except serial.SerialException as exc:
            print(f"  {dev}: {exc}")
            continue
        reply = query(port, LABEL_GET_PARAMS, bytes((0, 0)))
        if reply and len(reply[1]) >= 5:
            return port, reply[1]
        port.close()
        print(f"  {dev}: no Get Widget Parameters reply (log port, or not the widget)")
    return None, None


# --------------------------------------------------------------------------- patterns


def hue_to_rgb(hue, peak):
    hue %= HUE_STEPS
    ramp = hue % 256
    sector = hue // 256
    full = (
        (255, ramp, 0),  # red -> yellow
        (255 - ramp, 255, 0),  # yellow -> green
        (0, 255, ramp),  # green -> cyan
        (0, 255 - ramp, 255),  # cyan -> blue
        (ramp, 0, 255),  # blue -> magenta
        (255, 0, 255 - ramp),  # magenta -> red
    )[sector]
    return [(v * peak + 127) // 255 for v in full]


def rainbow(t, args):
    """Same picture as dmx_tx_test: RGB groups, one rainbow across the universe."""
    groups = UNIVERSE // 3
    base = int((t % args.period) / args.period * HUE_STEPS)
    channels = [0] * UNIVERSE
    for g in range(groups):
        channels[3 * g : 3 * g + 3] = hue_to_rgb(base + g * HUE_STEPS // groups, args.peak)
    return channels


def chase(t, args):
    """One channel at a time at `peak`, four channels a second, over the first 12."""
    channels = [0] * UNIVERSE
    channels[int(t * 4) % 12] = args.peak
    return channels


def static(_t, args):
    channels = [0] * UNIVERSE
    for ch, value in args.set_values:
        channels[ch - 1] = value
    return channels


def blackout(_t, _args):
    return [0] * UNIVERSE


PATTERNS = {"rainbow": rainbow, "chase": chase, "static": static, "blackout": blackout}


def parse_set(text):
    values = []
    for item in text.split(","):
        ch, _, val = item.partition("=")
        ch, val = int(ch), int(val)
        if not 1 <= ch <= UNIVERSE or not 0 <= val <= 255:
            raise argparse.ArgumentTypeError(f"bad channel=value: {item!r}")
        values.append((ch, val))
    return values


# --------------------------------------------------------------------------- main


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", help="serial port (default: auto-detect by VID/PID, then probe)")
    ap.add_argument("--pattern", choices=PATTERNS, default="rainbow")
    ap.add_argument("--set", dest="set_values", type=parse_set, default=[], help="channel=value[,...] for --pattern static")
    ap.add_argument("--fps", type=float, default=40.0, help="frames per second to send (default 40)")
    ap.add_argument("--duration", type=float, default=0.0, help="seconds to run (default: until Ctrl-C)")
    ap.add_argument("--peak", type=int, default=128, help="brightest level for rainbow/chase (default 128 = 50%%)")
    ap.add_argument("--period", type=float, default=6.0, help="rainbow cycle time in seconds (default 6)")
    ap.add_argument("--channels", type=int, default=UNIVERSE, help="channels per frame, 1-512 (default 512)")
    ap.add_argument("--hold", action="store_true", help="leave the last frame on the wire instead of blacking out on exit")
    ap.add_argument("--query", action="store_true", help="identify the widget and exit without sending DMX")
    args = ap.parse_args()

    if args.pattern == "static" and not args.set_values:
        ap.error("--pattern static needs --set channel=value[,...]")
    if not 1 <= args.channels <= UNIVERSE:
        ap.error("--channels must be 1..512")

    port, params = open_widget(args.port)
    if port is None:
        sys.exit("no Enttec widget found — is enttec_test or ftdi_test flashed, and is the Pico's USB plugged in?")

    units = 10.67  # break / MAB are reported in 10.67 us units
    print(
        f"widget on {port.name}: firmware v{params[1]}.{params[0]:02d}, "
        f"break {params[2] * units:.0f} us, MAB {params[3] * units:.0f} us, refresh {params[4]} Hz, "
        f"{len(params) - 5} user bytes"
    )
    reply = query(port, LABEL_GET_SERIAL)
    if reply and len(reply[1]) >= 4:
        sn = reply[1]
        print(f"serial number: {sn[3]:02x}{sn[2]:02x}{sn[1]:02x}{sn[0]:02x}")
    else:
        print("serial number: no reply")

    if args.query:
        port.close()
        return

    pattern = PATTERNS[args.pattern]
    interval = 1.0 / args.fps
    print(f"sending {args.pattern} at {args.fps:g} fps, {args.channels} channels — Ctrl-C to stop")

    t0 = time.monotonic()
    frame_index = 0
    sent_since_print = 0
    last_print = t0
    try:
        while True:
            now = time.monotonic()
            t = now - t0
            if args.duration and t >= args.duration:
                break

            channels = pattern(t, args)
            port.write(message(LABEL_OUTPUT_DMX, bytes([0]) + bytes(channels[: args.channels])))
            frame_index += 1
            sent_since_print += 1

            if now - last_print >= 1.0:
                rate = sent_since_print / (now - last_print)
                print(f"\r{rate:5.1f} frames/s   ch1-3 = {channels[0]:3d} {channels[1]:3d} {channels[2]:3d}   ", end="", flush=True)
                sent_since_print = 0
                last_print = now

            # Schedule against t0 so sleep jitter doesn't accumulate.
            time.sleep(max(0.0, t0 + frame_index * interval - time.monotonic()))
    except KeyboardInterrupt:
        pass
    finally:
        print()
        if not args.hold:
            port.write(message(LABEL_OUTPUT_DMX, bytes(1 + UNIVERSE)))
            port.flush()
            print("blackout sent")
        port.close()


if __name__ == "__main__":
    main()
