//! CPU-only stress fixtures for brush stability, not ZBrush parity or frame timings.
//!
//! Run `cargo run --locked -p obrush-core --release --example crease_stability_probe`.
//! Output is JSON Lines. A reference-normal reversal can also be legitimate large
//! deformation; it is a diagnostic, not a general self-intersection detector.
use glam::DVec3;
use obrush_core::{BrushKind, BrushSettings, Mesh, Ray, Vec3};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Serialize)]
struct Metrics {
    nonfinite_vertices: usize,
    near_zero_area_faces: usize,
    low_quality_faces: usize,
    reference_normal_reversed_faces: usize,
    adjacent_normal_angle_over_160_edges: usize,
    min_area_ratio: f64,
    min_edge_ratio: f64,
    min_triangle_quality: f64,
    max_vertex_displacement: f64,
    minimum_radial_change: f64,
    maximum_radial_change: f64,
    maximum_inward_displacement: f64,
}

fn precise(v: Vec3) -> DVec3 {
    DVec3::new(v.x as f64, v.y as f64, v.z as f64)
}

fn metrics(mesh: &Mesh, initial: &Mesh) -> Metrics {
    let mut result = Metrics {
        nonfinite_vertices: mesh.positions.iter().filter(|p| !p.is_finite()).count(),
        near_zero_area_faces: 0,
        low_quality_faces: 0,
        reference_normal_reversed_faces: 0,
        adjacent_normal_angle_over_160_edges: 0,
        min_area_ratio: f64::INFINITY,
        min_edge_ratio: f64::INFINITY,
        min_triangle_quality: f64::INFINITY,
        max_vertex_displacement: 0.0,
        minimum_radial_change: 0.0,
        maximum_radial_change: 0.0,
        maximum_inward_displacement: 0.0,
    };
    let mut edges = HashMap::<(u32, u32), DVec3>::new();
    for (p, original) in mesh.positions.iter().zip(&initial.positions) {
        let p = precise(*p);
        let original = precise(*original);
        result.max_vertex_displacement = result.max_vertex_displacement.max(p.distance(original));
        let radial_change = p.length() - original.length();
        result.minimum_radial_change = result.minimum_radial_change.min(radial_change);
        result.maximum_radial_change = result.maximum_radial_change.max(radial_change);
        result.maximum_inward_displacement = result
            .maximum_inward_displacement
            .max(-(p - original).dot(original.normalize_or_zero()));
    }
    for &[a, b, c] in &mesh.triangles {
        let p = [a, b, c].map(|i| precise(mesh.positions[i as usize]));
        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
        let sum_edge_squared =
            p[0].distance_squared(p[1]) + p[1].distance_squared(p[2]) + p[2].distance_squared(p[0]);
        let quality = if sum_edge_squared > 0.0 {
            2.0 * 3.0_f64.sqrt() * cross.length() / sum_edge_squared
        } else {
            0.0
        };
        result.min_triangle_quality = result.min_triangle_quality.min(quality);
        result.near_zero_area_faces += usize::from(cross.length_squared() <= 1e-24);
        result.low_quality_faces += usize::from(quality <= 1e-3);
        let normal = cross.normalize_or_zero();
        for (i, j) in [(0, 1), (1, 2), (2, 0)] {
            let vertices = [a, b, c];
            let key = (vertices[i].min(vertices[j]), vertices[i].max(vertices[j]));
            if let Some(other) = edges.get(&key) {
                result.adjacent_normal_angle_over_160_edges +=
                    usize::from(normal.dot(*other) < -0.939_692_620_785_908_4);
            } else {
                edges.insert(key, normal);
            }
        }
    }
    // Original parent triangles remain indexable after conforming refinement.
    // Their reference ratios do not pretend added vertices have rest positions.
    for &triangle in &initial.triangles {
        let p = triangle.map(|i| precise(mesh.positions[i as usize]));
        let original = triangle.map(|i| precise(initial.positions[i as usize]));
        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
        let original_cross = (original[1] - original[0]).cross(original[2] - original[0]);
        result.min_area_ratio = result
            .min_area_ratio
            .min(cross.length() / original_cross.length());
        result.reference_normal_reversed_faces += usize::from(cross.dot(original_cross) <= 0.0);
        for (i, j) in [(0, 1), (1, 2), (2, 0)] {
            result.min_edge_ratio = result
                .min_edge_ratio
                .min(p[i].distance(p[j]) / original[i].distance(original[j]));
        }
    }
    result
}

/// Per-dab comparison avoids treating a gradually rotated surface as a flip.
fn step_failures(mesh: &Mesh, previous: &[Vec3], previous_triangles: &[[u32; 3]]) -> (usize, f64) {
    let mut flipped = 0;
    let mut min_area_ratio = f64::INFINITY;
    for &[a, b, c] in previous_triangles {
        let p = [a, b, c].map(|i| precise(mesh.positions[i as usize]));
        let old = [a, b, c].map(|i| precise(previous[i as usize]));
        let cross = (p[1] - p[0]).cross(p[2] - p[0]);
        let old_cross = (old[1] - old[0]).cross(old[2] - old[0]);
        if old_cross.length_squared() > 0.0 {
            flipped += usize::from(cross.dot(old_cross) <= 0.0);
            min_area_ratio = min_area_ratio.min(cross.length() / old_cross.length());
        }
    }
    (flipped, min_area_ratio)
}

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    level: u32,
    radius: f32,
    strength: f32,
    symmetry: bool,
    invert: bool,
    passes: usize,
    stationary: bool,
    continuous_turnaround: bool,
    move_delta: Option<Vec3>,
}

