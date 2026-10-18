"""Compare controlled coarse-edit static subdivision in pinned SculptGL and Rust.

Usage: python3 scripts/compare-uv-coarse-edit.py REFERENCE_RESULT HOST_PROBE_DIR
Both paths must contain immutable receipts. The reference's single vertex edit
is diagnostic, not a normal brush or undo workflow.
"""
import hashlib
import json
import math
import pathlib
import sys


def digest(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def snapshot(item):
    if digest(item["path"]) != item["sha256"]:
        raise ValueError("Reference snapshot changed: " + item["path"])
    mesh = json.loads(pathlib.Path(item["path"]).read_text())["meshes"][0]
    return {
        field: [mesh[field][i:i + width] for i in range(0, len(mesh[field]), width)]
        for field, width in (("positions", 3), ("normals", 3), ("faces", 4))
    }


def index(rows):
    result = {}
    for vertex, point in enumerate(rows):
        key = tuple(round(value, 5) for value in point)
        if key in result:
            raise ValueError("Duplicate physical fine point in comparison")
        result[key] = vertex
    return result


def errors(source, host, mapping):
    differences = [abs(a - b)
                   for source_index, host_index in mapping
                   for a, b in zip(source[source_index], host[host_index])]
    return {"components": len(differences),
            "max_component_error": max(differences, default=0),
            "rms_component_error": math.sqrt(sum(x * x for x in differences) / len(differences))}


def displacement(before, after):
    return [[a - b for a, b in zip(new, old)] for old, new in zip(before, after)]


def unit(values):
    length = math.sqrt(sum(value * value for value in values))
    return [value / length for value in values] if length else [0.0, 0.0, 0.0]


def subtract(a, b):
    return [x - y for x, y in zip(a, b)]


def add(a, b):
    return [x + y for x, y in zip(a, b)]


def scale(a, factor):
    return [x * factor for x in a]


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0]]


def face_average_normals(positions, faces):
    """Independent raw-face normal oracle, distinct from OBrush shading normals."""
    sums = [[0.0, 0.0, 0.0] for _ in positions]
    for raw in faces:
        face = [vertex for vertex in raw if vertex != 4294967295]
        points = [positions[vertex] for vertex in face]
        normal = cross(subtract(points[1], points[0]), subtract(points[2], points[0]))
        if len(face) == 4:
            normal = add(normal, cross(subtract(points[2], points[3]),
                                       subtract(points[2], points[0])))
        for vertex in face:
            sums[vertex] = add(sums[vertex], normal)
    return list(map(unit, sums))


