//! Real shared production UI/session/queue/clock contracts, not browser/reference proof.
use crate::continuous_host::{
    ClockPolicy, ContinuousMode, HostContinuousClock, Readiness, StopReason, StrokeKey,
};
use crate::dab_queue::DabQueue;
use crate::source_paint_host::{
    self, PaintMode, RawPressureSample, SharedSourcePaint, SourcePaintInput,
    SourcePaintPreferences, SourcePaintSession,
};
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect};
use obrush_core::{BrushKind, BrushSettings, Document, MaskConvention, Mesh, PaintViewCapture};
fn doc() -> Document {
    let mut m = Mesh::from_obj("v 0 0 0\nv .5 0 0\nv 0 .5 0\nf 1 2 3\n").unwrap();
    m.mask_convention = MaskConvention::UnprotectedF32V1;
    m.masks.fill(1.);
    m.colors.fill([0.; 3]);
    m.materials = Some(vec![[0.25, 0.75]; 3]);
    Document::new(m)
}
fn prefs() -> SourcePaintPreferences {
    let mut p = SourcePaintPreferences {
        mode: PaintMode::SourceOrdinary,
        ..Default::default()
    };
    p.options.intensity = 0.5;
    p.options.hardness = 1.;
    p.options.color = [1., 0.5, 0.25];
    p.options.roughness = 0.75;
    p.options.metallic = 0.25;
    p
}
fn settings() -> BrushSettings {
    BrushSettings {
        kind: BrushKind::Paint,
        symmetry_x: false,
        adaptive_detail: false,
        ..Default::default()
    }
}
fn input(x: f64, y: f64) -> SourcePaintInput {
    SourcePaintInput {
        view: PaintViewCapture {
            world_to_clip: [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., -1., 0., 0., 0., 0., 1.,
            ],
            viewport: [0., 0., 1000., 1000.],
        },
        screen: [x, y],
        raw: RawPressureSample {
            pressure: 0.5,
            pen: false,
            browser_number: true,
            absent: true,
        },
        in_contact: true,
    }
}
fn session(d: &mut Document, continuous: bool) -> SharedSourcePaint {
    SourcePaintSession::prepare_for_stroke(
        d,
        &prefs(),
        &settings(),
        true,
        continuous,
        input(500., 500.),
    )
    .unwrap()
}
fn rgb(d: &Document) -> Vec<[u32; 3]> {
    d.require_active_mesh()
        .unwrap()
        .colors
        .iter()
        .map(|c| c.map(f32::to_bits))
        .collect()
}
fn first(d: &Document) {
    assert_eq!(rgb(d), [[0x3f000000, 0x3e800000, 0x3e000000]; 3]);
}
#[test]
fn h01_actual_lock_checkbox_intent_commits_the_shared_per_tool_preference_candidate() {
    let ctx = egui::Context::default();
    super::configure_theme(&ctx);
    let mut locked = false;
    let mut response = None;
    let mut frame = |events: Vec<Event>, locked: &mut bool| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(176., 100.))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_width(160.);
                    let clip = ui.clip_rect();
                    let r = source_paint_host::render_lock_control(ui, locked);
                    assert!(r.enabled() && clip.contains_rect(r.interact_rect));
                    response = Some(r);
                });
            },
        );
        response.clone().unwrap()
    };
    let point = frame(Vec::new(), &mut locked).interact_rect.center();
    for pressed in [true, false] {
        frame(
            vec![
                Event::PointerMoved(point),
                Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
            &mut locked,
        );
    }
    assert!(locked);
    assert_eq!(
        source_paint_host::SOURCE_PAINT_LOCK_CONTROL,
        "source_paint.lock_position"
    );
    let b = settings();
    let store =
        crate::brush_preferences::BrushPreferenceStore::from_legacy(&b, None, false).unwrap();
    let candidate = super::preference_view_candidate(&store, &b, false, 75., locked)
        .unwrap()
        .unwrap();
    assert!(candidate.tool(BrushKind::Paint).lock_position);
    assert!(!candidate.tool(BrushKind::Draw).lock_position);
}
#[test]
fn h02_press_is_no_dab_then_normal_locked_session_uses_source_half_not_legacy_kernel() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let s = session(&mut d, false);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    let r = source_paint_host::apply_shared(&mut d, &s, input(1000., 500.)).unwrap();
    assert_eq!(r.changed_vertices, 3);
    first(&d);
    s.lock().unwrap().finish_locked(&mut d).unwrap();
    assert!(d.can_undo());
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn h03_real_dab_queue_keeps_only_locked_release_endpoint_and_has_no_tail() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let s = session(&mut d, false);
    let mut q = DabQueue::default();
    q.begin();
    for x in 501..541 {
        for sample in s
            .lock()
            .unwrap()
            .path_samples(input(f64::from(x), 500.))
            .unwrap()
        {
            q.source_paint_dab(s.clone(), sample);
        }
    }
    for sample in s.lock().unwrap().path_samples(input(1000., 500.)).unwrap() {
        q.source_paint_dab(s.clone(), sample);
    }
    let p = q.release_source_paint_locked(&mut d, &s);
    assert!(p.error.is_none() && p.changed);
    first(&d);
    assert_eq!(q.metrics()["pending_dabs"], 0);
    let after = d.to_json().unwrap();
    assert!(!q.process(&mut d, usize::MAX, None).changed);
    assert_eq!(d.to_json().unwrap(), after);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
