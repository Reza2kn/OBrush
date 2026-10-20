//! Five bounded f64 numeric drafts for actual ordinary Source geometry controls.
//! A focus/idle/invalid/Escape frame cannot rewrite saved preference words.
use crate::source_geometry_host::{SourceGeometryMode, SourceGeometryPreferences};
use eframe::egui::{self, Response};
use std::collections::BTreeMap;

#[derive(Default)]
struct Draft {
    text: String,
    word: Option<u64>,
    dirty: bool,
    focused: bool,
    invalid: bool,
}
impl Draft {
    fn reset(&mut self, v: f64) {
        self.text = v.to_string();
        self.word = Some(v.to_bits());
        self.dirty = false;
        self.focused = false;
        self.invalid = false;
    }
}
pub(crate) struct GeometryEditorDrafts {
    entries: [Draft; 5],
}
impl Default for GeometryEditorDrafts {
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| Default::default()),
        }
    }
}
const KEYS: [&str; 5] = [
    "source_geometry.radius",
    "source_geometry.intensity",
    "source_geometry.pressure.radius",
    "source_geometry.pressure.intensity",
    "source_geometry.mirror.offset",
];
const LABELS: [&str; 5] = [
    "Radius (viewport points)",
    "Intensity",
    "Pressure radius factor",
    "Pressure intensity factor",
    "Source mirror offset (model radius fraction)",
];
fn values(p: SourceGeometryPreferences) -> [f64; 4] {
    [
        p.radius,
        p.options.intensity,
        p.radius_factor,
        p.options.intensity_factor,
    ]
}
fn set(p: &mut SourceGeometryPreferences, i: usize, v: f64) {
    match i {
        0 => p.radius = v,
        1 => p.options.intensity = v,
        2 => p.radius_factor = v,
        3 => p.options.intensity_factor = v,
        _ => p.mirror_offset = v,
    }
}
fn limits(i: usize) -> (f64, f64) {
    if i == 0 {
        (5., 500.)
    } else if i == 4 {
        (-1.0, 1.0)
    } else {
        (0., 1.)
    }
}
impl GeometryEditorDrafts {
    pub(crate) fn reset(&mut self, p: SourceGeometryPreferences) {
        for (d, v) in self.entries.iter_mut().zip(values(p)) {
            d.reset(v);
        }
        self.entries[4].reset(p.mirror_offset);
    }
}
#[derive(Default)]
pub(crate) struct GeometryNumericUi {
    pub controls: BTreeMap<String, Response>,
    pub candidate: Option<SourceGeometryPreferences>,
    pub rejected: Option<String>,
}
pub(crate) fn render(
    ui: &mut egui::Ui,
    current: SourceGeometryPreferences,
    drafts: &mut GeometryEditorDrafts,
) -> GeometryNumericUi {
    let mut out = GeometryNumericUi::default();
    if current.mode != SourceGeometryMode::SourceOrdinary {
        return out;
    }
    let mut candidate = current;
    let mut changed = false;
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    for (i, value) in values(current).into_iter().enumerate() {
        let draft = &mut drafts.entries[i];
        let prior_focus = draft.focused;
        if draft.word.is_none() || (!draft.focused && draft.word != Some(value.to_bits())) {
            draft.reset(value);
        }
        if current.options.tool == obrush_core::SourceGeometryTool::Drag && matches!(i, 1 | 3) {
            continue;
        }
        let label = match i {
            0 => crate::ui_text::caption(crate::ui_text::CaptionKey::CD2E97146E61C),
            1 => crate::ui_text::caption(crate::ui_text::CaptionKey::CA3BA7BB8E3A4),
            2 => crate::ui_text::caption(crate::ui_text::CaptionKey::CB4F9B2FD35C6),
            3 => crate::ui_text::caption(crate::ui_text::CaptionKey::C824E3CA3BB64),
            _ => unreachable!("Bounded four-field geometry roster"),
        };
        ui.label(label);
        let (low, high) = limits(i);
        let mut rail = value;
        let response = ui
            .scope(|ui| {
                ui.spacing_mut().slider_width = ui.available_width();
                ui.add(egui::Slider::new(&mut rail, low..=high).show_value(false))
            })
            .inner;
        if response.changed() && rail.to_bits() != value.to_bits() {
            set(&mut candidate, i, rail);
            draft.reset(rail);
            changed = true;
        }
        out.controls.insert(KEYS[i].into(), response);
        let key = format!("{}.value", KEYS[i]);
        let id = ui.make_persistent_id(&key);
        let focused = ui.memory(|m| m.has_focus(id));
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let response = ui.add(
            egui::TextEdit::singleline(&mut draft.text)
                .id(id)
                .char_limit(64)
                .desired_width(ui.available_width()),
        );
        if response.changed() {
            draft.dirty = true;
            draft.invalid = false;
        }
        if escape && (prior_focus || focused) {
            draft.reset(value);
            ui.memory_mut(|m| m.surrender_focus(id));
        } else if draft.dirty && ((focused && enter) || response.lost_focus()) {
            match draft
                .text
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && (low..=high).contains(n))
            {
                Some(v) => {
                    if v.to_bits() != value.to_bits() {
                        set(&mut candidate, i, v);
                        changed = true;
                    }
                    draft.reset(v);
                }
                None => {
                    draft.invalid = true;
                    out.rejected = Some(format!(
                        "{} requires a finite value from {low} to {high}.",
                        LABELS[i]
                    ));
                }
            }
        }
        draft.focused = response.has_focus();
        if draft.invalid {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                "Enter a value in range, or Escape.",
            );
        }
        out.controls.insert(key, response);
        ui.add_space(3.);
    }
    if changed && out.rejected.is_none() {
        match candidate.validate() {
            Ok(()) => out.candidate = Some(candidate),
            Err(e) => out.rejected = Some(e.to_string()),
        }
    }
    out
}

