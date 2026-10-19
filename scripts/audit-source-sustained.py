#!/usr/bin/env python3
"""Independently audit one sustained Source browser result's public project files.

Usage: python3 scripts/audit-source-sustained.py PATH/TO/result.json
The browser's diagnostic counters are never used as a geometry oracle.
"""

import gc
import hashlib
import json
import sys
from pathlib import Path

import numpy as np

FROZEN_COHORT_COUNTS = {
    "10k": (10114, 20224),
    "100k": (99602, 199200),
    "1m": (999002, 1998000),
}


def digest(path):
    value = hashlib.sha256()
    with open(path, "rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def load_document(path):
    with open(path, "rb") as source:
        return json.load(source)


def main(result_path):
    result = load_document(result_path)
    fixture = result["fixture"]
    expected = FROZEN_COHORT_COUNTS.get(fixture["label"])
    paths = {name: Path(result[name]["path"]) for name in ("before", "after", "stable", "undo", "redo")}
    checks = {
        name + "_file_sha256": digest(path) == result[name]["sha256"]
        for name, path in paths.items()
    }
    checks["fixture_contract_exact"] = bool(
        expected
        and (fixture["vertices"], fixture["triangles"]) == expected
        and digest(fixture["path"]) == fixture["sha256"]
    )
    checks["stable_exact"] = checks["after_file_sha256"] and digest(paths["stable"]) == digest(paths["after"])
    checks["undo_exact"] = checks["before_file_sha256"] and digest(paths["undo"]) == digest(paths["before"])
    checks["redo_exact"] = checks["after_file_sha256"] and digest(paths["redo"]) == digest(paths["after"])

    before = load_document(paths["before"])
    initial = before.pop("mesh")
    old_positions = np.asarray(initial.pop("positions"), dtype=np.float32)
    old_normals = np.asarray(initial.pop("normals"), dtype=np.float32)
    faces = np.asarray(initial["triangles"], dtype=np.int64)
    del initial["triangles"]
    gc.collect()

    after = load_document(paths["after"])
    final = after.pop("mesh")
    new_positions = np.asarray(final.pop("positions"), dtype=np.float32)
    new_normals = np.asarray(final.pop("normals"), dtype=np.float32)
    new_faces = np.asarray(final.pop("triangles"), dtype=np.int64)
    checks["document_metadata_exact"] = before == after
    checks["mesh_nonposition_channels_exact"] = initial == final
    checks["triangles_exact"] = np.array_equal(faces, new_faces)
    checks["declared_vertex_and_triangle_counts"] = (
        expected is not None
        and old_positions.shape == new_positions.shape == (expected[0], 3)
        and faces.shape == new_faces.shape == (expected[1], 3)
        and old_normals.shape == new_normals.shape == (expected[0], 3)
    )
    checks["finite_positions_and_normals"] = bool(
        np.isfinite(new_positions).all() and np.isfinite(new_normals).all()
    )
    changed = np.any(old_positions.view(np.uint32) != new_positions.view(np.uint32), axis=1)
    changed_ids = np.flatnonzero(changed)
    delta = new_positions[changed].astype(np.float64) - old_positions[changed].astype(np.float64)
    displacement = np.linalg.norm(delta, axis=1)
    checks["useful_edit"] = bool(changed_ids.size)
    touched_faces = np.any(changed[faces], axis=1)
    ids = faces[touched_faces]
    old_tri = old_positions[ids].astype(np.float64)
    new_tri = new_positions[ids].astype(np.float64)
    old_cross = np.cross(old_tri[:, 1] - old_tri[:, 0], old_tri[:, 2] - old_tri[:, 0])
    new_cross = np.cross(new_tri[:, 1] - new_tri[:, 0], new_tri[:, 2] - new_tri[:, 0])
    old_area = np.linalg.norm(old_cross, axis=1) * 0.5
    new_area = np.linalg.norm(new_cross, axis=1) * 0.5
    reversals = int(np.count_nonzero(np.einsum("ij,ij->i", old_cross, new_cross) <= 0))
    degenerates = int(np.count_nonzero((old_area > 1e-12) & (new_area <= 1e-12)))
    checks["no_new_degenerate_changed_faces"] = degenerates == 0
    checks["no_changed_face_orientation_reversal"] = reversals == 0
    stretch = 0.0
    for a, b in ((0, 1), (1, 2), (2, 0)):
        old_len = np.linalg.norm(old_tri[:, a] - old_tri[:, b], axis=1)
        new_len = np.linalg.norm(new_tri[:, a] - new_tri[:, b], axis=1)
        if old_len.size:
            stretch = max(stretch, float(np.max(new_len / np.maximum(old_len, 1e-20))))
    output = {
        "result_sha256": digest(result_path),
        "checks": checks,
        "failed_checks": [name for name, passed in checks.items() if not passed],
        "counts": {"vertices": int(old_positions.shape[0]), "triangles": int(faces.shape[0]),
                   "changed_vertices": int(changed_ids.size), "changed_faces": int(ids.shape[0])},
        "shape": {"min_changed_face_area_before": float(np.min(old_area)) if old_area.size else None,
                  "min_changed_face_area_after": float(np.min(new_area)) if new_area.size else None,
                  "maximum_changed_edge_stretch": stretch,
                  "maximum_displacement": float(np.max(displacement)) if displacement.size else 0.0,
                  "rms_displacement_on_changed": float(np.sqrt(np.mean(displacement ** 2))) if displacement.size else 0.0,
                  "new_degenerate": degenerates, "orientation_reversals": reversals},
    }
    print(json.dumps(output, indent=2, allow_nan=False))
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: audit-source-sustained.py PATH/TO/result.json")
    raise SystemExit(main(Path(sys.argv[1])))
