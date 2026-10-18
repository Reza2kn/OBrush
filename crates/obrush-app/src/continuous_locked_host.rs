//! Real current-surface locked timer gesture over legacy OBrush response.
//! Screen anchor math remains f64; core f32 conversions are explicitly reported.
use crate::locked_anchor::{LockedAnchor, LockedFrame};
use crate::source_paint_host::SourcePaintInput;
use glam::{DMat4, DVec3};
use obrush_core::{
    BrushKind, BrushSettings, Document, LockedAccumulationReport, LockedAccumulationSample,
    LockedAccumulator, Vec3, prepare_source_paint_hit,
};

#[derive(Clone, Debug)]
pub struct LockedPreparationReport {
    pub source_sequence: u64,
    pub press64: [f64; 2],
    pub pointer64: [f64; 2],
    pub anchored_pick64: [f64; 2],
    pub screen_radius64: f64,
    pub local_center64: [f64; 3],
    pub local_radius64: f64,
    pub core_center32: [f32; 3],
    pub core_radius32: f32,
    pub core_pressure32: f32,
    pub raw_pressure64: f64,
    pub raw_browser_number: bool,
    pub raw_pressure_absent: bool,
    pub triangle_index: usize,
    pub maximum_center_rounding_error: f64,
    pub radius_rounding_error: f64,
    pub view64: [f64; 16],
    pub viewport64: [f64; 4],
}

impl LockedPreparationReport {
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "source_sequence":self.source_sequence,"press64":self.press64,
            "pointer64":self.pointer64,"anchored_pick64":self.anchored_pick64,
            "screen_radius64":self.screen_radius64,"local_center64":self.local_center64,
            "local_radius64":self.local_radius64,"core_center32":self.core_center32,
            "core_radius32":self.core_radius32,"core_pressure32":self.core_pressure32,
            "raw_pressure64":self.raw_pressure64,"raw_browser_number":self.raw_browser_number,
            "raw_pressure_absent":self.raw_pressure_absent,"triangle_index":self.triangle_index,
            "maximum_center_rounding_error":self.maximum_center_rounding_error,
            "radius_rounding_error":self.radius_rounding_error,"view64":self.view64,
            "viewport64":self.viewport64
        })
    }
}

pub struct PreparedLockedDab {
    pub sample: LockedAccumulationSample,
    pub report: LockedPreparationReport,
}
pub enum PreparedLockedTick {
    Hold,
    Miss,
    Dab(Box<PreparedLockedDab>),
}
impl PreparedLockedTick {
    pub fn anchored_hit(&self) -> bool {
        matches!(self, Self::Dab(_))
    }
}

pub struct ContinuousLockedGesture {
    engine: LockedAccumulator,
    anchor: LockedAnchor,
    subtool: u32,
    level: usize,
    pose: Option<[f32; 16]>,
    viewport: [f64; 4],
    tangent_world64: [f64; 3],
    base_rotation: f32,
    last_preparation: Option<LockedPreparationReport>,
    last_effect: Option<LockedAccumulationReport>,
    failed: Option<String>,
}

