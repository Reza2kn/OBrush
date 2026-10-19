//! Normal scene export, explicit visible preflight and history-neutral bytes.
use super::*;

#[derive(Clone)]
pub(super) struct PendingSglExport {
    pub(super) revision: u64,
    pub(super) settings: obrush_core::SglExportSettings,
    pub(super) report: obrush_core::SglExportReport,
    pub(super) selected_ids: Vec<u32>,
    pub(super) active_id: Option<u32>,
}

/// The SGL action exports the whole scene, including a valid zero-object scene.
/// Only the ordinary mesh omission controls depend on the active mesh's fields.
pub(super) struct MeshExportActionControls {
    pub(super) allow_attribute_loss: Option<egui::Response>,
    pub(super) scene: egui::Response,
    pub(super) active: egui::Response,
}

pub(super) fn mesh_export_action_controls(
    ui: &mut egui::Ui,
    active_mesh: Option<&Mesh>,
    allow_attribute_loss: &mut bool,
) -> MeshExportActionControls {
    let has_attributes = active_mesh.is_some_and(Mesh::has_attributes);
    let allow_attribute_loss = Some(ui.checkbox(
        allow_attribute_loss,
        "Allow mesh export loss (attributes, source pose/provenance)",
    ));
    let scene = ui.button(crate::ui_text::caption(
        crate::ui_text::CaptionKey::C024FEA2A04D9,
    ));
    if has_attributes {
        inspector_layout::prose(
            ui,
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C0C60415D550D,
            ))
            .size(10.0)
            .color(MUTED),
        );
    }
    let active = ui.button("Export scene mesh");
    MeshExportActionControls {
        allow_attribute_loss,
        scene,
        active,
    }
}

struct SglReviewActionControls {
    allow: Option<egui::Response>,
    download: egui::Response,
    cancel: egui::Response,
}

