//! Real Context event deliveries exercise the shared production contact adapter.
//! No pointer/focus setter, synthetic up, fake GL or OBrushApp is constructed.
//! Whole application/native/browser consumers remain separate required gates.
use super::*;
use crate::shortcut_profile::{
    Command, KeyDelivery, Modifiers as KeyModifiers, ShortcutContext, ShortcutResolver,
};
use eframe::egui::{Event, Modifiers, PointerButton, Rect, Vec2};

fn pointer(pressed: bool) -> Event {
    Event::PointerButton {
        pos: Pos2::new(50., 50.),
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}
fn moved(x: f32, y: f32) -> Event {
    Event::PointerMoved(Pos2::new(x, y))
}
fn key(key: egui::Key, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}
fn raw(events: Vec<Event>, focused: bool) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(100., 100.))),
        events,
        focused,
        ..Default::default()
    }
}
/// Idle fixture owners use the actual Source resolver and production ordered
/// focus/button adapter. Only exposed C/X are admitted here; S needs its own
/// checked tiny snapshot in O07. This never changes Context input state.
fn source_walk(ctx: &egui::Context, state: &mut ModalState, picker_ready: bool) -> Vec<Movement> {
    let (final_focused, events) = ctx.input(|i| (i.focused, i.events.clone()));
    let mut event_focused = final_focused;
    if !final_focused {
        latch_focus_contact(state);
        state.cancel("profile or focus changed");
    }
    let resolver = ShortcutResolver::new(PreferenceProfile::SculptGl);
    let mut movements = Vec::new();
    for event in &events {
        observe_modal_contact_event(state, event, final_focused, &mut event_focused, true);
        match event {
            Event::Key {
                key,
                pressed,
                repeat,
                modifiers,
                ..
            } => {
                let code = match key {
                    egui::Key::C => 67,
                    egui::Key::X => 88,
                    egui::Key::S => 83,
                    _ => continue,
                };
                let input = KeyDelivery {
                    code,
                    pressed: *pressed,
                    repeat: *repeat,
                    modifiers: KeyModifiers {
                        control: modifiers.ctrl,
                        shift: modifiers.shift,
                        alt: modifiers.alt,
                        command: modifiers.command,
                    },
                };
                let context = ShortcutContext {
                    gui_focus: false,
                    idle: !state.primary_down && !state.picker,
                    camera_rotating: false,
                };
                for command in resolver.resolve(input, context) {
                    match command {
                        Command::ModalStart(kind)
                            if delivery_focus_admitted(ctx, event_focused) =>
                        {
                            state.start(kind, Some(50.), kind != Modal::Picker || picker_ready);
                        }
                        Command::ModalEnd(kind) => state.end(kind),
                        _ => {}
                    }
                }
            }
            Event::PointerMoved(p) => movements.push(state.move_pointer(
                [f64::from(p.x), f64::from(p.y)],
                Some(0.35),
                true,
            )),
            Event::PointerButton { pos, .. } => {
                state.last_pointer = Some([f64::from(pos.x), f64::from(pos.y)])
            }
            Event::PointerGone => state.cancel("pointer left window"),
            _ => {}
        }
    }
    movements
}
fn owned_contact(ctx: &egui::Context, state: &mut ModalState) {
    let _ = ctx.run(
        raw(
            vec![moved(50., 50.), key(egui::Key::C, true), pointer(true)],
            true,
        ),
        |ctx| {
            state.begin_frame();
            source_walk(ctx, state, false);
            assert!(state.intensity && state.primary_down && state.suppress_contact);
            assert!(ctx.input(|i| i.pointer.primary_down() && i.pointer.primary_pressed()));
            assert!(!state.focus_contact_recovery);
        },
    );
}
fn lost_and_regained_without_up(
    ctx: &egui::Context,
    state: &mut ModalState,
    profile: PreferenceProfile,
) {
    let _ = ctx.run(
        raw(vec![Event::WindowFocused(false), Event::PointerGone], false),
        |ctx| {
            state.begin_frame();
            if profile == PreferenceProfile::SculptGl {
                source_walk(ctx, state, false);
            } else {
                state.cancel("profile or focus changed");
                reconcile_legacy_primary(state, ctx, profile);
            }
            assert!(ctx.input(|i| i.pointer.primary_down()));
            assert!(state.primary_down && state.suppress_contact && state.focus_contact_recovery);
            assert!(!state.active());
            assert!(state.queries.rows().is_empty());
        },
    );
    let _ = ctx.run(raw(vec![Event::WindowFocused(true)], true), |ctx| {
        state.begin_frame();
        if profile == PreferenceProfile::SculptGl {
            source_walk(ctx, state, false);
        } else {
            state.cancel("profile or focus changed");
            reconcile_legacy_primary(state, ctx, profile);
        }
        assert!(ctx.input(|i| i.pointer.primary_down()));
        assert!(!ctx.input(|i| i.pointer.primary_pressed() || i.pointer.primary_released()));
        assert!(state.primary_down && state.suppress_contact && state.focus_contact_recovery);
        assert!(!state.active());
        assert!(contact_owned(state));
    });
}
fn useful_legacy_draw(doc: &mut Document) -> usize {
    let hit = doc
        .require_active_mesh()
        .unwrap()
        .raycast(Ray {
            origin: Vec3::new(0., 0., 3.),
            direction: -Vec3::Z,
        })
        .unwrap();
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        radius: 0.8,
        strength: 0.35,
        adaptive_detail: false,
        symmetry_x: false,
        ..Default::default()
    };
    doc.begin_stroke();
    doc.require_active_mesh_mut().unwrap().apply_brush(
        hit.position,
        hit.normal,
        Vec3::ZERO,
        &settings,
    )
}

