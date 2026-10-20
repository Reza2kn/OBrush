#!/usr/bin/env python3
"""Read-only C326 public-project footprint audit; writes one diagnostic receipt."""

import hashlib
import json
import math
from pathlib import Path
import sys


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def project(point: list[float], matrix: list[float], viewport: list[float]) -> list[float]:
    x, y, z = point
    cx = matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12]
    cy = matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13]
    cw = matrix[3] * x + matrix[7] * y + matrix[11] * z + matrix[15]
    if not math.isfinite(cw) or cw <= 0:
        raise ValueError(f"invalid projected w={cw}")
    return [
        viewport[0] + (cx / cw + 1) * (viewport[2] - viewport[0]) / 2,
        viewport[1] + (1 - cy / cw) * (viewport[3] - viewport[1]) / 2,
    ]


def main() -> int:
    source = Path(sys.argv[1]).resolve()
    raw = source.read_bytes()
    browser = json.loads(raw)
    directory = source.parent
    ring_path = directory / "cursor-radius-result.json"
    ring_raw = ring_path.read_bytes()
    ring = json.loads(ring_raw)
    saves = {row["save"]["name"]: row["save"] for row in browser["cases"] if "save" in row}
    cases = {}
    for pointer_type in ("mouse", "pen"):
        before = saves[f"{pointer_type}-before"]
        after = saves[f"{pointer_type}-after"]
        before_raw = (directory / "downloads" / before["guid"]).read_bytes()
        after_raw = (directory / "downloads" / after["guid"]).read_bytes()
        if digest(before_raw) != before["sha256"] or digest(after_raw) != after["sha256"]:
            raise ValueError(f"{pointer_type}: saved project digest mismatch")
        before_doc = json.loads(before_raw)
        after_doc = json.loads(after_raw)
        hover = next(
            row["state"]
            for row in browser["cases"]
            if row.get("name") == f"{pointer_type}-contact-hover"
        )
        matrix = hover["camera"]["view_projection"]
        viewport = hover["camera"]["viewport"]
        positions_before = before_doc["mesh"]["positions"]
        positions_after = after_doc["mesh"]["positions"]
        if len(positions_before) != len(positions_after):
            raise ValueError(f"{pointer_type}: topology changed")
        radial_distances = []
        for old, new in zip(positions_before, positions_after):
            if math.dist(old, new) <= 1e-12:
                continue
            midpoint = [(a + b) / 2 for a, b in zip(old, new)]
            css = project(midpoint, matrix, viewport)
            radial_distances.append(math.dist(css, [800, 500]))
        if not radial_distances:
            raise ValueError(f"{pointer_type}: no changed vertices")
        ring_radius = min(row["annulus_median_css"] for row in ring["cases"].values())
        limit = ring_radius + 2
        cases[pointer_type] = {
            "changed_vertices": len(radial_distances),
            "maximum_changed_radius_css": max(radial_distances),
            "minimum_ring_radius_css": ring_radius,
            "limit_css": limit,
            "outside_count": sum(value > limit for value in radial_distances),
            "other_objects_equal": (
                [obj for obj in before_doc["scene"]["objects"] if obj["id"] != 1]
                == [obj for obj in after_doc["scene"]["objects"] if obj["id"] != 1]
            ),
        }
    receipt = {
        "browser_result_sha256": digest(raw),
        "ring_receipt_sha256": digest(ring_raw),
        "dpr": browser["dpr"],
        "cases": cases,
        "status": "passed" if all(
            row["outside_count"] == 0 and row["other_objects_equal"] for row in cases.values()
        ) else "failed",
    }
    out = directory / "contact-footprint-result.json"
    out.write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({"status": receipt["status"], "result": str(out)}))
    return 0 if receipt["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
