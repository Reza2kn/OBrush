#[path = "common/paint_fixture.rs"]
mod fixture;
use fixture::{bits, point, settings, sheet, surface};
use obrush_core::{
    Document, LockedStamp, LockedStampSample, Mesh, RasterAlpha, Vec3, DEFAULT_VERTEX_MATERIAL,
};

fn paint(mesh: &mut Mesh, s: &obrush_core::BrushSettings) -> obrush_core::BrushResult {
    mesh.apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, s)
}
fn material_settings() -> obrush_core::BrushSettings {
    let mut s = settings();
    s.paint_rgb = false;
    s.paint_roughness = true;
    s.roughness = 0.8;
    s.metallic = 0.6;
    s
}
fn sample(radius: f32) -> LockedStampSample {
    LockedStampSample {
        radius,
        alpha_tangent: Some(Vec3::X),
        ..Default::default()
    }
}
fn rich() -> Mesh {
    let mut m = sheet();
    m.materials = Some(
        (0..m.positions.len())
            .map(|i| [0.2 + (i % 5) as f32 / 16.0, 0.3 + (i % 3) as f32 / 16.0])
            .collect(),
    );
    m.surface = Some(surface(&m));
    m.validate().unwrap();
    m
}
fn redo_document() -> Document {
    let mut d = Document::new(sheet());
    d.begin_stroke();
    let mut s = settings();
    s.color = [0.8, 0.1, 0.2];
    paint(d.active_mesh_mut().unwrap(), &s);
    d.end_stroke();
    assert!(d.undo());
    assert!(d.can_redo());
    d
}

#[test]
fn legacy_settings_deserialize_exact_rgb_only_and_implicit_material_defaults() {
    let original = settings();
    let mut json = serde_json::to_value(&original).unwrap();
    for key in [
        "paint_rgb",
        "paint_roughness",
        "paint_metallic",
        "roughness",
        "metallic",
    ] {
        json.as_object_mut().unwrap().remove(key);
    }
    let restored: obrush_core::BrushSettings = serde_json::from_value(json).unwrap();
    assert!(restored.paint_rgb);
    assert!(!restored.paint_roughness);
    assert!(!restored.paint_metallic);
    assert_eq!(
        [restored.roughness, restored.metallic],
        DEFAULT_VERTEX_MATERIAL
    );
}

#[test]
fn every_channel_configuration_serializes_and_roundtrips() {
    for mask in 0..8 {
        let mut s = settings();
        s.paint_rgb = mask & 1 != 0;
        s.paint_roughness = mask & 2 != 0;
        s.paint_metallic = mask & 4 != 0;
        s.roughness = 0.3;
        s.metallic = 0.95;
        let restored: obrush_core::BrushSettings =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(s).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
    }
}

#[test]
fn rgb_only_final_bits_match_32_frozen_base_cases_in_every_mesh_channel() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/legacy_paint_bits.json")).unwrap();
    let measured = fixture::legacy_cases();
    assert_eq!(
        measured, expected,
        "All32 complete mesh/result bit ledgers must remain exact"
    );
    println!("Legacy RGB32cases*13dabs: differing complete bit ledgers=0");
}

#[test]
fn roughness_only_has_known_masked_pressure_hardness_response() {
    let mut m = rich();
    let original = m.snapshot();
    let p = point(4, 0);
    m.masks[p] = 0.5;
    let mut s = material_settings();
    s.pressure = 0.5;
    s.hardness = 0.5;
    let previous = m.materials.as_ref().unwrap()[p][0];
    let result = paint(&mut m, &s);
    assert!(result.changed_vertices > 0);
    let amount = 0.25_f64 * f64::from(s.strength) * 0.5 * 0.75 * 0.5;
    let expected = f64::from(previous) + (f64::from(s.roughness) - f64::from(previous)) * amount;
    let measured = m.materials.as_ref().unwrap()[p][0];
    let error = (f64::from(measured) - expected).abs();
    assert!(error < 2e-7);
    println!(
        "Roughness scalar: expected={expected}, measured={measured}, error={error}, gate=0.0000002"
    );
    assert_eq!(m.colors, original.colors);
    assert_eq!(m.positions, original.positions);
    assert_eq!(m.normals, original.normals);
    assert_eq!(m.triangles, original.triangles);
    assert_eq!(m.surface, original.surface);
    for (a, b) in m
        .materials
        .as_ref()
        .unwrap()
        .iter()
        .zip(original.materials.as_ref().unwrap())
    {
        assert_eq!(a[1].to_bits(), b[1].to_bits());
    }
}

