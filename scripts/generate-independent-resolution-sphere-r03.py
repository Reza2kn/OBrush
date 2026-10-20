#!/usr/bin/env python3
"""Generate the independently authored, closed UV sphere for R03.

The fixture contains only positions and outward-facing triangular faces. Its
regular sampling makes a short brush stroke affect many vertices while keeping
the OBJ small enough for repeated browser imports.
"""

from math import cos, pi, sin
from pathlib import Path


LATITUDES = 96
LONGITUDES = 192
OUTPUT = Path("crates/obrush-core/tests/fixtures/independent-resolution-dense-r03.obj")


def vertex(theta: float, phi: float) -> tuple[float, float, float]:
    return sin(theta) * cos(phi), cos(theta), sin(theta) * sin(phi)


vertices: list[tuple[float, float, float]] = [(0.0, 1.0, 0.0)]
for latitude in range(1, LATITUDES):
    theta = pi * latitude / LATITUDES
    for longitude in range(LONGITUDES):
        vertices.append(vertex(theta, 2 * pi * longitude / LONGITUDES))
vertices.append((0.0, -1.0, 0.0))


def ring(latitude: int, longitude: int) -> int:
    # OBJ indices are one-based. latitude 1 is the first ring below north.
    return 2 + (latitude - 1) * LONGITUDES + longitude % LONGITUDES


faces: list[tuple[int, int, int]] = []
for longitude in range(LONGITUDES):
    faces.append((1, ring(1, longitude), ring(1, longitude + 1)))
for latitude in range(1, LATITUDES - 1):
    for longitude in range(LONGITUDES):
        a = ring(latitude, longitude)
        b = ring(latitude + 1, longitude)
        c = ring(latitude + 1, longitude + 1)
        d = ring(latitude, longitude + 1)
        faces.extend(((a, b, d), (d, b, c)))
south = len(vertices)
for longitude in range(LONGITUDES):
    faces.append((ring(LATITUDES - 1, longitude), south, ring(LATITUDES - 1, longitude + 1)))
faces = [(a, c, b) for a, b, c in faces]

assert len(vertices) == 18_242
assert len(faces) == 36_480

# A positive signed volume is an independent winding sanity check.
volume6 = 0.0
for a, b, c in faces:
    ax, ay, az = vertices[a - 1]
    bx, by, bz = vertices[b - 1]
    cx, cy, cz = vertices[c - 1]
    volume6 += ax * (by * cz - bz * cy)
    volume6 += ay * (bz * cx - bx * cz)
    volume6 += az * (bx * cy - by * cx)
assert volume6 > 24.0, volume6

lines = ["# Independently generated closed sphere for the R03 resolution stroke gate", "o R03DenseSphere"]
lines.extend(f"v {x:.9f} {y:.9f} {z:.9f}" for x, y, z in vertices)
lines.extend(f"f {a} {b} {c}" for a, b, c in faces)
OUTPUT.write_text("\n".join(lines) + "\n", encoding="ascii")
print(f"{OUTPUT}: {len(vertices)} vertices, {len(faces)} triangles, signed volume {volume6 / 6:.6f}")
