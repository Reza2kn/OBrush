//! Read-only clipped interaction geometry for normal tracked UI controls.
//!
//! A positive rectangle does not establish hover, layer occlusion, enabledness
//! or click acceptance. Keep the existing layout rectangle and enabled flag.
use std::collections::BTreeMap;

use eframe::egui;

const MAX_CONTROLS: usize = 512;
const MAX_CONTROL_KEY_BYTES: usize = 128;

#[derive(Default)]
pub(super) struct HitRectDiagnostics {
    entries: BTreeMap<String, Option<[f32; 4]>>,
    truncated: bool,
    focus: BTreeMap<String, serde_json::Value>,
}

impl HitRectDiagnostics {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.focus.clear();
        self.truncated = false;
    }

    pub(super) fn record(&mut self, key: &str, response: Option<&egui::Response>) {
        if key.len() > MAX_CONTROL_KEY_BYTES
            || (self.entries.len() >= MAX_CONTROLS && !self.entries.contains_key(key))
        {
            self.truncated = true;
            return;
        }
        self.entries.insert(key.to_owned(), hit_rect(response));
        self.focus.insert(
            key.to_owned(),
            response
                .map(|r| {
                    serde_json::json!({
                        "id64_hex": format!("{:016x}", r.id.value()),
                        "has_focus": r.has_focus(),
                    })
                })
                .unwrap_or(serde_json::Value::Null),
        );
    }

    pub(super) fn entries(&self) -> &BTreeMap<String, Option<[f32; 4]>> {
        &self.entries
    }

    pub(super) fn focus_entries(&self) -> &BTreeMap<String, serde_json::Value> {
        &self.focus
    }

    pub(super) fn truncated(&self) -> bool {
        self.truncated
    }
}

