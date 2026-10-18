//! Actual egui scalar rails/editors; authored unrun, no browser claim.
use super::source_geometry_ui::*;
use crate::source_geometry_host::{SourceGeometryMode, SourceGeometryPreferences};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};
const EDITORS: [&str; 4] = [
    "source_geometry.radius.value",
    "source_geometry.intensity.value",
    "source_geometry.pressure.radius.value",
    "source_geometry.pressure.intensity.value",
];
struct Fixture {
    ctx: egui::Context,
    prefs: SourceGeometryPreferences,
    drafts: GeometryEditorDrafts,
    width: f32,
    time: f64,
}
impl Fixture {
    fn new(width: f32) -> Self {
        let ctx = egui::Context::default();
        super::configure_theme(&ctx);
        Self {
            ctx,
            prefs: SourceGeometryPreferences {
                mode: SourceGeometryMode::SourceOrdinary,
                ..Default::default()
            },
            drafts: Default::default(),
            width,
            time: 0.,
        }
    }
    fn frame(&mut self, events: Vec<Event>) -> (GeometryNumericUi, Rect) {
        self.time += 1. / 60.;
        let mut view = None;
        let _ = self.ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(self.width + 16., 800.),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(self.width);
                    view = Some((render(ui, self.prefs, &mut self.drafts), ui.clip_rect()));
                });
            },
        );
        view.unwrap()
    }
    fn text(&mut self, key: &str, text: &str) {
        let (view, _) = self.frame(vec![]);
        let response = &view.controls[key];
        assert!(response.enabled() && response.interact_rect.is_positive());
        let point = response.interact_rect.center();
        for pressed in [true, false] {
            self.frame(vec![
                Event::PointerMoved(point),
                Event::PointerButton {
                    pos: point,
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
        let out = self
            .frame(vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }])
            .0;
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        out
    }
}
#[test]
fn all_rails_and_editors_stay_inside_narrow_and_wide_tool_clip() {
    for width in [160., 412.] {
        let mut f = Fixture::new(width);
        let (view, clip) = f.frame(vec![]);
        assert_eq!(view.controls.len(), 8);
        for r in view.controls.values() {
            assert!(r.enabled());
            assert!(r.interact_rect.is_positive());
            assert!(clip.contains_rect(r.interact_rect));
        }
    }
}
#[test]
fn idle_frames_preserve_non_round_decimal_f64_words() {
    let mut f = Fixture::new(160.);
    f.prefs.radius = 74.00000000000001;
    f.prefs.options.intensity = f64::from_bits(0x3fd0000000000001);
    f.prefs.radius_factor = -0.;
    let original = f.prefs;
    for _ in 0..12 {
        assert!(f.frame(vec![]).0.candidate.is_none());
        assert!(same_words(original, f.prefs));
    }
}
#[test]
fn exact_quarter_enter_commit_is_finite_and_only_changes_that_field() {
    let mut f = Fixture::new(160.);
    let old = f.prefs;
    f.text(EDITORS[1], "0.25");
    let p = f.key(Key::Enter).candidate.unwrap();
    assert_eq!(p.options.intensity.to_bits(), 0x3fd0000000000000);
    assert_eq!(p.radius.to_bits(), old.radius.to_bits());
    assert_eq!(p.radius_factor.to_bits(), old.radius_factor.to_bits());
    assert_eq!(
        p.options.intensity_factor.to_bits(),
        old.options.intensity_factor.to_bits()
    );
}
#[test]
fn escape_uses_prior_actual_focus_and_never_commits_dirty_text() {
    let mut f = Fixture::new(160.);
    let old = f.prefs;
    f.text(EDITORS[3], "0.875");
    assert!(f.key(Key::Escape).candidate.is_none());
    assert!(same_words(old, f.prefs));
    for _ in 0..3 {
        assert!(f.frame(vec![]).0.candidate.is_none());
    }
}
#[test]
fn invalid_nan_infinity_out_of_range_and_overlong_text_never_emit_candidate() {
    for text in [
        "NaN",
        "inf",
        "-0.1",
        "1.1",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    ] {
        let mut f = Fixture::new(160.);
        f.text(EDITORS[1], text);
        let view = f.key(Key::Enter);
        assert!(view.candidate.is_none());
        assert!(view.rejected.is_some());
    }
}
#[test]
fn deliberate_signed_zero_edit_is_retained_but_same_word_enter_is_noop() {
    let mut f = Fixture::new(160.);
    f.text(EDITORS[2], "-0");
    let p = f.key(Key::Enter).candidate.unwrap();
    assert_eq!(p.radius_factor.to_bits(), (-0f64).to_bits());
    f.prefs = p;
    f.text(EDITORS[2], "-0.0");
    assert!(f.key(Key::Enter).candidate.is_none());
}
#[test]
fn invalid_or_same_word_candidates_never_call_release_barrier() {
    let f = Fixture::new(160.);
    let old = f.prefs;
    assert!(matches!(
        commit_candidate(old, old, || panic!("same words must not release")),
        Ok(GeometryCommit::Unchanged)
    ));
    let mut invalid = old;
    invalid.radius = f64::NAN;
    assert!(commit_candidate(old, invalid, || panic!("invalid values must not release")).is_err());
}
#[test]
fn rejected_real_release_does_not_install_the_valid_numeric_candidate() {
    let f = Fixture::new(160.);
    let old = f.prefs;
    let mut p = old;
    p.options.intensity = 0.25;
    let mut calls = 0;
    assert!(matches!(
        commit_candidate(old, p, || {
            calls += 1;
            false
        }),
        Ok(GeometryCommit::ReleaseRejected)
    ));
    assert_eq!(calls, 1);
    assert!(same_words(old, f.prefs));
}
