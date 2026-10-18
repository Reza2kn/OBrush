use super::*;

fn input(dpr: f32, events: Vec<egui::Event>) -> egui::RawInput {
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, Vec2::splat(2000.0))),
        events,
        ..Default::default()
    };
    input
        .viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .native_pixels_per_point = Some(dpr);
    crate::ui_fonts::bound_raw_input(&mut input);
    input
}

#[test]
fn all_admitted_locales_keep_complete_brush_text_inside_the_actual_painted_tiles() {
    for dpr in [1.0, 2.0] {
        let ctx = egui::Context::default();
        let mut fonts = crate::ui_fonts::UiFonts::default();
        for locale in crate::app_preferences::Locale::ALL {
            for _ in 0..2 {
                let _ = ctx.run(input(dpr, vec![]), |ctx| fonts.update(ctx, locale));
            }
            assert!(fonts.capability().ready());
            assert_eq!(fonts.capability().effective(), locale);
            let resolver = crate::shortcut_profile::ShortcutResolver::new(
                crate::app_preferences::PreferenceProfile::OBrush,
            );
            crate::ui_text::install_admitted(fonts.capability(), &resolver);
            let mut tiles = Vec::new();
            let output = ctx.run(input(dpr, vec![]), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let height = BRUSHES
                        .iter()
                        .map(|(kind, _, _)| {
                            brush_label_galley(ui, &crate::ui_text::brush_name(*kind))
                                .size()
                                .y
                        })
                        .fold(67.0_f32, |h, label_height| {
                            h.max(38.0 + label_height + 14.0)
                        });
                    for (index, (kind, _, key)) in BRUSHES.iter().enumerate() {
                        let label = crate::ui_text::brush_name(*kind);
                        let response = brush_tile(ui, &label, key, false, index, height);
                        tiles.push((label, response.rect));
                    }
                });
            });
            assert_eq!(tiles.len(), 14);
            for (label, rect) in tiles {
                let text = output
                    .shapes
                    .iter()
                    .find_map(|shape| {
                        if let egui::Shape::Text(text) = &shape.shape
                            && text.galley.job.text == label
                        {
                            Some(text)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| panic!("No actual text shape for {locale:?}/{label}"));
                assert!(!text.galley.elided, "Full label was elided: {label}");
                let bounds = text.galley.rect.translate(text.pos.to_vec2());
                assert!(
                    rect.shrink(3.0).contains_rect(bounds),
                    "{locale:?}/{label}: {bounds:?} outside {rect:?}"
                );
                assert!(
                    bounds.top() >= rect.top() + 38.0,
                    "Label overlaps brush icon: {label}"
                );
                assert_eq!(rect.width(), 57.0);
                if locale == crate::app_preferences::Locale::English {
                    assert_eq!(rect.height(), 67.0, "English tile size changed");
                }
            }
        }
    }
}

#[test]
fn wrapped_tile_uses_its_full_visible_rectangle_for_normal_press_and_release() {
    let ctx = egui::Context::default();
    let label = "Mise à l’échelle locale";
    let mut rect = egui::Rect::NOTHING;
    let _ = ctx.run(input(1.0, vec![]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let height = (38.0 + brush_label_galley(ui, label).size().y + 14.0).max(67.0);
            rect = brush_tile(ui, label, "L", false, 12, height).rect;
        });
    });
    assert!(rect.height() > 67.0);
    let point = Pos2::new(rect.center().x, rect.bottom() - 5.0);
    let mut clicks = 0;
    for pressed in [true, false] {
        let _ = ctx.run(
            input(
                1.0,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            ),
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let height = (38.0 + brush_label_galley(ui, label).size().y + 14.0).max(67.0);
                    let response = brush_tile(ui, label, "L", false, 12, height);
                    assert_eq!(response.rect, rect);
                    clicks += usize::from(response.clicked());
                });
            },
        );
    }
    assert_eq!(clicks, 1);
}
