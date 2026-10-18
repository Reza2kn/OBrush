//! Independent scalar/face/channel contracts; source labels are not parity proof.
use obrush_core::{
    CornerUv, Document, Mesh, ObjectTransform, StaticSubdivisionMap, StaticSubdivisionSettings,
    SurfaceTopology, Vec3, MAX_TRIANGLES, MAX_VERTICES, TRIANGLE_CORNER,
};
use serde_json::Value;
const SMOOTH: StaticSubdivisionSettings = StaticSubdivisionSettings {
    linear_edges: false,
};
const LINEAR: StaticSubdivisionSettings = StaticSubdivisionSettings { linear_edges: true };
fn mesh(points: &[[f32; 3]], faces: &[[u32; 4]]) -> Mesh {
    let mut text = String::new();
    for &[x, y, z] in points {
        text.push_str(&format!("v {x} {y} {z}\n"));
    }
    for &[a, b, c, d] in faces {
        text.push_str(&format!("f {} {} {}\n", a + 1, b + 1, c + 1));
        if d != TRIANGLE_CORNER {
            text.push_str(&format!("f {} {} {}\n", a + 1, c + 1, d + 1));
        }
    }
    let mut m = Mesh::from_obj(&text).unwrap();
    m.install_attributes(
        None,
        Some(SurfaceTopology {
            faces: faces.to_vec(),
            uv: None,
        }),
    )
    .unwrap();
    m
}
fn quad() -> Mesh {
    mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[[0, 1, 2, 3]],
    )
}
fn cube() -> Mesh {
    mesh(
        &[
            [-1., -1., -1.],
            [1., -1., -1.],
            [1., 1., -1.],
            [-1., 1., -1.],
            [-1., -1., 1.],
            [1., -1., 1.],
            [1., 1., 1.],
            [-1., 1., 1.],
        ],
        &[
            [3, 2, 1, 0],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ],
    )
}
fn pyramid() -> Mesh {
    mesh(
        &[
            [-1., -1., 0.],
            [1., -1., 0.],
            [1., 1., 0.],
            [-1., 1., 0.],
            [0., 0., 1.],
        ],
        &[
            [3, 2, 1, 0],
            [0, 1, 4, TRIANGLE_CORNER],
            [1, 2, 4, TRIANGLE_CORNER],
            [2, 3, 4, TRIANGLE_CORNER],
            [3, 0, 4, TRIANGLE_CORNER],
        ],
    )
}
fn decorate(m: &mut Mesh) {
    for i in 0..m.positions.len() {
        m.colors[i] = [i as f32 / 10., (i + 1) as f32 / 10., 0.5];
        m.masks[i] = i as f32 / 10.;
    }
    m.materials = Some(m.colors.iter().map(|c| [c[0], c[1]]).collect());
}
fn bits(a: Vec3, b: Vec3) {
    assert_eq!(
        a.to_array().map(f32::to_bits),
        b.to_array().map(f32::to_bits)
    );
}
fn json(m: &Mesh) -> String {
    serde_json::to_string(m).unwrap()
}
fn state(d: &Document) -> String {
    d.to_json().unwrap()
}
fn seams(m: &mut Mesh) {
    let faces = &m.surface.as_ref().unwrap().faces;
    let mut coordinates = Vec::new();
    let mut face_indices = Vec::new();
    for (f, face) in faces.iter().enumerate() {
        let n = if face[3] == TRIANGLE_CORNER { 3 } else { 4 };
        let start = coordinates.len() as u32;
        let mut row = [TRIANGLE_CORNER; 4];
        for (corner, index) in row.iter_mut().enumerate().take(n) {
            *index = start + corner as u32;
            coordinates.push([f as f32 * 2. + (corner % 2) as f32, (corner / 2) as f32]);
        }
        face_indices.push(row);
    }
    m.surface.as_mut().unwrap().uv = Some(CornerUv {
        coordinates,
        face_indices,
    });
    m.validate().unwrap();
}

