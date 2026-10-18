#!/usr/bin/env python3
"""Compare independent OBrush cube-edge output with a captured SculptGL MC run."""
import hashlib
import json
import math
import sys
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def reference_mesh(snapshot):
    data = json.loads(snapshot.read_text())
    mesh = next(m for m in data["meshes"] if m["id"] == data["active_id"])
    matrix = mesh["matrix"]
    local = list(zip(*(iter(mesh["positions"]),) * 3))
    points = [
        tuple(sum(matrix[axis + 4 * j] * p[j] for j in range(3)) + matrix[axis + 12] for axis in range(3))
        for p in local
    ]
    faces = [tuple(mesh["faces"][i:i + 3]) for i in range(0, len(mesh["faces"]), 4)]
    if any(mesh["faces"][i + 3] != 4294967295 for i in range(0, len(mesh["faces"]), 4)):
        raise ValueError("Reference output contains a nontriangle face")
    return points, faces


def volume(points, faces):
    total = 0.0
    for a, b, c in faces:
        p, q, r = points[a], points[b], points[c]
        total += (p[0] * (q[1] * r[2] - q[2] * r[1])
                  + p[1] * (q[2] * r[0] - q[0] * r[2])
                  + p[2] * (q[0] * r[1] - q[1] * r[0])) / 6.0
    return total


