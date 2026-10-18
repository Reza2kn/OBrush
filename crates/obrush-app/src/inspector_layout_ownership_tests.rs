//! Genuine Egui input ownership gates. These do not assign offsets/focus/time,
//! instantiate a fake renderer/app, or replace browser Doc/GPU/history checks.
use super::*;

#[derive(Clone, Copy)]
enum Contents {
    Cameras,
    Combos,
}

struct OwnedFrame {
    controls: Vec<PaintedControl>,
    content_width: f32,
    panel_width: f32,
    physical_viewport: egui::Rect,
    offset: egui::Vec2,
    bar: Option<egui::Response>,
    central: Option<egui::Response>,
}

fn owned_frame(
    ctx: &egui::Context,
    dpr: f32,
    events: Vec<egui::Event>,
    contents: Contents,
    selected: &mut [usize; 5],
    central: bool,
) -> (egui::FullOutput, OwnedFrame) {
    let mut controls = Vec::new();
    let mut content_width = 0.0;
    let mut panel_width = 0.0;
    let mut physical_viewport = egui::Rect::NOTHING;
    let mut offset = egui::Vec2::ZERO;
    let mut scroll_id = None;
    let mut central_response = None;
    let output = ctx.run(input(dpr, events), |ctx| {
        controls.clear();
        let panel = egui::SidePanel::right("inspector")
            .exact_width(235.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(preferences_host::panel(ctx))
                    .inner_margin(egui::Margin::symmetric(18, 18)),
            )
            .show(ctx, |ui| {
                assert_eq!(ui.available_width(), 199.0);
                assert_eq!(ui.spacing().scroll.allocated_width(), 14.0);
                assert!(!ui.spacing().scroll.floating);
                let scroll = inspector_layout::scroll(ui, |ui| {
                    content_width = inspector_layout::bounded_width(ui);
                    match contents {
                        Contents::Cameras => controls = show_actions(ui).into_iter().collect(),
                        Contents::Combos => {
                            for (index, case) in ComboCase::ALL.into_iter().enumerate() {
                                controls.push(show_combo(ui, case, &mut selected[index]).0);
                            }
                        }
                    }
                    ui.add_space(2600.0);
                });
                physical_viewport = scroll.inner_rect;
                offset = scroll.state.offset;
                scroll_id = Some(scroll.id);
            });
        panel_width = panel.response.rect.width();
        if central {
            egui::CentralPanel::default().show(ctx, |ui| {
                central_response = Some(
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag())
                        .1,
                );
            });
        }
    });
    let bar = scroll_id.and_then(|id| ctx.read_response(id.with(1)));
    (
        output,
        OwnedFrame {
            controls,
            content_width,
            panel_width,
            physical_viewport,
            offset,
            bar,
            central: central_response,
        },
    )
}

fn accepted(frame: &OwnedFrame) -> Vec<usize> {
    frame
        .controls
        .iter()
        .enumerate()
        .filter_map(|(index, control)| control.clicked.then_some(index))
        .collect()
}

fn shapes(frame: &OwnedFrame) -> Vec<(egui::Id, egui::Rect)> {
    frame
        .controls
        .iter()
        .map(|control| (control.id, control.rect))
        .collect()
}

fn keyboard(key: egui::Key, pressed: bool) -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]
}

fn setup(dpr: f32, locale: crate::app_preferences::Locale) -> egui::Context {
    let ctx = egui::Context::default();
    configure_theme(&ctx);
    admit(&ctx, dpr, locale);
    ctx
}

#[test]
fn a01_every_locale_dpr_and_cold_frame_keeps_exact_panel_and_physical_content_bounds() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup(dpr, locale);
            let mut selected = [1, 1, 0, 2, 2];
            for _ in 0..12 {
                let (output, actual) =
                    owned_frame(&ctx, dpr, vec![], Contents::Combos, &mut selected, false);
                assert_eq!(actual.panel_width, 235.0);
                assert_eq!(actual.content_width, 185.0);
                assert!(actual.physical_viewport.width() >= actual.content_width);
                assert_eq!(actual.offset, egui::Vec2::ZERO);
                for control in &actual.controls {
                    assert_horizontal_bounds(control);
                    assert!(control.rect.right() <= actual.physical_viewport.right());
                    assert_text(&output, control, true);
                }
            }
        }
    }
}

#[test]
fn a02_all_cold_camera_contacts_keep_ids_rects_and_no_prepress_backdrop_motion() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup(dpr, locale);
            let mut selected = [0; 5];
            let _ = owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, false);
            let (_, baseline) =
                owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, false);
            let original = shapes(&baseline);
            if locale == crate::app_preferences::Locale::English {
                assert_eq!(original[3].1.center().y - original[0].1.center().y, 33.0);
            }
            // The repeated Top->Center transition includes the original -33px
            // movement BEFORE Down, without an artificial hover/setup frame.
            for index in [0, 1, 2, 3, 0, 1, 2, 3] {
                let point = original[index].1.center();
                let mut clicks = Vec::new();
                for pressed in [true, false] {
                    let (_, actual) = owned_frame(
                        &ctx,
                        dpr,
                        pointer(point, pressed),
                        Contents::Cameras,
                        &mut selected,
                        false,
                    );
                    assert_eq!(actual.panel_width, 235.0);
                    assert_eq!(shapes(&actual), original);
                    assert_eq!(actual.offset, egui::Vec2::ZERO);
                    clicks.extend(accepted(&actual));
                }
                assert_eq!(clicks, [index], "{locale:?}/{dpr}/{index}");
            }
        }
    }
}

