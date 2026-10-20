#!/usr/bin/env python3
"""Offline adjacent-fold localization for retained public OBrush projects."""

import json
import math
import sys
from collections import defaultdict


def subtract(a, b):
    return [x - y for x, y in zip(a, b)]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def norm(a):
    length = math.sqrt(sum(x * x for x in a))
    return [x / length for x in a]


def fold_edges(mesh):
    vertices = mesh["positions"]
    faces = mesh["triangles"]
    normals = []
    edges = defaultdict(list)
    for face_id, face in enumerate(faces):
        a, b, c = (vertices[index] for index in face)
        normals.append(norm(cross(subtract(b, a), subtract(c, a))))
        for start, end in ((face[0], face[1]), (face[1], face[2]), (face[2], face[0])):
            edges[tuple(sorted((start, end)))].append(face_id)
    result = {}
    for edge, pair in edges.items():
        if len(pair) != 2:
            continue
        first, second = (normals[face_id] for face_id in pair)
        degrees = math.degrees(math.acos(max(-1.0, min(1.0, sum(x * y for x, y in zip(first, second))))))
        result[edge] = (degrees, pair)
    return result


old, new = (json.load(open(path, encoding="utf-8"))["mesh"] for path in sys.argv[1:3])
before = fold_edges(old)
after = fold_edges(new)
for edge, (angle, pair) in sorted(after.items(), key=lambda item: -item[1][0])[:12]:
    points = [new["positions"][index] for index in edge]
    print(json.dumps({"edge": edge, "after_degrees": angle, "before_degrees": before.get(edge, (None,))[0], "faces": pair, "positions": points, "new_vertex": max(edge) >= len(old["positions"]), "old_faces": [old["triangles"][face] for face in pair if face < len(old["triangles"])], "new_faces": [new["triangles"][face] for face in pair]}))