#[test]
fn closed_quad_cube_has_twenty_six_points_without_six_render_diagonals() {
    let out = cube().subdivided_static(&SMOOTH).unwrap();
    assert_eq!(
        (
            out.report.true_edges,
            out.report.input_quads,
            out.report.output_vertices,
            out.report.output_faces,
            out.report.output_triangles
        ),
        (12, 6, 26, 24, 48)
    );
    assert_eq!(
        out.forward_map.child_faces_for_parent(0),
        Some([0, 1, 2, 3])
    );
    assert_eq!(out.forward_map.child_faces_for_parent(u32::MAX), None);
}
#[test]
fn cube_catmull_clark_corner_edge_and_center_match_independent_rational_values() {
    let out = cube().subdivided_static(&SMOOTH).unwrap();
    bits(out.mesh.positions[0], Vec3::splat(-((5. / 9.) as f32)));
    bits(out.mesh.positions[8], Vec3::new(0., 0.75, -0.75));
    bits(out.mesh.positions[12], Vec3::new(0., 0., -1.));
}
#[test]
fn mixed_pyramid_valence_edge_and_child_order_match_hand_authored_golden() {
    let golden: Value =
        serde_json::from_str(include_str!("fixtures/independent-static-golden.json")).unwrap();
    let p = &golden["mixed_quad_pyramid"];
    let out = pyramid().subdivided_static(&SMOOTH).unwrap();
    assert_eq!(
        (
            out.report.true_edges,
            out.report.output_vertices,
            out.report.output_faces,
            out.report.output_triangles
        ),
        (8, 14, 20, 24)
    );
    let expected =
        |key: &str| Vec3::from_array(std::array::from_fn(|i| p[key][i].as_f64().unwrap() as f32));
    bits(out.mesh.positions[0], expected("corner_even_0"));
    bits(out.mesh.positions[4], expected("apex_even_4"));
    bits(out.mesh.positions[7], expected("mixed_edge_0_1"));
    bits(out.mesh.positions[11], expected("triangle_edge_0_4"));
    bits(out.mesh.positions[9], expected("quad_center"));
    let faces = &out.mesh.surface.as_ref().unwrap().faces;
    assert_eq!(
        &faces[..4],
        &[[3, 5, 9, 8], [5, 2, 6, 9], [9, 6, 1, 7], [8, 9, 7, 0]]
    );
    assert_eq!(
        &faces[4..8],
        &[
            [7, 10, 11, TRIANGLE_CORNER],
            [0, 7, 11, TRIANGLE_CORNER],
            [7, 1, 10, TRIANGLE_CORNER],
            [11, 10, 4, TRIANGLE_CORNER]
        ]
    );
}
#[test]
fn warren_triangle_valence_four_remains_distinct_from_legacy_cosine_loop() {
    let mut tri = pyramid();
    tri.surface = None;
    let actual = tri.subdivided_static(&SMOOTH).unwrap();
    bits(actual.mesh.positions[4], Vec3::new(0., 0., 0.625));
    let mut legacy = tri;
    legacy.subdivide_smooth().unwrap();
    assert_eq!(legacy.positions[4].z, 0.515625);
    assert_ne!(actual.mesh.positions[4], legacy.positions[4]);
}
#[test]
fn single_quad_boundary_quirk_keeps_each_even_corner_in_smooth_and_linear_modes() {
    for mode in [SMOOTH, LINEAR] {
        let source = quad();
        let out = source.subdivided_static(&mode).unwrap();
        assert_eq!(&out.mesh.positions[..4], &source.positions);
        assert_eq!(
            (
                out.report.output_vertices,
                out.report.output_faces,
                out.report.output_triangles
            ),
            (9, 4, 8)
        );
    }
}
#[test]
fn linear_edges_preserve_closed_control_vertices_and_use_odd_midpoints() {
    let source = cube();
    let out = source.subdivided_static(&LINEAR).unwrap();
    assert_eq!(&out.mesh.positions[..8], &source.positions);
    bits(out.mesh.positions[8], Vec3::new(0., 1., -1.));
}
#[test]
fn two_triangle_boundary_else_if_keeps_one_face_corners_and_smooths_two_face_corners() {
    let source = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[[0, 1, 2, TRIANGLE_CORNER], [0, 2, 3, TRIANGLE_CORNER]],
    );
    let out = source.subdivided_static(&SMOOTH).unwrap();
    bits(out.mesh.positions[0], Vec3::new(0.125, 0.125, 0.));
    bits(out.mesh.positions[1], source.positions[1]);
    bits(out.mesh.positions[2], Vec3::new(0.875, 0.875, 0.));
    bits(out.mesh.positions[3], source.positions[3]);
}
#[test]
fn old_rgb_material_and_mask_rows_copy_exactly_instead_of_smoothing() {
    let mut source = pyramid();
    decorate(&mut source);
    source.colors[0][0] = -0.;
    source.materials.as_mut().unwrap()[0][0] = -0.;
    source.masks[0] = -0.;
    let out = source.subdivided_static(&SMOOTH).unwrap();
    for i in 0..source.positions.len() {
        assert_eq!(
            out.mesh.colors[i].map(f32::to_bits),
            source.colors[i].map(f32::to_bits)
        );
        assert_eq!(
            out.mesh.materials.as_ref().unwrap()[i].map(f32::to_bits),
            source.materials.as_ref().unwrap()[i].map(f32::to_bits)
        );
        assert_eq!(out.mesh.masks[i].to_bits(), source.masks[i].to_bits());
    }
}
#[test]
fn odd_mixed_edge_and_face_materials_use_the_same_reference_rgb_stencils() {
    let mut source = pyramid();
    decorate(&mut source);
    let out = source.subdivided_static(&SMOOTH).unwrap();
    for (material, color) in out
        .mesh
        .materials
        .as_ref()
        .unwrap()
        .iter()
        .zip(&out.mesh.colors)
    {
        assert_eq!(
            material.map(f32::to_bits),
            [color[0].to_bits(), color[1].to_bits()]
        );
    }
    assert_eq!(out.mesh.materials.as_ref().unwrap()[9], [0.15, 0.25]);
}
#[test]
fn typed_array_two_incident_edge_rounding_is_not_a_fused_stencil() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/independent-static-rounding.json")).unwrap();
    let mut source = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., -1., 0.]],
        &[[0, 1, 2, TRIANGLE_CORNER], [1, 0, 3, TRIANGLE_CORNER]],
    );
    for i in 0..4 {
        source.colors[i][0] = fixture["inputs"][i].as_f64().unwrap() as f32;
    }
    let out = source.subdivided_static(&SMOOTH).unwrap();
    assert_eq!(
        out.mesh.colors[4][0].to_bits(),
        fixture["expected_bits"].as_u64().unwrap() as u32
    );
    assert_ne!(
        out.mesh.colors[4][0].to_bits(),
        fixture["incorrect_fused_bits"].as_u64().unwrap() as u32
    );
}
#[test]
fn missing_material_and_uv_channels_stay_absent() {
    let out = quad().subdivided_static(&SMOOTH).unwrap();
    assert!(out.mesh.materials.is_none());
    assert!(out.mesh.surface.as_ref().unwrap().uv.is_none());
}
#[test]
fn quad_uv_center_edge_values_and_corner_seams_survive() {
    let mut source = quad();
    source.surface.as_mut().unwrap().uv = Some(CornerUv {
        coordinates: vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
        face_indices: vec![[0, 1, 2, 3]],
    });
    let out = source.subdivided_static(&SMOOTH).unwrap();
    let uv = out.mesh.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(uv.coordinates.len(), 9);
    assert_eq!(uv.coordinates[4], [0.5, 0.]);
    assert_eq!(uv.coordinates[8], [0.5, 0.5]);
    assert_eq!(
        uv.face_indices,
        vec![[0, 4, 8, 7], [4, 1, 5, 8], [8, 5, 2, 6], [7, 8, 6, 3]]
    );
}
#[test]
fn mixed_chart_edge_seams_do_not_weld_when_geometry_is_shared() {
    let mut source = pyramid();
    seams(&mut source);
    let out = source.subdivided_static(&SMOOTH).unwrap();
    assert_eq!(out.report.output_uv_coordinates, 33);
    let uv = out.mesh.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_ne!(uv.face_indices[2][3], uv.face_indices[4][0]);
    assert_eq!(
        &uv.coordinates[..16],
        &source
            .surface
            .as_ref()
            .unwrap()
            .uv
            .as_ref()
            .unwrap()
            .coordinates
    );
}
#[test]
fn continuous_chart_edges_intern_once_even_with_reversed_incidence() {
    let mut source = mesh(
        &[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        &[[0, 1, 2, TRIANGLE_CORNER], [0, 2, 3, TRIANGLE_CORNER]],
    );
    source.surface.as_mut().unwrap().uv = Some(CornerUv {
        coordinates: vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
        face_indices: vec![[0, 1, 2, TRIANGLE_CORNER], [0, 2, 3, TRIANGLE_CORNER]],
    });
    let out = source.subdivided_static(&SMOOTH).unwrap();
    assert_eq!(out.report.output_uv_coordinates, 9);
    let uv = out.mesh.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(uv.face_indices[0][2], uv.face_indices[4][0]);
}
#[test]
fn equal_value_distinct_uv_chart_ids_and_signed_zero_prefix_remain_distinct() {
    let mut source = pyramid();
    seams(&mut source);
    let uv = source.surface.as_mut().unwrap().uv.as_mut().unwrap();
    uv.coordinates[0][0] = -0.;
    uv.coordinates[4] = uv.coordinates[3];
    uv.coordinates[5] = uv.coordinates[2];
    let out = source.subdivided_static(&SMOOTH).unwrap();
    let uv = out.mesh.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(uv.coordinates[0][0].to_bits(), (-0f32).to_bits());
    assert_eq!(uv.coordinates.len(), 33);
}
#[test]
fn three_face_nonmanifold_edge_uses_source_midpoint_fallback() {
    let source = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., -1., 0.],
            [0., 0., 1.],
        ],
        &[
            [0, 1, 2, TRIANGLE_CORNER],
            [1, 0, 3, TRIANGLE_CORNER],
            [0, 1, 4, TRIANGLE_CORNER],
        ],
    );
    let out = source.subdivided_static(&SMOOTH).unwrap();
    assert_eq!(out.report.nonmanifold_edges, 1);
    bits(out.mesh.positions[5], Vec3::new(0.5, 0., 0.));
}
#[test]
fn map_reuse_accepts_geometric_rgb_and_material_edits_but_rejects_presence_or_uv_changes() {
    let mut source = pyramid();
    decorate(&mut source);
    seams(&mut source);
    let map = StaticSubdivisionMap::build(&source, SMOOTH).unwrap();
    source.positions[4].z += 0.1;
    source.colors[0][0] = 0.8;
    source.materials.as_mut().unwrap()[0][0] = 0.7;
    assert!(map.apply(&source).is_ok());
    let before = json(&source);
    source.materials = None;
    assert!(map.apply(&source).unwrap_err().contains("stale"));
    source = serde_json::from_str(&before).unwrap();
    source
        .surface
        .as_mut()
        .unwrap()
        .uv
        .as_mut()
        .unwrap()
        .coordinates[0][0] = 0.25;
    assert!(map.apply(&source).unwrap_err().contains("stale"));
}
#[test]
fn stale_same_size_face_reorder_and_surface_detachment_are_rejected() {
    let source = pyramid();
    let map = StaticSubdivisionMap::build(&source, SMOOTH).unwrap();
    let mut changed = source.snapshot();
    changed.surface = None;
    assert!(map.apply(&changed).is_err());
    let mut changed = source;
    changed.triangles.swap(2, 3);
    changed.surface.as_mut().unwrap().faces.swap(1, 2);
    assert!(map.apply(&changed).is_err());
}
#[test]
fn malformed_channels_and_nonfinite_or_collapsed_geometry_reject_without_mutation() {
    for kind in 0..4 {
        let mut source = pyramid();
        match kind {
            0 => source.positions[0].x = f32::NAN,
            1 => source.positions[1] = source.positions[0],
            2 => source.colors[0][0] = 1.1,
            _ => source.surface.as_mut().unwrap().faces[0][0] = 999,
        };
        let before = source.positions.clone();
        assert!(source.subdivided_static(&SMOOTH).is_err());
        assert_eq!(source.positions.len(), before.len());
        for (a, b) in source.positions.iter().zip(before) {
            bits(*a, b);
        }
    }
}
#[test]
fn triangle_growth_limit_is_preflighted_and_source_unchanged() {
    let mut source = Mesh::sphere(0);
    source.triangles = vec![source.triangles[0]; MAX_TRIANGLES / 4 + 1];
    let before = source.triangles.len();
    assert!(source
        .subdivided_static(&SMOOTH)
        .unwrap_err()
        .contains("limits"));
    assert_eq!(source.triangles.len(), before);
}
#[test]
fn point_growth_limit_is_preflighted_without_building_a_large_map() {
    let mut source = Mesh::sphere(0);
    source.positions.resize(MAX_VERTICES, Vec3::Z);
    source.normals.resize(MAX_VERTICES, Vec3::Z);
    source.colors.resize(MAX_VERTICES, [0.2; 3]);
    source.masks.resize(MAX_VERTICES, 0.);
    assert!(source
        .subdivided_static(&SMOOTH)
        .unwrap_err()
        .contains("limits"));
}
#[test]
fn persistent_static_smooth_and_linear_schemes_use_v6_and_roundtrip_exactly() {
    for settings in [SMOOTH, LINEAR] {
        let mut doc = Document::new(pyramid());
        doc.add_static_subdivision_level(&settings).unwrap();
        let encoded = state(&doc);
        let value: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(value["version"], 6);
        assert_eq!(
            value["multiresolution"]["scheme"],
            if settings.linear_edges {
                "sculptgl_static_linear_edges_v1"
            } else {
                "sculptgl_static_smooth_v1"
            }
        );
        assert_eq!(state(&Document::from_json(&encoded).unwrap()), encoded);
    }
}
#[test]
fn static_level_creation_and_second_level_have_complete_undo_redo() {
    let mut doc = Document::new(pyramid());
    decorate(doc.active_mesh_mut().unwrap());
    seams(doc.active_mesh_mut().unwrap());
    let before = state(&doc);
    doc.add_static_subdivision_level(&SMOOTH).unwrap();
    let first = state(&doc);
    doc.add_static_subdivision_level(&SMOOTH).unwrap();
    let second = state(&doc);
    assert!(doc.undo());
    assert_eq!(state(&doc), first);
    assert!(doc.undo());
    assert_eq!(state(&doc), before);
    assert!(doc.redo());
    assert_eq!(state(&doc), first);
    assert!(doc.redo());
    assert_eq!(state(&doc), second);
}
#[test]
fn static_navigation_keeps_fine_residuals_and_coarse_channels_without_drift() {
    let mut doc = Document::new(pyramid());
    decorate(doc.active_mesh_mut().unwrap());
    seams(doc.active_mesh_mut().unwrap());
    doc.add_static_subdivision_level(&SMOOTH).unwrap();
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().positions[4].z += 0.01;
    doc.active_mesh_mut().unwrap().colors[5][0] = 0.3;
    doc.active_mesh_mut().unwrap().recompute_normals();
    doc.end_stroke();
    let fine = json(doc.active_mesh().unwrap());
    doc.select_level(0).unwrap();
    let coarse = json(doc.active_mesh().unwrap());
    for _ in 0..3 {
        doc.select_level(1).unwrap();
        assert_eq!(json(doc.active_mesh().unwrap()), fine);
        doc.select_level(0).unwrap();
        assert_eq!(json(doc.active_mesh().unwrap()), coarse);
    }
}