#[test]
fn metallic_only_has_known_scalar_response_and_leaves_roughness_bits() {
    let mut m = rich();
    let original = m.snapshot();
    let mut s = material_settings();
    s.paint_roughness = false;
    s.paint_metallic = true;
    let p = point(0, 0);
    let old = m.materials.as_ref().unwrap()[p][1];
    assert!(paint(&mut m, &s).changed_vertices > 0);
    let expected =
        f64::from(old) + (f64::from(s.metallic) - f64::from(old)) * f64::from(s.strength) * 0.25;
    let measured = m.materials.as_ref().unwrap()[p][1];
    let error = (f64::from(measured) - expected).abs();
    assert!(error < 2e-7);
    println!(
        "Metallic scalar: expected={expected}, measured={measured}, error={error}, gate=0.0000002"
    );
    for (a, b) in m
        .materials
        .as_ref()
        .unwrap()
        .iter()
        .zip(original.materials.as_ref().unwrap())
    {
        assert_eq!(a[0].to_bits(), b[0].to_bits());
    }
    assert_eq!(m.colors, original.colors);
}

#[test]
fn combined_channels_count_each_changed_vertex_once() {
    let mut rgb = rich();
    let mut rough = rgb.snapshot();
    let mut metal = rgb.snapshot();
    let mut all = rgb.snapshot();
    let mut s = material_settings();
    s.paint_rgb = true;
    s.color = [0.9, 0.1, 0.6];
    let mut r = s.clone();
    r.paint_roughness = false;
    let rgb_count = paint(&mut rgb, &r).changed_vertices;
    r.paint_rgb = false;
    r.paint_roughness = true;
    let rough_count = paint(&mut rough, &r).changed_vertices;
    r.paint_roughness = false;
    r.paint_metallic = true;
    let metal_count = paint(&mut metal, &r).changed_vertices;
    s.paint_metallic = true;
    let all_count = paint(&mut all, &s).changed_vertices;
    assert!(all_count > 0);
    assert_eq!(all_count, rgb_count);
    assert_eq!(all_count, rough_count);
    assert_eq!(all_count, metal_count);
    assert_eq!(all.colors, rgb.colors);
    for i in 0..all.positions.len() {
        assert_eq!(
            all.materials.as_ref().unwrap()[i][0],
            rough.materials.as_ref().unwrap()[i][0]
        );
        assert_eq!(
            all.materials.as_ref().unwrap()[i][1],
            metal.materials.as_ref().unwrap()[i][1]
        );
    }
}

#[test]
fn disabled_scalar_channel_keeps_signed_zero_bits_when_other_material_edits() {
    let mut m = sheet();
    m.materials = Some(vec![[-0.0, 0.2]; m.positions.len()]);
    let mut s = material_settings();
    s.paint_roughness = false;
    s.paint_metallic = true;
    assert!(paint(&mut m, &s).changed_vertices > 0);
    assert!(m
        .materials
        .unwrap()
        .iter()
        .all(|p| p[0].to_bits() == (-0.0_f32).to_bits()));
}

#[test]
fn selected_equal_zero_scalar_keeps_its_bits_when_the_other_channel_edits() {
    let mut m = sheet();
    m.materials = Some(vec![[-0.0, 0.2]; m.positions.len()]);
    let mut s = material_settings();
    s.roughness = 0.0;
    s.paint_metallic = true;
    assert!(paint(&mut m, &s).changed_vertices > 0);
    assert!(m
        .materials
        .unwrap()
        .iter()
        .all(|p| p[0].to_bits() == (-0.0_f32).to_bits()));
}

