//! Independent absolute stamp previews, exact state restoration and history.
//! These gates do not establish SculptGL/ZBrush reference response equality.
use obrush_core::{
    BrushKind, BrushSettings, Document, LockedStamp, LockedStampAction, LockedStampSample, Mesh,
    RasterAlpha, Vec3,
};

const SIDE: usize = 17;
fn index(x: i32, y: i32) -> usize {
    (y + 8) as usize * SIDE + (x + 8) as usize
}
fn sheet() -> Mesh {
    let mut text = String::new();
    for row in 0..SIDE {
        for col in 0..SIDE {
            let x = (col as i32 - 8) as f32 * 0.0625;
            let y = (row as i32 - 8) as f32 * 0.0625;
            text.push_str(&format!("v {x} {y} 0 0.2 0.4 0.6\n"));
        }
    }
    for row in 0..SIDE - 1 {
        for col in 0..SIDE - 1 {
            let a = row * SIDE + col + 1;
            let b = a + 1;
            let c = a + SIDE;
            let d = c + 1;
            text.push_str(&format!("f {a} {b} {d}\nf {a} {d} {c}\n"));
        }
    }
    Mesh::from_obj(&text).unwrap()
}
fn rough_sheet() -> Mesh {
    let mut mesh = sheet();
    mesh.positions[index(2, 1)] += Vec3::new(0.015625, 0.0, 0.025);
    mesh.recompute_normals();
    mesh
}
fn settings(kind: BrushKind) -> BrushSettings {
    BrushSettings {
        kind,
        radius: 0.4,
        strength: 0.35,
        symmetry_x: false,
        adaptive_detail: false,
        alpha_tangent: Some(Vec3::X),
        ..Default::default()
    }
}
fn sample(radius: f32) -> LockedStampSample {
    LockedStampSample {
        radius,
        alpha_rotation: 0.0,
        alpha_tangent: Some(Vec3::X),
        pressure: 1.0,
    }
}
fn exact(a: &Mesh, b: &Mesh) {
    assert_eq!(a.positions.len(), b.positions.len());
    assert_eq!(a.normals.len(), b.normals.len());
    assert_eq!(a.masks.len(), b.masks.len());
    assert_eq!(a.colors.len(), b.colors.len());
    assert_eq!(a.triangles, b.triangles);
    for i in 0..a.positions.len() {
        assert_eq!(
            a.positions[i].to_array().map(f32::to_bits),
            b.positions[i].to_array().map(f32::to_bits),
            "position{i}"
        );
        assert_eq!(
            a.normals[i].to_array().map(f32::to_bits),
            b.normals[i].to_array().map(f32::to_bits),
            "normal{i}"
        );
        assert_eq!(a.masks[i].to_bits(), b.masks[i].to_bits(), "mask{i}");
        assert_eq!(
            a.colors[i].map(f32::to_bits),
            b.colors[i].map(f32::to_bits),
            "color{i}"
        );
    }
}
fn supported() -> impl Iterator<Item = BrushKind> {
    BrushKind::ALL
        .into_iter()
        .filter(|kind| LockedStamp::supports_kind(*kind))
}

#[test]
fn supported_kinds_match_the_exposed_ordinary_source_workflow() {
    let actual = supported().collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            BrushKind::Draw,
            BrushKind::Clay,
            BrushKind::Inflate,
            BrushKind::Smooth,
            BrushKind::Flatten,
            BrushKind::Crease,
            BrushKind::Mask,
            BrushKind::Paint,
            BrushKind::Pinch
        ]
    );
    for kind in [
        BrushKind::Move,
        BrushKind::Drag,
        BrushKind::Twist,
        BrushKind::LocalScale,
        BrushKind::Morph,
    ] {
        assert!(!LockedStamp::supports_kind(kind));
        assert!(LockedStamp::prepare(&sheet(), Vec3::ZERO, Vec3::Z, &settings(kind)).is_err());
    }
}

