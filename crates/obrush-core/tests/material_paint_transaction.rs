// Shared independent fixture module also includes the separate legacy ledger generator.
#[allow(dead_code)]
#[path = "common/paint_fixture.rs"]
mod fixture;
use fixture::{bits, settings, sheet, surface};
use obrush_core::{Document, LockedStamp, LockedStampSample, Vec3, DEFAULT_VERTEX_MATERIAL};

fn retained() -> Document {
    let mut mesh = sheet();
    mesh.surface = Some(surface(&mesh));
    let mut d = Document::new(mesh);
    d.add_subdivision_level().unwrap();
    d
}
fn options() -> obrush_core::BrushSettings {
    let mut s = settings();
    s.paint_rgb = false;
    s.paint_roughness = true;
    s.roughness = 0.8;
    s.paint_metallic = true;
    s.metallic = 0.6;
    s
}
fn paint(d: &mut Document) {
    assert!(
        d.active_mesh_mut()
            .unwrap()
            .apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &options())
            .changed_vertices
            > 0
    );
}
fn seeded_redo() -> Document {
    let mut d = retained();
    d.begin_stroke();
    let s = settings();
    assert!(
        d.active_mesh_mut()
            .unwrap()
            .apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &s)
            .changed_vertices
            > 0
    );
    d.end_stroke();
    assert!(d.undo());
    assert!(d.can_redo());
    d
}

#[test]
fn explicit_begin_is_exact_history_capture_without_material_storage() {
    let mut d = seeded_redo();
    let before = d.to_json().unwrap();
    let mesh = bits(d.active_mesh().unwrap());
    d.begin_material_paint_stroke();
    d.begin_material_paint_stroke();
    d.begin_stroke();
    assert!(d.active_mesh().unwrap().materials.is_none());
    assert_eq!(bits(d.active_mesh().unwrap()), mesh);
    assert!(d.can_redo());
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}

#[test]
fn first_retained_material_stroke_restores_implicit_levels_with_one_undo() {
    let mut d = retained();
    let before = d.to_json().unwrap();
    let uv = d.active_mesh().unwrap().surface.clone();
    d.begin_material_paint_stroke();
    for _ in 0..5 {
        paint(&mut d);
    }
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    let after = d.to_json().unwrap();
    assert_eq!(d.active_mesh().unwrap().surface, uv);
    assert!(d.active_mesh().unwrap().materials.is_some());
    assert_eq!(d.level_count(), 2);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.active_mesh().unwrap().materials.is_none());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    d.select_level(0).unwrap();
    assert!(d
        .active_mesh()
        .unwrap()
        .materials
        .as_ref()
        .unwrap()
        .iter()
        .all(|v| v.map(f32::to_bits) == DEFAULT_VERTEX_MATERIAL.map(f32::to_bits)));
}

#[test]
fn lower_level_first_paint_propagates_to_all_finer_levels_and_roundtrips() {
    let mut d = retained();
    d.add_subdivision_level().unwrap();
    d.select_level(0).unwrap();
    let before = d.to_json().unwrap();
    d.begin_material_paint_stroke();
    paint(&mut d);
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    for level in 0..3 {
        d.select_level(level).unwrap();
        d.active_mesh().unwrap().validate().unwrap();
        assert!(d
            .active_mesh()
            .unwrap()
            .materials
            .as_ref()
            .unwrap()
            .iter()
            .any(|v| v[0] > DEFAULT_VERTEX_MATERIAL[0]));
        assert!(d
            .active_mesh()
            .unwrap()
            .surface
            .as_ref()
            .unwrap()
            .uv
            .is_some());
    }
    let encoded = d.to_json().unwrap();
    assert_eq!(
        Document::from_json(&encoded).unwrap().to_json().unwrap(),
        encoded
    );
}

