//! Independent migration contracts, not SculptGL feature/rendering parity.
use obrush_core::{
    AttributeExportPolicy, BrushKind, BrushSettings, CornerUv, DecimateSettings, Document,
    LockedStamp, LockedStampSample, MaskExtractSettings, Mesh, ObjectTransform, PlyEncoding,
    RemeshSettings, StlEncoding, SubtoolMergePolicy, SubtoolMergeSettings, SurfaceTopology, Vec3,
    DEFAULT_VERTEX_MATERIAL, TRIANGLE_CORNER,
};
use serde::Serialize;
use serde_json::Value;

fn square() -> Mesh {
    Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3\nf 1 3 4\n").unwrap()
}
fn seam() -> SurfaceTopology {
    SurfaceTopology {
        faces: vec![[0, 1, 2, TRIANGLE_CORNER], [0, 2, 3, TRIANGLE_CORNER]],
        uv: Some(CornerUv {
            coordinates: vec![[0., 0.], [1., 0.], [1., 1.], [2., 0.], [3., 1.], [2., 1.]],
            face_indices: vec![[0, 1, 2, TRIANGLE_CORNER], [3, 4, 5, TRIANGLE_CORNER]],
        }),
    }
}
fn rich_square() -> Mesh {
    let mut mesh = square();
    mesh.install_attributes(
        Some(vec![[0., 1.], [0.25, 0.75], [0.5, 0.5], [1., 0.]]),
        Some(seam()),
    )
    .unwrap();
    mesh
}
fn mirror_materials(mesh: &mut Mesh) {
    for (i, color) in mesh.colors.iter_mut().enumerate() {
        *color = [
            ((i * 7) % 19) as f32 / 18.,
            ((i * 13) % 23) as f32 / 22.,
            0.5,
        ];
    }
    mesh.materials = Some(mesh.colors.iter().map(|c| [c[0], c[1]]).collect());
}
fn exact_material_rgb(mesh: &Mesh) {
    let materials = mesh.materials.as_ref().unwrap();
    assert_eq!(materials.len(), mesh.positions.len());
    for (material, color) in materials.iter().zip(&mesh.colors) {
        assert_eq!(
            material.map(f32::to_bits),
            [color[0].to_bits(), color[1].to_bits()]
        );
    }
}
fn channels(mesh: &Mesh) -> String {
    serde_json::to_string(mesh).unwrap()
}
fn state(document: &Document) -> String {
    document.to_json().unwrap()
}

#[test]
fn legacy_none_mesh_bytes_match_independent_five_channel_record() {
    #[derive(Serialize)]
    struct Legacy<'a> {
        positions: &'a [Vec3],
        normals: &'a [Vec3],
        triangles: &'a [[u32; 3]],
        masks: &'a [f32],
        colors: &'a [[f32; 3]],
    }
    let mesh = square();
    let expected = serde_json::to_string(&Legacy {
        positions: &mesh.positions,
        normals: &mesh.normals,
        triangles: &mesh.triangles,
        masks: &mesh.masks,
        colors: &mesh.colors,
    })
    .unwrap();
    assert_eq!(channels(&mesh), expected);
    assert_eq!(
        state(&Document::new(mesh)),
        format!("{{\"version\":2,\"name\":\"Untitled sculpture\",\"mesh\":{expected}}}")
    );
}

#[test]
fn legacy_versions_one_through_five_have_none_attributes() {
    let base = state(&Document::new(square()));
    for version in [1, 2] {
        let source = base.replacen("\"version\":2", &format!("\"version\":{version}"), 1);
        let parsed = Document::from_json(&source).unwrap();
        assert!(!parsed.active_mesh().unwrap().has_attributes());
    }
    let mut doc = Document::new(square());
    doc.add_layer("layer").unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&state(&doc)).unwrap()["version"],
        3
    );
    assert!(!Document::from_json(&state(&doc))
        .unwrap()
        .active_mesh()
        .unwrap()
        .has_attributes());
    doc.store_morph_target().unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&state(&doc)).unwrap()["version"],
        4
    );
    doc.add_subtool(square(), "second").unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&state(&doc)).unwrap()["version"],
        5
    );
    assert!(!Document::from_json(&state(&doc))
        .unwrap()
        .active_mesh()
        .unwrap()
        .has_attributes());
}

