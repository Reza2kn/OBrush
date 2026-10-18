//! Sustained native core workload; input duration is separate from CPU runtime.
//! OBRUSH_LONG_PROJECT loads a private versioned project, otherwise sphere6.
//! OBRUSH_LONG_EXPORT_DIR writes measured final meshes outside timed sections.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};
use std::{path::PathBuf, time::Instant};

fn summary(mut values: Vec<f64>) -> serde_json::Value {
    if values.is_empty() {
        return serde_json::json!({"samples":0,"mean_ms":null,"p50_ms":null,"p95_ms":null,"max_ms":null});
    }
    values.sort_unstable_by(f64::total_cmp);
    serde_json::json!({"samples":values.len(),"mean_ms":values.iter().sum::<f64>()/values.len() as f64,
        "p50_ms":values[values.len()/2],"p95_ms":values[(values.len()*95/100).min(values.len()-1)],"max_ms":values.last()})
}

fn main() {
    let source = if let Some(path) = std::env::var_os("OBRUSH_LONG_PROJECT") {
        Document::from_json(&std::fs::read_to_string(path).expect("read private input"))
            .expect("validate private project")
            .mesh
            .expect("Long-stroke probe requires a nonempty active mesh")
    } else {
        Mesh::sphere(6)
    };
    let exports = std::env::var_os("OBRUSH_LONG_EXPORT_DIR").map(PathBuf::from);
    if let Some(directory) = &exports {
        std::fs::create_dir_all(directory).expect("create private fixture output");
    }
    let inputs: Vec<_> = (0..1200)
        .map(|i| {
            let phase = std::f32::consts::TAU * i as f32 / 180.0;
            Ray {
                origin: Vec3::new(0.28 * phase.cos(), 0.1 + 0.27 * phase.sin(), 4.0),
                direction: -Vec3::Z,
            }
        })
        .collect();
    for kind in [BrushKind::Draw, BrushKind::Crease] {
        let settings = BrushSettings {
            kind,
            radius: 0.18,
            strength: 0.35,
            symmetry_x: true,
            invert: false,
            ..Default::default()
        };
        let mut mesh = source.snapshot();
        let mut picks = Vec::new();
        let mut dabs = Vec::new();
        let mut total = Vec::new();
        let mut windows = Vec::new();
        let mut missed = 0;
        let mut refined = 0;
        let mut changed = 0;
        let started = Instant::now();
        for (i, &ray) in inputs.iter().enumerate() {
            let start = Instant::now();
            let hit = mesh.raycast(ray);
            let pick_ms = start.elapsed().as_secs_f64() * 1000.0;
            picks.push(pick_ms);
            if let Some(hit) = hit {
                let start = Instant::now();
                let result =
                    mesh.apply_brush_with_result(hit.position, hit.normal, Vec3::ZERO, &settings);
                let dab_ms = start.elapsed().as_secs_f64() * 1000.0;
                dabs.push(dab_ms);
                total.push(pick_ms + dab_ms);
                refined += result.refined_vertices;
                changed += result.changed_vertices;
            } else {
                missed += 1;
            }
            if (i + 1) % 300 == 0 {
                let recent = &total[total.len().saturating_sub(300)..];
                windows.push(serde_json::json!({"nominal_input_seconds":(i+1) as f64/60.0,
                    "vertices":mesh.positions.len(),"triangles":mesh.triangles.len(),"combined":summary(recent.to_vec())}));
            }
        }
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        if let Some(directory) = &exports {
            std::fs::write(
                directory.join(format!("{}.mesh.json", kind.name())),
                serde_json::to_vec(&mesh).expect("serialize measured private mesh"),
            )
            .expect("export private mesh outside timing");
        }
        println!(
            "{}",
            serde_json::json!({"brush":kind.name(),"radius":0.18,"strength":0.35,"symmetry_x":true,"invert":false,
            "nominal_input_seconds":20,"input_samples_per_second":60,"requested_dabs":inputs.len(),"missed_rays":missed,
            "initial_vertices":source.positions.len(),"initial_triangles":source.triangles.len(),"vertices":mesh.positions.len(),"triangles":mesh.triangles.len(),
            "refined_vertices_total":refined,"changed_vertices_total":changed,"pick":summary(picks),"sculpt":summary(dabs),"combined":summary(total),"windows":windows,"cpu_wall_ms":elapsed_ms,"valid_structure":mesh.validate().is_ok(),
            "ray_inputs":inputs.iter().map(|r|serde_json::json!({"origin":r.origin.to_array(),"direction":r.direction.to_array()})).collect::<Vec<_>>(),
            "scope":"Native CPU raycast+core sculpt/refinement, no history/UI/render/export inside timing; 20s nominal input duration is distinct from measured compute runtime, not browser FPS."})
        );
    }
}
