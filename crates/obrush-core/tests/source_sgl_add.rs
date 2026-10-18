//! Independently authored SGL words and complete append/zero/history gates.
use obrush_core::*;
fn u(bytes: &mut Vec<u8>, value: u32, order: SglByteOrder) {
    bytes.extend_from_slice(&match order {
        SglByteOrder::LittleEndian => value.to_le_bytes(),
        SglByteOrder::BigEndian => value.to_be_bytes(),
    });
}
fn f(bytes: &mut Vec<u8>, value: f32, order: SglByteOrder) {
    u(bytes, value.to_bits(), order);
}
fn fixture(version: u32, count: u32, order: SglByteOrder) -> Vec<u8> {
    let mut b = Vec::new();
    u(&mut b, version, order);
    if version >= 2 {
        for word in [1, 1, 0, 1, 2] {
            u(&mut b, word, order);
        }
        f(&mut b, 57.25, order);
        u(&mut b, 1, order);
    }
    u(&mut b, count, order);
    for ordinal in 0..count {
        if version >= 2 {
            for word in [2, 3, 0, 1] {
                u(&mut b, word, order);
            }
            f(&mut b, 0.75, order);
        }
        for v in [3.25, -0.125, 1.5] {
            f(&mut b, v, order);
        }
        for v in [
            1.0,
            0.0,
            0.0,
            0.0,
            0.25,
            1.5,
            0.0,
            0.0,
            0.0,
            0.0,
            0.75,
            0.0,
            10.0 + ordinal as f32 * 3.0,
            -2.0,
            4.0,
            1.0,
        ] {
            f(&mut b, v, order);
        }
        f(&mut b, 17.25, order);
        u(&mut b, 4, order);
        for p in [
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ] {
            for v in p {
                f(&mut b, v, order);
            }
        }
        u(&mut b, 4, order);
        for rgb in [
            [0.125, 0.25, 0.75],
            [0.25, 0.5, 0.125],
            [0.75, 0.125, 0.5],
            [0.5, 0.75, 0.25],
        ] {
            for v in rgb {
                f(&mut b, v, order);
            }
        }
        u(&mut b, 4, order);
        for raw in [1u32, 0x8000_0000, 0x3f00_0000, 0x3f80_0000] {
            f(&mut b, 0.3125, order);
            f(&mut b, 0.1875, order);
            u(&mut b, raw, order);
        }
        u(&mut b, 1, order);
        for word in [0, 1, 2, 3] {
            u(&mut b, word, order);
        }
        u(&mut b, 4, order);
        for uv in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]] {
            for v in uv {
                f(&mut b, v, order);
            }
        }
        u(&mut b, 1, order);
        for word in [0, 1, 2, 3] {
            u(&mut b, word, order);
        }
    }
    b
}
fn settings(order: SglByteOrder) -> SglStageSettings {
    SglStageSettings {
        byte_order: Some(order),
        ..Default::default()
    }
}
fn receiving() -> Document {
    let mut d = Document::new(Mesh::sphere(0));
    d.name = "Receiving studio".into();
    d
}

