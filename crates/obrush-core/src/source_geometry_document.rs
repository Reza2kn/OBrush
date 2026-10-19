//! Explicit transient Source geometry state + complete Document history.
//! Cache rows are not serialized; reimport/load is a declared fresh seed policy.
use super::{Document, HISTORY_BUDGET_BYTES};
use crate::{
    SourceGeometryError, SourceGeometryNoop, SourceGeometryNormalState, SourceGeometryOptions,
    SourceGeometryPass, SourceGeometryReport, SourceGeometryStroke,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use web_time::Instant;

// Transient ownership, never serialized. Both claim families use one unique
// allocator, so independently created documents cannot share an active claim.
pub(super) const WARM_PAINT_TAG: u64 = 1u64 << 63;
pub(super) static SOURCE_CLAIM_NONCE: AtomicU64 = AtomicU64::new(0);
/// CPU wall-time attribution for one successful Source geometry release.
/// These stages are diagnostic only; they do not authorize a geometry edit.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct SourceGeometryReleaseTiming {
    pub validation_ms: f64,
    pub history_ms: f64,
    pub balance_ms: f64,
    pub finalize_ms: f64,
    pub total_ms: f64,
}
pub(super) fn allocate_source_claim(counter: &AtomicU64) -> Result<u64, SourceGeometryError> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1).filter(|next| *next < WARM_PAINT_TAG)
        })
        .map(|previous| previous + 1)
        .map_err(|_| SourceGeometryError::StorageLimit)
}

/// Constructible only inside this Document child after complete public-entry
/// validation. It holds an exclusive borrow until the one atomic core call ends;
/// no proof is stored or authorized by an Arc/Weak/pointer/epoch between dabs.
pub(crate) struct ValidatedSourceDab<'a> {
    mesh: &'a mut crate::Mesh,
    state: &'a SourceGeometryNormalState,
    pass: SourceGeometryPass,
    pressure: f64,
    in_contact: bool,
}
impl<'a> ValidatedSourceDab<'a> {
    fn from_checked_document(
        mesh: &'a mut crate::Mesh,
        state: &'a SourceGeometryNormalState,
        pass: SourceGeometryPass,
        pressure: f64,
        in_contact: bool,
        stroke: &SourceGeometryStroke,
    ) -> Result<Self, SourceGeometryError> {
        // All public Mesh/cache/domain/input checks have just run in the same
        // exclusive Document entry. Generation was previously checked here at
        // the old core-call boundary, after existing repository/cap preflight.
        if stroke
            .expected_generation
            .is_some_and(|g| g != state.generation())
        {
            return Err(SourceGeometryError::StaleNormalState);
        }
        Ok(Self {
            mesh,
            state,
            pass,
            pressure,
            in_contact,
        })
    }
    pub(crate) fn with_parts<R>(
        self,
        apply: impl FnOnce(
            &'a mut crate::Mesh,
            &'a SourceGeometryNormalState,
            SourceGeometryPass,
            f64,
            bool,
        ) -> R,
    ) -> R {
        apply(
            self.mesh,
            self.state,
            self.pass,
            self.pressure,
            self.in_contact,
        )
    }
}

/// Controlled counterpart of ValidatedSourceDab. Construction stays private
/// to the Document child that just checked the complete public entry. The
/// exclusive mesh borrow and all captured inputs are consumed in this call;
/// this never authorizes a later dab from a pointer, epoch, or saved flag.
pub(crate) struct ValidatedControlledSourceDab<'a> {
    mesh: &'a mut crate::Mesh,
    state: &'a Arc<SourceGeometryNormalState>,
    contacts: [Option<SourceGeometryPass>; 2],
    pressure: f64,
    in_contact: bool,
    controls: &'a crate::SourceGeometryControls,
    tracks: crate::SourceGeometryAlphaTracks,
}
impl<'a> ValidatedControlledSourceDab<'a> {
    #[allow(clippy::too_many_arguments)] // Bind every validated atomic input.
    fn from_checked_document(
        mesh: &'a mut crate::Mesh,
        state: &'a Arc<SourceGeometryNormalState>,
        contacts: [Option<SourceGeometryPass>; 2],
        pressure: f64,
        in_contact: bool,
        controls: &'a crate::SourceGeometryControls,
        tracks: crate::SourceGeometryAlphaTracks,
        stroke: &SourceGeometryStroke,
    ) -> Result<Self, SourceGeometryError> {
        // Kept at the original inner boundary, after repository/cap admission.
        // Mesh/state/domain/options/tracks/contact/scalar checks already ran
        // within this exclusive Document entry; no mutable borrow escaped.
        if stroke
            .expected_generation
            .is_some_and(|g| g != state.generation())
        {
            return Err(SourceGeometryError::StaleNormalState);
        }
        Ok(Self {
            mesh,
            state,
            contacts,
            pressure,
            in_contact,
            controls,
            tracks,
        })
    }
    pub(crate) fn with_parts<R>(
        self,
        apply: impl FnOnce(
            &'a mut crate::Mesh,
            &'a Arc<SourceGeometryNormalState>,
            [Option<SourceGeometryPass>; 2],
            f64,
            bool,
            &'a crate::SourceGeometryControls,
            crate::SourceGeometryAlphaTracks,
        ) -> R,
    ) -> R {
        apply(
            self.mesh,
            self.state,
            self.contacts,
            self.pressure,
            self.in_contact,
            self.controls,
            self.tracks,
        )
    }
}

