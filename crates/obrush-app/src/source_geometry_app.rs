//! Actual opt-in ordinary geometry UI/session/queue/clock consumer.
//! Source geometry owns its Document transaction. Old Paint stays independent.
use super::*;
use crate::source_geometry_host::{
    self as geometry, SourceGeometryMode, SourceGeometryPreferences,
};
use obrush_core::{SourceClayOrderPolicy, SourceGeometryError, SourceGeometryTool};

pub(super) fn smooth_tangent_control(ui: &mut egui::Ui, tangent: &mut bool) -> egui::Response {
    ui.checkbox(
        tangent,
        crate::ui_text::caption(crate::ui_text::CaptionKey::CA0143501678A),
    )
}

pub(super) struct GeometryPreferenceStore {
    rows: [SourceGeometryPreferences; 7],
}
impl Default for GeometryPreferenceStore {
    fn default() -> Self {
        let ordinary_clay = SourceGeometryPreferences {
            options: obrush_core::SourceGeometryOptions {
                clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
                ..Default::default()
            },
            ..Default::default()
        };
        Self {
            rows: [
                ordinary_clay,
                ordinary_clay,
                SourceGeometryPreferences::inflate(),
                SourceGeometryPreferences::pinch(),
                SourceGeometryPreferences::smooth(),
                SourceGeometryPreferences::flatten(),
                SourceGeometryPreferences::crease(),
            ],
        }
    }
}
impl GeometryPreferenceStore {
    pub(super) fn index(kind: BrushKind) -> Option<usize> {
        match kind {
            BrushKind::Draw => Some(0),
            BrushKind::Clay => Some(1),
            BrushKind::Inflate => Some(2),
            BrushKind::Pinch => Some(3),
            BrushKind::Smooth => Some(4),
            BrushKind::Flatten => Some(5),
            BrushKind::Crease => Some(6),
            _ => None,
        }
    }
    pub(super) fn get(&self, kind: BrushKind) -> Option<SourceGeometryPreferences> {
        Self::index(kind).map(|i| self.rows[i])
    }
    pub(super) fn set(
        &mut self,
        kind: BrushKind,
        p: SourceGeometryPreferences,
    ) -> Result<(), SourceGeometryError> {
        p.validate()?;
        let i = Self::index(kind).ok_or(SourceGeometryError::UnsupportedDomain)?;
        self.rows[i] = p;
        Ok(())
    }
}
fn preferences_for_active_object(
    document: &Document,
    mut prefs: SourceGeometryPreferences,
) -> SourceGeometryPreferences {
    if let Some(metadata) = document.active_object_metadata() {
        prefs.mirror_offset = metadata.symmetry_offset;
    }
    prefs
}
impl OBrushApp {
    /// Source objects own their normalized mirror offset. Plain OBrush meshes
    /// still use the older per-tool Source preference until they gain metadata.
    pub(super) fn effective_source_geometry_preferences(
        &self,
        kind: BrushKind,
    ) -> Option<SourceGeometryPreferences> {
        self.source_geometry_preferences
            .get(kind)
            .map(|prefs| preferences_for_active_object(&self.document, prefs))
    }
    pub(super) fn source_geometry_requested(&self, kind: BrushKind) -> bool {
        self.source_geometry_preferences
            .get(kind)
            .is_some_and(|p| p.mode == SourceGeometryMode::SourceOrdinary)
    }
    pub(super) fn source_geometry_settings(
        &self,
        kind: BrushKind,
    ) -> Result<BrushSettings, SourceGeometryError> {
        let mut settings = self
            .preferences
            .preview(
                temporary_modifiers(self.preferences.selected(), kind),
                ContactInput::Mouse,
                None,
                None,
            )
            .map_err(|_| SourceGeometryError::InvalidInput)?
            .settings;
        settings.kind = kind;
        // Explicit fixed topology belongs to the selected Source response,
        // not to Legacy's automatically recomputed adaptive capability.
        settings.adaptive_detail = false;
        Ok(settings)
    }
    pub(super) fn source_geometry_diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"per_tool":self.source_geometry_preferences.rows.map(SourceGeometryPreferences::diagnostics),
            "active_object_symmetry_offset":self.document.active_object_metadata().map(|m|m.symmetry_offset),
            "active":self.stroke.as_ref().and_then(|s|s.source_geometry.as_ref()).and_then(|s|s.lock().ok()).map(|s|s.diagnostics()),
            "last_completed":self.last_source_geometry_report,"state_bytes":self.document.source_geometry_retained_bytes(),
            "alpha_tracks":self.source_geometry_alpha_tracks.lock().ok().map(|t|serde_json::json!({"tracks":t.snapshot(),"claim":t.active_claim()})),
            "scope":"ordinary/timer including plain/tangent Smooth, side-selective Flatten and proxy-based Crease; fixed static U; captured mirror/nearest alpha; no Lock/manual rotation/layers/retained hierarchy; bounded ordered static selection supports general Clay; explicit singleleaf compatibility requires<=100; supplied face IDs/order and precise OBrush camera remain unpaired Source; SourcePaint uses captured validated warm normals when available/cold fresh otherwise; warm hierarchy/layers blocked"})
    }
    pub(super) fn track_source_geometry_control(&mut self, key: &str, response: &egui::Response) {
        self.track_morph_control(key, response);
        self.source_geometry_controls_hit_rect
            .record(key, Some(response));
    }
    pub(super) fn commit_source_geometry_preferences(
        &mut self,
        kind: BrushKind,
        candidate: SourceGeometryPreferences,
    ) -> bool {
        let Some(old) = self.effective_source_geometry_preferences(kind) else {
            return false;
        };
        let stored_offset = self
            .source_geometry_preferences
            .get(kind)
            .expect("effective preferences have a stored tool row")
            .mirror_offset;
        match super::source_geometry_ui::commit_candidate(old, candidate, || {
            self.finish_stroke();
            !self.stroke_rejected
        }) {
            Ok(super::source_geometry_ui::GeometryCommit::Unchanged) => true,
            Ok(super::source_geometry_ui::GeometryCommit::Accepted(p)) => {
                let mut stored = p;
                if self.document.active_object_metadata().is_some() {
                    if p.mirror_offset.to_bits() != old.mirror_offset.to_bits()
                        && let Err(error) = self
                            .document
                            .set_active_source_symmetry_offset(p.mirror_offset)
                    {
                        self.message = error;
                        return false;
                    }
                    // A source-object edit must not leak into plain meshes'
                    // older per-tool fallback value.
                    stored.mirror_offset = stored_offset;
                }
                match self.source_geometry_preferences.set(kind, stored) {
                    Ok(()) => true,
                    Err(e) => {
                        self.message = e.to_string();
                        false
                    }
                }
            }
            Ok(super::source_geometry_ui::GeometryCommit::ReleaseRejected) => {
                if let Some(i) = GeometryPreferenceStore::index(kind) {
                    self.source_geometry_editor_drafts[i].reset(old);
                }
                false
            }
            Err(e) => {
                self.message = e.to_string();
                false
            }
        }
    }
    pub(super) fn source_geometry_available(
        &self,
        kind: BrushKind,
        prefs: SourceGeometryPreferences,
    ) -> Result<(), String> {
        if self.egui_ctx.zoom_factor() != 1. {
            return Err("Source ordinary currently requires UI scale100%.".into());
        }
        let settings = self
            .source_geometry_settings(kind)
            .map_err(|e| e.to_string())?;
        if settings.symmetry_offset != 0.0 {
            return Err("Clear the legacy absolute mirror offset first; Source has its own radius-relative offset.".into());
        }
        if settings.alpha_rotation != 0.0 {
            return Err(
                "Clear manual alpha rotation first; Source alpha follows captured stroke travel."
                    .into(),
            );
        }
        if self.preferences.tool(kind).lock_position {
            return Err("Turn off Lock stamp center in OBrush settings first.".into());
        }
        if settings.invert {
            return Err(if kind == BrushKind::Smooth {
                "Source Smooth has no Negative/invert option; release Alt and clear OBrush invert."
            } else {
                "Turn off OBrush invert first; use the Source Negative option."
            }
            .into());
        }
        geometry::blocker_controls(&self.document,&settings,false,prefs).map_err(|e|match e {
            SourceGeometryError::UnsupportedSmoothNegative=>"Source Smooth has no Negative option.".into(),
            SourceGeometryError::UnsupportedSmoothOrder=>"Source Smooth requires ordered static original-face selection.".into(),
            SourceGeometryError::UnsupportedFlattenOrder=>"Source Flatten requires ordered static original-face selection.".into(),
            SourceGeometryError::UnsupportedClayOrder=>"Single-leaf Clay requires at most100 original faces. Select ordered static octree for general Clay, or turn off its area plane for plain Brush.".into(),
            SourceGeometryError::MaskConvention=>"Inspect complete mask precision, then explicitly convert to raw Source U before enabling.".into(),
            SourceGeometryError::StaleNormalState=>"Stored Source normals are stale after other geometry edits. Reset the transient field explicitly.".into(),
            SourceGeometryError::UnsupportedDomain=>"Source geometry requires a visible active mesh without retained hierarchy or layers.".into(),
            other=>other.to_string(),
        })
    }
    pub(super) fn source_geometry_panel(&mut self, ui: &mut egui::Ui) {
        let kind = self.brush.kind;
        let Some(current) = self.effective_source_geometry_preferences(kind) else {
            return;
        };
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C3B6794E612E8,
            ))
            .size(10.)
            .color(MUTED),
        );
        let available = super::source_geometry_ui::panel_preflight(
            self.stroke
                .as_ref()
                .and_then(|stroke| stroke.source_geometry.as_ref()),
            || self.source_geometry_available(kind, current),
        );
        let mut candidate = current;
        let legacy = ui.selectable_value(&mut candidate.mode, SourceGeometryMode::Legacy, "OBrush");
        self.track_source_geometry_control("source_geometry.mode.legacy", &legacy);
        let source = ui.add_enabled(
            available.is_ok(),
            egui::Button::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C28CBD374B931,
            ))
            .selected(current.mode == SourceGeometryMode::SourceOrdinary),
        );
        self.track_source_geometry_control("source_geometry.mode.source", &source);
        if source.clicked() {
            candidate.mode = SourceGeometryMode::SourceOrdinary;
        }
        if let Err(e) = available {
            ui.label(RichText::new(e.to_string()).size(10.).color(MUTED));
        }
        ui.label(
            RichText::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::CCA96CD519A3E,
            ))
            .size(10.)
            .color(MUTED),
        );
        self.source_geometry_mask_controls(ui);
        let reset = ui.add_enabled(
            self.document
                .active_subtool_id()
                .and_then(|id| self.document.source_geometry_state(id))
                .is_some(),
            egui::Button::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C339E0571E3B3,
            )),
        );
        self.track_source_geometry_control("source_geometry.normals.reset", &reset);
        if reset.clicked() {
            self.finish_stroke();
            if !self.stroke_rejected {
                match self.document.reset_source_geometry_normal_state() {
                    Ok(true) => {
                        self.egui_ctx.request_repaint();
                        self.message =
                            "Transient Source normals reset; the next useful stroke starts fresh."
                                .into();
                    }
                    Ok(false) => {}
                    Err(e) => self.message = e.to_string(),
                }
            }
        }
        if candidate.mode != current.mode {
            self.commit_source_geometry_preferences(kind, candidate);
            return;
        }
        // Source options can be configured before enabling, so a large mesh
        // can explicitly choose plain Brush instead of the blocked Clay scope.
        let i = GeometryPreferenceStore::index(kind).expect("supported kind");
        let numeric = super::source_geometry_ui::render(
            ui,
            current,
            &mut self.source_geometry_editor_drafts[i],
        );
        for (key, response) in &numeric.controls {
            self.track_source_geometry_control(key, response);
        }
        if let Some(e) = numeric.rejected {
            self.message = e;
        }
        candidate = numeric.candidate.unwrap_or(current);
        let offset = super::source_geometry_ui::render_mirror_offset(
            ui,
            candidate,
            &mut self.source_geometry_editor_drafts[i],
        );
        for (key, response) in &offset.controls {
            self.track_source_geometry_control(key, response);
        }
        if let Some(error) = offset.rejected {
            self.message = error;
        }
        if let Some(next) = offset.candidate {
            candidate = next;
        }
        let response = ui
            .checkbox(
                &mut candidate.options.prevent_folds,
                crate::ui_text::caption(crate::ui_text::CaptionKey::PreventFolds),
            )
            .on_hover_text(crate::ui_text::caption(
                crate::ui_text::CaptionKey::PreventFoldsHelp,
            ));
        self.track_source_geometry_control("source_geometry.prevent_folds", &response);
        if let SourceGeometryTool::Smooth { ref mut tangent } = candidate.options.tool {
            let r = smooth_tangent_control(ui, tangent);
            self.track_source_geometry_control("source_geometry.smooth.tangent", &r);
            let r = ui.checkbox(
                &mut candidate.options.culling,
                crate::ui_text::caption(crate::ui_text::CaptionKey::CB67CE0968F72),
            );
            self.track_source_geometry_control("source_geometry.culling", &r);
        } else {
            if !matches!(
                candidate.options.tool,
                SourceGeometryTool::Flatten | SourceGeometryTool::Crease
            ) {
                let order = super::source_geometry_ui::render_selection_order(
                    ui,
                    candidate.options.clay_order,
                );
                for (key, response) in &order.controls {
                    self.track_source_geometry_control(key, response);
                }
                if let Some(selection) = order.candidate {
                    candidate.options.clay_order = selection;
                }
            }
            for (key, label, value) in [
                (
                    "source_geometry.culling",
                    &crate::ui_text::caption(crate::ui_text::CaptionKey::CB67CE0968F72),
                    &mut candidate.options.culling,
                ),
                (
                    "source_geometry.negative",
                    &crate::ui_text::caption(crate::ui_text::CaptionKey::C2580AF0E9971),
                    &mut candidate.options.negative,
                ),
            ] {
                let r = ui.checkbox(value, label);
                self.track_source_geometry_control(key, &r);
            }
        }
        if let SourceGeometryTool::Brush {
            ref mut clay,
            ref mut accumulate,
        } = candidate.options.tool
        {
            let r = ui.checkbox(
                clay,
                crate::ui_text::caption(crate::ui_text::CaptionKey::C1757011FD6FE),
            );
            self.track_source_geometry_control("source_geometry.clay", &r);
            let r = ui.checkbox(
                accumulate,
                crate::ui_text::caption(crate::ui_text::CaptionKey::C0961041BAAB8),
            );
            self.track_source_geometry_control("source_geometry.accumulate", &r);
        }
        if current.mode == SourceGeometryMode::SourceOrdinary {
            self.source_geometry_alpha_panel(ui);
        }
        let mut continuous = self.preferences.global().continuous;
        let r = ui.checkbox(
            &mut continuous,
            crate::ui_text::caption(crate::ui_text::CaptionKey::C54ED4988F9F3),
        );
        self.track_source_geometry_control("source_geometry.continuous", &r);
        if r.changed() {
            self.set_continuous_mode(continuous);
        }
        if !super::source_geometry_ui::same_words(candidate, current) {
            self.commit_source_geometry_preferences(kind, candidate);
        }
    }
    pub(super) fn capture_source_geometry_input(
        &self,
        pos: Pos2,
        rect: egui::Rect,
    ) -> Result<geometry::SourceGeometryInput, SourceGeometryError> {
        geometry::capture(
            &self.camera,
            pos,
            rect,
            &self.source_paint_raw,
            self.egui_ctx.zoom_factor(),
        )
    }
    pub(super) fn begin_source_geometry_stroke(
        &mut self,
        ctx: &egui::Context,
        pos: Pos2,
        rect: egui::Rect,
        kind: BrushKind,
        invert: bool,
    ) {
        self.finish_stroke();
        if self.stroke_rejected {
            return;
        }
        let Some(prefs) = self.effective_source_geometry_preferences(kind) else {
            return;
        };
        let mut settings = match self.source_geometry_settings(kind) {
            Ok(s) => s,
            Err(e) => {
                self.message = e.to_string();
                return;
            }
        };
        settings.invert ^= invert;
        let session = self
            .capture_source_geometry_input(pos, rect)
            .and_then(|input| {
                geometry::SourceGeometrySession::begin_with_controls(
                    &mut self.document,
                    &settings,
                    self.preferences.tool(kind).lock_position,
                    prefs,
                    input,
                    self.source_geometry_alpha_tracks.clone(),
                )
            });
        let session = match session {
            Ok(s) => s,
            Err(e) => {
                self.message = e.to_string();
                return;
            }
        };
        let continuous = if self.preferences.global().continuous {
            let prepared = (|| -> Result<StrokeKey, String> {
                let next = self
                    .next_continuous_serial
                    .checked_add(1)
                    .ok_or("Continuous identifiers exhausted")?;
                let id = self
                    .document
                    .active_subtool_id()
                    .ok_or("No active Source target")?;
                let now = self.continuous_now().ok_or("Continuous clock overflow")?;
                let key = StrokeKey {
                    serial: self.next_continuous_serial,
                    subtool_id: id,
                    kind,
                };
                self.continuous_clock.start(key, true, false, now)?;
                self.next_continuous_serial = next;
                Ok(key)
            })();
            match prepared {
                Ok(key) => {
                    ctx.request_repaint_after(std::time::Duration::from_micros(16_600));
                    Some(key)
                }
                Err(e) => {
                    if let Ok(mut s) = session.lock() {
                        let _ = s.cancel(&mut self.document);
                    }
                    self.message = e;
                    return;
                }
            }
        } else {
            None
        };
        // Session owns begin; no generic queued Begin/End marker is admitted.
        self.stroke = Some(ActiveStroke {
            settings,
            screen_radius: ScreenRadiusPreferences {
                enabled: true,
                pixels: prefs.radius as f32,
            },
            pressure_factors: None,
            last_screen: Some(pos),
            sampler: StrokeSampler::new(pos, 1.),
            movement: None,
            local_transform: None,
            locked_stamp: None,
            kind,
            invert,
            queued: true,
            continuous,
            source_paint: None,
            source_paint_display_before: None,
            source_geometry: Some(session),
            source_geometry_dirty_before: Some(self.dirty),
            continuous_locked: None,
        });
    }
    pub(super) fn sample_source_geometry_to(
        &mut self,
        session: geometry::SharedSourceGeometry,
        pos: Pos2,
        rect: egui::Rect,
    ) {
        let result = self.capture_source_geometry_input(pos, rect).and_then(|i| {
            session
                .lock()
                .map_err(|_| SourceGeometryError::StrokeBindingChanged)?
                .path_samples(i)
        });
        match result {
            Ok(samples) => {
                for input in samples {
                    self.pending_dabs
                        .source_geometry_dab(session.clone(), input);
                }
            }
            Err(e) => self.reject_source_geometry(session, e),
        }
    }
    pub(super) fn stamp_source_geometry(
        &mut self,
        session: geometry::SharedSourceGeometry,
        pos: Pos2,
        rect: egui::Rect,
    ) {
        match self
            .capture_source_geometry_input(pos, rect)
            .and_then(|i| geometry::apply_shared(&mut self.document, &session, i))
        {
            Ok(report) => {
                if report.changed_vertices > 0 {
                    self.changed();
                }
                if report.derived_cache_changed {
                    self.egui_ctx.request_repaint();
                }
            }
            Err(e) => self.reject_source_geometry(session, e),
        }
    }
    pub(super) fn reject_source_geometry(
        &mut self,
        session: geometry::SharedSourceGeometry,
        e: SourceGeometryError,
    ) {
        if let Ok(mut s) = session.lock() {
            s.failed = Some(e.clone());
        }
        self.unprocessed_release = None;
        self.stop_continuous(StopReason::Command);
        self.finish_stroke_deferred();
        self.stroke_rejected = true;
        self.message = e.to_string();
    }
    pub(super) fn finish_source_geometry_session(
        &mut self,
        stroke: &ActiveStroke,
        session: geometry::SharedSourceGeometry,
    ) {
        let progress = self
            .pending_dabs
            .release_source_geometry(&mut self.document);
        if progress.changed {
            self.changed();
        }
        if progress.derived_cache_changed {
            self.egui_ctx.request_repaint();
        }
        let result = session
            .lock()
            .map_err(|_| SourceGeometryError::StrokeBindingChanged)
            .and_then(|mut s| {
                let failed = s.failed.clone();
                s.finish(&mut self.document)?;
                failed.map_or(Ok(()), Err)
            });
        if let Err(e) = &result {
            self.dirty = stroke.source_geometry_dirty_before.unwrap_or(self.dirty);
            self.revision = self.revision.wrapping_add(1);
            self.stroke_rejected = true;
            self.message = e.to_string();
        }
        self.last_source_geometry_report=session.lock().ok().map(|s|serde_json::json!({"revision":self.revision,"error":result.err(),"dirty":self.dirty,"capture":s.diagnostics()}));
    }
    pub(super) fn interrupt_source_geometry_tail(&mut self, reason: &str) -> bool {
        let Some(session) = self.stroke.as_ref().and_then(|s| s.source_geometry.clone()) else {
            return false;
        };
        if let Ok(mut s) = session.lock() {
            s.failed.get_or_insert(SourceGeometryError::InvalidInput);
        }
        self.message = format!("Source geometry interrupted: {reason}");
        true
    }
}

