#!/usr/bin/env python3
"""Read-only geometric fold audit for a versioned .obrush project.

Requires NumPy. Does not alter the supplied project or write a repaired mesh.
"""
import argparse
import hashlib
import json
from pathlib import Path

import numpy as np


def edge_pairs(triangles):
    edges = np.concatenate((triangles[:, [0, 1]], triangles[:, [1, 2]], triangles[:, [2, 0]]))
    faces = np.tile(np.arange(len(triangles)), 3)
    directed_forward = edges[:, 0] < edges[:, 1]
    keys = np.sort(edges, axis=1)
    order = np.lexsort((keys[:, 1], keys[:, 0]))
    keys, faces, directed_forward = keys[order], faces[order], directed_forward[order]
    unique, first, counts = np.unique(keys, axis=0, return_index=True, return_counts=True)
    manifold = np.all(counts == 2)
    if not manifold:
        return unique, None, {'boundary_edges': int(np.sum(counts == 1)), 'nonmanifold_edges': int(np.sum(counts > 2)), 'opposite_winding_pairs': None}
    pairs = faces.reshape(-1, 2)
    opposite = directed_forward.reshape(-1, 2)[:, 0] != directed_forward.reshape(-1, 2)[:, 1]
    return unique, pairs, {'boundary_edges': 0, 'nonmanifold_edges': 0, 'opposite_winding_pairs': int(np.sum(opposite))}


def metrics(positions, triangles, edges, pairs):
    a, b, c = (positions[triangles[:, k]] for k in range(3))
    cross = np.cross(b - a, c - a)
    area = np.linalg.norm(cross, axis=1) / 2
    lengths = np.stack((np.linalg.norm(b-a, axis=1), np.linalg.norm(c-b, axis=1), np.linalg.norm(a-c, axis=1)), axis=1)
    quality = 4*np.sqrt(3)*area/np.maximum(np.sum(lengths*lengths, axis=1), 1e-30)
    normals = cross/np.maximum(2*area[:, None], 1e-30)
    if pairs is None:
        return {'area_min': float(area.min()), 'quality_min': float(quality.min())}, area, quality, cross, None
    cosine = np.einsum('ij,ij->i', normals[pairs[:, 0]], normals[pairs[:, 1]])
    angle = np.degrees(np.arccos(np.clip(cosine, -1, 1)))
    centers = (positions[edges[:, 0]]+positions[edges[:, 1]])/2
    over120 = centers[angle > 120]
    summary = {
        'vertices': int(len(positions)), 'triangles': int(len(triangles)), 'edges': int(len(edges)),
        'finite_positions': bool(np.isfinite(positions).all()),
        'minimum_face_area': float(area.min()), 'median_face_area': float(np.median(area)),
        'faces_area_below_1e-10': int(np.sum(area < 1e-10)),
        'minimum_equilateral_quality': float(quality.min()),
        'faces_quality_below_0_05': int(np.sum(quality < .05)),
        'faces_quality_below_0_1': int(np.sum(quality < .1)),
        'fold_edges_over_90': int(np.sum(angle > 90)),
        'fold_edges_over_120': int(np.sum(angle > 120)),
        'fold_edges_over_149_9': int(np.sum(angle > 149.9)),
        'maximum_fold_degrees': float(angle.max()),
        'fold_edges_over_90_x_negative': int(np.sum(centers[angle > 90, 0] < -1e-4)),
        'fold_edges_over_90_x_positive': int(np.sum(centers[angle > 90, 0] > 1e-4)),
        'fold_edges_over_120_bbox': {'min': over120.min(axis=0).tolist(), 'max': over120.max(axis=0).tolist()} if len(over120) else None,
    }
    return summary, area, quality, cross, angle


