#!/usr/bin/env python3
"""Render the simple OBrush SVG mark without external Python dependencies.

The canonical SVG has one rounded square and two concentric circles. Keep the
renderer deliberately limited to those primitives so generated assets are
repeatable and cannot silently reinterpret a changed illustration.
"""

import argparse
import shutil
import struct
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "assets/brand"
SOURCE = BRAND / "obrush-icon.svg"
SIZES = (16, 32, 64, 128, 180, 256, 512, 1024)


def color(value: str) -> tuple[int, int, int]:
    if len(value) != 7 or not value.startswith("#"):
        raise ValueError(f"expected an opaque #RRGGBB fill, got {value!r}")
    return tuple(int(value[index : index + 2], 16) for index in (1, 3, 5))


def geometry() -> tuple[tuple[float, ...], tuple[float, ...], tuple[float, ...]]:
    root = ET.parse(SOURCE).getroot()
    if root.attrib.get("viewBox") != "0 0 1024 1024":
        raise ValueError("icon source must have the 1024-square viewBox")
    shapes = list(root)
    if [shape.tag.rsplit("}", 1)[-1] for shape in shapes] != ["rect", "circle", "circle"]:
        raise ValueError("icon source must contain one rect and two circles")
    rect, outer, inner = shapes
    background = (
        *(float(rect.attrib[key]) for key in ("x", "y", "width", "height", "rx")),
        *color(rect.attrib["fill"]),
    )
    circles = []
    for shape in (outer, inner):
        circles.append(
            (
                *(float(shape.attrib[key]) for key in ("cx", "cy", "r")),
                *color(shape.attrib["fill"]),
            )
        )
    return background, circles[0], circles[1]


def in_round_rect(x: float, y: float, rect: tuple[float, ...]) -> bool:
    left, top, width, height, radius = rect[:5]
    center_x = min(max(x, left + radius), left + width - radius)
    center_y = min(max(y, top + radius), top + height - radius)
    return (x - center_x) ** 2 + (y - center_y) ** 2 <= radius**2


def render(size: int, shapes: tuple[tuple[float, ...], ...]) -> bytes:
    background, outer, inner = shapes
    # Four samples per axis give smooth edges at dock, touch and favicon sizes.
    samples = 4
    row_bytes = bytearray()
    for py in range(size):
        row_bytes.append(0)  # PNG filter: None
        for px in range(size):
            red = green = blue = alpha = 0
            for sy in range(samples):
                y = (py + (sy + 0.5) / samples) * 1024 / size
                for sx in range(samples):
                    x = (px + (sx + 0.5) / samples) * 1024 / size
                    if not in_round_rect(x, y, background):
                        continue
                    rgb = background[5:]
                    if (x - outer[0]) ** 2 + (y - outer[1]) ** 2 <= outer[2] ** 2:
                        rgb = outer[3:]
                    if (x - inner[0]) ** 2 + (y - inner[1]) ** 2 <= inner[2] ** 2:
                        rgb = inner[3:]
                    red += int(rgb[0])
                    green += int(rgb[1])
                    blue += int(rgb[2])
                    alpha += 1
            if alpha:
                row_bytes.extend(
                    (
                        round(red / alpha),
                        round(green / alpha),
                        round(blue / alpha),
                        round(alpha * 255 / (samples * samples)),
                    )
                )
            else:
                row_bytes.extend((0, 0, 0, 0))

    def chunk(kind: bytes, payload: bytes) -> bytes:
        return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload))

    return b"".join(
        (
            b"\x89PNG\r\n\x1a\n",
            chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)),
            chunk(b"IDAT", zlib.compress(bytes(row_bytes), 9)),
            chunk(b"IEND", b""),
        )
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare committed PNG derivatives without writing")
    args = parser.parse_args()
    shapes = geometry()
    png = {size: render(size, shapes) for size in SIZES}
    names = {32: "favicon-32.png", 180: "apple-touch-icon.png", 256: "obrush-icon-256.png"}
    for size, name in names.items():
        target = BRAND / name
        if args.check:
            if not target.exists() or target.read_bytes() != png[size]:
                raise SystemExit(f"stale generated icon: {target}")
        else:
            target.write_bytes(png[size])
            print(target.relative_to(ROOT))
    if args.check:
        return
    if not shutil.which("iconutil"):
        raise SystemExit("iconutil is required to generate the macOS .icns file")
    with tempfile.TemporaryDirectory(prefix="obrush-icon-") as directory:
        iconset = Path(directory) / "OBRush.iconset"
        iconset.mkdir()
        for size in (16, 32, 64, 128, 256, 512):
            (iconset / f"icon_{size}x{size}.png").write_bytes(png[size])
            (iconset / f"icon_{size}x{size}@2x.png").write_bytes(png[size * 2])
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(BRAND / "OBRush.icns")], check=True)
    print((BRAND / "OBRush.icns").relative_to(ROOT))


if __name__ == "__main__":
    main()