#[test]
fn preparation_is_nonmutating_and_rejects_invalid_capture_before_cloning() {
    let mesh = sheet();
    let original = mesh.snapshot();
    for scenario in 0..9 {
        let mut config = settings(BrushKind::Draw);
        let mut center = Vec3::ZERO;
        let mut normal = Vec3::Z;
        match scenario {
            0 => center.x = f32::NAN,
            1 => normal = Vec3::ZERO,
            2 => config.radius = -1.0,
            3 => config.strength = f32::INFINITY,
            4 => config.hardness = f32::NAN,
            5 => config.symmetry_offset = 1_000_001.0,
            6 => config.culling_axis = Some(Vec3::ZERO),
            7 => config.color[0] = f32::NAN,
            _ => config.alpha_tangent = Some(Vec3::new(f32::INFINITY, 0.0, 0.0)),
        };
        assert!(LockedStamp::prepare(&mesh, center, normal, &config).is_err());
        exact(&mesh, &original);
    }
    let stamp =
        LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &settings(BrushKind::Draw)).unwrap();
    assert!(stamp.at_origin());
    assert!(stamp.last_report().is_none());
    exact(&mesh, &original);
}

#[test]
fn every_positive_preview_matches_one_direct_dab_against_the_source() {
    let mut original = rough_sheet();
    for (i, mask) in original.masks.iter_mut().enumerate() {
        *mask = (i % 7) as f32 / 6.0;
    }
    for kind in supported() {
        let mut actual = original.snapshot();
        let mut direct = original.snapshot();
        let config = settings(kind);
        let input = LockedStampSample {
            pressure: 0.7,
            ..sample(0.375)
        };
        let mut stamp = LockedStamp::prepare(&actual, Vec3::ZERO, Vec3::Z, &config).unwrap();
        let report = stamp.apply(&mut actual, input).unwrap();
        let mut one = config.clone();
        one.radius = input.radius;
        one.alpha_rotation = input.alpha_rotation;
        one.alpha_tangent = input.alpha_tangent;
        one.pressure = input.pressure;
        let result = direct.apply_brush_with_result(Vec3::ZERO, Vec3::Z, Vec3::ZERO, &one);
        exact(&actual, &direct);
        assert_eq!(report.dab_changed_vertices, result.changed_vertices);
        assert_eq!(report.dab_displacement_scale, result.displacement_scale);
        assert_eq!(report.current_vertices, actual.positions.len());
        assert_eq!(report.kind, kind);
    }
}

#[test]
fn repeating_identical_samples_has_zero_real_changes_and_no_detail_accumulation() {
    let mut mesh = Mesh::sphere(2);
    let mut config = settings(BrushKind::Draw);
    config.adaptive_detail = true;
    let mut stamp = LockedStamp::prepare(&mesh, Vec3::Z, Vec3::Z, &config).unwrap();
    let first = stamp.apply(&mut mesh, sample(0.4)).unwrap();
    assert!(first.changed());
    assert!(first.refined_vertices > 0);
    assert!(!stamp.at_origin());
    let expected = mesh.snapshot();
    for seq in 2..=5 {
        let report = stamp.apply(&mut mesh, sample(0.4)).unwrap();
        assert!(!report.changed());
        assert_eq!(report.sequence, seq);
        assert_eq!(report.refined_vertices, first.refined_vertices);
        exact(&mesh, &expected);
    }
}

