use obrush_core::{
    paint_tablet_local_radius_squared, BrushKind, BrushSettings, CornerUv, Document,
    MaskConvention, MaskConversionPolicy, Mesh, PaintResponseAlphaFrame, PaintResponseOperation,
    PaintResponseOptions, PaintResponsePass, PaintResponseRequest, PaintTabletIntensity,
    RasterAlpha, SurfaceTopology, Vec3, DEFAULT_VERTEX_MATERIAL, TRIANGLE_CORNER,
};
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/independent-paint-response.json")).unwrap()
}
fn vec3(v: &Value) -> Vec3 {
    Vec3::new(
        v[0].as_f64().unwrap() as f32,
        v[1].as_f64().unwrap() as f32,
        v[2].as_f64().unwrap() as f32,
    )
}
fn words3(v: &Value) -> [f32; 3] {
    std::array::from_fn(|c| f32::from_bits(v[c].as_u64().unwrap() as u32))
}
fn words2(v: &Value) -> [f32; 2] {
    std::array::from_fn(|c| f32::from_bits(v[c].as_u64().unwrap() as u32))
}
fn mesh(explicit: bool) -> Mesh {
    let f = fixture();
    mesh_from_fixture(&f, explicit)
}
fn mesh_from_fixture(f: &Value, explicit: bool) -> Mesh {
    let mut m = Mesh::cube();
    m.positions = f["positions"]
        .as_array()
        .unwrap()
        .iter()
        .map(vec3)
        .collect();
    m.normals = f["normals"].as_array().unwrap().iter().map(vec3).collect();
    m.triangles = f["triangles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| std::array::from_fn(|c| v[c].as_u64().unwrap() as u32))
        .collect();
    m.colors = f["initial_color_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(words3)
        .collect();
    m.masks = f["mask_bits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| f32::from_bits(v.as_u64().unwrap() as u32))
        .collect();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.materials = explicit.then(|| {
        f["initial_material_bits"]
            .as_array()
            .unwrap()
            .iter()
            .map(words2)
            .collect()
    });
    m.surface = None;
    m.validate().unwrap();
    m
}
fn manifold_unmasked() -> Mesh {
    let mut m = white_unmasked();
    // The extra boundary witness face in the ordinary stencil fixture creates
    // a third face on edge4–8. Retained smooth levels use the separate9-row grid.
    m.triangles.pop();
    m.positions.pop();
    m.normals.pop();
    m.masks.pop();
    m.colors.pop();
    m.validate().unwrap();
    m
}
fn identity(center_x: f32) -> PaintResponseAlphaFrame {
    PaintResponseAlphaFrame {
        look_at: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -center_x, 0.0, 0.0, 1.0,
        ],
    }
}
fn pass(center_x: f32, mirrored_x: bool, alpha: bool) -> PaintResponsePass {
    PaintResponsePass {
        center: [f64::from(center_x), 0.0, 0.0],
        local_radius_squared: 1.0,
        eye_direction: [0.0, 0.0, -1.0],
        mirrored_x,
        alpha_frame: alpha.then(|| identity(center_x)),
    }
}
fn request() -> PaintResponseRequest {
    PaintResponseRequest {
        kind: BrushKind::Paint,
        operation: PaintResponseOperation::OrdinaryDab,
        in_contact: true,
        options: PaintResponseOptions {
            intensity: 1.0,
            hardness: 1.0,
            color: [1.0; 3],
            write_roughness: false,
            write_metallic: false,
            ..Default::default()
        },
        first: pass(0.0, false, false),
        mirrored: None,
    }
}
fn white_unmasked() -> Mesh {
    let mut m = mesh(false);
    m.masks.fill(1.0);
    m.colors.fill([0.0; 3]);
    m
}
fn alpha() -> RasterAlpha {
    RasterAlpha::new(3, 2, vec![0, 64, 255, 32, 128, 192]).unwrap()
}
fn bits(m: &Mesh) -> Value {
    json!({"p":m.positions.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),
        "n":m.normals.iter().map(|p|p.to_array().map(f32::to_bits)).collect::<Vec<_>>(),"t":m.triangles,
        "mask":m.masks.iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"convention":m.mask_convention,
        "rgb":m.colors.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>(),
        "material":m.materials.as_ref().map(|m|m.iter().map(|p|p.map(f32::to_bits)).collect::<Vec<_>>()),"surface":m.surface})
}
fn geometric_bits(m: &Mesh) -> Value {
    let mut b = bits(m);
    b.as_object_mut().unwrap().remove("rgb");
    b.as_object_mut().unwrap().remove("material");
    b
}
fn assert_rejected(m: &mut Mesh, r: &PaintResponseRequest) {
    let before = bits(m);
    assert!(m.apply_paint_response(r).is_err());
    assert_eq!(bits(m), before);
}

#[test]
fn independent_f64_golden320_matches_channels_pressure_hardness_alpha_and_ordered_symmetry() {
    let f = fixture();
    assert_eq!(f["cases"].as_array().unwrap().len(), 320);
    let mut max_channel = 0.0_f64;
    let mut max_weight = 0.0_f64;
    for case in f["cases"].as_array().unwrap() {
        let mut m = mesh_from_fixture(&f, case["materials_explicit"].as_bool().unwrap());
        let before_geometry = geometric_bits(&m);
        let before_channels = bits(&m);
        let flags = case["write_flags"].as_u64().unwrap();
        let has_alpha = case["alpha"].as_bool().unwrap();
        let symmetry = case["symmetry"].as_bool().unwrap();
        let mut r = request();
        r.options = PaintResponseOptions {
            intensity: case["intensity"].as_f64().unwrap(),
            hardness: case["hardness"].as_f64().unwrap(),
            color: words3(&f["target_color_bits"]),
            roughness: f32::from_bits(f["target_material_bits"][0].as_u64().unwrap() as u32),
            metallic: f32::from_bits(f["target_material_bits"][1].as_u64().unwrap() as u32),
            write_rgb: flags & 1 != 0,
            write_roughness: flags & 2 != 0,
            write_metallic: flags & 4 != 0,
            culling: case["culling"].as_bool().unwrap(),
            tablet: PaintTabletIntensity {
                pressure: case["pressure"].as_f64().unwrap(),
                factor: case["pressure_factor"].as_f64().unwrap(),
            },
            alpha: has_alpha.then(alpha),
        };
        r.first = pass(if symmetry { -0.125 } else { 0.0 }, false, has_alpha);
        r.mirrored = symmetry.then(|| pass(0.125, true, has_alpha));
        let report = m.apply_paint_response(&r).unwrap();
        m.validate().unwrap();
        assert_eq!(geometric_bits(&m), before_geometry, "{}", case["id"]);
        for (row, expected) in m
            .colors
            .iter()
            .zip(case["expected_color_bits"].as_array().unwrap())
        {
            for (a, b) in row.iter().zip(words3(expected)) {
                max_channel = max_channel.max((f64::from(*a) - f64::from(b)).abs());
            }
        }
        match (&m.materials, case["expected_material_bits"].as_array()) {
            (None, None) => assert!(!report.materialized),
            (Some(actual), Some(expected)) => {
                for (row, expected) in actual.iter().zip(expected) {
                    for (a, b) in row.iter().zip(words2(expected)) {
                        max_channel = max_channel.max((f64::from(*a) - f64::from(b)).abs());
                    }
                }
            }
            _ => panic!("{}: unexpected material allocation", case["id"]),
        }
        if flags & 1 == 0 {
            assert_eq!(
                bits(&m)["rgb"],
                before_channels["rgb"],
                "disabled RGB {}",
                case["id"]
            );
        }
        if let Some(actual) = &m.materials {
            let original = mesh_from_fixture(&f, case["materials_explicit"].as_bool().unwrap());
            for (vertex, row) in actual.iter().enumerate() {
                let old = original
                    .materials
                    .as_ref()
                    .map_or(DEFAULT_VERTEX_MATERIAL, |v| v[vertex]);
                for (channel, flag) in [2, 4].into_iter().enumerate() {
                    if flags & flag == 0 {
                        assert_eq!(
                            row[channel].to_bits(),
                            old[channel].to_bits(),
                            "disabled scalar {}",
                            case["id"]
                        );
                    }
                }
            }
        }
        if flags != 0 {
            max_weight = max_weight.max(
                (report.maximum_weight - case["source_maximum_weight"].as_f64().unwrap()).abs(),
            );
        }
        assert!(
            max_channel <= 2e-7,
            "{} channel max {max_channel}",
            case["id"]
        );
        assert!(
            max_weight <= 2e-12,
            "{} weight max {max_weight}",
            case["id"]
        );
    }
    println!("Independent Paint320: channel max {max_channel}, gate2e-7; weight max {max_weight}, gate2e-12");
}

#[test]
fn request_defaults_are_source_values_and_do_not_replace_legacy_default_channel_flags() {
    let s = PaintResponseOptions::default();
    assert_eq!([s.intensity, s.hardness], [0.75, 0.75]);
    assert_eq!(s.color, [1.0, 0.766, 0.336]);
    assert_eq!([s.roughness, s.metallic], [0.3, 0.95]);
    assert!(s.write_rgb && s.write_roughness && s.write_metallic);
    assert!(!s.culling);
    assert_eq!(
        s.tablet,
        PaintTabletIntensity {
            pressure: 0.5,
            factor: 0.0
        }
    );
    let legacy = BrushSettings::default();
    assert!(legacy.paint_rgb);
    assert!(!legacy.paint_roughness && !legacy.paint_metallic);
    assert_eq!([legacy.roughness, legacy.metallic], DEFAULT_VERTEX_MATERIAL);
}

#[test]
fn center_blend_uses_full_source_intensity_without_legacy_quarter_multiplier() {
    let mut m = white_unmasked();
    let mut r = request();
    r.options.intensity = 0.75;
    m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[4], [0.75; 3]);
    assert!(m.materials.is_none());
}
#[test]
fn hardness_zero_and_one_have_distinct_known_radial_amounts() {
    let mut m = white_unmasked();
    let mut r = request();
    r.options.intensity = 0.75;
    r.options.hardness = 0.0;
    m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[3], [0.29296875; 3]);
    let mut hard = white_unmasked();
    r.options.hardness = 1.0;
    hard.apply_paint_response(&r).unwrap();
    assert_eq!(hard.colors[3], [0.75; 3]);
}
#[test]
fn raw_source_pressure_zero_can_be_active_while_contact_eligibility_still_suppresses_everything() {
    let mut m = white_unmasked();
    let mut r = request();
    r.options.intensity = 0.25;
    r.options.tablet = PaintTabletIntensity {
        pressure: 0.0,
        factor: 0.73,
    };
    m.apply_paint_response(&r).unwrap();
    assert!((f64::from(m.colors[4][0]) - 0.0675).abs() < 2e-7);
    let mut no_contact = white_unmasked();
    let before = bits(&no_contact);
    r.in_contact = false;
    let report = no_contact.apply_paint_response(&r).unwrap();
    assert_eq!(report.changed_vertices, 0);
    assert_eq!(bits(&no_contact), before);
}
#[test]
fn source_tablet_intensity_and_radius_helpers_have_independent_endpoint_oracles() {
    for (pressure, intensity_multiplier, radius_squared) in
        [(0.0, 0.27, 0.25), (0.5, 1.0, 4.0), (1.0, 1.73, 12.25)]
    {
        let value = PaintTabletIntensity {
            pressure,
            factor: 0.73,
        }
        .multiplier()
        .unwrap();
        assert!((value - intensity_multiplier).abs() < 2e-14);
        assert_eq!(
            paint_tablet_local_radius_squared(4.0, pressure, 0.75).unwrap(),
            radius_squared
        );
    }
    assert_eq!(
        paint_tablet_local_radius_squared(4.0, 0.0, 1.0).unwrap(),
        0.0
    );
}
#[test]
fn prepared_radius_is_used_once_instead_of_reapplying_tablet_radius() {
    let mut m = white_unmasked();
    let mut r = request();
    r.first.local_radius_squared = paint_tablet_local_radius_squared(4.0, 0.0, 0.75).unwrap();
    r.options.tablet = PaintTabletIntensity {
        pressure: 0.0,
        factor: 0.0,
    };
    m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[3], [1.0; 3]);
    assert_eq!(m.colors[0], [0.0; 3]);
}
#[test]
fn source_culling_uses_away_eye_direction_and_includes_exact_grazing_normals() {
    let mut m = white_unmasked();
    let mut r = request();
    r.options.culling = true;
    m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[0], [1.0; 3]);
    assert_eq!(m.colors[1], [0.0; 3]);
    assert_eq!(m.colors[2], [1.0; 3]);
    assert_eq!(m.colors[6], [1.0; 3]);
}
#[test]
fn disabled_culling_permits_backfacing_vertices() {
    let mut m = white_unmasked();
    let r = request();
    m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[1], [1.0; 3]);
}
#[test]
fn exact_sphere_boundary_is_excluded_even_for_hardness_one() {
    let mut m = white_unmasked();
    let r = request();
    let report = m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[9], [0.0; 3]);
    assert_eq!(report.first_selected_vertices, 9);
}
#[test]
fn nearest_alpha_center_uses_one_texel_instead_of_bilinear_luminance() {
    let mut m = white_unmasked();
    let mut r = request();
    r.options.alpha = Some(alpha());
    r.first.alpha_frame = Some(identity(0.0));
    m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[4], [128.0 / 255.0; 3]);
}
#[test]
fn rectangular_alpha_aspect_clips_inside_the_spherical_footprint() {
    let mut m = white_unmasked();
    let mut r = request();
    r.options.alpha = Some(RasterAlpha::new(3, 2, vec![255; 6]).unwrap());
    r.first.alpha_frame = Some(identity(0.0));
    r.first.local_radius_squared = 0.5;
    m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[1], [0.0; 3]);
    assert_eq!(m.colors[4], [1.0; 3]);
}
#[test]
fn mirrored_alpha_lookup_flips_x_without_guessing_geometry_reflections() {
    let mut primary = white_unmasked();
    let mut mirrored = white_unmasked();
    let mut r = request();
    r.options.alpha = Some(alpha());
    r.first.alpha_frame = Some(identity(0.0));
    primary.apply_paint_response(&r).unwrap();
    r.first.mirrored_x = true;
    mirrored.apply_paint_response(&r).unwrap();
    assert_eq!(primary.colors[3], [192.0 / 255.0; 3]);
    assert_eq!(mirrored.colors[3], [32.0 / 255.0; 3]);
}
#[test]
fn overlapping_mirror_passes_blend_twice_and_report_unique_changed_rows() {
    let mut m = white_unmasked();
    let mut r = request();
    r.options.intensity = 0.5;
    r.mirrored = Some(pass(0.0, true, false));
    let result = m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[4], [0.75; 3]);
    assert_eq!(result.changed_vertices, 9);
    assert_eq!(result.mirrored_selected_vertices, 9);
}

