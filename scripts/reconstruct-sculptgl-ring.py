"""Reconstruct the pinned SculptGL first-neighbor choice from a plain OBJ mesh.

Usage: python3 scripts/reconstruct-sculptgl-ring.py REFERENCE_C102_RESULT HOST_SUBDIVISION_PROBE_DIR [HOST_COARSE_PROBE_DIR]
This diagnostic consumes only host parent topology/UVs to predict the source
face visit, subdivision child order and fine-vertex neighbor. The reference
snapshots are used afterward as independent validation, never as input to the
prediction. No product mesh or saved project is changed.
"""
import hashlib
import json
import math
import pathlib
import sys


SENTINEL = 4294967295


def digest(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def snapshot(item):
    if digest(item["path"]) != item["sha256"]:
        raise ValueError("Reference snapshot hash changed")
    mesh = json.loads(pathlib.Path(item["path"]).read_text())["meshes"][0]
    return {
        "positions": [mesh["positions"][i:i + 3]
                      for i in range(0, len(mesh["positions"]), 3)],
        "faces": [mesh["faces"][i:i + 4]
                  for i in range(0, len(mesh["faces"]), 4)],
    }


def point_key(point):
    return tuple(round(value, 5) for value in point)


def active(face):
    return [vertex for vertex in face if vertex != SENTINEL]


def oriented_cycle(face):
    return min(tuple(face[offset:] + face[:offset]) for offset in range(len(face)))


def post_transform_visit(parent):
    """Bounded face visitation with the source's 32-entry UV-aware cache rule."""
    faces, uv_faces = parent["faces"], parent["face_uvs"]
    count = len(parent["positions"])
    first_uv = [None] * count
    extra_uv = [[] for _ in range(count)]
    for face, uv_face in zip(faces, uv_faces):
        for vertex, uv in zip(face, uv_face):
            if vertex == SENTINEL:
                continue
            if first_uv[vertex] is None:
                first_uv[vertex] = uv
            elif uv != first_uv[vertex] and uv not in extra_uv[vertex]:
                extra_uv[vertex].append(uv)
    if any(value is None for value in first_uv):
        raise ValueError("Isolated vertex in diagnostic input")
    starts, next_id = [], count
    for row in extra_uv:
        starts.append(next_id)
        next_id += len(row)
    pseudo_faces = []
    for face, uv_face in zip(faces, uv_faces):
        pseudo_faces.append([
            vertex if uv == first_uv[vertex] else starts[vertex] + extra_uv[vertex].index(uv)
            for vertex, uv in zip(face, uv_face) if vertex != SENTINEL
        ])
    owner = list(range(count)) + [vertex for vertex, row in enumerate(extra_uv) for _ in row]
    incidents = [[] for _ in range(count)]
    for face_id, face in enumerate(faces):
        for vertex in active(face):
            incidents[vertex].insert(0, face_id)
    live = [len(incidents[vertex]) for vertex in owner]
    timestamp = [0] * next_id
    clock, cursor, fan = 33, 0, 0
    emitted = [False] * len(faces)
    dead_end, visit = [], []
    while fan >= 0:
        candidates = []
        for face_id in incidents[owner[fan]]:
            if emitted[face_id]:
                continue
            emitted[face_id] = True
            visit.append(face_id)
            for uv_vertex in pseudo_faces[face_id]:
                dead_end.append(uv_vertex)
                candidates.append(uv_vertex)
                live[uv_vertex] -= 1
                if clock - timestamp[uv_vertex] > 32:
                    timestamp[uv_vertex] = clock
                    clock += 1
        fan, best_priority = -1, -1
        for vertex in candidates:
            remaining = live[vertex]
            if remaining <= 0:
                continue
            age = clock - timestamp[vertex]
            priority = age if age + 2 * remaining <= 32 else 0
            if priority > best_priority:
                fan, best_priority = vertex, priority
        if fan >= 0:
            continue
        while dead_end:
            vertex = dead_end.pop()
            if live[vertex] > 0:
                fan = vertex
                break
        if fan >= 0:
            continue
        while cursor < next_id:
            vertex = cursor
            cursor += 1
            if live[vertex] > 0:
                fan = vertex
                break
    if len(visit) != len(faces):
        raise ValueError("Face visit did not cover the whole mesh")
    return visit


def child_labels(faces, vertex_count):
    """Topology-only first-encounter edge and quad-center child ordering."""
    labels = [("v", vertex) for vertex in range(vertex_count)]
    edges = {}
    child_faces = []
    for raw in faces:
        vertices = active(raw)
        mids = []
        for corner, vertex in enumerate(vertices):
            next_vertex = vertices[(corner + 1) % len(vertices)]
            edge = ("e", min(vertex, next_vertex), max(vertex, next_vertex))
            if edge not in edges:
                edges[edge] = len(labels)
                labels.append(edge)
            mids.append(edge)
        if len(vertices) == 3:
            a, b, c = [("v", vertex) for vertex in vertices]
            x, y, z = mids
            child_faces.extend(((x, y, z), (a, x, z), (x, b, y), (y, c, z)))
        elif len(vertices) == 4:
            a, b, c, d = [("v", vertex) for vertex in vertices]
            x, y, z, w = mids
            center = ("f",) + tuple(sorted(vertices))
            if center in labels:
                raise ValueError("Duplicate quad face in diagnostic input")
            labels.append(center)
            child_faces.extend(((a, x, center, w), (x, b, y, center),
                                (center, y, c, z), (w, center, z, d)))
        else:
            raise ValueError("Expected triangle or quad")
    return labels, child_faces


def first_ring_neighbor(child_faces):
    # Incident face rows are visited in reverse face order. Its first pair is
    # asymmetric at the last corner, as observed from the source ring contract.
    first = {}
    for face in child_faces:
        for corner, vertex in enumerate(face):
            if corner == 0:
                neighbor = face[-1]
            elif corner == 1:
                neighbor = face[0]
            elif corner == 2:
                neighbor = face[3] if len(face) == 4 else face[0]
            else:
                neighbor = face[0]
            first[vertex] = neighbor
    return first


def minus(a, b):
    return [x - y for x, y in zip(a, b)]


def plus(a, b):
    return [x + y for x, y in zip(a, b)]


def times(a, scale):
    return [x * scale for x in a]


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0]]


