//! Independent persistent-level contracts. World-space detail transport is not
//! ZBrush multiresolution/HD parity or normal-frame displacement transport.
use obrush_core::{DecimateSettings, Document, Mesh, Vec3};
use serde_json::{json, Value};

fn exact_mesh(a: &Mesh, b: &Mesh) {
    assert_eq!(a.positions, b.positions);
    assert_eq!(a.normals, b.normals);
    assert_eq!(a.triangles, b.triangles);
    assert_eq!(a.masks, b.masks);
    assert_eq!(a.colors, b.colors);
}

fn json_state(document: &Document) -> String {
    document.to_json().unwrap()
}

fn edited_levels() -> Document {
    let mut document = Document::new(Mesh::sphere(1));
    document.name = "Independent multires fixture".into();
    document.add_subdivision_level().unwrap();
    document.add_subdivision_level().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0] += Vec3::new(0.03, 0.02, 0.07);
    document.active_mesh_mut().unwrap().masks[0] = 0.45;
    document.active_mesh_mut().unwrap().colors[0] = [0.1, 0.2, 0.9];
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document
}

#[test]
fn levels_navigate_without_geometric_drift_or_extra_undo_entries() {
    let mut document = Document::new(Mesh::sphere(1));
    let coarse = document.active_mesh().unwrap().snapshot();
    document.add_subdivision_level().unwrap();
    let fine = document.active_mesh().unwrap().snapshot();
    assert_eq!((document.current_level(), document.level_count()), (1, 2));
    assert_eq!(fine.triangles.len(), coarse.triangles.len() * 4);
    for _ in 0..5 {
        document.select_level(0).unwrap();
        exact_mesh(document.active_mesh().unwrap(), &coarse);
        document.select_level(1).unwrap();
        exact_mesh(document.active_mesh().unwrap(), &fine);
    }
    assert!(document.undo());
    assert_eq!(document.level_count(), 1);
    exact_mesh(document.active_mesh().unwrap(), &coarse);
    assert!(!document.undo());
    assert!(document.redo());
    assert_eq!(document.level_count(), 2);
    exact_mesh(document.active_mesh().unwrap(), &fine);
}

#[test]
fn coarse_translation_retains_fine_sculpt_and_attribute_detail() {
    let mut document = edited_levels();
    let fine = document.active_mesh().unwrap().snapshot();
    document.select_level(0).unwrap();
    let coarse = document.active_mesh().unwrap().snapshot();
    // Fine sculpting is retained on the fine level, never copied into coarse.
    exact_mesh(&coarse, &Mesh::sphere(1));
    let delta = Vec3::new(0.14, -0.06, 0.11);
    document.begin_stroke();
    for point in &mut document.active_mesh_mut().unwrap().positions {
        *point += delta;
    }
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.select_level(2).unwrap();
    for (before, after) in fine
        .positions
        .iter()
        .zip(&document.active_mesh().unwrap().positions)
    {
        assert!((*after - (*before + delta)).length() < 2e-6);
    }
    assert_eq!(document.active_mesh().unwrap().masks, fine.masks);
    assert_eq!(document.active_mesh().unwrap().colors, fine.colors);
    assert_eq!(document.active_mesh().unwrap().triangles, fine.triangles);
}

#[test]
fn localized_coarse_deltas_affect_only_their_refinement_support() {
    let mut document = edited_levels();
    let fine = document.active_mesh().unwrap().snapshot();
    document.select_level(0).unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0] += Vec3::Z * 0.05;
    document.active_mesh_mut().unwrap().masks[0] = 0.4;
    document.active_mesh_mut().unwrap().colors[0] = [0.7, 0.3, 0.1];
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.select_level(2).unwrap();
    // Original icosphere vertex3 is opposite vertex0 and outside two Loop
    // interpolation neighborhoods. The test does not copy interpolation code.
    assert_eq!(
        document.active_mesh().unwrap().positions[3],
        fine.positions[3]
    );
    assert_eq!(document.active_mesh().unwrap().masks[3], fine.masks[3]);
    assert_eq!(document.active_mesh().unwrap().colors[3], fine.colors[3]);
    assert_ne!(
        document.active_mesh().unwrap().positions[0],
        fine.positions[0]
    );
    assert!(document
        .active_mesh()
        .unwrap()
        .masks
        .iter()
        .all(|m| (0.0..=1.0).contains(m)));
    assert!(document
        .active_mesh()
        .unwrap()
        .colors
        .iter()
        .flatten()
        .all(|c| (0.0..=1.0).contains(c)));
}

