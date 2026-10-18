//! Core-only target/guard fixtures; these do not establish reference parity.
use obrush_core::{BrushKind, BrushSettings, Document, Mesh, Vec3};
use std::collections::BTreeMap;

fn bits(mesh: &Mesh) -> Vec<[u32; 3]> {
    mesh.positions
        .iter()
        .map(|p| p.to_array().map(f32::to_bits))
        .collect()
}

#[test]
fn signed_zero_targets_swap_restore_and_serialize_exactly() {
    let mesh = Mesh::from_obj("v -0.0 0 0\nv 1 0 -0.0\nv 0 1 0\nf 1 2 3\n").unwrap();
    let mut document = Document::new(mesh);
    let original = bits(document.active_mesh().unwrap());
    document.store_morph_target().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].z = 0.125;
    document.active_mesh_mut().unwrap().positions[0].x = 0.0;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    let edited = bits(document.active_mesh().unwrap());
    let state = document.to_json().unwrap();
    document.swap_morph_target().unwrap();
    assert_eq!(bits(document.active_mesh().unwrap()), original);
    document.swap_morph_target().unwrap();
    assert_eq!(bits(document.active_mesh().unwrap()), edited);
    assert_eq!(document.to_json().unwrap(), state);
    document.restore_morph_target().unwrap();
    assert_eq!(bits(document.active_mesh().unwrap()), original);
    let saved = document.to_json().unwrap();
    assert_eq!(
        Document::from_json(&saved).unwrap().to_json().unwrap(),
        saved
    );
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), state);
}

#[test]
fn generic_mesh_morph_has_no_implicit_target_or_effect() {
    let mut mesh = Mesh::sphere(2);
    let before = serde_json::to_string(&mesh).unwrap();
    let settings = BrushSettings {
        kind: BrushKind::Morph,
        ..BrushSettings::default()
    };
    let result = mesh.apply_brush_with_result(Vec3::Z, Vec3::Z, Vec3::Z, &settings);
    assert_eq!(result.changed_vertices, 0);
    assert_eq!(serde_json::to_string(&mesh).unwrap(), before);
    assert_eq!(
        BrushKind::ALL,
        [
            BrushKind::Draw,
            BrushKind::Clay,
            BrushKind::Inflate,
            BrushKind::Smooth,
            BrushKind::Flatten,
            BrushKind::Crease,
            BrushKind::Move,
            BrushKind::Mask,
            BrushKind::Paint,
            BrushKind::Morph,
            BrushKind::Pinch,
            BrushKind::Twist,
            BrushKind::LocalScale,
            BrushKind::Drag,
        ]
    );
    assert_eq!(BrushKind::ALL[9], BrushKind::Morph);
}

#[test]
fn stored_target_history_is_preserved_by_idle_and_no_contact_dabs() {
    let mut document = Document::new(Mesh::sphere(1));
    document.store_morph_target().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0] *= 1.02;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.undo());
    let state = document.to_json().unwrap();
    assert!(document.can_redo());
    document.store_morph_target().unwrap();
    document.restore_morph_target().unwrap();
    document.swap_morph_target().unwrap();
    let settings = BrushSettings {
        kind: BrushKind::Morph,
        pressure: 0.0,
        ..BrushSettings::default()
    };
    let result = document
        .apply_morph_brush_with_result(Vec3::Z, Vec3::Z, &settings)
        .unwrap();
    assert_eq!(result.changed_vertices, 0);
    assert_eq!(document.to_json().unwrap(), state);
    assert!(document.can_redo());
}

#[test]
fn large_local_restore_uses_the_shared_quality_and_fold_guard() {
    let mut document = Document::new(Mesh::sphere(3));
    document.store_morph_target().unwrap();
    document.begin_stroke();
    for point in &mut document.active_mesh_mut().unwrap().positions {
        point.z += 0.8;
    }
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    let old = document.active_mesh().unwrap().snapshot();
    let vertex = old
        .positions
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.z.total_cmp(&b.1.z))
        .unwrap()
        .0;
    let settings = BrushSettings {
        kind: BrushKind::Morph,
        radius: 0.35,
        strength: 1.0,
        symmetry_x: false,
        adaptive_detail: true,
        ..BrushSettings::default()
    };
    let result = document
        .apply_morph_brush_with_result(old.positions[vertex], Vec3::Z, &settings)
        .unwrap();
    assert!(result.changed_vertices > 0);
    assert!(result.displacement_scale > 0.0 && result.displacement_scale < 1.0);
    assert_eq!(result.refined_vertices, 0);
    assert_eq!(document.active_mesh().unwrap().triangles, old.triangles);
    assert_eq!(
        document.active_mesh().unwrap().positions.len(),
        old.positions.len()
    );
    assert!(document.active_mesh().unwrap().positions[vertex].z < old.positions[vertex].z - 0.001);
    let mut edges = BTreeMap::<(u32, u32), Vec<usize>>::new();
    let mut normals = Vec::new();
    for (face, triangle) in document.active_mesh().unwrap().triangles.iter().enumerate() {
        let [a, b, c] =
            triangle.map(|i| document.active_mesh().unwrap().positions[i as usize].as_dvec3());
        let cross = (b - a).cross(c - a);
        let longest = (b - a)
            .length_squared()
            .max((c - a).length_squared())
            .max((c - b).length_squared());
        assert!(cross.length() / longest >= 0.1 - 1e-6);
        let [oa, ob, oc] = triangle.map(|i| old.positions[i as usize].as_dvec3());
        assert!(cross.dot((ob - oa).cross(oc - oa)) > 0.0);
        normals.push(cross.normalize());
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            edges.entry((a.min(b), a.max(b))).or_default().push(face);
        }
    }
    for faces in edges.values() {
        assert_eq!(faces.len(), 2);
        assert!(normals[faces[0]].dot(normals[faces[1]]) >= 0.5 - 1e-6);
    }
}

#[test]
fn pending_invalid_stroke_cannot_become_a_new_morph_target() {
    let mut document = Document::new(Mesh::sphere(0));
    document.store_morph_target().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0] *= 1.02;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().normals[0].x = f32::NAN;
    assert!(document.store_morph_target().is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.can_redo());
}