#[test]
fn intermediate_radius_and_angle_samples_do_not_change_the_final_stamp() {
    for kind in supported() {
        let original = rough_sheet();
        let mut config = settings(kind);
        config.alpha = Some(RasterAlpha::new(2, 2, vec![0, 255, 64, 192]).unwrap());
        let mut one = original.snapshot();
        let mut many = original.snapshot();
        let mut a = LockedStamp::prepare(&one, Vec3::ZERO, Vec3::Z, &config).unwrap();
        let mut b = LockedStamp::prepare(&many, Vec3::ZERO, Vec3::Z, &config).unwrap();
        let target = LockedStampSample {
            radius: 0.375,
            alpha_rotation: 0.65,
            alpha_tangent: Some(Vec3::X),
            pressure: 0.8,
        };
        a.apply(&mut one, target).unwrap();
        for input in [
            sample(0.2),
            LockedStampSample {
                alpha_rotation: 1.2,
                ..sample(0.45)
            },
            LockedStampSample {
                pressure: 0.3,
                ..sample(0.3)
            },
            target,
        ] {
            b.apply(&mut many, input).unwrap();
        }
        exact(&one, &many);
    }
}

#[test]
fn changing_adaptive_radius_replaces_previous_detail_and_zero_restores_source() {
    let original = Mesh::sphere(2);
    let mut mesh = original.snapshot();
    let mut config = settings(BrushKind::Draw);
    config.adaptive_detail = true;
    let mut stamp = LockedStamp::prepare(&mesh, Vec3::Z, Vec3::Z, &config).unwrap();
    for radius in [0.25, 0.4, 0.6, 0.3] {
        let report = stamp.apply(&mut mesh, sample(radius)).unwrap();
        assert_eq!(
            mesh.positions.len(),
            original.positions.len() + report.refined_vertices
        );
        assert_eq!(report.original_vertices, original.positions.len());
        mesh.validate().unwrap();
    }
    let report = stamp.apply(&mut mesh, sample(0.0)).unwrap();
    assert!(report.changed());
    assert!(report.at_origin);
    assert_eq!(report.action, LockedStampAction::Restore);
    assert!(stamp.at_origin());
    exact(&mesh, &original);
}

#[test]
fn zero_radius_restores_geometry_mask_color_and_signed_zero_exactly() {
    for kind in [BrushKind::Draw, BrushKind::Mask, BrushKind::Paint] {
        let mut mesh = sheet();
        mesh.positions[index(0, 0)].z = -0.0;
        mesh.normals[index(0, 0)].x = -0.0;
        mesh.masks[index(0, 0)] = -0.0;
        let original = mesh.snapshot();
        let mut stamp = LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &settings(kind)).unwrap();
        assert!(stamp.apply(&mut mesh, sample(0.4)).unwrap().changed());
        let restored = stamp.apply(&mut mesh, sample(0.0)).unwrap();
        assert!(restored.changed());
        assert_eq!(restored.dab_changed_vertices, 0);
        assert!(stamp.at_origin());
        exact(&mesh, &original);
        assert!(!stamp.apply(&mut mesh, sample(0.0)).unwrap().changed());
    }
}

#[test]
fn no_contact_keeps_last_preview_and_cancel_restores_without_positive_pressure() {
    let mut mesh = sheet();
    let original = mesh.snapshot();
    let mut stamp =
        LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &settings(BrushKind::Draw)).unwrap();
    stamp.apply(&mut mesh, sample(0.4)).unwrap();
    let preview = mesh.snapshot();
    let input = LockedStampSample {
        pressure: 0.0,
        ..sample(0.25)
    };
    let report = stamp.apply(&mut mesh, input).unwrap();
    assert!(!report.changed());
    assert_eq!(report.effective_pressure, 0.0);
    exact(&mesh, &preview);
    let cancelled = stamp.cancel(&mut mesh).unwrap();
    assert!(cancelled.changed());
    assert_eq!(cancelled.action, LockedStampAction::Cancel);
    assert!(cancelled.at_origin);
    exact(&mesh, &original);
}

#[test]
fn zero_radius_restoration_is_explicit_even_with_no_contact() {
    let mut mesh = sheet();
    let original = mesh.snapshot();
    let mut stamp =
        LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &settings(BrushKind::Draw)).unwrap();
    stamp.apply(&mut mesh, sample(0.4)).unwrap();
    let result = stamp
        .apply(
            &mut mesh,
            LockedStampSample {
                pressure: 0.0,
                ..sample(0.0)
            },
        )
        .unwrap();
    assert!(result.changed());
    assert!(result.at_origin);
    exact(&mesh, &original);
}

