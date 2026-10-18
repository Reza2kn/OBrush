#!/usr/bin/env python3
"""Compare paired OBrush/reference extraction snapshots without assuming vertex order.

Usage: compare-mask-extract-reference.py reference-result.json obrush-sheet.json
       obrush-cube.json OUTPUT.json
       compare-mask-extract-reference.py reference-result.json obrush-single.json OUTPUT.json
"""
import collections
import json
import math
import pathlib
import sys


def vertices(values):
    if values and isinstance(values[0], (list, dict)):
        return [tuple(v[k] for k in ("x", "y", "z")) if isinstance(v, dict) else tuple(v) for v in values]
    return [tuple(values[i : i + 3]) for i in range(0, len(values), 3)]


def triangles_reference(values):
    result = []
    for i in range(0, len(values), 4):
        a, b, c, d = values[i : i + 4]
        result.append((a, b, c))
        if d != 4294967295:
            result.append((a, c, d))
    return result


def topology(positions, triangles):
    edges = collections.Counter(
        tuple(sorted((face[i], face[(i + 1) % 3])))
        for face in triangles
        for i in range(3)
    )
    areas = []
    volume = 0.0
    for a, b, c in triangles:
        p, q, r = (positions[i] for i in (a, b, c))
        u = [q[i] - p[i] for i in range(3)]
        v = [r[i] - p[i] for i in range(3)]
        cross = (
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        )
        areas.append(math.sqrt(sum(x * x for x in cross)))
        volume += sum(p[i] * cross[i] for i in range(3)) / 6
    boundary = sum(count == 1 for count in edges.values())
    return {
        "vertices": len(positions),
        "triangles": len(triangles),
        "boundary_edges": boundary,
        "nonmanifold_edges": sum(count > 2 for count in edges.values()),
        "collapsed_triangles": sum(area == 0 for area in areas),
        "minimum_double_area": min(areas),
        "signed_volume_if_closed": volume if boundary == 0 else None,
    }


def nearest(a, b):
    distances = [min(math.dist(p, q) for q in b) for p in a]
    return {"max": max(distances), "rms": math.sqrt(sum(d * d for d in distances) / len(distances))}


def compare(reference, own):
    reference_positions = vertices(reference["created"]["positions"])
    own_mesh = own["part"]
    own_positions = vertices(own_mesh["positions"])
    reference_faces = triangles_reference(reference["created"]["faces_array"])
    own_faces = [tuple(face) for face in own_mesh["triangles"]]
    return {
        "fixture": reference["fixture"],
        "thickness": reference["thickness"],
        "reference": topology(reference_positions, reference_faces),
        "obrush": topology(own_positions, own_faces),
        "nearest_reference_to_obrush": nearest(reference_positions, own_positions),
        "nearest_obrush_to_reference": nearest(own_positions, reference_positions),
        "note": "Nearest-point distance ignores face correspondence and is not a parity criterion by itself.",
    }


def main():
    if len(sys.argv) not in (4, 5):
        raise SystemExit(__doc__)
    paths = list(map(pathlib.Path, sys.argv[1:]))
    reference_path, output_path = paths[0], paths[-1]
    reference = json.loads(reference_path.read_text())
    if len(paths) == 3:
        fixture_names = {
            case["fixture"] for case in reference["cases"] if case["name"] == "mask-extract-reference"
        }
        if len(fixture_names) != 1:
            raise SystemExit("Single-input comparison requires exactly one reference fixture")
        own = {fixture_names.pop(): json.loads(paths[1].read_text())}
    else:
        own = {
            "independent-masked-sheet.obj": json.loads(paths[1].read_text()),
            "independent-masked-cube.obj": json.loads(paths[2].read_text()),
        }
    rows = [
        compare(case, next(item for item in own[case["fixture"]]["cases"] if item["thickness"] == case["thickness"]))
        for case in reference["cases"]
        if case["name"] == "mask-extract-reference"
    ]
    output_path.write_text(json.dumps({"reference_result": str(reference_path), "obrush_inputs": [str(path) for path in paths[1:-1]], "cases": rows}, indent=2) + "\n")
    print(json.dumps(rows, indent=2))


if __name__ == "__main__":
    main()