#[test]
fn o01_actual_egui_focus_loss_retains_primary_without_buttonup() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    owned_contact(&ctx, &mut state);
    lost_and_regained_without_up(&ctx, &mut state, PreferenceProfile::SculptGl);
    let _ = ctx.run(raw(vec![], true), |ctx| {
        state.begin_frame();
        assert!(source_walk(ctx, &mut state, false).is_empty());
        assert!(ctx.input(|i| i.pointer.primary_down()));
        assert!(!ctx.input(|i| i.pointer.primary_pressed()));
        assert!(state.focus_contact_recovery && contact_owned(&state));
        assert!(!state.active());
    });
}

#[test]
fn o02_fresh_focused_primary_recovers_lost_outside_release_and_legacy_history() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    owned_contact(&ctx, &mut state);
    lost_and_regained_without_up(&ctx, &mut state, PreferenceProfile::OBrush);
    let mut doc = Document::new(Mesh::sphere(1));
    let before = doc.to_json().unwrap();
    let mut changed = 0;
    // A genuine new down follows focus regain. There was no up delivery during
    // the lost contact; the next actual up belongs to this new ordinary stroke.
    let _ = ctx.run(raw(vec![moved(50., 50.), pointer(true)], true), |ctx| {
        state.begin_frame();
        state.cancel("profile or focus changed");
        reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
        assert!(
            ctx.input(|i| i.focused && i.pointer.primary_down() && i.pointer.primary_pressed())
        );
        assert!(state.primary_down);
        assert!(!state.suppress_contact && !state.suppress_frame);
        assert!(!state.focus_contact_recovery && !contact_owned(&state));
        assert!(!state.active());
        changed = useful_legacy_draw(&mut doc);
    });
    assert!(changed > 0);
    let _ = ctx.run(raw(vec![pointer(false)], true), |ctx| {
        state.begin_frame();
        reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
        assert!(ctx.input(|i| i.pointer.primary_released()));
        assert!(!state.primary_down);
        doc.end_stroke();
    });
    let after = doc.to_json().unwrap();
    assert_ne!(after, before);
    assert!(doc.undo());
    assert_eq!(doc.to_json().unwrap(), before);
    assert!(!doc.can_undo());
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), after);
}

#[test]
fn o03_source_profile_fresh_primary_is_ordinary_not_modal_resume() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    owned_contact(&ctx, &mut state);
    lost_and_regained_without_up(&ctx, &mut state, PreferenceProfile::SculptGl);
    let doc = Document::new(Mesh::sphere(0));
    let before = doc.to_json().unwrap();
    let _ = ctx.run(raw(vec![pointer(true), moved(75., 70.)], true), |ctx| {
        state.begin_frame();
        let moves = source_walk(ctx, &mut state, false);
        assert!(ctx.input(|i| i.pointer.primary_pressed() && i.pointer.primary_down()));
        assert!(!state.active());
        assert_eq!(state.radius_anchor, None);
        assert!(state.queries.rows().is_empty());
        assert!(!state.suppress_contact && !state.suppress_frame && !state.focus_contact_recovery);
        assert_eq!(moves, [Movement::default()]);
        assert_eq!(doc.to_json().unwrap(), before);
        assert!(!doc.can_undo());
    });
}

#[test]
fn o04_refocus_and_held_move_do_not_clear_quarantine() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    owned_contact(&ctx, &mut state);
    lost_and_regained_without_up(&ctx, &mut state, PreferenceProfile::SculptGl);
    let _ = ctx.run(
        raw(
            vec![moved(90., 80.), key(egui::Key::X, true), moved(20., 10.)],
            true,
        ),
        |ctx| {
            state.begin_frame();
            let movements = source_walk(ctx, &mut state, false);
            assert_eq!(movements, [Movement::default(), Movement::default()]);
            assert!(ctx.input(|i| i.pointer.primary_down()));
            assert!(!ctx.input(|i| i.pointer.primary_pressed()));
            assert!(
                state.primary_down
                    && state.suppress_contact
                    && state.focus_contact_recovery
                    && contact_owned(&state)
            );
            assert!(!state.active());
            assert!(state.queries.rows().is_empty());
        },
    );
}

