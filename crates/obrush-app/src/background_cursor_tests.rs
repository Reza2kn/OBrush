//! Independently fixed public-formula words plus actual egui input/control frames.
//! These do not assert rendered pixels, native pressure or reference geometry.
use crate::background_cursor::*;
use eframe::egui::{self, Pos2, Rect, Vec2};

fn scope() -> CursorScope {
    CursorScope {
        viewport: [1024, 768],
        radius: 50.0,
        pixels_per_point: 1.0,
        zoom_factor: 1.0,
        empty_document: true,
        empty_scene: true,
        no_pick: true,
        idle: true,
        screen_space: true,
        symmetry: true,
    }
}
fn words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect()
}
fn pointer_frame(
    ctx: &egui::Context,
    pointer: &mut CursorPointer,
    events: Vec<egui::Event>,
    modal: bool,
    nonempty: bool,
) -> Rect {
    let mut rect = Rect::NOTHING;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0))),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (_, response) =
                    ui.allocate_exact_size(Vec2::new(280.0, 180.0), egui::Sense::click_and_drag());
                rect = response.rect;
                pointer.observe_egui(ctx, &response, nonempty, modal);
            });
        },
    );
    rect
}
fn button(position: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn independent_circle50_and_full_dot12_words_match_fixed_public_witnesses() {
    let circle = circle_vertices();
    let dot = dot_vertices();
    assert_eq!(
        circle.map(f32::to_bits).as_slice(),
        words(include_bytes!("background_cursor_witnesses/circle50.f32le"))
    );
    assert_eq!(
        dot.map(f32::to_bits).as_slice(),
        words(include_bytes!("background_cursor_witnesses/dot12.f32le"))
    );
    assert_eq!(circle.len() / 3, 50);
    assert_eq!(dot.len() / 3, 12);
    assert_eq!(&dot[..3], &[0.0, 0.0, 0.0]);
    assert_eq!(&dot[3..6], &dot[33..36]);
    assert_eq!(VERTEX_BYTES, 744);
    assert_eq!(COLOR.map(f32::to_bits), [0x3f4c_cccd, 0x3ecc_cccd, 0]);
}

#[test]
fn default_corner_matrix_uses_retained_f32_ortho_before_scalar_product() {
    let frame = CursorPointer::default().capture(scope()).unwrap();
    let m = matrices(frame).unwrap();
    let expected = [
        0x3dc8_0000,
        0,
        0,
        0,
        0,
        0x3e05_5556,
        0,
        0,
        0,
        0,
        0xc0a0_0000,
        0,
        0xbf80_0000,
        0x3f80_0000,
        0,
        0x3f80_0000,
    ];
    assert_eq!(m.circle.map(f32::to_bits), expected);
    assert_eq!(m.dot.map(f32::to_bits), expected);
    assert_eq!(m.symmetry[15], 1.0);
    assert!(m.symmetry[..15].iter().all(|v| v.to_bits() == 0));
    // Independent counterexample: direct division rounds one ULP differently.
    assert_eq!((100.0_f64 / 768.0) as f32, f32::from_bits(0x3e05_5555));
    assert_ne!(m.circle[5], (100.0_f64 / 768.0) as f32);
}

#[test]
fn fixed_interior_and_edge_translation_words_are_not_hidden_camera_inputs() {
    let mut p = CursorPointer::default();
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 768.0));
    for (position, expected) in [
        (Pos2::new(128.0, 96.0), [0xbf40_0000, 0x3f40_0000]),
        (Pos2::new(8.0, 8.0), [0xbf7c_0000, 0x3f7a_aaab]),
    ] {
        p.observe(
            position,
            rect,
            1.0,
            true,
            false,
            "independent normal input witness",
        );
        let m = matrices(p.capture(scope()).unwrap()).unwrap();
        assert_eq!([m.circle[12].to_bits(), m.circle[13].to_bits()], expected);
        assert_eq!([m.dot[12].to_bits(), m.dot[13].to_bits()], expected);
    }
}

#[test]
fn normal_profile_controls_are_clickable_without_a_mesh_or_source_tool_menu() {
    let ctx = egui::Context::default();
    let mut profile = CursorProfile::default();
    let mut rect = Rect::NOTHING;
    let mut pass = |events: Vec<egui::Event>| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let controls = render_profile(ui, &mut profile);
                    assert_eq!(controls.len(), 2);
                    assert!(controls[1].1.enabled());
                    rect = controls[1].1.interact_rect;
                });
            },
        );
        (profile, rect)
    };
    let (initial, rect) = pass(vec![]);
    assert_eq!(initial, CursorProfile::Studio);
    assert!(rect.is_positive());
    pass(vec![
        egui::Event::PointerMoved(rect.center()),
        button(rect.center(), true),
    ]);
    let (selected, _) = pass(vec![button(rect.center(), false)]);
    assert_eq!(selected, CursorProfile::SourceBackground);
}

