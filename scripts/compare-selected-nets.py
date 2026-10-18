#!/usr/bin/env python3
"""Pair selected-mesh Surface Nets/Block outputs with a pinned SculptGL capture."""
import hashlib
import json
import math
import sys
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cyclic(face):
    face = tuple(face)
    return min(face[index:] + face[:index] for index in range(4))


def main():
    reference_path, obrush_path, output_path = map(Path, sys.argv[1:4])
    snapshot = json.loads(reference_path.read_text())
    reference = next(mesh for mesh in snapshot["meshes"] if mesh["id"] == snapshot["active_id"])
    local_data = json.loads(obrush_path.read_text())
    if obrush_path.suffix == ".obrush":
        mesh = local_data["mesh"]
        local = {**mesh, "faces": mesh["surface"]["faces"]}
    else:
        local = local_data[0]
    matrix = reference["matrix"]
    local_points = [tuple(point) for point in local["positions"]]
    reference_points = [
        tuple(sum(matrix[axis + 4 * column] * reference["positions"][index + column]
                  for column in range(3)) + matrix[axis + 12] for axis in range(3))
        for index in range(0, len(reference["positions"]), 3)
    ]
    if len(reference_points) != len(local_points):
        raise ValueError("Vertex counts differ")
    mapping = []
    distances = []
    for point in reference_points:
        squares = [sum((point[axis] - other[axis]) ** 2 for axis in range(3))
                   for other in local_points]
        index = min(range(len(squares)), key=squares.__getitem__)
        mapping.append(index)
        distances.append(math.sqrt(squares[index]))
    if len(set(mapping)) != len(mapping):
        raise ValueError("Nearest vertex mapping is not bijective")
    reference_faces = [tuple(reference["faces"][index:index + 4])
                       for index in range(0, len(reference["faces"]), 4)]
    local_faces = local["faces"]
    mapped_reference_faces = {cyclic(tuple(mapping[index] for index in face))
                              for face in reference_faces}
    actual_faces = {cyclic(face) for face in local_faces}
    def channel_error(ref, own, components):
        errors = [abs(ref[i * 3 + component] - own[mapping[i]][component])
                  for i in range(len(mapping)) for component in range(components)]
        return {"values": len(errors), "maximum": max(errors),
                "rms": math.sqrt(sum(error * error for error in errors) / len(errors))}
    rgb = channel_error(reference["colors"], local["colors"], 3)
    material = channel_error(reference["materials"], local["materials"], 2)
    unprotected = local.get("mask_convention") == "unprotected_f32_v1"
    weights = [abs(reference["materials"][i * 3 + 2] -
                   (local["masks"][mapping[i]] if unprotected else 1 - local["masks"][mapping[i]]))
               for i in range(len(mapping))]
    report = {
        "reference": {"path": str(reference_path), "sha256": digest(reference_path)},
        "obrush": {"path": str(obrush_path), "sha256": digest(obrush_path)},
        "vertices": len(mapping), "reference_quads": len(reference_faces),
        "obrush_quads": len(local_faces),
        "matched_oriented_quads": len(mapped_reference_faces & actual_faces),
        "unmatched_reference_oriented_quads": len(mapped_reference_faces - actual_faces),
        "unmatched_obrush_oriented_quads": len(actual_faces - mapped_reference_faces),
        "world_position_maximum": max(distances),
        "world_position_rms": math.sqrt(sum(distance * distance for distance in distances) / len(distances)),
        "rgb": rgb, "roughness_metallic": material,
        "unmasked_weight": {"values": len(weights), "maximum": max(weights),
                            "rms": math.sqrt(sum(weight * weight for weight in weights) / len(weights))},
    }
    rounding = 3e-8 if obrush_path.suffix == ".obrush" else 0.0
    report["gates"] = {
        "one_to_one_vertex_map": len(set(mapping)) == len(mapping),
        "all_oriented_quads": len(reference_faces) == len(local_faces) == len(mapped_reference_faces & actual_faces),
        "positions_within_2e_7": report["world_position_maximum"] <= 2e-7,
        "rgb_within_save_rounding": rgb["maximum"] <= rounding,
        "material_within_save_rounding": material["maximum"] <= rounding,
        "unmasked_exact": report["unmasked_weight"]["maximum"] == 0.0,
    }
    output_path.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    if not all(report["gates"].values()):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
