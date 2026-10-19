//! Opt-in ordinary pointer admission for one checked Source Worker gesture.
//! The public Document remains untouched until final publication.
use super::*;
use crate::source_geometry_host::{self as geometry, SourceGeometryMode};
use crate::source_paint_host::RawPaintInput;
use obrush_core::{Mesh, SourceGeometryTool};

pub(super) struct HeldSourceWorker {
    pub epoch: u64,
    pub revision: u64,
    pub preview: Option<Arc<Mesh>>,
    last_screen: [f64; 2],
    radius: f64,
    radius_factor: f64,
    options: obrush_core::SourceGeometryOptions,
    viewport: egui::Rect,
    view: obrush_core::PaintViewCapture,
}

fn same_view(a: obrush_core::PaintViewCapture, b: obrush_core::PaintViewCapture) -> bool {
    a.world_to_clip
        .iter()
        .zip(b.world_to_clip)
        .all(|(a, b)| a.to_bits() == b.to_bits())
        && a.viewport
            .iter()
            .zip(b.viewport)
            .all(|(a, b)| a.to_bits() == b.to_bits())
}

fn input_at(
    app: &OBrushApp,
    x: f64,
    y: f64,
    pressure: f64,
    pen: bool,
) -> Result<geometry::SourceGeometryInput, String> {
    if !x.is_finite() || !y.is_finite() || !pressure.is_finite() || !(0.0..=1.0).contains(&pressure)
    {
        return Err("Source Worker pointer input is invalid".into());
    }
    let pos = egui::pos2(x as f32, y as f32);
    if !pos.is_finite() || !app.last_viewport.contains(pos) || app.egui_ctx.zoom_factor() != 1.0 {
        return Err("Source Worker pointer left the prepared 100% viewport".into());
    }
    let mut raw = RawPaintInput::default();
    raw.receive(
        pressure,
        pen,
        false,
        true,
        Some([f64::from(pos.x), f64::from(pos.y)]),
    );
    geometry::capture(&app.camera, pos, app.last_viewport, &raw, 1.0)
        .map_err(|error| error.to_string())
}

fn wire_input(
    input: geometry::SourceGeometryInput,
    radius: f64,
    radius_factor: f64,
) -> serde_json::Value {
    serde_json::json!({
        "world_to_clip": input.view.world_to_clip,
        "viewport": input.view.viewport,
        "screen": input.screen,
        "screen_radius": radius,
        "pressure": input.raw.pressure,
        "radius_factor": radius_factor,
        "in_contact": input.in_contact,
    })
}

impl OBrushApp {
    pub fn source_worker_held_pointer_in_viewport(&self, x: f64, y: f64) -> bool {
        x.is_finite()
            && y.is_finite()
            && self.last_viewport.contains(egui::pos2(x as f32, y as f32))
    }

    pub fn source_worker_held_begin(
        &mut self,
        x: f64,
        y: f64,
        pressure: f64,
        pen: bool,
        modifiers: bool,
    ) -> Result<String, String> {
        if self.source_worker_held.is_some()
            || self.stroke.is_some()
            || !self.pending_dabs.is_empty()
        {
            return Err("A Source Worker or ordinary stroke is already active".into());
        }
        if modifiers || self.brush.kind != BrushKind::Draw || self.topology_job.is_some() {
            return Err(
                "Worker trial accepts unmodified Source Draw with no background operation".into(),
            );
        }
        let prefs = self
            .effective_source_geometry_preferences(BrushKind::Draw)
            .ok_or("Source Draw preferences are unavailable")?;
        if prefs.mode != SourceGeometryMode::SourceOrdinary
            || !matches!(prefs.options.tool, SourceGeometryTool::Brush { .. })
            || prefs.mirror_offset != 0.0
        {
            return Err("Worker trial requires plain Source Draw with zero mirror offset".into());
        }
        self.source_geometry_available(BrushKind::Draw, prefs)?;
        let input = input_at(self, x, y, pressure, pen)?;
        let mesh = self
            .document
            .active_mesh()
            .ok_or("Source Worker has no active mesh")?;
        let ray = input.view.ray(input.screen)?;
        if obrush_core::prepare_source_paint_hit(
            mesh,
            self.document.active_object_metadata().map(|m| &m.affine),
            ray,
        )?
        .is_none()
        {
            return Err("Source Worker pointer has no surface hit".into());
        }
        let (epoch, revision, project, options) = self.source_worker_cold_begin()?;
        self.source_worker_held = Some(HeldSourceWorker {
            epoch,
            revision,
            preview: None,
            last_screen: input.screen,
            radius: prefs.radius,
            radius_factor: prefs.radius_factor,
            options: prefs.options,
            viewport: self.last_viewport,
            view: input.view,
        });
        Ok(serde_json::json!({
            "epoch":epoch.to_string(),"revision":revision.to_string(),
            "project":project,"options":options,"radius":prefs.radius,
            "initial_input":wire_input(input, prefs.radius, prefs.radius_factor),
            "warm":self.document.has_source_geometry_normal_state(),
        })
        .to_string())
    }

