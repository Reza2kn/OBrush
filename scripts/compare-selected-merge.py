#!/usr/bin/env python3
"""Compare pinned SculptGL and OBrush selected-merge browser records.

The fixtures deliberately have unique world positions, so attributes can be
matched by position even when SculptGL reorders vertices during MeshStatic.init.
Faces are compared as oriented cycles of world-space points.
"""

import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

TRIANGLE_CORNER = 0xFFFFFFFF


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_mesh(result):
    scene = result["after_merge"]
    index = len(scene["vertices"]) - 1
    flat = scene["vertices"][index]
    positions = [tuple(flat[i : i + 3]) for i in range(0, len(flat), 3)]
    colors = scene["colors"][index]
    materials = scene["materials"][index]
    faces = scene["faces"][index]
    return {
        "positions": positions,
        "colors": [colors[i : i + 3] for i in range(0, len(colors), 3)],
        "materials": [materials[i : i + 3][:2] for i in range(0, len(materials), 3)],
        "faces": [faces[i : i + 4] for i in range(0, len(faces), 4)],
    }


def root_mesh(result):
    mesh = result["after_merge"]["project_data"]["mesh"]
    return {
        "positions": [tuple(point) for point in mesh["positions"]],
        "colors": mesh["colors"],
        "materials": mesh["materials"],
        "faces": mesh["surface"]["faces"],
    }


def face_cycles(mesh):
    cycles = []
    for face in mesh["faces"]:
        points = [mesh["positions"][index] for index in face if index != TRIANGLE_CORNER]
        if len(points) not in (3, 4):
            raise ValueError("Expected an oriented triangle or quad")
        cycles.append(min(tuple(points[i:] + points[:i]) for i in range(len(points))))
    return Counter(cycles)


def compare(source_path, root_path):
    source = json.loads(source_path.read_text())
    root = json.loads(root_path.read_text())
    a, b = source_mesh(source), root_mesh(root)
    if len(set(a["positions"])) != len(a["positions"]) or len(set(b["positions"])) != len(
        b["positions"]
    ):
        raise ValueError("Fixture has duplicate world points; position-keyed comparison is invalid")
    same_positions = set(a["positions"]) == set(b["positions"])
    same_faces = face_cycles(a) == face_cycles(b)
    max_rgb = None
    max_material = None
    if same_positions:
        left = {point: i for i, point in enumerate(a["positions"])}
        right = {point: i for i, point in enumerate(b["positions"])}
        max_rgb = max(
            abs(x - y)
            for point in left
            for x, y in zip(a["colors"][left[point]], b["colors"][right[point]])
        )
        max_material = max(
            abs(x - y)
            for point in left
            for x, y in zip(a["materials"][left[point]], b["materials"][right[point]])
        )
    passed = (
        source["status"].startswith("PASS")
        and root["status"].startswith("PASS")
        and not source["console_errors"]
        and not root["console_errors"]
        and not source["exceptions"]
        and not root["exceptions"]
        and same_positions
        and same_faces
        and max_rgb is not None
        and max_rgb <= 3e-8
        and max_material is not None
        and max_material <= 1e-8
    )
    return {
        "source_record": str(source_path),
        "source_record_sha256": sha256(source_path),
        "obrush_record": str(root_path),
        "obrush_record_sha256": sha256(root_path),
        "vertices": [len(a["positions"]), len(b["positions"])],
        "oriented_faces": [len(a["faces"]), len(b["faces"])],
        "world_position_sets_exact": same_positions,
        "oriented_world_face_multisets_exact": same_faces,
        "max_rgb_component_difference": max_rgb,
        "max_material_component_difference": max_material,
        "tolerances": {"rgb": 3e-8, "material": 1e-8},
        "status": "PASS" if passed else "FAIL",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source_boxes", type=Path)
    parser.add_argument("obrush_boxes", type=Path)
    parser.add_argument("source_affine", type=Path)
    parser.add_argument("obrush_affine", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = {
        "independent_three_boxes": compare(args.source_boxes, args.obrush_boxes),
        "independent_affine_two_meshes": compare(args.source_affine, args.obrush_affine),
        "scope": "Positions, oriented faces, RGB and the two shared material channels; masks, UVs, source provenance, camera and latency are separate gates",
    }
    result["status"] = (
        "PASS" if all(value["status"] == "PASS" for key, value in result.items() if isinstance(value, dict)) else "FAIL"
    )
    encoded = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(encoded)
    else:
        print(encoded, end="")
    if result["status"] != "PASS":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