#[test]
fn every_other_tool_and_nonordinary_operation_rejects_atomically() {
    let mut m = white_unmasked();
    for kind in BrushKind::ALL {
        if kind != BrushKind::Paint {
            let mut r = request();
            r.kind = kind;
            assert_rejected(&mut m, &r);
        }
    }
    for operation in [
        PaintResponseOperation::LockedPreview,
        PaintResponseOperation::LockedAccumulation,
        PaintResponseOperation::Fill,
        PaintResponseOperation::Picker,
        PaintResponseOperation::DynamicTopology,
    ] {
        let mut r = request();
        r.operation = operation;
        assert_rejected(&mut m, &r);
    }
}

#[test]
fn legacy_protected_mask_convention_requires_explicit_conversion() {
    let mut m = white_unmasked();
    m.mask_convention = MaskConvention::ProtectedF32V1;
    assert_rejected(&mut m, &request());
    m = m
        .converted_mask_convention(
            MaskConvention::UnprotectedF32V1,
            MaskConversionPolicy::RequireRoundtripExact,
        )
        .unwrap()
        .0;
    assert_eq!(
        m.apply_paint_response(&request()).unwrap().changed_vertices,
        0
    );
}

#[test]
fn all_no_effect_paths_keep_storage_and_redo_exact() {
    for scenario in 0..6 {
        let mut d = Document::new(white_unmasked());
        let mut seed = request();
        seed.options.intensity = 0.25;
        d.begin_stroke();
        d.require_active_mesh_mut()
            .unwrap()
            .apply_paint_response(&seed)
            .unwrap();
        d.end_stroke();
        assert!(d.undo());
        assert!(d.can_redo());
        let mut r = request();
        r.options.write_roughness = true;
        r.options.write_metallic = true;
        match scenario {
            0 => {
                r.options.write_rgb = false;
                r.options.write_roughness = false;
                r.options.write_metallic = false;
            }
            1 => r.in_contact = false,
            2 => r.options.intensity = 0.0,
            3 => d.require_active_mesh_mut().unwrap().masks.fill(0.0),
            4 => {
                r.options.alpha = Some(RasterAlpha::new(2, 2, vec![0; 4]).unwrap());
                r.first.alpha_frame = Some(identity(0.0));
            }
            5 => r.first.local_radius_squared = 0.0,
            _ => unreachable!(),
        }
        let before = bits(d.require_active_mesh().unwrap());
        let history = d.to_json().unwrap();
        d.begin_material_paint_stroke();
        let report = d
            .require_active_mesh_mut()
            .unwrap()
            .apply_paint_response(&r)
            .unwrap();
        assert_eq!(report.changed_vertices, 0, "scenario {scenario}");
        assert_eq!(report.brush_result().changed_vertices, 0);
        assert_eq!(report.brush_result().displacement_scale, 0.0);
        assert!(!report.materialized);
        d.end_stroke();
        assert!(d.can_redo(), "scenario {scenario}");
        assert_eq!(bits(d.require_active_mesh().unwrap()), before);
        assert_eq!(d.to_json().unwrap(), history);
        assert!(d.require_active_mesh().unwrap().materials.is_none());
    }
}