#[test]
fn v6_roundtrip_keeps_materials_uv_seams_and_signed_zero_bits() {
    let mut mesh = rich_square();
    mesh.materials.as_mut().unwrap()[0][0] = -0.0;
    mesh.surface
        .as_mut()
        .unwrap()
        .uv
        .as_mut()
        .unwrap()
        .coordinates[0][1] = -0.0;
    let doc = Document::new(mesh);
    let saved = state(&doc);
    assert_eq!(serde_json::from_str::<Value>(&saved).unwrap()["version"], 6);
    assert_eq!(state(&Document::from_json(&saved).unwrap()), saved);
}

#[test]
fn attributes_in_an_inactive_part_force_v6_and_survive_selection() {
    let mut doc = Document::new(rich_square());
    let first = doc.active_subtool_id().unwrap();
    let second = doc.add_subtool(square(), "plain").unwrap();
    let encoded = state(&doc);
    assert_eq!(
        serde_json::from_str::<Value>(&encoded).unwrap()["version"],
        6
    );
    let mut parsed = Document::from_json(&encoded).unwrap();
    parsed.select_subtool(first).unwrap();
    assert_eq!(
        channels(parsed.active_mesh().unwrap()),
        channels(&rich_square())
    );
    parsed.select_subtool(second).unwrap();
    assert!(!parsed.active_mesh().unwrap().has_attributes());
}

#[test]
fn attrs_in_retained_level_force_v6_and_do_not_disappear_on_navigation() {
    let mut doc = Document::new(rich_square());
    doc.add_subdivision_level().unwrap();
    let fine = channels(doc.active_mesh().unwrap());
    doc.select_level(0).unwrap();
    let coarse = channels(doc.active_mesh().unwrap());
    for _ in 0..3 {
        doc.select_level(1).unwrap();
        assert_eq!(channels(doc.active_mesh().unwrap()), fine);
        doc.select_level(0).unwrap();
        assert_eq!(channels(doc.active_mesh().unwrap()), coarse);
    }
    let encoded = state(&doc);
    assert_eq!(
        serde_json::from_str::<Value>(&encoded).unwrap()["version"],
        6
    );
    assert_eq!(state(&Document::from_json(&encoded).unwrap()), encoded);
}

#[test]
fn legacy_version_with_rich_active_retained_or_inactive_mesh_is_rejected() {
    let mut docs = vec![Document::new(rich_square())];
    let mut levels = Document::new(rich_square());
    levels.add_subdivision_level().unwrap();
    docs.push(levels);
    let mut scene = Document::new(rich_square());
    scene.add_subtool(square(), "plain").unwrap();
    docs.push(scene);
    for doc in docs {
        let mut json: Value = serde_json::from_str(&state(&doc)).unwrap();
        json["version"] = 5.into();
        assert!(Document::from_json(&json.to_string()).is_err());
    }
}

#[test]
fn nonfinite_out_of_range_and_wrong_length_materials_reject_atomically() {
    let mut mesh = rich_square();
    let before = channels(&mesh);
    for materials in [
        vec![[0.2; 2]; 3],
        vec![[f32::NAN, 0.2]; 4],
        vec![[f32::INFINITY, 0.2]; 4],
        vec![[-0.01, 0.2]; 4],
        vec![[1.01, 0.2]; 4],
    ] {
        assert!(mesh
            .install_attributes(Some(materials), Some(seam()))
            .is_err());
        assert_eq!(channels(&mesh), before);
    }
}

#[test]
fn invalid_uv_shape_indices_coordinates_and_surface_mapping_reject_atomically() {
    let mut invalid = Vec::new();
    let mut s = seam();
    s.uv.as_mut().unwrap().face_indices.pop();
    invalid.push(s);
    let mut s = seam();
    s.uv.as_mut().unwrap().face_indices[0][0] = 6;
    invalid.push(s);
    let mut s = seam();
    s.uv.as_mut().unwrap().face_indices[0][3] = 0;
    invalid.push(s);
    let mut s = seam();
    s.uv.as_mut().unwrap().coordinates[0][0] = f32::NAN;
    invalid.push(s);
    let mut s = seam();
    s.uv.as_mut().unwrap().coordinates[0][0] = 1_000_001.;
    invalid.push(s);
    let mut s = seam();
    s.faces[0].swap(0, 1);
    invalid.push(s);
    let mut s = seam();
    s.faces.push(s.faces[0]);
    invalid.push(s);
    let mut s = seam();
    s.faces[0][0] = s.faces[0][1];
    invalid.push(s);
    let mut mesh = rich_square();
    let before = channels(&mesh);
    for s in invalid {
        assert!(mesh.install_attributes(None, Some(s)).is_err());
        assert_eq!(channels(&mesh), before);
    }
}

