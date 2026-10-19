"""Independently author three separated closed SGL v3 boxes."""
from hashlib import sha256
from pathlib import Path
import json
import struct

HERE = Path(__file__).parent
u32 = lambda x: struct.pack("<I", x)
f32 = lambda x: struct.pack("<f", x)
IDENTITY = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]


def box(low, high, near, far, color):
    positions = [
        [low, low, near], [high, low, near],
        [high, high, near], [low, high, near],
        [low, low, far], [high, low, far],
        [high, high, far], [low, high, far],
    ]
    faces = [
        [0, 1, 2, 3], [4, 7, 6, 5],
        [4, 5, 1, 0], [3, 2, 6, 7],
        [4, 0, 3, 7], [1, 5, 6, 2],
    ]
    return {
        "positions": positions,
        "colors": [color] * 8,
        "materials": [[0.625, 0.0, 1.0]] * 8,
        "faces": faces,
        "uv": [],
        "face_uv": [],
    }


left = box(0, 1, 1, 0, [0.875, 0.125, 0.125])
center = box(0, 1, 1, 0, [0.125, 0.75, 0.25])
right = box(0, 1, 1, 0, [0.2, 0.35, 0.9])
for offset, obj in [(-2, left), (0, center), (2, right)]:
    for vertex in obj["positions"]:
        vertex[0] += offset
data = bytearray()
data += b"".join(map(u32, [3, 0, 0, 0, 1, 0]))
data += f32(45) + u32(0) + u32(3)
for obj in [left, center, right]:
    data += b"".join(map(u32, [0, 3, 0, 0])) + f32(1)
    data += b"".join(map(f32, [0, 0, 0] + IDENTITY + [1]))
    for key in ["positions", "colors", "materials", "faces", "uv", "face_uv"]:
        rows = obj[key]
        data += u32(len(rows))
        encode = u32 if key in {"faces", "face_uv"} else f32
        data += b"".join(encode(value) for row in rows for value in row)

fixture = HERE / "independent-three-closed-boxes.sgl"
fixture.write_bytes(data)
manifest = {
    "fixture": fixture.name,
    "bytes": len(data),
    "sha256": sha256(data).hexdigest(),
    "source": "independent binary SGL v3 writer; no copied reference asset",
    "left": left,
    "center": center,
    "right": right,
    "expected": "last imported right box selected; physical Shift-click membership sequence right+left+center-left leaves right+center",
}
(HERE / "fixture.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps({"bytes": len(data), "sha256": manifest["sha256"]}))
