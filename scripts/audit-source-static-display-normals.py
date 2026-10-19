#!/usr/bin/env python3
"""Independent f32-word oracle for a *fresh* Source original-face GPU upload.

This intentionally does not assert edited/Undo/Redo retained display normals:
those depend on selected-row history absent from the public project format.
"""

import argparse
import hashlib
import json
import math
import struct
from pathlib import Path

TRIANGLE_CORNER = 0xFFFFFFFF


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0])


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def original_face_display_hash(project):
    mesh = project["mesh"]
    positions = [tuple(f32(x) for x in row) for row in mesh["positions"]]
    if mesh.get("surface") is not None:
        faces = mesh["surface"]["faces"]
        origin = "surface.faces"
    else:
        faces = [(*row, TRIANGLE_CORNER) for row in mesh["triangles"]]
        origin = "triangles"
    if not positions or not faces:
        raise ValueError("empty mesh")
    if any(len(p) != 3 or not all(map(math.isfinite, p)) for p in positions):
        raise ValueError("invalid position row")
    for face in faces:
        if len(face) != 4 or any(
            not isinstance(index, int) or index < 0 or index >= len(positions)
            for index in face[:3]
        ) or (face[3] != TRIANGLE_CORNER and (
            not isinstance(face[3], int) or face[3] < 0 or face[3] >= len(positions)
        )):
            raise ValueError("invalid face row")
    cached = []
    for a, b, c, d in faces:
        pa, pb, pc = positions[a], positions[b], positions[c]
        edge = sub(pc, pa)
        normal = cross(sub(pb, pa), edge)
        if d != TRIANGLE_CORNER:
            other = cross(sub(pc, positions[d]), edge)
            normal = tuple(x + y for x, y in zip(normal, other))
        normal = tuple(f32(x) for x in normal)
        if not all(map(math.isfinite, normal)):
            raise ValueError("invalid face normal")
        cached.append(normal)
    sums = [[0.0, 0.0, 0.0] for _ in positions]
    counts = [0] * len(positions)
    for face_index in range(len(faces) - 1, -1, -1):
        face = faces[face_index]
        for vertex in face[:3 if face[3] == TRIANGLE_CORNER else 4]:
            counts[vertex] += 1
            for channel in range(3):
                sums[vertex][channel] += cached[face_index][channel]
    digest = hashlib.sha256()
    for count, summed in zip(counts, sums):
        factor = 0.0 if count == 0 else 1.0 / count
        row = tuple(f32(component * factor) for component in summed)
        if not all(map(math.isfinite, row)):
            raise ValueError("invalid vertex normal")
        digest.update(struct.pack("<fff", *row))
    public_digest = hashlib.sha256()
    for row in mesh["normals"]:
        public_digest.update(struct.pack("<fff", *(f32(x) for x in row)))
    return {"vertices": len(positions), "faces": len(faces), "face_origin": origin,
            "expected_static_display_normal_words_sha256": digest.hexdigest(),
            "public_mesh_normal_words_sha256": public_digest.hexdigest()}


def audit(project_path, readback_path, stage):
    project_bytes = Path(project_path).read_bytes()
    project = json.loads(project_bytes)
    readback = json.loads(Path(readback_path).read_bytes())
    observed = readback["stages"][stage]
    report = {"stage": stage, "public_save_sha256": hashlib.sha256(project_bytes).hexdigest(),
              "browser_readback_sha256": hashlib.sha256(Path(readback_path).read_bytes()).hexdigest(),
              **original_face_display_hash(project),
              "active_vbo_candidates": observed["active_vbo_candidates"],
              "actual_gpu_normal_words_sha256": observed["normal_words_sha256"],
              "gl_error": observed["gl_error"]}
    report["static_display_words_exact"] = (
        observed["active_vbo_candidates"] == 1 and observed["gl_error"] == 0
        and report["expected_static_display_normal_words_sha256"]
        == report["actual_gpu_normal_words_sha256"])
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project", type=Path)
    parser.add_argument("readback", type=Path)
    parser.add_argument("--stage", default="before", choices=("before", "after", "stable", "undo", "redo"))
    args = parser.parse_args()
    report = audit(args.project, args.readback, args.stage)
    print(json.dumps(report, indent=2, allow_nan=False))
    return 0 if report["static_display_words_exact"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
