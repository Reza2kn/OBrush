//! Normal egui widget/input checks for the five affected inspector controls.
//! These use real admitted fonts and captions; they do not construct a fake
//! renderer/app, set production state, or substitute a document/GPU oracle.
use super::*;

#[derive(Clone, Copy, Debug)]
enum ComboCase {
    Framing,
    Mode,
    ByteOrder,
    Export,
    UvSize,
}

impl ComboCase {
    const ALL: [Self; 5] = [
        Self::Framing,
        Self::Mode,
        Self::ByteOrder,
        Self::Export,
        Self::UvSize,
    ];

    fn salt(self) -> &'static str {
        match self {
            Self::Framing => "camera_framing",
            Self::Mode => "camera_mode",
            Self::ByteOrder => "source_sgl_order",
            Self::Export => "mesh_export_format",
            Self::UvSize => "uv_bake_size",
        }
    }

    fn rows(self) -> Vec<(String, &'static str)> {
        match self {
            Self::Framing => vec![
                ("OBrush".into(), "camera.framing.legacy"),
                ("SculptGL static".into(), "camera.framing.source"),
            ],
            Self::Mode => [
                (CameraMode::Orbit, "camera.mode.orbit"),
                (CameraMode::Spherical, "camera.mode.spherical"),
                (CameraMode::Plane, "camera.mode.plane"),
            ]
            .map(|(mode, key)| (crate::ui_text::camera_mode_name(mode), key))
            .to_vec(),
            Self::ByteOrder => [
                (
                    crate::source_sgl_import::ByteOrderPolicy::ReferenceNative,
                    "file.source_sgl.order.native",
                ),
                (
                    crate::source_sgl_import::ByteOrderPolicy::LittleEndian,
                    "file.source_sgl.order.little",
                ),
                (
                    crate::source_sgl_import::ByteOrderPolicy::BigEndian,
                    "file.source_sgl.order.big",
                ),
            ]
            .map(|(policy, key)| (policy.name().into(), key))
            .to_vec(),
            Self::Export => [
                (ExportFormat::Obj, "file.export_format.obj"),
                (ExportFormat::StlAscii, "file.export_format.stl_ascii"),
                (ExportFormat::StlBinary, "file.export_format.stl_binary"),
                (ExportFormat::PlyAscii, "file.export_format.ply_ascii"),
                (ExportFormat::PlyBinary, "file.export_format.ply_binary"),
            ]
            .map(|(format, key)| (format.name().into(), key))
            .to_vec(),
            Self::UvSize => [
                (256, "file.uv_size.256"),
                (512, "file.uv_size.512"),
                (1024, "file.uv_size.1024"),
                (2048, "file.uv_size.2048"),
                (4096, "file.uv_size.4096"),
            ]
            .map(|(pixels, key)| (pixels.to_string(), key))
            .to_vec(),
        }
    }

    fn selected(self, index: usize) -> String {
        match self {
            Self::Framing if index == 1 => {
                crate::ui_text::caption(crate::ui_text::CaptionKey::C499EEDC4E01E)
            }
            Self::UvSize => format!(
                "{}: {}",
                crate::ui_text::caption(crate::ui_text::CaptionKey::CFAA8A951AE15),
                self.rows()[index].0,
            ),
            _ => self.rows()[index].0.clone(),
        }
    }
}

#[derive(Debug)]
struct PaintedControl {
    full: String,
    rect: egui::Rect,
    clip: egui::Rect,
    id: egui::Id,
    clicked: bool,
}

fn witness(ui: &egui::Ui, full: String, response: &egui::Response) -> PaintedControl {
    PaintedControl {
        full,
        rect: response.rect,
        clip: ui.clip_rect(),
        id: response.id,
        clicked: response.clicked(),
    }
}

fn input(dpr: f32, events: Vec<egui::Event>) -> egui::RawInput {
    let mut raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            Vec2::new(1600.0, 2000.0),
        )),
        events,
        ..Default::default()
    };
    raw.viewports
        .get_mut(&egui::ViewportId::ROOT)
        .unwrap()
        .native_pixels_per_point = Some(dpr);
    crate::ui_fonts::bound_raw_input(&mut raw);
    raw
}