fn ready(key: StrokeKey) -> Readiness {
    Readiness {
        key: Some(key),
        mode_enabled: true,
        primary_down: true,
        pressure: 1.,
        focused: true,
        navigating: false,
        target_visible: true,
        pointer_in_viewport: true,
        surface_hit: true,
        queue_empty: true,
        operation_busy: false,
    }
}
#[test]
fn h04_only_real_consumed_locked_permits_accumulate_shared_source_rows() {
    let mut d = doc();
    let s = session(&mut d, true);
    let before = d.to_json().unwrap();
    assert!(
        s.lock()
            .unwrap()
            .locked_timer_ready(&d, input(1000., 500.))
            .unwrap()
    );
    assert_eq!(d.to_json().unwrap(), before);
    let key = StrokeKey {
        serial: 77,
        subtool_id: 1,
        kind: BrushKind::Paint,
    };
    let mut clock = HostContinuousClock::new(ClockPolicy::default()).unwrap();
    clock.start_locked_accumulation(key, true, 0).unwrap();
    let mut q = DabQueue::default();
    q.begin();
    q.process(&mut d, usize::MAX, None);
    for (frame, now) in [(1, 16_600), (2, 33_200)] {
        let permit = clock.poll(frame, now, ready(key)).unwrap();
        assert_eq!(permit.mode(), ContinuousMode::LockedAccumulation);
        clock.consume(permit, frame, ready(key)).unwrap();
        q.source_paint_dab(s.clone(), input(1000., 500.));
        assert!(q.process(&mut d, 1, None).error.is_none());
    }
    assert_eq!(rgb(&d), [[0x3f400000, 0x3ec00000, 0x3e400000]; 3]);
    clock.stop(StopReason::Released);
    q.release_source_paint_locked(&mut d, &s);
    assert!(clock.poll(3, 49_800, ready(key)).is_none());
    assert_eq!(q.metrics()["pending_dabs"], 0);
    assert!(d.undo());
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn h05_pause_skips_timer_backlog_and_zero_travel_never_restores_accumulation() {
    let mut d = doc();
    let s = session(&mut d, true);
    let key = StrokeKey {
        serial: 78,
        subtool_id: 1,
        kind: BrushKind::Paint,
    };
    let mut clock = HostContinuousClock::new(ClockPolicy::default()).unwrap();
    clock.start_locked_accumulation(key, true, 0).unwrap();
    assert!(clock.poll(1, 1_000_000, ready(key)).is_none());
    let permit = clock.poll(2, 1_016_600, ready(key)).unwrap();
    clock.consume(permit, 2, ready(key)).unwrap();
    source_paint_host::apply_shared(&mut d, &s, input(1000., 500.)).unwrap();
    first(&d);
    let edited = d.to_json().unwrap();
    assert!(
        s.lock()
            .unwrap()
            .locked_timer_ready(&d, input(500., 500.))
            .unwrap()
    );
    assert_eq!(
        source_paint_host::apply_shared(&mut d, &s, input(500., 500.))
            .unwrap()
            .changed_vertices,
        0
    );
    assert_eq!(d.to_json().unwrap(), edited);
    clock.stop(StopReason::Released);
    s.lock().unwrap().finish_locked(&mut d).unwrap();
    assert!(clock.poll(3, 2_000_000, ready(key)).is_none());
}
#[test]
fn h06_new_normal_locked_admission_preserves_ordinary_and_unsupported_option_refusals() {
    let mut d = doc();
    assert!(source_paint_host::blocker(&d, &settings(), true).is_err());
    assert!(
        SourcePaintSession::prepare(&d, &prefs(), &settings(), true, input(500., 500.)).is_err()
    );
    assert!(source_paint_host::stroke_blocker(&d, &prefs(), &settings(), true).is_ok());
    for which in 0..3 {
        let mut bad = settings();
        match which {
            0 => bad.symmetry_x = true,
            1 => bad.invert = true,
            _ => bad.alpha = Some(obrush_core::RasterAlpha::new(1, 1, vec![255]).unwrap()),
        };
        let before = d.to_json().unwrap();
        assert!(
            SourcePaintSession::prepare_for_stroke(
                &mut d,
                &prefs(),
                &bad,
                true,
                false,
                input(500., 500.)
            )
            .is_err()
        );
        assert_eq!(d.to_json().unwrap(), before);
    }
}
#[test]
fn h07_live_delta_and_origin_decision_differ_from_kernel_rest_counts() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let s = session(&mut d, false);
    source_paint_host::apply_shared(&mut d, &s, input(1000., 500.)).unwrap();
    assert_eq!(
        source_paint_host::apply_shared(&mut d, &s, input(1000., 500.))
            .unwrap()
            .changed_vertices,
        0
    );
    assert_eq!(
        s.lock().unwrap().diagnostics()["locked"]["last_report"]["dab"]["changed_vertices"],
        3
    );
    let restored = source_paint_host::apply_shared(&mut d, &s, input(500., 500.)).unwrap();
    assert_eq!(restored.changed_rgb_vertices, 3);
    assert!(s.lock().unwrap().is_locked_at_original());
    s.lock().unwrap().finish_locked(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(!d.can_undo());
    // This shared producer is called AFTER actual release progress in app.rs.
    // A restoration itself changed RGB, yet the original display flags win.
    let (mut dirty, mut painted) = (true, true);
    source_paint_host::restore_locked_display(&mut dirty, &mut painted, Some((false, false)), true);
    assert_eq!((dirty, painted), (false, false));
    source_paint_host::restore_locked_display(
        &mut dirty,
        &mut painted,
        Some((false, false)),
        false,
    );
    assert_eq!((dirty, painted), (false, false));
    (dirty, painted) = (true, true);
    source_paint_host::restore_locked_display(
        &mut dirty,
        &mut painted,
        Some((false, false)),
        false,
    );
    assert_eq!((dirty, painted), (true, true));
}
#[test]
fn h08_no_channels_preserve_prior_real_redo_and_lazy_none_materials() {
    let mut d = doc();
    d.require_active_mesh_mut().unwrap().materials = None;
    d.begin_stroke();
    d.require_active_mesh_mut().unwrap().colors[0] = [0.125; 3];
    d.end_stroke();
    assert!(d.undo());
    let before = d.to_json().unwrap();
    let mut p = prefs();
    p.options.write_rgb = false;
    p.options.write_roughness = false;
    p.options.write_metallic = false;
    let s = SourcePaintSession::prepare_for_stroke(
        &mut d,
        &p,
        &settings(),
        true,
        false,
        input(500., 500.),
    )
    .unwrap();
    assert_eq!(
        source_paint_host::apply_shared(&mut d, &s, input(1000., 500.))
            .unwrap()
            .changed_vertices,
        0
    );
    s.lock().unwrap().finish_locked(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    assert!(d.require_active_mesh().unwrap().materials.is_none());
    assert!(d.source_geometry_state(1).is_none());
}
#[test]
fn h09_whole_owned_rejection_and_cancel_are_real_queue_history_operations() {
    let mut d = doc();
    let before = d.to_json().unwrap();
    let s = session(&mut d, false);
    let mut q = DabQueue::default();
    q.begin();
    q.source_paint_dab(s.clone(), input(1000., 500.));
    q.process(&mut d, usize::MAX, None);
    first(&d);
    d.name = "outside-slot public edit".into();
    q.source_paint_dab(s.clone(), input(1100., 500.));
    let p = q.process(&mut d, 1, None);
    assert!(p.error.is_some());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.take_layer_error().unwrap().contains("whole original"));
    q.release_source_paint_locked(&mut d, &s);
    assert!(!d.can_undo());
    let s = session(&mut d, false);
    source_paint_host::apply_shared(&mut d, &s, input(1000., 500.)).unwrap();
    s.lock().unwrap().cancel_locked(&mut d).unwrap();
    s.lock().unwrap().finish_locked(&mut d).unwrap();
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn h10_diagnostics_bind_precise_actual_anchor_radius_raw_input_and_owned_mode() {
    let mut d = doc();
    let s = session(&mut d, false);
    source_paint_host::apply_shared(&mut d, &s, input(1000., 500.)).unwrap();
    let r = s.lock().unwrap().diagnostics();
    let prep = &r["locked"]["preparation"];
    assert_eq!(prep["press64"], serde_json::json!([500., 500.]));
    assert_eq!(prep["pointer64"], serde_json::json!([1000., 500.]));
    assert_eq!(prep["screen_radius64"], 500.);
    assert!((prep["anchored_pick64"][0].as_f64().unwrap() - 500.001).abs() <= 2e-13);
    assert!((prep["local_center64"][0].as_f64().unwrap() - 0.000002).abs() <= 1e-16);
    assert!((prep["local_radius_squared64"].as_f64().unwrap() - 1.).abs() <= 1e-15);
    assert_eq!(prep["raw_pressure64"], 0.5);
    assert_eq!(prep["browser_number"], true);
    assert_eq!(prep["absent"], true);
    assert_eq!(prep["pen"], false);
    assert_eq!(prep["triangle_index"], 0);
    assert_eq!(r["locked"]["mode"], "AbsolutePreview");
    s.lock().unwrap().finish_locked(&mut d).unwrap();
}
