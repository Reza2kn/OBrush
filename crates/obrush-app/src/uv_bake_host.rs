//! Captured, cancellable PNG exports. Geometry/history stay in the shared core.
use super::*;
use obrush_core::{UvBakeChannel, UvBakeExport, UvBakeReport, UvBakeSize};

pub(super) struct UvBakeHostJob {
    id: u32,
    revision: u64,
    part: u32,
    size: UvBakeSize,
    channel: UvBakeChannel,
    filename: String,
    started: web_time::Instant,
    #[cfg(not(target_arch = "wasm32"))]
    receiver: std::sync::mpsc::Receiver<Result<UvBakeExport, String>>,
    #[cfg(not(target_arch = "wasm32"))]
    cancel: Arc<std::sync::atomic::AtomicBool>,
}
fn channel_name(channel: UvBakeChannel) -> &'static str {
    match channel {
        UvBakeChannel::Color => "color",
        UvBakeChannel::Roughness => "roughness",
        UvBakeChannel::Metallic => "metallic",
    }
}
fn filename(name: &str, channel: UvBakeChannel) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    format!(
        "{}_{}.png",
        if safe.is_empty() { "Untitled" } else { &safe },
        channel_name(channel)
    )
}
fn validate_export(
    size: UvBakeSize,
    channel: UvBakeChannel,
    export: &UvBakeExport,
) -> Result<(), String> {
    let n = size.pixels();
    let png = &export.png;
    let r: &UvBakeReport = &export.report;
    let pixels = n * n;
    if r.size != n
        || r.channel != channel
        || r.covered_pixels
            .checked_add(r.filled_gutter_pixels)
            .and_then(|v| v.checked_add(r.empty_black_pixels))
            != Some(pixels)
        || r.raster_work != r.estimated_raster_work
        || r.scratch_bytes_bound > obrush_core::MAX_UV_BAKE_SCRATCH_BYTES
        || r.raster_work > obrush_core::MAX_UV_BAKE_RASTER_WORK
        || png.len() < 33
        || png.len() > pixels * 4 + n + 1024 * 1024
        || &png[..8] != b"\x89PNG\r\n\x1a\n"
        || &png[12..16] != b"IHDR"
        || u32::from_be_bytes(png[16..20].try_into().map_err(|_| "Missing PNG width")?) as usize
            != n
        || u32::from_be_bytes(png[20..24].try_into().map_err(|_| "Missing PNG height")?) as usize
            != n
        || png[24] != 8
        || png[25] != 6
    {
        return Err("Background UV PNG dimensions/report do not match the captured request".into());
    }
    Ok(())
}
impl OBrushApp {
    pub(super) fn cancel_uv_bake(&mut self, announce: bool) {
        if let Some(job) = self.uv_bake_job.take() {
            #[cfg(target_arch = "wasm32")]
            cancel_uv_bake_worker(job.id);
            #[cfg(not(target_arch = "wasm32"))]
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            if announce {
                self.message =
                    "Texture export cancelled. Your sculpt and history were kept.".into();
            }
        }
    }
    pub(super) fn start_uv_bake(&mut self, channel: UvBakeChannel) {
        if self.uv_bake_job.is_some() || self.topology_job.is_some() {
            return;
        }
        self.finish_stroke();
        if self.stroke_rejected || !self.document.active_subtool_visible() {
            return;
        }
        if self
            .document
            .active_mesh()
            .and_then(|m| m.surface.as_ref())
            .and_then(|s| s.uv.as_ref())
            .is_none()
        {
            self.message = "The active mesh needs UV coordinates before texture export.".into();
            return;
        }
        let Some(id) = self.next_uv_bake_id.checked_add(1) else {
            self.message = "Texture export request limit reached. Save and restart.".into();
            return;
        };
        self.next_uv_bake_id = id;
        let revision = self.revision;
        let Some(part) = self.document.active_subtool_id() else {
            return;
        };
        let size = self.uv_bake_size;
        let filename = filename(&self.document.name, channel);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mesh = match self.document.require_active_mesh() {
                Ok(mesh) => mesh.snapshot(),
                Err(error) => {
                    self.message = error;
                    return;
                }
            };
            let ctx = self.egui_ctx.clone();
            let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let worker_cancel = cancel.clone();
            let (sender, receiver) = std::sync::mpsc::channel();
            match std::thread::Builder::new()
                .name("obrush-uv-bake".into())
                .spawn(move || {
                    let result = obrush_core::bake_uv_with_cancel(mesh, size, channel, || {
                        worker_cancel.load(std::sync::atomic::Ordering::Relaxed)
                    });
                    let _ = sender.send(result);
                    ctx.request_repaint();
                }) {
                Ok(_) => {
                    self.uv_bake_job = Some(UvBakeHostJob {
                        id,
                        revision,
                        part,
                        size,
                        channel,
                        filename,
                        started: web_time::Instant::now(),
                        receiver,
                        cancel,
                    })
                }
                Err(error) => {
                    self.message = format!("Cannot start texture export: {error}");
                    return;
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let project = match self.document.to_json() {
                Ok(p) => p,
                Err(error) => {
                    self.message = error;
                    return;
                }
            };
            self.uv_bake_job = Some(UvBakeHostJob {
                id,
                revision,
                part,
                size,
                channel,
                filename,
                started: web_time::Instant::now(),
            });
            if let Err(error) =
                start_uv_bake_worker(id, &project, size.pixels() as u32, channel_name(channel))
            {
                self.uv_bake_job = None;
                self.message = format!("Cannot start texture export: {error:?}");
                return;
            }
        }
        self.last_uv_bake_report = None;
        self.message = "Baking a copy of the active mesh. You can orbit or cancel.".into();
        self.egui_ctx.request_repaint();
    }
    fn accept_uv_bake(&mut self, job: UvBakeHostJob, result: Result<UvBakeExport, String>) {
        if job.revision != self.revision || Some(job.part) != self.document.active_subtool_id() {
            self.message = "Texture export discarded because the document changed.".into();
            return;
        }
        match result.and_then(|export| {
            validate_export(job.size, job.channel, &export)?;
            Ok(export)
        }) {
            Ok(export) => {
                self.last_uv_bake_report = Some(export.report);
                self.write_file_contents_named(export.png, "png", false, job.filename);
            }
            Err(error) => self.message = format!("Texture export failed: {error}"),
        }
    }
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn complete_uv_bake(&mut self, id: u32, success: bool, png: &[u8], report: &str) {
        if self.uv_bake_job.as_ref().is_none_or(|j| j.id != id) {
            return;
        }
        let job = self.uv_bake_job.take().expect("matching request checked");
        let result = if success {
            if png.len() > job.size.pixels().pow(2) * 4 + job.size.pixels() + 1024 * 1024
                || report.len() > 4096
            {
                Err("Background UV result exceeds its allocation limit".into())
            } else {
                serde_json::from_str::<UvBakeReport>(report)
                    .map_err(|e| e.to_string())
                    .and_then(|r| {
                        let mut bytes = Vec::new();
                        bytes
                            .try_reserve_exact(png.len())
                            .map_err(|_| "Cannot retain bounded exported PNG")?;
                        bytes.extend_from_slice(png);
                        Ok(UvBakeExport {
                            png: bytes,
                            report: r,
                        })
                    })
            }
        } else {
            Err(report.chars().take(1024).collect())
        };
        self.accept_uv_bake(job, result);
        self.egui_ctx.request_repaint();
    }
    pub(super) fn poll_uv_bake(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(job) = self.uv_bake_job.as_ref() {
            let result = match job.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Texture worker ended unexpectedly".into()))
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                let job = self.uv_bake_job.take().expect("active worker checked");
                self.accept_uv_bake(job, result);
            }
        }
        if self.uv_bake_job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
    pub(super) fn uv_bake_metrics(&self) -> serde_json::Value {
        serde_json::json!({"size":self.uv_bake_size.pixels(),"busy":self.uv_bake_job.is_some(),"request":self.uv_bake_job.as_ref().map(|j|serde_json::json!({"id":j.id,"revision":j.revision,"subtool_id":j.part,"size":j.size.pixels(),"channel":channel_name(j.channel),"elapsed_seconds":j.started.elapsed().as_secs_f64()})),"report":self.last_uv_bake_report})
    }
    pub(super) fn uv_bake_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(14.0);
        let has_uv = self
            .document
            .active_mesh()
            .and_then(|m| m.surface.as_ref())
            .and_then(|s| s.uv.as_ref())
            .is_some();
        let selected_size = format!(
            "{}: {}",
            crate::ui_text::caption(crate::ui_text::CaptionKey::CFAA8A951AE15),
            self.uv_bake_size.pixels()
        );
        let combo = inspector_layout::combo(ui, "uv_bake_size", &selected_size, |ui| {
            for size in UvBakeSize::ALL {
                let row =
                    ui.selectable_value(&mut self.uv_bake_size, size, size.pixels().to_string());
                self.track_layer_control(&format!("file.uv_size.{}", size.pixels()), &row);
            }
        })
        .response;
        self.track_layer_control("file.uv_size", &combo);
        for (channel, label) in [
            (
                UvBakeChannel::Color,
                &crate::ui_text::caption(crate::ui_text::CaptionKey::C413F1679B6B3),
            ),
            (
                UvBakeChannel::Roughness,
                &crate::ui_text::caption(crate::ui_text::CaptionKey::CFAF8AA87390F),
            ),
            (
                UvBakeChannel::Metallic,
                &crate::ui_text::caption(crate::ui_text::CaptionKey::C85F5FE9A6AF0),
            ),
        ] {
            let button = ui.add_enabled(
                has_uv
                    && self.document.active_subtool_visible()
                    && self.topology_job.is_none()
                    && self.uv_bake_job.is_none(),
                egui::Button::new(label),
            );
            self.track_layer_control(
                &format!("file.uv_export.{}", channel_name(channel)),
                &button,
            );
            if button.clicked() {
                self.start_uv_bake(channel);
            }
        }
        if !has_uv {
            inspector_layout::prose(
                ui,
                RichText::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CDB546E0641A5,
                ))
                .size(10.0)
                .color(MUTED),
            );
        }
        if self.uv_bake_job.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CF4E1D9942518,
                ));
            });
            let cancel = ui.button(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C2D89DD5192E5,
            ));
            self.track_layer_control("file.uv_cancel", &cancel);
            if cancel.clicked() {
                self.cancel_uv_bake(true);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(catch, js_namespace = window, js_name = obrushStartUVBake)]
    fn start_uv_bake_worker(
        id: u32,
        project: &str,
        size: u32,
        channel: &str,
    ) -> Result<(), wasm_bindgen::JsValue>;
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = obrushCancelUVBake)]
    fn cancel_uv_bake_worker(id: u32);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn export() -> UvBakeExport {
        let mesh = Mesh::import_obj(
            b"v 0 0 0 1 0 0\nv 1 0 0 0 1 0\nv 0 1 0 0 0 1\nvt 0 0\nvt 1 0\nvt 0 1\nf 1/1 2/2 3/3\n",
        )
        .unwrap()
        .mesh;
        obrush_core::bake_uv_with_cancel(mesh, UvBakeSize::S256, UvBakeChannel::Color, || false)
            .unwrap()
    }
    #[test]
    fn captured_export_dimensions_channel_and_coverage_are_checked() {
        let mut e = export();
        assert!(validate_export(UvBakeSize::S256, UvBakeChannel::Color, &e).is_ok());
        assert!(validate_export(UvBakeSize::S512, UvBakeChannel::Color, &e).is_err());
        assert!(validate_export(UvBakeSize::S256, UvBakeChannel::Metallic, &e).is_err());
        e.report.empty_black_pixels += 1;
        assert!(validate_export(UvBakeSize::S256, UvBakeChannel::Color, &e).is_err());
    }
    #[test]
    fn truncated_or_wrong_png_header_cannot_be_saved_as_completed_export() {
        let mut e = export();
        e.png[16] ^= 1;
        assert!(validate_export(UvBakeSize::S256, UvBakeChannel::Color, &e).is_err());
        e.png.truncate(32);
        assert!(validate_export(UvBakeSize::S256, UvBakeChannel::Color, &e).is_err());
    }
    #[test]
    fn exported_names_are_captured_sanitized_and_channel_specific() {
        assert_eq!(filename("", UvBakeChannel::Color), "Untitled_color.png");
        assert_eq!(
            filename("../test\nname", UvBakeChannel::Metallic),
            "___test_name_metallic.png"
        );
        assert!(filename(&"a".repeat(100), UvBakeChannel::Roughness).starts_with(&"a".repeat(80)));
    }
}
