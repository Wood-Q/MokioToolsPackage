#!/usr/bin/env python3
"""Generate Mokio's app icons using only the Python stdlib.

Produces (in this directory):
  32x32.png, 128x128.png   — referenced by tauri.conf.json
  icon.icns                — built via `iconutil` from a generated .iconset
  icon.ico                 — an ICO wrapping a 256x256 PNG

Design: a teal rounded square with a dark checkmark ("install done").
Re-run any time: `python3 icons/gen_icons.py`.
"""
import math
import os
import struct
import subprocess
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
TEAL = (90, 209, 199)
DARK = (8, 26, 22)

# Normalised checkmark polyline (x,y) in 0..1 space.
POLY = [(0.30, 0.55), (0.46, 0.72), (0.74, 0.33)]


def _seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    if dx == 0 and dy == 0:
        return math.hypot(px - ax, py - ay)
    t = ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)
    t = max(0.0, min(1.0, t))
    cx, cy = ax + t * dx, ay + t * dy
    return math.hypot(px - cx, py - cy)


def render(size):
    """Return RGBA bytes for an icon of the given pixel size."""
    s = size
    buf = bytearray(s * s * 4)
    pts = [(x * s, y * s) for x, y in POLY]
    half = max(2.0, 0.075 * s)
    radius = 0.16 * s  # rounded-square corner radius
    margin = 0.04 * s
    for y in range(s):
        for x in range(s):
            i = (y * s + x) * 4
            # rounded-square mask (transparent outside corners)
            in_x = margin <= x < s - margin
            in_y = margin <= y < s - margin
            if not (in_x and in_y):
                buf[i : i + 4] = b"\x00\x00\x00\x00"
                continue
            # corner test
            cx = min(x - margin, s - margin - 1 - x)
            cy = min(y - margin, s - margin - 1 - y)
            if cx < radius and cy < radius:
                if math.hypot(radius - cx, radius - cy) > radius:
                    buf[i : i + 4] = b"\x00\x00\x00\x00"
                    continue
            r, g, b = TEAL
            d = min(_seg_dist(x + 0.5, y + 0.5, pts[0][0], pts[0][1], pts[1][0], pts[1][1]),
                    _seg_dist(x + 0.5, y + 0.5, pts[1][0], pts[1][1], pts[2][0], pts[2][1]))
            if d <= half:
                r, g, b = DARK
            buf[i], buf[i + 1], buf[i + 2], buf[i + 3] = r, g, b, 255
    return bytes(buf)


def write_png(path, size):
    raw = b""
    stride = size * 4
    px = render(size)
    for y in range(size):
        raw += b"\x00" + px[y * stride : (y + 1) * stride]

    def chunk(typ, data):
        return (
            struct.pack(">I", len(data))
            + typ
            + data
            + struct.pack(">I", zlib.crc32(typ + data) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    idat = zlib.compress(raw, 9)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", ihdr))
        f.write(chunk(b"IDAT", idat))
        f.write(chunk(b"IEND", b""))
    print("wrote", path, size)


def write_ico(path, size):
    """ICO wrapping a PNG entry (valid for modern Windows + accepted by Tauri)."""
    raw = b""
    stride = size * 4
    px = render(size)
    for y in range(size):
        raw += b"\x00" + px[y * stride : (y + 1) * stride]

    def chunk(typ, data):
        return (
            struct.pack(">I", len(data))
            + typ
            + data
            + struct.pack(">I", zlib.crc32(typ + data) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    w = 0 if size >= 256 else size
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", w, w, 0, 0, 1, 32, len(png), 22)
    with open(path, "wb") as f:
        f.write(header + entry + png)
    print("wrote", path, "(ico wraps", size, "png)")


def build_icns():
    iconset = os.path.join(HERE, "icon.iconset")
    os.makedirs(iconset, exist_ok=True)
    spec = {
        "icon_16x16.png": 16,
        "icon_16x16@2x.png": 32,
        "icon_32x32.png": 32,
        "icon_32x32@2x.png": 64,
        "icon_128x128.png": 128,
        "icon_128x128@2x.png": 256,
        "icon_256x256.png": 256,
        "icon_256x256@2x.png": 512,
        "icon_512x512.png": 512,
        "icon_512x512@2x.png": 1024,
    }
    for name, size in spec.items():
        write_png(os.path.join(iconset, name), size)
    out = os.path.join(HERE, "icon.icns")
    subprocess.run(["iconutil", "-c", "icns", iconset, "-o", out], check=True)
    subprocess.run(["rm", "-rf", iconset], check=True)
    print("wrote", out)


def main():
    write_png(os.path.join(HERE, "32x32.png"), 32)
    write_png(os.path.join(HERE, "128x128.png"), 128)
    write_ico(os.path.join(HERE, "icon.ico"), 256)
    build_icns()


if __name__ == "__main__":
    main()