#[test]
fn first_real_material_edit_allocates_defaults_only_for_untouched_points() {
    let mut m = sheet();
    let mut s = material_settings();
    s.radius = 0.2;
    let result = paint(&mut m, &s);
    assert!(result.changed_vertices > 0);
    let values = m.materials.unwrap();
    assert_eq!(values.len(), 17 * 17);
    assert!(values[point(0, 0)][0] > DEFAULT_VERTEX_MATERIAL[0]);
    assert_eq!(
        values[point(8, 8)].map(f32::to_bits),
        DEFAULT_VERTEX_MATERIAL.map(f32::to_bits)
    );
    assert!(values
        .iter()
        .all(|v| v[1].to_bits() == DEFAULT_VERTEX_MATERIAL[1].to_bits()));
}

#[test]
fn material_channel_allocation_limit_rejects_before_any_rgb_or_scalar_write() {
    let mut m = sheet();
    let existing_non_color_bytes = std::mem::size_of::<Mesh>()
        + m.positions.capacity() * std::mem::size_of::<Vec3>()
        + m.normals.capacity() * std::mem::size_of::<Vec3>()
        + m.triangles.capacity() * std::mem::size_of::<[u32; 3]>()
        + m.masks.capacity() * std::mem::size_of::<f32>();
    let limit = obrush_core::resource_limits::MAX_SCENE_STORAGE_BYTES;
    let required_material_bytes = m.positions.len() * std::mem::size_of::<[f32; 2]>();
    // A valid public mesh has enough reserved RGB capacity to leave less
    // room than the new material channel needs. Only 289 entries are filled.
    let capacity = (limit - existing_non_color_bytes - required_material_bytes / 2)
        / std::mem::size_of::<[f32; 3]>();
    let mut colors = Vec::with_capacity(capacity);
    colors.extend_from_slice(&m.colors);
    m.colors = colors;
    m.validate().unwrap();
    let before = bits(&m);
    let mut s = material_settings();
    s.paint_rgb = true;
    assert_eq!(paint(&mut m, &s).changed_vertices, 0);
    assert!(m.materials.is_none());
    assert_eq!(bits(&m), before);
    m.validate().unwrap();
}

#[test]
fn implicit_default_targets_never_allocate_or_create_history() {
    let mut d = redo_document();
    let before = bits(d.active_mesh().unwrap());
    let mut s = material_settings();
    s.roughness = DEFAULT_VERTEX_MATERIAL[0];
    s.paint_metallic = true;
    s.metallic = DEFAULT_VERTEX_MATERIAL[1];
    d.begin_stroke();
    assert_eq!(paint(d.active_mesh_mut().unwrap(), &s).changed_vertices, 0);
    d.end_stroke();
    assert_eq!(bits(d.active_mesh().unwrap()), before);
    assert!(d.active_mesh().unwrap().materials.is_none());
    assert!(d.can_redo());
}

#[test]
fn zero_strength_contact_black_alpha_full_masks_and_no_channels_do_not_allocate() {
    for scenario in 0..6 {
        let mut d = redo_document();
        let mut s = material_settings();
        match scenario {
            0 => s.strength = 0.0,
            1 => s.pressure = 0.0,
            2 => s.alpha = Some(RasterAlpha::new(2, 2, vec![0; 4]).unwrap()),
            3 => d.active_mesh_mut().unwrap().masks.fill(1.0),
            4 => s.paint_roughness = false,
            5 => s.radius = 0.0000011,
            _ => unreachable!(),
        };
        let before = bits(d.active_mesh().unwrap());
        if scenario == 5 {
            s.strength = f32::from_bits(1);
        }
        d.begin_stroke();
        assert_eq!(paint(d.active_mesh_mut().unwrap(), &s).changed_vertices, 0);
        d.end_stroke();
        assert_eq!(bits(d.active_mesh().unwrap()), before);
        assert!(d.active_mesh().unwrap().materials.is_none());
        assert!(d.can_redo());
    }
}

#[test]
fn missed_footprint_never_allocates() {
    let mut m = sheet();
    let before = bits(&m);
    let s = material_settings();
    let r = m.apply_brush_with_result(Vec3::new(5.0, 5.0, 0.0), Vec3::Z, Vec3::ZERO, &s);
    assert_eq!(r.changed_vertices, 0);
    assert_eq!(bits(&m), before);
}