#[test]
fn a03_real_camera_button_drag_does_not_click_or_drag_backdrop() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup(dpr, locale);
            let mut selected = [0; 5];
            let _ = owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, false);
            let (_, before) =
                owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, false);
            let id = before.controls[0].id;
            let point = before.controls[0].rect.center();
            let (_, down) = owned_frame(
                &ctx,
                dpr,
                pointer(point, true),
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert!(accepted(&down).is_empty());
            assert_eq!(down.offset, egui::Vec2::ZERO);
            let moved = point + egui::vec2(0.0, 100.0);
            let (_, held) = owned_frame(
                &ctx,
                dpr,
                vec![egui::Event::PointerMoved(moved)],
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert!(ctx.read_response(id).unwrap().dragged());
            assert!(accepted(&held).is_empty());
            assert_eq!(held.offset, egui::Vec2::ZERO);
            let (_, up) = owned_frame(
                &ctx,
                dpr,
                pointer(moved, false),
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert!(accepted(&up).is_empty());
            assert_eq!(up.offset, egui::Vec2::ZERO);
            assert!(!ctx.read_response(id).unwrap().is_pointer_button_down_on());
        }
    }
}

#[test]
fn a04_genuine_empty_background_drag_keeps_original_scroll_owner() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup(dpr, locale);
            let mut selected = [0; 5];
            let (_, before) =
                owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, false);
            let point = egui::pos2(before.physical_viewport.center().x, 300.0);
            let _ = owned_frame(
                &ctx,
                dpr,
                vec![egui::Event::PointerMoved(point)],
                Contents::Cameras,
                &mut selected,
                false,
            );
            let (_, down) = owned_frame(
                &ctx,
                dpr,
                pointer(point, true),
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert_eq!(down.offset, egui::Vec2::ZERO);
            let moved = point - egui::vec2(0.0, 60.0);
            let (_, held) = owned_frame(
                &ctx,
                dpr,
                vec![egui::Event::PointerMoved(moved)],
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert!(
                held.offset.y > down.offset.y,
                "actual background drag did not scroll"
            );
            assert!(accepted(&held).is_empty());
            let (_, up) = owned_frame(
                &ctx,
                dpr,
                pointer(moved, false),
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert!(up.offset.y > 0.0);
            assert!(accepted(&up).is_empty());
        }
    }
}

#[test]
fn a05_real_wheel_and_scrollbar_keep_scrolling_without_camera_clicks() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup(dpr, locale);
            let mut selected = [0; 5];
            let mut actual =
                owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, false).1;
            // This separate scrollbar gate observes natural animation; the
            // strict cold A01/A02/L01-L03 do not wait for this completion.
            for _ in 0..12 {
                actual = owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, false).1;
            }
            let point = egui::pos2(actual.physical_viewport.center().x, 300.0);
            let (_, wheel) = owned_frame(
                &ctx,
                dpr,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -60.0),
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert!(wheel.offset.y > actual.offset.y);
            assert!(accepted(&wheel).is_empty());
            let bar = wheel.bar.expect("actual visible scroll bar response");
            assert!(bar.sense.senses_click() && bar.sense.senses_drag());
            let point = bar.rect.center();
            let (_, down) = owned_frame(
                &ctx,
                dpr,
                pointer(point, true),
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert!(down.bar.as_ref().unwrap().is_pointer_button_down_on());
            let moved = point + egui::vec2(0.0, 80.0);
            let (_, held) = owned_frame(
                &ctx,
                dpr,
                vec![egui::Event::PointerMoved(moved)],
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert!(held.bar.as_ref().unwrap().dragged());
            assert!(held.offset.y > down.offset.y);
            assert!(accepted(&held).is_empty());
            let (_, up) = owned_frame(
                &ctx,
                dpr,
                pointer(moved, false),
                Contents::Cameras,
                &mut selected,
                false,
            );
            assert!(accepted(&up).is_empty());
        }
    }
}