#[test]
fn valid_quad_preserves_corner_order_and_rejects_another_diagonal_explicitly() {
    let mut mesh = square();
    let mut s = SurfaceTopology {
        faces: vec![[0, 1, 2, 3]],
        uv: Some(CornerUv {
            coordinates: vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            face_indices: vec![[0, 1, 2, 3]],
        }),
    };
    mesh.install_attributes(None, Some(s.clone())).unwrap();
    let before = channels(&mesh);
    assert!(mesh.subdivide().unwrap_err().contains("quad"));
    assert_eq!(channels(&mesh), before);
    assert!(mesh.subdivide_smooth().unwrap_err().contains("quad"));
    assert_eq!(channels(&mesh), before);
    s.faces[0] = [3, 0, 1, 2];
    assert!(mesh.install_attributes(None, Some(s)).is_err());
    assert_eq!(channels(&mesh), before);
}

#[test]
fn linear_subdivision_preserves_two_distinct_shared_edge_uv_midpoints() {
    let mut mesh = rich_square();
    mesh.subdivide().unwrap();
    mesh.validate().unwrap();
    let surface = mesh.surface.as_ref().unwrap();
    let uv = surface.uv.as_ref().unwrap();
    assert_eq!(
        (
            mesh.positions.len(),
            mesh.triangles.len(),
            uv.coordinates.len()
        ),
        (9, 8, 12)
    );
    let left = uv.face_indices[0][2] as usize;
    let right = uv.face_indices[4][1] as usize;
    assert_ne!(left, right);
    assert_eq!(uv.coordinates[left], [0.5, 0.5]);
    assert_eq!(uv.coordinates[right], [2.5, 0.5]);
    assert_eq!(surface.faces.len(), 8);
}

#[test]
fn uv_edge_interning_preserves_equal_valued_distinct_seams() {
    let mut mesh = rich_square();
    let uv = mesh.surface.as_mut().unwrap().uv.as_mut().unwrap();
    uv.coordinates[3] = uv.coordinates[0];
    uv.coordinates[4] = uv.coordinates[2];
    mesh.subdivide().unwrap();
    let uv = mesh.surface.as_ref().unwrap().uv.as_ref().unwrap();
    let a = uv.face_indices[0][2];
    let b = uv.face_indices[4][1];
    assert_ne!(a, b);
    assert_eq!(uv.coordinates[a as usize], uv.coordinates[b as usize]);
}

#[test]
fn continuous_uv_shared_edge_is_interned_once_with_reversed_face_winding() {
    let mut mesh = square();
    let mut s = seam();
    s.uv.as_mut().unwrap().face_indices[1] = [0, 2, 5, TRIANGLE_CORNER];
    mesh.install_attributes(None, Some(s)).unwrap();
    mesh.subdivide().unwrap();
    let uv = mesh.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(uv.coordinates.len(), 11);
    assert_eq!(uv.face_indices[0][2], uv.face_indices[4][1]);
}

#[test]
fn loop_subdivision_uses_linear_uv_midpoints_while_materials_follow_loop_weights() {
    let mut mesh = rich_square();
    mirror_materials(&mut mesh);
    mesh.subdivide_smooth().unwrap();
    exact_material_rgb(&mesh);
    let uv = mesh.surface.as_ref().unwrap().uv.as_ref().unwrap();
    assert_eq!(uv.coordinates[uv.face_indices[0][2] as usize], [0.5, 0.5]);
    assert_eq!(
        mesh.materials.as_ref().unwrap()[0][0].to_bits(),
        ((0.0_f32 * 0.75) + (7. / 18.) * 0.125 + (2. / 18.) * 0.125).to_bits()
    );
}

#[test]
fn linear_material_interpolation_matches_independent_edge_values() {
    let mut mesh = square();
    mesh.install_attributes(
        Some(vec![[0., 1.], [0.25, 0.75], [0.5, 0.5], [1., 0.]]),
        None,
    )
    .unwrap();
    mesh.subdivide().unwrap();
    let m = mesh.materials.unwrap();
    assert_eq!(m[4], [0.125, 0.875]);
    assert_eq!(m[5], [0.375, 0.625]);
    assert_eq!(m[6], [0.25, 0.75]);
}