#[test]
fn source_static_v3_lower_copies_fine_even_rows_without_losing_uv_or_history() {
    for settings in [SMOOTH, LINEAR] {
        let mut doc = Document::new(pyramid());
        decorate(doc.active_mesh_mut().unwrap());
        seams(doc.active_mesh_mut().unwrap());
        let original = state(&doc);
        let original_surface = doc.active_mesh().unwrap().surface.clone();
        doc.add_source_static_subdivision_level(&settings).unwrap();
        let fine = doc.active_mesh().unwrap().snapshot();
        let fine_state = state(&doc);
        let value: Value = serde_json::from_str(&fine_state).unwrap();
        assert_eq!(
            value["multiresolution"]["scheme"],
            if settings.linear_edges {
                "sculptgl_static_linear_edges_v3"
            } else {
                "sculptgl_static_smooth_v3"
            }
        );
        doc.select_level(0).unwrap();
        let coarse = doc.active_mesh().unwrap();
        let count = coarse.positions.len();
        assert_eq!(coarse.positions, fine.positions[..count]);
        assert_eq!(coarse.colors, fine.colors[..count]);
        assert_eq!(coarse.masks, fine.masks[..count]);
        assert_eq!(
            coarse.materials.as_ref().unwrap(),
            &fine.materials.as_ref().unwrap()[..count]
        );
        assert_eq!(coarse.surface, original_surface);
        assert_eq!(
            state(&Document::from_json(&state(&doc)).unwrap()),
            state(&doc)
        );
        doc.select_level(1).unwrap();
        assert_eq!(doc.active_mesh().unwrap().positions, fine.positions);
        assert_eq!(doc.active_mesh().unwrap().surface, fine.surface);
        let after_navigation = state(&doc);
        assert!(doc.undo());
        assert_eq!(state(&doc), original);
        assert!(doc.redo());
        assert_eq!(state(&doc), after_navigation);
    }
}

