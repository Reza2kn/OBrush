//! Independent geometry-layer contracts. These tests make no ZBrush parity
//! claim: paint/masks are unlayered, and a layer stack freezes its active level.
use obrush_core::{Document, Mesh, Vec3};
use serde_json::Value;

fn state(document: &Document) -> String {
    document.to_json().unwrap()
}

fn exact_surface(actual: &Mesh, expected: &Mesh) {
    let position_bits = |mesh: &Mesh| {
        mesh.positions
            .iter()
            .flat_map(|point| point.to_array().map(f32::to_bits))
            .collect::<Vec<_>>()
    };
    assert_eq!(position_bits(actual), position_bits(expected));
    assert_eq!(actual.triangles, expected.triangles);
    assert_eq!(
        actual
            .masks
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        expected
            .masks
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>()
    );
    let color_bits = |mesh: &Mesh| {
        mesh.colors
            .iter()
            .flat_map(|color| color.map(f32::to_bits))
            .collect::<Vec<_>>()
    };
    assert_eq!(color_bits(actual), color_bits(expected));
}

fn translate(document: &mut Document, offset: Vec3) {
    document.begin_stroke();
    for point in &mut document.active_mesh_mut().unwrap().positions {
        *point += offset;
    }
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
}

fn record_vertex(document: &mut Document, index: usize, offset: Vec3) {
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[index] += offset;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
}

#[test]
fn exact_unit_endpoints_survive_repeated_visibility_and_fractional_weights() {
    let mut document = Document::new(Mesh::sphere(1));
    let base = document.active_mesh().unwrap().snapshot();
    let index = document.add_layer("Forms").unwrap();
    assert_eq!(index, 0);
    assert_eq!(document.recording_layer(), Some(index));
    assert!(!document.layer_summaries()[index].has_geometry);
    record_vertex(&mut document, 5, Vec3::new(0.027, -0.013, 0.039));
    let recorded = document.active_mesh().unwrap().snapshot();
    assert!(document.layer_summaries()[index].has_geometry);
    document.set_recording_layer(None).unwrap();
    for _ in 0..64 {
        for intensity in [0.31, 0.93, 0.0, 1.0] {
            document.set_layer_intensity(index, intensity).unwrap();
        }
        exact_surface(document.active_mesh().unwrap(), &recorded);
        document.set_layer_visibility(index, false).unwrap();
        exact_surface(document.active_mesh().unwrap(), &base);
        document.set_layer_visibility(index, true).unwrap();
        exact_surface(document.active_mesh().unwrap(), &recorded);
    }
    let bytes = state(&document);
    let restored = Document::from_json(&bytes).unwrap();
    assert_eq!(state(&restored), bytes);
    exact_surface(restored.active_mesh().unwrap(), &recorded);
    assert!(!restored.can_undo());
}

#[test]
fn two_layers_blend_additively_against_independent_f64_position_oracle() {
    let mut document = Document::new(Mesh::sphere(0));
    let base = document.active_mesh().unwrap().positions[0];
    let first = document.add_layer("First").unwrap();
    record_vertex(&mut document, 0, Vec3::new(0.023, 0.031, -0.017));
    let first_recorded = document.active_mesh().unwrap().positions[0];
    document.set_layer_intensity(first, 0.25).unwrap();
    assert_eq!(document.recording_layer(), None);
    let mixed_before_second = document.active_mesh().unwrap().positions[0];
    let second = document.add_layer("Second").unwrap();
    record_vertex(&mut document, 0, Vec3::new(-0.019, 0.007, 0.041));
    let second_recorded = document.active_mesh().unwrap().positions[0];
    document.set_recording_layer(None).unwrap();
    document.set_layer_intensity(first, 0.625).unwrap();
    document.set_layer_intensity(second, 0.375).unwrap();
    // Recording captures the f32 geometry actually edited, rather than the
    // requested offsets. The oracle only adds those measured displacements.
    let expected = base.as_dvec3()
        + (first_recorded.as_dvec3() - base.as_dvec3()) * 0.625
        + (second_recorded.as_dvec3() - mixed_before_second.as_dvec3()) * 0.375;
    assert!((document.active_mesh().unwrap().positions[0].as_dvec3() - expected).length() < 2e-7);
    let bytes = state(&document);
    assert_eq!(state(&Document::from_json(&bytes).unwrap()), bytes);
}

