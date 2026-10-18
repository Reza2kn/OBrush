//! Independent literal rawscale7 versus currentcenter0. No display or source kernel.
use obrush_core::*;
const INPUT: &[u8] = include_bytes!("fixtures/independent-center-export-v3-le.sgl");
fn stage(bytes: &[u8]) -> StagedSglScene {
    StagedSglScene::from_sgl_bytes(bytes, &SglStageSettings::default()).unwrap()
}
fn initialized(bytes: &[u8]) -> StagedSglScene {
    stage(bytes)
        .prepare_source_import_options(SourceImportOptions {
            scale_center: false,
            vertex_srgb: true,
        })
        .unwrap()
        .staged
}
fn document(staged: StagedSglScene) -> Document {
    let mut d = Document::empty();
    d.install_staged_sgl_scene(staged).unwrap();
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
        loss_policy: SglExportLossPolicy::AllowAndReport,
    }
}
fn repeated(count: u32) -> Vec<u8> {
    let mut b = INPUT[..36].to_vec();
    b[32..36].copy_from_slice(&count.to_le_bytes());
    for _ in 0..count {
        b.extend_from_slice(&INPUT[36..]);
    }
    b
}
#[test]
fn source_options_only_marks_new_objects_for_all_four_flag_combinations() {
    assert!(!stage(INPUT).objects[0].source_center_initialized);
    for scale_center in [false, true] {
        for vertex_srgb in [false, true] {
            let p = stage(INPUT)
                .prepare_source_import_options(SourceImportOptions {
                    scale_center,
                    vertex_srgb,
                })
                .unwrap();
            assert!(p.staged.objects[0].source_center_initialized);
            assert_eq!(
                p.staged.objects[0]
                    .affine
                    .interaction_center
                    .map(f32::to_bits),
                [0; 3]
            );
            assert_eq!(
                p.staged.objects[0].origin.raw.center.map(f32::to_bits),
                [0x42c80000, 0x42b40000, 0x42a00000]
            );
        }
    }
}
#[test]
fn center_only_export_keeps_raw_scale_seven_and_all_other_original_words() {
    let raw = SglScene::from_bytes(INPUT).unwrap();
    let d = document(initialized(INPUT));
    let before = d.to_json().unwrap();
    let e = d.export_sgl(&settings()).unwrap();
    let current = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(current.meshes[0].center.map(f32::to_bits), [0; 3]);
    assert_eq!(current.meshes[0].scale.to_bits(), 0x40e00000);
    assert_eq!(e.report.changed_source_centers, 1);
    assert_eq!(e.report.changed_source_scales, 0);
    assert_eq!(e.report.maximum_derived_scale_rounding_error, 0.0);
    assert_eq!(e.report.borrowed_unchanged_source_objects, 0);
    let mut wanted = INPUT.to_vec();
    for offset in [56, 60, 64] {
        wanted[offset..offset + 4].copy_from_slice(&0u32.to_le_bytes());
    }
    assert_eq!(e.bytes, wanted);
    assert_eq!(
        current.meshes[0]
            .positions
            .iter()
            .map(|v| v.map(f32::to_bits))
            .collect::<Vec<_>>(),
        raw.meshes[0]
            .positions
            .iter()
            .map(|v| v.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(
        d.active_object_metadata()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .object
            .raw
            .center
            .map(f32::to_bits),
        [0x42c80000, 0x42b40000, 0x42a00000]
    );
}
#[test]
fn general_staging_and_old_missing_json_policy_keep_archived_center_and_scale() {
    let d = document(stage(INPUT));
    let json = d.to_json().unwrap();
    assert!(!json.contains("source_center_initialized"));
    let old = Document::from_json(&json).unwrap();
    assert!(
        !old.active_object_metadata()
            .unwrap()
            .source_center_initialized
    );
    let e = old.export_sgl(&settings()).unwrap();
    assert_eq!(e.bytes, INPUT);
    assert_eq!(e.report.changed_source_centers, 0);
    assert_eq!(e.report.changed_source_scales, 0);
    assert_eq!(e.report.borrowed_unchanged_source_objects, 1);
    assert_eq!(old.to_json().unwrap(), json);
}
#[test]
fn initialized_center_equal_to_raw_keeps_unchanged_source_borrow() {
    let mut input = INPUT.to_vec();
    for offset in [56, 60, 64] {
        input[offset..offset + 4].copy_from_slice(&0u32.to_le_bytes());
    }
    let d = document(initialized(&input));
    assert!(
        d.active_object_metadata()
            .unwrap()
            .source_center_initialized
    );
    let e = d.export_sgl(&settings()).unwrap();
    assert_eq!(e.bytes, input);
    assert_eq!(e.report.borrowed_unchanged_source_objects, 1);
    assert_eq!(e.report.changed_source_centers, 0);
    assert_eq!(e.report.changed_source_scales, 0);
}
#[test]
fn source_policy_save_reload_and_undo_redo_preserve_flag_and_raw_origin() {
    let mut d = Document::empty();
    let before = d.to_json().unwrap();
    d.append_staged_sgl_scene(initialized(INPUT)).unwrap();
    let after = d.to_json().unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&after).unwrap()["scene_v7"]["objects"][0]
            ["source_center_initialized"],
        true
    );
    let reloaded = Document::from_json(&after).unwrap();
    assert_eq!(reloaded.to_json().unwrap(), after);
    assert!(
        reloaded
            .active_object_metadata()
            .unwrap()
            .source_center_initialized
    );
    assert_eq!(
        reloaded.export_sgl(&settings()).unwrap().bytes,
        d.export_sgl(&settings()).unwrap().bytes
    );
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert!(
        d.active_object_metadata()
            .unwrap()
            .source_center_initialized
    );
}
#[test]
fn mixed_append_keeps_old_policy_false_and_last_new_selected_with_one_history_group() {
    let mut d = document(stage(INPUT));
    let original = d.to_json().unwrap();
    let prior_undo = d.can_undo();
    d.append_staged_sgl_scene(initialized(INPUT)).unwrap();
    assert_eq!(d.active_subtool_id(), Some(2));
    assert_eq!(d.selected_subtool_ids(), vec![2]);
    assert!(
        !d.subtool_object_metadata(1)
            .unwrap()
            .unwrap()
            .source_center_initialized
    );
    assert!(
        d.subtool_object_metadata(2)
            .unwrap()
            .unwrap()
            .source_center_initialized
    );
    let e = d.export_sgl(&settings()).unwrap();
    let current = SglScene::from_bytes(&e.bytes).unwrap();
    assert_eq!(
        current.meshes[0].center.map(f32::to_bits),
        [0x42c80000, 0x42b40000, 0x42a00000]
    );
    assert_eq!(current.meshes[1].center.map(f32::to_bits), [0; 3]);
    assert_eq!(e.report.changed_source_centers, 1);
    assert_eq!(e.report.changed_source_scales, 0);
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), original);
    assert_eq!(d.can_undo(), prior_undo);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn empty_source_import_is_document_and_redo_noop() {
    let mut d = document(stage(INPUT));
    d.add_subtool(Mesh::sphere(0), "Prior redo").unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let empty = initialized(&repeated(0));
    assert!(empty.objects.is_empty());
    d.append_staged_sgl_scene(empty).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn thirty_two_source_policy_rows_preserve_capacity_bounds() {
    let b = repeated(32);
    let p = stage(&b)
        .prepare_source_import_options(SourceImportOptions {
            scale_center: false,
            vertex_srgb: true,
        })
        .unwrap();
    assert_eq!(p.staged.objects.len(), 32);
    assert!(p.staged.objects.iter().all(|o| o.source_center_initialized));
    assert_eq!(p.report.initialized_centers.len(), 32);
    assert_eq!(
        p.report.options_storage_bytes_bound,
        p.report.initialized_centers.capacity() * std::mem::size_of::<SourceImportCenterReport>()
            + std::mem::size_of::<[[u32; 3]; MAX_SUBTOOLS]>()
            + 128
    );
    let d = document(p.staged);
    let e = d.export_sgl(&settings()).unwrap();
    assert_eq!(e.report.changed_source_centers, 32);
    assert_eq!(e.report.changed_source_scales, 0);
    assert!(StagedSglScene::from_sgl_bytes(&repeated(33), &SglStageSettings::default()).is_err());
}
#[test]
fn malformed_v7_boolean_rejects_without_consuming_redo() {
    let mut receiving = document(stage(INPUT));
    receiving
        .add_subtool(Mesh::sphere(0), "Prior redo")
        .unwrap();
    assert!(receiving.undo());
    let before = receiving.to_json().unwrap();
    let source = document(initialized(INPUT));
    let mut value: serde_json::Value = serde_json::from_str(&source.to_json().unwrap()).unwrap();
    value["scene_v7"]["objects"][0]["source_center_initialized"] = serde_json::json!(1);
    assert!(Document::from_json(&serde_json::to_string(&value).unwrap()).is_err());
    assert_eq!(receiving.to_json().unwrap(), before);
    assert!(receiving.can_redo());
}
#[test]
fn reject_loss_center_export_is_document_and_redo_noop() {
    let mut d = document(initialized(INPUT));
    d.add_subtool(Mesh::sphere(0), "Prior redo").unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let settings = SglExportSettings {
        loss_policy: SglExportLossPolicy::RejectLoss,
        ..settings()
    };
    assert_eq!(
        d.plan_sgl_export(&settings).unwrap().changed_source_centers,
        1
    );
    assert!(d.export_sgl(&settings).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
