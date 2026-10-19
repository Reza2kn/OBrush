//! Captured ordinary Source geometry session shared by queued/direct/timer
//! callers. Normal UI/renderer wiring is a separately preregistered cohort.
//! Precise input uses the current ordinary OBrush camera/affine policy; the
//! source kernel label is not a claim of V8 picking/camera/optimizer equality.
use crate::renderer::Camera;
use crate::source_paint_host::{RawPaintInput, SourcePaintInput};
use eframe::egui::{Pos2, Rect};
use obrush_core::{
    BrushKind, BrushSettings, Document, PaintPreparationInput, SourceGeometryError,
    SourceGeometryNoop, SourceGeometryOptions, SourceGeometryPass, SourceGeometryReport,
    SourceGeometryStroke, SourceGeometryTool,
};
use std::sync::{Arc, Mutex};
use web_time::Instant;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SourceGeometryStageTime {
    pub preparation: std::time::Duration,
    pub retained_match: std::time::Duration,
    pub contact_prepare: std::time::Duration,
    pub deformation: std::time::Duration,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceGeometryMode {
    #[default]
    Legacy,
    SourceOrdinary,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceGeometryPreferences {
    pub mode: SourceGeometryMode,
    pub radius: f64,
    pub radius_factor: f64,
    pub options: SourceGeometryOptions,
    pub mirror_offset: f64,
}
impl Default for SourceGeometryPreferences {
    fn default() -> Self {
        Self {
            mode: SourceGeometryMode::Legacy,
            radius: 50.,
            radius_factor: 0.75,
            options: Default::default(),
            mirror_offset: 0.0,
        }
    }
}
impl SourceGeometryPreferences {
    pub fn inflate() -> Self {
        Self {
            options: SourceGeometryOptions::inflate(),
            ..Self::default()
        }
    }
    pub fn pinch() -> Self {
        Self {
            options: SourceGeometryOptions::pinch(),
            ..Self::default()
        }
    }
    pub fn smooth() -> Self {
        Self {
            options: SourceGeometryOptions::smooth(),
            ..Self::default()
        }
    }
    pub fn flatten() -> Self {
        Self {
            options: SourceGeometryOptions::flatten(),
            ..Self::default()
        }
    }
    pub fn crease() -> Self {
        Self {
            radius: 25.0,
            options: SourceGeometryOptions::crease(),
            ..Self::default()
        }
    }
    pub fn validate(self) -> Result<(), SourceGeometryError> {
        self.options.validate()?;
        if !self.radius.is_finite()
            || !(5.0..=500.0).contains(&self.radius)
            || !self.radius_factor.is_finite()
            || !(0.0..=1.0).contains(&self.radius_factor)
            || !self.mirror_offset.is_finite()
            || !(-1.0..=1.0).contains(&self.mirror_offset)
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        Ok(())
    }
}
impl SourceGeometryPreferences {
    pub fn diagnostics(self) -> serde_json::Value {
        serde_json::json!({"mode":match self.mode {SourceGeometryMode::Legacy=>"Legacy",SourceGeometryMode::SourceOrdinary=>"SourceOrdinary"},
            "radius":self.radius,"radius_factor":self.radius_factor,"options":self.options,"mirror_offset":self.mirror_offset})
    }
}

/// Rendering ownership is independent of idle tool preferences: switching
/// tools does not rewrite the retained geometric normal producer.
pub(crate) enum SourceNormalRenderOwner {
    Idle,
    Geometry,
    CapturedPaint(Option<(u32, Arc<obrush_core::SourceGeometryNormalState>)>),
    Legacy,
}

pub(crate) fn normal_render_states(
    document: &Document,
    visible_ids: impl Iterator<Item = u32>,
    owner: SourceNormalRenderOwner,
) -> Vec<(u32, Arc<obrush_core::SourceGeometryNormalState>)> {
    match owner {
        SourceNormalRenderOwner::Idle | SourceNormalRenderOwner::Geometry => visible_ids
            .filter_map(|id| document.source_geometry_state(id).map(|state| (id, state)))
            .collect(),
        SourceNormalRenderOwner::CapturedPaint(state) => state.into_iter().collect(),
        SourceNormalRenderOwner::Legacy => Vec::new(),
    }
}

pub type SourceGeometryInput = SourcePaintInput;
pub type SharedSourceGeometry = Arc<Mutex<SourceGeometrySession>>;
#[derive(Default)]
pub struct SourceGeometryAlphaState {
    tracks: obrush_core::SourceGeometryAlphaTracks,
    owner: Option<u64>,
    paint_owner: Option<Arc<()>>,
}
impl SourceGeometryAlphaState {
    pub fn snapshot(&self) -> obrush_core::SourceGeometryAlphaTracks {
        self.tracks
    }
    pub fn active_claim(&self) -> Option<u64> {
        self.owner
    }
}
/// A Paint lease shares the actual Picking-like state with geometry while
/// remaining distinct from the document's geometry stroke sequence.
/// The Arc token prevents an expired Paint session from releasing another owner.
pub(crate) struct SourcePaintAlphaLease {
    shared: SharedSourceGeometryAlphaTracks,
    token: Arc<()>,
    before: obrush_core::SourceGeometryAlphaTracks,
    expected: obrush_core::SourceGeometryAlphaTracks,
    closed: bool,
}
impl SourcePaintAlphaLease {
    pub(crate) fn begin(shared: SharedSourceGeometryAlphaTracks) -> Result<Self, String> {
        let mut state = shared.lock().map_err(|_| "Source alpha state poisoned")?;
        if state.owner.is_some() || state.paint_owner.is_some() {
            return Err("Source alpha state already belongs to an active stroke".into());
        }
        state.tracks.validate().map_err(|e| e.to_string())?;
        let token = Arc::new(());
        let before = state.tracks;
        state.paint_owner = Some(token.clone());
        drop(state);
        Ok(Self {
            shared,
            token,
            before,
            expected: before,
            closed: false,
        })
    }
    pub(crate) fn tracks(&self) -> Result<obrush_core::SourceGeometryAlphaTracks, String> {
        let state = self
            .shared
            .lock()
            .map_err(|_| "Source alpha state poisoned")?;
        self.validate_owner(&state)?;
        Ok(state.tracks)
    }
    fn validate_owner(&self, state: &SourceGeometryAlphaState) -> Result<(), String> {
        if self.closed
            || state.owner.is_some()
            || !state
                .paint_owner
                .as_ref()
                .is_some_and(|t| Arc::ptr_eq(t, &self.token))
            || !same_tracks(state.tracks, self.expected)
        {
            return Err("Source Paint alpha lease or captured frame changed".into());
        }
        Ok(())
    }
    pub(crate) fn commit<T>(
        &mut self,
        next: obrush_core::SourceGeometryAlphaTracks,
        apply: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        next.validate().map_err(|e| e.to_string())?;
        let mut state = self
            .shared
            .lock()
            .map_err(|_| "Source alpha state poisoned")?;
        self.validate_owner(&state)?;
        let result = apply()?;
        state.tracks = next;
        self.expected = next;
        Ok(result)
    }
    pub(crate) fn publish(
        &mut self,
        next: obrush_core::SourceGeometryAlphaTracks,
    ) -> Result<(), String> {
        self.commit(next, || Ok(()))
    }
    pub(crate) fn finish(&mut self, restore: bool) {
        if self.closed {
            return;
        }
        if let Ok(mut state) = self.shared.lock()
            && state
                .paint_owner
                .as_ref()
                .is_some_and(|t| Arc::ptr_eq(t, &self.token))
        {
            if restore {
                state.tracks = self.before;
            }
            state.paint_owner = None;
        }
        self.closed = true;
    }
}
impl Drop for SourcePaintAlphaLease {
    fn drop(&mut self) {
        self.finish(true);
    }
}

pub type SharedSourceGeometryAlphaTracks = Arc<Mutex<SourceGeometryAlphaState>>;

pub fn capture(
    camera: &Camera,
    pos: Pos2,
    rect: Rect,
    raw: &RawPaintInput,
    zoom: f32,
) -> Result<SourceGeometryInput, SourceGeometryError> {
    crate::source_paint_host::capture_at_zoom(camera, pos, rect, raw, zoom)
        .map_err(|_| SourceGeometryError::InvalidInput)
}
pub fn blocker(
    doc: &Document,
    settings: &BrushSettings,
    locked: bool,
    prefs: SourceGeometryPreferences,
) -> Result<(), SourceGeometryError> {
    prefs.validate()?;
    match (settings.kind, prefs.options.tool) {
        (BrushKind::Draw | BrushKind::Clay, SourceGeometryTool::Brush { .. })
        | (BrushKind::Inflate, SourceGeometryTool::Inflate)
        | (BrushKind::Smooth, SourceGeometryTool::Smooth { .. }) => {}
        _ => return Err(SourceGeometryError::UnsupportedDomain),
    }
    if settings.alpha.is_some()
        || settings.alpha_rotation != 0.
        || settings.symmetry_x
        || settings.symmetry_offset != 0.
        || locked
        || settings.adaptive_detail
    {
        return Err(SourceGeometryError::UnsupportedDomain);
    }
    // Negative uses Source options, not a silently inherited legacy invert
    // value. An unexpected live modifier must end/restart the captured stroke.
    if settings.invert {
        return Err(
            if matches!(prefs.options.tool, SourceGeometryTool::Smooth { .. }) {
                SourceGeometryError::UnsupportedSmoothNegative
            } else {
                SourceGeometryError::UnsupportedDomain
            },
        );
    }
    doc.source_geometry_blocker(prefs.options)
}
fn validate_input(input: SourceGeometryInput) -> Result<(), SourceGeometryError> {
    input
        .view
        .validate()
        .map_err(|_| SourceGeometryError::InvalidInput)?;
    if input.screen.iter().any(|v| !v.is_finite())
        || !input.raw.pressure.is_finite()
        || !(0.0..=1.0).contains(&input.raw.pressure)
        || input.in_contact != (!input.raw.pen || input.raw.pressure > 0.)
    {
        return Err(SourceGeometryError::InvalidInput);
    }
    Ok(())
}

const MAX_DAB_OBSERVATIONS: usize = 64;

struct DabOutcome {
    selected_vertices: usize,
    front_vertices: usize,
    changed_vertices: usize,
    refreshed_normals: usize,
    displacement_scale: f32,
    noop: Option<SourceGeometryNoop>,
}
impl DabOutcome {
    fn from_report(report: &SourceGeometryReport) -> Self {
        Self {
            selected_vertices: report.selected_vertices,
            front_vertices: report.front_vertices,
            changed_vertices: report.changed_vertices,
            refreshed_normals: report.source_normal_rows_refreshed,
            displacement_scale: report.displacement_scale,
            noop: report.noop,
        }
    }
    fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"selected_vertices":self.selected_vertices,"front_vertices":self.front_vertices,
            "changed_vertices":self.changed_vertices,"source_normal_rows_refreshed":self.refreshed_normals,
            "displacement_scale":self.displacement_scale,"noop":self.noop})
    }
}
struct DabObservation {
    ordinal: u64,
    input: SourceGeometryInput,
    passes: [Option<SourceGeometryPass>; 2],
    outcomes: [Option<DabOutcome>; 2],
    error: Option<SourceGeometryError>,
}
#[derive(Default)]
struct DabObservations {
    total: u64,
    rows: Vec<DabObservation>,
    retention_failure: Option<&'static str>,
}
impl DabObservations {
    fn retain(&mut self, mut row: DabObservation) {
        self.total = self.total.saturating_add(1);
        if self.rows.len() >= MAX_DAB_OBSERVATIONS {
            self.retention_failure.get_or_insert("Capacity");
            return;
        }
        if self.rows.try_reserve(1).is_err() {
            self.retention_failure.get_or_insert("Allocation");
            return;
        }
        row.ordinal = self.total;
        self.rows.push(row);
    }
    fn diagnostics(&self) -> serde_json::Value {
        let rows: Vec<_> = self.rows.iter().map(|r| serde_json::json!({
            "ordinal":r.ordinal,"screen64":r.input.screen,
            "screen_bits64":r.input.screen.map(|v|format!("{:016x}",v.to_bits())),
            "pressure64":r.input.raw.pressure,"pressure_bits64":format!("{:016x}",r.input.raw.pressure.to_bits()),
            "in_contact":r.input.in_contact,"view64":r.input.view.world_to_clip,"viewport64":r.input.view.viewport,
            "passes":r.passes,"outcomes":r.outcomes.each_ref().map(|r|r.as_ref().map(DabOutcome::diagnostics)),"error":r.error,
        })).collect();
        serde_json::json!({"limit":MAX_DAB_OBSERVATIONS,"total_calls":self.total,"complete":self.retention_failure.is_none()&&self.rows.len() as u64==self.total,
            "retention_failure":self.retention_failure,"retained_bytes":self.rows.capacity()*std::mem::size_of::<DabObservation>(),
            "rows":rows,"scope":"Actual admitted apply calls in order, primary before mirror; bounded prefix, explicit incomplete after capacity/allocation. Diagnostics do not stop/alter sculpting. Lifecycle calls rejected before apply are not dabs."})
    }
}

