//! Representative same-input CPU timings, excluding app frames/history/rendering.
//! Run `cargo run --locked -p obrush-core --release --example core_performance_probe`.
//! Optional OBRUSH_PERF_EXPORT_DIR exports final OBJ shapes outside timed sections.
use obrush_core::{BrushKind, BrushSettings, Mesh, Ray, Vec3};
use std::path::PathBuf;
use std::time::Instant;

fn summary(mut values: Vec<f64>) -> serde_json::Value {
    if values.is_empty() {
        return serde_json::json!({"samples":0,"mean_ms":null,"p50_ms":null,"p95_ms":null,"max_ms":null});
    }
    values.sort_unstable_by(f64::total_cmp);
    serde_json::json!({
        "samples":values.len(),
        "mean_ms":values.iter().sum::<f64>()/values.len().max(1) as f64,
        "p50_ms":values[values.len()/2],
        "p95_ms":values[(values.len()*95/100).min(values.len()-1)],
        "max_ms":values.last().unwrap()
    })
}

fn rays(kind: &str, rounds: usize) -> Vec<Ray> {
    let ray = |x, y| Ray {
        origin: Vec3::new(x, y, 3.0),
        direction: -Vec3::Z,
    };
    let mut result = Vec::new();
    for round in 0..rounds {
        match kind {
            "point" => result.push(ray(0.23, 0.09)),
            "loops" => {
                for i in 0..=64 {
                    let sign = if round % 2 == 0 { 1.0 } else { -1.0 };
                    let phase = sign * std::f32::consts::TAU * i as f32 / 64.0;
                    result.push(ray(0.21 + 0.19 * phase.cos(), 0.06 + 0.19 * phase.sin()));
                }
            }
            "line" | "crossing" => {
                let lines = if kind == "crossing" { 2 } else { 1 };
                for line in 0..lines {
                    for i in 0..=40 {
                        let i = if round % 2 == 0 { i } else { 40 - i };
                        let s = -0.4 + 0.8 * i as f32 / 40.0;
                        result.push(if line == 1 {
                            ray(0.09, s)
                        } else {
                            ray(s, 0.09)
                        });
                    }
                }
            }
            _ => unreachable!("named deterministic fixture"),
        }
    }
    result
}

fn main() {
    let exports = std::env::var_os("OBRUSH_PERF_EXPORT_DIR").map(PathBuf::from);
    if let Some(directory) = &exports {
        std::fs::create_dir_all(directory).expect("create independent fixture directory");
    }
    for (name, kind, rounds) in [
        ("line_overlap", "line", 4),
        ("crossing", "crossing", 4),
        ("loops", "loops", 8),
        ("repeated_point_tip", "point", 128),
    ] {
        let mut mesh = Mesh::sphere(6);
        let settings = BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.18,
            strength: 0.35,
            symmetry_x: true,
            invert: false,
            ..BrushSettings::default()
        };
        let inputs = rays(kind, rounds);
        let mut pick = Vec::new();
        let mut sculpt = Vec::new();
        let mut total = Vec::new();
        let mut refined = 0;
        let mut limited = 0;
        let mut missed = 0;
        let mut changed = 0;
        for &ray in &inputs {
            let start = Instant::now();
            let hit = mesh.raycast(ray);
            let picked = start.elapsed().as_secs_f64() * 1000.0;
            pick.push(picked);
            if let Some(hit) = hit {
                let start = Instant::now();
                let result =
                    mesh.apply_brush_with_result(hit.position, hit.normal, Vec3::ZERO, &settings);
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                sculpt.push(elapsed);
                total.push(picked + elapsed);
                refined += result.refined_vertices;
                limited += usize::from(result.detail_limited);
                changed += result.changed_vertices;
            } else {
                missed += 1;
            }
        }
        if let Some(directory) = &exports {
            std::fs::write(
                directory.join(format!("{name}_level6_rounds{rounds}.obj")),
                mesh.to_obj(),
            )
            .expect("write independent result outside timing");
            std::fs::write(
                directory.join(format!("{name}_level6_rounds{rounds}.mesh.json")),
                serde_json::to_vec(&mesh).expect("serialize measured mesh fields"),
            )
            .expect("write measured mesh fields outside timing");
        }
        println!(
            "{}",
            serde_json::json!({
                "case":name,"rounds":rounds,"sphere_level":6,"radius":0.18,"strength":0.35,"symmetry_x":true,"invert":false,
                "requested_dabs":inputs.len(),"ray_inputs":inputs.iter().map(|r|serde_json::json!({"origin":r.origin.to_array(),"direction":r.direction.to_array()})).collect::<Vec<_>>(),
                "pick":summary(pick),"sculpt":summary(sculpt),"combined":summary(total),
                "vertices":mesh.positions.len(),"triangles":mesh.triangles.len(),"refined_vertices_total":refined,"detail_limited_dabs":limited,"missed_rays":missed,"changed_vertices_total":changed,"valid_structure":mesh.validate().is_ok(),
                "scope":"Native CPU only. Individual current-surface raycast and core Draw/refinement calls, no external snapshots/diagnostic scans/history/UI/GPU or browser frame-rate claim."
            })
        );
    }
}
