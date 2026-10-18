//! Smooth raised strokes need quality checks beyond finite coordinates and
//! unchanged winding. Fixtures here are independent of the reference product.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Ray, Vec3};
use std::collections::{HashMap, HashSet};

fn maximum_normal_jump(mesh: &Mesh) -> f64 {
    let mut edges = HashMap::new();
    let mut maximum = 0.0_f64;
    for &[a, b, c] in &mesh.triangles {
        let [p, q, r] = [a, b, c].map(|vertex| mesh.positions[vertex as usize].as_dvec3());
        let normal = (q - p).cross(r - p).normalize_or_zero();
        for (a, b) in [(a, b), (b, c), (c, a)] {
            if let Some(other) = edges.insert((a.min(b), a.max(b)), normal) {
                maximum = maximum.max(normal.dot(other).clamp(-1.0, 1.0).acos().to_degrees());
            }
        }
    }
    maximum
}

#[test]
fn local_linear_face_splitting_preserves_existing_corner_normals() {
    // Two perpendicular faces share this corner. Halving just one face's area
    // must not change that corner's shading when its angle stays the same.
    let mut mesh =
        Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nv 0 -1 0\nf 1 2 3\nf 1 4 5\n").unwrap();
    let old_normals = mesh.normals.clone();
    mesh.positions.push(Vec3::new(0.5, 0.0, 0.0));
    mesh.masks.push(0.0);
    mesh.colors.push(mesh.colors[0]);
    mesh.triangles = vec![[0, 5, 2], [5, 1, 2], [0, 3, 4]];
    mesh.recompute_normals();
    mesh.validate().unwrap();
    for (new, old) in mesh.normals.iter().zip(old_normals) {
        assert!(new.distance(old) < 1e-6);
    }
    assert!(mesh.normals[0].distance((Vec3::X + Vec3::Z).normalize()) < 1e-6);
}

#[test]
fn overlapping_draw_strokes_raise_a_smooth_refined_surface_with_one_undo_step() {
    let original = Mesh::sphere(4);
    let mut document = Document::new(original.snapshot());
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        radius: 0.18,
        strength: 0.35,
        symmetry_x: true,
        ..Default::default()
    };
    let mut additions = 0;
    document.begin_stroke();
    for pass in 0..4 {
        for step in 0..29 {
            let index = if pass % 2 == 0 { step } else { 28 - step };
            let hit = document
                .active_mesh()
                .unwrap()
                .raycast(Ray {
                    origin: Vec3::new(-0.35 + 0.025 * index as f32, 0.04, 3.0),
                    direction: -Vec3::Z,
                })
                .unwrap();
            let result = document.active_mesh_mut().unwrap().apply_brush_with_result(
                hit.position,
                hit.normal,
                Vec3::ZERO,
                &settings,
            );
            additions += result.refined_vertices;
        }
    }
    document.end_stroke();
    document.active_mesh().unwrap().validate().unwrap();
    assert!(
        additions > 0,
        "stretched/coarse patches need editable detail"
    );
    assert!(maximum_normal_jump(document.active_mesh().unwrap()) < 60.001);
    let height = document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .map(|point| point.length() - 1.0)
        .fold(0.0_f32, f32::max);
    assert!(
        height > 0.08,
        "Draw must still build useful volume: {height}"
    );
    assert!(document
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .all(|point| point.is_finite()));
    let edited = document.active_mesh().unwrap().snapshot();
    assert!(document.undo());
    assert_eq!(
        document.active_mesh().unwrap().positions,
        original.positions
    );
    assert_eq!(
        document.active_mesh().unwrap().triangles,
        original.triangles
    );
    assert_eq!(document.active_mesh().unwrap().masks, original.masks);
    assert_eq!(document.active_mesh().unwrap().colors, original.colors);
    assert!(!document.can_undo());
    assert!(document.redo());
    assert_eq!(document.active_mesh().unwrap().positions, edited.positions);
    assert_eq!(document.active_mesh().unwrap().triangles, edited.triangles);
}

#[test]
fn masked_or_zero_strength_draw_does_not_add_topology_or_history() {
    let mut mesh = Mesh::sphere(2);
    mesh.masks.fill(1.0);
    let original = mesh.snapshot();
    let mut document = Document::new(mesh);
    document.begin_stroke();
    let masked = document.active_mesh_mut().unwrap().apply_brush_with_result(
        Vec3::Z,
        Vec3::Z,
        Vec3::ZERO,
        &BrushSettings::default(),
    );
    document.end_stroke();
    assert_eq!(masked.changed_vertices, 0);
    assert_eq!(masked.refined_vertices, 0);
    assert_eq!(
        document.active_mesh().unwrap().positions,
        original.positions
    );
    assert_eq!(
        document.active_mesh().unwrap().triangles,
        original.triangles
    );
    assert!(!document.can_undo());
    document.active_mesh_mut().unwrap().masks.fill(0.0);
    let idle = document.active_mesh_mut().unwrap().apply_brush_with_result(
        Vec3::Z,
        Vec3::Z,
        Vec3::ZERO,
        &BrushSettings {
            strength: 0.0,
            ..Default::default()
        },
    );
    assert_eq!(idle.changed_vertices, 0);
    assert_eq!(idle.refined_vertices, 0);
    assert_eq!(
        document.active_mesh().unwrap().triangles,
        original.triangles
    );
}

#[test]
fn repeated_draw_refinement_keeps_originally_exact_mirror_pairs_exact() {
    let mut mesh = Mesh::sphere(3);
    let settings = BrushSettings {
        radius: 0.18,
        strength: 0.35,
        symmetry_x: true,
        ..Default::default()
    };
    for _ in 0..12 {
        let hit = mesh
            .raycast(Ray {
                origin: Vec3::new(0.20, 0.07, 3.0),
                direction: -Vec3::Z,
            })
            .unwrap();
        mesh.apply_brush(hit.position, hit.normal, Vec3::ZERO, &settings);
        let key = |p: Vec3| p.to_array().map(|v| if v == 0.0 { 0 } else { v.to_bits() });
        let positions: HashSet<_> = mesh.positions.iter().map(|&p| key(p)).collect();
        for &position in &mesh.positions {
            assert!(positions.contains(&key(Vec3::new(-position.x, position.y, position.z))));
        }
    }
    assert!(mesh.positions.len() > Mesh::sphere(3).positions.len());
}
