//! Source-only preregistered v7 migration gates; these are not host/browser proof.
use obrush_core::{
    Document, MaskConvention, Mesh, ObjectTransform, Ray, SglByteOrder, SglScene, SglStageSettings,
    StagedSglScene, Vec3,
};
use serde_json::{json, Value};
use std::sync::Arc;
const AFFINE: &[u8] = include_bytes!("fixtures/independent-affine-two-mesh.sgl");
const V2: &[u8] = include_bytes!("fixtures/independent-v2-negative-markers.sgl");
const EMPTY: &[u8] = include_bytes!("fixtures/independent-empty-scene.sgl");
const LEGACY: &[u8] = include_bytes!("fixtures/independent-legacy1-absent-fields.sgl");
const LEGACY_JSON:&str="{\"version\":2,\"name\":\"Untitled sculpture\",\"mesh\":{\"positions\":[[0.0,0.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0]],\"normals\":[[0.0,0.0,1.0],[0.0,0.0,1.0],[0.0,0.0,1.0]],\"triangles\":[[0,1,2]],\"masks\":[0.0,0.0,0.0],\"colors\":[[1.0,1.0,1.0],[1.0,1.0,1.0],[1.0,1.0,1.0]]}}";
fn import(bytes: &[u8]) -> Document {
    let mut d = Document::empty();
    d.replace_sgl_bytes(bytes, &Default::default()).unwrap();
    d
}
fn value(d: &Document) -> Value {
    serde_json::from_str(&d.to_json().unwrap()).unwrap()
}
fn bytes_from_hex(value: &Value) -> Vec<u8> {
    let hex = value.as_str().unwrap();
    hex.as_bytes()
        .chunks_exact(2)
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect()
}
fn change(d: &Document, edit: impl FnOnce(&mut Value)) -> String {
    let mut v = value(d);
    edit(&mut v);
    serde_json::to_string(&v).unwrap()
}
fn raw_words(d: &Document) -> Vec<u32> {
    let raw = &d
        .active_object_metadata()
        .unwrap()
        .origin
        .as_ref()
        .unwrap()
        .object
        .raw;
    raw.positions
        .iter()
        .chain(raw.colors.iter().flatten())
        .chain(raw.materials.iter().flatten())
        .flat_map(|v| v.map(f32::to_bits))
        .collect()
}

#[test]
fn empty_is_absence_in_every_active_accessor() {
    let mut d = Document::empty();
    assert!(d.is_empty());
    assert!(d.active_mesh().is_none());
    assert!(d.active_mesh_mut().is_none());
    assert!(d.require_active_mesh().is_err());
    assert!(d.require_active_mesh_mut().is_err());
    assert_eq!(d.active_subtool_id(), None);
    assert_eq!(d.active_subtool_center(), None);
    assert_eq!(d.level_count(), 0);
    assert_eq!(d.subtool_count(), 0);
    assert!(!d.active_subtool_visible());
    assert!(d.subtool_summaries().is_empty());
    assert!(d.subtool_render_snapshots().is_empty());
    assert!(d.selected_subtool_ids().is_empty());
}

#[test]
fn selected_group_numeric_transform_is_world_space_atomic_and_one_undo() {
    let mut d = import(AFFINE);
    let mut centers = Vec::new();
    for id in [1, 2] {
        d.select_subtool(id).unwrap();
        centers.push(d.active_subtool_center().unwrap());
    }
    d.select_subtools(&[1, 2], Some(2)).unwrap();
    let expected = (centers[0] + centers[1]) * 0.5;
    assert!((d.selected_subtools_center().unwrap() - expected).length() < 1e-6);
    let before = d.to_json().unwrap();
    let samples = |document: &Document| {
        document
            .subtool_render_snapshots()
            .into_iter()
            .map(|item| {
                let point = item.mesh.positions[0];
                (
                    item.id,
                    item.metadata
                        .as_ref()
                        .map_or(point.as_dvec3(), |m| m.affine.world_point(point).unwrap()),
                )
            })
            .collect::<Vec<_>>()
    };
    let original_points = samples(&d);
    let transform = ObjectTransform {
        translation: Vec3::new(0.25, -0.125, 0.5),
        pivot: expected,
        ..Default::default()
    };
    assert!(d.transform_selected_subtools(&transform).unwrap());
    assert_eq!(d.selected_subtool_ids(), vec![1, 2]);
    assert_eq!(d.active_subtool_id(), Some(2));
    for ((before_id, before_point), (after_id, after_point)) in
        original_points.iter().zip(samples(&d))
    {
        assert_eq!(*before_id, after_id);
        assert!((after_point - *before_point - transform.translation.as_dvec3()).length() < 1e-6);
    }
    let after = d.to_json().unwrap();
    assert_ne!(before, after);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    assert!(d
        .transform_selected_subtools(&ObjectTransform {
            scale: f32::NAN,
            ..Default::default()
        })
        .is_err());
    assert_eq!(d.to_json().unwrap(), after);
    assert!(!d.transform_selected_subtools(&Default::default()).unwrap());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn selected_group_transform_rejects_late_invalid_member_without_partial_edit() {
    let mut d = Document::new(Mesh::sphere(0));
    let far_id = d.add_subtool(Mesh::sphere(0), "At world bound").unwrap();
    d.transform_active_subtool(&ObjectTransform {
        translation: Vec3::new(999_999.0, 0.0, 0.0),
        ..Default::default()
    })
    .unwrap();
    d.select_subtools(&[1, far_id], Some(1)).unwrap();
    let before = d.to_json().unwrap();
    assert!(d
        .transform_selected_subtools(&ObjectTransform {
            translation: Vec3::X,
            ..Default::default()
        })
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.selected_subtool_ids(), vec![1, far_id]);
    assert_eq!(d.active_subtool_id(), Some(1));
    assert!(d.undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), before);
}

