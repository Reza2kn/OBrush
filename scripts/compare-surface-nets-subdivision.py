#!/usr/bin/env python3
"""Compare a paired closed Surface Nets subdivision using full oriented faces."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rotations(face):
    values = tuple(face)
    return min(values[index:] + values[:index] for index in range(len(values)))


def points(flat):
    return [tuple(flat[index:index + 3]) for index in range(0, len(flat), 3)]


def mapped_faces(flat, vertex_map=None):
    faces = flat if flat and isinstance(flat[0], list) else [
        flat[index:index + 4] for index in range(0, len(flat), 4)
    ]
    if vertex_map is None:
        return Counter(rotations(face) for face in faces)
    return Counter(rotations(vertex_map[index] for index in face) for face in faces)


def nearest_unique(reference, own, cell_size):
    bins = defaultdict(list)
    for index, point in enumerate(reference):
        bins[tuple(math.floor(value / cell_size) for value in point)].append(index)
    mapped = []
    errors = []
    missing = 0
    for point in own:
        key = tuple(math.floor(value / cell_size) for value in point)
        candidates = [
            index
            for dx in (-1, 0, 1)
            for dy in (-1, 0, 1)
            for dz in (-1, 0, 1)
            for index in bins.get((key[0] + dx, key[1] + dy, key[2] + dz), ())
        ]
        if not candidates:
            mapped.append(None)
            missing += 1
            continue
        index = min(candidates, key=lambda item: math.dist(point, reference[item]))
        mapped.append(index)
        errors.append(math.dist(point, reference[index]))
    return mapped, errors, missing


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference_snapshot", type=Path)
    parser.add_argument("obrush_project", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--world-tolerance", type=float, default=2e-6)
    args = parser.parse_args()
    scene = json.loads(args.reference_snapshot.read_text())
    reference_mesh = next(mesh for mesh in scene["meshes"] if mesh["id"] == scene["active_id"])
    own_mesh = json.loads(args.obrush_project.read_text())["mesh"]
    transform = reference_mesh["matrix"]
    scale = transform[0]
    assert scale > 0 and transform[5] == scale and transform[10] == scale
    assert all(transform[index] == 0 for index in (1, 2, 4, 6, 8, 9))
    translation = transform[12:15]
    reference = [
        tuple(translation[axis] + scale * point[axis] for axis in range(3))
        for point in points(reference_mesh["positions"])
    ]
    own = [tuple(point) for point in own_mesh["positions"]]
    assert own_mesh["surface"] is not None
    assert len(reference_mesh["faces"]) % 4 == 0
    assert all(
        len(set(face)) == 4 and all(isinstance(index, int) and 0 <= index < len(reference) for index in face)
        for face in (
            reference_mesh["faces"][start:start + 4]
            for start in range(0, len(reference_mesh["faces"]), 4)
        )
    )
    assert len(own_mesh["triangles"]) == 2 * len(own_mesh["surface"]["faces"])
    assert all(
        len(set(face)) == 4
        and all(isinstance(index, int) and 0 <= index < len(own) for index in face)
        and own_mesh["triangles"][2 * start:2 * start + 2]
        == [[face[0], face[1], face[2]], [face[0], face[2], face[3]]]
        for start, face in enumerate(own_mesh["surface"]["faces"])
    )
    extent = max(max(point[axis] for point in reference) - min(point[axis] for point in reference) for axis in range(3))
    cell_size = extent / 20
    mapped, errors, missing = nearest_unique(reference, own, cell_size)
    ref_faces = mapped_faces(reference_mesh["faces"])
    own_faces = (
        mapped_faces(own_mesh["surface"]["faces"], mapped)
        if missing == 0 else Counter()
    )
    report = {
        "reference_snapshot": str(args.reference_snapshot),
        "reference_snapshot_sha256": digest(args.reference_snapshot),
        "obrush_project": str(args.obrush_project),
        "obrush_project_sha256": digest(args.obrush_project),
        "world_tolerance": args.world_tolerance,
        "reference_vertices": len(reference),
        "obrush_vertices": len(own),
        "reference_quads": len(reference_mesh["faces"]) // 4,
        "obrush_quads": len(own_mesh["surface"]["faces"]),
        "missing_nearest_candidates": missing,
        "nonunique_nearest_ids": len(mapped) - len(set(mapped)) if missing == 0 else None,
        "max_nearest_world_error": max(errors, default=None),
        "rms_nearest_world_error": math.sqrt(sum(error * error for error in errors) / len(errors)) if errors else None,
        "unmatched_reference_oriented_quads": sum((ref_faces - own_faces).values()),
        "unmatched_obrush_oriented_quads": sum((own_faces - ref_faces).values()),
        "cell_size_for_candidate_search": cell_size,
    }
    report["within_tolerance_and_same_topology"] = (
        len(reference) == len(own)
        and report["reference_quads"] == report["obrush_quads"]
        and missing == 0
        and report["nonunique_nearest_ids"] == 0
        and report["max_nearest_world_error"] is not None
        and report["max_nearest_world_error"] <= args.world_tolerance
        and report["unmatched_reference_oriented_quads"] == 0
        and report["unmatched_obrush_oriented_quads"] == 0
    )
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    if not report["within_tolerance_and_same_topology"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
