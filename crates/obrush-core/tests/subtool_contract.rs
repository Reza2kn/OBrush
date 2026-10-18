//! Independent SubTool foundation contracts. These locally created fixtures
//! establish OBrush behavior only, not reference SubTool/folder equivalence.
use obrush_core::{
    execute_topology_project, DecimateSettings, Document, Mesh, Ray, TopologyOperation, Vec3,
};
use serde_json::{json, Value};
use std::sync::Arc;

fn state(document: &Document) -> String {
    document.to_json().unwrap()
}

fn json_state(document: &Document) -> Value {
    serde_json::from_str(&state(document)).unwrap()
}

fn payload(document: &Document) -> Value {
    let value = json_state(document);
    let mut result = serde_json::Map::new();
    for key in ["mesh", "multiresolution", "layers", "morph_target"] {
        if let Some(value) = value.get(key) {
            result.insert(key.into(), value.clone());
        }
    }
    Value::Object(result)
}

fn edit(document: &mut Document, vertex: usize) {
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[vertex] += Vec3::new(0.0021, -0.0017, 0.0033);
    document.active_mesh_mut().unwrap().masks[vertex] = 0.37;
    document.active_mesh_mut().unwrap().colors[vertex] = [0.13, 0.29, 0.91];
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
}

fn rich() -> Document {
    let mut document = Document::new(Mesh::sphere(1));
    document.name = "Whole scene title".into();
    document.add_subdivision_level().unwrap();
    document.store_morph_target().unwrap();
    document.add_layer("Fine offsets").unwrap();
    edit(&mut document, 17);
    document
}

fn pair() -> (Document, u32, u32) {
    let mut document = rich();
    let a = document.active_subtool_id().unwrap();
    let b = document
        .add_subtool(Mesh::sphere(2), "Other object")
        .unwrap();
    (document, a, b)
}

fn simple_pair() -> (Document, u32, u32) {
    let mut document = Document::new(Mesh::sphere(1));
    let a = document.active_subtool_id().unwrap();
    let b = document.add_subtool(Mesh::sphere(1), "Second").unwrap();
    (document, a, b)
}

#[test]
fn deleting_selected_subtools_is_atomic_and_preserves_survivor_and_history() {
    let (mut document, first, middle) = pair();
    let last = document
        .add_subtool(Mesh::sphere(0), "Last object")
        .unwrap();
    document
        .select_subtools(&[last, first], Some(first))
        .unwrap();
    let before = state(&document);
    document.delete_selected_subtools().unwrap();
    assert_eq!(document.subtool_count(), 1);
    assert_eq!(document.active_subtool_id(), Some(middle));
    assert_eq!(document.selected_subtool_ids(), vec![middle]);
    let after = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), after);

    // Deleting the entire remaining selection yields the source's empty-scene
    // case through the existing checked clear transaction.
    document.delete_selected_subtools().unwrap();
    assert_eq!(document.subtool_count(), 0);
    assert!(document.active_subtool_id().is_none());
    assert!(document.undo());
    assert_eq!(state(&document), after);
}

#[test]
fn selected_duplicate_appends_full_copies_with_source_order_and_two_history_steps() {
    let (mut document, _first, middle) = pair();
    let last = document
        .add_subtool(Mesh::sphere(0), "Last object")
        .unwrap();
    document
        .select_subtools(&[last, middle], Some(middle))
        .unwrap();
    let before = state(&document);
    let original_order: Vec<_> = document
        .subtool_summaries()
        .iter()
        .map(|object| object.id)
        .collect();
    let copied = document.duplicate_selected_subtools().unwrap();
    assert_eq!(copied.len(), 2);
    assert_eq!(document.subtool_count(), 5);
    let order: Vec<_> = document
        .subtool_summaries()
        .iter()
        .map(|object| object.id)
        .collect();
    assert_eq!(&order[..3], original_order.as_slice());
    assert_eq!(&order[3..], copied.as_slice());
    assert_eq!(document.active_subtool_id(), Some(middle));
    assert_eq!(document.selected_subtool_ids(), vec![middle]);
    let after = state(&document);
    let mut comparison = Document::from_json(&after).unwrap();
    comparison.select_subtool(last).unwrap();
    let last_payload = payload(&comparison);
    comparison.select_subtool(copied[0]).unwrap();
    assert_eq!(payload(&comparison), last_payload);
    comparison.select_subtool(middle).unwrap();
    let middle_payload = payload(&comparison);
    comparison.select_subtool(copied[1]).unwrap();
    assert_eq!(payload(&comparison), middle_payload);

    assert!(document.undo());
    assert_eq!(document.subtool_count(), 4);
    assert_eq!(document.selected_subtool_ids(), vec![copied[0]]);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(document.subtool_count(), 4);
    assert_eq!(document.selected_subtool_ids(), vec![copied[0]]);
    assert!(document.redo());
    assert_eq!(state(&document), after);
}

