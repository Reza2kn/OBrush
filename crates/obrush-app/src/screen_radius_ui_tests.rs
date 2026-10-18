//! NUM1..4 exercise actual egui inputs and existing host preference/history APIs.
//! They do not claim browser, GPU, physical pen, or reference execution.
use crate::screen_radius_ui::{self, KEY, NumericRadiusUi, ScreenRadiusDraft};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect};
use obrush_core::{BrushKind, Document, Mesh, Ray};

struct View {
    numeric: NumericRadiusUi,
    slider: egui::Response,
    blur: egui::Response,
    clip: Rect,
}
struct Editor {
    ctx: egui::Context,
    draft: ScreenRadiusDraft,
    owner: BrushKind,
    accepted: f32,
    enabled: bool,
    width: f32,
    time: f64,
}
impl Editor {
    fn new(width: f32) -> Self {
        let ctx = egui::Context::default();
        super::configure_theme(&ctx);
        Self {
            ctx,
            draft: Default::default(),
            owner: BrushKind::Draw,
            accepted: 75.0,
            enabled: true,
            width,
            time: 0.0,
        }
    }
    fn frame(&mut self, events: Vec<Event>) -> View {
        self.time += 1.0 / 60.0;
        self.draft.begin_frame();
        let mut out = None;
        let _ = self.ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(self.width + 16.0, 220.0),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(self.width);
                    ui.spacing_mut().slider_width = ui.available_width();
                    let clip = ui.clip_rect();
                    let slider = ui.add_enabled(
                        self.enabled,
                        egui::Slider::new(&mut self.accepted, 5.0..=500.0).show_value(false),
                    );
                    let numeric = screen_radius_ui::render(
                        ui,
                        self.owner,
                        self.accepted,
                        self.enabled,
                        &mut self.draft,
                    );
                    let blur = ui.button("Next control");
                    out = Some(View {
                        numeric,
                        slider,
                        blur,
                        clip,
                    });
                });
            },
        );
        self.draft
            .finish_frame(&self.ctx, self.owner, self.accepted, self.enabled);
        out.unwrap()
    }
    fn pointer(&mut self, point: Pos2, pressed: bool) -> View {
        self.frame(vec![
            Event::PointerMoved(point),
            Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            },
        ])
    }
    fn key(&mut self, key: Key, pressed: bool, modifiers: Modifiers) -> View {
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        }])
    }
    fn focus(&mut self) {
        let view = self.frame(vec![]);
        assert!(view.numeric.response.enabled());
        assert!(view.clip.contains_rect(view.numeric.response.rect));
        let point = view.numeric.response.interact_rect.center();
        self.pointer(point, true);
        let focused = self.pointer(point, false);
        assert!(
            focused.numeric.response.has_focus(),
            "actual numeric control focus"
        );
    }
    fn type_text(&mut self, text: &str, paste: bool) {
        self.focus();
        let command = Modifiers {
            command: true,
            mac_cmd: true,
            ..Modifiers::NONE
        };
        self.key(Key::A, true, command);
        self.key(Key::A, false, command);
        let event = if text.is_empty() {
            Event::Key {
                key: Key::Backspace,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }
        } else if paste {
            Event::Paste(text.into())
        } else {
            Event::Text(text.into())
        };
        let typed = self.frame(vec![event]);
        if text.is_empty() {
            assert_eq!(
                self.draft.diagnostics()["text"],
                "",
                "actual selection deletion supplies the empty lexical case"
            );
            self.key(Key::Backspace, false, Modifiers::NONE);
        }
        assert!(
            typed.numeric.candidate.is_none(),
            "draft text cannot commit early"
        );
        assert!(
            self.draft.diagnostics()["text"]
                .as_str()
                .unwrap()
                .chars()
                .count()
                <= 64
        );
    }
    fn replace(&mut self, text: &str, paste: bool, terminal: Key) -> NumericRadiusUi {
        self.type_text(text, paste);
        let out = self.key(terminal, true, Modifiers::NONE).numeric;
        self.key(terminal, false, Modifiers::NONE);
        out
    }
}

