mod alpha_decode;
mod app;
pub mod app_preferences;
mod background_cursor;
mod background_cursor_gl;
#[cfg(test)]
mod background_cursor_tests;
mod background_mask;
pub mod brush_preferences;
#[cfg(test)]
mod continuous_clock_contract;
mod continuous_dab;
mod continuous_host;
mod continuous_locked_host;
mod dab_queue;
mod environment_assets;
mod environment_gl;
mod environment_glsl;
#[allow(dead_code)] // Independent scalar oracle; GPU interpolation is verified separately.
mod environment_ibl;
mod footprint;
pub mod i18n;
mod locked_anchor;
mod matcap;
mod move_drag;
mod navigation;
mod pressure;
pub mod public_text_keys;
pub mod renderer;
mod resident_studio;
#[cfg(test)]
mod scene_consumer_contract;
mod scene_import;
mod scene_space;
mod screen_radius_ui;
pub mod shortcut_profile;
mod source_camera_host;
#[cfg(test)]
mod source_camera_integration_tests;
#[allow(dead_code)] // Pure camera oracles are tested independently of GPU execution.
mod source_camera_math;
mod source_contour_gl;
mod source_display;
#[cfg(target_arch = "wasm32")]
mod source_geometry_browser_worker;
pub mod source_geometry_host;
mod source_geometry_worker_host;
mod source_mask_view;
mod source_modal;
mod source_paint_cursor;
mod source_paint_host;
mod source_paint_ui;
mod source_sgl_import;
pub mod source_text_keys;
mod source_wire_view;
mod startup_model;
pub mod startup_options;
mod stroke;
mod ui_fonts;
pub mod ui_text;
mod ui_text_fragments;
mod uv_texture_request;
mod uv_view;
mod viewport_focus;
pub use app::OBrushApp;
pub use uv_view::ViewTexture;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// The browser canvas must not add implicit MSAA after viewport presentation.
/// Studio MSAA and Source FXAA remain owned by the existing renderer controls.
#[cfg(target_arch = "wasm32")]
fn prepare_webgl2_canvas(canvas: &web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
    let requested = web_sys::WebGlContextAttributes::new();
    requested.set_antialias(false);
    let context = canvas
        .get_context_with_context_options("webgl2", requested.as_ref())?
        .ok_or_else(|| JsValue::from_str("OBrush requires an available WebGL2 canvas context"))?
        .dyn_into::<web_sys::WebGl2RenderingContext>()
        .map_err(|_| JsValue::from_str("OBrush canvas did not return a WebGL2 context"))?;
    let actual = context
        .get_context_attributes()
        .ok_or_else(|| JsValue::from_str("OBrush WebGL2 context attributes are unavailable"))?;
    match actual.get_antialias() {
        Some(false) => Ok(()),
        Some(true) => Err(JsValue::from_str(
            "OBrush WebGL2 context reports implicit antialiasing enabled",
        )),
        None => Err(JsValue::from_str(
            "OBrush WebGL2 context did not report its antialias setting",
        )),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
}

#[cfg(target_arch = "wasm32")]
impl Default for WebHandle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            runner: eframe::WebRunner::new(),
        }
    }

    pub async fn start(&self, canvas: web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
        self.start_with_query(canvas, String::new(), String::new())
            .await
    }
    /// Normal browser boot sends the raw location.search and navigator.language
    /// through the same Rust resolver used by explicit native CLI startup.
    pub async fn start_with_query(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        search: String,
        navigator: String,
    ) -> Result<(), JsValue> {
        let startup =
            crate::startup_options::StartupOptions::capture_auto(&search, Some(&navigator))
                .map_err(JsValue::from_str)?;
        prepare_webgl2_canvas(&canvas)?;
        self.runner
            .start(
                canvas,
                eframe::WebOptions {
                    depth_buffer: 24,
                    webgl_context_option: eframe::WebGlContextOption::WebGl2,
                    ..Default::default()
                },
                Box::new(move |cc| Ok(Box::new(OBrushApp::new_with_startup(cc, startup)?))),
            )
            .await
    }
    /// Read-only prevention policy for normal canvas keyboard delivery.
    pub fn source_shortcut_profile(&self) -> bool {
        self.runner
            .app_mut::<OBrushApp>()
            .is_some_and(|app| app.source_shortcut_profile())
    }
    pub fn load_startup_model_bytes(&self, id: u32, bytes: &[u8]) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_startup_model_bytes(id, bytes)
            .map_err(|e| JsValue::from_str(&e))
    }
    pub fn fail_startup_model(&self, id: u32, error: String) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.fail_startup_model(id, &error);
        }
    }

    pub fn load_document(&self, name: String, contents: String) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_document(&name, &contents)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Separate diagnostic WebGPU owner may capture the normal studio's
    /// actual document. The result must return through the checked core edit.
    pub fn resident_studio_begin(&self) -> Result<String, JsValue> {
        let (id, project) = self
            .runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .resident_studio_begin()
            .map_err(|error| JsValue::from_str(&error))?;
        Ok(serde_json::json!({"id":id.to_string(),"project":project}).to_string())
    }

    /// Checked, debug-only cold Source Worker transport without live preview.
    /// Its final result still receives full scene and retained-state checks.
    #[cfg(debug_assertions)]
    pub fn source_worker_cold_begin(&self) -> Result<String, JsValue> {
        let (epoch, revision, project, options) = self
            .runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .source_worker_cold_begin()
            .map_err(|error| JsValue::from_str(&error))?;
        Ok(
            serde_json::json!({"epoch":epoch.to_string(),"revision":revision.to_string(),
            "project":project,"options":options})
            .to_string(),
        )
    }

    #[cfg(debug_assertions)]
    pub fn source_worker_cold_commit(
        &self,
        epoch: &str,
        revision: &str,
        delta: &[u8],
        retained: &[u8],
    ) -> Result<bool, JsValue> {
        let epoch = epoch
            .parse::<u64>()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let revision = revision
            .parse::<u64>()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .source_worker_cold_commit_diagnostic(epoch, revision, delta, retained)
            .map_err(|error| JsValue::from_str(&error))
    }

    #[cfg(debug_assertions)]
    pub fn source_worker_cold_cancel(&self, epoch: &str) -> bool {
        let Ok(epoch) = epoch.parse::<u64>() else {
            return false;
        };
        self.runner
            .app_mut::<OBrushApp>()
            .is_some_and(|mut app| app.source_worker_cold_cancel(epoch))
    }

    #[cfg(debug_assertions)]
    pub fn source_worker_seed_packet(&self, epoch: &str) -> Result<Vec<u8>, JsValue> {
        let epoch = epoch
            .parse::<u64>()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .source_worker_seed_packet(epoch)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// The experimental browser held-pointer route. Every message is bound to
    /// the Rust studio's captured epoch/revision before any renderer or public
    /// document publication.
    pub fn source_worker_held_begin(
        &self,
        x: f64,
        y: f64,
        pressure: f64,
        pen: bool,
        modifiers: bool,
    ) -> Result<String, JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .source_worker_held_begin(x, y, pressure, pen, modifiers)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn source_worker_held_pointer_in_viewport(&self, x: f64, y: f64) -> bool {
        self.runner
            .app_mut::<OBrushApp>()
            .is_some_and(|app| app.source_worker_held_pointer_in_viewport(x, y))
    }

    pub fn source_worker_held_seed_packet(&self, epoch: &str) -> Result<Vec<u8>, JsValue> {
        let epoch = epoch
            .parse::<u64>()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .source_worker_seed_packet(epoch)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn source_worker_held_samples(
        &self,
        epoch: &str,
        x: f64,
        y: f64,
        pressure: f64,
        pen: bool,
    ) -> Result<String, JsValue> {
        let epoch = epoch
            .parse::<u64>()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .source_worker_held_samples(epoch, x, y, pressure, pen)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn source_worker_held_preview(
        &self,
        epoch: &str,
        revision: &str,
        delta: &[u8],
    ) -> Result<(), JsValue> {
        let epoch = epoch
            .parse::<u64>()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let revision = revision
            .parse::<u64>()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .source_worker_held_preview(epoch, revision, delta)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn source_worker_held_commit(
        &self,
        epoch: &str,
        revision: &str,
        delta: &[u8],
        retained: &[u8],
    ) -> Result<bool, JsValue> {
        let epoch = epoch
            .parse::<u64>()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let revision = revision
            .parse::<u64>()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .source_worker_held_commit(epoch, revision, delta, retained)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn source_worker_held_cancel(&self, epoch: &str) -> bool {
        let Ok(epoch) = epoch.parse::<u64>() else {
            return false;
        };
        self.runner
            .app_mut::<OBrushApp>()
            .is_some_and(|mut app| app.source_worker_held_cancel(epoch))
    }

    /// Eight little-endian token bytes followed by the versioned, checked
    /// GPU upload. The token is passed back as a decimal string at commit.
    pub fn resident_studio_begin_upload(&self) -> Result<Vec<u8>, JsValue> {
        let (id, upload) = self
            .runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .resident_studio_begin_upload()
            .map_err(|error| JsValue::from_str(&error))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(8 + upload.len())
            .map_err(|_| JsValue::from_str("Resident upload envelope allocation failed"))?;
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&upload);
        Ok(bytes)
    }

    pub fn resident_studio_ray(&self, x: f32, y: f32) -> Result<Vec<f64>, JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .resident_studio_ray(x, y)
            .map(|ray| ray.to_vec())
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn resident_studio_commit(&self, id: &str, sparse_bytes: &[u8]) -> Result<bool, JsValue> {
        let id = id
            .parse::<u64>()
            .map_err(|_| JsValue::from_str("Invalid resident studio capture token"))?;
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .resident_studio_commit(id, sparse_bytes)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn resident_studio_commit_timing(&self) -> Result<String, JsValue> {
        let app = self
            .runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?;
        serde_json::to_string(&app.resident_studio_commit_timing())
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    pub fn resident_studio_cancel(&self, id: &str) -> Result<(), JsValue> {
        let id = id
            .parse::<u64>()
            .map_err(|_| JsValue::from_str("Invalid resident studio capture token"))?;
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .resident_studio_cancel(id)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Read-only shared policy used by the browser before reading the file.
    pub fn document_input_limit(&self, name: &str) -> usize {
        obrush_core::resource_limits::document_input_limit(name)
    }

    pub fn load_document_bytes(&self, name: String, contents: &[u8]) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_document_bytes(&name, contents)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn cancel_source_sgl_file_request(&self, id: u32) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.cancel_source_sgl_file_request(id);
        }
    }

    pub fn begin_next_source_sgl_file_request(&self, native_little: bool) -> Result<u32, JsValue> {
        let order = if native_little {
            obrush_core::SglByteOrder::LittleEndian
        } else {
            obrush_core::SglByteOrder::BigEndian
        };
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .begin_next_source_sgl_file_request(order)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn load_source_sgl_for_request(
        &self,
        id: u32,
        name: String,
        contents: &[u8],
        native_little: bool,
    ) -> Result<(), JsValue> {
        let order = if native_little {
            obrush_core::SglByteOrder::LittleEndian
        } else {
            obrush_core::SglByteOrder::BigEndian
        };
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_source_sgl_for_request(id, &name, contents, order)
            .map_err(|e| JsValue::from_str(&e))
    }

    /// Diagnostic counters for bounded-latency stroke verification.
    pub fn sculpt_metrics(&self) -> String {
        self.runner
            .app_mut::<OBrushApp>()
            .map(|app| app.sculpt_metrics())
            .unwrap_or_else(|| "{}".into())
    }

    pub fn brush_metrics(&self) -> String {
        self.runner
            .app_mut::<OBrushApp>()
            .map(|app| app.brush_metrics())
            .unwrap_or_else(|| "{}".into())
    }

    pub fn set_source_paint_input(
        &self,
        pressure: f64,
        pen: bool,
        released: bool,
        x: f64,
        y: f64,
        cancelled: bool,
    ) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.set_source_paint_input(pressure, pen, released, x, y, cancelled);
        }
    }
    pub fn set_input_pressure(&self, pressure: f32, pen: bool, released: bool) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.set_input_pressure(pressure, pen, released);
        }
    }

    pub fn queue_browser_pen_move(&self, pressure: f32, x: f32, y: f32) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.queue_browser_pen_move(pressure, x, y);
        }
    }

    /// Forwards actual DOM navigation-button state, including chord pointer
    /// moves which do not produce a separate browser pointer-down event.
    pub fn set_navigation_buttons(&self, buttons: u16) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.set_navigation_buttons(buttons);
        }
    }

    /// Capture actual CSS-point coordinates before the web runner combines a
    /// press/move/release into one frame. This also covers mouse chords.
    pub fn set_navigation_pointer(&self, buttons: u16, x: f32, y: f32, shift: bool) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.set_navigation_pointer(buttons, x, y, shift);
        }
    }

    /// Cancels an interrupted mouse/pen contact without treating ordinary
    /// pointer-up events as cancellation.
    pub fn cancel_pointer_contact(&self) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.cancel_pointer_contact();
        }
    }

    pub fn load_alpha(
        &self,
        name: String,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_alpha(&name, width, height, rgba)
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn load_uv_texture(
        &self,
        name: String,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_uv_texture(&name, width, height, rgba)
            .map_err(|e| JsValue::from_str(&e))
    }
    pub fn load_uv_texture_png(&self, name: String, bytes: &[u8]) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_uv_texture_png(&name, bytes)
            .map_err(|e| JsValue::from_str(&e))
    }
    pub fn uv_texture_request_is_current(&self, id: u32) -> bool {
        self.runner
            .app_mut::<OBrushApp>()
            .is_some_and(|app| app.uv_texture_request_is_current(id))
    }
    pub fn cancel_uv_texture_request(&self, id: u32) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.cancel_uv_texture_request(id);
        }
    }
    pub fn load_uv_texture_png_for_request(
        &self,
        id: u32,
        name: String,
        bytes: &[u8],
    ) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_uv_texture_png_for_request(id, &name, bytes)
            .map_err(|e| JsValue::from_str(&e))
    }
    pub fn background_request_is_current(&self, id: u32) -> bool {
        self.runner
            .app_mut::<OBrushApp>()
            .is_some_and(|app| app.background_request_is_current(id))
    }
    pub fn cancel_background_request(&self, id: u32) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.cancel_background_request(id);
        }
    }
    pub fn load_background_image_for_request(
        &self,
        id: u32,
        name: String,
        bytes: &[u8],
    ) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_background_image_for_request(id, &name, bytes)
            .map_err(|e| JsValue::from_str(&e))
    }
    /// Diagnostic render-only seam mapping; never changes the document.
    pub fn uv_render_metrics(&self) -> String {
        self.runner
            .app_mut::<OBrushApp>()
            .map(|app| app.uv_render_metrics())
            .unwrap_or_else(|| "{}".into())
    }

    pub fn load_alpha_for_request(
        &self,
        id: u32,
        name: String,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_alpha_for_request(id, &name, width, height, rgba)
            .map_err(|e| JsValue::from_str(&e))
    }

    /// Imports original PNG bytes through the same bounded decoder as desktop.
    pub fn load_alpha_png_for_request(
        &self,
        id: u32,
        name: String,
        png: &[u8],
    ) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_alpha_png_for_request(id, &name, png)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Imports original PNG or JPEG bytes through the bounded shared decoder.
    pub fn load_alpha_image_for_request(
        &self,
        id: u32,
        name: String,
        bytes: &[u8],
    ) -> Result<(), JsValue> {
        self.runner
            .app_mut::<OBrushApp>()
            .ok_or_else(|| JsValue::from_str("OBrush is not running"))?
            .load_alpha_image_for_request(id, &name, bytes)
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn complete_topology(&self, id: u32, success: bool, contents: String) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.complete_topology(id, success, &contents);
        }
    }

    pub fn complete_uv_bake(&self, id: u32, success: bool, png: &[u8], report: String) {
        if let Some(mut app) = self.runner.app_mut::<OBrushApp>() {
            app.complete_uv_bake(id, success, png, &report);
        }
    }

    pub fn has_panicked(&self) -> bool {
        self.runner.has_panicked()
    }
    pub fn panic_message(&self) -> Option<String> {
        self.runner.panic_summary().map(|summary| summary.message())
    }
}

