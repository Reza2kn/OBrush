//! Opt-in Source Paint. The old alphaNone/symmetry-off APIs stay explicit;
//! the normal tracked route captures imported alphas and local mirrored rays.
use crate::renderer::Camera;
use eframe::egui::{Pos2, Rect};
use glam::DMat4;
use obrush_core::{
    BrushKind, BrushSettings, Document, MaskConvention, PaintResponseOperation,
    PaintResponseOptions, PaintResponseReport, PaintResponseRequest, PaintViewCapture,
};
use std::sync::{Arc, Mutex};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PaintMode {
    #[default]
    Legacy,
    SourceOrdinary,
}
#[derive(Clone, Debug)]
pub struct SourcePaintPreferences {
    pub mode: PaintMode,
    pub radius: f64,
    pub radius_factor: f64,
    pub mirror_offset: f64,
    pub options: PaintResponseOptions,
}
impl Default for SourcePaintPreferences {
    fn default() -> Self {
        Self {
            mode: PaintMode::Legacy,
            radius: 75.,
            radius_factor: 0.75,
            mirror_offset: 0.0,
            options: PaintResponseOptions::default(),
        }
    }
}
impl SourcePaintPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if !self.radius.is_finite()
            || !(5.0..=500.0).contains(&self.radius)
            || !self.radius_factor.is_finite()
            || !(0.0..=1.0).contains(&self.radius_factor)
            || !self.mirror_offset.is_finite()
            || !(-1.0..=1.0).contains(&self.mirror_offset)
            || !self.options.intensity.is_finite()
            || !(0.0..=1.0).contains(&self.options.intensity)
            || !self.options.hardness.is_finite()
            || !(0.0..=1.0).contains(&self.options.hardness)
            || !self.options.tablet.factor.is_finite()
            || !(0.0..=1.0).contains(&self.options.tablet.factor)
            || self
                .options
                .color
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || [self.options.roughness, self.options.metallic]
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || self.options.alpha.is_some()
        {
            return Err("Source ordinary Paint preferences are invalid or outside the supported no-alpha scope".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawPressureSample {
    pub pressure: f64,
    pub pen: bool,
    pub browser_number: bool,
    pub absent: bool,
}
impl Default for RawPressureSample {
    fn default() -> Self {
        Self {
            pressure: 0.5,
            pen: false,
            browser_number: false,
            absent: true,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct RawPaintInput {
    active_pen: bool,
    pub current: RawPressureSample,
    pub terminal: Option<RawPressureSample>,
    pub terminal_screen: Option<[f64; 2]>,
    pub screen: Option<[f64; 2]>,
    pub error: Option<String>,
}
impl RawPaintInput {
    pub fn receive(
        &mut self,
        pressure: f64,
        pen: bool,
        released: bool,
        browser_number: bool,
        screen: Option<[f64; 2]>,
    ) {
        if !pressure.is_finite()
            || !(0.0..=1.0).contains(&pressure)
            || screen.is_some_and(|p| p.iter().any(|v| !v.is_finite()))
        {
            self.error = Some("Raw Paint input is nonfinite or out of range".into());
            return;
        }
        if released {
            self.active_pen = false;
            self.terminal.get_or_insert(self.current);
            if let Some(screen) = screen {
                // The precise bridge follows the legacy f32 hook. Its actual
                // release coordinates supersede the legacy last-move fallback.
                self.terminal_screen = Some(screen);
            } else if self.terminal_screen.is_none() {
                self.terminal_screen = self.screen;
            }
        } else {
            self.active_pen = pen;
            self.current = RawPressureSample {
                pressure: if pen { pressure } else { 0.5 },
                pen,
                browser_number,
                absent: !pen,
            };
            self.error = None;
        }
        if screen.is_some() {
            self.screen = screen;
        }
    }
    pub fn sample(&self) -> Result<RawPressureSample, String> {
        if let Some(e) = &self.error {
            return Err(e.clone());
        }
        Ok(self.terminal.unwrap_or(self.current))
    }
    pub fn finish(&mut self) {
        self.terminal = None;
        self.terminal_screen = None;
        if !self.active_pen && self.current.pen {
            self.current = RawPressureSample::default();
        }
    }
    pub fn position(&self, fallback: Pos2) -> [f64; 2] {
        self.terminal_screen
            .or(self.screen)
            .unwrap_or([f64::from(fallback.x), f64::from(fallback.y)])
    }
}
#[derive(Clone, Copy, Debug)]
pub struct SourcePaintInput {
    pub view: PaintViewCapture,
    pub screen: [f64; 2],
    pub raw: RawPressureSample,
    pub in_contact: bool,
}
impl SourcePaintInput {
    fn validate(&self) -> Result<(), String> {
        self.view.validate()?;
        if self.screen.iter().any(|v| !v.is_finite())
            || !self.raw.pressure.is_finite()
            || !(0.0..=1.0).contains(&self.raw.pressure)
            || self.in_contact != (!self.raw.pen || self.raw.pressure > 0.0)
        {
            return Err("Source Paint sample has invalid raw input/contact provenance".into());
        }
        Ok(())
    }
}
pub fn capture_at_zoom(
    camera: &Camera,
    pos: Pos2,
    rect: Rect,
    raw: &RawPaintInput,
    zoom_factor: f32,
) -> Result<SourcePaintInput, String> {
    if zoom_factor != 1.0 {
        return Err("Source Paint requires UI scale 100% until precise CSS-to-logical zoom preparation is implemented".into());
    }
    capture(camera, pos, rect, raw)
}

pub fn capture(
    camera: &Camera,
    pos: Pos2,
    rect: Rect,
    raw: &RawPaintInput,
) -> Result<SourcePaintInput, String> {
    let matrix = DMat4::from_cols_array(
        &camera
            .projection(rect.aspect_ratio())
            .to_cols_array()
            .map(f64::from),
    ) * camera.view64();
    let view = PaintViewCapture {
        world_to_clip: matrix.to_cols_array(),
        viewport: [
            f64::from(rect.left()),
            f64::from(rect.top()),
            f64::from(rect.width()),
            f64::from(rect.height()),
        ],
    };
    view.validate()?;
    let raw_sample = raw.sample()?;
    let screen = raw.position(pos);
    Ok(SourcePaintInput {
        view,
        screen,
        raw: raw_sample,
        in_contact: !raw_sample.pen || raw_sample.pressure > 0.,
    })
}
type PreparedPaintContact = (
    obrush_core::PaintResponsePass,
    obrush_core::PreparedSourcePaintHit,
    obrush_core::SourceNormalReport,
    obrush_core::SourcePickedNormalDisposition,
    obrush_core::SourcePaintNormalProducerReport,
);
#[derive(Clone, Copy)]
struct PreparedPaintMirror {
    pass: obrush_core::PaintResponsePass,
    geometry_pass: obrush_core::SourceGeometryPass,
    triangle: usize,
    picked: obrush_core::SourcePickedNormalDisposition,
}
pub type SharedSourcePaint = Arc<Mutex<SourcePaintSession>>;
pub struct SourcePaintSession {
    pub preferences: SourcePaintPreferences,
    pub subtool: u32,
    pub level: usize,
    pub failed: Option<String>,
    pub last_report: Option<PaintResponseReport>,
    pub last_input: Option<SourcePaintInput>,
    pub last_triangle: Option<usize>,
    pub last_pass: Option<obrush_core::PaintResponsePass>,
    cursor_query: Option<Arc<obrush_core::SourcePreparedPickQuery>>,
    mirror_plane: Option<obrush_core::SourceMirrorPlane>,
    last_mirror: Option<PreparedPaintMirror>,
    last_mirror_alpha_track: Option<obrush_core::SourceGeometryAlphaTrack>,
    alpha_lease: Option<crate::source_geometry_host::SourcePaintAlphaLease>,
    last_alpha_track: Option<obrush_core::SourceGeometryAlphaTrack>,
    normal_field: Option<obrush_core::SourceNormalField>,
    retained_normal_state: Option<Arc<obrush_core::SourceGeometryNormalState>>,
    warm_history_owner: Option<obrush_core::SourceRetainedPaintStroke>,
    warm_history_rollback: Option<bool>,
    last_normal_producer_report: Option<obrush_core::SourcePaintNormalProducerReport>,
    last_normal_report: Option<obrush_core::SourceNormalReport>,
    last_picked_normal: Option<obrush_core::SourcePickedNormalReport>,
    last_picked_normal_disposition: Option<obrush_core::SourcePickedNormalDisposition>,
    last_screen: [f64; 2],
    pose: Option<[f32; 16]>,
    locked_stroke: Option<obrush_core::SourcePaintLockedStroke>,
    locked_anchor: Option<crate::locked_anchor::LockedAnchor>,
    locked_viewport: Option<[f64; 4]>,
    last_locked_preparation: Option<serde_json::Value>,
    last_locked_report: Option<obrush_core::SourcePaintLockedReport>,
}
pub fn blocker(doc: &Document, settings: &BrushSettings, locked: bool) -> Result<(), String> {
    let mesh = doc.require_active_mesh()?;
    if settings.kind != BrushKind::Paint {
        return Err("Source response supports Paint only; temporary Smooth/Mask retain their Legacy response".into());
    }
    if mesh.mask_convention != MaskConvention::UnprotectedF32V1 {
        return Err("Convert the complete active object's mask representation explicitly before using Source Paint".into());
    }
    if settings.alpha.is_some() {
        return Err("Source alpha frame tracking is not wired yet; select None in Legacy".into());
    }
    if settings.symmetry_x || settings.symmetry_offset != 0. {
        return Err("Source mirrored-ray/relative-offset preparation is not wired yet; disable symmetry in Legacy".into());
    }
    if settings.invert || settings.alpha_rotation != 0. || locked {
        return Err("Source Paint supports ordinary dabs only: inverse, manual rotation and Lock are unavailable".into());
    }
    if !doc.active_subtool_visible() {
        return Err("Show the active Source Paint part".into());
    }
    if let Some(state) = doc
        .active_subtool_id()
        .and_then(|id| doc.source_geometry_state(id))
    {
        retained_blocker(doc, &state)?;
        return Ok(());
    }
    let required = obrush_core::SourceNormalField::storage_requirement(mesh)?;
    if required.peak_bytes > obrush_core::MAX_SOURCE_NORMAL_BYTES {
        return Err(
            "Source Paint original-face normal cache exceeds128MiB; Legacy is not substituted"
                .into(),
        );
    }
    Ok(())
}
fn retained_blocker(
    doc: &Document,
    state: &obrush_core::SourceGeometryNormalState,
) -> Result<(), String> {
    doc.source_geometry_blocker(obrush_core::SourceGeometryOptions::inflate())
        .map_err(|e|format!("Retained Source Paint normals unavailable: {e}; finish and reset transient Source normals explicitly"))?;
    obrush_core::SourcePaintNormalView::RetainedGeometry(state).validate(doc.require_active_mesh()?)
        .map_err(|e|format!("Retained Source Paint normal producer rejected: {e:?}; reset transient Source normals explicitly"))
}
impl SourcePaintSession {
    pub fn diagnostics(&self) -> serde_json::Value {
        let options = &self.preferences.options;
        serde_json::json!({
            "part":self.subtool,"level":self.level,"failed":self.failed,"cursor_snapshot_captured":self.cursor_query.is_some(),
            "captured":{"radius":self.preferences.radius,"radius_factor":self.preferences.radius_factor,
                "intensity":options.intensity,"hardness":options.hardness,"pressure_factor":options.tablet.factor,
                "rgb":options.color,"roughness":options.roughness,"metallic":options.metallic,
                "write_rgb":options.write_rgb,"write_roughness":options.write_roughness,
                "write_metallic":options.write_metallic,"culling":options.culling,
                "pose32":self.pose},
            "triangle":self.last_triangle,
            "captured_mirror_plane":self.mirror_plane,
            "captured_mirror_offset":self.preferences.mirror_offset,
            "mirrored_pass":self.last_mirror.map(|m|serde_json::json!({"pass":m.geometry_pass,"triangle":m.triangle,"picked_normal":m.picked,"mirrored_x":true})),
            "mirror_alpha_track":self.last_mirror_alpha_track,
            "mirror_alpha_frame_words":self.last_mirror_alpha_track.map(|t|t.frame.map(f32::to_bits)),
            "pass64":self.last_pass.map(|p|serde_json::json!({"center":p.center,"eye":p.eye_direction,"radius_squared":p.local_radius_squared,"mirrored_x":p.mirrored_x})),
            "input":self.last_input.map(|i|serde_json::json!({"screen64":i.screen,"pressure64":i.raw.pressure,
                "browser_number":i.raw.browser_number,"pen":i.raw.pen,"absent":i.raw.absent,
                "in_contact":i.in_contact,"view64":i.view.world_to_clip,"viewport64":i.view.viewport})),
            "last_report":self.last_report,
            "normal_policy":self.last_normal_report,
            "normal_producer":self.last_normal_producer_report,
            "captured_normal_producer":self.retained_normal_state.as_ref().map(|s|obrush_core::SourcePaintNormalView::RetainedGeometry(s).report()),
            "warm_history_owner":self.warm_history_owner.as_ref().map(|s|s.sequence()),
            "warm_history_rollback":self.warm_history_rollback,
            "cold_fresh_producer":self.retained_normal_state.is_none(),
            "picked_normal":self.last_picked_normal,
            "picked_normal_disposition":self.last_picked_normal_disposition,
            "alpha": options.alpha.as_ref().map(|a| serde_json::json!({"width":a.width(), "height":a.height(),
                "checksum":a.pixels().iter().fold(0xcbf29ce484222325u64,|s,v|(s^u64::from(*v)).wrapping_mul(0x100000001b3))})),
            "alpha_track": self.last_alpha_track,
            "alpha_frame_words": self.last_alpha_track.map(|t|t.frame.map(f32::to_bits)),
            "picked_normal_scope":"alphaNone permits undefined IDW; tracked ordinary alpha uses shared press/dab frames and rejects undefined sampled normals",
            "locked":self.locked_stroke.as_ref().map(|s|serde_json::json!({
                "mode":s.mode(),"closed":s.is_closed(),"at_original":s.at_original(),
                "sequence":s.sequence(),"preparation":self.last_locked_preparation,
                "last_report":self.last_locked_report,"scope":"static; alphaNone; symmetryoff; actualf64 anchoredray; camera/pressure/dynamic/alpha/mirror parity unverified"}))
        })
    }
    fn record_noop(&mut self, input: SourcePaintInput) -> PaintResponseReport {
        let report = PaintResponseReport::default();
        self.last_input = Some(input);
        self.last_normal_report = None;
        self.last_normal_producer_report = None;
        self.last_picked_normal = None;
        self.last_picked_normal_disposition = None;
        self.last_pass = None;
        self.last_triangle = None;
        self.last_mirror = None;
        self.last_report = Some(report.clone());
        report
    }
    pub fn prepare(
        doc: &Document,
        prefs: &SourcePaintPreferences,
        settings: &BrushSettings,
        locked: bool,
        input: SourcePaintInput,
    ) -> Result<SharedSourcePaint, String> {
        prefs.validate()?;
        input.validate()?;
        if prefs.mode != PaintMode::SourceOrdinary {
            return Err("A real Source Paint mode capture is required".into());
        }
        blocker(doc, settings, locked)?;
        let subtool = doc
            .active_subtool_id()
            .ok_or("Source Paint needs an active part")?;
        // A zero-pressure press is allowed to establish a contact gesture, but
        // does not manufacture a positive radius or apply a dab. The original
        // precise ray must hit the active current surface before Begin.
        if obrush_core::prepare_source_paint_hit(
            doc.require_active_mesh()?,
            doc.active_object_metadata().map(|m| &m.affine),
            input.view.ray(input.screen)?,
        )?
        .is_none()
        {
            return Err("Source Paint press missed its active local surface".into());
        }
        Ok(Arc::new(Mutex::new(Self {
            preferences: prefs.clone(),
            subtool,
            level: doc.current_level(),
            failed: None,
            cursor_query: None,
            mirror_plane: None,
            last_mirror: None,
            last_mirror_alpha_track: None,
            alpha_lease: None,
            last_alpha_track: None,
            normal_field: None,
            retained_normal_state: doc.source_geometry_state(subtool),
            warm_history_owner: None,
            warm_history_rollback: None,
            last_normal_producer_report: None,
            last_normal_report: None,
            last_picked_normal: None,
            last_picked_normal_disposition: None,
            last_report: None,
            last_input: None,
            last_triangle: None,
            last_pass: None,
            last_screen: input.screen,
            locked_stroke: None,
            locked_anchor: None,
            locked_viewport: None,
            last_locked_preparation: None,
            last_locked_report: None,
            pose: doc
                .active_object_metadata()
                .map(|m| m.affine.local_to_world),
        })))
    }
    /// Captured actual warm producer for the normal render route. This cannot
    /// be mutated by callers or replaced with an arbitrary prescribed field.
    pub fn retained_normal_state(&self) -> Option<Arc<obrush_core::SourceGeometryNormalState>> {
        self.retained_normal_state.clone()
    }
    fn validate_captured_producer(&self, doc: &Document) -> Result<(), String> {
        let current = doc.source_geometry_state(self.subtool);
        match (&self.retained_normal_state, current.as_ref()) {
            (None, None) => Ok(()),
            (Some(captured), Some(current)) if Arc::ptr_eq(captured, current) => {
                retained_blocker(doc, captured)
            }
            _ => Err(
                "Source Paint captured normal producer lifetime changed; no fresh fallback".into(),
            ),
        }
    }
    pub fn path_samples(
        &mut self,
        input: SourcePaintInput,
    ) -> Result<Vec<SourcePaintInput>, String> {
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        input.validate()?;
        if self.is_locked() {
            let mut one = Vec::new();
            one.try_reserve_exact(1)
                .map_err(|_| "Cannot reserve locked Source endpoint")?;
            one.push(input);
            self.last_screen = input.screen;
            return Ok(one);
        }
        let dx = input.screen[0] - self.last_screen[0];
        let dy = input.screen[1] - self.last_screen[1];
        let distance = (dx * dx + dy * dy).sqrt();
        let spacing = 0.15 * self.preferences.radius;
        if !distance.is_finite() || distance > 65_536.0 {
            return Err("Source Paint path is nonfinite or exceeds bounded viewport travel".into());
        }
        if distance <= spacing {
            return Ok(Vec::new());
        }
        let count = (distance / spacing).floor() as usize;
        let start = count.saturating_sub(32);
        let mut out = Vec::new();
        out.try_reserve_exact(count.min(32))
            .map_err(|_| "Cannot reserve bounded Source Paint path")?;
        for i in start + 1..=count {
            let t = i as f64 / count as f64;
            let mut dab = input;
            dab.screen = [self.last_screen[0] + dx * t, self.last_screen[1] + dy * t];
            out.push(dab);
        }
        self.last_screen = input.screen;
        Ok(out)
    }
    fn prepare_contact(
        &mut self,
        doc: &Document,
        input: SourcePaintInput,
    ) -> Result<Option<PreparedPaintContact>, String> {
        let mesh = doc.require_active_mesh()?;
        let preparation = obrush_core::PaintPreparationInput {
            view: input.view,
            screen: input.screen,
            screen_radius: self.preferences.radius,
            pressure: input.raw.pressure,
            radius_factor: self.preferences.radius_factor,
        };
        let prepared = if let Some(state) = self.retained_normal_state.as_ref() {
            let Some(prepared) = obrush_core::prepare_source_paint_pass_alpha_none_with_retained(
                mesh,
                doc.active_object_metadata().map(|m| &m.affine),
                preparation,
                state,
            )?
            else {
                return Ok(None);
            };
            (
                prepared.pass,
                prepared.hit,
                prepared.normal_producer.normal_policy,
                prepared.picked_normal,
                prepared.normal_producer,
            )
        } else {
            let prepared = obrush_core::prepare_source_paint_pass_alpha_none(
                mesh,
                doc.active_object_metadata().map(|m| &m.affine),
                obrush_core::PaintPreparationInput {
                    view: input.view,
                    screen: input.screen,
                    screen_radius: self.preferences.radius,
                    pressure: input.raw.pressure,
                    radius_factor: self.preferences.radius_factor,
                },
                &mut self.normal_field,
            )?;
            let Some(prepared) = prepared else {
                return Ok(None);
            };
            let producer = obrush_core::SourcePaintNormalView::FreshStatic(
                self.normal_field
                    .as_ref()
                    .ok_or("Prepared Source Paint has no cold normal field")?,
            )
            .report();
            (
                prepared.pass,
                prepared.hit,
                prepared.normal_policy,
                prepared.picked_normal,
                producer,
            )
        };
        Ok(Some(prepared))
    }
    fn normal_view(&self) -> Result<obrush_core::SourcePaintNormalView<'_>, String> {
        if let Some(state) = &self.retained_normal_state {
            Ok(obrush_core::SourcePaintNormalView::RetainedGeometry(state))
        } else {
            Ok(obrush_core::SourcePaintNormalView::FreshStatic(
                self.normal_field
                    .as_ref()
                    .ok_or("Source Paint normal field missing")?,
            ))
        }
    }
    fn prepare_mirror_contact(
        &mut self,
        doc: &Document,
        input: SourcePaintInput,
        press: bool,
    ) -> Result<Option<PreparedPaintMirror>, String> {
        let Some(plane) = self.mirror_plane else {
            return Ok(None);
        };
        let mesh = doc.require_active_mesh()?;
        let preparation = obrush_core::PaintPreparationInput {
            view: input.view,
            screen: input.screen,
            screen_radius: self.preferences.radius,
            pressure: input.raw.pressure,
            radius_factor: self.preferences.radius_factor,
        };
        let prepare = if press {
            obrush_core::prepare_source_geometry_press_contacts
        } else {
            obrush_core::prepare_source_geometry_contacts
        };
        let contacts = prepare(
            mesh,
            doc.active_object_metadata().map(|m| &m.affine),
            preparation,
            Some(plane),
        )
        .map_err(|e| e.to_string())?;
        let Some(pass) = contacts.passes[1] else {
            return Ok(None);
        };
        if self.retained_normal_state.is_none() {
            obrush_core::SourceNormalField::refresh(
                &mut self.normal_field,
                mesh,
                obrush_core::SourceNormalPolicy::StaticOriginalFaces,
            )?;
        }
        let picked = self
            .normal_view()?
            .picked_normal_alpha_none(
                mesh,
                obrush_core::SourceOriginalFaceContact {
                    face_index: pass.original_face_index,
                    local_point: pass.center,
                },
            )
            .map_err(|e| format!("Source Paint mirrored normal rejected: {e:?}"))?;
        Ok(Some(PreparedPaintMirror {
            pass: obrush_core::PaintResponsePass {
                center: pass.center,
                eye_direction: pass.eye_direction,
                local_radius_squared: pass.local_radius_squared,
                mirrored_x: true,
                alpha_frame: None,
            },
            geometry_pass: pass,
            triangle: contacts.triangles[1].ok_or("Mirrored contact triangle missing")?,
            picked,
        }))
    }
    pub fn apply(
        &mut self,
        doc: &mut Document,
        input: SourcePaintInput,
    ) -> Result<PaintResponseReport, String> {
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        if self.is_locked() {
            return self.apply_locked(doc, input);
        }
        let outcome: Result<PaintResponseReport, String> = (|| {
            input.validate()?;
            if let Some(state) = self.retained_normal_state.as_ref() {
                if let Some(owner) = self.warm_history_owner.as_ref() {
                    doc.validate_retained_source_paint_stroke(owner)
                        .map_err(|e| e.to_string())?;
                } else {
                    self.warm_history_owner = Some(
                        doc.claim_retained_source_paint_stroke(self.subtool, self.level, state)
                            .map_err(|e| e.to_string())?,
                    );
                }
            }
            if Some(self.subtool) != doc.active_subtool_id()
                || self.level != doc.current_level()
                || !doc.active_subtool_visible()
                || self.pose.map(|p| p.map(f32::to_bits))
                    != doc
                        .active_object_metadata()
                        .map(|m| m.affine.local_to_world.map(f32::to_bits))
            {
                return Err("Source Paint gesture binding changed".into());
            }
            self.validate_captured_producer(doc)?;
            if !input.in_contact {
                return Ok(self.record_noop(input));
            }
            let primary = self.prepare_contact(doc, input)?;
            let mut mirror = self.prepare_mirror_contact(doc, input, false)?;
            let (mut first, triangle, original_face, normal_policy, picked_normal, producer_report) =
                if let Some((pass, hit, normal, picked, producer)) = primary {
                    (
                        pass,
                        hit.triangle_index,
                        hit.original_face_index.unwrap_or(hit.triangle_index),
                        normal,
                        picked,
                        producer,
                    )
                } else if let Some(m) = mirror {
                    // Pinned makeStroke copies the primary miss's zero radius to
                    // the mirrored Picking. Strict distance <0 selects no rows;
                    // preserve its actual frame update rather than a stale radius.
                    let producer = self.normal_view()?.report();
                    (
                        m.pass,
                        m.triangle,
                        m.geometry_pass.original_face_index,
                        producer.normal_policy,
                        m.picked,
                        producer,
                    )
                } else {
                    return Ok(self.record_noop(input));
                };
            let next_tracks = if let Some(lease) = &self.alpha_lease {
                let mut tracks = lease.tracks()?;
                if !first.mirrored_x {
                    tracks.primary = tracks
                        .primary
                        .updated(
                            obrush_core::SourceGeometryPass {
                                center: first.center,
                                eye_direction: first.eye_direction,
                                local_radius_squared: first.local_radius_squared,
                                original_face_index: original_face,
                            },
                            picked_normal,
                        )
                        .map_err(|e| e.to_string())?;
                }
                if let Some(m) = &mut mirror {
                    tracks.mirror = tracks
                        .mirror
                        .updated(m.geometry_pass, m.picked)
                        .map_err(|e| e.to_string())?;
                    if self.preferences.options.alpha.is_some() && m.pass.local_radius_squared > 0.
                    {
                        if let Some(reason) = tracks.mirror.undefined_normal {
                            return Err(format!(
                                "Source Paint mirrored alpha has an undefined picked normal: {reason:?}"
                            ));
                        }
                        m.pass.alpha_frame = Some(obrush_core::PaintResponseAlphaFrame {
                            look_at: tracks.mirror.frame,
                        });
                    }
                }
                if first.mirrored_x {
                    first = mirror
                        .ok_or("Mirrored-only Source Paint contact missing")?
                        .pass;
                } else if self.preferences.options.alpha.is_some()
                    && first.local_radius_squared > 0.
                {
                    if let Some(reason) = tracks.primary.undefined_normal {
                        return Err(format!(
                            "Source Paint alpha has an undefined picked normal: {reason:?}"
                        ));
                    }
                    first.alpha_frame = Some(obrush_core::PaintResponseAlphaFrame {
                        look_at: tracks.primary.frame,
                    });
                }
                Some(tracks)
            } else {
                None
            };
            let mut options = self.preferences.options.clone();
            options.tablet.pressure = input.raw.pressure;
            let request = PaintResponseRequest {
                kind: BrushKind::Paint,
                operation: PaintResponseOperation::OrdinaryDab,
                in_contact: input.in_contact,
                options,
                first,
                mirrored: if first.mirrored_x {
                    None
                } else {
                    mirror.map(|m| m.pass)
                },
            };
            if request.options.write_roughness || request.options.write_metallic {
                doc.begin_material_paint_stroke();
            }
            doc.begin_source_attribute_history(self.subtool)
                .map_err(|e| e.to_string())?;
            let mut apply = || {
                if let Some(state) = self.retained_normal_state.as_ref() {
                    doc.require_active_mesh_mut()?
                        .apply_paint_response_with_retained_source_controls(&request, state)
                } else {
                    doc.require_active_mesh_mut()?
                        .apply_paint_response_with_source_normals(
                            &request,
                            self.normal_field
                                .as_ref()
                                .ok_or("Prepared Source Paint has no normal field")?,
                        )
                }
            };
            let result = if let Some(tracks) = next_tracks {
                let result = self
                    .alpha_lease
                    .as_mut()
                    .ok_or("Source Paint alpha lease missing")?
                    .commit(tracks, apply)?;
                self.last_alpha_track = Some(tracks.primary);
                if mirror.is_some() {
                    self.last_mirror_alpha_track = Some(tracks.mirror);
                }
                result
            } else {
                apply()?
            };
            self.last_normal_report = Some(normal_policy);
            self.last_normal_producer_report = Some(producer_report);
            self.last_picked_normal = picked_normal.raw_report();
            self.last_picked_normal_disposition = Some(picked_normal);
            self.last_pass = Some(first);
            self.last_triangle = Some(triangle);
            self.last_mirror = mirror;
            self.last_input = Some(input);
            self.last_report = Some(result.clone());
            Ok(result)
        })();
        if let Err(e) = &outcome {
            self.finish_alpha(true);
            if let Some(owner) = self.warm_history_owner.as_ref() {
                let restored = doc
                    .reject_retained_source_paint_stroke(owner, e.clone())
                    .is_ok();
                self.warm_history_rollback = Some(restored);
                if restored {
                    self.last_report = Some(PaintResponseReport::default());
                    self.last_normal_report = None;
                    self.last_normal_producer_report = None;
                    self.last_picked_normal = None;
                    self.last_picked_normal_disposition = None;
                    self.last_pass = None;
                    self.last_triangle = None;
                    self.last_mirror = None;
                }
            }
            self.failed = Some(e.clone());
        }
        outcome
    }
}
/// Actual normal UI chooses this explicit admission for a captured Lock.
/// The old ordinary blocker(locked=true) remains a truthful ordinary-only API.
pub const SOURCE_PAINT_LOCK_CONTROL: &str = "source_paint.lock_position";
pub fn render_lock_control(ui: &mut eframe::egui::Ui, locked: &mut bool) -> eframe::egui::Response {
    ui.checkbox(
        locked,
        crate::ui_text::caption(crate::ui_text::CaptionKey::C189B82C8A250),
    )
}

/// Called after publishing actual release progress, so an origin restoration
/// cannot be dirtied again by that same progress. Useful edits keep their flags.
pub(crate) fn restore_locked_display(
    dirty: &mut bool,
    painted: &mut bool,
    original: Option<(bool, bool)>,
    at_original: bool,
) {
    if at_original && let Some((old_dirty, old_painted)) = original {
        *dirty = old_dirty;
        *painted = old_painted;
    }
}

fn ordinary_tracked_blocker(
    doc: &Document,
    prefs: &SourcePaintPreferences,
    settings: &BrushSettings,
) -> Result<(), String> {
    prefs.validate()?;
    let mut ordinary = settings.clone();
    ordinary.alpha = None;
    ordinary.symmetry_x = false;
    blocker(doc, &ordinary, false)
}

pub fn stroke_blocker(
    doc: &Document,
    prefs: &SourcePaintPreferences,
    settings: &BrushSettings,
    locked: bool,
) -> Result<(), String> {
    if !locked {
        return ordinary_tracked_blocker(doc, prefs, settings);
    }
    prefs.validate()?;
    blocker(doc, settings, false)?;
    doc.source_paint_locked_blocker(&prefs.options)
}
impl SourcePaintSession {
    pub fn prepare_tracked_for_stroke(
        doc: &mut Document,
        prefs: &SourcePaintPreferences,
        settings: &BrushSettings,
        locked: bool,
        continuous: bool,
        input: SourcePaintInput,
        tracks: crate::source_geometry_host::SharedSourceGeometryAlphaTracks,
    ) -> Result<SharedSourcePaint, String> {
        if locked {
            return Self::prepare_for_stroke(doc, prefs, settings, true, continuous, input);
        }
        ordinary_tracked_blocker(doc, prefs, settings)?;
        let mut ordinary = settings.clone();
        ordinary.alpha = None;
        ordinary.symmetry_x = false;
        let mut captured = prefs.clone();
        captured.options.alpha = None;
        let session = Self::prepare(doc, &captured, &ordinary, false, input)?;
        let mut s = session
            .lock()
            .map_err(|_| "Source Paint capture poisoned")?;
        s.preferences.options.alpha = settings.alpha.clone();
        if settings.symmetry_x {
            let center = doc
                .source_geometry_interaction_center()
                .map_err(|e| e.to_string())?;
            s.mirror_plane = Some(
                doc.source_geometry_mirror_plane(center, prefs.mirror_offset, [1., 0., 0.])
                    .map_err(|e| e.to_string())?,
            );
        }
        let mut lease = crate::source_geometry_host::SourcePaintAlphaLease::begin(tracks)?;
        if input.in_contact
            && let Some((pass, hit, _, picked, _)) = s.prepare_contact(doc, input)?
        {
            let mut next = lease.tracks()?;
            next.primary = next
                .primary
                .updated(
                    obrush_core::SourceGeometryPass {
                        center: pass.center,
                        eye_direction: pass.eye_direction,
                        local_radius_squared: pass.local_radius_squared,
                        original_face_index: hit.original_face_index.unwrap_or(hit.triangle_index),
                    },
                    picked,
                )
                .map_err(|e| e.to_string())?;
            if let Some(mirror) = s.prepare_mirror_contact(doc, input, true)? {
                next.mirror = next
                    .mirror
                    .updated(mirror.geometry_pass, mirror.picked)
                    .map_err(|e| e.to_string())?;
                s.last_mirror_alpha_track = Some(next.mirror);
            }
            lease.publish(next)?;
            s.last_alpha_track = Some(next.primary);
        }
        s.alpha_lease = Some(lease);
        drop(s);
        Ok(session)
    }
    pub(crate) fn capture_cursor(&mut self, query: Arc<obrush_core::SourcePreparedPickQuery>) {
        self.cursor_query = Some(query);
    }
    /// The host captures this sealed geometry before Begin. Fixed-topology
    /// Paint only changes attributes; every geometry command closes the stroke.
    /// Failed/foreign/expired active-part bindings cannot submit captured dots.
    pub(crate) fn cursor_capture(
        &self,
        doc: &Document,
    ) -> Option<crate::source_paint_cursor::Capture> {
        if self.failed.is_some()
            || self.is_locked()
            || Some(self.subtool) != doc.active_subtool_id()
            || self.level != doc.current_level()
            || !doc.active_subtool_visible()
            || self.pose.map(|p| p.map(f32::to_bits))
                != doc
                    .active_object_metadata()
                    .map(|m| m.affine.local_to_world.map(f32::to_bits))
        {
            return None;
        }
        Some(crate::source_paint_cursor::Capture {
            query: self.cursor_query.as_ref()?.clone(),
            pose: doc.active_object_metadata().map(|m| m.affine.clone()),
            plane: self.mirror_plane,
        })
    }
    pub(crate) fn finish_alpha(&mut self, restore: bool) {
        if let Some(lease) = &mut self.alpha_lease {
            lease.finish(restore || self.failed.is_some());
        }
    }
    pub fn is_locked(&self) -> bool {
        self.locked_stroke.is_some()
    }
    pub fn is_locked_at_original(&self) -> bool {
        self.locked_stroke.as_ref().is_some_and(|s| s.at_original())
    }
    pub fn locked_mode(&self) -> Option<obrush_core::SourcePaintLockedMode> {
        self.locked_stroke.as_ref().map(|s| s.mode())
    }
    pub fn prepare_for_stroke(
        doc: &mut Document,
        prefs: &SourcePaintPreferences,
        settings: &BrushSettings,
        locked: bool,
        continuous: bool,
        input: SourcePaintInput,
    ) -> Result<SharedSourcePaint, String> {
        if !locked {
            return Self::prepare(doc, prefs, settings, false, input);
        }
        stroke_blocker(doc, prefs, settings, true)?;
        let anchor = crate::locked_anchor::LockedAnchor::new(input.screen)?;
        let session = Self::prepare(doc, prefs, settings, false, input)?;
        let mut captured = session
            .lock()
            .map_err(|_| "Source Paint Lock capture mutex failed")?;
        let mode = if continuous {
            obrush_core::SourcePaintLockedMode::ContinuousAccumulation
        } else {
            obrush_core::SourcePaintLockedMode::AbsolutePreview
        };
        let engine = doc.begin_source_paint_locked(&prefs.options, mode)?;
        captured.locked_stroke = Some(engine);
        captured.locked_anchor = Some(anchor);
        captured.locked_viewport = Some(input.view.viewport);
        drop(captured);
        Ok(session)
    }
    fn locked_sample(
        &mut self,
        doc: &Document,
        input: SourcePaintInput,
    ) -> Result<
        (
            obrush_core::SourcePaintLockedSample,
            Option<obrush_core::PaintResponsePass>,
            Option<usize>,
        ),
        String,
    > {
        input.validate()?;
        if Some(self.subtool) != doc.active_subtool_id()
            || self.level != doc.current_level()
            || self.pose.map(|p| p.map(f32::to_bits))
                != doc
                    .active_object_metadata()
                    .map(|m| m.affine.local_to_world.map(f32::to_bits))
            || self.locked_viewport.map(|p| p.map(f64::to_bits))
                != Some(input.view.viewport.map(f64::to_bits))
        {
            return Err("Source Paint Lock captured part/level/pose/viewport changed".into());
        }
        self.validate_captured_producer(doc)?;
        let anchor = self
            .locked_anchor
            .ok_or("Source Paint Lock has no captured anchor")?;
        let frame = anchor.sample(input.screen)?;
        let crate::locked_anchor::LockedFrame::Dab {
            screen_radius,
            anchored_pick,
            direction,
        } = frame
        else {
            self.last_locked_preparation = Some(serde_json::json!({"press64":anchor.press(),
                "pointer64":input.screen,"screen_radius64":0.0,"anchored_pick64":null,
                "disposition":"SourceUndefinedZeroTravel","raw_pressure64":input.raw.pressure,
                "pen":input.raw.pen,"browser_number":input.raw.browser_number,"absent":input.raw.absent}));
            return Ok((obrush_core::SourcePaintLockedSample::ZeroTravel, None, None));
        };
        self.last_locked_preparation = Some(serde_json::json!({"press64":anchor.press(),
            "pointer64":input.screen,"screen_radius64":screen_radius,"anchored_pick64":anchored_pick,
            "direction64":direction,"raw_pressure64":input.raw.pressure,"pen":input.raw.pen,
            "browser_number":input.raw.browser_number,"absent":input.raw.absent,
            "world_to_clip64":input.view.world_to_clip,"viewport64":input.view.viewport}));
        if !input.in_contact {
            return Ok((obrush_core::SourcePaintLockedSample::NoContact, None, None));
        }
        let travel = obrush_core::LockTravelRadius::new(screen_radius)?;
        let prepared = obrush_core::prepare_source_paint_locked_pass(
            doc.require_active_mesh()?,
            doc.active_object_metadata().map(|m| &m.affine),
            obrush_core::PaintPreparationInput {
                view: input.view,
                screen: anchored_pick,
                screen_radius,
                pressure: input.raw.pressure,
                radius_factor: self.preferences.radius_factor,
            },
            travel,
        )?;
        let Some((pass, hit)) = prepared else {
            return Ok((obrush_core::SourcePaintLockedSample::Miss, None, None));
        };
        if let Some(report) = self.last_locked_preparation.as_mut() {
            report["local_center64"] = serde_json::json!(pass.center);
            report["local_radius_squared64"] = serde_json::json!(pass.local_radius_squared);
            report["eye64"] = serde_json::json!(pass.eye_direction);
            report["triangle_index"] = serde_json::json!(hit.triangle_index);
            report["original_face_index"] = serde_json::json!(hit.original_face_index);
        }
        Ok((
            obrush_core::SourcePaintLockedSample::Dab {
                pass,
                face_index: hit.original_face_index.unwrap_or(hit.triangle_index),
                pressure: input.raw.pressure,
            },
            Some(pass),
            Some(hit.triangle_index),
        ))
    }
    pub fn locked_timer_ready(
        &mut self,
        doc: &Document,
        input: SourcePaintInput,
    ) -> Result<bool, String> {
        let (sample, _, _) = self.locked_sample(doc, input)?;
        // At zero travel the timer may consume a skip permit with no fabricated
        // hit/kernel and no restoration of existing accumulated work.
        Ok(matches!(
            sample,
            obrush_core::SourcePaintLockedSample::Dab { .. }
                | obrush_core::SourcePaintLockedSample::ZeroTravel
        ))
    }
    fn apply_locked(
        &mut self,
        doc: &mut Document,
        input: SourcePaintInput,
    ) -> Result<PaintResponseReport, String> {
        let prepared = self.locked_sample(doc, input);
        let (sample, pass, triangle) = match prepared {
            Ok(v) => v,
            Err(error) => {
                let _ = self.cancel_locked(doc);
                self.failed = Some(error.clone());
                return Err(error);
            }
        };
        let engine = self
            .locked_stroke
            .as_mut()
            .ok_or("Source Paint Lock engine missing")?;
        let result = doc.apply_source_paint_locked(engine, sample);
        match result {
            Ok(report) => {
                self.last_input = Some(input);
                self.last_pass = pass;
                self.last_triangle = triangle;
                self.last_normal_producer_report = report.producer;
                self.last_normal_report = report.producer.map(|p| p.normal_policy);
                self.last_picked_normal = report.picked_normal.and_then(|p| p.raw_report());
                self.last_picked_normal_disposition = report.picked_normal;
                self.last_report = Some(report.live.clone());
                self.last_locked_report = Some(report.clone());
                Ok(report.live)
            }
            Err(error) => {
                self.failed = Some(error.clone());
                Err(error)
            }
        }
    }
    pub fn cancel_locked(&mut self, doc: &mut Document) -> Result<(), String> {
        let Some(engine) = self.locked_stroke.as_mut() else {
            return Ok(());
        };
        if engine.is_closed() {
            return Ok(());
        }
        let report = doc.cancel_source_paint_locked(engine)?;
        self.last_report = Some(report.live.clone());
        self.last_locked_report = Some(report);
        Ok(())
    }
    pub fn finish_locked(&mut self, doc: &mut Document) -> Result<(), String> {
        let engine = self
            .locked_stroke
            .as_mut()
            .ok_or("Source Paint Lock finish needs a captured owner")?;
        if engine.is_closed() {
            return Ok(());
        }
        let result = doc.end_source_paint_locked(engine);
        if let Err(error) = &result {
            self.failed = Some(error.clone());
        }
        result
    }
}

pub fn apply_shared(
    doc: &mut Document,
    session: &SharedSourcePaint,
    input: SourcePaintInput,
) -> Result<PaintResponseReport, String> {
    session
        .lock()
        .map_err(|_| "Source Paint session lock failed")?
        .apply(doc, input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dab_queue::DabQueue;
    use eframe::egui;
    use obrush_core::{MaskConversionPolicy, Mesh};
    fn source_preferences() -> SourcePaintPreferences {
        SourcePaintPreferences {
            mode: PaintMode::SourceOrdinary,
            ..Default::default()
        }
    }
    fn doc() -> Document {
        let mut m = Mesh::from_obj("v -0.1 -0.1 0\nv 0.1 -0.1 0\nv 0 0.1 0\nf 1 2 3\n").unwrap();
        m.install_mask_channel(vec![1.; 3], MaskConvention::UnprotectedF32V1)
            .unwrap();
        Document::new(m)
    }
    fn legacy() -> BrushSettings {
        BrushSettings {
            kind: BrushKind::Paint,
            symmetry_x: false,
            adaptive_detail: false,
            ..Default::default()
        }
    }
    fn input() -> SourcePaintInput {
        let c = Camera {
            yaw: 0.,
            pitch: 0.,
            ..Default::default()
        };
        capture(
            &c,
            Pos2::new(500., 500.),
            Rect::from_min_size(Pos2::ZERO, egui::vec2(1000., 1000.)),
            &RawPaintInput::default(),
        )
        .unwrap()
    }
    fn session(d: &Document) -> SharedSourcePaint {
        SourcePaintSession::prepare(d, &source_preferences(), &legacy(), false, input()).unwrap()
    }
    #[test]
    fn browser_number_and_terminal_screen_survive_legacy_mouse_overlay() {
        let mut r = RawPaintInput::default();
        r.receive(0.3000000001, true, false, true, Some([500.00000001, 499.]));
        r.receive(0., true, true, true, Some([501.00000001, 498.]));
        r.receive(1., false, false, false, None);
        let p = r.sample().unwrap();
        assert_eq!(p.pressure, 0.3000000001);
        assert!(p.browser_number && p.pen);
        assert_eq!(r.position(Pos2::ZERO), [501.00000001, 498.]);
        r.finish();
        assert_eq!(r.sample().unwrap().pressure, 0.5);
    }
    #[test]
    fn legacy_release_hook_cannot_quantize_actual_source_release_coordinates() {
        let mut r = RawPaintInput::default();
        r.receive(0.3000000001, true, false, true, Some([500.00000001, 499.]));
        r.receive(0.0, true, true, false, None);
        r.receive(0.0, true, true, true, Some([501.00000001, 498.]));
        assert_eq!(r.sample().unwrap().pressure, 0.3000000001);
        assert!(r.sample().unwrap().browser_number);
        assert_eq!(r.position(Pos2::ZERO), [501.00000001, 498.]);
    }
    #[test]
    fn mouse_and_native_absent_use_source_neutral_half() {
        assert_eq!(RawPaintInput::default().sample().unwrap().pressure, 0.5);
        let mut r = RawPaintInput::default();
        r.receive(1., false, false, true, None);
        assert_eq!(r.sample().unwrap().pressure, 0.5);
        assert!(r.sample().unwrap().absent);
    }
    #[test]
    fn invalid_raw_input_remains_an_error_instead_of_full_pressure() {
        let mut r = RawPaintInput::default();
        r.receive(f64::NAN, true, false, true, None);
        assert!(r.sample().is_err());
        r.receive(0.2, true, false, true, None);
        assert_eq!(r.sample().unwrap().pressure, 0.2);
    }
    #[test]
    fn unprepared_ui_zoom_is_rejected_instead_of_misinterpreting_raw_css_points() {
        let c = Camera::default();
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 1000.0));
        assert!(capture_at_zoom(&c, rect.center(), rect, &RawPaintInput::default(), 1.0).is_ok());
        for scale in [0.5, 1.25, 2.0, f32::NAN] {
            assert!(
                capture_at_zoom(&c, rect.center(), rect, &RawPaintInput::default(), scale).is_err()
            );
        }
    }
    #[test]
    fn source_selection_does_not_implicitly_convert_protected_masks() {
        let mut d = Document::new(Mesh::sphere(1));
        let before = d.to_json().unwrap();
        assert!(
            SourcePaintSession::prepare(&d, &source_preferences(), &legacy(), false, input())
                .is_err()
        );
        assert_eq!(d.to_json().unwrap(), before);
        d.convert_active_subtool_masks(
            MaskConvention::UnprotectedF32V1,
            MaskConversionPolicy::RequireRoundtripExact,
        )
        .unwrap();
        assert_eq!(
            d.require_active_mesh().unwrap().mask_convention,
            MaskConvention::UnprotectedF32V1
        );
    }
    #[test]
    fn empty_document_rejects_without_a_placeholder_or_history() {
        let d = Document::empty();
        assert!(
            SourcePaintSession::prepare(&d, &source_preferences(), &legacy(), false, input())
                .is_err()
        );
        assert!(d.is_empty());
        assert!(!d.can_undo());
    }
    #[test]
    fn unsupported_options_are_explicit_real_admission_blockers() {
        let d = doc();
        let mut s = legacy();
        s.symmetry_x = true;
        assert!(blocker(&d, &s, false).unwrap_err().contains("mirrored"));
        s.symmetry_x = false;
        s.invert = true;
        assert!(blocker(&d, &s, false).is_err());
        s.invert = false;
        s.alpha = Some(obrush_core::RasterAlpha::new(1, 1, vec![255]).unwrap());
        assert!(blocker(&d, &s, false).unwrap_err().contains("alpha"));
    }
    #[test]
    fn source_direct_and_actual_queue_dispatch_produce_exact_project_channels() {
        let mut direct = doc();
        let mut queued = doc();
        let a = session(&direct);
        let b = session(&queued);
        direct.begin_stroke();
        apply_shared(&mut direct, &a, input()).unwrap();
        direct.end_stroke();
        let mut q = DabQueue::default();
        q.begin();
        q.source_paint_dab(b, input());
        let p = q.process(&mut queued, 1, None);
        assert!(p.changed && p.painted && p.error.is_none());
        q.release(&mut queued);
        assert_eq!(direct.to_json().unwrap(), queued.to_json().unwrap());
    }
    #[test]
    fn many_actual_source_dabs_keep_one_complete_undo_redo() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let a = session(&d);
        let mut q = DabQueue::default();
        q.begin();
        for _ in 0..4 {
            q.source_paint_dab(a.clone(), input());
            q.process(&mut d, 1, None);
        }
        q.release(&mut d);
        let after = d.to_json().unwrap();
        assert_ne!(before, after);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn captured_response_preferences_ignore_later_ui_candidate_changes() {
        let mut d = doc();
        let mut prefs = source_preferences();
        let a = SourcePaintSession::prepare(&d, &prefs, &legacy(), false, input()).unwrap();
        prefs.options.intensity = 0.;
        d.begin_stroke();
        assert!(apply_shared(&mut d, &a, input()).unwrap().changed_vertices > 0);
        d.end_stroke();
    }
    #[test]
    fn source_no_channel_noop_keeps_redo_and_absent_materials() {
        let mut d = doc();
        d.invert_mask();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut prefs = source_preferences();
        prefs.options.write_rgb = false;
        prefs.options.write_roughness = false;
        prefs.options.write_metallic = false;
        let a = SourcePaintSession::prepare(&d, &prefs, &legacy(), false, input()).unwrap();
        d.begin_stroke();
        assert_eq!(
            apply_shared(&mut d, &a, input()).unwrap().changed_vertices,
            0
        );
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.can_redo());
        assert!(d.require_active_mesh().unwrap().materials.is_none());
    }
    #[test]
    fn source_no_contact_skips_radius_zero_without_material_allocation() {
        let mut d = doc();
        let a = session(&d);
        let before = d.to_json().unwrap();
        let mut p = input();
        p.in_contact = false;
        p.raw.pen = true;
        p.raw.pressure = 0.;
        d.begin_stroke();
        assert_eq!(apply_shared(&mut d, &a, p).unwrap().changed_vertices, 0);
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.require_active_mesh().unwrap().materials.is_none());
    }
    #[test]
    fn a_failed_source_session_prevents_queued_release_or_later_work() {
        let mut d = doc();
        let a = session(&d);
        let before = d.to_json().unwrap();
        let mut bad = input();
        bad.raw.pressure = f64::NAN;
        let mut q = DabQueue::default();
        q.begin();
        q.source_paint_dab(a.clone(), bad);
        q.source_paint_dab(a, input());
        let p = q.process(&mut d, 1, None);
        assert!(p.error.is_some());
        q.release(&mut d);
        assert!(q.is_empty());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
    }
    #[test]
    fn real_source_path_retains_double_samples_and_bounds_overload() {
        let d = doc();
        let a = session(&d);
        let mut input = input();
        input.screen = [2000.00000001, 500.];
        let rows = a.lock().unwrap().path_samples(input).unwrap();
        assert_eq!(rows.len(), 32);
        assert_eq!(rows.last().unwrap().screen, input.screen);
        assert_ne!(rows[0].screen[0], f64::from(rows[0].screen[0] as f32));
    }
    #[test]
    fn zero_pressure_press_with_full_radius_factor_is_noop_until_contact() {
        let mut d = doc();
        let mut prefs = source_preferences();
        prefs.radius_factor = 1.0;
        let mut zero = input();
        zero.raw.pen = true;
        zero.raw.pressure = 0.0;
        zero.in_contact = false;
        let before = d.to_json().unwrap();
        let a = SourcePaintSession::prepare(&d, &prefs, &legacy(), false, zero).unwrap();
        d.begin_stroke();
        assert_eq!(apply_shared(&mut d, &a, zero).unwrap().changed_vertices, 0);
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
        d.begin_stroke();
        assert!(apply_shared(&mut d, &a, input()).unwrap().changed_vertices > 0);
        d.end_stroke();
    }
    #[test]
    fn temporary_tool_is_not_silently_treated_as_source_paint() {
        let d = doc();
        for kind in [BrushKind::Smooth, BrushKind::Mask, BrushKind::Draw] {
            let mut options = legacy();
            options.kind = kind;
            assert!(
                SourcePaintSession::prepare(&d, &source_preferences(), &options, false, input())
                    .is_err()
            );
        }
        let prefs = SourcePaintPreferences::default();
        assert!(SourcePaintSession::prepare(&d, &prefs, &legacy(), false, input()).is_err());
    }
    #[test]
    fn finite_unbounded_travel_is_rejected_without_advancing_path_origin() {
        let d = doc();
        let a = session(&d);
        let mut bad = input();
        bad.screen[0] = 1.0e100;
        assert!(a.lock().unwrap().path_samples(bad).is_err());
        let mut good = input();
        good.screen[0] += 15.0;
        let rows = a.lock().unwrap().path_samples(good).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].screen, good.screen);
    }
    #[test]
    fn invalid_sample_is_rejected_even_when_it_claims_no_contact() {
        let mut d = doc();
        let a = session(&d);
        let before = d.to_json().unwrap();
        let mut bad = input();
        bad.raw.pen = true;
        bad.raw.pressure = f64::NAN;
        bad.in_contact = false;
        d.begin_stroke();
        assert!(apply_shared(&mut d, &a, bad).is_err());
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(a.lock().unwrap().failed.is_some());
        assert!(!d.can_undo());
    }
    #[test]
    fn source_color_material_stroke_preserves_geometry_masks_and_uv() {
        let mut d = doc();
        let before = d.require_active_mesh().unwrap().clone();
        let a = session(&d);
        d.begin_stroke();
        apply_shared(&mut d, &a, input()).unwrap();
        d.end_stroke();
        let after = d.require_active_mesh().unwrap();
        assert_eq!(after.positions, before.positions);
        assert_eq!(after.triangles, before.triangles);
        assert_eq!(
            after.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            before.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        assert_eq!(after.normals, before.normals);
        assert!(after.materials.is_some());
    }
    #[test]
    fn actual_source_profile_queue_uses_geometry_normals_not_saved_legacy_directions() {
        let mut d = doc();
        d.require_active_mesh_mut()
            .unwrap()
            .normals
            .fill(obrush_core::Vec3::NEG_Z);
        let before = d.require_active_mesh().unwrap().normals.clone();
        let mut prefs = source_preferences();
        prefs.options.culling = true;
        let a = SourcePaintSession::prepare(&d, &prefs, &legacy(), false, input()).unwrap();
        let mut q = DabQueue::default();
        q.begin();
        q.source_paint_dab(a.clone(), input());
        let result = q.process(&mut d, 1, None);
        q.release(&mut d);
        assert!(result.changed && result.painted && result.error.is_none());
        assert_eq!(d.require_active_mesh().unwrap().normals, before);
        let locked = a.lock().unwrap();
        let field = locked.normal_field.as_ref().unwrap();
        assert!(field.vertex_normals().iter().all(|n| n.z > 0.0));
        assert_eq!(
            locked.diagnostics()["normal_policy"]["policy"],
            "StaticOriginalFaces"
        );
    }
    #[test]
    fn actual_source_session_reuses_normals_across_color_and_material_ticks_without_geometry_edits()
    {
        let mut d = doc();
        let a = session(&d);
        let before = d.require_active_mesh().unwrap().normals.clone();
        d.begin_material_paint_stroke();
        apply_shared(&mut d, &a, input()).unwrap();
        let words = a
            .lock()
            .unwrap()
            .normal_field
            .as_ref()
            .unwrap()
            .vertex_normals()
            .to_vec();
        apply_shared(&mut d, &a, input()).unwrap();
        d.end_stroke();
        assert_eq!(
            a.lock()
                .unwrap()
                .normal_field
                .as_ref()
                .unwrap()
                .vertex_normals(),
            words
        );
        assert_eq!(d.require_active_mesh().unwrap().normals, before);
        assert!(d.undo());
        assert!(d.redo());
    }
    #[test]
    fn source_no_contact_clears_normal_provenance_and_leaves_captured_mesh_exact() {
        let mut d = doc();
        let a = session(&d);
        d.begin_material_paint_stroke();
        apply_shared(&mut d, &a, input()).unwrap();
        d.end_stroke();
        let before = d.to_json().unwrap();
        let mut none = input();
        none.raw = RawPressureSample {
            pressure: 0.0,
            pen: true,
            browser_number: true,
            absent: false,
        };
        none.in_contact = false;
        assert_eq!(apply_shared(&mut d, &a, none).unwrap().changed_vertices, 0);
        assert_eq!(
            a.lock().unwrap().diagnostics()["normal_policy"],
            serde_json::Value::Null
        );
        assert_eq!(before, d.to_json().unwrap());
    }
    #[test]
    fn actual_source_preparation_records_number_idw_and_preserves_saved_normals() {
        let mut d = doc();
        d.require_active_mesh_mut()
            .unwrap()
            .normals
            .fill(obrush_core::Vec3::NEG_Z);
        let normals = d.require_active_mesh().unwrap().normals.clone();
        let a = session(&d);
        d.begin_material_paint_stroke();
        apply_shared(&mut d, &a, input()).unwrap();
        d.end_stroke();
        let session = a.lock().unwrap();
        let report = session.last_picked_normal.as_ref().unwrap();
        assert_eq!(report.normal64, [0., 0., 1.]);
        assert!(report.direction_defined);
        assert_eq!(report.local_point64, session.last_pass.unwrap().center);
        assert_eq!(report.corner_count, 3);
        assert_eq!(report.distances64[3], None);
        assert_eq!(d.require_active_mesh().unwrap().normals, normals);
        assert_eq!(
            session.diagnostics()["picked_normal"]["distance_policy"],
            "scaled-compensated-hypot3; Math.hypot bit equivalence unpaired"
        );
    }
    #[test]
    fn valid_pen_no_contact_clears_actual_idw_report_without_model_change() {
        let mut d = doc();
        let a = session(&d);
        d.begin_material_paint_stroke();
        apply_shared(&mut d, &a, input()).unwrap();
        d.end_stroke();
        let before = d.to_json().unwrap();
        assert!(a.lock().unwrap().last_picked_normal.is_some());
        let mut release = input();
        release.in_contact = false;
        release.raw.pen = true;
        release.raw.pressure = 0.;
        release.raw.absent = false;
        assert_eq!(
            apply_shared(&mut d, &a, release).unwrap().changed_vertices,
            0
        );
        assert!(a.lock().unwrap().last_picked_normal.is_none());
        assert_eq!(before, d.to_json().unwrap());
    }
    fn idw_vertex_document(opposed: bool) -> Document {
        let text = if opposed {
            "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\nf 1 3 2\n"
        } else {
            "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n"
        };
        let mut m = Mesh::import_obj(text.as_bytes()).unwrap().mesh;
        m.colors.fill([0.; 3]);
        m.install_mask_channel(vec![1.; 3], MaskConvention::UnprotectedF32V1)
            .unwrap();
        Document::new(m)
    }
    fn idw_vertex_input(screen: [f64; 2]) -> SourcePaintInput {
        SourcePaintInput {
            view: PaintViewCapture {
                world_to_clip: glam::DMat4::IDENTITY.to_cols_array(),
                viewport: [0., 0., 100., 100.],
            },
            screen,
            raw: RawPressureSample::default(),
            in_contact: true,
        }
    }
    fn idw_vertex_preferences() -> SourcePaintPreferences {
        SourcePaintPreferences {
            mode: PaintMode::SourceOrdinary,
            radius: 5.,
            radius_factor: 0.,
            mirror_offset: 0.,
            options: PaintResponseOptions {
                intensity: 0.25,
                hardness: 1.,
                color: [1., 0.5, 0.25],
                roughness: 0.5,
                metallic: 0.75,
                culling: false,
                ..Default::default()
            },
        }
    }
    fn tracked(
        doc: &mut Document,
        prefs: &SourcePaintPreferences,
        settings: &BrushSettings,
        input: SourcePaintInput,
        tracks: crate::source_geometry_host::SharedSourceGeometryAlphaTracks,
    ) -> SharedSourcePaint {
        SourcePaintSession::prepare_tracked_for_stroke(
            doc, prefs, settings, false, false, input, tracks,
        )
        .unwrap()
    }
    #[test]
    fn tracked_mirrored_overlap_blends_twice_and_retains_one_complete_history_step() {
        let mut d = idw_vertex_document(false);
        let before = d.to_json().unwrap();
        let mut p = idw_vertex_preferences();
        p.radius = 100.;
        let mut b = legacy();
        b.symmetry_x = true;
        let i = idw_vertex_input([45., 55.]);
        let s = tracked(&mut d, &p, &b, i, Arc::new(Mutex::new(Default::default())));
        d.begin_material_paint_stroke();
        let result = apply_shared(&mut d, &s, i).unwrap();
        d.end_stroke();
        s.lock().unwrap().finish_alpha(false);
        assert_eq!(result.first_selected_vertices, 3);
        assert_eq!(result.mirrored_selected_vertices, 3);
        let mesh = d.require_active_mesh().unwrap();
        for color in &mesh.colors {
            assert_eq!(
                color.map(f32::to_bits),
                [0.4375f32, 0.21875, 0.109375].map(f32::to_bits)
            );
        }
        let original = obrush_core::DEFAULT_VERTEX_MATERIAL;
        let mut rm = original;
        for _ in 0..2 {
            rm = [
                (f64::from(rm[0]) * 0.75 + 0.5 * 0.25) as f32,
                (f64::from(rm[1]) * 0.75 + 0.75 * 0.25) as f32,
            ];
        }
        assert!(
            mesh.materials
                .as_ref()
                .unwrap()
                .iter()
                .all(|x| x.map(f32::to_bits) == rm.map(f32::to_bits))
        );
        assert_eq!(
            mesh.positions,
            idw_vertex_document(false)
                .require_active_mesh()
                .unwrap()
                .positions
        );
        let session = s.lock().unwrap();
        assert!(
            (session.last_mirror.unwrap().pass.center[0] + session.last_pass.unwrap().center[0])
                .abs()
                < 1e-14
        );
        drop(session);
        let after = d.to_json().unwrap();
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn tracked_mirror_offset_and_alpha_are_captured_from_real_local_picks() {
        let mut d = idw_vertex_document(false);
        let mut p = idw_vertex_preferences();
        p.radius = 100.;
        p.mirror_offset = 0.1;
        let mut b = legacy();
        b.symmetry_x = true;
        b.alpha = Some(
            obrush_core::RasterAlpha::new(
                4,
                4,
                vec![
                    16, 224, 80, 144, 208, 48, 176, 112, 96, 160, 32, 240, 128, 64, 255, 192,
                ],
            )
            .unwrap(),
        );
        let s = tracked(
            &mut d,
            &p,
            &b,
            idw_vertex_input([45., 55.]),
            Arc::new(Mutex::new(Default::default())),
        );
        p.mirror_offset = -0.9;
        b.symmetry_x = false;
        b.alpha = None;
        d.begin_material_paint_stroke();
        let result = apply_shared(&mut d, &s, idw_vertex_input([55., 55.])).unwrap();
        d.end_stroke();
        s.lock().unwrap().finish_alpha(false);
        assert!(result.changed_vertices > 0);
        let capture = s.lock().unwrap();
        assert_eq!(capture.preferences.mirror_offset, 0.1);
        let first = capture.last_pass.unwrap();
        let mirrored = capture.last_mirror.unwrap();
        let plane = capture.mirror_plane.unwrap();
        assert!(plane.origin[0] > 0.);
        assert!(
            (first.center[0] + mirrored.pass.center[0] - 2. * f64::from(plane.origin[0])).abs()
                < 1e-14
        );
        assert_eq!(
            first.local_radius_squared.to_bits(),
            mirrored.pass.local_radius_squared.to_bits()
        );
        assert!(mirrored.pass.mirrored_x);
        assert_eq!(capture.last_alpha_track.unwrap().frame[1], 1.);
        assert_eq!(capture.last_mirror_alpha_track.unwrap().frame[1], -1.);
        assert_eq!(
            capture.preferences.options.alpha.as_ref().unwrap().pixels()[4],
            208
        );
    }
    #[test]
    fn tracked_primary_miss_mirror_hit_uses_actual_zero_radius_and_preserves_redo() {
        let mut mesh =
            Mesh::from_obj("v -0.9 -0.9 0\nv -0.1 -0.9 0\nv -0.5 0.9 0\nv 0.9 0.9 0\nf 1 2 3\n")
                .unwrap();
        mesh.install_mask_channel(vec![1.; 4], MaskConvention::UnprotectedF32V1)
            .unwrap();
        mesh.colors.fill([0.; 3]);
        let mut d = Document::new(mesh);
        let mut p = idw_vertex_preferences();
        p.radius = 100.;
        let press = idw_vertex_input([25., 50.]);
        let old = SourcePaintSession::prepare(&d, &p, &legacy(), false, press).unwrap();
        d.begin_material_paint_stroke();
        assert!(apply_shared(&mut d, &old, press).unwrap().changed_vertices > 0);
        d.end_stroke();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut b = legacy();
        b.symmetry_x = true;
        b.alpha = Some(obrush_core::RasterAlpha::new(1, 1, vec![255]).unwrap());
        let s = tracked(
            &mut d,
            &p,
            &b,
            press,
            Arc::new(Mutex::new(Default::default())),
        );
        d.begin_material_paint_stroke();
        let result = apply_shared(&mut d, &s, idw_vertex_input([75., 50.])).unwrap();
        d.end_stroke();
        s.lock().unwrap().finish_alpha(false);
        assert_eq!(result.changed_vertices, 0);
        assert_eq!(result.first_selected_vertices, 0);
        assert_eq!(result.mirrored_selected_vertices, 0);
        assert_eq!(d.to_json().unwrap(), before);
        let session = s.lock().unwrap();
        assert_eq!(session.last_mirror.unwrap().pass.local_radius_squared, 0.);
        assert!(session.last_pass.unwrap().mirrored_x);
        drop(session);
        assert!(d.redo());
    }
    #[test]
    fn tracked_mirror_rejects_invalid_offset_and_locked_route_still_refuses_symmetry() {
        let d = idw_vertex_document(false);
        let mut p = idw_vertex_preferences();
        let mut b = legacy();
        b.symmetry_x = true;
        assert!(stroke_blocker(&d, &p, &b, false).is_ok());
        assert!(stroke_blocker(&d, &p, &b, true).is_err());
        assert!(
            SourcePaintSession::prepare(&d, &p, &b, false, idw_vertex_input([45., 55.])).is_err()
        );
        for offset in [f64::NAN, f64::INFINITY, 1.0000000001, -1.0000000001] {
            p.mirror_offset = offset;
            assert!(stroke_blocker(&d, &p, &b, false).is_err());
        }
    }
    #[test]
    fn tracked_asymmetric_alpha_changes_rgb_materials_with_whole_undo_redo() {
        let mut d = idw_vertex_document(false);
        let before = d.to_json().unwrap();
        let mut p = idw_vertex_preferences();
        p.radius = 100.;
        let mut b = legacy();
        b.alpha = Some(
            obrush_core::RasterAlpha::new(
                4,
                4,
                vec![
                    16, 224, 80, 144, 208, 48, 176, 112, 96, 160, 32, 240, 128, 64, 255, 192,
                ],
            )
            .unwrap(),
        );
        let tracks = Arc::new(Mutex::new(Default::default()));
        let press = idw_vertex_input([45., 55.]);
        let s = tracked(&mut d, &p, &b, press, tracks.clone());
        // Capture must neither paint nor open history, and queued work owns its bitmap.
        assert_eq!(d.to_json().unwrap(), before);
        b.alpha = None;
        d.begin_material_paint_stroke();
        let moved = idw_vertex_input([55., 55.]);
        assert!(apply_shared(&mut d, &s, moved).unwrap().changed_vertices > 0);
        let report = s.lock().unwrap().diagnostics();
        assert_eq!(report["alpha"]["width"], 4);
        // Horizontal motion on the +Z planar fixture: exact independent cardinal frame.
        let frame = s.lock().unwrap().last_alpha_track.unwrap().frame;
        assert_eq!(frame[0], 0.);
        assert_eq!(frame[1], 1.);
        assert_eq!(frame[4], 1.);
        assert_eq!(frame[5], 0.);
        assert_eq!(frame[10], -1.);
        let after_mesh = d.require_active_mesh().unwrap();
        assert_eq!(
            after_mesh.positions,
            idw_vertex_document(false)
                .require_active_mesh()
                .unwrap()
                .positions
        );
        assert!(after_mesh.materials.is_some());
        let pass = s.lock().unwrap().last_pass.unwrap();
        let side = pass.local_radius_squared.sqrt() * std::f64::consts::FRAC_1_SQRT_2;
        let pixels = [
            16., 224., 80., 144., 208., 48., 176., 112., 96., 160., 32., 240., 128., 64., 255.,
            192.,
        ];
        let materials = after_mesh.materials.as_ref().unwrap();
        for (id, point) in after_mesh.positions.iter().enumerate() {
            // Independent cardinal +X stroke projection, nearest texel, hardness 1.
            let x = (f64::from(point.y) - pass.center[1]) / side;
            let y = (f64::from(point.x) - pass.center[0]) / side;
            let alpha = if x.abs() > 1. || y.abs() > 1. {
                0.
            } else {
                let col = ((0.5 - 0.5 * x) * 4.) as usize;
                let row = ((0.5 - 0.5 * y) * 4.) as usize;
                pixels[col + 4 * row] / 255.
            };
            let w = alpha * 0.25;
            let expected = [w as f32, (w * 0.5) as f32, (w * 0.25) as f32];
            assert_eq!(
                after_mesh.colors[id].map(f32::to_bits),
                expected.map(f32::to_bits)
            );
            let original = obrush_core::DEFAULT_VERTEX_MATERIAL;
            let expected = [
                (f64::from(original[0]) * (1. - w) + 0.5 * w) as f32,
                (f64::from(original[1]) * (1. - w) + 0.75 * w) as f32,
            ];
            assert_eq!(materials[id].map(f32::to_bits), expected.map(f32::to_bits));
        }
        d.end_stroke();
        s.lock().unwrap().finish_alpha(false);
        let after = d.to_json().unwrap();
        assert_ne!(after, before);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
        // A completed lease releases immediately even while diagnostic Arc stays alive.
        let next = tracked(&mut d, &p, &b, press, tracks);
        drop(next);
    }
    #[test]
    fn tracked_zero_alpha_preserves_absent_materials_and_actual_redo() {
        let mut d = idw_vertex_document(false);
        d.begin_material_paint_stroke();
        let mut p = idw_vertex_preferences();
        p.radius = 100.;
        let old =
            SourcePaintSession::prepare(&d, &p, &legacy(), false, idw_vertex_input([45., 55.]))
                .unwrap();
        apply_shared(&mut d, &old, idw_vertex_input([45., 55.])).unwrap();
        d.end_stroke();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut b = legacy();
        b.alpha = Some(obrush_core::RasterAlpha::new(1, 1, vec![0]).unwrap());
        let tracks = Arc::new(Mutex::new(Default::default()));
        let s = tracked(&mut d, &p, &b, idw_vertex_input([45., 55.]), tracks);
        d.begin_material_paint_stroke();
        assert_eq!(
            apply_shared(&mut d, &s, idw_vertex_input([55., 55.]))
                .unwrap()
                .changed_vertices,
            0
        );
        d.end_stroke();
        s.lock().unwrap().finish_alpha(false);
        assert_eq!(before, d.to_json().unwrap());
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert!(d.redo());
    }
    #[test]
    fn tracked_frame_ownership_excludes_foreign_strokes_and_restores_on_drop_or_failure() {
        let mut d = idw_vertex_document(false);
        let p = idw_vertex_preferences();
        let b = legacy();
        let tracks = Arc::new(Mutex::new(
            crate::source_geometry_host::SourceGeometryAlphaState::default(),
        ));
        let initial = tracks.lock().unwrap().snapshot();
        let i = idw_vertex_input([45., 55.]);
        let s = tracked(&mut d, &p, &b, i, tracks.clone());
        assert!(
            SourcePaintSession::prepare_tracked_for_stroke(
                &mut d,
                &p,
                &b,
                false,
                false,
                i,
                tracks.clone()
            )
            .is_err()
        );
        drop(s);
        assert!(crate::source_geometry_host::same_tracks(
            initial,
            tracks.lock().unwrap().snapshot()
        ));
        let s = tracked(&mut d, &p, &b, i, tracks.clone());
        let mut invalid = i;
        invalid.screen[0] = f64::NAN;
        assert!(apply_shared(&mut d, &s, invalid).is_err());
        assert!(crate::source_geometry_host::same_tracks(
            initial,
            tracks.lock().unwrap().snapshot()
        ));
        let newer = tracked(&mut d, &p, &b, i, tracks.clone());
        drop(s); // Expired session must not release a newer lease.
        assert!(
            SourcePaintSession::prepare_tracked_for_stroke(&mut d, &p, &b, false, false, i, tracks)
                .is_err()
        );
        drop(newer);
    }
    #[test]
    fn tracked_ordinary_alpha_admission_keeps_lock_rotation_and_inverse_blockers() {
        let mut d = idw_vertex_document(false);
        let p = idw_vertex_preferences();
        let mut b = legacy();
        b.alpha = Some(obrush_core::RasterAlpha::new(1, 1, vec![255]).unwrap());
        assert!(stroke_blocker(&d, &p, &b, false).is_ok());
        assert!(stroke_blocker(&d, &p, &b, true).is_err());
        for variant in 1..3 {
            let mut wrong = b.clone();
            match variant {
                0 => wrong.symmetry_x = true,
                1 => wrong.invert = true,
                _ => wrong.alpha_rotation = 1.,
            };
            assert!(stroke_blocker(&d, &p, &wrong, false).is_err());
        }
        let before = d.to_json().unwrap();
        let tracks = Arc::new(Mutex::new(Default::default()));
        let s = tracked(&mut d, &p, &b, idw_vertex_input([25., 75.]), tracks);
        d.begin_material_paint_stroke();
        // At an exact corner IDW is undefined. Alpha must reject rather than invent a normal.
        assert!(apply_shared(&mut d, &s, idw_vertex_input([25., 75.])).is_err());
        d.end_stroke();
        assert_eq!(before, d.to_json().unwrap());
    }
    #[test]
    fn actual_alpha_none_exact_vertex_rgb_material_one_whole_undo_redo() {
        let mut d = idw_vertex_document(false);
        let before = d.to_json().unwrap();
        let i = idw_vertex_input([25., 75.]);
        let a = SourcePaintSession::prepare(&d, &idw_vertex_preferences(), &legacy(), false, i)
            .unwrap();
        d.begin_material_paint_stroke();
        let report = apply_shared(&mut d, &a, i).unwrap();
        d.end_stroke();
        assert_eq!(report.changed_vertices, 1);
        assert_eq!(report.maximum_weight, 0.25);
        assert!(report.materialized);
        let m = d.require_active_mesh().unwrap();
        assert_eq!(
            m.colors[0].map(f32::to_bits),
            [0x3e800000, 0x3e000000, 0x3d800000]
        );
        assert_eq!(
            m.materials.as_ref().unwrap()[0].map(f32::to_bits),
            [0x3e851eb8, 0x3e7d70a4]
        );
        let diagnostic = a.lock().unwrap().diagnostics();
        assert_eq!(
            diagnostic["picked_normal_disposition"]["state"],
            "undefined"
        );
        assert_eq!(
            diagnostic["picked_normal_disposition"]["reason"]["reason"],
            "exact_vertex_contact"
        );
        assert!(diagnostic["picked_normal"].is_null());
        assert!(diagnostic["failed"].is_null());
        let after = d.to_json().unwrap();
        assert_ne!(before, after);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.undo());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
        assert!(!d.redo());
    }
    #[test]
    fn actual_alpha_none_vertex_material_only_keeps_rgb_and_every_geometry_channel() {
        let mut d = idw_vertex_document(false);
        let original = d.require_active_mesh().unwrap().clone();
        let mut prefs = idw_vertex_preferences();
        prefs.options.write_rgb = false;
        let i = idw_vertex_input([25., 75.]);
        let a = SourcePaintSession::prepare(&d, &prefs, &legacy(), false, i).unwrap();
        d.begin_material_paint_stroke();
        let report = apply_shared(&mut d, &a, i).unwrap();
        d.end_stroke();
        assert_eq!(report.changed_rgb_vertices, 0);
        assert_eq!(report.changed_material_vertices, 1);
        let mut restored = d.require_active_mesh().unwrap().clone();
        assert_eq!(restored.colors, original.colors);
        assert_eq!(
            restored.materials.as_ref().unwrap()[0].map(f32::to_bits),
            [0x3e851eb8, 0x3e7d70a4]
        );
        restored.materials = None;
        assert_eq!(
            serde_json::to_vec(&restored).unwrap(),
            serde_json::to_vec(&original).unwrap()
        );
    }
    #[test]
    fn actual_protected_vertex_undefined_idw_keeps_noop_redo_and_absent_materials() {
        let mut d = idw_vertex_document(false);
        d.require_active_mesh_mut().unwrap().masks.fill(0.);
        d.begin_stroke();
        d.require_active_mesh_mut().unwrap().colors[1] = [0.5; 3];
        d.end_stroke();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let i = idw_vertex_input([25., 75.]);
        let a = SourcePaintSession::prepare(&d, &idw_vertex_preferences(), &legacy(), false, i)
            .unwrap();
        d.begin_material_paint_stroke();
        let r = apply_shared(&mut d, &a, i).unwrap();
        d.end_stroke();
        assert_eq!(r.changed_vertices, 0);
        assert!(!r.materialized);
        assert_eq!(before, d.to_json().unwrap());
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert_eq!(
            a.lock().unwrap().diagnostics()["picked_normal_disposition"]["state"],
            "undefined"
        );
        assert!(d.redo());
        assert_eq!(d.require_active_mesh().unwrap().colors[1], [0.5; 3]);
    }
    #[test]
    fn actual_no_channel_vertex_undefined_idw_keeps_noop_redo_and_no_allocation() {
        let mut d = idw_vertex_document(false);
        d.begin_stroke();
        d.require_active_mesh_mut().unwrap().colors[1] = [0.5; 3];
        d.end_stroke();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut prefs = idw_vertex_preferences();
        prefs.options.write_rgb = false;
        prefs.options.write_roughness = false;
        prefs.options.write_metallic = false;
        let i = idw_vertex_input([25., 75.]);
        let a = SourcePaintSession::prepare(&d, &prefs, &legacy(), false, i).unwrap();
        d.begin_material_paint_stroke();
        let r = apply_shared(&mut d, &a, i).unwrap();
        d.end_stroke();
        assert_eq!(r.changed_vertices, 0);
        assert!(!r.materialized);
        assert_eq!(before, d.to_json().unwrap());
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert!(a.lock().unwrap().diagnostics()["failed"].is_null());
        assert!(d.redo());
    }
    #[test]
    fn actual_alpha_none_zero_direction_reports_zero_and_still_paints_without_culling() {
        let mut d = idw_vertex_document(true);
        let before = d.to_json().unwrap();
        let mut prefs = idw_vertex_preferences();
        prefs.radius = 50.;
        let i = idw_vertex_input([50., 50.]);
        let a = SourcePaintSession::prepare(&d, &prefs, &legacy(), false, i).unwrap();
        d.begin_material_paint_stroke();
        let r = apply_shared(&mut d, &a, i).unwrap();
        d.end_stroke();
        assert_eq!(r.changed_vertices, 3);
        assert!(r.materialized);
        assert!(
            d.require_active_mesh()
                .unwrap()
                .colors
                .iter()
                .all(|c| c.map(f32::to_bits) == [0x3e800000, 0x3e000000, 0x3d800000])
        );
        let report = a.lock().unwrap().diagnostics();
        assert_eq!(
            report["picked_normal_disposition"]["reason"]["reason"],
            "zero_direction"
        );
        assert_eq!(
            report["picked_normal"]["normal64"],
            serde_json::json!([0., 0., 0.])
        );
        assert_eq!(report["picked_normal"]["direction_defined"], false);
        let after = d.to_json().unwrap();
        assert_ne!(before, after);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn actual_zero_direction_culling_keeps_source_acceptance_and_future_alpha_stays_blocked() {
        let mut d = idw_vertex_document(true);
        let before = d.to_json().unwrap();
        let mut prefs = idw_vertex_preferences();
        prefs.radius = 50.;
        prefs.options.culling = true;
        let i = idw_vertex_input([50., 50.]);
        let a = SourcePaintSession::prepare(&d, &prefs, &legacy(), false, i).unwrap();
        d.begin_material_paint_stroke();
        let r = apply_shared(&mut d, &a, i).unwrap();
        d.end_stroke();
        assert_eq!(r.changed_vertices, 3);
        let mesh = d.require_active_mesh().unwrap();
        assert!(
            mesh.colors
                .iter()
                .all(|c| c.map(f32::to_bits) == [0x3e800000, 0x3e000000, 0x3d800000])
        );
        assert!(
            mesh.materials
                .as_ref()
                .unwrap()
                .iter()
                .all(|m| m.map(f32::to_bits) == [0x3e851eb8, 0x3e7d70a4])
        );
        let after = d.to_json().unwrap();
        let mut geometry_before =
            serde_json::from_str::<serde_json::Value>(&before).unwrap()["mesh"].clone();
        let mut geometry_after =
            serde_json::from_str::<serde_json::Value>(&after).unwrap()["mesh"].clone();
        for geometry in [&mut geometry_before, &mut geometry_after] {
            let fields = geometry.as_object_mut().unwrap();
            assert!(fields.remove("colors").is_some());
            let _ = fields.remove("materials");
        }
        assert_eq!(geometry_before, geometry_after);
        assert_ne!(before, after);
        assert!(d.undo());
        assert_eq!(before, d.to_json().unwrap());
        assert!(d.redo());
        assert_eq!(after, d.to_json().unwrap());
        assert_eq!(
            a.lock().unwrap().diagnostics()["picked_normal_disposition"]["reason"]["reason"],
            "zero_direction"
        );
        let mut alpha = prefs;
        alpha.options.alpha = Some(obrush_core::RasterAlpha::new(1, 1, vec![255]).unwrap());
        assert!(SourcePaintSession::prepare(&d, &alpha, &legacy(), false, i).is_err());
        assert_eq!(d.to_json().unwrap(), after);
        let mut release = i;
        release.in_contact = false;
        release.raw.pen = true;
        release.raw.pressure = 0.;
        release.raw.absent = false;
        assert_eq!(
            apply_shared(&mut d, &a, release).unwrap().changed_vertices,
            0
        );
        assert!(a.lock().unwrap().diagnostics()["picked_normal_disposition"].is_null());
        assert!(a.lock().unwrap().diagnostics()["picked_normal"].is_null());
        assert_eq!(d.to_json().unwrap(), after);
    }
}

#[cfg(test)]
mod retained_producer_tests {
    use super::*;
    use crate::dab_queue::DabQueue;
    use obrush_core::{
        SourceGeometryOptions, SourceGeometryPass, SourcePaintNormalProducerKind,
        SourcePickedNormalDisposition, SourcePickedNormalUndefined,
    };
    fn fresh() -> Document {
        let mut m = obrush_core::Mesh::from_obj(
            "v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nf 1 2 3\nf 3 5 4\n",
        )
        .unwrap();
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m.masks.fill(1.);
        m.colors.fill([1.; 3]);
        Document::new(m)
    }
    fn warm() -> Document {
        let mut d = fresh();
        let mut g = d
            .begin_source_geometry_stroke(SourceGeometryOptions {
                intensity: 0.625,
                intensity_factor: 0.,
                ..SourceGeometryOptions::inflate()
            })
            .unwrap();
        let pass = SourceGeometryPass {
            center: [0.; 3],
            eye_direction: [0., 0., -1.],
            local_radius_squared: 4.,
            original_face_index: 0,
        };
        assert_eq!(
            d.apply_source_geometry_dab(&mut g, pass, 0.5, true)
                .unwrap()
                .changed_vertices,
            1
        );
        d.end_source_geometry_stroke(&g).unwrap();
        d
    }
    fn settings() -> BrushSettings {
        BrushSettings {
            kind: BrushKind::Paint,
            symmetry_x: false,
            adaptive_detail: false,
            ..Default::default()
        }
    }
    fn prefs() -> SourcePaintPreferences {
        SourcePaintPreferences {
            mode: PaintMode::SourceOrdinary,
            radius: 10.,
            radius_factor: 0.,
            mirror_offset: 0.,
            options: PaintResponseOptions {
                intensity: 0.5,
                hardness: 1.,
                color: [0., 0.5, 1.],
                culling: false,
                write_rgb: true,
                write_roughness: false,
                write_metallic: false,
                ..Default::default()
            },
        }
    }
    fn input() -> SourcePaintInput {
        SourcePaintInput {
            view: PaintViewCapture {
                world_to_clip: [
                    0.125, 0., 0., 0., 0., 0.125, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
                ],
                viewport: [0., 0., 100., 100.],
            },
            screen: [75., 50.],
            raw: RawPressureSample::default(),
            in_contact: true,
        }
    }
    fn prepare(d: &Document) -> SharedSourcePaint {
        SourcePaintSession::prepare(d, &prefs(), &settings(), false, input()).unwrap()
    }
    fn prior_redo(d: &mut Document) -> String {
        d.begin_stroke();
        d.name = "actual prior future".into();
        d.end_stroke();
        let future = d.to_json().unwrap();
        assert!(d.undo());
        future
    }
    #[test]
    fn failed_tracked_mirror_rolls_back_whole_history_frames_and_clears_contact_diagnostics() {
        let mut d = warm();
        let future = prior_redo(&mut d);
        let before = d.to_json().unwrap();
        let mut p = prefs();
        p.radius = 100.;
        p.mirror_offset = 0.25;
        let mut b = settings();
        b.symmetry_x = true;
        let mut i = input();
        i.screen = [55., 45.];
        let tracks = Arc::new(Mutex::new(
            crate::source_geometry_host::SourceGeometryAlphaState::default(),
        ));
        let initial = tracks.lock().unwrap().snapshot();
        let s = SourcePaintSession::prepare_tracked_for_stroke(
            &mut d,
            &p,
            &b,
            false,
            false,
            i,
            tracks.clone(),
        )
        .unwrap();
        d.begin_stroke();
        assert!(apply_shared(&mut d, &s, i).unwrap().changed_vertices > 0);
        assert!(s.lock().unwrap().last_mirror.is_some());
        i.screen[0] = f64::NAN;
        assert!(apply_shared(&mut d, &s, i).is_err());
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(crate::source_geometry_host::same_tracks(
            initial,
            tracks.lock().unwrap().snapshot()
        ));
        let capture = s.lock().unwrap().diagnostics();
        assert!(capture["mirrored_pass"].is_null());
        assert!(capture["pass64"].is_null());
        assert_eq!(capture["warm_history_rollback"], true);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
    #[test]
    fn warm_tracked_mirror_keeps_the_actual_normal_producer_and_whole_history() {
        let mut d = warm();
        let id = d.active_subtool_id().unwrap();
        let original = d.source_geometry_state(id).unwrap();
        let before = d.to_json().unwrap();
        let mut p = prefs();
        p.radius = 100.;
        p.mirror_offset = 0.25;
        let mut b = settings();
        b.symmetry_x = true;
        let mut i = input();
        i.screen = [55., 45.]; // Interior primary and reflected contacts, not old corner/mirror miss.
        let s = SourcePaintSession::prepare_tracked_for_stroke(
            &mut d,
            &p,
            &b,
            false,
            false,
            i,
            Arc::new(Mutex::new(Default::default())),
        )
        .unwrap();
        d.begin_stroke();
        let result = apply_shared(&mut d, &s, i).unwrap();
        d.end_stroke();
        s.lock().unwrap().finish_alpha(false);
        assert!(result.changed_vertices > 0);
        assert!(result.mirrored_selected_vertices > 0);
        assert!(Arc::ptr_eq(
            &original,
            &d.source_geometry_state(id).unwrap()
        ));
        let session = s.lock().unwrap();
        assert_eq!(
            session.last_normal_producer_report.unwrap().kind,
            SourcePaintNormalProducerKind::RetainedGeometry
        );
        assert!(session.last_mirror.is_some());
        drop(session);
        let after = d.to_json().unwrap();
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(Arc::ptr_eq(
            &original,
            &d.source_geometry_state(id).unwrap()
        ));
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn warm_actual_prepare_direct_contact_provenance_and_readonly_render_producer() {
        let mut d = warm();
        let state = d.source_geometry_state(1).unwrap();
        let before = d.to_json().unwrap();
        let a = prepare(&d);
        {
            let s = a.lock().unwrap();
            assert!(s.normal_field.is_none());
            assert!(Arc::ptr_eq(&state, &s.retained_normal_state().unwrap()));
        }
        d.begin_stroke();
        let r = apply_shared(&mut d, &a, input()).unwrap();
        assert_eq!(r.changed_rgb_vertices, 1);
        d.end_stroke();
        assert_eq!(
            d.require_active_mesh().unwrap().colors[1].map(f32::to_bits),
            [0x3f000000, 0x3f400000, 0x3f800000]
        );
        {
            let s = a.lock().unwrap();
            let report = s.last_normal_producer_report.unwrap();
            assert_eq!(report.kind, SourcePaintNormalProducerKind::RetainedGeometry);
            assert_eq!(report.generation, Some(state.generation()));
            assert_eq!(s.last_pass.unwrap().center, [4., 0., 0.]);
            assert_eq!(s.last_triangle, Some(0));
            assert!(matches!(
                s.last_picked_normal_disposition.unwrap(),
                SourcePickedNormalDisposition::Undefined {
                    reason: SourcePickedNormalUndefined::ExactVertexContact {
                        vertex_id: 1,
                        corner: 1
                    },
                    ..
                }
            ));
            assert!(s.normal_field.is_none());
            assert!(Arc::ptr_eq(&state, &s.retained_normal_state().unwrap()));
        }
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
    }
    #[test]
    fn warm_direct_and_real_queue_have_identical_channels_history_and_release_expiry() {
        let mut direct = warm();
        let mut queued = warm();
        let before = queued.to_json().unwrap();
        let a = prepare(&direct);
        let b = prepare(&queued);
        direct.begin_stroke();
        apply_shared(&mut direct, &a, input()).unwrap();
        direct.end_stroke();
        let mut q = DabQueue::default();
        q.begin();
        q.source_paint_dab(b.clone(), input());
        let p = q.process(&mut queued, 1, None);
        assert!(p.changed && p.painted && p.error.is_none());
        assert_eq!(p.dabs, 1);
        q.release(&mut queued);
        assert!(q.is_empty());
        assert_eq!(queued.to_json().unwrap(), direct.to_json().unwrap());
        let after = queued.to_json().unwrap();
        assert!(apply_shared(&mut queued, &b, input()).is_err());
        assert_eq!(queued.to_json().unwrap(), after);
        assert_eq!(b.lock().unwrap().warm_history_rollback, Some(false));
        assert!(queued.undo());
        assert_eq!(queued.to_json().unwrap(), before);
        assert!(queued.redo());
        assert_eq!(queued.to_json().unwrap(), after);
    }
    #[test]
    fn repeated_actual_timer_application_uses_captured_cache_one_whole_stroke() {
        let mut d = warm();
        let state = d.source_geometry_state(1).unwrap();
        let before = d.to_json().unwrap();
        let a = prepare(&d);
        let mut q = DabQueue::default();
        q.begin();
        for _ in 0..4 {
            q.source_paint_dab(a.clone(), input());
            let p = q.process(&mut d, 1, None);
            assert!(p.changed && p.error.is_none());
        }
        q.release(&mut d);
        assert!(q.is_empty());
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert_eq!(
            d.require_active_mesh().unwrap().colors[1].map(f32::to_bits),
            [0x3d800000, 0x3f080000, 0x3f800000]
        );
        let after = d.to_json().unwrap();
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn warm_material_only_queue_keeps_rgb_and_same_producer_lazy_channels() {
        let mut d = warm();
        let before = d.to_json().unwrap();
        let state = d.source_geometry_state(1).unwrap();
        let mut p = prefs();
        p.options.intensity = 1.;
        p.options.write_rgb = false;
        p.options.write_roughness = true;
        p.options.write_metallic = true;
        p.options.roughness = 0.5;
        p.options.metallic = 1.;
        let a = SourcePaintSession::prepare(&d, &p, &settings(), false, input()).unwrap();
        let mut q = DabQueue::default();
        q.begin();
        q.source_paint_dab(a.clone(), input());
        let progress = q.process(&mut d, 1, None);
        assert!(progress.changed);
        assert!(!progress.painted);
        assert!(progress.error.is_none());
        q.release(&mut d);
        assert_eq!(d.require_active_mesh().unwrap().colors, vec![[1.; 3]; 5]);
        assert_eq!(
            d.require_active_mesh().unwrap().materials.as_ref().unwrap()[1].map(f32::to_bits),
            [0x3f000000, 0x3f800000]
        );
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert!(a.lock().unwrap().normal_field.is_none());
        let after = d.to_json().unwrap();
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn warm_useful_paint_then_stale_rejection_rolls_back_whole_snapshot_and_real_redo() {
        let mut d = warm();
        let future = prior_redo(&mut d);
        let before = d.to_json().unwrap();
        let state = d.source_geometry_state(1).unwrap();
        let a = prepare(&d);
        let mut q = DabQueue::default();
        q.begin();
        q.source_paint_dab(a.clone(), input());
        assert!(q.process(&mut d, 1, None).painted);
        d.require_active_mesh_mut().unwrap().positions[2].z = 0.25;
        q.source_paint_dab(a.clone(), input());
        let rejected = q.process(&mut d, 1, None);
        assert!(rejected.error.is_some());
        q.release(&mut d);
        assert!(q.is_empty());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert_eq!(a.lock().unwrap().warm_history_rollback, Some(true));
        // The actual app's existing finish_stroke_deferred consumes this channel
        // and restores captured dirty/use_vertex_color; this test proves producer.
        assert!(d.take_layer_error().unwrap().contains("reset"));
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
    #[test]
    fn captured_warm_producer_reset_is_rejected_without_fresh_allocation() {
        let mut d = warm();
        let a = prepare(&d);
        let before = d.to_json().unwrap();
        d.reset_source_geometry_normal_state().unwrap();
        assert!(apply_shared(&mut d, &a, input()).is_err());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.source_geometry_state(1).is_none());
        let s = a.lock().unwrap();
        assert!(s.normal_field.is_none());
        assert!(s.last_normal_producer_report.is_none());
        assert!(s.failed.is_some());
    }
    #[test]
    fn foreign_warm_session_failure_cannot_restore_either_independent_document() {
        let mut a_doc = warm();
        let mut b_doc = warm();
        let a = prepare(&a_doc);
        let b = prepare(&b_doc);
        a_doc.begin_stroke();
        b_doc.begin_stroke();
        apply_shared(&mut a_doc, &a, input()).unwrap();
        apply_shared(&mut b_doc, &b, input()).unwrap();
        let aa = a_doc.to_json().unwrap();
        let bb = b_doc.to_json().unwrap();
        assert!(apply_shared(&mut b_doc, &a, input()).is_err());
        assert_eq!(a_doc.to_json().unwrap(), aa);
        assert_eq!(b_doc.to_json().unwrap(), bb);
        assert_eq!(a.lock().unwrap().warm_history_rollback, Some(false));
        assert!(b_doc.take_layer_error().is_none());
        {
            let s = a.lock().unwrap();
            a_doc
                .validate_retained_source_paint_stroke(s.warm_history_owner.as_ref().unwrap())
                .unwrap();
        }
        {
            let s = b.lock().unwrap();
            b_doc
                .validate_retained_source_paint_stroke(s.warm_history_owner.as_ref().unwrap())
                .unwrap();
        }
        a_doc.end_stroke();
        b_doc.end_stroke();
    }
    #[test]
    fn valid_no_contact_warm_sample_clears_reports_and_preserves_real_redo() {
        let mut d = warm();
        let future = prior_redo(&mut d);
        let before = d.to_json().unwrap();
        let state = d.source_geometry_state(1).unwrap();
        let a = prepare(&d);
        let mut none = input();
        none.in_contact = false;
        none.raw = RawPressureSample {
            pressure: 0.,
            pen: true,
            browser_number: true,
            absent: false,
        };
        d.begin_stroke();
        assert_eq!(apply_shared(&mut d, &a, none).unwrap().changed_vertices, 0);
        d.end_stroke();
        let s = a.lock().unwrap();
        assert!(s.last_normal_report.is_none());
        assert!(s.last_normal_producer_report.is_none());
        assert!(s.last_picked_normal_disposition.is_none());
        assert!(s.last_pass.is_none());
        assert!(s.normal_field.is_none());
        drop(s);
        assert_eq!(d.to_json().unwrap(), before);
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
    #[test]
    fn cold_source_paint_keeps_existing_fresh_route_and_no_document_cache() {
        let mut d = fresh();
        let a = prepare(&d);
        d.begin_stroke();
        apply_shared(&mut d, &a, input()).unwrap();
        d.end_stroke();
        let s = a.lock().unwrap();
        assert!(s.retained_normal_state.is_none());
        assert!(s.warm_history_owner.is_none());
        assert!(s.normal_field.is_some());
        assert_eq!(
            s.last_normal_producer_report.unwrap().kind,
            SourcePaintNormalProducerKind::FreshStatic
        );
        assert_eq!(s.last_normal_producer_report.unwrap().generation, None);
        assert!(d.source_geometry_state(1).is_none());
        assert_eq!(
            d.require_active_mesh().unwrap().colors[1].map(f32::to_bits),
            [0x3f000000, 0x3f400000, 0x3f800000]
        );
    }
    #[test]
    fn captured_warm_parameters_survive_later_ui_candidate_changes() {
        let mut d = warm();
        let mut p = prefs();
        let a = SourcePaintSession::prepare(&d, &p, &settings(), false, input()).unwrap();
        p.options.color = [1., 0., 0.];
        p.options.intensity = 0.;
        assert_eq!(p.options.intensity, 0.);
        d.begin_stroke();
        apply_shared(&mut d, &a, input()).unwrap();
        d.end_stroke();
        assert_eq!(
            d.require_active_mesh().unwrap().colors[1].map(f32::to_bits),
            [0x3f000000, 0x3f400000, 0x3f800000]
        );
        assert_eq!(a.lock().unwrap().preferences.options.intensity, 0.5);
        assert_eq!(a.lock().unwrap().preferences.options.color, [0., 0.5, 1.]);
    }
}