#[test]
fn num1_actual_pointer_text_enter_blur_and_exact64_visible_narrow_wide() {
    for width in [160.0, 280.0] {
        let mut e = Editor::new(width);
        let idle = e.frame(vec![]);
        assert!(idle.numeric.candidate.is_none());
        assert!(idle.slider.enabled() && idle.clip.contains_rect(idle.slider.rect));
        assert!(idle.clip.contains_rect(idle.numeric.response.rect));
        let out = e.replace("64", false, Key::Enter);
        assert_eq!(out.candidate.unwrap().to_bits(), 0x42800000);
        e.accepted = 64.0;
        let same = e.replace("64.000", false, Key::Enter);
        assert!(
            same.candidate.is_none(),
            "same stored word is a genuine no-op"
        );
        e.accepted = 75.0;
        e.type_text(&format!("64{}", " ".repeat(62)), true);
        let view = e.frame(vec![]);
        let point = view.blur.interact_rect.center();
        let pressed = e.pointer(point, true);
        let released = e.pointer(point, false);
        let candidate = pressed.numeric.candidate.or(released.numeric.candidate);
        assert_eq!(
            candidate.unwrap().to_bits(),
            0x42800000,
            "ordinary focus transfer commits bounded64-character text"
        );
        let mut hits = super::ui_hit_rect::HitRectDiagnostics::default();
        hits.record(KEY, Some(&released.numeric.response));
        assert!(hits.entries()[KEY].is_some());
    }
}

#[test]
fn num2_invalid_oversize_and_escape_preserve_words_project_and_genuine_redo() {
    let mut doc = Document::new(Mesh::sphere(0));
    doc.begin_stroke();
    doc.name = "genuine future".into();
    doc.end_stroke();
    let future = doc.to_json().unwrap();
    assert!(doc.undo());
    let before = doc.to_json().unwrap();
    for text in [
        "NaN",
        "inf",
        "-inf",
        "1e999",
        "4.99999999",
        "500.0000001",
        "",
        "pixels",
    ] {
        let mut e = Editor::new(160.0);
        let out = e.replace(text, false, Key::Enter);
        assert!(out.candidate.is_none(), "invalid lexical input: {text}");
        assert_eq!(e.accepted.to_bits(), 75.0f32.to_bits());
        assert_eq!(e.draft.diagnostics()["invalid"], true);
        assert_eq!(doc.to_json().unwrap(), before);
        assert!(doc.can_redo());
    }
    for paste in [false, true] {
        let mut e = Editor::new(160.0);
        let over = format!("64{}", " ".repeat(63));
        assert_eq!(over.chars().count(), 65);
        let out = e.replace(&over, paste, Key::Enter);
        assert!(
            out.candidate.is_none(),
            "char_limit must not admit truncated valid64 prefix"
        );
        assert_eq!(e.draft.diagnostics()["overlength"], true);
        assert_eq!(e.accepted.to_bits(), 75.0f32.to_bits());
    }
    let mut e = Editor::new(160.0);
    assert!(e.replace("64", false, Key::Escape).candidate.is_none());
    assert_eq!(e.draft.diagnostics()["text"], "75");
    assert_eq!(e.draft.diagnostics()["dirty"], false);
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), future);
}