#[test]
fn posed_selected_group_masked_translation_uses_source_unprotected_weights() {
    let mut d = import(AFFINE);
    d.select_subtools(&[2, 1], Some(2)).unwrap();
    let before_json = d.to_json().unwrap();
    let before = d.subtool_render_snapshots();
    let delta = Vec3::new(0.25, -0.125, 0.0);
    assert!(d.translate_selected_subtools_masked(delta).unwrap());
    let after = d.subtool_render_snapshots();
    assert_eq!(d.selected_subtool_ids(), vec![2, 1]);
    for (source, edited) in before.iter().zip(&after) {
        assert_eq!(source.id, edited.id);
        assert_eq!(source.mesh.masks, edited.mesh.masks);
        assert_eq!(source.mesh.mask_convention, edited.mesh.mask_convention);
        assert_eq!(source.mesh.triangles, edited.mesh.triangles);
        assert_eq!(
            source.metadata.as_ref().unwrap().affine.local_to_world,
            edited.metadata.as_ref().unwrap().affine.local_to_world
        );
        for (vertex, (&old, &new)) in source
            .mesh
            .positions
            .iter()
            .zip(&edited.mesh.positions)
            .enumerate()
        {
            let weight = source
                .mesh
                .mask_convention
                .influence(source.mesh.masks[vertex]);
            let pose = &source.metadata.as_ref().unwrap().affine;
            let before_world = pose.world_point(old).unwrap();
            let after_world = pose.world_point(new).unwrap();
            let error =
                (after_world - before_world - delta.as_dvec3() * f64::from(weight)).length();
            assert!(error < 3e-7, "part {} vertex {vertex}: {error}", source.id);
            if weight == 0.0 {
                assert_eq!(
                    old.to_array().map(f32::to_bits),
                    new.to_array().map(f32::to_bits)
                );
            }
        }
    }
    let after_json = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before_json);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after_json);
}

#[test]
fn posed_selected_group_masked_rotation_scale_uses_world_pivot_and_weights() {
    let mut document = import(AFFINE);
    document.select_subtools(&[2, 1], Some(2)).unwrap();
    let before_json = document.to_json().unwrap();
    let before = document.subtool_render_snapshots();
    let edit = ObjectTransform {
        translation: Vec3::new(0.02, -0.015, 0.01),
        rotation_degrees: Vec3::new(7.0, -3.0, 5.0),
        scale: 1.05,
        pivot: Vec3::new(0.25, -0.125, 0.5),
    };
    assert!(document.transform_selected_subtools_masked(&edit).unwrap());
    let after = document.subtool_render_snapshots();
    let angles = edit.rotation_degrees.as_dvec3();
    let rotation = glam::DMat3::from_rotation_z(angles.z.to_radians())
        * glam::DMat3::from_rotation_y(angles.y.to_radians())
        * glam::DMat3::from_rotation_x(angles.x.to_radians());
    let full = |world: glam::DVec3| {
        edit.pivot.as_dvec3()
            + rotation * (world - edit.pivot.as_dvec3()) * f64::from(edit.scale)
            + edit.translation.as_dvec3()
    };
    for (source, changed) in before.iter().zip(&after) {
        assert_eq!(source.id, changed.id);
        assert_eq!(source.mesh.masks, changed.mesh.masks);
        assert_eq!(source.mesh.triangles, changed.mesh.triangles);
        let original_pose = &source.metadata.as_ref().unwrap().affine;
        let updated_pose = &changed.metadata.as_ref().unwrap().affine;
        assert_eq!(original_pose.local_to_world, updated_pose.local_to_world);
        let old_center = original_pose
            .world_point(Vec3::from_array(original_pose.interaction_center))
            .unwrap();
        let new_center = updated_pose
            .world_point(Vec3::from_array(updated_pose.interaction_center))
            .unwrap();
        assert!((new_center - full(old_center)).length() < 3e-7);
        for (vertex, (&old, &new)) in source
            .mesh
            .positions
            .iter()
            .zip(&changed.mesh.positions)
            .enumerate()
        {
            let weight = source
                .mesh
                .mask_convention
                .influence_f64(source.mesh.masks[vertex]);
            let world = original_pose.world_point(old).unwrap();
            let actual = updated_pose.world_point(new).unwrap();
            let expected = world + (full(world) - world) * weight;
            assert!(
                (actual - expected).length() < 4e-7,
                "part {} vertex {vertex}: {:?} vs {:?}",
                source.id,
                actual,
                expected
            );
            if weight == 0.0 {
                assert_eq!(
                    old.to_array().map(f32::to_bits),
                    new.to_array().map(f32::to_bits)
                );
            }
        }
    }
    let after_json = document.to_json().unwrap();
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before_json);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), after_json);
}

