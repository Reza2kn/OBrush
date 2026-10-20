#!/usr/bin/env python3
"""Independently author a CC0 noisy curved SGL v3 Smooth/Tangent fixture."""
from pathlib import Path
import hashlib
import json
import math
import struct

GRID = 21
WIDTH = 0.5
ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / 'tests/fixtures/independent-noisy-smooth-c283.sgl'
META = ROOT / 'tests/fixtures/independent-noisy-smooth-c283.json'
U = 0xFFFFFFFF


def word(value):
    return struct.unpack('<I', struct.pack('<f', float(value)))[0]


def rows(values, width):
    if any(len(row) != width for row in values):
        raise ValueError('wrong row width')
    return [len(values), *(part for row in values for part in row)]


vertices = []
colors = []
materials = []
faces = []
for iy in range(GRID):
    y = (iy / (GRID - 1) - 0.5) * WIDTH
    for ix in range(GRID):
        x = (ix / (GRID - 1) - 0.5) * WIDTH
        z = 0.02 + 0.003 * math.sin(ix * 0.9) * math.cos(iy * 0.7)
        if (ix, iy) == (10, 10):
            z += 0.008
        vertices.append([word(x), word(y), word(z)])
        identity = len(vertices) - 1
        colors.append([word(((identity >> shift) & 255) / 255) for shift in (0, 8, 16)])
        unprotected = 0.0 if (ix, iy) == (9, 10) else 0.5 if (ix, iy) == (11, 10) else 1.0
        materials.append([word(0.5), word(0.5), word(unprotected)])
for iy in range(GRID - 1):
    for ix in range(GRID - 1):
        a = iy * GRID + ix
        b = a + 1
        c = a + GRID
        d = c + 1
        faces.extend(([a, b, d, U], [a, d, c, U]))

identity = [word(x) for x in (1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)]
words = [3, 1, 0, 1, 0, 0, word(40), 1, 1]
words += [0, 0, 0, 0, word(1)]
words += [word(0), word(0), word(0), *identity, word(1)]
for values, width in ((vertices, 3), (colors, 3), (materials, 3), (faces, 4), ([], 2), ([], 4)):
    words += rows(values, width)
bytes_out = struct.pack('<' + 'I' * len(words), *words)
DEST.write_bytes(bytes_out)
manifest = {
    'source': 'Independent Python-authored SGL v3; no SculptGL or OBrush encoder',
    'license': 'CC0',
    'fixture': str(DEST.relative_to(ROOT)),
    'bytes': len(bytes_out),
    'sha256': hashlib.sha256(bytes_out).hexdigest(),
    'grid_side': GRID,
    'vertices': len(vertices),
    'triangles': len(faces),
    'protected_rgb_id': 10 * GRID + 9,
    'half_unprotected_rgb_id': 10 * GRID + 11,
    'spike_rgb_id': 10 * GRID + 10,
    'z_formula': '0.02 + 0.003*sin(ix*0.9)*cos(iy*0.7) + center_spike_0.008',
}
META.write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(manifest))