fn run(case: Case) {
    let initial = Mesh::sphere(case.level);
    let mut mesh = initial.clone();
    let settings = BrushSettings {
        kind: BrushKind::Crease,
        radius: case.radius,
        strength: case.strength,
        symmetry_x: case.symmetry,
        invert: case.invert,
        ..BrushSettings::default()
    };
    let mut move_metrics = None;
    if let Some(delta) = case.move_delta {
        let hit = mesh
            .raycast(Ray {
                origin: Vec3::new(-0.23, 0.09, 3.0),
                direction: -Vec3::Z,
            })
            .expect("independent fixture starts under ray");
        let move_settings = BrushSettings {
            kind: BrushKind::Move,
            invert: false,
            ..settings.clone()
        };
        mesh.apply_brush(hit.position, hit.normal, delta, &move_settings);
        move_metrics = Some(metrics(&mesh, &initial));
    }
    let dabs_per_pass = if case.stationary { 1 } else { 41 };
    let start = Instant::now();
    let mut missed_rays = 0;
    let mut changed_vertices_total = 0;
    let mut applied_dabs = 0;
    let mut step_flip_events = 0;
    let mut first_step_flip_dab = None;
    let mut min_step_area_ratio = f64::INFINITY;
    for pass in 0..case.passes {
        for step in 0..dabs_per_pass {
            if case.continuous_turnaround && pass > 0 && step == 0 {
                continue;
            }
            let x = if case.stationary {
                0.23
            } else {
                let step = if pass % 2 == 0 { step } else { 40 - step };
                -0.4 + 0.8 * step as f32 / 40.0
            };
            if let Some(hit) = mesh.raycast(Ray {
                origin: Vec3::new(x, 0.09, 3.0),
                direction: -Vec3::Z,
            }) {
                let previous = mesh.positions.clone();
                let previous_triangles = mesh.triangles.clone();
                changed_vertices_total +=
                    mesh.apply_brush(hit.position, hit.normal, Vec3::ZERO, &settings);
                applied_dabs += 1;
                let (flips, area_ratio) = step_failures(&mesh, &previous, &previous_triangles);
                step_flip_events += flips;
                min_step_area_ratio = min_step_area_ratio.min(area_ratio);
                if flips > 0 && first_step_flip_dab.is_none() {
                    first_step_flip_dab = Some(applied_dabs);
                }
            } else {
                missed_rays += 1;
            }
        }
    }
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{}",
        serde_json::json!({
            "case": case.name,
            "reference_basis": "original-parent displacement/ratios; pre-dab-parent reversals; final-child quality/folds",
            "sphere_level": case.level,
            "vertices": mesh.positions.len(),
            "triangles": mesh.triangles.len(),
            "brush": "crease",
            "radius": case.radius,
            "strength": case.strength,
            "symmetry_x": case.symmetry,
            "invert": case.invert,
            "passes": case.passes,
            "dabs_per_pass": dabs_per_pass,
            "crease_dabs": applied_dabs,
            "continuous_turnaround":case.continuous_turnaround,
            "path": if case.stationary { "each dab ray (0.23,0.09,3) toward -Z, re-pick current mesh" } else { "alternating direction 41-dab lines x=-0.4..0.4 step0.02,y0.09,z3 toward-Z; re-pick every dab" },
            "move": case.move_delta.map(|delta| serde_json::json!({"dabs":1,"ray":[-0.23,0.09,3.0],"delta":delta.to_array(),"radius":case.radius,"strength":case.strength,"symmetry_x":case.symmetry,"invert":false})),
            "after_move_metrics": move_metrics,
            "final_metrics": metrics(&mesh,&initial),
            "missed_rays": missed_rays,
            "changed_vertices_total": changed_vertices_total,
            "per_dab_flip_events":step_flip_events,
            "first_per_dab_flip":first_step_flip_dab,
            "minimum_per_dab_area_ratio":min_step_area_ratio,
            "cpu_crease_elapsed_ms": elapsed_ms,
            "valid_structure": mesh.validate().is_ok(),
            "notes":"Final area/edge ratios use initial sphere triangle/edge. Per-dab flips compare to positions immediately before that dab. Quality=4sqrt(3)*area/sum(edge_length_squared), equilateral=1. Reversed reference normals and extreme adjacent angles diagnose deformation, not general self-intersection. CPU time includes per-dab diagnostic scans, excludes rendering/UI/final metric scan."
        })
    );
}

fn main() {
    let default = Case {
        name: "",
        level: 6,
        radius: 0.18,
        strength: 0.35,
        symmetry: true,
        invert: false,
        passes: 1,
        stationary: false,
        continuous_turnaround: false,
        move_delta: None,
    };
    for passes in [1, 2, 4, 16] {
        run(Case {
            name: "default_repeated_line",
            passes,
            ..default
        });
    }
    run(Case {
        name: "default_symmetry_off",
        symmetry: false,
        passes: 4,
        ..default
    });
    run(Case {
        name: "default_continuous_turnaround",
        continuous_turnaround: true,
        passes: 4,
        ..default
    });
    for level in [5, 7] {
        run(Case {
            name: "density_repeated_line",
            level,
            passes: 16,
            ..default
        });
    }
    for passes in [64, 256] {
        run(Case {
            name: "repeated_same_point",
            stationary: true,
            passes,
            ..default
        });
    }
    run(Case {
        name: "strong_repeated_line",
        strength: 1.0,
        passes: 16,
        ..default
    });
    run(Case {
        name: "inverted_repeated_line",
        invert: true,
        passes: 16,
        ..default
    });
    for delta in [Vec3::new(0.6, 0.0, 0.0), Vec3::new(1.2, 0.4, 0.0)] {
        run(Case {
            name: "move_then_crease",
            move_delta: Some(delta),
            passes: 4,
            ..default
        });
    }
}