def unit(a):
    length = math.sqrt(dot(a, a))
    return times(a, 1 / length) if length else [0.0, 0.0, 0.0]


def source_normal_frame(positions, faces):
    normals = [[0.0, 0.0, 0.0] for _ in positions]
    for raw in faces:
        face = active(raw)
        points = [positions[index] for index in face]
        area = cross(minus(points[1], points[0]), minus(points[2], points[0]))
        if len(face) == 4:
            area = plus(area, cross(minus(points[2], points[3]),
                                    minus(points[2], points[0])))
        for vertex in face:
            normals[vertex] = plus(normals[vertex], area)
    return list(map(unit, normals))


def synthesize(old_positions, old_prediction, new_prediction, normals, neighbors):
    result = []
    for vertex, neighbor in enumerate(neighbors):
        normal = normals[vertex]
        old_edge = minus(old_prediction[neighbor], old_prediction[vertex])
        new_edge = minus(new_prediction[neighbor], new_prediction[vertex])
        old_tangent = unit(minus(old_edge, times(normal, dot(old_edge, normal))))
        new_tangent = unit(minus(new_edge, times(normal, dot(new_edge, normal))))
        residual = minus(old_positions[vertex], old_prediction[vertex])
        value = plus(new_prediction[vertex], plus(
            times(normal, dot(normal, residual)), plus(
                times(new_tangent, dot(old_tangent, residual)),
                times(cross(normal, new_tangent),
                      dot(cross(normal, old_tangent), residual)))))
        result.append(value)
    return result