/// Executes on the dedicated worker without constructing a viewport or runner.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn topology_operation(project: &str, operation: &str) -> Result<String, JsValue> {
    obrush_core::execute_topology_project(project, operation).map_err(|e| JsValue::from_str(&e))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct UvBakeWorkerOutput {
    png: Option<Vec<u8>>,
    report: String,
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl UvBakeWorkerOutput {
    pub fn take_png(&mut self) -> Vec<u8> {
        self.png.take().unwrap_or_default()
    }
    pub fn report(&self) -> String {
        self.report.clone()
    }
}

/// Runs only on the dedicated worker. It never constructs a runner or viewport.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn uv_bake_operation(
    project: &str,
    size: u32,
    channel: &str,
) -> Result<UvBakeWorkerOutput, JsValue> {
    let run = || -> Result<UvBakeWorkerOutput, String> {
        let size = obrush_core::UvBakeSize::ALL
            .into_iter()
            .find(|s| s.pixels() == size as usize)
            .ok_or("Unsupported UV texture size")?;
        let channel = match channel {
            "color" => obrush_core::UvBakeChannel::Color,
            "roughness" => obrush_core::UvBakeChannel::Roughness,
            "metallic" => obrush_core::UvBakeChannel::Metallic,
            _ => return Err("Unknown UV bake channel".into()),
        };
        let document = obrush_core::Document::from_json(project)?;
        let export = obrush_core::bake_uv_with_cancel(
            document.mesh.ok_or("UV bake request has no active mesh")?,
            size,
            channel,
            || false,
        )?;
        Ok(UvBakeWorkerOutput {
            report: serde_json::to_string(&export.report).map_err(|e| e.to_string())?,
            png: Some(export.png),
        })
    };
    run().map_err(|e| JsValue::from_str(&e))
}
