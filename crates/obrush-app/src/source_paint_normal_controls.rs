//! Actual reset control and release barrier shared by the normal Paint panel
//! and independent egui/history contracts. This is derived cache state only.
use eframe::egui;
use obrush_core::Document;

pub(super) const RESET_KEY: &str = "source_paint.normals.reset";
pub(super) fn render(ui: &mut egui::Ui, has_state: bool) -> egui::Response {
    ui.add_enabled_ui(has_state, |ui| {
        ui.add_sized(
            [ui.available_width(), 28.],
            egui::Button::new("Reset Source normals"),
        )
    })
    .inner
}
pub(super) fn after_release(doc: &mut Document, accepted: bool) -> Result<bool, String> {
    if !accepted {
        return Ok(false);
    }
    doc.reset_source_geometry_normal_state()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use obrush_core::{MaskConvention, Mesh, SourceGeometryOptions, SourceGeometryPass};
    #[test]
    fn real_reset_response_is_disabled_when_cold_and_inside_narrow_panel_when_warm() {
        for width in [160., 280.] {
            let ctx = egui::Context::default();
            super::super::configure_theme(&ctx);
            for has_state in [false, true] {
                let mut response = None;
                let mut clip = None;
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width + 16., 100.),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            ui.set_width(width);
                            clip = Some(ui.clip_rect());
                            response = Some(render(ui, has_state));
                        });
                    },
                );
                let r = response.unwrap();
                assert_eq!(r.enabled(), has_state);
                assert!(r.interact_rect.is_positive());
                assert!(clip.unwrap().contains_rect(r.interact_rect));
                assert_eq!(RESET_KEY, "source_paint.normals.reset");
            }
        }
    }
    #[test]
    fn rejected_release_preserves_exact_cache_project_and_real_redo() {
        let mut m = Mesh::from_obj("v -1 -1 0\nv 1 -1 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m.masks.fill(1.);
        let mut d = Document::new(m);
        let mut g = d
            .begin_source_geometry_stroke(SourceGeometryOptions::inflate())
            .unwrap();
        d.apply_source_geometry_dab(
            &mut g,
            SourceGeometryPass {
                center: [0.; 3],
                eye_direction: [0., 0., -1.],
                local_radius_squared: 4.,
                original_face_index: 0,
            },
            0.5,
            true,
        )
        .unwrap();
        d.end_source_geometry_stroke(&g).unwrap();
        let state = d.source_geometry_state(1).unwrap();
        let before = d.to_json().unwrap();
        let mut second = d
            .begin_source_geometry_stroke(SourceGeometryOptions::inflate())
            .unwrap();
        d.apply_source_geometry_dab(
            &mut second,
            SourceGeometryPass {
                center: [0.; 3],
                eye_direction: [0., 0., -1.],
                local_radius_squared: 4.,
                original_face_index: 0,
            },
            0.5,
            true,
        )
        .unwrap();
        d.end_source_geometry_stroke(&second).unwrap();
        let redo = d.to_json().unwrap();
        let redo_state = d.source_geometry_state(1).unwrap();
        assert!(d.undo());
        assert!(!after_release(&mut d, false).unwrap());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(std::sync::Arc::ptr_eq(
            &state,
            &d.source_geometry_state(1).unwrap()
        ));
        assert!(after_release(&mut d, true).unwrap());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.source_geometry_state(1).is_none());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), redo);
        assert!(std::sync::Arc::ptr_eq(
            &redo_state,
            &d.source_geometry_state(1).unwrap()
        ));
    }
}