#[test]
fn stale_external_attributes_topology_and_signed_zero_edits_are_rejected() {
    for changed in 0..7 {
        let mut mesh = sheet();
        let mut stamp =
            LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &settings(BrushKind::Draw)).unwrap();
        stamp.apply(&mut mesh, sample(0.4)).unwrap();
        match changed {
            0 => mesh.positions[0].x += 0.01,
            1 => mesh.normals[0].x = 0.1,
            2 => mesh.masks[0] = 0.5,
            3 => mesh.colors[0][0] = 0.8,
            4 => mesh.triangles[0].swap(1, 2),
            5 => mesh.positions[0].z = -0.0,
            _ => mesh.normals[0].x = -0.0,
        };
        let external = mesh.snapshot();
        let report = stamp.last_report().cloned();
        assert!(stamp.apply(&mut mesh, sample(0.2)).is_err());
        assert!(stamp.cancel(&mut mesh).is_err());
        exact(&mesh, &external);
        assert_eq!(stamp.last_report(), report.as_ref());
    }
}

#[test]
fn stale_truncated_attributes_reject_without_panicking() {
    let mut mesh = sheet();
    let mut stamp =
        LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &settings(BrushKind::Draw)).unwrap();
    mesh.normals.pop();
    assert!(stamp.apply(&mut mesh, sample(0.4)).is_err());
    assert_eq!(mesh.normals.len(), SIDE * SIDE - 1);
}

#[test]
fn invalid_sample_fields_leave_current_mesh_and_report_unchanged() {
    for invalid in 0..6 {
        let mut mesh = sheet();
        let mut stamp =
            LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &settings(BrushKind::Draw)).unwrap();
        stamp.apply(&mut mesh, sample(0.4)).unwrap();
        let accepted = mesh.snapshot();
        let report = stamp.last_report().cloned();
        let mut input = sample(0.2);
        match invalid {
            0 => input.radius = -0.1,
            1 => input.radius = f32::NAN,
            2 => input.radius = 1_000_001.0,
            3 => input.alpha_rotation = f32::INFINITY,
            4 => input.alpha_tangent = Some(Vec3::new(0.0, f32::NAN, 0.0)),
            _ => input.pressure = f32::NAN,
        };
        assert!(stamp.apply(&mut mesh, input).is_err());
        exact(&mesh, &accepted);
        assert_eq!(stamp.last_report(), report.as_ref());
    }
}

#[test]
fn a_whole_locked_preview_gesture_is_one_exact_document_undo_step() {
    for kind in supported() {
        let mut document = Document::new(rough_sheet());
        let original = document.active_mesh().unwrap().snapshot();
        let mut stamp = LockedStamp::prepare(
            document.active_mesh().unwrap(),
            Vec3::ZERO,
            Vec3::Z,
            &settings(kind),
        )
        .unwrap();
        document.begin_stroke();
        for radius in [0.2, 0.4, 0.3] {
            stamp
                .apply(document.active_mesh_mut().unwrap(), sample(radius))
                .unwrap();
        }
        document.end_stroke();
        let final_mesh = document.active_mesh().unwrap().snapshot();
        assert!(document.can_undo(), "{kind}");
        assert!(document.undo());
        exact(document.active_mesh().unwrap(), &original);
        assert!(!document.can_undo());
        assert!(document.redo());
        exact(document.active_mesh().unwrap(), &final_mesh);
    }
}

