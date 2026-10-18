//! Independent canonical mask, numerical, topology and transactional contracts.
use obrush_core::{
    AttributeExportPolicy, BrushKind, BrushSettings, DecimateSettings, Document, LockedStamp,
    LockedStampSample, MaskConvention, MaskConversionPolicy, MaskExtractSettings,
    MaskFilterOperation, MaskFilterSettings, MaskRegion, MaskRegionSettings, Mesh, ObjectTransform,
    RemeshSettings, SglScene, StaticSubdivisionMap, StaticSubdivisionSettings, Vec3, MAX_VERTICES,
};
use serde_json::Value;
const U: MaskConvention = MaskConvention::UnprotectedF32V1;
const P: MaskConvention = MaskConvention::ProtectedF32V1;
const EXACT: MaskConversionPolicy = MaskConversionPolicy::RequireRoundtripExact;
const ALLOW: MaskConversionPolicy = MaskConversionPolicy::AllowAndReport;
fn triangle() -> Mesh {
    Mesh::from_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap()
}
fn source(values: Vec<f32>) -> Mesh {
    let mut mesh = triangle();
    mesh.install_mask_channel(values, U).unwrap();
    mesh
}
fn bits(mesh: &Mesh) -> Vec<u32> {
    mesh.masks.iter().map(|v| v.to_bits()).collect()
}
fn exact(a: &Mesh, b: &Mesh) {
    assert_eq!(
        serde_json::to_string(a).unwrap(),
        serde_json::to_string(b).unwrap()
    );
}
fn settings(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        radius: 2.0,
        strength: 0.5,
        symmetry_x: false,
        adaptive_detail: false,
        ..Default::default()
    }
}
fn scalars() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/independent-mask-convention-scalars.json"
    ))
    .unwrap()
}
fn source_document() -> Document {
    Document::from_json(include_str!("fixtures/independent-unprotected-v7.obrush")).unwrap()
}
fn region(erase: bool) -> MaskRegionSettings {
    MaskRegionSettings {
        region: MaskRegion::Rectangle {
            start: [0., 0.],
            end: [100., 100.],
        },
        view_projection: [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ],
        viewport: [0., 0., 100., 100.],
        erase,
    }
}

