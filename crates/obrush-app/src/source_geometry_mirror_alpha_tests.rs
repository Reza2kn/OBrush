//! Actual numeric widgets and shared normal host/queue; GL/browser remains UNRUN.
use super::super::source_geometry_app::GeometryPreferenceStore;
use super::*;
use crate::dab_queue::DabQueue;
use crate::source_geometry_host::{self as host, SourceGeometryMode, SourceGeometrySession};
use crate::source_paint_host::RawPressureSample;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use obrush_core::{
    BrushKind, BrushSettings, Document, MaskConvention, Mesh, PaintViewCapture, RasterAlpha,
    SourceGeometryError, SourceGeometryTool,
};
use std::cell::Cell;
use std::sync::{Arc, Mutex};

fn document() -> Document {
    let mut m = Mesh::from_obj(
        "v 0 0 0\nv -4 -4 0\nv 4 -4 0\nv 4 4 0\nv -4 4 0\nf 1 2 3\nf 1 3 4\nf 1 4 5\nf 1 5 2\n",
    )
    .unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.0);
    Document::new(m)
}
fn preferences() -> SourceGeometryPreferences {
    let mut p = GeometryPreferenceStore::default()
        .get(BrushKind::Draw)
        .unwrap();
    p.mode = SourceGeometryMode::SourceOrdinary;
    p.radius = 25.0;
    p.radius_factor = 0.0;
    p.options.tool = SourceGeometryTool::Brush {
        clay: false,
        accumulate: false,
    };
    p.options.intensity = 0.625;
    p
}
fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Draw,
        symmetry_x: true,
        adaptive_detail: false,
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
fn track_words(t: &host::SharedSourceGeometryAlphaTracks) -> serde_json::Value {
    serde_json::to_value(t.lock().unwrap().snapshot()).unwrap()
}

