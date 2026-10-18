//! Transfer keyboard ownership after a real press in the sculpt viewport.
//! egui's general keyboard-interest flag includes focused brush buttons, which
//! otherwise survive a drag and consume a subsequent brush shortcut.
use eframe::egui;

pub(crate) fn handoff_viewport_focus(ctx: &egui::Context, rect: egui::Rect) {
    let pressed = ctx.input(|input| {
        input.events.iter().any(|event| {
            matches!(event, egui::Event::PointerButton { pos, pressed: true, .. }
                if pos.is_finite() && rect.contains(*pos))
        })
    });
    if pressed {
        ctx.memory_mut(|memory| {
            if let Some(id) = memory.focused() {
                memory.surrender_focus(id);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focused_text_editor() -> (egui::Context, egui::Id) {
        let ctx = egui::Context::default();
        let mut id = egui::Id::NULL;
        let mut text = String::from("0.180");
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = ui.text_edit_singleline(&mut text);
                id = response.id;
                response.request_focus();
            });
        });
        assert!(ctx.wants_keyboard_input());
        (ctx, id)
    }

    #[test]
    fn canvas_press_ends_existing_editor_focus_before_a_drag_release_shortcut() {
        let (ctx, id) = focused_text_editor();
        let viewport = egui::Rect::from_min_max(egui::pos2(400.0, 100.0), egui::pos2(900.0, 700.0));
        let raw = egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: viewport.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| {
            assert!(ctx.memory(|memory| memory.has_focus(id)));
            handoff_viewport_focus(ctx, viewport);
            assert!(!ctx.wants_keyboard_input());
        });
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::OpenBracket,
                    physical_key: Some(egui::Key::OpenBracket),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| {
                assert!(ctx.input(|input| input.key_pressed(egui::Key::OpenBracket)));
                assert!(!ctx.wants_keyboard_input());
            },
        );
    }

    #[test]
    fn editor_hover_and_panel_contact_keep_keyboard_ownership() {
        for pressed in [false, true] {
            let (ctx, id) = focused_text_editor();
            let viewport =
                egui::Rect::from_min_max(egui::pos2(400.0, 100.0), egui::pos2(900.0, 700.0));
            let raw = egui::RawInput {
                events: if pressed {
                    vec![egui::Event::PointerButton {
                        pos: egui::pos2(20.0, 20.0),
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    }]
                } else {
                    vec![egui::Event::PointerMoved(viewport.center())]
                },
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| {
                assert!(ctx.memory(|memory| memory.has_focus(id)));
                handoff_viewport_focus(ctx, viewport);
                assert!(ctx.memory(|memory| memory.has_focus(id)));
                assert!(ctx.wants_keyboard_input());
            });
        }
    }
}