#[test]
fn selected_isolation_and_show_all_change_visibility_with_whole_scene_history() {
    let (mut document, first, middle) = pair();
    let last = document
        .add_subtool(Mesh::sphere(0), "Last object")
        .unwrap();
    document
        .select_subtools(&[last, middle], Some(middle))
        .unwrap();
    let before = state(&document);
    document.isolate_selected_subtools().unwrap();
    assert_eq!(
        document
            .subtool_summaries()
            .iter()
            .map(|part| (part.id, part.visible))
            .collect::<Vec<_>>(),
        vec![(first, false), (middle, true), (last, true)]
    );
    assert_eq!(document.selected_subtool_ids(), vec![last, middle]);
    let isolated = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), isolated);

    document.show_all_subtools().unwrap();
    assert!(document.subtool_summaries().iter().all(|part| part.visible));
    assert_eq!(state(&document), before);
    assert!(document.undo());
    assert_eq!(state(&document), isolated);
    assert!(document.redo());
    assert_eq!(state(&document), before);

    // A hidden selected part remains hidden during isolation; Show all makes
    // every part visible, including parts hidden before the isolate action.
    document.set_subtool_visible(middle, false).unwrap();
    let mixed = state(&document);
    document.isolate_selected_subtools().unwrap();
    assert_eq!(
        document
            .subtool_summaries()
            .iter()
            .map(|part| part.visible)
            .collect::<Vec<_>>(),
        vec![false, false, true]
    );
    assert!(document.undo());
    assert_eq!(state(&document), mixed);
    document.show_all_subtools().unwrap();
    assert!(document.subtool_summaries().iter().all(|part| part.visible));
    assert!(document.undo());
    assert_eq!(state(&document), mixed);

    document.set_solo(true).unwrap();
    let solo = state(&document);
    document.isolate_selected_subtools().unwrap();
    assert!(!document.solo());
    assert_eq!(
        document
            .subtool_summaries()
            .iter()
            .map(|part| part.effective_visible)
            .collect::<Vec<_>>(),
        vec![false, false, true]
    );
    assert!(document.undo());
    assert_eq!(state(&document), solo);
}

fn surface_bits(mesh: &Mesh) -> Value {
    json!({
        "positions": mesh.positions.iter().map(|p| p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "normals": mesh.normals.iter().map(|p| p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "masks": mesh.masks.iter().map(|p| p.to_bits()).collect::<Vec<_>>(),
        "colors": mesh.colors.iter().map(|p| p.map(f32::to_bits)).collect::<Vec<_>>(),
        "triangles": mesh.triangles,
    })
}

fn ordered(value: &Value, reverse: bool) -> String {
    match value {
        Value::Object(map) => {
            let mut fields: Vec<_> = map.iter().collect();
            if reverse {
                fields.reverse();
            }
            format!(
                "{{{}}}",
                fields
                    .into_iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        ordered(value, reverse)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|v| ordered(v, reverse))
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => value.to_string(),
    }
}

#[test]
fn implicit_scene_readers_and_true_noops_do_not_promote_or_consume_history() {
    let mut document = Document::new(Mesh::sphere(0));
    let before = state(&document);
    assert_eq!(document.active_subtool_id().unwrap(), 1);
    assert_eq!(document.subtool_count(), 1);
    assert!(!document.solo());
    let name = document.subtool_summaries()[0].name.clone();
    document.select_subtool(1).unwrap();
    document.set_subtool_name(1, &name).unwrap();
    document.set_subtool_visible(1, true).unwrap();
    document.set_solo(false).unwrap();
    document.reorder_subtool(1, 0).unwrap();
    assert_eq!(document.subtool_render_snapshots().len(), 1);
    assert!(document
        .raycast_subtools(Ray {
            origin: Vec3::Z * 4.0,
            direction: -Vec3::Z
        })
        .unwrap()
        .is_some());
    assert_eq!(state(&document), before);
    assert_eq!(json_state(&document)["version"], 2);
    assert!(!document.undo());
    document.set_subtool_name(1, "Object A").unwrap();
    assert_eq!(json_state(&document)["version"], 5);
    let promoted = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), promoted);
    document.set_subtool_name(1, &name).unwrap();
    assert_eq!(json_state(&document)["version"], 5);
}

#[test]
fn switching_preserves_complete_per_object_state_without_navigation_undo() {
    let mut document = rich();
    let before_add = state(&document);
    let a_payload = payload(&document);
    let a = document.active_subtool_id().unwrap();
    let b = document
        .add_subtool(Mesh::sphere(0), "Small object")
        .unwrap();
    let b_payload = payload(&document);
    for _ in 0..3 {
        document.select_subtool(a).unwrap();
        assert_eq!(payload(&document), a_payload);
        assert_eq!(
            (
                document.level_count(),
                document.current_level(),
                document.layer_count()
            ),
            (2, 1, 1)
        );
        assert!(document.morph_target_summary().unwrap().available);
        document.select_subtool(b).unwrap();
        assert_eq!(payload(&document), b_payload);
        assert_eq!((document.level_count(), document.layer_count()), (1, 0));
        assert!(!document.has_morph_target());
    }
    let added = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), before_add);
    assert!(document.redo());
    assert_eq!(state(&document), added);
}

