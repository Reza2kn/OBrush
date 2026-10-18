//! CPU fixtures for smooth raised Draw strokes, not reference parity or frame rates.
//! Run `cargo run --locked -p obrush-core --release --example draw_overlap_probe`.
//! Set OBRUSH_DRAW_PROBE_EXPORT_DIR to export independently generated OBJ fixtures.
use glam::DVec3;
use obrush_core::{BrushKind, BrushSettings, Mesh, Ray, Vec3};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

struct Edge {
    vertices: [usize; 2],
    faces: [usize; 2],
    length: f64,
}

struct Topology {
    edges: Vec<Edge>,
    neighbors: Vec<Vec<usize>>,
}

fn precise(v: Vec3) -> DVec3 {
    v.as_dvec3()
}

impl Topology {
    fn new(mesh: &Mesh) -> Self {
        let mut edges = HashMap::<(usize, usize), Vec<usize>>::new();
        let mut neighbors = vec![Vec::new(); mesh.positions.len()];
        for (face, &[a, b, c]) in mesh.triangles.iter().enumerate() {
            for (a, b) in [(a, b), (b, c), (c, a)] {
                let (a, b) = (a as usize, b as usize);
                edges.entry((a.min(b), a.max(b))).or_default().push(face);
                neighbors[a].push(b);
                neighbors[b].push(a);
            }
        }
        for list in &mut neighbors {
            list.sort_unstable();
            list.dedup();
        }
        let mut edges: Vec<_> = edges
            .into_iter()
            .filter_map(|((a, b), faces)| {
                (faces.len() == 2).then(|| Edge {
                    vertices: [a, b],
                    faces: [faces[0], faces[1]],
                    length: precise(mesh.positions[a]).distance(precise(mesh.positions[b])),
                })
            })
            .collect();
        edges.sort_unstable_by_key(|e| e.vertices);
        Self { edges, neighbors }
    }
}

fn percentile(values: &mut [f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_unstable_by(f64::total_cmp);
    values[((values.len() - 1) as f64 * quantile).round() as usize]
}

#[derive(Serialize)]
struct Metrics {
    edited_vertices: usize,
    nonfinite_vertices: usize,
    max_adjacent_normal_angle_degrees: f64,
    adjacent_angle_over_30_edges: usize,
    adjacent_angle_over_60_edges: usize,
    adjacent_angle_over_90_edges: usize,
    edited_patch_adjacent_angle_p95_degrees: f64,
    max_edge_length_world: f64,
    min_edge_length_world: f64,
    max_edge_length_over_initial_median: f64,
    original_vertex_edge_max_stretch: f64,
    edited_patch_edge_length_p95_world: f64,
    min_triangle_quality: f64,
    max_normalized_normal_laplacian: f64,
    edited_patch_normal_laplacian_p95: f64,
    edited_patch_normal_laplacian_mean: f64,
    max_radial_height: f64,
    max_vertex_displacement: f64,
}

fn metrics(
    mesh: &Mesh,
    initial: &Mesh,
    topology: &Topology,
    reference_edge_median: f64,
) -> Metrics {
    let edited: Vec<_> = mesh
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            initial
                .positions
                .get(i)
                .is_none_or(|q| precise(*p).distance(precise(*q)) > 1e-6)
        })
        .collect();
    let mut result = Metrics {
        edited_vertices: edited.iter().filter(|&&v| v).count(),
        nonfinite_vertices: mesh.positions.iter().filter(|p| !p.is_finite()).count(),
        max_adjacent_normal_angle_degrees: 0.0,
        adjacent_angle_over_30_edges: 0,
        adjacent_angle_over_60_edges: 0,
        adjacent_angle_over_90_edges: 0,
        edited_patch_adjacent_angle_p95_degrees: 0.0,
        max_edge_length_world: 0.0,
        min_edge_length_world: f64::INFINITY,
        max_edge_length_over_initial_median: 0.0,
        original_vertex_edge_max_stretch: 0.0,
        edited_patch_edge_length_p95_world: 0.0,
        min_triangle_quality: f64::INFINITY,
        max_normalized_normal_laplacian: 0.0,
        edited_patch_normal_laplacian_p95: 0.0,
        edited_patch_normal_laplacian_mean: 0.0,
        max_radial_height: 0.0,
        max_vertex_displacement: 0.0,
    };
    let mut face_normals = Vec::with_capacity(mesh.triangles.len());
    for &[a, b, c] in &mesh.triangles {
        let p = [a, b, c].map(|i| precise(mesh.positions[i as usize]));
        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
        let denominator =
            p[0].distance_squared(p[1]) + p[1].distance_squared(p[2]) + p[2].distance_squared(p[0]);
        let quality = if denominator > 0.0 {
            2.0 * 3.0_f64.sqrt() * cross.length() / denominator
        } else {
            0.0
        };
        result.min_triangle_quality = result.min_triangle_quality.min(quality);
        face_normals.push(cross.normalize_or_zero());
    }
    let mut patch_angles = Vec::new();
    let mut patch_edge_lengths = Vec::new();
    for edge in &topology.edges {
        let [a, b] = edge.faces;
        let angle = face_normals[a]
            .dot(face_normals[b])
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        result.max_adjacent_normal_angle_degrees =
            result.max_adjacent_normal_angle_degrees.max(angle);
        result.adjacent_angle_over_30_edges += usize::from(angle > 30.0);
        result.adjacent_angle_over_60_edges += usize::from(angle > 60.0);
        result.adjacent_angle_over_90_edges += usize::from(angle > 90.0);
        let [a, b] = edge.vertices;
        let length = precise(mesh.positions[a]).distance(precise(mesh.positions[b]));
        result.max_edge_length_world = result.max_edge_length_world.max(length);
        result.min_edge_length_world = result.min_edge_length_world.min(length);
        result.max_edge_length_over_initial_median = result
            .max_edge_length_over_initial_median
            .max(length / reference_edge_median);
        if let (Some(p), Some(q)) = (initial.positions.get(a), initial.positions.get(b)) {
            result.original_vertex_edge_max_stretch = result
                .original_vertex_edge_max_stretch
                .max(length / precise(*p).distance(precise(*q)));
        }
        if edited[a] || edited[b] {
            patch_angles.push(angle);
            patch_edge_lengths.push(length);
        }
    }
    let mut patch_laplacian = Vec::new();
    for (i, neighbors) in topology.neighbors.iter().enumerate() {
        let p = precise(mesh.positions[i]);
        let original = initial.positions.get(i).map(|p| precise(*p));
        result.max_radial_height = result
            .max_radial_height
            .max(p.length() - original.map_or(1.0, |p| p.length()));
        if let Some(original) = original {
            result.max_vertex_displacement =
                result.max_vertex_displacement.max(p.distance(original));
        }
        if neighbors.is_empty() {
            continue;
        }
        let mut average = DVec3::ZERO;
        let mut mean_edge = 0.0;
        for &j in neighbors {
            let q = precise(mesh.positions[j]);
            average += q;
            mean_edge += p.distance(q);
        }
        average /= neighbors.len() as f64;
        mean_edge /= neighbors.len() as f64;
        let normal = precise(mesh.normals[i]).normalize_or_zero();
        let residual = if mean_edge > 0.0 {
            (p - average).dot(normal).abs() / mean_edge
        } else {
            0.0
        };
        result.max_normalized_normal_laplacian =
            result.max_normalized_normal_laplacian.max(residual);
        if edited[i] {
            patch_laplacian.push(residual);
        }
    }
    result.edited_patch_normal_laplacian_mean =
        patch_laplacian.iter().sum::<f64>() / patch_laplacian.len().max(1) as f64;
    result.edited_patch_normal_laplacian_p95 = percentile(&mut patch_laplacian, 0.95);
    result.edited_patch_adjacent_angle_p95_degrees = percentile(&mut patch_angles, 0.95);
    result.edited_patch_edge_length_p95_world = percentile(&mut patch_edge_lengths, 0.95);
    result
}

