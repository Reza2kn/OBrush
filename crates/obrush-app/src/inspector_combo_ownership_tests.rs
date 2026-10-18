//! Actual stock-combo response augmentation and normal-event ownership guards.
use super::*;

struct ComboFrame {
    response: egui::Response,
    expected_id: egui::Id,
    layer: egui::LayerId,
    clip: egui::Rect,
    offset: egui::Vec2,
    rows: Vec<(usize, &'static str, egui::Response)>,
    callbacks: usize,
}

fn actual_combo_frame(
    ctx: &egui::Context,
    dpr: f32,
    events: Vec<egui::Event>,
    case: ComboCase,
    selected: &mut usize,
) -> ComboFrame {
    let mut result = None;
    let _ = ctx.run(input(dpr, events), |ctx| {
        let mut fields = None;
        let panel = egui::SidePanel::right("inspector")
            .exact_width(235.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(preferences_host::panel(ctx))
                    .inner_margin(egui::Margin::symmetric(18, 18)),
            )
            .show(ctx, |ui| {
                let scroll = inspector_layout::scroll(ui, |ui| {
                    let expected_id = ui.make_persistent_id(egui::Id::new(case.salt()));
                    let layer = ui.layer_id();
                    let clip = ui.clip_rect();
                    let full = case.selected(*selected);
                    let mut rows = Vec::new();
                    let mut callbacks = 0;
                    let response = inspector_layout::combo(ui, case.salt(), &full, |ui| {
                        callbacks += 1;
                        for (index, (label, key)) in case.rows().into_iter().enumerate() {
                            rows.push((index, key, ui.selectable_value(selected, index, label)));
                        }
                    })
                    .response;
                    fields = Some((response, expected_id, layer, clip, rows, callbacks));
                    ui.add_space(2600.0);
                });
                let (response, expected_id, layer, clip, rows, callbacks) = fields.take().unwrap();
                result = Some(ComboFrame {
                    response,
                    expected_id,
                    layer,
                    clip,
                    offset: scroll.state.offset,
                    rows,
                    callbacks,
                });
            });
        assert_eq!(panel.response.rect.width(), 235.0);
    });
    result.unwrap()
}

fn setup_combo(dpr: f32, locale: crate::app_preferences::Locale) -> egui::Context {
    let ctx = egui::Context::default();
    configure_theme(&ctx);
    admit(&ctx, dpr, locale);
    ctx
}

fn key(key: egui::Key, pressed: bool) -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]
}

#[test]
fn b01_real_combo_preserves_identity_plane_clip_and_one_callback_per_actual_pass() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup_combo(dpr, locale);
            for case in ComboCase::ALL {
                let mut selected = 0;
                let _ = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
                let before = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
                assert_eq!(before.response.id, before.expected_id);
                assert_eq!(before.response.layer_id, before.layer);
                assert_eq!(
                    before.response.interact_rect,
                    before.response.rect.intersect(before.clip)
                );
                assert!(before.response.sense.senses_click());
                assert!(before.response.sense.senses_drag());
                assert!(before.response.sense.is_focusable());
                assert_eq!(before.callbacks, 0);
                assert!(!before.response.changed());
                let rect = before.response.rect;
                let id = before.response.id;
                for pressed in [true, false] {
                    let actual = actual_combo_frame(
                        &ctx,
                        dpr,
                        pointer(rect.center(), pressed),
                        case,
                        &mut selected,
                    );
                    assert_eq!(actual.response.id, id);
                    assert_eq!(actual.response.rect, rect);
                    assert_eq!(actual.response.layer_id, actual.layer);
                    assert_eq!(
                        actual.response.interact_rect,
                        actual.response.rect.intersect(actual.clip)
                    );
                    assert!(actual.callbacks <= 1);
                }
                assert!(egui::Popup::is_any_open(&ctx));
                let open = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
                assert_eq!(open.callbacks, 1);
                assert_eq!(open.rows.len(), case.rows().len());
                for (index, key, row) in &open.rows {
                    assert_eq!(*key, case.rows()[*index].1);
                    assert_eq!(row.layer_id.order, egui::Order::Foreground);
                }
                for pressed in [true, false] {
                    let _ = actual_combo_frame(
                        &ctx,
                        dpr,
                        key(egui::Key::Escape, pressed),
                        case,
                        &mut selected,
                    );
                }
                assert!(!egui::Popup::is_any_open(&ctx));
            }
        }
    }
}

