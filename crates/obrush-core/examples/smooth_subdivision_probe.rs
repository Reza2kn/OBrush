//! Local CPU measurements, not a ZBrush comparison or input-to-display benchmark.
//! Run with `cargo run --release -p obrush-core --example smooth_subdivision_probe`.
use obrush_core::{BrushKind, BrushSettings, Mesh, Ray, Vec3};
use std::time::Instant;

fn main() {
    for level in [6, 7] {
        let start = Instant::now();
        let mut mesh = Mesh::sphere(level);
        let generation_ms = start.elapsed().as_secs_f64() * 1_000.0;
        let vertices_before = mesh.positions.len();
        let triangles_before = mesh.triangles.len();
        let mut pick_ms = Vec::new();
        for sample in 0..20 {
            let start = Instant::now();
            let hit = mesh.raycast(Ray {
                origin: Vec3::new(sample as f32 * 0.01, 0.0, 3.0),
                direction: -Vec3::Z,
            });
            assert!(hit.is_some());
            pick_ms.push(start.elapsed().as_secs_f64() * 1_000.0);
        }
        pick_ms.sort_by(f64::total_cmp);
        let settings = BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.12,
            strength: 0.3,
            invert: false,
            symmetry_x: false,
            color: [0.9, 0.1, 0.2],
            ..Default::default()
        };
        let start = Instant::now();
        let changed_vertices = mesh.apply_brush(Vec3::Z, Vec3::Z, Vec3::ZERO, &settings);
        let brush_dab_ms = start.elapsed().as_secs_f64() * 1_000.0;
        let start = Instant::now();
        mesh.subdivide_smooth().unwrap();
        let smooth_refinement_ms = start.elapsed().as_secs_f64() * 1_000.0;
        println!(
            "{}",
            serde_json::json!({
                "sphere_level": level,
                "vertices_before": vertices_before,
                "triangles_before": triangles_before,
                "generation_ms": generation_ms,
                "pick_samples": 20,
                "pick_median_ms": (pick_ms[9] + pick_ms[10]) * 0.5,
                "pick_p95_ms": pick_ms[18],
                "draw_radius": settings.radius,
                "draw_strength": settings.strength,
                "changed_vertices": changed_vertices,
                "brush_dab_ms": brush_dab_ms,
                "smooth_refinement_ms": smooth_refinement_ms,
                "vertices_after": mesh.positions.len(),
                "triangles_after": mesh.triangles.len(),
                "limitation": "CPU-only local sample; excludes history, GPU uploads, rendering and browser execution"
            })
        );
    }
}