fn step_flips(mesh: &Mesh, previous: &[Vec3], previous_triangles: &[[u32; 3]]) -> usize {
    previous_triangles
        .iter()
        .filter(|&&indices| {
            let p = indices.map(|i| precise(mesh.positions[i as usize]));
            let old = indices.map(|i| precise(previous[i as usize]));
            let a = (p[1] - p[0]).cross(p[2] - p[0]);
            let b = (old[1] - old[0]).cross(old[2] - old[0]);
            b.length_squared() > 0.0 && a.dot(b) <= 0.0
        })
        .count()
}

#[derive(Clone, Copy, Debug)]
enum Path {
    Line,
    Crossing,
    Loops,
    Silhouette,
    RepeatPoint,
}

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    level: u32,
    rounds: usize,
    path: Path,
}

fn rays(case: Case) -> Vec<Ray> {
    let view = if matches!(case.path, Path::Silhouette) {
        Vec3::new(0.6, 0.25, 1.0).normalize()
    } else {
        Vec3::Z
    };
    let right = Vec3::Y.cross(view).normalize();
    let up = view.cross(right).normalize();
    let ray = |x, y| Ray {
        origin: view * 3.0 + right * x + up * y,
        direction: -view,
    };
    let mut result = Vec::new();
    for round in 0..case.rounds {
        match case.path {
            Path::RepeatPoint => result.push(ray(0.23, 0.09)),
            Path::Loops => {
                for i in 0..=64 {
                    let sign = if round % 2 == 0 { 1.0 } else { -1.0 };
                    let phase = sign * std::f32::consts::TAU * i as f32 / 64.0;
                    result.push(ray(0.21 + 0.19 * phase.cos(), 0.06 + 0.19 * phase.sin()));
                }
            }
            Path::Line | Path::Crossing | Path::Silhouette => {
                let count = if matches!(case.path, Path::Crossing) {
                    2
                } else {
                    1
                };
                for line in 0..count {
                    for i in 0..=40 {
                        let i = if round % 2 == 0 { i } else { 40 - i };
                        let s = -0.4 + 0.8 * i as f32 / 40.0;
                        result.push(match case.path {
                            Path::Crossing if line == 1 => ray(0.09, s),
                            Path::Silhouette => ray(s * 0.875, 0.86),
                            _ => ray(s, 0.09),
                        });
                    }
                }
            }
        }
    }
    result
}