#[test]
fn edit_undo_restores_its_object_and_redo_restores_the_pre_undo_selection() {
    let (mut document, a, b) = simple_pair();
    document.select_subtool(a).unwrap();
    let before = payload(&document);
    edit(&mut document, 7);
    let edited = payload(&document);
    document.select_subtool(b).unwrap();
    let selected_b = state(&document);
    assert!(document.undo());
    assert_eq!(document.active_subtool_id().unwrap(), a);
    assert_eq!(payload(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), selected_b);
    document.select_subtool(a).unwrap();
    assert_eq!(payload(&document), edited);
}

#[test]
fn exact_duplicate_copies_rich_inactive_state_but_later_edits_are_independent() {
    let (mut document, a, b) = pair();
    let b_payload = payload(&document);
    document.select_subtool(a).unwrap();
    let a_payload = payload(&document);
    document.select_subtool(b).unwrap();
    let before_duplicate = state(&document);
    let c = document.duplicate_subtool(a).unwrap();
    assert_ne!(c, a);
    assert_ne!(c, b);
    assert_eq!(document.active_subtool_id().unwrap(), c);
    assert_eq!(payload(&document), a_payload);
    let duplicated = state(&document);
    edit(&mut document, 23);
    document.set_layer_name(0, "Duplicate offsets").unwrap();
    document.store_morph_target().unwrap();
    let changed_c = payload(&document);
    assert_ne!(changed_c, a_payload);
    document.select_subtool(a).unwrap();
    assert_eq!(payload(&document), a_payload);
    document.select_subtool(b).unwrap();
    assert_eq!(payload(&document), b_payload);
    document.select_subtool(c).unwrap();
    assert_eq!(payload(&document), changed_c);
    let mut original = Document::from_json(&duplicated).unwrap();
    // A freshly imported scene does not inherit the producer's history.
    assert!(!original.undo());
    // Duplicate creation is one exact scene history item in the producer.
    let mut clean = Document::from_json(&before_duplicate).unwrap();
    clean.duplicate_subtool(a).unwrap();
    assert!(clean.undo());
    assert_eq!(state(&clean), before_duplicate);
}

#[test]
fn scene_metadata_actions_preserve_ids_geometry_and_project_title() {
    let (mut document, a, b) = pair();
    let original = state(&document);
    let before = document.subtool_summaries();
    document.set_subtool_name(a, "Renamed 独立").unwrap();
    assert_eq!(document.name, "Whole scene title");
    let renamed = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), original);
    assert!(document.redo());
    assert_eq!(state(&document), renamed);
    document.reorder_subtool(b, 0).unwrap();
    assert_eq!(
        document
            .subtool_summaries()
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        vec![b, a]
    );
    let reordered = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), renamed);
    assert!(document.redo());
    assert_eq!(state(&document), reordered);
    document.set_subtool_visible(a, false).unwrap();
    let hidden = state(&document);
    document.set_solo(true).unwrap();
    let solo = state(&document);
    assert!(document.undo());
    assert_eq!(state(&document), hidden);
    assert!(document.redo());
    assert_eq!(state(&document), solo);
    for summary in document.subtool_summaries() {
        let old = before.iter().find(|s| s.id == summary.id).unwrap();
        assert_eq!(
            (summary.vertices, summary.triangles),
            (old.vertices, old.triangles)
        );
    }
}

#[test]
fn deletion_retains_one_object_and_allocator_does_not_reuse_deleted_ids() {
    let (mut document, a, b) = simple_pair();
    let c = document.add_subtool(Mesh::sphere(0), "Third").unwrap();
    let before = state(&document);
    document.delete_subtool(c).unwrap();
    assert_eq!(document.subtool_count(), 2);
    assert_eq!(document.active_subtool_id().unwrap(), b);
    assert!(document.undo());
    assert_eq!(state(&document), before);
    document.delete_subtool(c).unwrap();
    let d = document.add_subtool(Mesh::sphere(0), "Fourth").unwrap();
    assert!(d > c);
    document.delete_subtool(a).unwrap();
    assert_eq!(document.active_subtool_id().unwrap(), d);
    document.delete_subtool(b).unwrap();
    let one = state(&document);
    assert_eq!(document.subtool_count(), 1);
    assert_eq!(json_state(&document)["version"], 5);
    document.delete_subtool(d).unwrap();
    assert_eq!(document.subtool_count(), 0);
    assert!(document.active_mesh().is_none());
    assert!(document.undo());
    assert_eq!(state(&document), one);
}

