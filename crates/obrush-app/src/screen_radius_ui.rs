//! One bounded numeric companion to the existing screen-radius slider.
//! It emits a UI candidate; the application owns its existing release barrier.
use eframe::egui::{self, Response};
use obrush_core::BrushKind;

pub(crate) const KEY: &str = "settings.screen_radius.value";
const MAX_CHARS: usize = 64;

#[derive(Default)]
pub(crate) struct ScreenRadiusDraft {
    text: String,
    owner: Option<BrushKind>,
    word: Option<u32>,
    enabled: bool,
    dirty: bool,
    focused: bool,
    invalid: bool,
    overlength: bool,
    observed: bool,
    id: Option<egui::Id>,
}
impl ScreenRadiusDraft {
    fn reset(&mut self, owner: BrushKind, accepted: f32, enabled: bool) {
        self.text = accepted.to_string();
        self.owner = Some(owner);
        self.word = Some(accepted.to_bits());
        self.enabled = enabled;
        self.dirty = false;
        self.focused = false;
        self.invalid = false;
        self.overlength = false;
    }
    pub(crate) fn cancel(
        &mut self,
        ctx: &egui::Context,
        owner: BrushKind,
        accepted: f32,
        enabled: bool,
    ) {
        if let Some(id) = self.id {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
        self.reset(owner, accepted, enabled);
    }
    pub(crate) fn begin_frame(&mut self) {
        self.observed = false;
    }
    pub(crate) fn finish_frame(
        &mut self,
        ctx: &egui::Context,
        owner: BrushKind,
        accepted: f32,
        enabled: bool,
    ) {
        if !self.observed {
            self.cancel(ctx, owner, accepted, enabled);
        }
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "owner":self.owner.map(|k|k.name()),
            "accepted_word_hex":self.word.map(|w|format!("{w:08x}")),
            "text":self.text,"dirty":self.dirty,"focused":self.focused,
            "invalid":self.invalid,"overlength":self.overlength,
            "enabled":self.enabled,"observed":self.observed,
            "max_characters":MAX_CHARS
        })
    }
}

pub(crate) struct NumericRadiusUi {
    pub response: Response,
    pub candidate: Option<f32>,
}

// Egui's TextBuffer::insert_text_at truncates before parsing. Observe bounded
// input lengths and actual previous selection, without cloning pasted strings.
// Ambiguous mixed deletion/insertion frames are conservatively bounded too.
fn insertion_overlength(ui: &egui::Ui, id: egui::Id, text: &str) -> bool {
    let mut length = text.chars().take(MAX_CHARS + 1).count();
    let mut selected = egui::TextEdit::load_state(ui.ctx(), id)
        .and_then(|s| s.cursor.char_range())
        .map(|r| r.as_sorted_char_range().len().min(length))
        .unwrap_or(0);
    ui.input(|i| {
        for event in &i.events {
            match event {
                egui::Event::Key {
                    key: egui::Key::A,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => selected = length,
                egui::Event::Text(t)
                | egui::Event::Paste(t)
                | egui::Event::Ime(egui::ImeEvent::Commit(t)) => {
                    let inserted = t.chars().take(MAX_CHARS + 1).count();
                    length = length.saturating_sub(selected).saturating_add(inserted);
                    selected = 0;
                    if inserted > MAX_CHARS || length > MAX_CHARS {
                        return true;
                    }
                }
                egui::Event::Cut => {
                    length = length.saturating_sub(selected);
                    selected = 0;
                }
                egui::Event::Key {
                    key: egui::Key::Backspace | egui::Key::Delete,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.is_none() => {
                    // No selection: do not guess whether the cursor can delete.
                    // This may conservatively reject a mixed limit-sized frame.
                    length = length.saturating_sub(selected);
                    selected = 0;
                }
                egui::Event::Ime(egui::ImeEvent::Preedit(t))
                    if t.chars().take(MAX_CHARS + 1).count() > MAX_CHARS =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    })
}

pub(crate) fn render(
    ui: &mut egui::Ui,
    owner: BrushKind,
    accepted: f32,
    enabled: bool,
    draft: &mut ScreenRadiusDraft,
) -> NumericRadiusUi {
    let id = ui.make_persistent_id(KEY);
    let boundary = draft.owner != Some(owner)
        || draft.word != Some(accepted.to_bits())
        || draft.enabled != enabled;
    if boundary {
        draft.cancel(ui.ctx(), owner, accepted, enabled);
        ui.memory_mut(|m| m.surrender_focus(id));
    }
    draft.id = Some(id);
    draft.observed = true;
    let prior_focus = draft.focused;
    let focused_before = ui.memory(|m| m.has_focus(id));
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
    // Pointer/Tab focus may be acquired inside TextEdit in this same frame.
    // Read raw bounds first, then attribute them to its actual consumed mutation.
    let input_overlength = insertion_overlength(ui, id, &draft.text);
    let response = ui.add_enabled(
        enabled,
        egui::TextEdit::singleline(&mut draft.text)
            .id(id)
            .char_limit(MAX_CHARS)
            .desired_width(ui.available_width())
            .text_color(if draft.invalid {
                egui::Color32::LIGHT_RED
            } else {
                ui.visuals().text_color()
            }),
    );
    let overflow = input_overlength && (focused_before || response.changed());
    if response.changed() {
        draft.dirty = true;
        draft.invalid = false;
        draft.overlength = overflow;
    }
    if overflow {
        draft.dirty = true;
        draft.invalid = true;
        draft.overlength = true;
    }
    let mut candidate = None;
    if escape && (prior_focus || focused_before) {
        draft.cancel(ui.ctx(), owner, accepted, enabled);
    } else if enabled
        && !boundary
        && draft.dirty
        && ((focused_before && enter) || response.lost_focus())
    {
        let parsed = (!draft.overlength)
            .then(|| draft.text.trim().parse::<f64>().ok())
            .flatten()
            .filter(|v| v.is_finite() && (5.0..=500.0).contains(v))
            .map(|v| v as f32)
            .filter(|v| v.is_finite() && (5.0..=500.0).contains(v));
        if let Some(value) = parsed {
            if value.to_bits() != accepted.to_bits() {
                candidate = Some(value);
            }
            draft.reset(owner, value, enabled);
        } else {
            draft.invalid = true;
        }
    }
    draft.focused = ui.memory(|m| m.has_focus(id));
    NumericRadiusUi {
        response,
        candidate,
    }
}