#[test]
fn source_static_v3_descends_two_levels_and_legacy_v1_stays_exact() {
    let mut source = pyramid();
    seams(&mut source);
    let mut current = Document::new(source.snapshot());
    current
        .add_source_static_subdivision_level(&SMOOTH)
        .unwrap();
    current
        .add_source_static_subdivision_level(&SMOOTH)
        .unwrap();
    let finest = current.active_mesh().unwrap().snapshot();
    current.select_level(0).unwrap();
    assert_eq!(
        current.active_mesh().unwrap().positions,
        finest.positions[..source.positions.len()]
    );
    current.select_level(2).unwrap();
    assert_eq!(current.active_mesh().unwrap().positions, finest.positions);
    assert_eq!(
        state(&Document::from_json(&state(&current)).unwrap()),
        state(&current)
    );

    let mut legacy = Document::new(source.snapshot());
    legacy.add_static_subdivision_level(&SMOOTH).unwrap();
    let legacy_fine = state(&legacy);
    legacy.select_level(0).unwrap();
    assert_eq!(legacy.active_mesh().unwrap().positions, source.positions);
    legacy.select_level(1).unwrap();
    assert_eq!(state(&legacy), legacy_fine);
}

#[test]
fn source_static_v3_coarse_edit_reconciles_fine_without_changing_uv() {
    let mut source = pyramid();
    seams(&mut source);
    let mut doc = Document::new(source);
    doc.add_source_static_subdivision_level(&SMOOTH).unwrap();
    let original_fine = doc.active_mesh().unwrap().snapshot();
    doc.select_level(0).unwrap();
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().positions[0].z += 0.01;
    doc.active_mesh_mut().unwrap().recompute_normals();
    doc.end_stroke();
    doc.select_level(1).unwrap();
    let changed = doc.active_mesh().unwrap();
    assert_ne!(changed.positions, original_fine.positions);
    assert_eq!(changed.surface, original_fine.surface);
    assert_eq!(changed.triangles, original_fine.triangles);
    assert_eq!(
        state(&Document::from_json(&state(&doc)).unwrap()),
        state(&doc)
    );
}