#[test]
fn masks_and_colors_survive_blend_delete_and_bake_without_entering_deltas() {
    let mut document = Document::new(Mesh::sphere(0));
    let layer = document.add_layer("Unpainted geometry").unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().masks[3] = 0.73;
    document.active_mesh_mut().unwrap().colors[3] = [0.13, 0.27, 0.91];
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
    assert!(!document.layer_summaries()[layer].has_geometry);
    record_vertex(&mut document, 3, Vec3::new(0.011, 0.017, 0.023));
    let masks = document.active_mesh().unwrap().masks.clone();
    let colors = document.active_mesh().unwrap().colors.clone();
    document.set_layer_intensity(layer, 0.5).unwrap();
    document.set_layer_visibility(layer, false).unwrap();
    assert_eq!(document.active_mesh().unwrap().masks, masks);
    assert_eq!(document.active_mesh().unwrap().colors, colors);
    document.set_layer_visibility(layer, true).unwrap();
    let before_bake = state(&document);
    let baked_surface = document.active_mesh().unwrap().snapshot();
    document.bake_layers().unwrap();
    assert_eq!(document.layer_count(), 0);
    exact_surface(document.active_mesh().unwrap(), &baked_surface);
    assert!(document.undo());
    assert_eq!(state(&document), before_bake);
    document.delete_layer(layer).unwrap();
    assert_eq!(document.active_mesh().unwrap().masks, masks);
    assert_eq!(document.active_mesh().unwrap().colors, colors);
}

#[test]
fn recording_target_changes_are_explicit_and_setters_are_one_undo_step() {
    let mut document = Document::new(Mesh::sphere(0));
    let layer = document.add_layer("Recorded").unwrap();
    record_vertex(&mut document, 0, Vec3::new(0.02, 0.01, 0.03));
    let recorded = state(&document);
    document.set_layer_intensity(layer, 0.5).unwrap();
    let half = state(&document);
    assert_eq!(document.recording_layer(), None);
    assert!(document.set_recording_layer(Some(layer)).is_err());
    assert_eq!(state(&document), half);
    assert!(document.undo());
    assert_eq!(state(&document), recorded);
    assert_eq!(document.recording_layer(), Some(layer));
    assert!(document.redo());
    assert_eq!(state(&document), half);
    document.set_layer_intensity(layer, 1.0).unwrap();
    document.set_recording_layer(Some(layer)).unwrap();
    document.set_layer_visibility(layer, false).unwrap();
    let hidden = state(&document);
    assert_eq!(document.recording_layer(), None);
    assert!(document.set_recording_layer(Some(layer)).is_err());
    assert_eq!(state(&document), hidden);
}

#[test]
fn no_op_setters_and_exact_return_strokes_preserve_available_redo() {
    let mut document = Document::new(Mesh::sphere(0));
    let layer = document.add_layer("No-op contract").unwrap();
    record_vertex(&mut document, 0, Vec3::new(0.031, -0.017, 0.023));
    let edited = state(&document);
    assert!(document.undo());
    let before = state(&document);
    assert!(document.can_redo());
    document.set_layer_intensity(layer, 1.0).unwrap();
    document.set_layer_visibility(layer, true).unwrap();
    document.set_layer_name(layer, "No-op contract").unwrap();
    document.set_recording_layer(Some(layer)).unwrap();
    document.begin_stroke();
    let original = document.active_mesh().unwrap().positions[0];
    document.active_mesh_mut().unwrap().positions[0] += Vec3::new(0.11, 0.07, -0.03);
    document.active_mesh_mut().unwrap().positions[0] = original;
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
    assert_eq!(state(&document), before);
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(state(&document), edited);
}

