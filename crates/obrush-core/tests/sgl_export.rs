//! Independent binary words, exact channels and neutral-history export contracts.
use obrush_core::*;
fn word(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn f(b: &mut Vec<u8>, v: f32) {
    word(b, v.to_bits());
}
fn fixture(version: u32, absent: bool, negative: bool) -> Vec<u8> {
    let mut b = Vec::new();
    word(&mut b, version);
    if version >= 2 {
        for v in [1, 0, 0, 0, 0] {
            word(&mut b, v);
        }
        f(&mut b, 40.0);
        word(&mut b, 1);
    }
    word(&mut b, 1);
    if version >= 2 {
        for v in [5, 3, 0, 0] {
            word(&mut b, v);
        }
        f(&mut b, 1.0);
    }
    for v in [9.0, 8.0, 7.0] {
        f(&mut b, v);
    }
    for v in [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 12.0, 0.0, 0.0, 1.0,
    ] {
        f(&mut b, v);
    }
    f(&mut b, 99.0);
    word(&mut b, 3);
    for p in [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]] {
        for v in p {
            f(&mut b, v);
        }
    }
    word(&mut b, if absent { 0 } else { 3 });
    if !absent {
        for rgb in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
            for v in rgb {
                f(&mut b, v);
            }
        }
    }
    word(&mut b, if absent { 0 } else { 3 });
    if !absent {
        for z in [1u32, 0x80000000, 0x3f800000] {
            f(&mut b, 0.3);
            f(&mut b, 0.0);
            word(&mut b, z);
        }
    }
    let marker = if negative { 0xfffffff9 } else { u32::MAX };
    word(&mut b, 1);
    for v in [0, 1, 2, marker] {
        word(&mut b, v);
    }
    word(&mut b, 3);
    for uv in [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]] {
        for v in uv {
            f(&mut b, v);
        }
    }
    word(&mut b, 1);
    for v in [0, 1, 2, marker] {
        word(&mut b, v);
    }
    b
}
fn doc(bytes: &[u8]) -> Document {
    let mut d = Document::empty();
    d.replace_sgl_bytes(bytes, &SglStageSettings::default())
        .unwrap();
    d
}
fn settings() -> SglExportSettings {
    SglExportSettings {
        scope: SglExportScope::AllScene,
        byte_order: SglByteOrder::LittleEndian,
        display: SglDisplaySettings {
            show_grid: true,
            show_symmetry_line: false,
            show_contour: false,
        },
        camera: SglCameraSettings {
            projection: 0,
            mode: 0,
            fov_degrees: 40.0,
            use_pivot: true,
        },
        plain_render: baseline_source_render_defaults(),
        legacy_constructor_render: baseline_source_render_defaults(),
        plain_appearance_conversion: false,
        declared_display_defaults: false,
        loss_policy: SglExportLossPolicy::RejectLoss,
    }
}
fn allowed() -> SglExportSettings {
    SglExportSettings {
        loss_policy: SglExportLossPolicy::AllowAndReport,
        ..settings()
    }
}
#[test]
fn unchanged_v3_export_keeps_all_original_words_and_redo() {
    let bytes = fixture(3, false, false);
    let mut d = doc(&bytes);
    d.add_subtool(Mesh::sphere(0), "extra").unwrap();
    d.undo();
    let before = d.to_json().unwrap();
    let r = d.plan_sgl_export(&settings()).unwrap();
    assert_eq!(r.borrowed_unchanged_source_objects, 1);
    assert_eq!(r.mapped_scratch_bytes, 0);
    assert!(!r.requires_allow());
    let exported = d.export_sgl(&settings()).unwrap();
    assert_eq!(exported.bytes, bytes);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn canonical_v3_upgrade_keeps_surviving_v2_words_exact() {
    let bytes = fixture(2, false, false);
    let d = doc(&bytes);
    let export = d.export_sgl(&settings()).unwrap();
    assert_eq!(export.report.source_version_conversions, 1);
    assert_eq!(u32::from_le_bytes(export.bytes[..4].try_into().unwrap()), 3);
    assert_eq!(&export.bytes[4..], &bytes[4..]);
}
#[test]
fn legacy_negative_markers_need_allow_and_normalize_only_two_words() {
    let bytes = fixture(2, false, true);
    let d = doc(&bytes);
    let before = d.to_json().unwrap();
    let plan = d.plan_sgl_export(&settings()).unwrap();
    assert_eq!(
        (plan.normalized_geometry_markers, plan.normalized_uv_markers),
        (1, 1)
    );
    assert!(d.export_sgl(&settings()).is_err());
    let e = d.export_sgl(&allowed()).unwrap();
    let raw = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(raw.meshes[0].faces[0], [0, 1, 2, u32::MAX]);
    assert_eq!(
        raw.meshes[0].uv.as_ref().unwrap().face_indices[0],
        [0, 1, 2, u32::MAX]
    );
    assert_eq!(raw.meshes[0].center, [9.0, 8.0, 7.0]);
    assert_eq!(raw.meshes[0].scale, 99.0);
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn v1_defaults_are_reported_raw_absence_and_origin_center_scale_survive() {
    let bytes = fixture(1, true, true);
    let d = doc(&bytes);
    let plan = d.plan_sgl_export(&settings()).unwrap();
    assert_eq!(plan.defaulted_header_fields, 7);
    assert_eq!(plan.defaulted_render_objects, 1);
    assert!(d.export_sgl(&settings()).is_err());
    let e = d.export_sgl(&allowed()).unwrap();
    let raw = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(raw.version, 3);
    assert!(raw.meshes[0].colors.is_none());
    assert!(raw.meshes[0].materials.is_none());
    assert_eq!(raw.meshes[0].render.as_ref().unwrap().shader, 5);
    assert_eq!(raw.meshes[0].center, [9.0, 8.0, 7.0]);
    assert_eq!(raw.meshes[0].scale, 99.0);
    assert!(d.active_object_metadata().unwrap().render.is_none());
    assert!(d
        .active_object_metadata()
        .unwrap()
        .origin
        .as_ref()
        .unwrap()
        .object
        .raw
        .materials
        .is_none());
}
#[test]
fn changed_render_keeps_unedited_source_center_scale_and_absent_channels() {
    let mut d = doc(&fixture(3, true, false));
    let mut render = d.active_object_metadata().unwrap().render.clone().unwrap();
    render.matcap = 8;
    d.set_active_source_render(render).unwrap();
    let e = d.export_sgl(&allowed()).unwrap();
    let raw = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(raw.meshes[0].render.as_ref().unwrap().matcap, 8);
    assert_eq!(raw.meshes[0].center, [9.0, 8.0, 7.0]);
    assert_eq!(raw.meshes[0].scale, 99.0);
    assert!(raw.meshes[0].colors.is_none() && raw.meshes[0].materials.is_none());
}
#[test]
fn unprotected_tiny_and_negative_zero_words_are_never_complemented() {
    let d = doc(&fixture(3, false, false));
    let e = d.export_sgl(&settings()).unwrap();
    let raw = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(
        raw.meshes[0]
            .materials
            .as_ref()
            .unwrap()
            .iter()
            .map(|v| v[2].to_bits())
            .collect::<Vec<_>>(),
        vec![1, 0x80000000, 0x3f800000]
    );
    assert_eq!(e.report.protected_mask_words_rounded, 0);
}
#[test]
fn protected_min_subnormal_signed_zero_and_decimal_rounding_are_preflighted() {
    let mut d = Document::new(Mesh::sphere(0));
    d.require_active_mesh_mut().unwrap().masks[..3].copy_from_slice(&[
        f32::from_bits(1),
        -0.0,
        0.1,
    ]);
    let before = d.to_json().unwrap();
    let r = d.plan_sgl_export(&settings()).unwrap();
    assert_eq!(r.protected_mask_words_rounded, 3);
    assert!(r.maximum_mask_roundtrip_error > 2e-8);
    assert!(d.export_sgl(&settings()).is_err());
    let e = d.export_sgl(&allowed()).unwrap();
    let raw = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(raw.meshes[0].materials.as_ref().unwrap()[0][2], 1.0);
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn empty_source_scene_writes_real_v3_zero_count_and_preserves_no_history() {
    let mut b = Vec::new();
    for v in [3, 1, 0, 0, 0, 0] {
        word(&mut b, v);
    }
    f(&mut b, 40.0);
    word(&mut b, 1);
    word(&mut b, 0);
    let d = doc(&b);
    let before = d.to_json().unwrap();
    let e = d.export_sgl(&settings()).unwrap();
    assert_eq!(e.bytes, b);
    assert_eq!(e.report.objects, 0);
    assert!(d.is_empty());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn invalid_header_and_unsupported_scopes_reject_atomically() {
    let d = doc(&fixture(3, false, false));
    let before = d.to_json().unwrap();
    for scope in [SglExportScope::VisibleScene, SglExportScope::ActivePart] {
        let mut s = settings();
        s.scope = scope;
        assert!(d.plan_sgl_export(&s).is_err());
    }
    let mut s = settings();
    s.camera.fov_degrees = f32::NAN;
    assert!(d.export_sgl(&s).is_err());
    assert_eq!(d.to_json().unwrap(), before);
}

#[test]
fn selected_scope_exports_selection_order_including_hidden_part_without_mutation() {
    let mut d = doc(&fixture(3, false, false));
    let source = d.active_subtool_id().unwrap();
    let omitted = d.add_subtool(Mesh::sphere(0), "middle").unwrap();
    let last = d.add_subtool(Mesh::cube(), "last").unwrap();
    d.set_subtool_visible(source, false).unwrap();
    // Selection order is deliberately different from scene order. SculptGL's
    // export filter visits the selection array and includes hidden selections.
    d.select_subtools(&[last, source], Some(last)).unwrap();
    assert!(!d.selected_subtool_ids().contains(&omitted));
    let before = d.to_json().unwrap();
    let mut selected = allowed();
    selected.scope = SglExportScope::SelectedParts;
    let report = d.plan_sgl_export(&selected).unwrap();
    assert_eq!(report.objects, 2);
    assert_eq!(report.omitted_visibility_objects, 1);
    let export = d.export_sgl(&selected).unwrap();
    assert_eq!(export.report, report);
    assert_eq!(export.bytes.len(), report.encoded_bytes);
    let decoded = SglScene::from_bytes(&export.bytes).unwrap();
    assert_eq!(decoded.meshes.len(), 2);
    assert_eq!(
        decoded.meshes[0].positions.len(),
        Mesh::cube().positions.len()
    );
    assert_eq!(
        decoded.meshes[0].matrix,
        glam::Mat4::IDENTITY.to_cols_array()
    );
    assert_eq!(decoded.meshes[1].positions.len(), 3);
    assert_eq!(decoded.meshes[1].matrix[12].to_bits(), 12.0_f32.to_bits());
    assert_eq!(d.to_json().unwrap(), before);

    selected.loss_policy = SglExportLossPolicy::RejectLoss;
    assert!(d.export_sgl(&selected).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    let empty = Document::empty();
    selected.loss_policy = SglExportLossPolicy::AllowAndReport;
    assert!(empty.export_sgl(&selected).is_err());
}
#[test]
fn hidden_solo_names_ids_and_selection_have_exact_reports() {
    let mut d = doc(&fixture(3, false, false));
    let first = d.active_subtool_id().unwrap();
    let second = d.add_subtool(Mesh::sphere(0), "named extra").unwrap();
    d.set_subtool_visible(first, false).unwrap();
    d.select_subtool(first).unwrap();
    d.set_solo(true).unwrap();
    let r = d.plan_sgl_export(&allowed()).unwrap();
    assert_eq!(r.objects, 2);
    assert_eq!(r.omitted_visibility_objects, 1);
    assert_eq!(r.omitted_subtool_names, 1);
    assert!(r.omitted_selection && r.omitted_solo);
    d.delete_subtool(second).unwrap();
    let e = d.export_sgl(&allowed()).unwrap();
    assert_eq!(SglScene::from_bytes(&e.bytes).unwrap().meshes.len(), 1);
}
#[test]
fn header_changes_need_allow_and_output_contains_current_actual_policy() {
    let d = doc(&fixture(3, false, false));
    let mut s = settings();
    s.camera.fov_degrees = 120.0;
    s.display.show_grid = false;
    let plan = d.plan_sgl_export(&s).unwrap();
    assert_eq!(plan.changed_header_fields, 2);
    assert!(d.export_sgl(&s).is_err());
    s.loss_policy = SglExportLossPolicy::AllowAndReport;
    let e = d.export_sgl(&s).unwrap();
    let raw = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(raw.camera.unwrap().fov_degrees, 120.0);
    assert!(!raw.display.unwrap().show_grid);
}
#[test]
fn render_default_for_legacy_is_separate_from_current_plain_pbr_view() {
    let d = doc(&fixture(1, true, false));
    let mut s = allowed();
    s.plain_render.shader = 0;
    let e = d.export_sgl(&s).unwrap();
    assert_eq!(
        SglScene::from_bytes(&e.bytes).unwrap().meshes[0]
            .render
            .as_ref()
            .unwrap()
            .shader,
        5
    );
}

#[test]
fn big_endian_input_canonical_menu_output_is_little_endian_without_word_loss() {
    let little = fixture(3, false, false);
    let big: Vec<u8> = little
        .chunks_exact(4)
        .flat_map(|w| [w[3], w[2], w[1], w[0]])
        .collect();
    let d = doc(&big);
    let e = d.export_sgl(&settings()).unwrap();
    assert_eq!(e.report.source_byte_order_conversions, 1);
    assert_eq!(e.bytes, little);
    assert_eq!(
        d.scene_origin().unwrap().byte_order,
        SglByteOrder::BigEndian
    );
}
#[test]
fn active_stroke_is_an_explicit_preflight_barrier_and_no_history_is_consumed() {
    let mut d = doc(&fixture(3, false, false));
    let before = d.to_json().unwrap();
    d.begin_stroke();
    assert!(d.plan_sgl_export(&settings()).is_err());
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_redo());
}
#[test]
fn actual_local_affine_rgb_material_uv_and_quad_fields_roundtrip_after_edits() {
    let mut d = doc(&fixture(3, false, false));
    let original = d.to_json().unwrap();
    d.begin_stroke();
    let mesh = d.require_active_mesh_mut().unwrap();
    mesh.colors[1] = [0.25, 0.5, 0.75];
    mesh.materials.as_mut().unwrap()[1] = [0.125, 0.625];
    mesh.masks[1] = f32::from_bits(2);
    d.end_stroke();
    let before = d.to_json().unwrap();
    let e = d.export_sgl(&allowed()).unwrap();
    let raw = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(raw.meshes[0].matrix[12], 12.0);
    assert_eq!(raw.meshes[0].positions[0], [-1.0, -1.0, 0.0]);
    assert_eq!(raw.meshes[0].colors.as_ref().unwrap()[1], [0.25, 0.5, 0.75]);
    assert_eq!(
        raw.meshes[0].materials.as_ref().unwrap()[1],
        [0.125, 0.625, f32::from_bits(2)]
    );
    assert_eq!(
        raw.meshes[0].uv.as_ref().unwrap().face_indices[0],
        [0, 1, 2, u32::MAX]
    );
    assert_eq!(d.to_json().unwrap(), before);
    d.undo();
    assert_eq!(d.to_json().unwrap(), original);
}
#[test]
fn retained_singleton_is_reported_even_when_no_lower_or_higher_level_exists() {
    let mut d = Document::new(Mesh::sphere(0));
    d.add_subdivision_level().unwrap();
    d.delete_lower_subdivision_levels().unwrap();
    assert_eq!(d.level_count(), 1);
    let p = d.plan_sgl_export(&allowed()).unwrap();
    assert_eq!(p.omitted_retained_levels, 0);
    assert_eq!(p.omitted_retained_hierarchies, 1);
    assert!(p.requires_allow());
}

#[test]
fn original_quads_and_equal_valued_distinct_uv_seam_ids_survive() {
    let obj=b"v 0 0 0 1 0 0\nv 1 0 0 0 1 0\nv 1 1 0 0 0 1\nv 0 1 0 1 1 1\nv 2 0 0 1 0 1\nv 2 1 0 0 1 1\nvt 0 0\nvt 0.5 0\nvt 0.5 1\nvt 0 1\nvt 0.5 0\nvt 1 0\nvt 1 1\nvt 0.5 1\nf 1/1 2/2 3/3 4/4\nf 2/5 5/6 6/7 3/8\n";
    let d = Document::new(Mesh::import_obj(obj).unwrap().mesh);
    let e = d.export_sgl(&allowed()).unwrap();
    let raw = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(raw.meshes[0].faces, vec![[0, 1, 2, 3], [1, 4, 5, 2]]);
    let uv = raw.meshes[0].uv.as_ref().unwrap();
    assert_eq!(uv.coordinates[1], uv.coordinates[4]);
    assert_eq!(uv.coordinates[2], uv.coordinates[7]);
    assert_eq!(uv.face_indices, vec![[0, 1, 2, 3], [4, 5, 6, 7]]);
}
#[test]
fn hostile_live_counts_reject_preflight_before_mapped_channel_allocation() {
    let mut d = Document::new(Mesh::sphere(0));
    d.require_active_mesh_mut()
        .unwrap()
        .positions
        .resize(MAX_VERTICES + 1, Vec3::ZERO);
    assert!(d.plan_sgl_export(&allowed()).is_err());
}