#[test]
fn implicit_material_target_defaults_do_not_materialize_for_nonzero_weight() {
    let mut m = white_unmasked();
    let mut r = request();
    r.options.write_rgb = false;
    r.options.write_roughness = true;
    r.options.write_metallic = true;
    [r.options.roughness, r.options.metallic] = DEFAULT_VERTEX_MATERIAL;
    let before = bits(&m);
    let report = m.apply_paint_response(&r).unwrap();
    assert_eq!(report.maximum_weight, 1.0);
    assert_eq!(report.changed_vertices, 0);
    assert!(!report.materialized);
    assert_eq!(bits(&m), before);
}

#[test]
fn signed_zero_noops_and_smallest_raw_unprotected_value_remain_exact() {
    let mut m = white_unmasked();
    m.colors.fill([-0.0, 0.0, -0.0]);
    m.masks.fill(-0.0);
    let before = bits(&m);
    assert_eq!(
        m.apply_paint_response(&request()).unwrap().changed_vertices,
        0
    );
    assert_eq!(bits(&m), before);
    m.masks[4] = f32::from_bits(1);
    let report = m.apply_paint_response(&request()).unwrap();
    assert_eq!(report.changed_vertices, 1);
    assert_eq!(m.colors[4].map(f32::to_bits), [1; 3]);
    assert_eq!(m.masks[4].to_bits(), 1);
    let mut r = request();
    r.options.color = [0.0; 3];
    m.colors[4] = [-0.0; 3];
    let before = bits(&m);
    assert_eq!(m.apply_paint_response(&r).unwrap().changed_vertices, 0);
    assert_eq!(bits(&m), before);
}