#[test]
fn source_sgl_add_appends_ordered_fresh_ids_and_selects_only_last() {
    let mut d = receiving();
    let old = d.to_json().unwrap();
    let old_id = d.active_subtool_id().unwrap();
    let old_mesh = d.active_mesh().unwrap().to_obj();
    let report = d
        .append_sgl_bytes(
            &fixture(3, 2, SglByteOrder::LittleEndian),
            &settings(SglByteOrder::LittleEndian),
        )
        .unwrap();
    assert!(report.document_changed);
    assert_eq!(report.added_ids, vec![2, 3]);
    assert_eq!(d.subtool_count(), 3);
    assert_eq!(d.active_subtool_id(), Some(3));
    assert_eq!(d.selected_subtool_ids(), vec![3]);
    assert_eq!(d.subtool_mesh(old_id).unwrap().to_obj(), old_mesh);
    assert_eq!(d.name, "Receiving studio");
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), old);
    assert!(!d.undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn source_sgl_add_keeps_raw_headers_material_words_uv_and_local_affine() {
    let mut d = receiving();
    let b = fixture(3, 1, SglByteOrder::LittleEndian);
    d.append_sgl_bytes(&b, &settings(SglByteOrder::LittleEndian))
        .unwrap();
    let mesh = d.active_mesh().unwrap();
    assert_eq!(mesh.positions[0], Vec3::new(-1.0, -1.0, 0.0));
    assert_eq!(mesh.surface.as_ref().unwrap().faces, vec![[0, 1, 2, 3]]);
    assert_eq!(mesh.masks[0].to_bits(), 1);
    assert_eq!(mesh.masks[1].to_bits(), 0x8000_0000);
    assert_eq!(mesh.mask_convention, MaskConvention::UnprotectedF32V1);
    let meta = d.active_object_metadata().unwrap();
    assert_eq!(meta.affine.local_to_world[12], 10.0);
    let origin = meta.origin.as_ref().unwrap();
    assert_eq!(origin.object.raw.center, [3.25, -0.125, 1.5]);
    assert_eq!(origin.object.raw.scale.to_bits(), 17.25_f32.to_bits());
    assert_eq!(
        origin.object.raw.uv.as_ref().unwrap().face_indices,
        vec![[0, 1, 2, 3]]
    );
    assert_eq!(
        origin.object.raw.materials.as_ref().unwrap()[0][2].to_bits(),
        1
    );
    let saved = d.to_json().unwrap();
    assert_eq!(
        Document::from_json(&saved).unwrap().to_json().unwrap(),
        saved
    );
}
#[test]
fn source_sgl_add_zero_modern_returns_header_but_exactly_preserves_doc_and_redo() {
    let mut d = receiving();
    d.add_subtool(Mesh::sphere(0), "Later").unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let state = (
        d.can_undo(),
        d.can_redo(),
        d.active_subtool_id(),
        d.selected_subtool_ids(),
    );
    let report = d
        .append_sgl_bytes(
            &fixture(3, 0, SglByteOrder::LittleEndian),
            &settings(SglByteOrder::LittleEndian),
        )
        .unwrap();
    assert!(!report.document_changed);
    assert!(report.added_ids.is_empty());
    assert!(
        report
            .input_header
            .display
            .as_ref()
            .unwrap()
            .show_symmetry_line
    );
    assert_eq!(
        report
            .input_header
            .camera
            .as_ref()
            .unwrap()
            .fov_degrees
            .to_bits(),
        57.25_f32.to_bits()
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(
        (
            d.can_undo(),
            d.can_redo(),
            d.active_subtool_id(),
            d.selected_subtool_ids()
        ),
        state
    );
    assert!(d.redo());
    assert_eq!(d.subtool_count(), 2);
}
#[test]
fn source_sgl_add_zero_preserves_pending_noop_stroke_and_empty_allocator() {
    let mut d = Document::empty();
    let before = d.to_json().unwrap();
    d.begin_stroke();
    let r = d
        .append_sgl_bytes(
            &fixture(2, 0, SglByteOrder::LittleEndian),
            &settings(SglByteOrder::LittleEndian),
        )
        .unwrap();
    assert!(!r.document_changed);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    d.end_stroke();
    let r = d
        .append_sgl_bytes(
            &fixture(3, 1, SglByteOrder::LittleEndian),
            &settings(SglByteOrder::LittleEndian),
        )
        .unwrap();
    assert_eq!(r.added_ids, vec![1]);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn source_sgl_add_new_parts_preserve_old_levels_layers_morph_and_visibility() {
    let mut d = Document::new(Mesh::sphere(0));
    d.add_subdivision_level().unwrap();
    d.store_morph_target().unwrap();
    d.add_layer("Stored clay").unwrap();
    let first = d.active_subtool_id().unwrap();
    let second = d.add_subtool(Mesh::sphere(0), "Other").unwrap();
    d.set_subtool_visible(second, false).unwrap();
    d.select_subtool(first).unwrap();
    let before = d.to_json().unwrap();
    d.append_sgl_bytes(
        &fixture(3, 2, SglByteOrder::LittleEndian),
        &settings(SglByteOrder::LittleEndian),
    )
    .unwrap();
    assert_eq!(d.subtool_count(), 4);
    assert!(
        !d.subtool_summaries()
            .iter()
            .find(|s| s.id == second)
            .unwrap()
            .visible
    );
    d.select_subtool(first).unwrap();
    assert_eq!(d.level_count(), 2);
    assert_eq!(d.layer_count(), 1);
    assert!(d.has_morph_target());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn source_sgl_add_active_stroke_rejection_is_atomic() {
    let mut d = receiving();
    d.begin_stroke();
    let before = d.to_json().unwrap();
    let history = (d.can_undo(), d.can_redo());
    assert!(d
        .append_sgl_bytes(
            &fixture(3, 1, SglByteOrder::LittleEndian),
            &settings(SglByteOrder::LittleEndian)
        )
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!((d.can_undo(), d.can_redo()), history);
    d.end_stroke();
    assert!(!d.can_undo());
}
#[test]
fn source_sgl_add_thirty_two_part_limit_and_second_object_corruption_keep_redo() {
    let mut d = receiving();
    for _ in 1..MAX_SUBTOOLS {
        d.add_subtool(Mesh::sphere(0), "Kept").unwrap();
    }
    let before = d.to_json().unwrap();
    assert!(d
        .append_sgl_bytes(
            &fixture(3, 1, SglByteOrder::LittleEndian),
            &settings(SglByteOrder::LittleEndian)
        )
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let mut bad = fixture(3, 2, SglByteOrder::LittleEndian);
    bad.truncate(bad.len() - 4);
    assert!(d
        .append_sgl_bytes(&bad, &settings(SglByteOrder::LittleEndian))
        .is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn source_sgl_add_preserves_receiving_legacy_header_when_new_file_has_no_globals() {
    let mut d = Document::empty();
    d.replace_sgl_bytes(
        &fixture(3, 1, SglByteOrder::LittleEndian),
        &settings(SglByteOrder::LittleEndian),
    )
    .unwrap();
    let header = d.scene_origin().unwrap().clone();
    d.append_sgl_bytes(
        &fixture(1, 1, SglByteOrder::LittleEndian),
        &settings(SglByteOrder::LittleEndian),
    )
    .unwrap();
    assert_eq!(d.scene_origin().unwrap().display, header.display);
    assert_eq!(d.scene_origin().unwrap().camera, header.camera);
    let origin = d.active_object_metadata().unwrap().origin.as_ref().unwrap();
    assert_eq!(origin.header.version, 1);
    assert!(origin.header.display.is_none());
    assert!(origin.header.camera.is_none());
}
#[test]
fn source_sgl_add_version_zero_explicit_both_orders_and_auto_rejection_stay_separate() {
    for order in [SglByteOrder::LittleEndian, SglByteOrder::BigEndian] {
        let b = fixture(0, 1, order);
        assert!(SglScene::from_bytes(&b).is_err());
        let mut d = receiving();
        let r = d.append_sgl_bytes(&b, &settings(order)).unwrap();
        assert_eq!(r.input_header.version, 0);
        assert_eq!(r.input_header.byte_order, order);
        assert!(r.input_header.camera.is_none());
        assert_eq!(
            d.active_object_metadata()
                .unwrap()
                .origin
                .as_ref()
                .unwrap()
                .object
                .version,
            0
        );
    }
}
#[test]
fn source_sgl_add_mixed_raw_origins_survive_save_undo_redo() {
    let mut d = Document::empty();
    d.replace_sgl_bytes(
        &fixture(2, 1, SglByteOrder::BigEndian),
        &settings(SglByteOrder::BigEndian),
    )
    .unwrap();
    let old_id = d.active_subtool_id().unwrap();
    let old_origin = d
        .active_object_metadata()
        .unwrap()
        .origin
        .as_ref()
        .unwrap()
        .clone();
    let before = d.to_json().unwrap();
    d.append_sgl_bytes(
        &fixture(0, 1, SglByteOrder::LittleEndian),
        &settings(SglByteOrder::LittleEndian),
    )
    .unwrap();
    let after = d.to_json().unwrap();
    assert_eq!(
        d.subtool_object_metadata(old_id)
            .unwrap()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .object
            .raw
            .as_ref(),
        old_origin.object.raw.as_ref()
    );
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn source_sgl_add_empty_after_clear_uses_retained_next_id() {
    let mut d = receiving();
    let old = d.add_subtool(Mesh::sphere(0), "Second").unwrap();
    d.clear_scene().unwrap();
    let before = d.to_json().unwrap();
    let r = d
        .append_sgl_bytes(
            &fixture(3, 1, SglByteOrder::LittleEndian),
            &settings(SglByteOrder::LittleEndian),
        )
        .unwrap();
    assert_eq!(r.added_ids, vec![old + 1]);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