#[test]
fn empty_bake_and_no_layer_strokes_retain_original_history_semantics() {
    let mut document = Document::new(Mesh::sphere(0));
    let original = state(&document);
    translate(&mut document, Vec3::new(0.13, -0.07, 0.09));
    let edited = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), original);
    document.bake_layers().unwrap();
    document.set_recording_layer(None).unwrap();
    document.begin_stroke();
    document.end_stroke();
    assert_eq!(state(&document), original);
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(state(&document), edited);
}

#[test]
fn topology_violation_rolls_back_geometry_attributes_layers_and_redo() {
    let mut document = Document::new(Mesh::sphere(1));
    document.add_layer("Bound topology").unwrap();
    record_vertex(&mut document, 0, Vec3::new(0.02, 0.01, 0.03));
    assert!(document.undo());
    let before = state(&document);
    assert!(document.can_redo());
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].x += 0.04;
    document.active_mesh_mut().unwrap().masks[0] = 0.87;
    document.active_mesh_mut().unwrap().colors[0] = [0.9, 0.1, 0.2];
    document.active_mesh_mut().unwrap().triangles[0].swap(0, 1);
    document.end_stroke();
    assert!(document.take_layer_error().is_some());
    assert!(document.take_layer_error().is_none());
    assert_eq!(state(&document), before);
    assert!(document.can_redo());
    assert!(document.replace_mesh(Mesh::cube()).is_err());
    assert!(document.subdivide().is_err());
    assert!(document.subdivide_smooth().is_err());
    assert_eq!(state(&document), before);
    assert!(document.can_redo());
}

#[test]
fn finite_but_collapsing_blend_is_rejected_as_a_complete_no_op() {
    // Both recorded endpoints have a valid triangle; halfway between them the
    // third vertex lies on the first. Layer validation is distinct from a Move
    // brush fold guard: this test addresses a finite zero-area composition.
    let mut mesh = Mesh::sphere(0);
    mesh.positions = vec![Vec3::ZERO, Vec3::X, Vec3::Y];
    mesh.normals = vec![Vec3::Z; 3];
    mesh.masks = vec![0.0; 3];
    mesh.colors = vec![[0.5; 3]; 3];
    mesh.triangles = vec![[0, 1, 2]];
    let mut document = Document::new(mesh);
    let layer = document.add_layer("Finite endpoints").unwrap();
    record_vertex(&mut document, 2, -Vec3::Y * 2.0);
    let before = state(&document);
    assert!(document.set_layer_intensity(layer, 0.5).is_err());
    assert_eq!(state(&document), before);
    assert_eq!(document.recording_layer(), Some(layer));
    assert!(document.undo());
    assert!(!document.layer_summaries()[layer].has_geometry);
    assert!(document.redo());
    assert_eq!(state(&document), before);
}

#[test]
fn invalid_public_mesh_edits_roll_back_the_complete_recording_stroke() {
    let mut document = Document::new(Mesh::sphere(0));
    document.add_layer("Validated recording").unwrap();
    let before = state(&document);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].x = f32::NAN;
    document.active_mesh_mut().unwrap().masks[1] = 1.5;
    document.active_mesh_mut().unwrap().colors[2] = [0.7, 0.2, 0.9];
    document.end_stroke();
    assert!(document.take_layer_error().is_some());
    assert_eq!(state(&document), before);
    assert!(!document.layer_summaries()[0].has_geometry);
}

#[test]
fn invalid_normal_only_stroke_cannot_escape_the_no_op_fast_path() {
    let mut document = Document::new(Mesh::sphere(0));
    document.add_layer("Derived normal validation").unwrap();
    record_vertex(&mut document, 0, Vec3::new(0.02, 0.01, -0.03));
    let edited = state(&document);
    assert!(document.undo());
    let before = state(&document);
    let normal_bits: Vec<_> = document
        .active_mesh()
        .unwrap()
        .normals
        .iter()
        .flat_map(|normal| normal.to_array().map(f32::to_bits))
        .collect();
    for invalid in [f32::NAN, f32::INFINITY] {
        document.begin_stroke();
        // same_surface deliberately omits derived normals, so this corruption
        // must still be rejected before an unchanged-surface shortcut returns.
        document.active_mesh_mut().unwrap().normals[0].x = invalid;
        document.end_stroke();
        assert!(document.take_layer_error().is_some());
        assert!(document.take_layer_error().is_none());
        assert_eq!(state(&document), before);
        assert_eq!(
            document
                .active_mesh()
                .unwrap()
                .normals
                .iter()
                .flat_map(|normal| normal.to_array().map(f32::to_bits))
                .collect::<Vec<_>>(),
            normal_bits
        );
        assert!(document.can_redo());
    }
    assert!(document.redo());
    assert_eq!(state(&document), edited);
}

