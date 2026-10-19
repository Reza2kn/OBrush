"""Author a closed, irregular UV sphere for physical Shift-drag Smooth checks.

This is independent geometry encoded directly as public SGL v3. No commercial
asset or library mesh is used. The front mound gives Smooth measurable work.
"""

from hashlib import sha256
from math import cos, exp, pi, sin
from pathlib import Path
import json
import struct

HERE = Path(__file__).parent
RINGS = 48
SECTORS = 96
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
        x = sin(polar) * cos(azimuth)
        y = cos(polar)
        z = sin(polar) * sin(azimuth)
        mound = 0.26 * exp(-((x - 0.17) ** 2 + (y - 0.14) ** 2) / 0.025)
        radius = 1.0 + mound * max(z, 0)
        positions.append([radius * x, radius * y, radius * z])
positions.append([0.0, -1.0, 0.0])
south = len(positions) - 1


def row(ring, sector):
    return 1 + (ring - 1) * SECTORS + sector % SECTORS


faces = []
for sector in range(SECTORS):
    faces.append([0, row(1, sector + 1), row(1, sector), TRIANGLE])
for ring in range(1, RINGS - 1):
    for sector in range(SECTORS):
        faces.append([row(ring, sector), row(ring, sector + 1),
                      row(ring + 1, sector + 1), row(ring + 1, sector)])
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
    data += b"".join(encode(value) for row in rows for value in row)

fixture = HERE / "independent-dense-shift-smooth.sgl"
fixture.write_bytes(data)
manifest = {
    "fixture": fixture.name,
    "sha256": sha256(data).hexdigest(),
    "bytes": len(data),
    "vertices": len(positions),
    "faces": len(faces),
    "rings": RINGS,
    "sectors": SECTORS,
    "mound": "radial +0.26 exp(-((x-0.17)^2+(y-0.14)^2)/0.025) max(z,0)",
    "source": "independent analytic sphere and binary SGL v3 writer",
}
(HERE / "dense-shift-smooth-fixture.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps(manifest))
