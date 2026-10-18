//! Real typed session/queue and actual tangent widget; authored unrun.
use super::super::source_geometry_ui::{self, GeometryCommit};
use super::{GeometryPreferenceStore, smooth_tangent_control};
use crate::dab_queue::DabQueue;
use crate::source_geometry_host::{
    self as host, SourceGeometryMode, SourceGeometryPreferences, SourceGeometrySession,
};
use crate::source_paint_host::RawPressureSample;
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
use obrush_core::*;
use std::cell::Cell;
use std::sync::{Arc, Mutex};

fn doc() -> Document {
    let mut m = Mesh::import_obj(
        b"v 0 0 1\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nf 1 2 3\nf 1 3 4\nf 1 4 2\nf 2 4 3\n",
    )
    .unwrap()
    .mesh;
    m.surface = Some(SurfaceTopology {
        faces: vec![
            [0, 1, 2, TRIANGLE_CORNER],
            [0, 2, 3, TRIANGLE_CORNER],
            [0, 3, 1, TRIANGLE_CORNER],
            [1, 3, 2, TRIANGLE_CORNER],
        ],
        uv: None,
    });
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    m.validate().unwrap();
    Document::new(m)
}
fn prefs(tangent: bool) -> SourceGeometryPreferences {
    SourceGeometryPreferences {
        mode: SourceGeometryMode::SourceOrdinary,
        radius: 25.0,
        radius_factor: 0.0,
        options: SourceGeometryOptions {
            tool: SourceGeometryTool::Smooth { tangent },
            intensity: 0.375,
            ..SourceGeometryOptions::smooth()
        },
        ..SourceGeometryPreferences::smooth()
    }
}
fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Smooth,
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
fn fifth_saved_row_is_real_and_tangent_retention_isolated_from_all_old_tools() {
    let mut store = GeometryPreferenceStore::default();
    let old = [
        BrushKind::Draw,
        BrushKind::Clay,
        BrushKind::Inflate,
        BrushKind::Pinch,
    ]
    .map(|k| store.get(k).unwrap());
    let smooth = store.get(BrushKind::Smooth).unwrap();
    assert_eq!(smooth, SourceGeometryPreferences::smooth());
    assert_eq!(GeometryPreferenceStore::index(BrushKind::Smooth), Some(4));
    store.set(BrushKind::Smooth, prefs(true)).unwrap();
    assert_eq!(store.get(BrushKind::Smooth).unwrap(), prefs(true));
    for (i, k) in [
        BrushKind::Draw,
        BrushKind::Clay,
        BrushKind::Inflate,
        BrushKind::Pinch,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(store.get(k).unwrap(), old[i]);
    }
    assert_eq!(
        super::super::initial_brush_settings().strength.to_bits(),
        0.35f32.to_bits()
    );
    assert!(store.get(BrushKind::Twist).is_none());
}

#[test]
fn matching_old_ordinary_and_controlled_smooth_admission_does_not_admit_mispaired_or_lock() {
    let d = doc();
    let before = d.to_json().unwrap();
    assert!(host::blocker(&d, &settings(), false, prefs(false)).is_ok());
    assert!(host::blocker_controls(&d, &settings(), false, prefs(false)).is_ok());
    for kind in [BrushKind::Draw, BrushKind::Pinch, BrushKind::Move] {
        let wrong = BrushSettings { kind, ..settings() };
        assert!(host::blocker_controls(&d, &wrong, false, prefs(false)).is_err());
    }
    assert!(host::blocker_controls(&d, &settings(), true, prefs(false)).is_err());
    assert_eq!(
        host::blocker_controls(
            &d,
            &settings(),
            false,
            SourceGeometryPreferences {
                options: SourceGeometryOptions {
                    negative: true,
                    ..prefs(false).options
                },
                ..prefs(false)
            }
        )
        .unwrap_err(),
        SourceGeometryError::UnsupportedSmoothNegative
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.source_geometry_retained_bytes(), 0);
}

#[test]
fn normal_queue_has_no_press_dab_then_one_latest_useful_complete_history() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let s = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        prefs(false),
        input(),
        tracks(),
    )
    .unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    assert!(!d.can_undo());
    let mut q = DabQueue::default();
    for _ in 0..200 {
        q.source_geometry_dab(s.clone(), input());
    }
    let p = q.release_source_geometry(&mut d);
    assert!(p.changed && p.error.is_none());
    assert_eq!(p.dabs, 1);
    assert!(q.is_empty());
    let captured = s.lock().unwrap();
    assert_eq!(captured.preferences(), prefs(false));
    let pass = captured.last_pass.unwrap();
    assert_eq!(pass.center, [0.0, 0.0, 1.0]);
    assert_eq!(pass.local_radius_squared, 4.0);
    drop(captured);
    s.lock().unwrap().finish(&mut d).unwrap();
    let after = d.to_json().unwrap();
    assert_eq!(
        d.require_active_mesh().unwrap().positions[0]
            .to_array()
            .map(f32::to_bits),
        [0x3e000000, 0, 0x3f200000]
    );
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}