#[test]
fn only_selected_material_scalar_changes_and_rgb_is_bit_preserved() {
    for roughness in [true, false] {
        let mut m = mesh(true);
        m.masks.fill(1.0);
        let before = bits(&m);
        let original = m.materials.clone().unwrap();
        let mut r = request();
        r.options.write_rgb = false;
        r.options.write_roughness = roughness;
        r.options.write_metallic = !roughness;
        r.options.roughness = 0.875;
        r.options.metallic = 0.75;
        let report = m.apply_paint_response(&r).unwrap();
        assert_eq!(report.changed_rgb_vertices, 0);
        assert!(report.changed_material_vertices > 0);
        assert!(!report.materialized);
        assert_eq!(bits(&m)["rgb"], before["rgb"]);
        for (i, row) in m.materials.as_ref().unwrap().iter().enumerate() {
            let disabled = usize::from(roughness);
            assert_eq!(row[disabled].to_bits(), original[i][disabled].to_bits());
        }
    }
}

#[test]
fn exact_x_boundary_preserves_source_flat_index_row_alias() {
    let mut m = white_unmasked();
    let mut r = request();
    r.options.alpha = Some(RasterAlpha::new(2, 2, vec![0, 64, 128, 255]).unwrap());
    r.first.local_radius_squared = 1.9999999999999996;
    let mut frame = identity(0.0);
    frame.look_at[12] = -1.0;
    frame.look_at[13] = 1.0;
    r.first.alpha_frame = Some(frame);
    m.apply_paint_response(&r).unwrap();
    // xn=-1, yn=+1 gives column=width, row=0, index=2.
    assert_eq!(m.colors[4], [128.0 / 255.0; 3]);
}