fn sgl_review_action_controls(
    ui: &mut egui::Ui,
    report: &obrush_core::SglExportReport,
    allow_conversions: &mut bool,
) -> SglReviewActionControls {
    let allow = report.requires_allow().then(|| {
        ui.checkbox(
            allow_conversions,
            crate::ui_text::caption(crate::ui_text::CaptionKey::CC9F29BB8DD49),
        )
    });
    let download = ui.add_enabled(
        !report.requires_allow() || *allow_conversions,
        egui::Button::new(crate::ui_text::caption(
            crate::ui_text::CaptionKey::CEDBF3E66F400,
        )),
    );
    let cancel = ui.button(crate::ui_text::caption(
        crate::ui_text::CaptionKey::CB18728F64DCE,
    ));
    SglReviewActionControls {
        allow,
        download,
        cancel,
    }
}
impl OBrushApp {
    fn current_sgl_export_settings(&self) -> Result<obrush_core::SglExportSettings, String> {
        let rect = self.last_viewport;
        if !rect.is_finite() || rect.width() <= 0.0 || rect.height() <= 0.0 {
            return Err("Wait for the viewport before exporting its camera policy".into());
        }
        let source_display = self
            .source_sgl_import
            .display_for_export(&self.document)
            .cloned();
        let display = self
            .viewport_style
            .source_display
            .export_header(source_display.as_ref());
        let camera = obrush_core::SglCameraSettings {
            projection: u32::from(self.camera.orthographic),
            mode: match self.camera.mode {
                CameraMode::Orbit => 0,
                CameraMode::Spherical => 1,
                CameraMode::Plane => 2,
            },
            fov_degrees: if self.camera.source_vertical_fov {
                self.camera.fov_degrees
            } else if rect.aspect_ratio() >= 1.0 {
                self.camera.fov_degrees.clamp(10.0, 90.0)
            } else {
                self.camera.vertical_fov(rect.aspect_ratio()).to_degrees()
            },
            use_pivot: self.camera.use_pivot,
        };
        let shader = match self.viewport_style.shading {
            ShadingMode::Studio | ShadingMode::Matcap => 5,
            ShadingMode::Pbr => 0,
            ShadingMode::Normal => 2,
            ShadingMode::Uv => 4,
            ShadingMode::Flat => 1,
        };
        let plain_render = obrush_core::SglRenderSettings {
            shader,
            matcap: self.viewport_style.matcap_index,
            show_wireframe: self.viewport_style.wireframe,
            flat_shading: self.viewport_style.flat_shading,
            opacity: self.viewport_style.opacity,
        };
        Ok(obrush_core::SglExportSettings {
            scope: if self.sgl_export_selected_only {
                obrush_core::SglExportScope::SelectedParts
            } else {
                obrush_core::SglExportScope::AllScene
            },
            byte_order: obrush_core::SglByteOrder::LittleEndian,
            display,
            camera,
            plain_render,
            legacy_constructor_render: obrush_core::baseline_source_render_defaults(),
            plain_appearance_conversion: self.viewport_style.shading == ShadingMode::Studio,
            declared_display_defaults: source_display.is_none(),
            loss_policy: obrush_core::SglExportLossPolicy::RejectLoss,
        })
    }
    pub(super) fn begin_sgl_export(&mut self) {
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        match self
            .current_sgl_export_settings()
            .and_then(|s| self.document.plan_sgl_export(&s).map(|r| (s, r)))
        {
            Ok((settings, report)) => {
                self.pending_sgl_export = Some(PendingSglExport {
                    revision: self.revision,
                    settings,
                    report,
                    selected_ids: self.document.selected_subtool_ids(),
                    active_id: self.document.active_subtool_id(),
                });
                self.allow_sgl_conversions = false;
            }
            Err(error) => self.message = format!("SGL preflight rejected: {error}"),
        }
    }
    pub(super) fn sgl_export_review(&mut self, ctx: &egui::Context) {
        let Some(PendingSglExport {
            revision,
            settings,
            report,
            selected_ids,
            active_id,
        }) = self.pending_sgl_export.clone()
        else {
            return;
        };
        let mut open = true;
        let mut export = false;
        let mut cancel = false;
        egui::Window::new(crate::ui_text::caption(crate::ui_text::CaptionKey::CE6E53CBDE7E0)).open(&mut open).collapsible(false).resizable(false).default_width(460.0).show(ctx,|ui|{
            ui.label(if settings.scope == obrush_core::SglExportScope::SelectedParts {
                "Scope: selected parts (including hidden selected parts)"
            } else {
                "Scope: all parts"
            });
            ui.label(format!("Version 3 · {} parts · {} points · {} faces",report.objects,report.vertices,report.faces));
            ui.label(format!("{} original UV coordinates. Local geometry and each object’s affine matrix are kept.",report.uv_coordinates));
            ui.label(format!("Mask words rounded: {} · maximum protection error {:.9e}",report.protected_mask_words_rounded,report.maximum_mask_roundtrip_error));
            ui.label(format!("Source version/order conversions: {}/{}. Triangle/UV marker normalization: {}/{}.",report.source_version_conversions,report.source_byte_order_conversions,report.normalized_geometry_markers,report.normalized_uv_markers));
            ui.label(format!("Header/render defaults: {}/{}. Changed header/center/scale fields: {}/{}/{}.",report.defaulted_header_fields+report.declared_display_default_fields,report.defaulted_render_objects,report.changed_header_fields,report.changed_source_centers,report.changed_source_scales));
            ui.label(format!("SGL omits {} retained levels, {} layers and {} Morph Targets; {} names, {} stable IDs and {} hidden flags.",report.omitted_retained_levels,report.omitted_layers,report.omitted_morph_targets,report.omitted_subtool_names,report.omitted_stable_ids,report.omitted_visibility_objects));
            ui.label(format!("Retained hierarchies omitted: {}. Selection omitted: {} · Solo omitted: {} · custom picking policies omitted: {}.",report.omitted_retained_hierarchies,report.omitted_selection,report.omitted_solo,report.omitted_picking_policy_objects));
            if report.omitted_symmetry_offset_objects > 0 {
                ui.label(format!("{} nonzero per-object symmetry offsets cannot be stored in SGL. Save .obrush to retain them.", report.omitted_symmetry_offset_objects));
            }
            ui.label(format!("New RGB/material channels: {}/{}. Maximum derived-scale rounding error: {:.9e}.",report.absent_rgb_channels_created,report.absent_material_channels_created,report.maximum_derived_scale_rounding_error));
            ui.label(format!("Normals will be recomputed for {} points. {} source-origin archives are not embedded. SGL carries no OBrush history; save .obrush to retain those fields.",report.normals_recomputed_on_import_vertices,report.immutable_origins_not_embedded));
            if report.ordinary_appearance_conversions>0{ui.label(crate::ui_text::caption(crate::ui_text::CaptionKey::CD223A8379435));}
            ui.label(crate::ui_text::caption(crate::ui_text::CaptionKey::C9BC0507778FE));
            if report.declared_display_default_fields>0{ui.label(crate::ui_text::caption(crate::ui_text::CaptionKey::C4FF709B6A931));}
            let actions = sgl_review_action_controls(ui, &report, &mut self.allow_sgl_conversions);
            if let Some(allow) = &actions.allow {
                self.track_layer_control("file.sgl.allow", allow);
            }
            self.track_layer_control("file.sgl.download", &actions.download);
            export = actions.download.clicked();
            self.track_layer_control("file.sgl.cancel", &actions.cancel);
            cancel = actions.cancel.clicked();
        });
        if !open || cancel {
            self.pending_sgl_export = None;
            return;
        }
        if export {
            let current = match self.current_sgl_export_settings() {
                Ok(s) => s,
                Err(error) => {
                    self.message = error;
                    self.pending_sgl_export = None;
                    return;
                }
            };
            let same = current == settings
                && current.camera.fov_degrees.to_bits() == settings.camera.fov_degrees.to_bits()
                && current.plain_render.opacity.to_bits()
                    == settings.plain_render.opacity.to_bits();
            if revision != self.revision
                || !same
                || selected_ids != self.document.selected_subtool_ids()
                || active_id != self.document.active_subtool_id()
            {
                self.message = "Scene or camera changed; open a new export review.".into();
                self.pending_sgl_export = None;
                return;
            }
            let mut approved = settings;
            approved.loss_policy = if self.allow_sgl_conversions {
                obrush_core::SglExportLossPolicy::AllowAndReport
            } else {
                obrush_core::SglExportLossPolicy::RejectLoss
            };
            match self.document.export_sgl(&approved) {
                Ok(export) => {
                    self.last_sgl_export_report = Some(export.report);
                    self.write_file_contents(export.bytes, "sgl", false);
                }
                Err(error) => self.message = format!("SGL export rejected: {error}"),
            }
            self.pending_sgl_export = None;
        }
    }
}

