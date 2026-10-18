//! Independent byte-level SGL fixtures; no Mesh conversion or upstream encoder.
use obrush_core::{
    SglByteOrder, SglScene, MAX_SGL_UV_COORDS, MAX_SUBTOOLS, MAX_TRIANGLES, MAX_VERTICES,
};

fn word(bytes: &mut Vec<u8>, value: u32, big: bool) {
    bytes.extend_from_slice(&if big {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    });
}
fn float(bytes: &mut Vec<u8>, value: f32, big: bool) {
    word(bytes, value.to_bits(), big);
}
fn header(version: u32, big: bool, count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [version, 1, 0, 1, 1, 2] {
        word(&mut bytes, value, big);
    }
    float(&mut bytes, 63.5, big);
    word(&mut bytes, 1, big);
    word(&mut bytes, count, big);
    bytes
}
fn metadata(big: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [5, 17, 1, 0] {
        word(&mut bytes, value, big);
    }
    float(&mut bytes, 0.75, big);
    for value in [-0.0_f32, 0.25, 0.5] {
        float(&mut bytes, value, big);
    }
    for value in [
        2.0_f32, 0.0, 0.0, -0.0, 0.5, 3.0, 0.0, 0.0, 0.0, 0.25, -4.0, 0.0, 5.0, -6.0, 7.0, 1.0,
    ] {
        float(&mut bytes, value, big);
    }
    float(&mut bytes, 123.25, big);
    bytes
}
fn one_mesh(version: u32, big: bool, attributes: bool, quad: bool) -> Vec<u8> {
    let mut bytes = metadata(big);
    let vertices = if quad { 4 } else { 3 };
    word(&mut bytes, vertices, big);
    for p in [
        [-0.0_f32, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ]
    .iter()
    .take(vertices as usize)
    {
        for &v in p {
            float(&mut bytes, v, big);
        }
    }
    for material in [false, true] {
        word(&mut bytes, if attributes { vertices } else { 0 }, big);
        if attributes {
            for i in 0..vertices {
                let values = if material {
                    [
                        [0.1, 0.9, 1.0],
                        [0.2, 0.8, 0.0],
                        [0.3, 0.7, 1.0],
                        [0.4, 0.6, 1.0],
                    ][i as usize]
                } else {
                    [i as f32 / 3.0, 0.5, -0.0]
                };
                for value in values {
                    float(&mut bytes, value, big);
                }
            }
        }
    }
    word(&mut bytes, 1, big);
    let marker = if version == 2 { 0xFFFF_FFFE } else { u32::MAX };
    for value in [0, 1, 2, if quad { 3 } else { marker }] {
        word(&mut bytes, value, big);
    }
    word(&mut bytes, if attributes { vertices + 1 } else { 0 }, big);
    if attributes {
        for i in 0..=vertices {
            float(&mut bytes, i as f32 * 0.25, big);
            float(&mut bytes, if i == vertices { -1.25 } else { 0.75 }, big);
        }
    }
    word(&mut bytes, u32::from(attributes), big);
    if attributes {
        for value in [vertices, 1, 2, if quad { 3 } else { marker }] {
            word(&mut bytes, value, big);
        }
    }
    bytes
}
fn fixture(version: u32, big: bool, attributes: bool, quad: bool) -> Vec<u8> {
    [
        header(version, big, 1),
        one_mesh(version, big, attributes, quad),
    ]
    .concat()
}
fn set_word(data: &mut [u8], offset: usize, value: u32, big: bool) {
    data[offset..offset + 4].copy_from_slice(&if big {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    });
}
fn set_float(data: &mut [u8], offset: usize, value: f32) {
    set_word(data, offset, value.to_bits(), false);
}
#[test]
fn independent_v3_little_endian_fields_match_exact_expectations() {
    let data = fixture(3, false, true, true);
    let scene = SglScene::from_bytes(&data).unwrap();
    assert_eq!(scene.version, 3);
    assert_eq!(scene.byte_order, SglByteOrder::LittleEndian);
    assert_eq!(
        (
            scene.display.as_ref().unwrap().show_grid,
            scene.display.as_ref().unwrap().show_symmetry_line,
            scene.display.as_ref().unwrap().show_contour
        ),
        (true, false, true)
    );
    assert_eq!(
        (
            scene.camera.as_ref().unwrap().projection,
            scene.camera.as_ref().unwrap().mode,
            scene.camera.as_ref().unwrap().fov_degrees,
            scene.camera.as_ref().unwrap().use_pivot
        ),
        (1, 2, 63.5, true)
    );
    let mesh = &scene.meshes[0];
    assert_eq!(
        (
            mesh.render.as_ref().unwrap().shader,
            mesh.render.as_ref().unwrap().matcap,
            mesh.render.as_ref().unwrap().show_wireframe,
            mesh.render.as_ref().unwrap().flat_shading,
            mesh.render.as_ref().unwrap().opacity
        ),
        (5, 17, true, false, 0.75)
    );
    assert_eq!(mesh.center[0].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(mesh.scale, 123.25);
    assert_eq!(mesh.matrix[10], -4.0);
    assert_eq!(&mesh.matrix[12..], [5.0, -6.0, 7.0, 1.0]);
    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.faces, [[0, 1, 2, 3]]);
    assert_eq!(mesh.materials.as_ref().unwrap()[1], [0.2, 0.8, 0.0]);
    assert_eq!(mesh.uv.as_ref().unwrap().coordinates.len(), 5);
    assert_eq!(mesh.uv.as_ref().unwrap().face_indices, [[4, 1, 2, 3]]);
    assert_eq!(mesh.face_arity(0, 3).unwrap(), 4);
    assert_eq!(scene.to_bytes().unwrap(), data);
}
#[test]
fn independent_v3_big_endian_is_detected_and_preserved() {
    let data = fixture(3, true, true, false);
    let scene = SglScene::from_bytes(&data).unwrap();
    assert_eq!(scene.byte_order, SglByteOrder::BigEndian);
    assert_eq!(scene.to_bytes().unwrap(), data);
    assert_eq!(scene.meshes[0].faces, [[0, 1, 2, u32::MAX]]);
}
#[test]
fn independent_v2_negative_triangle_marker_bits_are_not_canonicalized() {
    for big in [false, true] {
        let data = fixture(2, big, true, false);
        let scene = SglScene::from_bytes(&data).unwrap();
        assert_eq!(scene.version, 2);
        assert_eq!(scene.meshes[0].faces[0][3], 0xFFFF_FFFE);
        assert_eq!(
            scene.meshes[0].uv.as_ref().unwrap().face_indices[0][3],
            0xFFFF_FFFE
        );
        assert_eq!(scene.meshes[0].face_arity(0, 2).unwrap(), 3);
        assert_eq!(scene.to_bytes().unwrap(), data);
    }
}
#[test]
fn v2_quad_records_remain_quads() {
    let data = fixture(2, false, true, true);
    let scene = SglScene::from_bytes(&data).unwrap();
    assert_eq!(scene.meshes[0].face_arity(0, 2).unwrap(), 4);
    assert_eq!(scene.to_bytes().unwrap(), data);
}
#[test]
fn empty_scene_round_trips_both_supported_versions_and_orders() {
    for version in [2, 3] {
        for big in [false, true] {
            let data = header(version, big, 0);
            let scene = SglScene::from_bytes(&data).unwrap();
            assert!(scene.meshes.is_empty());
            assert_eq!(scene.to_bytes().unwrap(), data);
        }
    }
}
#[test]
fn absent_channels_stay_absent_instead_of_fabricated_defaults() {
    let data = fixture(3, false, false, false);
    let scene = SglScene::from_bytes(&data).unwrap();
    let mesh = &scene.meshes[0];
    assert!(mesh.colors.is_none());
    assert!(mesh.materials.is_none());
    assert!(mesh.uv.is_none());
    assert_eq!(scene.to_bytes().unwrap(), data);
}
#[test]
fn multimesh_order_different_arity_and_raw_metadata_are_retained() {
    let data = [
        header(3, false, 2),
        one_mesh(3, false, true, false),
        one_mesh(3, false, false, true),
    ]
    .concat();
    let scene = SglScene::from_bytes(&data).unwrap();
    assert_eq!(scene.meshes.len(), 2);
    assert_eq!(scene.meshes[0].face_arity(0, 3).unwrap(), 3);
    assert_eq!(scene.meshes[1].face_arity(0, 3).unwrap(), 4);
    assert!(scene.meshes[0].colors.is_some());
    assert!(scene.meshes[1].colors.is_none());
    assert_eq!(scene.to_bytes().unwrap(), data);
}
#[test]
fn every_truncated_prefix_of_small_independent_fixture_rejects() {
    let data = fixture(3, false, true, true);
    for cut in 0..data.len() {
        assert!(
            SglScene::from_bytes(&data[..cut]).is_err(),
            "accepted prefix {cut}"
        );
    }
}
#[test]
fn undeclared_trailing_words_and_unaligned_bytes_reject() {
    let data = fixture(3, false, true, true);
    for tail in [vec![0], vec![0; 4], vec![255; 8]] {
        let mut bad = data.clone();
        bad.extend(tail);
        assert!(SglScene::from_bytes(&bad).is_err());
    }
}
#[test]
fn unsupported_versions_are_not_relabelled() {
    for version in [0, 4, u32::MAX] {
        let data = header(version, false, 0);
        assert!(SglScene::from_bytes(&data).is_err());
    }
}
#[test]
fn hostile_scene_and_vertex_counts_fail_before_reserving_payload() {
    let mut data = fixture(3, false, false, false);
    set_word(&mut data, 32, (MAX_SUBTOOLS + 1) as u32, false);
    assert!(SglScene::from_bytes(&data)
        .unwrap_err()
        .contains("mesh count"));
    for count in [0, MAX_VERTICES as u32 + 1, u32::MAX] {
        let mut data = fixture(3, false, false, false);
        set_word(&mut data, 136, count, false);
        assert!(SglScene::from_bytes(&data).is_err());
    }
}
#[test]
fn hostile_face_and_uv_counts_are_bounded() {
    let valid = fixture(3, false, false, false);
    for (offset, cap) in [
        (184, MAX_TRIANGLES),
        (204, MAX_SGL_UV_COORDS),
        (208, MAX_TRIANGLES),
    ] {
        let mut bad = valid.clone();
        set_word(&mut bad, offset, (cap + 1) as u32, false);
        assert!(SglScene::from_bytes(&bad).is_err());
    }
}
#[test]
fn incomplete_optional_vertex_channels_are_errors() {
    for offset in [176, 180] {
        let mut bad = fixture(3, false, false, false);
        set_word(&mut bad, offset, 1, false);
        assert!(SglScene::from_bytes(&bad).is_err());
    }
}
#[test]
fn negative_first_corner_and_out_of_range_quad_corner_reject() {
    for (offset, value) in [(188, u32::MAX), (192, 3), (200, 3)] {
        let mut bad = fixture(3, false, false, false);
        set_word(&mut bad, offset, value, false);
        assert!(SglScene::from_bytes(&bad).is_err());
    }
}
#[test]
fn repeated_geometric_corners_reject_without_touching_original_bytes() {
    let mut bad = fixture(3, false, false, false);
    set_word(&mut bad, 192, 0, false);
    let before = bad.clone();
    assert!(SglScene::from_bytes(&bad).is_err());
    assert_eq!(bad, before);
}
#[test]
fn v3_other_negative_words_are_not_silently_triangle_markers() {
    let mut bad = fixture(3, false, false, false);
    set_word(&mut bad, 200, 0xFFFF_FFFE, false);
    assert!(SglScene::from_bytes(&bad).is_err());
}
#[test]
fn all_v2_negative_fourth_markers_preserve_raw_bits() {
    for marker in [0x8000_0000, 0xFFFF_FFFD, u32::MAX] {
        let mut data = fixture(2, false, false, false);
        set_word(&mut data, 200, marker, false);
        let scene = SglScene::from_bytes(&data).unwrap();
        assert_eq!(scene.meshes[0].faces[0][3], marker);
        assert_eq!(scene.to_bytes().unwrap(), data);
    }
}
#[test]
fn noncanonical_flags_and_invalid_camera_enums_reject() {
    for offset in [4, 8, 12, 28, 44, 48] {
        let mut bad = fixture(3, false, false, false);
        set_word(&mut bad, offset, 2, false);
        assert!(SglScene::from_bytes(&bad).is_err());
    }
    for (offset, value) in [(16, 2), (20, 3), (36, 13)] {
        let mut bad = fixture(3, false, false, false);
        set_word(&mut bad, offset, value, false);
        assert!(SglScene::from_bytes(&bad).is_err());
    }
}
#[test]
fn invalid_fov_and_opacity_reject_finite_and_nonfinite_ranges() {
    for value in [0.0, -1.0, 180.0, 1000.0, f32::NAN, f32::INFINITY] {
        let mut bad = fixture(3, false, false, false);
        set_float(&mut bad, 24, value);
        assert!(SglScene::from_bytes(&bad).is_err());
    }
    for value in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        let mut bad = fixture(3, false, false, false);
        set_float(&mut bad, 52, value);
        assert!(SglScene::from_bytes(&bad).is_err());
    }
}
#[test]
fn invalid_transform_nonfinite_projective_singular_or_scale_reject() {
    for (offset, value) in [
        (56, f32::NAN),
        (68, f32::INFINITY),
        (80, 0.1),
        (128, 0.0),
        (132, 0.0),
        (132, -1.0),
        (132, f32::MAX),
        (68, 0.0),
    ] {
        let mut bad = fixture(3, false, false, false);
        set_float(&mut bad, offset, value);
        assert!(
            SglScene::from_bytes(&bad).is_err(),
            "offset{offset} value{value}"
        );
    }
}
#[test]
fn reflection_shear_and_redundant_scale_mismatch_preserve_exact_bits() {
    let data = fixture(3, false, false, false);
    let scene = SglScene::from_bytes(&data).unwrap();
    assert_eq!(scene.meshes[0].matrix[4], 0.5);
    assert_eq!(scene.meshes[0].matrix[10], -4.0);
    assert_eq!(scene.meshes[0].scale, 123.25);
    assert_eq!(scene.to_bytes().unwrap(), data);
}
#[test]
fn excessive_local_or_transformed_coordinates_reject() {
    for (offset, value) in [(140, 1_000_001.0), (116, 1_000_001.0), (140, 999_999.0)] {
        let mut bad = fixture(3, false, false, false);
        set_float(&mut bad, offset, value);
        assert!(SglScene::from_bytes(&bad).is_err());
    }
}
#[test]
fn raw_material_masking_channel_is_preserved_without_inversion() {
    let data = fixture(3, false, true, false);
    let scene = SglScene::from_bytes(&data).unwrap();
    let materials = scene.meshes[0].materials.as_ref().unwrap();
    assert_eq!(
        materials.iter().map(|v| v[2]).collect::<Vec<_>>(),
        [1.0, 0.0, 1.0]
    );
    assert_eq!(scene.to_bytes().unwrap(), data);
}
#[test]
fn bad_color_material_and_uv_numbers_reject() {
    let scene = SglScene::from_bytes(&fixture(3, false, true, true)).unwrap();
    for field in [0, 1, 2] {
        let mut bad = scene.clone();
        match field {
            0 => bad.meshes[0].colors.as_mut().unwrap()[0][0] = 1.1,
            1 => bad.meshes[0].materials.as_mut().unwrap()[0][2] = f32::NAN,
            _ => bad.meshes[0].uv.as_mut().unwrap().coordinates[0][0] = f32::INFINITY,
        };
        assert!(bad.to_bytes().is_err());
    }
}
#[test]
fn uv_mapping_count_missing_coordinates_arity_and_missing_indices_reject() {
    let scene = SglScene::from_bytes(&fixture(3, false, true, true)).unwrap();
    for mode in 0..4 {
        let mut bad = scene.clone();
        let uv = bad.meshes[0].uv.as_mut().unwrap();
        match mode {
            0 => uv.face_indices.clear(),
            1 => uv.coordinates.clear(),
            2 => uv.face_indices[0][3] = u32::MAX,
            _ => uv.face_indices[0][0] = 999,
        };
        assert!(bad.to_bytes().is_err());
    }
}
#[test]
fn uv_repeated_corner_indices_and_tiled_coordinates_remain_valid() {
    let mut scene = SglScene::from_bytes(&fixture(3, false, true, true)).unwrap();
    scene.meshes[0].uv.as_mut().unwrap().face_indices[0] = [0, 0, 0, 0];
    scene.meshes[0].uv.as_mut().unwrap().coordinates[0] = [-10.0, 20.0];
    let data = scene.to_bytes().unwrap();
    let decoded = SglScene::from_bytes(&data).unwrap();
    assert_eq!(
        decoded.meshes[0].uv.as_ref().unwrap().face_indices[0],
        [0; 4]
    );
    assert_eq!(
        decoded.meshes[0].uv.as_ref().unwrap().coordinates[0],
        [-10.0, 20.0]
    );
}
#[test]
fn triangle_and_quad_face_uv_marker_semantics_must_match() {
    let mut scene = SglScene::from_bytes(&fixture(2, false, true, false)).unwrap();
    scene.meshes[0].uv.as_mut().unwrap().face_indices[0][3] = 0;
    assert!(scene.to_bytes().is_err());
}
#[test]
fn encoding_validates_user_mutated_records_before_allocating_output() {
    let scene = SglScene::from_bytes(&fixture(3, false, true, true)).unwrap();
    for mode in 0..6 {
        let mut bad = scene.clone();
        match mode {
            0 => bad.version = 4,
            1 => bad.meshes[0].positions.clear(),
            2 => bad.meshes[0].faces.clear(),
            3 => {
                bad.meshes[0].colors.as_mut().unwrap().pop().unwrap();
            }
            4 => bad.camera.as_mut().unwrap().mode = 99,
            _ => bad.meshes = vec![bad.meshes[0].clone(); 33],
        };
        assert!(bad.to_bytes().is_err());
    }
}
#[test]
fn original_nonplanar_quads_are_preserved_without_forced_triangulation() {
    let mut scene = SglScene::from_bytes(&fixture(3, false, true, true)).unwrap();
    scene.meshes[0].positions[2][2] = 0.2;
    let data = scene.to_bytes().unwrap();
    let decoded = SglScene::from_bytes(&data).unwrap();
    assert_eq!(decoded.meshes[0].faces, [[0, 1, 2, 3]]);
    assert_eq!(decoded.meshes[0].positions[2][2], 0.2);
}
#[test]
fn mesh_arity_accessor_rejects_bad_version_or_index() {
    let scene = SglScene::from_bytes(&fixture(3, false, false, false)).unwrap();
    assert!(scene.meshes[0].face_arity(1, 3).is_err());
    assert!(scene.meshes[0].face_arity(0, 4).is_err());
}
#[test]
fn aggregate_vertices_reject_before_second_vertex_array_allocation() {
    let n = 500_001;
    let mut first = metadata(false);
    word(&mut first, n, false);
    for i in 0..n {
        for value in [
            if i == 1 { 1.0 } else { 0.0 },
            if i == 2 { 1.0 } else { 0.0 },
            0.0,
        ] {
            float(&mut first, value, false);
        }
    }
    for value in [0, 0, 1, 0, 1, 2, u32::MAX, 0, 0] {
        word(&mut first, value, false);
    }
    let mut second = metadata(false);
    word(&mut second, n, false);
    let bytes = [header(3, false, 2), first, second].concat();
    assert!(SglScene::from_bytes(&bytes)
        .unwrap_err()
        .contains("aggregate vertex"));
}
#[test]
fn aggregate_quad_equivalent_triangle_limit_is_checked() {
    let mut scene = SglScene::from_bytes(&fixture(3, false, false, true)).unwrap();
    scene.meshes[0].faces = vec![[0, 1, 2, 3]; MAX_TRIANGLES / 2 + 1];
    assert!(scene.validate().unwrap_err().contains("aggregate triangle"));
}
#[test]
fn actual_empty_export_byte_pattern_is_supported_without_fabricated_mesh() {
    let words = [3_u32, 1, 0, 0, 0, 0, 45_f32.to_bits(), 1, 0];
    let data = words
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    let scene = SglScene::from_bytes(&data).unwrap();
    assert!(scene.meshes.is_empty());
    assert_eq!(scene.camera.as_ref().unwrap().fov_degrees, 45.0);
    assert_eq!(scene.to_bytes().unwrap(), data);
}

