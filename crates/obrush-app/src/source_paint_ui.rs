//! Normal Source Paint numeric UI. These drafts never own document or history.
use std::collections::BTreeMap;

use eframe::egui::{self, Response};

use crate::source_paint_host::{PaintMode, SourcePaintPreferences};

const MAX_TEXT_CHARS: usize = 64;
const FIELD_COUNT: usize = 11;

#[derive(Clone, Copy)]
enum Field {
    Radius,
    Intensity,
    Hardness,
    PressureIntensity,
    PressureRadius,
    Roughness,
    Metallic,
    Red,
    Green,
    Blue,
    MirrorOffset,
}
const FIELDS: [Field; FIELD_COUNT] = [
    Field::Radius,
    Field::Intensity,
    Field::Hardness,
    Field::PressureIntensity,
    Field::PressureRadius,
    Field::Roughness,
    Field::Metallic,
    Field::Red,
    Field::Green,
    Field::Blue,
    Field::MirrorOffset,
];
#[derive(Clone, Copy)]
enum Number {
    F64(f64),
    F32(f32),
}
impl Number {
    fn word(self) -> u64 {
        match self {
            Self::F64(n) => n.to_bits(),
            Self::F32(n) => u64::from(n.to_bits()),
        }
    }
    fn text(self) -> String {
        let text = match self {
            Self::F64(n) => n.to_string(),
            Self::F32(n) => n.to_string(),
        };
        if text.len() <= MAX_TEXT_CHARS {
            text
        } else {
            match self {
                Self::F64(n) => format!("{n:e}"),
                Self::F32(n) => format!("{n:e}"),
            }
        }
    }
    fn as_f64(self) -> f64 {
        match self {
            Self::F64(n) => n,
            Self::F32(n) => f64::from(n),
        }
    }
}
impl Field {
    fn key(self) -> &'static str {
        match self {
            Self::Radius => "source_paint.radius",
            Self::Intensity => "source_paint.intensity",
            Self::Hardness => "source_paint.hardness",
            Self::PressureIntensity => "source_paint.pressure.intensity",
            Self::PressureRadius => "source_paint.pressure.radius",
            Self::Roughness => "source_paint.roughness.target",
            Self::Metallic => "source_paint.metallic.target",
            Self::Red => "source_paint.color.r",
            Self::Green => "source_paint.color.g",
            Self::Blue => "source_paint.color.b",
            Self::MirrorOffset => "source_paint.mirror.offset",
        }
    }
    fn caption_key(self) -> crate::ui_text::CaptionKey {
        match self {
            Self::Radius => crate::ui_text::CaptionKey::CD2E97146E61C,
            Self::Intensity => crate::ui_text::CaptionKey::CA3BA7BB8E3A4,
            Self::Hardness => crate::ui_text::CaptionKey::C37B37490F9D1,
            Self::PressureIntensity => crate::ui_text::CaptionKey::C824E3CA3BB64,
            Self::PressureRadius => crate::ui_text::CaptionKey::CB4F9B2FD35C6,
            Self::Roughness => crate::ui_text::CaptionKey::C855D42092810,
            Self::Metallic => crate::ui_text::CaptionKey::CF0EB81275BC4,
            Self::Red => crate::ui_text::CaptionKey::C8B35DA5A3251,
            Self::Green => crate::ui_text::CaptionKey::C57F53962541F,
            Self::Blue => crate::ui_text::CaptionKey::C61AE79146911,
            Self::MirrorOffset => {
                unreachable!("Source mirror offset uses its explicit existing geometry label")
            }
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Radius => "Radius (viewport points)",
            Self::Intensity => "Intensity",
            Self::Hardness => "Hardness",
            Self::PressureIntensity => "Pressure intensity factor",
            Self::PressureRadius => "Pressure radius factor",
            Self::Roughness => "Roughness target",
            Self::Metallic => "Metallic target",
            Self::Red => "Red (0–1)",
            Self::Green => "Green (0–1)",
            Self::Blue => "Blue (0–1)",
            Self::MirrorOffset => "Source mirror offset (model radius fraction)",
        }
    }
    fn has_rail(self) -> bool {
        !matches!(self, Self::Red | Self::Green | Self::Blue)
    }
    fn is_f32(self) -> bool {
        matches!(
            self,
            Self::Roughness | Self::Metallic | Self::Red | Self::Green | Self::Blue
        )
    }
    fn limits(self) -> (f64, f64) {
        if matches!(self, Self::MirrorOffset) {
            (-1., 1.)
        } else if matches!(self, Self::Radius) {
            (5., 500.)
        } else {
            (0., 1.)
        }
    }
    fn get(self, p: &SourcePaintPreferences) -> Number {
        match self {
            Self::MirrorOffset => Number::F64(p.mirror_offset),
            Self::Radius => Number::F64(p.radius),
            Self::Intensity => Number::F64(p.options.intensity),
            Self::Hardness => Number::F64(p.options.hardness),
            Self::PressureIntensity => Number::F64(p.options.tablet.factor),
            Self::PressureRadius => Number::F64(p.radius_factor),
            Self::Roughness => Number::F32(p.options.roughness),
            Self::Metallic => Number::F32(p.options.metallic),
            Self::Red => Number::F32(p.options.color[0]),
            Self::Green => Number::F32(p.options.color[1]),
            Self::Blue => Number::F32(p.options.color[2]),
        }
    }
    fn set(self, p: &mut SourcePaintPreferences, number: Number) {
        match (self, number) {
            (Self::MirrorOffset, Number::F64(n)) => p.mirror_offset = n,
            (Self::Radius, Number::F64(n)) => p.radius = n,
            (Self::Intensity, Number::F64(n)) => p.options.intensity = n,
            (Self::Hardness, Number::F64(n)) => p.options.hardness = n,
            (Self::PressureIntensity, Number::F64(n)) => p.options.tablet.factor = n,
            (Self::PressureRadius, Number::F64(n)) => p.radius_factor = n,
            (Self::Roughness, Number::F32(n)) => p.options.roughness = n,
            (Self::Metallic, Number::F32(n)) => p.options.metallic = n,
            (Self::Red, Number::F32(n)) => p.options.color[0] = n,
            (Self::Green, Number::F32(n)) => p.options.color[1] = n,
            (Self::Blue, Number::F32(n)) => p.options.color[2] = n,
            _ => unreachable!("UI field declared numeric type must agree"),
        }
    }
    fn parse(self, text: &str) -> Result<Number, String> {
        let text = text.trim();
        let parsed = if self.is_f32() {
            text.parse::<f32>().ok().map(Number::F32)
        } else {
            text.parse::<f64>().ok().map(Number::F64)
        };
        let (min, max) = self.limits();
        parsed
            .filter(|n| n.as_f64().is_finite() && (min..=max).contains(&n.as_f64()))
            .ok_or_else(|| {
                format!(
                    "{} must be a finite number between {min} and {max}.",
                    self.label()
                )
            })
    }
}
#[derive(Default)]
struct Draft {
    text: String,
    word: Option<u64>,
    dirty: bool,
    invalid: bool,
    focused: bool,
}
impl Draft {
    fn reset(&mut self, number: Number) {
        self.text = number.text();
        self.word = Some(number.word());
        self.dirty = false;
        self.invalid = false;
        self.focused = false;
    }
}
pub(crate) struct SourcePaintEditorDrafts {
    entries: [Draft; FIELD_COUNT],
}
impl Default for SourcePaintEditorDrafts {
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| Draft::default()),
        }
    }
}
impl SourcePaintEditorDrafts {
    pub(crate) fn reset(&mut self, preferences: &SourcePaintPreferences) {
        for (field, draft) in FIELDS.into_iter().zip(&mut self.entries) {
            draft.reset(field.get(preferences));
        }
    }
    #[cfg(test)]
    pub(crate) fn text_for(&self, key: &str) -> Option<&str> {
        FIELDS
            .into_iter()
            .enumerate()
            .find(|(_, f)| format!("{}.value", f.key()) == key)
            .map(|(i, _)| self.entries[i].text.as_str())
    }
}
#[derive(Default)]
pub(crate) struct NumericPaintUi {
    pub(crate) controls: BTreeMap<String, Response>,
    pub(crate) candidate: Option<SourcePaintPreferences>,
    pub(crate) rejected: Option<String>,
}

