//! Independent connectivity/channel/history gates, not consumer parity proof.
use obrush_core::{
    CornerUv, Document, Mesh, MorphTargetDeletionDisposition as Disposition,
    ReverseSubdivisionSettings, StaticSubdivisionSettings, SurfaceTopology, Vec3, MAX_REVERSE_WORK,
    TRIANGLE_CORNER,
};
use serde_json::{json, Value};
const SMOOTH: StaticSubdivisionSettings = StaticSubdivisionSettings {
    linear_edges: false,
};
const LINEAR: StaticSubdivisionSettings = StaticSubdivisionSettings { linear_edges: true };
fn state(d: &Document) -> String {
    d.to_json().unwrap()
}
fn mesh_bits(m: &Mesh) -> String {
    serde_json::to_string(m).unwrap()
}
fn fixture(name: &str) -> (Mesh, Value) {
    let text = match name {
        "triangle" => include_str!("fixtures/independent-reverse-triangle.json"),
        "quad" => include_str!("fixtures/independent-reverse-quad.json"),
        "uv-seam-patch" => include_str!("fixtures/independent-reverse-uv-seam-patch.json"),
        _ => include_str!("fixtures/independent-reverse-mixed-pyramid.json"),
    };
    let v: Value = serde_json::from_str(text).unwrap();
    let points: Vec<[f32; 3]> = serde_json::from_value(v["positions"].clone()).unwrap();
    let faces: Vec<[u32; 4]> = serde_json::from_value(v["faces"].clone()).unwrap();
    let mut obj = String::new();
    for [x, y, z] in points {
        obj.push_str(&format!("v {x} {y} {z}\n"));
    }
    for &[a, b, c, d] in &faces {
        obj.push_str(&format!("f {} {} {}\n", a + 1, b + 1, c + 1));
        if d != TRIANGLE_CORNER {
            obj.push_str(&format!("f {} {} {}\n", a + 1, c + 1, d + 1));
        }
    }
    let mut m = Mesh::from_obj(&obj).unwrap();
    m.install_attributes(
        None,
        Some(SurfaceTopology {
            faces,
            uv: if v["uv"].is_object() {
                Some(serde_json::from_value(v["uv"].clone()).unwrap())
            } else {
                None
            },
        }),
    )
    .unwrap();
    (m, v)
}
fn hierarchy(mode: usize) -> Document {
    let mut d = Document::new(Mesh::sphere(0));
    for _ in 0..3 {
        match mode {
            0 => d.add_subdivision_level().unwrap(),
            1 => d.add_static_subdivision_level(&SMOOTH).unwrap(),
            _ => d.add_static_subdivision_level(&LINEAR).unwrap(),
        }
    }
    d
}
fn ensure_reloads(d: &Document) {
    let s = state(d);
    assert_eq!(state(&Document::from_json(&s).unwrap()), s);
}
#[test]
fn lower_deletion_preserves_visible_mesh_and_each_scheme_exactly() {
    for mode in 0..3 {
        let mut d = hierarchy(mode);
        d.select_level(2).unwrap();
        let before = state(&d);
        let bits = mesh_bits(d.require_active_mesh().unwrap());
        let r = d.delete_lower_subdivision_levels().unwrap();
        assert_eq!(
            (
                r.previous_levels,
                r.retained_levels,
                r.previous_active_level,
                r.active_level
            ),
            (4, 2, 2, 0)
        );
        assert!(r.removed_vertices > 0 && r.removed_triangles > 0);
        assert_eq!(mesh_bits(d.require_active_mesh().unwrap()), bits);
        let after = state(&d);
        ensure_reloads(&d);
        assert!(d.undo());
        assert_eq!(state(&d), before);
        assert!(d.redo());
        assert_eq!(state(&d), after);
    }
}
#[test]
fn higher_deletion_preserves_active_mesh_and_each_scheme_exactly() {
    for mode in 0..3 {
        let mut d = hierarchy(mode);
        d.select_level(1).unwrap();
        let before = state(&d);
        let bits = mesh_bits(d.require_active_mesh().unwrap());
        let r = d.delete_higher_subdivision_levels().unwrap();
        assert_eq!(
            (
                r.previous_levels,
                r.retained_levels,
                r.previous_active_level,
                r.active_level
            ),
            (4, 2, 1, 1)
        );
        assert_eq!(mesh_bits(d.require_active_mesh().unwrap()), bits);
        let after = state(&d);
        ensure_reloads(&d);
        assert!(d.undo());
        assert_eq!(state(&d), before);
        assert!(d.redo());
        assert_eq!(state(&d), after);
    }
}
#[test]
fn one_level_retains_scheme_and_can_subdivide_again() {
    for mode in 0..3 {
        let mut d = hierarchy(mode);
        let r = d.delete_lower_subdivision_levels().unwrap();
        assert_eq!(r.retained_levels, 1);
        assert_eq!(d.current_level(), 0);
        let v: Value = serde_json::from_str(&state(&d)).unwrap();
        assert_eq!(v["multiresolution"]["levels"], json!([null]));
        ensure_reloads(&d);
        match mode {
            0 => d.add_subdivision_level().unwrap(),
            1 => d.add_static_subdivision_level(&SMOOTH).unwrap(),
            _ => d.add_static_subdivision_level(&LINEAR).unwrap(),
        };
        assert_eq!(d.level_count(), 2);
    }
}
#[test]
fn static_one_level_cannot_silently_become_legacy_or_other_mode() {
    let mut d = hierarchy(1);
    d.delete_lower_subdivision_levels().unwrap();
    let s = state(&d);
    assert!(d.add_subdivision_level().is_err());
    assert!(d.add_static_subdivision_level(&LINEAR).is_err());
    assert_eq!(state(&d), s);
    assert_eq!(serde_json::from_str::<Value>(&s).unwrap()["version"], 6);
}
#[test]
fn lower_rebases_a_retained_target_and_undo_restores_its_binding() {
    let mut d = hierarchy(1);
    d.select_level(2).unwrap();
    d.store_morph_target().unwrap();
    let before = state(&d);
    let r = d.delete_lower_subdivision_levels().unwrap();
    assert_eq!(
        r.morph_target,
        Disposition::Rebased {
            previous_level: 2,
            creation_level: 0
        }
    );
    assert!(d.morph_target_summary().unwrap().available);
    ensure_reloads(&d);
    let after = state(&d);
    assert!(d.undo());
    assert_eq!(state(&d), before);
    assert!(d.redo());
    assert_eq!(state(&d), after);
}
#[test]
fn lower_explicitly_reports_a_target_bound_to_a_removed_level() {
    let mut d = hierarchy(0);
    d.select_level(0).unwrap();
    d.store_morph_target().unwrap();
    d.select_level(2).unwrap();
    let before = state(&d);
    let r = d.delete_lower_subdivision_levels().unwrap();
    assert_eq!(r.morph_target, Disposition::Dropped { previous_level: 0 });
    assert!(!d.has_morph_target());
    ensure_reloads(&d);
    assert!(d.undo());
    assert_eq!(state(&d), before);
}
#[test]
fn higher_explicitly_reports_a_target_bound_to_a_removed_level() {
    let mut d = hierarchy(2);
    d.store_morph_target().unwrap();
    d.select_level(1).unwrap();
    let before = state(&d);
    let r = d.delete_higher_subdivision_levels().unwrap();
    assert_eq!(r.morph_target, Disposition::Dropped { previous_level: 3 });
    assert!(!d.has_morph_target());
    assert!(d.undo());
    assert_eq!(state(&d), before);
}
#[test]
fn higher_preserves_a_retained_target_at_a_different_level() {
    let mut d = hierarchy(1);
    d.select_level(0).unwrap();
    d.store_morph_target().unwrap();
    d.select_level(2).unwrap();
    let r = d.delete_higher_subdivision_levels().unwrap();
    assert_eq!(r.morph_target, Disposition::Preserved { creation_level: 0 });
    assert!(!d.morph_target_summary().unwrap().available);
    ensure_reloads(&d);
}
#[test]
fn noops_preserve_redo_and_open_stroke_boundaries() {
    let mut d = hierarchy(1);
    d.select_level(0).unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[0] *= 1.001;
    d.require_active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    assert!(d.undo());
    let before = state(&d);
    d.begin_stroke();
    let r = d.delete_lower_subdivision_levels().unwrap();
    assert_eq!(r.removed_vertices, 0);
    assert_eq!(state(&d), before);
    assert!(d.can_redo());
    d.end_stroke();
    assert!(d.can_redo());
    assert!(d.redo());
    d.select_level(d.level_count() - 1).unwrap();
    let before = state(&d);
    assert_eq!(
        d.delete_higher_subdivision_levels()
            .unwrap()
            .removed_triangles,
        0
    );
    assert_eq!(state(&d), before);
}
#[test]
fn no_hierarchy_deletions_preserve_legacy_bytes_and_redo() {
    let mut d = Document::new(Mesh::sphere(0));
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[0] *= 1.001;
    d.require_active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    assert!(d.undo());
    let s = state(&d);
    assert_eq!(
        d.delete_lower_subdivision_levels().unwrap().retained_levels,
        1
    );
    assert_eq!(
        d.delete_higher_subdivision_levels()
            .unwrap()
            .removed_vertices,
        0
    );
    assert_eq!(state(&d), s);
    assert!(d.can_redo());
}
#[test]
fn deletion_preserves_bound_recording_layers_and_signed_zero_channels() {
    let mut d = hierarchy(1);
    d.select_level(2).unwrap();
    d.add_layer("Retained layer").unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[0].z += 0.001;
    d.require_active_mesh_mut().unwrap().masks[0] = -0.0;
    d.require_active_mesh_mut().unwrap().colors[0][0] = -0.0;
    d.require_active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    let before = state(&d);
    let bits = mesh_bits(d.require_active_mesh().unwrap());
    let layers = d.layer_summaries();
    d.delete_lower_subdivision_levels().unwrap();
    assert_eq!(mesh_bits(d.require_active_mesh().unwrap()), bits);
    assert_eq!(
        serde_json::to_string(&d.layer_summaries()).unwrap(),
        serde_json::to_string(&layers).unwrap()
    );
    ensure_reloads(&d);
    assert!(d.undo());
    assert_eq!(state(&d), before);
}
#[test]
fn deletion_keeps_inactive_scene_payloads_and_selection_exact() {
    let mut d = hierarchy(1);
    let original = d.active_subtool_id();
    let second = d.add_subtool(Mesh::sphere(0), "Other").unwrap();
    d.set_subtool_visible(second, false).unwrap();
    d.select_subtool(original.expect("nonempty fixture id"))
        .unwrap();
    d.select_level(1).unwrap();
    d.set_solo(true).unwrap();
    let before = state(&d);
    let inactive: Value = serde_json::from_str(&before).unwrap();
    let bits = mesh_bits(d.require_active_mesh().unwrap());
    d.delete_higher_subdivision_levels().unwrap();
    assert_eq!(d.active_subtool_id(), original);
    assert_eq!(mesh_bits(d.require_active_mesh().unwrap()), bits);
    assert!(d.solo());
    let after: Value = serde_json::from_str(&state(&d)).unwrap();
    assert_eq!(after["scene"], inactive["scene"]);
    ensure_reloads(&d);
    assert!(d.undo());
    assert_eq!(state(&d), before);
}
#[test]
fn invalid_encoding_budget_rejects_deletion_without_clearing_redo() {
    let mut d = hierarchy(1);
    d.select_level(1).unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[0] *= 1.001;
    d.require_active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    assert!(d.undo());
    d.name = "x".repeat(4097);
    let mesh = mesh_bits(d.require_active_mesh().unwrap());
    assert!(d.delete_lower_subdivision_levels().is_err());
    assert!(d.can_redo());
    assert_eq!(d.level_count(), 4);
    assert_eq!(d.current_level(), 1);
    assert_eq!(mesh_bits(d.require_active_mesh().unwrap()), mesh);
}
#[test]
fn empty_or_wrong_active_one_level_project_is_rejected_atomically() {
    let mut d = hierarchy(1);
    d.delete_lower_subdivision_levels().unwrap();
    let valid = state(&d);
    for corruption in 0..3 {
        let mut v: Value = serde_json::from_str(&valid).unwrap();
        match corruption {
            0 => v["multiresolution"]["levels"] = json!([]),
            1 => v["multiresolution"]["active_level"] = json!(1),
            _ => v["multiresolution"]["levels"][0] = v["mesh"].clone(),
        };
        assert!(Document::from_json(&serde_json::to_string(&v).unwrap()).is_err());
        assert_eq!(state(&d), valid);
    }
}
#[test]
fn lower_deletion_reconciles_coarse_edit_before_retaining_finer_level() {
    let mut d = hierarchy(1);
    d.select_level(1).unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[0].z += 0.002;
    d.require_active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    let mut oracle = Document::from_json(&state(&d)).unwrap();
    oracle.select_level(3).unwrap();
    let expected = mesh_bits(oracle.require_active_mesh().unwrap());
    d.delete_lower_subdivision_levels().unwrap();
    d.select_level(2).unwrap();
    assert_eq!(mesh_bits(d.require_active_mesh().unwrap()), expected);
}
#[test]
fn explicit_lower_higher_sequence_retains_uv_and_material_bits() {
    let (fine, _) = fixture("quad");
    let mut coarse = fine
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap()
        .coarse;
    coarse.materials = Some(vec![[0.25, -0.0]; coarse.positions.len()]);
    let f = &coarse.surface.as_ref().unwrap().faces;
    let uv = CornerUv {
        coordinates: vec![[-0.0, 0.0], [1., 0.], [1., 1.], [0., 1.]],
        face_indices: vec![[0, 1, 2, 3]; f.len()],
    };
    coarse.surface.as_mut().unwrap().uv = Some(uv);
    let mut d = Document::new(coarse);
    d.add_static_subdivision_level(&SMOOTH).unwrap();
    d.add_static_subdivision_level(&SMOOTH).unwrap();
    d.select_level(1).unwrap();
    let before = state(&d);
    let bits = mesh_bits(d.require_active_mesh().unwrap());
    d.delete_lower_subdivision_levels().unwrap();
    d.delete_higher_subdivision_levels().unwrap();
    assert_eq!(d.level_count(), 1);
    assert_eq!(mesh_bits(d.require_active_mesh().unwrap()), bits);
    ensure_reloads(&d);
    assert!(d.undo());
    assert!(d.undo());
    assert_eq!(state(&d), before);
}
#[test]
fn independent_triangle_connectivity_recovers_reordered_even_vertices() {
    check_fixture("triangle");
}
#[test]
fn independent_quad_connectivity_recovers_without_render_diagonal_edges() {
    check_fixture("quad");
}
#[test]
fn independent_mixed_connectivity_recovers_all_oriented_child_patches() {
    check_fixture("mixed-pyramid");
}
#[test]
fn reverse_command_retains_original_fine_channels_navigation_and_history() {
    for name in ["triangle", "quad", "mixed-pyramid"] {
        let (fine, _) = fixture(name);
        let correspondence = fine
            .reversed_static(&ReverseSubdivisionSettings::default())
            .unwrap()
            .correspondence;
        let mut document = Document::new(fine.snapshot());
        let before = state(&document);
        let report = document
            .reverse_source_static_subdivision_level(&ReverseSubdivisionSettings::default())
            .unwrap();
        assert_eq!(report.fine_vertices, fine.positions.len(), "{name}");
        assert_eq!((document.level_count(), document.current_level()), (2, 0));
        let coarse = state(&document);
        ensure_reloads(&document);
        document.select_level(1).unwrap();
        let raised = document.require_active_mesh().unwrap();
        for (predicted, &original) in correspondence.predicted_vertex_to_fine.iter().enumerate() {
            let original = original as usize;
            assert_eq!(
                raised.positions[predicted].to_array().map(f32::to_bits),
                fine.positions[original].to_array().map(f32::to_bits),
                "{name} position {predicted}"
            );
            assert_eq!(
                raised.colors[predicted].map(f32::to_bits),
                fine.colors[original].map(f32::to_bits),
                "{name} color {predicted}"
            );
            assert_eq!(
                raised.masks[predicted].to_bits(),
                fine.masks[original].to_bits(),
                "{name} mask {predicted}"
            );
        }
        if let Some(original_uv) = fine.surface.as_ref().and_then(|s| s.uv.as_ref()) {
            let raised_uv = raised.surface.as_ref().unwrap().uv.as_ref().unwrap();
            assert_eq!(
                raised_uv.coordinates, original_uv.coordinates,
                "{name} UV table"
            );
            for (predicted, &original_face) in
                correspondence.predicted_face_to_fine.iter().enumerate()
            {
                let rotation = usize::from(correspondence.predicted_face_rotation[predicted]);
                let source = original_uv.face_indices[original_face as usize];
                let actual = raised_uv.face_indices[predicted];
                let count = if actual[3] == TRIANGLE_CORNER { 3 } else { 4 };
                for corner in 0..count {
                    assert_eq!(
                        actual[corner],
                        source[(rotation + corner) % count],
                        "{name} UV face {predicted}"
                    );
                }
            }
        }
        ensure_reloads(&document);
        document.select_level(0).unwrap();
        assert_eq!(state(&document), coarse, "{name} lower return");
        assert!(document.undo(), "{name} undo");
        assert_eq!(state(&document), before, "{name} original undo");
        assert!(document.redo(), "{name} redo");
        assert_eq!(state(&document), coarse, "{name} coarse redo");
    }
}

