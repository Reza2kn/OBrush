#!/usr/bin/env python3
"""Audit C261 OBrush pre-stroke Save rays on the frozen orthographic front path."""

import argparse
import hashlib
import json
from pathlib import Path


def front_hit(points, triangles, x, y):
    best = None
    for face, (a, b, c) in enumerate(triangles):
        p, q, r = points[a], points[b], points[c]
        den = (q[1] - r[1]) * (p[0] - r[0]) + (r[0] - q[0]) * (p[1] - r[1])
        if abs(den) < 1e-12:
            continue
        u = ((q[1] - r[1]) * (x - r[0]) + (r[0] - q[0]) * (y - r[1])) / den
        v = ((r[1] - p[1]) * (x - r[0]) + (p[0] - r[0]) * (y - r[1])) / den
        w = 1 - u - v
        if min(u, v, w) < -1e-8:
            continue
        z = u * p[2] + v * q[2] + w * r[2]
        if best is None or z > best["front_z"]:
            best = {"face": face, "front_z": z}
    return best


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("receipts", nargs="+", type=Path)
    args = parser.parse_args()
    for receipt in args.receipts:
        result = json.loads(receipt.read_text())
        doc_path = receipt.with_name("obrush-before.obrush")
        doc = json.loads(doc_path.read_text())
        points, triangles = doc["mesh"]["positions"], doc["mesh"]["triangles"]
        assert len(points) == 4514 and len(triangles) == 9024
        camera = result["camera_match"]["final"]["camera"]
        assert camera["orthographic"] and camera["yaw"] == 0 and camera["pitch"] == 0
        radius = result["normalized_radius_pair"]["obrush"]
        plane_x = result["frozen"]["offset"] * radius
        rows = []
        for x, y, _ in result["frozen"]["local_path"]:
            mirror_x = 2 * plane_x - x
            direct = front_hit(points, triangles, x, y)
            mirror = front_hit(points, triangles, mirror_x, y)
            rows.append({"xy": [x, y], "mirror_xy": [mirror_x, y], "direct": direct, "mirror": mirror})
        assert all(row["direct"] and row["mirror"] and row["direct"]["front_z"] > 0 and row["mirror"]["front_z"] > 0 for row in rows)
        audit = {"scope": "Retrospective ray audit of the normal pre-stroke Save, frozen orthographic front camera and measured offset plane", "receipt_sha256": hashlib.sha256(receipt.read_bytes()).hexdigest(), "obrush_before_sha256": hashlib.sha256(doc_path.read_bytes()).hexdigest(), "plane_x": plane_x, "rows": rows}
        dest = receipt.with_name("pre-stroke-save-ray-audit.json")
        dest.write_text(json.dumps(audit, indent=2) + "\n")
        print(json.dumps({"receipt": str(receipt), "audit": str(dest), "rows": rows}))


if __name__ == "__main__":
    main()