impl OBrushApp {
    pub(super) fn source_geometry_mask_controls(&mut self, ui: &mut egui::Ui) {
        let preview = ui.add_enabled(
            self.document.active_subtool_visible(),
            egui::Button::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C4DFABF7285EC,
            )),
        );
        self.track_source_geometry_control("source_geometry.mask.preview", &preview);
        if preview.clicked() {
            self.finish_stroke();
            if !self.stroke_rejected {
                match self.document.active_subtool_mask_conversion_report(
                    obrush_core::MaskConvention::UnprotectedF32V1,
                ) {
                    Ok(report) => {
                        self.source_mask_preview = Some((
                            self.revision,
                            self.document.active_subtool_id(),
                            self.document.current_level(),
                            report,
                        ))
                    }
                    Err(error) => self.message = error,
                }
            }
        }
        if let Some((revision, id, level, report)) = self.source_mask_preview.clone() {
            let current = revision == self.revision
                && id == self.document.active_subtool_id()
                && level == self.document.current_level();
            ui.label(format!(
                "{} levels · {} mask values · {} roundtrip words change",
                report.levels.len(),
                report.vertices,
                report.roundtrip_changed_words
            ));
            ui.label(format!(
                "Max absolute {:.8e}, relative {:.8e}; {} signed-zero-only words",
                report.max_abs_roundtrip_error,
                report.max_relative_roundtrip_error,
                report.signed_zero_only_changes
            ));
            for row in &report.levels {
                if let Some(w) = row.first_changed_word {
                    ui.label(format!(
                        "Level {} / vertex {}: {:08x} → {:08x} → {:08x}",
                        row.level + 1,
                        w.vertex,
                        w.source_bits,
                        w.converted_bits,
                        w.roundtrip_bits
                    ));
                }
            }
            let allow = ui.checkbox(
                &mut self.source_allow_mask_loss,
                crate::ui_text::caption(crate::ui_text::CaptionKey::C1595029C133A),
            );
            self.track_source_geometry_control("source_geometry.mask.allow_loss", &allow);
            let convert = ui.add_enabled(
                current
                    && report.changed
                    && (report.roundtrip_changed_words == 0 || self.source_allow_mask_loss),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C826E8E985810,
                )),
            );
            self.track_source_geometry_control("source_geometry.mask.convert", &convert);
            if convert.clicked() {
                self.finish_stroke();
                if !self.stroke_rejected {
                    let fresh = self.document.active_subtool_mask_conversion_report(
                        obrush_core::MaskConvention::UnprotectedF32V1,
                    );
                    if fresh.as_ref().is_ok_and(|fresh| fresh == &report) {
                        let policy = if self.source_allow_mask_loss {
                            obrush_core::MaskConversionPolicy::AllowAndReport
                        } else {
                            obrush_core::MaskConversionPolicy::RequireRoundtripExact
                        };
                        match self.document.convert_active_subtool_masks(
                            obrush_core::MaskConvention::UnprotectedF32V1,
                            policy,
                        ) {
                            Ok(result) => {
                                if result.changed {
                                    self.changed();
                                }
                                self.message="Complete active mask representation converted; one Undo restores all levels.".into();
                                self.source_mask_preview = None;
                            }
                            Err(e) => self.message = e,
                        }
                    } else {
                        self.message = "Mask precision preview is stale; inspect it again.".into();
                        self.source_mask_preview = None;
                    }
                }
            }
        }
    }
}