#[test]
fn visibility_solo_nearest_picking_and_active_overlap_ties_are_explicit() {
    let mut document = Document::new(Mesh::sphere(1));
    let a = document.active_subtool_id().unwrap();
    let mut near = Mesh::sphere(1);
    for position in &mut near.positions {
        *position += Vec3::Z * 2.0;
    }
    near.recompute_normals();
    let b = document.add_subtool(near, "Near").unwrap();
    let ray = Ray {
        origin: Vec3::Z * 6.0,
        direction: -Vec3::Z,
    };
    let hit = document.raycast_subtools(ray).unwrap().unwrap();
    assert_eq!(hit.id, b);
    let scaled = document
        .raycast_subtools(Ray {
            direction: -Vec3::Z * 3.7,
            ..ray
        })
        .unwrap()
        .unwrap();
    assert_eq!(scaled.id, b);
    assert!((hit.hit.distance - scaled.hit.distance).abs() <= 1e-6);
    document.set_subtool_visible(b, false).unwrap();
    assert!(!document.active_subtool_visible());
    assert_eq!(document.raycast_subtools(ray).unwrap().unwrap().id, a);
    document.set_solo(true).unwrap();
    assert!(document.raycast_subtools(ray).unwrap().is_none());
    assert!(document.subtool_render_snapshots().is_empty());
    document.select_subtool(a).unwrap();
    assert_eq!(document.raycast_subtools(ray).unwrap().unwrap().id, a);
    document.set_solo(false).unwrap();
    assert!(
        !document
            .subtool_summaries()
            .iter()
            .find(|s| s.id == b)
            .unwrap()
            .visible
    );
    let c = document.duplicate_subtool(a).unwrap();
    assert_eq!(document.raycast_subtools(ray).unwrap().unwrap().id, c);
    document.select_subtool(a).unwrap();
    assert_eq!(document.raycast_subtools(ray).unwrap().unwrap().id, a);
    assert!(document
        .raycast_subtools(Ray {
            direction: Vec3::ZERO,
            ..ray
        })
        .unwrap()
        .is_none());
}

#[test]
fn inactive_render_arcs_are_stable_and_held_snapshots_survive_edits() {
    let (mut document, a, b) = pair();
    let first = document.subtool_render_snapshots();
    let second = document.subtool_render_snapshots();
    let held = first.iter().find(|s| s.id == a).unwrap().mesh.clone();
    assert!(Arc::ptr_eq(
        &held,
        &second.iter().find(|s| s.id == a).unwrap().mesh
    ));
    let frozen = surface_bits(&held);
    let b_surface = surface_bits(document.active_mesh().unwrap());
    document.select_subtool(a).unwrap();
    edit(&mut document, 12);
    assert_eq!(surface_bits(&held), frozen);
    assert_ne!(surface_bits(document.active_mesh().unwrap()), frozen);
    document.select_subtool(b).unwrap();
    assert_eq!(surface_bits(document.active_mesh().unwrap()), b_surface);
    assert_eq!(surface_bits(&held), frozen);
}

#[test]
fn failed_and_noop_scene_actions_preserve_exact_state_and_available_redo() {
    let (mut document, a, b) = simple_pair();
    edit(&mut document, 2);
    let edited = state(&document);
    assert!(document.undo());
    let before = state(&document);
    document.select_subtool(b).unwrap();
    document.set_subtool_name(b, "Second").unwrap();
    document.set_subtool_visible(b, true).unwrap();
    document.set_solo(false).unwrap();
    document.reorder_subtool(b, 1).unwrap();
    for name in ["", "\n", "abc\u{7f}", &"x".repeat(257), &"é".repeat(129)] {
        assert!(document.set_subtool_name(a, name).is_err());
    }
    assert!(document.select_subtool(999).is_err());
    assert!(document.duplicate_subtool(999).is_err());
    assert!(document.delete_subtool(999).is_err());
    assert!(document.reorder_subtool(a, 99).is_err());
    let mut invalid = Mesh::sphere(0);
    invalid.positions[0].x = f32::NAN;
    assert!(document.add_subtool(invalid, "Invalid").is_err());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), edited);
}