impl ContinuousLockedGesture {
    pub fn prepare(
        doc: &Document,
        settings: &BrushSettings,
        input: SourcePaintInput,
        tangent_world64: [f64; 3],
    ) -> Result<Self, String> {
        validate_input(input, settings.pressure)?;
        if !doc.active_subtool_visible() || !LockedAccumulator::supports_kind(settings.kind) {
            return Err(
                "Continuous Lock requires a visible part and an exposed ordinary legacy tool"
                    .into(),
            );
        }
        let mesh = doc.require_active_mesh()?;
        let tangent = DVec3::from_array(tangent_world64);
        if !tangent.is_finite() || tangent.length_squared() == 0.0 {
            return Err("Continuous Lock tangent capture is invalid".into());
        }
        let hit = prepare_source_paint_hit(
            mesh,
            doc.active_object_metadata().map(|m| &m.affine),
            input.view.ray(input.screen)?,
        )?;
        if hit.is_none() {
            return Err("Continuous Lock press missed its active current surface".into());
        }
        let mut effective = settings.clone();
        // Visible effective locked policy, without mutating saved ordinary prefs.
        effective.adaptive_detail = false;
        let engine = LockedAccumulator::prepare(mesh, &effective)?;
        Ok(Self {
            engine,
            anchor: LockedAnchor::new(input.screen)?,
            subtool: doc
                .active_subtool_id()
                .ok_or("No active Continuous Lock part")?,
            level: doc.current_level(),
            pose: doc
                .active_object_metadata()
                .map(|m| m.affine.local_to_world),
            viewport: input.view.viewport,
            tangent_world64,
            base_rotation: settings.alpha_rotation,
            last_preparation: None,
            last_effect: None,
            failed: None,
        })
    }
    pub fn kind(&self) -> BrushKind {
        self.engine.kind()
    }
    pub fn at_original(&self) -> bool {
        self.engine.at_original()
    }
    pub fn paints_rgb(&self) -> bool {
        self.kind() == BrushKind::Paint && self.engine.settings().paint_rgb
    }
    pub fn paints_material(&self) -> bool {
        self.kind() == BrushKind::Paint
            && (self.engine.settings().paint_roughness || self.engine.settings().paint_metallic)
    }
    pub fn prepare_tick(
        &mut self,
        doc: &Document,
        input: SourcePaintInput,
        legacy_pressure: f32,
    ) -> Result<PreparedLockedTick, String> {
        let outcome = self.prepare_tick_inner(doc, input, legacy_pressure);
        if let Err(error) = &outcome {
            self.failed = Some(error.clone());
        }
        outcome
    }
    fn require_binding(&self, doc: &Document) -> Result<(), String> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        if Some(self.subtool) != doc.active_subtool_id()
            || self.level != doc.current_level()
            || !doc.active_subtool_visible()
            || self.pose.map(|p| p.map(f32::to_bits))
                != doc
                    .active_object_metadata()
                    .map(|m| m.affine.local_to_world.map(f32::to_bits))
        {
            return Err("Continuous Lock captured part/level/pose was changed".into());
        }
        Ok(())
    }
    fn prepare_tick_inner(
        &self,
        doc: &Document,
        input: SourcePaintInput,
        legacy_pressure: f32,
    ) -> Result<PreparedLockedTick, String> {
        self.require_binding(doc)?;
        validate_input(input, legacy_pressure)?;
        if self.viewport.map(f64::to_bits) != input.view.viewport.map(f64::to_bits) {
            return Err("Continuous Lock viewport changed; finish this gesture".into());
        }
        let frame = self.anchor.sample(input.screen)?;
        let LockedFrame::Dab {
            screen_radius,
            anchored_pick,
            direction,
        } = frame
        else {
            return Ok(PreparedLockedTick::Hold);
        };
        let mesh = doc.require_active_mesh()?;
        let pose = doc.active_object_metadata().map(|m| &m.affine);
        let Some(hit) = prepare_source_paint_hit(mesh, pose, input.view.ray(anchored_pick)?)?
        else {
            return Ok(PreparedLockedTick::Miss);
        };
        // OBrush precise camera and source row-scale approximation; no source
        // camera/inverse32 or ray/alpha/mirror response equality is claimed.
        let world = DVec3::from_array(hit.world_position);
        let projection = input.view.project(world)?;
        let edge = input.view.unproject(
            [projection[0] + screen_radius, projection[1]],
            projection[2],
        )?;
        let local_radius = ((edge - world).length_squared()
            / pose.map_or(1.0, |p| p.source_scale_squared()))
        .sqrt();
        let radius32 = local_radius as f32;
        if !local_radius.is_finite()
            || local_radius <= 0.0
            || local_radius > 1_000_000.0
            || !radius32.is_finite()
            || radius32 <= 0.0
        {
            return Err(
                "Continuous Lock prepared radius cannot fit its declared legacy core bound".into(),
            );
        }
        let center64 = DVec3::from_array(hit.local_position);
        let center = center64.as_vec3();
        let normal = normal_at(mesh, hit.triangle_index, center64)?;
        let inverse = self.pose.map_or(DMat4::IDENTITY, |m| {
            DMat4::from_cols_array(&m.map(f64::from)).inverse()
        });
        let tangent = inverse
            .transform_vector3(DVec3::from_array(self.tangent_world64))
            .as_vec3();
        let toward_eye = -DVec3::from_array(hit.local_eye_direction).as_vec3();
        if !center.is_finite()
            || !tangent.is_finite()
            || tangent.length_squared() == 0.0
            || !toward_eye.is_finite()
            || toward_eye.length_squared() == 0.0
        {
            return Err(
                "Continuous Lock legacy center/tangent/culling conversion is invalid".into(),
            );
        }
        let rotation = f64::from(self.base_rotation) - direction[1].atan2(direction[0]);
        let sample = LockedAccumulationSample {
            center,
            normal,
            radius: radius32,
            pressure: legacy_pressure,
            alpha_rotation: rotation as f32,
            alpha_tangent: Some(tangent),
            culling_axis: Some(toward_eye),
        };
        let report = LockedPreparationReport {
            source_sequence: self.engine.sequence(),
            press64: self.anchor.press(),
            pointer64: input.screen,
            anchored_pick64: anchored_pick,
            screen_radius64: screen_radius,
            local_center64: hit.local_position,
            local_radius64: local_radius,
            core_center32: center.to_array(),
            core_radius32: radius32,
            core_pressure32: legacy_pressure,
            raw_pressure64: input.raw.pressure,
            raw_browser_number: input.raw.browser_number,
            raw_pressure_absent: input.raw.absent,
            triangle_index: hit.triangle_index,
            maximum_center_rounding_error: (center64 - center.as_dvec3()).abs().max_element(),
            radius_rounding_error: (local_radius - f64::from(radius32)).abs(),
            view64: input.view.world_to_clip,
            viewport64: input.view.viewport,
        };
        Ok(PreparedLockedTick::Dab(Box::new(PreparedLockedDab {
            sample,
            report,
        })))
    }
    /// Consumed permission and current prepared dab are dispatched directly:
    /// no Begin, queued timer work, rest preview or release endpoint lives here.
    pub fn apply_tick(
        &mut self,
        doc: &mut Document,
        prepared: PreparedLockedTick,
    ) -> Result<Option<LockedAccumulationReport>, String> {
        self.require_binding(doc)?;
        let PreparedLockedTick::Dab(dab) = prepared else {
            return Ok(None);
        };
        let PreparedLockedDab { sample, report } = *dab;
        if report.source_sequence != self.engine.sequence() {
            self.failed = Some(
                "Continuous Lock prepared sample outlived its current-surface sequence".into(),
            );
            return Err(self.failed.clone().unwrap());
        }
        if self.paints_material() {
            doc.begin_material_paint_stroke();
        }
        let result = doc
            .require_active_mesh_mut()
            .and_then(|m| self.engine.apply(m, sample));
        match result {
            Ok(effect) => {
                self.last_preparation = Some(report);
                self.last_effect = Some(effect.clone());
                Ok(Some(effect))
            }
            Err(error) => {
                self.failed = Some(error.clone());
                Err(error)
            }
        }
    }
    pub fn cancel(&mut self, doc: &mut Document) -> Result<LockedAccumulationReport, String> {
        self.require_binding(doc)?;
        let result = doc
            .require_active_mesh_mut()
            .and_then(|m| self.engine.cancel(m))?;
        self.last_effect = Some(result.clone());
        Ok(result)
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"kind":self.kind().name(),"part":self.subtool,"level":self.level,
            "sequence":self.engine.sequence(),"at_original":self.at_original(),"failed":self.failed,
            "effective_fixed_topology":true,"effective_accumulate":true,"kernel":"legacy OBrush; precise current camera; declared f32 core conversions",
            "last_preparation":self.last_preparation.as_ref().map(LockedPreparationReport::diagnostics),"last_effect":self.last_effect})
    }
}
fn validate_input(input: SourcePaintInput, pressure: f32) -> Result<(), String> {
    input.view.validate()?;
    if input.screen.iter().any(|v| !v.is_finite())
        || !input.raw.pressure.is_finite()
        || !(0.0..=1.0).contains(&input.raw.pressure)
        || !pressure.is_finite()
        || !(0.0..=1.0).contains(&pressure)
    {
        return Err("Continuous Lock input/contact is invalid".into());
    }
    Ok(())
}
fn normal_at(mesh: &obrush_core::Mesh, row: usize, point: DVec3) -> Result<Vec3, String> {
    let ids = mesh
        .triangles
        .get(row)
        .ok_or("Missing Continuous Lock triangle identity")?;
    let [a, b, c] = ids.map(|i| mesh.positions[i as usize].as_dvec3());
    let (e1, e2, d) = (b - a, c - a, point - a);
    let (d00, d01, d11, d20, d21) = (e1.dot(e1), e1.dot(e2), e2.dot(e2), d.dot(e1), d.dot(e2));
    let denominator = d00 * d11 - d01 * d01;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err("Continuous Lock picked normal has degenerate weights".into());
    }
    let u = (d11 * d20 - d01 * d21) / denominator;
    let v = (d00 * d21 - d01 * d20) / denominator;
    let normals = ids.map(|i| mesh.normals[i as usize].as_dvec3());
    let normal = (normals[0] * (1.0 - u - v) + normals[1] * u + normals[2] * v)
        .normalize()
        .as_vec3();
    if !normal.is_finite() || normal.length_squared() == 0.0 {
        return Err("Continuous Lock picked normal is invalid".into());
    }
    Ok(normal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_paint_host::RawPressureSample;
    use glam::DMat4;
    use obrush_core::{Mesh, PaintViewCapture, RasterAlpha};
    fn doc() -> Document {
        let mut m =
            Mesh::from_obj("v -0.002 -0.002 0\nv 0.002 -0.002 0\nv 0 0.002 0\nf 1 2 3\n").unwrap();
        m.colors.fill([0.0; 3]);
        Document::new(m)
    }
    fn options(kind: BrushKind) -> BrushSettings {
        BrushSettings {
            kind,
            radius: 0.1,
            strength: 1.0,
            hardness: 1.0,
            pressure: 1.0,
            pressure_strength: false,
            pressure_radius: false,
            backface_culling: false,
            symmetry_x: false,
            adaptive_detail: true,
            color: [1.0; 3],
            ..Default::default()
        }
    }
    fn input(x: f64, y: f64) -> SourcePaintInput {
        SourcePaintInput {
            view: PaintViewCapture {
                world_to_clip: DMat4::IDENTITY.to_cols_array(),
                viewport: [0.0, 0.0, 10_000.0, 10_000.0],
            },
            screen: [x, y],
            raw: RawPressureSample {
                pressure: 0.5,
                pen: false,
                browser_number: true,
                absent: true,
            },
            in_contact: true,
        }
    }
    fn gesture(d: &Document, s: &BrushSettings) -> ContinuousLockedGesture {
        ContinuousLockedGesture::prepare(d, s, input(5000.0, 5000.0), [1.0, 0.0, 0.0]).unwrap()
    }
    fn tick(
        g: &mut ContinuousLockedGesture,
        d: &mut Document,
        x: f64,
        y: f64,
    ) -> LockedAccumulationReport {
        let p = g.prepare_tick(d, input(x, y), 1.0).unwrap();
        g.apply_tick(d, p).unwrap().unwrap()
    }
    #[test]
    fn independent_identity_view_three_four_five_anchor_radius_and_normal() {
        let d = doc();
        let mut g = gesture(&d, &options(BrushKind::Paint));
        let PreparedLockedTick::Dab(p) = g.prepare_tick(&d, input(5003.0, 5004.0), 1.0).unwrap()
        else {
            panic!("anchored hit")
        };
        assert_eq!(p.report.screen_radius64, 5.0);
        assert!((p.report.local_radius64 - 0.001).abs() < 2e-16);
        assert!((p.report.local_center64[0] - 1.2e-7).abs() < 2e-16);
        assert!((p.report.local_center64[1] + 1.6e-7).abs() < 2e-16);
        assert_eq!(p.sample.normal, Vec3::Z);
        assert_eq!(p.report.raw_pressure64, 0.5);
        assert_eq!(p.report.core_pressure32, 1.0);
    }
    #[test]
    fn pointer_background_still_uses_the_real_near_press_anchored_hit() {
        let d = doc();
        let mut g = gesture(&d, &options(BrushKind::Paint));
        let i = input(7000.0, 5000.0);
        assert!(
            prepare_source_paint_hit(
                d.require_active_mesh().unwrap(),
                None,
                i.view.ray(i.screen).unwrap()
            )
            .unwrap()
            .is_none()
        );
        assert!(g.prepare_tick(&d, i, 1.0).unwrap().anchored_hit());
    }
    #[test]
    fn independent_constant_paint_tick_recurrence_is_additive_not_rest_preview() {
        let mut d = doc();
        let mut g = gesture(&d, &options(BrushKind::Paint));
        d.begin_stroke();
        for expected in [0.25_f32, 0.4375, 0.578125] {
            tick(&mut g, &mut d, 5075.0, 5000.0);
            for color in &d.require_active_mesh().unwrap().colors {
                assert_eq!(color.map(f32::to_bits), [expected.to_bits(); 3]);
            }
        }
        d.end_stroke();
        assert!(!g.at_original());
    }
    #[test]
    fn zero_travel_skips_pick_and_preserves_prior_ticks_without_history_split() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let mut g = gesture(&d, &options(BrushKind::Paint));
        d.begin_stroke();
        tick(&mut g, &mut d, 5075.0, 5000.0);
        let accepted = serde_json::to_string(d.require_active_mesh().unwrap()).unwrap();
        let p = g.prepare_tick(&d, input(5000.0, 5000.0), 1.0).unwrap();
        assert!(matches!(p, PreparedLockedTick::Hold));
        assert!(g.apply_tick(&mut d, p).unwrap().is_none());
        assert_eq!(
            serde_json::to_string(d.require_active_mesh().unwrap()).unwrap(),
            accepted
        );
        d.end_stroke();
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
    }
    #[test]
    fn complete_cancel_restores_preexisting_redo_without_material_or_extra_undo() {
        let mut d = doc();
        d.invert_mask();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut s = options(BrushKind::Paint);
        s.paint_roughness = true;
        s.roughness = 0.8;
        let mut g = gesture(&d, &s);
        d.begin_material_paint_stroke();
        tick(&mut g, &mut d, 5075.0, 5000.0);
        assert!(d.require_active_mesh().unwrap().materials.is_some());
        g.cancel(&mut d).unwrap();
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.can_redo());
        assert!(!d.can_undo());
    }
    #[test]
    fn repeated_material_ticks_seed_every_retained_level_in_one_history_entry() {
        let mut d = doc();
        d.add_subdivision_level().unwrap();
        let before = d.to_json().unwrap();
        let mut s = options(BrushKind::Paint);
        s.paint_rgb = false;
        s.paint_metallic = true;
        s.metallic = 0.8;
        let mut g = gesture(&d, &s);
        d.begin_material_paint_stroke();
        for _ in 0..3 {
            tick(&mut g, &mut d, 5075.0, 5000.0);
        }
        d.end_stroke();
        assert!(d.take_layer_error().is_none());
        let after = d.to_json().unwrap();
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);
        d.select_level(0).unwrap();
        assert!(d.require_active_mesh().unwrap().materials.is_some());
    }
    #[test]
    fn source_only_fixed_topology_override_does_not_modify_saved_preferences() {
        let d = doc();
        let saved = options(BrushKind::Draw);
        assert!(saved.adaptive_detail);
        let g = gesture(&d, &saved);
        assert!(saved.adaptive_detail);
        assert!(!g.engine.settings().adaptive_detail);
        assert!(g.engine.settings().draw_accumulate);
    }
    #[test]
    fn alpha_and_symmetry_inverse_flags_are_actually_retained_in_legacy_capture() {
        let d = doc();
        let mut s = options(BrushKind::Paint);
        s.symmetry_x = true;
        s.symmetry_offset = 0.125;
        s.invert = true;
        s.alpha = Some(RasterAlpha::new(2, 2, vec![0, 255, 255, 0]).unwrap());
        let g = gesture(&d, &s);
        assert_eq!(g.engine.settings().symmetry_offset, 0.125);
        assert!(g.engine.settings().symmetry_x && g.engine.settings().invert);
        assert_eq!(
            g.engine.settings().alpha.as_ref().unwrap().pixels(),
            s.alpha.as_ref().unwrap().pixels()
        );
    }
    #[test]
    fn actual_black_alpha_tick_is_noop_while_white_alpha_applies() {
        let mut d = doc();
        let before = d.to_json().unwrap();
        let mut s = options(BrushKind::Paint);
        s.alpha = Some(RasterAlpha::new(1, 1, vec![0]).unwrap());
        let mut g = gesture(&d, &s);
        d.begin_stroke();
        assert_eq!(tick(&mut g, &mut d, 5075.0, 5000.0).changed_vertices, 0);
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(!d.can_undo());
        s.alpha = Some(RasterAlpha::new(1, 1, vec![255]).unwrap());
        let mut g = gesture(&d, &s);
        d.begin_stroke();
        assert!(tick(&mut g, &mut d, 5075.0, 5000.0).changed_vertices > 0);
        d.end_stroke();
    }
    #[test]
    fn captured_channel_targets_do_not_follow_later_ui_settings() {
        let mut d = doc();
        let mut s = options(BrushKind::Paint);
        let mut g = gesture(&d, &s);
        s.color = [0.0; 3];
        s.strength = 0.0;
        d.begin_stroke();
        tick(&mut g, &mut d, 5075.0, 5000.0);
        d.end_stroke();
        assert_eq!(d.require_active_mesh().unwrap().colors[0], [0.25; 3]);
    }
    #[test]
    fn no_selected_channels_is_no_history_and_does_not_allocate_materials() {
        let mut d = doc();
        d.invert_mask();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let mut s = options(BrushKind::Paint);
        s.paint_rgb = false;
        s.paint_roughness = false;
        s.paint_metallic = false;
        let mut g = gesture(&d, &s);
        d.begin_stroke();
        tick(&mut g, &mut d, 5075.0, 5000.0);
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.can_redo());
        assert!(d.require_active_mesh().unwrap().materials.is_none());
    }
    #[test]
    fn old_prepared_sample_cannot_be_replayed_after_another_actual_tick() {
        let mut d = doc();
        let mut g = gesture(&d, &options(BrushKind::Paint));
        d.begin_stroke();
        let old = g.prepare_tick(&d, input(5075.0, 5000.0), 1.0).unwrap();
        tick(&mut g, &mut d, 5075.0, 5000.0);
        let before = serde_json::to_string(d.require_active_mesh().unwrap()).unwrap();
        assert!(g.apply_tick(&mut d, old).is_err());
        assert_eq!(
            serde_json::to_string(d.require_active_mesh().unwrap()).unwrap(),
            before
        );
        d.end_stroke();
    }
    #[test]
    fn external_color_edit_is_atomic_stale_rejection_not_rest_reapplication() {
        let mut d = doc();
        let mut g = gesture(&d, &options(BrushKind::Paint));
        d.require_active_mesh_mut().unwrap().colors[0] = [0.125; 3];
        let before = d.to_json().unwrap();
        let p = g.prepare_tick(&d, input(5075.0, 5000.0), 1.0).unwrap();
        assert!(g.apply_tick(&mut d, p).is_err());
        assert_eq!(d.to_json().unwrap(), before);
    }
    #[test]
    fn viewport_and_part_changes_reject_before_another_tick() {
        let mut d = doc();
        let mut g = gesture(&d, &options(BrushKind::Paint));
        let mut i = input(5075.0, 5000.0);
        i.view.viewport[2] = 9000.0;
        assert!(g.prepare_tick(&d, i, 1.0).is_err());
        let mut g = gesture(&d, &options(BrushKind::Paint));
        d.add_subtool(Mesh::cube(), "other").unwrap();
        assert!(g.prepare_tick(&d, input(5075.0, 5000.0), 1.0).is_err());
    }
    #[test]
    fn invalid_raw_pressure_rejects_without_consuming_engine_or_modifying_document() {
        let d = doc();
        let mut g = gesture(&d, &options(BrushKind::Paint));
        let before = d.to_json().unwrap();
        let mut i = input(5075.0, 5000.0);
        i.raw.pressure = f64::NAN;
        assert!(g.prepare_tick(&d, i, 1.0).is_err());
        assert_eq!(g.engine.sequence(), 0);
        assert_eq!(d.to_json().unwrap(), before);
    }
    #[test]
    fn ordinary_unsupported_tool_and_empty_document_do_not_create_placeholder_or_undo() {
        for kind in [
            BrushKind::Move,
            BrushKind::Drag,
            BrushKind::Twist,
            BrushKind::LocalScale,
            BrushKind::Morph,
        ] {
            assert!(
                ContinuousLockedGesture::prepare(
                    &doc(),
                    &options(kind),
                    input(5000.0, 5000.0),
                    [1.0, 0.0, 0.0]
                )
                .is_err()
            );
        }
        let empty = Document::empty();
        assert!(
            ContinuousLockedGesture::prepare(
                &empty,
                &options(BrushKind::Paint),
                input(5000.0, 5000.0),
                [1.0, 0.0, 0.0]
            )
            .is_err()
        );
        assert!(empty.is_empty() && !empty.can_undo());
    }
}