#[test]
fn loaded_source_static_v2_keeps_its_world_offset_policy() {
    let mut source = pyramid();
    seams(&mut source);
    let mut fresh = Document::new(source);
    fresh.add_source_static_subdivision_level(&SMOOTH).unwrap();
    let mut old_project: serde_json::Value = serde_json::from_str(&state(&fresh)).unwrap();
    old_project["multiresolution"]["scheme"] =
        serde_json::Value::String("sculptgl_static_smooth_v2".into());
    old_project["multiresolution"]
        .as_object_mut()
        .unwrap()
        .remove("source_root_order");
    let old_project = old_project.to_string();
    assert_ne!(old_project, state(&fresh));
    let mut legacy = Document::from_json(&old_project).unwrap();
    let old_child = legacy.active_mesh().unwrap().snapshot();
    legacy.select_level(0).unwrap();
    let old_parent = legacy.active_mesh().unwrap().snapshot();
    let old_prediction = old_parent.subdivided_static(&SMOOTH).unwrap().mesh;
    legacy.active_mesh_mut().unwrap().positions[4].z += 0.125;
    legacy.active_mesh_mut().unwrap().recompute_normals();
    let new_prediction = legacy
        .active_mesh()
        .unwrap()
        .subdivided_static(&SMOOTH)
        .unwrap()
        .mesh;
    legacy.select_level(1).unwrap();
    for vertex in 0..old_child.positions.len() {
        assert_eq!(
            legacy.active_mesh().unwrap().positions[vertex],
            old_child.positions[vertex]
                + (new_prediction.positions[vertex] - old_prediction.positions[vertex])
        );
    }
}