fn admit(ctx: &egui::Context, dpr: f32, locale: crate::app_preferences::Locale) {
    let mut fonts = crate::ui_fonts::UiFonts::default();
    for _ in 0..2 {
        let _ = ctx.run(input(dpr, vec![]), |ctx| fonts.update(ctx, locale));
    }
    assert!(
        fonts.capability().ready(),
        "Font admission failed for {locale:?}/{dpr}"
    );
    assert_eq!(fonts.capability().effective(), locale);
    let resolver = crate::shortcut_profile::ShortcutResolver::new(
        crate::app_preferences::PreferenceProfile::OBrush,
    );
    crate::ui_text::install_admitted(fonts.capability(), &resolver);
}

fn frame<R>(
    ctx: &egui::Context,
    dpr: f32,
    events: Vec<egui::Event>,
    mut add: impl FnMut(&mut egui::Ui) -> R,
) -> (egui::FullOutput, R) {
    let mut result = None;
    let output = ctx.run(input(dpr, events), |ctx| {
        let panel = egui::SidePanel::right("inspector")
            .exact_width(235.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(preferences_host::panel(ctx))
                    .inner_margin(egui::Margin::symmetric(18, 18)),
            )
            .show(ctx, |ui| {
                inspector_layout::scroll(ui, |ui| {
                    inspector_layout::constrain_content(ui);
                    assert!(inspector_layout::bounded_width(ui) > 0.0);
                    assert!(inspector_layout::bounded_width(ui) <= 199.0);
                    result = Some(add(ui));
                    // Exercise the actual nonfloating scroll gutter on
                    // later passes, without overriding scroll policy.
                    ui.add_space(2600.0);
                });
            });
        assert_eq!(panel.response.rect.width(), 235.0, "Inspector was widened");
    });
    (output, result.unwrap())
}

