//! Independent precision, effective-level, transaction and scope contracts.
//! No source/reference runtime equivalence is asserted by these local gates.
use obrush_core::{
    BrushKind, BrushSettings, Document, MaskConvention, MaskConversionPolicy,
    MaskHierarchyConversionReport, Mesh, StaticSubdivisionSettings, Vec3,
};
use serde_json::Value;
const P: MaskConvention = MaskConvention::ProtectedF32V1;
const U: MaskConvention = MaskConvention::UnprotectedF32V1;
const EXACT: MaskConversionPolicy = MaskConversionPolicy::RequireRoundtripExact;
const ALLOW: MaskConversionPolicy = MaskConversionPolicy::AllowAndReport;
fn triangle(convention: MaskConvention) -> Mesh {
    let mut m = Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
    m.install_mask_channel(vec![0.25, 0.5, 0.75], convention)
        .unwrap();
    m
}
fn bits(m: &Mesh) -> Vec<u32> {
    m.masks.iter().map(|v| v.to_bits()).collect()
}
fn words_document(convention: MaskConvention) -> Document {
    let rows = oracle()["rows"].as_array().unwrap().clone();
    let mut m = triangle(convention);
    m.positions.resize(rows.len(), Vec3::new(0.25, 0.25, 0.));
    m.normals.resize(rows.len(), Vec3::Z);
    m.colors.resize(rows.len(), [0.65, 0.69, 0.74]);
    m.masks = rows
        .iter()
        .map(|r| f32::from_bits(r["source_bits"].as_u64().unwrap() as u32))
        .collect();
    m.validate().unwrap();
    Document::new(m)
}
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/independent-hierarchy-mask-precision.json"
    ))
    .unwrap()
}
fn retained(convention: MaskConvention, static_mode: Option<bool>) -> Document {
    let mut d = Document::new(triangle(convention));
    for _ in 0..2 {
        if let Some(linear_edges) = static_mode {
            d.add_static_subdivision_level(&StaticSubdivisionSettings { linear_edges })
                .unwrap();
        } else {
            d.add_subdivision_level().unwrap();
        }
    }
    d.select_level(0).unwrap();
    d
}
fn value(d: &Document) -> Value {
    serde_json::from_str(&d.to_json().unwrap()).unwrap()
}
fn level_meshes(v: &Value) -> Vec<&Value> {
    if let Some(levels) = v["multiresolution"]["levels"].as_array() {
        levels
            .iter()
            .map(|m| if m.is_null() { &v["mesh"] } else { m })
            .collect()
    } else {
        vec![&v["mesh"]]
    }
}
fn mask_word_list(v: &Value) -> Vec<u32> {
    v["masks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| (x.as_f64().unwrap() as f32).to_bits())
        .collect()
}
fn without_masks(mut v: Value) -> Value {
    match &mut v {
        Value::Object(map) => {
            map.remove("version");
            map.remove("masks");
            map.remove("mask_convention");
            for child in map.values_mut() {
                *child = without_masks(child.take());
            }
        }
        Value::Array(rows) => {
            for child in rows {
                *child = without_masks(child.take());
            }
        }
        _ => (),
    }
    v
}
fn assert_complements(before: &Value, after: &Value) {
    let a = level_meshes(before);
    let b = level_meshes(after);
    assert_eq!(a.len(), b.len());
    for (old, new) in a.iter().zip(b) {
        let expected: Vec<u32> = old["masks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| (1.0_f32 - x.as_f64().unwrap() as f32).to_bits())
            .collect();
        assert_eq!(mask_word_list(new), expected);
    }
    assert_eq!(without_masks(before.clone()), without_masks(after.clone()));
}
fn hidden_loss(convention: MaskConvention) -> Document {
    let d = retained(convention, None);
    let mut v = value(&d);
    v["multiresolution"]["levels"][2]["masks"][0] = serde_json::json!(f32::from_bits(1));
    Document::from_json(&serde_json::to_string(&v).unwrap()).unwrap()
}
fn assert_oracle_report(r: &MaskHierarchyConversionReport) {
    let expected = oracle();
    let e = &expected["report"];
    assert_eq!(r.vertices, e["vertices"].as_u64().unwrap() as usize);
    assert_eq!(
        r.roundtrip_changed_words,
        e["roundtrip_changed_words"].as_u64().unwrap() as usize
    );
    assert_eq!(
        r.signed_zero_only_changes,
        e["signed_zero_only_changes"].as_u64().unwrap() as usize
    );
    assert_eq!(
        r.max_abs_roundtrip_error,
        e["max_abs_roundtrip_error"].as_f64().unwrap()
    );
    assert_eq!(
        r.max_relative_roundtrip_error,
        e["max_relative_roundtrip_error"].as_f64().unwrap()
    );
}
fn brush(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        symmetry_x: false,
        adaptive_detail: false,
        radius: 0.5,
        strength: 0.35,
        ..Default::default()
    }
}

#[test]
fn exact_rational_32_word_oracle_protected_to_source_report_and_channel() {
    let mut d = words_document(P);
    let preview = d.active_subtool_mask_conversion_report(U).unwrap();
    assert_oracle_report(&preview);
    let applied = d.convert_active_subtool_masks(U, ALLOW).unwrap();
    assert_eq!(preview, applied);
    let expected: Vec<u32> = oracle()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["converted_bits"].as_u64().unwrap() as u32)
        .collect();
    assert_eq!(bits(d.require_active_mesh().unwrap()), expected);
}
#[test]
fn exact_rational_32_word_oracle_source_to_protected_report_and_channel() {
    let mut d = words_document(U);
    let report = d.convert_active_subtool_masks(P, ALLOW).unwrap();
    assert_oracle_report(&report);
    let expected: Vec<u32> = oracle()["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["converted_bits"].as_u64().unwrap() as u32)
        .collect();
    assert_eq!(bits(d.require_active_mesh().unwrap()), expected);
}
#[test]
fn first_loss_witness_reports_vertex_and_all_three_raw_words() {
    let d = words_document(U);
    let r = d.active_subtool_mask_conversion_report(P).unwrap();
    let w = r.levels[0].first_changed_word.unwrap();
    assert_eq!(
        (w.vertex, w.source_bits, w.converted_bits, w.roundtrip_bits),
        (1, 0x80000000, 0x3f800000, 0)
    );
}
#[test]
fn exact_policy_rejects_loss_before_any_plain_channel_or_history_change() {
    let mut d = words_document(P);
    let before = d.to_json().unwrap();
    assert!(!d.can_undo());
    assert!(d
        .convert_active_subtool_masks(U, EXACT)
        .unwrap_err()
        .contains("17 f32 words"));
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
}
#[test]
fn read_only_preview_keeps_scene_complete_and_redo_available() {
    let mut d = retained(P, None);
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let r = d.active_subtool_mask_conversion_report(U).unwrap();
    assert!(r.changed);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn report_counts_each_effective_level_once_not_active_mesh_twice() {
    let d = retained(P, None);
    let r = d.active_subtool_mask_conversion_report(U).unwrap();
    assert!(r.retained_hierarchy);
    assert_eq!(r.active_level, 0);
    assert_eq!(
        r.levels
            .iter()
            .map(|r| r.precision.vertices)
            .collect::<Vec<_>>(),
        [3, 6, 15]
    );
    assert_eq!(r.vertices, 24);
    assert_eq!(r.levels.iter().filter(|r| r.active).count(), 1);
    assert_eq!(
        r.levels.iter().map(|r| r.level).collect::<Vec<_>>(),
        [0, 1, 2]
    );
}
#[test]
fn hidden_fine_level_loss_rejects_exact_policy_atomically_with_redo() {
    let mut d = hidden_loss(P);
    d.invert_mask();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let r = d.active_subtool_mask_conversion_report(U).unwrap();
    assert_eq!(r.levels[0].precision.roundtrip_changed_words, 0);
    assert_eq!(r.levels[2].first_changed_word.unwrap().source_bits, 1);
    assert_eq!(r.roundtrip_changed_words, 1);
    assert!(d.convert_active_subtool_masks(U, EXACT).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn explicitly_allowed_hidden_level_loss_keeps_precision_report_and_one_undo() {
    let mut d = hidden_loss(U);
    let before = d.to_json().unwrap();
    let r = d.convert_active_subtool_masks(P, ALLOW).unwrap();
    assert_eq!(r.roundtrip_changed_words, 1);
    assert_eq!(r.max_relative_roundtrip_error, 1.);
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo()); // Document::from_json starts a fresh history.
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn every_retained_level_converts_and_v7_reload_keeps_hierarchy() {
    let mut d = retained(P, None);
    let before = value(&d);
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    let after = value(&d);
    assert_eq!(after["version"], 7);
    assert_complements(&before, &after);
    for m in level_meshes(&after) {
        assert_eq!(m["mask_convention"], "unprotected_f32_v1");
    }
    let loaded = Document::from_json(&d.to_json().unwrap()).unwrap();
    assert_eq!(value(&loaded), after);
    assert_eq!(loaded.level_count(), 3);
}
#[test]
fn whole_hierarchy_inverse_exact_return_is_one_transaction_per_operation() {
    let mut d = retained(P, None);
    let before = d.to_json().unwrap();
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    let middle = d.to_json().unwrap();
    d.convert_active_subtool_masks(P, EXACT).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), middle);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn loop_scheme_and_active_index_are_preserved_on_fine_level_conversion() {
    let mut d = retained(P, None);
    d.select_level(2).unwrap();
    let before = value(&d);
    let r = d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert_eq!(r.active_level, 2);
    assert_eq!(r.levels[2].precision.vertices, 15);
    assert_complements(&before, &value(&d));
    assert_eq!(d.current_level(), 2);
}
#[test]
fn static_smooth_retained_stencils_survive_conversion_and_navigation() {
    let mut d = retained(P, Some(false));
    let before = value(&d);
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert_complements(&before, &value(&d));
    assert!(
        !d.retained_static_subdivision_settings()
            .unwrap()
            .linear_edges
    );
    d.select_level(2).unwrap();
    d.select_level(0).unwrap();
    assert_eq!(without_masks(value(&d)), without_masks(before));
}
#[test]
fn static_linear_retained_stencils_survive_conversion() {
    let mut d = retained(P, Some(true));
    let before = value(&d);
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert_complements(&before, &value(&d));
    assert!(
        d.retained_static_subdivision_settings()
            .unwrap()
            .linear_edges
    );
}
#[test]
fn singleton_retained_scheme_stays_retained_and_is_reported_once() {
    let mut d = retained(P, Some(false));
    d.delete_higher_subdivision_levels().unwrap();
    assert!(d.has_retained_subdivision_hierarchy());
    let r = d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert!(r.retained_hierarchy);
    assert_eq!(r.levels.len(), 1);
    assert!(d.has_retained_subdivision_hierarchy());
    assert!(d.retained_static_subdivision_settings().is_some());
}
#[test]
fn existing_plain_converter_retained_rejection_is_unchanged() {
    let mut d = retained(P, None);
    let before = d.to_json().unwrap();
    assert!(d
        .convert_mask_convention(U, ALLOW)
        .unwrap_err()
        .contains("retained"));
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn same_convention_noop_preserves_source_tiny_signed_zero_words_and_redo() {
    let mut d = words_document(U);
    d.invert_mask();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let words = bits(d.require_active_mesh().unwrap());
    let r = d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert!(!r.changed);
    assert_eq!(r.roundtrip_changed_words, 0);
    assert_eq!(bits(d.require_active_mesh().unwrap()), words);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn same_convention_retained_noop_preserves_pending_public_edits_and_redo() {
    let mut d = retained(U, None);
    d.convert_active_subtool_masks(P, EXACT).unwrap();
    assert!(d.undo());
    d.require_active_mesh_mut().unwrap().colors[0] = [0.125, 0.25, 0.5];
    let before = d.to_json().unwrap();
    let r = d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert!(!r.changed);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn pending_coarse_mask_edits_reconcile_before_conversion_not_after() {
    let mut d = retained(P, None);
    d.require_active_mesh_mut().unwrap().masks[0] = 0.125;
    let before = value(&d); // The serialized effective hierarchy is the oracle.
    let r = d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert_eq!(r.roundtrip_changed_words, 0);
    assert_complements(&before, &value(&d));
}
#[test]
fn pending_coarse_geometry_rgb_edits_keep_effective_nonmask_data_exact() {
    let mut d = retained(P, None);
    d.require_active_mesh_mut().unwrap().positions[0].z = 0.03125;
    d.require_active_mesh_mut().unwrap().colors[0] = [0.25, 0.5, 0.75];
    d.require_active_mesh_mut().unwrap().recompute_normals();
    let before = value(&d);
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert_complements(&before, &value(&d));
    assert!(d.undo());
    assert_eq!(value(&d), before);
}
#[test]
fn visible_protected_signed_zero_is_reported_despite_legacy_numeric_noop() {
    let mut m = triangle(P);
    m.masks[0] = 0.;
    let mut d = Document::new(m);
    d.add_subdivision_level().unwrap();
    d.add_subdivision_level().unwrap();
    d.select_level(0).unwrap();
    d.require_active_mesh_mut().unwrap().masks[0] = -0.;
    // This active raw word must not be replaced by its older retained baseline.
    let r = d.active_subtool_mask_conversion_report(U).unwrap();
    assert_eq!(
        r.levels[0].first_changed_word.unwrap().source_bits,
        0x80000000
    );
    assert!(r.signed_zero_only_changes >= 1);
    assert!(d.convert_active_subtool_masks(U, EXACT).is_err());
    assert_eq!(
        d.require_active_mesh_mut().unwrap().masks[0].to_bits(),
        0x80000000
    );
}
#[test]
fn open_real_gesture_rejects_conversion_without_finishing_or_splitting_it() {
    let mut d = Document::new(triangle(P));
    let before = d.to_json().unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().colors[0] = [0.25, 0.5, 0.75];
    assert!(d.active_subtool_mask_conversion_report(U).is_err());
    assert!(d.convert_active_subtool_masks(U, EXACT).is_err());
    assert!(d.can_undo()); // The still-open real edit remains available.
    d.require_active_mesh_mut().unwrap().colors[1] = [0.5, 0.5, 0.5];
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn open_material_gesture_rejects_conversion_without_clearing_marker() {
    let mut d = retained(P, None);
    let before = d.to_json().unwrap();
    d.begin_material_paint_stroke();
    assert!(d.convert_active_subtool_masks(U, EXACT).is_err());
    let mut s = brush(BrushKind::Paint);
    s.paint_rgb = false;
    s.paint_roughness = true;
    s.roughness = 0.7;
    assert!(
        d.require_active_mesh_mut()
            .unwrap()
            .apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &s)
            .changed_vertices
            > 0
    );
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    assert!(d.require_active_mesh_mut().unwrap().materials.is_some());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn hidden_active_subtool_rejects_precision_transition_and_keeps_redo() {
    let mut d = Document::new(triangle(P));
    let id = d.active_subtool_id().unwrap();
    d.set_subtool_visible(id, false).unwrap();
    d.set_subtool_name(id, "hidden").unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    assert!(d
        .convert_active_subtool_masks(U, ALLOW)
        .unwrap_err()
        .contains("Show"));
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn active_scene_scope_keeps_inactive_different_convention_complete() {
    let mut d = retained(P, None);
    let active = d.active_subtool_id().unwrap();
    let mut other = triangle(U);
    other.masks[0] = f32::from_bits(1);
    other.masks[1] = -0.;
    let other_id = d.add_subtool(other, "canonical other").unwrap();
    d.select_subtool(active).unwrap();
    let before = value(&d);
    let r = d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert_eq!(r.subtool_id, active);
    assert_eq!(r.vertices, 24);
    assert_eq!(value(&d)["scene"], before["scene"]);
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(value(&d), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    d.select_subtool(other_id).unwrap();
    assert_eq!(d.require_active_mesh_mut().unwrap().mask_convention, U);
    assert_eq!(bits(d.require_active_mesh().unwrap())[..2], [1, 0x80000000]);
}
#[test]
fn layers_visibility_intensity_recording_and_morph_binding_survive() {
    let mut d = retained(P, None);
    d.store_morph_target().unwrap();
    let layer = d.add_layer("recorded").unwrap();
    d.set_recording_layer(Some(layer)).unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[0].z += 0.03125;
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    // Existing recording requires a visible unit-intensity layer. Keep a
    // second meaningful hidden/scaled layer rather than expecting the first
    // layer to retain a recording state already cleared by its own setup.
    let hidden = d.add_layer("hidden scaled").unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[1].z += 0.0625;
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    d.set_layer_intensity(hidden, 0.5).unwrap();
    d.set_layer_visibility(hidden, false).unwrap();
    d.set_recording_layer(Some(layer)).unwrap();
    assert_eq!(d.recording_layer(), Some(layer));
    let before = value(&d);
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert_complements(&before, &value(&d));
    assert_eq!(d.recording_layer(), Some(layer));
    assert!(d.has_morph_target());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(value(&d), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn converted_retained_layers_still_record_one_later_geometry_stroke() {
    let mut d = retained(P, None);
    let layer = d.add_layer("after conversion").unwrap();
    d.set_recording_layer(Some(layer)).unwrap();
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    let before = d.to_json().unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[0].z += 0.015625;
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn retained_materials_and_uv_raw_words_are_exactly_preserved() {
    let mut d = Document::from_json(include_str!(
        "fixtures/independent-material-uv-seam-v6.obrush"
    ))
    .unwrap();
    d.add_static_subdivision_level(&StaticSubdivisionSettings { linear_edges: true })
        .unwrap();
    let before = value(&d);
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert_complements(&before, &value(&d));
    let loaded = Document::from_json(&d.to_json().unwrap()).unwrap();
    assert_eq!(value(&loaded), value(&d));
}
#[test]
fn implicit_materials_stay_absent_across_every_level() {
    let mut d = retained(P, None);
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    for mesh in level_meshes(&value(&d)) {
        assert!(mesh.get("materials").is_none());
    }
    assert!(d.require_active_mesh_mut().unwrap().materials.is_none());
}
#[test]
fn invalid_active_mask_is_rejected_without_sanitizing_or_clearing_redo() {
    let mut d = retained(P, None);
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert!(d.undo());
    d.require_active_mesh_mut().unwrap().masks[0] = f32::NAN;
    let words = bits(d.require_active_mesh().unwrap());
    assert!(d.convert_active_subtool_masks(U, ALLOW).is_err());
    assert_eq!(bits(d.require_active_mesh().unwrap()), words);
    assert_eq!(d.require_active_mesh_mut().unwrap().mask_convention, P);
    assert!(d.can_redo());
}
#[test]
fn stale_active_topology_and_attribute_presence_fail_generic_binding_atomically() {
    let mut d = retained(P, None);
    let before = d.to_json().unwrap();
    d.require_active_mesh_mut().unwrap().triangles[0].swap(0, 1);
    assert!(d.convert_active_subtool_masks(U, ALLOW).is_err());
    d.require_active_mesh_mut().unwrap().triangles[0].swap(0, 1);
    d.require_active_mesh_mut().unwrap().materials =
        Some(vec![
            [0.18, 0.08];
            d.require_active_mesh_mut().unwrap().positions.len()
        ]);
    assert!(d.convert_active_subtool_masks(U, ALLOW).is_err());
    d.require_active_mesh_mut().unwrap().materials = None;
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn plain_half_masks_still_create_undo_when_only_convention_changes() {
    let mut m = triangle(P);
    m.masks.fill(0.5);
    let mut d = Document::new(m);
    let before = d.to_json().unwrap();
    d.convert_active_subtool_masks(U, EXACT).unwrap();
    assert_eq!(
        bits(d.require_active_mesh().unwrap()),
        vec![0.5_f32.to_bits(); 3]
    );
    assert!(d.can_undo());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn complete_undo_budget_is_checked_before_candidate_or_history_clone() {
    let mut d = retained(P, None);
    d.name = String::with_capacity(obrush_core::resource_limits::MAX_HISTORY_BYTES);
    d.name.push_str("Small name, excessive original capacity");
    let before = d.to_json().unwrap();
    let capacity = d.name.capacity();
    assert!(d
        .convert_active_subtool_masks(U, EXACT)
        .unwrap_err()
        .contains("complete original undo state"));
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.name.capacity(), capacity);
}
