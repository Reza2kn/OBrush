//! Authored, unrun future actual-egui contracts for the reviewed Source UI helper.
//! This file is intended as app::source_paint_ui_contract_tests after Root approval.
//! It has no fake widget implementation and does not claim a compiled result.
use crate::source_paint_host::{PaintMode, SourcePaintPreferences};
use crate::source_paint_ui::{NumericPaintUi, SourcePaintEditorDrafts, render_numeric_preferences};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};

const EDITORS: [&str; 10] = [
    "source_paint.radius.value",
    "source_paint.intensity.value",
    "source_paint.hardness.value",
    "source_paint.pressure.intensity.value",
    "source_paint.pressure.radius.value",
    "source_paint.roughness.target.value",
    "source_paint.metallic.target.value",
    "source_paint.color.r.value",
    "source_paint.color.g.value",
    "source_paint.color.b.value",
];

fn current() -> SourcePaintPreferences {
    SourcePaintPreferences {
        mode: PaintMode::SourceOrdinary,
        ..Default::default()
    }
}
fn words(p: &SourcePaintPreferences) -> Vec<u64> {
    let o = &p.options;
    vec![
        p.radius.to_bits(),
        p.radius_factor.to_bits(),
        o.intensity.to_bits(),
        o.hardness.to_bits(),
        o.tablet.factor.to_bits(),
        o.tablet.pressure.to_bits(),
        u64::from(o.color[0].to_bits()),
        u64::from(o.color[1].to_bits()),
        u64::from(o.color[2].to_bits()),
        u64::from(o.roughness.to_bits()),
        u64::from(o.metallic.to_bits()),
        u64::from(o.write_rgb),
        u64::from(o.write_roughness),
        u64::from(o.write_metallic),
        u64::from(o.culling),
    ]
}
struct Fixture {
    ctx: egui::Context,
    width: f32,
    time: f64,
    prefs: SourcePaintPreferences,
    drafts: SourcePaintEditorDrafts,
}
impl Fixture {
    fn new(width: f32) -> Self {
        let ctx = egui::Context::default();
        super::configure_theme(&ctx);
        Self {
            ctx,
            width,
            time: 0.0,
            prefs: current(),
            drafts: Default::default(),
        }
    }
    fn frame(&mut self, events: Vec<Event>) -> (NumericPaintUi, Rect) {
        self.time += 1.0 / 60.0;
        let mut output = None;
        let _ = self.ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(self.width + 16.0, 1300.0),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(self.width);
                    let clip = ui.clip_rect();
                    output = Some((
                        render_numeric_preferences(ui, &self.prefs, &mut self.drafts),
                        clip,
                    ));
                });
            },
        );
        output.expect("actual numeric UI rendered")
    }
    fn focus(&mut self, key: &str) {
        let (view, _) = self.frame(Vec::new());
        let response = view.controls.get(key).expect("actual tracked editor");
        let point = response.interact_rect.center();
        assert!(response.enabled() && response.interact_rect.is_positive());
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
        assert!(
            self.ctx.memory(|m| m.focused()).is_some(),
            "real editor received focus"
        );
    }
    fn replace(&mut self, key: &str, text: &str, terminal: Key) -> NumericPaintUi {
        self.focus(key);
        let command = Modifiers {
            command: true,
            mac_cmd: true,
            ..Modifiers::NONE
        };
        let _ = self.frame(vec![Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: command,
        }]);
        let _ = self.frame(vec![Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: command,
        }]);
        let (typing, _) = self.frame(vec![Event::Text(text.to_owned())]);
        assert!(
            typing.candidate.is_none(),
            "text draft is not a partial preference commit"
        );
        let (out, _) = self.frame(vec![Event::Key {
            key: terminal,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        let _ = self.frame(vec![Event::Key {
            key: terminal,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        out
    }
    fn accept(&mut self, key: &str, text: &str) {
        let out = self.replace(key, text, Key::Enter);
        assert!(out.rejected.is_none());
        let candidate = out.candidate.expect("actual accepted finite change");
        candidate.validate().unwrap();
        self.prefs = candidate;
    }
}

#[test]
fn narrow160_source_scalar_rails_and_all_editors_are_inside_actual_content_clip() {
    let mut f = Fixture::new(160.0);
    let (view, clip) = f.frame(Vec::new());
    for key in EDITORS {
        let r = &view.controls[key];
        assert!(r.enabled() && r.interact_rect.is_positive(), "{key}");
        assert!(
            clip.contains_rect(r.rect) && clip.contains_rect(r.interact_rect),
            "{key}"
        );
    }
    for key in [
        "source_paint.radius",
        "source_paint.intensity",
        "source_paint.hardness",
        "source_paint.pressure.intensity",
        "source_paint.pressure.radius",
        "source_paint.roughness.target",
        "source_paint.metallic.target",
    ] {
        let r = &view.controls[key];
        assert!(
            r.interact_rect.is_positive()
                && clip.contains_rect(r.rect)
                && clip.contains_rect(r.interact_rect),
            "existing rail {key}"
        );
    }
}
#[test]
fn wide320_retains_same_keys_and_inside_positive_value_interactions() {
    let mut f = Fixture::new(320.0);
    let (view, clip) = f.frame(Vec::new());
    for key in EDITORS {
        let r = &view.controls[key];
        assert!(
            r.enabled()
                && r.interact_rect.is_positive()
                && clip.contains_rect(r.rect)
                && clip.contains_rect(r.interact_rect),
            "{key}"
        );
    }
}
#[test]
fn eight_idle_source_frames_keep_literal_default_color_and_material_words() {
    let mut f = Fixture::new(160.0);
    let before = words(&f.prefs);
    assert_eq!(
        f.prefs.options.color.map(f32::to_bits),
        [0x3f800000, 0x3f441893, 0x3eac0831]
    );
    assert_eq!(
        [
            f.prefs.options.roughness.to_bits(),
            f.prefs.options.metallic.to_bits()
        ],
        [0x3e99999a, 0x3f733333]
    );
    for _ in 0..8 {
        let (view, _) = f.frame(Vec::new());
        assert!(view.candidate.is_none() && view.rejected.is_none());
        assert_eq!(words(&f.prefs), before);
    }
}
#[test]
fn focus_without_text_is_not_a_color_float_or_preference_change() {
    let mut f = Fixture::new(160.0);
    let before = words(&f.prefs);
    f.focus("source_paint.color.g.value");
    let (view, _) = f.frame(Vec::new());
    assert!(view.candidate.is_none() && view.rejected.is_none());
    assert_eq!(words(&f.prefs), before);
}
#[test]
fn actual_numeric_rgb_entries_reach_quarter_targets_without_hsva_or_byte_conversion() {
    let mut f = Fixture::new(160.0);
    f.accept("source_paint.color.r.value", "0.25");
    f.accept("source_paint.color.r.value", "1");
    f.accept("source_paint.color.g.value", "0.5");
    f.accept("source_paint.color.b.value", "0.25");
    assert_eq!(
        f.prefs.options.color.map(f32::to_bits),
        [0x3f800000, 0x3f000000, 0x3e800000]
    );
}
#[test]
fn actual_green_only_entry_preserves_red_blue_and_material_target_words() {
    let mut f = Fixture::new(320.0);
    let before = words(&f.prefs);
    f.accept("source_paint.color.g.value", "0.5");
    let after = words(&f.prefs);
    for i in 0..before.len() {
        if i != 7 {
            assert_eq!(after[i], before[i], "field {i}");
        }
    }
    assert_eq!(f.prefs.options.color[1].to_bits(), 0x3f000000);
}
#[test]
fn actual_numeric_scalar_and_material_values_reach_frozen_core8_input_contract() {
    let mut f = Fixture::new(160.0);
    for (key, text) in [
        ("source_paint.radius.value", "5"),
        ("source_paint.intensity.value", "0.25"),
        ("source_paint.hardness.value", "1"),
        ("source_paint.pressure.radius.value", "0"),
        ("source_paint.roughness.target.value", "0.5"),
        ("source_paint.metallic.target.value", "0.75"),
    ] {
        f.accept(key, text);
    }
    assert_eq!(f.prefs.radius, 5.0);
    assert_eq!(f.prefs.radius_factor, 0.0);
    assert_eq!(f.prefs.options.intensity, 0.25);
    assert_eq!(f.prefs.options.hardness, 1.0);
    assert_eq!(f.prefs.options.tablet.factor, 0.0);
    assert_eq!(
        [
            f.prefs.options.roughness.to_bits(),
            f.prefs.options.metallic.to_bits()
        ],
        [0x3f000000, 0x3f400000]
    );
}
#[test]
fn invalid_nonfinite_rgb_entry_is_rejected_atomically_in_the_actual_editor() {
    for text in ["NaN", "inf", "-inf"] {
        let mut f = Fixture::new(160.0);
        let before = words(&f.prefs);
        let out = f.replace("source_paint.color.g.value", text, Key::Enter);
        assert!(out.candidate.is_none() && out.rejected.is_some(), "{text}");
        assert_eq!(words(&f.prefs), before);
    }
}
#[test]
fn malformed_and_out_of_range_rgb_entries_are_not_clamped_or_substituted() {
    for text in ["abc", "1.01", "-0.01"] {
        let mut f = Fixture::new(320.0);
        let before = words(&f.prefs);
        let out = f.replace("source_paint.color.r.value", text, Key::Enter);
        assert!(out.candidate.is_none() && out.rejected.is_some(), "{text}");
        assert_eq!(words(&f.prefs), before);
    }
}
#[test]
fn radius_outside_the_existing5_to500_scope_is_atomic_invalid_input() {
    let mut f = Fixture::new(160.0);
    let before = words(&f.prefs);
    let out = f.replace("source_paint.radius.value", "4", Key::Enter);
    assert!(out.candidate.is_none() && out.rejected.is_some());
    assert_eq!(words(&f.prefs), before);
}
#[test]
fn escape_discards_an_actual_numeric_text_draft_without_assigning_preferences() {
    let mut f = Fixture::new(160.0);
    let before = words(&f.prefs);
    let out = f.replace("source_paint.color.b.value", "0.5", Key::Escape);
    assert!(out.candidate.is_none() && out.rejected.is_none());
    assert_eq!(words(&f.prefs), before);
}
#[test]
fn accepted_same_numeric_word_is_a_real_noop_without_new_candidate() {
    let mut f = Fixture::new(320.0);
    let before = words(&f.prefs);
    let out = f.replace("source_paint.radius.value", "75", Key::Enter);
    assert!(out.candidate.is_none() && out.rejected.is_none());
    assert_eq!(words(&f.prefs), before);
}

#[test]
fn smallest_f64_factor_keeps_its_word_and_all_ten_drafts_bounded_on_idle() {
    let mut f = Fixture::new(160.0);
    f.prefs.options.tablet.factor = f64::from_bits(1);
    f.prefs.validate().unwrap();
    let before = words(&f.prefs);
    let (view, _) = f.frame(Vec::new());
    assert!(view.candidate.is_none() && view.rejected.is_none());
    assert_eq!(words(&f.prefs), before);
    for key in EDITORS {
        assert!(f.drafts.text_for(key).unwrap().len() <= 64);
    }
    assert_eq!(f.prefs.options.tablet.factor.to_bits(), 1);
}