def main():
    if len(sys.argv) not in (3, 4):
        raise SystemExit(__doc__)
    reference_path, host_dir = map(pathlib.Path, sys.argv[1:3])
    coarse_dir = pathlib.Path(sys.argv[3]) if len(sys.argv) == 4 else None
    reference = json.loads(reference_path.read_text())
    output = {"reference_result_sha256": digest(reference_path), "cases": []}
    for case in reference["cases"]:
        if not case["name"].startswith("reference-uv-coarse-edit-"):
            continue
        name = case["name"].removeprefix("reference-uv-coarse-edit-")
        host_path = host_dir / (name + ".json")
        host = json.loads(host_path.read_text())
        parent, child = host["parent"], host["child"]
        if digest(host["input"]) != case["fixture"]["sha256"]:
            raise ValueError("Same-OBJ fixture changed: " + name)
        face_visit = post_transform_visit(parent)
        optimized_faces = [parent["faces"][face_id] for face_id in face_visit]
        source_parent, source_child = snapshot(case["before"]), snapshot(case["fine"])
        predicted_parent_faces = [tuple(point_key(parent["positions"][vertex])
                                        for vertex in active(face))
                                  for face in optimized_faces]
        observed_parent_faces = [tuple(point_key(source_parent["positions"][vertex])
                                       for vertex in active(face))
                                 for face in source_parent["faces"]]
        if predicted_parent_faces != observed_parent_faces:
            raise ValueError("Optimized parent face order differs: " + name)
        host_labels, host_child_faces = child_labels(parent["faces"], len(parent["positions"]))
        optimized_labels, optimized_child_faces = child_labels(
            optimized_faces, len(parent["positions"]))
        if len(host_labels) != len(child["positions"]):
            raise ValueError("Host child label count differs")
        label_to_host_id = {label: vertex for vertex, label in enumerate(host_labels)}
        host_child_faces_as_ids = [[label_to_host_id[label] for label in face]
                                   for face in host_child_faces]
        if [oriented_cycle(face) for face in host_child_faces_as_ids] != [
                oriented_cycle(active(face)) for face in child["faces"]]:
            raise ValueError("Host child label reconstruction differs")
        predicted_child_faces = [tuple(point_key(child["positions"][label_to_host_id[label]])
                                       for label in face)
                                 for face in optimized_child_faces]
        observed_child_faces = [tuple(point_key(source_child["positions"][vertex])
                                      for vertex in active(face))
                                for face in source_child["faces"]]
        if predicted_child_faces != observed_child_faces:
            raise ValueError("Optimized child face order differs: " + name)
        predicted_neighbor = first_ring_neighbor(optimized_child_faces)
        source_point_to_index = {point_key(point): index for index, point
                                 in enumerate(source_child["positions"])}
        mismatches = []
        for label in optimized_labels:
            host_vertex = label_to_host_id[label]
            point = point_key(child["positions"][host_vertex])
            source_vertex = source_point_to_index[point]
            source_neighbor = case["source_frame"]["first_neighbor"][source_vertex]
            actual = point_key(source_child["positions"][source_neighbor])
            predicted = point_key(child["positions"][label_to_host_id[predicted_neighbor[label]]])
            if predicted != actual:
                mismatches.append({"vertex": point, "predicted_neighbor": predicted,
                                   "reference_neighbor": actual})
        item = {"fixture": name, "fixture_sha256": case["fixture"]["sha256"],
                                "host_probe_path": str(host_path), "host_probe_sha256": digest(host_path),
                                "optimized_parent_face_order": face_visit,
                                "parent_face_rows_exact": True, "child_face_rows_exact": True,
                                "fine_vertices": len(optimized_labels),
                                "first_neighbor_mismatches": mismatches}
        if coarse_dir:
            coarse_path = coarse_dir / (name + ".json")
            coarse = json.loads(coarse_path.read_text())
            neighbor_ids = [label_to_host_id[predicted_neighbor[label]] for label in host_labels]
            normals = source_normal_frame(coarse["fine_before"], coarse["fine_faces"])
            predicted_positions = synthesize(
                coarse["fine_before"], coarse["old_prediction"],
                coarse["new_prediction"], normals, neighbor_ids)
            source_after = snapshot(case["edited_fine"])
            fine_to_source = {point_key(point): index for index, point
                              in enumerate(source_child["positions"])}
            component_errors = [
                abs(actual - predicted)
                for host_vertex, position in enumerate(coarse["fine_before"])
                for actual, predicted in zip(
                    source_after["positions"][fine_to_source[point_key(position)]],
                    predicted_positions[host_vertex])
            ]
            item.update({"host_coarse_probe_path": str(coarse_path),
                         "host_coarse_probe_sha256": digest(coarse_path),
                         "predicted_frame_max_absolute_component_error": max(component_errors),
                         "predicted_frame_rms_absolute_component_error": math.sqrt(
                             sum(error * error for error in component_errors)
                             / len(component_errors))})
        output["cases"].append(item)
    if len(output["cases"]) != 3:
        raise ValueError("Expected three paired fixtures")
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
