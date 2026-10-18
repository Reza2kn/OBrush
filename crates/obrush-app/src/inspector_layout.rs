//! Horizontal bounds for the existing inspector child, without a new UI ID scope.
//! Full source captions remain in every galley and hover; only closed combo
//! selection text may truncate. Popup rows and prose wrap instead.
use super::*;

/// Keep content width stable while the existing nonfloating scrollbar animates.
/// The parent supplies physical bounds before ScrollArea pixel-rounds its child.
pub(super) fn scroll<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    let parent = ui.available_rect_before_wrap();
    let reserved_width = parent.width() - ui.spacing().scroll.allocated_width();
    let reserved_width = if reserved_width.is_finite() {
        reserved_width.max(0.0)
    } else {
        0.0
    };
    let reserved_right = parent.left() + reserved_width;
    egui::ScrollArea::vertical()
        .id_salt("inspector_scroll")
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            let mut clip = ui.clip_rect();
            clip.min.x = clip.min.x.max(parent.left()).max(ui.max_rect().left());
            clip.max.x = clip
                .max
                .x
                .min(reserved_right)
                .min(parent.left() + viewport.width())
                .max(clip.min.x);
            ui.set_clip_rect(clip);
            ui.set_max_width((clip.right() - ui.max_rect().left()).max(0.0));
            constrain_content(ui);
            add_contents(ui)
        })
}

pub(super) fn bounded_width(ui: &egui::Ui) -> f32 {
    let available = ui.available_rect_before_wrap();
    let clip = ui.clip_rect();
    let width = available.right().min(clip.right()) - available.left().max(clip.left());
    if width.is_finite() {
        width.max(0.0)
    } else {
        0.0
    }
}

pub(super) fn constrain_content(ui: &mut egui::Ui) {
    // A wrapped horizontal row starts at one interactive row high and may
    // grow vertically. Preserve its real scroll/parent vertical clip.
    let mut clip = ui.clip_rect();
    clip.min.x = clip.min.x.max(ui.max_rect().left());
    clip.max.x = clip.max.x.min(ui.max_rect().right());
    ui.set_clip_rect(clip);
    let width = bounded_width(ui);
    ui.set_max_width(width);
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
    ui.spacing_mut().slider_width = width;
}

pub(super) fn combo<R>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    full_selected: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let width = bounded_width(ui);
    // ComboBox::width is a minimum, and its own wrap width omits its outer
    // button padding. Pre-layout the unchanged original text with both
    // margins charged, then hand that exact galley to the ordinary ComboBox.
    let text_width = (width
        - 2.0 * ui.spacing().button_padding.x
        - ui.spacing().icon_spacing
        - ui.spacing().icon_width)
        .max(0.0);
    let selected = egui::WidgetText::from(full_selected).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        text_width,
        egui::TextStyle::Button,
    );
    let mut response = egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected)
        .width(width)
        .truncate()
        .show_ui(ui, |ui| {
            // ComboBox deliberately sets Extend immediately before this
            // callback. Restore a bounded wrapping policy for actual rows.
            constrain_content(ui);
            add_contents(ui)
        });
    // Augment this exact stock combo widget; no new ID, layout or popup call.
    // Its press must not also become the ScrollArea's immediate backdrop drag.
    let changed = response.response.changed();
    let intrinsic_size = response.response.intrinsic_size;
    response.response = response.response.interact(egui::Sense::click_and_drag());
    if changed {
        response.response.mark_changed();
    }
    response.response.intrinsic_size = intrinsic_size;
    response.response = response.response.on_hover_text(full_selected);
    response
}

pub(super) fn prose(ui: &mut egui::Ui, text: RichText) -> egui::Response {
    let galley = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        bounded_width(ui),
        egui::TextStyle::Body,
    );
    ui.add(egui::Label::new(galley).wrap())
}

pub(super) fn navigation_value(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
) -> egui::InnerResponse<(egui::Response, egui::Response)> {
    let width = bounded_width(ui);
    // Prepare both unchanged texts against the row's full width before
    // placing either one. A long label can then put the complete percentage
    // on the next row, without laying it out at the first row's tiny remainder.
    let label = egui::WidgetText::from(
        RichText::new(label)
            .size(11.0)
            .color(preferences_host::muted(ui)),
    )
    .into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        width,
        egui::TextStyle::Body,
    );
    let value = egui::WidgetText::from(RichText::new(value).size(11.0).monospace()).into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        width,
        egui::TextStyle::Body,
    );
    ui.horizontal_wrapped(|ui| {
        constrain_content(ui);
        let label = ui.add(egui::Label::new(label).wrap());
        let value = ui.add(egui::Label::new(value).wrap());
        (label, value)
    })
}

pub(super) fn camera_actions(ui: &mut egui::Ui) -> [egui::Response; 4] {
    ui.horizontal_wrapped(|ui| {
        constrain_content(ui);
        let fit = ui.add(
            egui::Button::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C565099F6DF99,
            ))
            .sense(egui::Sense::click_and_drag()),
        );
        let front = ui
            .add(
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C7688CD7B2014,
                ))
                .sense(egui::Sense::click_and_drag()),
            )
            .on_hover_text(view_hint(crate::shortcut_profile::Command::ViewFront));
        let left = ui
            .add(
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CCB106B20524D,
                ))
                .sense(egui::Sense::click_and_drag()),
            )
            .on_hover_text(view_hint(crate::shortcut_profile::Command::ViewLeft));
        let top = ui
            .add(
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C360B1EA7EA20,
                ))
                .sense(egui::Sense::click_and_drag()),
            )
            .on_hover_text(view_hint(crate::shortcut_profile::Command::ViewTop));
        [fit, front, left, top]
    })
    .inner
}

fn view_hint(command: crate::shortcut_profile::Command) -> String {
    crate::ui_text::command_hint(command)
        .unwrap_or_else(|| "No assigned shortcut in this profile".to_owned())
}