fn show_combo(
    ui: &mut egui::Ui,
    case: ComboCase,
    selected: &mut usize,
) -> (PaintedControl, Vec<(usize, &'static str, PaintedControl)>) {
    let full = case.selected(*selected);
    let mut rows = Vec::new();
    let response = inspector_layout::combo(ui, case.salt(), &full, |ui| {
        for (index, (label, key)) in case.rows().into_iter().enumerate() {
            let row = ui.selectable_value(selected, index, &label);
            rows.push((index, key, witness(ui, label, &row)));
        }
    })
    .response;
    (witness(ui, full, &response), rows)
}

fn assert_horizontal_bounds(control: &PaintedControl) {
    assert!(
        control.rect.left() >= control.clip.left(),
        "Left cutoff: {control:?}"
    );
    assert!(
        control.rect.right() <= control.clip.right(),
        "Right cutoff: {control:?}"
    );
}

fn assert_text(output: &egui::FullOutput, control: &PaintedControl, allow_elision: bool) {
    assert_horizontal_bounds(control);
    let text = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape
                && text.galley.job.text == control.full
                && control.rect.contains(text.pos)
            {
                Some(text)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("No actual full-text galley in control: {control:?}"));
    assert_eq!(
        text.galley.job.text, control.full,
        "Stored original text changed"
    );
    if !allow_elision {
        assert!(!text.galley.elided, "Full text was elided: {control:?}");
    }
    let bounds = text.galley.rect.translate(text.pos.to_vec2());
    assert!(
        bounds.left() >= control.clip.left(),
        "Painted left cutoff: {control:?}/{bounds:?}"
    );
    assert!(
        bounds.right() <= control.clip.right(),
        "Painted right cutoff: {control:?}/{bounds:?}"
    );
    assert!(
        control.clip.contains_rect(bounds),
        "Actual text is not completely inside its clip: {control:?}/{bounds:?}"
    );
}

fn pointer(point: Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(point),
        egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn action_labels() -> [String; 4] {
    [
        crate::ui_text::CaptionKey::C565099F6DF99,
        crate::ui_text::CaptionKey::C7688CD7B2014,
        crate::ui_text::CaptionKey::CCB106B20524D,
        crate::ui_text::CaptionKey::C360B1EA7EA20,
    ]
    .map(crate::ui_text::caption)
}

fn show_actions(ui: &mut egui::Ui) -> [PaintedControl; 4] {
    let labels = action_labels();
    let actions = inspector_layout::camera_actions(ui);
    std::array::from_fn(|index| witness(ui, labels[index].clone(), &actions[index]))
}

fn paragraphs() -> Vec<String> {
    let mut paragraphs = [
        crate::ui_text::CaptionKey::CF5D41785D514,
        crate::ui_text::CaptionKey::C1F51DB0D93A3,
        crate::ui_text::CaptionKey::C0C60415D550D,
        crate::ui_text::CaptionKey::CDB546E0641A5,
        crate::ui_text::CaptionKey::C1F3E8A33E7AB,
    ]
    .map(crate::ui_text::caption)
    .to_vec();
    // Independent report/warning contents use the exact production format,
    // including the unchanged numbers, names and property words.
    paragraphs.extend([
        "Last mesh export omitted 123 material rows, 456 source mask values, 789 original faces and 1,234 UV coordinates. Project data was kept.".into(),
        "Last import: 12,345 points / 23,456 triangles".into(),
        "Skipped fields: independent_property_name, independent_extended_property_name".into(),
        "Independent import warning: this deliberately long source property name remains visible without rewriting or eliding the warning.".into(),
    ]);
    paragraphs
}

fn show_prose(ui: &mut egui::Ui) -> Vec<PaintedControl> {
    paragraphs()
        .into_iter()
        .map(|full| {
            let response = inspector_layout::prose(
                ui,
                RichText::new(&full)
                    .size(10.0)
                    .color(preferences_host::muted(ui)),
            );
            witness(ui, full, &response)
        })
        .collect()
}

#[test]
fn l01_all_ten_admitted_locales_at_dpr_one_and_two_keep_five_closed_combos_inside_the_fixed_inspector()
 {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = egui::Context::default();
            configure_theme(&ctx);
            admit(&ctx, dpr, locale);
            // Independent selected values intentionally exercise the long
            // spherical/source/native/colored-STL/Size:1024 closed labels.
            let mut selected = [1, 1, 0, 2, 2];
            for _ in 0..2 {
                let _ = frame(&ctx, dpr, vec![], |ui| {
                    for (index, case) in ComboCase::ALL.into_iter().enumerate() {
                        let _ = show_combo(ui, case, &mut selected[index]);
                    }
                });
            }
            let (output, controls) = frame(&ctx, dpr, vec![], |ui| {
                ComboCase::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(index, case)| {
                        let (control, rows) = show_combo(ui, case, &mut selected[index]);
                        assert!(
                            rows.is_empty(),
                            "Popup unexpectedly open: {locale:?}/{case:?}"
                        );
                        control
                    })
                    .collect::<Vec<_>>()
            });
            assert_eq!(selected, [1, 1, 0, 2, 2]);
            assert_eq!(controls.len(), 5);
            for control in controls {
                assert_text(&output, &control, true);
            }
            assert!(!egui::Popup::is_any_open(&ctx));
        }
    }
}