#[test]
fn invalid_pending_stroke_aborts_every_dependent_scene_action_and_keeps_redo() {
    for action in 0..8 {
        let (mut document, a, b) = simple_pair();
        edit(&mut document, 3);
        let redo = state(&document);
        assert!(document.undo());
        let before = state(&document);
        document.begin_stroke();
        document.active_mesh_mut().unwrap().positions[0].x = f32::NAN;
        let result = match action {
            0 => document.select_subtool(a),
            1 => document.add_subtool(Mesh::sphere(0), "Added").map(|_| ()),
            2 => document.duplicate_subtool(a).map(|_| ()),
            3 => document.set_subtool_name(a, "New name"),
            4 => document.set_subtool_visible(a, false),
            5 => document.set_solo(true),
            6 => document.reorder_subtool(a, 1),
            _ => document.delete_subtool(b),
        };
        assert!(result.is_err(), "pending invalid action {action} proceeded");
        assert_eq!(
            state(&document),
            before,
            "action {action} changed the scene"
        );
        assert!(document.redo(), "action {action} consumed redo");
        assert_eq!(state(&document), redo);
    }
}

#[test]
fn max_objects_names_and_allocator_exhaustion_are_atomic() {
    let mut document = Document::new(Mesh::sphere(0));
    for index in 1..32 {
        document
            .add_subtool(Mesh::sphere(0), &format!("Object {index}"))
            .unwrap();
    }
    let before = state(&document);
    assert!(document.add_subtool(Mesh::sphere(0), "33rd").is_err());
    assert!(document.duplicate_subtool(1).is_err());
    assert_eq!(state(&document), before);
    let (source, _, _) = simple_pair();
    let mut value = json_state(&source);
    value["scene"]["next_id"] = json!(u32::MAX);
    let mut exhausted = Document::from_json(&value.to_string()).unwrap();
    let before = state(&exhausted);
    assert!(exhausted.add_subtool(Mesh::sphere(0), "Added").is_err());
    assert!(exhausted.duplicate_subtool(1).is_err());
    assert_eq!(state(&exhausted), before);
    exhausted.set_subtool_name(1, &"é".repeat(128)).unwrap();
    let duplicated = exhausted.duplicate_subtool(1);
    assert!(duplicated.is_err());
}

#[test]
fn v5_import_rejects_hostile_scene_membership_identity_and_names_in_both_field_orders() {
    let (document, _, _) = simple_pair();
    let before = state(&document);
    let valid = json_state(&document);
    let mut mutations = Vec::new();
    let mut add = |label: &str, mutate: fn(&mut Value)| {
        let mut value = valid.clone();
        mutate(&mut value);
        mutations.push((label.to_owned(), value));
    };
    add("duplicate IDs", |v| {
        v["scene"]["objects"][1]["id"] = json!(1)
    });
    add("zero ID", |v| v["scene"]["objects"][0]["id"] = json!(0));
    add("absent active", |v| v["scene"]["active_id"] = json!(99));
    add("colliding next", |v| v["scene"]["next_id"] = json!(2));
    add("u32 overflow", |v| {
        v["scene"]["next_id"] = json!(4_294_967_296_u64)
    });
    add("active nonnull", |v| {
        v["scene"]["objects"][1]["payload"] = v["scene"]["objects"][0]["payload"].clone()
    });
    add("inactive null", |v| {
        v["scene"]["objects"][0]["payload"] = Value::Null
    });
    add("missing payload", |v| {
        v["scene"]["objects"][0]
            .as_object_mut()
            .unwrap()
            .remove("payload");
    });
    add("empty scene", |v| v["scene"]["objects"] = json!([]));
    add("unknown scheme", |v| {
        v["scene"]["scheme"] = json!("folders_unverified")
    });
    add("unknown scene field", |v| v["scene"]["folder"] = json!(0));
    add("empty name", |v| {
        v["scene"]["objects"][0]["name"] = json!("")
    });
    add("control name", |v| {
        v["scene"]["objects"][0]["name"] = json!("hello\nworld")
    });
    add("overbyte name", |v| {
        v["scene"]["objects"][0]["name"] = json!("é".repeat(129))
    });
    add("version5 no scene", |v| {
        v.as_object_mut().unwrap().remove("scene");
    });
    for version in 1..5 {
        let mut value = valid.clone();
        value["version"] = json!(version);
        mutations.push((format!("legacy version{version} scene"), value));
    }
    let mut too_many = valid.clone();
    let objects = too_many["scene"]["objects"].as_array_mut().unwrap();
    let template = objects[0].clone();
    for id in 3..=33 {
        let mut object = template.clone();
        object["id"] = json!(id);
        objects.push(object);
    }
    too_many["scene"]["next_id"] = json!(34);
    mutations.push(("33 objects".into(), too_many));
    for (label, value) in mutations {
        for reverse in [false, true] {
            assert!(
                Document::from_json(&ordered(&value, reverse)).is_err(),
                "accepted {label}, reverse={reverse}"
            );
        }
    }
    assert_eq!(state(&document), before);
}