impl OBrushApp {
    pub(super) fn source_geometry_continuous_frame(
        &mut self,
        ctx: &egui::Context,
        rect: egui::Rect,
        key: StrokeKey,
        session: geometry::SharedSourceGeometry,
    ) {
        let captured = match session.lock() {
            Ok(s) => s.preferences(),
            Err(_) => {
                self.stop_continuous(StopReason::Command);
                self.finish_stroke_deferred();
                self.message = "Source geometry session lock failed.".into();
                return;
            }
        };
        let (position, down, focused) = ctx.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.primary_down(),
                i.focused,
            )
        });
        let position = position.filter(|p| p.is_finite() && rect.contains(*p));
        let prepared = position
            .map(|p| self.capture_source_geometry_input(p, rect))
            .transpose();
        let input = match prepared {
            Ok(value) => value,
            Err(error) => {
                if let Ok(mut source) = session.lock() {
                    source.failed = Some(error.clone());
                }
                self.unprocessed_release = None;
                self.stop_continuous(StopReason::InvalidContact);
                self.finish_stroke_deferred();
                self.stroke_rejected = true;
                self.message = error.to_string();
                return;
            }
        };
        let Some(now) = self.continuous_now() else {
            self.stop_continuous(StopReason::ClockOverflow);
            self.finish_stroke_deferred();
            return;
        };
        let surface_hit = input.is_some_and(|input| {
            self.document
                .require_active_mesh()
                .and_then(|mesh| {
                    obrush_core::prepare_source_paint_pass(
                        mesh,
                        self.document.active_object_metadata().map(|m| &m.affine),
                        obrush_core::PaintPreparationInput {
                            view: input.view,
                            screen: input.screen,
                            screen_radius: captured.radius,
                            pressure: input.raw.pressure,
                            radius_factor: captured.radius_factor,
                        },
                    )
                })
                .is_ok_and(|p| p.is_some())
        });
        let ready = Readiness {
            key: (Some(key.subtool_id) == self.document.active_subtool_id()).then_some(key),
            mode_enabled: self.preferences.global().continuous,
            primary_down: down,
            pressure: if input.is_some_and(|p| p.in_contact) {
                1.0
            } else {
                0.0
            },
            focused,
            navigating: ctx.input(|i| viewport_navigation_requested(i, self.navigation_down())),
            target_visible: self.document.active_subtool_visible(),
            pointer_in_viewport: position.is_some(),
            surface_hit,
            queue_empty: self.pending_dabs.is_empty(),
            operation_busy: self.topology_job.is_some(),
        };
        let report = self.continuous_clock.report();
        if report.active_key.is_none()
            && matches!(
                report.last_stop,
                Some(StopReason::NoContact | StopReason::TargetLost)
            )
            && ready.primary_down
            && ready.focused
            && !ready.navigating
            && ready.target_visible
            && ready.surface_hit
            && ready.queue_empty
            && ready.pressure > 0.
            && let Err(error) = self.continuous_clock.start(key, true, false, now)
        {
            self.message = error.to_string();
            return;
        }
        if let Some(permit) = self
            .continuous_clock
            .poll(ctx.cumulative_frame_nr(), now, ready)
        {
            if let Err(error) =
                self.continuous_clock
                    .consume(permit, ctx.cumulative_frame_nr(), ready)
            {
                self.stop_continuous(StopReason::Command);
                self.finish_stroke_deferred();
                self.message = error.to_string();
                return;
            }
            if let Some(input) = input {
                self.last_continuous_tick = Some(permit.at_micros());
                self.pending_dabs.source_geometry_dab(session, input);
                let progress = self.pending_dabs.process(&mut self.document, 1, None);
                self.handle_dab_progress(progress);
                debug_assert!(self.pending_dabs.is_empty());
            }
        }
        let report = self.continuous_clock.report();
        if report.active_key.is_none()
            && !matches!(
                report.last_stop,
                Some(StopReason::NoContact | StopReason::TargetLost)
            )
        {
            self.finish_stroke_deferred();
            return;
        }
        if let Some(delay) = self.continuous_clock.micros_until_due(now) {
            ctx.request_repaint_after(std::time::Duration::from_micros(delay.max(1)));
        }
    }
}

