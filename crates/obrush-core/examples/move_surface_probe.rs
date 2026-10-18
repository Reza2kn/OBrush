//! Independent small-radius Move regression and scoped native timings.
use obrush_core::{BrushKind, BrushSettings, Mesh, Ray, Vec3};
use std::collections::BTreeMap;
use std::time::Instant;

fn metrics(mesh: &Mesh) -> serde_json::Value {
    let mut minimum_quality = f64::INFINITY;
    let mut maximum_edge: f64 = 0.0;
    let mut edges = BTreeMap::<(u32, u32), Vec<(u32, glam::DVec3)>>::new();
    for (face, &triangle) in mesh.triangles.iter().enumerate() {
        let [a, b, c] = triangle.map(|v| mesh.positions[v as usize].as_dvec3());
        let cross = (b - a).cross(c - a);
        let longest = a
            .distance_squared(b)
            .max(b.distance_squared(c))
            .max(c.distance_squared(a));
        minimum_quality = minimum_quality.min(cross.length() / longest);
        maximum_edge = maximum_edge.max(longest.sqrt());
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            edges
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push((face as u32, cross.normalize_or_zero()));
        }
    }
    let mut maximum_fold: f64 = 0.0;
    let mut invalid_edges = 0;
    for faces in edges.values() {
        if faces.len() != 2 {
            invalid_edges += 1;
        }
        if faces.len() == 2 {
            maximum_fold = maximum_fold.max(
                faces[0]
                    .1
                    .dot(faces[1].1)
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees(),
            );
        }
    }
    let maximum_radius = mesh
        .positions
        .iter()
        .map(|p| p.as_dvec3().length())
        .fold(0.0, f64::max);
    serde_json::json!({"vertices":mesh.positions.len(), "triangles":mesh.triangles.len(), "valid":mesh.validate().is_ok(), "minimum_area_over_longest_edge_squared":minimum_quality, "maximum_edge":maximum_edge, "maximum_adjacent_fold_degrees":maximum_fold, "non_two_incidence_edges":invalid_edges,"maximum_distance_from_original_sphere_origin":maximum_radius})
}

fn main() {
    for (name, level, center_x, radius, delta) in [
        (
            "small_long_tangent",
            5,
            0.3,
            0.037,
            Vec3::new(0.0, 0.6, 0.0),
        ),
        ("small_long_normal", 5, 0.3, 0.037, Vec3::new(0.0, 0.0, 0.6)),
        ("seam_overlap", 5, 0.01, 0.037, Vec3::new(0.2, 0.1, 0.2)),
        ("dense_long_normal", 6, 0.3, 0.037, Vec3::new(0.0, 0.0, 0.6)),
        (
            "small_proportional_pull",
            6,
            0.3,
            0.037,
            Vec3::new(0.0, 0.0, 0.03),
        ),
        ("broad_long_normal", 6, 0.3, 0.18, Vec3::new(0.0, 0.0, 0.6)),
    ] {
        let mut mesh = Mesh::sphere(level);
        let original = mesh.clone();
        let hit = mesh
            .raycast(Ray {
                origin: Vec3::new(center_x, 0.0, 3.0),
                direction: -Vec3::Z,
            })
            .unwrap();
        let settings = BrushSettings {
            kind: BrushKind::Move,
            radius,
            strength: 0.79,
            symmetry_x: true,
            ..BrushSettings::default()
        };
        let start = Instant::now();
        let mut moving = mesh
            .prepare_move_stroke(hit.position, hit.normal, &settings)
            .unwrap();
        let preparation_ms = start.elapsed().as_secs_f64() * 1000.0;
        let mut times = Vec::new();
        let mut accepted = 0;
        let mut refined = 0;
        let mut last_scale = 0.0;
        for sample in 1..=240 {
            let start = Instant::now();
            let result = moving.apply(&mut mesh, delta * (sample as f32 / 240.0), &settings);
            times.push(start.elapsed().as_secs_f64() * 1000.0);
            accepted += usize::from(result.changed_vertices != 0);
            refined += result.refined_vertices;
            last_scale = result.displacement_scale;
        }
        let maximum_original_displacement = original
            .positions
            .iter()
            .zip(&mesh.positions)
            .map(|(a, b)| f64::from(a.distance(*b)))
            .fold(0.0, f64::max);
        times.sort_unstable_by(f64::total_cmp);
        println!(
            "{}",
            serde_json::json!({"case":name,"sphere_level":level,"radius":settings.radius,"strength":settings.strength,"symmetry_x":true,"total_delta":delta.to_array(),"input_samples":240,"accepted_samples":accepted,"maximum_original_displacement":maximum_original_displacement,"refined_vertices":refined,"last_displacement_scale":last_scale,"preparation_ms":preparation_ms,"metrics":metrics(&mesh),"apply_ms":{"p50":times[times.len()/2],"p95":times[times.len()*95/100],"maximum":times[times.len()-1]},"scope":"Native CPU core Move calls only; preparation reported separately, diagnostics outside timers; not browser/input-to-display timing or ZBrush parity."})
        );
    }
}