#[test]
fn o05_unfocused_and_ordered_unfocused_new_down_do_not_recover() {
    for (events, final_focus) in [
        (vec![pointer(true)], false),
        (
            vec![
                Event::WindowFocused(false),
                pointer(true),
                Event::WindowFocused(true),
            ],
            true,
        ),
    ] {
        let ctx = egui::Context::default();
        let mut state = ModalState::default();
        owned_contact(&ctx, &mut state);
        lost_and_regained_without_up(&ctx, &mut state, PreferenceProfile::SculptGl);
        let _ = ctx.run(raw(events, final_focus), |ctx| {
            state.begin_frame();
            source_walk(ctx, &mut state, false);
            assert!(state.primary_down && state.suppress_contact && state.focus_contact_recovery);
            assert!(contact_owned(&state));
            assert!(!state.active());
            assert!(state.queries.rows().is_empty());
        });
    }
}

#[test]
fn o06_down_without_focus_discontinuity_retains_owned_contact() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    owned_contact(&ctx, &mut state);
    let _ = ctx.run(
        raw(
            vec![key(egui::Key::C, false), pointer(true), moved(90., 90.)],
            true,
        ),
        |ctx| {
            state.begin_frame();
            let moves = source_walk(ctx, &mut state, false);
            assert!(ctx.input(|i| i.pointer.primary_pressed()));
            assert!(!state.active());
            assert!(
                state.primary_down
                    && state.suppress_contact
                    && state.suppress_frame
                    && contact_owned(&state)
            );
            assert!(!state.focus_contact_recovery);
            assert_eq!(moves, [Movement::default()]);
        },
    );
}

#[test]
fn o07_ordered_sameframe_focus_loss_regain_freshdown_preserves_new_press() {
    let mesh = Mesh::import_obj(b"v -1 -1 0\nv 1 -1 0\nv 1 1 0\nv -1 1 0\nf 1 2 3 4\n")
        .unwrap()
        .mesh;
    let mut admission = SourcePickSnapshotAdmission::new(Arc::new(mesh)).unwrap();
    while !admission.step(16384).unwrap().ready {}
    let snapshot = admission.finish().unwrap();
    assert!(snapshot.complete_query_supported());
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    let _ = ctx.run(
        raw(vec![moved(50., 50.), key(egui::Key::S, true)], true),
        |ctx| {
            state.begin_frame();
            source_walk(ctx, &mut state, snapshot.complete_query_supported());
            assert!(state.picker);
        },
    );
    let mut doc = Document::new(Mesh::sphere(1));
    let first = doc.to_json().unwrap();
    doc.replace_mesh(Mesh::sphere(2)).unwrap();
    let prior = doc.to_json().unwrap();
    let mut changed = 0;
    let _ = ctx.run(
        raw(
            vec![
                moved(55., 50.),
                pointer(true),
                Event::WindowFocused(false),
                Event::WindowFocused(true),
                pointer(true),
            ],
            true,
        ),
        |ctx| {
            state.begin_frame();
            source_walk(ctx, &mut state, snapshot.complete_query_supported());
            assert!(ctx.input(|i| i.pointer.primary_pressed() && i.pointer.primary_down()));
            assert!(!state.active());
            assert!(state.queries.rows().is_empty());
            assert_eq!(state.radius_anchor, None);
            assert!(
                !state.focus_contact_recovery && !state.suppress_contact && !state.suppress_frame
            );
            changed = useful_legacy_draw(&mut doc);
        },
    );
    assert!(changed > 0);
    let _ = ctx.run(raw(vec![pointer(false)], true), |ctx| {
        state.begin_frame();
        source_walk(ctx, &mut state, false);
        doc.end_stroke();
    });
    let after = doc.to_json().unwrap();
    assert_ne!(after, prior);
    assert!(doc.undo());
    assert_eq!(doc.to_json().unwrap(), prior);
    assert!(doc.undo());
    assert_eq!(doc.to_json().unwrap(), first);
    assert!(!doc.can_undo());
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), prior);
    assert!(doc.redo());
    assert_eq!(doc.to_json().unwrap(), after);
}

#[test]
fn o08_genuine_release_then_next_fresh_contact_remains_exact() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    owned_contact(&ctx, &mut state);
    lost_and_regained_without_up(&ctx, &mut state, PreferenceProfile::OBrush);
    let _ = ctx.run(raw(vec![pointer(false)], true), |ctx| {
        state.begin_frame();
        state.cancel("profile or focus changed");
        reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
        assert!(ctx.input(|i| i.pointer.primary_released() && !i.pointer.primary_down()));
        assert!(!state.primary_down && !state.suppress_contact && !state.focus_contact_recovery);
        assert!(state.suppress_frame && contact_owned(&state));
    });
    let _ = ctx.run(raw(vec![pointer(true)], true), |ctx| {
        state.begin_frame();
        state.cancel("profile or focus changed");
        reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
        assert!(ctx.input(|i| i.pointer.primary_pressed()));
        assert!(state.primary_down);
        assert!(!state.suppress_contact && !state.suppress_frame && !state.focus_contact_recovery);
        assert!(!contact_owned(&state));
    });
}