#[test]
fn posed_selected_group_axis_scale_respects_world_axes_and_source_masks() {
    let scale = Vec3::new(1.2, 0.8, 1.05);
    let pivot = Vec3::new(0.25, -0.125, 0.5);
    for respect_masks in [false, true] {
        let mut document = import(AFFINE);
        document.select_subtools(&[2, 1], Some(2)).unwrap();
        let before_json = document.to_json().unwrap();
        let before = document.subtool_render_snapshots();
        assert!(document
            .scale_selected_subtools_axis(scale, pivot, respect_masks)
            .unwrap());
        let after = document.subtool_render_snapshots();
        for (source, changed) in before.iter().zip(&after) {
            assert_eq!(source.id, changed.id);
            assert_eq!(source.mesh.masks, changed.mesh.masks);
            assert_eq!(source.mesh.triangles, changed.mesh.triangles);
            let original_pose = &source.metadata.as_ref().unwrap().affine;
            let updated_pose = &changed.metadata.as_ref().unwrap().affine;
            if respect_masks {
                assert_eq!(original_pose.local_to_world, updated_pose.local_to_world);
            } else {
                assert_eq!(source.mesh.positions, changed.mesh.positions);
            }
            for (vertex, (&old, &new)) in source
                .mesh
                .positions
                .iter()
                .zip(&changed.mesh.positions)
                .enumerate()
            {
                let weight = if respect_masks {
                    f64::from(
                        source
                            .mesh
                            .mask_convention
                            .influence(source.mesh.masks[vertex]),
                    )
                } else {
                    1.0
                };
                let original_world = original_pose.world_point(old).unwrap();
                let full =
                    pivot.as_dvec3() + (original_world - pivot.as_dvec3()) * scale.as_dvec3();
                let expected = original_world + (full - original_world) * weight;
                let actual = updated_pose.world_point(new).unwrap();
                assert!(
                    (actual - expected).length() < 4e-7,
                    "mask {respect_masks}, part {} vertex {vertex}",
                    source.id
                );
                if respect_masks && weight == 0.0 {
                    assert_eq!(
                        old.to_array().map(f32::to_bits),
                        new.to_array().map(f32::to_bits)
                    );
                }
            }
        }
        let after_json = document.to_json().unwrap();
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before_json);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after_json);
    }
}