#[test]
fn undefined_y_boundary_rejects_after_earlier_valid_proposals_without_any_write() {
    let mut m = white_unmasked();
    m.positions[8] = Vec3::new(0.0, -1.0, 0.0);
    m.validate().unwrap();
    let mut r = request();
    r.options.write_roughness = true;
    r.options.alpha = Some(RasterAlpha::new(2, 2, vec![255; 4]).unwrap());
    r.first.local_radius_squared = 1.9999999999999996;
    r.first.alpha_frame = Some(identity(0.0));
    assert_rejected(&mut m, &r);
    assert!(m.materials.is_none());
}

#[test]
fn high_pressure_valid_extrapolation_is_not_silently_clamped() {
    let mut m = white_unmasked();
    m.colors.fill([0.25; 3]);
    let mut r = request();
    r.options.color = [0.5; 3];
    r.options.tablet = PaintTabletIntensity {
        pressure: 1.0,
        factor: 1.0,
    };
    let report = m.apply_paint_response(&r).unwrap();
    assert_eq!(report.maximum_weight, 2.0);
    assert_eq!(m.colors[4], [0.75; 3]);
}

#[test]
fn extrapolation_outside_normalized_channels_rejects_entire_mesh_atomically() {
    let mut m = white_unmasked();
    m.colors.fill([0.5; 3]);
    m.colors[8] = [0.0; 3];
    let mut r = request();
    r.options.tablet = PaintTabletIntensity {
        pressure: 1.0,
        factor: 1.0,
    };
    r.options.color = [0.75; 3];
    r.options.write_roughness = true;
    assert_rejected(&mut m, &r);
}

