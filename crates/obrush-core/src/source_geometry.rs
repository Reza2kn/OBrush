//! Opt-in ordinary Source geometry brushes. Legacy and fresh-static normal APIs
//! remain separate. This is a bounded independent implementation, not a claim
//! about paired Source camera, optimizer, octree, input or image equivalence.
use crate::source_ordered_tree::{
    OrderedTree, SourceLeafHandle, SourceOrderedSelection, SourceOrderedTreeSnapshot,
};
use crate::{
    MaskConvention, Mesh, SourceNormalField, SourceNormalPolicy, SourceOriginalFaceContact,
    SourcePickedNormalDisposition, SourcePickedNormalError, SourcePickedNormalRejection,
    SourcePickedNormalUndefined, Vec3, TRIANGLE_CORNER,
};
use serde::Serialize;
use std::fmt;
use std::sync::{Arc, Weak};
use std::time::Duration;
use web_time::Instant;

#[path = "source_smooth.rs"]
mod source_smooth;

#[path = "source_flatten.rs"]
mod source_flatten;

#[path = "source_crease.rs"]
mod source_crease;

pub const MAX_SOURCE_GEOMETRY_SCRATCH_BYTES: usize =
    crate::resource_limits::MAX_GEOMETRY_SCRATCH_BYTES;
pub const MAX_SOURCE_GEOMETRY_STATE_BYTES: usize = crate::resource_limits::MAX_GEOMETRY_STATE_BYTES;
pub const MAX_SOURCE_GEOMETRY_OPERATION_BYTES: usize =
    crate::resource_limits::MAX_GEOMETRY_OPERATION_BYTES;
pub const SOURCE_CLAY_SINGLE_LEAF_FACES: usize = 100;
/// Bounded CPU changed-row envelope consumed by the browser's sparse upload.
/// This is an optimization hint; any missing lineage uses complete row checks.
pub const SOURCE_GEOMETRY_CHUNK_ROWS: usize = 2048;
const SOURCE_GEOMETRY_CHUNK_WORDS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourceGeometryTool {
    Brush { clay: bool, accumulate: bool },
    Inflate,
    Pinch,
    Smooth { tangent: bool },
    Flatten,
    Crease,
}