#[test]
fn masked_translation_refuses_unmatched_retained_detail_atomically() {
    let mut d = Document::new(Mesh::sphere(0));
    d.active_mesh_mut().unwrap().masks[0] = 0.5;
    d.add_subdivision_level().unwrap();
    let before = d.to_json().unwrap();
    assert!(d
        .translate_selected_subtools_masked(Vec3::new(0.1, 0.0, 0.0))
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn empty_serializes_v7_without_mesh_or_fake_object() {
    let d = Document::empty();
    let v = value(&d);
    assert_eq!(v["version"], 7);
    assert!(v.get("mesh").is_none());
    assert!(v["scene"]["active_id"].is_null());
    assert_eq!(v["scene"]["objects"], json!([]));
    let r = Document::from_json(&d.to_json().unwrap()).unwrap();
    assert!(r.is_empty());
    assert_eq!(value(&r), v);
}
#[test]
fn legacy_plain_json_remains_exact() {
    let d = Document::from_json(LEGACY_JSON).unwrap();
    assert_eq!(d.to_json().unwrap(), LEGACY_JSON);
    assert!(d.active_object_metadata().is_none());
    assert!(d.scene_origin().is_none());
    assert_eq!(d.active_subtool_id(), Some(1));
}
#[test]
fn empty_downgrade_rejected_for_every_old_version() {
    let d = Document::empty();
    for version in 1..=6 {
        assert!(Document::from_json(&change(&d, |v| v["version"] = json!(version))).is_err());
    }
}
#[test]
fn plain_mask_only_v7_stays_without_scene_v7_dto() {
    let mut d = Document::from_json(LEGACY_JSON).unwrap();
    d.active_mesh_mut().unwrap().mask_convention = MaskConvention::UnprotectedF32V1;
    d.active_mesh_mut().unwrap().masks.fill(1.0);
    let v = value(&d);
    assert_eq!(v["version"], 7);
    assert!(v.get("scene_v7").is_none());
    assert_eq!(
        value(&Document::from_json(&d.to_json().unwrap()).unwrap()),
        v
    );
}
#[test]
fn empty_stroke_clear_invert_preserve_redo_and_have_no_effect() {
    let mut d = Document::new(Mesh::sphere(0));
    d.clear_scene().unwrap();
    let empty = d.to_json().unwrap();
    d.begin_stroke();
    d.clear_mask();
    d.invert_mask();
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), empty);
    assert!(d.undo());
    assert!(d.redo());
    assert!(!d.clear_scene().unwrap());
    assert!(!d.can_redo());
}
#[test]
fn empty_topology_and_morph_operations_fail_safely() {
    let mut d = Document::empty();
    let before = d.to_json().unwrap();
    assert!(d.subdivide().is_err());
    assert!(d.subdivide_smooth().is_err());
    assert!(d.select_level(0).is_err());
    assert!(d.add_layer("Layer").is_err());
    assert!(d.store_morph_target().is_err());
    assert!(d.transform_active_subtool(&Default::default()).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
}
#[test]
fn empty_selection_and_invalid_solo_are_checked() {
    let mut d = Document::empty();
    d.select_subtools(&[], None).unwrap();
    assert!(d.select_subtools(&[1], Some(1)).is_err());
    assert!(d.set_solo(true).is_err());
    assert!(d.select_subtool(1).is_err());
    assert!(d.delete_subtool(1).is_err());
    assert!(d.duplicate_subtool(1).is_err());
}
#[test]
fn empty_picking_returns_no_hit() {
    let d = Document::empty();
    assert!(d
        .raycast_subtools(Ray {
            origin: Vec3::Z,
            direction: -Vec3::Z
        })
        .unwrap()
        .is_none());
}
#[test]
fn delete_last_is_undoable_and_preserves_allocator() {
    let mut d = Document::new(Mesh::sphere(0));
    let original = d.to_json().unwrap();
    d.delete_subtool(1).unwrap();
    assert!(d.is_empty());
    let empty = d.to_json().unwrap();
    let id = d.add_subtool(Mesh::sphere(0), "New").unwrap();
    assert_eq!(id, 2);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), empty);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), original);
    assert!(d.redo());
    assert!(d.is_empty());
}
#[test]
fn replace_geometry_into_empty_uses_retained_allocator() {
    let mut d = Document::new(Mesh::sphere(0));
    d.clear_scene().unwrap();
    d.replace_mesh(Mesh::sphere(0)).unwrap();
    assert_eq!(d.active_subtool_id(), Some(2));
    assert!(d.undo());
    assert!(d.is_empty());
}
#[test]
fn installing_empty_sgl_retains_exact_encoded_header() {
    let d = import(EMPTY);
    assert!(d.is_empty());
    let header = d.scene_origin().unwrap();
    assert_eq!(header.version, 3);
    assert_eq!(header.camera.as_ref().unwrap().fov_degrees, 33.0);
    let r = Document::from_json(&d.to_json().unwrap()).unwrap();
    assert_eq!(r.scene_origin().unwrap().camera.as_ref().unwrap().mode, 2);
    assert_eq!(value(&r), value(&d));
}
#[test]
fn rich_scene_then_empty_then_undo_restores_every_field() {
    let mut d = import(AFFINE);
    d.select_subtools(&[1, 2], Some(1)).unwrap();
    let rich = d.to_json().unwrap();
    d.replace_sgl_bytes(EMPTY, &Default::default()).unwrap();
    assert!(d.is_empty());
    let empty = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), rich);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), empty);
}
#[test]
fn legacy_layers_target_and_geometry_return_after_empty_import_undo() {
    let mut d = Document::new(Mesh::sphere(0));
    d.add_subdivision_level().unwrap();
    d.store_morph_target().unwrap();
    d.add_layer("Hidden").unwrap();
    let prior = d.to_json().unwrap();
    d.replace_sgl_bytes(EMPTY, &Default::default()).unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), prior);
    assert_eq!(d.level_count(), 2);
    assert!(d.has_morph_target());
}
#[test]
fn staged_custom_stable_ids_order_and_selection_are_not_reindexed() {
    let mut staged = StagedSglScene::from_sgl_bytes(AFFINE, &Default::default()).unwrap();
    staged.objects[0].id = 37;
    staged.objects[1].id = 89;
    staged.active_id = Some(89);
    staged.selected_ids = vec![89, 37];
    staged.next_id = 90;
    let mut d = Document::empty();
    d.install_staged_sgl_scene(staged).unwrap();
    assert_eq!(d.active_subtool_id(), Some(89));
    assert_eq!(d.selected_subtool_ids(), [89, 37]);
    assert_eq!(
        d.subtool_summaries()
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        [37, 89]
    );
    assert_eq!(
        value(&Document::from_json(&d.to_json().unwrap()).unwrap()),
        value(&d)
    );
}
#[test]
fn every_affine_render_origin_bit_survives_project_reload() {
    let mut d = import(AFFINE);
    d.select_subtool(1).unwrap();
    let before = d.to_json().unwrap();
    let r = Document::from_json(&before).unwrap();
    assert_eq!(r.to_json().unwrap(), before);
    let m = r.active_object_metadata().unwrap();
    assert_eq!(m.affine.local_to_world[0], -2.0);
    assert_eq!(m.affine.local_to_world[4], 1.0);
    let raw = &m.origin.as_ref().unwrap().object.raw;
    assert_eq!(raw.center, [123.0, -99.0, 0.25]);
    assert_eq!(raw.scale, 9.0);
    assert_eq!(m.affine.interaction_center, [1.0, 0.5, 0.0]);
}
#[test]
fn source_raw_tiny_z_marker_uv_and_absence_are_preserved() {
    let d = import(V2);
    let r = Document::from_json(&d.to_json().unwrap()).unwrap();
    let mesh = r.active_mesh().unwrap();
    assert_eq!(
        mesh.masks.iter().map(|m| m.to_bits()).collect::<Vec<_>>(),
        [1, 0x33000000, 0x33000001]
    );
    assert_eq!(mesh.mask_convention, MaskConvention::UnprotectedF32V1);
    let raw = &r
        .active_object_metadata()
        .unwrap()
        .origin
        .as_ref()
        .unwrap()
        .object
        .raw;
    assert_eq!(raw.faces[0][3], 0x80000001);
    assert_eq!(raw.uv.as_ref().unwrap().face_indices[0][3], 0xfffffffe);
    assert_eq!(raw_words(&r), raw_words(&d));
}
#[test]
fn origin_pool_binary_matches_independent_v2_input_exactly() {
    let d = import(V2);
    let v = value(&d);
    assert_eq!(v["scene_v7"]["origins"].as_array().unwrap().len(), 1);
    assert_eq!(bytes_from_hex(&v["scene_v7"]["origins"][0]["bytes"]), V2);
    assert_eq!(v["scene_v7"]["origins"][0]["byte_order"], "little_endian");
}
#[test]
fn legacy_big_endian_origin_and_absent_metadata_remain_exact() {
    let d = import(LEGACY);
    let v = value(&d);
    assert_eq!(
        bytes_from_hex(&v["scene_v7"]["origins"][0]["bytes"]),
        LEGACY
    );
    let r = Document::from_json(&d.to_json().unwrap()).unwrap();
    let m = r.active_object_metadata().unwrap();
    assert!(m.render.is_none());
    assert!(m.origin.as_ref().unwrap().object.raw.colors.is_none());
    assert!(r.scene_origin().unwrap().camera.is_none());
    assert_eq!(
        m.origin.as_ref().unwrap().object.byte_order,
        SglByteOrder::BigEndian
    );
}
#[test]
fn duplicate_shares_one_origin_pool_entry_and_restores_on_undo() {
    let mut d = import(V2);
    let prior = d.to_json().unwrap();
    let raw = d
        .active_object_metadata()
        .unwrap()
        .origin
        .as_ref()
        .unwrap()
        .object
        .raw
        .clone();
    let id = d.duplicate_subtool(1).unwrap();
    assert_eq!(id, 2);
    let duplicate = d
        .active_object_metadata()
        .unwrap()
        .origin
        .as_ref()
        .unwrap()
        .object
        .raw
        .clone();
    assert!(Arc::ptr_eq(&raw, &duplicate));
    let v = value(&d);
    assert_eq!(v["scene_v7"]["origins"].as_array().unwrap().len(), 1);
    let r = Document::from_json(&d.to_json().unwrap()).unwrap();
    let snapshots = r.subtool_render_snapshots();
    let a = &snapshots[0]
        .metadata
        .as_ref()
        .unwrap()
        .origin
        .as_ref()
        .unwrap()
        .object
        .raw;
    let b = &snapshots[1]
        .metadata
        .as_ref()
        .unwrap()
        .origin
        .as_ref()
        .unwrap()
        .object
        .raw;
    assert!(Arc::ptr_eq(a, b));
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), prior);
}
#[test]
fn source_and_live_scene_headers_are_not_conflated() {
    let d = import(V2);
    let changed = change(&d, |v| {
        v["scene_v7"]["header"]["camera"]["fov_degrees"] = json!(33.0)
    });
    let r = Document::from_json(&changed).unwrap();
    assert_eq!(
        r.scene_origin()
            .unwrap()
            .camera
            .as_ref()
            .unwrap()
            .fov_degrees,
        33.0
    );
    assert_eq!(
        r.active_object_metadata()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .header
            .camera
            .as_ref()
            .unwrap()
            .fov_degrees,
        179.0
    );
    assert_eq!(
        bytes_from_hex(&value(&r)["scene_v7"]["origins"][0]["bytes"]),
        V2
    );
}
#[test]
fn affine_transform_changes_pose_and_retains_local_geometry_and_raw_origin() {
    let mut d = import(AFFINE);
    d.select_subtool(1).unwrap();
    let local = d.active_mesh().unwrap().positions.clone();
    let raw = raw_words(&d);
    let before = d.to_json().unwrap();
    let old = d.active_object_metadata().unwrap().affine.local_to_world;
    d.transform_active_subtool(&ObjectTransform {
        translation: Vec3::new(1.0, 2.0, 3.0),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(d.active_mesh().unwrap().positions, local);
    assert_eq!(raw_words(&d), raw);
    let matrix = d.active_object_metadata().unwrap().affine.local_to_world;
    assert_eq!(&matrix[..12], &old[..12]);
    assert_eq!(
        &matrix[12..15],
        &[old[12] + 1.0, old[13] + 2.0, old[14] + 3.0]
    );
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn affine_picking_is_explicitly_rejected_until_adapter_is_installed() {
    let d = import(AFFINE);
    let err = d
        .raycast_subtools(Ray {
            origin: Vec3::Z,
            direction: -Vec3::Z,
        })
        .unwrap_err();
    assert!(err.contains("local/world"));
}
#[test]
fn unsupported_affine_merge_and_empty_mask_extraction_are_atomic() {
    let mut d = import(AFFINE);
    let before = d.to_json().unwrap();
    assert!(d
        .merge_subtools(&obrush_core::SubtoolMergeSettings {
            ids: vec![1, 2],
            name: "Merged".into(),
            policy: obrush_core::SubtoolMergePolicy::RequirePlain,
            allow_source_pose_loss: false,
            allow_mask_precision_loss: false,
            allow_uv_loss: false,
        })
        .is_err());
    assert!(d.extract_mask_as_subtool(&Default::default()).is_err());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn explicit_source_pose_merge_bakes_world_points_and_preserves_exact_undo() {
    let mut d = import(AFFINE);
    d.select_subtools(&[2, 1], Some(2)).unwrap();
    let before = d.to_json().unwrap();
    let snapshots = d.subtool_render_snapshots();
    let mut expected = Vec::new();
    for id in [2, 1] {
        let part = snapshots.iter().find(|part| part.id == id).unwrap();
        let affine = &part.metadata.as_ref().unwrap().affine;
        expected.extend(
            part.mesh
                .positions
                .iter()
                .map(|&point| affine.world_point(point).unwrap().as_vec3()),
        );
    }
    let mut settings = obrush_core::SubtoolMergeSettings {
        ids: vec![2, 1],
        name: "Merged source".into(),
        policy: obrush_core::SubtoolMergePolicy::FlattenDisplayed,
        allow_source_pose_loss: true,
        allow_mask_precision_loss: false,
        allow_uv_loss: false,
    };
    assert!(d.merge_subtools(&settings).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    settings.allow_mask_precision_loss = true;
    assert!(d.merge_subtools(&settings).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    settings.allow_uv_loss = true;
    let report = d.merge_subtools(&settings).unwrap();
    assert_eq!(report.consumed_ids, [2, 1]);
    assert_eq!(report.baked_pose_ids, [2, 1]);
    assert_eq!(report.discarded_source_origin_ids, [2, 1]);
    assert_eq!(report.mask_roundtrip_changed_words, 3);
    assert!(!report.discarded_uv_ids.is_empty());
    assert_eq!(report.merged_id, 2);
    assert_eq!(d.active_mesh().unwrap().positions, expected);
    assert!(d.active_object_metadata().is_none());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn mask_extraction_from_posed_source_bakes_world_points_and_preserves_origin() {
    let mut d = import(AFFINE);
    let affine = d.active_object_metadata().unwrap().affine.clone();
    let source_positions = d.active_mesh().unwrap().positions.clone();
    assert_ne!(
        affine.world_point(source_positions[0]).unwrap().as_vec3(),
        source_positions[0]
    );
    let source_masks = d.active_mesh().unwrap().masks.clone();
    let source_id = d.active_subtool_id().unwrap();
    let raw = raw_words(&d);
    d.begin_stroke();
    let mesh = d.active_mesh_mut().unwrap();
    mesh.masks[0] = mesh.mask_convention.full_value();
    d.end_stroke();
    let before = d.to_json().unwrap();
    for thickness in [0.0, -0.125, 0.125] {
        let mut trial = Document::from_json(&before).unwrap();
        let (part_id, report) = trial
            .extract_mask_as_subtool_with_surface_options(
                &obrush_core::MaskExtractSettings {
                    threshold: 0.5,
                    thickness,
                },
                obrush_core::MaskExtractSelection::WholeIncidentFaces,
                true,
                true,
            )
            .unwrap();
        assert!(report.output_vertices > 0 && report.output_triangles > 0);
        assert_eq!(trial.active_subtool_id(), Some(source_id));
        assert_eq!(trial.active_mesh().unwrap().positions, source_positions);
        assert_ne!(trial.active_mesh().unwrap().masks, source_masks);
        assert_eq!(trial.active_object_metadata().unwrap().affine, affine);
        assert_eq!(raw_words(&trial), raw);
        let after = trial.to_json().unwrap();
        trial.select_subtool(part_id).unwrap();
        let part = trial.active_mesh().unwrap();
        assert!(trial.active_object_metadata().is_none());
        let near_z = 0.5 + if thickness < 0.0 { -0.01 } else { 0.01 };
        let far_z = near_z + thickness;
        assert!(part
            .positions
            .iter()
            .all(|point| { (point.z - near_z).abs() < 2e-7 || (point.z - far_z).abs() < 2e-7 }));
        assert!(part.positions.iter().all(|point| {
            source_positions.iter().any(|source| {
                let world = affine.world_point(*source).unwrap().as_vec3();
                point.distance(world) < thickness.abs() + 0.03
            })
        }));
        trial.select_subtool(source_id).unwrap();
        assert!(trial.undo());
        assert_eq!(trial.to_json().unwrap(), before);
        assert!(trial.redo());
        assert_eq!(trial.to_json().unwrap(), after);
    }
}
#[test]
fn reflected_posed_source_extracts_with_world_winding() {
    let mut d = import(AFFINE);
    d.select_subtool(1).unwrap();
    let affine = d.active_object_metadata().unwrap().affine.clone();
    assert!(affine.determinant() < 0.0);
    d.begin_stroke();
    let mesh = d.active_mesh_mut().unwrap();
    mesh.masks[0] = mesh.mask_convention.full_value();
    d.end_stroke();
    let before = d.to_json().unwrap();
    let (part_id, report) = d
        .extract_mask_as_subtool_with_surface_options(
            &obrush_core::MaskExtractSettings {
                threshold: 0.5,
                thickness: 0.125,
            },
            obrush_core::MaskExtractSelection::WholeIncidentFaces,
            true,
            true,
        )
        .unwrap();
    assert!(report.output_triangles > 0);
    d.select_subtool(part_id).unwrap();
    let part = d.active_mesh().unwrap();
    part.validate().unwrap();
    assert!(part.triangles.iter().all(|&[a, b, c]| {
        let [a, b, c] = [a, b, c].map(|i| part.positions[i as usize]);
        (b - a).cross(c - a).length() > 1e-6
    }));
    d.select_subtool(1).unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn posed_extraction_rejects_world_f32_face_collapse_atomically() {
    let mut source = import(AFFINE);
    source.begin_stroke();
    let mesh = source.active_mesh_mut().unwrap();
    mesh.masks[0] = mesh.mask_convention.full_value();
    source.end_stroke();
    let mut data = value(&source);
    let affine = &mut data["scene_v7"]["objects"][1]["affine"]["local_to_world"];
    affine[0] = json!(0.000001);
    affine[12] = json!(999999.0);
    let mut trial = Document::from_json(&data.to_string()).unwrap();
    let before = trial.to_json().unwrap();
    let error = trial
        .extract_mask_as_subtool_with_surface_options(
            &obrush_core::MaskExtractSettings {
                threshold: 0.5,
                thickness: 0.0,
            },
            obrush_core::MaskExtractSelection::WholeIncidentFaces,
            true,
            true,
        )
        .unwrap_err();
    assert!(error.contains("collapses patch face"), "{error}");
    assert_eq!(trial.to_json().unwrap(), before);
}
#[test]
fn posed_extraction_can_activate_world_part_without_losing_source_pose() {
    let mut d = import(AFFINE);
    d.begin_stroke();
    let mesh = d.active_mesh_mut().unwrap();
    mesh.masks[0] = mesh.mask_convention.full_value();
    d.end_stroke();
    let source_id = d.active_subtool_id().unwrap();
    let source_pose = d.active_object_metadata().unwrap().affine.clone();
    let before = d.to_json().unwrap();
    let (part_id, _) = d
        .extract_mask_as_subtool_with_surface_options(
            &obrush_core::MaskExtractSettings {
                threshold: 0.5,
                thickness: 0.0,
            },
            obrush_core::MaskExtractSelection::WholeIncidentFaces,
            false,
            true,
        )
        .unwrap();
    assert_eq!(d.active_subtool_id(), Some(part_id));
    assert!(d.active_object_metadata().is_none());
    let after = d.to_json().unwrap();
    d.select_subtool(source_id).unwrap();
    assert_eq!(d.active_object_metadata().unwrap().affine, source_pose);
    d.select_subtool(part_id).unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn fixed_local_edit_keeps_original_provenance_and_complete_undo() {
    let mut d = import(V2);
    let before = d.to_json().unwrap();
    let raw = raw_words(&d);
    d.begin_stroke();
    d.active_mesh_mut().unwrap().positions[0].z = 0.01;
    d.active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    assert_eq!(raw_words(&d), raw);
    assert_ne!(d.to_json().unwrap(), before);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(raw_words(&d), raw);
}
#[test]
fn direct_active_removal_rolls_back_open_stroke() {
    let mut d = import(V2);
    let before = d.to_json().unwrap();
    d.begin_stroke();
    d.mesh = None;
    d.end_stroke();
    assert!(d.take_layer_error().unwrap().contains("Direct"));
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn public_option_cannot_hide_invalid_empty_membership_on_save() {
    let mut d = import(AFFINE);
    d.mesh = None;
    assert!(d.to_json().is_err());
    let mut d = Document::empty();
    d.mesh = Some(Mesh::sphere(0));
    assert!(d.to_json().is_err());
}
#[test]
fn malformed_second_mesh_does_not_replace_or_discard_redo() {
    let mut d = Document::new(Mesh::sphere(0));
    d.clear_scene().unwrap();
    d.undo();
    let before = d.to_json().unwrap();
    assert!(d.can_redo());
    assert!(d
        .replace_sgl_bytes(
            include_bytes!("fixtures/independent-invalid-second-mesh.sgl"),
            &Default::default()
        )
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn unsupported_tiny_affine_does_not_finish_pending_stroke() {
    let mut d = Document::new(Mesh::sphere(0));
    let original = d.to_json().unwrap();
    d.begin_stroke();
    d.active_mesh_mut().unwrap().masks[0] = 0.5;
    let pending = d.to_json().unwrap();
    assert!(d
        .replace_sgl_bytes(
            include_bytes!("fixtures/independent-tiny-invertible-affine.sgl"),
            &Default::default()
        )
        .is_err());
    assert_eq!(d.to_json().unwrap(), pending);
    d.end_stroke();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), original);
}
#[test]
fn invalid_staged_active_id_rejected_before_mutation() {
    let mut d = Document::new(Mesh::sphere(0));
    let before = d.to_json().unwrap();
    let mut staged = StagedSglScene::from_sgl_bytes(AFFINE, &Default::default()).unwrap();
    staged.active_id = Some(999);
    assert!(d.install_staged_sgl_scene(staged).is_err());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn v7_affine_origin_semantics_cannot_be_hidden_under_v6() {
    let d = import(V2);
    assert!(Document::from_json(&change(&d, |v| v["version"] = json!(6))).is_err());
}
#[test]
fn duplicate_metadata_ids_and_unknown_ids_are_rejected() {
    let d = import(V2);
    assert!(Document::from_json(&change(&d, |v| {
        let row = v["scene_v7"]["objects"][0].clone();
        v["scene_v7"]["objects"].as_array_mut().unwrap().push(row);
    }))
    .is_err());
    assert!(
        Document::from_json(&change(&d, |v| v["scene_v7"]["objects"][0]["id"] = json!(999)))
            .is_err()
    );
}
#[test]
fn unreferenced_or_out_of_range_origin_pool_data_is_rejected() {
    let d = import(V2);
    assert!(Document::from_json(
        &change(&d, |v| v["scene_v7"]["objects"][0]["origin"] = Value::Null)
    )
    .is_err());
    assert!(
        Document::from_json(&change(&d, |v| v["scene_v7"]["objects"][0]["origin"] = json!(99)))
            .is_err()
    );
}
#[test]
fn invalid_hex_truncation_and_endian_version_mismatch_rejected() {
    let d = import(V2);
    for hex in ["0", "xyz0", "02000000", "ffffffff00000000"] {
        assert!(Document::from_json(
            &change(&d, |v| v["scene_v7"]["origins"][0]["bytes"] = json!(hex))
        )
        .is_err());
    }
    assert!(
        Document::from_json(&change(&d, |v| v["scene_v7"]["origins"][0]["byte_order"] =
            json!("big_endian")))
        .is_err()
    );
}
#[test]
fn singular_projective_affine_and_invalid_render_values_rejected() {
    let d = import(V2);
    for (index, value) in [(0, 0.0), (3, 0.1), (15, 0.0)] {
        assert!(
            Document::from_json(&change(&d, |v| v["scene_v7"]["objects"][0]["affine"]
                ["local_to_world"][index] =
                json!(value)))
            .is_err()
        );
    }
    assert!(
        Document::from_json(&change(&d, |v| v["scene_v7"]["objects"][0]["render"]
            ["opacity"] = json!(1.1)))
        .is_err()
    );
    assert!(
        Document::from_json(&change(&d, |v| v["scene_v7"]["objects"][0]["render"]
            ["shader"] = json!(13)))
        .is_err()
    );
}
#[test]
fn invalid_selection_and_too_many_metadata_records_rejected() {
    let d = import(V2);
    for ids in [json!([]), json!([1, 1]), json!([999])] {
        assert!(Document::from_json(&change(&d, |v| v["scene_v7"]["selected_ids"] = ids)).is_err());
    }
    assert!(Document::from_json(&change(&d, |v| {
        let row = v["scene_v7"]["objects"][0].clone();
        v["scene_v7"]["objects"] = json!(vec![row; 33]);
    }))
    .is_err());
}
#[test]
fn header_unknown_fields_and_camera_enum_rejected() {
    let d = import(V2);
    assert!(
        Document::from_json(&change(&d, |v| v["scene_v7"]["header"]["camera"]["mode"] =
            json!(99)))
        .is_err()
    );
    assert!(Document::from_json(&change(&d, |v| v["scene_v7"]["header"]
        ["invented_environment"] = json!(3)))
    .is_err());
}
#[test]
fn affine_signed_zero_pose_changes_are_not_lost_in_history() {
    let d = import(V2);
    let a = change(&d, |v| {
        v["scene_v7"]["objects"][0]["affine"]["local_to_world"][12] = json!(-0.0)
    });
    let mut receiver = Document::from_json(&a).unwrap();
    let before = receiver.to_json().unwrap();
    receiver.replace_document(d).unwrap();
    assert!(receiver.undo());
    assert_eq!(receiver.to_json().unwrap(), before);
}
#[test]
fn raw_v0_explicit_order_remains_explicit_after_v7_reload() {
    let mut d = Document::empty();
    let settings = SglStageSettings {
        byte_order: Some(SglByteOrder::LittleEndian),
        ..Default::default()
    };
    let raw = include_bytes!("fixtures/independent-legacy0-explicit-order.sgl");
    d.replace_sgl_bytes(raw, &settings).unwrap();
    let r = Document::from_json(&d.to_json().unwrap()).unwrap();
    assert_eq!(
        r.active_object_metadata()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .object
            .version,
        0
    );
    let v = value(&r);
    let bytes = bytes_from_hex(&v["scene_v7"]["origins"][0]["bytes"]);
    assert_eq!(&bytes, raw);
    assert!(SglScene::from_bytes(&bytes).is_err());
}
#[test]
fn empty_project_cannot_retain_active_layer_or_mesh_payload() {
    let d = Document::empty();
    let mut v = value(&d);
    v["mesh"] = serde_json::from_str::<Value>(LEGACY_JSON).unwrap()["mesh"].clone();
    assert!(Document::from_json(&v.to_string()).is_err());
    let v = change(&d, |v| v["scene"]["active_id"] = json!(1));
    assert!(Document::from_json(&v).is_err());
}

#[test]
fn inactive_delete_keeps_other_selected_ids_and_header() {
    let mut d = import(AFFINE);
    let third = d.add_subtool(Mesh::sphere(0), "Third").unwrap();
    d.select_subtools(&[1, 2, third], Some(1)).unwrap();
    let header = d.scene_origin().unwrap().camera.clone();
    d.delete_subtool(2).unwrap();
    assert_eq!(d.selected_subtool_ids(), [1, third]);
    assert_eq!(d.active_subtool_id(), Some(1));
    assert_eq!(d.scene_origin().unwrap().camera, header);
}
#[test]
fn active_delete_keeps_surviving_selection_and_restores_on_undo() {
    let mut d = import(AFFINE);
    let third = d.add_subtool(Mesh::sphere(0), "Third").unwrap();
    d.select_subtools(&[1, 2, third], Some(2)).unwrap();
    let before = d.to_json().unwrap();
    d.delete_subtool(2).unwrap();
    assert_eq!(d.selected_subtool_ids(), [1, third]);
    assert_eq!(d.active_subtool_id(), Some(third));
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn two_mesh_origin_blob_is_rejected_without_partial_install() {
    let d = import(V2);
    let malformed = change(&d, |v| {
        v["scene_v7"]["origins"][0]["bytes"] = json!(AFFINE
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>())
    });
    assert!(Document::from_json(&malformed)
        .unwrap_err()
        .contains("exactly one"));
}
#[test]
fn raw_origin_count_overflow_is_rejected_before_array_allocation() {
    let d = import(V2);
    let mut hostile = V2.to_vec();
    // Independent v2 binary layout:36B scene header +20B render +80B transform.
    hostile[136..140].copy_from_slice(&u32::MAX.to_le_bytes());
    let malformed = change(&d, |v| {
        v["scene_v7"]["origins"][0]["bytes"] = json!(hostile
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>())
    });
    assert!(Document::from_json(&malformed).is_err());
}
#[test]
fn affine_center_uses_derived_local_center_not_raw_serialized_center() {
    let mut d = import(AFFINE);
    d.select_subtool(1).unwrap();
    assert_eq!(d.active_subtool_center(), Some(Vec3::new(2.5, -0.5, 1.0)));
    d.begin_stroke();
    d.active_mesh_mut().unwrap().positions[0].x -= 0.01;
    d.active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    assert_eq!(
        d.active_object_metadata()
            .unwrap()
            .affine
            .interaction_center,
        [1.0, 0.5, 0.0]
    );
}
