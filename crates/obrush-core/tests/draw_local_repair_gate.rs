//! Independent candidate gate for local Draw refinement on authored geometry.
//! These fixtures describe OBrush behavior, not ZBrush or SculptGL parity.
//! Run explicitly with `cargo test --test draw_local_repair_gate -- --ignored --nocapture`;
//! remove `ignore` only after both fixtures pass unchanged on the integrated candidate.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};
use std::collections::HashMap;

fn patch(sliver: bool) -> Mesh {
    const SIDE: usize = 21;
    let mut obj = String::new();
    for j in 0..SIDE {
        for i in 0..SIDE {
            let mut x = (i as f32 - 10.0) * 0.1;
            let y = (j as f32 - 10.0) * 0.1;
            if sliver && i == 10 && j == 10 {
                // A near-coincident, independently authored interior edge.
                x = -0.082;
            }
            let z = 0.18 * (1.0 - 0.65 * (x * x + y * y));
            obj.push_str(&format!("v {x:.6} {y:.6} {z:.6}\n"));
        }
    }
    for j in 0..SIDE - 1 {
        for i in 0..SIDE - 1 {
            let a = j * SIDE + i + 1;
            let b = a + 1;
            let c = a + SIDE;
            let d = c + 1;
            obj.push_str(&format!("f {a} {b} {d}\nf {a} {d} {c}\n"));
        }
    }
    let mut mesh = Mesh::from_obj(&obj).expect("authored patch parses");
    for (index, p) in mesh.positions.iter().enumerate() {
        mesh.colors[index] = [0.2 + 0.1 * p.x, 0.5 + 0.1 * p.y, 0.7];
        if p.x.abs() > 0.8 || p.y.abs() > 0.8 {
            mesh.masks[index] = 1.0;
        }
    }
    mesh.materials = Some(
        mesh.positions
            .iter()
            .map(|p| [0.3 + 0.1 * p.x, 0.6])
            .collect(),
    );
    mesh.validate().unwrap();
    mesh
}

fn topology_quality(mesh: &Mesh) -> (f64, f64) {
    mesh.validate().unwrap();
    let mut min_q = f64::INFINITY;
    let mut max_fold = 0.0_f64;
    let mut edges = HashMap::<(u32, u32), Vec<(bool, glam::DVec3)>>::new();
    let mut links = vec![HashMap::<u32, Vec<u32>>::new(); mesh.positions.len()];
    for &tri in &mesh.triangles {
        let [a, b, c] = tri.map(|i| mesh.positions[i as usize].as_dvec3());
        let cross = (b - a).cross(c - a);
        let edge2 = [
            a.distance_squared(b),
            b.distance_squared(c),
            c.distance_squared(a),
        ];
        assert!(
            cross.is_finite() && cross.length_squared() > 0.0,
            "zero-area face"
        );
        min_q = min_q.min(cross.length() / edge2.into_iter().fold(0.0_f64, f64::max));
        for (u, v) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            edges
                .entry((u.min(v), u.max(v)))
                .or_default()
                .push((u < v, cross.normalize()));
        }
        for (center, a, b) in [
            (tri[0], tri[1], tri[2]),
            (tri[1], tri[2], tri[0]),
            (tri[2], tri[0], tri[1]),
        ] {
            links[center as usize].entry(a).or_default().push(b);
            links[center as usize].entry(b).or_default().push(a);
        }
    }
    for incident in edges.values() {
        assert!(incident.len() <= 2, "non-manifold edge");
        if incident.len() == 2 {
            assert_ne!(incident[0].0, incident[1].0, "same-winding edge");
            max_fold = max_fold.max(
                incident[0]
                    .1
                    .dot(incident[1].1)
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees(),
            );
        }
    }
    for (vertex, link) in links.iter().enumerate() {
        let boundary_ends = link
            .values()
            .filter(|neighbors| neighbors.len() == 1)
            .count();
        assert!(
            link.values()
                .all(|neighbors| (1..=2).contains(&neighbors.len())),
            "non-manifold vertex link degree at {vertex}"
        );
        assert!(
            boundary_ends == 0 || boundary_ends == 2,
            "vertex {vertex} has a pinched or branching boundary"
        );
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![*link.keys().next().expect("every vertex is incident")];
        while let Some(neighbor) = stack.pop() {
            if seen.insert(neighbor) {
                stack.extend(&link[&neighbor]);
            }
        }
        assert_eq!(seen.len(), link.len(), "vertex {vertex} has disjoint fans");
    }
    (min_q, max_fold)
}