#[test]
fn invalid_normalized_inputs_are_rejected_even_when_the_channel_is_disabled() {
    for input in [f64::NAN, f64::INFINITY, -0.01, 1.01] {
        for field in 0..4 {
            let mut r = request();
            match field {
                0 => r.options.intensity = input,
                1 => r.options.hardness = input,
                2 => r.options.tablet.pressure = input,
                3 => r.options.tablet.factor = input,
                _ => unreachable!(),
            }
            assert_rejected(&mut white_unmasked(), &r);
        }
    }
    for input in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
        for field in 0..3 {
            let mut r = request();
            match field {
                0 => r.options.color[1] = input,
                1 => r.options.roughness = input,
                2 => r.options.metallic = input,
                _ => unreachable!(),
            }
            assert_rejected(&mut white_unmasked(), &r);
        }
    }
}

#[test]
fn invalid_prepared_pick_frame_radius_and_order_do_not_get_normalized_or_ignored() {
    for scenario in 0..12 {
        let mut r = request();
        match scenario {
            0 => r.first.eye_direction = [0.0; 3],
            1 => r.first.eye_direction[0] = f64::NAN,
            2 => r.first.center[2] = f64::INFINITY,
            3 => r.first.center[0] = 1_000_001.0,
            4 => r.first.local_radius_squared = -0.1,
            5 => r.first.local_radius_squared = f64::INFINITY,
            6 => r.first.local_radius_squared = 1_000_000_000_001.0,
            7 => r.options.alpha = Some(alpha()),
            8 => r.first.alpha_frame = Some(identity(0.0)),
            9 => r.mirrored = Some(pass(0.0, false, false)),
            10 => {
                let mut second = pass(0.0, true, false);
                second.local_radius_squared = 0.5;
                r.mirrored = Some(second);
            }
            11 => r.first.eye_direction[0] = 1_000_001.0,
            _ => unreachable!(),
        }
        assert_rejected(&mut white_unmasked(), &r);
    }
    let mut r = request();
    r.options.alpha = Some(alpha());
    let mut frame = identity(0.0);
    frame.look_at[11] = f32::NAN;
    r.first.alpha_frame = Some(frame);
    assert_rejected(&mut white_unmasked(), &r);
}