#[test]
fn refinement_material_midpoints_and_conformity_weights_follow_rgb_exactly() {
    let mut mesh = Mesh::sphere(1);
    mirror_materials(&mut mesh);
    let result = mesh.refine_brush_patch(Vec3::Z, 0.18, true).unwrap();
    assert!(result.vertices_added > 0);
    exact_material_rgb(&mesh);
    mesh.validate().unwrap();
}

#[test]
fn decimation_material_collapse_weights_follow_rgb_exactly() {
    let mut mesh = Mesh::sphere(2);
    mirror_materials(&mut mesh);
    let (result, report) = mesh
        .decimated(&DecimateSettings {
            target_ratio: 0.7,
            max_error: 0.2,
            preserve_sharp_features: false,
            ..Default::default()
        })
        .unwrap();
    assert!(report.collapses > 0);
    exact_material_rgb(&result);
    result.validate().unwrap();
}

#[test]
fn clipping_material_interpolation_uses_threshold_and_duplicates_both_shells() {
    let mut mesh = square();
    mesh.masks = vec![0., 1., 1., 0.];
    mesh.colors = mesh.positions.iter().map(|p| [p.x, p.y, 0.5]).collect();
    mesh.materials = Some(mesh.positions.iter().map(|p| [p.x, p.y]).collect());
    let (result, report) = mesh
        .extract_mask(&MaskExtractSettings {
            threshold: 0.25,
            thickness: 0.02,
        })
        .unwrap();
    exact_material_rgb(&result);
    let materials = result.materials.as_ref().unwrap();
    for i in 0..report.patch_vertices {
        assert_eq!(materials[i], materials[i + report.patch_vertices]);
    }
    assert!(materials.iter().any(|m| m[0] == 0.25));
    assert!(result.masks.iter().all(|&m| m == 0.));
}

#[test]
fn remesh_materials_use_same_nearest_triangle_barycentric_transfer_as_rgb() {
    let mut mesh = Mesh::sphere(0);
    mirror_materials(&mut mesh);
    let (result, _) = mesh
        .remeshed(&RemeshSettings {
            resolution: 16,
            smoothing_steps: 0,
            ..Default::default()
        })
        .unwrap();
    exact_material_rgb(&result);
    result.validate().unwrap();
}

#[test]
fn unsupported_surface_topology_mutations_reject_but_extraction_preserves_source() {
    let mut mesh = rich_square();
    mesh.masks[1] = 1.0;
    let before = channels(&mesh);
    assert!(mesh
        .refine_brush_patch(Vec3::new(0.5, 0.5, 0.), 0.18, false)
        .unwrap_err()
        .contains("original face/UV"));
    assert_eq!(channels(&mesh), before);
    assert!(mesh
        .decimated(&DecimateSettings::default())
        .unwrap_err()
        .contains("original face/UV"));
    assert_eq!(channels(&mesh), before);
    let (extracted, report) = mesh.extract_mask(&MaskExtractSettings::default()).unwrap();
    assert!(report.output_vertices > report.patch_vertices);
    assert!(extracted.surface.is_none());
    assert!(extracted.materials.is_some());
    assert_eq!(channels(&mesh), before);
    assert!(mesh
        .remeshed(&RemeshSettings {
            resolution: 16,
            ..Default::default()
        })
        .unwrap_err()
        .contains("allow UV loss"));
    assert_eq!(channels(&mesh), before);
}

#[test]
fn fourteen_fixed_topology_tools_preserve_materials_and_uv_bits() {
    for &kind in &BrushKind::ALL {
        let mut mesh = Mesh::sphere(1);
        mirror_materials(&mut mesh);
        let surface = SurfaceTopology {
            faces: mesh
                .triangles
                .iter()
                .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
                .collect(),
            uv: Some(CornerUv {
                coordinates: mesh.positions.iter().map(|p| [p.x, p.y]).collect(),
                face_indices: mesh
                    .triangles
                    .iter()
                    .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
                    .collect(),
            }),
        };
        mesh.install_attributes(mesh.materials.clone(), Some(surface))
            .unwrap();
        let materials = mesh.materials.clone();
        let surface = mesh.surface.clone();
        mesh.apply_brush(
            Vec3::Z,
            Vec3::Z,
            Vec3::new(0.01, 0., 0.),
            &BrushSettings {
                kind,
                radius: 0.7,
                strength: 0.1,
                symmetry_x: false,
                adaptive_detail: false,
                ..Default::default()
            },
        );
        assert_eq!(mesh.materials, materials, "{kind:?}");
        assert_eq!(mesh.surface, surface, "{kind:?}");
        mesh.validate().unwrap();
    }
}

