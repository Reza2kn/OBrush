use super::*;

fn reserve_vertex_headroom(mesh: &mut Mesh, extra: usize) {
    mesh.positions.try_reserve_exact(extra).unwrap();
    mesh.normals.try_reserve_exact(extra).unwrap();
    mesh.masks.try_reserve_exact(extra).unwrap();
    mesh.colors.try_reserve_exact(extra).unwrap();
    if let Some(materials) = &mut mesh.materials {
        materials.try_reserve_exact(extra).unwrap();
    }
}

#[test]
fn in_place_admission_requires_existing_capacity_without_mutating_source() {
    let mut source = Mesh::sphere(0);
    source.positions.shrink_to_fit();
    source.normals.shrink_to_fit();
    source.masks.shrink_to_fit();
    source.colors.shrink_to_fit();
    let original = serde_json::to_vec(&source).unwrap();
    let capacities = [
        source.positions.capacity(),
        source.normals.capacity(),
        source.masks.capacity(),
        source.colors.capacity(),
    ];
    let triangles = source.triangles.clone();
    assert!(!can_publish_splice_in_place(
        &source, &source, &triangles, 1
    ));
    assert_eq!(serde_json::to_vec(&source).unwrap(), original);
    assert_eq!(
        [
            source.positions.capacity(),
            source.normals.capacity(),
            source.masks.capacity(),
            source.colors.capacity(),
        ],
        capacities
    );
    reserve_vertex_headroom(&mut source, 1);
    assert!(can_publish_splice_in_place(&source, &source, &triangles, 1));
}

#[test]
fn capacity_admitted_and_copied_fallback_draw_match_exact_mesh() {
    let mut base = Mesh::sphere(7);
    for position in &mut base.positions {
        *position *= 20.0;
    }
    base.materials = Some(vec![[0.3, 0.7]; base.positions.len()]);
    base.validate().unwrap();
    let settings = BrushSettings {
        radius: 0.18,
        strength: 0.35,
        symmetry_x: true,
        ..Default::default()
    };
    let center = Vec3::new(0.0, 0.0, 20.0);
    let normal = Vec3::Z;
    let mut fallback = base.snapshot();
    fallback.positions.shrink_to_fit();
    fallback.normals.shrink_to_fit();
    fallback.masks.shrink_to_fit();
    fallback.colors.shrink_to_fit();
    fallback.materials.as_mut().unwrap().shrink_to_fit();
    let before_fallback = fallback.positions.capacity();
    let mut admitted = base.snapshot();
    reserve_vertex_headroom(&mut admitted, 2048);
    let before_admitted = admitted.positions.capacity();
    let fallback_result = fallback
        .try_local_draw_window(center, normal, Vec3::ZERO, &settings)
        .unwrap();
    let admitted_result = admitted
        .try_local_draw_window(center, normal, Vec3::ZERO, &settings)
        .unwrap();
    assert!(fallback_result.changed_vertices > 0);
    assert!(fallback_result.refined_vertices > 0);
    assert_eq!(fallback_result, admitted_result);
    assert!(fallback.positions.len() > before_fallback);
    assert!(admitted.positions.len() <= before_admitted);
    assert_eq!(admitted.positions.capacity(), before_admitted);
    assert!(fallback.positions.capacity() > before_fallback);
    fallback.validate().unwrap();
    admitted.validate().unwrap();
    assert_eq!(
        serde_json::to_vec(&fallback).unwrap(),
        serde_json::to_vec(&admitted).unwrap()
    );
}

#[test]
fn unchanged_topology_reuses_original_triangle_buffer_and_matches_full_dab() {
    let base = Mesh::sphere(7);
    let settings = BrushSettings {
        radius: 0.18,
        strength: 0.35,
        symmetry_x: true,
        ..Default::default()
    };
    let mut window = base.snapshot();
    let triangle_ptr = window.triangles.as_ptr();
    let triangle_capacity = window.triangles.capacity();
    let mut full = base.snapshot();
    let (expected, _) =
        full.apply_brush_dab(Vec3::Z, Vec3::Z, Vec3::ZERO, &settings, None, true, true);
    let result = window
        .try_local_draw_window(Vec3::Z, Vec3::Z, Vec3::ZERO, &settings)
        .unwrap();
    assert!(result.changed_vertices > 0);
    assert_eq!(result.refined_vertices, 0);
    assert_eq!(result, expected);
    assert_eq!(window.triangles.as_ptr(), triangle_ptr);
    assert_eq!(window.triangles.capacity(), triangle_capacity);
    window.validate().unwrap();
    assert_eq!(
        serde_json::to_vec(&window).unwrap(),
        serde_json::to_vec(&full).unwrap()
    );
    let before = serde_json::to_vec(&window).unwrap();
    let no_contact =
        window.try_local_draw_window(Vec3::new(0.0, 0.0, 8.0), Vec3::Z, Vec3::ZERO, &settings);
    assert!(no_contact.is_none());
    assert_eq!(serde_json::to_vec(&window).unwrap(), before);
}