    pub fn source_worker_held_samples(
        &mut self,
        epoch: u64,
        x: f64,
        y: f64,
        pressure: f64,
        pen: bool,
    ) -> Result<String, String> {
        let input = input_at(self, x, y, pressure, pen)?;
        let prefs = self
            .effective_source_geometry_preferences(BrushKind::Draw)
            .ok_or("Source Draw preferences disappeared")?;
        let settings = self
            .source_geometry_settings(BrushKind::Draw)
            .map_err(|error| error.to_string())?;
        if settings.alpha.is_some()
            || settings.symmetry_x
            || settings.invert
            || self.preferences.tool(BrushKind::Draw).lock_position
            || self.preferences.global().continuous
        {
            return Err("Source Worker static brush controls changed during the gesture".into());
        }
        let held = self
            .source_worker_held
            .as_mut()
            .ok_or("No held Source Worker gesture")?;
        if held.epoch != epoch
            || held.revision != self.revision
            || held.viewport != self.last_viewport
            || !same_view(held.view, input.view)
        {
            return Err("Held Source Worker pointer has a stale owner or viewport".into());
        }
        if self.brush.kind != BrushKind::Draw
            || prefs.mode != SourceGeometryMode::SourceOrdinary
            || prefs.mirror_offset != 0.0
            || prefs.options != held.options
            || prefs.radius.to_bits() != held.radius.to_bits()
            || prefs.radius_factor.to_bits() != held.radius_factor.to_bits()
        {
            return Err("Source Worker brush controls changed during the gesture".into());
        }
        let dx = input.screen[0] - held.last_screen[0];
        let dy = input.screen[1] - held.last_screen[1];
        let distance = (dx * dx + dy * dy).sqrt();
        if !distance.is_finite() || distance > 65_536.0 {
            return Err("Source Worker pointer path is invalid".into());
        }
        let spacing = 0.15 * held.radius;
        if distance <= spacing {
            return Ok("[]".into());
        }
        let count = (distance / spacing).floor() as usize;
        let start = count.saturating_sub(32);
        let mut values = Vec::new();
        values
            .try_reserve_exact(count.min(32))
            .map_err(|_| "Cannot reserve Source Worker path")?;
        for i in start + 1..=count {
            let t = i as f64 / count as f64;
            let mut sample = input;
            sample.screen = [held.last_screen[0] + dx * t, held.last_screen[1] + dy * t];
            values.push(wire_input(sample, held.radius, held.radius_factor));
        }
        held.last_screen = input.screen;
        serde_json::to_string(&values).map_err(|error| error.to_string())
    }

    pub fn source_worker_held_preview(
        &mut self,
        epoch: u64,
        revision: u64,
        delta: &[u8],
    ) -> Result<(), String> {
        let held = self
            .source_worker_held
            .as_mut()
            .ok_or("No held Source Worker gesture")?;
        if held.epoch != epoch || held.revision != revision || self.revision != revision {
            return Err("Source Worker preview has a stale owner or revision".into());
        }
        held.preview = self
            .source_worker_cold
            .preview(&self.document, epoch, revision, delta)?;
        self.egui_ctx.request_repaint();
        Ok(())
    }

    pub fn source_worker_held_commit(
        &mut self,
        epoch: u64,
        revision: u64,
        delta: &[u8],
        retained: &[u8],
    ) -> Result<bool, String> {
        let held = self
            .source_worker_held
            .take()
            .ok_or("No held Source Worker gesture")?;
        if held.epoch != epoch || held.revision != revision {
            self.source_worker_cold.cancel(held.epoch);
            return Err("Source Worker finish has a stale gesture identity".into());
        }
        let prefs = self.effective_source_geometry_preferences(BrushKind::Draw);
        let settings = self.source_geometry_settings(BrushKind::Draw).ok();
        let view_unchanged = input_at(self, held.last_screen[0], held.last_screen[1], 0.5, false)
            .is_ok_and(|input| same_view(held.view, input.view));
        if self.brush.kind != BrushKind::Draw
            || held.viewport != self.last_viewport
            || !view_unchanged
            || prefs.is_none_or(|prefs| {
                prefs.mode != SourceGeometryMode::SourceOrdinary
                    || prefs.options != held.options
                    || prefs.radius.to_bits() != held.radius.to_bits()
                    || prefs.radius_factor.to_bits() != held.radius_factor.to_bits()
                    || prefs.mirror_offset != 0.0
            })
            || settings.is_none_or(|settings| {
                settings.alpha.is_some() || settings.symmetry_x || settings.invert
            })
            || self.preferences.tool(BrushKind::Draw).lock_position
            || self.preferences.global().continuous
        {
            self.source_worker_cold.cancel(epoch);
            return Err("Source Worker controls changed before release publication".into());
        }
        let result = self.source_worker_cold_commit(epoch, revision, delta, retained);
        self.egui_ctx.request_repaint();
        result
    }

    pub fn source_worker_held_cancel(&mut self, epoch: u64) -> bool {
        if self
            .source_worker_held
            .as_ref()
            .is_some_and(|held| held.epoch == epoch)
        {
            self.source_worker_held = None;
            self.egui_ctx.request_repaint();
            self.source_worker_cold.cancel(epoch)
        } else {
            false
        }
    }
}
