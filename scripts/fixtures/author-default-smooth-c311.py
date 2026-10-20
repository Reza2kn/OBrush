#!/usr/bin/env python3
"""Author an independent asymmetric curved SGL v3 default-Smooth fixture."""

from pathlib import Path
import hashlib
import json
import math
import struct

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "tests/fixtures/independent-default-smooth-c311.sgl"
META = ROOT / "tests/fixtures/independent-default-smooth-c311.json"
SIDE = 25
WIDTH = 0.56
TRIANGLE = 0xFFFFFFFF


def word(value):
    return struct.unpack("<I", struct.pack("<f", float(value)))[0]


def rows(values, width):
    assert all(len(row) == width for row in values)
    return [len(values), *(item for row in values for item in row)]


vertices, colors, materials, faces = [], [], [], []
for iy in range(SIDE):
    y = (iy / (SIDE - 1) - 0.5) * WIDTH
    for ix in range(SIDE):
        x = (ix / (SIDE - 1) - 0.5) * WIDTH
        ridge = 0.005 * math.sin(ix * 0.57 + iy * 0.19)
        curved = 0.013 * math.exp(-((x - 0.035) ** 2 + (y + 0.025) ** 2) / 0.004)
        spike = 0.011 if (ix, iy) == (12, 13) else 0.0
        z = 0.018 + ridge + curved + spike
        vertices.append([word(x), word(y), word(z)])
        identity = len(vertices) - 1
        colors.append([word(((identity >> shift) & 255) / 255) for shift in (0, 8, 16)])
        protection = 0.0 if (ix, iy) == (11, 13) else 0.5 if (ix, iy) == (13, 13) else 1.0
        materials.append([word(0.5), word(0.5), word(protection)])
for iy in range(SIDE - 1):
    for ix in range(SIDE - 1):
        a = iy * SIDE + ix
        b, c, d = a + 1, a + SIDE, a + SIDE + 1
        faces.extend(([a, b, d, TRIANGLE], [a, d, c, TRIANGLE]))

identity = [word(x) for x in (1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)]
words = [3, 1, 0, 1, 0, 0, word(40), 1, 1]
words += [0, 0, 0, 0, word(1)]
words += [word(0), word(0), word(0), *identity, word(1)]
for values, width in ((vertices, 3), (colors, 3), (materials, 3), (faces, 4), ([], 2), ([], 4)):
    words += rows(values, width)
data = struct.pack("<" + "I" * len(words), *words)
DEST.write_bytes(data)
manifest = {
    "source": "Independent Python-authored SGL v3; no SculptGL or OBrush encoder",
    "license": "CC0",
    "fixture": str(DEST.relative_to(ROOT)),
    "bytes": len(data),
    "sha256": hashlib.sha256(data).hexdigest(),
    "grid_side": SIDE,
    "vertices": len(vertices),
    "triangles": len(faces),
    "protected_rgb_id": 13 * SIDE + 11,
    "half_unprotected_rgb_id": 13 * SIDE + 13,
    "spike_rgb_id": 13 * SIDE + 12,
    "z_formula": "0.018 + 0.005*sin(ix*0.57+iy*0.19) + 0.013*exp(-((x-0.035)^2+(y+0.025)^2)/0.004) + 0.011 at (12,13)",
}
META.write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps(manifest))