#[test]
fn b02_popup_row_to_closed_combo_press_cannot_scroll_and_real_drag_cannot_toggle() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup_combo(dpr, locale);
            let case = ComboCase::Mode;
            let mut selected = 0;
            let _ = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
            let first = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
            let original_id = first.response.id;
            let original_rect = first.response.rect;
            for pressed in [true, false] {
                let _ = actual_combo_frame(
                    &ctx,
                    dpr,
                    pointer(original_rect.center(), pressed),
                    case,
                    &mut selected,
                );
            }
            let open = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
            let point = open.rows.last().unwrap().2.rect.center();
            let target_id = open.rows.last().unwrap().2.id;
            let mut accepted = Vec::new();
            for pressed in [true, false] {
                let actual =
                    actual_combo_frame(&ctx, dpr, pointer(point, pressed), case, &mut selected);
                accepted.extend(
                    actual
                        .rows
                        .iter()
                        .filter_map(|(index, _, row)| row.clicked().then_some((*index, row.id))),
                );
                assert!(actual.callbacks <= 1);
            }
            assert_eq!(accepted, [(2, target_id)]);
            assert_eq!(selected, 2);
            assert!(!egui::Popup::is_any_open(&ctx));
            let closed = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
            assert_eq!(closed.response.id, original_id);
            assert_eq!(closed.response.rect, original_rect);
            assert_eq!(closed.offset, egui::Vec2::ZERO);
            assert!(point.y > original_rect.center().y);
            // No preparatory hover: preserve genuine popup-row->combo delta.
            for pressed in [true, false] {
                let actual = actual_combo_frame(
                    &ctx,
                    dpr,
                    pointer(original_rect.center(), pressed),
                    case,
                    &mut selected,
                );
                assert_eq!(actual.offset, egui::Vec2::ZERO);
                assert_eq!(actual.response.rect, original_rect);
                assert!(actual.callbacks <= 1);
            }
            assert!(egui::Popup::is_any_open(&ctx));
            for pressed in [true, false] {
                let _ = actual_combo_frame(
                    &ctx,
                    dpr,
                    key(egui::Key::Escape, pressed),
                    case,
                    &mut selected,
                );
            }
            assert!(!egui::Popup::is_any_open(&ctx));
            let _ = actual_combo_frame(
                &ctx,
                dpr,
                pointer(original_rect.center(), true),
                case,
                &mut selected,
            );
            let moved = original_rect.center() + egui::vec2(0.0, 100.0);
            let held = actual_combo_frame(
                &ctx,
                dpr,
                vec![egui::Event::PointerMoved(moved)],
                case,
                &mut selected,
            );
            assert!(held.response.dragged());
            assert_eq!(held.offset, egui::Vec2::ZERO);
            assert!(!held.response.clicked());
            let up = actual_combo_frame(&ctx, dpr, pointer(moved, false), case, &mut selected);
            assert!(!up.response.clicked());
            assert_eq!(up.offset, egui::Vec2::ZERO);
            assert_eq!(selected, 2);
            assert!(!egui::Popup::is_any_open(&ctx));
        }
    }
}

#[test]
fn b03_normal_tab_and_enter_open_each_combo_and_normal_row_contact_selects_once() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = setup_combo(dpr, locale);
            for case in ComboCase::ALL {
                let mut selected = 0;
                let _ = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
                let initial = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
                let id = initial.response.id;
                for _ in 0..16 {
                    if ctx.memory(|memory| memory.focused()) == Some(id) {
                        break;
                    }
                    let _ = actual_combo_frame(
                        &ctx,
                        dpr,
                        key(egui::Key::Tab, true),
                        case,
                        &mut selected,
                    );
                    let _ = actual_combo_frame(
                        &ctx,
                        dpr,
                        key(egui::Key::Tab, false),
                        case,
                        &mut selected,
                    );
                }
                assert_eq!(ctx.memory(|memory| memory.focused()), Some(id));
                for pressed in [true, false] {
                    let actual = actual_combo_frame(
                        &ctx,
                        dpr,
                        key(egui::Key::Enter, pressed),
                        case,
                        &mut selected,
                    );
                    assert!(actual.callbacks <= 1);
                }
                assert!(egui::Popup::is_any_open(&ctx));
                let open = actual_combo_frame(&ctx, dpr, vec![], case, &mut selected);
                assert_eq!(open.callbacks, 1);
                let index = case.rows().len() - 1;
                let target = &open.rows[index];
                let point = target.2.rect.center();
                let target_id = target.2.id;
                let mut accepted = Vec::new();
                for pressed in [true, false] {
                    let actual =
                        actual_combo_frame(&ctx, dpr, pointer(point, pressed), case, &mut selected);
                    accepted.extend(
                        actual.rows.iter().filter_map(|(index, _, row)| {
                            row.clicked().then_some((*index, row.id))
                        }),
                    );
                    assert!(actual.callbacks <= 1);
                }
                assert_eq!(accepted, [(index, target_id)]);
                assert_eq!(selected, index);
                assert!(!egui::Popup::is_any_open(&ctx));
            }
        }
    }
}
