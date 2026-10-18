//! Independent object-isolation and nearest-visible-picking core fixtures.
use obrush_core::{Document, Mesh, Ray, Vec3};
use std::sync::Arc;

fn translated(offset: Vec3) -> Mesh {
    let mut mesh = Mesh::sphere(1);
    for point in &mut mesh.positions {
        *point += offset;
    }
    mesh.recompute_normals();
    mesh
}

#[test]
fn inactive_render_arcs_remain_stable_and_duplicate_edits_are_independent() {
    let mut document = Document::new(Mesh::sphere(1));
    let first = document.active_subtool_id().unwrap();
    let second = document
        .add_subtool(translated(Vec3::X * 2.5), "Second")
        .unwrap();
    let before = document.subtool_render_snapshots();
    let first_arc = before.iter().find(|o| o.id == first).unwrap().mesh.clone();
    let first_bits = serde_json::to_string(first_arc.as_ref()).unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].y += 0.01;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.take_layer_error().is_none());
    let after = document.subtool_render_snapshots();
    assert!(Arc::ptr_eq(
        &first_arc,
        &after.iter().find(|o| o.id == first).unwrap().mesh
    ));
    assert_eq!(
        serde_json::to_string(first_arc.as_ref()).unwrap(),
        first_bits
    );
    let second_positions = document.active_mesh().unwrap().positions.clone();
    let duplicate = document.duplicate_subtool(second).unwrap();
    assert_eq!(document.active_subtool_id().unwrap(), duplicate);
    assert_eq!(document.active_mesh().unwrap().positions, second_positions);
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].y += 0.02;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    document.select_subtool(second).unwrap();
    assert_eq!(document.active_mesh().unwrap().positions, second_positions);
    document.select_subtool(first).unwrap();
    assert_eq!(
        serde_json::to_string(document.active_mesh().unwrap()).unwrap(),
        first_bits
    );
}

#[test]
fn nearest_pick_honors_occluders_visibility_solo_and_active_duplicate_ties() {
    let mut document = Document::new(Mesh::sphere(2));
    let back = document.active_subtool_id().unwrap();
    let mut front_mesh = Mesh::sphere(2);
    for point in &mut front_mesh.positions {
        point.z += 0.6;
    }
    let front = document.add_subtool(front_mesh, "Front").unwrap();
    let ray = Ray {
        origin: Vec3::Z * 3.0,
        direction: -Vec3::Z,
    };
    document.select_subtool(back).unwrap();
    assert_eq!(document.raycast_subtools(ray).unwrap().unwrap().id, front);
    document.set_subtool_visible(front, false).unwrap();
    assert_eq!(document.raycast_subtools(ray).unwrap().unwrap().id, back);
    document.set_subtool_visible(front, true).unwrap();
    document.set_solo(true).unwrap();
    assert_eq!(document.raycast_subtools(ray).unwrap().unwrap().id, back);
    document.set_solo(false).unwrap();
    let duplicate = document.duplicate_subtool(front).unwrap();
    assert_eq!(
        document.raycast_subtools(ray).unwrap().unwrap().id,
        duplicate
    );
    document.set_solo(true).unwrap();
    document.set_subtool_visible(duplicate, false).unwrap();
    assert!(!document.active_subtool_visible());
    assert!(document.raycast_subtools(ray).unwrap().is_none());
    assert!(document.subtool_render_snapshots().is_empty());
}

#[test]
fn legacy_one_object_encoding_changes_only_after_meaningful_scene_usage() {
    let mut document = Document::new(Mesh::sphere(0));
    let original = document.to_json().unwrap();
    document.set_solo(false).unwrap();
    document.set_subtool_visible(1, true).unwrap();
    document.reorder_subtool(1, 0).unwrap();
    document.select_subtool(1).unwrap();
    assert_eq!(document.to_json().unwrap(), original);
    document.set_subtool_name(1, "One").unwrap();
    let scene = document.to_json().unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&scene).unwrap()["version"],
        5
    );
    assert_eq!(
        Document::from_json(&scene).unwrap().to_json().unwrap(),
        scene
    );
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), original);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), scene);
}

#[test]
fn first_scene_action_cannot_capture_a_rejected_pending_stroke() {
    let mut document = Document::new(Mesh::sphere(0));
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0] *= 1.01;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().normals[0].x = f32::NAN;
    assert!(document.add_subtool(Mesh::sphere(0), "Rejected").is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert_eq!(document.subtool_count(), 1);
    assert!(document.can_redo());
}

#[test]
fn failed_scene_decode_does_not_poison_later_or_parallel_valid_imports() {
    let mut document = Document::new(Mesh::sphere(0));
    document
        .add_subtool(translated(Vec3::X * 2.0), "Other")
        .unwrap();
    let valid = document.to_json().unwrap();
    let mut corrupt: serde_json::Value = serde_json::from_str(&valid).unwrap();
    corrupt["scene"]["objects"][0]["id"] = corrupt["scene"]["objects"][1]["id"].clone();
    let corrupt = serde_json::to_string(&corrupt).unwrap();
    for _ in 0..8 {
        assert!(Document::from_json(&corrupt).is_err());
        assert_eq!(
            Document::from_json(&valid).unwrap().to_json().unwrap(),
            valid
        );
    }
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let valid = valid.clone();
            std::thread::spawn(move || {
                assert_eq!(
                    Document::from_json(&valid).unwrap().to_json().unwrap(),
                    valid
                )
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn whole_document_open_rejects_a_bad_pending_scene_stroke_before_replacement() {
    let mut document = Document::new(Mesh::sphere(0));
    document
        .add_subtool(translated(Vec3::X * 2.0), "Other")
        .unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().positions[0].y += 0.01;
    document.active_mesh_mut().unwrap().recompute_normals();
    document.end_stroke();
    assert!(document.undo());
    let before = document.to_json().unwrap();
    document.begin_stroke();
    document.active_mesh_mut().unwrap().normals[0].x = f32::NAN;
    assert!(document
        .replace_document(Document::new(Mesh::sphere(1)))
        .is_err());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.can_redo());
}