#[test]
fn unconnected_rows_are_excluded_and_report_counts_actual_selected_rows() {
    let mut m = white_unmasked();
    let index = m.positions.len();
    m.positions.push(Vec3::ZERO);
    m.normals.push(Vec3::Z);
    m.masks.push(1.0);
    m.colors.push([-0.0; 3]);
    m.validate().unwrap();
    let report = m.apply_paint_response(&request()).unwrap();
    assert_eq!(report.first_selected_vertices, 9);
    assert_eq!(report.changed_vertices, 9);
    assert_eq!(m.colors[index].map(f32::to_bits), [0x80000000; 3]);
}

#[test]
fn rich_uv_faces_geometry_normals_masks_and_disabled_channels_are_bit_exact() {
    let mut m = mesh(true);
    m.surface = Some(SurfaceTopology {
        faces: m
            .triangles
            .iter()
            .map(|t| [t[0], t[1], t[2], TRIANGLE_CORNER])
            .collect(),
        uv: Some(CornerUv {
            coordinates: m
                .positions
                .iter()
                .enumerate()
                .map(|(i, p)| [p.x + 0.5, i as f32 / 16.0])
                .collect(),
            face_indices: m
                .triangles
                .iter()
                .map(|t| [t[0], t[1], t[2], TRIANGLE_CORNER])
                .collect(),
        }),
    });
    m.validate().unwrap();
    let before = geometric_bits(&m);
    let mut r = request();
    r.options.write_roughness = true;
    assert!(m.apply_paint_response(&r).unwrap().changed_vertices > 0);
    assert_eq!(geometric_bits(&m), before);
}

#[test]
fn material_only_lazy_storage_is_one_exact_document_history_entry() {
    let mut d = Document::new(white_unmasked());
    let before = d.to_json().unwrap();
    let mut r = request();
    r.options.intensity = 0.125;
    r.options.write_rgb = false;
    r.options.write_roughness = true;
    r.options.write_metallic = true;
    d.begin_material_paint_stroke();
    for _ in 0..5 {
        assert!(
            d.require_active_mesh_mut()
                .unwrap()
                .apply_paint_response(&r)
                .unwrap()
                .changed_vertices
                > 0
        );
    }
    d.end_stroke();
    let after = d.to_json().unwrap();
    assert_ne!(before, after);
    assert!(d.require_active_mesh().unwrap().materials.is_some());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.require_active_mesh().unwrap().materials.is_none());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn retained_level_materialization_uses_existing_explicit_transaction_without_extra_history() {
    let mut d = Document::new(manifold_unmasked());
    d.add_subdivision_level().unwrap();
    let before = d.to_json().unwrap();
    let mut r = request();
    r.options.write_rgb = false;
    r.options.write_roughness = true;
    d.begin_material_paint_stroke();
    assert!(
        d.require_active_mesh_mut()
            .unwrap()
            .apply_paint_response(&r)
            .unwrap()
            .changed_vertices
            > 0
    );
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    d.select_level(0).unwrap();
    assert!(d.require_active_mesh().unwrap().materials.is_some());
    d.require_active_mesh().unwrap().validate().unwrap();
}

#[test]
fn generic_retained_material_presence_change_keeps_existing_atomic_rejection_policy() {
    let mut d = Document::new(manifold_unmasked());
    d.add_subdivision_level().unwrap();
    let before = d.to_json().unwrap();
    let mut r = request();
    r.options.write_rgb = false;
    r.options.write_metallic = true;
    d.begin_stroke();
    assert!(
        d.require_active_mesh_mut()
            .unwrap()
            .apply_paint_response(&r)
            .unwrap()
            .changed_vertices
            > 0
    );
    d.end_stroke();
    assert!(d.take_layer_error().is_some());
    assert_eq!(d.to_json().unwrap(), before);
}