#[test]
fn a06_normal_tab_enter_and_space_keep_camera_keyboard_focus() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup(dpr, locale);
            let mut selected = [0; 5];
            let _ = owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, false);
            let (_, initial) =
                owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, false);
            let mut chosen = None;
            for _ in 0..16 {
                let _ = owned_frame(
                    &ctx,
                    dpr,
                    keyboard(egui::Key::Tab, true),
                    Contents::Cameras,
                    &mut selected,
                    false,
                );
                let _ = owned_frame(
                    &ctx,
                    dpr,
                    keyboard(egui::Key::Tab, false),
                    Contents::Cameras,
                    &mut selected,
                    false,
                );
                let focused = ctx.memory(|memory| memory.focused());
                chosen = initial
                    .controls
                    .iter()
                    .position(|control| Some(control.id) == focused);
                if chosen.is_some() {
                    break;
                }
            }
            let index = chosen.expect("normal Tab must reach a camera control");
            let id = initial.controls[index].id;
            assert!(ctx.read_response(id).unwrap().sense.is_focusable());
            for key in [egui::Key::Enter, egui::Key::Space] {
                let mut clicks = Vec::new();
                for pressed in [true, false] {
                    let (_, actual) = owned_frame(
                        &ctx,
                        dpr,
                        keyboard(key, pressed),
                        Contents::Cameras,
                        &mut selected,
                        false,
                    );
                    clicks.extend(accepted(&actual));
                    assert_eq!(ctx.memory(|memory| memory.focused()), Some(id));
                }
                assert_eq!(clicks, [index]);
            }
        }
    }
}

#[test]
fn a07_inspector_contact_cannot_become_viewport_press_and_fresh_viewport_drag_survives() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup(dpr, locale);
            let mut selected = [0; 5];
            let _ = owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, true);
            let (_, initial) =
                owned_frame(&ctx, dpr, vec![], Contents::Cameras, &mut selected, true);
            for (index, control) in initial.controls.iter().enumerate() {
                let mut clicks = Vec::new();
                for pressed in [true, false] {
                    let (_, actual) = owned_frame(
                        &ctx,
                        dpr,
                        pointer(control.rect.center(), pressed),
                        Contents::Cameras,
                        &mut selected,
                        true,
                    );
                    clicks.extend(accepted(&actual));
                    let central = actual.central.unwrap();
                    assert!(!central.contains_pointer());
                    assert!(!central.is_pointer_button_down_on());
                    assert!(!central.dragged() && !central.clicked());
                }
                assert_eq!(clicks, [index]);
            }
            let point = initial.central.unwrap().rect.center();
            let (_, down) = owned_frame(
                &ctx,
                dpr,
                pointer(point, true),
                Contents::Cameras,
                &mut selected,
                true,
            );
            assert!(down.central.as_ref().unwrap().is_pointer_button_down_on());
            assert!(accepted(&down).is_empty());
            let moved = point + egui::vec2(20.0, 20.0);
            let (_, held) = owned_frame(
                &ctx,
                dpr,
                vec![egui::Event::PointerMoved(moved)],
                Contents::Cameras,
                &mut selected,
                true,
            );
            assert!(held.central.as_ref().unwrap().dragged());
            assert!(accepted(&held).is_empty());
            let (_, up) = owned_frame(
                &ctx,
                dpr,
                pointer(moved, false),
                Contents::Cameras,
                &mut selected,
                true,
            );
            assert!(accepted(&up).is_empty());
            assert!(!up.central.unwrap().is_pointer_button_down_on());
        }
    }
}

#[test]
fn a08_every_row_of_five_popups_accepts_its_real_named_target() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup(dpr, locale);
            for case in ComboCase::ALL {
                let mut selected = 0;
                for target_index in 0..case.rows().len() {
                    let _ = frame(&ctx, dpr, vec![], |ui| show_combo(ui, case, &mut selected));
                    let (_, (closed, _)) =
                        frame(&ctx, dpr, vec![], |ui| show_combo(ui, case, &mut selected));
                    let id = closed.id;
                    for pressed in [true, false] {
                        let _ = frame(&ctx, dpr, pointer(closed.rect.center(), pressed), |ui| {
                            show_combo(ui, case, &mut selected)
                        });
                    }
                    assert!(egui::Popup::is_any_open(&ctx));
                    let (output, (_, rows)) =
                        frame(&ctx, dpr, vec![], |ui| show_combo(ui, case, &mut selected));
                    assert_eq!(rows.len(), case.rows().len());
                    let target = &rows[target_index];
                    assert_eq!(target.0, target_index);
                    assert_eq!(target.1, case.rows()[target_index].1);
                    assert_text(&output, &target.2, false);
                    let target_id = target.2.id;
                    let point = target.2.rect.center();
                    let mut clicks = Vec::new();
                    for pressed in [true, false] {
                        let (_, (_, rows)) = frame(&ctx, dpr, pointer(point, pressed), |ui| {
                            show_combo(ui, case, &mut selected)
                        });
                        clicks.extend(rows.into_iter().filter_map(|(index, key, row)| {
                            row.clicked.then_some((index, key, row.id))
                        }));
                    }
                    assert_eq!(
                        clicks,
                        [(target_index, case.rows()[target_index].1, target_id)]
                    );
                    assert_eq!(selected, target_index);
                    let (_, (closed, rows)) =
                        frame(&ctx, dpr, vec![], |ui| show_combo(ui, case, &mut selected));
                    assert_eq!(closed.id, id);
                    assert!(rows.is_empty());
                    assert!(!egui::Popup::is_any_open(&ctx));
                }
            }
        }
    }
}