#[test]
fn cancelled_and_zero_returned_gestures_preserve_a_genuine_preexisting_redo() {
    for cancel in [false, true] {
        let mut document = Document::new(sheet());
        document.begin_stroke();
        document.active_mesh_mut().unwrap().colors[0] = [0.7, 0.3, 0.2];
        document.end_stroke();
        assert!(document.undo());
        let original = document.active_mesh().unwrap().snapshot();
        let mut stamp = LockedStamp::prepare(
            document.active_mesh().unwrap(),
            Vec3::ZERO,
            Vec3::Z,
            &settings(BrushKind::Draw),
        )
        .unwrap();
        document.begin_stroke();
        stamp
            .apply(document.active_mesh_mut().unwrap(), sample(0.4))
            .unwrap();
        if cancel {
            stamp.cancel(document.active_mesh_mut().unwrap()).unwrap();
        } else {
            stamp
                .apply(document.active_mesh_mut().unwrap(), sample(0.0))
                .unwrap();
        }
        document.end_stroke();
        exact(document.active_mesh().unwrap(), &original);
        assert!(!document.can_undo());
        assert!(document.can_redo());
        assert!(document.redo());
        assert_eq!(document.active_mesh().unwrap().colors[0], [0.7, 0.3, 0.2]);
    }
}

#[test]
fn zero_strength_black_alpha_and_full_protection_produce_no_history() {
    for case in 0..3 {
        for kind in supported() {
            let mut document = Document::new(rough_sheet());
            let mut config = settings(kind);
            match case {
                0 => config.strength = 0.0,
                1 => config.alpha = Some(RasterAlpha::new(2, 2, vec![0; 4]).unwrap()),
                _ => document.active_mesh_mut().unwrap().masks.fill(1.0),
            };
            let original = document.active_mesh().unwrap().snapshot();
            let mut stamp = LockedStamp::prepare(
                document.active_mesh().unwrap(),
                Vec3::ZERO,
                Vec3::Z,
                &config,
            )
            .unwrap();
            document.begin_stroke();
            let report = stamp
                .apply(document.active_mesh_mut().unwrap(), sample(0.4))
                .unwrap();
            assert!(!report.changed(), "{kind}");
            document.end_stroke();
            exact(document.active_mesh().unwrap(), &original);
            assert!(!document.can_undo());
        }
    }
}

#[test]
fn rotation_and_tangent_change_the_directional_alpha_in_an_absolute_stamp() {
    let original = sheet();
    let mut mesh = original.snapshot();
    let mut config = settings(BrushKind::Mask);
    config.alpha = Some(RasterAlpha::new(2, 1, vec![0, 255]).unwrap());
    let mut stamp = LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &config).unwrap();
    stamp.apply(&mut mesh, sample(0.4)).unwrap();
    let horizontal = mesh.snapshot();
    stamp
        .apply(
            &mut mesh,
            LockedStampSample {
                alpha_rotation: std::f32::consts::FRAC_PI_2,
                ..sample(0.4)
            },
        )
        .unwrap();
    assert!(horizontal.masks[index(4, 0)] > horizontal.masks[index(0, 4)] * 1.5);
    assert!(mesh.masks[index(0, 4)] > mesh.masks[index(4, 0)] * 1.5);
    assert!((horizontal.masks[index(4, 0)] - mesh.masks[index(0, 4)]).abs() < 2e-7);
    stamp
        .apply(
            &mut mesh,
            LockedStampSample {
                alpha_tangent: Some(Vec3::Y),
                ..sample(0.4)
            },
        )
        .unwrap();
    assert!((horizontal.masks[index(4, 0)] - mesh.masks[index(0, 4)]).abs() < 2e-7);
    assert_eq!(mesh.positions, original.positions);
}