// Append after freezing the initial modern codec/test cohort.
fn legacy_fixture(version: u32, big: bool, attributes: bool, quad: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    word(&mut bytes, version, big);
    word(&mut bytes, 1, big);
    // Legacy layout omits exactly the modern five-word mesh render block.
    let mesh = one_mesh(2, big, attributes, quad);
    bytes.extend_from_slice(&mesh[20..]);
    bytes
}
#[test]
fn legacy_v1_both_orders_preserve_geometry_material_uv_quad_and_absence() {
    for big in [false, true] {
        for quad in [false, true] {
            let bytes = legacy_fixture(1, big, true, quad);
            let scene = SglScene::from_bytes(&bytes).unwrap();
            assert_eq!(scene.version, 1);
            assert!(scene.display.is_none());
            assert!(scene.camera.is_none());
            assert!(scene.meshes[0].render.is_none());
            assert!(scene.meshes[0].materials.is_some());
            assert!(scene.meshes[0].uv.is_some());
            assert_eq!(
                scene.meshes[0].face_arity(0, 1).unwrap(),
                if quad { 4 } else { 3 }
            );
            assert_eq!(scene.to_bytes().unwrap(), bytes);
        }
    }
}
#[test]
fn legacy_v1_empty_scene_has_original_eight_byte_header() {
    for big in [false, true] {
        let mut bytes = Vec::new();
        word(&mut bytes, 1, big);
        word(&mut bytes, 0, big);
        let scene = SglScene::from_bytes(&bytes).unwrap();
        assert!(scene.meshes.is_empty());
        assert_eq!(scene.to_bytes().unwrap(), bytes);
    }
}
#[test]
fn source_derived_v0_requires_explicit_order_and_preserves_original_version() {
    for (big, order) in [
        (false, SglByteOrder::LittleEndian),
        (true, SglByteOrder::BigEndian),
    ] {
        let bytes = legacy_fixture(0, big, true, false);
        assert!(SglScene::from_bytes(&bytes)
            .unwrap_err()
            .contains("explicit"));
        let scene = SglScene::from_bytes_with_order(&bytes, order).unwrap();
        assert_eq!(scene.version, 0);
        assert_eq!(scene.byte_order, order);
        assert!(scene.camera.is_none());
        assert_eq!(scene.to_bytes().unwrap(), bytes);
        let wrong = if big {
            SglByteOrder::LittleEndian
        } else {
            SglByteOrder::BigEndian
        };
        assert!(SglScene::from_bytes_with_order(&bytes, wrong).is_err());
    }
}
#[test]
fn source_derived_v0_empty_is_not_heuristically_assigned_an_order() {
    let bytes = [0_u8; 8];
    assert!(SglScene::from_bytes(&bytes).is_err());
    for order in [SglByteOrder::LittleEndian, SglByteOrder::BigEndian] {
        let scene = SglScene::from_bytes_with_order(&bytes, order).unwrap();
        assert_eq!(scene.byte_order, order);
        assert_eq!(scene.to_bytes().unwrap(), bytes);
    }
}
#[test]
fn legacy_metadata_cannot_be_invented_or_modern_metadata_silently_removed() {
    let legacy = SglScene::from_bytes(&legacy_fixture(1, false, true, true)).unwrap();
    let modern = SglScene::from_bytes(&fixture(3, false, true, true)).unwrap();
    let mut bad = legacy.clone();
    bad.camera = modern.camera.clone();
    assert!(bad.to_bytes().is_err());
    let mut bad = legacy.clone();
    bad.display = modern.display.clone();
    assert!(bad.to_bytes().is_err());
    let mut bad = legacy;
    bad.meshes[0].render = modern.meshes[0].render.clone();
    assert!(bad.to_bytes().is_err());
    for mode in 0..3 {
        let mut bad = modern.clone();
        match mode {
            0 => bad.camera = None,
            1 => bad.display = None,
            _ => bad.meshes[0].render = None,
        }
        assert!(bad.to_bytes().is_err());
    }
}
#[test]
fn legacy_multimesh_order_and_arity_round_trip_exactly() {
    let a = legacy_fixture(1, false, true, false);
    let b = legacy_fixture(1, false, false, true);
    let mut bytes = Vec::new();
    word(&mut bytes, 1, false);
    word(&mut bytes, 2, false);
    bytes.extend_from_slice(&a[8..]);
    bytes.extend_from_slice(&b[8..]);
    let scene = SglScene::from_bytes(&bytes).unwrap();
    assert_eq!(scene.meshes.len(), 2);
    assert_eq!(scene.to_bytes().unwrap(), bytes);
}
#[test]
fn explicit_order_decoder_does_not_reinterpret_unsupported_or_mismatched_versions() {
    assert!(SglScene::from_bytes_with_order(
        &fixture(3, false, false, false),
        SglByteOrder::BigEndian
    )
    .is_err());
    assert!(
        SglScene::from_bytes_with_order(&header(4, false, 0), SglByteOrder::LittleEndian).is_err()
    );
}
#[test]
fn legacy_truncated_prefixes_are_rejected_without_inventing_missing_fields() {
    let bytes = legacy_fixture(1, false, true, true);
    for cut in 0..bytes.len() {
        assert!(SglScene::from_bytes(&bytes[..cut]).is_err());
    }
}