#[test]
fn source_static_v3_coarse_edit_transports_through_two_levels_and_reloads() {
    let mut source = pyramid();
    seams(&mut source);
    let mut doc = Document::new(source);
    doc.add_source_static_subdivision_level(&SMOOTH).unwrap();
    doc.add_source_static_subdivision_level(&SMOOTH).unwrap();
    let original_fine = doc.active_mesh().unwrap().positions.clone();
    doc.select_level(0).unwrap();
    doc.active_mesh_mut().unwrap().positions[4].z += 0.05;
    doc.active_mesh_mut().unwrap().recompute_normals();
    doc.select_level(2).unwrap();
    assert_ne!(doc.active_mesh().unwrap().positions, original_fine);
    let edited = state(&doc);
    let mut restored = Document::from_json(&edited).unwrap();
    assert_eq!(state(&restored), edited);
    restored.select_level(0).unwrap();
    restored.select_level(2).unwrap();
    let drift = restored
        .active_mesh()
        .unwrap()
        .positions
        .iter()
        .zip(doc.active_mesh().unwrap().positions.iter())
        .map(|(returned, original)| (*returned - *original).abs().max_element())
        .fold(0.0f32, f32::max);
    println!("two-level edited fine return max component drift {drift:.9e}");
    assert_eq!(drift, 0.0, "Two-level return drifted by {drift}");
    assert_eq!(
        restored.active_mesh().unwrap().surface,
        doc.active_mesh().unwrap().surface
    );
}