#[test]
fn actual_egui_viewport_input_retains_position_on_release_outside_modal_and_gone() {
    let ctx = egui::Context::default();
    let mut p = CursorPointer::default();
    let rect = pointer_frame(&ctx, &mut p, vec![], false, false);
    let position = rect.min + Vec2::new(128.0, 96.0);
    pointer_frame(
        &ctx,
        &mut p,
        vec![egui::Event::PointerMoved(position)],
        false,
        false,
    );
    let frame = p.capture(scope()).unwrap();
    assert_eq!(frame.pointer_pixels, [128.0, 96.0]);
    assert_eq!(frame.provenance.unwrap().serial, 1);
    let held = frame.pointer_pixels;
    for (events, modal) in [
        (vec![button(rect.min + Vec2::new(8.0, 8.0), false)], false),
        (vec![egui::Event::PointerGone], false),
        (
            vec![egui::Event::PointerMoved(Pos2::new(310.0, 230.0))],
            false,
        ),
        (
            vec![egui::Event::PointerMoved(rect.min + Vec2::new(8.0, 8.0))],
            true,
        ),
    ] {
        pointer_frame(&ctx, &mut p, events, modal, false);
        let frame = p.capture(scope()).unwrap();
        assert_eq!(frame.pointer_pixels, held);
        assert_eq!(frame.provenance.unwrap().serial, 1);
    }
    pointer_frame(
        &ctx,
        &mut p,
        vec![button(rect.min + Vec2::new(8.0, 8.0), true)],
        false,
        false,
    );
    assert_eq!(p.capture(scope()).unwrap().pointer_pixels, [8.0, 8.0]);
}

#[test]
fn f32_transport_words_are_retained_without_claiming_recovered_javascript_precision() {
    let mut p = CursorPointer::default();
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 768.0));
    let input = Pos2::new(39.032_17, 96.0);
    p.observe(input, rect, 1.0, true, false, "egui PointerMoved f32");
    let frame = p.capture(scope()).unwrap();
    assert_eq!(frame.pointer_pixels[0], f64::from(input.x));
    assert_eq!(
        frame.provenance.unwrap().logical_words[0],
        input.x.to_bits()
    );
    assert_ne!(frame.pointer_pixels[0], 39.032168080808106_f64);
    let metrics = p.diagnostics();
    assert_eq!(metrics["last_input"]["viewport_owned"], true);
    assert_eq!(metrics["pixel_words"][0], input.x.to_bits());
}

#[test]
fn scope_refuses_surface_stale_active_world_radius_and_unverified_pixel_scales() {
    let p = CursorPointer::default();
    let cases = [
        CursorScope {
            empty_document: false,
            ..scope()
        },
        CursorScope {
            empty_scene: false,
            ..scope()
        },
        CursorScope {
            no_pick: false,
            ..scope()
        },
        CursorScope {
            idle: false,
            ..scope()
        },
        CursorScope {
            screen_space: false,
            ..scope()
        },
        CursorScope {
            pixels_per_point: 2.0,
            ..scope()
        },
        CursorScope {
            zoom_factor: 1.1,
            ..scope()
        },
        CursorScope {
            radius: f32::NAN,
            ..scope()
        },
        CursorScope {
            radius: 501.0,
            ..scope()
        },
        CursorScope {
            viewport: [0, 768],
            ..scope()
        },
        CursorScope {
            viewport: [32_769, 768],
            ..scope()
        },
    ];
    for candidate in cases {
        assert!(p.capture(candidate).is_err());
    }
    let mut stale = CursorPointer::default();
    stale.observe(
        Pos2::new(8.0, 8.0),
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 768.0)),
        1.0,
        true,
        true,
        "real input on nonempty view",
    );
    assert!(stale.capture(scope()).is_err());
    assert_eq!(stale.diagnostics()["nonempty_input_seen"], true);
}

#[test]
fn invalid_words_and_over_budget_batches_reject_without_zero_fallback() {
    let mut p = CursorPointer::default();
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 768.0));
    p.observe(
        Pos2::new(128.0, 96.0),
        rect,
        1.0,
        true,
        false,
        "finite witness",
    );
    p.observe(
        Pos2::new(f32::from_bits(0x7fc0_1234), f32::INFINITY),
        rect,
        1.0,
        true,
        false,
        "invalid witness",
    );
    assert!(p.capture(scope()).is_err());
    let metrics = p.diagnostics();
    assert_eq!(metrics["rejected_words"][0], 0x7fc0_1234_u32);
    assert_eq!(metrics["pixels"], serde_json::json!([128.0, 96.0]));
    let ctx = egui::Context::default();
    pointer_frame(
        &ctx,
        &mut p,
        vec![egui::Event::PointerMoved(Pos2::new(50.0, 50.0)); 129],
        false,
        false,
    );
    assert!(p.capture(scope()).is_err());
    assert_eq!(
        p.diagnostics()["rejected"],
        "Cursor frame pointer budget exceeded"
    );
    assert_eq!(p.diagnostics()["pixels"], serde_json::json!([128.0, 96.0]));
}

#[test]
fn resize_keeps_actual_stored_pixels_and_all_three_matrices_remain_bounded() {
    let mut p = CursorPointer::default();
    p.observe(
        Pos2::new(900.0, 700.0),
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 768.0)),
        1.0,
        true,
        false,
        "real retained pointer",
    );
    let f = p
        .capture(CursorScope {
            viewport: [512, 384],
            ..scope()
        })
        .unwrap();
    assert_eq!(f.pointer_pixels, [900.0, 700.0]);
    let m = matrices(f).unwrap();
    assert!(
        m.circle
            .iter()
            .chain(&m.dot)
            .chain(&m.symmetry)
            .all(|v| v.is_finite())
    );
    assert_eq!(m.circle[12], 2.515625);
    assert_eq!(m.circle[13].to_bits(), 0xc029_5556);
}