#[test]
fn dirty_coarse_project_roundtrip_retains_high_detail_and_all_fields() {
    let mut document = edited_levels();
    document.select_level(0).unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].x += 0.02;
    document.active_mesh_mut().unwrap().masks[1] = 0.8;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    let bytes = json_state(&document);
    let value: Value = serde_json::from_str(&bytes).unwrap();
    assert_eq!(value["version"], 2);
    assert!(value["multiresolution"]["levels"][0].is_null());
    let mut restored = Document::from_json(&bytes).unwrap();
    assert_eq!(restored.level_count(), 3);
    assert_eq!(restored.current_level(), 0);
    assert_eq!(json_state(&restored), bytes);
    document.select_level(2).unwrap();
    restored.select_level(2).unwrap();
    exact_mesh(
        restored.active_mesh().unwrap(),
        document.active_mesh().unwrap(),
    );
    assert_eq!(json_state(&restored), json_state(&document));
    assert!(!restored.can_undo());
}

#[test]
fn sculpt_undo_redo_and_topology_replacement_restore_the_whole_stack() {
    let mut document = edited_levels();
    let before = json_state(&document);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[2].z += 0.03;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    let after = json_state(&document);
    assert!(document.undo());
    assert_eq!(json_state(&document), before);
    assert!(document.redo());
    assert_eq!(json_state(&document), after);
    document.replace_mesh(Mesh::cube()).unwrap();
    assert_eq!(document.level_count(), 1);
    let replacement = json_state(&document);
    assert!(document.undo());
    assert_eq!(json_state(&document), after);
    assert_eq!(document.level_count(), 3);
    assert!(document.redo());
    assert_eq!(json_state(&document), replacement);
}

#[test]
fn imports_replace_name_levels_and_mesh_as_one_undoable_state() {
    let mut existing = Document::new(Mesh::cube());
    existing.name = "Original document".into();
    let old = json_state(&existing);
    let imported = edited_levels();
    let new = json_state(&imported);
    existing.replace_document(imported).unwrap();
    assert_eq!(json_state(&existing), new);
    assert!(existing.undo());
    assert_eq!(json_state(&existing), old);
    assert!(existing.redo());
    assert_eq!(json_state(&existing), new);
    let mut invalid = Document::new(Mesh::sphere(0));
    invalid.name = "x".repeat(4097);
    assert!(existing.replace_document(invalid).is_err());
    assert_eq!(json_state(&existing), new);
}

#[test]
fn failures_and_decimation_noop_preserve_hierarchy_and_available_redo() {
    let mut document = edited_levels();
    assert!(document.undo()); // Leave a genuine fine-stroke redo available.
    let original = json_state(&document);
    assert!(document.can_redo());
    let no_op = DecimateSettings {
        target_ratio: 1.0,
        ..DecimateSettings::default()
    };
    assert_eq!(document.decimate(&no_op).unwrap().collapses, 0);
    assert_eq!(json_state(&document), original);
    assert!(document.can_redo());
    assert!(document.select_level(usize::MAX).is_err());
    assert_eq!(json_state(&document), original);
    assert!(document.can_redo());
    document.select_level(0).unwrap();
    let lower = json_state(&document);
    assert!(document
        .add_subdivision_level()
        .unwrap_err()
        .contains("highest"));
    assert_eq!(json_state(&document), lower);
    assert!(document.can_redo());
}

