#!/usr/bin/env python3
"""Compare an independent OBrush Block remesh with pinned SculptGL snapshots."""

import argparse
import collections
import hashlib
import json
from pathlib import Path


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cyclic_triangle(points):
    points = tuple(points)
    return min(points, points[1:] + points[:1], points[2:] + points[:2])


def reference_world_points(mesh):
    matrix = mesh["matrix"]
    positions = mesh["positions"]
    return [
        tuple(
            sum(matrix[column * 4 + row] * positions[index + column] for column in range(3))
            + matrix[12 + row]
            for row in range(3)
        )
        for index in range(0, len(positions), 3)
    ]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference_result", type=Path)
    parser.add_argument("obrush_output", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--resolution", type=int, default=64)
    parser.add_argument("--fixture", default="independent-filter-cube.obj")
    args = parser.parse_args()

    result = json.loads(args.reference_result.read_text())
    case = next(item for item in result["cases"] if item["name"] == "reference-remesh-block-true")
    assert case["block"] is True and case["resolution"] == args.resolution
    reference_snapshot = Path(case["after"]["path"])
    reference = json.loads(reference_snapshot.read_text())
    mesh = next(mesh for mesh in reference["meshes"] if mesh["id"] == reference["active_id"])
    obrush = json.loads(args.obrush_output.read_text())
    reference_points = reference_world_points(mesh)
    obrush_points = [tuple(point) for point in obrush["positions"]]
    reference_positions = collections.Counter(reference_points)
    obrush_positions = collections.Counter(obrush_points)

    reference_faces = mesh["faces"]
    assert len(reference_faces) % 4 == 0
    reference_triangles = collections.Counter()
    for index in range(0, len(reference_faces), 4):
        a, b, c, d = reference_faces[index : index + 4]
        for triangle in ((a, b, c), (a, c, d)):
            reference_triangles[cyclic_triangle(reference_points[vertex] for vertex in triangle)] += 1
    obrush_triangles = collections.Counter(
        cyclic_triangle(obrush_points[vertex] for vertex in triangle)
        for triangle in obrush["triangles"]
    )
    report = {
        "fixture": args.fixture,
        "reference_bundle_sha256": "79b24bf6e1818cbb468c53cfe49783de419fd3e58f72b220aeb9308932e47600",
        "resolution": args.resolution,
        "reference_snapshot": str(reference_snapshot),
        "reference_snapshot_sha256": sha256(reference_snapshot),
        "obrush_output": str(args.obrush_output),
        "obrush_output_sha256": sha256(args.obrush_output),
        "reference_vertices": len(reference_points),
        "obrush_vertices": len(obrush_points),
        "reference_quads": len(reference_faces) // 4,
        "reference_triangles_after_quad_split": sum(reference_triangles.values()),
        "obrush_triangles": len(obrush["triangles"]),
        "unmatched_reference_points": sum((reference_positions - obrush_positions).values()),
        "unmatched_obrush_points": sum((obrush_positions - reference_positions).values()),
        "unmatched_reference_oriented_triangles": sum(
            (reference_triangles - obrush_triangles).values()
        ),
        "unmatched_obrush_oriented_triangles": sum(
            (obrush_triangles - reference_triangles).values()
        ),
    }
    report["exact_world_geometry"] = all(
        report[key] == 0
        for key in (
            "unmatched_reference_points",
            "unmatched_obrush_points",
            "unmatched_reference_oriented_triangles",
            "unmatched_obrush_oriented_triangles",
        )
    )
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    if not report["exact_world_geometry"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