#[test]
fn captured_tangent_survives_caller_edit_and_finished_session_refuses_tail() {
    let mut d = doc();
    let mut p = prefs(true);
    let s = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        p,
        input(),
        tracks(),
    )
    .unwrap();
    p.options.tool = SourceGeometryTool::Smooth { tangent: false };
    assert_eq!(
        p.options.tool,
        SourceGeometryTool::Smooth { tangent: false }
    );
    assert_eq!(
        s.lock().unwrap().preferences().options.tool,
        SourceGeometryTool::Smooth { tangent: true }
    );
    s.lock().unwrap().finish(&mut d).unwrap();
    let before = d.to_json().unwrap();
    assert_eq!(
        host::apply_shared(&mut d, &s, input()).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert_eq!(d.to_json().unwrap(), before);
}

#[test]
fn tangent_change_releases_old_queue_before_committing_new_preference() {
    let mut d = doc();
    let old = prefs(false);
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
    let new = prefs(true);
    assert!(!source_geometry_ui::same_words(old, new));
    let result = source_geometry_ui::commit_candidate(old, new, || {
        let result = q.release_source_geometry(&mut d);
        assert!(result.changed && result.error.is_none());
        s.lock().unwrap().finish(&mut d).is_ok()
    })
    .unwrap();
    assert!(matches!(result,GeometryCommit::Accepted(p) if p==new));
    assert!(q.is_empty());
    assert_eq!(
        d.require_active_mesh().unwrap().positions[0]
            .to_array()
            .map(f32::to_bits),
        [0x3e000000, 0, 0x3f200000]
    );
}

#[test]
fn rejected_release_and_invalid_or_unchanged_tangent_candidates_do_not_commit() {
    let old = prefs(false);
    let calls = Cell::new(0);
    assert!(matches!(
        source_geometry_ui::commit_candidate(old, prefs(true), || {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap(),
        GeometryCommit::ReleaseRejected
    ));
    assert_eq!(calls.get(), 1);
    assert!(matches!(
        source_geometry_ui::commit_candidate(old, old, || {
            calls.set(calls.get() + 1);
            true
        })
        .unwrap(),
        GeometryCommit::Unchanged
    ));
    let bad = SourceGeometryPreferences {
        options: SourceGeometryOptions {
            negative: true,
            ..old.options
        },
        ..old
    };
    assert!(
        source_geometry_ui::commit_candidate(old, bad, || {
            calls.set(calls.get() + 1);
            true
        })
        .is_err()
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn actual_narrow_and_wide_tangent_checkbox_response_is_visible_and_edits_real_candidate() {
    for width in [160.0, 360.0] {
        let ctx = egui::Context::default();
        super::super::configure_theme(&ctx);
        let mut tangent = false;
        let mut response = None;
        let mut time = 0.0;
        let mut frame = |events: Vec<Event>| {
            time += 1.0 / 60.0;
            let mut changed = false;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(width + 16.0, 200.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.set_width(width);
                        let r = smooth_tangent_control(ui, &mut tangent);
                        assert!(r.enabled());
                        assert!(ui.clip_rect().contains_rect(r.rect));
                        changed = r.changed();
                        response = Some(r);
                    });
                },
            );
            (changed, response.as_ref().unwrap().interact_rect.center())
        };
        let (_, point) = frame(vec![]);
        for pressed in [true, false] {
            let (changed, _) = frame(vec![
                Event::PointerMoved(point),
                Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ]);
            if !pressed {
                assert!(changed);
            }
        }
        assert!(tangent);
        assert!(response.unwrap().id != egui::Id::NULL);
        let old = prefs(false);
        assert!(!source_geometry_ui::same_words(old, prefs(tangent)));
    }
}

#[test]
fn invalid_precise_input_rolls_back_owned_smooth_and_restores_prior_real_redo() {
    let mut d = doc();
    d.begin_stroke();
    d.name = "real future".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let global = tracks();
    let prior = serde_json::to_value(global.lock().unwrap().snapshot()).unwrap();
    let s = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        prefs(false),
        input(),
        global.clone(),
    )
    .unwrap();
    assert_eq!(
        host::apply_shared(&mut d, &s, input())
            .unwrap()
            .changed_vertices,
        4
    );
    let mut bad = input();
    bad.view.viewport[2] = 0.0;
    assert!(host::apply_shared(&mut d, &s, bad).is_err());
    s.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(
        serde_json::to_value(global.lock().unwrap().snapshot()).unwrap(),
        prior
    );
    assert!(global.lock().unwrap().active_claim().is_none());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), future);
}