#[test]
fn full_masks_protect_both_existing_material_and_rgb_exactly() {
    let mut m = rich();
    m.masks.fill(1.0);
    let before = bits(&m);
    let mut s = material_settings();
    s.paint_rgb = true;
    s.paint_metallic = true;
    assert_eq!(paint(&mut m, &s).changed_vertices, 0);
    assert_eq!(bits(&m), before);
}

#[test]
fn partial_masks_scale_both_material_targets_by_the_same_known_weight() {
    let mut m = sheet();
    let p = point(0, 0);
    m.masks[p] = 0.75;
    let mut s = material_settings();
    s.paint_metallic = true;
    paint(&mut m, &s);
    let actual = m.materials.as_ref().unwrap()[p];
    for channel in 0..2 {
        let target = [s.roughness, s.metallic][channel];
        let expected = f64::from(DEFAULT_VERTEX_MATERIAL[channel])
            + (f64::from(target) - f64::from(DEFAULT_VERTEX_MATERIAL[channel]))
                * 0.25
                * f64::from(s.strength)
                * 0.25;
        assert!((f64::from(actual[channel]) - expected).abs() < 2e-7);
    }
}

#[test]
fn grayscale_alpha_scales_rgb_and_materials_with_one_shared_influence() {
    let mut a = rich();
    let mut b = a.snapshot();
    let original = a.snapshot();
    let mut s = material_settings();
    s.paint_rgb = true;
    s.paint_metallic = true;
    paint(&mut a, &s);
    s.alpha = Some(RasterAlpha::new(1, 1, vec![128]).unwrap());
    paint(&mut b, &s);
    let p = point(0, 0);
    for channel in 0..2 {
        let old = original.materials.as_ref().unwrap()[p][channel];
        let expected = f64::from(old)
            + (f64::from(a.materials.as_ref().unwrap()[p][channel]) - f64::from(old)) * 128.0
                / 255.0;
        assert!((f64::from(b.materials.as_ref().unwrap()[p][channel]) - expected).abs() < 2e-7);
    }
    for channel in 0..3 {
        let old = original.colors[p][channel];
        let expected =
            f64::from(old) + (f64::from(a.colors[p][channel]) - f64::from(old)) * 128.0 / 255.0;
        assert!((f64::from(b.colors[p][channel]) - expected).abs() < 2e-7);
    }
}

#[test]
fn culling_can_include_back_faces_for_material_only_paint() {
    let original = sheet();
    let mut front = original.snapshot();
    for n in &mut front.normals {
        *n = -Vec3::Z;
    }
    let mut back = front.snapshot();
    let mut s = material_settings();
    assert_eq!(paint(&mut front, &s).changed_vertices, 0);
    assert!(front.materials.is_none());
    s.backface_culling = false;
    assert!(paint(&mut back, &s).changed_vertices > 0);
}

#[test]
fn offset_symmetry_keeps_material_pairs_exact_and_seam_not_doubled() {
    let mut m = sheet();
    for p in &mut m.positions {
        p.x += 1.0;
    }
    let mut s = material_settings();
    s.paint_metallic = true;
    s.symmetry_x = true;
    s.symmetry_offset = 1.0;
    let r = m.apply_brush_with_result(Vec3::new(1.0, 0.0, 0.0), Vec3::Z, Vec3::ZERO, &s);
    assert!(r.changed_vertices > 0);
    for y in -8..=8 {
        for x in -8..=8 {
            assert_eq!(
                m.materials.as_ref().unwrap()[point(x, y)].map(f32::to_bits),
                m.materials.as_ref().unwrap()[point(-x, y)].map(f32::to_bits)
            );
        }
    }
    let expected = f64::from(DEFAULT_VERTEX_MATERIAL[0])
        + (f64::from(s.roughness) - f64::from(DEFAULT_VERTEX_MATERIAL[0]))
            * f64::from(s.strength)
            * 0.25;
    assert!((f64::from(m.materials.as_ref().unwrap()[point(0, 0)][0]) - expected).abs() < 2e-7);
}

