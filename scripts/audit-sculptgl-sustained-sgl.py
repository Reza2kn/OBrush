#!/usr/bin/env python3
"""Independently audit source SculptGL's sustained binary SGL files.

Reads bounded version-3 raw words; does not use OBrush's SGL parser. The
native/browser file hashes and immutable channels are checked before shape
measurements. This does not establish matched-stroke numerical parity.
"""

import hashlib
import json
import sys
from pathlib import Path

import numpy as np

MAX_BYTES = 128 * 1024 * 1024
LIMITS = {"positions": 1_000_000, "colors": 1_000_000,
          "materials": 1_000_000, "faces": 2_000_000,
          "coordinates": 6_000_000, "uvfaces": 2_000_000}
WIDTHS = {"positions": 3, "colors": 3, "materials": 3,
          "faces": 4, "coordinates": 2, "uvfaces": 4}


def digest(path):
    value = hashlib.sha256()
    with open(path, "rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def parse_sgl(path):
    path = Path(path)
    if path.stat().st_size > MAX_BYTES or path.stat().st_size % 4:
        raise ValueError("SGL input exceeds bounded word length")
    words = np.memmap(path, dtype="<u4", mode="r")
    offset = 0

    def take(count):
        nonlocal offset
        if count < 0 or count > words.size - offset:
            raise ValueError("SGL truncated or invalid count")
        rows = words[offset:offset + count]
        offset += count
        return rows

    if int(take(1)[0]) != 3:
        raise ValueError("Expected version-3 little-endian SGL")
    header = take(7)
    objects = int(take(1)[0])
    if objects != 1:
        raise ValueError("This frozen source audit expects one object")
    meta = take(5 + 3 + 16 + 1)
    channels = {}
    for name, width in WIDTHS.items():
        count = int(take(1)[0])
        if count > LIMITS[name]:
            raise ValueError(f"SGL {name} exceeds bound")
        channels[name] = take(count * width).reshape(count, width)
    if offset != words.size:
        raise ValueError("SGL has undeclared trailing words")
    return {"header": header, "meta": meta, **channels}


def main(path):
    result_path = Path(path)
    result = json.loads(result_path.read_bytes())
    if result.get("side") != "source" or not result.get("status", "").startswith("Completed actual sustained"):
        raise ValueError("Expected completed source browser run")
    paths = {key: Path(result[key]["path"]) for key in ("before", "after", "stable", "undo", "redo")}
    checks = {f"{key}_hash": digest(value) == result[key]["sha256"] for key, value in paths.items()}
    checks["stable_exact"] = digest(paths["stable"]) == digest(paths["after"])
    checks["undo_exact"] = digest(paths["undo"]) == digest(paths["before"])
    checks["redo_exact"] = digest(paths["redo"]) == digest(paths["after"])
    before, after = parse_sgl(paths["before"]), parse_sgl(paths["after"])
    checks["single_fixture_vertex_count"] = before["positions"].shape[0] == result["fixture"]["vertices"]
    for key in ("header", "meta", "colors", "materials", "faces", "coordinates", "uvfaces"):
        checks[f"{key}_exact"] = np.array_equal(before[key], after[key])
    old = before["positions"].view("<f4").reshape(-1, 3)
    new = after["positions"].view("<f4").reshape(-1, 3)
    checks["finite_positions"] = bool(np.isfinite(old).all() and np.isfinite(new).all())
    changed = np.any(before["positions"] != after["positions"], axis=1)
    checks["useful_edit"] = bool(changed.any())
    faces = before["faces"]
    if faces.size and (np.any(faces[:, :3] >= old.shape[0]) or np.any((faces[:, 3] != 0xFFFFFFFF) & (faces[:, 3] >= old.shape[0]))):
        raise ValueError("SGL face index exceeds vertex count")
    counts = np.where(faces[:, 3] == 0xFFFFFFFF, 1, 2)
    triangle_count = int(np.sum(counts))
    checks["fixture_triangle_count"] = triangle_count == result["fixture"]["triangles"]
    touched = np.any(changed[faces[:, :3]], axis=1)
    quads = faces[:, 3] != 0xFFFFFFFF
    touched |= quads & changed[faces[:, 3].clip(max=changed.size - 1)]
    touched_faces = faces[touched]
    triangle_rows = [touched_faces[:, :3]]
    touched_quads = touched_faces[touched_faces[:, 3] != 0xFFFFFFFF]
    if touched_quads.size:
        triangle_rows.append(touched_quads[:, [0, 2, 3]])
    triangles = np.concatenate(triangle_rows).astype(np.int64, copy=False)
    old_tri = old[triangles].astype(np.float64)
    new_tri = new[triangles].astype(np.float64)
    old_cross = np.cross(old_tri[:, 1] - old_tri[:, 0], old_tri[:, 2] - old_tri[:, 0])
    new_cross = np.cross(new_tri[:, 1] - new_tri[:, 0], new_tri[:, 2] - new_tri[:, 0])
    old_area = np.linalg.norm(old_cross, axis=1) * 0.5
    new_area = np.linalg.norm(new_cross, axis=1) * 0.5
    reversals = int(np.count_nonzero(np.einsum("ij,ij->i", old_cross, new_cross) <= 0))
    degenerates = int(np.count_nonzero((old_area > 1e-12) & (new_area <= 1e-12)))
    checks["no_new_degenerate_changed_faces"] = degenerates == 0
    checks["no_changed_face_orientation_reversal"] = reversals == 0
    delta = new[changed].astype(np.float64) - old[changed].astype(np.float64)
    displacement = np.linalg.norm(delta, axis=1)
    output = {
        "result_sha256": digest(result_path),
        "checks": checks,
        "failed_checks": [key for key, passed in checks.items() if not passed],
        "counts": {"vertices": int(old.shape[0]), "triangles": triangle_count,
                   "changed_vertices": int(np.count_nonzero(changed)),
                   "changed_triangles": int(triangles.shape[0])},
        "shape": {"maximum_displacement": float(displacement.max()) if displacement.size else 0.0,
                  "rms_displacement_on_changed": float(np.sqrt(np.mean(displacement ** 2))) if displacement.size else 0.0,
                  "new_degenerate": degenerates, "orientation_reversals": reversals},
    }
    print(json.dumps(output, indent=2, allow_nan=False))
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: audit-sculptgl-sustained-sgl.py RESULT.json")
    raise SystemExit(main(sys.argv[1]))