#[test]
fn num3_actual_owner_mode_external_value_load_and_hidden_boundaries_cancel_draft() {
    for boundary in 0..5 {
        let mut e = Editor::new(160.0);
        e.type_text("64", false);
        match boundary {
            0 => e.owner = BrushKind::Clay,
            1 => e.enabled = false,
            2 => e.accepted = 123.0,
            3 => e.draft.cancel(&e.ctx, e.owner, e.accepted, e.enabled),
            _ => {
                e.draft.begin_frame();
                e.draft.finish_frame(&e.ctx, e.owner, e.accepted, e.enabled);
            }
        }
        let out = e.key(Key::Enter, true, Modifiers::NONE);
        assert!(out.numeric.candidate.is_none(), "stale boundary {boundary}");
        assert_eq!(e.draft.diagnostics()["dirty"], false);
        assert_eq!(e.draft.diagnostics()["text"], e.accepted.to_string());
        assert_eq!(e.draft.diagnostics()["owner"], e.owner.name());
        assert_eq!(out.numeric.response.enabled(), e.enabled);
        e.enabled = true;
        assert!(
            e.key(Key::Enter, false, Modifiers::NONE)
                .numeric
                .candidate
                .is_none()
        );
        assert!(
            e.key(Key::Enter, true, Modifiers::NONE)
                .numeric
                .candidate
                .is_none()
        );
    }
    let mut e = Editor::new(160.0);
    e.type_text("64", false);
    let view = e.frame(vec![]);
    let point = view.slider.interact_rect.right_center() - egui::vec2(1.0, 0.0);
    let press = e.pointer(point, true);
    e.pointer(point, false);
    assert!(press.slider.changed(), "normal original slider input");
    assert_ne!(e.accepted.to_bits(), 75.0f32.to_bits());
    assert_eq!(e.draft.diagnostics()["dirty"], false);
    assert!(
        e.key(Key::Enter, true, Modifiers::NONE)
            .numeric
            .candidate
            .is_none()
    );
}

#[test]
fn num4_numeric64_uses_existing_candidate_and_old_captured_queue_history_barrier() {
    let mut brush = super::initial_brush_settings();
    brush.kind = BrushKind::Paint;
    brush.radius = 2.0;
    brush.symmetry_x = false;
    brush.pressure = 1.0;
    let mut prefs = super::BrushPreferenceStore::from_legacy(&brush, None, false).unwrap();
    let mut saved = prefs.tool(BrushKind::Paint).clone();
    saved.screen_space_radius = true;
    saved.screen_radius_pixels = 75.0;
    prefs.set_tool(BrushKind::Paint, saved).unwrap();
    let captured = prefs
        .preview(
            super::PreferenceModifiers::default(),
            super::ContactInput::Mouse,
            None,
            None,
        )
        .unwrap();
    let mut mesh = Mesh::from_obj("v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n").unwrap();
    mesh.colors.fill([0.0; 3]);
    let mut doc = Document::new(mesh);
    let before = doc.to_json().unwrap();
    let mut q = super::DabQueue::default();
    q.begin();
    q.dab(
        Ray {
            origin: super::Vec3::new(0.0, 0.0, 3.0),
            direction: -super::Vec3::Z,
        },
        captured.settings.clone(),
        super::Vec3::ZERO,
    );
    let mut editor = Editor::new(160.0);
    editor.owner = BrushKind::Paint;
    let value = editor.replace("64", false, Key::Enter).candidate.unwrap();
    assert_eq!(value.to_bits(), 0x42800000);
    let candidate =
        super::preference_view_candidate(&prefs, &captured.settings, true, value, false)
            .unwrap()
            .unwrap();
    assert_eq!(
        prefs.tool(BrushKind::Paint).screen_radius_pixels.to_bits(),
        75.0f32.to_bits()
    );
    assert_eq!(
        captured.screen_radius.base_pixels.to_bits(),
        75.0f32.to_bits()
    );
    assert_eq!(
        doc.to_json().unwrap(),
        before,
        "UI draft/candidate cannot edit the sculpt"
    );
    let release = q.release(&mut doc);
    assert!(release.changed && release.painted && release.error.is_none());
    assert!(q.is_empty());
    let after = doc.to_json().unwrap();
    prefs = candidate;
    assert_eq!(
        prefs.tool(BrushKind::Paint).screen_radius_pixels.to_bits(),
        0x42800000
    );
    assert_eq!(
        captured.screen_radius.base_pixels.to_bits(),
        75.0f32.to_bits()
    );
    prefs.set_selected(BrushKind::Draw).unwrap();
    assert_eq!(
        prefs.tool(BrushKind::Draw).screen_radius_pixels.to_bits(),
        75.0f32.to_bits()
    );
    prefs.set_selected(BrushKind::Paint).unwrap();
    assert_eq!(
        prefs.tool(BrushKind::Paint).screen_radius_pixels.to_bits(),
        0x42800000
    );
    assert!(
        super::preference_view_candidate(&prefs, &captured.settings, true, value, false)
            .unwrap()
            .is_none()
    );
    assert!(
        super::preference_view_candidate(&prefs, &captured.settings, true, f32::NAN, false)
            .is_err()
    );
    assert_eq!(doc.to_json().unwrap(), after);
    assert!(doc.undo());
    assert_eq!(doc.to_json().unwrap(), before);
    assert!(!doc.can_undo());
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), after);
}