pub(crate) fn same_words(a: SourceGeometryPreferences, b: SourceGeometryPreferences) -> bool {
    a.mode == b.mode
        && a.options.tool == b.options.tool
        && a.options.culling == b.options.culling
        && a.options.negative == b.options.negative
        && a.options.clay_order == b.options.clay_order
        && a.options.prevent_folds == b.options.prevent_folds
        && values(a).map(f64::to_bits) == values(b).map(f64::to_bits)
        && a.mirror_offset.to_bits() == b.mirror_offset.to_bits()
}

#[derive(Default)]
pub(crate) struct GeometrySelectionUi {
    pub controls: BTreeMap<String, Response>,
    pub candidate: Option<obrush_core::SourceClayOrderPolicy>,
}
pub(crate) fn render_selection_order(
    ui: &mut egui::Ui,
    current: obrush_core::SourceClayOrderPolicy,
) -> GeometrySelectionUi {
    use obrush_core::SourceClayOrderPolicy::{OrderedStaticOctree, SingleLeaf};
    let mut out = GeometrySelectionUi::default();
    ui.label(crate::ui_text::caption(
        crate::ui_text::CaptionKey::C2C8B48FE1BC1,
    ));
    for (key, label, policy) in [
        (
            "source_geometry.selection.ordered",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::CB385F53D6E13),
            OrderedStaticOctree,
        ),
        (
            "source_geometry.selection.single_leaf",
            &crate::ui_text::caption(crate::ui_text::CaptionKey::C34C01497A0F9),
            SingleLeaf,
        ),
    ] {
        let response = ui.add_sized(
            [ui.available_width(), 24.],
            egui::Button::selectable(current == policy, label).wrap(),
        );
        if response.clicked() && current != policy {
            out.candidate = Some(policy);
        }
        out.controls.insert(key.into(), response);
    }
    out
}

