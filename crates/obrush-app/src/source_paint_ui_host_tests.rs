//! H01–H04: actual egui intents drive the same preference commit barrier used
//! by the app, with the real Source session/DabQueue/Document history path.
//! H05 is separately authored browser protocol, not an additional Rust test.
use crate::dab_queue::DabQueue;
use crate::source_paint_host::{
    PaintMode, RawPressureSample, SourcePaintInput, SourcePaintPreferences, SourcePaintSession,
};
use crate::source_paint_ui::{
    NumericPaintUi, PreferenceCommit, SourcePaintEditorDrafts, commit_candidate,
    render_numeric_preferences,
};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
use obrush_core::{
    BrushKind, BrushSettings, Document, MaskConvention, Mesh, PaintResponseOptions,
    PaintViewCapture,
};

struct Editor {
    ctx: egui::Context,
    prefs: SourcePaintPreferences,
    drafts: SourcePaintEditorDrafts,
    time: f64,
}
impl Editor {
    fn new(prefs: SourcePaintPreferences) -> Self {
        let ctx = egui::Context::default();
        super::configure_theme(&ctx);
        Self {
            ctx,
            prefs,
            drafts: Default::default(),
            time: 0.0,
        }
    }
    fn frame(&mut self, events: Vec<Event>) -> NumericPaintUi {
        self.time += 1.0 / 60.0;
        let mut view = None;
        let _ = self.ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(176.0, 1300.0))),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(160.0);
                    view = Some(render_numeric_preferences(
                        ui,
                        &self.prefs,
                        &mut self.drafts,
                    ));
                });
            },
        );
        view.unwrap()
    }
    fn enter(&mut self, key: &str, text: &str) -> NumericPaintUi {
        let view = self.frame(Vec::new());
        let response = &view.controls[key];
        assert!(response.enabled() && response.interact_rect.is_positive());
        let point = response.interact_rect.center();
        for pressed in [true, false] {
            let _ = self.frame(vec![
                Event::PointerMoved(point),
                Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ]);
        }
        assert!(self.ctx.memory(|m| m.focused()).is_some());
        let command = Modifiers {
            command: true,
            mac_cmd: true,
            ..Default::default()
        };
        for pressed in [true, false] {
            let _ = self.frame(vec![Event::Key {
                key: Key::A,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: command,
            }]);
        }
        assert!(
            self.frame(vec![Event::Text(text.to_owned())])
                .candidate
                .is_none()
        );
        self.frame(vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }])
    }
}
fn prefs(intensity: f64) -> SourcePaintPreferences {
    SourcePaintPreferences {
        mode: PaintMode::SourceOrdinary,
        radius: 5.0,
        radius_factor: 0.0,
        mirror_offset: 0.0,
        options: PaintResponseOptions {
            intensity,
            hardness: 1.0,
            color: [1.0, 0.5, 0.25],
            write_roughness: false,
            write_metallic: false,
            culling: false,
            ..Default::default()
        },
    }
}
fn document() -> Document {
    // Independent literal3-point plane; exactvertex0 projects to25,75 under
    // identity100². This is a local queue/history contract, not browser contact.
    let mut mesh = Mesh::import_obj(b"v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n")
        .unwrap()
        .mesh;
    mesh.colors.fill([0.0; 3]);
    mesh.install_mask_channel(vec![1.0; 3], MaskConvention::UnprotectedF32V1)
        .unwrap();
    Document::new(mesh)
}
fn input() -> SourcePaintInput {
    SourcePaintInput {
        view: PaintViewCapture {
            world_to_clip: glam::DMat4::IDENTITY.to_cols_array(),
            viewport: [0.0, 0.0, 100.0, 100.0],
        },
        screen: [25.0, 75.0],
        raw: RawPressureSample::default(),
        in_contact: true,
    }
}
fn session(
    doc: &Document,
    p: &SourcePaintPreferences,
) -> crate::source_paint_host::SharedSourcePaint {
    SourcePaintSession::prepare(
        doc,
        p,
        &BrushSettings {
            kind: BrushKind::Paint,
            symmetry_x: false,
            adaptive_detail: false,
            ..Default::default()
        },
        false,
        input(),
    )
    .unwrap()
}
fn known_redo(doc: &mut Document) -> String {
    let mut q = DabQueue::default();
    let captured = session(doc, &prefs(0.25));
    q.begin();
    q.source_paint_dab(captured, input());
    let progress = q.release(doc);
    assert!(progress.changed && progress.painted && progress.error.is_none());
    assert!(doc.take_layer_error().is_none());
    let known = doc.to_json().unwrap();
    assert!(doc.undo());
    known
}

#[test]
fn h01_default_and_idle_normal_ui_never_change_document_or_source_float_words() {
    let doc = document();
    let before = doc.to_json().unwrap();
    let mut legacy = Editor::new(SourcePaintPreferences::default());
    let initial_rgb = legacy.prefs.options.color.map(f32::to_bits);
    assert_eq!(initial_rgb, [0x3f800000, 0x3f441893, 0x3eac0831]);
    let view = legacy.frame(Vec::new());
    assert!(view.controls.is_empty() && view.candidate.is_none() && view.rejected.is_none());
    assert!(matches!(
        commit_candidate(view.candidate, || panic!("idle must not release history")),
        PreferenceCommit::Unchanged
    ));
    assert_eq!(legacy.prefs.mode, PaintMode::Legacy);
    legacy.prefs.mode = PaintMode::SourceOrdinary;
    for _ in 0..8 {
        let view = legacy.frame(Vec::new());
        assert!(view.candidate.is_none() && view.rejected.is_none());
        assert_eq!(legacy.prefs.options.color.map(f32::to_bits), initial_rgb);
        assert_eq!(doc.to_json().unwrap(), before);
    }
    assert!(doc.require_active_mesh().unwrap().materials.is_none());
}

