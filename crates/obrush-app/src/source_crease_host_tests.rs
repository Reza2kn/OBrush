//! Production Crease preferences, numeric controls, capture, queue and history.
//! These host tests do not establish actual browser/reference/device parity.
use super::super::source_geometry_ui::{self, GeometryEditorDrafts};
use super::GeometryPreferenceStore;
use crate::dab_queue::DabQueue;
use crate::source_geometry_host::{
    self as host, SourceGeometryMode, SourceGeometryPreferences, SourceGeometrySession,
};
use crate::source_paint_host::RawPressureSample;
use eframe::egui::{self, Pos2, Rect, Vec2};
use obrush_core::*;
use std::sync::{Arc, Mutex};

fn doc() -> Document {
    let mut m =
        Mesh::from_obj("v 0 0 0\nv 4 -4 0\nv 4 4 0\nv -3 3 0\nv -3 -3 0\nf 1 2 3\nf 1 4 5\n")
            .unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, TRIANGLE_CORNER], [0, 3, 4, TRIANGLE_CORNER]],
        uv: None,
    });
    m.validate().unwrap();
    Document::new(m)
}
fn prefs() -> SourceGeometryPreferences {
    SourceGeometryPreferences {
        mode: SourceGeometryMode::SourceOrdinary,
        radius_factor: 0.0,
        options: SourceGeometryOptions {
            intensity: 0.625,
            prevent_folds: false,
            ..SourceGeometryOptions::crease()
        },
        ..SourceGeometryPreferences::crease()
    }
}
fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Crease,
        adaptive_detail: false,
        symmetry_x: false,
        alpha: None,
        ..Default::default()
    }
}
fn input() -> host::SourceGeometryInput {
    host::SourceGeometryInput {
        view: PaintViewCapture {
            world_to_clip: [
                0.25, 0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0, -0.1, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            viewport: [0.0, 0.0, 100.0, 100.0],
        },
        screen: [56.25, 50.0],
        raw: RawPressureSample::default(),
        in_contact: true,
    }
}
fn tracks() -> host::SharedSourceGeometryAlphaTracks {
    Arc::new(Mutex::new(Default::default()))
}

#[test]
fn h01_seventh_row_preserves_six_old_rows_and_legacy_defaults() {
    let mut store = GeometryPreferenceStore::default();
    let kinds = [
        BrushKind::Draw,
        BrushKind::Clay,
        BrushKind::Inflate,
        BrushKind::Pinch,
        BrushKind::Smooth,
        BrushKind::Flatten,
    ];
    let old = kinds.map(|kind| store.get(kind).unwrap());
    let clay = SourceGeometryPreferences {
        options: SourceGeometryOptions {
            clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        old,
        [
            clay,
            clay,
            SourceGeometryPreferences::inflate(),
            SourceGeometryPreferences::pinch(),
            SourceGeometryPreferences::smooth(),
            SourceGeometryPreferences::flatten()
        ]
    );
    let p = store.get(BrushKind::Crease).unwrap();
    assert_eq!(GeometryPreferenceStore::index(BrushKind::Crease), Some(6));
    assert_eq!(p, SourceGeometryPreferences::crease());
    assert_eq!(p.mode, SourceGeometryMode::Legacy);
    assert_eq!(p.radius, 25.0);
    assert_eq!(p.radius_factor, 0.75);
    assert_eq!(p.options.intensity, 0.75);
    assert_eq!(p.options.intensity_factor, 0.0);
    assert!(p.options.negative && p.options.prevent_folds && !p.options.culling);
    store.set(BrushKind::Crease, prefs()).unwrap();
    assert_eq!(store.get(BrushKind::Crease).unwrap(), prefs());
    assert_eq!(kinds.map(|kind| store.get(kind).unwrap()), old);
    assert_eq!(
        super::super::initial_brush_settings().radius.to_bits(),
        0.18f32.to_bits()
    );
    assert_eq!(
        super::super::initial_brush_settings().strength.to_bits(),
        0.35f32.to_bits()
    );
    assert!(store.get(BrushKind::Twist).is_none());
}

#[test]
fn h02_production_crease_numeric_widgets_are_idle_and_clip_safe() {
    for width in [190.0, 340.0] {
        let ctx = egui::Context::default();
        let mut drafts = GeometryEditorDrafts::default();
        let p = prefs();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 900.0))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let out = source_geometry_ui::render(ui, p, &mut drafts);
                    assert!(out.candidate.is_none() && out.rejected.is_none());
                    for key in [
                        "source_geometry.radius",
                        "source_geometry.intensity",
                        "source_geometry.pressure.radius",
                        "source_geometry.pressure.intensity",
                    ] {
                        for key in [key.to_string(), format!("{key}.value")] {
                            let response = &out.controls[&key];
                            assert!(response.enabled());
                            assert!(ui.clip_rect().contains_rect(response.rect));
                        }
                    }
                });
            },
        );
        assert_eq!(p, prefs());
    }
}