#[test]
fn layers_on_coarse_level_save_and_bake_preserving_fine_residuals() {
    let mut document = Document::new(Mesh::sphere(1));
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    record_vertex(&mut document, 17, Vec3::new(0.019, -0.023, 0.037));
    let fine = document.active_mesh().unwrap().snapshot();
    document.select_level(0).unwrap();
    let layer = document.add_layer("Coarse forms").unwrap();
    let coarse_before = document.active_mesh().unwrap().positions.clone();
    translate(&mut document, Vec3::new(0.14, -0.06, 0.11));
    document.set_layer_intensity(layer, 0.5).unwrap();
    let layered = state(&document);
    assert!(document.layer_topology_locked());
    assert_eq!((document.current_level(), document.level_count()), (0, 3));
    assert!(document.select_level(2).is_err());
    assert!(document.add_subdivision_level().is_err());
    assert_eq!(state(&document), layered);
    let mut restored = Document::from_json(&layered).unwrap();
    assert_eq!(state(&restored), layered);
    let expected_translation = document.active_mesh().unwrap().positions[0] - coarse_before[0];
    document.bake_layers().unwrap();
    let baked = state(&document);
    assert_eq!(document.layer_count(), 0);
    assert_eq!((document.current_level(), document.level_count()), (0, 3));
    assert!(document.undo());
    assert_eq!(state(&document), layered);
    assert!(document.redo());
    assert_eq!(state(&document), baked);
    restored.bake_layers().unwrap();
    document.select_level(2).unwrap();
    restored.select_level(2).unwrap();
    exact_surface(
        restored.active_mesh().unwrap(),
        document.active_mesh().unwrap(),
    );
    assert_eq!(document.active_mesh().unwrap().triangles, fine.triangles);
    assert_eq!(document.active_mesh().unwrap().masks, fine.masks);
    assert_eq!(document.active_mesh().unwrap().colors, fine.colors);
    // Existing Loop interpolation has bounded f32 rounding. A coarse global
    // translation should preserve every independently sculpted fine residual.
    for (before, after) in fine
        .positions
        .iter()
        .zip(&document.active_mesh().unwrap().positions)
    {
        assert!((*after - (*before + expected_translation)).length() < 3e-6);
    }
}

#[test]
fn limits_and_invalid_values_reject_atomically_without_consuming_redo() {
    let mut document = Document::new(Mesh::sphere(0));
    for index in 0..16 {
        assert_eq!(
            document.add_layer(&format!("Layer {index}")).unwrap(),
            index
        );
    }
    record_vertex(&mut document, 0, Vec3::new(0.02, 0.01, 0.03));
    assert!(document.undo());
    let before = state(&document);
    assert!(document.add_layer("Seventeenth").is_err());
    assert!(document.set_layer_name(0, &"x".repeat(257)).is_err());
    assert!(document.set_layer_name(0, &"é".repeat(129)).is_err());
    assert!(document.add_layer(&"é".repeat(129)).is_err());
    assert!(document.set_layer_intensity(0, f32::NAN).is_err());
    assert!(document.set_layer_intensity(0, f32::INFINITY).is_err());
    assert!(document.set_layer_intensity(0, -0.01).is_err());
    assert!(document.set_layer_intensity(0, 1.01).is_err());
    assert!(document.set_layer_visibility(usize::MAX, false).is_err());
    assert!(document.delete_layer(usize::MAX).is_err());
    assert!(document.set_recording_layer(Some(usize::MAX)).is_err());
    assert_eq!(state(&document), before);
    assert!(document.can_redo());
}

