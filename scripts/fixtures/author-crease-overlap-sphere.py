"""Author an independent closed curved sphere for repeated Crease safety checks."""

from hashlib import sha256
from math import cos, pi, sin
from pathlib import Path
import json
import struct

HERE = Path(__file__).parent
RINGS = 96
SECTORS = 192
TRIANGLE = 0xFFFFFFFF
IDENTITY = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]


def u32(value):
    return struct.pack("<I", value)


def f32(value):
    return struct.pack("<f", value)


positions = [[0.0, 1.0, 0.0]]
for ring in range(1, RINGS):
    polar = pi * ring / RINGS
    for sector in range(SECTORS):
        azimuth = 2 * pi * sector / SECTORS
        positions.append([sin(polar) * cos(azimuth), cos(polar), sin(polar) * sin(azimuth)])
positions.append([0.0, -1.0, 0.0])
south = len(positions) - 1


def row(ring, sector):
    return 1 + (ring - 1) * SECTORS + sector % SECTORS


faces = []
for sector in range(SECTORS):
    faces.append([0, row(1, sector + 1), row(1, sector), TRIANGLE])
for ring in range(1, RINGS - 1):
    for sector in range(SECTORS):
        faces.append([row(ring, sector), row(ring, sector + 1), row(ring + 1, sector + 1), row(ring + 1, sector)])
for sector in range(SECTORS):
    faces.append([row(RINGS - 1, sector), row(RINGS - 1, sector + 1), south, TRIANGLE])

data = bytearray()
data += b"".join(map(u32, [3, 0, 0, 0, 1, 0]))
data += f32(45) + u32(0) + u32(1)
data += b"".join(map(u32, [0, 3, 0, 0])) + f32(1)
data += b"".join(map(f32, [0, 0, 0] + IDENTITY + [1]))
for rows, encode in [
    (positions, f32),
    ([[0.72, 0.49, 0.36]] * len(positions), f32),
    ([[0.625, 0.0, 1.0]] * len(positions), f32),
    (faces, u32),
    ([], f32),
    ([], u32),
]:
    data += u32(len(rows))
    data += b"".join(encode(value) for record in rows for value in record)

fixture = HERE / "independent-crease-overlap-sphere.sgl"
fixture.write_bytes(data)
manifest = {
    "fixture": fixture.name,
    "sha256": sha256(data).hexdigest(),
    "bytes": len(data),
    "vertices": len(positions),
    "faces": len(faces),
    "rings": RINGS,
    "sectors": SECTORS,
    "surface": "independent unit UV sphere, closed, triangles at poles and quads elsewhere",
}
(HERE / "crease-overlap-sphere-fixture.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps(manifest))