#[test]
fn h03_actual_queue_uses_captured_preferences_and_closes_one_history_entry() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let shared_tracks = tracks();
    let mut caller = prefs();
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        caller,
        input(),
        shared_tracks.clone(),
    )
    .unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(shared_tracks.lock().unwrap().active_claim().is_some());
    caller.options.negative = false;
    caller.options.intensity = 0.0;
    assert_ne!(caller, session.lock().unwrap().preferences());
    assert_eq!(session.lock().unwrap().preferences(), prefs());
    let mut q = DabQueue::default();
    for _ in 0..200 {
        q.source_geometry_dab(session.clone(), input());
    }
    let result = q.release_source_geometry(&mut d);
    assert!(result.changed && result.error.is_none());
    assert_eq!(result.dabs, 1);
    assert!(q.is_empty());
    assert_eq!(
        d.require_active_mesh().unwrap().positions[0]
            .to_array()
            .map(f32::to_bits),
        [0x3caa199a, 0, 0xbd8a17a3]
    );
    session.lock().unwrap().finish(&mut d).unwrap();
    assert!(shared_tracks.lock().unwrap().active_claim().is_none());
    let after = d.to_json().unwrap();
    assert_eq!(
        host::apply_shared(&mut d, &session, input()).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert_eq!(d.to_json().unwrap(), after);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn h04_mismatched_tools_and_unsupported_settings_refuse_without_history_changes() {
    let d = doc();
    let before = d.to_json().unwrap();
    assert_eq!(
        host::blocker(&d, &settings(), false, prefs()).unwrap_err(),
        SourceGeometryError::UnsupportedDomain
    );
    for p in [
        SourceGeometryPreferences::pinch(),
        SourceGeometryPreferences::flatten(),
    ] {
        assert!(
            host::blocker_controls(
                &d,
                &settings(),
                false,
                SourceGeometryPreferences {
                    mode: SourceGeometryMode::SourceOrdinary,
                    ..p
                }
            )
            .is_err()
        );
    }
    for (rotation, offset, invert, adaptive, locked) in [
        (0.5, 0.0, false, false, false),
        (0.0, 0.2, false, false, false),
        (0.0, 0.0, true, false, false),
        (0.0, 0.0, false, true, false),
        (0.0, 0.0, false, false, true),
    ] {
        assert_eq!(
            host::blocker_controls(
                &d,
                &BrushSettings {
                    alpha_rotation: rotation,
                    symmetry_offset: offset,
                    invert,
                    adaptive_detail: adaptive,
                    ..settings()
                },
                locked,
                prefs()
            )
            .unwrap_err(),
            SourceGeometryError::UnsupportedDomain
        );
    }
    assert_eq!(
        host::blocker_controls(
            &d,
            &settings(),
            false,
            SourceGeometryPreferences {
                options: SourceGeometryOptions {
                    clay_order: SourceClayOrderPolicy::SingleLeaf,
                    ..prefs().options
                },
                ..prefs()
            }
        )
        .unwrap_err(),
        SourceGeometryError::UnsupportedCreaseOrder
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert_eq!(d.source_geometry_retained_bytes(), 0);
}

#[test]
fn h05_failed_second_input_cancels_useful_first_dab_and_preserves_prior_redo_and_tracks() {
    let mut d = doc();
    d.begin_stroke();
    d.name = "genuine future".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let shared_tracks = tracks();
    let initial = shared_tracks.lock().unwrap().snapshot();
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        prefs(),
        input(),
        shared_tracks.clone(),
    )
    .unwrap();
    assert_eq!(
        host::apply_shared(&mut d, &session, input())
            .unwrap()
            .changed_vertices,
        1
    );
    let mut invalid = input();
    invalid.screen[0] = f64::NAN;
    assert_eq!(
        host::apply_shared(&mut d, &session, invalid).unwrap_err(),
        SourceGeometryError::InvalidInput
    );
    session.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    let current = shared_tracks.lock().unwrap();
    assert!(current.active_claim().is_none());
    assert!(host::same_tracks(initial, current.snapshot()));
    drop(current);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), future);
}

#[test]
fn h06_captured_alpha_and_mirror_match_direct_shared_core_and_history() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let mut caller = settings();
    caller.symmetry_x = true;
    caller.alpha = Some(RasterAlpha::new(1, 1, vec![128]).unwrap());
    let shared_tracks = tracks();
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &caller,
        false,
        prefs(),
        input(),
        shared_tracks.clone(),
    )
    .unwrap();
    caller.alpha = Some(RasterAlpha::new(1, 1, vec![0]).unwrap());
    caller.symmetry_x = false;
    assert!(!caller.symmetry_x);
    let report = host::apply_shared(&mut d, &session, input()).unwrap();
    assert_eq!(report.selected_vertices, 2);
    assert_eq!(report.changed_vertices, 1);
    let diagnostics = session.lock().unwrap().diagnostics();
    let read_pass = |value: &serde_json::Value| SourceGeometryPass {
        center: serde_json::from_value(value["center"].clone()).unwrap(),
        eye_direction: serde_json::from_value(value["eye_direction"].clone()).unwrap(),
        local_radius_squared: value["local_radius_squared"].as_f64().unwrap(),
        original_face_index: value["original_face_index"].as_u64().unwrap() as usize,
    };
    let primary = read_pass(&diagnostics["controlled"]["contacts"][0]);
    let mirror = read_pass(&diagnostics["controlled"]["contacts"][1]);
    let mut expected = doc();
    let mut stroke = expected
        .begin_source_geometry_stroke(prefs().options)
        .unwrap();
    expected
        .apply_source_geometry_controlled_dab(
            &mut stroke,
            [Some(primary), Some(mirror)],
            0.5,
            true,
            &SourceGeometryControls {
                alpha: Some(RasterAlpha::new(1, 1, vec![128]).unwrap()),
                mirror: true,
            },
            Default::default(),
        )
        .unwrap();
    expected.end_source_geometry_stroke(&stroke).unwrap();
    session.lock().unwrap().finish(&mut d).unwrap();
    let after = d.to_json().unwrap();
    assert_eq!(after, expected.to_json().unwrap());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