#[cfg(test)]
#[path = "source_geometry_order_tests.rs"]
mod selection_order_tests;
pub(crate) enum GeometryCommit {
    Unchanged,
    Accepted(SourceGeometryPreferences),
    ReleaseRejected,
}
pub(crate) fn commit_candidate(
    current: SourceGeometryPreferences,
    candidate: SourceGeometryPreferences,
    release: impl FnOnce() -> bool,
) -> Result<GeometryCommit, obrush_core::SourceGeometryError> {
    candidate.validate()?;
    if same_words(current, candidate) {
        return Ok(GeometryCommit::Unchanged);
    }
    if !release() {
        return Ok(GeometryCommit::ReleaseRejected);
    }
    Ok(GeometryCommit::Accepted(candidate))
}

#[cfg(test)]
#[path = "source_geometry_mirror_alpha_tests.rs"]
mod mirror_alpha_host_tests;

pub(crate) fn render_mirror_offset(
    ui: &mut egui::Ui,
    current: SourceGeometryPreferences,
    drafts: &mut GeometryEditorDrafts,
) -> GeometryNumericUi {
    let mut out = GeometryNumericUi::default();
    if current.mode != SourceGeometryMode::SourceOrdinary {
        return out;
    }
    let mut candidate = current;
    let mut changed = false;
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    {
        let (i, value) = (4, current.mirror_offset);
        let draft = &mut drafts.entries[i];
        let prior_focus = draft.focused;
        if draft.word.is_none() || (!draft.focused && draft.word != Some(value.to_bits())) {
            draft.reset(value);
        }
        ui.label(LABELS[i]);
        let (low, high) = limits(i);
        let mut rail = value;
        let response = ui
            .scope(|ui| {
                ui.spacing_mut().slider_width = ui.available_width();
                ui.add(egui::Slider::new(&mut rail, low..=high).show_value(false))
            })
            .inner;
        if response.changed() && rail.to_bits() != value.to_bits() {
            set(&mut candidate, i, rail);
            draft.reset(rail);
            changed = true;
        }
        out.controls.insert(KEYS[i].into(), response);
        let key = format!("{}.value", KEYS[i]);
        let id = ui.make_persistent_id(&key);
        let focused = ui.memory(|m| m.has_focus(id));
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let response = ui.add(
            egui::TextEdit::singleline(&mut draft.text)
                .id(id)
                .char_limit(64)
                .desired_width(ui.available_width()),
        );
        if response.changed() {
            draft.dirty = true;
            draft.invalid = false;
        }
        if escape && (prior_focus || focused) {
            draft.reset(value);
            ui.memory_mut(|m| m.surrender_focus(id));
        } else if draft.dirty && ((focused && enter) || response.lost_focus()) {
            match draft
                .text
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && (low..=high).contains(n))
            {
                Some(v) => {
                    if v.to_bits() != value.to_bits() {
                        set(&mut candidate, i, v);
                        changed = true;
                    }
                    draft.reset(v);
                }
                None => {
                    draft.invalid = true;
                    out.rejected = Some(format!(
                        "{} requires a finite value from {low} to {high}.",
                        LABELS[i]
                    ));
                }
            }
        }
        draft.focused = response.has_focus();
        if draft.invalid {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                "Enter a value in range, or Escape.",
            );
        }
        out.controls.insert(key, response);
        ui.add_space(3.);
    }
    if changed && out.rejected.is_none() {
        match candidate.validate() {
            Ok(()) => out.candidate = Some(candidate),
            Err(e) => out.rejected = Some(e.to_string()),
        }
    }
    out
}

#[cfg(test)]
#[path = "source_geometry_press_phase_tests.rs"]
mod press_phase_host_tests;

/// Display preflight only. Actual press and every dab retain their own complete
/// validation; an already owned session is not revalidated to paint its panel.
pub(crate) fn panel_preflight(
    source_session: Option<&crate::source_geometry_host::SharedSourceGeometry>,
    validate: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if source_session.is_some() {
        Ok(())
    } else {
        validate()
    }
}