#[test]
fn retained_material_stroke_with_recording_layer_restores_complete_state() {
    let mut d = retained();
    let layer = d.add_layer("shape").unwrap();
    d.set_recording_layer(Some(layer)).unwrap();
    let before = d.to_json().unwrap();
    let positions = d.active_mesh().unwrap().positions.clone();
    d.begin_material_paint_stroke();
    paint(&mut d);
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    let after = d.to_json().unwrap();
    assert_eq!(d.active_mesh().unwrap().positions, positions);
    assert_eq!(d.recording_layer(), Some(layer));
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn retained_material_stroke_in_scene_keeps_other_parts_and_complete_history() {
    let mut d = retained();
    let active = d.active_subtool_id().unwrap();
    let other = d.add_subtool(sheet(), "other").unwrap();
    d.select_subtool(active).unwrap();
    let before = d.to_json().unwrap();
    d.begin_material_paint_stroke();
    paint(&mut d);
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    d.select_subtool(other).unwrap();
    assert!(d.active_mesh().unwrap().materials.is_none());
    d.select_subtool(active).unwrap();
    assert!(d.active_mesh().unwrap().materials.is_some());
}

#[test]
fn generic_retained_attribute_installation_still_rejects_presence_change() {
    let mut d = retained();
    let before = d.to_json().unwrap();
    let values = vec![[0.4, 0.2]; d.active_mesh().unwrap().positions.len()];
    assert!(d
        .install_mesh_attributes(Some(values), d.active_mesh().unwrap().surface.clone())
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
}

#[test]
fn explicit_default_only_representation_restores_none_and_redo() {
    let mut d = seeded_redo();
    let before = d.to_json().unwrap();
    d.begin_material_paint_stroke();
    d.active_mesh_mut().unwrap().materials = Some(vec![
        DEFAULT_VERTEX_MATERIAL;
        d.active_mesh().unwrap().positions.len()
    ]);
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}

#[test]
fn no_material_delta_can_still_commit_rgb_in_one_explicit_stroke() {
    let mut d = retained();
    let before = d.to_json().unwrap();
    d.begin_material_paint_stroke();
    let s = settings();
    assert!(
        d.active_mesh_mut()
            .unwrap()
            .apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &s)
            .changed_vertices
            > 0
    );
    d.active_mesh_mut().unwrap().materials = Some(vec![
        DEFAULT_VERTEX_MATERIAL;
        d.active_mesh().unwrap().positions.len()
    ]);
    d.end_stroke();
    assert!(d.active_mesh().unwrap().materials.is_none());
    assert!(d.take_layer_error().is_none());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn explicit_material_marker_does_not_permit_uv_correspondence_changes() {
    let mut d = seeded_redo();
    let before = d.to_json().unwrap();
    d.begin_material_paint_stroke();
    paint(&mut d);
    d.active_mesh_mut()
        .unwrap()
        .surface
        .as_mut()
        .unwrap()
        .uv
        .as_mut()
        .unwrap()
        .coordinates[0][0] += 0.125;
    d.end_stroke();
    assert!(d.take_layer_error().is_some());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}

#[test]
fn explicit_material_marker_does_not_permit_retained_topology_changes() {
    let mut d = seeded_redo();
    let before = d.to_json().unwrap();
    d.begin_material_paint_stroke();
    paint(&mut d);
    d.active_mesh_mut().unwrap().triangles[0].swap(0, 1);
    d.end_stroke();
    assert!(d.take_layer_error().is_some());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}

#[test]
fn failed_project_output_limit_rolls_back_seeded_materials_and_history() {
    let mut d = seeded_redo();
    let before = d.to_json().unwrap();
    d.begin_material_paint_stroke();
    paint(&mut d);
    d.name = "x".repeat(4097);
    d.end_stroke();
    assert!(d.take_layer_error().is_some());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}

#[test]
fn locked_retained_material_zero_and_cancel_preserve_implicit_state_and_redo() {
    for cancel in [false, true] {
        let mut d = seeded_redo();
        let before = d.to_json().unwrap();
        d.begin_material_paint_stroke();
        let s = options();
        let mut stamp =
            LockedStamp::prepare(d.active_mesh().unwrap(), Vec3::ZERO, Vec3::Z, &s).unwrap();
        let sample = LockedStampSample {
            radius: 0.5,
            alpha_tangent: Some(Vec3::X),
            ..Default::default()
        };
        assert!(stamp
            .apply(d.active_mesh_mut().unwrap(), sample)
            .unwrap()
            .changed());
        if cancel {
            assert!(stamp
                .cancel(d.active_mesh_mut().unwrap())
                .unwrap()
                .changed());
        } else {
            assert!(stamp
                .apply(
                    d.active_mesh_mut().unwrap(),
                    LockedStampSample {
                        radius: 0.0,
                        ..sample
                    }
                )
                .unwrap()
                .changed());
        }
        d.end_stroke();
        assert!(d.take_layer_error().is_none());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.can_redo());
    }
}

#[test]
fn locked_retained_material_previews_commit_exact_one_stroke() {
    let mut d = retained();
    let before = d.to_json().unwrap();
    d.begin_material_paint_stroke();
    let s = options();
    let mut stamp =
        LockedStamp::prepare(d.active_mesh().unwrap(), Vec3::ZERO, Vec3::Z, &s).unwrap();
    for radius in [0.2, 0.4, 0.3, 0.5] {
        stamp
            .apply(
                d.active_mesh_mut().unwrap(),
                LockedStampSample {
                    radius,
                    alpha_tangent: Some(Vec3::X),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn explicit_permission_does_not_leak_into_next_ordinary_stroke() {
    let mut d = retained();
    d.begin_material_paint_stroke();
    d.end_stroke();
    let before = d.to_json().unwrap();
    d.begin_stroke();
    paint(&mut d);
    d.end_stroke();
    assert!(d.take_layer_error().is_some());
    assert_eq!(d.to_json().unwrap(), before);
}