#[test]
fn additive_control_admission_keeps_old_refusal_scope() {
    let d = document();
    let mut s = settings();
    s.alpha = Some(RasterAlpha::new(1, 1, vec![128]).unwrap());
    assert!(host::blocker_controls(&d, &s, false, preferences()).is_ok());
    assert_eq!(
        host::blocker(&d, &s, false, preferences()).unwrap_err(),
        SourceGeometryError::UnsupportedDomain
    );
}
#[test]
fn manual_rotation_absolute_offset_lock_and_invalid_new_offset_reject() {
    let d = document();
    let mut s = settings();
    s.alpha_rotation = 0.25;
    assert!(host::blocker_controls(&d, &s, false, preferences()).is_err());
    s.alpha_rotation = 0.0;
    s.symmetry_offset = 0.25;
    assert!(host::blocker_controls(&d, &s, false, preferences()).is_err());
    s.symmetry_offset = 0.0;
    assert!(host::blocker_controls(&d, &s, true, preferences()).is_err());
    for offset in [f64::NAN, f64::INFINITY, 1.01] {
        let mut p = preferences();
        p.mirror_offset = offset;
        assert!(p.validate().is_err());
    }
}
#[test]
fn mirror_offset_per_tool_and_same_words_barrier_are_independent() {
    let mut store = GeometryPreferenceStore::default();
    let draw = store.get(BrushKind::Draw).unwrap();
    let inflate = store.get(BrushKind::Inflate).unwrap();
    let mut clay = store.get(BrushKind::Clay).unwrap();
    clay.mirror_offset = -0.25;
    store.set(BrushKind::Clay, clay).unwrap();
    assert_eq!(store.get(BrushKind::Draw).unwrap(), draw);
    assert_eq!(store.get(BrushKind::Inflate).unwrap(), inflate);
    let n = Cell::new(0);
    assert!(matches!(
        commit_candidate(clay, clay, || {
            n.set(n.get() + 1);
            true
        })
        .unwrap(),
        GeometryCommit::Unchanged
    ));
    assert_eq!(n.get(), 0);
    let mut candidate = clay;
    candidate.mirror_offset = 0.25;
    assert!(!same_words(clay, candidate));
    assert!(matches!(
        commit_candidate(clay, candidate, || {
            n.set(n.get() + 1);
            false
        })
        .unwrap(),
        GeometryCommit::ReleaseRejected
    ));
    assert_eq!(n.get(), 1);
    assert_eq!(store.get(BrushKind::Clay).unwrap(), clay);
}
struct NumericFixture {
    ctx: egui::Context,
    drafts: GeometryEditorDrafts,
    prefs: SourceGeometryPreferences,
    width: f32,
    time: f64,
}
impl NumericFixture {
    fn new(width: f32) -> Self {
        let ctx = egui::Context::default();
        super::super::configure_theme(&ctx);
        Self {
            ctx,
            drafts: Default::default(),
            prefs: preferences(),
            width,
            time: 0.0,
        }
    }
    fn frame(&mut self, events: Vec<Event>) -> (GeometryNumericUi, Rect) {
        self.time += 1.0 / 60.0;
        let mut out = None;
        let _ = self.ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(self.width + 16.0, 950.0),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(self.width);
                    out = Some((
                        render_mirror_offset(ui, self.prefs, &mut self.drafts),
                        ui.clip_rect(),
                    ));
                });
            },
        );
        out.unwrap()
    }
    fn text(&mut self, text: &str) {
        let p = self.frame(Vec::new()).0.controls["source_geometry.mirror.offset.value"]
            .interact_rect
            .center();
        for pressed in [true, false] {
            self.frame(vec![
                Event::PointerMoved(p),
                Event::PointerButton {
                    pos: p,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ]);
        }
        let command = Modifiers {
            command: true,
            mac_cmd: true,
            ..Default::default()
        };
        for pressed in [true, false] {
            self.frame(vec![Event::Key {
                key: Key::A,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: command,
            }]);
        }
        assert!(
            self.frame(vec![Event::Text(text.into())])
                .0
                .candidate
                .is_none()
        );
    }
    fn key(&mut self, key: Key) -> GeometryNumericUi {
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }])
        .0
    }
}
#[test]
fn actual_offset_editor_and_rail_fit_narrow_and_wide_without_idle_write() {
    for width in [160.0, 280.0] {
        let mut f = NumericFixture::new(width);
        for _ in 0..3 {
            let (out, clip) = f.frame(Vec::new());
            assert!(out.candidate.is_none());
            for key in [
                "source_geometry.mirror.offset",
                "source_geometry.mirror.offset.value",
            ] {
                let r = &out.controls[key];
                assert!(r.enabled());
                assert!(r.interact_rect.is_positive());
                assert!(clip.contains_rect(r.interact_rect));
            }
        }
    }
}
#[test]
fn actual_offset_enter_commits_only_declared_f64_field() {
    let mut f = NumericFixture::new(160.0);
    let before = f.prefs;
    f.text("-0.25");
    let out = f.key(Key::Enter);
    let p = out.candidate.expect("Normal dirty Enter must commit");
    assert_eq!(p.mirror_offset.to_bits(), (-0.25f64).to_bits());
    let mut restored = p;
    restored.mirror_offset = before.mirror_offset;
    assert!(same_words(restored, before));
}
#[test]
fn actual_offset_invalid_and_escape_preserve_saved_words_and_release() {
    for text in ["NaN", "1.1", "-inf"] {
        let mut f = NumericFixture::new(160.0);
        let before = f.prefs;
        f.text(text);
        let out = f.key(Key::Enter);
        assert!(out.candidate.is_none());
        assert!(out.rejected.is_some());
        assert!(same_words(f.prefs, before));
    }
    let mut f = NumericFixture::new(160.0);
    let before = f.prefs;
    f.text("0.75");
    assert!(f.key(Key::Escape).candidate.is_none());
    assert!(f.frame(Vec::new()).0.candidate.is_none());
    let n = Cell::new(0);
    commit_candidate(before, f.prefs, || {
        n.set(n.get() + 1);
        true
    })
    .unwrap();
    assert_eq!(n.get(), 0);
}
#[test]
fn actual_captured_mirror_queue_owns_one_useful_whole_stroke() {
    let mut d = document();
    let before = d.to_json().unwrap();
    let global = tracks();
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        preferences(),
        input(),
        global,
    )
    .unwrap();
    let mut q = DabQueue::default();
    q.source_geometry_dab(session.clone(), input());
    let r = q.release_source_geometry(&mut d);
    assert!(r.changed && r.error.is_none());
    assert_eq!(r.dabs, 1);
    assert!(q.is_empty());
    session.lock().unwrap().finish(&mut d).unwrap();
    let lock = session.lock().unwrap();
    let report = lock.last_controlled.as_ref().unwrap();
    assert_eq!(report.contacts[0].unwrap().center, [0.5, 0.0, 0.0]);
    assert_eq!(report.contacts[1].unwrap().center, [-0.5, 0.0, 0.0]);
    assert_eq!(report.passes[0].as_ref().unwrap().changed_vertices, 1);
    assert_eq!(report.passes[1].as_ref().unwrap().changed_vertices, 1);
    drop(lock);
    assert_eq!(
        d.active_mesh().unwrap().positions[0].z.to_bits(),
        0x3e730000
    );
    let after = d.to_json().unwrap();
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), after);
}
#[test]
fn alpha_bitmap_and_mirror_capture_survive_caller_edits() {
    let mut d = document();
    let mut s = settings();
    s.alpha = Some(RasterAlpha::new(1, 1, vec![128]).unwrap());
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &s,
        false,
        preferences(),
        input(),
        tracks(),
    )
    .unwrap();
    s.alpha = Some(RasterAlpha::new(1, 1, vec![0]).unwrap());
    s.symmetry_x = false;
    assert!(!s.symmetry_x && !s.alpha.as_ref().unwrap().has_signal());
    let mut q = DabQueue::default();
    q.source_geometry_dab(session.clone(), input());
    let r = q.release_source_geometry(&mut d);
    assert!(r.changed && r.error.is_none());
    let diag = session.lock().unwrap().diagnostics();
    assert_eq!(diag["captured_controls"]["mirror"], true);
    assert_eq!(diag["captured_controls"]["alpha"]["has_signal"], true);
    assert_eq!(
        diag["controlled"]["alpha_dimensions"],
        serde_json::json!([1, 1])
    );
    session.lock().unwrap().finish(&mut d).unwrap();
}
#[test]
fn accepted_then_rejected_input_restores_geometry_tracks_and_real_redo() {
    let mut d = document();
    d.begin_stroke();
    d.name = "genuine future".into();
    d.end_stroke();
    let future = d.to_json().unwrap();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let global = tracks();
    let initial = track_words(&global);
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        preferences(),
        input(),
        global.clone(),
    )
    .unwrap();
    assert!(
        session
            .lock()
            .unwrap()
            .apply(&mut d, input())
            .unwrap()
            .changed_vertices
            > 0
    );
    let mut invalid = input();
    invalid.view.viewport[2] = 0.0;
    assert!(session.lock().unwrap().apply(&mut d, invalid).is_err());
    session.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!(track_words(&global), initial);
    assert!(d.redo());
    assert_eq!(d.to_json().unwrap(), future);
}
#[test]
fn global_source_tracks_survive_closed_noop_and_next_admitted_tool() {
    let mut d = document();
    let global = tracks();
    let mut s = settings();
    s.symmetry_x = false;
    let first = SourceGeometrySession::begin_with_controls(
        &mut d,
        &s,
        false,
        preferences(),
        input(),
        global.clone(),
    )
    .unwrap();
    first.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(
        global.lock().unwrap().snapshot().primary.origin,
        [0.5, 0.0, 0.0]
    );
    assert!(!d.can_undo());
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    s.kind = BrushKind::Inflate;
    let mut p = SourceGeometryPreferences::inflate();
    p.mode = SourceGeometryMode::SourceOrdinary;
    p.radius = 25.0;
    p.radius_factor = 0.0;
    let mut next = input();
    next.screen[0] = 57.8125;
    let second =
        SourceGeometrySession::begin_with_controls(&mut d, &s, false, p, next, global.clone())
            .unwrap();
    assert_eq!(
        global.lock().unwrap().snapshot().primary.origin,
        [0.625, 0.0, 0.0]
    );
    second.lock().unwrap().finish(&mut d).unwrap();
    assert!(!d.can_undo());
}
#[test]
fn no_contact_keeps_tracks_and_release_has_no_timer_tail() {
    let mut d = document();
    let global = tracks();
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        preferences(),
        input(),
        global.clone(),
    )
    .unwrap();
    let before = track_words(&global);
    let mut none = input();
    none.raw.pen = true;
    none.raw.pressure = 0.0;
    none.raw.absent = false;
    none.in_contact = false;
    let mut q = DabQueue::default();
    q.source_geometry_dab(session.clone(), none);
    let r = q.release_source_geometry(&mut d);
    assert_eq!(r.dabs, 1);
    assert!(!r.changed && r.error.is_none());
    assert!(q.is_empty());
    session.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(track_words(&global), before);
    assert!(!d.can_undo());
}
#[test]
fn normalized_offset_change_finishes_old_captured_queue_before_install() {
    let mut d = document();
    let old = preferences();
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &settings(),
        false,
        old,
        input(),
        tracks(),
    )
    .unwrap();
    let mut q = DabQueue::default();
    q.source_geometry_dab(session.clone(), input());
    let mut candidate = old;
    candidate.mirror_offset = 0.25;
    let n = Cell::new(0);
    let commit = commit_candidate(old, candidate, || {
        n.set(n.get() + 1);
        let progress = q.release_source_geometry(&mut d);
        assert!(progress.error.is_none());
        session.lock().unwrap().finish(&mut d).is_ok()
    })
    .unwrap();
    assert_eq!(n.get(), 1);
    let GeometryCommit::Accepted(next) = commit else {
        panic!("Real old-setting barrier must complete");
    };
    assert_eq!(next.mirror_offset, 0.25);
    assert_eq!(session.lock().unwrap().preferences().mirror_offset, 0.0);
    assert!(q.is_empty());
    assert!(d.can_undo());
}
#[test]
fn wrong_document_finish_cancel_cannot_release_identical_owned_tracks() {
    let mut owner = document();
    let mut foreign = document();
    let before_foreign = foreign.to_json().unwrap();
    let global = tracks();
    let session = SourceGeometrySession::begin_with_controls(
        &mut owner,
        &settings(),
        false,
        preferences(),
        input(),
        global.clone(),
    )
    .unwrap();
    let before = track_words(&global);
    let claim = global.lock().unwrap().active_claim();
    assert!(claim.is_some());
    assert_eq!(
        session
            .lock()
            .unwrap()
            .apply(&mut foreign, input())
            .unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert!(session.lock().unwrap().failed.is_none());
    assert_eq!(
        session.lock().unwrap().finish(&mut foreign).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert_eq!(
        session.lock().unwrap().cancel(&mut foreign).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert_eq!(global.lock().unwrap().active_claim(), claim);
    assert_eq!(track_words(&global), before);
    assert_eq!(foreign.to_json().unwrap(), before_foreign);
    assert!(
        session
            .lock()
            .unwrap()
            .apply(&mut owner, input())
            .unwrap()
            .changed_vertices
            > 0
    );
    session.lock().unwrap().finish(&mut owner).unwrap();
    assert!(global.lock().unwrap().active_claim().is_none());
    let newer = SourceGeometrySession::begin_with_controls(
        &mut foreign,
        &settings(),
        false,
        preferences(),
        input(),
        global.clone(),
    )
    .unwrap();
    let newer_claim = global.lock().unwrap().active_claim();
    let newer_words = track_words(&global);
    assert_ne!(newer_claim, claim);
    assert_eq!(
        session.lock().unwrap().cancel(&mut owner).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert_eq!(global.lock().unwrap().active_claim(), newer_claim);
    assert_eq!(track_words(&global), newer_words);
    newer.lock().unwrap().cancel(&mut foreign).unwrap();
}
#[test]
fn zero_byte_stamp_updates_tracks_without_mesh_cache_or_undo() {
    let mut d = document();
    let before = d.to_json().unwrap();
    let global = tracks();
    let mut s = settings();
    s.alpha = Some(RasterAlpha::new(1, 1, vec![0]).unwrap());
    let session = SourceGeometrySession::begin_with_controls(
        &mut d,
        &s,
        false,
        preferences(),
        input(),
        global.clone(),
    )
    .unwrap();
    let mut moved = input();
    moved.screen[0] = 57.8125;
    let r = session.lock().unwrap().apply(&mut d, moved).unwrap();
    assert_eq!(r.changed_vertices, 0);
    assert_eq!(
        global.lock().unwrap().snapshot().primary.origin,
        [0.625, 0.0, 0.0]
    );
    assert_eq!(d.source_geometry_retained_bytes(), 0);
    session.lock().unwrap().finish(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
}