impl OBrushApp {
    fn source_geometry_alpha_panel(&mut self, ui: &mut egui::Ui) {
        let mut mirror = self.brush.symmetry_x;
        let r = ui.checkbox(&mut mirror, "X mirror symmetry");
        self.track_source_geometry_control("source_geometry.mirror", &r);
        self.track_morph_control("settings.symmetry", &r);
        if r.changed() {
            self.brush.symmetry_x = mirror;
            self.commit_brush_view();
        }
        let mut selection = self.preferences.tool(self.preferences.selected()).alpha;
        let label = alpha_selection_name(&self.preferences, selection).unwrap_or("None".into());
        let label = crate::ui_text::alpha_selected_name(selection, &label);
        let entries: Vec<_> = self
            .preferences
            .library()
            .entries()
            .iter()
            .map(|e| (e.id, e.name.clone()))
            .collect();
        let selector = egui::ComboBox::from_id_salt("source_geometry_alpha_library")
            .selected_text(label.clone())
            .width(ui.available_width())
            .truncate()
            .show_ui(ui, |ui| {
                let none = ui.selectable_value(
                    &mut selection,
                    AlphaSelection::None,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::C98589F40BA48),
                );
                self.track_source_geometry_control("alpha.option.none", &none);
                let square = ui.selectable_value(
                    &mut selection,
                    AlphaSelection::Square,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CA0EA42396A95),
                );
                self.track_source_geometry_control("alpha.option.square", &square);
                let skin = ui.selectable_value(
                    &mut selection,
                    AlphaSelection::Skin,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CA7F63B73C877),
                );
                self.track_source_geometry_control("alpha.option.skin", &skin);
                for (id, name) in entries {
                    let option =
                        ui.selectable_value(&mut selection, AlphaSelection::Imported(id), name);
                    self.track_source_geometry_control(
                        &format!("alpha.option.imported.{id}"),
                        &option,
                    );
                }
            });
        let selector_response = selector.response.on_hover_text(label);
        self.track_source_geometry_control("alpha.library", &selector_response);
        if selection != self.preferences.tool(self.preferences.selected()).alpha {
            self.select_alpha(selection);
        }
        let conversion = egui::ComboBox::from_id_salt("source_geometry_alpha_conversion")
            .selected_text(match self.alpha_conversion {
                AlphaConversion::LegacyWeightedOpacity => {
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CD3053E29DD1F)
                }
                AlphaConversion::SculptGlMeanRgb => {
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CDC59BA10F835)
                }
            })
            .show_ui(ui, |ui| {
                let weighted = ui.selectable_value(
                    &mut self.alpha_conversion,
                    AlphaConversion::LegacyWeightedOpacity,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CD3053E29DD1F),
                );
                self.track_source_geometry_control("alpha.convert.weighted_opacity", &weighted);
                let mean = ui.selectable_value(
                    &mut self.alpha_conversion,
                    AlphaConversion::SculptGlMeanRgb,
                    crate::ui_text::caption(crate::ui_text::CaptionKey::CDC59BA10F835),
                );
                self.track_source_geometry_control("alpha.convert.mean_rgb", &mean);
            });
        self.track_source_geometry_control("alpha.conversion", &conversion.response);
        ui.horizontal(|ui| {
            let import = ui
                .button(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::C1BE6D2F01842,
                ))
                .on_hover_text(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CF4FB2B02B8FF,
                ));
            self.track_source_geometry_control("alpha.import", &import);
            if import.clicked() {
                self.import_alpha();
            }
            let clear = ui.add_enabled(
                self.brush.alpha.is_some(),
                egui::Button::new(crate::ui_text::caption(
                    crate::ui_text::CaptionKey::CFF7BC7F66552,
                )),
            );
            self.track_source_geometry_control("alpha.clear", &clear);
            if clear.clicked() {
                self.select_alpha(AlphaSelection::None);
            }
        });
        let remove = ui.add_enabled(
            matches!(
                self.preferences.tool(self.preferences.selected()).alpha,
                AlphaSelection::Imported(_)
            ),
            egui::Button::new(crate::ui_text::caption(
                crate::ui_text::CaptionKey::C56AB4FE19F89,
            )),
        );
        self.track_source_geometry_control("alpha.remove", &remove);
        if remove.clicked() {
            self.remove_selected_alpha();
        }

        let reset = ui.add_enabled(
            self.brush.alpha_rotation != 0.0,
            egui::Button::new("Clear unsupported manual alpha rotation"),
        );
        self.track_source_geometry_control("source_geometry.alpha.rotation.reset", &reset);
        if reset.clicked() {
            self.brush.alpha_rotation = 0.0;
            self.commit_brush_view();
        }
        ui.label(
            RichText::new(
                "Alpha follows the stroke direction. Square and Skin use OBrush presets.",
            )
            .size(10.0)
            .color(MUTED),
        );
    }
}