def frame_response(old_positions, old_prediction, new_prediction,
                   normals, neighbors):
    output = []
    for vertex, neighbor in enumerate(neighbors):
        normal = normals[vertex]
        old_edge = subtract(old_prediction[neighbor], old_prediction[vertex])
        new_edge = subtract(new_prediction[neighbor], new_prediction[vertex])
        old_tangent = unit(subtract(old_edge, scale(normal, dot(old_edge, normal))))
        new_tangent = unit(subtract(new_edge, scale(normal, dot(new_edge, normal))))
        detail = subtract(old_positions[vertex], old_prediction[vertex])
        old_binormal = cross(normal, old_tangent)
        new_binormal = cross(normal, new_tangent)
        output.append(add(new_prediction[vertex], add(
            scale(normal, dot(normal, detail)), add(
                scale(new_tangent, dot(old_tangent, detail)),
                scale(new_binormal, dot(old_binormal, detail))))))
    return output


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    reference_path, host_dir = map(pathlib.Path, sys.argv[1:])
    reference = json.loads(reference_path.read_text())
    if reference["status"] != "completed reference-only controlled coarse-edit subdivision fixtures":
        raise ValueError("Reference diagnostic did not complete")
    result = {"reference_result_sha256": digest(reference_path), "cases": []}
    for case in reference["cases"]:
        if not case["name"].startswith("reference-uv-coarse-edit-"):
            continue
        fixture = case["name"].removeprefix("reference-uv-coarse-edit-")
        host_path = host_dir / (fixture + ".json")
        host = json.loads(host_path.read_text())
        if digest(host["input"]) != case["fixture"]["sha256"]:
            raise ValueError("Paired OBJ fixture hash differs: " + fixture)
        fine = snapshot(case["fine"])
        edited = snapshot(case["edited_fine"])
        coarse = snapshot(case["coarse"])
        edited_coarse = snapshot(case["edited_coarse"])
        source_points, host_points = index(fine["positions"]), index(host["fine_before"])
        if set(source_points) != set(host_points):
            raise ValueError("Paired fine physical point sets differ: " + fixture)
        mapping = [(source_points[key], host_points[key]) for key in source_points]
        reference_old = [case["old_prediction"][i:i + 3]
                         for i in range(0, len(case["old_prediction"]), 3)]
        reference_new = [case["new_prediction"][i:i + 3]
                         for i in range(0, len(case["new_prediction"]), 3)]
        source_neighbor = case["source_frame"]["first_neighbor"]
        host_first_neighbor = [None] * len(host_points)
        for raw_face in host["fine_faces"]:
            face = [vertex for vertex in raw_face if vertex != 4294967295]
            for corner, vertex in enumerate(face):
                if host_first_neighbor[vertex] is None:
                    host_first_neighbor[vertex] = face[corner - 1]
        neighbor_mismatches = 0
        for source_index, host_index in mapping:
            source_neighbor_key = tuple(round(value, 5)
                                        for value in fine["positions"][source_neighbor[source_index]])
            if host_first_neighbor[host_index] != host_points[source_neighbor_key]:
                neighbor_mismatches += 1
        unit_source_normals = list(map(unit, fine["normals"]))
        unit_host_normals = list(map(unit, host["fine_before_normals"]))
        source_style_normals = face_average_normals(host["fine_before"], host["fine_faces"])
        source_neighbor_in_host_order = [None] * len(host_points)
        for source_index, host_index in mapping:
            source_neighbor_key = tuple(round(value, 5)
                                        for value in fine["positions"][source_neighbor[source_index]])
            source_neighbor_in_host_order[host_index] = host_points[source_neighbor_key]
        diagnostic_response = frame_response(
            host["fine_before"], host["old_prediction"], host["new_prediction"],
            source_style_normals, source_neighbor_in_host_order)
        target = tuple(round(value, 5) for value in case["edit"]["before"])
        host_target = index(host["coarse_before"]).get(target)
        if host_target != host["vertex"]:
            raise ValueError("Coarse edit targets different physical vertices: " + fixture)
        source_coarse_delta = displacement(coarse["positions"], edited_coarse["positions"])
        host_coarse_delta = displacement(host["coarse_before"], host["coarse_after"])
        source_coarse_index = index(coarse["positions"])[target]
        if source_coarse_delta[source_coarse_index] != case["edit"]["delta"]:
            raise ValueError("Reference coarse mutation differs from declared delta")
        if host_coarse_delta[host_target] != case["edit"]["delta"]:
            raise ValueError("Host coarse mutation differs from declared delta")
        result["cases"].append({
            "fixture": fixture,
            "fixture_sha256": case["fixture"]["sha256"],
            "host_probe_path": str(host_path),
            "host_probe_sha256": digest(host_path),
            "fine_vertices": len(mapping),
            "edit_delta": case["edit"]["delta"],
            "old_prediction": errors(reference_old, host["old_prediction"], mapping),
            "new_prediction": errors(reference_new, host["new_prediction"], mapping),
            "unit_fine_normals": errors(unit_source_normals, unit_host_normals, mapping),
            "diagnostic_face_average_unit_normals": errors(
                unit_source_normals, source_style_normals, mapping),
            "first_neighbor_mismatched_vertices": neighbor_mismatches,
            "edited_fine_absolute": errors(edited["positions"], host["fine_after"], mapping),
            "edited_fine_displacement": errors(
                displacement(fine["positions"], edited["positions"]),
                displacement(host["fine_before"], host["fine_after"]), mapping),
            "diagnostic_reference_neighbor_frame_absolute": errors(
                edited["positions"], diagnostic_response, mapping),
        })
    if len(result["cases"]) != 3:
        raise ValueError("Expected exactly three controlled coarse-edit fixtures")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