pub struct SourceGeometrySession {
    preferences: SourceGeometryPreferences,
    subtool: u32,
    level: usize,
    pose: Option<[f32; 16]>,
    stroke: SourceGeometryStroke,
    last_screen: [f64; 2],
    finished: bool,
    pub failed: Option<SourceGeometryError>,
    pub last_report: Option<SourceGeometryReport>,
    pub last_input: Option<SourceGeometryInput>,
    pub last_pass: Option<SourceGeometryPass>,
    pub last_triangle: Option<usize>,
    controls: Option<CapturedGeometryControls>,
    pub last_controlled: Option<obrush_core::SourceGeometryControlledReport>,
    pub last_mirror_triangle: Option<usize>,
    pub last_mirror_plane: Option<obrush_core::SourceMirrorPlane>,
    dab_observations: DabObservations,
    last_stage_time: Option<SourceGeometryStageTime>,
    last_render_snapshot: Option<obrush_core::CertifiedSourceRenderSnapshot>,
    last_release_timing: Option<obrush_core::SourceGeometryReleaseTiming>,
    locked_press: Option<SourceGeometryInput>,
    locked_press_tracks: Option<obrush_core::SourceGeometryAlphaTracks>,
    last_locked_preview: Option<[f64; 3]>,
}
impl SourceGeometrySession {
    pub fn begin(
        doc: &mut Document,
        settings: &BrushSettings,
        locked: bool,
        prefs: SourceGeometryPreferences,
        input: SourceGeometryInput,
    ) -> Result<SharedSourceGeometry, SourceGeometryError> {
        prefs.validate()?;
        validate_input(input)?;
        if prefs.mode != SourceGeometryMode::SourceOrdinary {
            return Err(SourceGeometryError::UnsupportedDomain);
        }
        blocker(doc, settings, locked, prefs)?;
        Self::begin_admitted(doc, prefs, input, false)
    }
    fn begin_admitted(
        doc: &mut Document,
        prefs: SourceGeometryPreferences,
        input: SourceGeometryInput,
        selection_history: bool,
    ) -> Result<SharedSourceGeometry, SourceGeometryError> {
        prefs.validate()?;
        validate_input(input)?;
        if prefs.mode != SourceGeometryMode::SourceOrdinary {
            return Err(SourceGeometryError::UnsupportedDomain);
        }
        let mesh = doc
            .active_mesh()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        let hit = obrush_core::prepare_source_paint_hit(
            mesh,
            doc.active_object_metadata().map(|m| &m.affine),
            input
                .view
                .ray(input.screen)
                .map_err(|_| SourceGeometryError::InvalidInput)?,
        )
        .map_err(|_| SourceGeometryError::InvalidInput)?;
        if hit.is_none() {
            return Err(SourceGeometryError::MissingOriginalFace);
        }
        let subtool = doc
            .active_subtool_id()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        let level = doc.current_level();
        let pose = doc
            .active_object_metadata()
            .map(|m| m.affine.local_to_world);
        let stroke = if selection_history {
            doc.begin_source_geometry_selection_history(prefs.options)?
        } else {
            doc.begin_source_geometry_stroke(prefs.options)?
        };
        Ok(Arc::new(Mutex::new(Self {
            preferences: prefs,
            subtool,
            level,
            pose,
            stroke,
            last_screen: input.screen,
            finished: false,
            failed: None,
            last_report: None,
            last_input: None,
            last_pass: None,
            last_triangle: None,
            controls: None,
            last_controlled: None,
            last_mirror_triangle: None,
            last_mirror_plane: None,
            dab_observations: DabObservations::default(),
            last_stage_time: None,
            last_render_snapshot: None,
            last_release_timing: None,
            locked_press: None,
            locked_press_tracks: None,
            last_locked_preview: None,
        })))
    }
    pub fn preferences(&self) -> SourceGeometryPreferences {
        self.preferences
    }
    pub fn path_samples(
        &mut self,
        input: SourceGeometryInput,
    ) -> Result<Vec<SourceGeometryInput>, SourceGeometryError> {
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        if self.finished {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        validate_input(input)?;
        if let Some(press) = self.locked_press {
            let dx = input.screen[0] - press.screen[0];
            let dy = input.screen[1] - press.screen[1];
            if !dx.is_finite() || !dy.is_finite() || (dx * dx + dy * dy).sqrt() > 65_536. {
                return Err(SourceGeometryError::InvalidInput);
            }
            self.last_screen = input.screen;
            return Ok(vec![input]);
        }
        let dx = input.screen[0] - self.last_screen[0];
        let dy = input.screen[1] - self.last_screen[1];
        let distance = (dx * dx + dy * dy).sqrt();
        let spacing = 0.15 * self.preferences.radius;
        if !distance.is_finite() || distance > 65_536. {
            return Err(SourceGeometryError::InvalidInput);
        }
        if distance <= spacing {
            return Ok(Vec::new());
        }
        let count = (distance / spacing).floor() as usize;
        let start = count.saturating_sub(32);
        let mut out = Vec::new();
        out.try_reserve_exact(count.min(32))
            .map_err(|_| SourceGeometryError::Allocation)?;
        for i in start + 1..=count {
            let t = i as f64 / count as f64;
            let mut sample = input;
            sample.screen = [self.last_screen[0] + dx * t, self.last_screen[1] + dy * t];
            out.push(sample);
        }
        self.last_screen = input.screen;
        Ok(out)
    }
    pub fn apply(
        &mut self,
        doc: &mut Document,
        input: SourceGeometryInput,
    ) -> Result<SourceGeometryReport, SourceGeometryError> {
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        if self.finished {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        if self.controls.is_some() {
            let result = if self.locked_press.is_some() {
                let result = self.apply_locked(doc, input);
                if let Err(error) = &result {
                    if doc.source_geometry_active_claim() == Some(self.stroke.claim_id()) {
                        let _ = doc.cancel_source_geometry_stroke(&self.stroke);
                    }
                    self.failed = Some(error.clone());
                }
                result
            } else {
                self.apply_controlled(doc, input)
            };
            self.observe_dab(input, &result);
            return result;
        }
        let result = (|| {
            validate_input(input)?;
            if Some(self.subtool) != doc.active_subtool_id()
                || self.level != doc.current_level()
                || !doc.active_subtool_visible()
                || self.pose.map(|m| m.map(f32::to_bits))
                    != doc
                        .active_object_metadata()
                        .map(|m| m.affine.local_to_world.map(f32::to_bits))
            {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
            if !input.in_contact {
                return Ok(self.record_noop(input, SourceGeometryNoop::NoContact));
            }
            let prepared = obrush_core::prepare_source_paint_pass(
                doc.active_mesh()
                    .ok_or(SourceGeometryError::UnsupportedDomain)?,
                doc.active_object_metadata().map(|m| &m.affine),
                PaintPreparationInput {
                    view: input.view,
                    screen: input.screen,
                    screen_radius: self.preferences.radius,
                    pressure: input.raw.pressure,
                    radius_factor: self.preferences.radius_factor,
                },
            )
            .map_err(|_| SourceGeometryError::InvalidInput)?;
            let Some((pass, hit)) = prepared else {
                return Ok(self.record_noop(input, SourceGeometryNoop::EmptySelection));
            };
            let geometry = SourceGeometryPass {
                center: pass.center,
                eye_direction: pass.eye_direction,
                local_radius_squared: pass.local_radius_squared,
                original_face_index: hit.original_face_index.unwrap_or(hit.triangle_index),
            };
            let report = doc.apply_source_geometry_dab(
                &mut self.stroke,
                geometry,
                input.raw.pressure,
                input.in_contact,
            )?;
            self.last_report = Some(report.clone());
            self.last_input = Some(input);
            self.last_pass = Some(geometry);
            self.last_triangle = Some(hit.triangle_index);
            Ok(report)
        })();
        if let Err(e) = &result {
            self.failed = Some(e.clone());
        }
        self.observe_dab(input, &result);
        result
    }
    fn observe_dab(
        &mut self,
        input: SourceGeometryInput,
        result: &Result<SourceGeometryReport, SourceGeometryError>,
    ) {
        let (passes, outcomes, error) = match result {
            Ok(report) => match self.last_controlled.as_ref() {
                Some(controlled) => (
                    controlled.contacts,
                    controlled
                        .passes
                        .each_ref()
                        .map(|p| p.as_ref().map(DabOutcome::from_report)),
                    None,
                ),
                None => (
                    [self.last_pass, None],
                    [Some(DabOutcome::from_report(report)), None],
                    None,
                ),
            },
            Err(error) => ([None, None], [None, None], Some(error.clone())),
        };
        self.dab_observations.retain(DabObservation {
            ordinal: 0,
            input,
            passes,
            outcomes,
            error,
        });
    }
    fn record_noop(
        &mut self,
        input: SourceGeometryInput,
        why: SourceGeometryNoop,
    ) -> SourceGeometryReport {
        let report = SourceGeometryReport::noop(why);
        self.last_report = Some(report.clone());
        self.last_input = Some(input);
        self.last_pass = None;
        self.last_triangle = None;
        self.last_mirror_triangle = None;
        self.last_mirror_plane = None;
        self.last_controlled = None;
        report
    }
    pub fn finish(&mut self, doc: &mut Document) -> Result<(), SourceGeometryError> {
        if self.finished {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        if self.controls.is_some()
            && self.failed.is_none()
            && doc.source_geometry_active_claim() != Some(self.stroke.claim_id())
        {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        // Rejected input cannot leave partially accepted earlier dabs as a
        // falsely completed Source workflow. Exact cancellation retains redo.
        let result = if self.failed.is_some() {
            if doc.source_geometry_active_claim() == Some(self.stroke.claim_id()) {
                doc.cancel_source_geometry_stroke(&self.stroke)
            } else {
                Ok(())
            }
        } else {
            match doc.end_source_geometry_stroke_timed(&self.stroke) {
                Ok(timing) => {
                    self.last_release_timing = Some(timing);
                    Ok(())
                }
                Err(error) => Err(error),
            }
        };
        self.release_tracks(result.is_err() || self.failed.is_some());
        self.finished = true;
        result
    }
    pub fn cancel(&mut self, doc: &mut Document) -> Result<(), SourceGeometryError> {
        if self.finished {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        if self.controls.is_some()
            && doc.source_geometry_active_claim() != Some(self.stroke.claim_id())
        {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        let result = doc.cancel_source_geometry_stroke(&self.stroke);
        if result.is_ok() {
            self.release_tracks(true);
        }
        self.finished = true;
        result
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"part":self.subtool,"level":self.level,"captured":self.preferences.diagnostics(),
            "pose32":self.pose,"finished":self.finished,"failed":self.failed,
            "last_report":self.last_report,"last_pass":self.last_pass,"triangle":self.last_triangle,"dab_observations":self.dab_observations.diagnostics(),"release_timing":self.last_release_timing,
            "controlled":self.last_controlled,"mirror_triangle":self.last_mirror_triangle,"mirror_plane":self.last_mirror_plane,"captured_controls":self.controls.as_ref().map(CapturedGeometryControls::diagnostics),
            "locked_press":self.locked_press.map(|i|i.screen),"locked_preview":self.last_locked_preview,
            "input":self.last_input.map(|i|serde_json::json!({"screen64":i.screen,
                "pressure64":i.raw.pressure,"pen":i.raw.pen,"absent":i.raw.absent,
                "browser_number":i.raw.browser_number,"in_contact":i.in_contact,
                "view64":i.view.world_to_clip,"viewport64":i.view.viewport})),
            "scope":if self.locked_press.is_some(){"Source Brush Lock absolute preview on static U; alpha origin retained, Source brush response; C276 integer-coordinate pair passed, fractional coordinates and broader Lock behavior remain open"}else if self.controls.is_some(){"ordinary static U; captured nearest alpha/local X mirror; two pre-dab contacts then sequential passes; no Lock/manual rotation/dynamic/hierarchy; Source camera/hypot/ray candidate order and cross-Legacy tracks unpaired"}else{"old ordinary U/static/alphaNone/no symmetry/no Lock; bounded ordered static selection or explicit singleleaf<=100 compatibility; supplied faces/order and precise OBrush camera not paired Source equality"}})
    }
}
pub fn apply_shared(
    doc: &mut Document,
    session: &SharedSourceGeometry,
    input: SourceGeometryInput,
) -> Result<SourceGeometryReport, SourceGeometryError> {
    session
        .lock()
        .map_err(|_| SourceGeometryError::StrokeBindingChanged)?
        .apply(doc, input)
}

pub(crate) fn apply_shared_with_timing(
    doc: &mut Document,
    session: &SharedSourceGeometry,
    input: SourceGeometryInput,
) -> (
    Result<SourceGeometryReport, SourceGeometryError>,
    Option<SourceGeometryStageTime>,
    Option<obrush_core::CertifiedSourceRenderSnapshot>,
) {
    match session.lock() {
        Ok(mut session) => {
            let result = session.apply(doc, input);
            (
                result,
                session.last_stage_time,
                session.last_render_snapshot.take(),
            )
        }
        Err(_) => (Err(SourceGeometryError::StrokeBindingChanged), None, None),
    }
}

struct CapturedGeometryControls {
    options: obrush_core::SourceGeometryControls,
    center: [f32; 3],
    tracks: SharedSourceGeometryAlphaTracks,
    before: obrush_core::SourceGeometryAlphaTracks,
    expected: obrush_core::SourceGeometryAlphaTracks,
    claim: u64,
    alpha_checksum: Option<u64>,
}
impl CapturedGeometryControls {
    fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"mirror":self.options.mirror,"center32":self.center,
            "alpha":self.options.alpha.as_ref().map(|a|serde_json::json!({
                "dimensions":[a.width(),a.height()],"bytes":a.pixels().len(),"has_signal":a.has_signal(),
                "byte_checksum_fnv1a64_hex":self.alpha_checksum.map(|sum|format!("{sum:016x}")),
            })),"tracks":self.expected,"claim":self.claim,"scope":"admitted Source geometry only; global primary/mirror tracks across Legacy/other-tool events unpaired; nearest alpha/live coordinates; static two-prep primary-before-mirror; precise OBrush picking/hypot policy"})
    }
}
pub fn blocker_controls(
    doc: &Document,
    settings: &BrushSettings,
    locked: bool,
    prefs: SourceGeometryPreferences,
) -> Result<(), SourceGeometryError> {
    prefs.validate()?;
    if settings.alpha_rotation != 0.0 || settings.symmetry_offset != 0.0 || settings.adaptive_detail
    {
        return Err(SourceGeometryError::UnsupportedDomain);
    }
    if locked
        && (!matches!(
            prefs.options.tool,
            SourceGeometryTool::Brush { clay: false, .. }
        ) || settings.kind != BrushKind::Draw
            || settings.symmetry_x)
    {
        return Err(SourceGeometryError::UnsupportedDomain);
    }
    if settings.invert {
        return Err(
            if settings.kind == BrushKind::Smooth
                && matches!(prefs.options.tool, SourceGeometryTool::Smooth { .. })
            {
                SourceGeometryError::UnsupportedSmoothNegative
            } else {
                SourceGeometryError::UnsupportedDomain
            },
        );
    }
    // Old constructor/API keeps its original refusal contract. The new route
    // explicitly owns captured alpha/mirror, rather than silently ignoring them.
    let mut plain = settings.clone();
    plain.alpha = None;
    plain.symmetry_x = false;
    match (plain.kind, prefs.options.tool) {
        (BrushKind::Pinch, SourceGeometryTool::Pinch)
        | (BrushKind::Flatten, SourceGeometryTool::Flatten)
        | (BrushKind::Crease, SourceGeometryTool::Crease) => {
            doc.source_geometry_blocker(prefs.options)
        }
        _ => blocker(doc, &plain, false, prefs),
    }
}
impl SourceGeometrySession {
    pub fn begin_with_controls(
        doc: &mut Document,
        settings: &BrushSettings,
        locked: bool,
        prefs: SourceGeometryPreferences,
        input: SourceGeometryInput,
        tracks: SharedSourceGeometryAlphaTracks,
    ) -> Result<SharedSourceGeometry, SourceGeometryError> {
        blocker_controls(doc, settings, locked, prefs)?;
        validate_input(input)?;
        let center = doc.source_geometry_interaction_center()?;
        let before = {
            let current = tracks
                .lock()
                .map_err(|_| SourceGeometryError::StrokeBindingChanged)?;
            if current.owner.is_some() || current.paint_owner.is_some() {
                return Err(SourceGeometryError::StrokeAlreadyOpen);
            }
            current.tracks
        };
        before.validate()?;
        let controls = obrush_core::SourceGeometryControls {
            alpha: settings.alpha.clone(),
            mirror: settings.symmetry_x,
        };
        let shared = Self::begin_admitted(doc, prefs, input, true)?;
        let result = (|| {
            let mut session = shared
                .lock()
                .map_err(|_| SourceGeometryError::StrokeBindingChanged)?;
            let plane = if controls.mirror {
                Some(doc.source_geometry_mirror_plane(
                    center,
                    prefs.mirror_offset,
                    [1.0, 0.0, 0.0],
                )?)
            } else {
                None
            };
            let contacts = obrush_core::prepare_source_geometry_press_contacts(
                doc.active_mesh()
                    .ok_or(SourceGeometryError::UnsupportedDomain)?,
                doc.active_object_metadata().map(|m| &m.affine),
                PaintPreparationInput {
                    view: input.view,
                    screen: input.screen,
                    screen_radius: prefs.radius,
                    pressure: input.raw.pressure,
                    radius_factor: prefs.radius_factor,
                },
                plane,
            )?;
            let initialized =
                doc.prepare_source_geometry_press_alpha(&session.stroke, contacts.passes, before)?;
            if locked {
                let mut press_tracks = initialized;
                press_tracks.primary.keep_origin = true;
                session.locked_press = Some(input);
                session.locked_press_tracks = Some(press_tracks);
            }
            let mut current = tracks
                .lock()
                .map_err(|_| SourceGeometryError::StrokeBindingChanged)?;
            if current.owner.is_some()
                || current.paint_owner.is_some()
                || !same_tracks(current.tracks, before)
            {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
            let claim = session.stroke.claim_id();
            if claim == 0 {
                return Err(SourceGeometryError::NoOpenStroke);
            }
            current.tracks = initialized;
            current.owner = Some(claim);
            let alpha_checksum = controls.alpha.as_ref().map(|a| {
                a.pixels().iter().fold(0xcbf29ce484222325u64, |sum, v| {
                    (sum ^ u64::from(*v)).wrapping_mul(0x100000001b3)
                })
            });
            session.controls = Some(CapturedGeometryControls {
                options: controls,
                center,
                tracks: tracks.clone(),
                before,
                expected: initialized,
                claim,
                alpha_checksum,
            });
            Ok(())
        })();
        if let Err(e) = result {
            if let Ok(session) = shared.lock() {
                let _ = doc.cancel_source_geometry_stroke(&session.stroke);
            }
            return Err(e);
        }
        Ok(shared)
    }
    fn apply_locked(
        &mut self,
        doc: &mut Document,
        input: SourceGeometryInput,
    ) -> Result<SourceGeometryReport, SourceGeometryError> {
        validate_input(input)?;
        if !input.in_contact {
            return Ok(self.record_noop(input, SourceGeometryNoop::NoContact));
        }
        let press = self.locked_press.ok_or(SourceGeometryError::InvalidInput)?;
        if Some(self.subtool) != doc.active_subtool_id()
            || self.level != doc.current_level()
            || !doc.active_subtool_visible()
            || self.pose.map(|m| m.map(f32::to_bits))
                != doc
                    .active_object_metadata()
                    .map(|m| m.affine.local_to_world.map(f32::to_bits))
            || input.view.world_to_clip.map(f64::to_bits)
                != press.view.world_to_clip.map(f64::to_bits)
            || input.view.viewport.map(f64::to_bits) != press.view.viewport.map(f64::to_bits)
        {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        let capture = self
            .controls
            .as_mut()
            .ok_or(SourceGeometryError::InvalidInput)?;
        let mut global = capture
            .tracks
            .lock()
            .map_err(|_| SourceGeometryError::StrokeBindingChanged)?;
        if global.owner != Some(capture.claim)
            || !same_tracks(global.tracks, capture.expected)
            || doc.source_geometry_active_claim() != Some(self.stroke.claim_id())
        {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        let tracks = self
            .locked_press_tracks
            .ok_or(SourceGeometryError::InvalidInput)?;
        let result = doc.replace_source_geometry_locked_preview(
            &mut self.stroke,
            press.screen,
            PaintPreparationInput {
                view: input.view,
                screen: input.screen,
                screen_radius: self.preferences.radius,
                pressure: input.raw.pressure,
                radius_factor: self.preferences.radius_factor,
            },
            &capture.options,
            tracks,
        )?;
        capture.claim = self.stroke.claim_id();
        capture.expected = result.report.tracks;
        global.owner = Some(capture.claim);
        global.tracks = capture.expected;
        self.last_locked_preview = Some([
            result.radius_pixels,
            result.synthetic_screen[0],
            result.synthetic_screen[1],
        ]);
        self.last_input = Some(input);
        self.last_pass = result.report.contacts[0];
        self.last_triangle = None;
        self.last_controlled = Some(result.report.clone());
        self.last_render_snapshot = None;
        self.last_stage_time = None;
        self.last_report = Some(result.report.combined.clone());
        Ok(result.report.combined)
    }
    fn apply_controlled(
        &mut self,
        doc: &mut Document,
        input: SourceGeometryInput,
    ) -> Result<SourceGeometryReport, SourceGeometryError> {
        self.last_stage_time = None;
        self.last_render_snapshot = None;
        let mut stage = SourceGeometryStageTime::default();
        if doc.source_geometry_active_claim() != Some(self.stroke.claim_id()) {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        let result = (|| {
            validate_input(input)?;
            if Some(self.subtool) != doc.active_subtool_id()
                || self.level != doc.current_level()
                || !doc.active_subtool_visible()
                || self.pose.map(|m| m.map(f32::to_bits))
                    != doc
                        .active_object_metadata()
                        .map(|m| m.affine.local_to_world.map(f32::to_bits))
            {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
            if !input.in_contact {
                return Ok(self.record_noop(input, SourceGeometryNoop::NoContact));
            }
            let capture = self
                .controls
                .as_ref()
                .ok_or(SourceGeometryError::InvalidInput)?;
            if doc.active_object_metadata().is_some_and(|m| {
                m.affine.interaction_center.map(f32::to_bits) != capture.center.map(f32::to_bits)
            }) {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
            let plane = if capture.options.mirror {
                Some(doc.source_geometry_mirror_plane(
                    capture.center,
                    self.preferences.mirror_offset,
                    [1.0, 0.0, 0.0],
                )?)
            } else {
                None
            };
            let preparation_started = Instant::now();
            let prepared = doc.prepare_source_geometry_controlled_dab(
                &mut self.stroke,
                PaintPreparationInput {
                    view: input.view,
                    screen: input.screen,
                    screen_radius: self.preferences.radius,
                    pressure: input.raw.pressure,
                    radius_factor: self.preferences.radius_factor,
                },
                plane,
            )?;
            stage.preparation = preparation_started.elapsed();
            let preparation_parts = prepared.preparation_time();
            stage.retained_match = preparation_parts.retained_match;
            stage.contact_prepare = preparation_parts.contact_prepare;
            let triangles = prepared.contacts().triangles;
            let contacts = prepared.contacts().passes;
            let mut global = capture
                .tracks
                .lock()
                .map_err(|_| SourceGeometryError::StrokeBindingChanged)?;
            if global.owner != Some(capture.claim) || !same_tracks(global.tracks, capture.expected)
            {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
            if capture.options.alpha.is_none() && !capture.options.mirror && contacts[0].is_none() {
                drop(global);
                return Ok(self.record_noop(input, SourceGeometryNoop::EmptySelection));
            }
            // Controlled captures own their selected-vertex history even with
            // no alpha or mirror. The old narrow constructor retains its own
            // single-pass API and snapshot-history contract.
            let deformation_started = Instant::now();
            let (report, render_snapshot) = prepared.apply_with_render_snapshot(
                input.raw.pressure,
                true,
                &capture.options,
                global.tracks,
            )?;
            stage.deformation = deformation_started.elapsed();
            self.last_render_snapshot = render_snapshot;
            global.tracks = report.tracks;
            drop(global);
            self.controls
                .as_mut()
                .ok_or(SourceGeometryError::InvalidInput)?
                .expected = report.tracks;
            self.last_report = Some(report.combined.clone());
            self.last_input = Some(input);
            self.last_pass = contacts[0];
            self.last_triangle = triangles[0];
            self.last_mirror_triangle = triangles[1];
            self.last_mirror_plane = plane;
            self.last_controlled = Some(report.clone());
            Ok(report.combined)
        })();
        self.last_stage_time = Some(stage);
        if let Err(e) = &result {
            self.failed = Some(e.clone());
        }
        result
    }
    fn release_tracks(&mut self, restore: bool) {
        if let Some(c) = &self.controls
            && let Ok(mut current) = c.tracks.lock()
        {
            // Global unique Doc claim, not equal words or Arc address,
            // prevents foreign/expired sessions changing a newer gesture.
            if current.owner == Some(c.claim) {
                if restore {
                    current.tracks = c.before;
                } else if self.locked_press.is_some() {
                    current.tracks.primary.keep_origin = false;
                }
                current.owner = None;
            }
        }
    }
}
pub(crate) fn same_tracks(
    a: obrush_core::SourceGeometryAlphaTracks,
    b: obrush_core::SourceGeometryAlphaTracks,
) -> bool {
    [a.primary, a.mirror]
        .into_iter()
        .zip([b.primary, b.mirror])
        .all(|(a, b)| {
            a.origin.map(f64::to_bits) == b.origin.map(f64::to_bits)
                && a.frame.map(f32::to_bits) == b.frame.map(f32::to_bits)
                && a.side.to_bits() == b.side.to_bits()
                && a.undefined_normal == b.undefined_normal
                && a.keep_origin == b.keep_origin
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_paint_host::RawPressureSample;
    use obrush_core::{MaskConvention, Mesh, PaintViewCapture};
    fn doc() -> Document {
        let mut m = Mesh::from_obj("v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n").unwrap();
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m.masks.fill(1.);
        Document::new(m)
    }
    fn settings() -> BrushSettings {
        BrushSettings {
            kind: BrushKind::Inflate,
            symmetry_x: false,
            adaptive_detail: false,
            ..Default::default()
        }
    }
    fn preferences() -> SourceGeometryPreferences {
        SourceGeometryPreferences {
            mode: SourceGeometryMode::SourceOrdinary,
            radius: 75.,
            radius_factor: 0.,
            ..SourceGeometryPreferences::inflate()
        }
    }
    fn input() -> SourceGeometryInput {
        SourceGeometryInput {
            view: PaintViewCapture {
                world_to_clip: glam::DMat4::IDENTITY.to_cols_array(),
                viewport: [0., 0., 100., 100.],
            },
            screen: [50., 50.],
            raw: RawPressureSample::default(),
            in_contact: true,
        }
    }
    #[test]
    fn locked_preview_stale_alpha_lease_cancels_before_release() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let mut brush = settings();
        brush.kind = BrushKind::Draw;
        let prefs = SourceGeometryPreferences {
            mode: SourceGeometryMode::SourceOrdinary,
            radius: 40.,
            options: SourceGeometryOptions {
                tool: SourceGeometryTool::Brush {
                    clay: false,
                    accumulate: false,
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let tracks = Arc::new(Mutex::new(SourceGeometryAlphaState::default()));
        let session = SourceGeometrySession::begin_with_controls(
            &mut d,
            &brush,
            true,
            prefs,
            input(),
            tracks.clone(),
        )
        .unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        let mut moved = input();
        moved.screen = [90., 50.];
        assert!(
            apply_shared(&mut d, &session, moved)
                .unwrap()
                .changed_vertices
                > 0
        );
        assert_ne!(d.to_json().unwrap(), before);
        let complete_preview = d.to_json().unwrap();
        let mut no_contact = moved;
        no_contact.raw = RawPressureSample {
            pressure: 0.,
            pen: true,
            browser_number: true,
            absent: false,
        };
        no_contact.in_contact = false;
        assert_eq!(
            apply_shared(&mut d, &session, no_contact).unwrap().noop,
            Some(SourceGeometryNoop::NoContact)
        );
        assert_eq!(d.to_json().unwrap(), complete_preview);
        assert_eq!(d.history_depths(), (0, 0));
        tracks.lock().unwrap().tracks.primary.side += 1.;
        moved.screen = [50., 10.];
        assert_eq!(
            apply_shared(&mut d, &session, moved).unwrap_err(),
            SourceGeometryError::StrokeBindingChanged
        );
        assert_eq!(d.to_json().unwrap(), before);
        assert_eq!(d.source_geometry_active_claim(), None);
        session.lock().unwrap().finish(&mut d).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
        assert!(!d.can_redo());
        assert!(tracks.lock().unwrap().owner.is_none());
    }
    #[test]
    fn timed_controlled_dab_matches_the_ordinary_document_and_history() {
        let mut measured = doc();
        let mut reference = doc();
        let before = measured.to_json().unwrap();
        let first = SourceGeometrySession::begin_with_controls(
            &mut measured,
            &settings(),
            false,
            preferences(),
            input(),
            Arc::new(Mutex::new(SourceGeometryAlphaState::default())),
        )
        .unwrap();
        let second = SourceGeometrySession::begin_with_controls(
            &mut reference,
            &settings(),
            false,
            preferences(),
            input(),
            Arc::new(Mutex::new(SourceGeometryAlphaState::default())),
        )
        .unwrap();
        let (measured_report, stage, _) = apply_shared_with_timing(&mut measured, &first, input());
        assert!(stage.is_some());
        assert_eq!(
            measured_report.unwrap().changed_vertices,
            apply_shared(&mut reference, &second, input())
                .unwrap()
                .changed_vertices
        );
        first.lock().unwrap().finish(&mut measured).unwrap();
        second.lock().unwrap().finish(&mut reference).unwrap();
        let after = measured.to_json().unwrap();
        assert_ne!(before, after);
        assert_eq!(after, reference.to_json().unwrap());
        assert!(measured.undo());
        assert!(reference.undo());
        assert_eq!(measured.to_json().unwrap(), before);
        assert_eq!(measured.to_json().unwrap(), reference.to_json().unwrap());
    }
    #[test]
    fn begin_has_no_initial_dab_or_cache_or_undo() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let s = SourceGeometrySession::begin(&mut d, &settings(), false, preferences(), input())
            .unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert_eq!(d.source_geometry_retained_bytes(), 0);
        assert!(!d.can_undo());
        assert!(s.lock().unwrap().last_report.is_none());
        s.lock().unwrap().finish(&mut d).unwrap();
        assert!(!d.can_undo());
    }
    #[test]
    fn actual_captured_radius_produces_useful_geometry_and_whole_history() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let s = SourceGeometrySession::begin(&mut d, &settings(), false, preferences(), input())
            .unwrap();
        let r = apply_shared(&mut d, &s, input()).unwrap();
        assert_eq!(r.changed_vertices, 3);
        s.lock().unwrap().finish(&mut d).unwrap();
        let after = d.to_json().unwrap();
        assert_ne!(after, before);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn captured_preferences_survive_caller_edit_and_finish_rejects_tail() {
        let mut d = doc();
        let mut p = preferences();
        let s = SourceGeometrySession::begin(&mut d, &settings(), false, p, input()).unwrap();
        p.options.intensity = 0.;
        assert_eq!(p.options.intensity, 0.);
        assert_eq!(s.lock().unwrap().preferences().options.intensity, 0.3);
        assert!(apply_shared(&mut d, &s, input()).unwrap().changed_vertices > 0);
        s.lock().unwrap().finish(&mut d).unwrap();
        assert_eq!(
            apply_shared(&mut d, &s, input()).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
    }
    #[test]
    fn pen_zero_contact_clears_pick_provenance_without_mutation() {
        let mut d = doc();
        let s = SourceGeometrySession::begin(&mut d, &settings(), false, preferences(), input())
            .unwrap();
        apply_shared(&mut d, &s, input()).unwrap();
        let before = d.to_json().unwrap();
        let mut zero = input();
        zero.raw = RawPressureSample {
            pressure: 0.,
            pen: true,
            browser_number: true,
            absent: false,
        };
        zero.in_contact = false;
        let report = apply_shared(&mut d, &s, zero).unwrap();
        assert_eq!(report.noop, Some(SourceGeometryNoop::NoContact));
        assert_eq!(d.to_json().unwrap(), before);
        let lock = s.lock().unwrap();
        assert!(lock.last_pass.is_none());
        assert!(lock.last_triangle.is_none());
        assert!(lock.last_report.as_ref().unwrap().picked_normal.is_none());
    }
    #[test]
    fn invalid_provenance_rejects_and_cancellation_restores_whole_document() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let s = SourceGeometrySession::begin(&mut d, &settings(), false, preferences(), input())
            .unwrap();
        apply_shared(&mut d, &s, input()).unwrap();
        let mut bad = input();
        bad.in_contact = false;
        assert_eq!(
            apply_shared(&mut d, &s, bad).unwrap_err(),
            SourceGeometryError::InvalidInput
        );
        s.lock().unwrap().finish(&mut d).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
        assert_eq!(d.source_geometry_retained_bytes(), 0);
    }
    #[test]
    fn bounded_path_spacing_drops_old_work_without_extra_endpoint() {
        let mut d = doc();
        let s = SourceGeometrySession::begin(&mut d, &settings(), false, preferences(), input())
            .unwrap();
        let mut far = input();
        far.screen = [600., 50.];
        let rows = s.lock().unwrap().path_samples(far).unwrap();
        assert_eq!(rows.len(), 32);
        assert_eq!(rows.last().unwrap().screen, far.screen);
        assert!(s.lock().unwrap().path_samples(far).unwrap().is_empty());
        s.lock().unwrap().cancel(&mut d).unwrap();
        assert!(!d.can_undo());
    }
    #[test]
    fn observations_preserve_actual_useful_no_contact_and_no_hit_order() {
        for controlled in [false, true] {
            let mut d = doc();
            let before = d.to_json().unwrap();
            let session = if controlled {
                SourceGeometrySession::begin_with_controls(
                    &mut d,
                    &settings(),
                    false,
                    preferences(),
                    input(),
                    Arc::new(Mutex::new(SourceGeometryAlphaState::default())),
                )
                .unwrap()
            } else {
                SourceGeometrySession::begin(&mut d, &settings(), false, preferences(), input())
                    .unwrap()
            };
            assert!(session.lock().unwrap().dab_observations.rows.is_empty());
            let useful = apply_shared(&mut d, &session, input()).unwrap();
            assert_eq!(useful.changed_vertices, 3);
            let mut no_contact = input();
            no_contact.raw = RawPressureSample {
                pressure: 0.0,
                pen: true,
                browser_number: true,
                absent: false,
            };
            no_contact.in_contact = false;
            assert_eq!(
                apply_shared(&mut d, &session, no_contact).unwrap().noop,
                Some(SourceGeometryNoop::NoContact)
            );
            let mut missed = input();
            missed.screen = [99.0, 1.0];
            assert_eq!(
                apply_shared(&mut d, &session, missed).unwrap().noop,
                Some(SourceGeometryNoop::EmptySelection)
            );
            let mut lock = session.lock().unwrap();
            let trace = lock.dab_observations.diagnostics();
            assert_eq!(trace["total_calls"], 3);
            assert_eq!(trace["complete"], true);
            let rows = trace["rows"].as_array().unwrap();
            assert_eq!(
                rows.iter()
                    .map(|v| v["ordinal"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                [1, 2, 3]
            );
            assert_eq!(rows[0]["outcomes"][0]["changed_vertices"], 3);
            assert!(rows[0]["passes"][0].is_object());
            assert!(rows[0]["passes"][1].is_null());
            assert!(rows[1]["passes"][0].is_null());
            assert_eq!(rows[1]["outcomes"][0]["noop"], "NoContact");
            assert!(rows[2]["passes"][0].is_null());
            assert_eq!(rows[2]["outcomes"][0]["noop"], "EmptySelection");
            lock.finish(&mut d).unwrap();
            drop(lock);
            let after = d.to_json().unwrap();
            assert_ne!(after, before);
            assert!(d.undo());
            assert_eq!(d.to_json().unwrap(), before);
            assert!(d.redo());
            assert_eq!(d.to_json().unwrap(), after);
        }
    }
    #[test]
    fn observation_capacity_does_not_end_or_mutate_a_real_stroke() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let session =
            SourceGeometrySession::begin(&mut d, &settings(), false, preferences(), input())
                .unwrap();
        let mut no_contact = input();
        no_contact.raw = RawPressureSample {
            pressure: 0.0,
            pen: true,
            browser_number: true,
            absent: false,
        };
        no_contact.in_contact = false;
        for _ in 0..MAX_DAB_OBSERVATIONS + 1 {
            apply_shared(&mut d, &session, no_contact).unwrap();
        }
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
        assert_eq!(
            apply_shared(&mut d, &session, input())
                .unwrap()
                .changed_vertices,
            3
        );
        let mut lock = session.lock().unwrap();
        let trace = lock.dab_observations.diagnostics();
        assert_eq!(trace["total_calls"], (MAX_DAB_OBSERVATIONS + 2) as u64);
        assert_eq!(
            trace["rows"].as_array().unwrap().len(),
            MAX_DAB_OBSERVATIONS
        );
        assert_eq!(trace["complete"], false);
        assert_eq!(trace["retention_failure"], "Capacity");
        lock.finish(&mut d).unwrap();
        assert_eq!(
            lock.apply(&mut d, input()).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(
            lock.dab_observations.total,
            (MAX_DAB_OBSERVATIONS + 2) as u64
        );
        drop(lock);
        let after = d.to_json().unwrap();
        assert_ne!(after, before);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn controlled_mirror_observation_keeps_both_actual_ordered_contacts() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let mut brush = settings();
        brush.symmetry_x = true;
        let mut press = input();
        press.screen[0] = 45.0;
        let session = SourceGeometrySession::begin_with_controls(
            &mut d,
            &brush,
            false,
            preferences(),
            press,
            Arc::new(Mutex::new(SourceGeometryAlphaState::default())),
        )
        .unwrap();
        assert!(
            apply_shared(&mut d, &session, press)
                .unwrap()
                .changed_vertices
                > 0
        );
        let mut lock = session.lock().unwrap();
        let actual = lock.last_controlled.as_ref().unwrap();
        let primary = actual.contacts[0].unwrap();
        let mirror = actual.contacts[1].unwrap();
        assert!(primary.center[0] < 0.0);
        assert!(mirror.center[0] > 0.0);
        let row = &lock.dab_observations.rows[0];
        assert_eq!(lock.dab_observations.total, 1);
        assert_eq!(
            row.passes[0].unwrap().center.map(f64::to_bits),
            primary.center.map(f64::to_bits)
        );
        assert_eq!(
            row.passes[1].unwrap().center.map(f64::to_bits),
            mirror.center.map(f64::to_bits)
        );
        for (observed, actual) in row.outcomes.iter().zip(actual.passes.iter()) {
            assert_eq!(
                observed.as_ref().unwrap().changed_vertices,
                actual.as_ref().unwrap().changed_vertices
            );
        }
        lock.finish(&mut d).unwrap();
        drop(lock);
        let after = d.to_json().unwrap();
        assert_ne!(before, after);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn invalid_dab_records_first_error_raw_bits_and_preserves_rollback() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let session =
            SourceGeometrySession::begin(&mut d, &settings(), false, preferences(), input())
                .unwrap();
        let mut invalid = input();
        invalid.screen[0] = f64::from_bits(0x7ff8000000000042);
        assert_eq!(
            apply_shared(&mut d, &session, invalid).unwrap_err(),
            SourceGeometryError::InvalidInput
        );
        assert_eq!(
            apply_shared(&mut d, &session, input()).unwrap_err(),
            SourceGeometryError::InvalidInput
        );
        let mut lock = session.lock().unwrap();
        let trace = lock.dab_observations.diagnostics();
        assert_eq!(trace["total_calls"], 1);
        assert_eq!(trace["complete"], true);
        assert_eq!(trace["rows"][0]["screen_bits64"][0], "7ff8000000000042");
        assert!(trace["rows"][0]["screen64"][0].is_null());
        assert_eq!(trace["rows"][0]["error"], "InvalidInput");
        assert_eq!(trace["rows"][0]["passes"], serde_json::json!([null, null]));
        lock.finish(&mut d).unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
    }
    #[test]
    fn unsupported_settings_block_before_document_history() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let mut unsupported = settings();
        unsupported.symmetry_x = true;
        assert!(
            SourceGeometrySession::begin(&mut d, &unsupported, false, preferences(), input())
                .is_err()
        );
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
    }
}

#[cfg(test)]
mod idle_normal_render_tests {
    use super::*;
    use obrush_core::{MaskConvention, Mesh, SourceNormalField, SourceNormalPolicy};
    fn useful_document() -> (Document, String, String) {
        let mut mesh = Mesh::import_obj(
            b"v 0 0 1\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nf 1 2 3\nf 1 3 4\nf 1 4 2\nf 2 4 3\n",
        )
        .unwrap()
        .mesh;
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let mut doc = Document::new(mesh);
        let before = doc.to_json().unwrap();
        let options = SourceGeometryOptions {
            tool: SourceGeometryTool::Smooth { tangent: true },
            intensity: 0.13,
            prevent_folds: false,
            ..SourceGeometryOptions::smooth()
        };
        let mut stroke = doc.begin_source_geometry_stroke(options).unwrap();
        let pass = SourceGeometryPass {
            center: [0.1268937959470357, 0.06249996161826249, 0.8106062424346874],
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 0.11794690396893492,
            original_face_index: 0,
        };
        assert_eq!(
            doc.apply_source_geometry_dab(&mut stroke, pass, 0.5, true)
                .unwrap()
                .changed_vertices,
            1
        );
        doc.end_source_geometry_stroke(&stroke).unwrap();
        let after = doc.to_json().unwrap();
        (doc, before, after)
    }
    #[test]
    fn idle_render_keeps_literal_warm_neighbor_and_history_across_tool_independent_owner() {
        let (mut doc, before, after) = useful_document();
        let id = doc.active_subtool_id().unwrap();
        let retained = doc.source_geometry_state(id).unwrap();
        let fresh = SourceNormalField::compute(
            doc.active_mesh().unwrap(),
            SourceNormalPolicy::StaticOriginalFaces,
        )
        .unwrap();
        assert_eq!(
            retained.vertex_normals()[1].to_array().map(f32::to_bits),
            [0x3f2aaaab, 0, 0]
        );
        assert_ne!(fresh.vertex_normals()[1], retained.vertex_normals()[1]);
        let idle = normal_render_states(&doc, [id].into_iter(), SourceNormalRenderOwner::Idle);
        assert_eq!(idle.len(), 1);
        assert_eq!(idle[0].0, id);
        assert!(Arc::ptr_eq(&idle[0].1, &retained));
        assert_eq!(doc.to_json().unwrap(), after);
        assert!(doc.undo());
        assert_eq!(doc.to_json().unwrap(), before);
        assert!(
            normal_render_states(&doc, [id].into_iter(), SourceNormalRenderOwner::Idle).is_empty()
        );
        assert!(doc.redo());
        assert_eq!(doc.to_json().unwrap(), after);
        let restored = normal_render_states(&doc, [id].into_iter(), SourceNormalRenderOwner::Idle);
        assert_eq!(restored[0].1.vertex_normals(), retained.vertex_normals());
    }
    #[test]
    fn idle_render_selection_does_not_change_active_legacy_or_captured_paint_ownership() {
        let (doc, _, after) = useful_document();
        let id = doc.active_subtool_id().unwrap();
        let retained = doc.source_geometry_state(id).unwrap();
        assert!(
            normal_render_states(&doc, [id].into_iter(), SourceNormalRenderOwner::Legacy)
                .is_empty()
        );
        assert!(
            normal_render_states(
                &doc,
                [id].into_iter(),
                SourceNormalRenderOwner::CapturedPaint(None)
            )
            .is_empty()
        );
        let paint = normal_render_states(
            &doc,
            [id].into_iter(),
            SourceNormalRenderOwner::CapturedPaint(Some((id, retained.clone()))),
        );
        assert_eq!(paint.len(), 1);
        assert!(Arc::ptr_eq(&paint[0].1, &retained));
        let geometry =
            normal_render_states(&doc, [id].into_iter(), SourceNormalRenderOwner::Geometry);
        assert!(Arc::ptr_eq(&geometry[0].1, &retained));
        assert!(
            normal_render_states(&doc, [u32::MAX].into_iter(), SourceNormalRenderOwner::Idle)
                .is_empty()
        );
        assert_eq!(doc.to_json().unwrap(), after);
    }
}