#[cfg(test)]
#[path = "source_pinch_tests.rs"]
mod source_pinch_tests;

#[cfg(test)]
#[path = "source_smooth_host_tests.rs"]
mod smooth_tests;

#[cfg(test)]
#[path = "source_flatten_host_tests.rs"]
mod flatten_tests;

#[cfg(test)]
#[path = "source_crease_host_tests.rs"]
mod crease_tests;

#[cfg(test)]
#[path = "source_history_host_tests.rs"]
mod history_tests;

#[cfg(test)]
mod source_symmetry_offset_tests {
    use super::*;

    #[test]
    fn active_source_object_overrides_transient_tool_offset_without_mutating_it() {
        let mut document = Document::empty();
        document
            .replace_sgl_bytes(
                include_bytes!("../../obrush-core/tests/fixtures/independent-affine-two-mesh.sgl"),
                &obrush_core::SglStageSettings::default(),
            )
            .unwrap();
        let tool = SourceGeometryPreferences {
            mirror_offset: -0.75,
            ..Default::default()
        };
        assert_eq!(
            preferences_for_active_object(&document, tool).mirror_offset,
            0.0
        );
        document.set_active_source_symmetry_offset(0.25).unwrap();
        assert_eq!(
            preferences_for_active_object(&document, tool).mirror_offset,
            0.25
        );
        assert_eq!(tool.mirror_offset, -0.75);
        document.select_subtools(&[1], Some(1)).unwrap();
        assert_eq!(
            preferences_for_active_object(&document, tool).mirror_offset,
            0.0
        );
        document.select_subtools(&[2], Some(2)).unwrap();
        assert_eq!(
            preferences_for_active_object(&document, tool).mirror_offset,
            0.25
        );
    }
}