#[test]
fn source_static_v3_level_deletion_retains_the_corresponding_frame_map() {
    for lower in [true, false] {
        let mut source = pyramid();
        seams(&mut source);
        let mut doc = Document::new(source);
        doc.add_source_static_subdivision_level(&SMOOTH).unwrap();
        doc.add_source_static_subdivision_level(&SMOOTH).unwrap();
        doc.select_level(1).unwrap();
        let report = if lower {
            doc.delete_lower_subdivision_levels().unwrap()
        } else {
            doc.delete_higher_subdivision_levels().unwrap()
        };
        assert_eq!(report.retained_levels, 2);
        let edited = state(&doc);
        let mut restored = Document::from_json(&edited).unwrap();
        assert_eq!(state(&restored), edited);
        restored.select_level(0).unwrap();
        restored.active_mesh_mut().unwrap().positions[4].z += 0.03;
        restored.active_mesh_mut().unwrap().recompute_normals();
        restored.select_level(1).unwrap();
        assert!(restored
            .active_mesh()
            .unwrap()
            .positions
            .iter()
            .all(|point| point.is_finite()));
        assert_eq!(
            state(&Document::from_json(&state(&restored)).unwrap()),
            state(&restored)
        );
    }
}
#[test]
fn coarse_delta_transport_reuses_static_not_legacy_loop_and_preserves_uv() {
    let mut doc = Document::new(pyramid());
    decorate(doc.active_mesh_mut().unwrap());
    seams(doc.active_mesh_mut().unwrap());
    doc.add_static_subdivision_level(&SMOOTH).unwrap();
    let original = doc.active_mesh().unwrap().snapshot();
    doc.select_level(0).unwrap();
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().positions[4].z += 0.125;
    doc.active_mesh_mut().unwrap().materials.as_mut().unwrap()[4][0] += 0.125;
    doc.active_mesh_mut().unwrap().recompute_normals();
    doc.end_stroke();
    doc.select_level(1).unwrap();
    assert_eq!(
        doc.active_mesh().unwrap().positions[4].z,
        (original.positions[4].z + 0.125 * 0.625)
    );
    assert_eq!(
        doc.active_mesh().unwrap().materials.as_ref().unwrap()[4][0],
        original.materials.as_ref().unwrap()[4][0] + 0.125
    );
    assert_eq!(doc.active_mesh().unwrap().surface, original.surface);
}
#[test]
fn changing_retained_scheme_or_using_legacy_add_does_not_mutate_the_stack() {
    let mut doc = Document::new(pyramid());
    doc.add_static_subdivision_level(&SMOOTH).unwrap();
    let before = state(&doc);
    assert!(doc.add_static_subdivision_level(&LINEAR).is_err());
    assert_eq!(state(&doc), before);
    assert!(doc.add_subdivision_level().is_err());
    assert_eq!(state(&doc), before);
    doc.select_level(0).unwrap();
    let before = state(&doc);
    assert!(doc.add_static_subdivision_level(&SMOOTH).is_err());
    assert_eq!(state(&doc), before);
}
#[test]
fn static_schemes_cannot_hide_in_a_legacy_version() {
    let mut doc = Document::new(pyramid());
    doc.add_static_subdivision_level(&SMOOTH).unwrap();
    let mut value: Value = serde_json::from_str(&state(&doc)).unwrap();
    value["version"] = 5.into();
    assert!(Document::from_json(&value.to_string()).is_err());
}
#[test]
fn legacy_level_scheme_bytes_and_cosine_math_stay_unchanged() {
    let mut source = pyramid();
    source.surface = None;
    let mut expected = source.snapshot();
    expected.subdivide_smooth().unwrap();
    let mut doc = Document::new(source);
    doc.add_subdivision_level().unwrap();
    assert_eq!(json(doc.active_mesh().unwrap()), json(&expected));
    let value: Value = serde_json::from_str(&state(&doc)).unwrap();
    assert_eq!(value["version"], 2);
    assert_eq!(value["multiresolution"]["scheme"], "loop_world_offsets_v1");
}
#[test]
fn static_scene_transform_preserves_scheme_materials_and_corner_uv_levels() {
    let mut source = pyramid();
    decorate(&mut source);
    seams(&mut source);
    let mut doc = Document::new(source);
    doc.add_static_subdivision_level(&SMOOTH).unwrap();
    let uv = doc.active_mesh().unwrap().surface.clone();
    let m = doc.active_mesh().unwrap().materials.clone();
    let before = state(&doc);
    doc.transform_active_subtool(&ObjectTransform {
        translation: Vec3::new(0.2, 0.3, 0.4),
        scale: 1.2,
        ..Default::default()
    })
    .unwrap();
    let after = state(&doc);
    assert_eq!(doc.active_mesh().unwrap().surface, uv);
    assert_eq!(doc.active_mesh().unwrap().materials, m);
    assert_eq!(state(&Document::from_json(&after).unwrap()), after);
    assert!(doc.undo());
    assert_eq!(state(&doc), before);
    assert!(doc.redo());
    assert_eq!(state(&doc), after);
}

#[test]
fn regular_quad_valence_four_center_uses_nine_sixteenths() {
    let source = mesh(
        &[
            [-1., -1., 0.],
            [0., -1., 0.],
            [1., -1., 0.],
            [-1., 0., 0.],
            [0., 0., 1.],
            [1., 0., 0.],
            [-1., 1., 0.],
            [0., 1., 0.],
            [1., 1., 0.],
        ],
        &[[0, 1, 4, 3], [1, 2, 5, 4], [3, 4, 7, 6], [4, 5, 8, 7]],
    );
    let out = source.subdivided_static(&SMOOTH).unwrap();
    assert_eq!(out.mesh.positions[4].z, 0.5625);
}