fn run(case: Case, exports: Option<&PathBuf>) {
    let initial = Mesh::sphere(case.level);
    let topology = Topology::new(&initial);
    let mut initial_edge_lengths: Vec<_> = topology.edges.iter().map(|e| e.length).collect();
    let reference_edge_median = percentile(&mut initial_edge_lengths, 0.5);
    let mut mesh = initial.clone();
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        radius: 0.18,
        strength: 0.35,
        symmetry_x: true,
        invert: false,
        ..BrushSettings::default()
    };
    let rays = rays(case);
    let start = Instant::now();
    let mut flip_events = 0;
    let mut missed_rays = 0;
    let mut changed_vertices_total = 0;
    for ray in &rays {
        if let Some(hit) = mesh.raycast(*ray) {
            let old = mesh.positions.clone();
            let old_triangles = mesh.triangles.clone();
            changed_vertices_total +=
                mesh.apply_brush(hit.position, hit.normal, Vec3::ZERO, &settings);
            // Refinement appends vertices while retaining old vertex indices.
            // Compare each pre-dab face using its original vertices after the dab.
            flip_events += step_flips(&mesh, &old, &old_triangles);
        } else {
            missed_rays += 1;
        }
    }
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let export_name = format!("{}_level{}_rounds{}", case.name, case.level, case.rounds);
    if let Some(directory) = exports {
        std::fs::write(
            directory.join(format!("sphere{}_before.obj", case.level)),
            initial.to_obj(),
        )
        .expect("write initial independent fixture");
        std::fs::write(directory.join(format!("{export_name}.obj")), mesh.to_obj())
            .expect("write sculpted independent fixture");
    }
    println!(
        "{}",
        serde_json::json!({
            "case":case.name,"sphere_level":case.level,"rounds":case.rounds,
            "vertices":mesh.positions.len(),"triangles":mesh.triangles.len(),
            "brush":"draw","radius":0.18,"strength":0.35,"symmetry_x":true,"invert":false,
            "path_kind":format!("{:?}",case.path),"ray_inputs":rays.iter().map(|ray|serde_json::json!({"origin":ray.origin.to_array(),"direction":ray.direction.to_array()})).collect::<Vec<_>>(),
            "requested_dabs":rays.len(),"missed_rays":missed_rays,"changed_vertices_total":changed_vertices_total,
            "per_dab_flip_events":flip_events,"initial_edge_median_world":reference_edge_median,
            "initial_metrics":metrics(&initial,&initial,&topology,reference_edge_median),
            "final_metrics":metrics(&mesh,&initial,&Topology::new(&mesh),reference_edge_median),"valid_structure":mesh.validate().is_ok(),
            "cpu_elapsed_ms":elapsed_ms,"export_obj_basename":format!("{export_name}.obj"),
            "notes":"Every dab live-picks the current mesh. Final metrics rebuild actual current adjacency. Appended vertices count as edited and use unit-sphere radius for height; original vertex displacements remain comparable. Per-dab flips inspect pre-dab faces using their preserved original vertex indices after the dab, including when refinement subdivides those faces. Angular/roughness diagnostics do not prove general self-intersection or reference quality. CPU timing includes full per-dab diagnostic scans and snapshots, excludes final metrics/export/rendering/UI."
        })
    );
}

fn main() {
    let exports = std::env::var_os("OBRUSH_DRAW_PROBE_EXPORT_DIR").map(PathBuf::from);
    if let Some(directory) = &exports {
        std::fs::create_dir_all(directory).expect("create independent fixture export directory");
    }
    for (name, path, rounds) in [
        ("line_control", Path::Line, 1),
        ("line_overlap", Path::Line, 4),
        ("crossing", Path::Crossing, 2),
        ("crossing", Path::Crossing, 4),
        ("crossing", Path::Crossing, 8),
        ("loops", Path::Loops, 4),
        ("loops", Path::Loops, 8),
        ("silhouette_tilted", Path::Silhouette, 4),
        ("silhouette_tilted", Path::Silhouette, 8),
    ] {
        run(
            Case {
                name,
                level: 6,
                rounds,
                path,
            },
            exports.as_ref(),
        );
    }
    run(
        Case {
            name: "crossing_finer",
            level: 7,
            rounds: 4,
            path: Path::Crossing,
        },
        exports.as_ref(),
    );
    for rounds in [64, 128] {
        run(
            Case {
                name: "repeated_point_tip",
                level: 6,
                rounds,
                path: Path::RepeatPoint,
            },
            exports.as_ref(),
        );
    }
}
