//! Genuine saved row, production numeric widgets, capture/queue/barrier and
//! warm-Paint host contracts. Full OBrushApp/browser/physical input remain UNRUN.
use super::super::source_geometry_ui::{self, GeometryCommit, GeometryEditorDrafts};
use super::GeometryPreferenceStore;
use crate::dab_queue::DabQueue;
use crate::source_geometry_host::{
    self as host, SourceGeometryMode, SourceGeometryPreferences, SourceGeometrySession,
};
use crate::source_paint_host::{
    PaintMode, RawPressureSample, SourcePaintPreferences, SourcePaintSession,
};
use eframe::egui::{self, Pos2, Rect, Vec2};
use obrush_core::*;
use std::cell::Cell;
use std::sync::{Arc, Mutex};

fn doc() -> Document {
    let mut m = Mesh::import_obj(include_bytes!(
        "../../obrush-core/tests/fixtures/independent-flatten-corners.obj"
    ))
    .unwrap()
    .mesh;
    m.surface = Some(SurfaceTopology {
        faces: vec![[0, 1, 2, TRIANGLE_CORNER], [3, 4, 5, TRIANGLE_CORNER]],
        uv: None,
    });
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m.validate().unwrap();
    Document::new(m)
}
fn prefs() -> SourceGeometryPreferences {
    SourceGeometryPreferences {
        mode: SourceGeometryMode::SourceOrdinary,
        radius: 25.0,
        radius_factor: 0.0,
        options: SourceGeometryOptions {
            intensity: 0.25,
            ..SourceGeometryOptions::flatten()
        },
        ..SourceGeometryPreferences::flatten()
    }
}
fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Flatten,
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
        screen: [50.0, 50.0],
        raw: RawPressureSample::default(),
        in_contact: true,
    }
}
fn tracks() -> host::SharedSourceGeometryAlphaTracks {
    Arc::new(Mutex::new(Default::default()))
}

