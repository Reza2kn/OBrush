#!/usr/bin/env python3
"""Compare complete smooth Surface Nets geometry with pinned SculptGL output."""

import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cyclic(ids):
    ids = tuple(ids)
    return min(ids[index:] + ids[:index] for index in range(len(ids)))


def cell(point):
    # A paired Surface Nets cell has one vertex. The allowance handles f32
    # coordinates that round just below an integer after output alignment.
    return tuple(math.floor(value + 1e-5) for value in point)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference_result", type=Path)
    parser.add_argument("obrush_output", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--resolution", required=True, type=int)
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--world-tolerance", type=float, default=2e-7)
    args = parser.parse_args()

    result = json.loads(args.reference_result.read_text())
    case = next(item for item in result["cases"] if item["name"] == "reference-remesh-block-false")
    assert case["resolution"] == args.resolution
    snapshot = Path(case["after"]["path"])
    scene = json.loads(snapshot.read_text())
    mesh = next(item for item in scene["meshes"] if item["id"] == scene["active_id"])
    own = json.loads(args.obrush_output.read_text())
    matrix = mesh["matrix"]
    scale = matrix[0]
    assert scale > 0.0 and matrix[5] == scale and matrix[10] == scale
    assert all(matrix[i] == 0.0 for i in (1, 2, 4, 6, 8, 9))
    translation = matrix[12:15]
    reference_local = [
        tuple(mesh["positions"][index : index + 3])
        for index in range(0, len(mesh["positions"]), 3)
    ]
    own_world = [tuple(point) for point in own["positions"]]
    reference_keys = [cell(point) for point in reference_local]
    own_local = [
        tuple((point[axis] - translation[axis]) / scale for axis in range(3))
        for point in own_world
    ]
    own_keys = [cell(point) for point in own_local]
    reference_ids = {key: index for index, key in enumerate(reference_keys)}
    reference_counts = Counter(reference_keys)
    own_counts = Counter(own_keys)
    mapped = [reference_ids.get(key) for key in own_keys]
    errors = [
        math.dist(own_world[index], tuple(
            translation[axis] + scale * reference_local[mapped[index]][axis]
            for axis in range(3)
        ))
        for index in range(len(mapped))
        if mapped[index] is not None
    ]
    reference_faces = mesh["faces"]
    assert len(reference_faces) % 4 == 0
    reference_triangles = Counter()
    for index in range(0, len(reference_faces), 4):
        a, b, c, d = reference_faces[index : index + 4]
        reference_triangles[cyclic((a, b, c))] += 1
        reference_triangles[cyclic((a, c, d))] += 1
    own_triangles = Counter()
    own_quads = own.get("surface", {}).get("faces")
    reference_quads = Counter(
        cyclic(reference_faces[index : index + 4])
        for index in range(0, len(reference_faces), 4)
    )
    mapped_quads = Counter()
    if all(index is not None for index in mapped):
        own_triangles = Counter(
            cyclic(mapped[index] for index in triangle)
            for triangle in own["triangles"]
        )
        if own_quads is not None:
            mapped_quads = Counter(
                cyclic(mapped[index] for index in quad) for quad in own_quads
            )
    report = {
        "fixture": args.fixture,
        "resolution": args.resolution,
        "world_tolerance": args.world_tolerance,
        "reference_bundle_sha256": "79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600",
        "reference_snapshot": str(snapshot),
        "reference_snapshot_sha256": digest(snapshot),
        "obrush_output": str(args.obrush_output),
        "obrush_output_sha256": digest(args.obrush_output),
        "reference_vertices": len(reference_local),
        "obrush_vertices": len(own_world),
        "reference_quads": len(reference_faces) // 4,
        "reference_triangles_after_split": sum(reference_triangles.values()),
        "obrush_triangles": len(own["triangles"]),
        "duplicate_reference_cell_keys": len(reference_keys) - len(reference_ids),
        "unmatched_reference_cells": sum((reference_counts - own_counts).values()),
        "unmatched_obrush_cells": sum((own_counts - reference_counts).values()),
        "world_position_max_error": max(errors, default=None),
        "world_position_rms_error": math.sqrt(sum(error * error for error in errors) / len(errors)) if errors else None,
        "unmatched_reference_oriented_triangles": sum((reference_triangles - own_triangles).values()),
        "unmatched_obrush_oriented_triangles": sum((own_triangles - reference_triangles).values()),
        "obrush_quads": len(own_quads) if own_quads is not None else None,
        "unmatched_reference_oriented_quads": (
            sum((reference_quads - mapped_quads).values()) if own_quads is not None else None
        ),
        "unmatched_obrush_oriented_quads": (
            sum((mapped_quads - reference_quads).values()) if own_quads is not None else None
        ),
    }
    report["within_tolerance_and_same_topology"] = (
        report["duplicate_reference_cell_keys"] == 0
        and report["unmatched_reference_cells"] == 0
        and report["unmatched_obrush_cells"] == 0
        and report["unmatched_reference_oriented_triangles"] == 0
        and report["unmatched_obrush_oriented_triangles"] == 0
        and (own_quads is None or (
            report["unmatched_reference_oriented_quads"] == 0
            and report["unmatched_obrush_oriented_quads"] == 0
        ))
        and report["world_position_max_error"] is not None
        and report["world_position_max_error"] <= args.world_tolerance
    )
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    if not report["within_tolerance_and_same_topology"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