#[cfg(test)]
mod empty_scene_export_ui_tests {
    use super::*;
    use obrush_core::{
        SglByteOrder, SglCameraSettings, SglDisplaySettings, SglExportLossPolicy, SglExportScope,
        SglExportSettings, SglStageSettings,
    };

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(480.0, 320.0),
            )),
            events,
            ..Default::default()
        }
    }

    fn scene_button_count(shape: &egui::Shape) -> usize {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().map(scene_button_count).sum(),
            egui::Shape::Text(text) => usize::from(text.galley.job.text == "Export SGL scene…"),
            _ => 0,
        }
    }

    fn settings(declared_defaults: bool) -> SglExportSettings {
        SglExportSettings {
            scope: SglExportScope::AllScene,
            byte_order: SglByteOrder::LittleEndian,
            display: SglDisplaySettings {
                show_grid: false,
                show_symmetry_line: false,
                show_contour: false,
            },
            camera: SglCameraSettings {
                projection: 0,
                mode: 0,
                fov_degrees: 40.0,
                use_pivot: false,
            },
            plain_render: obrush_core::baseline_source_render_defaults(),
            legacy_constructor_render: obrush_core::baseline_source_render_defaults(),
            plain_appearance_conversion: false,
            declared_display_defaults: declared_defaults,
            loss_policy: SglExportLossPolicy::RejectLoss,
        }
    }

    fn render_review(
        ctx: &egui::Context,
        report: &obrush_core::SglExportReport,
        allow: &mut bool,
        events: Vec<egui::Event>,
    ) -> SglReviewActionControls {
        let mut controls = None;
        let _ = ctx.run(input(events), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                controls = Some(sgl_review_action_controls(ui, report, allow));
            });
        });
        controls.expect("actual review actions were rendered")
    }

    #[test]
    fn normal_scene_export_control_is_single_and_enabled_for_none_plain_and_attributed() {
        let empty = Document::empty();
        let plain = Document::new(Mesh::cube());
        let mut mesh = Mesh::cube();
        mesh.materials = Some(vec![[0.25, 0.75]; mesh.positions.len()]);
        let attributed = Document::new(mesh);
        for document in [&empty, &plain, &attributed] {
            let ctx = egui::Context::default();
            let before = document.to_json().unwrap();
            let mut allow_loss = false;
            let mut controls = None;
            let output = ctx.run(input(vec![]), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    controls = Some(mesh_export_action_controls(
                        ui,
                        document.active_mesh(),
                        &mut allow_loss,
                    ));
                });
            });
            let controls = controls.expect("actual normal export actions were rendered");
            assert!(controls.scene.enabled());
            assert!(!controls.scene.clicked());
            assert!(controls.allow_attribute_loss.is_some());
            assert_eq!(
                output
                    .shapes
                    .iter()
                    .map(|clipped| scene_button_count(&clipped.shape))
                    .sum::<usize>(),
                1,
                "exactly one normal scene-export label is painted"
            );
            assert!(!allow_loss);
            assert_eq!(document.to_json().unwrap(), before);
        }
    }

    #[test]
    fn empty_default_review_requires_real_allow_click_before_enabling_download() {
        let document = Document::empty();
        let before = document.to_json().unwrap();
        let report = document.plan_sgl_export(&settings(true)).unwrap();
        assert_eq!((report.objects, report.vertices, report.faces), (0, 0, 0));
        assert_eq!(report.encoded_bytes, 36);
        assert!(report.requires_allow());
        let ctx = egui::Context::default();
        let mut allow = false;
        let actions = render_review(&ctx, &report, &mut allow, vec![]);
        assert!(!actions.download.enabled());
        assert!(actions.cancel.enabled());
        let position = actions
            .allow
            .expect("declared defaults require Allow")
            .rect
            .center();
        for pressed in [true, false] {
            let actions = render_review(
                &ctx,
                &report,
                &mut allow,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            if !pressed {
                assert!(allow, "normal checkbox click accepted explicit Allow");
                assert!(actions.download.enabled());
                assert!(actions.cancel.enabled());
            }
            assert_eq!(document.to_json().unwrap(), before);
        }
        assert!(!document.can_undo());
        assert!(!document.can_redo());
    }

    #[test]
    fn empty_source_review_and_encoding_preserve_whole_document_and_genuine_redo() {
        // Independently declared v3 header: no display flags, perspective Orbit,
        // FOV 40, no pivot, and an actual zero mesh count. No phantom geometry.
        let expected: Vec<u8> = [3_u32, 0, 0, 0, 0, 0, 40.0_f32.to_bits(), 0, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        let mut document = Document::empty();
        document
            .replace_sgl_bytes(&expected, &SglStageSettings::default())
            .unwrap();
        document.add_subtool(Mesh::cube(), "redo cube").unwrap();
        let after_add = document.to_json().unwrap();
        assert!(document.undo());
        assert!(document.active_mesh().is_none());
        assert!(document.active_subtool_id().is_none());
        let before = document.to_json().unwrap();
        let undo_before = document.can_undo();
        assert!(document.can_redo());
        let settings = settings(false);
        let report = document.plan_sgl_export(&settings).unwrap();
        assert_eq!((report.objects, report.vertices, report.faces), (0, 0, 0));
        assert_eq!(report.encoded_bytes, 36);
        assert!(!report.requires_allow());
        let ctx = egui::Context::default();
        let mut allow = false;
        let actions = render_review(&ctx, &report, &mut allow, vec![]);
        assert!(actions.allow.is_none());
        assert!(actions.download.enabled());
        assert!(actions.cancel.enabled());
        let export = document.export_sgl(&settings).unwrap();
        assert_eq!(export.bytes, expected);
        assert_eq!(export.report.objects, 0);
        assert_eq!(document.to_json().unwrap(), before);
        assert_eq!(document.can_undo(), undo_before);
        assert!(document.can_redo());
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after_add);
    }
}
