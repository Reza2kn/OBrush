//! Ordered-focus regressions use a single real Context across loss and gain.
//! Shared production contact adapters are exercised without fake up/focus/GL.
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
fn c_down() -> Event {
    Event::Key {
        key: egui::Key::C,
        physical_key: Some(egui::Key::C),
        pressed: true,
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
fn source_walk(ctx: &egui::Context, state: &mut ModalState) {
    let (final_focused, events) = ctx.input(|i| (i.focused, i.events.clone()));
    let mut ordered = ordered_frame_focus(state, final_focused);
    if !final_focused {
        latch_focus_contact(state);
        state.cancel("profile or focus changed");
    }
    let resolver = ShortcutResolver::new(PreferenceProfile::SculptGl);
    for event in &events {
        observe_modal_contact_event(state, event, final_focused, &mut ordered, true);
        if matches!(
            event,
            Event::Key {
                key: egui::Key::C,
                pressed: true,
                repeat: false,
                ..
            }
        ) {
            let commands = resolver.resolve(
                KeyDelivery {
                    code: 67,
                    pressed: true,
                    repeat: false,
                    modifiers: KeyModifiers::default(),
                },
                ShortcutContext {
                    gui_focus: false,
                    idle: !state.primary_down && !state.picker,
                    camera_rotating: false,
                },
            );
            for command in commands {
                if let Command::ModalStart(kind) = command
                    && delivery_focus_admitted(ctx, ordered)
                {
                    state.start(kind, None, true);
                }
            }
        }
    }
}
fn own_then_lose(ctx: &egui::Context, state: &mut ModalState, profile: PreferenceProfile) {
    let _ = ctx.run(raw(vec![c_down(), pointer(true)], true), |ctx| {
        state.begin_frame();
        source_walk(ctx, state);
        assert!(state.intensity && state.primary_down && state.suppress_contact);
    });
    let _ = ctx.run(
        raw(vec![Event::WindowFocused(false), Event::PointerGone], false),
        |ctx| {
            state.begin_frame();
            if profile == PreferenceProfile::SculptGl {
                source_walk(ctx, state);
            } else {
                state.cancel("profile or focus changed");
                reconcile_legacy_primary(state, ctx, profile);
            }
            assert!(ctx.input(|i| i.pointer.primary_down()));
            assert!(state.focus_contact_recovery && state.suppress_contact);
            assert_eq!(state.ordered_window_focus, Some(false));
            assert!(!state.active());
        },
    );
}
fn useful_draw(doc: &mut Document) -> usize {
    let hit = doc
        .require_active_mesh()
        .unwrap()
        .raycast(Ray {
            origin: Vec3::new(0., 0., 3.),
            direction: -Vec3::Z,
        })
        .unwrap();
    doc.begin_stroke();
    doc.require_active_mesh_mut().unwrap().apply_brush(
        hit.position,
        hit.normal,
        Vec3::ZERO,
        &BrushSettings {
            kind: BrushKind::Draw,
            radius: 0.8,
            strength: 0.35,
            adaptive_detail: false,
            symmetry_x: false,
            ..Default::default()
        },
    )
}

#[test]
fn cf01_source_prior_loss_down_before_gain_stays_quarantined() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    own_then_lose(&ctx, &mut state, PreferenceProfile::SculptGl);
    let _ = ctx.run(
        raw(vec![pointer(true), Event::WindowFocused(true)], true),
        |ctx| {
            state.begin_frame();
            source_walk(ctx, &mut state);
            assert!(
                ctx.input(|i| i.focused && i.pointer.primary_down() && i.pointer.primary_pressed())
            );
            assert_eq!(state.ordered_window_focus, Some(true));
            assert!(
                state.focus_contact_recovery
                    && state.primary_down
                    && state.suppress_contact
                    && state.suppress_frame
            );
            assert!(!state.active());
            assert!(state.queries.rows().is_empty());
            assert!(contact_owned(&state));
            assert_eq!(
                state.move_pointer([80., 80.], Some(0.35), true),
                Movement::default()
            );
        },
    );
}

#[test]
fn cf02_legacy_prior_loss_down_before_gain_stays_quarantined() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    own_then_lose(&ctx, &mut state, PreferenceProfile::OBrush);
    let mut doc = Document::new(Mesh::sphere(1));
    let before = doc.to_json().unwrap();
    let mut changed = 0;
    let _ = ctx.run(
        raw(vec![pointer(true), Event::WindowFocused(true)], true),
        |ctx| {
            state.begin_frame();
            state.cancel("profile or focus changed");
            reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
            assert!(ctx.input(|i| i.focused && i.pointer.primary_pressed()));
            assert_eq!(state.ordered_window_focus, Some(true));
            if !contact_owned(&state) && ctx.input(|i| i.pointer.primary_pressed()) {
                changed = useful_draw(&mut doc);
            }
            assert!(
                state.focus_contact_recovery
                    && state.primary_down
                    && state.suppress_contact
                    && contact_owned(&state)
            );
        },
    );
    assert_eq!(changed, 0);
    assert_eq!(doc.to_json().unwrap(), before);
    assert!(!doc.can_undo());
}