#[test]
fn aggregate_base_and_delta_budget_rejects_before_installing_a_stack() {
    // Valid flat meshes may contain unused vertices. This fixture reaches the
    // aggregate vector budget with one proposed layer, without dense triangles
    // or sixteen independently maximal arrays.
    let mut mesh = Mesh::sphere(0);
    mesh.positions.resize(500_001, Vec3::ZERO);
    mesh.normals.resize(500_001, Vec3::ZERO);
    mesh.masks.resize(500_001, 0.0);
    mesh.colors.resize(500_001, [0.0; 3]);
    mesh.validate().unwrap();
    let mut document = Document::new(mesh);
    let positions = document.active_mesh().unwrap().positions.clone();
    assert!(document.add_layer("Over aggregate budget").is_err());
    assert_eq!(document.layer_count(), 0);
    assert!(!document.layer_topology_locked());
    assert!(!document.can_undo());
    assert_eq!(document.active_mesh().unwrap().positions, positions);
}

#[test]
fn base_edits_outside_recording_remain_when_the_layer_is_hidden() {
    let mut document = Document::new(Mesh::sphere(0));
    let base = document.active_mesh().unwrap().snapshot();
    let layer = document.add_layer("Detail").unwrap();
    record_vertex(&mut document, 0, Vec3::new(0.02, 0.01, 0.03));
    document.set_recording_layer(None).unwrap();
    record_vertex(&mut document, 3, Vec3::new(-0.017, 0.023, 0.031));
    let edited = document.active_mesh().unwrap().snapshot();
    document.set_layer_visibility(layer, false).unwrap();
    assert_eq!(
        document.active_mesh().unwrap().positions[0],
        base.positions[0]
    );
    assert_eq!(
        document.active_mesh().unwrap().positions[3],
        edited.positions[3]
    );
    document.set_layer_visibility(layer, true).unwrap();
    exact_surface(document.active_mesh().unwrap(), &edited);
}

#[test]
fn layered_import_replacement_restores_complete_state_with_one_undo() {
    let mut existing = Document::new(Mesh::cube());
    existing.name = "Original".into();
    translate(&mut existing, Vec3::new(0.1, 0.2, 0.3));
    assert!(existing.undo());
    let before = state(&existing);
    assert!(existing.can_redo());
    let mut incoming = Document::new(Mesh::sphere(1));
    incoming.name = "Imported layers".into();
    incoming.add_subdivision_level().unwrap();
    incoming.add_layer("Imported detail").unwrap();
    record_vertex(&mut incoming, 5, Vec3::new(0.03, 0.01, -0.02));
    let imported = state(&incoming);
    assert_eq!(
        serde_json::from_str::<Value>(&imported).unwrap()["version"],
        3
    );
    existing
        .replace_document(Document::from_json(&imported).unwrap())
        .unwrap();
    assert_eq!(state(&existing), imported);
    assert!(!existing.can_redo());
    assert!(existing.undo());
    assert_eq!(state(&existing), before);
    assert!(existing.redo());
    assert_eq!(state(&existing), imported);
    existing.bake_layers().unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&state(&existing)).unwrap()["version"],
        2
    );
}

#[test]
fn v3_mesh_composition_and_versions_are_validated_before_import_replacement() {
    let mut document = Document::new(Mesh::sphere(0));
    document.add_layer("Source geometry").unwrap();
    record_vertex(&mut document, 0, Vec3::new(0.01, 0.02, -0.03));
    let source: Value = serde_json::from_str(&state(&document)).unwrap();
    let mut existing = Document::new(Mesh::cube());
    translate(&mut existing, Vec3::new(0.1, 0.2, 0.3));
    assert!(existing.undo());
    let before = state(&existing);
    for corruption in 0..4 {
        let mut payload = source.clone();
        match corruption {
            0 => payload["version"] = serde_json::json!(1),
            1 => payload["version"] = serde_json::json!(2),
            2 => payload["mesh"]["positions"][0][0] = serde_json::json!(0.125),
            _ => payload["mesh"]["triangles"][0] = serde_json::json!([0, 1, 2]),
        }
        let decoded = Document::from_json(&payload.to_string());
        assert!(decoded.is_err(), "v3 corruption {corruption} accepted");
        // The caller only replaces a document after successful decoding. This
        // deliberately exercises the actual load transaction boundary.
        if let Ok(imported) = decoded {
            existing.replace_document(imported).unwrap();
        }
        assert_eq!(state(&existing), before);
        assert!(existing.can_redo());
    }
}