#[test]
fn independent_twelve_source_words_are_direct_influence() {
    for row in scalars()["rows"].as_array().unwrap() {
        let raw = f32::from_bits(row["raw_unprotected_bits"].as_u64().unwrap() as u32);
        assert_eq!(U.influence(raw).to_bits(), raw.to_bits());
        assert_eq!(
            U.protection_for_display(raw).to_bits(),
            row["lossy_derived_protection_bits"].as_u64().unwrap() as u32
        );
        assert_eq!(U.influence_f64(raw), f64::from(raw));
    }
}
#[test]
fn independent_legacy_protected_complement_bit_ledger_is_unchanged() {
    for row in scalars()["rows"].as_array().unwrap() {
        let raw = f32::from_bits(row["raw_unprotected_bits"].as_u64().unwrap() as u32);
        assert_eq!(
            P.influence(raw).to_bits(),
            row["lossy_derived_protection_bits"].as_u64().unwrap() as u32
        );
        assert_eq!(P.influence_f64(raw), 1.0 - f64::from(raw));
        assert_eq!(P.protection_for_display(raw).to_bits(), raw.to_bits());
    }
}
#[test]
fn smallest_positive_weights_define_double_center_and_normal() {
    let mesh = source(vec![f32::from_bits(1); 3]);
    let sum: f64 = (0..3).map(|i| mesh.mask_influence_f64(i)).sum();
    let center = (0..3).fold(glam::DVec3::ZERO, |v, i| {
        v + mesh.positions[i].as_dvec3() * mesh.mask_influence_f64(i)
    }) / sum;
    let normal = (0..3)
        .fold(glam::DVec3::ZERO, |v, i| {
            v + mesh.normals[i].as_dvec3() * mesh.mask_influence_f64(i)
        })
        .normalize();
    assert!(sum > 0.);
    assert_eq!(center.to_array(), [1. / 3., 1. / 3., 0.]);
    assert_eq!(normal, glam::DVec3::Z);
}
#[test]
fn tiny_source_weight_is_not_fully_protected_despite_display_rounding() {
    let mesh = source(vec![f32::from_bits(1), 0., 1.]);
    assert_eq!(mesh.protection_for_display(0), 1.);
    assert!(!mesh.fully_protected(0));
    assert!(mesh.fully_protected(1));
    assert!(!mesh.has_protection(2));
}
#[test]
fn threshold_endpoints_do_not_round_tiny_source_values_into_protection() {
    let tiny = f32::from_bits(1);
    assert!(!U.protection_at_least(tiny, 1.));
    assert!(U.protection_at_least(-0., 1.));
    assert!(!U.protection_at_least(1., tiny));
    assert!(U.protection_at_least(f32::from_bits(1.0f32.to_bits() - 1), tiny));
    assert!(!U.protection_equals(tiny, 1.));
    assert!(U.protection_equals(-0., 1.));
}
#[test]
fn independent_v7_tiny_and_signed_zero_words_survive_save_reload() {
    let doc = source_document();
    assert_eq!(
        bits(doc.active_mesh().unwrap()),
        [1, 0x80000000, 0x3f000000]
    );
    let json = doc.to_json().unwrap();
    assert_eq!(serde_json::from_str::<Value>(&json).unwrap()["version"], 7);
    exact(
        doc.active_mesh().unwrap(),
        Document::from_json(&json).unwrap().active_mesh().unwrap(),
    );
}
#[test]
fn default_and_explicit_protected_conventions_emit_identical_legacy_bytes() {
    let mesh = triangle();
    let legacy = serde_json::to_string(&mesh).unwrap();
    assert!(!legacy.contains("mask_convention"));
    let mut dto: Value = serde_json::from_str(&legacy).unwrap();
    dto["mask_convention"] = "protected_f32_v1".into();
    let decoded: Mesh = serde_json::from_value(dto).unwrap();
    assert_eq!(decoded.mask_convention, P);
    assert_eq!(serde_json::to_string(&decoded).unwrap(), legacy);
}
#[test]
fn source_convention_hidden_under_every_supported_old_version_rejects() {
    let mut dto: Value =
        serde_json::from_str(include_str!("fixtures/independent-unprotected-v7.obrush")).unwrap();
    for version in 1..=6 {
        dto["version"] = version.into();
        assert!(Document::from_json(&dto.to_string())
            .unwrap_err()
            .contains("version 7"));
    }
}
#[test]
fn source_convention_hidden_only_in_retained_level_rejects_old_version() {
    let mut doc = Document::new(triangle());
    doc.add_subdivision_level().unwrap();
    let mut dto: Value = serde_json::from_str(&doc.to_json().unwrap()).unwrap();
    dto["version"] = 6.into();
    dto["multiresolution"]["levels"][0]["mask_convention"] = "unprotected_f32_v1".into();
    assert!(Document::from_json(&dto.to_string())
        .unwrap_err()
        .contains("version 7"));
}
#[test]
fn source_convention_in_inactive_object_requires_version7() {
    let mut doc = source_document();
    doc.add_subtool(triangle(), "Plain").unwrap();
    let json = doc.to_json().unwrap();
    let mut dto: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(dto["version"], 7);
    dto["version"] = 6.into();
    assert!(Document::from_json(&dto.to_string())
        .unwrap_err()
        .contains("version 7"));
    let mut reloaded = Document::from_json(&json).unwrap();
    reloaded.select_subtool(1).unwrap();
    assert_eq!(
        bits(reloaded.active_mesh().unwrap()),
        [1, 0x80000000, 0x3f000000]
    );
}
#[test]
fn unknown_convention_rejects_decoding() {
    let mut dto: Value =
        serde_json::from_str(include_str!("fixtures/independent-unprotected-v7.obrush")).unwrap();
    dto["mesh"]["mask_convention"] = "other_raw_masks".into();
    assert!(Document::from_json(&dto.to_string()).is_err());
}
#[test]
fn mask_channel_length_failure_is_atomic() {
    let mut mesh = source(vec![f32::from_bits(1), -0., 0.5]);
    let before = mesh.snapshot();
    assert!(mesh.install_mask_channel(vec![0., 1.], P).is_err());
    exact(&mesh, &before);
}
#[test]
fn nonfinite_and_out_of_range_channel_failure_is_atomic() {
    let mut mesh = triangle();
    let before = mesh.snapshot();
    for bad in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        assert!(mesh.install_mask_channel(vec![0., bad, 1.], U).is_err());
        exact(&mesh, &before);
    }
}
#[test]
fn excessive_channel_capacity_is_rejected_before_install() {
    let mut mesh = triangle();
    let before = mesh.snapshot();
    let mut values = Vec::with_capacity(MAX_VERTICES + 1);
    values.extend([0., 0., 0.]);
    assert!(mesh
        .install_mask_channel(values, U)
        .unwrap_err()
        .contains("allocation"));
    exact(&mesh, &before);
}
#[test]
fn exact_conversion_rejects_tiny_or_signed_zero_loss_without_mutation() {
    let mesh = source(vec![f32::from_bits(1), -0., 0.5]);
    let before = mesh.snapshot();
    assert!(mesh
        .converted_mask_convention(P, EXACT)
        .unwrap_err()
        .contains("2 f32"));
    exact(&mesh, &before);
}
#[test]
fn explicit_conversion_report_matches_independent_roundtrip_ledger() {
    for row in scalars()["rows"].as_array().unwrap() {
        let word = row["raw_unprotected_bits"].as_u64().unwrap() as u32;
        let mesh = source(vec![f32::from_bits(word); 3]);
        let (converted, report) = mesh.converted_mask_convention(P, ALLOW).unwrap();
        assert_eq!(
            converted.masks[0].to_bits(),
            row["lossy_derived_protection_bits"].as_u64().unwrap() as u32
        );
        assert_eq!(
            report.roundtrip_changed_words,
            if row["raw_bitwise_roundtrip"].as_bool().unwrap() {
                0
            } else {
                3
            }
        );
        assert_eq!(
            report.max_abs_roundtrip_error,
            row["absolute_error"].as_f64().unwrap()
        );
        assert_eq!(
            report.signed_zero_only_changes,
            if word == 0x80000000 { 3 } else { 0 }
        );
    }
}
#[test]
fn exact_endpoint_half_conversion_is_reversible() {
    let mesh = source(vec![0., 0.5, 1.]);
    let (protected, r) = mesh.converted_mask_convention(P, EXACT).unwrap();
    assert_eq!(r.roundtrip_changed_words, 0);
    let (restored, _) = protected.converted_mask_convention(U, EXACT).unwrap();
    exact(&mesh, &restored);
}
#[test]
fn document_conversion_is_one_complete_undo_redo_transition() {
    let mut doc = source_document();
    let before = doc.to_json().unwrap();
    doc.convert_mask_convention(P, ALLOW).unwrap();
    let after = doc.to_json().unwrap();
    assert_eq!(doc.active_mesh().unwrap().mask_convention, P);
    assert!(doc.undo());
    assert_eq!(doc.to_json().unwrap(), before);
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), after);
}
#[test]
fn conversion_noop_preserves_available_redo() {
    let mut doc = Document::new(triangle());
    doc.convert_mask_convention(U, EXACT).unwrap();
    assert!(doc.undo());
    assert!(doc.can_redo());
    doc.convert_mask_convention(P, EXACT).unwrap();
    assert!(doc.can_redo());
    assert!(doc.redo());
    assert_eq!(doc.active_mesh().unwrap().mask_convention, U);
}
#[test]
fn failed_conversion_preserves_scene_and_redo() {
    let mut doc = source_document();
    doc.invert_mask();
    assert!(doc.undo());
    let before = doc.to_json().unwrap();
    assert!(doc.can_redo());
    assert!(doc.convert_mask_convention(P, EXACT).is_err());
    assert_eq!(doc.to_json().unwrap(), before);
    assert!(doc.can_redo());
}
#[test]
fn retained_hierarchy_conversion_rejects_without_changing_state() {
    let mut doc = Document::new(source(vec![0.25, 0.5, 0.75]));
    doc.add_subdivision_level().unwrap();
    let before = doc.to_json().unwrap();
    assert!(doc
        .convert_mask_convention(P, ALLOW)
        .unwrap_err()
        .contains("retained"));
    assert_eq!(doc.to_json().unwrap(), before);
}
#[test]
fn mask_brush_adds_and_removes_protection_in_source_storage() {
    let mut mesh = source(vec![0.5; 3]);
    let mut settings = settings(BrushKind::Mask);
    assert!(mesh.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings) > 0);
    let masked = mesh.masks[0];
    assert!(masked < 0.5);
    settings.invert = true;
    assert!(mesh.apply_brush(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &settings) > 0);
    assert!(mesh.masks[0] > masked);
    assert_eq!(mesh.mask_convention, U);
}
#[test]
fn source_clear_and_invert_keep_convention_and_whole_history() {
    let mut doc = source_document();
    let before = doc.to_json().unwrap();
    doc.clear_mask();
    assert_eq!(doc.active_mesh().unwrap().masks, [1.; 3]);
    assert_eq!(doc.active_mesh().unwrap().mask_convention, U);
    doc.invert_mask();
    assert_eq!(doc.active_mesh().unwrap().masks, [0.; 3]);
    assert!(doc.undo());
    assert_eq!(doc.active_mesh().unwrap().masks, [1.; 3]);
    assert!(doc.undo());
    assert_eq!(doc.to_json().unwrap(), before);
}
#[test]
fn grow_and_shrink_swap_canonical_extrema() {
    let mesh = source(vec![0.125, 0.5, 0.875]);
    for (operation, expected) in [
        (MaskFilterOperation::Grow, 0.125),
        (MaskFilterOperation::Shrink, 0.875),
    ] {
        let (values, _) = mesh
            .filtered_masks(&MaskFilterSettings {
                operation,
                iterations: 1,
                strength: 1.,
            })
            .unwrap();
        assert_eq!(values, vec![expected; 3]);
    }
}
#[test]
fn canonical_blur_preserves_smallest_equal_weight_without_complementing() {
    let mesh = source(vec![f32::from_bits(1); 3]);
    let (values, report) = mesh.filtered_masks(&MaskFilterSettings::default()).unwrap();
    assert_eq!(
        values.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        [1; 3]
    );
    assert_eq!(report.changed_vertices, 0);
}
#[test]
fn screen_region_endpoints_use_source_clear_and_full_values() {
    let mesh = source(vec![0.5; 3]);
    let (full, _) = mesh.region_masks(&region(false)).unwrap();
    let (clear, _) = mesh.region_masks(&region(true)).unwrap();
    assert_eq!(full, [0.; 3]);
    assert_eq!(clear, [1.; 3]);
}
#[test]
fn midpoint_subdivision_preserves_canonical_scalar_stencils() {
    let mut mesh = source(vec![0.125, 0.5, 0.875]);
    mesh.subdivide().unwrap();
    assert_eq!(mesh.mask_convention, U);
    assert_eq!(&mesh.masks[..3], &[0.125, 0.5, 0.875]);
    assert_eq!(&mesh.masks[3..], &[0.3125, 0.6875, 0.5]);
}
#[test]
fn legacy_loop_subdivision_preserves_source_convention() {
    let mut mesh = source(vec![0., 1., 0.]);
    mesh.subdivide_smooth().unwrap();
    assert_eq!(mesh.mask_convention, U);
    assert_eq!(mesh.masks, [0.125, 0.75, 0.125, 0.5, 0.5, 0.]);
}
#[test]
fn source_static_preserves_old_mask_words_and_new_convention() {
    let mesh = source(vec![f32::from_bits(1), -0., 0.5]);
    let result = mesh
        .subdivided_static(&StaticSubdivisionSettings {
            linear_edges: false,
        })
        .unwrap();
    assert_eq!(result.mesh.mask_convention, U);
    assert_eq!(&bits(&result.mesh)[..3], &bits(&mesh));
}
#[test]
fn static_map_rejects_stale_convention_even_if_raw_words_match() {
    let mesh = triangle();
    let map = StaticSubdivisionMap::build(
        &mesh,
        StaticSubdivisionSettings {
            linear_edges: false,
        },
    )
    .unwrap();
    let mut changed = mesh.snapshot();
    changed.mask_convention = U;
    assert!(map.apply(&changed).is_err());
}
#[test]
fn refinement_distinguishes_zero_from_positive_source_influence() {
    let mut zero = source(vec![0.; 3]);
    let mut positive = source(vec![f32::from_bits(1); 3]);
    zero.refine_brush_patch(Vec3::new(0.25, 0.25, 0.), 0.4, false)
        .unwrap();
    positive
        .refine_brush_patch(Vec3::new(0.25, 0.25, 0.), 0.4, false)
        .unwrap();
    assert_eq!(zero.positions.len(), 3);
    assert!(positive.positions.len() > 3);
    assert_eq!(positive.mask_convention, U);
}
#[test]
fn decimation_pinning_uses_source_semantics() {
    let mut free = Mesh::sphere(1);
    free.install_mask_channel(vec![1.; free.positions.len()], U)
        .unwrap();
    let mut pinned = free.snapshot();
    pinned.masks.fill(0.);
    let settings = DecimateSettings {
        max_error: 1.,
        preserve_sharp_features: false,
        ..Default::default()
    };
    let (reduced, _) = free.decimated(&settings).unwrap();
    let (retained, _) = pinned.decimated(&settings).unwrap();
    assert!(reduced.triangles.len() < free.triangles.len());
    assert_eq!(retained.triangles.len(), pinned.triangles.len());
    assert_eq!(reduced.mask_convention, U);
}
#[test]
fn remesh_carries_source_convention_and_constant_canonical_values() {
    let mut mesh = Mesh::sphere(0);
    mesh.install_mask_channel(vec![0.25; mesh.positions.len()], U)
        .unwrap();
    let (out, _) = mesh
        .remeshed(&RemeshSettings {
            resolution: 16,
            smoothing_steps: 0,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(out.mask_convention, U);
    assert!(out.masks.iter().all(|v| *v == 0.25));
}
#[test]
fn extraction_outputs_an_explicit_clear_source_mask() {
    let mut mesh = Mesh::sphere(0);
    mesh.install_mask_channel(vec![0.25; mesh.positions.len()], U)
        .unwrap();
    let (out, _) = mesh.extract_mask(&MaskExtractSettings::default()).unwrap();
    assert_eq!(out.mask_convention, U);
    assert!(out.masks.iter().all(|v| *v == 1.));
}
#[test]
fn concatenation_preserves_words_and_rejects_mixed_conventions() {
    let a = source(vec![f32::from_bits(1), -0., 0.5]);
    let b = source(vec![0.25, 0.75, 1.]);
    let merged = Mesh::concatenate(&[&a, &b]).unwrap();
    assert_eq!(merged.mask_convention, U);
    assert_eq!(
        bits(&merged),
        bits(&a).into_iter().chain(bits(&b)).collect::<Vec<_>>()
    );
    assert!(Mesh::concatenate(&[&a, &triangle()]).is_err());
}
#[test]
fn locked_stamp_detects_a_convention_only_stale_change() {
    let mut mesh = triangle();
    let mut stamp =
        LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &settings(BrushKind::Draw)).unwrap();
    mesh.mask_convention = U;
    let before = mesh.snapshot();
    assert!(stamp
        .apply(
            &mut mesh,
            LockedStampSample {
                radius: 2.,
                alpha_rotation: 0.,
                alpha_tangent: None,
                pressure: 1.
            }
        )
        .is_err());
    exact(&mesh, &before);
}
#[test]
fn anchored_move_detects_a_convention_only_stale_change() {
    let mut mesh = triangle();
    let settings = settings(BrushKind::Move);
    let mut move_stroke = mesh
        .prepare_move_stroke(Vec3::ZERO, Vec3::Z, &settings)
        .unwrap();
    mesh.mask_convention = U;
    let before = mesh.snapshot();
    assert_eq!(
        move_stroke
            .apply(&mut mesh, Vec3::new(0.1, 0., 0.), &settings)
            .changed_vertices,
        0
    );
    exact(&mesh, &before);
}
#[test]
fn geometry_layer_and_transform_history_preserve_source_mask_words() {
    let mut doc = source_document();
    let original = bits(doc.active_mesh().unwrap());
    doc.add_layer("Geometry").unwrap();
    doc.transform_active_subtool(&ObjectTransform {
        translation: Vec3::X,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(bits(doc.active_mesh().unwrap()), original);
    assert_eq!(doc.active_mesh().unwrap().mask_convention, U);
    assert!(doc.undo());
    assert_eq!(bits(doc.active_mesh().unwrap()), original);
    assert!(doc.redo());
    assert_eq!(bits(doc.active_mesh().unwrap()), original);
}
#[test]
fn morph_target_and_scene_duplicate_preserve_source_channel() {
    let mut doc = source_document();
    let original = bits(doc.active_mesh().unwrap());
    doc.store_morph_target().unwrap();
    doc.duplicate_subtool(1).unwrap();
    assert_eq!(bits(doc.active_mesh().unwrap()), original);
    assert_eq!(doc.active_mesh().unwrap().mask_convention, U);
    let json = doc.to_json().unwrap();
    let restored = Document::from_json(&json).unwrap();
    assert_eq!(bits(restored.active_mesh().unwrap()), original);
    assert_eq!(restored.active_mesh().unwrap().mask_convention, U);
}
#[test]
fn opt_in_obj_mat_retains_source_byte_channel_without_legacy_change() {
    let bytes = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n#MAT 102000304001506080\n";
    let source = Mesh::import_obj_with_mask_convention(bytes, U)
        .unwrap()
        .mesh;
    let legacy = Mesh::import_obj(bytes).unwrap().mesh;
    assert_eq!(source.mask_convention, U);
    assert_eq!(source.masks, [0., 1. / 255., 128. / 255.]);
    assert_eq!(legacy.mask_convention, P);
    assert_eq!(
        legacy.masks,
        source.masks.iter().map(|v| 1. - v).collect::<Vec<_>>()
    );
}
#[test]
fn opt_in_obj_without_mat_keeps_absent_source_channel_default() {
    let imported =
        Mesh::import_obj_with_mask_convention(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n", U)
            .unwrap()
            .mesh;
    assert_eq!(imported.mask_convention, P);
    assert!(imported.materials.is_none());
}
#[test]
fn checked_export_reports_source_channel_loss_without_editing_mesh() {
    let mesh = source(vec![f32::from_bits(1), -0., 0.5]);
    let before = mesh.snapshot();
    assert!(mesh
        .export_obj_with_policy(AttributeExportPolicy::RejectLoss)
        .is_err());
    let exported = mesh
        .export_obj_with_policy(AttributeExportPolicy::AllowAndReport)
        .unwrap();
    assert_eq!(exported.report.omitted_source_mask_vertices, 3);
    exact(&mesh, &before);
}
#[test]
fn independent_raw_sgl_material_z_can_populate_exact_canonical_channel() {
    let bytes = include_bytes!("fixtures/independent-v2-negative-markers.sgl");
    let raw = SglScene::from_bytes(bytes).unwrap();
    let values: Vec<_> = raw.meshes[0]
        .materials
        .as_ref()
        .unwrap()
        .iter()
        .map(|m| m[2])
        .collect();
    assert_eq!(
        values.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        [1, 0x33000000, 0x33000001]
    );
    let mesh = source(values);
    assert_eq!(bits(&mesh), [1, 0x33000000, 0x33000001]);
    assert_eq!(raw.to_bytes().unwrap(), bytes);
}
#[test]
fn direct_retained_convention_change_rolls_back_on_end_stroke() {
    let mut doc = Document::new(triangle());
    doc.add_subdivision_level().unwrap();
    let before = doc.to_json().unwrap();
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().mask_convention = U;
    doc.end_stroke();
    assert_eq!(doc.to_json().unwrap(), before);
    assert_eq!(doc.active_mesh().unwrap().mask_convention, P);
}
#[test]
fn canonical_schema_has_one_mask_channel_and_no_lossy_cache() {
    let doc = source_document();
    let dto: Value = serde_json::from_str(&doc.to_json().unwrap()).unwrap();
    let map = dto["mesh"].as_object().unwrap();
    let mask_keys: Vec<_> = map
        .keys()
        .filter(|k| k.contains("mask") || k.contains("protection"))
        .map(String::as_str)
        .collect();
    assert_eq!(mask_keys, ["mask_convention", "masks"]);
}

#[test]
fn morph_brush_uses_source_influence_and_preserves_protected_endpoint() {
    let mut doc = Document::new(source(vec![0.0, 1.0, 0.5]));
    doc.store_morph_target().unwrap();
    doc.begin_stroke();
    for point in &mut doc.active_mesh_mut().unwrap().positions {
        *point += Vec3::new(0.0, 0.0, 0.1);
    }
    doc.end_stroke();
    let before = doc.active_mesh().unwrap().snapshot();
    doc.begin_stroke();
    let result = doc
        .apply_morph_brush_with_result(
            Vec3::new(1.0, 0.0, 0.1),
            Vec3::Z,
            &settings(BrushKind::Morph),
        )
        .unwrap();
    doc.end_stroke();
    assert!(result.changed_vertices > 0);
    assert_eq!(doc.active_mesh().unwrap().positions[0], before.positions[0]);
    assert!(doc.active_mesh().unwrap().positions[1].z < before.positions[1].z);
    assert_eq!(bits(doc.active_mesh().unwrap()), bits(&before));
    assert_eq!(doc.active_mesh().unwrap().mask_convention, U);
    assert!(doc.undo());
    exact(doc.active_mesh().unwrap(), &before);
}

#[test]
fn explicit_invert_rounding_matches_ledger_and_undo_restores_original_words() {
    let mut doc = source_document();
    let original = bits(doc.active_mesh().unwrap());
    doc.invert_mask();
    assert_eq!(
        bits(doc.active_mesh().unwrap()),
        [0x3f800000, 0x3f800000, 0x3f000000]
    );
    doc.invert_mask();
    assert_eq!(bits(doc.active_mesh().unwrap()), [0, 0, 0x3f000000]);
    assert!(doc.undo());
    assert!(doc.undo());
    assert_eq!(bits(doc.active_mesh().unwrap()), original);
}
#[test]
fn signed_zero_only_source_edit_is_a_real_undoable_change() {
    let mut doc = source_document();
    let original = bits(doc.active_mesh().unwrap());
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().masks[1] = 0.0;
    doc.end_stroke();
    assert!(doc.can_undo());
    assert_eq!(bits(doc.active_mesh().unwrap())[1], 0);
    assert!(doc.undo());
    assert_eq!(bits(doc.active_mesh().unwrap()), original);
    assert!(doc.redo());
    assert_eq!(bits(doc.active_mesh().unwrap())[1], 0);
}
#[test]
fn retained_source_zero_residual_copies_changed_signed_zero_bits() {
    let mut doc = Document::new(source(vec![0.5, -0.0, 0.25]));
    doc.add_static_subdivision_level(&StaticSubdivisionSettings {
        linear_edges: false,
    })
    .unwrap();
    doc.select_level(0).unwrap();
    doc.begin_stroke();
    doc.active_mesh_mut().unwrap().masks[1] = 0.0;
    doc.end_stroke();
    doc.select_level(1).unwrap();
    assert_eq!(bits(doc.active_mesh().unwrap())[1], 0);
    assert_eq!(doc.active_mesh().unwrap().mask_convention, U);
    let restored = Document::from_json(&doc.to_json().unwrap()).unwrap();
    assert_eq!(bits(restored.active_mesh().unwrap())[1], 0);
}