#[test]
fn material_inverse_erases_to_implicit_defaults_without_touching_rgb() {
    let mut m = rich();
    let old = m.snapshot();
    let mut s = material_settings();
    s.invert = true;
    s.paint_metallic = true;
    paint(&mut m, &s);
    let p = point(0, 0);
    for (channel, &default) in DEFAULT_VERTEX_MATERIAL.iter().enumerate() {
        let before = old.materials.as_ref().unwrap()[p][channel];
        let after = m.materials.as_ref().unwrap()[p][channel];
        let expected = f64::from(before)
            + (f64::from(default) - f64::from(before)) * f64::from(s.strength) * 0.25;
        assert!((f64::from(after) - expected).abs() < 2e-7);
    }
    assert_eq!(m.colors, old.colors);
}

#[test]
fn invalid_scalar_targets_are_atomic_even_while_disabled() {
    for bad in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        for channel in 0..2 {
            let mut m = sheet();
            let before = bits(&m);
            let mut s = settings();
            if channel == 0 {
                s.roughness = bad
            } else {
                s.metallic = bad
            };
            assert_eq!(paint(&mut m, &s).changed_vertices, 0);
            assert_eq!(bits(&m), before);
            assert!(LockedStamp::prepare(&m, Vec3::ZERO, Vec3::Z, &s).is_err());
        }
    }
}

#[test]
fn malformed_existing_material_length_and_values_reject_before_rgb_changes() {
    for scenario in 0..3 {
        let mut m = sheet();
        let mut values = vec![DEFAULT_VERTEX_MATERIAL; m.positions.len()];
        match scenario {
            0 => {
                values.pop();
            }
            1 => values[point(0, 0)][0] = f32::NAN,
            2 => values[point(8, 8)][1] = 1.1,
            _ => unreachable!(),
        };
        m.materials = Some(values);
        let before = bits(&m);
        let mut s = material_settings();
        s.paint_rgb = true;
        assert_eq!(paint(&mut m, &s).changed_vertices, 0);
        assert_eq!(bits(&m), before);
    }
}

#[test]
fn material_only_stroke_has_one_complete_exact_undo_and_redo() {
    let mut d = Document::new(rich());
    let original = d.to_json().unwrap();
    let s = material_settings();
    d.begin_stroke();
    for _ in 0..8 {
        assert!(paint(d.active_mesh_mut().unwrap(), &s).changed_vertices > 0);
    }
    d.end_stroke();
    let painted = d.to_json().unwrap();
    assert_ne!(painted, original);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), original);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), painted);
}

#[test]
fn lazy_material_presence_is_one_history_entry_and_undo_returns_none() {
    let mut d = Document::new(sheet());
    let original = d.to_json().unwrap();
    d.begin_stroke();
    assert!(paint(d.active_mesh_mut().unwrap(), &material_settings()).changed_vertices > 0);
    d.end_stroke();
    let painted = d.to_json().unwrap();
    assert!(d.active_mesh().unwrap().materials.is_some());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), original);
    assert!(d.active_mesh().unwrap().materials.is_none());
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), painted);
}

