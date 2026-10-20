#!/usr/bin/env python3
"""Author a CC0 dyadic plane for an independent Pinch picker calibration."""

from pathlib import Path
import hashlib
import json
import struct

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "tests/fixtures/independent-source-pinch-c331.sgl"
META = DEST.with_suffix(".json")
SIDE = 33
TRIANGLE = 0xFFFFFFFF


def fword(value):
    return struct.unpack("<I", struct.pack("<f", float(value)))[0]


def block(rows, width):
    if any(len(row) != width for row in rows):
        raise ValueError("invalid row width")
    return [len(rows), *(part for row in rows for part in row)]


vertices, colors, materials, faces = [], [], [], []
for iy in range(SIDE):
    for ix in range(SIDE):
        # Every coordinate is an exact binary fraction; z and transform are zero/identity.
        vertices.append([fword((ix - 16) / 64), fword((iy - 16) / 64), fword(0)])
        rgb_id = len(vertices) - 1
        colors.append([fword(((rgb_id >> shift) & 255) / 255) for shift in (0, 8, 16)])
        materials.append([fword(0.5), fword(0.5), fword(1.0)])

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
    "position_formula": "x=(ix-16)/64, y=(iy-16)/64, z=0",
    "identity_transform": True,
    "all_material_protection": 1.0,
}
META.write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps(manifest))
