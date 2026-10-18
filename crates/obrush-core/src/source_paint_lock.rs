//! Explicit static Source Paint Lock transactions. Ordinary/Legacy APIs are unchanged.
//! Alpha/mirror/dynamic and exact source camera/pressure/history parity are not admitted.
use super::{
    state_origin_storage, Document, DocumentState, LimitedJsonWriter, OutputCounter, ProjectRef,
    HISTORY_BUDGET_BYTES,
};
use crate::{
    BrushKind, MaskConvention, Mesh, PaintResponseOperation, PaintResponseOptions,
    PaintResponsePass, PaintResponseReport, PaintResponseRequest, SourceGeometryNormalState,
    SourceGeometryOptions, SourceNormalField, SourceNormalPolicy, SourceOriginalFaceContact,
    SourcePaintNormalProducerReport, SourcePaintNormalView, SourcePickedNormalDisposition,
    MAX_SOURCE_GEOMETRY_OPERATION_BYTES, MAX_SOURCE_NORMAL_BYTES,
};
use serde::Serialize;
use std::sync::Arc;
#[path = "source_state_binding.rs"]
mod binding_binary;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourcePaintLockedMode {
    AbsolutePreview,
    ContinuousAccumulation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourcePaintLockedAction {
    Preview,
    Accumulate,
    RestoreOrigin,
    RestoreMiss,
    HoldZeroTravel,
    HoldMiss,
    HoldNoContact,
    NoContribution,
    Cancel,
}
#[derive(Clone, Copy, Debug)]
pub enum SourcePaintLockedSample {
    ZeroTravel,
    Miss,
    NoContact,
    Dab {
        pass: PaintResponsePass,
        face_index: usize,
        pressure: f64,
    },
}
#[derive(Clone, Debug, Serialize)]
pub struct SourcePaintLockedReport {
    pub sequence: u64,
    pub mode: SourcePaintLockedMode,
    pub action: SourcePaintLockedAction,
    pub part: u32,
    pub level: usize,
    pub at_original: bool,
    /// Changes from the previously visible sample, not from the rest stencil.
    pub live: PaintResponseReport,
    pub dab: PaintResponseReport,
    pub materials_before: bool,
    pub materials_after: bool,
    pub producer: Option<SourcePaintNormalProducerReport>,
    pub picked_normal: Option<SourcePickedNormalDisposition>,
}
/// A unique owned transaction. It is deliberately not Clone or serialized.
/// The baseline is Document's already-owned stroke snapshot, not another copy.
pub struct SourcePaintLockedStroke {
    token: u64,
    part: u32,
    level: usize,
    mode: SourcePaintLockedMode,
    options: PaintResponseOptions,
    expected: Vec<u8>,
    repository: Vec<(u32, Arc<SourceGeometryNormalState>)>,
    warm: Option<Arc<SourceGeometryNormalState>>,
    cold: Option<SourceNormalField>,
    sequence: u64,
    closed: bool,
    at_original: bool,
    last: Option<SourcePaintLockedReport>,
    operation_limit: usize,
}
impl SourcePaintLockedStroke {
    pub fn mode(&self) -> SourcePaintLockedMode {
        self.mode
    }
    pub fn at_original(&self) -> bool {
        self.at_original
    }
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn last_report(&self) -> Option<&SourcePaintLockedReport> {
        self.last.as_ref()
    }
    pub fn normal_state(&self) -> Option<Arc<SourceGeometryNormalState>> {
        self.warm.clone()
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    fn retained_bytes(&self) -> usize {
        self.expected
            .capacity()
            .saturating_add(std::mem::size_of::<Self>())
            .saturating_add(16_384)
            .saturating_add(super::source_geometry_document::retained_bytes(
                &self.repository,
                self.repository.capacity(),
            ))
            .saturating_add(self.warm.as_ref().map_or(0, |s| s.approximate_bytes()))
            .saturating_add(
                self.cold
                    .as_ref()
                    .map_or(0, |s| s.report().storage.retained_bytes),
            )
    }
    fn view(&self) -> Result<SourcePaintNormalView<'_>, String> {
        if let Some(state) = self.warm.as_ref() {
            Ok(SourcePaintNormalView::RetainedGeometry(state))
        } else {
            self.cold
                .as_ref()
                .map(SourcePaintNormalView::FreshStatic)
                .ok_or_else(|| "Source Paint Lock has no prepared normal field".into())
        }
    }
}
fn options_valid(o: &PaintResponseOptions) -> Result<(), String> {
    if o.alpha.is_some()
        || !o.intensity.is_finite()
        || !(0.0..=1.0).contains(&o.intensity)
        || !o.hardness.is_finite()
        || !(0.0..=1.0).contains(&o.hardness)
        || o.color
            .iter()
            .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
        || [o.roughness, o.metallic]
            .iter()
            .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
    {
        return Err("Static Source Paint Lock requires normalized options and alphaNone".into());
    }
    o.tablet.multiplier()?;
    Ok(())
}
fn current_bytes(d: &Document) -> usize {
    [
        std::mem::size_of::<DocumentState>(),
        d.mesh.as_ref().map_or(0, Mesh::approximate_bytes),
        d.name.capacity(),
        d.multiresolution
            .as_ref()
            .map_or(0, |s| s.approximate_bytes()),
        d.layers.as_ref().map_or(0, |s| s.approximate_bytes()),
        d.morph_target.as_ref().map_or(0, |s| s.approximate_bytes()),
        d.scene.as_ref().map_or(0, |s| s.approximate_bytes()),
        state_origin_storage(d.active_metadata.as_ref(), d.scene.as_ref()),
        d.active_metadata
            .as_ref()
            .map_or(0, |_| std::mem::size_of::<crate::SceneObjectMetadata>()),
        d.source_geometry_retained_bytes(),
    ]
    .into_iter()
    .fold(0, usize::saturating_add)
}
fn live_owned_bytes(d: &Document) -> usize {
    current_bytes(d)
        .saturating_add(
            d.stroke_start
                .as_ref()
                .map_or(0, DocumentState::approximate_bytes),
        )
        .saturating_add(
            d.undo
                .iter()
                .chain(d.redo.iter())
                .map(DocumentState::approximate_bytes)
                .fold(0, usize::saturating_add),
        )
}
fn budget(d: &Document, retained: usize, extra: usize, limit: usize) -> Result<usize, String> {
    let used = live_owned_bytes(d)
        .checked_add(retained)
        .and_then(|b| b.checked_add(extra))
        .ok_or("Source Paint Lock allocation accounting overflow")?;
    limit
        .checked_sub(used)
        .ok_or_else(|| "Source Paint Lock operation exceeds existing256MiB bound".into())
}
/// Bounded legacy project-ref vectors: one active plus at most32 scene objects,
/// each at most16 layer refs and8 hierarchy refs. Existing helpers allocate
/// these small vectors; charge their conservative maximum before calling them.
const BINDING_REFERENCE_SCRATCH_BYTES: usize = (crate::MAX_SUBTOOLS + 1)
    * (crate::MAX_SCULPT_LAYERS * 64 + crate::multires::MAX_SUBDIVISION_LEVELS * 16 + 512)
    + 4096;
fn metadata_matches_original(d: &Document, original: &DocumentState) -> bool {
    if !crate::scene_v7::optional_metadata_equal(
        d.active_metadata.as_ref(),
        original.active_metadata.as_ref(),
    ) {
        return false;
    }
    let header = match (&d.scene_origin, &original.scene_origin) {
        (None, None) => true,
        (Some(a), Some(b)) => crate::scene_v7::header_bits_equal(a, b),
        _ => false,
    };
    header
        && match (&d.scene, &original.scene) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.selected_ids == b.selected_ids
                    && a.objects.len() == b.objects.len()
                    && a.objects.iter().zip(&b.objects).all(|(a, b)| {
                        a.id == b.id
                            && crate::scene_v7::optional_metadata_equal(
                                a.payload.as_ref().and_then(|p| p.metadata.as_ref()),
                                b.payload.as_ref().and_then(|p| p.metadata.as_ref()),
                            )
                    })
            }
            _ => false,
        }
}
/// Exact stored pending-state representation. It deliberately does NOT reconcile
/// active Some materials into still-implicit retained None levels during preview.
/// Metadata/original bytes are compared separately against the owned snapshot,
/// avoiding a fresh encoded copy of immutable SGL origins on every dab.
/// This is private transaction evidence, never a new public project format.
fn binding_project<'a>(d: &'a Document, replacement: Option<&'a Mesh>) -> ProjectRef<'a> {
    ProjectRef {
        version: 0,
        name: &d.name,
        mesh: replacement.or(d.mesh.as_ref()),
        multiresolution: d.multiresolution.as_ref().map(|s| s.project_ref()),
        layers: d.layers.as_ref().map(|s| s.project_ref()),
        morph_target: d.morph_target.as_ref().map(|s| s.project_ref()),
        scene: d.scene.as_ref().map(|s| s.project_ref()),
        scene_v7: None,
    }
}
fn binding(d: &Document, replacement: Option<&Mesh>, limit: usize) -> Result<Vec<u8>, String> {
    let limit = limit
        .checked_sub(BINDING_REFERENCE_SCRATCH_BYTES)
        .ok_or("Source Paint Lock projected refs exceed charged workspace")?;
    let project = binding_project(d, replacement);
    let mut counter = OutputCounter { bytes: 0 };
    binding_binary::to_writer(&mut counter, &project)
        .map_err(|e| format!("Source Paint Lock stored binding rejected: {e}"))?;
    if counter.bytes > limit {
        return Err("Source Paint Lock binding exceeds charged workspace".into());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(counter.bytes)
        .map_err(|_| "Cannot reserve Source Paint Lock binding")?;
    if bytes.capacity() > limit {
        return Err("Source Paint Lock actual binding capacity exceeds workspace".into());
    }
    let mut writer = LimitedJsonWriter {
        bytes,
        limit: counter.bytes,
    };
    binding_binary::to_writer(&mut writer, &project)
        .map_err(|e| format!("Source Paint Lock binding encoding rejected: {e}"))?;
    Ok(writer.bytes)
}
fn binding_matches(d: &Document, expected: &[u8], limit: usize) -> Result<bool, String> {
    limit
        .checked_sub(BINDING_REFERENCE_SCRATCH_BYTES)
        .ok_or("Source Paint Lock projected refs exceed charged workspace")?;
    struct Compare<'a> {
        expected: &'a [u8],
        at: usize,
        differs: bool,
    }
    impl std::io::Write for Compare<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let end = self
                .at
                .checked_add(bytes.len())
                .ok_or_else(|| std::io::Error::other("Binding span"))?;
            if self.expected.get(self.at..end) != Some(bytes) {
                self.differs = true;
                return Err(std::io::Error::other("Stored binding differs"));
            }
            self.at = end;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let project = binding_project(d, None);
    let mut compare = Compare {
        expected,
        at: 0,
        differs: false,
    };
    let result = binding_binary::to_writer(&mut compare, &project);
    if compare.differs {
        return Ok(false);
    }
    result.map_err(|e| format!("Source Paint Lock binding comparison rejected: {e}"))?;
    Ok(compare.at == expected.len())
}
#[cfg(test)]
fn original_json_binding(
    d: &Document,
    replacement: Option<&Mesh>,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let limit = limit
        .checked_sub(BINDING_REFERENCE_SCRATCH_BYTES)
        .ok_or("Source Paint Lock projected refs exceed charged workspace")?;
    let project = binding_project(d, replacement);
    let mut counter = OutputCounter { bytes: 0 };
    serde_json::to_writer(&mut counter, &project).map_err(|e| e.to_string())?;
    if counter.bytes > limit {
        return Err("Original JSON binding exceeds workspace".into());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(counter.bytes)
        .map_err(|e| e.to_string())?;
    let mut writer = LimitedJsonWriter {
        bytes,
        limit: counter.bytes,
    };
    serde_json::to_writer(&mut writer, &project).map_err(|e| e.to_string())?;
    Ok(writer.bytes)
}

fn same_material(a: &Mesh, b: &Mesh, i: usize) -> bool {
    match (&a.materials, &b.materials) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            x.get(i).map(|x| x.map(f32::to_bits)) == y.get(i).map(|y| y.map(f32::to_bits))
        }
        _ => false,
    }
}
fn channels_same(a: &Mesh, b: &Mesh) -> bool {
    a.colors.len() == b.colors.len()
        && a.colors
            .iter()
            .zip(&b.colors)
            .all(|(x, y)| x.map(f32::to_bits) == y.map(f32::to_bits))
        && match (&a.materials, &b.materials) {
            (None, None) => true,
            (Some(x), Some(y)) => {
                x.len() == y.len()
                    && x.iter()
                        .zip(y)
                        .all(|(x, y)| x.map(f32::to_bits) == y.map(f32::to_bits))
            }
            _ => false,
        }
}
fn delta(a: &Mesh, b: &Mesh) -> PaintResponseReport {
    let mut r = PaintResponseReport::default();
    for i in 0..a
        .positions
        .len()
        .max(b.positions.len())
        .max(a.colors.len())
        .max(b.colors.len())
        .max(a.materials.as_ref().map_or(0, Vec::len))
        .max(b.materials.as_ref().map_or(0, Vec::len))
    {
        let color = a.colors.get(i).map(|x| x.map(f32::to_bits))
            != b.colors.get(i).map(|x| x.map(f32::to_bits));
        let material = !same_material(a, b, i);
        r.changed_rgb_vertices += usize::from(color);
        r.changed_material_vertices += usize::from(material);
        r.changed_vertices += usize::from(color || material);
    }
    r.materialized = a.materials.is_none() && b.materials.is_some();
    r
}
impl Document {
    pub fn source_paint_locked_blocker(&self, o: &PaintResponseOptions) -> Result<(), String> {
        options_valid(o)?;
        self.require_active_mesh()?.validate()?;
        if !self.active_subtool_visible()
            || self.require_active_mesh()?.mask_convention != MaskConvention::UnprotectedF32V1
        {
            return Err("Source Paint Lock requires a visible active canonicalU mesh".into());
        }
        if let Some(s) = self
            .active_subtool_id()
            .and_then(|id| self.source_geometry_state(id))
        {
            self.source_geometry_blocker(SourceGeometryOptions::inflate())
                .map_err(|e| e.to_string())?;
            SourcePaintNormalView::RetainedGeometry(&s)
                .validate(self.require_active_mesh()?)
                .map_err(|e| format!("Source Paint Lock retained producer rejected: {e:?}"))?;
        }
        Ok(())
    }
    pub fn begin_source_paint_locked(
        &mut self,
        o: &PaintResponseOptions,
        mode: SourcePaintLockedMode,
    ) -> Result<SourcePaintLockedStroke, String> {
        self.begin_source_paint_locked_with_limit(o, mode, MAX_SOURCE_GEOMETRY_OPERATION_BYTES)
    }
    fn begin_source_paint_locked_with_limit(
        &mut self,
        o: &PaintResponseOptions,
        mode: SourcePaintLockedMode,
        operation_limit: usize,
    ) -> Result<SourcePaintLockedStroke, String> {
        if operation_limit > MAX_SOURCE_GEOMETRY_OPERATION_BYTES {
            return Err("Source Paint Lock cannot raise its operation bound".into());
        }
        self.source_paint_locked_blocker(o)?;
        self.validate_complete_state()?;
        if self.stroke_start.is_some() || self.source_geometry_active.is_some() {
            return Err("Finish the preceding stroke before Source Paint Lock".into());
        }
        let part = self
            .active_subtool_id()
            .ok_or("No Source Paint Lock part")?;
        let original_bytes = current_bytes(self);
        if original_bytes > HISTORY_BUDGET_BYTES {
            return Err("Source Paint Lock original snapshot exceeds128MiB history bound".into());
        }
        let available = budget(self, 0, original_bytes, operation_limit)?;
        let expected = binding(self, None, available)?;
        let mut repository = Vec::new();
        repository
            .try_reserve_exact(self.source_geometry_states.len())
            .map_err(|_| "Cannot reserve captured Source Paint Lock repository")?;
        repository.extend(
            self.source_geometry_states
                .iter()
                .map(|(id, s)| (*id, s.clone())),
        );
        let mut stroke = SourcePaintLockedStroke {
            token: 0,
            part,
            level: self.current_level(),
            mode,
            options: o.clone(),
            expected,
            repository,
            warm: self.source_geometry_state(part),
            cold: None,
            sequence: 0,
            closed: false,
            at_original: true,
            last: None,
            operation_limit,
        };
        budget(
            self,
            stroke.retained_bytes(),
            original_bytes,
            operation_limit,
        )?;
        let token = super::source_geometry_document::allocate_source_claim(
            &super::source_geometry_document::SOURCE_CLAIM_NONCE,
        )
        .map_err(|e| e.to_string())?;
        self.begin_stroke();
        if self.stroke_start.is_none() {
            return Err("Source Paint Lock could not capture its original snapshot".into());
        }
        self.begin_source_attribute_history(part)
            .map_err(|e| e.to_string())?;
        self.source_geometry_token = token;
        self.source_geometry_active = Some(token);
        stroke.token = token;
        Ok(stroke)
    }
    fn locked_owner(&self, s: &SourcePaintLockedStroke) -> Result<(), String> {
        if s.closed || self.source_geometry_active != Some(s.token) || self.stroke_start.is_none() {
            return Err("Source Paint Lock owner is foreign, expired or closed".into());
        }
        Ok(())
    }
    fn locked_binding(&self, s: &SourcePaintLockedStroke) -> Result<(), String> {
        self.locked_owner(s)?;
        if self.active_subtool_id() != Some(s.part)
            || self.current_level() != s.level
            || !self.active_subtool_visible()
            || self.source_geometry_states.len() != s.repository.len()
            || self
                .source_geometry_states
                .iter()
                .zip(&s.repository)
                .any(|((a, x), (b, y))| a != b || !Arc::ptr_eq(x, y))
        {
            return Err("Source Paint Lock part/level/cache lifetime changed".into());
        }
        self.require_active_mesh()?.validate()?;
        if !metadata_matches_original(
            self,
            self.stroke_start
                .as_ref()
                .ok_or("Source Paint Lock original metadata missing")?,
        ) {
            return Err("Source Paint Lock raw metadata/header/selection changed".into());
        }
        if let Some(state) = s.warm.as_ref() {
            SourcePaintNormalView::RetainedGeometry(state)
                .validate(self.require_active_mesh()?)
                .map_err(|e| format!("Source Paint Lock warm producer became stale: {e:?}"))?;
        }
        let available = budget(self, s.retained_bytes(), 0, s.operation_limit)?;
        if !binding_matches(self, &s.expected, available)? {
            return Err(
                "Source Paint Lock exact stored mesh/scene/metadata binding changed".into(),
            );
        }
        Ok(())
    }
    pub fn apply_source_paint_locked(
        &mut self,
        s: &mut SourcePaintLockedStroke,
        sample: SourcePaintLockedSample,
    ) -> Result<SourcePaintLockedReport, String> {
        // A foreign owner may not even trigger rollback of either document.
        self.locked_owner(s)?;
        let result = self.apply_source_paint_locked_inner(s, sample);
        if let Err(error) = &result {
            let _ = self.reject_source_paint_locked(s, error.clone());
        }
        result
    }
    fn apply_source_paint_locked_inner(
        &mut self,
        s: &mut SourcePaintLockedStroke,
        sample: SourcePaintLockedSample,
    ) -> Result<SourcePaintLockedReport, String> {
        self.locked_binding(s)?;
        let next = s
            .sequence
            .checked_add(1)
            .ok_or("Source Paint Lock sequence exhausted")?;
        let restore = match sample {
            SourcePaintLockedSample::ZeroTravel
                if s.mode == SourcePaintLockedMode::AbsolutePreview =>
            {
                Some(SourcePaintLockedAction::RestoreOrigin)
            }
            SourcePaintLockedSample::Miss if s.mode == SourcePaintLockedMode::AbsolutePreview => {
                Some(SourcePaintLockedAction::RestoreMiss)
            }
            _ => None,
        };
        let mut producer = None;
        let mut picked_normal = None;
        let mut dab = PaintResponseReport::default();
        let mut candidate = None;
        let action = if let Some(action) = restore {
            if !s.at_original {
                let base = self
                    .stroke_start
                    .as_ref()
                    .and_then(|x| x.mesh.as_ref())
                    .ok_or("Source Paint Lock original mesh missing")?;
                budget(
                    self,
                    s.retained_bytes(),
                    base.approximate_bytes(),
                    s.operation_limit,
                )?;
                candidate = Some(base.try_snapshot()?);
            }
            action
        } else {
            match sample {
                SourcePaintLockedSample::ZeroTravel => SourcePaintLockedAction::HoldZeroTravel,
                SourcePaintLockedSample::Miss => SourcePaintLockedAction::HoldMiss,
                SourcePaintLockedSample::NoContact => SourcePaintLockedAction::HoldNoContact,
                SourcePaintLockedSample::Dab {
                    pass,
                    face_index,
                    pressure,
                } => {
                    if !pressure.is_finite()
                        || !(0.0..=1.0).contains(&pressure)
                        || pass.mirrored_x
                        || pass.alpha_frame.is_some()
                        || pass
                            .center
                            .iter()
                            .chain(&pass.eye_direction)
                            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
                        || pass.eye_direction.iter().all(|v| *v == 0.0)
                        || pass.local_radius_squared <= 0.0
                        || pass.local_radius_squared > 1_000_000_000_000.0
                        || !pass.local_radius_squared.is_finite()
                        || face_index
                            >= self
                                .require_active_mesh()?
                                .surface
                                .as_ref()
                                .map_or(self.require_active_mesh()?.triangles.len(), |s| {
                                    s.faces.len()
                                })
                    {
                        return Err(
                            "Source Paint Lock prepared sample is invalid or unsupported".into(),
                        );
                    }
                    let has_channel = s.options.write_rgb
                        || s.options.write_roughness
                        || s.options.write_metallic;
                    let intensity = s.options.intensity
                        * crate::PaintTabletIntensity {
                            pressure,
                            factor: s.options.tablet.factor,
                        }
                        .multiplier()?;
                    let base = if s.mode == SourcePaintLockedMode::AbsolutePreview {
                        self.stroke_start
                            .as_ref()
                            .and_then(|x| x.mesh.as_ref())
                            .ok_or("Source Paint Lock original mesh missing")?
                    } else {
                        self.require_active_mesh()?
                    };
                    if !has_channel || intensity == 0.0 || base.masks.iter().all(|m| *m == 0.0) {
                        if s.mode == SourcePaintLockedMode::AbsolutePreview && !s.at_original {
                            budget(
                                self,
                                s.retained_bytes(),
                                base.approximate_bytes(),
                                s.operation_limit,
                            )?;
                            candidate = Some(base.try_snapshot()?);
                        }
                        SourcePaintLockedAction::NoContribution
                    } else {
                        let outputs = base
                            .positions
                            .len()
                            .checked_mul(
                                usize::from(s.options.write_rgb) * 12
                                    + usize::from(
                                        s.options.write_roughness || s.options.write_metallic,
                                    ) * 8
                                    + 1,
                            )
                            .ok_or("Source Paint Lock scratch overflow")?;
                        if outputs > crate::paint_response::MAX_PAINT_RESPONSE_SCRATCH_BYTES {
                            return Err(
                                "Source Paint Lock preserves64MiB Paint scratch bound".into()
                            );
                        }
                        let normal_extra = if s.warm.is_none() && s.cold.is_none() {
                            SourceNormalField::storage_requirement(base)?.peak_bytes
                        } else {
                            0
                        };
                        let extra = base
                            .approximate_bytes()
                            .checked_add(outputs)
                            .and_then(|b| b.checked_add(normal_extra))
                            .ok_or("Source Paint Lock peak overflow")?;
                        budget(self, s.retained_bytes(), extra, s.operation_limit)?;
                        if s.warm.is_none() && s.cold.is_none() {
                            s.cold = Some(SourceNormalField::compute_with_budget(
                                base,
                                SourceNormalPolicy::StaticOriginalFaces,
                                MAX_SOURCE_NORMAL_BYTES,
                            )?);
                        }
                        let view = s.view()?;
                        view.validate(base)
                            .map_err(|e| format!("Source Paint Lock producer rejected: {e:?}"))?;
                        let normal = view
                            .picked_normal_alpha_none(
                                base,
                                SourceOriginalFaceContact {
                                    face_index,
                                    local_point: pass.center,
                                },
                            )
                            .map_err(|e| {
                                format!("Source Paint Lock picked-normal rejected: {e:?}")
                            })?;
                        let mut output = base.try_snapshot()?;
                        let mut options = s.options.clone();
                        options.tablet.pressure = pressure;
                        let request = PaintResponseRequest {
                            kind: BrushKind::Paint,
                            operation: PaintResponseOperation::OrdinaryDab,
                            in_contact: true,
                            options,
                            first: pass,
                            mirrored: None,
                        };
                        // The existing ordinary kernel's admission and arithmetic
                        // remain untouched; this owner supplies the lock semantics.
                        dab = match view {
                            SourcePaintNormalView::FreshStatic(f) => {
                                output.apply_paint_response_with_source_normals(&request, f)?
                            }
                            SourcePaintNormalView::RetainedGeometry(f) => output
                                .apply_paint_response_with_retained_source_normals(&request, f)?,
                        };
                        output.validate()?;
                        producer = Some(view.report());
                        picked_normal = Some(normal);
                        candidate = Some(output);
                        if s.mode == SourcePaintLockedMode::AbsolutePreview {
                            SourcePaintLockedAction::Preview
                        } else {
                            SourcePaintLockedAction::Accumulate
                        }
                    }
                }
            }
        };
        let before = self.require_active_mesh()?;
        let materials_before = before.materials.is_some();
        let mut live = PaintResponseReport::default();
        if let Some(output) = candidate {
            live = delta(before, &output);
            let at_original = channels_same(
                &output,
                self.stroke_start
                    .as_ref()
                    .and_then(|x| x.mesh.as_ref())
                    .ok_or("Source Paint Lock baseline missing")?,
            );
            let available = budget(
                self,
                s.retained_bytes(),
                output.approximate_bytes(),
                s.operation_limit,
            )?;
            let expected = binding(self, Some(&output), available)?;
            // Every new candidate/binding/fallible allocation has succeeded.
            if live.changed_vertices > 0 {
                if live.changed_material_vertices > 0 {
                    self.material_paint_stroke = true;
                }
                self.mesh = Some(output);
            }
            s.expected = expected;
            s.at_original = at_original;
        }
        let report = SourcePaintLockedReport {
            sequence: next,
            mode: s.mode,
            action,
            part: s.part,
            level: s.level,
            at_original: s.at_original,
            live,
            dab,
            materials_before,
            materials_after: self.require_active_mesh()?.materials.is_some(),
            producer,
            picked_normal,
        };
        s.sequence = next;
        s.last = Some(report.clone());
        Ok(report)
    }
    fn reject_source_paint_locked(
        &mut self,
        s: &mut SourcePaintLockedStroke,
        error: String,
    ) -> Result<(), String> {
        self.locked_owner(s)?;
        let previous = self
            .stroke_start
            .take()
            .ok_or("Source Paint Lock original state missing")?;
        self.exchange_state(previous);
        self.source_geometry_active = None;
        self.material_paint_stroke = false;
        self.layer_error = Some(format!(
            "Source Paint Lock rejected; whole original state restored: {error}"
        ));
        s.closed = true;
        s.at_original = true;
        Ok(())
    }
    pub fn cancel_source_paint_locked(
        &mut self,
        s: &mut SourcePaintLockedStroke,
    ) -> Result<SourcePaintLockedReport, String> {
        self.locked_owner(s)?;
        let before = self.require_active_mesh()?;
        let old = self
            .stroke_start
            .as_ref()
            .and_then(|x| x.mesh.as_ref())
            .ok_or("Source Paint Lock original state missing")?;
        let live = delta(before, old);
        let materials_before = before.materials.is_some();
        let materials_after = old.materials.is_some();
        let Some(next) = s.sequence.checked_add(1) else {
            self.reject_source_paint_locked(s, "Source Paint Lock sequence exhausted".into())?;
            return Err("Source Paint Lock sequence exhausted".into());
        };
        let previous = self
            .stroke_start
            .take()
            .ok_or("Source Paint Lock original state missing")?;
        self.exchange_state(previous);
        self.source_geometry_active = None;
        self.material_paint_stroke = false;
        s.closed = true;
        s.at_original = true;
        s.sequence = next;
        let report = SourcePaintLockedReport {
            sequence: next,
            mode: s.mode,
            action: SourcePaintLockedAction::Cancel,
            part: s.part,
            level: s.level,
            at_original: true,
            live,
            dab: PaintResponseReport::default(),
            materials_before,
            materials_after,
            producer: None,
            picked_normal: None,
        };
        s.last = Some(report.clone());
        Ok(report)
    }
    pub fn end_source_paint_locked(
        &mut self,
        s: &mut SourcePaintLockedStroke,
    ) -> Result<(), String> {
        self.locked_owner(s)?;
        if let Err(e) = self.locked_binding(s) {
            self.reject_source_paint_locked(s, e.clone())?;
            return Err(e);
        }
        // Only this validated owned end hands off to the established complete
        // material/hierarchy/layer/history commit and its rollback observer.
        self.source_geometry_active = None;
        self.end_stroke();
        s.closed = true;
        if let Some(e) = self.layer_error.as_ref() {
            s.at_original = true;
            return Err(e.clone());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "source_paint_lock_tests.rs"]
mod tests;