#[test]
fn h01_genuine_sixth_preferences_row_retains_all_five_old_rows_and_legacy_words() {
    let mut store = GeometryPreferenceStore::default();
    let kinds = [
        BrushKind::Draw,
        BrushKind::Clay,
        BrushKind::Inflate,
        BrushKind::Pinch,
        BrushKind::Smooth,
    ];
    let old = kinds.map(|k| store.get(k).unwrap());
    let flat = store.get(BrushKind::Flatten).unwrap();
    assert_eq!(GeometryPreferenceStore::index(BrushKind::Flatten), Some(5));
    assert_eq!(flat, SourceGeometryPreferences::flatten());
    assert_eq!(flat.mode, SourceGeometryMode::Legacy);
    assert_eq!(flat.radius, 50.0);
    assert_eq!(flat.radius_factor, 0.75);
    assert_eq!(flat.options.intensity, 0.75);
    assert_eq!(flat.options.intensity_factor, 0.0);
    assert!(flat.options.negative);
    assert!(!flat.options.culling);
    store.set(BrushKind::Flatten, prefs()).unwrap();
    assert_eq!(store.get(BrushKind::Flatten).unwrap(), prefs());
    for (i, k) in kinds.into_iter().enumerate() {
        assert_eq!(store.get(k).unwrap(), old[i]);
    }
    assert_eq!(
        super::super::initial_brush_settings().strength.to_bits(),
        0.35f32.to_bits()
    );
    assert!(store.get(BrushKind::Twist).is_none());
}
#[test]
fn h02_real_numeric_widgets_are_bounded_idle_and_clip_safe_for_flatten() {
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
                    assert!(out.candidate.is_none());
                    assert!(out.rejected.is_none());
                    for key in [
                        "source_geometry.radius",
                        "source_geometry.intensity",
                        "source_geometry.pressure.radius",
                        "source_geometry.pressure.intensity",
                    ] {
                        let rail = &out.controls[key];
                        let editor = &out.controls[&format!("{key}.value")];
                        assert!(rail.enabled() && editor.enabled());
                        assert!(ui.clip_rect().contains_rect(rail.rect));
                        assert!(ui.clip_rect().contains_rect(editor.rect));
                    }
                    assert!(!out.controls.contains_key("source_geometry.smooth.tangent"));
                    assert!(!out.controls.contains_key("source_geometry.clay"));
                    assert!(!out.controls.contains_key("source_geometry.accumulate"));
                });
            },
        );
        assert!(p.options.negative);
        assert_eq!(p.options.intensity.to_bits(), 0.25f64.to_bits());
    }
}
#[test]
fn h03_captured_controlled_queue_and_direct_route_have_one_history_and_no_tail() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let mut caller = prefs();
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        caller,
        input(),
        tracks(),
    )
    .unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    caller.options.negative = false;
    assert!(!caller.options.negative);
    assert!(session.lock().unwrap().preferences().options.negative);
    let mut q = DabQueue::default();
    for _ in 0..200 {
        q.source_geometry_dab(session.clone(), input());
    }
    let r = q.release_source_geometry(&mut d);
    assert!(r.changed && r.error.is_none());
    assert_eq!(r.dabs, 1);
    assert!(q.is_empty());
    assert_eq!(
        d.require_active_mesh().unwrap().positions[0].z.to_bits(),
        0x3f600000
    );
    session.lock().unwrap().finish(&mut d).unwrap();
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
    let mut direct = doc();
    let s = SourceGeometrySession::begin_with_controls(
        &mut direct,
        &settings(),
        false,
        prefs(),
        input(),
        tracks(),
    )
    .unwrap();
    assert_eq!(
        host::apply_shared(&mut direct, &s, input())
            .unwrap()
            .changed_vertices,
        1
    );
    s.lock().unwrap().finish(&mut direct).unwrap();
    assert_eq!(direct.to_json().unwrap(), after);
}
#[test]
fn h04_different_candidate_releases_old_queue_and_noop_or_invalid_never_does() {
    let mut d = doc();
    let old = prefs();
    let s = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        old,
        input(),
        tracks(),
    )
    .unwrap();
    let mut q = DabQueue::default();
    q.source_geometry_dab(s.clone(), input());
    let new = SourceGeometryPreferences {
        options: SourceGeometryOptions {
            negative: false,
            ..old.options
        },
        ..old
    };
    let committed = source_geometry_ui::commit_candidate(old, new, || {
        let r = q.release_source_geometry(&mut d);
        assert!(r.changed && r.error.is_none());
        s.lock().unwrap().finish(&mut d).is_ok()
    })
    .unwrap();
    assert!(matches!(committed,GeometryCommit::Accepted(p) if p==new));
    assert_eq!(
        d.require_active_mesh().unwrap().positions[0].z.to_bits(),
        0x3f600000
    );
    let calls = Cell::new(0);
    assert!(matches!(
        source_geometry_ui::commit_candidate(old, old, || {
            calls.set(calls.get() + 1);
            true
        })
        .unwrap(),
        GeometryCommit::Unchanged
    ));
    let bad = SourceGeometryPreferences {
        radius: f64::NAN,
        ..old
    };
    assert!(
        source_geometry_ui::commit_candidate(old, bad, || {
            calls.set(calls.get() + 1);
            true
        })
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    assert!(matches!(
        source_geometry_ui::commit_candidate(old, new, || false).unwrap(),
        GeometryCommit::ReleaseRejected
    ));
    assert_eq!(s.lock().unwrap().preferences(), old);
}
#[test]
fn h05_new_typed_controlled_admission_keeps_original_constructor_and_refusals() {
    let d = doc();
    let before = d.to_json().unwrap();
    assert!(host::blocker(&d, &settings(), false, prefs()).is_err());
    assert!(host::blocker_controls(&d, &settings(), false, prefs()).is_ok());
    for kind in [BrushKind::Draw, BrushKind::Smooth, BrushKind::Move] {
        assert!(
            host::blocker_controls(&d, &BrushSettings { kind, ..settings() }, false, prefs())
                .is_err()
        );
    }
    for (alpha_rotation, symmetry_offset, invert, adaptive) in [
        (0.1, 0.0, false, false),
        (0.0, 0.1, false, false),
        (0.0, 0.0, true, false),
        (0.0, 0.0, false, true),
    ] {
        assert!(
            host::blocker_controls(
                &d,
                &BrushSettings {
                    alpha_rotation,
                    symmetry_offset,
                    invert,
                    adaptive_detail: adaptive,
                    ..settings()
                },
                false,
                prefs()
            )
            .is_err()
        );
    }
    assert!(host::blocker_controls(&d, &settings(), true, prefs()).is_err());
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
        SourceGeometryError::UnsupportedFlattenOrder
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert_eq!(d.source_geometry_retained_bytes(), 0);
}
#[test]
fn h06_real_source_paint_capture_consumes_retained_flatten_field_and_history() {
    let mut d = doc();
    let g = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        prefs(),
        input(),
        tracks(),
    )
    .unwrap();
    assert_eq!(
        host::apply_shared(&mut d, &g, input())
            .unwrap()
            .changed_vertices,
        1
    );
    g.lock().unwrap().finish(&mut d).unwrap();
    let id = d.active_subtool_id().unwrap();
    let state = d.source_geometry_state(id).unwrap();
    assert_eq!(state.vertex_normals()[0], Vec3::new(-0.25, -0.25, 4.0));
    assert_eq!(state.vertex_normals()[1], Vec3::new(0.0, 0.0, 4.0));
    let old_normals = d.require_active_mesh().unwrap().normals.clone();
    let before = d.to_json().unwrap();
    let p = SourcePaintPreferences {
        mode: PaintMode::SourceOrdinary,
        radius: 25.0,
        radius_factor: 0.0,
        mirror_offset: 0.0,
        options: PaintResponseOptions {
            intensity: 1.0,
            color: [0.25, 0.5, 0.75],
            ..Default::default()
        },
    };
    let brush = BrushSettings {
        kind: BrushKind::Paint,
        adaptive_detail: false,
        symmetry_x: false,
        alpha: None,
        ..Default::default()
    };
    let paint = SourcePaintSession::prepare(&d, &p, &brush, false, input()).unwrap();
    let mut q = DabQueue::default();
    q.begin();
    q.source_paint_dab(paint.clone(), input());
    let r = q.process(&mut d, 1, None);
    assert!(r.changed && r.painted && r.error.is_none());
    q.release(&mut d);
    assert!(q.is_empty());
    assert!(Arc::ptr_eq(&state, &d.source_geometry_state(id).unwrap()));
    assert_eq!(d.require_active_mesh().unwrap().normals, old_normals);
    assert!(!paint.lock().unwrap().diagnostics()["captured_normal_producer"].is_null());
    let after = d.to_json().unwrap();
    assert_ne!(after, before);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
