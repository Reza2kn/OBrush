#!/usr/bin/env python3
"""Author a CC0 curved Pinch comparison mesh with an independent SGL v3 writer."""

from pathlib import Path
import hashlib
import json
import math
import struct

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "tests/fixtures/independent-source-pinch-c308.sgl"
META = DEST.with_suffix(".json")
SIDE = 49
SPAN = 0.84
TRIANGLE = 0xFFFFFFFF


def fword(value):
    return struct.unpack("<I", struct.pack("<f", float(value)))[0]


def block(rows, width):
    if any(len(row) != width for row in rows):
        raise ValueError("invalid row width")
    return [len(rows), *(part for row in rows for part in row)]


vertices, colors, materials, faces = [], [], [], []
for iy in range(SIDE):
    y = (iy / (SIDE - 1) - 0.5) * SPAN
    for ix in range(SIDE):
        x = (ix / (SIDE - 1) - 0.5) * SPAN
        z = 0.028 + 0.009 * math.sin(8.0 * x + 2.0 * y) + 0.006 * math.cos(6.0 * y - x)
        vertices.append([fword(x), fword(y), fword(z)])
        rgb_id = len(vertices) - 1
        colors.append([fword(((rgb_id >> shift) & 255) / 255) for shift in (0, 8, 16)])
        protection = 0.0 if (ix, iy) == (25, 24) else 0.5 if (ix, iy) == (26, 24) else 1.0
        materials.append([fword(0.5), fword(0.5), fword(protection)])

for iy in range(SIDE - 1):
    for ix in range(SIDE - 1):
        a = iy * SIDE + ix
        b = a + 1
        c = a + SIDE
        d = c + 1
        faces.extend(([a, b, d, TRIANGLE], [a, d, c, TRIANGLE]))

identity = [fword(x) for x in (1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)]
words = [3, 1, 0, 1, 0, 0, fword(40), 1, 1]
words += [0, 0, 0, 0, fword(1)]
words += [fword(0), fword(0), fword(0), *identity, fword(1)]
for data, width in ((vertices, 3), (colors, 3), (materials, 3), (faces, 4), ([], 2), ([], 4)):
    words += block(data, width)
encoded = struct.pack("<" + "I" * len(words), *words)
DEST.write_bytes(encoded)
manifest = {
    "source": "Independent Python-authored SGL v3; no SculptGL or OBrush encoder",
    "license": "CC0",
    "fixture": str(DEST.relative_to(ROOT)),
    "bytes": len(encoded),
    "sha256": hashlib.sha256(encoded).hexdigest(),
    "grid_side": SIDE,
    "vertices": len(vertices),
    "triangles": len(faces),
    "protected_rgb_id": 24 * SIDE + 25,
    "half_unprotected_rgb_id": 24 * SIDE + 26,
    "z_formula": "0.028 + 0.009*sin(8*x+2*y) + 0.006*cos(6*y-x)",
}
META.write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps(manifest))