#[test]
fn num5_fresh_sameframe_focus_input_bounds_follow_actual_response_ownership() {
    fn frame(
        ctx: &egui::Context,
        draft: &mut ScreenRadiusDraft,
        other: &mut String,
        events: Vec<Event>,
    ) -> (NumericRadiusUi, egui::Response) {
        draft.begin_frame();
        let mut out = None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(176.0, 220.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(160.0);
                    ui.spacing_mut().slider_width = ui.available_width();
                    let mut accepted = 75.0;
                    let _ = ui.add(egui::Slider::new(&mut accepted, 5.0..=500.0).show_value(false));
                    let numeric =
                        screen_radius_ui::render(ui, BrushKind::Draw, accepted, true, draft);
                    let unrelated_id = ui.make_persistent_id("unrelated_numeric_input");
                    let unrelated = ui.add(
                        egui::TextEdit::singleline(other)
                            .id(unrelated_id)
                            .char_limit(64)
                            .desired_width(ui.available_width()),
                    );
                    out = Some((numeric, unrelated));
                });
            },
        );
        draft.finish_frame(ctx, BrushKind::Draw, 75.0, true);
        out.unwrap()
    }
    fn focus_and_insert(point: Pos2, insert: Event) -> Vec<Event> {
        let command = Modifiers {
            command: true,
            mac_cmd: true,
            ..Modifiers::NONE
        };
        vec![
            Event::PointerMoved(point),
            Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
            Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: command,
            },
            Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: command,
            },
            insert,
        ]
    }
    for paste in [false, true] {
        let ctx = egui::Context::default();
        super::configure_theme(&ctx);
        let mut draft = ScreenRadiusDraft::default();
        let mut other = "unrelated".to_owned();
        frame(&ctx, &mut draft, &mut other, vec![]);
        let (idle, _) = frame(&ctx, &mut draft, &mut other, vec![]);
        assert!(!idle.response.has_focus());
        let text = format!("64{}", " ".repeat(63));
        assert_eq!(text.chars().count(), 65);
        let insert = if paste {
            Event::Paste(text)
        } else {
            Event::Text(text)
        };
        let (typed, _) = frame(
            &ctx,
            &mut draft,
            &mut other,
            focus_and_insert(idle.response.interact_rect.center(), insert),
        );
        assert!(
            typed.response.has_focus() && typed.response.changed(),
            "fresh actual focus and this widget consumed the same-frame input"
        );
        assert!(typed.candidate.is_none());
        assert_eq!(draft.diagnostics()["overlength"], true);
        assert_eq!(draft.diagnostics()["invalid"], true);
        assert!(
            draft.diagnostics()["text"]
                .as_str()
                .unwrap()
                .chars()
                .count()
                <= 64
        );
        let (commit, _) = frame(
            &ctx,
            &mut draft,
            &mut other,
            vec![Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(
            commit.candidate.is_none(),
            "truncated valid64 prefix cannot commit"
        );
        assert_eq!(draft.diagnostics()["accepted_word_hex"], "42960000");
    }
    // Positive ownership control: a newly focused actual64 input remains useful.
    let ctx = egui::Context::default();
    super::configure_theme(&ctx);
    let mut draft = ScreenRadiusDraft::default();
    let mut other = "other".to_owned();
    frame(&ctx, &mut draft, &mut other, vec![]);
    let (idle, _) = frame(&ctx, &mut draft, &mut other, vec![]);
    let (typed, _) = frame(
        &ctx,
        &mut draft,
        &mut other,
        focus_and_insert(
            idle.response.interact_rect.center(),
            Event::Text("64".into()),
        ),
    );
    assert!(typed.response.has_focus() && typed.response.changed());
    assert_eq!(draft.diagnostics()["overlength"], false);
    let (commit, _) = frame(
        &ctx,
        &mut draft,
        &mut other,
        vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(commit.candidate.unwrap().to_bits(), 0x42800000);

    // A focused full64-character buffer can retain identical text after a
    // truncated insertion. Its prior valid draft still must not become64.
    for paste in [false, true] {
        let ctx = egui::Context::default();
        super::configure_theme(&ctx);
        let mut draft = ScreenRadiusDraft::default();
        let mut other = "other".to_owned();
        frame(&ctx, &mut draft, &mut other, vec![]);
        let (idle, _) = frame(&ctx, &mut draft, &mut other, vec![]);
        let valid = format!("64{}", " ".repeat(62));
        let (typed, _) = frame(
            &ctx,
            &mut draft,
            &mut other,
            focus_and_insert(
                idle.response.interact_rect.center(),
                Event::Text(valid.clone()),
            ),
        );
        assert!(typed.response.has_focus());
        assert_eq!(draft.diagnostics()["invalid"], false);
        assert_eq!(draft.diagnostics()["text"], valid);
        for pressed in [true, false] {
            frame(
                &ctx,
                &mut draft,
                &mut other,
                vec![Event::Key {
                    key: Key::End,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
        let over = format!("64{}", " ".repeat(63));
        let insert = if paste {
            Event::Paste(over)
        } else {
            Event::Text(over)
        };
        let (full, _) = frame(&ctx, &mut draft, &mut other, vec![insert]);
        assert!(full.response.has_focus());
        assert_eq!(
            draft.diagnostics()["text"],
            valid,
            "insertion at the64-character cap did not change actual text"
        );
        assert_eq!(draft.diagnostics()["overlength"], true);
        let (commit, _) = frame(
            &ctx,
            &mut draft,
            &mut other,
            vec![Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        assert!(commit.candidate.is_none());
        assert_eq!(draft.diagnostics()["accepted_word_hex"], "42960000");
    }

    // A different actual editor owns the oversized paste: do not poison radius.
    let ctx = egui::Context::default();
    super::configure_theme(&ctx);
    let mut draft = ScreenRadiusDraft::default();
    let mut other = "other".to_owned();
    frame(&ctx, &mut draft, &mut other, vec![]);
    let (_, target) = frame(&ctx, &mut draft, &mut other, vec![]);
    let (untouched, changed_other) = frame(
        &ctx,
        &mut draft,
        &mut other,
        focus_and_insert(
            target.interact_rect.center(),
            Event::Paste(format!("64{}", " ".repeat(63))),
        ),
    );
    assert!(changed_other.has_focus() && changed_other.changed());
    assert!(!untouched.response.changed() && !untouched.response.has_focus());
    assert!(untouched.candidate.is_none());
    assert_eq!(draft.diagnostics()["overlength"], false);
    assert_eq!(draft.diagnostics()["invalid"], false);
    assert_eq!(draft.diagnostics()["dirty"], false);
    assert_eq!(draft.diagnostics()["text"], "75");
}