#[test]
fn malformed_inactive_geometry_hierarchy_layers_and_targets_are_never_ignored() {
    let (document, _, _) = pair();
    let valid = json_state(&document);
    let inactive = ["scene", "objects"];
    assert!(valid[inactive[0]][inactive[1]][0]["payload"]["layers"].is_object());
    let mut variants = Vec::new();
    let mut push = |label: &str, mutate: fn(&mut Value)| {
        let mut value = valid.clone();
        mutate(&mut value["scene"]["objects"][0]["payload"]);
        variants.push((label.to_owned(), value));
    };
    push("position type", |p| {
        p["mesh"]["positions"][0][0] = json!("NaN")
    });
    push("mask range", |p| p["mesh"]["masks"][0] = json!(1.1));
    push("attribute count", |p| {
        p["mesh"]["colors"].as_array_mut().unwrap().pop();
    });
    push("bad index", |p| {
        p["mesh"]["triangles"][0][0] = json!(99_999)
    });
    push("collapsed face", |p| {
        p["mesh"]["triangles"][0][0] = p["mesh"]["triangles"][0][1].clone()
    });
    push("hierarchy level", |p| {
        p["multiresolution"]["active_level"] = json!(99)
    });
    push("layer composition", |p| {
        p["layers"]["base_positions"][0][0] = json!(12.0)
    });
    push("target origin", |p| {
        p["morph_target"]["creation_level"] = json!(99)
    });
    push("target topology", |p| {
        p["morph_target"]["triangles"][0][0] = json!(99_999)
    });
    push("unknown payload field", |p| p["proprietary"] = json!(true));
    for (label, value) in variants {
        for reverse in [false, true] {
            assert!(
                Document::from_json(&ordered(&value, reverse)).is_err(),
                "accepted inactive {label}, reverse={reverse}"
            );
        }
    }
}

#[test]
fn legacy_versions_remain_readable_and_rich_scene_roundtrip_is_canonical() {
    let mut plain = Document::new(Mesh::sphere(0));
    let v2 = json_state(&plain);
    let mut v1 = v2.clone();
    v1["version"] = json!(1);
    assert_eq!(
        payload(&Document::from_json(&v1.to_string()).unwrap()),
        payload(&plain)
    );
    plain.add_layer("Old layer").unwrap();
    assert_eq!(json_state(&plain)["version"], 3);
    assert_eq!(
        state(&Document::from_json(&state(&plain)).unwrap()),
        state(&plain)
    );
    plain.store_morph_target().unwrap();
    assert_eq!(json_state(&plain)["version"], 4);
    assert_eq!(
        state(&Document::from_json(&state(&plain)).unwrap()),
        state(&plain)
    );
    let (document, _, _) = pair();
    let exact = state(&document);
    for reverse in [false, true] {
        let imported = Document::from_json(&ordered(&json_state(&document), reverse)).unwrap();
        assert_eq!(state(&imported), exact);
    }
}

#[test]
fn whole_scene_import_is_one_undo_and_does_not_import_foreign_history() {
    let (source, _, _) = pair();
    let mut recipient = Document::new(Mesh::sphere(0));
    recipient.name = "Recipient title".into();
    recipient.set_subtool_name(1, "Recipient object").unwrap();
    edit(&mut recipient, 3);
    let before = state(&recipient);
    let mut imported = Document::from_json(&state(&source)).unwrap();
    assert!(!imported.undo());
    recipient.replace_document(imported).unwrap();
    let after = state(&recipient);
    assert_eq!(after, state(&source));
    assert!(recipient.undo());
    assert_eq!(state(&recipient), before);
    assert!(recipient.redo());
    assert_eq!(state(&recipient), after);
}