#[test]
fn h02_actual_editor_commit_releases_old_captured_queue_once_before_new_preferences() {
    let mut doc = document();
    let original = doc.to_json().unwrap();
    let mut editor = Editor::new(prefs(0.75));
    let old = session(&doc, &editor.prefs);
    let mut q = DabQueue::default();
    q.begin();
    q.source_paint_dab(old.clone(), input());
    let intent = editor.enter("source_paint.intensity.value", "0.25");
    assert!(intent.rejected.is_none());
    assert_eq!(editor.prefs.options.intensity, 0.75);
    assert_eq!(old.lock().unwrap().preferences.options.intensity, 0.75);
    let mut releases = 0;
    let commit = commit_candidate(intent.candidate, || {
        releases += 1;
        let p = q.release(&mut doc);
        p.error.is_none() && doc.take_layer_error().is_none()
    });
    editor.prefs = match commit {
        PreferenceCommit::Accepted(p) => p,
        _ => panic!("valid old release/new candidate required"),
    };
    assert_eq!(releases, 1);
    assert_eq!(editor.prefs.options.intensity, 0.25);
    assert_eq!(old.lock().unwrap().preferences.options.intensity, 0.75);
    assert_eq!(
        doc.require_active_mesh().unwrap().colors[0].map(f32::to_bits),
        [0x3f400000, 0x3ec00000, 0x3e400000]
    );
    assert!(
        doc.require_active_mesh().unwrap().colors[1..]
            .iter()
            .all(|c| *c == [0.0; 3])
    );
    assert!(doc.require_active_mesh().unwrap().materials.is_none());
    assert!(q.is_empty());
    assert_eq!(q.metrics()["released_strokes"], 1);
    assert_eq!(q.metrics()["processed_dabs"], 1);
    let saved = doc.to_json().unwrap();
    assert!(doc.undo());
    assert_eq!(doc.to_json().unwrap(), original);
    assert!(!doc.undo());
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), saved);
    assert!(!doc.redo());
    let next = session(&doc, &editor.prefs);
    assert_eq!(
        next.lock().unwrap().diagnostics()["captured"]["intensity"],
        0.25
    );
}

#[test]
fn h03_actual_invalid_editor_does_not_release_or_allocate_or_destroy_genuine_redo() {
    let mut doc = document();
    let original = doc.to_json().unwrap();
    let known = known_redo(&mut doc);
    assert_eq!(doc.to_json().unwrap(), original);
    let mut editor = Editor::new(prefs(0.75));
    let old_color = editor.prefs.options.color.map(f32::to_bits);
    let intent = editor.enter("source_paint.color.g.value", "NaN");
    assert!(intent.rejected.is_some() && intent.candidate.is_none());
    assert!(matches!(
        commit_candidate(intent.candidate, || panic!(
            "invalid numeric intent must not release"
        )),
        PreferenceCommit::Unchanged
    ));
    assert_eq!(editor.prefs.options.color.map(f32::to_bits), old_color);
    assert_eq!(doc.to_json().unwrap(), original);
    assert!(doc.require_active_mesh().unwrap().materials.is_none());
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), known);
}

#[test]
fn h04_real_old_stroke_rejection_keeps_preferences_restores_drafts_and_preserves_redo() {
    let mut doc = document();
    let original = doc.to_json().unwrap();
    let known = known_redo(&mut doc);
    let mut editor = Editor::new(prefs(0.75));
    let old = session(&doc, &editor.prefs);
    let mut q = DabQueue::default();
    q.begin();
    let begin = q.process(&mut doc, 1, None);
    assert!(begin.error.is_none());
    // Deliberately malformed in-stroke external geometry exercises the real
    // existing Document commit rejection/restoration; no product guard changes.
    doc.require_active_mesh_mut().unwrap().positions[0].x = f32::NAN;
    q.source_paint_dab(old, input());
    let intent = editor.enter("source_paint.intensity.value", "0.25");
    assert!(intent.rejected.is_none() && intent.candidate.is_some());
    let mut releases = 0;
    let commit = commit_candidate(intent.candidate, || {
        releases += 1;
        let progress = q.release(&mut doc);
        let commit_error = doc.take_layer_error();
        progress.error.is_none() && commit_error.is_none()
    });
    assert!(matches!(commit, PreferenceCommit::ReleaseRejected));
    editor.drafts.reset(&editor.prefs);
    assert_eq!(editor.prefs.options.intensity, 0.75);
    assert_eq!(
        editor.drafts.text_for("source_paint.intensity.value"),
        Some("0.75")
    );
    assert_eq!(releases, 1);
    assert!(q.is_empty());
    assert_eq!(doc.to_json().unwrap(), original);
    assert!(doc.require_active_mesh().unwrap().materials.is_none());
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), known);
}