fn run_case(sliver: bool, radius: f32, contacts: usize) {
    let original = patch(sliver);
    let outside = original
        .positions
        .iter()
        .enumerate()
        .filter(|(_, p)| p.x.abs() > radius + 0.18 || p.y.abs() > radius + 0.18)
        .map(|(i, &p)| {
            (
                p,
                original.masks[i],
                original.colors[i],
                original.materials.as_ref().unwrap()[i],
            )
        })
        .collect::<Vec<_>>();
    let mut document = Document::new(original);
    let before = document.to_json().unwrap();
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        radius,
        strength: 0.35,
        symmetry_x: false,
        ..BrushSettings::default()
    };
    let mut lifts = Vec::new();
    let mut useful = 0;
    let mut no_op_run = 0;
    document.begin_stroke();
    for contact in 1..=contacts {
        let hit = document
            .active_mesh()
            .unwrap()
            .raycast(Ray {
                origin: Vec3::new(0.065, 0.035, 3.0),
                direction: -Vec3::Z,
            })
            .expect("interior hit");
        let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
            hit.position,
            hit.normal,
            Vec3::ZERO,
            &settings,
        );
        let after = document
            .active_mesh()
            .unwrap()
            .raycast(Ray {
                origin: Vec3::new(0.065, 0.035, 3.0),
                direction: -Vec3::Z,
            })
            .expect("live interior hit");
        let lift = after.position.z - hit.position.z;
        if result.changed_vertices > 0 && lift >= 0.002 * radius {
            useful += 1;
            no_op_run = 0;
        } else {
            no_op_run += 1;
        }
        lifts.push(lift);
        assert!(
            no_op_run <= 2,
            "sliver={sliver} radius={radius} stalled at contact {contact}: {result:?}, lift={lift}"
        );
        assert!(
            result.refined_vertices <= 1024,
            "unbounded local refinement"
        );
        topology_quality(document.active_mesh().unwrap());
    }
    document.end_stroke();
    let after = document.to_json().unwrap();
    let mesh = document.active_mesh().unwrap();
    for (p, mask, color, material) in outside {
        let id = mesh
            .positions
            .iter()
            .position(|&current| current == p)
            .expect("outside original vertex retained exactly");
        assert_eq!(mesh.masks[id], mask);
        assert_eq!(mesh.colors[id], color);
        assert_eq!(mesh.materials.as_ref().unwrap()[id], material);
    }
    let (quality, fold) = topology_quality(mesh);
    println!("sliver={sliver} radius={radius} useful={useful}/{contacts} lifts={lifts:?} verts={} faces={} min_q={quality:.7} max_fold={fold:.4}", mesh.positions.len(), mesh.triangles.len());
    assert!(
        useful >= if contacts == 12 { 8 } else { contacts * 4 / 5 },
        "Draw must remain useful through repeated contacts"
    );
    assert!(
        lifts.iter().sum::<f32>() >= 0.02 * radius,
        "cumulative surface lift must be substantial, not numerical motion"
    );
    assert_ne!(before, after);
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before, "exact one-step Undo");
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after, "exact one-step Redo");
}

#[test]
#[ignore = "candidate gate: run explicitly and preserve baseline failure"]
fn authored_coarse_curved_patch_remains_editable() {
    run_case(false, 1.0, 12);
}

#[test]
#[ignore = "candidate gate: run explicitly and preserve baseline failure"]
fn authored_interior_sliver_repairs_without_broad_shape_loss() {
    run_case(true, 0.18, 12);
}

#[test]
#[ignore = "stress gate: repeated local Draw must not settle into a safety-limit no-op"]
fn authored_sliver_remains_editable_after_many_contacts() {
    run_case(true, 0.18, 128);
}

#[test]
#[ignore = "independent 512-contact fresh sphere endurance gate"]
fn fresh_sphere_stationary_draw_still_builds_after_256_contacts() {
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
    let mut z256 = 0.0;
    let mut first_no_change = None;
    let mut no_change = 0;
    let start = std::time::Instant::now();
    for dab in 1..=512 {
        let hit = mesh.raycast(ray).expect("sphere remains pickable");
        let result = mesh.apply_brush_with_result(hit.position, hit.normal, Vec3::ZERO, &settings);
        if result.changed_vertices == 0 {
            no_change += 1;
            first_no_change.get_or_insert(dab);
        }
        if dab == 256 {
            z256 = mesh.raycast(ray).unwrap().position.z;
        }
    }
    let z512 = mesh.raycast(ray).unwrap().position.z;
    let (min_q, max_fold) = topology_quality(&mesh);
    eprintln!("fresh sphere 512 z256={z256:.8} z512={z512:.8} growth={:.8} first_no_change={first_no_change:?} no_change={no_change} q={min_q:.8} fold={max_fold:.5} verts={} tris={} elapsed_ms={}",
        z512-z256,mesh.positions.len(),mesh.triangles.len(),start.elapsed().as_millis());
    assert!(
        z512 - z256 >= 0.05,
        "Draw must keep raising the same sphere point after 256 contacts"
    );
    assert!(
        max_fold <= 60.001,
        "Draw must retain the 60-degree fold limit"
    );
}