def nearest_stats(source, target):
    distances = []
    for a in source:
        best = min(sum((a[i] - b[i]) ** 2 for i in range(3)) for b in target)
        distances.append(math.sqrt(best))
    distances.sort()
    return {
        "maximum": distances[-1],
        "rms": math.sqrt(sum(x * x for x in distances) / len(distances)),
        "median": distances[len(distances) // 2],
        "p95": distances[int((len(distances) - 1) * 0.95)],
        "exact_within_1e_6": sum(x <= 1e-6 for x in distances),
    }


def vertex_mapping(reference, obrush, tolerance=1e-6):
    mapping = []
    for point in reference:
        distances = [sum((point[i] - other[i]) ** 2 for i in range(3)) for other in obrush]
        best = min(range(len(distances)), key=distances.__getitem__)
        if distances[best] > tolerance * tolerance:
            raise ValueError("Unrelaxed vertex positions exceed mapping tolerance")
        mapping.append(best)
    if len(set(mapping)) != len(mapping):
        raise ValueError("Unrelaxed vertex map is not bijective")
    return mapping


def oriented(face):
    return min(tuple(face[i:] + face[:i]) for i in range(3))


def attribute_difference(reference, local, mapping, channels, stride=3, invert_local=False):
    differences = [
        abs(reference[i * stride + channel] -
            (1.0 - local[mapping[i]] if invert_local else
             local[mapping[i]] if stride == 1 else local[mapping[i]][channel]))
        for i in range(len(mapping)) for channel in range(channels)
    ]
    return {
        "values_compared": len(differences),
        "maximum_abs_difference": max(differences),
        "rms_abs_difference": math.sqrt(sum(value * value for value in differences) / len(differences)),
    }


def main():
    reference_dir, local_path, output_path = map(Path, sys.argv[1:4])
    reference_prefix = sys.argv[4] if len(sys.argv) > 4 else "mc-remesh"
    if local_path.is_dir():
        cases = []
        sources = []
        local_prefix = {
            "mc-fusion": "marching-cubes-fusion",
            "mc-mixed-uv-fusion": "marching-cubes-mixed-uv-fusion",
            "mc-seamed-mixed-uv-fusion": "marching-cubes-seamed-mixed-uv-fusion",
            "mc-posed-fusion": "marching-cubes-posed-fusion",
        }.get(reference_prefix, "marching-cubes")
        for relaxed in (False, True):
            project_path = local_path / f"{local_prefix}-{str(relaxed).lower()}-after.obrush"
            mesh = json.loads(project_path.read_text())["mesh"]
            cases.append({"relaxed": relaxed, "positions": mesh["positions"], "triangles": mesh["triangles"],
                          "colors": mesh["colors"], "materials": mesh.get("materials"), "masks": mesh["masks"],
                          "mask_convention": mesh.get("mask_convention")})
            sources.append({"path": str(project_path), "sha256": sha(project_path)})
        obrush_source = {"browser_projects": sources}
    else:
        cases = json.loads(local_path.read_text())
        obrush_source = {"path": str(local_path), "sha256": sha(local_path)}
    result = {"reference_snapshots": {}, "obrush_snapshot": obrush_source, "cases": []}
    raw = {}
    for case in cases:
        relaxed = case["relaxed"]
        reference_path = reference_dir / f"{reference_prefix}-{str(relaxed).lower()}-after.json"
        result["reference_snapshots"][str(relaxed).lower()] = {"path": str(reference_path), "sha256": sha(reference_path)}
        rp, rf = reference_mesh(reference_path)
        op = case["positions"]
        of = case["triangles"]
        raw[relaxed] = (rp, rf, op, of)
        result["cases"].append({
            "relaxed": relaxed,
            "reference": {"vertices": len(rp), "triangles": len(rf), "signed_world_volume": volume(rp, rf)},
            "obrush": {"vertices": len(op), "triangles": len(of), "signed_world_volume": volume(op, of)},
            "reference_to_obrush_nearest_world": nearest_stats(rp, op),
            "obrush_to_reference_nearest_world": nearest_stats(op, rp),
        })
    reference, reference_faces, obrush, obrush_faces = raw[False]
    mapping = vertex_mapping(reference, obrush)
    mapped_faces = [tuple(mapping[i] for i in face) for face in reference_faces]
    oriented_reference = {oriented(list(face)) for face in mapped_faces}
    oriented_obrush = {oriented(list(face)) for face in obrush_faces}
    result["unrelaxed_face_mapping"] = {
        "mapped_vertices": len(mapping),
        "oriented_reference_faces": len(oriented_reference),
        "oriented_obrush_faces": len(oriented_obrush),
        "oriented_intersection": len(oriented_reference & oriented_obrush),
        "unoriented_intersection": len({tuple(sorted(f)) for f in mapped_faces} & {tuple(sorted(f)) for f in obrush_faces}),
    }
    relaxed_reference, _, relaxed_obrush, _ = raw[True]
    mapped_distances = [math.dist(relaxed_reference[i], relaxed_obrush[mapping[i]]) for i in range(len(mapping))]
    relaxed_reference_faces = raw[True][1]
    relaxed_obrush_faces = raw[True][3]
    mapped_relaxed_faces = [tuple(mapping[i] for i in face) for face in relaxed_reference_faces]
    oriented_relaxed_reference = {oriented(list(face)) for face in mapped_relaxed_faces}
    oriented_relaxed_obrush = {oriented(list(face)) for face in relaxed_obrush_faces}
    result["relaxed_face_mapping"] = {
        "oriented_reference_faces": len(oriented_relaxed_reference),
        "oriented_obrush_faces": len(oriented_relaxed_obrush),
        "oriented_intersection": len(oriented_relaxed_reference & oriented_relaxed_obrush),
    }
    result["relaxed_mapped_position_difference"] = {
        "maximum": max(mapped_distances),
        "rms": math.sqrt(sum(d * d for d in mapped_distances) / len(mapped_distances)),
    }
    result["attribute_differences"] = []
    for case in cases:
        reference_path = reference_dir / f"{reference_prefix}-{str(case['relaxed']).lower()}-after.json"
        snapshot = json.loads(reference_path.read_text())
        mesh = next(mesh for mesh in snapshot["meshes"] if mesh["id"] == snapshot["active_id"])
        differences = {"relaxed": case["relaxed"]}
        if case.get("colors") is not None:
            differences["rgb"] = attribute_difference(mesh["colors"], case["colors"], mapping, 3)
        if case.get("materials") is not None:
            differences["roughness_metallic"] = attribute_difference(mesh["materials"], case["materials"], mapping, 2)
        if case.get("masks") is not None:
            reference_unmasked = mesh["materials"][2::3]
            differences["unmasked_weight"] = attribute_difference(
                reference_unmasked, case["masks"], mapping, 1, stride=1,
                invert_local=case.get("mask_convention") != "unprotected_f32_v1")
        result["attribute_differences"].append(differences)
    output_path.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result["cases"], indent=2))
    print(json.dumps({"unrelaxed_face_mapping": result["unrelaxed_face_mapping"], "relaxed_mapped_position_difference": result["relaxed_mapped_position_difference"]}, indent=2))


if __name__ == "__main__":
    main()