#[test]
fn corrupt_layer_metadata_is_rejected_without_partial_document_import() {
    let mut source_document = Document::new(Mesh::sphere(0));
    source_document.add_layer("Stored layer").unwrap();
    record_vertex(&mut source_document, 0, Vec3::new(0.017, 0.021, -0.013));
    let source: Value = serde_json::from_str(&state(&source_document)).unwrap();
    for corruption in 0..13 {
        let mut payload = source.clone();
        match corruption {
            0 => payload["layers"]["scheme"] = serde_json::json!("unknown_scheme"),
            1 => {
                payload["layers"]["base_positions"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            2 => {
                payload["layers"]["entries"][0]["deltas"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            3 => payload["layers"]["recording_layer"] = serde_json::json!(99),
            4 => payload["layers"]["entries"][0]["visible"] = serde_json::json!(false),
            5 => payload["layers"]["entries"][0]["intensity"] = serde_json::json!(0.5),
            6 => payload["layers"]["entries"][0]["intensity"] = serde_json::json!(1.5),
            7 => payload["layers"]["base_positions"][0] = serde_json::json!([1e9, 0.0, 0.0]),
            8 => {
                payload["layers"]["entries"][0]["deltas"][0] = serde_json::json!([1e100, 0.0, 0.0])
            }
            9 => payload["layers"]["entries"][0]["extra"] = serde_json::json!(true),
            10 => payload["layers"]["entries"] = serde_json::json!([]),
            11 => payload["layers"]["triangles"][0] = serde_json::json!([0, 1, 2]),
            _ => {
                let entry = payload["layers"]["entries"][0].clone();
                payload["layers"]["entries"] = Value::Array(vec![entry; 17]);
            }
        }
        assert!(
            Document::from_json(&payload.to_string()).is_err(),
            "layer metadata corruption {corruption} accepted"
        );
    }
    let layers = source["layers"].to_string();
    let duplicate = format!(
        "{{\"version\":3,\"name\":\"Duplicate field\",\"mesh\":{},\"layers\":{{\"scheme\":\"world_offsets_f64_v1\",{}}}}}",
        source["mesh"],
        &layers[1..layers.len() - 1]
    );
    assert!(Document::from_json(&duplicate)
        .unwrap_err()
        .contains("duplicate"));
}

#[test]
fn decode_budget_is_shared_between_base_and_delta_arrays_in_either_field_order() {
    let mesh = serde_json::to_string(&Mesh::sphere(0)).unwrap();
    // Together these exceed 1M vectors, despite each individual array fitting.
    // Compact text avoids the memory of a serde_json Value for 1M arrays.
    let base = format!("[{}[0,0,0]]", "[0,0,0],".repeat(500_000));
    let deltas = format!("[{}[0,0,0]]", "[0,0,0],".repeat(499_999));
    let entry =
        format!("{{\"name\":\"Budget\",\"intensity\":1,\"visible\":true,\"deltas\":{deltas}}}");
    for entries_first in [false, true] {
        let fields = if entries_first {
            format!("\"entries\":[{entry}],\"base_positions\":{base}")
        } else {
            format!("\"base_positions\":{base},\"entries\":[{entry}]")
        };
        let bytes = format!(
            "{{\"version\":3,\"name\":\"Bounded decoder\",\"mesh\":{mesh},\"layers\":{{\"scheme\":\"world_offsets_f64_v1\",\"triangles\":[[0,1,2]],\"recording_layer\":null,{fields}}}}}"
        );
        assert!(
            Document::from_json(&bytes)
                .unwrap_err()
                .contains("aggregate"),
            "combined vector budget was deferred for entries_first={entries_first}"
        );
    }
}