#[test]
fn temporary_shift_kind_resolves_the_genuine_saved_smooth_row_without_overwriting_draw() {
    let mut store = GeometryPreferenceStore::default();
    store.set(BrushKind::Smooth, prefs(true)).unwrap();
    let draw = store.get(BrushKind::Draw).unwrap();
    let effective = super::super::effective_brush_kind(BrushKind::Draw, Modifiers::SHIFT);
    assert_eq!(effective, BrushKind::Smooth);
    let saved = store.get(effective).unwrap();
    let seed = BrushSettings {
        kind: BrushKind::Draw,
        strength: 0.35,
        symmetry_x: false,
        adaptive_detail: false,
        ..Default::default()
    };
    let mut legacy =
        crate::brush_preferences::BrushPreferenceStore::from_legacy(&seed, None, false).unwrap();
    let mods = super::super::temporary_modifiers(BrushKind::Draw, effective);
    let contact = crate::brush_preferences::ContactInput::Pen(0.25);
    let idle = legacy.preview(mods, contact, None, None).unwrap();
    let gesture = legacy.begin_gesture(mods).unwrap();
    let pressed = legacy.sample(gesture.token(), contact, None, None).unwrap();
    assert_eq!(idle.settings.kind, BrushKind::Smooth);
    assert_eq!(pressed.settings.kind, BrushKind::Smooth);
    assert_eq!(
        serde_json::to_value(&idle.settings).unwrap(),
        serde_json::to_value(&pressed.settings).unwrap()
    );
    assert_eq!(idle.pressure, pressed.pressure);
    let mut actual_input = input();
    actual_input.raw = RawPressureSample {
        pressure: 0.25,
        pen: true,
        ..Default::default()
    };
    let mut d = doc();
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &pressed.settings,
        false,
        saved,
        actual_input,
        tracks(),
    )
    .unwrap();
    store.set(BrushKind::Smooth, prefs(false)).unwrap();
    assert_eq!(session.lock().unwrap().preferences(), prefs(true));
    session.lock().unwrap().finish(&mut d).unwrap();
    legacy.end_gesture(gesture.token()).unwrap();
    assert_eq!(legacy.selected(), BrushKind::Draw);
    assert_eq!(store.get(BrushKind::Draw).unwrap(), draw);
    assert_eq!(
        super::super::effective_brush_kind(BrushKind::Draw, Modifiers::NONE),
        BrushKind::Draw
    );
}

#[test]
fn prevent_fold_setting_change_finishes_old_queue_before_captured_option_commit() {
    let mut d = doc();
    let old = prefs(false);
    assert!(old.options.prevent_folds);
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        old,
        input(),
        tracks(),
    )
    .unwrap();
    let mut queue = DabQueue::default();
    queue.source_geometry_dab(session.clone(), input());
    let mut new = old;
    new.options.prevent_folds = false;
    assert!(!source_geometry_ui::same_words(old, new));
    let released = Cell::new(0);
    let committed = source_geometry_ui::commit_candidate(old, new, || {
        released.set(released.get() + 1);
        let report = queue.release_source_geometry(&mut d);
        assert!(report.changed && report.error.is_none());
        session.lock().unwrap().finish(&mut d).is_ok()
    })
    .unwrap();
    assert!(matches!(committed, GeometryCommit::Accepted(p) if p==new));
    assert_eq!(released.get(), 1);
    assert!(queue.is_empty());
    assert!(session.lock().unwrap().preferences().options.prevent_folds);
    assert_eq!(
        host::apply_shared(&mut d, &session, input()).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    // The old all-four .375 pass moved the tetra into x>=.125. Use the
    // analytically projected interior barycentric point [.5,.25,.25] of face0:
    // world [.25,.125,.375], with this fixture's fixed .25 orthographic matrix.
    let next_input = host::SourceGeometryInput {
        screen: [53.125, 48.4375],
        ..input()
    };
    let next = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        new,
        next_input,
        tracks(),
    )
    .unwrap();
    assert!(!next.lock().unwrap().preferences().options.prevent_folds);
    next.lock().unwrap().finish(&mut d).unwrap();
}

#[test]
fn live_inverted_smooth_refusal_is_specific_atomic_and_preserves_real_redo() {
    for tangent in [false, true] {
        for controlled in [false, true] {
            let mut d = doc();
            d.begin_stroke();
            d.name = "real future before inverted attempt".into();
            d.end_stroke();
            let future = d.to_json().unwrap();
            assert!(d.undo());
            let before = d.to_json().unwrap();
            let mut inverted = settings();
            inverted.invert = true;
            let global = tracks();
            let prior_tracks = serde_json::to_value(global.lock().unwrap().snapshot()).unwrap();
            let result = if controlled {
                SourceGeometrySession::begin_with_controls(
                    &mut d,
                    &inverted,
                    false,
                    prefs(tangent),
                    input(),
                    global.clone(),
                )
            } else {
                SourceGeometrySession::begin(&mut d, &inverted, false, prefs(tangent), input())
            };
            let error = match result {
                Ok(_) => panic!("Inverted Smooth must refuse before acquiring a session"),
                Err(error) => error,
            };
            assert_eq!(error, SourceGeometryError::UnsupportedSmoothNegative);
            assert_eq!(d.to_json().unwrap(), before);
            assert_eq!(d.source_geometry_retained_bytes(), 0);
            assert!(d.source_geometry_active_claim().is_none());
            assert!(global.lock().unwrap().active_claim().is_none());
            assert_eq!(
                serde_json::to_value(global.lock().unwrap().snapshot()).unwrap(),
                prior_tracks
            );
            assert!(d.redo());
            assert_eq!(d.to_json().unwrap(), future);
        }
    }
}