/// Current preferences are read-only. Only a changed rail or a finite validated
/// dirty text commit emits a candidate; idle and focus never parse stored words.
pub(crate) fn render_numeric_preferences(
    ui: &mut egui::Ui,
    current: &SourcePaintPreferences,
    drafts: &mut SourcePaintEditorDrafts,
) -> NumericPaintUi {
    let mut output = NumericPaintUi::default();
    if current.mode != PaintMode::SourceOrdinary {
        return output;
    }
    let mut candidate = current.clone();
    let mut changed = false;
    // egui clears current focus at begin_pass on Escape. Observe the key once
    // before visiting fields, and retain each field's prior actual focus.
    let escape_this_frame = ui.input(|i| i.key_pressed(egui::Key::Escape));
    for (index, field) in FIELDS.into_iter().enumerate() {
        let number = field.get(current);
        let draft = &mut drafts.entries[index];
        let previously_focused = draft.focused;
        if draft.word.is_none() || (!draft.focused && draft.word != Some(number.word())) {
            draft.reset(number);
        }
        if matches!(field, Field::MirrorOffset) {
            ui.label(field.label());
        } else {
            ui.label(crate::ui_text::caption(field.caption_key()));
        }
        if field.has_rail() {
            let (min, max) = field.limits();
            let response = ui
                .scope(|ui| {
                    ui.spacing_mut().slider_width = ui.available_width();
                    match number {
                        Number::F64(mut value) => {
                            let r =
                                ui.add(egui::Slider::new(&mut value, min..=max).show_value(false));
                            if r.changed() && value.to_bits() != number.word() {
                                let value = Number::F64(value);
                                field.set(&mut candidate, value);
                                draft.reset(value);
                                changed = true;
                            }
                            r
                        }
                        Number::F32(mut value) => {
                            let r = ui.add(
                                egui::Slider::new(&mut value, min as f32..=max as f32)
                                    .show_value(false),
                            );
                            if r.changed() && u64::from(value.to_bits()) != number.word() {
                                let value = Number::F32(value);
                                field.set(&mut candidate, value);
                                draft.reset(value);
                                changed = true;
                            }
                            r
                        }
                    }
                })
                .inner;
            output.controls.insert(field.key().to_owned(), response);
        }
        let key = format!("{}.value", field.key());
        let id = ui.make_persistent_id(&key);
        let focused_before = ui.memory(|m| m.has_focus(id));
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let response = ui.add(
            egui::TextEdit::singleline(&mut draft.text)
                .id(id)
                .char_limit(MAX_TEXT_CHARS)
                .desired_width(ui.available_width()),
        );
        if response.changed() {
            draft.dirty = true;
            draft.invalid = false;
        }
        if escape_this_frame && (previously_focused || focused_before) {
            draft.reset(number);
            ui.memory_mut(|m| m.surrender_focus(id));
        } else if draft.dirty && ((focused_before && enter) || response.lost_focus()) {
            match field.parse(&draft.text) {
                Ok(value) => {
                    if value.word() != number.word() {
                        field.set(&mut candidate, value);
                        changed = true;
                    }
                    draft.reset(value);
                }
                Err(error) => {
                    draft.invalid = true;
                    output.rejected.get_or_insert(error);
                }
            }
        }
        draft.focused = response.has_focus();
        if draft.invalid {
            ui.colored_label(
                egui::Color32::from_rgb(232, 122, 94),
                "Enter a finite value in the shown range, or press Escape.",
            );
        }
        output.controls.insert(key, response);
        ui.add_space(3.0);
    }
    if changed && output.rejected.is_none() {
        match candidate.validate() {
            Ok(()) => output.candidate = Some(candidate),
            Err(error) => output.rejected = Some(error),
        }
    }
    output
}

pub(crate) enum PreferenceCommit {
    Unchanged,
    Invalid(String),
    ReleaseRejected,
    Accepted(SourcePaintPreferences),
}
/// Shared actual host barrier: validate before old-stroke release, then publish
/// preferences only when the host reports that its release was accepted.
pub(crate) fn commit_candidate(
    candidate: Option<SourcePaintPreferences>,
    release_old_stroke: impl FnOnce() -> bool,
) -> PreferenceCommit {
    let Some(candidate) = candidate else {
        return PreferenceCommit::Unchanged;
    };
    if let Err(error) = candidate.validate() {
        return PreferenceCommit::Invalid(error);
    }
    if !release_old_stroke() {
        return PreferenceCommit::ReleaseRejected;
    }
    PreferenceCommit::Accepted(candidate)
}