#[test]
fn source_material_stroke_preserves_other_scene_parts_and_geometry_layer_recording() {
    let mut d = Document::new(white_unmasked());
    let other = d.add_subtool(mesh(true), "unaltered").unwrap();
    let other_bits = bits(d.require_active_mesh().unwrap());
    d.select_subtool(1).unwrap();
    let layer = d.add_layer("shape only").unwrap();
    d.set_recording_layer(Some(layer)).unwrap();
    let before = d.to_json().unwrap();
    let mut r = request();
    r.options.write_rgb = false;
    r.options.write_roughness = true;
    r.options.write_metallic = true;
    d.begin_material_paint_stroke();
    assert!(
        d.require_active_mesh_mut()
            .unwrap()
            .apply_paint_response(&r)
            .unwrap()
            .changed_vertices
            > 0
    );
    d.end_stroke();
    assert!(d.take_layer_error().is_none());
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
    d.select_subtool(other).unwrap();
    assert_eq!(bits(d.require_active_mesh().unwrap()), other_bits);
}

#[test]
fn local_radius_helper_rejects_invalid_or_excessive_effective_radius() {
    for radius in [f64::NAN, f64::INFINITY, -0.1, 1_000_000_000_001.0] {
        assert!(paint_tablet_local_radius_squared(radius, 0.5, 0.75).is_err());
    }
    for pressure in [f64::NAN, -0.01, 1.01] {
        assert!(paint_tablet_local_radius_squared(1.0, pressure, 0.75).is_err());
    }
    assert!(paint_tablet_local_radius_squared(1_000_000_000_000.0, 1.0, 1.0).is_err());
}

#[test]
fn ordered_symmetry_preserves_intermediate_float32_assignment_instead_of_collapsing_weights() {
    // Independent Python struct.pack Float32 oracle; a one-word distinction.
    let mut m = white_unmasked();
    m.colors.fill([f32::from_bits(1014637357); 3]);
    let mut r = request();
    r.options.color = [f32::from_bits(1050253722); 3];
    r.options.intensity = 0.13;
    r.mirrored = Some(pass(0.0, true, false));
    m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[4].map(f32::to_bits), [1034749659; 3]);
    assert_ne!(m.colors[4][0].to_bits(), 1034749658);
}

#[test]
fn malformed_mesh_is_rejected_before_any_attribute_commit() {
    for scenario in 0..3 {
        let mut m = white_unmasked();
        match scenario {
            0 => {
                m.colors.pop();
            }
            1 => m.normals[0].z = f32::NAN,
            2 => m.triangles[0][2] = 999,
            _ => unreachable!(),
        }
        assert_rejected(&mut m, &request());
    }
}

#[test]
fn double_precision_prepared_center_survives_near_vertex_float32_rounding() {
    let mut m = white_unmasked();
    let mut r = request();
    r.first.center = [0.99999998, 0.0, 0.0];
    r.first.local_radius_squared = 1e-15;
    r.options.hardness = 0.0;
    m.apply_paint_response(&r).unwrap();
    // Independent Python Float64 + explicit Float32 assignment oracle.
    let weight = f32::from_bits(1040864449);
    assert_eq!(m.colors[9].map(f32::to_bits), [weight.to_bits(); 3]);
    let mut rounded = white_unmasked();
    r.first.center[0] = f64::from(0.99999998_f64 as f32);
    rounded.apply_paint_response(&r).unwrap();
    assert_eq!(rounded.colors[9], [1.0; 3]);
    assert_ne!(bits(&m)["rgb"], bits(&rounded)["rgb"]);
}

#[test]
fn double_precision_away_eye_does_not_turn_a_backface_into_a_grazing_normal() {
    let mut m = white_unmasked();
    m.normals[4] = Vec3::new(
        std::f32::consts::FRAC_1_SQRT_2,
        std::f32::consts::FRAC_1_SQRT_2,
        0.0,
    );
    let mut r = request();
    r.options.culling = true;
    r.first.eye_direction = [1.00000002, -1.0, 0.0];
    let mut rounded = m.clone();
    m.apply_paint_response(&r).unwrap();
    assert_eq!(m.colors[4], [0.0; 3]);
    r.first.eye_direction[0] = f64::from(1.00000002_f64 as f32);
    rounded.apply_paint_response(&r).unwrap();
    assert_eq!(rounded.colors[4], [1.0; 3]);
}