#[test]
fn pressure_strength_and_radius_follow_the_single_dab_contract() {
    let original = sheet();
    let mut config = settings(BrushKind::Mask);
    config.pressure_radius = true;
    let mut mesh = original.snapshot();
    let mut stamp = LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &config).unwrap();
    let report = stamp
        .apply(
            &mut mesh,
            LockedStampSample {
                pressure: 0.5,
                ..sample(0.4)
            },
        )
        .unwrap();
    assert_eq!(report.effective_radius, Some(0.2));
    assert_eq!(mesh.masks[index(4, 0)], 0.0);
    let expected = 0.18_f64 * f64::from(config.strength) * 0.5;
    let error = (f64::from(mesh.masks[index(0, 0)]) - expected).abs();
    assert!(error < 2e-7);
    println!(
        "Locked Mask pressure=.5: expected={expected}, measured={}, error={error}",
        mesh.masks[index(0, 0)]
    );
    stamp
        .apply(
            &mut mesh,
            LockedStampSample {
                pressure: 1.5,
                ..sample(0.4)
            },
        )
        .unwrap();
    let full = mesh.snapshot();
    let report = stamp.apply(&mut mesh, sample(0.4)).unwrap();
    assert!(!report.changed());
    exact(&mesh, &full);
}

#[test]
fn captured_inverse_mask_settings_and_source_weights_restore_exactly() {
    let mut mesh = sheet();
    mesh.masks.fill(0.5);
    let original = mesh.snapshot();
    let mut config = settings(BrushKind::Mask);
    config.invert = true;
    let mut stamp = LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &config).unwrap();
    stamp.apply(&mut mesh, sample(0.4)).unwrap();
    assert!(mesh.masks[index(0, 0)] < 0.5);
    assert!(stamp.settings().invert);
    stamp.cancel(&mut mesh).unwrap();
    exact(&mesh, &original);
}

#[test]
fn offset_symmetry_is_kept_in_stamp_geometry_and_mask_profiles() {
    let mut original = sheet();
    for point in &mut original.positions {
        point.x += 1.0;
    }
    for kind in [
        BrushKind::Draw,
        BrushKind::Pinch,
        BrushKind::Mask,
        BrushKind::Paint,
    ] {
        let mut mesh = original.snapshot();
        let mut config = settings(kind);
        config.symmetry_x = true;
        config.symmetry_offset = 1.0;
        let mut stamp =
            LockedStamp::prepare(&mesh, Vec3::new(1.125, 0.0, 0.0), Vec3::Z, &config).unwrap();
        stamp.apply(&mut mesh, sample(0.375)).unwrap();
        for y in -8..=8 {
            for x in 0..=8 {
                let a = mesh.positions[index(x, y)];
                let b = mesh.positions[index(-x, y)];
                assert!((a - Vec3::new(2.0 - b.x, b.y, b.z)).abs().max_element() < 2e-6);
                assert_eq!(mesh.masks[index(x, y)], mesh.masks[index(-x, y)]);
                assert_eq!(mesh.colors[index(x, y)], mesh.colors[index(-x, y)]);
            }
        }
    }
}

#[test]
fn readonly_report_provenance_matches_actual_mesh_and_roundtrips() {
    let mut mesh = sheet();
    let mut config = settings(BrushKind::Draw);
    config.draw_accumulate = false;
    let mut stamp = LockedStamp::prepare(&mesh, Vec3::ZERO, Vec3::Z, &config).unwrap();
    let input = LockedStampSample {
        radius: 0.375,
        alpha_rotation: 0.4,
        alpha_tangent: Some(Vec3::X),
        pressure: 0.75,
    };
    let report = stamp.apply(&mut mesh, input).unwrap();
    assert_eq!(report.sequence, 1);
    assert_eq!(report.radius, input.radius);
    assert_eq!(report.alpha_rotation, input.alpha_rotation);
    assert_eq!(report.alpha_tangent, input.alpha_tangent);
    assert_eq!(report.pressure, input.pressure);
    assert_eq!(report.current_vertices, mesh.positions.len());
    assert_eq!(report.current_triangles, mesh.triangles.len());
    assert_eq!(stamp.last_report(), Some(&report));
    assert!(!stamp.settings().draw_accumulate);
    let serialized = serde_json::to_string(&report).unwrap();
    let round = serde_json::from_str(&serialized).unwrap();
    assert_eq!(report, round);
}
