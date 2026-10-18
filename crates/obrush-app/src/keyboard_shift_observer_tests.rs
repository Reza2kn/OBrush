//! Actual egui input observation; independently authored, not browser delivery proof.
use super::keyboard_input_observation;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2};

fn tab(pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key: Key::Tab,
        physical_key: Some(Key::Tab),
        pressed,
        repeat: false,
        modifiers,
    }
}

fn context_with_held_pointer() -> egui::Context {
    let ctx = egui::Context::default();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 400.0))),
            events: vec![
                Event::PointerMoved(Pos2::new(400.0, 300.0)),
                Event::PointerButton {
                    pos: Pos2::new(400.0, 300.0),
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = ui.button("First main widget");
                let _ = ui.button("Last preferences widget");
            });
        },
    );
    assert!(ctx.input(|input| input.pointer.primary_down()));
    ctx
}

fn observed_frame(
    ctx: &egui::Context,
    events: Vec<Event>,
    modifiers: Modifiers,
) -> (serde_json::Value, serde_json::Value) {
    let mut observed = None;
    let _ = ctx.run(
        egui::RawInput {
            events,
            modifiers,
            ..Default::default()
        },
        |ctx| {
            let focus_before = ctx.memory(|memory| memory.focused());
            let input_before = ctx.input(|input| {
                (
                    input.events.clone(),
                    input.modifiers,
                    input.pointer.primary_down(),
                )
            });
            let before = keyboard_input_observation(ctx, "before_shortcuts").unwrap();
            let after = keyboard_input_observation(ctx, "after_shortcuts").unwrap();
            assert_eq!(ctx.memory(|memory| memory.focused()), focus_before);
            assert_eq!(
                ctx.input(|input| (
                    input.events.clone(),
                    input.modifiers,
                    input.pointer.primary_down()
                )),
                input_before,
                "The production observer must not consume events or change input ownership"
            );
            assert_eq!(before["frame"], after["frame"]);
            assert_eq!(before["keys"], after["keys"]);
            assert_eq!(before["shift"], after["shift"]);
            assert_eq!(before["primary_down"], true);
            assert_eq!(before["primary_released"], false);
            observed = Some((before, after));
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = ui.button("First main widget");
                let _ = ui.button("Last preferences widget");
            });
        },
    );
    observed.unwrap()
}

#[test]
fn same_processed_reverse_chord_records_event_shift_and_final_clear_without_consuming() {
    let ctx = context_with_held_pointer();
    let (before, after) = observed_frame(
        &ctx,
        vec![tab(true, Modifiers::SHIFT), tab(false, Modifiers::NONE)],
        Modifiers::NONE,
    );
    assert_eq!(before["shift"], false);
    assert_eq!(before["command"], false);
    assert_eq!(before["keys"].as_array().unwrap().len(), 2);
    assert_eq!(
        before["keys"][0],
        serde_json::json!({"key":"Tab","pressed":true,"repeat":false,"shift":true})
    );
    assert_eq!(
        after["keys"][1],
        serde_json::json!({"key":"Tab","pressed":false,"repeat":false,"shift":false})
    );
    assert!(ctx.input(|input| input.key_pressed(Key::Tab)));
    assert!(!ctx.input(|input| input.modifiers.shift));
}

#[test]
fn split_processed_reverse_chord_exposes_shift_and_distinct_frames_instead_of_safe_witness() {
    let ctx = context_with_held_pointer();
    let (press, _) = observed_frame(&ctx, vec![tab(true, Modifiers::SHIFT)], Modifiers::SHIFT);
    let (release, after_release) =
        observed_frame(&ctx, vec![tab(false, Modifiers::NONE)], Modifiers::NONE);
    assert_eq!(press["shift"], true);
    assert_eq!(press["keys"][0]["pressed"], true);
    assert_eq!(press["keys"][0]["shift"], true);
    assert_eq!(release["shift"], false);
    assert_eq!(release["keys"][0]["pressed"], false);
    assert_eq!(release["keys"][0]["shift"], false);
    assert!(release["frame"].as_u64().unwrap() > press["frame"].as_u64().unwrap());
    assert_eq!(release["frame"], after_release["frame"]);
    assert_ne!(
        press["frame"], release["frame"],
        "Actual split delivery is not an admissible same-frame held-chord witness"
    );
}