#[test]
fn reverse_command_rejects_inconsistent_uv_midpoint_without_consuming_redo() {
    let (mut fine, _) = fixture("uv-seam-patch");
    fine.surface
        .as_mut()
        .unwrap()
        .uv
        .as_mut()
        .unwrap()
        .coordinates[4][0] += 0.02;
    let mut document = Document::new(fine);
    document.begin_stroke();
    document.require_active_mesh_mut().unwrap().positions[0].z += 0.01;
    document
        .require_active_mesh_mut()
        .unwrap()
        .recompute_normals();
    document.end_stroke();
    assert!(document.undo());
    let before = state(&document);
    assert!(document.can_redo());
    let error = document
        .reverse_source_static_subdivision_level(&ReverseSubdivisionSettings::default())
        .unwrap_err();
    assert!(error.contains("UV midpoint/center"), "{error}");
    assert_eq!(state(&document), before);
    assert!(document.can_redo());
}
#[test]
fn reverse_command_keeps_independent_uv_alias_ids_across_save_and_history() {
    let (fine, _) = fixture("uv-seam-patch");
    let original_uv = fine.surface.as_ref().unwrap().uv.as_ref().unwrap().clone();
    let mapping = fine
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap()
        .correspondence;
    let mut document = Document::new(fine);
    let before = state(&document);
    document
        .reverse_source_static_subdivision_level(&ReverseSubdivisionSettings::default())
        .unwrap();
    let coarse = state(&document);
    assert!(coarse.contains("sculptgl_static_smooth_v4"));
    ensure_reloads(&document);
    let mut corrupted: Value = serde_json::from_str(&coarse).unwrap();
    corrupted["multiresolution"]["levels"][1]["surface"]["uv"]["coordinates"][0][0] = json!(0.125);
    assert!(Document::from_json(&corrupted.to_string()).is_err());
    document.select_level(1).unwrap();
    let raised = document.require_active_mesh().unwrap();
    let actual_uv = raised.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(actual_uv.coordinates, original_uv.coordinates);
    for (new_face, &old_face) in mapping.predicted_face_to_fine.iter().enumerate() {
        let rotation = usize::from(mapping.predicted_face_rotation[new_face]);
        for corner in 0..4 {
            assert_eq!(
                actual_uv.face_indices[new_face][corner],
                original_uv.face_indices[old_face as usize][(rotation + corner) % 4]
            );
        }
    }
    document.select_level(0).unwrap();
    assert!(document.undo());
    assert_eq!(state(&document), before);
    assert!(document.redo());
    assert_eq!(state(&document), coarse);
}
#[test]
fn independent_two_step_grid_has_two_recoverable_structural_levels() {
    let mut obj = String::from("o Independent_reverse_two_step_grid\n");
    for y in 0..5 {
        for x in 0..5 {
            obj.push_str(&format!(
                "v {x} {y} {}\n",
                ((x * x + 3 * y * y + x * y) % 7) as f32 / 16.0
            ));
        }
    }
    let mut faces = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            let a = y * 5 + x + 1;
            obj.push_str(&format!("f {a} {} {}\n", a + 1, a + 6));
            obj.push_str(&format!("f {a} {} {}\n", a + 6, a + 5));
            faces.push([a - 1, a, a + 5, a + 4]);
        }
    }
    let source_faces = faces.clone();
    let mut fine = Mesh::from_obj(&obj).unwrap();
    fine.install_attributes(None, Some(SurfaceTopology { faces, uv: None }))
        .unwrap();
    fine.colors[12] = [0.125, 0.625, 0.875];
    fine.masks[12] = 0.375;
    fine.materials = Some(vec![[0.3, 0.7]; fine.positions.len()]);
    fine.materials.as_mut().unwrap()[12] = [0.8, 0.2];
    fine.validate().unwrap();
    let first = fine
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!(first.report.coarse_faces, 4);
    let second = first
        .coarse
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!(second.report.coarse_faces, 1);
    assert_ne!(
        second.correspondence.predicted_vertex_to_fine,
        (0..first.coarse.positions.len() as u32).collect::<Vec<_>>(),
        "the second root really requires hierarchy renumbering"
    );
    let mut document = Document::new(fine);
    document
        .reverse_source_static_subdivision_level(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!((document.level_count(), document.current_level()), (2, 0));
    document.select_level(1).unwrap();
    document.begin_stroke();
    {
        let upper = document.require_active_mesh_mut().unwrap();
        upper.positions[20].z += 0.03125;
        upper.colors[20] = [0.7, 0.2, 0.4];
        upper.masks[20] = 0.125;
        upper.materials.as_mut().unwrap()[20] = [0.6, 0.9];
        upper.recompute_normals();
    }
    document.end_stroke();
    let edited_fine = document.require_active_mesh().unwrap().snapshot();
    document.select_level(0).unwrap();
    let first_state = state(&document);
    document
        .reverse_source_static_subdivision_level(&ReverseSubdivisionSettings::default())
        .unwrap();
    let second_state = state(&document);
    assert_eq!((document.level_count(), document.current_level()), (3, 0));
    ensure_reloads(&document);
    document.select_level(2).unwrap();
    let raised = document.require_active_mesh().unwrap();
    let mut expected = edited_fine
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (
                p.to_array().map(f32::to_bits),
                edited_fine.colors[i].map(f32::to_bits),
                edited_fine.masks[i].to_bits(),
                edited_fine.materials.as_ref().unwrap()[i].map(f32::to_bits),
            )
        })
        .collect::<Vec<_>>();
    let mut actual = raised
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (
                p.to_array().map(f32::to_bits),
                raised.colors[i].map(f32::to_bits),
                raised.masks[i].to_bits(),
                raised.materials.as_ref().unwrap()[i].map(f32::to_bits),
            )
        })
        .collect::<Vec<_>>();
    expected.sort_unstable();
    actual.sort_unstable();
    assert_eq!(actual, expected, "highest physical points changed");
    document.select_level(0).unwrap();
    assert_eq!(state(&document), second_state);
    assert!(document.undo());
    assert_eq!(state(&document), first_state);
    assert!(document.redo());
    assert_eq!(state(&document), second_state);
    document.begin_stroke();
    {
        let coarse = document.require_active_mesh_mut().unwrap();
        coarse.positions[0].z += 0.0625;
        coarse.recompute_normals();
    }
    document.end_stroke();
    document.select_level(2).unwrap();
    assert!(
        document
            .require_active_mesh()
            .unwrap()
            .positions
            .iter()
            .zip(&edited_fine.positions)
            .any(|(a, b)| a.to_array().map(f32::to_bits) != b.to_array().map(f32::to_bits)),
        "coarse edit did not propagate through two recovered levels"
    );
    ensure_reloads(&document);
    let mut uv_fine = Mesh::from_obj(&obj).unwrap();
    let coordinates = (0..25)
        .map(|index| [(index % 5) as f32 / 4.0, (index / 5) as f32 / 4.0])
        .collect();
    let face_indices = source_faces.clone();
    uv_fine
        .install_attributes(
            None,
            Some(SurfaceTopology {
                faces: source_faces,
                uv: Some(CornerUv {
                    coordinates,
                    face_indices,
                }),
            }),
        )
        .unwrap();
    let uv_outcome = uv_fine
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    let predicted = uv_outcome.coarse.subdivided_static(&SMOOTH).unwrap().mesh;
    let original_uv = uv_fine.surface.as_ref().unwrap().uv.as_ref().unwrap();
    let predicted_uv = predicted.surface.as_ref().unwrap().uv.as_ref().unwrap();
    let mut uv_value_mismatch = 0;
    for (face_id, &original_face) in uv_outcome
        .correspondence
        .predicted_face_to_fine
        .iter()
        .enumerate()
    {
        let rotation = uv_outcome.correspondence.predicted_face_rotation[face_id] as usize;
        for corner in 0..4 {
            let old_id = original_uv.face_indices[original_face as usize][(rotation + corner) % 4];
            let new_id = predicted_uv.face_indices[face_id][corner];
            if original_uv.coordinates[old_id as usize] != predicted_uv.coordinates[new_id as usize]
            {
                uv_value_mismatch += 1;
            }
        }
    }
    assert_eq!(uv_value_mismatch, 0);
    let original_uv_table = original_uv.coordinates.clone();
    let mut uv_document = Document::new(uv_fine);
    uv_document
        .reverse_source_static_subdivision_level(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!(uv_document.level_count(), 2);
    assert_eq!(
        uv_document
            .require_active_mesh()
            .unwrap()
            .surface
            .as_ref()
            .unwrap()
            .uv
            .as_ref()
            .unwrap()
            .coordinates
            .len(),
        9
    );
    ensure_reloads(&uv_document);
    uv_document
        .reverse_source_static_subdivision_level(&ReverseSubdivisionSettings::default())
        .unwrap();
    let second_uv_state = state(&uv_document);
    assert_eq!(uv_document.level_count(), 3);
    assert_eq!(
        uv_document
            .require_active_mesh()
            .unwrap()
            .surface
            .as_ref()
            .unwrap()
            .uv
            .as_ref()
            .unwrap()
            .coordinates
            .len(),
        4
    );
    ensure_reloads(&uv_document);
    uv_document.select_level(2).unwrap();
    let finest = uv_document.require_active_mesh().unwrap();
    let retained_uv = finest.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(retained_uv.coordinates, original_uv_table);
    for (face, row) in finest
        .surface
        .as_ref()
        .unwrap()
        .faces
        .iter()
        .zip(&retained_uv.face_indices)
    {
        for corner in 0..4 {
            let position = finest.positions[face[corner] as usize];
            let expected_id = position.y.round() as u32 * 5 + position.x.round() as u32;
            assert_eq!(row[corner], expected_id, "fine UV corner ID changed");
        }
    }
    uv_document.select_level(0).unwrap();
    assert_eq!(state(&uv_document), second_uv_state);
    assert!(uv_document.undo());
    assert_eq!(uv_document.level_count(), 2);
    assert!(uv_document.redo());
    assert_eq!(state(&uv_document), second_uv_state);
}
#[test]
fn independent_uv_seam_grid_reverses_twice_without_losing_fine_aliases() {
    let mut obj = String::from("o Independent_reverse_two_step_seam_grid\n");
    for y in 0..5 {
        for x in 0..5 {
            obj.push_str(&format!(
                "v {x} {y} {}\n",
                ((x * x + 3 * y * y + x * y) % 7) as f32 / 16.0
            ));
        }
    }
    for y in 0..5 {
        for x in 0..5 {
            obj.push_str(&format!("vt {} {}\n", x as f32 / 4., y as f32 / 4.));
        }
    }
    for y in 0..5 {
        obj.push_str(&format!("vt 0.5 {}\n", y as f32 / 4.));
    }
    let seam_uv = |x: usize, y: usize, face_x: usize| -> usize {
        if x == 2 && face_x >= 2 {
            26 + y
        } else {
            y * 5 + x + 1
        }
    };
    for y in 0..4 {
        for x in 0..4 {
            let a = y * 5 + x + 1;
            obj.push_str(&format!(
                "f {a}/{} {}/{} {}/{} {}/{}\n",
                seam_uv(x, y, x),
                a + 1,
                seam_uv(x + 1, y, x),
                a + 6,
                seam_uv(x + 1, y + 1, x),
                a + 5,
                seam_uv(x, y + 1, x)
            ));
        }
    }
    let fine = Mesh::import_obj(obj.as_bytes()).unwrap().mesh;
    let source_uv = fine.surface.as_ref().unwrap().uv.as_ref().unwrap().clone();
    assert_eq!(source_uv.coordinates.len(), 30);
    let physical_corner_ids = |mesh: &Mesh| {
        let surface = mesh.surface.as_ref().unwrap();
        let uv = surface.uv.as_ref().unwrap();
        let mut rows = surface
            .faces
            .iter()
            .zip(&uv.face_indices)
            .flat_map(|(face, row)| {
                (0..4).map(|corner| {
                    let p = mesh.positions[face[corner] as usize];
                    (p.x.to_bits(), p.y.to_bits(), row[corner])
                })
            })
            .collect::<Vec<_>>();
        rows.sort_unstable();
        rows
    };
    let source_corners = physical_corner_ids(&fine);
    let mut document = Document::new(fine);
    document
        .reverse_source_static_subdivision_level(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!((document.level_count(), document.current_level()), (2, 0));
    let first = state(&document);
    assert_eq!(document.require_active_mesh().unwrap().positions.len(), 9);
    document
        .reverse_source_static_subdivision_level(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!((document.level_count(), document.current_level()), (3, 0));
    assert_eq!(document.require_active_mesh().unwrap().positions.len(), 4);
    let second = state(&document);
    ensure_reloads(&document);
    document.select_level(2).unwrap();
    let retained = document.require_active_mesh().unwrap();
    assert_eq!(
        retained
            .surface
            .as_ref()
            .unwrap()
            .uv
            .as_ref()
            .unwrap()
            .coordinates,
        source_uv.coordinates
    );
    assert_eq!(physical_corner_ids(retained), source_corners);
    document.select_level(0).unwrap();
    assert_eq!(state(&document), second);
    assert!(document.undo());
    assert_eq!(state(&document), first);
    assert!(document.redo());
    assert_eq!(state(&document), second);
}
fn check_fixture(name: &str) {
    let (m, v) = fixture(name);
    let before = mesh_bits(&m);
    let out = m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!(
        out.report.coarse_faces,
        v["expected_coarse_faces"].as_u64().unwrap() as usize
    );
    assert_eq!(
        out.report.coarse_quads,
        v["expected_coarse_quads"].as_u64().unwrap() as usize
    );
    let expected: Vec<u32> = serde_json::from_value(v["expected_even_fine_ids"].clone()).unwrap();
    assert_eq!(out.correspondence.coarse_vertex_to_fine, expected);
    assert_eq!(
        out.correspondence.predicted_vertex_to_fine.len(),
        m.positions.len()
    );
    assert_eq!(
        out.correspondence.predicted_face_to_fine.len(),
        m.surface.as_ref().unwrap().faces.len()
    );
    for (i, &f) in out.correspondence.coarse_vertex_to_fine.iter().enumerate() {
        assert_eq!(
            out.coarse.positions[i].to_array().map(f32::to_bits),
            m.positions[f as usize].to_array().map(f32::to_bits)
        );
    }
    assert_eq!(mesh_bits(&m), before);
}
#[test]
fn reverse_copies_even_rgb_protection_material_and_signed_zero_exactly() {
    let (mut m, _) = fixture("mixed-pyramid");
    for i in 0..m.positions.len() {
        m.colors[i] = [i as f32 / 32., -0.0, 0.75];
        m.masks[i] = i as f32 / 32.;
    }
    m.materials = Some(
        (0..m.positions.len())
            .map(|i| [i as f32 / 32., -0.0])
            .collect(),
    );
    let before = mesh_bits(&m);
    let out = m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    for (i, &f) in out.correspondence.coarse_vertex_to_fine.iter().enumerate() {
        assert_eq!(
            out.coarse.colors[i].map(f32::to_bits),
            m.colors[f as usize].map(f32::to_bits)
        );
        assert_eq!(out.coarse.masks[i].to_bits(), m.masks[f as usize].to_bits());
        assert_eq!(
            out.coarse.materials.as_ref().unwrap()[i].map(f32::to_bits),
            m.materials.as_ref().unwrap()[f as usize].map(f32::to_bits)
        );
    }
    assert_eq!(mesh_bits(&m), before);
}

#[test]
fn reverse_keeps_source_mask_convention_and_even_raw_words() {
    let (mut fine, _) = fixture("triangle");
    fine.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
    for (i, mask) in fine.masks.iter_mut().enumerate() {
        *mask = [-0.0, 1e-9, 0.25, 1.0][i % 4];
    }
    let before = mesh_bits(&fine);
    let out = fine
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!(out.coarse.mask_convention, fine.mask_convention);
    for (row, &source) in out.correspondence.coarse_vertex_to_fine.iter().enumerate() {
        assert_eq!(
            out.coarse.masks[row].to_bits(),
            fine.masks[source as usize].to_bits()
        );
    }
    assert_eq!(mesh_bits(&fine), before);
}

#[test]
fn source_mask_singleton_requires_v7_and_retains_fixed_correspondence_after_reload() {
    let mut mesh = Mesh::sphere(0);
    mesh.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
    mesh.masks.fill(1.0);
    let mut d = Document::new(mesh);
    d.add_subdivision_level().unwrap();
    d.delete_lower_subdivision_levels().unwrap();
    let saved = state(&d);
    let mut project: Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(project["version"], 7);
    let loaded = Document::from_json(&saved).unwrap();
    assert!(loaded.has_retained_subdivision_hierarchy());
    assert!(!loaded.supports_adaptive_detail());
    assert_eq!(state(&loaded), saved);
    project["version"] = json!(6);
    assert!(Document::from_json(&project.to_string()).is_err());
}
#[test]
fn reverse_does_not_infer_roles_from_coordinate_similarity_or_odd_channel_values() {
    let (mut m, _) = fixture("quad");
    let expected = m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap()
        .correspondence
        .coarse_vertex_to_fine;
    for i in 0..m.positions.len() {
        m.positions[i].z = (i as f32).sin() * 0.13;
        m.colors[i] = [0.2, 0.8, 0.4];
        m.masks[i] = 0.9;
    }
    m.recompute_normals();
    let out = m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!(out.correspondence.coarse_vertex_to_fine, expected);
}
#[test]
fn reverse_missing_materials_and_uv_remain_absent() {
    let (m, _) = fixture("quad");
    let out = m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert!(out.coarse.materials.is_none());
    assert!(out.coarse.surface.as_ref().unwrap().uv.is_none());
}
#[test]
fn reverse_imported_triangle_without_original_face_metadata_is_supported() {
    let (mut m, _) = fixture("triangle");
    m.surface = None;
    let out = m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!(out.report.coarse_vertices, 3);
}
#[test]
fn reverse_rejects_seed_conflicts_duplicates_indices_and_invalid_work_limits() {
    let (m, v) = fixture("triangle");
    let expected: Vec<u32> = serde_json::from_value(v["expected_even_fine_ids"].clone()).unwrap();
    let odd = (0..m.positions.len() as u32)
        .find(|i| !expected.contains(i))
        .unwrap();
    for settings in [
        ReverseSubdivisionSettings {
            even_seed_vertices: vec![odd],
            ..Default::default()
        },
        ReverseSubdivisionSettings {
            even_seed_vertices: vec![expected[0], expected[0]],
            ..Default::default()
        },
        ReverseSubdivisionSettings {
            even_seed_vertices: vec![m.positions.len() as u32],
            ..Default::default()
        },
        ReverseSubdivisionSettings {
            work_limit: 0,
            ..Default::default()
        },
        ReverseSubdivisionSettings {
            work_limit: MAX_REVERSE_WORK + 1,
            ..Default::default()
        },
    ] {
        assert!(m.reversed_static(&settings).is_err());
    }
}
#[test]
fn reverse_smaller_work_budget_rejects_atomically_and_later_valid_call_succeeds() {
    let (m, _) = fixture("mixed-pyramid");
    let before = mesh_bits(&m);
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings {
            work_limit: 16,
            ..Default::default()
        })
        .unwrap_err()
        .contains("work limit"));
    assert_eq!(mesh_bits(&m), before);
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .is_ok());
}
#[test]
fn reverse_incomplete_four_child_face_count_is_rejected() {
    let (mut m, _) = fixture("triangle");
    m.triangles.pop();
    m.surface.as_mut().unwrap().faces.pop();
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap_err()
        .contains("four fine faces"));
}
#[test]
fn reverse_inconsistent_winding_is_rejected() {
    let (mut m, _) = fixture("triangle");
    m.triangles[0].swap(0, 1);
    m.surface.as_mut().unwrap().faces[0].swap(0, 1);
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap_err()
        .contains("winding"));
}
#[test]
fn reverse_orphan_vertices_nonfinite_channels_and_invalid_indices_are_rejected() {
    let (m, _) = fixture("quad");
    for damaged in 0..4 {
        let mut x = m.snapshot();
        match damaged {
            0 => {
                x.positions.push(Vec3::Z);
                x.normals.push(Vec3::Z);
                x.colors.push([1.; 3]);
                x.masks.push(0.);
            }
            1 => x.colors[0][0] = f32::NAN,
            2 => x.surface.as_mut().unwrap().faces[0][0] = 999,
            _ => x.materials = Some(vec![[0., 0.]]),
        };
        assert!(x
            .reversed_static(&ReverseSubdivisionSettings::default())
            .is_err());
    }
}
#[test]
fn reverse_nonmanifold_duplicate_child_faces_reject_before_search() {
    let (mut m, _) = fixture("triangle");
    let tris = m.triangles.clone();
    let faces = m.surface.as_ref().unwrap().faces.clone();
    m.triangles.extend(tris);
    m.surface.as_mut().unwrap().faces.extend(faces);
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .is_err());
}
#[test]
fn reverse_can_process_disconnected_independent_components() {
    let (m, _) = fixture("quad");
    let mut other = m.snapshot();
    for p in &mut other.positions {
        p.x += 4.;
    }
    let merged = Mesh::concatenate(&[&m, &other]).unwrap();
    let out = merged
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!(out.report.components, 2);
    assert_eq!(out.report.coarse_faces, 2);
    assert_eq!(out.report.coarse_vertices, 8);
}
#[test]
fn reverse_independent_uv_charts_keep_equal_valued_seam_ids_distinct() {
    let (m, v) = fixture("uv-seam-patch");
    let before = mesh_bits(&m);
    let out = m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    let uv = out.coarse.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(
        uv.coordinates.len(),
        v["expected_coarse_uv_coordinates"].as_u64().unwrap() as usize
    );
    assert_eq!(out.report.fine_uv_coordinates_not_in_coarse, 10);
    assert_eq!(out.report.coarse_quads, 2);
    assert_eq!(
        uv.face_indices[0]
            .iter()
            .filter(|i| uv.face_indices[1].contains(i))
            .count(),
        0
    );
    assert_eq!(mesh_bits(&m), before);
}
#[test]
fn reverse_uv_midpoint_edits_reject_without_mutating_fine_surface() {
    let (mut m, _) = fixture("uv-seam-patch");
    m.surface.as_mut().unwrap().uv.as_mut().unwrap().coordinates[4][0] += 0.02;
    let before = mesh_bits(&m);
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap_err()
        .contains("UV midpoint/center"));
    assert_eq!(mesh_bits(&m), before);
}
#[test]
fn reverse_uv_child_seam_split_with_equal_values_is_retained() {
    let (mut m, _) = fixture("uv-seam-patch");
    let uv = m.surface.as_mut().unwrap().uv.as_mut().unwrap();
    let id = uv.face_indices[0][0];
    uv.coordinates.push(uv.coordinates[id as usize]);
    uv.face_indices[0][0] = uv.coordinates.len() as u32 - 1;
    let before = mesh_bits(&m);
    let recovered = m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert_eq!(recovered.report.coarse_faces, 2);
    assert_eq!(mesh_bits(&m), before);
}
#[test]
fn reverse_uv_child_cannot_merge_distinct_equal_valued_parent_charts() {
    let (mut m, _) = fixture("uv-seam-patch");
    let uv = m.surface.as_mut().unwrap().uv.as_mut().unwrap();
    assert_eq!(uv.coordinates[17], uv.coordinates[8]);
    uv.face_indices[0][0] = 8;
    let before = mesh_bits(&m);
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap_err()
        .contains("UV seam equivalence"));
    assert_eq!(mesh_bits(&m), before);
}
#[test]
fn reverse_corner_uv_signed_zero_is_copied_bit_exact() {
    let (mut m, _) = fixture("uv-seam-patch");
    m.surface.as_mut().unwrap().uv.as_mut().unwrap().coordinates[0][0] = -0.0;
    let out = m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap();
    assert!(out
        .coarse
        .surface
        .as_ref()
        .unwrap()
        .uv
        .as_ref()
        .unwrap()
        .coordinates
        .iter()
        .any(|v| v[0].to_bits() == (-0.0f32).to_bits()));
}
fn regular_torus() -> Mesh {
    let n = 4usize;
    let mut obj = String::new();
    let mut faces = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let u = x as f64 * std::f64::consts::TAU / n as f64;
            let v = y as f64 * std::f64::consts::TAU / n as f64;
            let r = 3. + v.cos();
            obj.push_str(&format!("v {} {} {}\n", r * u.cos(), r * u.sin(), v.sin()));
        }
    }
    for y in 0..n {
        for x in 0..n {
            let a = (y * n + x) as u32;
            let b = (y * n + (x + 1) % n) as u32;
            let c = (((y + 1) % n) * n + (x + 1) % n) as u32;
            let d = (((y + 1) % n) * n + x) as u32;
            faces.push([a, b, c, d]);
            obj.push_str(&format!(
                "f {} {} {}\nf {} {} {}\n",
                a + 1,
                b + 1,
                c + 1,
                a + 1,
                c + 1,
                d + 1
            ));
        }
    }
    let mut m = Mesh::from_obj(&obj).unwrap();
    m.install_attributes(None, Some(SurfaceTopology { faces, uv: None }))
        .unwrap();
    m.subdivided_static(&LINEAR).unwrap().mesh
}
#[test]
fn regular_unanchored_component_rejects_ambiguity_explicitly() {
    let m = regular_torus();
    let before = mesh_bits(&m);
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap_err()
        .contains("ambiguous"));
    assert_eq!(mesh_bits(&m), before);
}
#[test]
fn explicit_even_anchor_can_resolve_a_regular_component() {
    let m = regular_torus();
    let out = m
        .reversed_static(&ReverseSubdivisionSettings {
            even_seed_vertices: vec![0],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(out.report.coarse_vertices, 16);
    assert_eq!(out.report.coarse_quads, 16);
    assert_eq!(out.report.components, 1);
}
#[test]
fn every_disconnected_regular_component_requires_its_own_anchor() {
    let first = regular_torus();
    let mut second = first.snapshot();
    for p in &mut second.positions {
        p.x += 12.;
    }
    let m = Mesh::concatenate(&[&first, &second]).unwrap();
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings {
            even_seed_vertices: vec![0],
            ..Default::default()
        })
        .unwrap_err()
        .contains("ambiguous"));
    let out = m
        .reversed_static(&ReverseSubdivisionSettings {
            even_seed_vertices: vec![0, first.positions.len() as u32],
            ..Default::default()
        })
        .unwrap();
    assert_eq!(out.report.components, 2);
}
#[test]
fn reverse_table_count_cap_precedes_large_control_table_allocation() {
    let mut m = Mesh::sphere(0);
    let tri = m.triangles[0];
    m.triangles = vec![tri; 600_000];
    m.surface = Some(SurfaceTopology {
        faces: vec![[tri[0], tri[1], tri[2], TRIANGLE_CORNER]; 600_000],
        uv: None,
    });
    m.validate().unwrap();
    assert!(m
        .reversed_static(&ReverseSubdivisionSettings::default())
        .unwrap_err()
        .contains("control tables"));
}
#[test]
fn deletion_rejects_and_rolls_back_invalid_pending_stroke_with_redo_intact() {
    let mut d = hierarchy(1);
    d.select_level(1).unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[0] *= 1.001;
    d.require_active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    assert!(d.undo());
    let before = state(&d);
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().normals[0].x = f32::NAN;
    assert!(d.delete_higher_subdivision_levels().is_err());
    assert_eq!(state(&d), before);
    assert!(d.can_redo());
}
fn delete_then_draw_save_undo(lower: bool) {
    for mode in 0..3 {
        let mut d = hierarchy(mode);
        if !lower {
            d.select_level(0).unwrap();
        }
        let before_deletion = state(&d);
        if lower {
            d.delete_lower_subdivision_levels().unwrap();
        } else {
            d.delete_higher_subdivision_levels().unwrap();
        }
        assert_eq!(d.level_count(), 1);
        assert!(d.has_retained_subdivision_hierarchy());
        assert!(!d.supports_adaptive_detail());
        let deleted = state(&d);
        let original_points = d.require_active_mesh().unwrap().positions.len();
        let original_faces = d.require_active_mesh().unwrap().triangles.len();
        let settings = obrush_core::BrushSettings {
            radius: 0.3,
            strength: 0.35,
            symmetry_x: false,
            adaptive_detail: d.supports_adaptive_detail(),
            ..Default::default()
        };
        let point = d.require_active_mesh().unwrap().positions[0];
        let normal = d.require_active_mesh().unwrap().normals[0];
        d.begin_stroke();
        let result = d
            .require_active_mesh_mut()
            .unwrap()
            .apply_brush_with_result(point, normal, Vec3::ZERO, &settings);
        d.end_stroke();
        assert!(d.take_layer_error().is_none());
        assert!(result.changed_vertices > 0);
        assert_eq!(
            d.require_active_mesh().unwrap().positions.len(),
            original_points
        );
        assert_eq!(
            d.require_active_mesh().unwrap().triangles.len(),
            original_faces
        );
        let sculpted = state(&d);
        assert_ne!(sculpted, deleted);
        ensure_reloads(&d);
        assert!(d.undo());
        assert_eq!(state(&d), deleted);
        assert!(d.redo());
        assert_eq!(state(&d), sculpted);
        assert!(d.undo());
        assert!(d.undo());
        assert_eq!(state(&d), before_deletion);
    }
}
#[test]
fn delete_lower_then_draw_save_and_full_history_preserves_singleton_scheme() {
    delete_then_draw_save_undo(true);
}
#[test]
fn delete_higher_then_draw_save_and_full_history_preserves_singleton_scheme() {
    delete_then_draw_save_undo(false);
}
#[test]
fn direct_public_topology_edit_cannot_silently_detach_a_singleton_or_consume_redo() {
    let mut d = hierarchy(0);
    d.delete_lower_subdivision_levels().unwrap();
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().positions[0] *= 1.001;
    d.require_active_mesh_mut().unwrap().recompute_normals();
    d.end_stroke();
    assert!(d.undo());
    let before = state(&d);
    assert!(d.can_redo());
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().subdivide().unwrap();
    d.end_stroke();
    assert!(d
        .take_layer_error()
        .unwrap()
        .contains("hierarchy is retained"));
    assert_eq!(state(&d), before);
    assert!(d.can_redo());
    ensure_reloads(&d);
}
#[test]
fn adaptive_capability_respects_target_layers_and_surface_metadata_without_retained_levels() {
    let mut d = Document::new(Mesh::sphere(0));
    assert!(!d.has_retained_subdivision_hierarchy());
    assert!(d.supports_adaptive_detail());
    d.store_morph_target().unwrap();
    assert!(!d.supports_adaptive_detail());
    d.delete_morph_target().unwrap();
    assert!(d.supports_adaptive_detail());
    d.add_layer("Bound geometry").unwrap();
    assert!(!d.supports_adaptive_detail());
    let (fine, _) = fixture("quad");
    let d = Document::new(fine);
    assert!(!d.has_retained_subdivision_hierarchy());
    assert!(!d.supports_adaptive_detail());
}
#[test]
fn loop_lower_singleton_saves_and_reloads_as_version6() {
    let mut d = hierarchy(0);
    d.delete_lower_subdivision_levels().unwrap();
    let s = state(&d);
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["version"], 6);
    assert_eq!(v["multiresolution"]["scheme"], "loop_world_offsets_v1");
    assert_eq!(v["multiresolution"]["levels"], json!([null]));
    ensure_reloads(&d);
}
#[test]
fn loop_higher_singleton_saves_and_reloads_as_version6() {
    let mut d = hierarchy(0);
    d.select_level(0).unwrap();
    d.delete_higher_subdivision_levels().unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&state(&d)).unwrap()["version"],
        6
    );
    ensure_reloads(&d);
}
#[test]
fn active_loop_singleton_cannot_hide_in_legacy_version2_3_or4() {
    for legacy in 2..=4 {
        let mut d = hierarchy(0);
        d.delete_lower_subdivision_levels().unwrap();
        if legacy == 3 {
            d.add_layer("Legacy layer context").unwrap();
        }
        if legacy == 4 {
            d.store_morph_target().unwrap();
        }
        let mut v: Value = serde_json::from_str(&state(&d)).unwrap();
        v["version"] = json!(legacy);
        let e = Document::from_json(&serde_json::to_string(&v).unwrap()).unwrap_err();
        assert!(e.contains("version 6"), "{e}");
        v.as_object_mut().unwrap().remove("multiresolution");
        assert!(Document::from_json(&serde_json::to_string(&v).unwrap()).is_ok());
    }
}
#[test]
fn inactive_loop_singleton_cannot_hide_inside_version5_scene() {
    let mut d = hierarchy(0);
    d.delete_lower_subdivision_levels().unwrap();
    let original = d.active_subtool_id();
    d.add_subtool(Mesh::sphere(0), "Plain active part").unwrap();
    let mut v: Value = serde_json::from_str(&state(&d)).unwrap();
    assert_eq!(v["version"], 6);
    let object = v["scene"]["objects"]
        .as_array()
        .unwrap()
        .iter()
        .position(|p| p["id"] == json!(original))
        .unwrap();
    assert_eq!(
        v["scene"]["objects"][object]["payload"]["multiresolution"]["levels"],
        json!([null])
    );
    v["version"] = json!(5);
    assert!(Document::from_json(&serde_json::to_string(&v).unwrap())
        .unwrap_err()
        .contains("version 6"));
    v["scene"]["objects"][object]["payload"]
        .as_object_mut()
        .unwrap()
        .remove("multiresolution");
    assert!(Document::from_json(&serde_json::to_string(&v).unwrap()).is_ok());
}
#[test]
fn independent_historical_version2_two_level_loop_remains_valid_and_version2() {
    let raw = include_str!("fixtures/independent-legacy-loop-v2.obrush");
    let mut d = Document::from_json(raw).unwrap();
    assert_eq!(d.level_count(), 2);
    assert_eq!(d.current_level(), 1);
    let saved = state(&d);
    assert_eq!(serde_json::from_str::<Value>(&saved).unwrap()["version"], 2);
    assert_eq!(
        serde_json::from_str::<Value>(&saved).unwrap()["mesh"]["positions"],
        json!([
            [0.125, 0.125, 0.0],
            [0.75, 0.125, 0.0],
            [0.125, 0.75, 0.0],
            [0.5, 0.0, 0.0],
            [0.5, 0.5, 0.0],
            [0.0, 0.5, 0.0]
        ])
    );
    ensure_reloads(&d);
    d.select_level(0).unwrap();
    d.select_level(1).unwrap();
    assert_eq!(state(&d), saved);
}