#[test]
fn transforms_morph_and_layers_keep_the_same_bound_surface_channels() {
    let mut doc = Document::new(rich_square());
    let attrs = (
        doc.active_mesh().unwrap().materials.clone(),
        doc.active_mesh().unwrap().surface.clone(),
    );
    doc.store_morph_target().unwrap();
    doc.add_layer("shape").unwrap();
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().positions[0].z += 0.01;
    doc.active_mesh_mut().unwrap().recompute_normals();
    doc.end_stroke();
    doc.set_layer_intensity(0, 0.5).unwrap();
    doc.set_layer_visibility(0, false).unwrap();
    doc.bake_layers().unwrap();
    doc.swap_morph_target().unwrap();
    doc.restore_morph_target().unwrap();
    doc.transform_active_subtool(&ObjectTransform {
        translation: Vec3::new(0.5, 0.2, 0.1),
        rotation_degrees: Vec3::new(4., 8., 12.),
        scale: 1.2,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        (
            doc.active_mesh().unwrap().materials.clone(),
            doc.active_mesh().unwrap().surface.clone()
        ),
        attrs
    );
    let saved = state(&doc);
    assert_eq!(state(&Document::from_json(&saved).unwrap()), saved);
}

#[test]
fn installer_and_detachment_each_have_exact_complete_undo_redo() {
    let mut doc = Document::new(square());
    let plain = state(&doc);
    doc.install_mesh_attributes(rich_square().materials, Some(seam()))
        .unwrap();
    let rich = state(&doc);
    doc.detach_surface_topology().unwrap();
    let detached = state(&doc);
    assert!(doc.active_mesh().unwrap().materials.is_some());
    assert!(doc.active_mesh().unwrap().surface.is_none());
    assert!(doc.undo());
    assert_eq!(state(&doc), rich);
    assert!(doc.undo());
    assert_eq!(state(&doc), plain);
    assert!(doc.redo());
    assert_eq!(state(&doc), rich);
    assert!(doc.redo());
    assert_eq!(state(&doc), detached);
}

#[test]
fn invalid_installer_preserves_scene_history_redo_and_pending_stroke() {
    let mut doc = Document::new(rich_square());
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().colors[0] = [0.1, 0.2, 0.3];
    doc.end_stroke();
    assert!(doc.undo());
    let before = state(&doc);
    assert!(doc
        .install_mesh_attributes(Some(vec![[0.2; 2]; 3]), None)
        .is_err());
    assert_eq!(state(&doc), before);
    assert!(doc.can_redo());
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().positions[0].z += 0.01;
    let pending = state(&doc);
    assert!(doc
        .install_mesh_attributes(
            None,
            Some(SurfaceTopology {
                faces: vec![],
                uv: None
            })
        )
        .is_err());
    assert_eq!(state(&doc), pending);
    doc.end_stroke();
    assert!(doc.undo());
    assert_eq!(state(&doc), before);
}

#[test]
fn signed_zero_only_material_and_uv_edits_create_history() {
    let mut doc = Document::new(rich_square());
    let before = state(&doc);
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().materials.as_mut().unwrap()[0][0] = -0.0;
    doc.active_mesh_mut()
        .unwrap()
        .surface
        .as_mut()
        .unwrap()
        .uv
        .as_mut()
        .unwrap()
        .coordinates[0][0] = -0.0;
    doc.end_stroke();
    let after = state(&doc);
    assert_ne!(before, after);
    assert!(doc.undo());
    assert_eq!(state(&doc), before);
    assert!(doc.redo());
    assert_eq!(state(&doc), after);
}

#[test]
fn corrupted_v6_json_and_unknown_attribute_fields_are_not_installed() {
    let mut receiver = Document::new(square());
    let before = state(&receiver);
    let rich = state(&Document::new(rich_square()));
    for variant in 0..4 {
        let mut value: Value = serde_json::from_str(&rich).unwrap();
        match variant {
            0 => value["mesh"]["surface"]["uv"]["face_indices"][0][0] = 999.into(),
            1 => value["mesh"]["materials"][0][0] = 1.5.into(),
            2 => value["mesh"]["surface"]["unknown"] = true.into(),
            _ => value["mesh"]["surface"]["uv"]["coordinates"] = "malformed".into(),
        }
        let parsed = Document::from_json(&value.to_string());
        assert!(parsed.is_err());
        if let Ok(imported) = parsed {
            receiver.replace_document(imported).unwrap();
        }
        assert_eq!(state(&receiver), before);
        assert!(!receiver.can_undo());
    }
}

#[test]
fn whole_document_import_undo_restores_the_rich_original_scene() {
    let mut doc = Document::new(rich_square());
    doc.add_subtool(rich_square(), "second").unwrap();
    let before = state(&doc);
    doc.replace_document(Document::new(square())).unwrap();
    let after = state(&doc);
    assert!(doc.undo());
    assert_eq!(state(&doc), before);
    assert!(doc.redo());
    assert_eq!(state(&doc), after);
}

#[test]
fn duplicate_append_selection_and_scene_merge_preserve_all_attributes() {
    let mut doc = Document::new(rich_square());
    let first = doc.active_subtool_id().unwrap();
    let second = doc
        .add_subtool_preserving_normals(rich_square(), "second")
        .unwrap();
    let third = doc.duplicate_subtool(first).unwrap();
    assert_eq!(
        channels(doc.active_mesh().unwrap()),
        channels(&rich_square())
    );
    let before = state(&doc);
    let report = doc
        .merge_subtools(&SubtoolMergeSettings {
            ids: vec![first, second],
            name: "merged".into(),
            policy: SubtoolMergePolicy::RequirePlain,
            allow_source_pose_loss: false,
            allow_mask_precision_loss: false,
            allow_uv_loss: false,
        })
        .unwrap();
    let merged = state(&doc);
    assert_eq!(doc.active_mesh().unwrap().positions.len(), 8);
    assert_eq!(
        doc.active_mesh().unwrap().materials.as_ref().unwrap().len(),
        8
    );
    let uv = doc
        .active_mesh()
        .unwrap()
        .surface
        .as_ref()
        .unwrap()
        .uv
        .as_ref()
        .unwrap();
    assert_eq!(uv.coordinates.len(), 12);
    assert_eq!(uv.face_indices[2], [6, 7, 8, TRIANGLE_CORNER]);
    assert_ne!(report.merged_id, third);
    assert!(doc.undo());
    assert_eq!(state(&doc), before);
    assert!(doc.redo());
    assert_eq!(state(&doc), merged);
    doc.select_subtool(third).unwrap();
    assert_eq!(
        channels(doc.active_mesh().unwrap()),
        channels(&rich_square())
    );
}

#[test]
fn mixed_implicit_material_concatenation_uses_declared_defaults_and_no_welding() {
    let plain = square();
    let mut material = square();
    material.materials = Some(vec![[0.4, 0.7]; 4]);
    let result = Mesh::concatenate(&[&plain, &material]).unwrap();
    let m = result.materials.unwrap();
    assert_eq!(&m[..4], &[DEFAULT_VERTEX_MATERIAL; 4]);
    assert_eq!(&m[4..], &[[0.4, 0.7]; 4]);
}

#[test]
fn mixing_missing_uvs_rejects_without_inventing_texture_coordinates() {
    let a = rich_square();
    let b = square();
    assert!(Mesh::concatenate(&[&a, &b])
        .unwrap_err()
        .contains("invent missing UV"));
}

#[test]
fn default_export_checks_reject_rich_loss_and_explicit_policy_reports_without_mutation() {
    let mesh = rich_square();
    let before = channels(&mesh);
    assert!(mesh.to_stl(StlEncoding::Binary).is_err());
    assert!(mesh.to_ply(PlyEncoding::Ascii).is_err());
    assert!(mesh.export_obj_checked().is_err());
    for exported in [
        mesh.export_stl_with_policy(StlEncoding::Binary, AttributeExportPolicy::AllowAndReport)
            .unwrap(),
        mesh.export_ply_with_policy(PlyEncoding::Ascii, AttributeExportPolicy::AllowAndReport)
            .unwrap(),
    ] {
        assert!(!exported.bytes.is_empty());
        assert_eq!(exported.report.omitted_material_vertices, 4);
        assert_eq!(exported.report.omitted_original_faces, 2);
        assert_eq!(exported.report.omitted_uv_coordinates, 6);
        assert_eq!(exported.report.omitted_uv_faces, 2);
    }
    let obj = mesh
        .export_obj_with_policy(AttributeExportPolicy::AllowAndReport)
        .unwrap();
    assert_eq!(obj.report.omitted_material_vertices, 4);
    assert_eq!(obj.report.omitted_original_faces, 0);
    assert_eq!(obj.report.omitted_uv_coordinates, 0);
    let text = String::from_utf8(obj.bytes).unwrap();
    assert_eq!(text.lines().filter(|l| l.starts_with("vt ")).count(), 6);
    assert!(text.contains("f 1/1/1 2/2/2 3/3/3"));
    assert_eq!(channels(&mesh), before);
    assert!(mesh
        .export_stl_with_policy(StlEncoding::Binary, AttributeExportPolicy::RejectLoss)
        .is_err());
}

#[test]
fn legacy_none_export_bytes_are_exact_under_both_new_policies() {
    let mesh = square();
    for policy in [
        AttributeExportPolicy::RejectLoss,
        AttributeExportPolicy::AllowAndReport,
    ] {
        assert_eq!(
            mesh.export_stl_with_policy(StlEncoding::Binary, policy)
                .unwrap()
                .bytes,
            mesh.to_stl(StlEncoding::Binary).unwrap()
        );
        assert_eq!(
            mesh.export_ply_with_policy(PlyEncoding::BinaryLittleEndian, policy)
                .unwrap()
                .bytes,
            mesh.to_ply(PlyEncoding::BinaryLittleEndian).unwrap()
        );
        assert_eq!(
            mesh.export_obj_with_policy(policy).unwrap().bytes,
            mesh.to_obj().into_bytes()
        );
    }
}

#[test]
fn unknown_mesh_field_is_not_a_valid_attribute_channel() {
    let mut value: Value = serde_json::from_str(&state(&Document::new(rich_square()))).unwrap();
    value["mesh"]["materials"][0] = vec![0.2, 0.3, 0.4].into();
    assert!(Document::from_json(&value.to_string()).is_err());
}

#[test]
fn coarse_material_edits_transport_to_fine_levels_with_rgb_weights_and_uv_binding() {
    let mut mesh = rich_square();
    mirror_materials(&mut mesh);
    let mut doc = Document::new(mesh);
    doc.add_subdivision_level().unwrap();
    let fine_surface = doc.active_mesh().unwrap().surface.clone();
    doc.select_level(0).unwrap();
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().colors[0][0] = 0.2;
    doc.active_mesh_mut().unwrap().materials.as_mut().unwrap()[0][0] = 0.2;
    doc.end_stroke();
    doc.select_level(1).unwrap();
    exact_material_rgb(doc.active_mesh().unwrap());
    assert_eq!(doc.active_mesh().unwrap().surface, fine_surface);
    let encoded = state(&doc);
    assert_eq!(state(&Document::from_json(&encoded).unwrap()), encoded);
}

#[test]
fn changing_material_presence_or_uv_at_the_finest_retained_level_is_atomic() {
    let mut doc = Document::new(rich_square());
    doc.add_subdivision_level().unwrap();
    let before = state(&doc);
    assert!(doc
        .install_mesh_attributes(None, doc.active_mesh().unwrap().surface.clone())
        .unwrap_err()
        .contains("channel presence"));
    assert_eq!(state(&doc), before);
    let mut changed = doc.active_mesh().unwrap().surface.clone().unwrap();
    changed.uv.as_mut().unwrap().coordinates[0][0] = 0.4;
    assert!(doc
        .install_mesh_attributes(doc.active_mesh().unwrap().materials.clone(), Some(changed))
        .unwrap_err()
        .contains("UV/original-face"));
    assert_eq!(state(&doc), before);
    assert!(doc.detach_surface_topology().is_err());
    assert_eq!(state(&doc), before);
}

#[test]
fn retained_child_material_presence_and_uv_correspondence_are_validated_on_import() {
    let mut doc = Document::new(rich_square());
    doc.add_subdivision_level().unwrap();
    doc.select_level(0).unwrap();
    let before = state(&doc);
    for drop_material in [true, false] {
        let mut value: Value = serde_json::from_str(&before).unwrap();
        if drop_material {
            value["multiresolution"]["levels"][1]["materials"] = Value::Null;
        } else {
            value["multiresolution"]["levels"][1]["surface"]["uv"]["coordinates"][0][0] =
                0.3.into();
        }
        assert!(Document::from_json(&value.to_string()).is_err());
        assert_eq!(state(&doc), before);
    }
}

#[test]
fn mesh_attribute_installer_validates_existing_geometry_before_commit() {
    let mut mesh = square();
    mesh.positions[0].x = f32::NAN;
    assert!(mesh
        .install_attributes(Some(vec![[0.2, 0.3]; 4]), None)
        .is_err());
    assert!(mesh.materials.is_none());
}

#[test]
fn invalid_public_attribute_or_surface_topology_edit_rolls_back_without_spending_redo() {
    let mut doc = Document::new(rich_square());
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().colors[0] = [0.2; 3];
    doc.end_stroke();
    assert!(doc.undo());
    let before = state(&doc);
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().materials.as_mut().unwrap()[0][0] = 1.2;
    doc.end_stroke();
    assert!(doc.take_layer_error().is_some());
    assert_eq!(state(&doc), before);
    assert!(doc.can_redo());
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().triangles[0].swap(1, 2);
    doc.end_stroke();
    assert!(doc.take_layer_error().is_some());
    assert_eq!(state(&doc), before);
    assert!(doc.can_redo());
    assert!(doc.redo());
    assert_eq!(doc.active_mesh().unwrap().colors[0], [0.2; 3]);
}

#[test]
fn anchored_move_draw_twist_and_locked_stamps_reject_stale_attribute_edits() {
    let center = Vec3::new(0.5, 0.5, 0.0);
    let mut mesh = rich_square();
    let settings = BrushSettings {
        kind: BrushKind::Move,
        radius: 0.7,
        strength: 0.1,
        adaptive_detail: false,
        symmetry_x: false,
        ..Default::default()
    };
    let mut gesture = mesh
        .prepare_move_stroke(center, Vec3::Z, &settings)
        .unwrap();
    mesh.materials.as_mut().unwrap()[0][0] = 0.125;
    let before = channels(&mesh);
    assert_eq!(
        gesture
            .apply(&mut mesh, Vec3::new(0.01, 0., 0.), &settings)
            .changed_vertices,
        0
    );
    assert_eq!(channels(&mesh), before);
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        ..settings
    };
    let mut gesture = mesh.prepare_draw_stroke(&settings).unwrap();
    mesh.surface
        .as_mut()
        .unwrap()
        .uv
        .as_mut()
        .unwrap()
        .coordinates[0][0] = -0.0;
    let before = channels(&mesh);
    assert_eq!(
        gesture
            .apply(&mut mesh, center, Vec3::Z, &settings)
            .changed_vertices,
        0
    );
    assert_eq!(channels(&mesh), before);
    let settings = BrushSettings {
        kind: BrushKind::Twist,
        ..settings
    };
    let mut gesture = mesh
        .prepare_local_transform_stroke(center, Vec3::Z, Vec3::Z, &settings)
        .unwrap();
    mesh.materials.as_mut().unwrap()[0][0] = 0.25;
    let before = channels(&mesh);
    assert_eq!(
        gesture.apply(&mut mesh, 0.05, &settings).changed_vertices,
        0
    );
    assert_eq!(channels(&mesh), before);
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        ..settings
    };
    let mut stamp = LockedStamp::prepare(&mesh, center, Vec3::Z, &settings).unwrap();
    mesh.materials.as_mut().unwrap()[0][0] = 0.5;
    let before = channels(&mesh);
    assert!(stamp
        .apply(
            &mut mesh,
            LockedStampSample {
                radius: 0.7,
                alpha_rotation: 0.,
                alpha_tangent: None,
                pressure: 1.
            }
        )
        .unwrap_err()
        .contains("stale"));
    assert_eq!(channels(&mesh), before);
    assert!(stamp.cancel(&mut mesh).is_err());
    assert_eq!(channels(&mesh), before);
}

#[test]
fn independently_authored_v6_json_has_exact_material_uv_and_signed_zero_fields() {
    let source = include_str!("fixtures/independent-material-uv-seam-v6.obrush");
    let doc = Document::from_json(source).unwrap();
    assert_eq!(
        (
            doc.active_mesh().unwrap().positions.len(),
            doc.active_mesh().unwrap().triangles.len()
        ),
        (4, 2)
    );
    let materials = doc.active_mesh().unwrap().materials.as_ref().unwrap();
    assert_eq!(materials[0][0].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(materials[1], [0.25, 0.75]);
    assert_eq!(materials[3], [1., 0.]);
    let surface = doc.active_mesh().unwrap().surface.as_ref().unwrap();
    assert_eq!(surface.faces, seam().faces);
    let uv = surface.uv.as_ref().unwrap();
    assert_eq!(uv.coordinates.len(), 6);
    assert_eq!(uv.coordinates[0][1].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(uv.face_indices[1], [3, 4, 5, TRIANGLE_CORNER]);
    let encoded = state(&doc);
    assert_eq!(state(&Document::from_json(&encoded).unwrap()), encoded);
}
