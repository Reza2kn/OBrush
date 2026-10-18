//! Independent word fixtures and primary-equation literals SC00..SC06.
//! No production encoder or fixture outcome is used to create these inputs.
use obrush_core::*;
use std::sync::Arc;
const ZERO: u32 = 0;
const ONE: u32 = 0x3f800000;
const THREE: u32 = 0x40400000;
const FOUR: u32 = 0x40800000;
const NEG_ZERO: u32 = 0x80000000;
fn bytes(points: &[[u32; 3]], faces: &[[u32; 4]], objects: usize) -> Vec<u8> {
    let mut words = vec![3, 0, 0, 0, 0, 0, 0x42340000, 1, objects as u32];
    for _ in 0..objects {
        words.extend([5, 3, 0, 0, ONE]);
        words.extend([NEG_ZERO, 0x41100000, 0x40e00000]);
        words.extend([ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE]);
        words.push(ONE);
        words.push(points.len() as u32);
        for p in points {
            words.extend(p);
        }
        words.extend([0, 0]);
        words.push(faces.len() as u32);
        for f in faces {
            words.extend(f);
        }
        words.extend([0, 0]);
    }
    words.into_iter().flat_map(u32::to_le_bytes).collect()
}
fn tri() -> [[u32; 4]; 1] {
    [[0, 1, 2, u32::MAX]]
}
fn stage(input: &[u8]) -> StagedSglScene {
    StagedSglScene::from_sgl_bytes(input, &SglStageSettings::default()).unwrap()
}
fn prepared(input: &[u8], scale_center: bool, vertex_srgb: bool) -> PreparedSourceImportOptions {
    stage(input)
        .prepare_source_import_options(SourceImportOptions {
            scale_center,
            vertex_srgb,
        })
        .unwrap()
}
fn centers(input: &[u8]) -> [u32; 3] {
    prepared(input, false, true).staged.objects[0]
        .affine
        .interaction_center
        .map(f32::to_bits)
}
#[test]
fn sc00_source_default_color_and_normalized_centers_are_positive_zero_raw_positions_stay_negative()
{
    let input = include_bytes!("fixtures/source-import-options/flat-negative-zero-v3-le.sgl");
    for scale in [false, true] {
        for srgb in [false, true] {
            let p = prepared(input, scale, srgb);
            assert_eq!(
                p.staged.objects[0]
                    .affine
                    .interaction_center
                    .map(f32::to_bits),
                [0, 0, 0]
            );
            assert!(p.staged.objects[0]
                .mesh
                .positions
                .iter()
                .all(|v| v.z.to_bits() == NEG_ZERO));
            assert!(p.staged.objects[0]
                .origin
                .raw
                .positions
                .iter()
                .all(|v| v[2].to_bits() == NEG_ZERO));
            assert_eq!(p.report.initialized_centers[0].center_words, [0, 0, 0]);
        }
    }
}
#[test]
fn general_staging_keeps_prior_tight_center_until_source_options_are_applied() {
    let input = include_bytes!("fixtures/source-import-options/flat-negative-zero-v3-le.sgl");
    assert_eq!(
        stage(input).objects[0].affine.interaction_center[2].to_bits(),
        NEG_ZERO
    );
}
#[test]
fn sc01_nondegenerate_yz_triangle_collapsed_negative_zero_x() {
    let input = bytes(
        &[[NEG_ZERO, 0, 0], [NEG_ZERO, THREE, 0], [NEG_ZERO, 0, FOUR]],
        &tri(),
        1,
    );
    assert_eq!(centers(&input), [ZERO, 0x3fc00000, 0x40000000]);
}
#[test]
fn sc02_nondegenerate_xz_triangle_collapsed_negative_zero_y() {
    let input = bytes(
        &[[0, NEG_ZERO, 0], [THREE, NEG_ZERO, 0], [0, NEG_ZERO, FOUR]],
        &tri(),
        1,
    );
    assert_eq!(centers(&input), [0x3fc00000, ZERO, 0x40000000]);
}
#[test]
fn sc03_positive_minimum_subnormal_is_retained_raw_but_loose_center_rounds_positive_zero() {
    let input = bytes(&[[0, 0, 1], [THREE, 0, 1], [0, FOUR, 1]], &tri(), 1);
    let p = prepared(&input, false, true);
    assert_eq!(
        p.staged.objects[0]
            .affine
            .interaction_center
            .map(f32::to_bits),
        [0x3fc00000, 0x40000000, ZERO]
    );
    assert!(p.staged.objects[0]
        .mesh
        .positions
        .iter()
        .all(|v| v.z.to_bits() == 1));
}
#[test]
fn sc04_negative_minimum_subnormal_is_retained_raw_but_loose_center_rounds_positive_zero() {
    let z = 0x80000001;
    let input = bytes(&[[0, 0, z], [THREE, 0, z], [0, FOUR, z]], &tri(), 1);
    let p = prepared(&input, false, true);
    assert_eq!(
        p.staged.objects[0]
            .affine
            .interaction_center
            .map(f32::to_bits),
        [0x3fc00000, 0x40000000, ZERO]
    );
    assert!(p.staged.objects[0]
        .mesh
        .positions
        .iter()
        .all(|v| v.z.to_bits() == z));
}
#[test]
fn sc05_nondegenerate_closed_tetrahedron_keeps_negative_zero_geometry_words() {
    let input = bytes(
        &[
            [NEG_ZERO, NEG_ZERO, NEG_ZERO],
            [ONE, 0, 0],
            [0, ONE, 0],
            [0, 0, ONE],
        ],
        &[
            [0, 2, 1, u32::MAX],
            [0, 1, 3, u32::MAX],
            [0, 3, 2, u32::MAX],
            [1, 2, 3, u32::MAX],
        ],
        1,
    );
    let p = prepared(&input, false, true);
    assert_eq!(
        p.staged.objects[0]
            .affine
            .interaction_center
            .map(f32::to_bits),
        [0x3f000000; 3]
    );
    assert_eq!(
        p.staged.objects[0].mesh.positions[0]
            .to_array()
            .map(f32::to_bits),
        [NEG_ZERO; 3]
    );
}
#[test]
fn sc06_unreferenced_far_geometry_is_preserved_and_excluded_only_from_current_center() {
    let ninety_nine = 0x42c60000;
    let points = [
        [NEG_ZERO, 0, 0],
        [NEG_ZERO, THREE, 0],
        [NEG_ZERO, 0, FOUR],
        [ninety_nine; 3],
    ];
    let input = bytes(&points, &tri(), 1);
    let before = stage(&input);
    let p = prepared(&input, false, true);
    assert_eq!(
        p.staged.objects[0]
            .affine
            .interaction_center
            .map(f32::to_bits),
        [0, 0x3fc00000, 0x40000000]
    );
    assert_eq!(p.staged.objects[0].mesh.positions.len(), 4);
    assert_eq!(
        p.staged.objects[0].mesh.positions,
        before.objects[0].mesh.positions
    );
    assert_eq!(
        p.staged.objects[0].origin.raw.positions,
        before.objects[0].origin.raw.positions
    );
}
#[test]
fn center_phase_keeps_origin_arc_render_channels_and_matrix_and_charges_actual_report_capacity() {
    let input = include_bytes!("fixtures/source-import-options/flat-negative-zero-v3-le.sgl");
    let before = stage(input);
    let origin = before.objects[0].origin.raw.clone();
    let mesh = before.objects[0].mesh.clone();
    let matrix = before.objects[0].affine.local_to_world;
    let p = before
        .prepare_source_import_options(Default::default())
        .unwrap();
    let object = &p.staged.objects[0];
    assert!(Arc::ptr_eq(&origin, &object.origin.raw));
    assert_eq!(object.mesh.positions, mesh.positions);
    assert_eq!(object.mesh.normals, mesh.normals);
    assert_eq!(object.mesh.colors, mesh.colors);
    assert_eq!(object.mesh.masks, mesh.masks);
    assert_eq!(object.mesh.triangles, mesh.triangles);
    assert_eq!(object.affine.local_to_world, matrix);
    let row = &p.report.initialized_centers[0];
    assert_eq!(row.ordinal, 0);
    assert_eq!(row.previous_center_words[2], NEG_ZERO);
    assert_eq!(row.archived_center_words, origin.center.map(f32::to_bits));
    assert_eq!(row.referenced_triangle_corners, 6);
    assert_eq!(
        row.reason,
        SourceImportCenterReason::InitialLooseReferencedF32V1
    );
    assert_eq!(
        p.report.options_storage_bytes_bound,
        p.report.initialized_centers.capacity() * std::mem::size_of::<SourceImportCenterReport>()
            + std::mem::size_of::<[[u32; 3]; MAX_SUBTOOLS]>()
            + 128
    );
}
#[test]
fn thirty_two_reports_have_stable_ordinals_and_empty_scene_has_no_report_or_storage() {
    let input = bytes(
        &[[NEG_ZERO, 0, 0], [NEG_ZERO, THREE, 0], [NEG_ZERO, 0, FOUR]],
        &tri(),
        32,
    );
    let p = prepared(&input, false, true);
    assert_eq!(p.report.initialized_centers.len(), 32);
    for (i, row) in p.report.initialized_centers.iter().enumerate() {
        assert_eq!(row.ordinal, i);
        assert_eq!(row.center_words, [0, 0x3fc00000, 0x40000000]);
    }
    let empty = bytes(&[], &[], 0);
    let p = prepared(&empty, false, true);
    assert!(p.report.initialized_centers.is_empty());
    assert_eq!(p.report.options_storage_bytes_bound, 0);
}
#[test]
fn source_center_metadata_survives_save_reload_undo_redo_and_current_sgl_export() {
    let input = include_bytes!("fixtures/source-import-options/flat-negative-zero-v3-le.sgl");
    let p = prepared(input, false, true);
    let mut doc = Document::empty();
    let before = doc.to_json().unwrap();
    doc.append_staged_sgl_scene(p.staged).unwrap();
    let after = doc.to_json().unwrap();
    assert_eq!(
        Document::from_json(&after).unwrap().to_json().unwrap(),
        after
    );
    assert!(doc.undo());
    assert_eq!(doc.to_json().unwrap(), before);
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), after);
    let origin = doc.scene_origin().unwrap();
    let settings = SglExportSettings {
        scope: SglExportScope::AllScene,
        byte_order: SglByteOrder::LittleEndian,
        display: origin.display.clone().unwrap(),
        camera: origin.camera.clone().unwrap(),
        plain_render: baseline_source_render_defaults(),
        legacy_constructor_render: baseline_source_render_defaults(),
        plain_appearance_conversion: false,
        declared_display_defaults: false,
        loss_policy: SglExportLossPolicy::AllowAndReport,
    };
    let output = doc.export_sgl(&settings).unwrap();
    let current = SglScene::from_bytes(&output.bytes).unwrap();
    assert_eq!(current.meshes[0].center.map(f32::to_bits), [0, 0, 0]);
    assert!(current.meshes[0]
        .positions
        .iter()
        .all(|v| v[2].to_bits() == NEG_ZERO));
}
#[test]
fn malformed_second_object_cannot_install_into_a_receiving_document_with_redo() {
    let input = bytes(
        &[[NEG_ZERO, 0, 0], [NEG_ZERO, THREE, 0], [NEG_ZERO, 0, FOUR]],
        &tri(),
        2,
    );
    let mut candidate = stage(&input);
    candidate.objects[1].mesh.triangles[0][0] = u32::MAX;
    let mut doc = Document::new(Mesh::sphere(0));
    doc.add_subtool(Mesh::sphere(0), "Prior redo").unwrap();
    assert!(doc.undo());
    let before = doc.to_json().unwrap();
    let result = candidate
        .prepare_source_import_options(Default::default())
        .and_then(|p| doc.append_staged_sgl_scene(p.staged));
    assert!(result.is_err());
    assert_eq!(doc.to_json().unwrap(), before);
    assert!(doc.can_redo());
}