def laplacian_negative_control(positions, triangles, edges, pairs, baseline):
    angle = baseline[4]
    seed = np.unique(edges[angle > 90])
    selected = np.zeros(len(positions), dtype=bool)
    selected[seed] = True
    selected[edges[np.any(selected[edges], axis=1)].ravel()] = True  # one edge ring
    sums = np.zeros_like(positions)
    counts = np.zeros(len(positions), dtype=np.int32)
    np.add.at(sums, edges[:, 0], positions[edges[:, 1]])
    np.add.at(sums, edges[:, 1], positions[edges[:, 0]])
    np.add.at(counts, edges[:, 0], 1)
    np.add.at(counts, edges[:, 1], 1)
    proposed = positions.copy()
    proposed[selected] = positions[selected]*.9 + (sums/np.maximum(counts[:, None], 1))[selected]*.1
    next_summary, next_area, next_quality, next_cross, _ = metrics(proposed, triangles, edges, pairs)
    original_area, original_quality, original_cross = baseline[1:4]
    return {
        'scope': 'One simultaneous 0.1 uniform-Laplacian step on >90-degree edge endpoints and one edge ring; diagnostic only, not a safe repair',
        'selected_vertices': int(np.sum(selected)),
        'maximum_vertex_displacement': float(np.max(np.linalg.norm(proposed-positions, axis=1))),
        'fold_edges_over_90_before_after': [baseline[0]['fold_edges_over_90'], next_summary['fold_edges_over_90']],
        'fold_edges_over_120_before_after': [baseline[0]['fold_edges_over_120'], next_summary['fold_edges_over_120']],
        'minimum_equilateral_quality_before_after': [float(original_quality.min()), float(next_quality.min())],
        'faces_quality_below_0_05_before_after': [int(np.sum(original_quality < .05)), int(np.sum(next_quality < .05))],
        'new_reversed_faces': int(np.sum(np.einsum('ij,ij->i', original_cross, next_cross) <= 0)),
        'new_faces_area_below_1e-10': int(np.sum((original_area > 1e-10) & (next_area < 1e-10))),
    }


def run(path):
    raw = path.read_bytes()
    if len(raw) > 256_000_000:
        raise ValueError('Project exceeds diagnostic allocation limit')
    document = json.loads(raw)
    mesh = document['mesh']
    positions = np.asarray(mesh['positions'], dtype=np.float64)
    triangles = np.asarray(mesh['triangles'], dtype=np.int64)
    if positions.ndim != 2 or positions.shape[1] != 3 or triangles.ndim != 2 or triangles.shape[1] != 3:
        raise ValueError('Expected XYZ positions and triangle triplets')
    if len(positions) > 1_000_000 or len(triangles) > 2_000_000 or not np.isfinite(positions).all() or np.any(triangles < 0) or np.any(triangles >= len(positions)):
        raise ValueError('Input fails diagnostic allocation, finite, or index gate')
    edges, pairs, topology = edge_pairs(triangles)
    baseline = metrics(positions, triangles, edges, pairs)
    output = {'input_sha256': hashlib.sha256(raw).hexdigest(), 'version': document.get('version'), 'topology': topology, 'geometry': baseline[0]}
    if pairs is not None:
        output['negative_control'] = laplacian_negative_control(positions, triangles, edges, pairs, baseline)
    return output


def self_check():
    tetra_points = np.array([[1,1,1],[-1,-1,1],[-1,1,-1],[1,-1,-1]], dtype=float)
    tetra_faces = np.array([[0,2,1],[0,1,3],[0,3,2],[1,2,3]], dtype=np.int64)
    tetra_edges, tetra_pairs, tetra_topology = edge_pairs(tetra_faces)
    tetra = metrics(tetra_points, tetra_faces, tetra_edges, tetra_pairs)[0]
    assert tetra_topology['opposite_winding_pairs'] == 6 and tetra['fold_edges_over_90'] == 6
    assert abs(tetra['maximum_fold_degrees'] - 109.47122063449069) < 1e-10
    points = np.array([[0,0,0],[1,0,0],[0,1,0],[0,.5,np.sqrt(3)/2]], dtype=float)
    triangles = np.array([[0,1,2],[1,0,3]], dtype=np.int64)
    edges, pairs, topo = edge_pairs(triangles)
    values = metrics(points, triangles, edges, pairs)[0]
    assert topo['boundary_edges'] == 4 and pairs is None  # open diagnostic must not invent paired folds
    # Direct analytic normals on the shared [0,1] hinge are 120 degrees apart.
    n0 = np.cross(points[1]-points[0], points[2]-points[0])
    n1 = np.cross(points[0]-points[1], points[3]-points[1])
    angle = np.degrees(np.arccos(np.dot(n0,n1)/(np.linalg.norm(n0)*np.linalg.norm(n1))))
    assert abs(angle-120) < 1e-10 and values['area_min'] > 0
    print('Synthetic closed-tetrahedron and open-hinge self-check passed')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('project', nargs='?', type=Path)
    parser.add_argument('--self-check', action='store_true')
    args = parser.parse_args()
    if args.self_check:
        self_check()
    else:
        if args.project is None:
            parser.error('project path required')
        print(json.dumps(run(args.project), indent=2))