#[test]
fn extraordinary_quad_valence_five_uses_general_catmull_clark_rule() {
    let mut points = vec![[0., 0., 1.]];
    for i in 0..5 {
        let a = std::f32::consts::TAU * i as f32 / 5.;
        points.push([a.cos(), a.sin(), 0.]);
    }
    for i in 0..5 {
        let a = std::f32::consts::TAU * (i as f32 + 0.5) / 5.;
        points.push([2. * a.cos(), 2. * a.sin(), 0.]);
    }
    let faces = (0..5)
        .map(|i| [0, 1 + i, 6 + i, 1 + (i + 1) % 5])
        .collect::<Vec<_>>();
    let out = mesh(&points, &faces).subdivided_static(&SMOOTH).unwrap();
    assert_eq!(out.mesh.positions[0].z, 0.65);
}

#[test]
fn warren_triangle_valence_three_and_six_use_explicit_source_weights() {
    let source = mesh(
        &[[0., 0., 1.], [-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
        &[
            [0, 1, 2, TRIANGLE_CORNER],
            [0, 2, 3, TRIANGLE_CORNER],
            [0, 3, 1, TRIANGLE_CORNER],
            [1, 3, 2, TRIANGLE_CORNER],
        ],
    );
    assert_eq!(
        source.subdivided_static(&SMOOTH).unwrap().mesh.positions[0].z,
        0.4375
    );
    let mut points = vec![[0., 0., 1.]];
    for i in 0..6 {
        let a = std::f32::consts::TAU * i as f32 / 6.;
        points.push([a.cos(), a.sin(), 0.]);
    }
    let faces = (0..6)
        .map(|i| [0, 1 + i, 1 + (i + 1) % 6, TRIANGLE_CORNER])
        .collect::<Vec<_>>();
    assert_eq!(
        mesh(&points, &faces)
            .subdivided_static(&SMOOTH)
            .unwrap()
            .mesh
            .positions[0]
            .z,
        0.625
    );
}

#[test]
fn isolated_control_vertices_reject_before_a_reusable_map_is_returned() {
    let mut source = quad();
    source.positions.push(Vec3::new(3., 3., 3.));
    source.normals.push(Vec3::Z);
    source.colors.push([0.2; 3]);
    source.masks.push(0.);
    let before = json(&source);
    assert!(StaticSubdivisionMap::build(&source, SMOOTH)
        .unwrap_err()
        .contains("isolated"));
    assert_eq!(json(&source), before);
}

#[test]
fn maximum_unused_uv_channel_cannot_overflow_during_edge_birth() {
    let mut source = quad();
    source.surface.as_mut().unwrap().uv = Some(CornerUv {
        coordinates: vec![[0., 0.]; obrush_core::MAX_UV_COORDINATES],
        face_indices: vec![[0, 1, 2, 3]],
    });
    source.validate().unwrap();
    assert!(source
        .subdivided_static(&SMOOTH)
        .unwrap_err()
        .contains("UV coordinate limit"));
    assert_eq!(
        source
            .surface
            .as_ref()
            .unwrap()
            .uv
            .as_ref()
            .unwrap()
            .coordinates
            .len(),
        obrush_core::MAX_UV_COORDINATES
    );
}

#[test]
fn forward_map_storage_budget_includes_uv_tables_before_cloning_them() {
    let mut source = Mesh::sphere(7);
    let faces = source
        .triangles
        .iter()
        .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
        .collect::<Vec<_>>();
    let uv = CornerUv {
        coordinates: vec![[0., 0.]; obrush_core::MAX_UV_COORDINATES],
        face_indices: vec![[0, 1, 2, TRIANGLE_CORNER]; faces.len()],
    };
    source
        .install_attributes(
            None,
            Some(SurfaceTopology {
                faces,
                uv: Some(uv),
            }),
        )
        .unwrap();
    assert!(StaticSubdivisionMap::build(&source, SMOOTH)
        .unwrap_err()
        .contains("64 MiB"));
    assert_eq!(source.positions.len(), 163842);
}

#[test]
fn malformed_static_scheme_or_child_faces_do_not_replace_receiver_history() {
    let mut imported = Document::new(pyramid());
    imported.add_static_subdivision_level(&SMOOTH).unwrap();
    let encoded = state(&imported);
    let mut receiver = Document::new(quad());
    receiver.begin_stroke();
    receiver.active_mesh_mut().unwrap().colors[0] = [0.2; 3];
    receiver.end_stroke();
    assert!(receiver.undo());
    let before = state(&receiver);
    for invalid_scheme in [true, false] {
        let mut value: Value = serde_json::from_str(&encoded).unwrap();
        if invalid_scheme {
            value["multiresolution"]["scheme"] = "unknown_static_scheme".into();
        } else {
            value["mesh"]["surface"]["faces"][0][0] = 999.into();
        }
        assert!(Document::from_json(&value.to_string()).is_err());
        assert_eq!(state(&receiver), before);
        assert!(receiver.can_redo());
    }
}