fn hit_rect(response: Option<&egui::Response>) -> Option<[f32; 4]> {
    let rect = response?.interact_rect;
    if !rect.is_finite() || !rect.is_positive() {
        return None;
    }
    Some([rect.min.x, rect.min.y, rect.max.x, rect.max.y])
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Pos2, Rect, Vec2};

    fn selectable_frame(
        ctx: &egui::Context,
        clip: Rect,
        row: Rect,
        selection: &mut u32,
        events: Vec<egui::Event>,
    ) -> egui::Response {
        let mut response = None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_clip_rect(clip);
                    // Ui::selectable_value uses the same Button::selectable.
                    // Fixed dimensions make this an independent clip witness.
                    let row_response = ui.put(
                        row,
                        egui::Button::selectable(*selection == 6, "Matcap 6").min_size(row.size()),
                    );
                    if row_response.clicked() {
                        *selection = 6;
                    }
                    response = Some(row_response);
                });
            },
        );
        response.expect("actual selectable row was rendered")
    }

    #[test]
    fn genuine_tab_and_space_associate_keyed_widget_with_exact_focus_owner() {
        let ctx = egui::Context::default();
        let mut values = [false, false];
        let mut diagnostics = HitRectDiagnostics::default();
        let mut frame = |events| {
            diagnostics.clear();
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 240.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        for (i, key) in ["source_geometry.smooth.tangent", "other.control"]
                            .iter()
                            .enumerate()
                        {
                            let response = ui.checkbox(&mut values[i], *key);
                            diagnostics.record(key, Some(&response));
                        }
                    });
                },
            );
            (
                serde_json::to_value(diagnostics.focus_entries()).unwrap(),
                values,
            )
        };
        let key = |key| {
            vec![
                egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let (initial, _) = frame(vec![]);
        assert_eq!(
            initial["source_geometry.smooth.tangent"]["has_focus"],
            false
        );
        frame(key(egui::Key::Tab));
        let (focused, _) = frame(vec![]);
        let owner = ctx
            .memory(|m| m.focused())
            .expect("genuine Tab selected a focusable control");
        let expected = format!("{:016x}", owner.value());
        assert_eq!(
            focused["source_geometry.smooth.tangent"]["id64_hex"],
            expected
        );
        assert_eq!(focused["source_geometry.smooth.tangent"]["has_focus"], true);
        assert_eq!(focused["other.control"]["has_focus"], false);
        assert_eq!(
            focused["source_geometry.smooth.tangent"]["id64_hex"],
            initial["source_geometry.smooth.tangent"]["id64_hex"]
        );
        let (_, changed) = frame(key(egui::Key::Space));
        assert_eq!(changed, [true, false]);
        frame(key(egui::Key::Tab));
        let (second, _) = frame(vec![]);
        assert_eq!(second["source_geometry.smooth.tangent"]["has_focus"], false);
        assert_eq!(second["other.control"]["has_focus"], true);
        diagnostics.clear();
        assert!(diagnostics.focus_entries().is_empty());
        diagnostics.record("missing", None);
        assert!(diagnostics.focus_entries()["missing"].is_null());
    }

    fn pointer_events(position: Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    #[test]
    fn enabled_fully_clipped_selectable_row_is_unhittable_and_null_diagnostic() {
        let ctx = egui::Context::default();
        let row = Rect::from_min_max(egui::pos2(40.0, 100.0), egui::pos2(220.0, 140.0));
        let clip = Rect::from_min_max(Pos2::ZERO, egui::pos2(320.0, 72.0));
        let mut selection = 5;
        let response = selectable_frame(&ctx, clip, row, &mut selection, vec![]);
        assert!(response.enabled());
        assert_eq!(response.rect, row);
        assert_eq!(
            response.interact_rect,
            Rect::from_min_max(egui::pos2(40.0, 100.0), egui::pos2(220.0, 72.0))
        );
        assert_eq!(hit_rect(Some(&response)), None);
        for pressed in [true, false] {
            let response = selectable_frame(
                &ctx,
                clip,
                row,
                &mut selection,
                pointer_events(egui::pos2(130.0, 120.0), pressed),
            );
            assert!(response.enabled());
            assert!(!response.clicked());
            assert_eq!(selection, 5);
            assert_eq!(hit_rect(Some(&response)), None);
        }

        let mut diagnostics = HitRectDiagnostics::default();
        diagnostics.record("source.matcap.6", Some(&response));
        diagnostics.record("absent", None);
        assert_eq!(
            serde_json::to_value(diagnostics.entries()).unwrap(),
            serde_json::json!({"source.matcap.6": null, "absent": null})
        );
        for rect in [
            Rect::NOTHING,
            Rect::from_min_max(Pos2::ZERO, egui::pos2(0.0, 4.0)),
            Rect::from_min_max(Pos2::ZERO, egui::pos2(4.0, 0.0)),
            Rect::from_min_max(egui::pos2(4.0, 4.0), Pos2::ZERO),
            Rect::from_min_max(egui::pos2(f32::NAN, 0.0), egui::pos2(4.0, 4.0)),
            Rect::from_min_max(Pos2::ZERO, egui::pos2(f32::INFINITY, 4.0)),
        ] {
            // Malformed diagnostic inputs are separate from the real UI witness.
            let mut malformed = response.clone();
            malformed.interact_rect = rect;
            diagnostics.record("malformed", Some(&malformed));
            assert_eq!(diagnostics.entries()["malformed"], None);
            assert_eq!(
                serde_json::to_value(diagnostics.entries()).unwrap()["malformed"],
                serde_json::Value::Null
            );
        }

        diagnostics.clear();
        for index in 0..512 {
            diagnostics.record(&format!("control.{index}"), Some(&response));
        }
        assert_eq!(diagnostics.entries().len(), 512);
        assert!(!diagnostics.truncated());
        diagnostics.record("control.0", None);
        assert_eq!(diagnostics.entries().len(), 512);
        assert!(!diagnostics.truncated());
        diagnostics.record("overflow", Some(&response));
        assert_eq!(diagnostics.entries().len(), 512);
        assert!(!diagnostics.entries().contains_key("overflow"));
        assert!(diagnostics.truncated());
        diagnostics.clear();
        assert!(diagnostics.entries().is_empty());
        assert!(!diagnostics.truncated());
        diagnostics.record(&"x".repeat(129), None);
        assert!(diagnostics.entries().is_empty());
        assert!(diagnostics.truncated());
        diagnostics.record(&"x".repeat(128), None);
        assert_eq!(diagnostics.entries().len(), 1);
    }

    #[test]
    fn partly_clipped_selectable_row_accepts_hit_center_not_layout_center() {
        let ctx = egui::Context::default();
        let row = Rect::from_min_max(egui::pos2(40.0, 100.0), egui::pos2(220.0, 180.0));
        let clip = Rect::from_min_max(Pos2::ZERO, egui::pos2(320.0, 112.0));
        let expected_hit = [40.0, 100.0, 220.0, 112.0];
        let mut selection = 5;
        let response = selectable_frame(&ctx, clip, row, &mut selection, vec![]);
        assert!(response.enabled());
        assert_eq!(response.rect, row);
        assert_eq!(hit_rect(Some(&response)), Some(expected_hit));
        assert!(!response.interact_rect.contains(egui::pos2(130.0, 140.0)));
        assert!(response.interact_rect.contains(egui::pos2(130.0, 106.0)));
        for pressed in [true, false] {
            let response = selectable_frame(
                &ctx,
                clip,
                row,
                &mut selection,
                pointer_events(egui::pos2(130.0, 140.0), pressed),
            );
            assert!(!response.clicked());
            assert_eq!(selection, 5);
            assert_eq!(response.rect, row);
            assert_eq!(hit_rect(Some(&response)), Some(expected_hit));
        }
        for pressed in [true, false] {
            let response = selectable_frame(
                &ctx,
                clip,
                row,
                &mut selection,
                pointer_events(egui::pos2(130.0, 106.0), pressed),
            );
            assert!(response.enabled());
            assert_eq!(response.rect, row);
            assert_eq!(hit_rect(Some(&response)), Some(expected_hit));
            if !pressed {
                assert!(
                    response.clicked(),
                    "normal release inside the actual hit interval"
                );
                assert_eq!(selection, 6);
            }
            let mut diagnostics = HitRectDiagnostics::default();
            diagnostics.record("source.matcap.6", Some(&response));
            assert_eq!(
                serde_json::to_value(diagnostics.entries()).unwrap(),
                serde_json::json!({"source.matcap.6": [40.0, 100.0, 220.0, 112.0]})
            );
            assert!(!diagnostics.truncated());
        }
    }
}