/// A single fully checked contact preparation and its exclusive document borrow.
/// Dropping this value makes no edit. Applying it consumes the borrow and uses
/// the same atomic brush implementation as the standalone contact API.
/// No document or stroke mutation can occur between preparation and application.
pub struct PreparedSourceGeometryDab<'a> {
    document: &'a mut Document,
    stroke: &'a mut SourceGeometryStroke,
    contacts: crate::SourceGeometryContacts,
    retained_state_verified: bool,
    exact_domain_verified: bool,
    preparation_time: SourceGeometryPreparationTime,
}
/// An immutable render pair captured immediately after a checked controlled
/// dab. Its private constructor prevents an arbitrary mesh/state pair from
/// bypassing the renderer's ordinary stale-state validation.
#[derive(Clone)]
pub struct CertifiedSourceRenderSnapshot {
    snapshot: crate::SubtoolRenderSnapshot,
    state: Arc<SourceGeometryNormalState>,
}
impl CertifiedSourceRenderSnapshot {
    pub fn snapshot(&self) -> &crate::SubtoolRenderSnapshot {
        &self.snapshot
    }
    pub fn state(&self) -> &Arc<SourceGeometryNormalState> {
        &self.state
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SourceGeometryPreparationTime {
    pub retained_match: std::time::Duration,
    pub contact_prepare: std::time::Duration,
}
impl PreparedSourceGeometryDab<'_> {
    pub fn contacts(&self) -> crate::SourceGeometryContacts {
        self.contacts
    }
    pub fn preparation_time(&self) -> SourceGeometryPreparationTime {
        self.preparation_time
    }
    pub fn apply(
        self,
        pressure: f64,
        in_contact: bool,
        controls: &crate::SourceGeometryControls,
        tracks: crate::SourceGeometryAlphaTracks,
    ) -> Result<crate::SourceGeometryControlledReport, SourceGeometryError> {
        self.apply_inner(pressure, in_contact, controls, tracks, false)
            .map(|(report, _)| report)
    }
    pub fn apply_with_render_snapshot(
        self,
        pressure: f64,
        in_contact: bool,
        controls: &crate::SourceGeometryControls,
        tracks: crate::SourceGeometryAlphaTracks,
    ) -> Result<
        (
            crate::SourceGeometryControlledReport,
            Option<CertifiedSourceRenderSnapshot>,
        ),
        SourceGeometryError,
    > {
        self.apply_inner(pressure, in_contact, controls, tracks, true)
    }
    fn apply_inner(
        self,
        pressure: f64,
        in_contact: bool,
        controls: &crate::SourceGeometryControls,
        tracks: crate::SourceGeometryAlphaTracks,
        capture_render_snapshot: bool,
    ) -> Result<
        (
            crate::SourceGeometryControlledReport,
            Option<CertifiedSourceRenderSnapshot>,
        ),
        SourceGeometryError,
    > {
        let doc = self.document;
        doc.validate_source_geometry_stroke_binding(self.stroke)?;
        let mesh = doc.source_geometry_domain(self.stroke.options())?;
        // Contact preparation checked every Mesh row while this exclusive
        // borrow was held. The geometry-specific convention remains required.
        if mesh.mask_convention != crate::MaskConvention::UnprotectedF32V1 {
            return Err(SourceGeometryError::MaskConvention);
        }
        doc.source_geometry_state_blocker_admitted(
            mesh,
            self.stroke.options(),
            self.retained_state_verified,
        )?;
        let report = doc.apply_source_geometry_controlled_admitted(
            self.stroke,
            self.contacts.passes,
            pressure,
            in_contact,
            controls,
            tracks,
            self.exact_domain_verified,
        )?;
        let certified = if capture_render_snapshot && report.combined.changed_vertices > 0 {
            let id = doc
                .active_subtool_id()
                .ok_or(SourceGeometryError::UnsupportedDomain)?;
            let state = doc
                .source_geometry_state(id)
                .ok_or(SourceGeometryError::StaleNormalState)?;
            let snapshot = doc
                .subtool_render_snapshots()
                .into_iter()
                .find(|snapshot| snapshot.id == id && snapshot.active)
                .ok_or(SourceGeometryError::UnsupportedDomain)?;
            Some(CertifiedSourceRenderSnapshot { snapshot, state })
        } else {
            None
        };
        Ok((report, certified))
    }
}

pub(super) fn retained_bytes(
    rows: &[(u32, Arc<SourceGeometryNormalState>)],
    capacity: usize,
) -> usize {
    capacity
        .saturating_mul(std::mem::size_of::<(u32, Arc<SourceGeometryNormalState>)>())
        .saturating_add(
            rows.iter()
                .map(|(_, s)| s.approximate_bytes())
                .fold(0, usize::saturating_add),
        )
}

impl Document {
    /// Publish a completed off-thread Source gesture as one public history
    /// event. The caller must first prove a one-use scene/epoch witness and
    /// decode the checked mesh and retained-state packets. No fallible work
    /// remains after the state exchange begins.
    pub(crate) fn publish_cold_source_worker_result(
        &mut self,
        candidate: crate::Mesh,
        retained: Arc<SourceGeometryNormalState>,
    ) -> Result<bool, String> {
        if self.stroke_start.is_some()
            || self.source_geometry_active.is_some()
            || self.material_paint_stroke
            || self.has_source_geometry_normal_state()
        {
            return Err("Source worker publication requires an idle cold document".into());
        }
        let id = self.active_subtool_id().ok_or("No active Source part")?;
        let before = self.active_mesh().ok_or("No active Source mesh")?;
        // The delta decoder already checks these fields. Keep a second check
        // here so future callers cannot publish a changed mask, topology or
        // attribute through this private Document entry.
        crate::encode_source_worker_mesh_delta(before, &candidate, id)?;
        if !retained.matches_mesh(&candidate) {
            return Err("Source worker retained state does not match the candidate mesh".into());
        }
        if retained.approximate_bytes() > crate::MAX_SOURCE_GEOMETRY_STATE_BYTES {
            return Err("Source worker retained state exceeds its budget".into());
        }
        self.check_scene_candidate(&candidate, None, None, None, self.scene.as_ref())?;
        Self::check_output_budget(
            &candidate,
            &self.name,
            self.multiresolution.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
        )?;
        let previous = self.snapshot_state();
        if previous
            .approximate_bytes()
            .checked_add(retained.approximate_bytes())
            .is_none_or(|bytes| bytes > HISTORY_BUDGET_BYTES)
        {
            return Err("Source worker history exceeds its budget".into());
        }
        let changed = !before
            .positions
            .iter()
            .zip(&candidate.positions)
            .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits))
            || !before
                .normals
                .iter()
                .zip(&candidate.normals)
                .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits));
        // A no-op Source gesture can still establish transient retained state,
        // but must not erase the existing redo branch.
        self.source_geometry_states
            .try_reserve_exact(1)
            .map_err(|_| "Cannot reserve Source worker retained state")?;
        self.mesh = Some(candidate);
        self.source_geometry_states.push((id, retained));
        if changed {
            self.commit_previous(previous);
        }
        Ok(changed)
    }

    /// Adopt the caller's normal Paint snapshot after proving its captured
    /// per-object producer. Existing cold/generic history APIs are unchanged.
    pub fn claim_retained_source_paint_stroke(
        &mut self,
        part: u32,
        level: usize,
        state: &Arc<SourceGeometryNormalState>,
    ) -> Result<crate::SourceRetainedPaintStroke, SourceGeometryError> {
        self.claim_retained_source_paint_stroke_with_counter(
            part,
            level,
            state,
            &SOURCE_CLAIM_NONCE,
        )
    }
    fn claim_retained_source_paint_stroke_with_counter(
        &mut self,
        part: u32,
        level: usize,
        state: &Arc<SourceGeometryNormalState>,
        counter: &AtomicU64,
    ) -> Result<crate::SourceRetainedPaintStroke, SourceGeometryError> {
        if self.source_geometry_active.is_some() {
            return Err(SourceGeometryError::StrokeAlreadyOpen);
        }
        // Validate before creating a direct caller's lazy snapshot. A queued
        // Begin already owns the exact earlier state and must not be replaced.
        let (old_part, old_level, old_cache, mesh, unsupported) =
            if let Some(previous) = self.stroke_start.as_ref() {
                (
                    previous
                        .scene
                        .as_ref()
                        .map_or_else(|| previous.mesh.as_ref().map(|_| 1), |s| s.active_id),
                    previous.multiresolution.as_ref().map_or(0, |s| s.active),
                    previous
                        .source_geometry_states
                        .iter()
                        .find(|(id, _)| *id == part)
                        .map(|(_, s)| s),
                    previous.mesh.as_ref(),
                    previous.multiresolution.is_some() || previous.layers.is_some(),
                )
            } else {
                (
                    self.active_subtool_id(),
                    self.current_level(),
                    self.source_geometry_states
                        .iter()
                        .find(|(id, _)| *id == part)
                        .map(|(_, s)| s),
                    self.active_mesh(),
                    self.multiresolution.is_some() || self.layers.is_some(),
                )
            };
        if old_part != Some(part)
            || old_level != level
            || unsupported
            || !old_cache.is_some_and(|s| Arc::ptr_eq(s, state))
        {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        let mesh = mesh.ok_or(SourceGeometryError::InvalidMesh)?;
        mesh.validate()
            .map_err(|_| SourceGeometryError::InvalidMesh)?;
        if !state.matches_mesh(mesh) {
            return Err(SourceGeometryError::StaleNormalState);
        }
        let sequence = allocate_source_claim(counter)?;
        self.begin_stroke();
        if self.stroke_start.is_none() {
            return Err(SourceGeometryError::InvalidMesh);
        }
        self.begin_source_attribute_history(part)?;
        let token = sequence | WARM_PAINT_TAG;
        self.source_geometry_token = sequence;
        self.source_geometry_active = Some(token);
        Ok(crate::SourceRetainedPaintStroke {
            token,
            part,
            level,
            cache: Arc::downgrade(state),
        })
    }
    fn retained_paint_owner_matches(&self, owner: &crate::SourceRetainedPaintStroke) -> bool {
        let Some(previous) = self.stroke_start.as_ref() else {
            return false;
        };
        let Some(cache) = owner.cache.upgrade() else {
            return false;
        };
        let old_part = previous
            .scene
            .as_ref()
            .map_or_else(|| previous.mesh.as_ref().map(|_| 1), |s| s.active_id);
        let old_level = previous.multiresolution.as_ref().map_or(0, |s| s.active);
        self.source_geometry_active == Some(owner.token)
            && old_part == Some(owner.part)
            && old_level == owner.level
            && previous
                .source_geometry_states
                .iter()
                .any(|(id, s)| *id == owner.part && Arc::ptr_eq(s, &cache))
    }
    pub fn validate_retained_source_paint_stroke(
        &self,
        owner: &crate::SourceRetainedPaintStroke,
    ) -> Result<(), SourceGeometryError> {
        if !self.retained_paint_owner_matches(owner) {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        Ok(())
    }
    /// A failure restores every geometry/attribute/cache/scene/history scope in
    /// the owned snapshot. The existing commit-error observer restores host
    /// dirty/color state. Expired/foreign owners cannot exchange any state.
    pub fn reject_retained_source_paint_stroke(
        &mut self,
        owner: &crate::SourceRetainedPaintStroke,
        error: String,
    ) -> Result<(), SourceGeometryError> {
        self.validate_retained_source_paint_stroke(owner)?;
        let previous = self
            .stroke_start
            .take()
            .ok_or(SourceGeometryError::NoOpenStroke)?;
        self.exchange_state(previous);
        self.source_geometry_active = None;
        self.material_paint_stroke = false;
        self.layer_error = Some(error);
        Ok(())
    }
    /// Generic normal-queue release validates only a warm Paint claim. Other
    /// histories execute their established end path without this new policy.
    pub(super) fn prepare_retained_source_paint_end(&mut self) -> Result<(), SourceGeometryError> {
        let Some(token) = self
            .source_geometry_active
            .filter(|token| token & WARM_PAINT_TAG != 0)
        else {
            return Ok(());
        };
        let result = (|| {
            let previous = self
                .stroke_start
                .as_ref()
                .ok_or(SourceGeometryError::NoOpenStroke)?;
            let old_part = previous
                .scene
                .as_ref()
                .map_or_else(|| previous.mesh.as_ref().map(|_| 1), |s| s.active_id);
            let old_level = previous.multiresolution.as_ref().map_or(0, |s| s.active);
            if old_part != self.active_subtool_id() || old_level != self.current_level() {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
            let part = old_part.ok_or(SourceGeometryError::StrokeBindingChanged)?;
            let captured = previous
                .source_geometry_states
                .iter()
                .find(|(id, _)| *id == part)
                .map(|(_, s)| s)
                .ok_or(SourceGeometryError::StaleNormalState)?;
            let current = self
                .source_geometry_states
                .iter()
                .find(|(id, _)| *id == part)
                .map(|(_, s)| s)
                .ok_or(SourceGeometryError::StaleNormalState)?;
            if !Arc::ptr_eq(captured, current) {
                return Err(SourceGeometryError::StaleNormalState);
            }
            self.source_geometry_blocker(SourceGeometryOptions::inflate())?;
            Ok(())
        })();
        if let Some(error) = result
            .as_ref()
            .err()
            .filter(|_| self.source_geometry_active == Some(token))
        {
            if let Some(previous) = self.stroke_start.take() {
                self.exchange_state(previous);
            }
            self.source_geometry_active = None;
            self.material_paint_stroke = false;
            self.layer_error = Some(format!(
                "Retained Source Paint release rejected; original snapshot restored: {error}"
            ));
        }
        result
    }
    pub(super) fn prune_source_geometry_states(&mut self) {
        let scene = self.scene.as_ref();
        let standalone = self.mesh.is_some();
        self.source_geometry_states.retain(|(id, _)| {
            scene.map_or(standalone && *id == 1, |s| {
                s.objects.iter().any(|o| o.id == *id)
            })
        });
        if self.source_geometry_states.is_empty() {
            self.source_geometry_states = Vec::new();
        }
    }
    /// Read-only transient claim; not project/history data.
    pub fn source_geometry_active_claim(&self) -> Option<u64> {
        self.source_geometry_active
    }
    pub fn source_geometry_state(&self, id: u32) -> Option<Arc<SourceGeometryNormalState>> {
        self.source_geometry_states
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, s)| s.clone())
    }
    pub fn source_geometry_retained_bytes(&self) -> usize {
        retained_bytes(
            &self.source_geometry_states,
            self.source_geometry_states.capacity(),
        )
    }
    /// Presence, rather than estimated allocation size, is the admission
    /// witness for a cross-realm cold Source capture. A state can be semantically
    /// live even if a future compact representation reports zero payload bytes.
    pub fn has_source_geometry_normal_state(&self) -> bool {
        !self.source_geometry_states.is_empty()
    }
    fn source_geometry_domain(
        &self,
        options: SourceGeometryOptions,
    ) -> Result<&crate::Mesh, SourceGeometryError> {
        options.validate()?;
        if self.multiresolution.is_some() || self.layers.is_some() || !self.active_subtool_visible()
        {
            return Err(SourceGeometryError::UnsupportedDomain);
        }
        self.active_mesh()
            .ok_or(SourceGeometryError::UnsupportedDomain)
    }
    pub fn source_geometry_blocker(
        &self,
        options: SourceGeometryOptions,
    ) -> Result<(), SourceGeometryError> {
        let mesh = self.source_geometry_domain(options)?;
        crate::source_geometry::validate_mesh(mesh)?;
        self.source_geometry_state_blocker(mesh, options)
    }
    fn source_geometry_state_blocker(
        &self,
        mesh: &crate::Mesh,
        options: SourceGeometryOptions,
    ) -> Result<(), SourceGeometryError> {
        self.source_geometry_state_blocker_admitted(mesh, options, false)
    }
    /// A prepared dab may carry an exact retained-state match from contact
    /// preparation across its still-live exclusive Document borrow. Ordinary
    /// public entries and failed retained picks always perform the scan.
    fn source_geometry_state_blocker_admitted(
        &self,
        mesh: &crate::Mesh,
        options: SourceGeometryOptions,
        retained_state_verified: bool,
    ) -> Result<(), SourceGeometryError> {
        let id = self
            .active_subtool_id()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        if let Some(state) = self.source_geometry_state(id) {
            if !retained_state_verified && !state.matches_mesh(mesh) {
                return Err(SourceGeometryError::StaleNormalState);
            }
        }
        if matches!(
            options.tool,
            crate::SourceGeometryTool::Brush { clay: true, .. }
        ) && options.clay_order == crate::SourceClayOrderPolicy::SingleLeaf
            && mesh
                .surface
                .as_ref()
                .map_or(mesh.triangles.len(), |s| s.faces.len())
                > crate::SOURCE_CLAY_SINGLE_LEAF_FACES
        {
            return Err(SourceGeometryError::UnsupportedClayOrder);
        }
        Ok(())
    }
    /// Captures ordinary history, with no initial dab/cache/proxy allocation.
    pub fn begin_source_geometry_stroke(
        &mut self,
        options: SourceGeometryOptions,
    ) -> Result<SourceGeometryStroke, SourceGeometryError> {
        self.begin_source_geometry_stroke_with_counter(options, &SOURCE_CLAIM_NONCE)
    }
    /// Captured-control Source sessions additionally retain the pre-culling
    /// selected vertex union for reference normal history. The older ordinary
    /// dab API keeps its established snapshot history contract.
    pub fn begin_source_geometry_selection_history(
        &mut self,
        options: SourceGeometryOptions,
    ) -> Result<SourceGeometryStroke, SourceGeometryError> {
        let mut stroke = self.begin_source_geometry_stroke(options)?;
        stroke.history_selected = Some(Arc::new(Vec::new()));
        Ok(stroke)
    }
    fn begin_source_geometry_stroke_with_counter(
        &mut self,
        options: SourceGeometryOptions,
        counter: &AtomicU64,
    ) -> Result<SourceGeometryStroke, SourceGeometryError> {
        self.source_geometry_blocker(options)?;
        if self.stroke_start.is_some() {
            return Err(SourceGeometryError::StrokeAlreadyOpen);
        }
        let token = allocate_source_claim(counter)?;
        let mut stroke = SourceGeometryStroke::new(options)?;
        stroke.binding = Some((
            self.active_subtool_id()
                .ok_or(SourceGeometryError::UnsupportedDomain)?,
            self.current_level(),
        ));
        stroke.expected_generation = self
            .source_geometry_state(stroke.binding.expect("bound").0)
            .map(|s| s.generation());
        stroke.document_token = token;
        self.begin_stroke();
        if self.stroke_start.is_none() {
            return Err(SourceGeometryError::InvalidMesh);
        }
        self.source_geometry_token = token;
        self.source_geometry_active = Some(token);
        Ok(stroke)
    }
    pub fn apply_source_geometry_dab(
        &mut self,
        stroke: &mut SourceGeometryStroke,
        pass: SourceGeometryPass,
        pressure: f64,
        in_contact: bool,
    ) -> Result<SourceGeometryReport, SourceGeometryError> {
        if stroke.history_selected.is_some() {
            return Err(SourceGeometryError::UnsupportedDomain);
        }
        if self.stroke_start.is_none() || self.source_geometry_active != Some(stroke.document_token)
        {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        if stroke.binding
            != self
                .active_subtool_id()
                .map(|id| (id, self.current_level()))
        {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        self.source_geometry_blocker(stroke.options())?;
        let mesh = self
            .active_mesh()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        pass.validate(mesh)?;
        self.validate_source_geometry_capture(stroke)?;
        if !pressure.is_finite() || !(0.0..=1.0).contains(&pressure) {
            return Err(SourceGeometryError::InvalidInput);
        }
        if !in_contact {
            return Ok(SourceGeometryReport::noop(SourceGeometryNoop::NoContact));
        }
        let intensity = stroke.options().intensity
            * (1. + stroke.options().intensity_factor * (pressure * 2. - 1.));
        let id = self
            .active_subtool_id()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        let prior = self.source_geometry_state(id);
        if pass.local_radius_squared == 0. || (intensity == 0. && prior.is_none()) {
            return Ok(SourceGeometryReport::noop(
                SourceGeometryNoop::ZeroIntensity,
            ));
        }
        if prior.is_none() && !crate::source_geometry::possible_contribution(mesh, pass) {
            return Ok(SourceGeometryReport::noop(SourceGeometryNoop::AllProtected));
        }
        let state = match prior.as_ref() {
            Some(s) => s.clone(),
            None => Arc::new(SourceGeometryNormalState::seed(mesh)?),
        };
        let prior_bytes = prior.as_ref().map_or(0, |s| s.approximate_bytes());
        if prior.is_none() && self.source_geometry_states.len() >= crate::MAX_SUBTOOLS {
            return Err(SourceGeometryError::StorageLimit);
        }
        let mut repository = if prior.is_none() {
            let mut rows = Vec::new();
            rows.try_reserve_exact(self.source_geometry_states.len() + 1)
                .map_err(|_| SourceGeometryError::Allocation)?;
            rows.extend(
                self.source_geometry_states
                    .iter()
                    .map(|(id, s)| (*id, s.clone())),
            );
            Some(rows)
        } else {
            None
        };
        let entry_size = std::mem::size_of::<(u32, Arc<SourceGeometryNormalState>)>();
        let old_repository = self
            .source_geometry_states
            .capacity()
            .saturating_mul(entry_size);
        let next_repository = repository.as_ref().map_or(old_repository, |rows| {
            rows.capacity().saturating_mul(entry_size)
        });
        let other_cache = self
            .source_geometry_retained_bytes()
            .saturating_sub(old_repository)
            .saturating_sub(prior_bytes)
            .checked_add(next_repository)
            .ok_or(SourceGeometryError::StorageLimit)?;
        let previous = self
            .stroke_start
            .as_ref()
            .ok_or(SourceGeometryError::NoOpenStroke)?;
        let non_cache = previous.approximate_bytes().saturating_sub(retained_bytes(
            &previous.source_geometry_states,
            previous.source_geometry_states.capacity(),
        ));
        let max_state_bytes = crate::MAX_SOURCE_GEOMETRY_STATE_BYTES
            .checked_sub(other_cache)
            .and_then(|cache_limit| {
                HISTORY_BUDGET_BYTES
                    .checked_sub(non_cache.checked_add(other_cache)?)
                    .map(|scene_limit| cache_limit.min(scene_limit))
            })
            .ok_or(SourceGeometryError::StorageLimit)?;
        if state.approximate_bytes() > max_state_bytes {
            return Err(SourceGeometryError::StorageLimit);
        }
        let verified = ValidatedSourceDab::from_checked_document(
            self.mesh
                .as_mut()
                .ok_or(SourceGeometryError::UnsupportedDomain)?,
            &state,
            pass,
            pressure,
            in_contact,
            stroke,
        )?;
        let (report, next) = stroke.apply_document_seal(verified, max_state_bytes)?;
        if let Some(next) = next {
            // Actual candidate capacities and repository capacity are charged
            // before core acceptance; approximate_bytes includes the Arc node.
            let next = Arc::new(next);
            if let Some((_, old)) = self
                .source_geometry_states
                .iter_mut()
                .find(|(key, _)| *key == id)
            {
                *old = next;
            } else {
                let mut rows = repository.take().expect("reserved source repository");
                rows.push((id, next));
                self.source_geometry_states = rows;
            }
        }
        self.attach_source_selection_history(stroke);
        Ok(report)
    }
    /// Prepare contacts against the actual active pose, retaining exclusive
    /// access to both document and stroke until apply or drop. Existing public
    /// contact preparation performs the complete Mesh validation here.
    ///
    /// The document cannot be changed while a prepared value is retained:
    /// ```compile_fail
    /// use obrush_core::{Document, SourceGeometryStroke, PaintPreparationInput};
    /// fn cannot_edit(doc: &mut Document, stroke: &mut SourceGeometryStroke, input: PaintPreparationInput) {
    ///     let prepared = doc.prepare_source_geometry_controlled_dab(stroke, input, None).unwrap();
    ///     doc.name = "changed".into();
    ///     let _ = prepared.contacts();
    /// }
    /// ```
    /// Application consumes the prepared value:
    /// ```compile_fail
    /// use obrush_core::{Document, SourceGeometryStroke, PaintPreparationInput, SourceGeometryControls, SourceGeometryAlphaTracks};
    /// fn cannot_repeat(doc: &mut Document, stroke: &mut SourceGeometryStroke, input: PaintPreparationInput) {
    ///     let prepared = doc.prepare_source_geometry_controlled_dab(stroke, input, None).unwrap();
    ///     let _ = prepared.apply(0.5, true, &SourceGeometryControls::default(), SourceGeometryAlphaTracks::default());
    ///     let _ = prepared.contacts();
    /// }
    /// ```
    pub fn prepare_source_geometry_controlled_dab<'a>(
        &'a mut self,
        stroke: &'a mut SourceGeometryStroke,
        input: crate::PaintPreparationInput,
        mirror: Option<crate::SourceMirrorPlane>,
    ) -> Result<PreparedSourceGeometryDab<'a>, SourceGeometryError> {
        let mesh = self
            .active_mesh()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        let pose = self.active_object_metadata().map(|m| &m.affine);
        let retained_started = Instant::now();
        let retained = self
            .active_subtool_id()
            .and_then(|id| self.source_geometry_state(id));
        let verified = retained.as_deref().and_then(|state| {
            crate::paint_preparation::VerifiedRetainedPick::for_stroke_mesh(mesh, state, stroke)
        });
        let retained_state_verified = verified.is_some();
        let exact_domain_verified = verified.is_some_and(|pick| pick.exact_domain());
        let retained_match = retained_started.elapsed();
        let contacts_started = Instant::now();
        let contacts = match verified {
            Some(verified) => {
                crate::source_geometry_controls::prepare_source_geometry_contacts_with_state(
                    mesh, pose, input, mirror, verified,
                )?
            }
            _ => crate::prepare_source_geometry_contacts(mesh, pose, input, mirror)?,
        };
        let contact_prepare = contacts_started.elapsed();
        Ok(PreparedSourceGeometryDab {
            document: self,
            stroke,
            contacts,
            retained_state_verified,
            exact_domain_verified,
            preparation_time: SourceGeometryPreparationTime {
                retained_match,
                contact_prepare,
            },
        })
    }
    fn validate_source_geometry_stroke_binding(
        &self,
        stroke: &SourceGeometryStroke,
    ) -> Result<(), SourceGeometryError> {
        if self.stroke_start.is_none() || self.source_geometry_active != Some(stroke.document_token)
        {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        if stroke.binding
            != self
                .active_subtool_id()
                .map(|id| (id, self.current_level()))
        {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        Ok(())
    }
    pub fn apply_source_geometry_controlled_dab(
        &mut self,
        stroke: &mut SourceGeometryStroke,
        contacts: [Option<SourceGeometryPass>; 2],
        pressure: f64,
        in_contact: bool,
        controls: &crate::SourceGeometryControls,
        tracks: crate::SourceGeometryAlphaTracks,
    ) -> Result<crate::SourceGeometryControlledReport, SourceGeometryError> {
        self.validate_source_geometry_stroke_binding(stroke)?;
        self.source_geometry_blocker(stroke.options())?;
        self.apply_source_geometry_controlled_admitted(
            stroke, contacts, pressure, in_contact, controls, tracks, false,
        )
    }
    // Private continuation reachable only after full public validation, or
    // from the prepared value which still owns the same exclusive borrow.
    #[allow(clippy::too_many_arguments)] // Keep every captured admission input explicit.
    fn apply_source_geometry_controlled_admitted(
        &mut self,
        stroke: &mut SourceGeometryStroke,
        contacts: [Option<SourceGeometryPass>; 2],
        pressure: f64,
        in_contact: bool,
        controls: &crate::SourceGeometryControls,
        tracks: crate::SourceGeometryAlphaTracks,
        exact_domain_verified: bool,
    ) -> Result<crate::SourceGeometryControlledReport, SourceGeometryError> {
        let mesh = self
            .active_mesh()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        tracks.validate()?;
        if !controls.mirror && contacts[1].is_some() {
            return Err(SourceGeometryError::InvalidInput);
        }
        for pass in contacts.iter().flatten() {
            pass.validate(mesh)?;
        }
        if !exact_domain_verified {
            self.validate_source_geometry_capture(stroke)?;
        }
        if !pressure.is_finite() || !(0.0..=1.0).contains(&pressure) {
            return Err(SourceGeometryError::InvalidInput);
        }
        if !in_contact {
            return Ok(controlled_noop(
                contacts,
                tracks,
                controls,
                SourceGeometryNoop::NoContact,
            ));
        }
        let intensity = stroke.options().intensity
            * (1. + stroke.options().intensity_factor * (pressure * 2. - 1.));
        let id = self
            .active_subtool_id()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        let prior = self.source_geometry_state(id);
        if contacts
            .iter()
            .flatten()
            .all(|p| p.local_radius_squared == 0.0)
            || (intensity == 0. && prior.is_none())
        {
            let updated = self.prepare_source_geometry_press_alpha(stroke, contacts, tracks)?;
            stroke.record_history_noop(mesh, contacts)?;
            self.attach_source_selection_history(stroke);
            return Ok(controlled_noop(
                contacts,
                updated,
                controls,
                SourceGeometryNoop::ZeroIntensity,
            ));
        }
        if prior.is_none()
            && (!contacts
                .iter()
                .flatten()
                .any(|p| crate::source_geometry::possible_contribution(mesh, *p))
                || controls.alpha.as_ref().is_some_and(|a| !a.has_signal()))
        {
            let updated = self.prepare_source_geometry_press_alpha(stroke, contacts, tracks)?;
            stroke.record_history_noop(mesh, contacts)?;
            self.attach_source_selection_history(stroke);
            return Ok(controlled_noop(
                contacts,
                updated,
                controls,
                SourceGeometryNoop::AllProtected,
            ));
        }
        let state = match prior.as_ref() {
            Some(s) => s.clone(),
            None => Arc::new(SourceGeometryNormalState::seed(mesh)?),
        };
        let prior_bytes = prior.as_ref().map_or(0, |s| s.approximate_bytes());
        if prior.is_none() && self.source_geometry_states.len() >= crate::MAX_SUBTOOLS {
            return Err(SourceGeometryError::StorageLimit);
        }
        let mut repository = if prior.is_none() {
            let mut rows = Vec::new();
            rows.try_reserve_exact(self.source_geometry_states.len() + 1)
                .map_err(|_| SourceGeometryError::Allocation)?;
            rows.extend(
                self.source_geometry_states
                    .iter()
                    .map(|(id, s)| (*id, s.clone())),
            );
            Some(rows)
        } else {
            None
        };
        let entry_size = std::mem::size_of::<(u32, Arc<SourceGeometryNormalState>)>();
        let old_repository = self
            .source_geometry_states
            .capacity()
            .saturating_mul(entry_size);
        let next_repository = repository.as_ref().map_or(old_repository, |rows| {
            rows.capacity().saturating_mul(entry_size)
        });
        let other_cache = self
            .source_geometry_retained_bytes()
            .saturating_sub(old_repository)
            .saturating_sub(prior_bytes)
            .checked_add(next_repository)
            .ok_or(SourceGeometryError::StorageLimit)?;
        let previous = self
            .stroke_start
            .as_ref()
            .ok_or(SourceGeometryError::NoOpenStroke)?;
        let history_extra = if stroke.history_selected.is_some() {
            super::source_history::SourceHistoryEvent::selection_bytes(mesh.positions.len())
                .saturating_sub(
                    previous
                        .source_history
                        .as_ref()
                        .map_or(0, super::source_history::SourceHistoryEvent::bytes),
                )
        } else {
            0
        };
        let non_cache = previous
            .approximate_bytes()
            .saturating_sub(retained_bytes(
                &previous.source_geometry_states,
                previous.source_geometry_states.capacity(),
            ))
            .checked_add(history_extra)
            .ok_or(SourceGeometryError::StorageLimit)?;
        let max_state_bytes = crate::MAX_SOURCE_GEOMETRY_STATE_BYTES
            .checked_sub(other_cache)
            .and_then(|cache_limit| {
                HISTORY_BUDGET_BYTES
                    .checked_sub(
                        non_cache
                            .checked_add(other_cache)?
                            .checked_add(controls.retained_bytes())?,
                    )
                    .map(|scene_limit| cache_limit.min(scene_limit))
            })
            .ok_or(SourceGeometryError::StorageLimit)?;
        if state.approximate_bytes() > max_state_bytes {
            return Err(SourceGeometryError::StorageLimit);
        }
        let verified = ValidatedControlledSourceDab::from_checked_document(
            self.mesh
                .as_mut()
                .ok_or(SourceGeometryError::UnsupportedDomain)?,
            &state,
            contacts,
            pressure,
            in_contact,
            controls,
            tracks,
            stroke,
        )?;
        let (report, next) = stroke.apply_document_controlled_seal(verified, max_state_bytes)?;
        if let Some(next) = next {
            // Actual candidate capacities and repository capacity are charged
            // before core acceptance; approximate_bytes includes the Arc node.
            if let Some((_, old)) = self
                .source_geometry_states
                .iter_mut()
                .find(|(key, _)| *key == id)
            {
                *old = next;
            } else {
                let mut rows = repository.take().expect("reserved source repository");
                rows.push((id, next));
                self.source_geometry_states = rows;
            }
        }
        self.attach_source_selection_history(stroke);
        Ok(report)
    }

    /// Read-only Source press preparation. No absent persistent cache or history
    /// allocation is committed; a rejected alpha contact leaves exact document.
    pub fn prepare_source_geometry_press_alpha(
        &self,
        stroke: &SourceGeometryStroke,
        contacts: [Option<SourceGeometryPass>; 2],
        tracks: crate::SourceGeometryAlphaTracks,
    ) -> Result<crate::SourceGeometryAlphaTracks, SourceGeometryError> {
        if self.source_geometry_active != Some(stroke.document_token) || self.stroke_start.is_none()
        {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        if stroke.binding
            != self
                .active_subtool_id()
                .map(|id| (id, self.current_level()))
        {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        self.source_geometry_blocker(stroke.options())?;
        self.validate_source_geometry_capture(stroke)?;
        let mesh = self
            .active_mesh()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        let state = self
            .source_geometry_state(
                self.active_subtool_id()
                    .ok_or(SourceGeometryError::UnsupportedDomain)?,
            )
            .map_or_else(|| SourceGeometryNormalState::seed(mesh).map(Arc::new), Ok)?;
        stroke.prepare_alpha_tracks(mesh, &state, contacts, tracks)
    }
    pub fn source_geometry_interaction_center(&self) -> Result<[f32; 3], SourceGeometryError> {
        if let Some(metadata) = self.active_object_metadata() {
            return Ok(metadata.affine.interaction_center);
        }
        let mesh = self
            .active_mesh()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        let state = self
            .source_geometry_state(
                self.active_subtool_id()
                    .ok_or(SourceGeometryError::UnsupportedDomain)?,
            )
            .map_or_else(|| SourceGeometryNormalState::seed(mesh).map(Arc::new), Ok)?;
        if !state.matches_mesh(mesh) {
            return Err(SourceGeometryError::StaleNormalState);
        }
        Ok(state.interaction_center())
    }
    pub fn source_geometry_mirror_plane(
        &self,
        center: [f32; 3],
        relative_offset: f64,
        normal: [f64; 3],
    ) -> Result<crate::SourceMirrorPlane, SourceGeometryError> {
        let mesh = self
            .active_mesh()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        mesh.validate()
            .map_err(|_| SourceGeometryError::InvalidMesh)?;
        let state = self
            .source_geometry_state(
                self.active_subtool_id()
                    .ok_or(SourceGeometryError::UnsupportedDomain)?,
            )
            .map_or_else(|| SourceGeometryNormalState::seed(mesh).map(Arc::new), Ok)?;
        if !state.matches_mesh(mesh) {
            return Err(SourceGeometryError::StaleNormalState);
        }
        crate::SourceMirrorPlane::from_relative_offset(
            center,
            normal,
            relative_offset,
            state.root_loose(),
        )
    }
    fn validate_source_geometry_capture(
        &self,
        stroke: &SourceGeometryStroke,
    ) -> Result<(), SourceGeometryError> {
        let mesh = self
            .active_mesh()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        stroke.validate_domain(mesh)?;
        if !stroke.has_domain() {
            let previous = self
                .stroke_start
                .as_ref()
                .ok_or(SourceGeometryError::NoOpenStroke)?;
            let old = previous
                .mesh
                .as_ref()
                .ok_or(SourceGeometryError::InvalidMesh)?;
            if !previous.matches(self)
                || !super::mesh_channels_equal_bits(old, mesh)
                || old.mask_convention != mesh.mask_convention
                || old.normals.len() != mesh.normals.len()
                || old
                    .normals
                    .iter()
                    .zip(&mesh.normals)
                    .any(|(a, b)| a.to_array().map(f32::to_bits) != b.to_array().map(f32::to_bits))
            {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
        }
        Ok(())
    }
    pub fn end_source_geometry_stroke(
        &mut self,
        stroke: &SourceGeometryStroke,
    ) -> Result<(), SourceGeometryError> {
        self.end_source_geometry_stroke_with_cache_limit(
            stroke,
            crate::MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
    }
    pub fn end_source_geometry_stroke_timed(
        &mut self,
        stroke: &SourceGeometryStroke,
    ) -> Result<SourceGeometryReleaseTiming, SourceGeometryError> {
        self.end_source_geometry_stroke_timed_with_cache_limit(
            stroke,
            crate::MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
    }
    fn end_source_geometry_stroke_with_cache_limit(
        &mut self,
        stroke: &SourceGeometryStroke,
        max_cache_bytes: usize,
    ) -> Result<(), SourceGeometryError> {
        self.end_source_geometry_stroke_timed_with_cache_limit(stroke, max_cache_bytes)
            .map(|_| ())
    }
    fn end_source_geometry_stroke_timed_with_cache_limit(
        &mut self,
        stroke: &SourceGeometryStroke,
        max_cache_bytes: usize,
    ) -> Result<SourceGeometryReleaseTiming, SourceGeometryError> {
        if self.source_geometry_active != Some(stroke.document_token) {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        let started = Instant::now();
        let result = (|| {
            let mut timing = SourceGeometryReleaseTiming::default();
            let validation_started = Instant::now();
            self.validate_source_geometry_capture(stroke)?;
            if stroke.binding
                != Some((
                    self.active_subtool_id()
                        .ok_or(SourceGeometryError::UnsupportedDomain)?,
                    self.current_level(),
                ))
            {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
            self.source_geometry_blocker(stroke.options())?;
            if max_cache_bytes > crate::MAX_SOURCE_GEOMETRY_STATE_BYTES
                || self.source_geometry_retained_bytes() > max_cache_bytes
            {
                return Err(SourceGeometryError::StorageLimit);
            }
            timing.validation_ms = validation_started.elapsed().as_secs_f64() * 1_000.0;
            let history_started = Instant::now();
            let id = stroke.binding.expect("bound Source gesture").0;
            let previous = self
                .stroke_start
                .as_ref()
                .ok_or(SourceGeometryError::NoOpenStroke)?;
            let history = match stroke.history_selected.as_ref() {
                Some(selected) if selected.iter().any(|yes| *yes) && !previous.matches(self) => {
                    Some(super::source_history::SourceHistoryEvent::geometry(
                        id,
                        selected.clone(),
                    )?)
                }
                _ => None,
            };
            let history_bytes = history
                .as_ref()
                .map_or(0, super::source_history::SourceHistoryEvent::bytes);
            let prior_history_bytes = previous
                .source_history
                .as_ref()
                .map_or(0, super::source_history::SourceHistoryEvent::bytes);
            let non_cache = previous
                .approximate_bytes()
                .saturating_sub(retained_bytes(
                    &previous.source_geometry_states,
                    previous.source_geometry_states.capacity(),
                ))
                .saturating_sub(prior_history_bytes);
            if non_cache
                .checked_add(self.source_geometry_retained_bytes())
                .and_then(|n| n.checked_add(history_bytes))
                .is_none_or(|n| n > HISTORY_BUDGET_BYTES)
            {
                return Err(SourceGeometryError::StorageLimit);
            }
            timing.history_ms = history_started.elapsed().as_secs_f64() * 1_000.0;
            let balance_started = Instant::now();
            if let Some(state) = self.source_geometry_state(id) {
                let mesh = self
                    .active_mesh()
                    .ok_or(SourceGeometryError::UnsupportedDomain)?;
                if let Some(next) = stroke.balanced_state_after_document_validation(mesh, &state)? {
                    let other = self
                        .source_geometry_retained_bytes()
                        .saturating_sub(state.approximate_bytes());
                    let previous = self
                        .stroke_start
                        .as_ref()
                        .ok_or(SourceGeometryError::NoOpenStroke)?;
                    let non_cache = previous
                        .approximate_bytes()
                        .saturating_sub(retained_bytes(
                            &previous.source_geometry_states,
                            previous.source_geometry_states.capacity(),
                        ))
                        .saturating_sub(prior_history_bytes)
                        .checked_add(history_bytes)
                        .ok_or(SourceGeometryError::StorageLimit)?;
                    let cache_bytes = other
                        .checked_add(next.approximate_bytes())
                        .ok_or(SourceGeometryError::StorageLimit)?;
                    if cache_bytes > max_cache_bytes
                        || non_cache
                            .checked_add(cache_bytes)
                            .is_none_or(|n| n > HISTORY_BUDGET_BYTES)
                    {
                        return Err(SourceGeometryError::StorageLimit);
                    }
                    // Publish an exact derivative field; a geometry/project no-op
                    // still preserves the real redo because ordinary history
                    // comparison excludes transient cache identity.
                    *self
                        .source_geometry_states
                        .iter_mut()
                        .find(|(key, _)| *key == id)
                        .expect("existing retained Source state") = (id, Arc::new(next));
                }
            } else {
                stroke.validate_domain(
                    self.active_mesh()
                        .ok_or(SourceGeometryError::UnsupportedDomain)?,
                )?;
            }
            timing.balance_ms = balance_started.elapsed().as_secs_f64() * 1_000.0;
            let finalize_started = Instant::now();
            self.stroke_start
                .as_mut()
                .ok_or(SourceGeometryError::NoOpenStroke)?
                .source_history = history;
            self.end_stroke();
            if self.take_layer_error().is_some() {
                return Err(SourceGeometryError::InvalidMesh);
            }
            timing.finalize_ms = finalize_started.elapsed().as_secs_f64() * 1_000.0;
            timing.total_ms = started.elapsed().as_secs_f64() * 1_000.0;
            Ok(timing)
        })();
        if result.is_err()
            && self.source_geometry_active == Some(stroke.document_token)
            && self.stroke_start.is_some()
        {
            // The shared session consumes its ownership on finish even on Err;
            // an uncommitted end-only schedule must not leave work/token behind.
            self.cancel_source_geometry_stroke(stroke)?;
        }
        result
    }
    /// Cancel restores the exact full history snapshot, including Source cache
    /// rows outside the selected patch; it never creates an undo or drops redo.
    pub fn cancel_source_geometry_stroke(
        &mut self,
        stroke: &SourceGeometryStroke,
    ) -> Result<(), SourceGeometryError> {
        if self.source_geometry_active != Some(stroke.document_token) {
            return Err(SourceGeometryError::NoOpenStroke);
        }
        let previous = self
            .stroke_start
            .take()
            .ok_or(SourceGeometryError::NoOpenStroke)?;
        self.exchange_state(previous);
        self.source_geometry_active = None;
        self.layer_error = None;
        Ok(())
    }
    /// Visible explicit cache reset after a non-Source geometry mutation. Does
    /// not convert U, serialize transient normals or fabricate a warm reload.
    pub fn reset_source_geometry_normal_state(&mut self) -> Result<bool, SourceGeometryError> {
        if self.stroke_start.is_some() {
            return Err(SourceGeometryError::StrokeAlreadyOpen);
        }
        let id = self
            .active_subtool_id()
            .ok_or(SourceGeometryError::UnsupportedDomain)?;
        let old = self.source_geometry_states.len();
        self.source_geometry_states.retain(|(key, _)| *key != id);
        let changed = old != self.source_geometry_states.len();
        if self.source_geometry_states.is_empty() {
            self.source_geometry_states = Vec::new();
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod release_budget_tests {
    use super::*;
    use crate::{MaskConvention, Mesh, SourceClayOrderPolicy};
    #[test]
    fn owned_release_capacity_failure_restores_snapshot_and_genuine_redo() {
        let mut mesh = Mesh::from_obj("v -1 -1 0\nv 1 -1 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.);
        let mut d = Document::new(mesh);
        let before = d.to_json().unwrap();
        let options = SourceGeometryOptions {
            clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
            ..SourceGeometryOptions::inflate()
        };
        let pass = SourceGeometryPass {
            center: [0.; 3],
            eye_direction: [0., 0., -1.],
            local_radius_squared: 4.,
            original_face_index: 0,
        };
        let mut first = d.begin_source_geometry_stroke(options).unwrap();
        assert!(
            d.apply_source_geometry_dab(&mut first, pass, 0.5, true)
                .unwrap()
                .changed_vertices
                > 0
        );
        d.end_source_geometry_stroke(&first).unwrap();
        let edited = d.to_json().unwrap();
        assert!(d.undo());
        let mut rejected = d.begin_source_geometry_stroke(options).unwrap();
        d.apply_source_geometry_dab(&mut rejected, pass, 0.5, true)
            .unwrap();
        assert!(d.source_geometry_retained_bytes() > 0);
        assert_eq!(
            d.end_source_geometry_stroke_with_cache_limit(&rejected, 0)
                .unwrap_err(),
            SourceGeometryError::StorageLimit
        );
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.source_geometry_state(1).is_none());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), edited);
        let next = d.begin_source_geometry_stroke(options).unwrap();
        d.cancel_source_geometry_stroke(&next).unwrap();
    }
}

#[cfg(test)]
mod claim_ownership_tests {
    use super::*;
    use crate::{MaskConvention, Mesh};
    fn fresh() -> Document {
        let mut mesh =
            Mesh::from_obj("v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nf 1 2 3\nf 3 5 4\n")
                .unwrap();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.);
        mesh.colors.fill([1.; 3]);
        Document::new(mesh)
    }
    fn pass() -> SourceGeometryPass {
        SourceGeometryPass {
            center: [0.; 3],
            eye_direction: [0., 0., -1.],
            local_radius_squared: 4.,
            original_face_index: 0,
        }
    }
    fn warm() -> Document {
        let mut d = fresh();
        let options = SourceGeometryOptions {
            intensity: 0.625,
            intensity_factor: 0.,
            ..SourceGeometryOptions::inflate()
        };
        let mut g = d.begin_source_geometry_stroke(options).unwrap();
        assert_eq!(
            d.apply_source_geometry_dab(&mut g, pass(), 0.5, true)
                .unwrap()
                .changed_vertices,
            1
        );
        d.end_source_geometry_stroke(&g).unwrap();
        d
    }
    fn share_closed_cache(d: &Document) -> Document {
        assert!(d.stroke_start.is_none());
        let mut copy = Document::new(d.require_active_mesh().unwrap().snapshot());
        // This is an independent closed Document, not an invented public Clone.
        copy.source_geometry_states = d.source_geometry_states.clone();
        copy.source_geometry_token = d.source_geometry_token;
        copy
    }
    #[test]
    fn release_reuses_one_full_retained_match_and_stale_edit_still_cancels() {
        let mut d = warm();
        let options = SourceGeometryOptions {
            intensity: 0.625,
            ..SourceGeometryOptions::inflate()
        };
        let before = d.to_json().unwrap();
        let mut stroke = d.begin_source_geometry_stroke(options).unwrap();
        assert!(
            d.apply_source_geometry_dab(&mut stroke, pass(), 0.5, true)
                .unwrap()
                .changed_vertices
                > 0
        );
        crate::source_geometry::validation_probe::reset();
        d.end_source_geometry_stroke(&stroke).unwrap();
        assert_eq!(crate::source_geometry::validation_probe::counts()[1], 1);
        let after = d.to_json().unwrap();
        assert_ne!(after, before);
        assert!(d.undo());
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), after);

        let stale = d.begin_source_geometry_stroke(options).unwrap();
        d.mesh.as_mut().unwrap().positions[0].x += 0.125;
        assert_eq!(
            d.end_source_geometry_stroke(&stale).unwrap_err(),
            SourceGeometryError::StrokeBindingChanged
        );
        assert_eq!(d.to_json().unwrap(), after);
    }
    #[test]
    fn isolated_nonce_is_unique_and_has_distinct_namespace() {
        let counter = AtomicU64::new(0);
        let a = allocate_source_claim(&counter).unwrap();
        let b = allocate_source_claim(&counter).unwrap();
        assert_eq!((a, b), (1, 2));
        assert_eq!(a & WARM_PAINT_TAG, 0);
        assert_ne!(a, b | WARM_PAINT_TAG);
        assert_eq!((b | WARM_PAINT_TAG) & !WARM_PAINT_TAG, b);
    }
    #[test]
    fn isolated_nonce_exhaustion_never_wraps_or_reuses() {
        let counter = AtomicU64::new(WARM_PAINT_TAG - 2);
        assert_eq!(allocate_source_claim(&counter).unwrap(), WARM_PAINT_TAG - 1);
        for _ in 0..3 {
            assert_eq!(
                allocate_source_claim(&counter).unwrap_err(),
                SourceGeometryError::StorageLimit
            );
        }
        assert_eq!(counter.load(Ordering::Relaxed), WARM_PAINT_TAG - 1);
    }
    #[test]
    fn fresh_equal_documents_foreign_geometry_apply_end_cancel_are_atomic() {
        let mut a = fresh();
        let mut b = fresh();
        assert_eq!(a.source_geometry_token, b.source_geometry_token);
        let mut ga = a
            .begin_source_geometry_stroke(SourceGeometryOptions::inflate())
            .unwrap();
        let mut gb = b
            .begin_source_geometry_stroke(SourceGeometryOptions::inflate())
            .unwrap();
        assert_ne!(ga.document_token, gb.document_token);
        let before = b.to_json().unwrap();
        assert_eq!(
            b.apply_source_geometry_dab(&mut ga, pass(), 0.5, true)
                .unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(
            b.end_source_geometry_stroke(&ga).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(
            b.cancel_source_geometry_stroke(&ga).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(b.to_json().unwrap(), before);
        assert_eq!(b.source_geometry_active, Some(gb.document_token));
        assert!(
            b.apply_source_geometry_dab(&mut gb, pass(), 0.5, true)
                .unwrap()
                .changed_vertices
                > 0
        );
        b.cancel_source_geometry_stroke(&gb).unwrap();
        a.cancel_source_geometry_stroke(&ga).unwrap();
    }
    #[test]
    fn closed_shared_cache_equal_counter_geometry_claims_are_foreign() {
        let mut a = warm();
        let mut b = share_closed_cache(&a);
        let state = a.source_geometry_state(1).unwrap();
        assert!(Arc::ptr_eq(&state, &b.source_geometry_state(1).unwrap()));
        assert_eq!(a.source_geometry_token, b.source_geometry_token);
        let mut ga = a
            .begin_source_geometry_stroke(SourceGeometryOptions::inflate())
            .unwrap();
        let gb = b
            .begin_source_geometry_stroke(SourceGeometryOptions::inflate())
            .unwrap();
        let before = b.to_json().unwrap();
        assert_ne!(ga.document_token, gb.document_token);
        assert_eq!(
            b.apply_source_geometry_dab(&mut ga, pass(), 0.5, true)
                .unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(
            b.end_source_geometry_stroke(&ga).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(
            b.cancel_source_geometry_stroke(&ga).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(b.to_json().unwrap(), before);
        assert!(Arc::ptr_eq(&state, &b.source_geometry_state(1).unwrap()));
        b.cancel_source_geometry_stroke(&gb).unwrap();
        a.cancel_source_geometry_stroke(&ga).unwrap();
    }
    #[test]
    fn closed_shared_cache_equal_counter_warm_claims_cannot_restore_each_other() {
        let mut a = warm();
        let mut b = share_closed_cache(&a);
        let state = a.source_geometry_state(1).unwrap();
        let oa = a.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        let ob = b.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        assert_ne!(oa.token, ob.token);
        let before = b.to_json().unwrap();
        assert_eq!(
            b.validate_retained_source_paint_stroke(&oa).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(
            b.reject_retained_source_paint_stroke(&oa, "foreign".into())
                .unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(b.to_json().unwrap(), before);
        assert_eq!(b.source_geometry_active, Some(ob.token));
        assert!(b.layer_error.is_none());
        b.validate_retained_source_paint_stroke(&ob).unwrap();
        b.end_stroke();
        a.end_stroke();
    }
    #[test]
    fn legitimate_geometry_and_warm_owners_survive_document_move() {
        let mut d = fresh();
        let mut g = d
            .begin_source_geometry_stroke(SourceGeometryOptions::inflate())
            .unwrap();
        let mut moved = Box::new(d);
        assert!(
            moved
                .apply_source_geometry_dab(&mut g, pass(), 0.5, true)
                .unwrap()
                .changed_vertices
                > 0
        );
        moved.end_source_geometry_stroke(&g).unwrap();
        let state = moved.source_geometry_state(1).unwrap();
        let owner = moved
            .claim_retained_source_paint_stroke(1, 0, &state)
            .unwrap();
        let mut moved_again = *moved;
        moved_again
            .validate_retained_source_paint_stroke(&owner)
            .unwrap();
        moved_again
            .reject_retained_source_paint_stroke(&owner, "own rejection".into())
            .unwrap();
        assert!(moved_again.stroke_start.is_none());
        assert_eq!(
            moved_again.take_layer_error().as_deref(),
            Some("own rejection")
        );
    }
    #[test]
    fn expired_warm_claim_cannot_restore_a_later_claim_or_geometry() {
        let mut d = warm();
        let state = d.source_geometry_state(1).unwrap();
        let old = d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        d.end_stroke();
        let next = d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        assert_ne!(old.token, next.token);
        let before = d.to_json().unwrap();
        assert_eq!(
            d.reject_retained_source_paint_stroke(&old, "expired".into())
                .unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(d.to_json().unwrap(), before);
        d.end_stroke();
        let geometry = d
            .begin_source_geometry_stroke(SourceGeometryOptions::inflate())
            .unwrap();
        assert_eq!(
            d.reject_retained_source_paint_stroke(&next, "foreign namespace".into())
                .unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        d.cancel_source_geometry_stroke(&geometry).unwrap();
    }
    fn add_genuine_redo(d: &mut Document) -> String {
        d.begin_stroke();
        d.name = "genuine future state".into();
        d.end_stroke();
        let future = d.to_json().unwrap();
        assert!(d.undo());
        future
    }
    #[test]
    fn exhausted_geometry_claim_preserves_complete_state_and_real_redo() {
        let mut d = warm();
        let future = add_genuine_redo(&mut d);
        let before = d.to_json().unwrap();
        let state = d.source_geometry_state(1).unwrap();
        let counter = AtomicU64::new(WARM_PAINT_TAG - 1);
        let local = d.source_geometry_token;
        assert_eq!(
            d.begin_source_geometry_stroke_with_counter(SourceGeometryOptions::inflate(), &counter)
                .err()
                .unwrap(),
            SourceGeometryError::StorageLimit
        );
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.stroke_start.is_none());
        assert_eq!(d.source_geometry_token, local);
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
    #[test]
    fn exhausted_warm_direct_claim_preserves_errors_marker_snapshot_and_real_redo() {
        let mut d = warm();
        let future = add_genuine_redo(&mut d);
        let before = d.to_json().unwrap();
        let state = d.source_geometry_state(1).unwrap();
        d.layer_error = Some("existing observer".into());
        d.material_paint_stroke = true;
        let local = d.source_geometry_token;
        let counter = AtomicU64::new(WARM_PAINT_TAG - 1);
        assert_eq!(
            d.claim_retained_source_paint_stroke_with_counter(1, 0, &state, &counter)
                .unwrap_err(),
            SourceGeometryError::StorageLimit
        );
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.stroke_start.is_none());
        assert!(d.source_geometry_active.is_none());
        assert_eq!(d.source_geometry_token, local);
        assert!(d.material_paint_stroke);
        assert_eq!(d.layer_error.as_deref(), Some("existing observer"));
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
    #[test]
    fn invalid_direct_warm_binding_never_creates_snapshot_or_consumes_nonce() {
        let mut d = warm();
        let state = d.source_geometry_state(1).unwrap();
        let before = d.to_json().unwrap();
        let counter = AtomicU64::new(0);
        assert_eq!(
            d.claim_retained_source_paint_stroke_with_counter(1, 1, &state, &counter)
                .unwrap_err(),
            SourceGeometryError::StrokeBindingChanged
        );
        assert_eq!(counter.load(Ordering::Relaxed), 0);
        assert!(d.stroke_start.is_none());
        assert_eq!(d.to_json().unwrap(), before);
        d.require_active_mesh_mut().unwrap().positions[2].z = 0.25;
        assert_eq!(
            d.claim_retained_source_paint_stroke_with_counter(1, 0, &state, &counter)
                .unwrap_err(),
            SourceGeometryError::StaleNormalState
        );
        assert_eq!(counter.load(Ordering::Relaxed), 0);
        assert!(d.stroke_start.is_none());
    }
    #[test]
    fn owned_warm_rejection_restores_full_snapshot_cache_and_prior_redo() {
        let mut d = warm();
        let future = add_genuine_redo(&mut d);
        let before = d.to_json().unwrap();
        let state = d.source_geometry_state(1).unwrap();
        let owner = d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        d.require_active_mesh_mut().unwrap().colors[1] = [0.5, 0.75, 1.];
        d.require_active_mesh_mut().unwrap().positions[2].z = 0.25;
        d.reject_retained_source_paint_stroke(&owner, "stale producer".into())
            .unwrap();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert!(d.stroke_start.is_none());
        assert!(d.source_geometry_active.is_none());
        assert_eq!(d.take_layer_error().as_deref(), Some("stale producer"));
        d.end_stroke();
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }

    #[test]
    fn stale_release_after_material_edit_restores_full_snapshot_and_real_redo() {
        let mut d = warm();
        let future = add_genuine_redo(&mut d);
        let before = d.to_json().unwrap();
        let state = d.source_geometry_state(1).unwrap();
        let owner = d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        d.begin_material_paint_stroke();
        let mut materials = vec![crate::DEFAULT_VERTEX_MATERIAL; 5];
        materials[1] = [0.5, 1.];
        d.require_active_mesh_mut().unwrap().materials = Some(materials);
        d.require_active_mesh_mut().unwrap().positions[2].z = 0.25;
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.require_active_mesh().unwrap().materials.is_none());
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert!(d.take_layer_error().unwrap().contains("release rejected"));
        assert_eq!(
            d.validate_retained_source_paint_stroke(&owner).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
    #[test]
    fn release_cache_lifetime_change_restores_original_arc_without_history() {
        let mut d = warm();
        let future = add_genuine_redo(&mut d);
        let state = d.source_geometry_state(1).unwrap();
        let before = d.to_json().unwrap();
        d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        d.require_active_mesh_mut().unwrap().colors[1] = [0.5, 0.75, 1.];
        d.source_geometry_states.clear();
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(Arc::ptr_eq(&state, &d.source_geometry_state(1).unwrap()));
        assert!(d.take_layer_error().is_some());
        assert!(d.redo());
        assert_eq!(d.to_json().unwrap(), future);
    }
    #[test]
    fn release_binding_change_rolls_back_and_next_legitimate_claim_remains_usable() {
        let mut d = warm();
        let state = d.source_geometry_state(1).unwrap();
        let before = d.to_json().unwrap();
        let owner = d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        d.mesh = None;
        d.end_stroke();
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.take_layer_error().is_some());
        assert_eq!(
            d.validate_retained_source_paint_stroke(&owner).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        let next = d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        d.end_stroke();
        assert!(d.take_layer_error().is_none());
        assert_eq!(
            d.validate_retained_source_paint_stroke(&next).unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(d.to_json().unwrap(), before);
    }
    #[test]
    fn copied_opaque_handle_is_same_logical_owner_then_both_expire() {
        let mut d = warm();
        let state = d.source_geometry_state(1).unwrap();
        let owner = d.claim_retained_source_paint_stroke(1, 0, &state).unwrap();
        let copied = owner.clone();
        assert_eq!(owner.token, copied.token);
        d.validate_retained_source_paint_stroke(&copied).unwrap();
        let mut moved = Box::new(d);
        moved
            .reject_retained_source_paint_stroke(&copied, "copied same claim".into())
            .unwrap();
        assert_eq!(
            moved
                .validate_retained_source_paint_stroke(&owner)
                .unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        assert_eq!(
            moved
                .validate_retained_source_paint_stroke(&copied)
                .unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        let next = moved
            .claim_retained_source_paint_stroke(1, 0, &state)
            .unwrap();
        assert_ne!(next.token, owner.token);
        assert_eq!(
            moved
                .reject_retained_source_paint_stroke(&copied, "expired copy".into())
                .unwrap_err(),
            SourceGeometryError::NoOpenStroke
        );
        moved.validate_retained_source_paint_stroke(&next).unwrap();
        moved.end_stroke();
    }
}

fn controlled_noop(
    contacts: [Option<SourceGeometryPass>; 2],
    tracks: crate::SourceGeometryAlphaTracks,
    controls: &crate::SourceGeometryControls,
    reason: SourceGeometryNoop,
) -> crate::SourceGeometryControlledReport {
    crate::SourceGeometryControlledReport {
        combined: SourceGeometryReport::noop(reason),
        passes: [None, None],
        contacts,
        tracks,
        alpha_dimensions: controls.alpha.as_ref().map(|a| [a.width(), a.height()]),
    }
}

#[cfg(test)]
#[path = "prepared_geometry_dab_tests.rs"]
mod prepared_geometry_dab_tests;