#[test]
fn cf03_source_prior_loss_gain_before_down_admits_new_press() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    own_then_lose(&ctx, &mut state, PreferenceProfile::SculptGl);
    let _ = ctx.run(
        raw(vec![Event::WindowFocused(true), pointer(true)], true),
        |ctx| {
            state.begin_frame();
            source_walk(ctx, &mut state);
            assert!(
                ctx.input(|i| i.focused && i.pointer.primary_down() && i.pointer.primary_pressed())
            );
            assert_eq!(state.ordered_window_focus, Some(true));
            assert!(state.primary_down);
            assert!(
                !state.focus_contact_recovery
                    && !state.suppress_contact
                    && !state.suppress_frame
                    && !contact_owned(&state)
            );
            assert!(!state.active());
            assert_eq!(state.radius_anchor, None);
            assert!(state.queries.rows().is_empty());
        },
    );
}

#[test]
fn cf04_legacy_prior_loss_gain_before_down_preserves_core_history() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    own_then_lose(&ctx, &mut state, PreferenceProfile::OBrush);
    let mut doc = Document::new(Mesh::sphere(1));
    let before = doc.to_json().unwrap();
    let mut changed = 0;
    let _ = ctx.run(
        raw(vec![Event::WindowFocused(true), pointer(true)], true),
        |ctx| {
            state.begin_frame();
            state.cancel("profile or focus changed");
            reconcile_legacy_primary(&mut state, ctx, PreferenceProfile::OBrush);
            assert!(
                ctx.input(|i| i.focused && i.pointer.primary_down() && i.pointer.primary_pressed())
            );
            assert_eq!(state.ordered_window_focus, Some(true));
            assert!(!contact_owned(&state));
            assert!(
                !state.focus_contact_recovery && !state.suppress_contact && !state.suppress_frame
            );
            changed = useful_draw(&mut doc);
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
fn cf05_first_boot_focused_without_focus_event_preserves_defaults() {
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    assert_eq!(state.ordered_window_focus, None);
    assert!(!state.focus_contact_recovery);
    let _ = ctx.run(raw(vec![c_down(), pointer(true)], true), |ctx| {
        state.begin_frame();
        source_walk(ctx, &mut state);
        assert_eq!(state.ordered_window_focus, None);
        assert!(ctx.input(|i| i.focused && i.pointer.primary_pressed()));
        assert!(
            state.intensity && state.primary_down && state.suppress_contact && state.suppress_frame
        );
    });
}

#[test]
fn cf06_real_up_path_and_compatibility_walks_keep_focus_history() {
    for profile in [PreferenceProfile::SculptGl, PreferenceProfile::OBrush] {
        let ctx = egui::Context::default();
        let mut state = ModalState::default();
        own_then_lose(&ctx, &mut state, profile);
        let _ = ctx.run(raw(vec![pointer(false)], false), |ctx| {
            state.begin_frame();
            if profile == PreferenceProfile::SculptGl {
                source_walk(ctx, &mut state);
            } else {
                state.cancel("profile or focus changed");
                reconcile_legacy_primary(&mut state, ctx, profile);
            }
            assert!(ctx.input(|i| i.pointer.primary_released() && !i.pointer.primary_down()));
            assert!(
                !state.focus_contact_recovery && !state.primary_down && !state.suppress_contact
            );
            assert_eq!(state.ordered_window_focus, Some(false));
            assert!(state.suppress_frame);
        });
        let _ = ctx.run(raw(vec![], true), |ctx| {
            state.begin_frame();
            if profile == PreferenceProfile::SculptGl {
                source_walk(ctx, &mut state);
            } else {
                state.cancel("profile or focus changed");
                reconcile_legacy_primary(&mut state, ctx, profile);
            }
            assert_eq!(state.ordered_window_focus, Some(false));
            assert!(!state.active());
        });
        let _ = ctx.run(
            raw(vec![Event::WindowFocused(true), pointer(true)], true),
            |ctx| {
                state.begin_frame();
                if profile == PreferenceProfile::SculptGl {
                    source_walk(ctx, &mut state);
                } else {
                    state.cancel("profile or focus changed");
                    reconcile_legacy_primary(&mut state, ctx, profile);
                }
                assert_eq!(state.ordered_window_focus, Some(true));
                assert!(state.primary_down);
                assert!(!state.suppress_contact && !state.suppress_frame && !contact_owned(&state));
            },
        );
    }
    // An actual final-unfocused snapshot without a queued false event also
    // closes admission. No synthetic WindowFocused or button event is added.
    let ctx = egui::Context::default();
    let mut state = ModalState::default();
    let _ = ctx.run(raw(vec![], false), |ctx| {
        state.begin_frame();
        source_walk(ctx, &mut state);
        assert_eq!(state.ordered_window_focus, Some(false));
    });
    let _ = ctx.run(raw(vec![c_down()], true), |ctx| {
        state.begin_frame();
        source_walk(ctx, &mut state);
        assert!(!state.active());
        assert_eq!(state.ordered_window_focus, Some(false));
    });
}
