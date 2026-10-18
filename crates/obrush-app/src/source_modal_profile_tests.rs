//! Normal Egui deliveries use the real production reconciliation/gates. The
//! shared core Legacy brush/history is exercised; GPU/full-app UI is separate.
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
fn raw(events: Vec<Event>, focused: bool) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(100., 100.))),
        events,
        focused,
        ..Default::default()
    }
}
fn start_intensity_from_source_key(ctx: &egui::Context, state: &mut ModalState) {
    let resolver = ShortcutResolver::new(PreferenceProfile::SculptGl);
    for e in ctx.input(|i| i.events.clone()) {
        if let Event::Key {
            key: egui::Key::C,
            pressed: true,
            repeat: false,
            ..
        } = e
        {
            let commands = resolver.resolve(
                KeyDelivery {
                    code: 67,
                    pressed: true,
                    repeat: false,
                    modifiers: KeyModifiers::default(),
                },
                ShortcutContext::default(),
            );
            assert_eq!(commands, [Command::ModalStart(Modal::Intensity)]);
            if delivery_focus_admitted(ctx, true) {
                assert!(state.start(Modal::Intensity, None, true));
            }
        }
        observe_primary_event(state, &e);
    }
}
fn c_down() -> Event {
    Event::Key {
        key: egui::Key::C,
        physical_key: Some(egui::Key::C),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn actual_legacy_primary_release_after_source_handoff_allows_fresh_core_brush_and_history() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    let _ = ctx.run(raw(vec![c_down(), pointer(true)], true), |ctx| {
        state.begin_frame();
        start_intensity_from_source_key(ctx, &mut state);
        assert!(state.primary_down && state.suppress_contact);
    });
    let _ = ctx.run(
        raw(vec![Event::PointerMoved(Pos2::new(55., 50.))], true),
        |ctx| {
            state.begin_frame();
            reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
            state.cancel("profile or focus changed");
            assert!(contact_owned(&state));
            assert!(state.suppress_contact);
        },
    );
    let _ = ctx.run(raw(vec![pointer(false)], true), |ctx| {
        state.begin_frame();
        reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
        state.cancel("profile or focus changed");
        assert!(!state.primary_down && !state.suppress_contact);
        assert!(contact_owned(&state)); // the release frame itself stays quarantined
    });
    let mut doc = Document::new(Mesh::sphere(1));
    let before = doc.to_json().unwrap();
    let mut changed = 0;
    let _ = ctx.run(
        raw(
            vec![Event::PointerMoved(Pos2::new(50., 50.)), pointer(true)],
            true,
        ),
        |ctx| {
            state.begin_frame();
            reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
            state.cancel("profile or focus changed");
            assert!(!contact_owned(&state));
            assert!(ctx.input(|i| i.pointer.primary_pressed()));
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
            changed = doc.require_active_mesh_mut().unwrap().apply_brush(
                hit.position,
                hit.normal,
                Vec3::ZERO,
                &settings,
            );
        },
    );
    assert!(changed > 0);
    let _ = ctx.run(raw(vec![pointer(false)], true), |ctx| {
        state.begin_frame();
        reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
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
fn legacy_reconciliation_observes_buttons_and_never_applies_source_pointer_settings() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    let _ = ctx.run(raw(vec![c_down(), pointer(true)], true), |ctx| {
        state.begin_frame();
        start_intensity_from_source_key(ctx, &mut state);
    });
    let last = state.last_pointer;
    let _ = ctx.run(
        raw(
            vec![Event::PointerMoved(Pos2::new(90., 10.)), pointer(false)],
            true,
        ),
        |ctx| {
            state.begin_frame();
            reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
            state.cancel("profile or focus changed");
        },
    );
    assert_eq!(state.last_pointer, last);
    assert!(!state.active());
    assert!(state.queries.rows().is_empty());
    assert!(!state.suppress_contact);
}

#[test]
fn source_primary_observation_remains_ordered_in_its_own_walker() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    let _ = ctx.run(raw(vec![pointer(true)], true), |ctx| {
        state.begin_frame();
        reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::SculptGl);
        assert!(!state.primary_down);
    });
}

#[test]
fn final_unfocused_queued_key_cannot_rearm_even_before_window_event_is_walked() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    let _ = ctx.run(
        raw(
            vec![
                c_down(),
                Event::WindowFocused(false),
                Event::PointerMoved(Pos2::new(90., 50.)),
            ],
            false,
        ),
        |ctx| {
            state.begin_frame();
            start_intensity_from_source_key(ctx, &mut state);
            assert!(!state.active());
            assert!(!delivery_focus_admitted(ctx, true));
            assert_eq!(
                state.move_pointer([90., 50.], Some(0.35), true),
                Movement::default()
            );
        },
    );
}

#[test]
fn ordered_focus_loss_cancels_before_move_and_blocks_rearm_until_actual_focus_gain() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    let mut event_focused = true;
    let _ = ctx.run(raw(vec![c_down()], true), |ctx| {
        state.begin_frame();
        start_intensity_from_source_key(ctx, &mut state);
    });
    let _ = ctx.run(
        raw(
            vec![
                Event::WindowFocused(false),
                Event::PointerMoved(Pos2::new(90., 50.)),
                Event::WindowFocused(true),
            ],
            true,
        ),
        |ctx| {
            state.begin_frame();
            for event in ctx.input(|i| i.events.clone()) {
                observe_window_focus(&mut state, &event, &mut event_focused);
                if let Event::PointerMoved(p) = event {
                    assert!(!delivery_focus_admitted(ctx, event_focused));
                    assert_eq!(
                        state.move_pointer([f64::from(p.x), f64::from(p.y)], Some(0.35), true),
                        Movement::default()
                    );
                }
            }
            assert!(delivery_focus_admitted(ctx, event_focused));
            assert!(!state.active());
        },
    );
}

#[test]
fn pending_first_picker_epoch_is_not_replaced_by_second_sameframe_press() {
    let mut state = ModalState::default();
    assert!(picker_epoch_available(&state, false));
    assert!(state.start(Modal::Picker, None, true));
    state.next_event();
    state.move_pointer([10., 20.], None, false);
    let first = state.queries.rows()[0];
    state.end(Modal::Picker);
    assert!(!picker_epoch_available(&state, true));
    if picker_epoch_available(&state, true) {
        state.start(Modal::Picker, None, true);
    }
    state.next_event();
    state.move_pointer([30., 40.], None, false);
    state.end(Modal::Picker);
    assert_eq!(state.queries.rows(), [first]);
    assert_eq!(state.epoch, first.epoch);
    assert_eq!(first.screen, [10., 20.]);
}

#[test]
fn ended_empty_owner_is_unavailable_this_frame_and_nextframe_cleanup_is_explicit() {
    let mut state = ModalState::default();
    state.start(Modal::Picker, None, true);
    state.end(Modal::Picker);
    assert!(!picker_epoch_available(&state, true));
    state.begin_frame();
    assert!(picker_epoch_available(&state, false));
    assert!(state.queries.rows().is_empty());
    assert!(!state.active());
}