#[cfg(test)]
#[path = "resident_normal_contract_tests.rs"]
mod resident_normal_contract_tests;
impl Default for SourceGeometryTool {
    fn default() -> Self {
        Self::Brush {
            clay: true,
            accumulate: true,
        }
    }
}
/// Compatibility remains the default until the independent ordered-selection
/// and normal consumer gates are exercised. It is not a Source octree claim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum SourceClayOrderPolicy {
    #[default]
    SingleLeaf,
    OrderedStaticOctree,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SourceGeometryOptions {
    pub tool: SourceGeometryTool,
    pub intensity: f64,
    pub intensity_factor: f64,
    pub negative: bool,
    pub culling: bool,
    pub clay_order: SourceClayOrderPolicy,
    /// Preserve local face quality/orientation/fold limits by default. False
    /// applies the finite Source candidate verbatim, including possible folds.
    pub prevent_folds: bool,
}
impl Default for SourceGeometryOptions {
    fn default() -> Self {
        Self {
            tool: Default::default(),
            intensity: 0.5,
            intensity_factor: 0.,
            negative: false,
            culling: false,
            clay_order: SourceClayOrderPolicy::SingleLeaf,
            prevent_folds: true,
        }
    }
}
impl SourceGeometryOptions {
    pub fn inflate() -> Self {
        Self {
            tool: SourceGeometryTool::Inflate,
            intensity: 0.3,
            ..Self::default()
        }
    }
    pub fn pinch() -> Self {
        Self {
            tool: SourceGeometryTool::Pinch,
            intensity: 0.75,
            clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
            ..Self::default()
        }
    }
    pub fn smooth() -> Self {
        Self {
            tool: SourceGeometryTool::Smooth { tangent: false },
            intensity: 0.75,
            clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
            ..Self::default()
        }
    }
    pub fn flatten() -> Self {
        Self {
            tool: SourceGeometryTool::Flatten,
            intensity: 0.75,
            negative: true,
            clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
            ..Self::default()
        }
    }
    pub fn crease() -> Self {
        Self {
            tool: SourceGeometryTool::Crease,
            intensity: 0.75,
            negative: true,
            clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
            ..Self::default()
        }
    }
    pub fn validate(self) -> Result<(), SourceGeometryError> {
        if [self.intensity, self.intensity_factor]
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        if self.tool == SourceGeometryTool::Flatten
            && self.clay_order != SourceClayOrderPolicy::OrderedStaticOctree
        {
            return Err(SourceGeometryError::UnsupportedFlattenOrder);
        }
        if self.tool == SourceGeometryTool::Crease
            && self.clay_order != SourceClayOrderPolicy::OrderedStaticOctree
        {
            return Err(SourceGeometryError::UnsupportedCreaseOrder);
        }
        if matches!(self.tool, SourceGeometryTool::Smooth { .. }) {
            if self.negative {
                return Err(SourceGeometryError::UnsupportedSmoothNegative);
            }
            if self.clay_order != SourceClayOrderPolicy::OrderedStaticOctree {
                return Err(SourceGeometryError::UnsupportedSmoothOrder);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct SourceGeometryPass {
    pub center: [f64; 3],
    pub eye_direction: [f64; 3],
    pub local_radius_squared: f64,
    pub original_face_index: usize,
}
impl SourceGeometryPass {
    pub fn validate(self, mesh: &Mesh) -> Result<(), SourceGeometryError> {
        if self
            .center
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
            || self.eye_direction.iter().any(|v| !v.is_finite())
            || self.eye_direction.iter().all(|&v| v == 0.)
            || !dot(self.eye_direction, self.eye_direction).is_finite()
            || !self.local_radius_squared.is_finite()
            || !(0.0..=1_000_000_000_000.).contains(&self.local_radius_squared)
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        if self.original_face_index >= face_count(mesh) {
            return Err(SourceGeometryError::MissingOriginalFace);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum SourceGeometryError {
    InvalidInput,
    InvalidMesh,
    MaskConvention,
    MissingOriginalFace,
    StaleNormalState,
    UnsupportedDomain,
    UnsupportedClayOrder,
    UnsupportedSmoothNegative,
    UnsupportedSmoothOrder,
    UnsupportedFlattenOrder,
    UnsupportedCreaseOrder,
    ClayRootEscape,
    StorageLimit,
    Allocation,
    UndefinedPickedNormal(SourcePickedNormalUndefined),
    RejectedPickedNormal(SourcePickedNormalRejection),
    UndefinedInflateNormal { vertex: usize },
    NonfiniteCandidate,
    UndefinedAlphaLookup,
    StrokeBindingChanged,
    StrokeAlreadyOpen,
    NoOpenStroke,
}
impl fmt::Display for SourceGeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if matches!(self, Self::UnsupportedSmoothNegative) {
            return f.write_str("Smooth has no Negative/invert option in SculptGL mode. Release Alt and clear invert.");
        }
        write!(f, "Source geometry rejected: {self:?}")
    }
}
impl std::error::Error for SourceGeometryError {}

/// A separate retained Source raw cache. A warmed field is NOT a fresh whole
/// original-face mean. Unselected raw rows deliberately keep their prior words.
#[derive(Debug)]
pub struct SourceGeometryNormalState {
    faces: Arc<Vec<[u32; 4]>>,
    // Canonical first-triangle row for each original face. Sharing this with
    // balanced states lets retained ray candidates skip a whole-face scan.
    face_triangle_rows: Arc<Vec<u32>>,
    position_words: Vec<[u32; 3]>,
    face_normals: Vec<Vec3>,
    vertex_normals: Vec<Vec3>,
    stored_original_faces: bool,
    interaction_center: [f32; 3],
    ordered_tree: OrderedTree,
    generation: u64,
    render_predecessor: Option<Weak<SourceGeometryNormalState>>,
    render_selected_chunks: [u64; SOURCE_GEOMETRY_CHUNK_WORDS],
}
impl SourceGeometryNormalState {
    pub fn seed(mesh: &Mesh) -> Result<Self, SourceGeometryError> {
        validate_mesh(mesh)?;
        Self::seed_validated(mesh)
    }
    pub(crate) fn seed_for_history(mesh: &Mesh) -> Result<Self, SourceGeometryError> {
        mesh.validate()
            .map_err(|_| SourceGeometryError::InvalidMesh)?;
        Self::seed_validated(mesh)
    }
    fn seed_validated(mesh: &Mesh) -> Result<Self, SourceGeometryError> {
        let required = normal_bytes(mesh.positions.len(), face_count(mesh))?;
        let seed_storage = SourceNormalField::storage_requirement(mesh)
            .map_err(|_| SourceGeometryError::InvalidMesh)?;
        if required
            .checked_add(seed_storage.peak_bytes)
            .is_none_or(|v| v > MAX_SOURCE_GEOMETRY_OPERATION_BYTES)
        {
            return Err(SourceGeometryError::StorageLimit);
        }
        let raw = SourceNormalField::compute(mesh, SourceNormalPolicy::StaticOriginalFaces)
            .map_err(|_| SourceGeometryError::InvalidMesh)?;
        let crate::source_normals::SourceNormalParts {
            face_rows,
            position_words,
            face_normals,
            vertex_normals,
        } = raw.into_geometry_parts();
        let mut face_triangle_rows = reserved(face_rows.len())?;
        let mut triangle_row = 0usize;
        for face in &face_rows {
            face_triangle_rows
                .push(u32::try_from(triangle_row).map_err(|_| SourceGeometryError::StorageLimit)?);
            triangle_row = triangle_row
                .checked_add(usize::from(face[3] != TRIANGLE_CORNER) + 1)
                .ok_or(SourceGeometryError::StorageLimit)?;
        }
        if triangle_row != mesh.triangles.len() {
            return Err(SourceGeometryError::InvalidMesh);
        }
        let ordered_tree = OrderedTree::build(&mesh.positions, &face_rows)?;
        let state = Self {
            faces: Arc::new(face_rows),
            face_triangle_rows: Arc::new(face_triangle_rows),
            position_words,
            face_normals,
            vertex_normals,
            stored_original_faces: mesh.surface.is_some(),
            interaction_center: std::array::from_fn(|i| {
                ((ordered_tree.root_loose()[i] + ordered_tree.root_loose()[i + 3]) * 0.5) as f32
            }),
            ordered_tree,
            generation: 0,
            render_predecessor: None,
            render_selected_chunks: [0; SOURCE_GEOMETRY_CHUNK_WORDS],
        };
        if state.approximate_bytes() > MAX_SOURCE_GEOMETRY_STATE_BYTES
            || state
                .approximate_bytes()
                .checked_add(seed_storage.peak_bytes)
                .is_none_or(|v| v > MAX_SOURCE_GEOMETRY_OPERATION_BYTES)
        {
            return Err(SourceGeometryError::StorageLimit);
        }
        Ok(state)
    }
    pub fn interaction_center(&self) -> [f32; 3] {
        self.interaction_center
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Returns only chunks selected by a fully checked controlled operation
    /// whose predecessor is the exact still-owned state already rendered.
    pub fn render_selected_chunks_from(
        &self,
        previous: &Arc<Self>,
    ) -> Option<[u64; SOURCE_GEOMETRY_CHUNK_WORDS]> {
        self.render_predecessor
            .as_ref()
            .filter(|owner| owner.as_ptr() == Arc::as_ptr(previous))
            .map(|_| self.render_selected_chunks)
    }
    /// Exact retained topology allocation identity. States derived from one
    /// fixed-topology stroke share both immutable arrays; independently built
    /// or topology-replaced states do not gain this witness from equal counts.
    pub fn shares_topology_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.faces, &other.faces)
            && Arc::ptr_eq(&self.face_triangle_rows, &other.face_triangle_rows)
    }
    pub fn vertex_normals(&self) -> &[Vec3] {
        &self.vertex_normals
    }
    pub fn face_normals(&self) -> &[Vec3] {
        &self.face_normals
    }
    /// Read-only warm producer metadata. Storage includes the retained tree and
    /// exact input witnesses already charged to Document/history; borrowing the
    /// producer allocates no replacement cache or clone of its rows.
    pub fn normal_report(&self) -> crate::SourceNormalReport {
        crate::SourceNormalReport {
            policy: SourceNormalPolicy::StaticSelectedRefresh,
            face_origin: if self.stored_original_faces {
                crate::SourceNormalFaceOrigin::StoredOriginalFaces
            } else {
                crate::SourceNormalFaceOrigin::TriangleFaces
            },
            faces: self.faces.len(),
            quads: self
                .faces
                .iter()
                .filter(|f| f[3] != TRIANGLE_CORNER)
                .count(),
            vertices: self.vertex_normals.len(),
            unused_vertices: self.ordered_tree.unused_vertices(),
            zero_vertex_normals: self
                .vertex_normals
                .iter()
                .filter(|n| **n == Vec3::ZERO)
                .count(),
            zero_face_normals: self
                .face_normals
                .iter()
                .filter(|n| **n == Vec3::ZERO)
                .count(),
            supplied_face_order_preserved: true,
            storage: crate::SourceNormalStorage {
                retained_bytes: self.approximate_bytes(),
                peak_bytes: self.approximate_bytes(),
            },
        }
    }

    pub fn root_loose(&self) -> [f64; 6] {
        self.ordered_tree.root_loose()
    }
    pub fn root_split(&self) -> [f64; 6] {
        self.ordered_tree.root_split()
    }
    /// Read-only ordered producer. This validates every public mutable position
    /// and original/canonical face row before exposing candidates.
    pub fn ordered_selection(
        &self,
        mesh: &Mesh,
        pass: SourceGeometryPass,
    ) -> Result<SourceOrderedSelection, SourceGeometryError> {
        validate_mesh(mesh)?;
        pass.validate(mesh)?;
        if !self.matches_mesh(mesh) {
            return Err(SourceGeometryError::StaleNormalState);
        }
        self.ordered_tree
            .check_selection_budget(mesh.positions.len(), self.approximate_bytes())?;
        self.ordered_tree.select(
            &mesh.positions,
            &self.faces,
            pass.center,
            pass.local_radius_squared,
        )
    }
    pub fn ordered_tree_snapshot(&self) -> Result<SourceOrderedTreeSnapshot, SourceGeometryError> {
        self.ordered_tree.snapshot()
    }
    pub fn ordered_affected_faces(
        &self,
        selected: &[usize],
    ) -> Result<Vec<u32>, SourceGeometryError> {
        self.ordered_tree.affected_faces(selected)
    }
    pub(crate) fn ray_candidate_faces(
        &self,
        origin: glam::DVec3,
        direction: glam::DVec3,
    ) -> Result<Vec<u32>, SourceGeometryError> {
        self.ordered_tree.ray_candidates(origin, direction)
    }
    pub(crate) fn first_triangle_row(&self, face: usize) -> Option<usize> {
        self.face_triangle_rows.get(face).map(|row| *row as usize)
    }
    pub fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            .saturating_add(self.faces.capacity().saturating_mul(16))
            .saturating_add(self.face_triangle_rows.capacity().saturating_mul(4))
            .saturating_add(
                2 * (std::mem::size_of::<Vec<u32>>() + 2 * std::mem::size_of::<usize>()),
            )
            .saturating_add(self.ordered_tree.bytes())
            .saturating_add(self.position_words.capacity().saturating_mul(12))
            .saturating_add(self.face_normals.capacity().saturating_mul(12))
            .saturating_add(self.vertex_normals.capacity().saturating_mul(12))
            .saturating_add(2 * std::mem::size_of::<usize>()) // retained Arc node
            // A weak predecessor can keep its Arc control allocation alive
            // after the previous state payload is dropped. Charge that node
            // conservatively before a controlled dab can publish the hint.
            .saturating_add(std::mem::size_of::<Self>() + 2 * std::mem::size_of::<usize>())
    }
    pub fn matches_mesh(&self, mesh: &Mesh) -> bool {
        #[cfg(test)]
        validation_probe::record(1);
        if !self.matches_history_topology(mesh)
            || mesh
                .positions
                .iter()
                .zip(&self.position_words)
                .any(|(p, w)| p.to_array().map(f32::to_bits) != *w)
        {
            return false;
        }
        true
    }
    pub(crate) fn matches_history_topology(&self, mesh: &Mesh) -> bool {
        if mesh.surface.is_some() != self.stored_original_faces
            || mesh.positions.len() != self.position_words.len()
        {
            return false;
        }
        let mut triangle = 0;
        for (index, &face) in self.faces.iter().enumerate() {
            if face_at(mesh, index) != Some(face)
                || mesh.triangles.get(triangle) != Some(&[face[0], face[1], face[2]])
            {
                return false;
            }
            triangle += 1;
            if face[3] != TRIANGLE_CORNER {
                if mesh.triangles.get(triangle) != Some(&[face[0], face[2], face[3]]) {
                    return false;
                }
                triangle += 1;
            }
        }
        face_count(mesh) == self.faces.len() && triangle == mesh.triangles.len()
    }
    pub fn clay_admitted(&self) -> bool {
        self.faces.len() <= SOURCE_CLAY_SINGLE_LEAF_FACES
    }
    pub fn picked_normal(
        &self,
        mesh: &Mesh,
        pass: SourceGeometryPass,
    ) -> Result<SourcePickedNormalDisposition, SourceGeometryError> {
        if !self.matches_mesh(mesh) {
            return Err(SourceGeometryError::StaleNormalState);
        }
        self.picked_normal_after_validation(mesh, pass)
    }
    fn picked_normal_after_validation(
        &self,
        mesh: &Mesh,
        pass: SourceGeometryPass,
    ) -> Result<SourcePickedNormalDisposition, SourceGeometryError> {
        let contact = SourceOriginalFaceContact {
            face_index: pass.original_face_index,
            local_point: pass.center,
        };
        match crate::source_picked_normal::prepare_source_picked_normal_raw(
            mesh,
            &self.vertex_normals,
            SourceNormalPolicy::StaticSelectedRefresh,
            contact,
        ) {
            Ok(report) if report.direction_defined => {
                Ok(SourcePickedNormalDisposition::Defined { report })
            }
            Ok(report) => Ok(SourcePickedNormalDisposition::Undefined {
                reason: SourcePickedNormalUndefined::ZeroDirection,
                original_face_index: pass.original_face_index,
                local_point64: pass.center,
                raw_report: Some(report),
            }),
            Err(SourcePickedNormalError::Undefined(reason)) => {
                Ok(SourcePickedNormalDisposition::Undefined {
                    reason,
                    original_face_index: pass.original_face_index,
                    local_point64: pass.center,
                    raw_report: None,
                })
            }
            Err(SourcePickedNormalError::Rejected(reason)) => {
                Err(SourceGeometryError::RejectedPickedNormal(reason))
            }
        }
    }
    fn refreshed(
        &self,
        positions: &[Vec3],
        selected: &[usize],
    ) -> Result<Self, SourceGeometryError> {
        self.refreshed_with_position_scope(positions, selected, false)
    }

    // A controlled dab has already checked the old mesh against this state
    // and writes positions only in `selected`. History restoration does not
    // have that guarantee and continues to rebuild every position witness.
    fn refreshed_after_selected_write(
        &self,
        positions: &[Vec3],
        selected: &[usize],
    ) -> Result<Self, SourceGeometryError> {
        self.refreshed_with_position_scope(positions, selected, true)
    }

    fn refreshed_with_position_scope(
        &self,
        positions: &[Vec3],
        selected: &[usize],
        selected_only: bool,
    ) -> Result<Self, SourceGeometryError> {
        // The static descending incident rings preserve the old whole-field
        // scatter order, including duplicate original corners. Refresh only
        // incident face rows; unrelated historical rows stay unchanged.
        let affected = self.ordered_tree.affected_faces(selected)?;
        let mut faces = copied(&self.face_normals)?;
        for &face in &affected {
            let i = face as usize;
            faces[i] = face_normal(positions, self.faces[i])?;
        }
        drop(affected);
        let mut vertices = copied(&self.vertex_normals)?;
        for &v in selected {
            let incident = self.ordered_tree.incident_faces(v)?;
            let mut sum = [0.0; 3];
            for &face in incident {
                let normal = faces[face as usize].to_array().map(f64::from);
                for c in 0..3 {
                    sum[c] += normal[c];
                }
            }
            let factor = if incident.is_empty() {
                0.0
            } else {
                1.0 / incident.len() as f64
            };
            vertices[v] = Vec3::from_array(sum.map(|n| (n * factor) as f32));
            if !vertices[v].is_finite() {
                return Err(SourceGeometryError::NonfiniteCandidate);
            }
        }
        let position_words = if selected_only {
            let mut words = copied(&self.position_words)?;
            for &v in selected {
                words[v] = positions[v].to_array().map(f32::to_bits);
            }
            words
        } else {
            collect_reserved(
                positions.iter().map(|p| p.to_array().map(f32::to_bits)),
                positions.len(),
            )?
        };
        Ok(Self {
            faces: self.faces.clone(),
            face_triangle_rows: self.face_triangle_rows.clone(),
            position_words,
            face_normals: faces,
            vertex_normals: vertices,
            stored_original_faces: self.stored_original_faces,
            interaction_center: self.interaction_center,
            ordered_tree: self
                .ordered_tree
                .refreshed(positions, &self.faces, selected)?,
            generation: self
                .generation
                .checked_add(1)
                .ok_or(SourceGeometryError::StorageLimit)?,
            render_predecessor: None,
            render_selected_chunks: [0; SOURCE_GEOMETRY_CHUNK_WORDS],
        })
    }
    /// History restores positions but refreshes only the captured rows from
    /// the CURRENT field. The selected rows need not cover all changed faces.
    pub(crate) fn refreshed_for_history(
        &self,
        mesh: &Mesh,
        selected: &[bool],
        max_bytes: usize,
    ) -> Result<Self, SourceGeometryError> {
        mesh.validate()
            .map_err(|_| SourceGeometryError::InvalidMesh)?;
        if !self.matches_history_topology(mesh) || selected.len() != mesh.positions.len() {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        let count = selected.iter().filter(|v| **v).count();
        let selection_bytes = count
            .checked_mul(std::mem::size_of::<usize>())
            .ok_or(SourceGeometryError::StorageLimit)?;
        operation_budget_with_scratch(mesh, self.approximate_bytes(), selection_bytes)?;
        let mut ids = reserved(count)?;
        ids.extend(
            selected
                .iter()
                .enumerate()
                .filter_map(|(v, yes)| yes.then_some(v)),
        );
        let next = self.refreshed(&mesh.positions, &ids)?;
        if next.approximate_bytes() > max_bytes {
            return Err(SourceGeometryError::StorageLimit);
        }
        Ok(next)
    }
    fn clay_centers_inside(&self, positions: &[Vec3]) -> bool {
        let split = self.root_split();
        self.faces.iter().all(|&face| {
            let mut lo = [f32::INFINITY; 3];
            let mut hi = [f32::NEG_INFINITY; 3];
            for v in corners(face) {
                for c in 0..3 {
                    let p = positions[v][c];
                    lo[c] = lo[c].min(p);
                    hi[c] = hi[c].max(p);
                }
            }
            (0..3).all(|c| {
                let center = ((f64::from(lo[c]) + f64::from(hi[c])) * 0.5) as f32;
                f64::from(center) > split[c] && f64::from(center) <= split[c + 3]
            })
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourceGeometryNoop {
    NoContact,
    ZeroIntensity,
    EmptySelection,
    AllProtected,
    ZeroPickedDirection,
    NoAreaDirection,
    GuardRejected,
    NoCoordinateWordChange,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SourceFlattenPlane {
    pub center64: [f64; 3],
    pub normal64: [f64; 3],
    pub weight_sum: f64,
    pub front_vertices: usize,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct SourceGeometryReport {
    pub changed_vertices: usize,
    pub derived_cache_changed: bool,
    pub selected_vertices: usize,
    pub front_vertices: usize,
    pub proxy_captured_vertices: usize,
    pub source_normal_rows_refreshed: usize,
    pub normal_generation: Option<u64>,
    pub picked_normal: Option<SourcePickedNormalDisposition>,
    pub displacement_scale: f32,
    pub noop: Option<SourceGeometryNoop>,
    pub clay_single_leaf: bool,
    pub ordered_static_selection: bool,
    pub ordered_root_rebuilds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatten_plane: Option<SourceFlattenPlane>,
    #[serde(skip)]
    pub phase_time: Option<SourceGeometryPhaseTime>,
}

/// Read-only wall times for a useful local Source geometry dab. These are
/// diagnostics, not project data or part of the reference brush response.
#[derive(Clone, Copy, Debug, Default)]
pub struct SourceGeometryPhaseTime {
    pub selection: Duration,
    pub old_copy: Duration,
    pub candidate_and_guard: Duration,
    pub state_advance: Duration,
    pub normal_refresh: Duration,
    pub publish: Duration,
}
impl SourceGeometryReport {
    pub fn brush_result(&self) -> crate::BrushResult {
        crate::BrushResult {
            changed_vertices: self.changed_vertices,
            displacement_scale: self.displacement_scale,
            refined_vertices: 0,
            detail_limited: false,
        }
    }
    pub fn noop(reason: SourceGeometryNoop) -> Self {
        Self {
            noop: Some(reason),
            ..Self::default()
        }
    }
}

/// First-touch proxy belongs to one captured ordinary stroke. State history is
/// retained by Document, not hidden in this gesture or discarded on release.
pub struct SourceGeometryStroke {
    options: SourceGeometryOptions,
    proxy: Vec<Vec3>,
    touched: Vec<bool>,
    pub(crate) binding: Option<(u32, usize)>,
    pub(crate) expected_generation: Option<u64>,
    pub(crate) document_token: u64,
    domain: Option<DomainWords>,
    visited_leaves: Vec<SourceLeafHandle>,
    /// Opt-in captured pre-culling union for normal history. Generic callers
    /// retain their original snapshot history and allocate no selection rows.
    pub(crate) history_selected: Option<Arc<Vec<bool>>>,
}
impl SourceGeometryStroke {
    pub fn new(options: SourceGeometryOptions) -> Result<Self, SourceGeometryError> {
        options.validate()?;
        Ok(Self {
            options,
            proxy: Vec::new(),
            touched: Vec::new(),
            binding: None,
            expected_generation: None,
            document_token: 0,
            domain: None,
            visited_leaves: Vec::new(),
            history_selected: None,
        })
    }
    /// Read-only transient globally unique claim; never project data.
    pub fn claim_id(&self) -> u64 {
        self.document_token
    }
    pub fn options(&self) -> SourceGeometryOptions {
        self.options
    }
    pub fn proxy_positions(&self) -> &[Vec3] {
        &self.proxy
    }
    /// Cold no-contribution optimization may omit a normal cache but must not
    /// omit the original pre-culling selection from a later useful history.
    pub(crate) fn record_history_noop(
        &mut self,
        mesh: &Mesh,
        contacts: [Option<SourceGeometryPass>; 2],
    ) -> Result<(), SourceGeometryError> {
        let Some(prior) = self.history_selected.as_ref() else {
            return Ok(());
        };
        operation_budget_with_scratch(mesh, 0, mesh.positions.len())?;
        let mut selected = if prior.is_empty() {
            filled(mesh.positions.len(), false)?
        } else {
            copied(prior.as_slice())?
        };
        if selected.len() != mesh.positions.len() {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        for pass in contacts.into_iter().flatten() {
            for index in 0..face_count(mesh) {
                let face = face_at(mesh, index).ok_or(SourceGeometryError::InvalidMesh)?;
                for vertex in corners(face) {
                    if squared_distance(mesh.positions[vertex], pass.center)
                        < pass.local_radius_squared
                    {
                        selected[vertex] = true;
                    }
                }
            }
        }
        self.history_selected = Some(Arc::new(selected));
        Ok(())
    }
    pub fn visited_leaf_count(&self) -> usize {
        self.visited_leaves.len()
    }
    pub fn visited_leaf_storage_bytes(&self) -> usize {
        self.visited_leaves
            .capacity()
            .saturating_mul(std::mem::size_of::<SourceLeafHandle>())
    }

    pub(crate) fn has_domain(&self) -> bool {
        self.domain.is_some()
    }
    pub(crate) fn validate_domain(&self, mesh: &Mesh) -> Result<(), SourceGeometryError> {
        if self.domain.as_ref().is_some_and(|d| !d.matches(mesh)) {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        Ok(())
    }
    pub fn apply(
        &mut self,
        mesh: &mut Mesh,
        state: &SourceGeometryNormalState,
        pass: SourceGeometryPass,
        pressure: f64,
        in_contact: bool,
    ) -> Result<(SourceGeometryReport, Option<SourceGeometryNormalState>), SourceGeometryError>
    {
        self.apply_with_state_limit(
            mesh,
            state,
            pass,
            pressure,
            in_contact,
            MAX_SOURCE_GEOMETRY_STATE_BYTES,
        )
    }
    pub(crate) fn apply_with_state_limit(
        &mut self,
        mesh: &mut Mesh,
        state: &SourceGeometryNormalState,
        pass: SourceGeometryPass,
        pressure: f64,
        in_contact: bool,
        max_state_bytes: usize,
    ) -> Result<(SourceGeometryReport, Option<SourceGeometryNormalState>), SourceGeometryError>
    {
        validate_mesh(mesh)?;
        self.options.validate()?;
        pass.validate(mesh)?;
        if !pressure.is_finite() || !(0.0..=1.0).contains(&pressure) {
            return Err(SourceGeometryError::InvalidInput);
        }
        if !state.matches_mesh(mesh)
            || self
                .expected_generation
                .is_some_and(|v| v != state.generation)
        {
            return Err(SourceGeometryError::StaleNormalState);
        }
        if self.domain.as_ref().is_some_and(|d| !d.matches(mesh)) {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        self.apply_validated_with_state_limit(
            mesh,
            state,
            pass,
            pressure,
            in_contact,
            max_state_bytes,
        )
    }
    pub(crate) fn apply_document_seal(
        &mut self,
        input: crate::document::ValidatedSourceDab<'_>,
        max_state_bytes: usize,
    ) -> Result<(SourceGeometryReport, Option<SourceGeometryNormalState>), SourceGeometryError>
    {
        input.with_parts(|mesh, state, pass, pressure, in_contact| {
            self.apply_validated_with_state_limit(
                mesh,
                state,
                pass,
                pressure,
                in_contact,
                max_state_bytes,
            )
        })
    }
    // Private common math; only callers above may enter after standalone full
    // checks or the consumed exclusive Document proof. Candidate/quality/cap
    // preflight and every scalar operation below remain unchanged.
    fn apply_validated_with_state_limit(
        &mut self,
        mesh: &mut Mesh,
        state: &SourceGeometryNormalState,
        pass: SourceGeometryPass,
        pressure: f64,
        in_contact: bool,
        max_state_bytes: usize,
    ) -> Result<(SourceGeometryReport, Option<SourceGeometryNormalState>), SourceGeometryError>
    {
        self.apply_controlled_math(
            mesh,
            state,
            pass,
            pressure,
            in_contact,
            max_state_bytes,
            None,
            None,
            0,
            None,
            None,
        )
    }
    #[allow(clippy::too_many_arguments)] // Captured atomic inputs remain distinct from old API.
    fn apply_controlled_math(
        &mut self,
        mesh: &mut Mesh,
        state: &SourceGeometryNormalState,
        pass: SourceGeometryPass,
        pressure: f64,
        in_contact: bool,
        max_state_bytes: usize,
        prepared: Option<&PreparedControlPass>,
        alpha: Option<&crate::RasterAlpha>,
        extra_peak: usize,
        smooth_stencil: Option<&source_smooth::SmoothStencil>,
        smooth_workspace: Option<&mut source_smooth::SmoothCandidateWorkspace>,
    ) -> Result<(SourceGeometryReport, Option<SourceGeometryNormalState>), SourceGeometryError>
    {
        self.apply_access_math(
            mesh,
            NormalStateAccess::Shared(state),
            pass,
            pressure,
            in_contact,
            max_state_bytes,
            prepared,
            alpha,
            extra_peak,
            smooth_stencil,
            smooth_workspace,
        )
    }
    #[allow(clippy::too_many_arguments)] // Captured atomic inputs remain distinct.
    fn apply_access_math(
        &mut self,
        mesh: &mut Mesh,
        mut state: NormalStateAccess<'_>,
        pass: SourceGeometryPass,
        pressure: f64,
        in_contact: bool,
        max_state_bytes: usize,
        prepared: Option<&PreparedControlPass>,
        alpha: Option<&crate::RasterAlpha>,
        extra_peak: usize,
        smooth_stencil: Option<&source_smooth::SmoothStencil>,
        smooth_workspace: Option<&mut source_smooth::SmoothCandidateWorkspace>,
    ) -> Result<(SourceGeometryReport, Option<SourceGeometryNormalState>), SourceGeometryError>
    {
        let original_generation = state.generation;
        let original_rebuilds = state.ordered_tree.rebuilds();
        if self.history_selected.is_some() && prepared.is_none() {
            return Err(SourceGeometryError::UnsupportedDomain);
        }
        if !in_contact {
            return Ok((
                SourceGeometryReport::noop(SourceGeometryNoop::NoContact),
                None,
            ));
        }
        let intensity =
            self.options.intensity * (1. + self.options.intensity_factor * (pressure * 2. - 1.));
        if pass.local_radius_squared == 0. {
            return Ok((
                SourceGeometryReport::noop(if self.options.tool == SourceGeometryTool::Flatten {
                    SourceGeometryNoop::NoAreaDirection
                } else {
                    SourceGeometryNoop::ZeroIntensity
                }),
                None,
            ));
        }
        let clay = matches!(
            self.options.tool,
            SourceGeometryTool::Brush { clay: true, .. }
        );
        if clay
            && self.options.clay_order == SourceClayOrderPolicy::SingleLeaf
            && !state.clay_admitted()
        {
            return Err(SourceGeometryError::UnsupportedClayOrder);
        }
        operation_budget_with_scratch(
            mesh,
            state
                .approximate_bytes()
                .checked_add(extra_peak)
                .ok_or(SourceGeometryError::StorageLimit)?,
            smooth_stencil
                .map_or(0, source_smooth::SmoothStencil::bytes)
                .checked_add(smooth_workspace.as_ref().map_or(0, |w| w.bytes()))
                .ok_or(SourceGeometryError::StorageLimit)?,
        )?;
        let selection_started = Instant::now();
        let ordered = self.options.clay_order == SourceClayOrderPolicy::OrderedStaticOctree;
        let (selected, visits) = if let Some(p) = prepared {
            (copied(&p.selected)?, copied(&p.visits)?)
        } else if ordered {
            let selection = state.ordered_tree.select(
                &mesh.positions,
                &state.faces,
                pass.center,
                pass.local_radius_squared,
            )?;
            (selection.selected_vertices, selection.visited_leaves)
        } else {
            (selected_vertices(mesh, &state.faces, pass)?, Vec::new())
        };
        // Reserve before any geometry mutation. Deduplicated visits produce the
        // same end-only schedule as the source flag pass, with a bounded list.
        let mut pending_visits = copied(&self.visited_leaves)?;
        pending_visits
            .try_reserve_exact(visits.len())
            .map_err(|_| SourceGeometryError::Allocation)?;
        for h in visits {
            if !pending_visits.contains(&h) {
                pending_visits.push(h);
            }
        }
        if pending_visits
            .capacity()
            .saturating_mul(std::mem::size_of::<SourceLeafHandle>())
            > crate::MAX_SOURCE_ORDERED_TREE_BYTES
        {
            return Err(SourceGeometryError::StorageLimit);
        }
        let result = (|| {
            if selected.is_empty() {
                return Ok((
                    SourceGeometryReport::noop(
                        if self.options.tool == SourceGeometryTool::Flatten {
                            SourceGeometryNoop::NoAreaDirection
                        } else {
                            SourceGeometryNoop::EmptySelection
                        },
                    ),
                    None,
                ));
            }
            let all_protected = selected.iter().all(|&v| mesh.masks[v] == 0.);
            let front = collect_reserved(
                selected.iter().copied().filter(|&v| {
                    let n = state.vertex_normals[v].to_array().map(f64::from);
                    dot(n, pass.eye_direction) <= 0.
                }),
                selected.len(),
            )?;
            let write = if self.options.culling {
                front.as_slice()
            } else {
                selected.as_slice()
            };
            let flatten_plane = if self.options.tool == SourceGeometryTool::Flatten {
                source_flatten::plane(mesh, &state.vertex_normals, &front)?
            } else {
                None
            };
            let preliminary = SourceGeometryReport {
                selected_vertices: selected.len(),
                front_vertices: front.len(),
                clay_single_leaf: clay && state.clay_admitted(),
                ordered_static_selection: ordered,
                ordered_root_rebuilds: state.ordered_tree.rebuilds(),
                flatten_plane,
                ..Default::default()
            };
            if self.options.tool == SourceGeometryTool::Flatten && flatten_plane.is_none() {
                let mut report = preliminary;
                report.noop = Some(SourceGeometryNoop::NoAreaDirection);
                return Ok((report, None));
            }
            if all_protected && clay {
                let mut report = preliminary;
                report.noop = Some(SourceGeometryNoop::NoAreaDirection);
                return Ok((report, None));
            }
            if intensity == 0. || all_protected {
                if clay {
                    let mut sum = [0.; 3];
                    for &v in &front {
                        let u = f64::from(mesh.masks[v]);
                        let n = state.vertex_normals[v].to_array().map(f64::from);
                        for c in 0..3 {
                            sum[c] += n[c] * u;
                        }
                    }
                    if dot(sum, sum) == 0. {
                        let mut report = preliminary;
                        report.noop = Some(SourceGeometryNoop::NoAreaDirection);
                        return Ok((report, None));
                    }
                }
                let reason = if intensity == 0. {
                    SourceGeometryNoop::ZeroIntensity
                } else {
                    SourceGeometryNoop::AllProtected
                };
                // Zero/protected safety ignores undefined displacement arithmetic,
                // but retains the real selected-row derived cache schedule.
                return self.refresh_only(
                    mesh,
                    &state,
                    write,
                    preliminary,
                    reason,
                    max_state_bytes,
                );
            }
            let picked = match prepared {
                Some(p) => p.picked,
                None => state.picked_normal_after_validation(mesh, pass)?,
            };
            if let Some(reason) = alpha
                .filter(|_| matches!(self.options.tool, SourceGeometryTool::Smooth { .. }))
                .and_then(|_| prepared.and_then(|p| p.track.undefined_normal))
            {
                return Err(SourceGeometryError::UndefinedPickedNormal(reason));
            }
            let direction = match (self.options.tool, picked) {
                (
                    SourceGeometryTool::Brush { clay: false, .. } | SourceGeometryTool::Crease,
                    SourcePickedNormalDisposition::Defined { report },
                ) => report.normal64,
                (
                    SourceGeometryTool::Brush { clay: false, .. } | SourceGeometryTool::Crease,
                    SourcePickedNormalDisposition::Undefined {
                        reason: SourcePickedNormalUndefined::ZeroDirection,
                        ..
                    },
                ) => [0.; 3],
                (
                    SourceGeometryTool::Brush { clay: false, .. } | SourceGeometryTool::Crease,
                    SourcePickedNormalDisposition::Undefined { reason, .. },
                ) => return Err(SourceGeometryError::UndefinedPickedNormal(reason)),
                _ => [0.; 3],
            };
            let mut report = SourceGeometryReport {
                selected_vertices: selected.len(),
                front_vertices: front.len(),
                picked_normal: Some(picked),
                clay_single_leaf: clay && state.clay_admitted(),
                ordered_static_selection: ordered,
                ordered_root_rebuilds: state.ordered_tree.rebuilds(),
                flatten_plane,
                ..Default::default()
            };
            if direction == [0.; 3]
                && matches!(
                    self.options.tool,
                    SourceGeometryTool::Brush { clay: false, .. }
                )
            {
                return self.refresh_only(
                    mesh,
                    &state,
                    write,
                    report,
                    SourceGeometryNoop::ZeroPickedDirection,
                    max_state_bytes,
                );
            }
            let radius = pass.local_radius_squared.sqrt();
            let plane = if clay {
                let mut sum = [0.; 3];
                let mut center = [0.; 3];
                let mut total = 0.;
                for &v in &front {
                    let u = f64::from(mesh.masks[v]);
                    let n = state.vertex_normals[v].to_array().map(f64::from);
                    for c in 0..3 {
                        sum[c] += n[c] * u;
                        center[c] += f64::from(mesh.positions[v][c]) * u;
                    }
                    total += u;
                }
                let length = dot(sum, sum).sqrt();
                if length == 0. {
                    report.noop = Some(SourceGeometryNoop::NoAreaDirection);
                    return Ok((report, None));
                }
                if !length.is_finite() || total <= 0. {
                    return Err(SourceGeometryError::NonfiniteCandidate);
                }
                let normal = sum.map(|v| v * (1. / length));
                let off = radius * 0.1 * if self.options.negative { -1. } else { 1. };
                for c in 0..3 {
                    center[c] = center[c] / total + normal[c] * off;
                }
                Some((center, normal))
            } else {
                None
            };
            let use_proxy = match self.options.tool {
                SourceGeometryTool::Brush { accumulate, .. } => !accumulate,
                SourceGeometryTool::Inflate | SourceGeometryTool::Crease => true,
                SourceGeometryTool::Pinch
                | SourceGeometryTool::Smooth { .. }
                | SourceGeometryTool::Flatten => false,
            };
            let proxy_plan = FirstTouchProxyPlan::prepare(self, mesh, &selected, use_proxy)?;
            report.proxy_captured_vertices = proxy_plan.captured_count();
            let selection_time = selection_started.elapsed();
            let copy_started = Instant::now();
            let old = copied(&mesh.positions)?;
            let old_copy_time = copy_started.elapsed();
            let candidate_started = Instant::now();
            // All candidates are computed against the unchanged old field.
            // Retaining only written rows avoids a second whole-mesh position
            // copy for a local dab, while preserving atomic rejection/rollback.
            let mut proposals = reserved(write.len())?;
            // Every entry checked the full expected domain for this exclusive
            // borrow. A cold stroke still reserves its witness before writes.
            // Warm witnesses stay owned and untouched until success, so errors
            // cannot replace expected attributes with newly observed input.
            let cold_domain = if self.domain.is_none() {
                Some(DomainWords::capture(mesh)?)
            } else {
                None
            };
            if let SourceGeometryTool::Smooth { tangent } = self.options.tool {
                let state_bytes = state
                    .approximate_bytes()
                    .checked_add(extra_peak)
                    .ok_or(SourceGeometryError::StorageLimit)?;
                let owned = if smooth_stencil.is_none() {
                    Some(source_smooth::SmoothStencil::build(
                        mesh,
                        &state,
                        &[selected.as_slice()],
                        state_bytes,
                    )?)
                } else {
                    None
                };
                let stencil = smooth_stencil
                    .or(owned.as_ref())
                    .ok_or(SourceGeometryError::InvalidInput)?;
                let mut owned_workspace = if smooth_workspace.is_none() {
                    Some(source_smooth::SmoothCandidateWorkspace::reserve(
                        mesh,
                        stencil,
                        write.len(),
                        state_bytes,
                    )?)
                } else {
                    None
                };
                let workspace = smooth_workspace
                    .or(owned_workspace.as_mut())
                    .ok_or(SourceGeometryError::InvalidInput)?;
                let candidates = stencil.candidates_into(
                    mesh,
                    &state.vertex_normals,
                    write,
                    intensity,
                    tangent,
                    state_bytes,
                    workspace,
                    |_, live| match alpha {
                        Some(alpha) => {
                            let p = prepared.ok_or(SourceGeometryError::InvalidInput)?;
                            p.track.sample(alpha, pass, p.mirrored, live)
                        }
                        None => Ok(1.0),
                    },
                )?;
                for &(v, candidate) in candidates {
                    proposals.push((v, candidate));
                }
            } else {
                let mut amplitude = intensity * radius * 0.1;
                if self.options.negative {
                    amplitude = -amplitude;
                }
                for &v in write {
                    let u = f64::from(mesh.masks[v]);
                    if u == 0. {
                        continue;
                    }
                    if self.options.tool == SourceGeometryTool::Crease {
                        let candidate = source_crease::CreaseDab {
                            pass,
                            normal: direction,
                            intensity,
                            negative: self.options.negative,
                        }
                        .candidate(
                            old[v],
                            proxy_plan.point(self, v, old[v]),
                            u,
                            |point| match alpha {
                                Some(alpha) => {
                                    let p = prepared.ok_or(SourceGeometryError::InvalidInput)?;
                                    p.track.sample(alpha, pass, p.mirrored, point)
                                }
                                None => Ok(1.0),
                            },
                        )?;
                        proposals.push((v, candidate));
                        continue;
                    }
                    if let Some(plane) = flatten_plane {
                        let candidate = source_flatten::candidate(
                            old[v],
                            pass,
                            intensity,
                            self.options.negative,
                            u,
                            plane,
                            |point| match alpha {
                                Some(alpha) => {
                                    let p = prepared.ok_or(SourceGeometryError::InvalidInput)?;
                                    p.track.sample(alpha, pass, p.mirrored, point)
                                }
                                None => Ok(1.0),
                            },
                        )?;
                        proposals.push((v, candidate));
                        continue;
                    }
                    let footprint = proxy_plan.point(self, v, old[v]);
                    let d = squared_distance(footprint, pass.center).sqrt() / radius;
                    if d >= 1. && self.options.tool != SourceGeometryTool::Pinch {
                        continue;
                    }
                    let q = d * d;
                    let mut weight = 3. * q * q - 4. * q * d + 1.;
                    let live = old[v].to_array().map(f64::from);
                    let delta = if let Some((center, normal)) = plane {
                        let distance = dot(std::array::from_fn(|c| live[c] - center[c]), normal);
                        let comp = if self.options.negative { -1. } else { 1. };
                        if distance * comp > 0. {
                            continue;
                        }
                        if let Some(alpha) = alpha {
                            let p = prepared.ok_or(SourceGeometryError::InvalidInput)?;
                            let a = p.track.sample(alpha, pass, p.mirrored, old[v])?;
                            weight *= distance * intensity * u * a;
                        } else {
                            weight *= distance * intensity * u;
                        }
                        normal.map(|n| -n * weight)
                    } else if self.options.tool == SourceGeometryTool::Inflate {
                        let normal = state.vertex_normals[v].to_array().map(f64::from);
                        let length = dot(normal, normal).sqrt();
                        if length == 0. {
                            return Err(SourceGeometryError::UndefinedInflateNormal { vertex: v });
                        }
                        let mut weight = amplitude * weight;
                        weight /= length;
                        if let Some(alpha) = alpha {
                            let p = prepared.ok_or(SourceGeometryError::InvalidInput)?;
                            let a = p.track.sample(alpha, pass, p.mirrored, old[v])?;
                            weight *= u * a;
                        } else {
                            weight *= u;
                        }
                        normal.map(|n| n * weight)
                    } else if self.options.tool == SourceGeometryTool::Pinch {
                        // Source gather is live-position radial motion, without the
                        // Brush radius amplitude or first-touch proxy. Both mirror
                        // ID lists are pre-dab; do not clip their post-primary d.
                        let mut deformation = intensity * 0.05;
                        if self.options.negative {
                            deformation = -deformation;
                        }
                        if let Some(alpha) = alpha {
                            let p = prepared.ok_or(SourceGeometryError::InvalidInput)?;
                            let a = p.track.sample(alpha, pass, p.mirrored, old[v])?;
                            weight *= deformation * u * a;
                        } else {
                            weight *= deformation * u;
                        }
                        std::array::from_fn(|c| (pass.center[c] - live[c]) * weight)
                    } else {
                        if let Some(alpha) = alpha {
                            let p = prepared.ok_or(SourceGeometryError::InvalidInput)?;
                            let a = p.track.sample(alpha, pass, p.mirrored, old[v])?;
                            weight *= u * amplitude * a;
                        } else {
                            weight *= u * amplitude;
                        }
                        direction.map(|n| n * weight)
                    };
                    let mut candidate = old[v];
                    for c in 0..3 {
                        let value = (live[c] + delta[c]) as f32;
                        if !value.is_finite() || value.abs() > 1_000_000. {
                            return Err(SourceGeometryError::NonfiniteCandidate);
                        }
                        // Literal protected/unchanged component words include signed zero.
                        if delta[c] != 0. {
                            candidate[c] = value;
                        }
                    }
                    proposals.push((v, candidate));
                }
            }
            let proposed_changed = proposals.iter().any(|(v, point)| {
                point.to_array().map(f32::to_bits) != old[*v].to_array().map(f32::to_bits)
            });
            // Only proposal rows can differ from `old`. Reserve before the
            // first write so allocation failure still preserves the mesh.
            let mut dirty = reserved(proposals.len())?;
            for &(v, point) in &proposals {
                mesh.positions[v] = point;
            }
            let scale = if self.options.prevent_folds {
                mesh.accept_source_geometry_candidate_sparse(&old, &proposals)
            } else {
                // The producer above already enforced finite coordinate and
                // operation/allocation limits. This explicit captured option
                // selects the full response; it does not change those limits.
                1.0
            };
            for &(v, _) in &proposals {
                if mesh.positions[v].to_array().map(f32::to_bits)
                    != old[v].to_array().map(f32::to_bits)
                {
                    dirty.push(v);
                }
            }
            dirty.sort_unstable();
            dirty.dedup();
            let changed = dirty.len();
            if clay && !ordered && !state.clay_centers_inside(&mesh.positions) {
                mesh.positions.copy_from_slice(&old);
                return Err(SourceGeometryError::ClayRootEscape);
            }
            if changed == 0 && proposed_changed {
                report.noop = Some(SourceGeometryNoop::GuardRejected);
                return Ok((report, None));
            }
            let candidate_time = candidate_started.elapsed();
            let advance_started = Instant::now();
            let advance = match state.advance_controlled(
                &mesh.positions,
                write,
                changed > 0,
                max_state_bytes,
            ) {
                Ok(v) => v,
                Err(e) => {
                    mesh.positions.copy_from_slice(&old);
                    return Err(e);
                }
            };
            let advance_time = advance_started.elapsed();
            let cache_changed = advance.cache_changed;
            let refreshed = advance.next.as_ref().unwrap_or(&state);
            let normals_started = Instant::now();
            if changed > 0 {
                mesh.recompute_normals_near_vertices_cached(&dirty);
            }
            let normal_time = normals_started.elapsed();
            let publish_started = Instant::now();
            report.changed_vertices = changed;
            report.displacement_scale = if changed == 0 { 0. } else { scale };
            report.source_normal_rows_refreshed = write.len();
            if changed == 0 {
                report.noop = Some(SourceGeometryNoop::NoCoordinateWordChange);
            }
            proxy_plan.publish(self, &old);
            // No fallible work remains. Only Legacy normal words can have
            // changed in this geometry operation; other fully checked rows
            // retain their exact allocations and bits.
            if let Some(domain) = cold_domain {
                self.domain = Some(domain);
            }
            let domain = self.domain.as_mut().expect("preflight expected domain");
            for (out, n) in domain.normals.iter_mut().zip(&mesh.normals) {
                *out = n.to_array().map(f32::to_bits);
            }
            report.phase_time = Some(SourceGeometryPhaseTime {
                selection: selection_time,
                old_copy: old_copy_time,
                candidate_and_guard: candidate_time,
                state_advance: advance_time,
                normal_refresh: normal_time,
                publish: publish_started.elapsed(),
            });
            if changed > 0 || cache_changed {
                report.derived_cache_changed = cache_changed;
                report.ordered_root_rebuilds = refreshed.ordered_tree.rebuilds();
                self.expected_generation = Some(refreshed.generation);
                report.normal_generation = Some(refreshed.generation);
                Ok((report, advance.next))
            } else {
                self.expected_generation = Some(original_generation);
                report.normal_generation = Some(original_generation);
                Ok((report, None))
            }
        })();
        if let Ok((_, next)) = &result {
            if next
                .as_ref()
                .map_or(state.ordered_tree.rebuilds(), |n| n.ordered_tree.rebuilds())
                != original_rebuilds
            {
                pending_visits.clear();
            }
            self.visited_leaves = pending_visits;
        }
        result
    }
    /// Derivative-only end schedule; Document owns atomic publication/history.
    /// End-of-stroke continuation after Document checked this same active mesh
    /// and retained state under one exclusive borrow. Public stroke entry
    /// points must still perform their full stale-state admission.
    pub(crate) fn balanced_state_after_document_validation(
        &self,
        mesh: &Mesh,
        state: &SourceGeometryNormalState,
    ) -> Result<Option<SourceGeometryNormalState>, SourceGeometryError> {
        if self
            .expected_generation
            .is_some_and(|g| g != state.generation)
        {
            return Err(SourceGeometryError::StaleNormalState);
        }
        self.validate_domain(mesh)?;
        operation_budget(mesh, state.approximate_bytes())?;
        let Some(tree) = state.ordered_tree.balanced(&self.visited_leaves)? else {
            return Ok(None);
        };
        Ok(Some(SourceGeometryNormalState {
            faces: state.faces.clone(),
            face_triangle_rows: state.face_triangle_rows.clone(),
            position_words: copied(&state.position_words)?,
            face_normals: copied(&state.face_normals)?,
            vertex_normals: copied(&state.vertex_normals)?,
            stored_original_faces: state.stored_original_faces,
            interaction_center: state.interaction_center,
            ordered_tree: tree,
            generation: state
                .generation
                .checked_add(1)
                .ok_or(SourceGeometryError::StorageLimit)?,
            render_predecessor: None,
            render_selected_chunks: [0; SOURCE_GEOMETRY_CHUNK_WORDS],
        }))
    }
    fn refresh_only(
        &mut self,
        mesh: &Mesh,
        state: &SourceGeometryNormalState,
        selected: &[usize],
        mut report: SourceGeometryReport,
        reason: SourceGeometryNoop,
        max_state_bytes: usize,
    ) -> Result<(SourceGeometryReport, Option<SourceGeometryNormalState>), SourceGeometryError>
    {
        let next = state.refreshed(&mesh.positions, selected)?;
        let changed = next
            .vertex_normals
            .iter()
            .zip(&state.vertex_normals)
            .any(|(a, b)| a.to_array().map(f32::to_bits) != b.to_array().map(f32::to_bits))
            || next
                .face_normals
                .iter()
                .zip(&state.face_normals)
                .any(|(a, b)| a.to_array().map(f32::to_bits) != b.to_array().map(f32::to_bits))
            || !next.ordered_tree.same_words(&state.ordered_tree);
        if changed && next.approximate_bytes() > max_state_bytes {
            return Err(SourceGeometryError::StorageLimit);
        }
        report.noop = Some(reason);
        report.source_normal_rows_refreshed = selected.len();
        report.derived_cache_changed = changed;
        report.ordered_root_rebuilds = next.ordered_tree.rebuilds();
        let generation = if changed {
            next.generation
        } else {
            state.generation
        };
        self.expected_generation = Some(generation);
        report.normal_generation = Some(generation);
        Ok((report, changed.then_some(next)))
    }
}

/// Only the controlled loop can lend an unaliased intermediate producer.
/// The public caller's original Arc remains alive throughout that operation.
enum NormalStateAccess<'a> {
    Shared(&'a SourceGeometryNormalState),
    Private(&'a mut SourceGeometryNormalState),
}
impl std::ops::Deref for NormalStateAccess<'_> {
    type Target = SourceGeometryNormalState;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Shared(s) => s,
            Self::Private(s) => s,
        }
    }
}
struct NormalAdvance {
    next: Option<SourceGeometryNormalState>,
    cache_changed: bool,
}
impl NormalStateAccess<'_> {
    #[cfg(test)]
    fn advance(
        &mut self,
        positions: &[Vec3],
        selected: &[usize],
        coordinates_changed: bool,
        max_bytes: usize,
    ) -> Result<NormalAdvance, SourceGeometryError> {
        self.advance_with_scope(positions, selected, coordinates_changed, max_bytes, false)
    }
    fn advance_controlled(
        &mut self,
        positions: &[Vec3],
        selected: &[usize],
        coordinates_changed: bool,
        max_bytes: usize,
    ) -> Result<NormalAdvance, SourceGeometryError> {
        self.advance_with_scope(positions, selected, coordinates_changed, max_bytes, true)
    }
    fn advance_with_scope(
        &mut self,
        positions: &[Vec3],
        selected: &[usize],
        coordinates_changed: bool,
        max_bytes: usize,
        controlled_selection: bool,
    ) -> Result<NormalAdvance, SourceGeometryError> {
        if let Self::Private(state) = self {
            // The copied path reserves each witness/normal vector's exact
            // length. Keep its retained-storage/cap behavior for unusual spare
            // capacities rather than silently changing admission accounting.
            if state.position_words.len() == state.position_words.capacity()
                && state.face_normals.len() == state.face_normals.capacity()
                && state.vertex_normals.len() == state.vertex_normals.capacity()
            {
                return state.advance_private(positions, selected, coordinates_changed, max_bytes);
            }
        }
        let next = if controlled_selection {
            self.refreshed_after_selected_write(positions, selected)?
        } else {
            self.refreshed(positions, selected)?
        };
        let normals_changed = if controlled_selection {
            // A checked controlled dab changes only selected vertex-normal
            // rows and their incident original-face rows. The refresh above
            // retains every other word, so scanning the million-row fields
            // again cannot discover another change.
            selected.iter().any(|&v| {
                next.vertex_normals[v].to_array().map(f32::to_bits)
                    != self.vertex_normals[v].to_array().map(f32::to_bits)
            }) || self
                .ordered_tree
                .affected_faces(selected)?
                .iter()
                .any(|&face| {
                    let index = face as usize;
                    next.face_normals[index].to_array().map(f32::to_bits)
                        != self.face_normals[index].to_array().map(f32::to_bits)
                })
        } else {
            next.vertex_normals
                .iter()
                .zip(&self.vertex_normals)
                .any(|(a, b)| a.to_array().map(f32::to_bits) != b.to_array().map(f32::to_bits))
                || next
                    .face_normals
                    .iter()
                    .zip(&self.face_normals)
                    .any(|(a, b)| a.to_array().map(f32::to_bits) != b.to_array().map(f32::to_bits))
        };
        let cache_changed = normals_changed || !next.ordered_tree.same_words(&self.ordered_tree);
        if (coordinates_changed || cache_changed) && next.approximate_bytes() > max_bytes {
            return Err(SourceGeometryError::StorageLimit);
        }
        Ok(NormalAdvance {
            next: Some(next),
            cache_changed,
        })
    }
}
impl SourceGeometryNormalState {
    fn advance_private(
        &mut self,
        positions: &[Vec3],
        selected: &[usize],
        coordinates_changed: bool,
        max_bytes: usize,
    ) -> Result<NormalAdvance, SourceGeometryError> {
        let affected = self.ordered_tree.affected_faces(selected)?;
        let mut normal_changed = false;
        for &face in &affected {
            let index = face as usize;
            let candidate = face_normal(positions, self.faces[index])?;
            normal_changed |= candidate.to_array().map(f32::to_bits)
                != self.face_normals[index].to_array().map(f32::to_bits);
        }
        // Every incident face of a selected vertex is refreshed. Recompute its
        // f32 face value during this read-only preflight, then sum in the same
        // descending f64 order, including duplicate corner contributions.
        for &v in selected {
            let incident = self.ordered_tree.incident_faces(v)?;
            let mut sum = [0.0; 3];
            for &face in incident {
                let normal = face_normal(positions, self.faces[face as usize])?
                    .to_array()
                    .map(f64::from);
                for c in 0..3 {
                    sum[c] += normal[c];
                }
            }
            let factor = if incident.is_empty() {
                0.0
            } else {
                1.0 / incident.len() as f64
            };
            let candidate = Vec3::from_array(sum.map(|n| (n * factor) as f32));
            if !candidate.is_finite() {
                return Err(SourceGeometryError::NonfiniteCandidate);
            }
            normal_changed |= candidate.to_array().map(f32::to_bits)
                != self.vertex_normals[v].to_array().map(f32::to_bits);
        }
        let tree = self
            .ordered_tree
            .refreshed(positions, &self.faces, selected)?;
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(SourceGeometryError::StorageLimit)?;
        let cache_changed = normal_changed || !tree.same_words(&self.ordered_tree);
        let bytes = self
            .approximate_bytes()
            .checked_sub(self.ordered_tree.bytes())
            .and_then(|n| n.checked_add(tree.bytes()))
            .ok_or(SourceGeometryError::StorageLimit)?;
        if (coordinates_changed || cache_changed) && bytes > max_bytes {
            return Err(SourceGeometryError::StorageLimit);
        }
        if coordinates_changed || cache_changed {
            // All fallible work has finished. The immutable input coordinates,
            // faces and incident rings are the exact preflight inputs. Repeating
            // these deterministic operations cannot introduce a new rejection.
            for &face in &affected {
                let index = face as usize;
                self.face_normals[index] =
                    face_normal(positions, self.faces[index]).expect("preflight face normal");
            }
            for &v in selected {
                let incident = self
                    .ordered_tree
                    .incident_faces(v)
                    .expect("preflight incident row");
                let mut sum = [0.0; 3];
                for &face in incident {
                    let normal = self.face_normals[face as usize].to_array().map(f64::from);
                    for c in 0..3 {
                        sum[c] += normal[c];
                    }
                }
                let factor = if incident.is_empty() {
                    0.0
                } else {
                    1.0 / incident.len() as f64
                };
                self.vertex_normals[v] = Vec3::from_array(sum.map(|n| (n * factor) as f32));
            }
            for (word, position) in self.position_words.iter_mut().zip(positions) {
                *word = position.to_array().map(f32::to_bits);
            }
            self.ordered_tree = tree;
            self.generation = generation;
        }
        Ok(NormalAdvance {
            next: None,
            cache_changed,
        })
    }
}

/// A dab may read a first-touch position before it can commit that capture.
/// Cold storage follows the original allocation/capture path. Warm storage
/// records only new IDs; existing proxy/touched words remain owned and unchanged
/// until all fallible mesh/cache/budget work has succeeded.
enum FirstTouchProxyPlan {
    Disabled,
    Cold {
        positions: Vec<Vec3>,
        touched: Vec<bool>,
        captured: usize,
    },
    Warm {
        captures: Vec<usize>,
    },
}
impl FirstTouchProxyPlan {
    fn prepare(
        stroke: &SourceGeometryStroke,
        mesh: &Mesh,
        selected: &[usize],
        enabled: bool,
    ) -> Result<Self, SourceGeometryError> {
        if !enabled {
            return Ok(Self::Disabled);
        }
        if stroke.proxy.is_empty() || stroke.touched.is_empty() {
            let mut positions = if stroke.proxy.is_empty() {
                copied(&mesh.positions)?
            } else {
                copied(&stroke.proxy)?
            };
            let mut touched = if stroke.touched.is_empty() {
                filled(mesh.positions.len(), false)?
            } else {
                copied(&stroke.touched)?
            };
            if positions.len() != mesh.positions.len() || touched.len() != mesh.positions.len() {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
            let mut captured = 0;
            for &v in selected {
                if !touched[v] {
                    positions[v] = mesh.positions[v];
                    touched[v] = true;
                    captured += 1;
                }
            }
            return Ok(Self::Cold {
                positions,
                touched,
                captured,
            });
        }
        if stroke.proxy.len() != mesh.positions.len()
            || stroke.touched.len() != mesh.positions.len()
        {
            return Err(SourceGeometryError::StrokeBindingChanged);
        }
        let count = selected.iter().filter(|&&v| !stroke.touched[v]).count();
        let mut captures = reserved(count)?;
        captures.extend(selected.iter().copied().filter(|&v| !stroke.touched[v]));
        // Selection producers already deduplicate. Retain the old first-touch
        // contract even for repeated private prepared IDs without a full V copy.
        captures.sort_unstable();
        captures.dedup();
        Ok(Self::Warm { captures })
    }
    fn captured_count(&self) -> usize {
        match self {
            Self::Disabled => 0,
            Self::Cold { captured, .. } => *captured,
            Self::Warm { captures } => captures.len(),
        }
    }
    // Callers use only the pre-culling selected IDs (or their write subset).
    fn point(&self, stroke: &SourceGeometryStroke, v: usize, live: Vec3) -> Vec3 {
        match self {
            Self::Disabled => live,
            Self::Cold { positions, .. } => positions[v],
            Self::Warm { .. } => {
                if stroke.touched[v] {
                    stroke.proxy[v]
                } else {
                    live
                }
            }
        }
    }
    fn publish(self, stroke: &mut SourceGeometryStroke, old: &[Vec3]) {
        match self {
            Self::Disabled => {
                stroke.proxy = Vec::new();
                stroke.touched = Vec::new();
            }
            Self::Cold {
                positions, touched, ..
            } => {
                stroke.proxy = positions;
                stroke.touched = touched;
            }
            Self::Warm { captures } => {
                for v in captures {
                    stroke.proxy[v] = old[v];
                    stroke.touched[v] = true;
                }
            }
        }
    }
}

/// Exact nongeometry domain witness, including the separate Legacy normal
/// cache. Public in-stroke attribute/UV changes cannot be swallowed as Source
/// input. Expected Legacy normal words advance only after this accepted dab.
type UvDomainWords = (Arc<Vec<[u32; 2]>>, Arc<Vec<[u32; 4]>>);

struct DomainWords {
    normals: Vec<[u32; 3]>,
    masks: Arc<Vec<u32>>,
    colors: Arc<Vec<[u32; 3]>>,
    materials: Option<Arc<Vec<[u32; 2]>>>,
    uv: Option<UvDomainWords>,
}
impl DomainWords {
    fn capture(m: &Mesh) -> Result<Self, SourceGeometryError> {
        Ok(Self {
            normals: collect_reserved(
                m.normals.iter().map(|n| n.to_array().map(f32::to_bits)),
                m.normals.len(),
            )?,
            masks: Arc::new(collect_reserved(
                m.masks.iter().map(|v| v.to_bits()),
                m.masks.len(),
            )?),
            colors: Arc::new(collect_reserved(
                m.colors.iter().map(|v| v.map(f32::to_bits)),
                m.colors.len(),
            )?),
            materials: m
                .materials
                .as_ref()
                .map(|a| collect_reserved(a.iter().map(|v| v.map(f32::to_bits)), a.len()))
                .transpose()?
                .map(Arc::new),
            uv: m
                .surface
                .as_ref()
                .and_then(|s| s.uv.as_ref())
                .map(|u| -> Result<_, SourceGeometryError> {
                    Ok((
                        Arc::new(collect_reserved(
                            u.coordinates.iter().map(|v| v.map(f32::to_bits)),
                            u.coordinates.len(),
                        )?),
                        Arc::new(copied(&u.face_indices)?),
                    ))
                })
                .transpose()?,
        })
    }
    fn staged_copy(&self) -> Result<Self, SourceGeometryError> {
        Ok(Self {
            normals: copied(&self.normals)?,
            masks: self.masks.clone(),
            colors: self.colors.clone(),
            materials: self.materials.clone(),
            uv: self.uv.clone(),
        })
    }
    fn matches(&self, m: &Mesh) -> bool {
        #[cfg(test)]
        validation_probe::record(2);
        self.normals.len() == m.normals.len()
            && self
                .normals
                .iter()
                .zip(&m.normals)
                .all(|(a, b)| *a == b.to_array().map(f32::to_bits))
            && self.masks.len() == m.masks.len()
            && self
                .masks
                .iter()
                .zip(&m.masks)
                .all(|(a, b)| *a == b.to_bits())
            && self.colors.len() == m.colors.len()
            && self
                .colors
                .iter()
                .zip(&m.colors)
                .all(|(a, b)| *a == b.map(f32::to_bits))
            && match (&self.materials, &m.materials) {
                (None, None) => true,
                (Some(a), Some(b)) => {
                    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| *a == b.map(f32::to_bits))
                }
                _ => false,
            }
            && match (&self.uv, m.surface.as_ref().and_then(|s| s.uv.as_ref())) {
                (None, None) => true,
                (Some((a, f)), Some(u)) => {
                    a.len() == u.coordinates.len()
                        && a.iter()
                            .zip(&u.coordinates)
                            .all(|(a, b)| *a == b.map(f32::to_bits))
                        && f.as_ref() == &u.face_indices
                }
                _ => false,
            }
    }
}

pub(crate) fn validate_mesh(mesh: &Mesh) -> Result<(), SourceGeometryError> {
    #[cfg(test)]
    validation_probe::record(0);
    if mesh.mask_convention != MaskConvention::UnprotectedF32V1 {
        return Err(SourceGeometryError::MaskConvention);
    }
    mesh.validate()
        .map_err(|_| SourceGeometryError::InvalidMesh)
}
pub(crate) fn possible_contribution(mesh: &Mesh, pass: SourceGeometryPass) -> bool {
    (0..face_count(mesh))
        .filter_map(|i| face_at(mesh, i))
        .any(|f| {
            corners(f).any(|v| {
                mesh.masks[v] > 0.
                    && squared_distance(mesh.positions[v], pass.center) < pass.local_radius_squared
            })
        })
}
fn selected_vertices(
    mesh: &Mesh,
    faces: &[[u32; 4]],
    pass: SourceGeometryPass,
) -> Result<Vec<usize>, SourceGeometryError> {
    let mut seen = filled(mesh.positions.len(), false)?;
    let mut out = reserved(mesh.positions.len())?;
    for &face in faces {
        for v in corners(face) {
            if seen[v] {
                continue;
            }
            seen[v] = true;
            if squared_distance(mesh.positions[v], pass.center) < pass.local_radius_squared {
                out.push(v);
            }
        }
    }
    Ok(out)
}
fn normal_bytes(v: usize, f: usize) -> Result<usize, SourceGeometryError> {
    let n = v
        .checked_mul(24)
        .and_then(|n| f.checked_mul(32).and_then(|a| n.checked_add(a)))
        .and_then(|n| n.checked_add(std::mem::size_of::<SourceGeometryNormalState>() + 16))
        .ok_or(SourceGeometryError::StorageLimit)?;
    if n > MAX_SOURCE_GEOMETRY_STATE_BYTES {
        return Err(SourceGeometryError::StorageLimit);
    }
    Ok(n)
}
fn operation_budget(mesh: &Mesh, state_bytes: usize) -> Result<(), SourceGeometryError> {
    operation_budget_with_scratch(mesh, state_bytes, 0)
}
fn operation_budget_with_scratch(
    mesh: &Mesh,
    state_bytes: usize,
    extra_scratch: usize,
) -> Result<(), SourceGeometryError> {
    let scratch = mesh
        .positions
        .len()
        .checked_mul(160)
        .and_then(|n| {
            mesh.triangles
                .len()
                .checked_mul(192)
                .and_then(|a| n.checked_add(a))
        })
        .and_then(|n| {
            mesh.surface
                .as_ref()
                .and_then(|s| s.uv.as_ref())
                .map_or(Some(n), |u| {
                    u.coordinates
                        .len()
                        .checked_mul(16)
                        .and_then(|a| {
                            u.face_indices
                                .len()
                                .checked_mul(32)
                                .and_then(|b| a.checked_add(b))
                        })
                        .and_then(|uv| n.checked_add(uv))
                })
        })
        .ok_or(SourceGeometryError::StorageLimit)?;
    let scratch = scratch
        .checked_add(extra_scratch)
        .ok_or(SourceGeometryError::StorageLimit)?;
    if scratch > MAX_SOURCE_GEOMETRY_SCRATCH_BYTES
        || state_bytes
            .checked_mul(2)
            .and_then(|n| n.checked_add(scratch))
            .is_none_or(|n| n > MAX_SOURCE_GEOMETRY_OPERATION_BYTES)
    {
        return Err(SourceGeometryError::StorageLimit);
    }
    Ok(())
}
fn face_count(mesh: &Mesh) -> usize {
    mesh.surface
        .as_ref()
        .map_or(mesh.triangles.len(), |s| s.faces.len())
}
fn face_at(mesh: &Mesh, index: usize) -> Option<[u32; 4]> {
    mesh.surface.as_ref().map_or_else(
        || {
            mesh.triangles
                .get(index)
                .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
        },
        |s| s.faces.get(index).copied(),
    )
}
fn corners(face: [u32; 4]) -> impl Iterator<Item = usize> {
    face.into_iter()
        .filter(|&v| v != TRIANGLE_CORNER)
        .map(|v| v as usize)
}
fn squared_distance(p: Vec3, c: [f64; 3]) -> f64 {
    let d = std::array::from_fn(|i| f64::from(p[i]) - c[i]);
    dot(d, d)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn face_normal(p: &[Vec3], f: [u32; 4]) -> Result<Vec3, SourceGeometryError> {
    let point = |v: u32| p[v as usize].to_array().map(f64::from);
    let a = point(f[0]);
    let b = point(f[1]);
    let c = point(f[2]);
    let ab = std::array::from_fn(|i| b[i] - a[i]);
    let ac = std::array::from_fn(|i| c[i] - a[i]);
    let mut n = cross(ab, ac);
    if f[3] != TRIANGLE_CORNER {
        let d = point(f[3]);
        let cd = std::array::from_fn(|i| c[i] - d[i]);
        let extra = cross(cd, ac);
        for i in 0..3 {
            n[i] += extra[i];
        }
    }
    let n = Vec3::from_array(n.map(|v| v as f32));
    if !n.is_finite() {
        return Err(SourceGeometryError::NonfiniteCandidate);
    }
    Ok(n)
}
fn reserved<T>(n: usize) -> Result<Vec<T>, SourceGeometryError> {
    let mut out = Vec::new();
    out.try_reserve_exact(n)
        .map_err(|_| SourceGeometryError::Allocation)?;
    Ok(out)
}
fn copied<T: Copy>(v: &[T]) -> Result<Vec<T>, SourceGeometryError> {
    let mut out = reserved(v.len())?;
    out.extend_from_slice(v);
    Ok(out)
}
fn filled<T: Clone>(n: usize, value: T) -> Result<Vec<T>, SourceGeometryError> {
    let mut out = reserved(n)?;
    out.resize(n, value);
    Ok(out)
}
fn collect_reserved<T>(
    iter: impl Iterator<Item = T>,
    n: usize,
) -> Result<Vec<T>, SourceGeometryError> {
    let mut out = reserved(n)?;
    out.extend(iter);
    Ok(out)
}

struct PreparedControlPass {
    pass: SourceGeometryPass,
    selected: Vec<usize>,
    visits: Vec<SourceLeafHandle>,
    picked: SourcePickedNormalDisposition,
    track: crate::SourceGeometryAlphaTrack,
    mirrored: bool,
}
impl SourceGeometryStroke {
    fn staged_copy(&self) -> Result<Self, SourceGeometryError> {
        Ok(Self {
            options: self.options,
            proxy: copied(&self.proxy)?,
            touched: copied(&self.touched)?,
            binding: self.binding,
            expected_generation: self.expected_generation,
            document_token: self.document_token,
            domain: self
                .domain
                .as_ref()
                .map(DomainWords::staged_copy)
                .transpose()?,
            visited_leaves: copied(&self.visited_leaves)?,
            history_selected: self.history_selected.clone(),
        })
    }
    fn staged_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            .saturating_add(self.proxy.capacity().saturating_mul(12))
            .saturating_add(self.touched.capacity())
            .saturating_add(self.visited_leaf_storage_bytes())
            .saturating_add(self.domain.as_ref().map_or(0, DomainWords::bytes))
            .saturating_add(self.history_selected.as_ref().map_or(0, |rows| {
                rows.capacity()
                    + std::mem::size_of::<Vec<bool>>()
                    + 2 * std::mem::size_of::<usize>()
            }))
    }
    pub fn prepare_alpha_tracks(
        &self,
        mesh: &Mesh,
        state: &SourceGeometryNormalState,
        contacts: [Option<SourceGeometryPass>; 2],
        tracks: crate::SourceGeometryAlphaTracks,
    ) -> Result<crate::SourceGeometryAlphaTracks, SourceGeometryError> {
        validate_mesh(mesh)?;
        self.validate_domain(mesh)?;
        tracks.validate()?;
        if !state.matches_mesh(mesh) {
            return Err(SourceGeometryError::StaleNormalState);
        }
        let mut out = tracks;
        for (index, contact) in contacts.into_iter().enumerate() {
            let Some(pass) = contact else { continue };
            pass.validate(mesh)?;
            let picked = state.picked_normal_after_validation(mesh, pass)?;
            let current = if index == 0 { out.primary } else { out.mirror };
            let next = current.updated(pass, picked)?;
            if index == 0 {
                out.primary = next
            } else {
                out.mirror = next
            }
        }
        Ok(out)
    }
    /// Public prescribed controls entry. Both selections/picked normals use the
    /// same validated pre-dab cache. Only after both passes succeed is this stroke
    /// published. A second-pass rejection restores exact public geometry words.
    #[allow(clippy::too_many_arguments)] // Explicit captured controls/limits.
    pub fn apply_controlled(
        &mut self,
        mesh: &mut Mesh,
        state: &Arc<SourceGeometryNormalState>,
        contacts: [Option<SourceGeometryPass>; 2],
        pressure: f64,
        in_contact: bool,
        controls: &crate::SourceGeometryControls,
        tracks: crate::SourceGeometryAlphaTracks,
        max_state_bytes: usize,
    ) -> Result<
        (
            crate::SourceGeometryControlledReport,
            Option<Arc<SourceGeometryNormalState>>,
        ),
        SourceGeometryError,
    > {
        validate_mesh(mesh)?;
        self.options.validate()?;
        tracks.validate()?;
        if !pressure.is_finite()
            || !(0.0..=1.0).contains(&pressure)
            || (!controls.mirror && contacts[1].is_some())
            || max_state_bytes > MAX_SOURCE_GEOMETRY_STATE_BYTES
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        for p in contacts.iter().flatten() {
            p.validate(mesh)?;
        }
        if !state.matches_mesh(mesh)
            || self
                .expected_generation
                .is_some_and(|g| g != state.generation)
        {
            return Err(SourceGeometryError::StaleNormalState);
        }
        self.validate_domain(mesh)?;
        self.apply_validated_controlled(
            mesh,
            state,
            contacts,
            pressure,
            in_contact,
            controls,
            tracks,
            max_state_bytes,
        )
    }
    pub(crate) fn apply_document_controlled_seal(
        &mut self,
        input: crate::document::ValidatedControlledSourceDab<'_>,
        max_state_bytes: usize,
    ) -> Result<
        (
            crate::SourceGeometryControlledReport,
            Option<Arc<SourceGeometryNormalState>>,
        ),
        SourceGeometryError,
    > {
        input.with_parts(
            |mesh, state, contacts, pressure, in_contact, controls, tracks| {
                self.apply_validated_controlled(
                    mesh,
                    state,
                    contacts,
                    pressure,
                    in_contact,
                    controls,
                    tracks,
                    max_state_bytes,
                )
            },
        )
    }
    // Identical atomic math below either the public standalone validation or
    // the consumed exclusive Document proof. No cross-call trust is retained.
    #[allow(clippy::too_many_arguments)] // Captured atomic inputs stay explicit.
    fn apply_validated_controlled(
        &mut self,
        mesh: &mut Mesh,
        state: &Arc<SourceGeometryNormalState>,
        contacts: [Option<SourceGeometryPass>; 2],
        pressure: f64,
        in_contact: bool,
        controls: &crate::SourceGeometryControls,
        tracks: crate::SourceGeometryAlphaTracks,
        max_state_bytes: usize,
    ) -> Result<
        (
            crate::SourceGeometryControlledReport,
            Option<Arc<SourceGeometryNormalState>>,
        ),
        SourceGeometryError,
    > {
        let dimensions = controls.alpha.as_ref().map(|a| [a.width(), a.height()]);
        let mut report = crate::SourceGeometryControlledReport {
            combined: SourceGeometryReport::noop(SourceGeometryNoop::EmptySelection),
            passes: [None, None],
            contacts,
            tracks,
            alpha_dimensions: dimensions,
        };
        if !in_contact {
            report.combined = SourceGeometryReport::noop(SourceGeometryNoop::NoContact);
            return Ok((report, None));
        }
        operation_budget(
            mesh,
            state
                .approximate_bytes()
                .checked_add(controls.retained_bytes())
                .ok_or(SourceGeometryError::StorageLimit)?,
        )?;
        let mut prepared: [Option<PreparedControlPass>; 2] = [None, None];
        let mut prepared_bytes = 0usize;
        for (index, contact) in contacts.into_iter().enumerate() {
            let Some(pass) = contact else { continue };
            let (selected, visits) =
                if self.options.clay_order == SourceClayOrderPolicy::OrderedStaticOctree {
                    let selection = state.ordered_tree.select(
                        &mesh.positions,
                        &state.faces,
                        pass.center,
                        pass.local_radius_squared,
                    )?;
                    (selection.selected_vertices, selection.visited_leaves)
                } else {
                    (selected_vertices(mesh, &state.faces, pass)?, Vec::new())
                };
            let picked = state.picked_normal_after_validation(mesh, pass)?;
            let current = if index == 0 {
                tracks.primary
            } else {
                tracks.mirror
            };
            let track = current.updated(pass, picked)?;
            if index == 0 {
                report.tracks.primary = track
            } else {
                report.tracks.mirror = track
            }
            prepared_bytes = prepared_bytes
                .checked_add(
                    selected
                        .capacity()
                        .saturating_mul(std::mem::size_of::<usize>()),
                )
                .and_then(|v| {
                    v.checked_add(
                        visits
                            .capacity()
                            .saturating_mul(std::mem::size_of::<SourceLeafHandle>()),
                    )
                })
                .ok_or(SourceGeometryError::StorageLimit)?;
            prepared[index] = Some(PreparedControlPass {
                pass,
                selected,
                visits,
                picked,
                track,
                mirrored: index == 1,
            });
        }
        let effective_intensity =
            self.options.intensity * (1.0 + self.options.intensity_factor * (pressure * 2.0 - 1.0));
        let smooth_stencil = if matches!(self.options.tool, SourceGeometryTool::Smooth { .. })
            && effective_intensity != 0.0
            && prepared
                .iter()
                .flatten()
                .any(|p| p.selected.iter().any(|&v| mesh.masks[v] != 0.0))
        {
            let selections = [
                prepared[0]
                    .as_ref()
                    .map_or(&[] as &[usize], |p| p.selected.as_slice()),
                prepared[1]
                    .as_ref()
                    .map_or(&[] as &[usize], |p| p.selected.as_slice()),
            ];
            let cost = state
                .approximate_bytes()
                .checked_add(prepared_bytes)
                .and_then(|v| v.checked_add(controls.retained_bytes()))
                .ok_or(SourceGeometryError::StorageLimit)?;
            Some(source_smooth::SmoothStencil::build(
                mesh,
                state,
                &selections,
                cost,
            )?)
        } else {
            None
        };
        if self.history_selected.is_some() {
            operation_budget_with_scratch(mesh, state.approximate_bytes(), mesh.positions.len())?;
        }
        let mut staged = self.staged_copy()?;
        if let Some(prior) = staged.history_selected.as_ref() {
            let mut selected = if prior.is_empty() {
                filled(mesh.positions.len(), false)?
            } else {
                copied(prior.as_slice())?
            };
            if selected.len() != mesh.positions.len() {
                return Err(SourceGeometryError::StrokeBindingChanged);
            }
            for pass in prepared.iter().flatten() {
                for &vertex in &pass.selected {
                    selected[vertex] = true;
                }
            }
            staged.history_selected = Some(Arc::new(selected));
        }
        let old_positions = copied(&mesh.positions)?;
        let old_normals = copied(&mesh.normals)?;
        let extra_peak = old_positions
            .capacity()
            .checked_mul(12)
            .and_then(|v| v.checked_add(old_normals.capacity().checked_mul(12)?))
            .and_then(|v| v.checked_add(staged.staged_bytes()))
            .and_then(|v| v.checked_add(prepared_bytes))
            .and_then(|v| v.checked_add(controls.retained_bytes()))
            .and_then(|v| v.checked_add(state.approximate_bytes()))
            .ok_or(SourceGeometryError::StorageLimit)?;
        // Complete Smooth workspace admission precedes the first primary write.
        // Mirror candidates reuse it but recompute their means from accepted live
        // primary geometry; no mirror candidate arrays are allocated in the loop.
        let mut smooth_workspace = if let Some(stencil) = smooth_stencil.as_ref() {
            let rows = prepared
                .iter()
                .flatten()
                .map(|p| p.selected.len())
                .max()
                .unwrap_or(0);
            let cost = state
                .approximate_bytes()
                .checked_add(extra_peak)
                .ok_or(SourceGeometryError::StorageLimit)?;
            Some(source_smooth::SmoothCandidateWorkspace::reserve(
                mesh, stencil, rows, cost,
            )?)
        } else {
            None
        };
        operation_budget_with_scratch(
            mesh,
            state
                .approximate_bytes()
                .checked_add(extra_peak)
                .ok_or(SourceGeometryError::StorageLimit)?,
            smooth_stencil
                .as_ref()
                .map_or(0, source_smooth::SmoothStencil::bytes)
                .checked_add(
                    smooth_workspace
                        .as_ref()
                        .map_or(0, source_smooth::SmoothCandidateWorkspace::bytes),
                )
                .ok_or(SourceGeometryError::StorageLimit)?,
        )?;
        let result = (|| {
            let mut current = state.clone();
            for (index, p) in prepared.iter().enumerate() {
                let Some(p) = p else { continue };
                // Static makeStroke prepares both picked directions before
                // either callback. Crease uses that captured direction even
                // though its mirror reads accepted live primary positions.
                // The first producer still aliases the caller's original Arc.
                // Only a fresh intermediate with no strong/weak aliases can
                // lend mutable buffers; shared or observed states keep copies.
                let access = match Arc::get_mut(&mut current) {
                    Some(private) => NormalStateAccess::Private(private),
                    None => NormalStateAccess::Shared(&current),
                };
                let (pass_report, next) = staged.apply_access_math(
                    mesh,
                    access,
                    p.pass,
                    pressure,
                    true,
                    max_state_bytes,
                    Some(p),
                    controls.alpha.as_ref(),
                    extra_peak,
                    smooth_stencil.as_ref(),
                    smooth_workspace.as_mut(),
                )?;
                if let Some(next) = next {
                    current = Arc::new(next);
                }
                // Flatten returns before updateAlpha when no area normal exists.
                // Other tools retain their original preparation/publication schedule.
                if self.options.tool == SourceGeometryTool::Flatten
                    && pass_report.noop == Some(SourceGeometryNoop::NoAreaDirection)
                {
                    if index == 0 {
                        report.tracks.primary = tracks.primary;
                    } else {
                        report.tracks.mirror = tracks.mirror;
                    }
                }
                report.passes[index] = Some(pass_report);
            }
            // A single pass already counted its exact changed rows. Only two
            // passes need the whole-mesh comparison, since their changed sets
            // can overlap or the second pass can restore a primary word.
            report.combined.changed_vertices = match (&report.passes[0], &report.passes[1]) {
                (Some(primary), None) => primary.changed_vertices,
                (None, Some(mirror)) => mirror.changed_vertices,
                (None, None) => 0,
                (Some(_), Some(_)) => mesh
                    .positions
                    .iter()
                    .zip(&old_positions)
                    .filter(|(a, b)| {
                        a.to_array().map(f32::to_bits) != b.to_array().map(f32::to_bits)
                    })
                    .count(),
            };
            for r in report.passes.iter().flatten() {
                if let Some(phase) = r.phase_time {
                    let total = report
                        .combined
                        .phase_time
                        .get_or_insert_with(SourceGeometryPhaseTime::default);
                    total.selection += phase.selection;
                    total.old_copy += phase.old_copy;
                    total.candidate_and_guard += phase.candidate_and_guard;
                    total.state_advance += phase.state_advance;
                    total.normal_refresh += phase.normal_refresh;
                    total.publish += phase.publish;
                }
                report.combined.derived_cache_changed |= r.derived_cache_changed;
                report.combined.selected_vertices = report
                    .combined
                    .selected_vertices
                    .saturating_add(r.selected_vertices);
                report.combined.front_vertices = report
                    .combined
                    .front_vertices
                    .saturating_add(r.front_vertices);
                report.combined.proxy_captured_vertices = report
                    .combined
                    .proxy_captured_vertices
                    .saturating_add(r.proxy_captured_vertices);
                report.combined.source_normal_rows_refreshed = report
                    .combined
                    .source_normal_rows_refreshed
                    .saturating_add(r.source_normal_rows_refreshed);
                report.combined.displacement_scale =
                    report.combined.displacement_scale.max(r.displacement_scale);
                report.combined.normal_generation = r.normal_generation;
                report.combined.picked_normal = r.picked_normal;
                if r.flatten_plane.is_some() {
                    report.combined.flatten_plane = r.flatten_plane;
                }
                report.combined.clay_single_leaf |= r.clay_single_leaf;
                report.combined.ordered_static_selection |= r.ordered_static_selection;
                report.combined.ordered_root_rebuilds = r.ordered_root_rebuilds;
                report.combined.noop = r.noop;
            }
            if report.combined.changed_vertices > 0 {
                report.combined.noop = None;
            }
            let mut next = (!Arc::ptr_eq(&current, state)).then_some(current);
            if let Some(private) = next.as_mut().and_then(Arc::get_mut) {
                // Both Source passes select against the captured pre-dab mesh.
                // Their union covers every position/normal row this checked
                // operation may publish. A later renderer must also prove the
                // exact predecessor Arc before using this small envelope.
                let mut chunks = [0_u64; SOURCE_GEOMETRY_CHUNK_WORDS];
                for selected in prepared.iter().flatten() {
                    for &vertex in &selected.selected {
                        let chunk = vertex / SOURCE_GEOMETRY_CHUNK_ROWS;
                        chunks[chunk / 64] |= 1_u64 << (chunk % 64);
                    }
                }
                private.render_predecessor = Some(Arc::downgrade(state));
                private.render_selected_chunks = chunks;
            }
            Ok((report, next))
        })();
        match result {
            Ok(value) => {
                *self = staged;
                Ok(value)
            }
            Err(e) => {
                mesh.positions.copy_from_slice(&old_positions);
                mesh.normals.copy_from_slice(&old_normals);
                Err(e)
            }
        }
    }
}
impl DomainWords {
    fn bytes(&self) -> usize {
        self.normals
            .capacity()
            .saturating_mul(12)
            .saturating_add(self.masks.capacity().saturating_mul(4))
            .saturating_add(self.colors.capacity().saturating_mul(12))
            .saturating_add(
                self.materials
                    .as_ref()
                    .map_or(0, |v| v.capacity().saturating_mul(8)),
            )
            .saturating_add(self.uv.as_ref().map_or(0, |(a, b)| {
                a.capacity()
                    .saturating_mul(8)
                    .saturating_add(b.capacity().saturating_mul(16))
            }))
    }
}

#[cfg(test)]
mod retained_budget_tests {
    use super::*;
    #[test]
    fn insufficient_retained_budget_restores_every_word_before_publication() {
        let mut mesh = Mesh::from_obj("v -1 -1 0\nv 1 -1 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.);
        let before = serde_json::to_value(&mesh).unwrap();
        let state = SourceGeometryNormalState::seed(&mesh).unwrap();
        let before_field = state.vertex_normals.clone();
        let mut stroke = SourceGeometryStroke::new(SourceGeometryOptions::inflate()).unwrap();
        let pass = SourceGeometryPass {
            center: [0.; 3],
            eye_direction: [0., 0., -1.],
            local_radius_squared: 4.,
            original_face_index: 0,
        };
        assert_eq!(
            stroke
                .apply_with_state_limit(&mut mesh, &state, pass, 0.5, true, 0)
                .unwrap_err(),
            SourceGeometryError::StorageLimit
        );
        assert_eq!(serde_json::to_value(&mesh).unwrap(), before);
        assert_eq!(state.vertex_normals, before_field);
        assert!(stroke.proxy_positions().is_empty());
        assert!(stroke.domain.is_none());
        assert_eq!(state.generation(), 0);
    }
}

#[cfg(test)]
pub(crate) mod validation_probe {
    use std::cell::Cell;
    thread_local! {static COUNTS:Cell<[usize;3]>=const {Cell::new([0;3])};}
    pub(crate) fn record(kind: usize) {
        COUNTS.with(|c| {
            let mut n = c.get();
            n[kind] = n[kind].saturating_add(1);
            c.set(n);
        });
    }
    pub(crate) fn reset() {
        COUNTS.with(|c| c.set([0; 3]));
    }
    pub(crate) fn counts() -> [usize; 3] {
        COUNTS.with(Cell::get)
    }
}
#[cfg(test)]
#[path = "source_geometry_seal_tests.rs"]
mod sealed_dab_tests;

#[cfg(test)]
#[path = "source_geometry_pinch_budget_tests.rs"]
mod pinch_budget_tests;

#[cfg(test)]
#[path = "source_flatten_unit_tests.rs"]
mod flatten_tests;

#[cfg(test)]
mod pinned_runtime_brush_pair {
    use super::*;

    #[test]
    fn plain_brush_moving_second_dab_matches_pinned_alpha_frame_triangle() {
        let mut mesh =
            Mesh::from_obj("v -0.5 -0.25 0\nv 0.5 -0.25 0\nv 0 0.5 0\nf 1 2 3\n").unwrap();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let state = Arc::new(SourceGeometryNormalState::seed(&mesh).unwrap());
        let mut stroke = SourceGeometryStroke::new(SourceGeometryOptions {
            tool: SourceGeometryTool::Brush {
                clay: false,
                accumulate: true,
            },
            intensity: 0.25,
            prevent_folds: false,
            ..Default::default()
        })
        .unwrap();
        let controls = crate::SourceGeometryControls {
            alpha: Some(
                crate::RasterAlpha::new(
                    16,
                    16,
                    include_bytes!(
                        "../../obrush-app/tests/fixtures/pinned-sculptgl-alpha-quantization16x16.bin"
                    )
                    .to_vec(),
                )
                .unwrap(),
            ),
            mirror: false,
        };
        let mut current = state;
        let mut tracks = crate::SourceGeometryAlphaTracks::default();
        for (center, words) in [
            (
                [0.0, 0.0, 0.0],
                [
                    0xbf000000, 0xbe800000, 0x3c48d0de, 0x3f000000, 0xbe800000, 0x3cb33b9e,
                    0x00000000, 0x3f000000, 0x3c6f28f3,
                ],
            ),
            (
                [0.125, 0.125, 0.0],
                [
                    0xbf000627, 0xbe7ff793, 0x3cb45763, 0x3effef3e, 0xbe7ff487, 0x3d100929,
                    0xb933b002, 0x3f0003d8, 0x3d04bc89,
                ],
            ),
        ] {
            let pass = SourceGeometryPass {
                center,
                eye_direction: [0.0, 0.0, -1.0],
                local_radius_squared: 2.0,
                original_face_index: 0,
            };
            let before_positions = mesh.positions.clone();
            let (report, next) = stroke
                .apply_controlled(
                    &mut mesh,
                    &current,
                    [Some(pass), None],
                    1.0,
                    true,
                    &controls,
                    tracks,
                    MAX_SOURCE_GEOMETRY_STATE_BYTES,
                )
                .unwrap();
            assert_eq!(report.combined.changed_vertices, 3);
            assert_eq!(
                mesh.positions
                    .iter()
                    .flat_map(|p| p.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>(),
                words
            );
            if center[0] != 0.0 {
                let alpha = controls.alpha.as_ref().unwrap();
                for (&position, expected_byte) in before_positions.iter().zip([106., 113., 138.]) {
                    assert_eq!(
                        report
                            .tracks
                            .primary
                            .sample(alpha, pass, false, position)
                            .unwrap(),
                        expected_byte / 255.0
                    );
                }
                let expected_frame = [
                    0xbf350304, 0x3f350298, 0x3c1da448, 0x00000000, 0x3f350304, 0x3f350661,
                    0xbb57e304, 0x00000000, 0xbc15a281, 0x3b929afe, 0xbf7ffc9c, 0x00000000,
                    0x00000000, 0xbe35047c, 0xba4f570d, 0x3f800000,
                ];
                for (index, (&actual, &reference)) in report
                    .tracks
                    .primary
                    .frame
                    .iter()
                    .zip(&expected_frame)
                    .enumerate()
                {
                    if index == 12 {
                        // Pinned gl-matrix writes +0; this f64 dot writes -0.
                        // Both sample the same alpha. Preserve the exact-frame
                        // failed diagnostic in C171 rather than hiding it.
                        assert_eq!(actual, 0.0);
                    } else {
                        assert_eq!(actual.to_bits(), reference);
                    }
                }
            }
            tracks = report.tracks;
            current = next.expect("changed dab refreshes Source normals");
        }
    }

    #[test]
    fn plain_brush_two_dabs_match_refreshed_pinned_normal_triangle_without_alpha() {
        let mut mesh =
            Mesh::from_obj("v -0.5 -0.25 0\nv 0.5 -0.25 0\nv 0 0.5 0\nf 1 2 3\n").unwrap();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let state = Arc::new(SourceGeometryNormalState::seed(&mesh).unwrap());
        let mut stroke = SourceGeometryStroke::new(SourceGeometryOptions {
            tool: SourceGeometryTool::Brush {
                clay: false,
                accumulate: true,
            },
            intensity: 0.25,
            prevent_folds: false,
            ..Default::default()
        })
        .unwrap();
        let pass = SourceGeometryPass {
            center: [0.0; 3],
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 2.0,
            original_face_index: 0,
        };
        let expected = [
            [
                0xbf000000, 0xbe800000, 0x3cef4a3b, 0x3f000000, 0xbe800000, 0x3cef4a3b, 0x00000000,
                0x3f000000, 0x3cfc01e4,
            ],
            [
                0xbf000000, 0xbe8007eb, 0x3d6f337a, 0x3f000000, 0xbe8007eb, 0x3d6f337a, 0x00000000,
                0x3efff7a9, 0x3d7be9c2,
            ],
        ];
        let mut current = state;
        for words in expected {
            let (report, next) = stroke
                .apply_controlled(
                    &mut mesh,
                    &current,
                    [Some(pass), None],
                    1.0,
                    true,
                    &Default::default(),
                    Default::default(),
                    MAX_SOURCE_GEOMETRY_STATE_BYTES,
                )
                .unwrap();
            assert_eq!(report.combined.changed_vertices, 3);
            assert_eq!(
                mesh.positions
                    .iter()
                    .flat_map(|p| p.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>(),
                words
            );
            current = next.expect("changed dab refreshes Source normals");
        }
    }

    #[test]
    fn plain_brush_two_dabs_match_pinned_alpha_runtime_triangle() {
        let mut mesh =
            Mesh::from_obj("v -0.5 -0.25 0\nv 0.5 -0.25 0\nv 0 0.5 0\nf 1 2 3\n").unwrap();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let initial = mesh.positions.clone();
        let state = Arc::new(SourceGeometryNormalState::seed(&mesh).unwrap());
        let mut stroke = SourceGeometryStroke::new(SourceGeometryOptions {
            tool: SourceGeometryTool::Brush {
                clay: false,
                accumulate: true,
            },
            intensity: 0.25,
            prevent_folds: false,
            ..Default::default()
        })
        .unwrap();
        let alpha = crate::RasterAlpha::new(
            16,
            16,
            include_bytes!(
                "../../obrush-app/tests/fixtures/pinned-sculptgl-alpha-quantization16x16.bin"
            )
            .to_vec(),
        )
        .unwrap();
        let pass = SourceGeometryPass {
            center: [0.0; 3],
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 2.0,
            original_face_index: 0,
        };
        let controls = crate::SourceGeometryControls {
            alpha: Some(alpha),
            mirror: false,
        };
        let (report, next) = stroke
            .apply_controlled(
                &mut mesh,
                &state,
                [Some(pass), None],
                1.0,
                true,
                &controls,
                Default::default(),
                MAX_SOURCE_GEOMETRY_STATE_BYTES,
            )
            .unwrap();
        assert_eq!(report.combined.changed_vertices, 3);
        let expected = [0x3c48d0de, 0x3cb33b9e, 0x3c6f28f3];
        for (i, (&before, &after)) in initial.iter().zip(&mesh.positions).enumerate() {
            assert_eq!(after.x.to_bits(), before.x.to_bits());
            assert_eq!(after.y.to_bits(), before.y.to_bits());
            assert_eq!(after.z.to_bits(), expected[i]);
        }
        let next = next.expect("changed dab refreshes Source normals");
        let (report, _) = stroke
            .apply_controlled(
                &mut mesh,
                &next,
                [Some(pass), None],
                1.0,
                true,
                &controls,
                Default::default(),
                MAX_SOURCE_GEOMETRY_STATE_BYTES,
            )
            .unwrap();
        assert_eq!(report.combined.changed_vertices, 3);
        let expected_second = [
            0xbf0007ba, 0xbe7ff56b, 0x3cc8cc30, 0x3effe46b, 0xbe7fed1d, 0x3d3330e6, 0xb9133f41,
            0x3f000327, 0x3cef223a,
        ];
        assert_eq!(
            mesh.positions
                .iter()
                .flat_map(|p| p.to_array().map(f32::to_bits))
                .collect::<Vec<_>>(),
            expected_second
        );
    }
}

#[cfg(test)]
mod normal_seed_ownership_regressions {
    use super::*;
    #[test]
    fn retained_topology_proof_rejects_changed_faces_diagonals_and_trailing_rows() {
        let mut triangle_mesh = Mesh::sphere(1);
        triangle_mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        triangle_mesh.masks.fill(1.0);
        let triangle_state = SourceGeometryNormalState::seed(&triangle_mesh).unwrap();
        assert!(triangle_state.matches_mesh(&triangle_mesh));
        triangle_mesh.triangles.push(triangle_mesh.triangles[0]);
        assert!(!triangle_state.matches_mesh(&triangle_mesh));

        let mut quad_mesh = Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n")
            .unwrap()
            .mesh;
        quad_mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        quad_mesh.masks.fill(1.0);
        let quad_state = SourceGeometryNormalState::seed(&quad_mesh).unwrap();
        assert!(quad_state.matches_mesh(&quad_mesh));
        let original = quad_mesh.triangles[1];
        quad_mesh.triangles[1].swap(0, 1);
        assert!(!quad_state.matches_mesh(&quad_mesh));
        quad_mesh.triangles[1] = original;
        quad_mesh.surface.as_mut().unwrap().faces[0][0] = 1;
        assert!(!quad_state.matches_mesh(&quad_mesh));
        quad_mesh.surface.as_mut().unwrap().faces[0][0] = 0;
        assert!(quad_state.matches_mesh(&quad_mesh));
    }
    #[test]
    fn moved_seed_preserves_all_rows_and_still_detects_public_mesh_edits() {
        for mut mesh in [Mesh::sphere(2), Mesh::cube()] {
            mesh.mask_convention = MaskConvention::UnprotectedF32V1;
            mesh.masks.fill(1.0);
            mesh.positions[0].x = -0.0;
            let before = serde_json::to_vec(&mesh).unwrap();
            let field =
                SourceNormalField::compute(&mesh, SourceNormalPolicy::StaticOriginalFaces).unwrap();
            let state = SourceGeometryNormalState::seed(&mesh).unwrap();
            assert_eq!(
                state.position_words,
                mesh.positions
                    .iter()
                    .map(|p| p.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                state
                    .face_normals
                    .iter()
                    .map(|p| p.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>(),
                field
                    .face_normals()
                    .iter()
                    .map(|p| p.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                state
                    .vertex_normals
                    .iter()
                    .map(|p| p.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>(),
                field
                    .vertex_normals()
                    .iter()
                    .map(|p| p.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>()
            );
            assert!(state.matches_mesh(&mesh));
            assert_eq!(state.generation(), 0);
            assert_eq!(serde_json::to_vec(&mesh).unwrap(), before);
            mesh.positions[0].x = 0.0;
            assert!(!state.matches_mesh(&mesh));
            mesh.positions[0].x = -0.0;
            assert!(state.matches_mesh(&mesh));
            mesh.triangles[0].swap(0, 1);
            assert!(!state.matches_mesh(&mesh));
        }
    }
}

#[cfg(test)]
mod local_normal_refresh_regressions {
    use super::*;
    // Frozen C48 scatter used as the differential oracle. Its arrays are
    // deliberately full-size; production uses the ordered incident rings.
    fn scatter(
        state: &SourceGeometryNormalState,
        positions: &[Vec3],
        selected: &[usize],
    ) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut tags = vec![false; positions.len()];
        for &v in selected {
            tags[v] = true;
        }
        let mut faces = state.face_normals.clone();
        for (i, face) in state.faces.iter().enumerate() {
            if corners(*face).any(|v| tags[v]) {
                faces[i] = face_normal(positions, *face).unwrap();
            }
        }
        let mut sums = vec![[0.0; 3]; positions.len()];
        let mut counts = vec![0u32; positions.len()];
        for (i, face) in state.faces.iter().enumerate().rev() {
            for v in corners(*face) {
                if tags[v] {
                    let n = faces[i].to_array().map(f64::from);
                    for c in 0..3 {
                        sums[v][c] += n[c];
                    }
                    counts[v] += 1;
                }
            }
        }
        let mut vertices = state.vertex_normals.clone();
        for &v in selected {
            let factor = if counts[v] == 0 {
                0.0
            } else {
                1.0 / f64::from(counts[v])
            };
            vertices[v] = Vec3::from_array(sums[v].map(|n| (n * factor) as f32));
        }
        (faces, vertices)
    }
    fn words(rows: &[Vec3]) -> Vec<[u32; 3]> {
        rows.iter()
            .map(|p| p.to_array().map(f32::to_bits))
            .collect()
    }
    fn quad() -> Mesh {
        let mut m = Mesh::cube();
        m.positions = vec![
            Vec3::ZERO,
            Vec3::X,
            Vec3::ONE,
            Vec3::Y,
            Vec3::new(0.125, 0.5, -0.25),
        ];
        m.triangles = vec![[0, 1, 2], [0, 2, 3]];
        m.normals = vec![Vec3::ZERO; 5];
        m.masks = vec![1.; 5];
        m.colors = vec![[0.5; 3]; 5];
        m.surface = Some(crate::SurfaceTopology {
            faces: vec![[0, 1, 2, 3]],
            uv: None,
        });
        m
    }
    #[test]
    fn controlled_selected_position_witness_matches_full_refresh_bit_for_bit() {
        let mut mesh = Mesh::sphere(3);
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        let state = SourceGeometryNormalState::seed(&mesh).unwrap();
        let selected = [0, 1, 1, mesh.positions.len() / 2];
        mesh.positions[0].x = -0.0;
        mesh.positions[1].z += 0.03125;
        mesh.positions[selected[3]].y -= 0.015625;
        let full = state.refreshed(&mesh.positions, &selected).unwrap();
        let scoped = state
            .refreshed_after_selected_write(&mesh.positions, &selected)
            .unwrap();
        assert_eq!(full.position_words, scoped.position_words);
        assert_eq!(words(&full.face_normals), words(&scoped.face_normals));
        assert_eq!(words(&full.vertex_normals), words(&scoped.vertex_normals));
        assert!(scoped.matches_mesh(&mesh));

        // History may replace an unrelated position, so it must keep the
        // complete rebuild rather than this controlled-dab shortcut.
        mesh.positions[3].z += 0.125;
        assert!(state
            .refreshed(&mesh.positions, &[0])
            .unwrap()
            .matches_mesh(&mesh));
        assert!(!state
            .refreshed_after_selected_write(&mesh.positions, &[0])
            .unwrap()
            .matches_mesh(&mesh));
    }
    #[test]
    fn fixed_topology_refresh_keeps_exact_face_storage_identity() {
        let mut mesh = quad();
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        let original = SourceGeometryNormalState::seed(&mesh).unwrap();
        let refreshed = original.refreshed(&mesh.positions, &[0]).unwrap();
        let independent = SourceGeometryNormalState::seed(&mesh).unwrap();
        assert!(original.shares_topology_storage(&refreshed));
        assert!(!original.shares_topology_storage(&independent));
    }
    #[test]
    fn incident_refresh_matches_frozen_scatter_for_partial_empty_and_duplicate_selections() {
        for mut mesh in [Mesh::sphere(2), Mesh::cube(), quad()] {
            mesh.mask_convention = MaskConvention::UnprotectedF32V1;
            mesh.masks.fill(1.);
            mesh.recompute_normals();
            let mut state = SourceGeometryNormalState::seed(&mesh).unwrap();
            for (step, selected) in [
                vec![0, 1],
                vec![2],
                vec![],
                vec![1, 1, 2],
                vec![mesh.positions.len() - 1],
            ]
            .into_iter()
            .enumerate()
            {
                for (i, p) in mesh.positions.iter_mut().enumerate() {
                    p.z += if i % 2 == 0 { 0.03125 } else { -0.015625 };
                }
                mesh.positions[0].x = if step % 2 == 0 { -0.0 } else { 0.0 };
                let expected = scatter(&state, &mesh.positions, &selected);
                let next = state.refreshed(&mesh.positions, &selected).unwrap();
                assert_eq!(words(&next.face_normals), words(&expected.0));
                assert_eq!(words(&next.vertex_normals), words(&expected.1));
                assert!(next.matches_mesh(&mesh));
                assert_eq!(next.generation(), state.generation() + 1);
                state = next;
            }
        }
    }
    #[test]
    fn incident_mean_keeps_descending_cancellation_and_unused_vertex_words() {
        let mut m = Mesh::cube();
        m.positions = vec![
            Vec3::ZERO,
            Vec3::new(1e6, 0., 0.),
            Vec3::new(0., 1e6, 0.),
            Vec3::new(0., 1e6, 0.),
            Vec3::new(1e6, 0., 0.),
            Vec3::new(0.001, 0., 0.),
            Vec3::new(0., 0.001, 0.),
            Vec3::Z,
        ];
        m.triangles = vec![[0, 1, 2], [0, 3, 4], [0, 5, 6]];
        m.normals = vec![Vec3::ZERO; 8];
        m.colors = vec![[0.5; 3]; 8];
        m.masks = vec![1.; 8];
        m.surface = None;
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m.recompute_normals();
        let state = SourceGeometryNormalState::seed(&m).unwrap();
        assert_eq!(state.ordered_tree.incident_faces(0).unwrap(), [2, 1, 0]);
        let next = state.refreshed(&m.positions, &[0, 0, 7]).unwrap();
        assert_eq!(next.vertex_normals[0].to_array().map(f32::to_bits), [0; 3]);
        assert_eq!(next.vertex_normals[7], Vec3::ZERO);
        let wrong = (f64::from(next.face_normals[0].z) + f64::from(next.face_normals[1].z))
            + f64::from(next.face_normals[2].z);
        assert!(wrong > 0.);
    }
}

#[cfg(test)]
mod warm_domain_reuse_regressions {
    use super::*;
    fn fixture() -> Mesh {
        let mut m=Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nv -4 4 0\nv 0 8 0\nvt 0 0\nvt 1 0\nvt 0 1\nvt 0.25 0.75\nvt 0.5 1\nvt 0.5 0.5\nf 1/1 2/2 3/3\nf 3/4 5/5 4/6\n").unwrap().mesh;
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m.masks.fill(1.);
        m.colors.fill([0.2, 0.4, 0.6]);
        m.materials = Some(vec![[0.2, 0.3]; m.positions.len()]);
        m
    }
    fn pass() -> SourceGeometryPass {
        SourceGeometryPass {
            center: [0.; 3],
            eye_direction: [0., 0., -1.],
            local_radius_squared: 4.,
            original_face_index: 0,
        }
    }
    fn pointers(d: &DomainWords) -> [usize; 6] {
        [
            d.normals.as_ptr() as usize,
            d.masks.as_ptr() as usize,
            d.colors.as_ptr() as usize,
            d.materials.as_ref().unwrap().as_ptr() as usize,
            d.uv.as_ref().unwrap().0.as_ptr() as usize,
            d.uv.as_ref().unwrap().1.as_ptr() as usize,
        ]
    }
    fn words(d: &DomainWords) -> serde_json::Value {
        serde_json::json!({
            "n": d.normals,
            "u": d.masks.as_ref(),
            "c": d.colors.as_ref(),
            "m": d.materials.as_ref().map(|v| v.as_ref()),
            "uv": d.uv.as_ref().map(|(a, b)| (a.as_ref(), b.as_ref())),
        })
    }
    fn warm() -> (Mesh, SourceGeometryNormalState, SourceGeometryStroke) {
        let mut m = fixture();
        let state = SourceGeometryNormalState::seed(&m).unwrap();
        let mut s = SourceGeometryStroke::new(SourceGeometryOptions {
            intensity: 0.625,
            ..SourceGeometryOptions::inflate()
        })
        .unwrap();
        let (r, next) = s.apply(&mut m, &state, pass(), 0.5, true).unwrap();
        assert_eq!(r.changed_vertices, 1);
        (m, next.unwrap(), s)
    }
    #[test]
    fn staged_stroke_shares_immutable_domain_but_owns_normal_witness() {
        let (mesh, _, stroke) = warm();
        let prior = stroke.domain.as_ref().unwrap();
        let mut staged = stroke.staged_copy().unwrap();
        let current = staged.domain.as_mut().unwrap();
        assert_ne!(current.normals.as_ptr(), prior.normals.as_ptr());
        assert!(Arc::ptr_eq(&current.masks, &prior.masks));
        assert!(Arc::ptr_eq(&current.colors, &prior.colors));
        assert!(Arc::ptr_eq(
            current.materials.as_ref().unwrap(),
            prior.materials.as_ref().unwrap()
        ));
        assert!(Arc::ptr_eq(
            &current.uv.as_ref().unwrap().0,
            &prior.uv.as_ref().unwrap().0
        ));
        assert!(Arc::ptr_eq(
            &current.uv.as_ref().unwrap().1,
            &prior.uv.as_ref().unwrap().1
        ));
        let original = prior.normals[0];
        current.normals[0][0] ^= 1;
        assert_eq!(prior.normals[0], original);
        assert!(prior.matches(&mesh));
        assert!(!current.matches(&mesh));
    }
    #[test]
    fn successful_warm_dabs_keep_expected_domain_allocations_and_full_entry_checks() {
        let (mut m, mut state, mut s) = warm();
        let ptr = pointers(s.domain.as_ref().unwrap());
        let attrs = words(s.domain.as_ref().unwrap());
        for _ in 0..3 {
            validation_probe::reset();
            let (r, next) = s.apply(&mut m, &state, pass(), 0.5, true).unwrap();
            assert_eq!(validation_probe::counts(), [1, 1, 1]);
            assert!(r.changed_vertices > 0);
            assert_eq!(pointers(s.domain.as_ref().unwrap()), ptr);
            let current = words(s.domain.as_ref().unwrap());
            for key in ["u", "c", "m", "uv"] {
                assert_eq!(current[key], attrs[key]);
            }
            assert_eq!(
                s.domain.as_ref().unwrap().normals,
                m.normals
                    .iter()
                    .map(|n| n.to_array().map(f32::to_bits))
                    .collect::<Vec<_>>()
            );
            state = next.unwrap();
        }
    }
    #[test]
    fn late_rejection_keeps_warm_domain_mesh_proxy_generation_and_visits_exact() {
        let (mut m, state, mut s) = warm();
        let mesh_before = serde_json::to_vec(&m).unwrap();
        let before = words(s.domain.as_ref().unwrap());
        let ptr = pointers(s.domain.as_ref().unwrap());
        let proxy = s.proxy.clone();
        let touched = s.touched.clone();
        let generation = s.expected_generation;
        let visits = s.visited_leaves.clone();
        assert_eq!(
            s.apply_with_state_limit(&mut m, &state, pass(), 0.5, true, 0)
                .unwrap_err(),
            SourceGeometryError::StorageLimit
        );
        assert_eq!(serde_json::to_vec(&m).unwrap(), mesh_before);
        assert_eq!(words(s.domain.as_ref().unwrap()), before);
        assert_eq!(pointers(s.domain.as_ref().unwrap()), ptr);
        assert_eq!(s.proxy, proxy);
        assert_eq!(s.touched, touched);
        assert_eq!(s.expected_generation, generation);
        assert_eq!(s.visited_leaves, visits);
        let (r, _) = s.apply(&mut m, &state, pass(), 0.5, true).unwrap();
        assert!(r.changed_vertices > 0);
    }
    #[test]
    fn public_domain_edits_are_never_adopted_by_warm_reuse() {
        for field in 0..6 {
            let (mut m, state, mut s) = warm();
            let prior = words(s.domain.as_ref().unwrap());
            let ptr = pointers(s.domain.as_ref().unwrap());
            match field {
                0 => m.normals[3].x += 0.25,
                1 => m.masks[3] = 0.5,
                2 => m.colors[3][0] = 0.6,
                3 => m.materials.as_mut().unwrap()[3][0] = 0.7,
                4 => m.surface.as_mut().unwrap().uv.as_mut().unwrap().coordinates[0][0] = 0.125,
                5 => m
                    .surface
                    .as_mut()
                    .unwrap()
                    .uv
                    .as_mut()
                    .unwrap()
                    .face_indices[0]
                    .swap(0, 1),
                _ => unreachable!(),
            };
            m.validate().unwrap();
            let changed = serde_json::to_vec(&m).unwrap();
            assert_eq!(
                s.apply(&mut m, &state, pass(), 0.5, true).unwrap_err(),
                SourceGeometryError::StrokeBindingChanged
            );
            assert_eq!(serde_json::to_vec(&m).unwrap(), changed);
            assert_eq!(words(s.domain.as_ref().unwrap()), prior);
            assert_eq!(pointers(s.domain.as_ref().unwrap()), ptr);
        }
    }
}

#[cfg(test)]
mod warm_proxy_reuse_regressions {
    use super::*;
    fn fixture() -> Mesh {
        let mut mesh = Mesh::import_obj(
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nv 2 0 0\nv 3 0 0\nv 2 1 0\nf 1 2 3\nf 4 5 6\n",
        )
        .unwrap()
        .mesh;
        mesh.mask_convention = crate::MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.);
        mesh
    }
    fn pass(second: bool) -> SourceGeometryPass {
        SourceGeometryPass {
            center: [if second { 2. } else { 0. }, 0., 0.],
            eye_direction: [0., 0., -1.],
            local_radius_squared: 0.25,
            original_face_index: usize::from(second),
        }
    }
    fn options() -> SourceGeometryOptions {
        SourceGeometryOptions {
            intensity: 0.625,
            prevent_folds: false,
            ..SourceGeometryOptions::inflate()
        }
    }
    fn proxy_words(s: &SourceGeometryStroke) -> Vec<[u32; 3]> {
        s.proxy
            .iter()
            .map(|v| v.to_array().map(f32::to_bits))
            .collect()
    }
    fn warm() -> (Mesh, SourceGeometryNormalState, SourceGeometryStroke) {
        let mut m = fixture();
        let state = SourceGeometryNormalState::seed(&m).unwrap();
        let mut s = SourceGeometryStroke::new(options()).unwrap();
        let (r, next) = s.apply(&mut m, &state, pass(false), 0.5, true).unwrap();
        assert_eq!(r.changed_vertices, 1);
        assert_eq!(r.proxy_captured_vertices, 1);
        (m, next.unwrap(), s)
    }
    #[test]
    fn useful_warm_first_touches_keep_owned_proxy_and_touched_allocations() {
        let (mut m, mut state, mut s) = warm();
        let pointers = (s.proxy.as_ptr(), s.touched.as_ptr());
        let initial = fixture();
        for index in 0..3 {
            validation_probe::reset();
            let (r, next) = s.apply(&mut m, &state, pass(true), 0.5, true).unwrap();
            assert_eq!(validation_probe::counts(), [1, 1, 1]);
            assert_eq!(r.changed_vertices, 1);
            assert_eq!(r.proxy_captured_vertices, usize::from(index == 0));
            assert_eq!((s.proxy.as_ptr(), s.touched.as_ptr()), pointers);
            assert_eq!(s.touched, vec![true, false, false, true, false, false]);
            assert_eq!(s.proxy, initial.positions);
            state = next.unwrap();
        }
    }
    #[test]
    fn late_failure_does_not_publish_new_first_touches_and_next_dab_captures_them() {
        let (mut m, state, mut s) = warm();
        let mesh = serde_json::to_vec(&m).unwrap();
        let proxy = proxy_words(&s);
        let touched = s.touched.clone();
        let pointers = (s.proxy.as_ptr(), s.touched.as_ptr());
        let generation = s.expected_generation;
        let visits = s.visited_leaves.clone();
        assert_eq!(
            s.apply_with_state_limit(&mut m, &state, pass(true), 0.5, true, 0)
                .unwrap_err(),
            SourceGeometryError::StorageLimit
        );
        assert_eq!(serde_json::to_vec(&m).unwrap(), mesh);
        assert_eq!(proxy_words(&s), proxy);
        assert_eq!(s.touched, touched);
        assert_eq!((s.proxy.as_ptr(), s.touched.as_ptr()), pointers);
        assert_eq!(s.expected_generation, generation);
        assert_eq!(s.visited_leaves, visits);
        let (r, _) = s.apply(&mut m, &state, pass(true), 0.5, true).unwrap();
        assert_eq!(r.changed_vertices, 1);
        assert_eq!(r.proxy_captured_vertices, 1);
        assert!(s.touched[3]);
    }
    #[test]
    fn capture_includes_selected_backfaces_even_when_no_coordinate_is_written() {
        let mut m = fixture();
        let state = SourceGeometryNormalState::seed(&m).unwrap();
        let mut s = SourceGeometryStroke::new(SourceGeometryOptions {
            culling: true,
            ..options()
        })
        .unwrap();
        for second in [false, true] {
            let p = SourceGeometryPass {
                eye_direction: [0., 0., 1.],
                ..pass(second)
            };
            let before = serde_json::to_vec(&m).unwrap();
            let (r, next) = s.apply(&mut m, &state, p, 0.5, true).unwrap();
            assert_eq!(r.selected_vertices, 1);
            assert_eq!(r.front_vertices, 0);
            assert_eq!(r.changed_vertices, 0);
            assert_eq!(r.proxy_captured_vertices, 1);
            assert_eq!(r.noop, Some(SourceGeometryNoop::NoCoordinateWordChange));
            assert!(next.is_none());
            assert_eq!(serde_json::to_vec(&m).unwrap(), before);
        }
        assert_eq!(s.touched, vec![true, false, false, true, false, false]);
    }
    #[test]
    fn local_plan_matches_frozen_proxy_copy_for_duplicate_and_previously_touched_ids() {
        let mut m = fixture();
        let mut s = SourceGeometryStroke::new(options()).unwrap();
        s.proxy = m.positions.clone();
        s.touched = vec![false; m.positions.len()];
        s.touched[0] = true;
        s.proxy[0].x = -0.0;
        m.positions[0].x = 0.25;
        m.positions[1].z = -0.0;
        m.positions[3].z = 0.125;
        let selected = [3, 0, 3, 1, 0, 1];
        let mut expected = s.proxy.clone();
        let mut flags = s.touched.clone();
        let mut count = 0;
        for &v in &selected {
            if !flags[v] {
                expected[v] = m.positions[v];
                flags[v] = true;
                count += 1;
            }
        }
        let pointers = (s.proxy.as_ptr(), s.touched.as_ptr());
        let plan = FirstTouchProxyPlan::prepare(&s, &m, &selected, true).unwrap();
        assert_eq!(plan.captured_count(), count);
        for &v in &selected {
            assert_eq!(
                plan.point(&s, v, m.positions[v])
                    .to_array()
                    .map(f32::to_bits),
                expected[v].to_array().map(f32::to_bits)
            );
        }
        plan.publish(&mut s, &m.positions);
        assert_eq!(
            proxy_words(&s),
            expected
                .iter()
                .map(|v| v.to_array().map(f32::to_bits))
                .collect::<Vec<_>>()
        );
        assert_eq!(s.touched, flags);
        assert_eq!((s.proxy.as_ptr(), s.touched.as_ptr()), pointers);
    }
    #[test]
    fn cold_and_disabled_plans_retain_original_storage_policy_and_abandonment() {
        let m = fixture();
        let mut s = SourceGeometryStroke::new(options()).unwrap();
        let plan = FirstTouchProxyPlan::prepare(&s, &m, &[3, 0, 3], true).unwrap();
        assert_eq!(plan.captured_count(), 2);
        drop(plan);
        assert!(s.proxy.is_empty() && s.touched.is_empty());
        FirstTouchProxyPlan::prepare(&s, &m, &[3, 0, 3], true)
            .unwrap()
            .publish(&mut s, &m.positions);
        assert_eq!(s.proxy, m.positions);
        assert_eq!(s.touched, vec![true, false, false, true, false, false]);
        let disabled = FirstTouchProxyPlan::prepare(&s, &m, &[3], false).unwrap();
        assert_eq!(disabled.captured_count(), 0);
        assert_eq!(disabled.point(&s, 3, m.positions[3]), m.positions[3]);
        disabled.publish(&mut s, &m.positions);
        assert!(s.proxy.is_empty() && s.touched.is_empty());
    }
}

#[cfg(test)]
mod private_normal_reuse_regressions {
    use super::*;
    fn words(s: &SourceGeometryNormalState) -> serde_json::Value {
        serde_json::json!({"faces":s.faces.as_slice(),"positions":s.position_words,"face_normals":s.face_normals.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),"vertex_normals":s.vertex_normals.iter().map(|v|v.to_array().map(f32::to_bits)).collect::<Vec<_>>(),"tree":s.ordered_tree_snapshot().unwrap(),"generation":s.generation,"center":s.interaction_center.map(f32::to_bits),"original":s.stored_original_faces,"bytes":s.approximate_bytes()})
    }
    fn pointers(s: &SourceGeometryNormalState) -> [usize; 3] {
        [
            s.position_words.as_ptr() as usize,
            s.face_normals.as_ptr() as usize,
            s.vertex_normals.as_ptr() as usize,
        ]
    }
    fn quad() -> Mesh {
        let mut m = Mesh::sphere(0);
        m.positions = vec![
            Vec3::new(-0.0, 0., 0.),
            Vec3::X,
            Vec3::ONE,
            Vec3::Y,
            Vec3::new(0.125, 0.5, -0.25),
        ];
        m.triangles = vec![[0, 1, 2], [0, 2, 3]];
        m.normals = vec![Vec3::Z; 5];
        m.masks = vec![1.; 5];
        m.colors = vec![[0.5; 3]; 5];
        m.surface = Some(crate::SurfaceTopology {
            faces: vec![[0, 1, 2, 3]],
            uv: None,
        });
        m.mask_convention = MaskConvention::UnprotectedF32V1;
        m
    }
    #[test]
    fn private_refresh_preserves_every_copied_field_order_history_and_owned_buffer() {
        for mut mesh in [Mesh::sphere(2), Mesh::cube(), quad()] {
            mesh.mask_convention = MaskConvention::UnprotectedF32V1;
            mesh.masks.fill(1.);
            mesh.recompute_normals();
            let mut shared = SourceGeometryNormalState::seed(&mesh).unwrap();
            let mut private = SourceGeometryNormalState::seed(&mesh).unwrap();
            let buffers = pointers(&private);
            for (step, selected) in [
                vec![0, 1],
                vec![2],
                vec![],
                vec![1, 1, 2],
                vec![mesh.positions.len() - 1],
            ]
            .into_iter()
            .enumerate()
            {
                for (i, p) in mesh.positions.iter_mut().enumerate() {
                    p.z += if i % 2 == 0 { 0.03125 } else { -0.015625 };
                }
                mesh.positions[0].x = if step % 2 == 0 { -0.0 } else { 0.0 };
                let original = words(&shared);
                let expected = NormalStateAccess::Shared(&shared)
                    .advance(&mesh.positions, &selected, true, usize::MAX)
                    .unwrap();
                let actual = NormalStateAccess::Private(&mut private)
                    .advance(&mesh.positions, &selected, true, usize::MAX)
                    .unwrap();
                assert!(actual.next.is_none());
                assert_eq!(actual.cache_changed, expected.cache_changed);
                assert_eq!(pointers(&private), buffers);
                assert_eq!(words(&shared), original);
                shared = expected.next.unwrap();
                assert_eq!(words(&private), words(&shared));
                assert!(private.matches_mesh(&mesh));
            }
        }
    }
    #[test]
    fn controlled_shared_cache_change_matches_full_scan_for_selected_writes() {
        for mut mesh in [Mesh::sphere(2), Mesh::cube(), quad()] {
            mesh.mask_convention = MaskConvention::UnprotectedF32V1;
            mesh.masks.fill(1.);
            mesh.recompute_normals();
            let state = SourceGeometryNormalState::seed(&mesh).unwrap();
            for (selected, delta) in [
                (&[0, 1, 1][..], 0.03125),
                (&[2][..], -0.015625),
                (&[][..], 0.),
            ] {
                let mut positions = mesh.positions.clone();
                for &v in selected {
                    positions[v].z += delta;
                }
                let changed = !selected.is_empty() && delta != 0.;
                let full = NormalStateAccess::Shared(&state)
                    .advance(&positions, selected, changed, usize::MAX)
                    .unwrap();
                let controlled = NormalStateAccess::Shared(&state)
                    .advance_controlled(&positions, selected, changed, usize::MAX)
                    .unwrap();
                assert_eq!(controlled.cache_changed, full.cache_changed);
                assert_eq!(words(&controlled.next.unwrap()), words(&full.next.unwrap()));
            }
        }
    }
    #[test]
    fn preflight_failures_leave_private_buffers_fields_and_generation_literal() {
        for failure in 0..3 {
            let mesh = quad();
            let mut state = SourceGeometryNormalState::seed(&mesh).unwrap();
            let mut positions = mesh.positions.clone();
            positions[0].z += 0.25;
            if failure == 1 {
                positions[0] = Vec3::ZERO;
                positions[1] = Vec3::new(f32::MAX, 0., 0.);
                positions[2] = Vec3::new(0., f32::MAX, 0.);
            }
            if failure == 2 {
                state.generation = u64::MAX;
            }
            let before = words(&state);
            let buffers = pointers(&state);
            let max = if failure == 0 { 0 } else { usize::MAX };
            let error = state
                .advance_private(&positions, &[0, 2], true, max)
                .err()
                .unwrap();
            assert_eq!(
                error,
                if failure == 1 {
                    SourceGeometryError::NonfiniteCandidate
                } else {
                    SourceGeometryError::StorageLimit
                }
            );
            assert_eq!(words(&state), before);
            assert_eq!(pointers(&state), buffers);
        }
    }
    #[test]
    fn no_change_private_refresh_retains_original_generation_and_all_words() {
        let mesh = quad();
        let mut state = SourceGeometryNormalState::seed(&mesh).unwrap();
        let before = words(&state);
        let buffers = pointers(&state);
        for selected in [&[][..], &[0, 0, 4][..]] {
            let advance = state
                .advance_private(&mesh.positions, selected, false, 0)
                .unwrap();
            assert!(!advance.cache_changed);
            assert!(advance.next.is_none());
            assert_eq!(words(&state), before);
            assert_eq!(pointers(&state), buffers);
        }
    }
    #[test]
    fn spare_capacity_and_observed_shared_state_keep_the_copy_path() {
        let mesh = quad();
        let mut private = SourceGeometryNormalState::seed(&mesh).unwrap();
        private.position_words.reserve(8);
        let before = words(&private);
        let buffers = pointers(&private);
        let advance = NormalStateAccess::Private(&mut private)
            .advance(&mesh.positions, &[0], true, usize::MAX)
            .unwrap();
        assert!(advance.next.is_some());
        assert_eq!(words(&private), before);
        assert_eq!(pointers(&private), buffers);
        let mut held = Arc::new(SourceGeometryNormalState::seed(&mesh).unwrap());
        let weak = Arc::downgrade(&held);
        assert!(Arc::get_mut(&mut held).is_none());
        let before = words(&held);
        let advance = NormalStateAccess::Shared(&held)
            .advance(&mesh.positions, &[0], true, usize::MAX)
            .unwrap();
        assert!(advance.next.is_some());
        assert_eq!(words(&held), before);
        assert_eq!(words(&weak.upgrade().unwrap()), before);
    }
    #[test]
    fn controlled_mirror_never_mutates_the_callers_original_producer_or_skips_full_entry_checks() {
        let mut mesh = Mesh::sphere(3);
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.);
        let mut contacts = [None, None];
        for (slot, x) in [0.173, -0.173].into_iter().enumerate() {
            let (face, hit) = mesh
                .raycast_indexed(crate::Ray {
                    origin: Vec3::new(x, 0.137, 4.),
                    direction: -Vec3::Z,
                })
                .unwrap();
            contacts[slot] = Some(SourceGeometryPass {
                center: hit.position.to_array().map(f64::from),
                eye_direction: [0., 0., -1.],
                local_radius_squared: 0.18f64 * 0.18,
                original_face_index: face,
            });
        }
        let original = Arc::new(SourceGeometryNormalState::seed(&mesh).unwrap());
        let original_words = words(&original);
        let mut current = original.clone();
        let mut stroke = SourceGeometryStroke::new(SourceGeometryOptions {
            tool: SourceGeometryTool::Brush {
                clay: false,
                accumulate: false,
            },
            intensity: 0.25,
            prevent_folds: false,
            clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
            ..Default::default()
        })
        .unwrap();
        let controls = crate::SourceGeometryControls {
            mirror: true,
            alpha: None,
        };
        let mut tracks = Default::default();
        for index in 0..3 {
            validation_probe::reset();
            let (report, next) = stroke
                .apply_controlled(
                    &mut mesh,
                    &current,
                    contacts,
                    0.5,
                    true,
                    &controls,
                    tracks,
                    MAX_SOURCE_GEOMETRY_STATE_BYTES,
                )
                .unwrap();
            assert_eq!(validation_probe::counts(), [1, 1, usize::from(index > 0)]);
            assert!(report.combined.changed_vertices > 0);
            tracks = report.tracks;
            current = next.unwrap();
            assert!(!Arc::ptr_eq(&current, &original));
            assert_eq!(words(&original), original_words);
            assert!(current.matches_mesh(&mesh));
        }
    }
}