#[test]
fn original_v1_files_migrate_and_malformed_hierarchies_are_rejected() {
    let document = edited_levels();
    let original_bytes = json_state(&document);
    let valid: Value = serde_json::from_str(&original_bytes).unwrap();
    let mut original = valid.clone();
    original.as_object_mut().unwrap().remove("multiresolution");
    original["version"] = json!(1);
    let migrated = Document::from_json(&original.to_string()).unwrap();
    assert_eq!(migrated.level_count(), 1);
    exact_mesh(
        migrated.active_mesh().unwrap(),
        document.active_mesh().unwrap(),
    );
    assert_eq!(
        serde_json::from_str::<Value>(&json_state(&migrated)).unwrap()["version"],
        2
    );
    for change in 0..9 {
        let mut damaged = valid.clone();
        match change {
            0 => damaged["multiresolution"]["active_level"] = json!(99),
            1 => damaged["multiresolution"]["scheme"] = json!("unrecognized_scheme"),
            2 => damaged["multiresolution"]["levels"][0] = Value::Null,
            3 => damaged["multiresolution"]["levels"][0]["masks"][0] = json!(1.5),
            4 => {
                damaged["mesh"]["triangles"]
                    .as_array_mut()
                    .unwrap()
                    .swap(0, 1);
            }
            5 => damaged["multiresolution"]["extra"] = json!("unsupported"),
            6 => damaged["version"] = json!(1),
            7 => damaged["mesh"]["positions"][0] = damaged["mesh"]["positions"][162].clone(),
            _ => {
                damaged["multiresolution"]["levels"][0]["positions"][0] =
                    damaged["multiresolution"]["levels"][0]["positions"][12].clone()
            }
        }
        assert!(
            Document::from_json(&damaged.to_string()).is_err(),
            "corruption {change} accepted"
        );
    }
    assert_eq!(json_state(&document), original_bytes);
}

#[test]
fn brush_topology_change_drops_levels_with_exact_undo() {
    let mut document = edited_levels();
    let before = json_state(&document);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().subdivide().unwrap();
    document.end_stroke();
    assert_eq!(document.level_count(), 1);
    assert!(document.undo());
    assert_eq!(json_state(&document), before);
}

#[test]
fn hierarchy_decode_bounds_inactive_and_active_inclusive_allocation() {
    // Compact, valid mesh storage with many unused vertices intentionally
    // exercises aggregate decoding before unsupported-topology inspection.
    let mut large = Mesh::sphere(0);
    large.positions.resize(550_000, Vec3::ZERO);
    large.normals.resize(550_000, Vec3::ZERO);
    large.masks.resize(550_000, 0.0);
    large.colors.resize(550_000, [0.0; 3]);
    let large_json = serde_json::to_string(&large).unwrap();
    let small_json = serde_json::to_string(&Mesh::sphere(0)).unwrap();
    let prefix = "{\"version\":2,\"name\":\"allocation fixture\",\"mesh\":";
    let metadata = ",\"multiresolution\":{\"scheme\":\"loop_world_offsets_v1\",\"active_level\":0,\"levels\":[null,";
    let inactive = format!("{prefix}{small_json}{metadata}{large_json},{large_json}]}}}}");
    assert!(Document::from_json(&inactive)
        .unwrap_err()
        .contains("aggregate"));
    let inclusive = format!("{prefix}{large_json}{metadata}{large_json}]}}}}");
    assert!(Document::from_json(&inclusive)
        .unwrap_err()
        .contains("aggregate"));
    let mut too_many: Value = serde_json::from_str(&json_state(&edited_levels())).unwrap();
    too_many["multiresolution"]["levels"] =
        json!([null, null, null, null, null, null, null, null, null]);
    assert!(Document::from_json(&too_many.to_string())
        .unwrap_err()
        .contains("Too many"));
}