#[test]
fn l02_every_locale_and_dpr_normally_opens_all_five_wrapped_popups_and_selects_exactly_the_requested_row()
 {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = egui::Context::default();
            configure_theme(&ctx);
            admit(&ctx, dpr, locale);
            for case in ComboCase::ALL {
                let mut selected = 0;
                let _ = frame(&ctx, dpr, vec![], |ui| show_combo(ui, case, &mut selected));
                let (_, (closed, _)) =
                    frame(&ctx, dpr, vec![], |ui| show_combo(ui, case, &mut selected));
                let original_id = closed.id;
                for pressed in [true, false] {
                    let _ = frame(&ctx, dpr, pointer(closed.rect.center(), pressed), |ui| {
                        show_combo(ui, case, &mut selected)
                    });
                }
                assert!(
                    egui::Popup::is_any_open(&ctx),
                    "Normal open failed: {locale:?}/{dpr}/{case:?}"
                );
                let (output, (open, rows)) =
                    frame(&ctx, dpr, vec![], |ui| show_combo(ui, case, &mut selected));
                assert_eq!(open.id, original_id, "Named combo ID changed");
                assert_eq!(rows.len(), case.rows().len());
                let expected_rows = case.rows();
                for (index, key, control) in &rows {
                    assert_eq!(control.full, expected_rows[*index].0);
                    assert_eq!(*key, expected_rows[*index].1);
                    assert_text(&output, control, false);
                }
                let target = rows.last().unwrap();
                let expected_index = target.0;
                let expected_id = target.2.id;
                let point = target.2.rect.center();
                let mut clicked_rows = Vec::new();
                for pressed in [true, false] {
                    let (_, (_, rows)) = frame(&ctx, dpr, pointer(point, pressed), |ui| {
                        show_combo(ui, case, &mut selected)
                    });
                    clicked_rows.extend(rows.into_iter().filter_map(|(index, _, control)| {
                        control.clicked.then_some((index, control.id))
                    }));
                }
                assert_eq!(
                    clicked_rows,
                    [(expected_index, expected_id)],
                    "Wrong accepted row: {locale:?}/{dpr}/{case:?}"
                );
                assert_eq!(selected, expected_index);
                let (output, (closed, rows)) =
                    frame(&ctx, dpr, vec![], |ui| show_combo(ui, case, &mut selected));
                assert!(rows.is_empty());
                assert!(!egui::Popup::is_any_open(&ctx));
                assert_eq!(closed.id, original_id);
                assert_eq!(closed.full, case.selected(expected_index));
                assert_text(&output, &closed, true);
            }
        }
    }
}

#[test]
fn l03_every_locale_and_dpr_keeps_all_four_full_camera_actions_reachable_by_real_primary_contact() {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = egui::Context::default();
            configure_theme(&ctx);
            admit(&ctx, dpr, locale);
            let _ = frame(&ctx, dpr, vec![], show_actions);
            let (output, controls) = frame(&ctx, dpr, vec![], show_actions);
            for control in &controls {
                assert_text(&output, control, false);
            }
            for (index, control) in controls.iter().enumerate() {
                let mut accepted = Vec::new();
                for pressed in [true, false] {
                    let (_, current) = frame(
                        &ctx,
                        dpr,
                        pointer(control.rect.center(), pressed),
                        show_actions,
                    );
                    for (actual_index, actual) in current.iter().enumerate() {
                        assert_eq!(actual.id, controls[actual_index].id);
                        assert_eq!(actual.rect, controls[actual_index].rect);
                        if actual.clicked {
                            accepted.push(actual_index);
                        }
                    }
                }
                assert_eq!(
                    accepted,
                    [index],
                    "Wrong camera action: {locale:?}/{dpr}/{index}"
                );
            }
        }
    }
}

#[test]
fn l04_every_locale_and_dpr_paints_the_complete_navigation_percentage_and_unmodified_prose_without_horizontal_cutoffs()
 {
    for dpr in [1.0, 2.0] {
        for locale in crate::app_preferences::Locale::ALL {
            let ctx = egui::Context::default();
            configure_theme(&ctx);
            admit(&ctx, dpr, locale);
            let show = |ui: &mut egui::Ui| {
                let label = crate::ui_text::caption(crate::ui_text::CaptionKey::CB480949ED50E);
                let navigation = inspector_layout::navigation_value(ui, &label, "100%").inner;
                let mut controls = vec![
                    witness(ui, label, &navigation.0),
                    witness(ui, "100%".into(), &navigation.1),
                ];
                controls.extend(show_prose(ui));
                controls
            };
            let _ = frame(&ctx, dpr, vec![], show);
            let (output, controls) = frame(&ctx, dpr, vec![], show);
            assert_eq!(controls[1].full, "100%");
            assert_eq!(
                controls
                    .iter()
                    .skip(2)
                    .map(|c| c.full.clone())
                    .collect::<Vec<_>>(),
                paragraphs()
            );
            for control in controls {
                assert_text(&output, &control, false);
            }
        }
    }
}

// Additive ownership gates; the four existing test bodies remain unchanged.
#[path = "inspector_layout_ownership_tests.rs"]
mod ownership;

// Existing 12 tests above/in ownership remain byte-exact.
#[path = "inspector_combo_ownership_tests.rs"]
mod combo_ownership;
