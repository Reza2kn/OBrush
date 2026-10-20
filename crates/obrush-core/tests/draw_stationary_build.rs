//! Repeated independent Draw contacts must keep raising the picked surface.
//! The ray is fixed in screen space while each dab picks the current mesh.
use obrush_core::{BrushKind, BrushSettings, Mesh, Ray, Vec3};
use std::collections::HashMap;

fn surface_quality(mesh: &Mesh) -> (f64, f64) {
    let mut min_quality = f64::INFINITY;
    let mut edges = HashMap::new();
    let mut max_fold = 0.0_f64;
    for &[a, b, c] in &mesh.triangles {
        let [p, q, r] = [a, b, c].map(|id| mesh.positions[id as usize].as_dvec3());
        let cross = (q - p).cross(r - p);
        let area2 = cross.length();
        let edges2 = p.distance_squared(q) + q.distance_squared(r) + r.distance_squared(p);
        min_quality = min_quality.min(2.0 * 3.0_f64.sqrt() * area2 / edges2);
        let normal = cross.normalize_or_zero();
        for (u, v) in [(a, b), (b, c), (c, a)] {
            if let Some(other) = edges.insert((u.min(v), u.max(v)), normal) {
                max_fold = max_fold.max(normal.dot(other).clamp(-1.0, 1.0).acos().to_degrees());
            }
        }
    }
    (min_quality, max_fold)
}

#[test]
#[ignore = "known failure: stationary Draw saturates on the fresh-sphere fixture"]
fn repeated_stationary_draw_contacts_build_on_a_fresh_sphere() {
    let mut mesh = Mesh::sphere(6);
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        radius: 0.18,
        strength: 0.35,
        symmetry_x: true,
        ..BrushSettings::default()
    };
    let ray = Ray {
        origin: Vec3::new(0.23, 0.09, 3.0),
        direction: -Vec3::Z,
    };
    let start_z = mesh.raycast(ray).unwrap().position.z;
    let mut samples = vec![(0, start_z)];
    let mut no_change = 0;
    let mut min_scale = 1.0_f32;
    let mut first_no_change = None;
    let mut first_limited = None;
    let mut diagnostics = Vec::new();
    for dab in 1..=256 {
        let hit = mesh.raycast(ray).unwrap();
        let within_radius = mesh
            .positions
            .iter()
            .filter(|p| p.distance_squared(hit.position) < settings.radius * settings.radius)
            .count();
        let result = mesh.apply_brush_with_result(hit.position, hit.normal, Vec3::ZERO, &settings);
        no_change += usize::from(result.changed_vertices == 0);
        min_scale = min_scale.min(result.displacement_scale);
        if result.changed_vertices == 0 {
            first_no_change.get_or_insert(dab);
        }
        if result.displacement_scale < 0.5 {
            first_limited.get_or_insert(dab);
        }
        if [1, 16, 64, 128, 256].contains(&dab) {
            samples.push((dab, mesh.raycast(ray).unwrap().position.z));
            diagnostics.push((
                dab,
                within_radius,
                result.changed_vertices,
                result.displacement_scale,
                result.refined_vertices,
                result.detail_limited,
                hit.normal,
            ));
        }
    }
    let (min_quality, max_fold) = surface_quality(&mesh);
    eprintln!("stationary Draw z={samples:?} first_no_change={first_no_change:?} first_limited={first_limited:?} no_change={no_change} min_scale={min_scale} min_quality={min_quality} max_fold={max_fold} vertices={} triangles={} diagnostics={diagnostics:?}", mesh.positions.len(), mesh.triangles.len());
    mesh.validate().unwrap();
    assert!(min_quality >= 0.15);
    assert!(max_fold <= 60.001);
    assert!(
        samples[5].1 - samples[4].1 >= 0.05,
        "Draw stopped building at one cursor point"
    );
}