#[test]
fn worker_reduction_changes_only_active_object_and_noop_preserves_whole_scene() {
    let (mut document, a, b) = pair();
    document.store_morph_target().unwrap();
    let before = state(&document);
    let inactive = json_state(&document)["scene"]["objects"][0]["payload"].clone();
    let operation = TopologyOperation::Decimate(DecimateSettings {
        target_ratio: 1.0,
        ..DecimateSettings::default()
    });
    let output: Value = serde_json::from_str(
        &execute_topology_project(&before, &serde_json::to_string(&operation).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(output["project"].as_str().unwrap(), before);
    let operation = TopologyOperation::Decimate(DecimateSettings {
        target_ratio: 0.7,
        max_error: 0.2,
        preserve_sharp_features: false,
        ..DecimateSettings::default()
    });
    let output: Value = serde_json::from_str(
        &execute_topology_project(&before, &serde_json::to_string(&operation).unwrap()).unwrap(),
    )
    .unwrap();
    assert!(output["report"]["report"]["collapses"].as_u64().unwrap() > 0);
    let mut reduced = Document::from_json(output["project"].as_str().unwrap()).unwrap();
    assert_eq!(reduced.active_subtool_id().unwrap(), b);
    assert!(!reduced.has_morph_target());
    assert_eq!(
        json_state(&reduced)["scene"]["objects"][0]["payload"],
        inactive
    );
    reduced.select_subtool(a).unwrap();
    assert!(reduced.has_morph_target());
    assert_eq!(reduced.layer_count(), 1);
    document
        .replace_document(Document::from_json(output["project"].as_str().unwrap()).unwrap())
        .unwrap();
    assert!(document.undo());
    assert_eq!(state(&document), before);
}

fn oversized_mesh(count: usize) -> Mesh {
    let mut mesh = Mesh::sphere(0);
    mesh.positions.resize(count, Vec3::ZERO);
    mesh.normals.resize(count, Vec3::ZERO);
    mesh.masks.resize(count, 0.0);
    mesh.colors.resize(count, [0.0; 3]);
    mesh.validate().unwrap();
    mesh
}

fn two_mesh_project(mesh: &str, reverse: bool) -> String {
    let value = json!({"version":5,"name":"Bounded independent fixture","mesh":"MESH_TOKEN",
        "scene":{"scheme":"subtools_v1","active_id":2,"next_id":3,"solo":false,
            "objects":[{"id":1,"name":"Inactive","visible":true,"payload":{"mesh":"MESH_TOKEN"}},
                {"id":2,"name":"Active","visible":true,"payload":null}]}});
    ordered(&value, reverse).replace("\"MESH_TOKEN\"", mesh)
}

#[test]
fn aggregate_geometry_any_field_order_and_decoder_cleanup_are_bounded() {
    // Independent compact points are intentionally unused by the 20 original
    // faces, isolating allocation/count bounds from subdivision algorithms.
    let mesh = oversized_mesh(500_000);
    let mesh_json = serde_json::to_string(&mesh).unwrap();
    for reverse in [false, true] {
        let input = two_mesh_project(&mesh_json, reverse);
        let document = Document::from_json(&input).unwrap();
        assert_eq!(
            document
                .subtool_summaries()
                .iter()
                .map(|s| s.vertices)
                .sum::<usize>(),
            1_000_000
        );
    }
    let larger = oversized_mesh(550_000);
    let larger_json = serde_json::to_string(&larger).unwrap();
    for reverse in [false, true] {
        let input = two_mesh_project(&larger_json, reverse);
        let error = Document::from_json(&input).unwrap_err();
        assert!(
            error.contains("aggregate"),
            "unexpected oversized decoder reason: {error}"
        );
        // A failed Document decode must not leave a charged TLS budget behind.
        let standalone: Mesh = serde_json::from_str(&larger_json).unwrap();
        assert_eq!(standalone.positions.len(), 550_000);
    }
    let mut document = Document::new(larger);
    document.set_subtool_name(1, "Large").unwrap();
    let before = state(&document);
    assert!(document
        .add_subtool(oversized_mesh(550_000), "Too much")
        .is_err());
    assert_eq!(state(&document), before);
}

#[test]
fn scene_decoder_threads_do_not_share_allocation_ledgers() {
    let mesh_json = serde_json::to_string(&oversized_mesh(500_000)).unwrap();
    let input = Arc::new(two_mesh_project(&mesh_json, true));
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let input = input.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let document = Document::from_json(&input).unwrap();
                assert_eq!(document.subtool_count(), 2);
                assert_eq!(document.active_mesh().unwrap().positions.len(), 500_000);
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn scene_wide_layer_vector_limit_is_checked_before_committing_another_object_layer() {
    let mut document = Document::new(oversized_mesh(500_000));
    document
        .add_layer("One million base plus delta vectors")
        .unwrap();
    document
        .add_subtool(Mesh::sphere(0), "Small second")
        .unwrap();
    let before = state(&document);
    assert!(document.add_layer("Beyond aggregate").is_err());
    assert_eq!(state(&document), before);
    assert!(document.undo());
    assert_eq!(document.subtool_count(), 1);
    assert_eq!(document.layer_count(), 1);
}

#[test]
fn raw_topology_growth_exceeding_scene_budget_rolls_back_target_attributes_and_redo() {
    let (mut document, _, _) = simple_pair();
    document.store_morph_target().unwrap();
    edit(&mut document, 3);
    let redo = state(&document);
    assert!(document.undo());
    let before = state(&document);
    document.begin_stroke();
    document
        .active_mesh_mut()
        .unwrap()
        .positions
        .resize(1_000_000, Vec3::ZERO);
    document
        .active_mesh_mut()
        .unwrap()
        .normals
        .resize(1_000_000, Vec3::ZERO);
    document
        .active_mesh_mut()
        .unwrap()
        .masks
        .resize(1_000_000, 0.0);
    document
        .active_mesh_mut()
        .unwrap()
        .colors
        .resize(1_000_000, [0.0; 3]);
    // Each active field is individually legal. Only the additional inactive
    // object's 42 vertices makes the whole scene exceed the shared budget.
    document.active_mesh().unwrap().validate().unwrap();
    document.end_stroke();
    let error = document.take_layer_error().unwrap();
    assert!(
        error.contains("aggregate"),
        "unexpected raw growth rejection: {error}"
    );
    assert_eq!(state(&document), before);
    assert!(document.has_morph_target());
    assert!(document.redo());
    assert_eq!(state(&document), redo);
}

// Split only the fields of one serialized JSON object, without decoding its
// large arrays into an unbounded serde_json::Value tree. Nested commas and
// escaped strings remain part of the borrowed field slices.
fn object_fields(input: &str) -> Vec<(&str, &str)> {
    assert!(input.starts_with('{') && input.ends_with('}'));
    let body = &input[1..input.len() - 1];
    let mut fields = Vec::new();
    let mut depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    for (index, byte) in body.bytes().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'[' | b'{' => depth += 1,
            b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                fields.push(&body[start..index]);
                start = index + 1;
            }
            _ => (),
        }
    }
    if !body.is_empty() {
        fields.push(&body[start..]);
    }
    fields
        .into_iter()
        .map(|field| field.split_once(':').unwrap())
        .collect()
}