#[test]
fn material_only_paint_is_history_aware_with_geometry_layer_recording() {
    let mut d = Document::new(rich());
    let layer = d.add_layer("shape").unwrap();
    d.set_recording_layer(Some(layer)).unwrap();
    let before = d.to_json().unwrap();
    d.begin_stroke();
    assert!(paint(d.active_mesh_mut().unwrap(), &material_settings()).changed_vertices > 0);
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert_ne!(before, after);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn material_only_scene_paint_preserves_other_parts_and_complete_history() {
    let mut d = Document::new(rich());
    d.add_subtool(sheet(), "other").unwrap();
    d.select_subtool(1).unwrap();
    let before = d.to_json().unwrap();
    d.begin_stroke();
    assert!(paint(d.active_mesh_mut().unwrap(), &material_settings()).changed_vertices > 0);
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn materialized_retained_levels_keep_material_stroke_history() {
    let mut d = Document::new(rich());
    d.add_subdivision_level().unwrap();
    let before = d.to_json().unwrap();
    d.begin_stroke();
    assert!(paint(d.active_mesh_mut().unwrap(), &material_settings()).changed_vertices > 0);
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    d.select_level(0).unwrap();
    d.active_mesh().unwrap().validate().unwrap();
}

#[test]
fn generic_retained_presence_edit_still_rolls_back_exactly() {
    let mut d = Document::new(sheet());
    d.add_subdivision_level().unwrap();
    let before = d.to_json().unwrap();
    d.begin_stroke();
    assert!(paint(d.active_mesh_mut().unwrap(), &material_settings()).changed_vertices > 0);
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.take_layer_error().is_some());
}

#[test]
fn locked_material_only_preview_counts_real_changes_and_identical_samples_are_empty() {
    let mut m = rich();
    let s = material_settings();
    let mut stamp = LockedStamp::prepare(&m, Vec3::ZERO, Vec3::Z, &s).unwrap();
    let first = stamp.apply(&mut m, sample(0.5)).unwrap();
    assert!(first.changed());
    assert!(first.changed_vertices > 0);
    assert_eq!(first.changed_vertices, first.dab_changed_vertices);
    let before = bits(&m);
    let repeated = stamp.apply(&mut m, sample(0.5)).unwrap();
    assert!(!repeated.changed());
    assert_eq!(bits(&m), before);
}

#[test]
fn locked_lazy_material_zero_and_cancel_restore_exact_none_and_redo() {
    for cancel in [false, true] {
        let mut d = redo_document();
        let before = d.to_json().unwrap();
        let s = material_settings();
        d.begin_stroke();
        let mut stamp =
            LockedStamp::prepare(d.active_mesh().unwrap(), Vec3::ZERO, Vec3::Z, &s).unwrap();
        assert!(stamp
            .apply(d.active_mesh_mut().unwrap(), sample(0.5))
            .unwrap()
            .changed());
        assert!(d.active_mesh().unwrap().materials.is_some());
        let report = if cancel {
            stamp.cancel(d.active_mesh_mut().unwrap()).unwrap()
        } else {
            stamp
                .apply(d.active_mesh_mut().unwrap(), sample(0.0))
                .unwrap()
        };
        assert!(report.changed());
        assert!(report.at_origin);
        assert!(d.active_mesh().unwrap().materials.is_none());
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.can_redo());
    }
}

#[test]
fn locked_material_gesture_groups_radius_previews_into_one_complete_history_step() {
    let mut d = Document::new(rich());
    let before = d.to_json().unwrap();
    let s = material_settings();
    d.begin_stroke();
    let mut stamp =
        LockedStamp::prepare(d.active_mesh().unwrap(), Vec3::ZERO, Vec3::Z, &s).unwrap();
    for radius in [0.2, 0.4, 0.3, 0.5] {
        stamp
            .apply(d.active_mesh_mut().unwrap(), sample(radius))
            .unwrap();
    }
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn locked_external_material_changes_and_representation_changes_are_stale() {
    for scenario in 0..3 {
        let mut m = rich();
        let s = material_settings();
        let mut stamp = LockedStamp::prepare(&m, Vec3::ZERO, Vec3::Z, &s).unwrap();
        match scenario {
            0 => m.materials.as_mut().unwrap()[point(0, 0)][0] += 0.01,
            1 => {
                m.materials.as_mut().unwrap().pop();
            }
            2 => m.materials = None,
            _ => unreachable!(),
        };
        let before = bits(&m);
        assert!(stamp.apply(&mut m, sample(0.5)).is_err());
        assert!(stamp.cancel(&mut m).is_err());
        assert_eq!(bits(&m), before);
    }
}

#[test]
fn legacy_rgb_paint_keeps_existing_materials_surface_and_geometry_unchanged() {
    let mut m = rich();
    let before = m.snapshot();
    assert!(paint(&mut m, &settings()).changed_vertices > 0);
    assert_eq!(m.materials, before.materials);
    assert_eq!(m.surface, before.surface);
    assert_eq!(m.positions, before.positions);
    assert_eq!(m.normals, before.normals);
    assert_eq!(m.masks, before.masks);
}
