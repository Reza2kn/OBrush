#!/usr/bin/env python3
"""Independently author a two-layer, opposite-wound SGL v3 Brush Culling fixture."""
from pathlib import Path
import hashlib
import json
import struct

GRID = 21
WIDTH = 0.5
FRONT_Z = 0.02
BACK_Z = 0.0
ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / 'tests/fixtures/independent-opposed-sheets-c279.sgl'
META = ROOT / 'tests/fixtures/independent-opposed-sheets-c279.json'
U = 0xFFFFFFFF


def word(value):
    return struct.unpack('<I', struct.pack('<f', float(value)))[0]


def rows(flat, width):
    if any(len(row) != width for row in flat):
        raise ValueError('wrong row width')
    return [len(flat), *(part for row in flat for part in row)]


vertices = []
colors = []
materials = []
faces = []
for layer, z in enumerate((FRONT_Z, BACK_Z)):
    base = len(vertices)
    for iy in range(GRID):
        y = (iy / (GRID - 1) - 0.5) * WIDTH
        for ix in range(GRID):
            x = (ix / (GRID - 1) - 0.5) * WIDTH
            vertices.append([word(x), word(y), word(z)])
            identity = len(vertices) - 1
            colors.append([word(((identity >> shift) & 255) / 255) for shift in (0, 8, 16)])
            materials.append([word(0.5), word(0.5), word(1)])
    for iy in range(GRID - 1):
        for ix in range(GRID - 1):
            a = base + iy * GRID + ix
            b = a + 1
            c = a + GRID
            d = c + 1
            if layer == 0:  # front normal +Z; source eye direction -Z
                faces.extend(([a, b, d, U], [a, d, c, U]))
            else:  # back normal -Z, identical XY coverage
                faces.extend(([a, d, b, U], [a, c, d, U]))

identity = [word(x) for x in (1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)]
words = [3, 1, 0, 1, 0, 0, word(40), 1, 1]  # SGL v3 header, one object
words += [0, 0, 0, 0, word(1)]  # PBR render, opaque
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
    'front_vertex_ids': [0, GRID * GRID - 1],
    'back_vertex_ids': [GRID * GRID, len(vertices) - 1],
    'front_z': FRONT_Z,
    'back_z': BACK_Z,
    'front_normal': [0, 0, 1],
    'back_normal': [0, 0, -1],
}
META.write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(manifest))