fn field<'a>(input: &'a str, name: &str) -> &'a str {
    let quoted = serde_json::to_string(name).unwrap();
    object_fields(input)
        .into_iter()
        .find(|(key, _)| *key == quoted)
        .unwrap()
        .1
}

fn reversed_object(input: &str, reverse: bool) -> String {
    let mut fields = object_fields(input);
    if reverse {
        fields.reverse();
    }
    format!(
        "{{{}}}",
        fields
            .into_iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[test]
fn mixed_active_inactive_layer_arrays_share_decode_budget_in_both_field_orders() {
    let mut large = Document::new(oversized_mesh(500_000));
    large.add_layer("Exactly one million vectors").unwrap();
    let large_project = state(&large);
    drop(large);
    let mut small = Document::new(Mesh::sphere(0));
    small.add_layer("Twenty-four additional vectors").unwrap();
    let mut value = json_state(&small);
    value["version"] = json!(5);
    value["scene"] = json!({"scheme":"subtools_v1","active_id":2,"next_id":3,"solo":false,
        "objects":[{"id":1,"name":"Large inactive","visible":true,"payload":{"mesh":"LARGE_MESH","layers":"LARGE_LAYERS"}},
                   {"id":2,"name":"Small active","visible":true,"payload":null}]});
    for reverse in [false, true] {
        let mesh = reversed_object(field(&large_project, "mesh"), reverse);
        let layers = reversed_object(field(&large_project, "layers"), reverse);
        let input = ordered(&value, reverse)
            .replace("\"LARGE_MESH\"", &mesh)
            .replace("\"LARGE_LAYERS\"", &layers);
        let error = Document::from_json(&input).unwrap_err();
        assert!(
            error.contains("aggregate"),
            "unexpected mixed layer rejection: {error}"
        );
        // Cleanup after a category failure must also permit ordinary imports.
        assert_eq!(
            state(&Document::from_json(&state(&small)).unwrap()),
            state(&small)
        );
    }
}

#[test]
fn duplicate_switch_and_roundtrip_preserve_signed_zero_bits_in_mesh_and_target() {
    let mut document = Document::new(Mesh::sphere(0));
    document.active_mesh_mut().unwrap().positions[4].x = -0.0;
    document.active_mesh_mut().unwrap().masks[4] = -0.0;
    document.active_mesh_mut().unwrap().colors[4][1] = -0.0;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.store_morph_target().unwrap();
    let original = surface_bits(document.active_mesh().unwrap());
    let a = document.active_subtool_id().unwrap();
    let b = document.duplicate_subtool(a).unwrap();
    assert_eq!(surface_bits(document.active_mesh().unwrap()), original);
    document.select_subtool(a).unwrap();
    assert_eq!(surface_bits(document.active_mesh().unwrap()), original);
    document.select_subtool(b).unwrap();
    let imported = Document::from_json(&state(&document)).unwrap();
    assert_eq!(surface_bits(imported.active_mesh().unwrap()), original);
    let value = json_state(&imported);
    assert_eq!(
        value["morph_target"]["positions"][4][0]
            .as_f64()
            .unwrap()
            .to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(
        value["scene"]["objects"][0]["payload"]["morph_target"]["positions"][4][0]
            .as_f64()
            .unwrap()
            .to_bits(),
        (-0.0_f64).to_bits()
    );
}
