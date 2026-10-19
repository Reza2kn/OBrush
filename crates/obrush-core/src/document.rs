use crate::layers::{LayerStack, LayersProject, LayersProjectRef};
use crate::morph::{MorphProject, MorphProjectRef, MorphTarget};
use crate::multires::{MultiresProject, MultiresProjectRef, Multiresolution};
use crate::resource_limits::MAX_PROJECT_BYTES as MAX_INPUT_BYTES;
use crate::scene_v7::SceneV7Project;
use crate::subtools::{
    ObjectBudget, ObjectProject, ObjectState, Scene, SceneProject, SceneProjectRef, Subtool,
};
use crate::{Mesh, SceneObjectMetadata, SglSceneOrigin};
#[path = "source_geometry_document.rs"]
mod source_geometry_document;
pub use source_geometry_document::{
    CertifiedSourceRenderSnapshot, PreparedSourceGeometryDab, SourceGeometryPreparationTime,
    SourceGeometryReleaseTiming,
};
pub(crate) use source_geometry_document::{ValidatedControlledSourceDab, ValidatedSourceDab};
#[path = "source_history.rs"]
mod source_history;
#[path = "source_sgl_add.rs"]
mod source_sgl_add;
use serde::{Deserialize, Serialize};
pub use source_sgl_add::SglAppendReport;
use std::collections::VecDeque;
use std::sync::Arc;
use web_time::Instant;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MeshReplaceTiming {
    pub validate_ms: f64,
    pub normals_ms: f64,
    pub scene_ms: f64,
    pub history_ms: f64,
    pub witness_ms: f64,
    pub sparse_normals: bool,
}

const PROJECT_VERSION: u32 = 2;
const LAYERS_PROJECT_VERSION: u32 = 3;
const MORPH_PROJECT_VERSION: u32 = 4;
const SCENE_PROJECT_VERSION: u32 = 5;
const ATTRIBUTES_PROJECT_VERSION: u32 = 6;
const MASK_CONVENTION_PROJECT_VERSION: u32 = 7;
const HISTORY_BUDGET_BYTES: usize = crate::resource_limits::MAX_HISTORY_BYTES;
const MIN_HISTORY_ENTRIES: usize = 3;
const MAX_HISTORY_ENTRIES: usize = 50;
const DEFAULT_HISTORY_ENTRIES: usize = 15;
const MAX_NAME_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum MorphTargetDeletionDisposition {
    Absent,
    Preserved {
        creation_level: usize,
    },
    Rebased {
        previous_level: usize,
        creation_level: usize,
    },
    Dropped {
        previous_level: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SubdivisionDeletionReport {
    pub previous_levels: usize,
    pub retained_levels: usize,
    pub previous_active_level: usize,
    pub active_level: usize,
    pub removed_vertices: usize,
    pub removed_triangles: usize,
    pub morph_target: MorphTargetDeletionDisposition,
}

/// One fully parsed Source File Add part. All parts are admitted together and
/// become one document history event.
pub struct ImportedSubtool {
    pub mesh: Mesh,
    pub name: String,
    pub preserve_normals: bool,
    pub metadata: SceneObjectMetadata,
}

#[derive(Debug)]
pub struct Document {
    /// Breaking v7 API: absence is a real empty document, never placeholder geometry.
    pub mesh: Option<Mesh>,
    pub name: String,
    multiresolution: Option<Multiresolution>,
    layers: Option<LayerStack>,
    morph_target: Option<MorphTarget>,
    scene: Option<Scene>,
    active_metadata: Option<SceneObjectMetadata>,
    scene_origin: Option<SglSceneOrigin>,
    layer_error: Option<String>,
    undo: VecDeque<DocumentState>,
    redo: VecDeque<DocumentState>,
    history_limit: usize,
    stroke_start: Option<DocumentState>,
    material_paint_stroke: bool,
    source_geometry_token: u64,
    source_geometry_active: Option<u64>,
    source_geometry_states: Vec<(u32, Arc<crate::SourceGeometryNormalState>)>,
}

/// Complete editable state, separate from history itself. Future scene fields
/// belong here so replacing an object does not produce partial undo snapshots.
#[derive(Debug)]
struct DocumentState {
    mesh: Option<Mesh>,
    name: String,
    multiresolution: Option<Multiresolution>,
    layers: Option<LayerStack>,
    morph_target: Option<MorphTarget>,
    scene: Option<Scene>,
    active_metadata: Option<SceneObjectMetadata>,
    scene_origin: Option<SglSceneOrigin>,
    source_geometry_states: Vec<(u32, Arc<crate::SourceGeometryNormalState>)>,
    source_history: Option<source_history::SourceHistoryEvent>,
}

#[path = "source_paint_lock.rs"]
mod source_paint_lock;
pub use source_paint_lock::{
    SourcePaintLockedAction, SourcePaintLockedMode, SourcePaintLockedReport,
    SourcePaintLockedSample, SourcePaintLockedStroke,
};

impl DocumentState {
    fn matches(&self, document: &Document) -> bool {
        (match (&self.mesh, &document.mesh) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.same_surface(b)
                    && ((self.layers.is_none()
                        && document.layers.is_none()
                        && self.morph_target.is_none()
                        && document.morph_target.is_none()
                        && self.scene.is_none()
                        && document.scene.is_none())
                        || mesh_channels_equal_bits(a, b))
            }
            _ => false,
        }) && crate::scene_v7::optional_metadata_equal(
            self.active_metadata.as_ref(),
            document.active_metadata.as_ref(),
        ) && match (&self.scene_origin, &document.scene_origin) {
            (None, None) => true,
            (Some(a), Some(b)) => crate::scene_v7::header_bits_equal(a, b),
            _ => false,
        } && self.name == document.name
            && match (&self.layers, &document.layers) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_state(b),
                _ => false,
            }
            && match (&self.morph_target, &document.morph_target) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_state(b),
                _ => false,
            }
            && match (&self.scene, &document.scene) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_state(b),
                _ => false,
            }
            && match (&self.multiresolution, &document.multiresolution) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_surface(b),
                _ => false,
            }
    }

    fn approximate_bytes(&self) -> usize {
        [
            std::mem::size_of::<Self>(),
            self.mesh.as_ref().map_or(0, Mesh::approximate_bytes),
            self.active_metadata
                .as_ref()
                .map_or(0, |_| std::mem::size_of::<SceneObjectMetadata>()),
            self.name.capacity(),
            self.multiresolution
                .as_ref()
                .map_or(0, Multiresolution::approximate_bytes),
            self.layers
                .as_ref()
                .map_or(0, LayerStack::approximate_bytes),
            self.morph_target
                .as_ref()
                .map_or(0, MorphTarget::approximate_bytes),
            self.scene.as_ref().map_or(0, Scene::approximate_bytes),
            state_origin_storage(self.active_metadata.as_ref(), self.scene.as_ref()),
            source_geometry_document::retained_bytes(
                &self.source_geometry_states,
                self.source_geometry_states.capacity(),
            ),
            self.source_history
                .as_ref()
                .map_or(0, source_history::SourceHistoryEvent::bytes),
        ]
        .into_iter()
        .fold(0, usize::saturating_add)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Project {
    version: u32,
    name: String,
    #[serde(default)]
    mesh: Option<Mesh>,
    #[serde(default)]
    multiresolution: Option<MultiresProject>,
    #[serde(default)]
    layers: Option<LayersProject>,
    #[serde(default)]
    morph_target: Option<MorphProject>,
    #[serde(default)]
    scene: Option<SceneProject>,
    #[serde(default)]
    scene_v7: Option<SceneV7Project>,
}

#[derive(Serialize)]
struct ProjectRef<'a> {
    version: u32,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    mesh: Option<&'a Mesh>,
    #[serde(skip_serializing_if = "Option::is_none")]
    multiresolution: Option<MultiresProjectRef<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    layers: Option<LayersProjectRef<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    morph_target: Option<MorphProjectRef<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scene: Option<SceneProjectRef<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scene_v7: Option<&'a SceneV7Project>,
}

/// Limit output while serializing rather than allocating an oversized JSON string
/// and rejecting it afterward. This also avoids cloning the full mesh to save it.
struct LimitedJsonWriter {
    bytes: Vec<u8>,
    limit: usize,
}

struct OutputCounter {
    bytes: usize,
}
impl std::io::Write for OutputCounter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("Project output-size overflow"))?;
        if self.bytes > MAX_INPUT_BYTES {
            return Err(std::io::Error::other(
                "Project exceeds the encoded project output limit",
            ));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Small projects retain geometric growth. Above this point, cap spare output
// storage at one chunk instead of doubling a large contiguous JSON allocation.
const JSON_OUTPUT_GROWTH_BYTES: usize = 8 * 1024 * 1024;

impl std::io::Write for LimitedJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let required = self.bytes.len().checked_add(bytes.len()).ok_or_else(|| {
            std::io::Error::other("Project exceeds the encoded project output limit")
        })?;
        if required > self.limit {
            return Err(std::io::Error::other(
                "Project exceeds the encoded project output limit",
            ));
        }
        if required > self.bytes.capacity() {
            let growth = self.bytes.capacity().clamp(8, JSON_OUTPUT_GROWTH_BYTES);
            let desired = self
                .bytes
                .capacity()
                .saturating_add(growth)
                .max(required)
                .min(self.limit);
            self.bytes
                .try_reserve_exact(desired - self.bytes.len())
                .map_err(|_| std::io::Error::other("Cannot allocate project output"))?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

macro_rules! active_mesh_ref {
    ($doc:expr) => {
        $doc.mesh.as_ref().ok_or("Document has no active mesh")?
    };
}
macro_rules! active_mesh_mut {
    ($doc:expr) => {
        $doc.mesh.as_mut().ok_or("Document has no active mesh")?
    };
}

impl Document {
    pub fn new(mesh: Mesh) -> Self {
        Self {
            mesh: Some(mesh),
            name: "Untitled sculpture".into(),
            multiresolution: None,
            layers: None,
            morph_target: None,
            scene: None,
            active_metadata: None,
            scene_origin: None,
            layer_error: None,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            history_limit: DEFAULT_HISTORY_ENTRIES,
            stroke_start: None,
            material_paint_stroke: false,
            source_geometry_token: 0,
            source_geometry_active: None,
            source_geometry_states: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self {
            mesh: None,
            name: "Untitled sculpture".into(),
            multiresolution: None,
            layers: None,
            morph_target: None,
            scene: Some(Scene::empty()),
            active_metadata: None,
            scene_origin: None,
            layer_error: None,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            history_limit: DEFAULT_HISTORY_ENTRIES,
            stroke_start: None,
            material_paint_stroke: false,
            source_geometry_token: 0,
            source_geometry_active: None,
            source_geometry_states: Vec::new(),
        }
    }
    pub fn active_mesh(&self) -> Option<&Mesh> {
        self.mesh.as_ref()
    }
    pub fn active_mesh_mut(&mut self) -> Option<&mut Mesh> {
        self.mesh.as_mut()
    }
    pub fn require_active_mesh(&self) -> Result<&Mesh, String> {
        self.mesh
            .as_ref()
            .ok_or_else(|| "Document has no active mesh".into())
    }
    pub fn require_active_mesh_mut(&mut self) -> Result<&mut Mesh, String> {
        self.mesh
            .as_mut()
            .ok_or_else(|| "Document has no active mesh".into())
    }
    pub fn is_empty(&self) -> bool {
        self.mesh.is_none()
    }

    /// Repeated calls while a stroke is open preserve its original snapshot.
    pub fn begin_stroke(&mut self) {
        if self.stroke_start.is_none()
            && self
                .mesh
                .as_ref()
                .is_some_and(|mesh| mesh.validate().is_ok())
        {
            self.layer_error = None;
            self.material_paint_stroke = false;
            self.stroke_start = Some(self.snapshot_state());
        }
    }

    /// Begin an ordinary whole-state stroke with permission to establish the
    /// implicit material defaults in an existing subdivision hierarchy. This
    /// captures history only; no channel storage is allocated until an accepted
    /// material change is committed. Generic attribute installation and ordinary
    /// strokes retain their material-presence restrictions.
    pub fn begin_material_paint_stroke(&mut self) {
        self.begin_stroke();
        if self.stroke_start.is_some() {
            self.material_paint_stroke = true;
        }
    }

    /// Empty strokes do not consume history or discard an available redo.
    pub fn end_stroke(&mut self) {
        if self.prepare_retained_source_paint_end().is_err() {
            return;
        }
        self.source_geometry_active = None;
        self.prune_source_geometry_states();
        let material_paint = std::mem::take(&mut self.material_paint_stroke);
        let Some(previous) = self.stroke_start.take() else {
            return;
        };
        if self.mesh.is_none() {
            self.exchange_state(previous);
            self.layer_error = Some(
                "Direct active-mesh removal is invalid; use clear_scene/delete_subtool".into(),
            );
            return;
        }
        if material_paint
            && previous
                .mesh
                .as_ref()
                .is_some_and(|m| m.materials.is_none())
            && self.mesh.as_ref().is_some_and(|m| m.materials.is_some())
        {
            let prepared = (|| -> Result<Option<Multiresolution>, String> {
                let mesh = self.require_active_mesh()?;
                mesh.validate()?;
                if mesh.materials.as_ref().is_some_and(|values| {
                    values.iter().all(|v| *v == crate::DEFAULT_VERTEX_MATERIAL)
                }) {
                    return Ok(None);
                }
                self.multiresolution
                    .as_ref()
                    .map(|stack| stack.with_material_paint_defaults(mesh))
                    .transpose()
            })();
            match prepared {
                Ok(stack) => {
                    if let Some(mesh) = self.mesh.as_mut() {
                        if mesh.materials.as_ref().is_some_and(|values| {
                            values.iter().all(|v| *v == crate::DEFAULT_VERTEX_MATERIAL)
                        }) {
                            mesh.materials = None;
                        }
                    }
                    if let Some(stack) = stack {
                        self.multiresolution = Some(stack);
                    }
                }
                Err(error) => {
                    self.exchange_state(previous);
                    self.layer_error = Some(error);
                    return;
                }
            }
        }
        let Some(mesh) = self.mesh.as_ref() else {
            return;
        };
        // A singleton is still a retained correspondence and scheme. Raw
        // public Mesh edits cannot detach it implicitly during a brush stroke.
        if self
            .multiresolution
            .as_ref()
            .is_some_and(|stack| stack.levels.len() == 1 && !stack.active_topology_matches(mesh))
        {
            self.exchange_state(previous);
            self.layer_error = Some(
                "Adaptive topology editing is disabled while a subdivision hierarchy is retained"
                    .into(),
            );
            return;
        }
        let mut active_mesh_and_pose_validated = false;
        if state_has_attributes(mesh, self.multiresolution.as_ref(), self.scene.as_ref())
            || previous.mesh.as_ref().is_some_and(|old| {
                state_has_attributes(
                    old,
                    previous.multiresolution.as_ref(),
                    previous.scene.as_ref(),
                )
            })
        {
            let checked = mesh.validate().and_then(|()| {
                let stack = self
                    .multiresolution
                    .as_ref()
                    .map(|s| s.reconciled(mesh))
                    .transpose()?;
                self.check_scene_candidate_inner(
                    mesh,
                    stack.as_ref(),
                    self.layers.as_ref(),
                    self.morph_target.as_ref(),
                    self.scene.as_ref(),
                    (true, false, None),
                )?;
                Self::check_output_budget(
                    mesh,
                    &self.name,
                    stack.as_ref(),
                    self.layers.as_ref(),
                    self.morph_target.as_ref(),
                )
            });
            if let Err(error) = checked {
                self.exchange_state(previous);
                self.layer_error = Some(error);
                return;
            }
            // No active Mesh or pose write occurs after this preflight. Keep
            // its proof only inside this one mutable Document call; public
            // entry points still validate their own borrowed candidate.
            active_mesh_and_pose_validated = true;
        }
        if previous.matches(self) {
            if !active_mesh_and_pose_validated
                && (self.layers.is_some() || self.morph_target.is_some() || self.scene.is_some())
            {
                if let Err(error) = mesh.validate() {
                    self.exchange_state(previous);
                    self.layer_error = Some(error);
                }
            }
            return;
        }
        if self.morph_target.is_some() {
            if let Err(error) = crate::morph::validate_source(mesh) {
                self.exchange_state(previous);
                self.layer_error = Some(error);
                return;
            }
        }
        if self.scene.is_some() && !active_mesh_and_pose_validated {
            if let Err(error) = mesh.validate() {
                self.exchange_state(previous);
                self.layer_error = Some(error);
                return;
            }
        }
        if let Some(layers) = &self.layers {
            let reconciled = layers.reconciled(mesh).and_then(|candidate| {
                let stack = self
                    .multiresolution
                    .as_ref()
                    .map(|s| s.reconciled(mesh))
                    .transpose()?;
                Self::check_output_budget(
                    mesh,
                    &self.name,
                    stack.as_ref(),
                    Some(&candidate),
                    self.morph_target.as_ref(),
                )?;
                Ok(candidate)
            });
            match reconciled {
                Ok(candidate) => self.layers = Some(candidate),
                Err(error) => {
                    self.exchange_state(previous);
                    self.layer_error = Some(error);
                    return;
                }
            }
        }
        // Topology-changing operations cannot retain an unrelated hierarchy.
        // The old whole-state snapshot still restores it through undo.
        if self
            .multiresolution
            .as_ref()
            .is_some_and(|stack| !stack.active_topology_matches(mesh))
        {
            self.multiresolution = None;
        }
        // Adaptive public-mesh edits may change the original correspondence.
        // A target at another retained level survives ordinary level edits.
        if self.morph_target.as_ref().is_some_and(|target| {
            Self::validate_morph_binding(target, mesh, self.multiresolution.as_ref()).is_err()
        }) {
            self.morph_target = None;
        }
        if self.morph_target.is_some() && self.layers.is_none() {
            let valid = self
                .multiresolution
                .as_ref()
                .map(|s| s.reconciled(mesh))
                .transpose()
                .and_then(|stack| {
                    Self::check_output_budget(
                        mesh,
                        &self.name,
                        stack.as_ref(),
                        None,
                        self.morph_target.as_ref(),
                    )
                });
            if let Err(error) = valid {
                self.exchange_state(previous);
                self.layer_error = Some(error);
                return;
            }
        }
        if self.scene.is_some() {
            if let Err(error) = self.check_scene_candidate_inner(
                mesh,
                self.multiresolution.as_ref(),
                self.layers.as_ref(),
                self.morph_target.as_ref(),
                self.scene.as_ref(),
                (true, active_mesh_and_pose_validated, None), // mesh passed preflight or the scene check above
            ) {
                self.exchange_state(previous);
                self.layer_error = Some(error);
                return;
            }
            active_mesh_and_pose_validated = true;
        }
        if let Err(error) =
            self.validate_scene_metadata_with_admitted_active(active_mesh_and_pose_validated)
        {
            self.exchange_state(previous);
            self.layer_error = Some(error);
            return;
        }
        if !previous.matches(self) {
            self.undo.push_back(previous);
            self.redo.clear();
            self.trim_history();
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
            || self
                .stroke_start
                .as_ref()
                .is_some_and(|previous| !previous.matches(self))
    }

    /// Session-level history length. The separate byte budget can retain fewer
    /// entries for a large document; neither limit is project content.
    pub fn history_limit(&self) -> usize {
        self.history_limit
    }

    pub fn history_depths(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }

    pub fn set_history_limit(&mut self, limit: usize) -> Result<(), String> {
        if !(MIN_HISTORY_ENTRIES..=MAX_HISTORY_ENTRIES).contains(&limit) {
            return Err("History length must be between 3 and 50".into());
        }
        self.history_limit = limit;
        // SculptGL keeps the nearest past states before any redo future when
        // lowering its stack. Discard the oldest Undo snapshots and then the
        // most distant Redo snapshots (front of our redo deque). This also
        // avoids evicting a reachable Undo merely because Redo is longer.
        while self.undo.len() > limit {
            self.undo.pop_front();
        }
        while self.undo.len() + self.redo.len() > limit {
            self.redo.pop_front();
        }
        self.trim_history();
        Ok(())
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
            && self
                .stroke_start
                .as_ref()
                .is_none_or(|previous| previous.matches(self))
    }

    pub fn undo(&mut self) -> bool {
        self.undo_with_source_history_limit(crate::MAX_SOURCE_GEOMETRY_STATE_BYTES)
    }
    fn undo_with_source_history_limit(&mut self, max_cache_bytes: usize) -> bool {
        self.end_stroke();
        let replacement = match self
            .undo
            .back()
            .map(|target| self.prepare_source_history(target, max_cache_bytes))
            .transpose()
        {
            Ok(replacement) => replacement.flatten(),
            Err(error) => {
                self.layer_error = Some(format!(
                    "Source undo rejected before history exchange: {error}"
                ));
                return false;
            }
        };
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        let current = self.exchange_state(previous);
        if let Some(rows) = replacement {
            self.source_geometry_states = rows;
        }
        self.redo.push_back(current);
        self.trim_history();
        true
    }

    pub fn redo(&mut self) -> bool {
        self.redo_with_source_history_limit(crate::MAX_SOURCE_GEOMETRY_STATE_BYTES)
    }
    fn redo_with_source_history_limit(&mut self, max_cache_bytes: usize) -> bool {
        self.end_stroke();
        let replacement = match self
            .redo
            .back()
            .map(|target| self.prepare_source_history(target, max_cache_bytes))
            .transpose()
        {
            Ok(replacement) => replacement.flatten(),
            Err(error) => {
                self.layer_error = Some(format!(
                    "Source redo rejected before history exchange: {error}"
                ));
                return false;
            }
        };
        let Some(next) = self.redo.pop_back() else {
            return false;
        };
        let current = self.exchange_state(next);
        if let Some(rows) = replacement {
            self.source_geometry_states = rows;
        }
        self.undo.push_back(current);
        self.trim_history();
        true
    }

    /// Change only surface/material channels, with a complete undo snapshot.
    /// Stored geometry levels retain their bindings; unsupported presence/UV changes reject atomically.
    pub fn install_mesh_attributes(
        &mut self,
        materials: Option<Vec<[f32; 2]>>,
        surface: Option<crate::SurfaceTopology>,
    ) -> Result<(), String> {
        let mut candidate = active_mesh_ref!(self).snapshot();
        candidate.install_attributes(materials, surface)?;
        candidate.validate()?;
        let stack = self
            .multiresolution
            .as_ref()
            .map(|s| s.reconciled(&candidate))
            .transpose()?;
        self.check_scene_candidate(
            &candidate,
            stack.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        Self::check_output_budget(
            &candidate,
            &self.name,
            stack.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
        )?;
        self.finish_layer_stroke()?;
        if active_mesh_ref!(self).attributes_equal_bits(&candidate) {
            return Ok(());
        }
        let previous = self.snapshot_state();
        self.mesh = Some(candidate);
        self.multiresolution = stack;
        self.commit_previous(previous);
        Ok(())
    }

    /// Deliberate, undoable removal of original face/UV metadata; materials remain.
    pub fn detach_surface_topology(&mut self) -> Result<(), String> {
        if self.multiresolution.is_some() {
            return Err("Bake retained levels before detaching original face/UV metadata".into());
        }
        self.install_mesh_attributes(active_mesh_ref!(self).materials.clone(), None)
    }

    pub fn replace_mesh(&mut self, mesh: Mesh) -> Result<(), String> {
        self.replace_mesh_inner(mesh, None, None, None)
    }

    /// The resident caller may request sparse normal refresh only after its
    /// source mesh matches an exact canonical-normal witness. Other callers
    /// always take the full normal rebuild in `replace_mesh`.
    pub(crate) fn replace_mesh_timed(
        &mut self,
        mesh: Mesh,
        resident_changed_vertices: &[usize],
        sparse_normals: bool,
    ) -> Result<MeshReplaceTiming, String> {
        let mut timing = MeshReplaceTiming::default();
        self.replace_mesh_inner(
            mesh,
            sparse_normals.then_some(resident_changed_vertices),
            Some(resident_changed_vertices),
            Some(&mut timing),
        )?;
        Ok(timing)
    }

    fn replace_mesh_inner(
        &mut self,
        mut mesh: Mesh,
        resident_dirty_vertices: Option<&[usize]>,
        resident_checked_vertices: Option<&[usize]>,
        mut timing: Option<&mut MeshReplaceTiming>,
    ) -> Result<(), String> {
        let started = Instant::now();
        self.require_baked_layers()?;
        if resident_checked_vertices.is_some() {
            mesh.validate_resident_sparse_replacement()?;
        } else {
            mesh.validate()?;
        }
        if let Some(timing) = &mut timing {
            timing.validate_ms = started.elapsed().as_secs_f64() * 1000.0;
        }
        let started = Instant::now();
        if let Some(dirty) = resident_dirty_vertices {
            mesh.recompute_normals_near_vertices(dirty);
        } else {
            mesh.recompute_normals();
        }
        if let Some(timing) = &mut timing {
            timing.normals_ms = started.elapsed().as_secs_f64() * 1000.0;
            timing.sparse_normals = resident_dirty_vertices.is_some();
        }
        if let Some(metadata) = &self.active_metadata {
            if let Some(changed) = resident_checked_vertices {
                metadata.validate_resident_sparse_positions(&mesh, changed)?;
            } else {
                metadata.validate(&mesh)?;
            }
        }
        self.finish_layer_stroke()?;
        let scene = self.scene.clone();
        if self.mesh.is_none() {
            return self
                .add_subtool_preserving_normals(mesh, "Sculpture")
                .map(|_| ());
        }
        let started = Instant::now();
        // This candidate and its active pose were validated above, before
        // `finish_layer_stroke`. Recheck scene structure and aggregate limits,
        // but do not rescan the same million vertices and source origin.
        self.check_scene_candidate_inner(
            &mesh,
            None,
            None,
            None,
            scene.as_ref(),
            (true, true, None),
        )?;
        if let Some(timing) = &mut timing {
            timing.scene_ms = started.elapsed().as_secs_f64() * 1000.0;
        }
        if timing.is_some() {
            let started = Instant::now();
            mesh.refresh_resident_normal_witness();
            if let Some(timing) = &mut timing {
                timing.witness_ms = started.elapsed().as_secs_f64() * 1000.0;
            }
        }
        let started = Instant::now();
        let previous = self.snapshot_state();
        self.mesh = Some(mesh);
        self.multiresolution = None;
        self.layers = None;
        self.morph_target = None;
        self.scene = scene;
        self.commit_previous(previous);
        if let Some(timing) = &mut timing {
            timing.history_ms = started.elapsed().as_secs_f64() * 1000.0;
        }
        Ok(())
    }

    /// A returned resident GPU candidate must not silently end another edit or
    /// discard retained geometry state through `replace_mesh`.
    pub(crate) fn resident_position_commit_ready(&self) -> Result<(), String> {
        if self.stroke_start.is_some()
            || self.material_paint_stroke
            || self.source_geometry_active.is_some()
        {
            return Err("Finish the active stroke before accepting resident positions".into());
        }
        if self.multiresolution.is_some() || self.layers.is_some() || self.morph_target.is_some() {
            return Err(
                "Resident position commit requires baked levels, layers and morph target".into(),
            );
        }
        Ok(())
    }

    /// Validated whole-document replacement keeps the receiving document's
    /// history; imported history is never carried across the file boundary.
    pub fn replace_document(&mut self, mut imported: Document) -> Result<(), String> {
        imported.finish_layer_stroke()?;
        imported.validate_complete_state()?;
        imported.to_json()?;
        if self.snapshot_state().approximate_bytes() > HISTORY_BUDGET_BYTES {
            return Err(
                "Replacement cannot retain the complete original scene within the history budget"
                    .into(),
            );
        }
        self.finish_layer_stroke()?;
        let previous = self.snapshot_state();
        let replacement = imported.snapshot_state();
        self.exchange_state(replacement);
        self.commit_previous(previous);
        Ok(())
    }

    pub fn current_level(&self) -> usize {
        self.multiresolution
            .as_ref()
            .map_or(0, |stack| stack.active)
    }

    pub fn level_count(&self) -> usize {
        if self.mesh.is_none() {
            0
        } else {
            self.multiresolution.as_ref().map_or(1, |s| s.levels.len())
        }
    }

    pub fn has_retained_subdivision_hierarchy(&self) -> bool {
        self.multiresolution.is_some()
    }

    pub fn supports_adaptive_detail(&self) -> bool {
        !self.has_retained_subdivision_hierarchy()
            && !self.layer_topology_locked()
            && !self.has_morph_target()
            && self
                .mesh
                .as_ref()
                .is_some_and(Mesh::supports_adaptive_detail)
    }

    pub fn delete_lower_subdivision_levels(&mut self) -> Result<SubdivisionDeletionReport, String> {
        self.delete_subdivision_levels(true)
    }

    pub fn delete_higher_subdivision_levels(
        &mut self,
    ) -> Result<SubdivisionDeletionReport, String> {
        self.delete_subdivision_levels(false)
    }

    fn delete_subdivision_levels(
        &mut self,
        lower: bool,
    ) -> Result<SubdivisionDeletionReport, String> {
        let previous_levels = self.level_count();
        let previous_active_level = self.current_level();
        let disposition =
            self.morph_target
                .as_ref()
                .map_or(MorphTargetDeletionDisposition::Absent, |target| {
                    MorphTargetDeletionDisposition::Preserved {
                        creation_level: target.creation_level,
                    }
                });
        let mut report = SubdivisionDeletionReport {
            previous_levels,
            retained_levels: previous_levels,
            previous_active_level,
            active_level: previous_active_level,
            removed_vertices: 0,
            removed_triangles: 0,
            morph_target: disposition,
        };
        let Some(current) = &self.multiresolution else {
            return Ok(report);
        };
        let start = if lower { current.active } else { 0 };
        let end = if lower {
            current.levels.len()
        } else {
            current.active + 1
        };
        // In particular, do not end an open stroke or clear redo for a no-op.
        if start == 0 && end == current.levels.len() {
            return Ok(report);
        }
        // A useful deletion is a separate operation after the pending stroke.
        // Finishing first establishes accepted material defaults in retained
        // levels before their correspondence is reconciled. No-op deletion
        // above deliberately leaves an open stroke and its redo untouched.
        self.finish_layer_stroke()?;
        let current = self.multiresolution.as_ref().ok_or_else(|| {
            "The pending stroke no longer has a retained subdivision hierarchy".to_string()
        })?;
        let mut stack = current.reconciled(active_mesh_ref!(self))?;
        if let Some(layers) = &self.layers {
            layers.reconciled(active_mesh_ref!(self))?;
        }
        if let Some(target) = &self.morph_target {
            Self::validate_morph_binding(target, active_mesh_ref!(self), Some(&stack))?;
        }
        for level in stack.levels[..start].iter().chain(&stack.levels[end..]) {
            report.removed_vertices += level.positions.len();
            report.removed_triangles += level.triangles.len();
        }
        let mut target = self.morph_target.clone();
        if let Some(bound) = &mut target {
            if !(start..end).contains(&bound.creation_level) {
                report.morph_target = MorphTargetDeletionDisposition::Dropped {
                    previous_level: bound.creation_level,
                };
                target = None;
            } else if start != 0 {
                let previous_level = bound.creation_level;
                bound.creation_level -= start;
                report.morph_target = MorphTargetDeletionDisposition::Rebased {
                    previous_level,
                    creation_level: bound.creation_level,
                };
            }
        }
        stack.retain_range(start, end)?;
        if let Some(target) = &target {
            Self::validate_morph_binding(target, active_mesh_ref!(self), Some(&stack))?;
        }
        report.retained_levels = stack.levels.len();
        report.active_level = stack.active;
        Self::check_output_budget(
            active_mesh_ref!(self),
            &self.name,
            Some(&stack),
            self.layers.as_ref(),
            target.as_ref(),
        )?;
        self.check_scene_candidate(
            active_mesh_ref!(self),
            Some(&stack),
            self.layers.as_ref(),
            target.as_ref(),
            self.scene.as_ref(),
        )?;
        // A deletion must not immediately evict its own undo entry. Account
        // for the complete original scene without allocating a second copy.
        let undo_bytes = std::mem::size_of::<DocumentState>()
            + self.mesh.as_ref().map_or(0, Mesh::approximate_bytes)
            + self.name.capacity()
            + self
                .multiresolution
                .as_ref()
                .map_or(0, Multiresolution::approximate_bytes)
            + self
                .layers
                .as_ref()
                .map_or(0, LayerStack::approximate_bytes)
            + self
                .morph_target
                .as_ref()
                .map_or(0, MorphTarget::approximate_bytes)
            + self.scene.as_ref().map_or(0, Scene::approximate_bytes)
            + self.source_geometry_retained_bytes();
        if undo_bytes > HISTORY_BUDGET_BYTES {
            return Err(
                "Subdivision deletion cannot retain its complete undo state within the history budget".into(),
            );
        }
        let previous = self.snapshot_state();
        self.multiresolution = Some(stack);
        self.morph_target = target;
        self.commit_previous(previous);
        Ok(report)
    }

    /// World AABB midpoint of the selected object's active surface.
    pub fn active_subtool_center(&self) -> Option<crate::Vec3> {
        let mesh = self.mesh.as_ref()?;
        let first = mesh.positions.first()?;
        let (mut minimum, mut maximum) = (first.as_dvec3(), first.as_dvec3());
        for point in &mesh.positions {
            if !point.is_finite() {
                return None;
            }
            minimum = minimum.min(point.as_dvec3());
            maximum = maximum.max(point.as_dvec3());
        }
        let center = (minimum + maximum) * 0.5;
        match &self.active_metadata {
            None => Some(center.as_vec3()),
            Some(m) => m
                .affine
                .world_point(crate::Vec3::from_array(m.affine.interaction_center))
                .ok()
                .map(|p| p.as_vec3()),
        }
    }

    /// Mean world center of the current selection, including posed objects.
    /// This is the shared pivot for a selected-group numeric transform.
    pub fn selected_subtools_center(&self) -> Option<crate::Vec3> {
        let selected = self.selected_subtool_ids();
        if selected.is_empty() {
            return None;
        }
        if selected.len() == 1 {
            return self.active_subtool_center();
        }
        let scene = self.scene.as_ref()?;
        let mut total = glam::DVec3::ZERO;
        for id in &selected {
            let object = scene.objects.iter().find(|object| object.id == *id)?;
            let (mesh, metadata) = match object.payload.as_ref() {
                Some(payload) => (payload.mesh.as_ref(), payload.metadata.as_ref()),
                None if Some(*id) == scene.active_id => {
                    (self.mesh.as_ref()?, self.active_metadata.as_ref())
                }
                None => return None,
            };
            let first = mesh.positions.first()?.as_dvec3();
            let (mut min, mut max) = (first, first);
            for point in &mesh.positions {
                if !point.is_finite() {
                    return None;
                }
                min = min.min(point.as_dvec3());
                max = max.max(point.as_dvec3());
            }
            let center = if let Some(metadata) = metadata {
                metadata
                    .affine
                    .world_point(crate::Vec3::from_array(metadata.affine.interaction_center))
                    .ok()?
            } else {
                (min + max) * 0.5
            };
            total += center;
        }
        let mean = total / selected.len() as f64;
        mean.is_finite().then(|| mean.as_vec3())
    }

    /// Apply one world transform to each selected object as one atomic history
    /// entry. All validation and per-object edits run on a detached candidate;
    /// a failed member leaves the document and its Undo/Redo stacks untouched.
    pub fn transform_selected_subtools(
        &mut self,
        transform: &crate::ObjectTransform,
    ) -> Result<bool, String> {
        self.edit_selected_subtools(|candidate| candidate.transform_active_subtool(transform))
    }

    /// Reference-style edit-matrix commit: each selected vertex moves toward
    /// its fully transformed world position by its unprotected mask fraction.
    /// Imported affine poses stay attached while points edit in local space.
    pub fn transform_selected_subtools_masked(
        &mut self,
        transform: &crate::ObjectTransform,
    ) -> Result<bool, String> {
        let affine = crate::transform::Affine::new(transform)?;
        self.edit_selected_subtools(|candidate| candidate.transform_active_subtool_masked(&affine))
    }

    /// Scale selected parts independently along world X/Y/Z around one world
    /// pivot. The source gizmo commits through mask weights even for axis scale;
    /// callers can explicitly choose the earlier whole-part metadata path.
    pub fn scale_selected_subtools_axis(
        &mut self,
        scale: crate::Vec3,
        pivot: crate::Vec3,
        respect_masks: bool,
    ) -> Result<bool, String> {
        let affine = crate::transform::Affine::axis_scale(scale, pivot)?;
        self.edit_selected_subtools(|candidate| {
            if respect_masks {
                candidate.transform_active_subtool_masked(&affine)
            } else {
                candidate.apply_active_affine(&affine)
            }
        })
    }

    /// Translation convenience path retained for existing callers.
    pub fn translate_selected_subtools_masked(
        &mut self,
        translation: crate::Vec3,
    ) -> Result<bool, String> {
        self.transform_selected_subtools_masked(&crate::ObjectTransform {
            translation,
            ..Default::default()
        })
    }

    fn edit_selected_subtools(
        &mut self,
        mut edit: impl FnMut(&mut Self) -> Result<bool, String>,
    ) -> Result<bool, String> {
        self.finish_layer_stroke()?;
        let selected = self.selected_subtool_ids();
        let active = self.active_subtool_id();
        if selected.is_empty() {
            return Err("Scene has no selected objects".into());
        }
        let state = self.snapshot_state();
        let mut candidate = Self {
            mesh: state.mesh,
            name: state.name,
            multiresolution: state.multiresolution,
            layers: state.layers,
            morph_target: state.morph_target,
            scene: state.scene,
            active_metadata: state.active_metadata,
            scene_origin: state.scene_origin,
            layer_error: self.layer_error.clone(),
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            history_limit: self.history_limit,
            stroke_start: None,
            material_paint_stroke: false,
            source_geometry_token: self.source_geometry_token,
            source_geometry_active: None,
            source_geometry_states: state.source_geometry_states,
        };
        let mut changed = false;
        for id in &selected {
            candidate.select_subtool(*id)?;
            changed |= edit(&mut candidate)?;
        }
        if !changed {
            return Ok(false);
        }
        candidate.select_subtools(&selected, active)?;
        candidate.to_json()?;
        let previous = self.snapshot_state();
        self.exchange_state(candidate.snapshot_state());
        self.commit_previous(previous);
        Ok(true)
    }

    fn transform_active_subtool_masked(
        &mut self,
        affine: &crate::transform::Affine,
    ) -> Result<bool, String> {
        if affine.identity {
            return Ok(false);
        }
        let source = self.require_active_mesh()?;
        source.validate()?;
        let weights = source
            .masks
            .iter()
            .map(|&raw| source.mask_convention.influence_f64(raw))
            .collect::<Vec<_>>();
        if weights.iter().all(|&weight| weight == 0.0) {
            return Ok(false);
        }
        if self.multiresolution.is_some() || self.layers.is_some() || self.morph_target.is_some() {
            return Err("Mask-weighted transforms with levels, layers or Morph Targets need a matched detail-transport policy".into());
        }
        let (pose, inverse) = if let Some(metadata) = &self.active_metadata {
            metadata.affine.validate()?;
            let pose = glam::DMat4::from_cols_array(&metadata.affine.local_to_world.map(f64::from));
            (pose, pose.inverse())
        } else {
            (glam::DMat4::IDENTITY, glam::DMat4::IDENTITY)
        };
        if !inverse.is_finite() {
            return Err("Masked transform inverse is not finite".into());
        }
        let mut candidate = source.snapshot();
        for (index, point) in candidate.positions.iter_mut().enumerate() {
            if weights[index] == 0.0 {
                continue;
            }
            let world = pose.transform_point3(point.as_dvec3());
            let transformed = affine.point(world);
            let next_world = world + (transformed - world) * weights[index];
            let next = inverse.transform_point3(next_world);
            if !next.is_finite() || next.abs().max_element() > 1_000_000.0 {
                return Err("Mask-weighted transform point exceeds coordinate limits".into());
            }
            *point = next.as_vec3();
        }
        for (face, &ids) in source.triangles.iter().enumerate() {
            let original = ids.map(|id| source.positions[id as usize].as_dvec3());
            let changed = ids.map(|id| candidate.positions[id as usize].as_dvec3());
            let old_normal = (original[1] - original[0]).cross(original[2] - original[0]);
            let new_normal = (changed[1] - changed[0]).cross(changed[2] - changed[0]);
            if !new_normal.is_finite()
                || new_normal.length_squared() == 0.0
                || !old_normal.is_finite()
                || old_normal.length_squared() == 0.0
                || (affine.is_translation_only() && new_normal.dot(old_normal) <= 0.0)
            {
                return Err(format!(
                    "Mask-weighted transform would collapse or fold face {face}"
                ));
            }
        }
        candidate.recompute_normals();
        candidate.validate()?;
        let mut metadata = self.active_metadata.clone();
        if let Some(metadata) = metadata.as_mut() {
            let source_center =
                crate::Vec3::from_array(metadata.affine.interaction_center).as_dvec3();
            let center =
                inverse.transform_point3(affine.point(pose.transform_point3(source_center)));
            if !center.is_finite() || center.abs().max_element() > 1_000_000.0 {
                return Err("Mask-weighted interaction center exceeds coordinate limits".into());
            }
            metadata.affine.interaction_center = center.as_vec3().to_array();
            metadata.validate(&candidate)?;
        }
        self.check_scene_candidate(&candidate, None, None, None, self.scene.as_ref())?;
        let previous = self.snapshot_state();
        self.mesh = Some(candidate);
        self.active_metadata = metadata;
        if let Err(error) = self.to_json() {
            self.exchange_state(previous);
            return Err(error);
        }
        self.commit_previous(previous);
        Ok(true)
    }

    /// Bake one positive uniform world transform through the complete active
    /// editable state. Hidden/faded layer data and origin-level targets follow;
    /// unrelated SubTools retain their immutable state and mesh Arc identities.
    pub fn transform_active_subtool(
        &mut self,
        transform: &crate::ObjectTransform,
    ) -> Result<bool, String> {
        let affine = crate::transform::Affine::new(transform)?;
        self.apply_active_affine(&affine)
    }

    fn apply_active_affine(&mut self, affine: &crate::transform::Affine) -> Result<bool, String> {
        self.finish_layer_stroke()?;
        if let Some(metadata) = &self.active_metadata {
            if affine.identity {
                return Ok(false);
            }
            let mut candidate = metadata.clone();
            let offset = affine.point(glam::DVec3::ZERO);
            let transform = glam::DMat4::from_cols(
                affine.vector(glam::DVec3::X).extend(0.0),
                affine.vector(glam::DVec3::Y).extend(0.0),
                affine.vector(glam::DVec3::Z).extend(0.0),
                offset.extend(1.0),
            );
            let matrix = transform
                * glam::DMat4::from_cols_array(&metadata.affine.local_to_world.map(f64::from));
            let next = matrix.to_cols_array().map(|v| v as f32);
            candidate.affine = crate::ObjectAffineF32V1 {
                local_to_world: next,
                interaction_center: metadata.affine.interaction_center,
            };
            candidate.validate(active_mesh_ref!(self))?;
            if candidate.same_bits(metadata) {
                return Ok(false);
            }
            let previous = self.snapshot_state();
            self.active_metadata = Some(candidate);
            if let Err(error) = self.to_json() {
                self.exchange_state(previous);
                return Err(error);
            }
            self.commit_previous(previous);
            return Ok(true);
        }
        crate::morph::validate_source(active_mesh_ref!(self))?;
        let source_levels = self
            .multiresolution
            .as_ref()
            .map(|stack| stack.reconciled(active_mesh_ref!(self)))
            .transpose()?;
        let source_layers = self
            .layers
            .as_ref()
            .map(|layers| layers.reconciled(active_mesh_ref!(self)))
            .transpose()?;
        if let Some(target) = &self.morph_target {
            Self::validate_morph_binding(target, active_mesh_ref!(self), source_levels.as_ref())?;
        }
        if affine.identity {
            return Ok(false);
        }
        let mut mesh = affine.mesh(active_mesh_ref!(self))?;
        let layers = match source_layers.as_ref() {
            None => None,
            Some(layers) => {
                let (layers, composed) =
                    layers.transformed(affine, active_mesh_ref!(self), &mesh)?;
                mesh = composed;
                Some(layers)
            }
        };
        let levels = source_levels
            .as_ref()
            .map(|stack| stack.transformed(affine, active_mesh_ref!(self), &mesh))
            .transpose()?;
        let target = self
            .morph_target
            .as_ref()
            .map(|target| target.transformed(affine))
            .transpose()?;
        if let Some(target) = &target {
            Self::validate_morph_binding(target, &mesh, levels.as_ref())?;
        }
        Self::check_output_budget(
            &mesh,
            &self.name,
            levels.as_ref(),
            layers.as_ref(),
            target.as_ref(),
        )?;
        self.check_scene_candidate(
            &mesh,
            levels.as_ref(),
            layers.as_ref(),
            target.as_ref(),
            self.scene.as_ref(),
        )?;
        // Reconciliation materializes an earlier coarse edit; it is not a
        // transform effect. Compare against that effective input, without
        // installing normalized baselines when all transformed data is equal.
        // f64 layer differences remain useful even if visible f32 points round
        // away. Stored levels also use bit equality, including signed zero.
        let unchanged_levels = match (&source_levels, &levels) {
            (None, None) => true,
            (Some(source), Some(candidate)) => {
                source.active == candidate.active
                    && source.levels.len() == candidate.levels.len()
                    && source
                        .levels
                        .iter()
                        .zip(&candidate.levels)
                        .all(|(a, b)| a.same_surface(b) && mesh_channels_equal_bits(a, b))
            }
            _ => false,
        };
        let unchanged_layers = match (&source_layers, &layers) {
            (None, None) => true,
            (Some(a), Some(b)) => a.same_state(b),
            _ => false,
        };
        let unchanged_target = match (&self.morph_target, &target) {
            (None, None) => true,
            (Some(a), Some(b)) => a.same_state(b),
            _ => false,
        };
        if active_mesh_ref!(self).same_surface(&mesh)
            && mesh_channels_equal_bits(active_mesh_ref!(self), &mesh)
            && unchanged_levels
            && unchanged_layers
            && unchanged_target
        {
            return Ok(false);
        }
        let previous = self.snapshot_state();
        self.mesh = Some(mesh);
        self.multiresolution = levels;
        self.layers = layers;
        self.morph_target = target;
        let changed = !previous.matches(self);
        if changed {
            self.commit_previous(previous);
        } else {
            self.exchange_state(previous);
        }
        Ok(changed)
    }

    pub fn active_subtool_id(&self) -> Option<u32> {
        self.scene
            .as_ref()
            .map_or_else(|| self.mesh.as_ref().map(|_| 1), |scene| scene.active_id)
    }

    pub fn subtool_count(&self) -> usize {
        self.scene
            .as_ref()
            .map_or_else(|| usize::from(self.mesh.is_some()), |s| s.objects.len())
    }

    pub fn solo(&self) -> bool {
        self.scene.as_ref().is_some_and(|scene| scene.solo)
    }
    pub fn active_subtool_visible(&self) -> bool {
        self.mesh.is_some()
            && self.scene.as_ref().is_none_or(|scene| {
                scene
                    .objects
                    .iter()
                    .find(|o| Some(o.id) == scene.active_id)
                    .is_some_and(|o| scene.effectively_visible(o))
            })
    }

    pub fn subtool_summaries(&self) -> Vec<crate::SubtoolSummary> {
        let Some(mesh) = self.mesh.as_ref() else {
            return Vec::new();
        };
        let scene = self
            .scene
            .clone()
            .unwrap_or_else(|| Scene::initial(&self.name));
        scene
            .objects
            .iter()
            .map(|object| {
                let active = Some(object.id) == scene.active_id;
                let mesh = object.payload.as_ref().map_or(mesh, |p| p.mesh.as_ref());
                crate::SubtoolSummary {
                    id: object.id,
                    name: object.name.clone(),
                    visible: object.visible,
                    effective_visible: scene.effectively_visible(object),
                    active,
                    vertices: mesh.positions.len(),
                    triangles: mesh.triangles.len(),
                }
            })
            .collect()
    }

    pub fn subtool_render_snapshots(&self) -> Vec<crate::SubtoolRenderSnapshot> {
        let Some(mesh) = self.mesh.as_ref() else {
            return Vec::new();
        };
        match &self.scene {
            None => vec![crate::SubtoolRenderSnapshot {
                id: 1,
                active: true,
                mesh: Arc::new(mesh.snapshot()),
                metadata: self.active_metadata.clone(),
            }],
            Some(scene) => scene
                .objects
                .iter()
                .filter(|o| scene.effectively_visible(o))
                .map(|o| crate::SubtoolRenderSnapshot {
                    id: o.id,
                    active: Some(o.id) == scene.active_id,
                    mesh: o
                        .payload
                        .as_ref()
                        .map_or_else(|| Arc::new(mesh.snapshot()), |p| p.mesh.clone()),
                    metadata: o
                        .payload
                        .as_ref()
                        .map_or_else(|| self.active_metadata.clone(), |p| p.metadata.clone()),
                })
                .collect(),
        }
    }

    /// Checked picking boundary: affine scenes require the dedicated local/world
    /// adapter. Returning an ordinary Hit for them would silently confuse spaces.
    pub fn raycast_subtools(&self, ray: crate::Ray) -> Result<Option<crate::SubtoolHit>, String> {
        if self.active_metadata.is_some()
            || self.scene.as_ref().is_some_and(|s| {
                s.objects
                    .iter()
                    .filter_map(|o| o.payload.as_ref())
                    .any(|p| p.metadata.is_some())
            })
        {
            return Err(
                "Affine scene picking requires the explicit local/world ray adapter".into(),
            );
        }
        let Some(mesh) = self.mesh.as_ref() else {
            return Ok(None);
        };
        let mut nearest: Option<crate::SubtoolHit> = None;
        let mut consider = |id, mesh: &Mesh| {
            if let Some(hit) = mesh.raycast(ray) {
                if nearest.as_ref().is_none_or(|best| {
                    hit.distance < best.hit.distance
                        || (hit.distance == best.hit.distance
                            && Some(id) == self.active_subtool_id()
                            && best.id != id)
                }) {
                    nearest = Some(crate::SubtoolHit { id, hit });
                }
            }
        };
        match &self.scene {
            None => consider(1, mesh),
            Some(scene) => {
                for object in &scene.objects {
                    if scene.effectively_visible(object) {
                        consider(
                            object.id,
                            object.payload.as_ref().map_or(mesh, |p| p.mesh.as_ref()),
                        );
                    }
                }
            }
        }
        Ok(nearest)
    }

    pub fn raycast_subtools_indexed(
        &self,
        ray: crate::Ray,
    ) -> Result<Option<(usize, crate::SubtoolHit)>, String> {
        if self.active_metadata.is_some()
            || self.scene.as_ref().is_some_and(|s| {
                s.objects
                    .iter()
                    .filter_map(|o| o.payload.as_ref())
                    .any(|p| p.metadata.is_some())
            })
        {
            return Err("Affine scene picking requires the explicit dual-space scene query".into());
        }
        let Some(active) = self.mesh.as_ref() else {
            return Ok(None);
        };
        let mut nearest: Option<(usize, crate::SubtoolHit)> = None;
        let mut consider = |id, mesh: &Mesh| {
            if let Some((row, hit)) = mesh.raycast_indexed(ray) {
                if nearest.as_ref().is_none_or(|(_, best)| {
                    hit.distance < best.hit.distance
                        || (hit.distance == best.hit.distance
                            && Some(id) == self.active_subtool_id()
                            && id != best.id)
                }) {
                    nearest = Some((row, crate::SubtoolHit { id, hit }));
                }
            }
        };
        match &self.scene {
            None => consider(1, active),
            Some(scene) => {
                for object in &scene.objects {
                    if scene.effectively_visible(object) {
                        consider(
                            object.id,
                            object.payload.as_ref().map_or(active, |p| p.mesh.as_ref()),
                        );
                    }
                }
            }
        }
        Ok(nearest)
    }

    /// Affine-aware dual-space scene query. The selected policy is explicit,
    /// and an original face row is retained for UV/material picking.
    pub fn raycast_scene(
        &self,
        ray: crate::SceneRay64,
        policy: crate::SglPickingPolicy,
    ) -> Result<Option<crate::SceneRayHit64>, String> {
        ray.validate()?;
        let Some(active) = self.mesh.as_ref() else {
            if self.scene.as_ref().is_some_and(|s| !s.objects.is_empty()) {
                return Err("Absent active mesh cannot pick a nonempty Scene".into());
            }
            return Ok(None);
        };
        let mut work = active.triangles.len();
        if let Some(scene) = &self.scene {
            scene.validate_structure()?;
            for object in &scene.objects {
                if scene.effectively_visible(object) {
                    if let Some(payload) = &object.payload {
                        work = work
                            .checked_add(payload.mesh.triangles.len())
                            .ok_or("Scene pick work overflow")?;
                    }
                }
            }
        }
        if work > crate::MAX_TRIANGLES {
            return Err("Scene pick exceeds aggregate two-million triangle work limit".into());
        }
        let mut nearest: Option<crate::SceneRayHit64> = None;
        let mut consider =
            |id, mesh: &Mesh, metadata: Option<&SceneObjectMetadata>| -> Result<(), String> {
                if let Some(hit) = crate::scene_hit::pick_object(id, mesh, metadata, ray)? {
                    let distance = hit.selection_distance(policy);
                    if nearest.as_ref().is_none_or(|best| {
                        distance < best.selection_distance(policy)
                            || (distance == best.selection_distance(policy)
                                && Some(id) == self.active_subtool_id()
                                && id != best.id)
                    }) {
                        nearest = Some(hit);
                    }
                }
                Ok(())
            };
        match &self.scene {
            None => consider(1, active, self.active_metadata.as_ref())?,
            Some(scene) => {
                for object in &scene.objects {
                    if scene.effectively_visible(object) {
                        match &object.payload {
                            Some(payload) => {
                                consider(object.id, &payload.mesh, payload.metadata.as_ref())?
                            }
                            None => consider(object.id, active, self.active_metadata.as_ref())?,
                        }
                    }
                }
            }
        }
        Ok(nearest)
    }

    /// Extract the visible active mask into an independent new active object.
    /// The original object's complete editable state is retained; append and
    /// whole-scene history use the same checked transaction as other SubTools.
    pub fn extract_mask_as_subtool(
        &mut self,
        settings: &crate::MaskExtractSettings,
    ) -> Result<(u32, crate::MaskExtractReport), String> {
        self.extract_mask_as_subtool_with_selection(settings, crate::MaskExtractSelection::Clipped)
    }

    pub fn extract_mask_as_subtool_with_selection(
        &mut self,
        settings: &crate::MaskExtractSettings,
        selection: crate::MaskExtractSelection,
    ) -> Result<(u32, crate::MaskExtractReport), String> {
        self.extract_mask_as_subtool_with_options(settings, selection, false)
    }

    /// Keep the original object selected after extraction when requested.
    /// The new object is installed as an inactive payload in the same checked
    /// scene transaction, so the complete operation still has one undo entry.
    pub fn extract_mask_as_subtool_with_options(
        &mut self,
        settings: &crate::MaskExtractSettings,
        selection: crate::MaskExtractSelection,
        keep_source_active: bool,
    ) -> Result<(u32, crate::MaskExtractReport), String> {
        self.extract_mask_as_subtool_with_surface_options(
            settings,
            selection,
            keep_source_active,
            false,
        )
    }

    pub fn extract_mask_as_subtool_with_surface_options(
        &mut self,
        settings: &crate::MaskExtractSettings,
        selection: crate::MaskExtractSelection,
        keep_source_active: bool,
        sculptgl_surface_offset: bool,
    ) -> Result<(u32, crate::MaskExtractReport), String> {
        self.finish_layer_stroke()?;
        if !self.active_subtool_visible() {
            return Err("Show the active SubTool before extracting its mask".into());
        }
        if self.subtool_count() >= crate::MAX_SUBTOOLS {
            return Err("At most 32 SubTools are supported".into());
        }
        let (mesh, report) = active_mesh_ref!(self).extract_mask_with_affine(
            settings,
            selection,
            sculptgl_surface_offset,
            self.active_metadata
                .as_ref()
                .map(|metadata| &metadata.affine),
        )?;
        let id = if keep_source_active {
            self.add_inactive_extracted_subtool(mesh, "Mask extract")?
        } else {
            self.add_subtool(mesh, "Mask extract")?
        };
        Ok((id, report))
    }

    fn add_inactive_extracted_subtool(
        &mut self,
        mut mesh: Mesh,
        name: &str,
    ) -> Result<u32, String> {
        crate::subtools::validate_name(name)?;
        mesh.validate()?;
        mesh.recompute_normals();
        let mut scene = self
            .scene
            .clone()
            .unwrap_or_else(|| Scene::initial(&self.name));
        if scene.objects.len() >= crate::MAX_SUBTOOLS {
            return Err("At most 32 SubTools are supported".into());
        }
        let id = scene.next_id;
        scene.next_id = id
            .checked_add(1)
            .ok_or("SubTool ID allocator is exhausted")?;
        scene.objects.push(Subtool {
            id,
            name: name.into(),
            visible: true,
            payload: Some(Arc::new(ObjectState::new(mesh, None, None, None))),
        });
        self.check_scene_candidate(
            active_mesh_ref!(self),
            self.multiresolution.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            Some(&scene),
        )?;
        let previous = self.snapshot_state();
        self.scene = Some(scene);
        self.commit_previous(previous);
        Ok(id)
    }

    pub fn add_subtool(&mut self, mesh: Mesh, name: &str) -> Result<u32, String> {
        self.add_subtool_impl(mesh, name, false, None, None)
    }

    /// Add a Source-profile primitive and its last four static subdivision
    /// levels as one validated scene/history transaction. The base and all
    /// subdivision work remain in the shared native/WASM core.
    pub fn add_source_primitive_subtool(
        &mut self,
        base: Mesh,
        name: &str,
        settings: &crate::StaticSubdivisionSettings,
        subdivisions: usize,
        translation: crate::Vec3,
    ) -> Result<u32, String> {
        base.validate()?;
        let stack = Multiresolution::source_primitive(&base, settings, subdivisions, translation)?;
        let mesh = stack.levels[stack.active].snapshot();
        self.add_subtool_impl(mesh, name, true, None, Some(stack))
    }

    /// Append a validated imported mesh retaining its supplied normal bits.
    /// Ordinary add_subtool still computes geometric normals for generated meshes.
    pub fn add_subtool_preserving_normals(
        &mut self,
        mesh: Mesh,
        name: &str,
    ) -> Result<u32, String> {
        self.add_subtool_impl(mesh, name, true, None, None)
    }

    /// Append a preflighted source mesh with its editable local-to-world pose.
    /// The metadata is admitted atomically with the mesh and one undo entry.
    pub fn add_subtool_with_metadata(
        &mut self,
        mesh: Mesh,
        name: &str,
        preserve_normals: bool,
        metadata: SceneObjectMetadata,
    ) -> Result<u32, String> {
        self.add_subtool_impl(mesh, name, preserve_normals, Some(metadata), None)
    }

    pub fn add_subtools_with_metadata(
        &mut self,
        mut parts: Vec<ImportedSubtool>,
    ) -> Result<Vec<u32>, String> {
        if parts.is_empty() || parts.len() > crate::MAX_SUBTOOLS {
            return Err("Source Add requires 1–32 imported parts".into());
        }
        if self.subtool_count().saturating_add(parts.len()) > crate::MAX_SUBTOOLS {
            return Err("At most32 SubTools are supported".into());
        }
        for part in &mut parts {
            crate::subtools::validate_name(&part.name)?;
            part.mesh.validate()?;
            if !part.preserve_normals {
                part.mesh.recompute_normals();
            }
            part.metadata.validate(&part.mesh)?;
        }
        self.finish_layer_stroke()?;
        let mut scene = self.scene.clone().unwrap_or_else(|| {
            if self.mesh.is_some() {
                Scene::initial(&self.name)
            } else {
                Scene::empty()
            }
        });
        if let Some(active) = scene.active_id {
            let index = scene.index(active)?;
            scene.objects[index].payload = Some(self.capture_object()?);
        }
        let mut ids = Vec::new();
        ids.try_reserve_exact(parts.len())
            .map_err(|_| "Cannot allocate bounded imported ID list")?;
        let last = parts.pop().expect("Nonempty imported parts");
        for part in parts {
            let id = scene.next_id;
            scene.next_id = id
                .checked_add(1)
                .ok_or("SubTool ID allocator is exhausted")?;
            ids.push(id);
            scene.objects.push(Subtool {
                id,
                name: part.name,
                visible: true,
                payload: Some(Arc::new(ObjectState::with_metadata(
                    part.mesh,
                    None,
                    None,
                    None,
                    Some(part.metadata),
                ))),
            });
        }
        let id = scene.next_id;
        scene.next_id = id
            .checked_add(1)
            .ok_or("SubTool ID allocator is exhausted")?;
        ids.push(id);
        scene.objects.push(Subtool {
            id,
            name: last.name,
            visible: true,
            payload: None,
        });
        scene.active_id = Some(id);
        scene.selected_ids = vec![id];
        self.check_scene_candidate_inner(
            &last.mesh,
            None,
            None,
            None,
            Some(&scene),
            (false, false, Some(&last.metadata)),
        )?;
        let previous = self.snapshot_state();
        self.mesh = Some(last.mesh);
        self.active_metadata = Some(last.metadata);
        self.multiresolution = None;
        self.layers = None;
        self.morph_target = None;
        self.scene = Some(scene);
        self.commit_previous(previous);
        Ok(ids)
    }

    fn add_subtool_impl(
        &mut self,
        mut mesh: Mesh,
        name: &str,
        preserve_normals: bool,
        metadata: Option<SceneObjectMetadata>,
        multiresolution: Option<Multiresolution>,
    ) -> Result<u32, String> {
        crate::subtools::validate_name(name)?;
        mesh.validate()?;
        self.finish_layer_stroke()?;
        if !preserve_normals {
            mesh.recompute_normals();
        }
        if let Some(metadata) = &metadata {
            metadata.validate(&mesh)?;
        }
        if let Some(stack) = &multiresolution {
            stack.validate_budget()?;
            stack.reconciled(&mesh)?;
            Self::check_output_budget(&mesh, &self.name, Some(stack), None, None)?;
        }
        let mut scene = self.scene.clone().unwrap_or_else(|| {
            if self.mesh.is_some() {
                Scene::initial(&self.name)
            } else {
                Scene::empty()
            }
        });
        if scene.objects.len() >= crate::MAX_SUBTOOLS {
            return Err("At most32 SubTools are supported".into());
        }
        let id = scene.next_id;
        scene.next_id = id
            .checked_add(1)
            .ok_or("SubTool ID allocator is exhausted")?;
        if let Some(active) = scene.active_id {
            let index = scene.index(active)?;
            scene.objects[index].payload = Some(self.capture_object()?);
        }
        scene.objects.push(Subtool {
            id,
            name: name.into(),
            visible: true,
            payload: None,
        });
        scene.active_id = Some(id);
        scene.selected_ids = vec![id];
        self.check_scene_candidate_inner(
            &mesh,
            multiresolution.as_ref(),
            None,
            None,
            Some(&scene),
            (false, false, metadata.as_ref()),
        )?;
        let previous = self.snapshot_state();
        self.mesh = Some(mesh);
        self.active_metadata = metadata;
        self.multiresolution = multiresolution;
        self.layers = None;
        self.morph_target = None;
        self.scene = Some(scene);
        self.commit_previous(previous);
        Ok(id)
    }

    /// Concatenate explicit effectively visible stable part IDs in supplied
    /// selection order and append the result after unselected parts.
    /// This consumes those parts without welding or boolean union. RequirePlain
    /// rejects rich state; FlattenDisplayed explicitly bakes current surfaces,
    /// dropping selected retained levels/layers/targets and reporting that loss.
    /// A valid pending stroke is finished first; the merge then commits one full
    /// scene undo entry. A selected active ID is retained for the result;
    /// unselected parts/IDs and Solo policy remain intact.
    pub fn merge_subtools(
        &mut self,
        settings: &crate::SubtoolMergeSettings,
    ) -> Result<crate::SubtoolMergeReport, String> {
        crate::subtools::validate_name(&settings.name)?;
        if settings.ids.len() < 2 || settings.ids.len() > crate::MAX_SUBTOOLS {
            return Err("Merge requires 2–32 explicit SubTool IDs".into());
        }
        let selected = settings
            .ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if selected.len() != settings.ids.len() || selected.contains(&0) {
            return Err("Merge SubTool IDs must be distinct positive values".into());
        }
        self.finish_layer_stroke()?;
        let mut scene = self
            .scene
            .clone()
            .ok_or("Merge requires a scene with multiple SubTools")?;
        scene.validate_structure()?;
        self.check_scene_candidate(
            active_mesh_ref!(self),
            self.multiresolution.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            Some(&scene),
        )?;
        for &id in &settings.ids {
            let index = scene.index(id)?;
            if !scene.effectively_visible(&scene.objects[index]) {
                return Err(
                    "Show every selected SubTool and leave Solo before merging multiple parts"
                        .into(),
                );
            }
        }
        let reuse_active_id = scene.active_id.is_some_and(|id| selected.contains(&id));
        let merged_id = if reuse_active_id {
            scene.active_id.ok_or("Scene has no active object")?
        } else {
            scene.next_id
        };
        let next_id = if reuse_active_id {
            scene.next_id
        } else {
            scene
                .next_id
                .checked_add(1)
                .ok_or("SubTool ID allocator is exhausted")?
        };
        let current = self.capture_object()?;
        let mut payloads = Vec::new();
        payloads
            .try_reserve_exact(selected.len())
            .map_err(|_| "Cannot allocate bounded merge selection")?;
        let mut consumed_ids = Vec::new();
        let mut flattened_subtools = Vec::new();
        let mut discarded_retained_levels = 0;
        let mut baked_layers = 0;
        let mut discarded_morph_targets = 0;
        let mut baked_pose_ids = Vec::new();
        let mut discarded_source_origin_ids = Vec::new();
        for &id in &settings.ids {
            let object = &scene.objects[scene.index(id)?];
            let payload = if Some(object.id) == scene.active_id {
                current.clone()
            } else {
                object
                    .payload
                    .clone()
                    .ok_or("Selected inactive SubTool payload is missing")?
            };
            if let Some(metadata) = &payload.metadata {
                if !settings.allow_source_pose_loss {
                    return Err("Merge of affine/source-origin objects needs explicit pose/provenance loss consent".into());
                }
                metadata.validate(&payload.mesh)?;
                baked_pose_ids.push(id);
                if metadata.origin.is_some() {
                    discarded_source_origin_ids.push(id);
                }
            }
            let rich = payload.multiresolution.is_some()
                || payload.layers.is_some()
                || payload.morph_target.is_some();
            if rich && settings.policy == crate::SubtoolMergePolicy::RequirePlain {
                return Err("Selected SubTools contain retained levels, layers or Morph Targets; explicitly choose FlattenDisplayed to consume that rich state".into());
            }
            if rich {
                flattened_subtools.push(object.id);
                discarded_retained_levels += payload
                    .multiresolution
                    .as_ref()
                    .map_or(0, |levels| levels.levels.len().saturating_sub(1));
                baked_layers += payload
                    .layers
                    .as_ref()
                    .map_or(0, |layers| layers.summaries().len());
                discarded_morph_targets += usize::from(payload.morph_target.is_some());
            }
            consumed_ids.push(object.id);
            payloads.push(payload);
        }
        let target_mask_convention = payloads
            .first()
            .ok_or("Merge selection is empty")?
            .mesh
            .mask_convention;
        let mut converted_mask_ids = Vec::new();
        let mut mask_roundtrip_changed_words = 0;
        let mut discarded_uv_ids = Vec::new();
        let mut posed_copies = Vec::new();
        posed_copies
            .try_reserve_exact(payloads.len())
            .map_err(|_| "Cannot allocate bounded posed merge selection")?;
        for (id, payload) in consumed_ids.iter().copied().zip(&payloads) {
            let needs_mask_conversion = payload.mesh.mask_convention != target_mask_convention;
            let has_uv = payload
                .mesh
                .surface
                .as_ref()
                .is_some_and(|surface| surface.uv.is_some());
            if payload.metadata.is_some()
                || needs_mask_conversion
                || (has_uv && settings.allow_uv_loss)
            {
                let mut posed = if needs_mask_conversion {
                    converted_mask_ids.push(id);
                    let (converted, report) = payload.mesh.converted_mask_convention(
                        target_mask_convention,
                        if settings.allow_mask_precision_loss {
                            crate::MaskConversionPolicy::AllowAndReport
                        } else {
                            crate::MaskConversionPolicy::RequireRoundtripExact
                        },
                    )?;
                    mask_roundtrip_changed_words += report.roundtrip_changed_words;
                    converted
                } else {
                    payload.mesh.try_snapshot()?
                };
                if let Some(metadata) = &payload.metadata {
                    for point in &mut posed.positions {
                        *point = metadata.affine.world_point_validated(*point)?.as_vec3();
                    }
                    posed.recompute_normals();
                }
                if has_uv && settings.allow_uv_loss {
                    posed.surface.as_mut().unwrap().uv = None;
                    discarded_uv_ids.push(id);
                }
                posed_copies.push(Some(posed));
            } else {
                posed_copies.push(None);
            }
        }
        let sources = payloads
            .iter()
            .zip(&posed_copies)
            .map(|(payload, posed)| posed.as_ref().unwrap_or(payload.mesh.as_ref()))
            .collect::<Vec<_>>();
        let mesh = Mesh::concatenate(&sources)?;
        // If the old active part was not chosen it becomes an immutable inactive
        // object; its complete rich state survives the new active selection.
        if !scene.active_id.is_some_and(|id| selected.contains(&id)) {
            let index = scene.index(scene.active_id.ok_or("Scene has no active object")?)?;
            scene.objects[index].payload = Some(current);
        }
        let merged = Subtool {
            id: merged_id,
            name: settings.name.clone(),
            visible: true,
            payload: None,
        };
        let mut objects = Vec::new();
        objects
            .try_reserve_exact(scene.objects.len() - selected.len() + 1)
            .map_err(|_| "Cannot allocate bounded merged scene")?;
        for object in scene.objects {
            if !selected.contains(&object.id) {
                objects.push(object);
            }
        }
        objects.push(merged);
        scene.objects = objects;
        scene.active_id = Some(merged_id);
        scene.selected_ids = vec![merged_id];
        scene.next_id = next_id;
        self.check_scene_candidate(&mesh, None, None, None, Some(&scene))?;
        let report = crate::SubtoolMergeReport {
            consumed_ids,
            merged_id,
            vertices: mesh.positions.len(),
            triangles: mesh.triangles.len(),
            flattened_subtools,
            discarded_retained_levels,
            baked_layers,
            discarded_morph_targets,
            baked_pose_ids,
            discarded_source_origin_ids,
            converted_mask_ids,
            mask_roundtrip_changed_words,
            discarded_uv_ids,
            policy: settings.policy,
        };
        let previous = self.snapshot_state();
        self.mesh = Some(mesh);
        self.active_metadata = None;
        self.multiresolution = None;
        self.layers = None;
        self.morph_target = None;
        self.scene = Some(scene);
        self.commit_previous(previous);
        Ok(report)
    }

    /// Reconstruct the union of the currently selected, visible plain solids
    /// as one new SubTool. Build and validate every candidate before a single
    /// scene-history commit; failed voxelization leaves the originals and redo.
    pub fn remesh_selected_subtools(
        &mut self,
        settings: &crate::RemeshSettings,
        policy: crate::SubtoolMergePolicy,
    ) -> Result<crate::SelectedRemeshReport, String> {
        self.remesh_selected_subtools_with_progress(settings, policy, |_| false)
    }

    pub fn remesh_selected_subtools_with_progress(
        &mut self,
        settings: &crate::RemeshSettings,
        policy: crate::SubtoolMergePolicy,
        cancel: impl FnMut(crate::RemeshProgress) -> bool,
    ) -> Result<crate::SelectedRemeshReport, String> {
        self.finish_layer_stroke()?;
        let mut scene = self
            .scene
            .clone()
            .ok_or("Selected remesh requires a scene with multiple SubTools")?;
        scene.validate_structure()?;
        if scene.selected_ids.len() < 2 {
            return Err("Select at least two SubTools for a fused remesh".into());
        }
        let selected = scene
            .selected_ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if selected.len() != scene.selected_ids.len() {
            return Err("Selected remesh contains duplicate SubTool IDs".into());
        }
        let created_id = scene.next_id;
        let next_id = created_id
            .checked_add(1)
            .ok_or("SubTool ID allocator is exhausted")?;
        let current = self.capture_object()?;
        let mut payloads = Vec::new();
        payloads
            .try_reserve_exact(selected.len())
            .map_err(|_| "Cannot allocate bounded remesh selection")?;
        let mut consumed_ids = Vec::new();
        let mut flattened_ids = Vec::new();
        let mut baked_pose_ids = Vec::new();
        let mut discarded_source_origin_ids = Vec::new();
        for &id in &scene.selected_ids {
            let object = &scene.objects[scene.index(id)?];
            if !scene.effectively_visible(object) {
                return Err(
                    "Show every selected SubTool and leave Solo before fused remeshing".into(),
                );
            }
            let payload = if Some(object.id) == scene.active_id {
                current.clone()
            } else {
                object
                    .payload
                    .clone()
                    .ok_or("Selected inactive SubTool payload is missing")?
            };
            if let Some(metadata) = &payload.metadata {
                if !settings.allow_source_pose_loss {
                    return Err("Selected remesh of affine/source-origin objects requires explicit pose/provenance loss consent".into());
                }
                metadata.validate(&payload.mesh)?;
                baked_pose_ids.push(object.id);
                if metadata.origin.is_some() {
                    discarded_source_origin_ids.push(object.id);
                }
            }
            let rich = payload.multiresolution.is_some()
                || payload.layers.is_some()
                || payload.morph_target.is_some();
            if rich && policy == crate::SubtoolMergePolicy::RequirePlain {
                return Err("Selected SubTools contain retained levels, layers or Morph Targets; explicitly choose FlattenDisplayed to consume that rich state".into());
            }
            if rich {
                flattened_ids.push(object.id);
            }
            consumed_ids.push(object.id);
            payloads.push(payload);
        }
        let mut posed_copies = Vec::new();
        posed_copies
            .try_reserve_exact(payloads.len())
            .map_err(|_| "Cannot allocate bounded posed remesh selection")?;
        // Source remesh grows the grid from unrounded transformed positions;
        // its vertex arrays then store those positions as f32. Keep that
        // distinction for posed parts, including unposed peers in the union.
        let mut pre_rounding_min = glam::DVec3::splat(f64::INFINITY);
        let mut pre_rounding_max = glam::DVec3::splat(f64::NEG_INFINITY);
        let mut has_posed_source = false;
        for payload in &payloads {
            if let Some(metadata) = &payload.metadata {
                has_posed_source = true;
                let mut copy = payload.mesh.try_snapshot()?;
                for point in &mut copy.positions {
                    let world = metadata.affine.world_point_validated(*point)?;
                    pre_rounding_min = pre_rounding_min.min(world);
                    pre_rounding_max = pre_rounding_max.max(world);
                    *point = world.as_vec3();
                }
                copy.recompute_normals();
                copy.validate()?;
                posed_copies.push(Some(copy));
            } else {
                for point in &payload.mesh.positions {
                    let world = point.as_dvec3();
                    pre_rounding_min = pre_rounding_min.min(world);
                    pre_rounding_max = pre_rounding_max.max(world);
                }
                posed_copies.push(None);
            }
        }
        let sources = payloads
            .iter()
            .zip(&posed_copies)
            .map(|(payload, copy)| copy.as_ref().unwrap_or(payload.mesh.as_ref()))
            .collect::<Vec<_>>();
        let uv_counts =
            sources
                .iter()
                .try_fold((0_usize, 0_usize), |(coordinates, faces), source| {
                    let Some(uv) = source
                        .surface
                        .as_ref()
                        .and_then(|surface| surface.uv.as_ref())
                    else {
                        return Ok::<_, String>((coordinates, faces));
                    };
                    Ok((
                        coordinates
                            .checked_add(uv.coordinates.len())
                            .ok_or("Selected UV coordinate count overflow")?,
                        faces
                            .checked_add(uv.face_indices.len())
                            .ok_or("Selected UV face count overflow")?,
                    ))
                })?;
        let mixed_uv = uv_counts.0 > 0
            && sources.iter().any(|source| {
                source
                    .surface
                    .as_ref()
                    .is_none_or(|surface| surface.uv.is_none())
            });
        if mixed_uv && !settings.allow_uv_loss {
            return Err(
                "Selected remesh would discard corner UVs; explicitly allow UV loss first".into(),
            );
        }
        // Concatenation cannot invent UVs for untextured parts. When the caller
        // accepts UV loss, detach only temporary copies for concatenation and
        // retain the original sources for UV-aware voxel sampling order.
        let combined = if mixed_uv {
            let mut detached = Vec::new();
            detached
                .try_reserve_exact(sources.len())
                .map_err(|_| "Cannot allocate bounded remesh UV selection")?;
            for source in &sources {
                if source
                    .surface
                    .as_ref()
                    .is_some_and(|surface| surface.uv.is_some())
                {
                    let mut copy = source.try_snapshot()?;
                    copy.surface.as_mut().unwrap().uv = None;
                    detached.push(Some(copy));
                } else {
                    detached.push(None);
                }
            }
            let joined = sources
                .iter()
                .zip(&detached)
                .map(|(&original, copy)| copy.as_ref().unwrap_or(original))
                .collect::<Vec<_>>();
            Mesh::concatenate(&joined)?
        } else {
            Mesh::concatenate(&sources)?
        };
        let source_face_optimization = payloads
            .iter()
            .map(|payload| payload.metadata.is_some())
            .collect::<Vec<_>>();
        let (mesh, mut remesh) = combined.remeshed_from_sources(
            settings,
            &sources,
            &source_face_optimization,
            has_posed_source.then_some((pre_rounding_min, pre_rounding_max)),
            cancel,
        )?;
        remesh.discarded_uv_coordinates = uv_counts.0;
        remesh.discarded_uv_faces = uv_counts.1;
        if !scene.active_id.is_some_and(|id| selected.contains(&id)) {
            let index = scene.index(scene.active_id.ok_or("Scene has no active object")?)?;
            scene.objects[index].payload = Some(current);
        }
        let first = scene
            .objects
            .iter()
            .position(|object| selected.contains(&object.id))
            .ok_or("Selected remesh has no source objects")?;
        let mut objects = Vec::new();
        objects
            .try_reserve_exact(scene.objects.len() - selected.len() + 1)
            .map_err(|_| "Cannot allocate bounded remeshed scene")?;
        for (index, object) in scene.objects.into_iter().enumerate() {
            if index == first {
                objects.push(Subtool {
                    id: created_id,
                    name: "Voxel remesh".into(),
                    visible: true,
                    payload: None,
                });
            }
            if !selected.contains(&object.id) {
                objects.push(object);
            }
        }
        scene.objects = objects;
        scene.active_id = Some(created_id);
        scene.selected_ids = vec![created_id];
        scene.next_id = next_id;
        self.check_scene_candidate(&mesh, None, None, None, Some(&scene))?;
        let report = crate::SelectedRemeshReport {
            consumed_ids,
            created_id,
            flattened_ids,
            baked_pose_ids,
            discarded_source_origin_ids,
            policy,
            remesh,
        };
        let previous = self.snapshot_state();
        self.mesh = Some(mesh);
        self.active_metadata = None;
        self.multiresolution = None;
        self.layers = None;
        self.morph_target = None;
        self.scene = Some(scene);
        self.commit_previous(previous);
        Ok(report)
    }

    pub fn select_subtool(&mut self, id: u32) -> Result<(), String> {
        self.finish_layer_stroke()?;
        if Some(id) == self.active_subtool_id() {
            return Ok(());
        }
        let mut scene = self
            .scene
            .clone()
            .ok_or("SubTool ID is not in this scene")?;
        let target_index = scene.index(id)?;
        let payload = scene.objects[target_index]
            .payload
            .clone()
            .ok_or("Inactive SubTool payload is missing")?;
        let current_index = scene.index(scene.active_id.ok_or("Scene has no active object")?)?;
        scene.objects[current_index].payload = Some(self.capture_object()?);
        scene.objects[target_index].payload = None;
        scene.active_id = Some(id);
        scene.selected_ids = vec![id];
        self.check_scene_candidate(
            &payload.mesh,
            payload.multiresolution.as_ref(),
            payload.layers.as_ref(),
            payload.morph_target.as_ref(),
            Some(&scene),
        )?;
        self.install_object(&payload);
        self.scene = Some(scene);
        Ok(())
    }

    pub fn duplicate_subtool(&mut self, id: u32) -> Result<u32, String> {
        self.finish_layer_stroke()?;
        let mut scene = self
            .scene
            .clone()
            .unwrap_or_else(|| Scene::initial(&self.name));
        let source_index = scene.index(id)?;
        if scene.objects.len() >= crate::MAX_SUBTOOLS {
            return Err("At most 32 SubTools are supported".into());
        }
        let new_id = scene.next_id;
        scene.next_id = new_id
            .checked_add(1)
            .ok_or("SubTool ID allocator is exhausted")?;
        let current = self.capture_object()?;
        let payload = scene.objects[source_index]
            .payload
            .clone()
            .unwrap_or_else(|| current.clone());
        let mut name = scene.objects[source_index].name.clone();
        while name.len() + 5 > crate::MAX_SUBTOOL_NAME_BYTES {
            name.pop();
        }
        name.push_str(" copy");
        let current_index = scene.index(scene.active_id.ok_or("Scene has no active object")?)?;
        scene.objects[current_index].payload = Some(current);
        scene.objects.insert(
            source_index + 1,
            Subtool {
                id: new_id,
                name,
                visible: true,
                payload: None,
            },
        );
        scene.active_id = Some(new_id);
        scene.selected_ids = vec![new_id];
        self.check_scene_candidate(
            &payload.mesh,
            payload.multiresolution.as_ref(),
            payload.layers.as_ref(),
            payload.morph_target.as_ref(),
            Some(&scene),
        )?;
        let previous = self.snapshot_state();
        self.install_object(&payload);
        self.scene = Some(scene);
        self.commit_previous(previous);
        Ok(new_id)
    }

    /// Match the source's selected-copy sequence: append copies in selection
    /// order, retain one history step per copy, then navigate back to the last
    /// original selection without creating another history entry. Every
    /// intermediate and final scene is checked before publishing the first.
    pub fn duplicate_selected_subtools(&mut self) -> Result<Vec<u32>, String> {
        self.finish_layer_stroke()?;
        let scene = self
            .scene
            .clone()
            .unwrap_or_else(|| Scene::initial(&self.name));
        let selected = scene.selected_ids.clone();
        if selected.is_empty() {
            return Ok(Vec::new());
        }
        if scene.objects.len() + selected.len() > crate::MAX_SUBTOOLS {
            return Err("At most 32 SubTools are supported".into());
        }
        let original_active = scene.active_id.ok_or("Scene has no active object")?;
        let current = self.capture_object()?;
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(selected.len())
            .map_err(|_| "Cannot allocate selected copy sources")?;
        for &id in &selected {
            let source = &scene.objects[scene.index(id)?];
            let payload = if id == original_active {
                current.clone()
            } else {
                source
                    .payload
                    .clone()
                    .ok_or("Inactive SubTool payload is missing")?
            };
            let mut name = source.name.clone();
            while name.len() + 5 > crate::MAX_SUBTOOL_NAME_BYTES {
                name.pop();
            }
            name.push_str(" copy");
            sources.push((name, payload));
        }

        let mut staged = Vec::new();
        staged
            .try_reserve_exact(sources.len())
            .map_err(|_| "Cannot allocate selected copy states")?;
        let mut ids = Vec::new();
        ids.try_reserve_exact(sources.len())
            .map_err(|_| "Cannot allocate selected copy IDs")?;
        let mut next_scene = scene;
        let mut active_payload = current;
        for (name, payload) in sources {
            let previous_active = next_scene.active_id.ok_or("Scene has no active object")?;
            let previous_index = next_scene.index(previous_active)?;
            next_scene.objects[previous_index].payload = Some(active_payload);
            let new_id = next_scene.next_id;
            next_scene.next_id = new_id
                .checked_add(1)
                .ok_or("SubTool ID allocator is exhausted")?;
            next_scene.objects.push(Subtool {
                id: new_id,
                name,
                visible: true,
                payload: None,
            });
            next_scene.active_id = Some(new_id);
            next_scene.selected_ids = vec![new_id];
            self.check_scene_candidate(
                &payload.mesh,
                payload.multiresolution.as_ref(),
                payload.layers.as_ref(),
                payload.morph_target.as_ref(),
                Some(&next_scene),
            )?;
            staged.push((next_scene.clone(), payload.clone()));
            ids.push(new_id);
            active_payload = payload;
        }

        let last_original = *selected.last().ok_or("Scene selection is empty")?;
        let current_index =
            next_scene.index(next_scene.active_id.ok_or("Scene has no active object")?)?;
        next_scene.objects[current_index].payload = Some(active_payload);
        let original_index = next_scene.index(last_original)?;
        let final_payload = next_scene.objects[original_index]
            .payload
            .take()
            .ok_or("Original selected SubTool payload is missing")?;
        next_scene.active_id = Some(last_original);
        next_scene.selected_ids = vec![last_original];
        self.check_scene_candidate(
            &final_payload.mesh,
            final_payload.multiresolution.as_ref(),
            final_payload.layers.as_ref(),
            final_payload.morph_target.as_ref(),
            Some(&next_scene),
        )?;
        for (scene, payload) in staged {
            let previous = self.snapshot_state();
            self.install_object(&payload);
            self.scene = Some(scene);
            self.commit_previous(previous);
        }
        self.install_object(&final_payload);
        self.scene = Some(next_scene);
        Ok(ids)
    }

    pub fn set_subtool_name(&mut self, id: u32, name: &str) -> Result<(), String> {
        crate::subtools::validate_name(name)?;
        self.edit_scene_metadata(|scene| {
            let index = scene.index(id)?;
            scene.objects[index].name = name.into();
            Ok(())
        })
    }
    pub fn set_subtool_visible(&mut self, id: u32, visible: bool) -> Result<(), String> {
        self.edit_scene_metadata(|scene| {
            let index = scene.index(id)?;
            scene.objects[index].visible = visible;
            Ok(())
        })
    }
    pub fn set_solo(&mut self, solo: bool) -> Result<(), String> {
        self.edit_scene_metadata(|scene| {
            scene.solo = solo;
            Ok(())
        })
    }
    pub fn isolate_selected_subtools(&mut self) -> Result<(), String> {
        self.edit_scene_metadata(|scene| {
            if scene.objects.len() < 2 || scene.selected_ids.len() == scene.objects.len() {
                return Ok(());
            }
            for object in &mut scene.objects {
                if !scene.selected_ids.contains(&object.id) {
                    object.visible = false;
                }
            }
            scene.solo = false;
            Ok(())
        })
    }
    pub fn show_all_subtools(&mut self) -> Result<(), String> {
        self.edit_scene_metadata(|scene| {
            for object in &mut scene.objects {
                object.visible = true;
            }
            scene.solo = false;
            Ok(())
        })
    }
    pub fn reorder_subtool(&mut self, id: u32, new_index: usize) -> Result<(), String> {
        self.edit_scene_metadata(|scene| {
            if new_index >= scene.objects.len() {
                return Err("SubTool index is out of range".into());
            }
            let index = scene.index(id)?;
            let object = scene.objects.remove(index);
            scene.objects.insert(new_index, object);
            Ok(())
        })
    }
    pub fn delete_subtool(&mut self, id: u32) -> Result<(), String> {
        self.finish_layer_stroke()?;
        let mut scene = self
            .scene
            .clone()
            .unwrap_or_else(|| Scene::initial(&self.name));
        let index = scene.index(id)?;
        if scene.objects.len() == 1 {
            return self.clear_scene().map(|_| ());
        }
        scene.objects.remove(index);
        scene.selected_ids.retain(|&selected| selected != id);
        let replacement = if Some(id) == scene.active_id {
            let selected = index.min(scene.objects.len() - 1);
            let payload = scene.objects[selected]
                .payload
                .take()
                .ok_or("Inactive SubTool payload is missing")?;
            scene.active_id = Some(scene.objects[selected].id);
            if !scene.selected_ids.contains(&scene.objects[selected].id) {
                scene.selected_ids.push(scene.objects[selected].id);
            }
            Some(payload)
        } else {
            None
        };
        if let Some(payload) = &replacement {
            self.check_scene_candidate(
                &payload.mesh,
                payload.multiresolution.as_ref(),
                payload.layers.as_ref(),
                payload.morph_target.as_ref(),
                Some(&scene),
            )?;
        } else {
            self.check_scene_candidate(
                active_mesh_ref!(self),
                self.multiresolution.as_ref(),
                self.layers.as_ref(),
                self.morph_target.as_ref(),
                Some(&scene),
            )?;
        }
        let previous = self.snapshot_state();
        if let Some(payload) = replacement {
            self.install_object(&payload);
        }
        self.scene = Some(scene);
        self.commit_previous(previous);
        Ok(())
    }
    /// Delete the complete scene selection in one undoable transaction. When
    /// unselected parts survive, keep one survivor active as required by the
    /// document's nonempty-scene invariant.
    pub fn delete_selected_subtools(&mut self) -> Result<(), String> {
        self.finish_layer_stroke()?;
        let mut scene = self
            .scene
            .clone()
            .unwrap_or_else(|| Scene::initial(&self.name));
        if scene.objects.is_empty() || scene.selected_ids.is_empty() {
            return Ok(());
        }
        if scene.selected_ids.len() == scene.objects.len() {
            return self.clear_scene().map(|_| ());
        }
        let selected = scene
            .selected_ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let first_removed = scene
            .objects
            .iter()
            .position(|object| selected.contains(&object.id))
            .ok_or("Scene selection has no objects")?;
        scene
            .objects
            .retain(|object| !selected.contains(&object.id));
        let survivor_index = first_removed.min(scene.objects.len() - 1);
        let survivor = &mut scene.objects[survivor_index];
        let payload = survivor
            .payload
            .take()
            .ok_or("Surviving SubTool payload is missing")?;
        scene.active_id = Some(survivor.id);
        scene.selected_ids = vec![survivor.id];
        self.check_scene_candidate(
            &payload.mesh,
            payload.multiresolution.as_ref(),
            payload.layers.as_ref(),
            payload.morph_target.as_ref(),
            Some(&scene),
        )?;
        let previous = self.snapshot_state();
        self.install_object(&payload);
        self.scene = Some(scene);
        self.commit_previous(previous);
        Ok(())
    }
    fn edit_scene_metadata(
        &mut self,
        edit: impl FnOnce(&mut Scene) -> Result<(), String>,
    ) -> Result<(), String> {
        self.finish_layer_stroke()?;
        let initial = self
            .scene
            .clone()
            .unwrap_or_else(|| Scene::initial(&self.name));
        let mut candidate = initial.clone();
        edit(&mut candidate)?;
        if candidate.same_state(&initial) {
            return Ok(());
        }
        self.check_scene_candidate(
            active_mesh_ref!(self),
            self.multiresolution.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            Some(&candidate),
        )?;
        let previous = self.snapshot_state();
        self.scene = Some(candidate);
        self.commit_previous(previous);
        Ok(())
    }
    fn capture_object(&self) -> Result<Arc<ObjectState>, String> {
        active_mesh_ref!(self).validate()?;
        let stack = self
            .multiresolution
            .as_ref()
            .map(|s| s.reconciled(active_mesh_ref!(self)))
            .transpose()?;
        let layers = self
            .layers
            .as_ref()
            .map(|l| l.reconciled(active_mesh_ref!(self)))
            .transpose()?;
        if let Some(target) = &self.morph_target {
            Self::validate_morph_binding(target, active_mesh_ref!(self), stack.as_ref())?;
        }
        Ok(Arc::new(ObjectState::with_metadata(
            active_mesh_ref!(self).snapshot(),
            stack,
            layers,
            self.morph_target.clone(),
            self.active_metadata.clone(),
        )))
    }
    fn install_object(&mut self, payload: &ObjectState) {
        self.mesh = Some(payload.mesh.snapshot());
        self.active_metadata = payload.metadata.clone();
        self.multiresolution = payload
            .multiresolution
            .as_ref()
            .map(Multiresolution::snapshot);
        self.layers = payload.layers.clone();
        self.morph_target = payload.morph_target.clone();
    }
    fn check_scene_candidate(
        &self,
        mesh: &Mesh,
        stack: Option<&Multiresolution>,
        layers: Option<&LayerStack>,
        target: Option<&MorphTarget>,
        scene: Option<&Scene>,
    ) -> Result<(), String> {
        self.check_scene_candidate_inner(mesh, stack, layers, target, scene, (false, false, None))
    }

    fn check_scene_candidate_inner(
        &self,
        mesh: &Mesh,
        stack: Option<&Multiresolution>,
        layers: Option<&LayerStack>,
        target: Option<&MorphTarget>,
        scene: Option<&Scene>,
        active_validated: (bool, bool, Option<&SceneObjectMetadata>),
    ) -> Result<(), String> {
        let (active_mesh_validated, active_pose_validated, metadata_override) = active_validated;
        // Existing fixed-local operations retain the selected object's pose.
        // New-selection transactions validate immutable target payloads before
        // install; applying the old active pose to that target would be wrong.
        let candidate_metadata = metadata_override.or_else(|| {
            if scene.map_or(self.scene.is_none(), |candidate| {
                candidate.active_id == self.active_subtool_id()
            }) {
                self.active_metadata.as_ref()
            } else {
                None
            }
        });
        if let Some(metadata) = candidate_metadata {
            if !active_pose_validated {
                metadata.validate(mesh)?;
            }
        }
        if state_has_attributes(mesh, stack, scene) {
            ObjectBudget::for_state(mesh, stack, layers, target)
                .validate_with_derived_bytes(self.source_geometry_retained_bytes())?;
        }
        let Some(scene) = scene else { return Ok(()) };
        if !active_mesh_validated {
            mesh.validate()?;
        }
        if self.name.len() > MAX_NAME_BYTES {
            return Err("Project name exceeds 4096 bytes".into());
        }
        scene.validate_structure()?;
        let active = ObjectBudget::for_state(mesh, stack, layers, target);
        let mut aggregate = active.plus(scene.inactive_budget());
        aggregate.bytes = active
            .bytes
            .saturating_add(scene.approximate_bytes())
            .saturating_add(state_origin_storage(candidate_metadata, Some(scene)));
        aggregate.validate_with_derived_bytes(self.source_geometry_retained_bytes())?;
        let upper = active
            .encoded
            .saturating_add(scene.encoded_upper_bound())
            .saturating_add(self.name.len() * 6 + 1024)
            .saturating_add(candidate_metadata.map_or(0, |_| 2048));
        if upper <= MAX_INPUT_BYTES {
            return Ok(());
        }
        let reconciled = stack.map(|s| s.reconciled(mesh)).transpose()?;
        let project = ProjectRef {
            version: if state_has_source_masks(mesh, stack, Some(scene)) {
                MASK_CONVENTION_PROJECT_VERSION
            } else if state_has_attributes(mesh, stack, Some(scene)) {
                ATTRIBUTES_PROJECT_VERSION
            } else {
                SCENE_PROJECT_VERSION
            },
            name: &self.name,
            mesh: Some(mesh),
            multiresolution: reconciled.as_ref().map(Multiresolution::project_ref),
            layers: layers.map(LayerStack::project_ref),
            morph_target: target.map(MorphTarget::project_ref),
            scene: Some(scene.project_ref()),
            scene_v7: None,
        };
        let mut writer = OutputCounter { bytes: 0 };
        serde_json::to_writer(&mut writer, &project)
            .map_err(|e| format!("Cannot serialize bounded scene: {e}"))
    }

    pub fn has_morph_target(&self) -> bool {
        self.morph_target.is_some()
    }

    /// Cached target sizes and level availability; no stored-point scan is
    /// needed to draw this summary. Operations additionally check exact faces.
    pub fn morph_target_summary(&self) -> Option<crate::MorphTargetSummary> {
        self.morph_target.as_ref().map(|target| {
            target.summary(
                target.creation_level == self.current_level()
                    && self
                        .mesh
                        .as_ref()
                        .is_some_and(|mesh| target.positions.len() == mesh.positions.len()),
            )
        })
    }

    /// Replace the single target with the current level's exact geometry.
    pub fn store_morph_target(&mut self) -> Result<(), String> {
        self.finish_layer_stroke()?;
        crate::morph::validate_source(active_mesh_ref!(self))?;
        let current_mesh = active_mesh_ref!(self);
        if self
            .morph_target
            .as_ref()
            .is_some_and(|target| target.matches_surface(current_mesh, self.current_level()))
        {
            return Ok(());
        }
        let target = MorphTarget::new(active_mesh_ref!(self), self.current_level())?;
        let stack = self
            .multiresolution
            .as_ref()
            .map(|s| s.reconciled(active_mesh_ref!(self)))
            .transpose()?;
        Self::check_output_budget(
            active_mesh_ref!(self),
            &self.name,
            stack.as_ref(),
            self.layers.as_ref(),
            Some(&target),
        )?;
        self.check_scene_candidate(
            active_mesh_ref!(self),
            stack.as_ref(),
            self.layers.as_ref(),
            Some(&target),
            self.scene.as_ref(),
        )?;
        let previous = self.snapshot_state();
        self.morph_target = Some(target);
        self.commit_previous(previous);
        Ok(())
    }

    pub fn delete_morph_target(&mut self) -> Result<(), String> {
        self.finish_layer_stroke()?;
        if self.morph_target.is_none() {
            return Ok(());
        }
        let previous = self.snapshot_state();
        self.morph_target = None;
        self.commit_previous(previous);
        Ok(())
    }

    /// Restore the entire stored surface. Masks and colors stay on the mesh;
    /// local protection belongs to the Morph brush rather than this operation.
    pub fn restore_morph_target(&mut self) -> Result<(), String> {
        self.apply_morph_surface(false)
    }

    /// Exchange current and target geometry on the target's creation level.
    pub fn swap_morph_target(&mut self) -> Result<(), String> {
        self.apply_morph_surface(true)
    }

    fn apply_morph_surface(&mut self, swap: bool) -> Result<(), String> {
        self.finish_layer_stroke()?;
        let target = self.available_morph_target()?.clone();
        crate::morph::validate_source(active_mesh_ref!(self))?;
        if crate::morph::positions_equal_bits(&active_mesh_ref!(self).positions, &target.positions)
        {
            return Ok(());
        }
        let replacement_target = if swap {
            target.with_positions(active_mesh_ref!(self))?
        } else {
            target.clone()
        };
        let mut mesh = active_mesh_ref!(self).snapshot();
        mesh.positions.clone_from(&target.positions);
        mesh.recompute_normals();
        let layers = self
            .layers
            .as_ref()
            .map(|layers| layers.reconciled(&mesh))
            .transpose()?;
        let stack = self
            .multiresolution
            .as_ref()
            .map(|s| s.reconciled(&mesh))
            .transpose()?;
        Self::check_output_budget(
            &mesh,
            &self.name,
            stack.as_ref(),
            layers.as_ref(),
            Some(&replacement_target),
        )?;
        self.check_scene_candidate(
            &mesh,
            stack.as_ref(),
            layers.as_ref(),
            Some(&replacement_target),
            self.scene.as_ref(),
        )?;
        let previous = self.snapshot_state();
        self.mesh = Some(mesh);
        self.layers = layers;
        self.morph_target = Some(replacement_target);
        self.commit_previous(previous);
        Ok(())
    }

    /// A dab within begin_stroke/end_stroke joins that stroke's undo entry.
    /// With no open stroke, a useful dab commits one complete undo operation.
    pub fn apply_morph_brush_with_result(
        &mut self,
        center: crate::Vec3,
        normal: crate::Vec3,
        settings: &crate::BrushSettings,
    ) -> Result<crate::BrushResult, String> {
        if settings.kind != crate::BrushKind::Morph {
            return Err("The Morph Target path requires the Morph brush".into());
        }
        let target = self.available_morph_target()?.clone();
        // No-contact and invalid sample input must not allocate a whole
        // candidate mesh. Like the generic brush, these samples have no effect.
        if settings.effective_radius().is_none()
            || !center.is_finite()
            || !normal.is_finite()
            || normal.normalize_or_zero() == crate::Vec3::ZERO
            || !settings.strength.is_finite()
            || settings.strength <= 0.0
            || !settings.alpha_rotation.is_finite()
            || settings.alpha_tangent.is_some_and(|t| !t.is_finite())
            || settings.alpha.as_ref().is_some_and(|a| !a.has_signal())
        {
            return Ok(crate::BrushResult::default());
        }
        active_mesh_ref!(self).validate()?;
        if crate::morph::positions_equal_bits(&active_mesh_ref!(self).positions, &target.positions)
        {
            // These exact points/faces were checked when storing/importing the
            // immutable target; current attributes/normals were checked above.
            return Ok(crate::BrushResult::default());
        }
        crate::morph::validate_geometry(
            &active_mesh_ref!(self).positions,
            &active_mesh_ref!(self).triangles,
        )?;
        let mut candidate = active_mesh_ref!(self).snapshot();
        let result = candidate.apply_morph_positions_with_result(
            &target.positions,
            center,
            normal,
            settings,
        );
        if result.changed_vertices == 0 {
            return Ok(result);
        }
        crate::morph::validate_source(&candidate)?;
        // Preflight layer capture before making the visible edit. End-stroke
        // installs the metadata, preserving the host's accepted f32 surface.
        let layers = self
            .layers
            .as_ref()
            .map(|layers| layers.reconciled(&candidate))
            .transpose()?;
        self.check_scene_candidate(
            &candidate,
            self.multiresolution.as_ref(),
            layers.as_ref(),
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        let standalone = self.stroke_start.is_none();
        if standalone {
            self.begin_stroke();
        }
        self.mesh = Some(candidate);
        if standalone {
            self.finish_layer_stroke()?;
        }
        Ok(result)
    }

    fn available_morph_target(&self) -> Result<&MorphTarget, String> {
        let target = self
            .morph_target
            .as_ref()
            .ok_or("Store a Morph Target before using Morph")?;
        if target.creation_level != self.current_level() {
            return Err(format!(
                "Morph Target is available at subdivision level {}",
                target.creation_level + 1
            ));
        }
        if !target.topology_matches(active_mesh_ref!(self)) {
            return Err("Morph Target vertex/triangle correspondence has changed".into());
        }
        Ok(target)
    }

    fn validate_morph_binding(
        target: &MorphTarget,
        mesh: &Mesh,
        stack: Option<&Multiresolution>,
    ) -> Result<(), String> {
        let binding = match stack {
            Some(stack) if target.creation_level == stack.active => mesh,
            Some(stack) => stack
                .levels
                .get(target.creation_level)
                .ok_or("Morph Target creation level is not retained")?,
            None if target.creation_level == 0 => mesh,
            None => return Err("Morph Target creation level requires a retained hierarchy".into()),
        };
        if !target.topology_matches(binding) {
            return Err("Morph Target does not match its creation level's ordered topology".into());
        }
        crate::morph::validate_source(binding)
    }

    /// Geometry-only fixed-topology layers; masks and colors remain mesh channels.
    pub fn layer_summaries(&self) -> Vec<crate::LayerSummary> {
        self.layers
            .as_ref()
            .map_or_else(Vec::new, LayerStack::summaries)
    }

    pub fn layer_count(&self) -> usize {
        self.layers.as_ref().map_or(0, LayerStack::count)
    }
    pub fn recording_layer(&self) -> Option<usize> {
        self.layers.as_ref().and_then(LayerStack::recording)
    }
    pub fn layer_topology_locked(&self) -> bool {
        self.layer_count() != 0
    }

    /// Rejected raw public-mesh strokes restore their complete snapshot. Hosts
    /// should refresh the display and show this error after closing a stroke.
    pub fn take_layer_error(&mut self) -> Option<String> {
        self.layer_error.take()
    }

    pub fn add_layer(&mut self, name: &str) -> Result<usize, String> {
        self.finish_layer_stroke()?;
        let mut layers = match &self.layers {
            Some(l) => l.reconciled(active_mesh_ref!(self))?,
            None => LayerStack::new(active_mesh_ref!(self))?,
        };
        let index = layers.add(name)?;
        self.commit_layer_candidate(layers)?;
        Ok(index)
    }

    pub fn set_recording_layer(&mut self, index: Option<usize>) -> Result<(), String> {
        if index.is_none() && self.layers.is_none() {
            return Ok(());
        }
        let mut layers = self.current_layers()?;
        layers.set_recording(index)?;
        self.commit_layer_candidate(layers)
    }

    pub fn set_layer_intensity(&mut self, index: usize, intensity: f32) -> Result<(), String> {
        let mut layers = self.current_layers()?;
        layers.set_intensity(index, intensity)?;
        self.commit_layer_candidate(layers)
    }

    pub fn set_layer_visibility(&mut self, index: usize, visible: bool) -> Result<(), String> {
        let mut layers = self.current_layers()?;
        layers.set_visibility(index, visible)?;
        self.commit_layer_candidate(layers)
    }

    pub fn set_layer_name(&mut self, index: usize, name: &str) -> Result<(), String> {
        let mut layers = self.current_layers()?;
        layers.set_name(index, name)?;
        self.commit_layer_candidate(layers)
    }

    pub fn delete_layer(&mut self, index: usize) -> Result<(), String> {
        let mut layers = self.current_layers()?;
        layers.delete(index)?;
        self.commit_layer_candidate(layers)
    }

    /// Keep the currently visible composition and remove editable layer data.
    /// Existing subdivision levels reconcile to that composition before unlock.
    pub fn bake_layers(&mut self) -> Result<(), String> {
        self.finish_layer_stroke()?;
        let Some(layers) = &self.layers else {
            return Ok(());
        };
        layers.reconciled(active_mesh_ref!(self))?;
        let stack = self
            .multiresolution
            .as_ref()
            .map(|s| s.reconciled(active_mesh_ref!(self)))
            .transpose()?;
        Self::check_output_budget(
            active_mesh_ref!(self),
            &self.name,
            stack.as_ref(),
            None,
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            active_mesh_ref!(self),
            stack.as_ref(),
            None,
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        self.finish_layer_stroke()?;
        let previous = self.snapshot_state();
        self.layers = None;
        self.multiresolution = stack;
        self.commit_previous(previous);
        Ok(())
    }

    fn current_layers(&mut self) -> Result<LayerStack, String> {
        self.finish_layer_stroke()?;
        self.layers
            .as_ref()
            .ok_or_else(|| "The document has no geometry layers".into())
            .and_then(|l| l.reconciled(active_mesh_ref!(self)))
    }

    fn commit_layer_candidate(&mut self, mut layers: LayerStack) -> Result<(), String> {
        let mesh = layers.recomposed(active_mesh_ref!(self))?;
        let stack = self
            .multiresolution
            .as_ref()
            .map(|s| s.reconciled(&mesh))
            .transpose()?;
        Self::check_output_budget(
            &mesh,
            &self.name,
            stack.as_ref(),
            if layers.count() == 0 {
                None
            } else {
                Some(&layers)
            },
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            &mesh,
            stack.as_ref(),
            if layers.count() == 0 {
                None
            } else {
                Some(&layers)
            },
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        self.finish_layer_stroke()?;
        let previous = self.snapshot_state();
        self.mesh = Some(mesh);
        self.layers = if layers.count() == 0 {
            None
        } else {
            Some(layers)
        };
        // Retain the subdivision baseline while layers are editable. Bake/save
        // reconciles net changes; repeated toggles do not accumulate fine drift.
        self.commit_previous(previous);
        Ok(())
    }

    fn commit_previous(&mut self, previous: DocumentState) {
        self.prune_source_geometry_states();
        if !previous.matches(self) {
            self.undo.push_back(previous);
            self.redo.clear();
            self.trim_history();
        }
    }

    fn finish_layer_stroke(&mut self) -> Result<(), String> {
        let had_open_stroke = self.stroke_start.is_some();
        if had_open_stroke {
            if let Err(error) = self.require_active_mesh().and_then(Mesh::validate) {
                if let Some(previous) = self.stroke_start.take() {
                    self.exchange_state(previous);
                }
                self.layer_error = Some(error.clone());
                return Err(error);
            }
        }
        self.end_stroke();
        if had_open_stroke {
            if let Some(error) = &self.layer_error {
                return Err(error.clone());
            }
        }
        Ok(())
    }

    fn require_baked_layers(&self) -> Result<(), String> {
        if self.layer_topology_locked() {
            Err("Bake geometry layers before topology changes or subdivision navigation".into())
        } else {
            Ok(())
        }
    }

    /// Source-static triangle/quad level with pinned channel/UV stencils.
    /// This explicit legacy entry point retains OBrush world-offset detail;
    /// the studio's source-static entry point uses versioned frame transport.
    pub fn add_static_subdivision_level(
        &mut self,
        settings: &crate::StaticSubdivisionSettings,
    ) -> Result<(), String> {
        self.add_static_subdivision_level_impl(settings, false)
    }

    /// Add a source-navigation static level. New studio hierarchies use this
    /// versioned scheme; preexisting static-v1 projects keep their old policy.
    pub fn add_source_static_subdivision_level(
        &mut self,
        settings: &crate::StaticSubdivisionSettings,
    ) -> Result<(), String> {
        self.add_static_subdivision_level_impl(settings, true)
    }

    /// Recover one retained lower level from a structurally subdivided mesh.
    /// The original fine surface remains as the upper slot with all vertex and
    /// UV channels mapped to its canonical child order. Failed recognition or
    /// capacity checks leave the visible document and history unchanged.
    pub fn reverse_source_static_subdivision_level(
        &mut self,
        settings: &crate::ReverseSubdivisionSettings,
    ) -> Result<crate::ReverseSubdivisionReport, String> {
        self.require_baked_layers()?;
        self.finish_layer_stroke()?;
        let (stack, report) = match &self.multiresolution {
            Some(existing) => {
                existing.reversed_again_static_source(active_mesh_ref!(self), settings)?
            }
            None => {
                let (stack, report, _) = Multiresolution::reversed_first_static_source(
                    active_mesh_ref!(self),
                    settings,
                )?;
                (stack, report)
            }
        };
        let coarse = stack.levels[0].snapshot();
        Self::check_output_budget(
            &coarse,
            &self.name,
            Some(&stack),
            None,
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            &coarse,
            Some(&stack),
            None,
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        let previous = self.snapshot_state();
        self.mesh = Some(coarse);
        self.multiresolution = Some(stack);
        self.commit_previous(previous);
        Ok(report)
    }

    pub fn can_reverse_source_static_subdivision_level(&self) -> bool {
        self.multiresolution.as_ref().is_none_or(|stack| {
            stack.active == 0
                && matches!(
                    stack.scheme,
                    crate::multires::SubdivisionScheme::SculptglStaticSmoothV3
                        | crate::multires::SubdivisionScheme::SculptglStaticSmoothV4
                )
                && stack.levels.len() < crate::multires::MAX_SUBDIVISION_LEVELS
        })
    }

    fn add_static_subdivision_level_impl(
        &mut self,
        settings: &crate::StaticSubdivisionSettings,
        source_navigation: bool,
    ) -> Result<(), String> {
        self.require_baked_layers()?;
        self.finish_layer_stroke()?;
        let stack = match &self.multiresolution {
            None if source_navigation => {
                Multiresolution::first_static_source(active_mesh_ref!(self), settings)?
            }
            None => Multiresolution::first_static(active_mesh_ref!(self), settings)?,
            Some(stack) => {
                if !stack.static_settings_match(settings) {
                    return Err("Retained subdivision scheme cannot change; keep its mode or explicitly bake/remove levels".into());
                }
                stack.added(active_mesh_ref!(self))?
            }
        };
        let mesh = stack.levels[stack.active].snapshot();
        Self::check_output_budget(
            &mesh,
            &self.name,
            Some(&stack),
            None,
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            &mesh,
            Some(&stack),
            None,
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        let previous = self.snapshot_state();
        self.mesh = Some(mesh);
        self.multiresolution = Some(stack);
        self.commit_previous(previous);
        Ok(())
    }

    /// Return a retained static scheme without inferring it from face counts.
    pub fn retained_static_subdivision_settings(&self) -> Option<crate::StaticSubdivisionSettings> {
        let stack = self.multiresolution.as_ref()?;
        match stack.scheme {
            crate::multires::SubdivisionScheme::LoopWorldOffsetsV1 => None,
            crate::multires::SubdivisionScheme::SculptglStaticSmoothV1
            | crate::multires::SubdivisionScheme::SculptglStaticSmoothV2
            | crate::multires::SubdivisionScheme::SculptglStaticSmoothV3
            | crate::multires::SubdivisionScheme::SculptglStaticSmoothV4 => {
                Some(crate::StaticSubdivisionSettings {
                    linear_edges: false,
                })
            }
            crate::multires::SubdivisionScheme::SculptglStaticLinearEdgesV1
            | crate::multires::SubdivisionScheme::SculptglStaticLinearEdgesV2
            | crate::multires::SubdivisionScheme::SculptglStaticLinearEdgesV3 => {
                Some(crate::StaticSubdivisionSettings { linear_edges: true })
            }
        }
    }

    /// Add a persistent Loop level. The complete hierarchy is one undo entry.
    /// Aggregate mesh and encoded-project limits are checked before commit.
    pub fn add_subdivision_level(&mut self) -> Result<(), String> {
        self.require_baked_layers()?;
        self.finish_layer_stroke()?;
        let stack = match &self.multiresolution {
            Some(stack) => {
                if stack.scheme.is_static() {
                    return Err(
                        "Use the retained source-static subdivision mode for this hierarchy".into(),
                    );
                }
                stack.added(active_mesh_ref!(self))?
            }
            None => Multiresolution::first(active_mesh_ref!(self))?,
        };
        let active = stack.levels[stack.active].snapshot();
        Self::check_output_budget(
            &active,
            &self.name,
            Some(&stack),
            None,
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            &active,
            Some(&stack),
            None,
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        self.end_stroke();
        self.begin_stroke();
        self.mesh = Some(active);
        self.multiresolution = Some(stack);
        self.finish_layer_stroke()?;
        Ok(())
    }

    /// Navigation does not consume an undo entry. Low-level deltas propagate
    /// upward; fine edits remain on their own level when navigating downward.
    pub fn select_level(&mut self, level: usize) -> Result<(), String> {
        self.require_active_mesh()?;
        self.require_baked_layers()?;
        self.finish_layer_stroke()?;
        let Some(current) = &self.multiresolution else {
            return if level == 0 {
                Ok(())
            } else {
                Err("The document has no stored subdivision levels".into())
            };
        };
        if level >= current.levels.len() {
            return Err("Subdivision level index is out of range".into());
        }
        let mut stack = current.reconciled(active_mesh_ref!(self))?;
        stack.lower_source_even_rows_to(level)?;
        stack.active = level;
        let active = stack.levels[level].snapshot();
        Self::check_output_budget(
            &active,
            &self.name,
            Some(&stack),
            None,
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            &active,
            Some(&stack),
            None,
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        self.end_stroke();
        self.mesh = Some(active);
        self.multiresolution = Some(stack);
        Ok(())
    }

    pub fn subdivide(&mut self) -> Result<(), String> {
        self.require_baked_layers()?;
        self.finish_layer_stroke()?;
        let mut mesh = active_mesh_ref!(self).snapshot();
        mesh.subdivide()?;
        self.replace_mesh(mesh)
    }

    /// One smooth Loop refinement, recorded as a single undoable operation.
    pub fn subdivide_smooth(&mut self) -> Result<(), String> {
        self.require_baked_layers()?;
        self.finish_layer_stroke()?;
        let mut mesh = active_mesh_ref!(self).snapshot();
        mesh.subdivide_smooth()?;
        self.replace_mesh(mesh)
    }

    /// Compute and validate first; a failed operation preserves mesh and redo.
    pub fn remesh(
        &mut self,
        settings: &crate::RemeshSettings,
    ) -> Result<crate::RemeshReport, String> {
        self.require_baked_layers()?;
        self.finish_layer_stroke()?;
        let (mesh, report) = active_mesh_ref!(self).remeshed(settings)?;
        self.replace_mesh(mesh)?;
        Ok(report)
    }

    pub fn decimate(
        &mut self,
        settings: &crate::DecimateSettings,
    ) -> Result<crate::DecimateReport, String> {
        self.require_baked_layers()?;
        self.finish_layer_stroke()?;
        let (mesh, report) = active_mesh_ref!(self).decimated(settings)?;
        if report.collapses != 0 {
            self.replace_mesh(mesh)?;
        }
        Ok(report)
    }

    /// One mask-only transaction. Geometry, normals, topology caches and
    /// precise layer/target state stay untouched. Retained levels transport
    /// mask deltas through the existing reconciliation/navigation policy.
    pub fn filter_mask(
        &mut self,
        settings: &crate::MaskFilterSettings,
    ) -> Result<crate::MaskFilterReport, String> {
        self.finish_layer_stroke()?;
        if !self.active_subtool_visible() {
            return Err("Show the active SubTool before filtering its mask".into());
        }
        let (masks, report) = active_mesh_ref!(self).filtered_masks(settings)?;
        if report.changed_vertices == 0 {
            return Ok(report);
        }
        self.commit_mask_candidate(masks)?;
        Ok(report)
    }

    pub fn filter_mask_sculptgl(
        &mut self,
        operation: crate::MaskFilterOperation,
    ) -> Result<crate::MaskFilterReport, String> {
        self.finish_layer_stroke()?;
        if !self.active_subtool_visible() {
            return Err("Show the active SubTool before filtering its mask".into());
        }
        let (masks, report) = active_mesh_ref!(self).sculptgl_filtered_masks(operation)?;
        if report.changed_vertices != 0 {
            self.commit_mask_candidate(masks)?;
        }
        Ok(report)
    }

    /// Through-depth screen-region masking is vertex-only and has the same
    /// mask/history/retained-level transaction policy as the scalar filters.
    pub fn mask_region(
        &mut self,
        settings: &crate::MaskRegionSettings,
    ) -> Result<crate::MaskRegionReport, String> {
        self.finish_layer_stroke()?;
        if !self.active_subtool_visible() {
            return Err("Show the active SubTool before region masking".into());
        }
        let (masks, report) = active_mesh_ref!(self).region_masks(settings)?;
        if report.changed_vertices != 0 {
            self.commit_mask_candidate(masks)?;
        }
        Ok(report)
    }

    fn commit_mask_candidate(&mut self, masks: Vec<f32>) -> Result<(), String> {
        let mut candidate = active_mesh_ref!(self).snapshot();
        candidate.masks = masks;
        let stack = self
            .multiresolution
            .as_ref()
            .map(|stack| stack.reconciled(&candidate))
            .transpose()?;
        if let Some(layers) = &self.layers {
            layers.validate_composition(&candidate)?;
        }
        if let Some(target) = &self.morph_target {
            Self::validate_morph_binding(target, &candidate, stack.as_ref())?;
        }
        Self::check_output_budget(
            &candidate,
            &self.name,
            stack.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            &candidate,
            stack.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        let previous = self.snapshot_state();
        active_mesh_mut!(self).masks = candidate.masks;
        self.commit_previous(previous);
        Ok(())
    }

    /// Explicit representation conversion. Retained stacks must not be reinterpreted
    /// by converting only the displayed level; whole-hierarchy conversion is deferred.
    pub fn convert_mask_convention(
        &mut self,
        convention: crate::MaskConvention,
        policy: crate::MaskConversionPolicy,
    ) -> Result<crate::MaskConversionReport, String> {
        if convention != active_mesh_ref!(self).mask_convention && self.multiresolution.is_some() {
            return Err(
                "Mask convention conversion requires a plain mesh without retained levels".into(),
            );
        }
        let (candidate, report) =
            active_mesh_ref!(self).converted_mask_convention(convention, policy)?;
        if convention == active_mesh_ref!(self).mask_convention {
            return Ok(report);
        }
        if !self.active_subtool_visible() {
            return Err("Show the active SubTool before converting its mask convention".into());
        }
        if let Some(layers) = &self.layers {
            layers.validate_composition(&candidate)?;
        }
        if let Some(target) = &self.morph_target {
            Self::validate_morph_binding(target, &candidate, None)?;
        }
        Self::check_output_budget(
            &candidate,
            &self.name,
            None,
            self.layers.as_ref(),
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            &candidate,
            None,
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        if self.snapshot_state().approximate_bytes() > HISTORY_BUDGET_BYTES {
            return Err(
                "Mask conversion cannot retain its original undo state within the history budget"
                    .into(),
            );
        }
        self.finish_layer_stroke()?;
        let previous = self.snapshot_state();
        active_mesh_mut!(self).masks = candidate.masks;
        active_mesh_mut!(self).mask_convention = convention;
        self.commit_previous(previous);
        Ok(report)
    }

    /// Inspect the effective active object's masks without changing data/history.
    /// A host must finish its old captured gesture before calling this API.
    /// Reports include every retained level once; inactive SubTools are excluded.
    pub fn active_subtool_mask_conversion_report(
        &self,
        destination: crate::MaskConvention,
    ) -> Result<crate::MaskHierarchyConversionReport, String> {
        self.prepare_mask_hierarchy_conversion(destination)
            .map(|(_, report)| report)
    }

    /// Explicitly convert the complete active object's mask representation as
    /// one undoable transaction. No geometry/paint/UV/layer/target data is converted.
    /// This operation is separate from the legacy plain-only conversion API.
    pub fn convert_active_subtool_masks(
        &mut self,
        destination: crate::MaskConvention,
        policy: crate::MaskConversionPolicy,
    ) -> Result<crate::MaskHierarchyConversionReport, String> {
        let (mut stack, report) = self.prepare_mask_hierarchy_conversion(destination)?;
        if !report.changed {
            return Ok(report);
        }
        if policy == crate::MaskConversionPolicy::RequireRoundtripExact
            && report.roundtrip_changed_words != 0
        {
            return Err(format!(
                "Whole-object mask conversion would lose {} f32 words across {} levels on roundtrip; inspect the precision report before explicitly allowing loss",
                report.roundtrip_changed_words, report.levels.len()
            ));
        }

        // The already reconciled hierarchy is a bounded private candidate. Only
        // masks change here, so no new subdivision or channel allocation occurs.
        if let Some(stack) = &mut stack {
            for level in &mut stack.levels {
                for value in &mut level.masks {
                    *value = 1.0_f32 - *value;
                }
                level.mask_convention = destination;
            }
            stack.validate_budget()?;
        }
        let plain = if stack.is_none() {
            Some(
                self.require_active_mesh()?
                    .converted_mask_convention(
                        destination,
                        crate::MaskConversionPolicy::AllowAndReport,
                    )?
                    .0,
            )
        } else {
            None
        };
        let candidate = stack.as_ref().map_or_else(
            || {
                plain
                    .as_ref()
                    .expect("a plain candidate exists without a hierarchy")
            },
            |stack| &stack.levels[stack.active],
        );
        if let Some(layers) = &self.layers {
            layers.validate_composition(candidate)?;
        }
        if let Some(target) = &self.morph_target {
            Self::validate_morph_binding(target, candidate, stack.as_ref())?;
        }
        Self::check_output_budget(
            candidate,
            &self.name,
            stack.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            candidate,
            stack.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        // Keep the visible channel and retained baseline independent. Reserve
        // before capturing history/committing; at most one million f32 values.
        let mut masks = Vec::new();
        masks
            .try_reserve_exact(candidate.masks.len())
            .map_err(|_| "Cannot allocate the converted active mask channel")?;
        masks.extend_from_slice(&candidate.masks);
        let previous = self.snapshot_state();
        self.require_active_mesh_mut()?.masks = masks;
        self.require_active_mesh_mut()?.mask_convention = destination;
        self.multiresolution = stack;
        self.commit_previous(previous);
        Ok(report)
    }

    fn prepare_mask_hierarchy_conversion(
        &self,
        destination: crate::MaskConvention,
    ) -> Result<
        (
            Option<Multiresolution>,
            crate::MaskHierarchyConversionReport,
        ),
        String,
    > {
        if self.stroke_start.is_some() {
            return Err(
                "Finish the active sculpt gesture before converting mask representation".into(),
            );
        }
        if !self.active_subtool_visible() {
            return Err("Show the active SubTool before converting its mask convention".into());
        }
        self.require_active_mesh()?.validate()?;
        if let Some(stack) = &self.multiresolution {
            stack.validate_budget()?;
        }
        // Check the complete active state and original undo size before cloning
        // a hierarchy. Existing reconciliation/subdivision limits stay intact.
        let active_budget = ObjectBudget::for_state(
            self.require_active_mesh()?,
            self.multiresolution.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
        );
        active_budget.validate_with_derived_bytes(self.source_geometry_retained_bytes())?;
        if let Some(scene) = &self.scene {
            scene.validate_structure()?;
            let mut aggregate = active_budget.plus(scene.inactive_budget());
            aggregate.bytes = active_budget
                .bytes
                .saturating_add(scene.approximate_bytes())
                .saturating_add(state_origin_storage(
                    self.active_metadata.as_ref(),
                    Some(scene),
                ));
            aggregate.validate_with_derived_bytes(self.source_geometry_retained_bytes())?;
        }
        let undo_bytes = state_origin_storage(self.active_metadata.as_ref(), self.scene.as_ref())
            + self
                .active_metadata
                .as_ref()
                .map_or(0, |_| std::mem::size_of::<SceneObjectMetadata>())
            + std::mem::size_of::<DocumentState>()
            + self.require_active_mesh()?.approximate_bytes()
            + self.name.capacity()
            + self
                .multiresolution
                .as_ref()
                .map_or(0, Multiresolution::approximate_bytes)
            + self
                .layers
                .as_ref()
                .map_or(0, LayerStack::approximate_bytes)
            + self
                .morph_target
                .as_ref()
                .map_or(0, MorphTarget::approximate_bytes)
            + self.scene.as_ref().map_or(0, Scene::approximate_bytes)
            + self.source_geometry_retained_bytes();
        if undo_bytes > HISTORY_BUDGET_BYTES {
            return Err(
                "Mask conversion cannot retain its complete original undo state within the history budget"
                    .into(),
            );
        }
        // Reconcile in the original convention. Converting first would change
        // the meaning of parent residuals or fail the generic binding contract.
        let mut stack = self
            .multiresolution
            .as_ref()
            .map(|stack| stack.reconciled(self.require_active_mesh()?))
            .transpose()?;
        if let Some(stack) = &mut stack {
            // Legacy reconciliation regards +0/-0 protection as numerically
            // equal. This precision operation must still inspect and convert
            // the visible raw words exactly; it does not change legacy stencils.
            stack.levels[stack.active]
                .masks
                .copy_from_slice(&self.require_active_mesh()?.masks);
        }
        let active = stack.as_ref().map_or(self.require_active_mesh()?, |stack| {
            &stack.levels[stack.active]
        });
        if let Some(layers) = &self.layers {
            layers.validate_composition(active)?;
        }
        if let Some(target) = &self.morph_target {
            Self::validate_morph_binding(target, active, stack.as_ref())?;
        }
        Self::check_output_budget(
            active,
            &self.name,
            stack.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
        )?;
        self.check_scene_candidate(
            active,
            stack.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        let mut report = crate::MaskHierarchyConversionReport::new(
            self.active_subtool_id()
                .ok_or("No active SubTool mask representation")?,
            stack.is_some(),
            self.current_level(),
            self.require_active_mesh()?.mask_convention,
            destination,
        );
        report
            .levels
            .try_reserve_exact(self.level_count())
            .map_err(|_| "Cannot allocate mask conversion level reports")?;
        if let Some(stack) = &stack {
            for (level, mesh) in stack.levels.iter().enumerate() {
                report.push(mesh, level)?;
            }
        } else {
            report.push(active, 0)?;
        }
        Ok((stack, report))
    }

    pub fn clear_mask(&mut self) {
        self.end_stroke();
        if self.mesh.is_none() {
            return;
        }
        self.begin_stroke();
        if let Some(mesh) = &mut self.mesh {
            mesh.masks.fill(mesh.mask_convention.clear_value());
        }
        self.end_stroke();
    }

    pub fn invert_mask(&mut self) {
        self.end_stroke();
        if self.mesh.is_none() {
            return;
        }
        self.begin_stroke();
        if let Some(mesh) = &mut self.mesh {
            for mask in &mut mesh.masks {
                *mask = 1.0 - mask.clamp(0.0, 1.0);
            }
        }
        self.end_stroke();
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate_complete_state()?;
        let mesh = self.mesh.as_ref();
        let stack = match mesh {
            Some(mesh) => self
                .multiresolution
                .as_ref()
                .map(|s| s.reconciled(mesh))
                .transpose()?,
            None => None,
        };
        let layers = match mesh {
            Some(mesh) => self
                .layers
                .as_ref()
                .map(|l| l.reconciled(mesh))
                .transpose()?,
            None => None,
        };
        let metadata = self.scene_v7_project()?;
        let version = if metadata.is_some()
            || mesh.is_none()
            || mesh.is_some_and(|mesh| {
                state_has_source_masks(mesh, stack.as_ref(), self.scene.as_ref())
            }) {
            MASK_CONVENTION_PROJECT_VERSION
        } else if mesh
            .is_some_and(|mesh| state_has_attributes(mesh, stack.as_ref(), self.scene.as_ref()))
        {
            ATTRIBUTES_PROJECT_VERSION
        } else if self.scene.is_some() {
            SCENE_PROJECT_VERSION
        } else {
            project_version(layers.is_some(), self.morph_target.is_some())
        };
        let project = ProjectRef {
            version,
            name: &self.name,
            mesh,
            multiresolution: stack.as_ref().map(Multiresolution::project_ref),
            layers: layers.as_ref().map(LayerStack::project_ref),
            morph_target: self.morph_target.as_ref().map(MorphTarget::project_ref),
            scene: self.scene.as_ref().map(Scene::project_ref),
            scene_v7: metadata.as_ref(),
        };
        let mut output = LimitedJsonWriter {
            bytes: Vec::new(),
            limit: MAX_INPUT_BYTES,
        };
        serde_json::to_writer(&mut output, &project)
            .map_err(|e| format!("Cannot serialize project: {e}"))?;
        String::from_utf8(output.bytes).map_err(|e| format!("Cannot serialize UTF-8 project: {e}"))
    }

    pub fn from_json(source: &str) -> Result<Self, String> {
        if source.len() > MAX_INPUT_BYTES {
            return Err("Project exceeds the encoded project input limit".into());
        }
        let _decode_scope = crate::decode_limits::DecodeScope::enter();
        let project: Project = serde_json::from_str(source)
            .map_err(|error| format!("Invalid OBrush project: {error}"))?;
        if project.version != 1
            && project.version != PROJECT_VERSION
            && project.version != LAYERS_PROJECT_VERSION
            && project.version != MORPH_PROJECT_VERSION
            && project.version != SCENE_PROJECT_VERSION
            && project.version != ATTRIBUTES_PROJECT_VERSION
            && project.version != MASK_CONVENTION_PROJECT_VERSION
        {
            return Err(format!(
                "Unsupported OBrush project version {}",
                project.version
            ));
        }
        if project.name.len() > MAX_NAME_BYTES {
            return Err("Project name exceeds 4096 bytes".into());
        }
        if project.version < MASK_CONVENTION_PROJECT_VERSION && project_has_source_masks(&project) {
            return Err(
                "Source unprotected mask convention requires OBrush project version 7".into(),
            );
        }
        if project.version == 1 && project.multiresolution.is_some() {
            return Err("Version 1 projects cannot contain subdivision metadata".into());
        }
        if project.version < LAYERS_PROJECT_VERSION && project.layers.is_some() {
            return Err("Version 1/2 projects cannot contain geometry-layer metadata".into());
        }
        if project.version == LAYERS_PROJECT_VERSION && project.layers.is_none() {
            return Err("Version 3 projects require geometry-layer metadata".into());
        }
        if project.version < MORPH_PROJECT_VERSION && project.morph_target.is_some() {
            return Err("Version 1/2/3 projects cannot contain Morph Target metadata".into());
        }
        if project.version == MORPH_PROJECT_VERSION && project.morph_target.is_none() {
            return Err("Version 4 projects require Morph Target metadata".into());
        }
        if project.version < SCENE_PROJECT_VERSION && project.scene.is_some() {
            return Err("Versions 1–4 cannot contain SubTool scene metadata".into());
        }
        if project.version == SCENE_PROJECT_VERSION && project.scene.is_none() {
            return Err("Version 5 requires SubTool scene metadata".into());
        }
        if project.version < ATTRIBUTES_PROJECT_VERSION && project_has_attributes(&project) {
            return Err(
                "Mesh materials/original faces/UV metadata require OBrush project version 6".into(),
            );
        }
        if project.version < 7
            && (project.mesh.is_none()
                || project.scene_v7.is_some()
                || project
                    .scene
                    .as_ref()
                    .is_some_and(|scene| scene.active_id.is_none() || scene.objects.is_empty()))
        {
            return Err(
                "Absent active/empty/affine/origin scene semantics require OBrush project version7"
                    .into(),
            );
        }
        let mut document = match project.mesh {
            Some(mut mesh) => {
                mesh.validate()?;
                mesh.recompute_normals();
                Self::new(mesh)
            }
            None => Self::empty(),
        };
        if document.mesh.is_none()
            && (project.multiresolution.is_some()
                || project.layers.is_some()
                || project.morph_target.is_some()
                || project.scene.is_none())
        {
            return Err("Empty project requires an explicit empty scene and no active level/layer/target state".into());
        }
        document.name = project.name;
        document.multiresolution = project
            .multiresolution
            .map(|project| Multiresolution::from_project(project, document.require_active_mesh()?))
            .transpose()?;
        document.layers = project
            .layers
            .map(|p| LayerStack::from_project(p, document.require_active_mesh()?))
            .transpose()?;
        document.morph_target = project
            .morph_target
            .map(MorphTarget::from_project)
            .transpose()?;
        if let Some(target) = &document.morph_target {
            Self::validate_morph_binding(
                target,
                document.require_active_mesh()?,
                document.multiresolution.as_ref(),
            )?;
        }
        if let Some(scene) = project.scene {
            let mut objects = Vec::new();
            for object in scene.objects {
                let payload = match object.payload {
                    None => None,
                    Some(payload) => Some(Arc::new(Self::object_from_project(payload)?)),
                };
                objects.push(Subtool {
                    id: object.id,
                    name: object.name,
                    visible: object.visible,
                    payload,
                });
            }
            let scene = Scene {
                selected_ids: scene.active_id.into_iter().collect(),
                active_id: scene.active_id,
                next_id: scene.next_id,
                solo: scene.solo,
                objects,
            };
            scene.validate_structure()?;
            document.scene = Some(scene);
        }
        if let Some(v7) = project.scene_v7 {
            document.install_v7_project_metadata(v7)?;
        }
        document.validate_complete_state()?;
        Ok(document)
    }

    fn object_from_project(project: ObjectProject) -> Result<ObjectState, String> {
        project.mesh.validate()?;
        let mut mesh = project.mesh;
        mesh.recompute_normals();
        let stack = project
            .multiresolution
            .map(|p| Multiresolution::from_project(p, &mesh))
            .transpose()?;
        let layers = project
            .layers
            .map(|p| LayerStack::from_project(p, &mesh))
            .transpose()?;
        let target = project
            .morph_target
            .map(MorphTarget::from_project)
            .transpose()?;
        if let Some(target) = &target {
            Self::validate_morph_binding(target, &mesh, stack.as_ref())?;
        }
        Ok(ObjectState::new(mesh, stack, layers, target))
    }

    /// Borrowed scene members for consumer preflight; hidden objects are included
    /// without cloning active vertex arrays or fabricating an empty member.
    pub fn subtool_mesh(&self, id: u32) -> Result<&Mesh, String> {
        if Some(id) == self.active_subtool_id() {
            return self.require_active_mesh();
        }
        let scene = self.scene.as_ref().ok_or("Unknown SubTool ID")?;
        let object = &scene.objects[scene.index(id)?];
        object
            .payload
            .as_ref()
            .map(|p| p.mesh.as_ref())
            .ok_or_else(|| "Inactive SubTool has no stored mesh".into())
    }
    pub fn subtool_object_metadata(&self, id: u32) -> Result<Option<&SceneObjectMetadata>, String> {
        if Some(id) == self.active_subtool_id() {
            self.require_active_mesh()?;
            return Ok(self.active_metadata.as_ref());
        }
        let scene = self.scene.as_ref().ok_or("Unknown SubTool ID")?;
        let object = &scene.objects[scene.index(id)?];
        Ok(object
            .payload
            .as_ref()
            .ok_or("Inactive SubTool has no stored state")?
            .metadata
            .as_ref())
    }
    /// A scene-wide selection policy cannot change merely because a plain part
    /// became active. Conflicting declared source policies require an explicit
    /// caller choice rather than silently guessing a distance convention.
    pub fn preferred_scene_picking_policy(&self) -> Result<crate::SglPickingPolicy, String> {
        let mut policy = None;
        let mut consider = |metadata: Option<&SceneObjectMetadata>| -> Result<(), String> {
            if let Some(metadata) = metadata {
                if policy.is_some_and(|p| p != metadata.picking_policy) {
                    return Err("Scene has conflicting declared picking policies; choose an explicit scene policy".into());
                }
                policy = Some(metadata.picking_policy);
            }
            Ok(())
        };
        consider(self.active_metadata.as_ref())?;
        if let Some(scene) = &self.scene {
            for object in &scene.objects {
                if let Some(payload) = &object.payload {
                    consider(payload.metadata.as_ref())?;
                }
            }
        }
        Ok(policy.unwrap_or(crate::SglPickingPolicy::WorldDistanceV1))
    }

    fn sgl_export_views(&self) -> Result<Vec<crate::sgl_export::ObjectView<'_>>, String> {
        self.validate_complete_state()?;
        if self.stroke_start.is_some() {
            return Err("Finish the active stroke before SGL preflight".into());
        }
        let mut views = Vec::new();
        views
            .try_reserve_exact(self.subtool_count())
            .map_err(|_| "Cannot allocate bounded SGL member views")?;
        if let Some(scene) = &self.scene {
            for object in &scene.objects {
                let (mesh, metadata, levels, retained, layers, morph) =
                    if Some(object.id) == scene.active_id {
                        (
                            self.require_active_mesh()?,
                            self.active_metadata.as_ref(),
                            self.multiresolution.as_ref().map_or(1, |s| s.levels.len()),
                            self.multiresolution.is_some(),
                            self.layers.as_ref().map_or(0, |s| s.count()),
                            self.morph_target.is_some(),
                        )
                    } else {
                        let p = object
                            .payload
                            .as_ref()
                            .ok_or("Inactive SGL member has no state")?;
                        (
                            &*p.mesh,
                            p.metadata.as_ref(),
                            p.multiresolution.as_ref().map_or(1, |s| s.levels.len()),
                            p.multiresolution.is_some(),
                            p.layers.as_ref().map_or(0, |s| s.count()),
                            p.morph_target.is_some(),
                        )
                    };
                views.push(crate::sgl_export::ObjectView {
                    id: object.id,
                    name: &object.name,
                    visible: object.visible,
                    mesh,
                    metadata,
                    levels,
                    retained,
                    layers,
                    morph,
                });
            }
        } else if let Some(mesh) = self.active_mesh() {
            views.push(crate::sgl_export::ObjectView {
                id: 1,
                name: &self.name,
                visible: true,
                mesh,
                metadata: self.active_metadata.as_ref(),
                levels: self.multiresolution.as_ref().map_or(1, |s| s.levels.len()),
                retained: self.multiresolution.is_some(),
                layers: self.layers.as_ref().map_or(0, |s| s.count()),
                morph: self.morph_target.is_some(),
            });
        }
        Ok(views)
    }
    pub(crate) fn scoped_sgl_export_views(
        &self,
        scope: crate::SglExportScope,
    ) -> Result<Vec<crate::sgl_export::ObjectView<'_>>, String> {
        let mut views = self.sgl_export_views()?;
        if scope == crate::SglExportScope::SelectedParts {
            let selected = self.selected_subtool_ids();
            if selected.is_empty() {
                return Err("Select at least one part before selected SGL export".into());
            }
            let mut ordered = Vec::new();
            ordered
                .try_reserve_exact(selected.len())
                .map_err(|_| "Cannot reserve selected SGL export views")?;
            for id in selected {
                let index = views
                    .iter()
                    .position(|view| view.id == id)
                    .ok_or("SGL selected export contains an unknown part ID")?;
                ordered.push(views.remove(index));
            }
            views = ordered;
        }
        Ok(views)
    }
    pub fn plan_sgl_export(
        &self,
        settings: &crate::SglExportSettings,
    ) -> Result<crate::SglExportReport, String> {
        let views = self.scoped_sgl_export_views(settings.scope)?;
        crate::sgl_export::plan(
            &views,
            settings,
            self.scene_origin.as_ref(),
            &self.selected_subtool_ids(),
            self.active_subtool_id(),
            self.solo(),
        )
    }
    /// History-neutral encoder. Any non-representable fields are reported before
    /// mapped channels are allocated, and rejection leaves all source state intact.
    pub fn export_sgl(
        &self,
        settings: &crate::SglExportSettings,
    ) -> Result<crate::SglExport, String> {
        let views = self.scoped_sgl_export_views(settings.scope)?;
        let report = crate::sgl_export::plan(
            &views,
            settings,
            self.scene_origin.as_ref(),
            &self.selected_subtool_ids(),
            self.active_subtool_id(),
            self.solo(),
        )?;
        crate::sgl_export::export(&views, settings, report)
    }

    pub fn active_object_metadata(&self) -> Option<&SceneObjectMetadata> {
        self.active_metadata.as_ref()
    }
    /// Edit SculptGL's normalized X symmetry offset on only the active Source
    /// object. The older absolute BrushSettings offset is deliberately separate.
    pub fn set_active_source_symmetry_offset(&mut self, offset: f64) -> Result<bool, String> {
        if !offset.is_finite() || !(-1.0..=1.0).contains(&offset) {
            return Err("Source symmetry offset must be finite and in [-1, 1]".into());
        }
        let mut candidate = self
            .active_metadata
            .clone()
            .ok_or("Active part has no Source object metadata")?;
        candidate.symmetry_offset = offset;
        candidate.validate(self.require_active_mesh()?)?;
        if self
            .active_metadata
            .as_ref()
            .is_some_and(|current| current.same_bits(&candidate))
        {
            return Ok(false);
        }
        self.finish_layer_stroke()?;
        self.check_scene_candidate(
            self.require_active_mesh()?,
            self.multiresolution.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        let previous = self.snapshot_state();
        self.active_metadata = Some(candidate);
        self.commit_previous(previous);
        Ok(true)
    }
    /// Explicit object appearance edit. Raw SGL origin is immutable provenance;
    /// this stores the editable render record and one complete scene undo state.
    pub fn set_active_source_render(
        &mut self,
        render: crate::SglRenderSettings,
    ) -> Result<bool, String> {
        let mut candidate = self
            .active_metadata
            .clone()
            .ok_or("Active part has no source appearance metadata")?;
        candidate.render = Some(render);
        candidate.validate(self.require_active_mesh()?)?;
        if self
            .active_metadata
            .as_ref()
            .is_some_and(|current| current.same_bits(&candidate))
        {
            return Ok(false);
        }
        self.finish_layer_stroke()?;
        candidate.validate(self.require_active_mesh()?)?;
        self.check_scene_candidate(
            self.require_active_mesh()?,
            self.multiresolution.as_ref(),
            self.layers.as_ref(),
            self.morph_target.as_ref(),
            self.scene.as_ref(),
        )?;
        let previous = self.snapshot_state();
        self.active_metadata = Some(candidate);
        self.commit_previous(previous);
        Ok(true)
    }
    /// Apply the Source wireframe controller to the current selection as one
    /// document transaction. A mixed selection takes the requested checkbox
    /// value, rather than inverting every part independently.
    pub fn set_selected_source_wireframe(&mut self, visible: bool) -> Result<bool, String> {
        self.edit_selected_subtools(|candidate| {
            let mut render = candidate
                .active_object_metadata()
                .ok_or("Selected part has no source appearance metadata")?
                .render
                .clone()
                .unwrap_or_else(crate::baseline_source_render_defaults);
            render.show_wireframe = visible;
            candidate.set_active_source_render(render)
        })
    }
    /// SculptGL's Transparency control applies one inverse-opacity value to
    /// every selected source mesh. Keep the selection edit atomic so a mixed
    /// or invalid scene cannot leave only some objects changed.
    pub fn set_selected_source_transparency(&mut self, percent: u32) -> Result<bool, String> {
        if percent > 100 {
            return Err("Source transparency must be between 0 and 100 percent".into());
        }
        let opacity = 1.0 - percent as f32 / 100.0;
        self.edit_selected_subtools(|candidate| {
            let mut render = candidate
                .active_object_metadata()
                .ok_or("Selected part has no source appearance metadata")?
                .render
                .clone()
                .unwrap_or_else(crate::baseline_source_render_defaults);
            render.opacity = opacity;
            candidate.set_active_source_render(render)
        })
    }
    /// Apply flat shading to every selected Source part atomically. The Source
    /// checkbox supplies one value for the selection, even if its prior values
    /// differ; plain parts cannot acquire Source render data implicitly.
    pub fn set_selected_source_flat_shading(&mut self, flat: bool) -> Result<bool, String> {
        self.edit_selected_subtools(|candidate| {
            let mut render = candidate
                .active_object_metadata()
                .ok_or("Selected part has no source appearance metadata")?
                .render
                .clone()
                .unwrap_or_else(crate::baseline_source_render_defaults);
            render.flat_shading = flat;
            candidate.set_active_source_render(render)
        })
    }
    pub fn scene_origin(&self) -> Option<&SglSceneOrigin> {
        self.scene_origin.as_ref()
    }
    pub fn selected_subtool_ids(&self) -> Vec<u32> {
        self.scene.as_ref().map_or_else(
            || self.active_subtool_id().into_iter().collect(),
            |s| s.selected_ids.clone(),
        )
    }
    /// Selection/navigation follows existing policy: no new Undo entry. Geometry
    /// transactions still snapshot the complete selected/active scene state.
    pub fn select_subtools(&mut self, ids: &[u32], active: Option<u32>) -> Result<(), String> {
        if ids.len() > crate::MAX_SUBTOOLS {
            return Err("Scene selection exceeds32 IDs".into());
        }
        if self.selected_subtool_ids() == ids && self.active_subtool_id() == active {
            return Ok(());
        }
        let mut seen = std::collections::HashSet::new();
        let scene = self.scene.clone().unwrap_or_else(|| {
            if self.mesh.is_some() {
                Scene::initial(&self.name)
            } else {
                Scene::empty()
            }
        });
        for &id in ids {
            scene.index(id)?;
            if !seen.insert(id) {
                return Err("Duplicate selected object ID".into());
            }
        }
        if scene.objects.is_empty() {
            if !ids.is_empty() || active.is_some() {
                return Err("Empty scene has no selection".into());
            }
            return Ok(());
        }
        let active = active.ok_or("Nonempty scene requires an active object")?;
        if !seen.contains(&active) {
            return Err("Active object must be selected".into());
        }
        self.finish_layer_stroke()?;
        if self.active_subtool_id() != Some(active) {
            self.select_subtool(active)?;
        }
        let mut scene = self
            .scene
            .clone()
            .unwrap_or_else(|| Scene::initial(&self.name));
        scene.selected_ids = ids.to_vec();
        scene.validate_structure()?;
        self.scene = Some(scene);
        Ok(())
    }
    /// One undoable true-zero-object transition; retained history contains the
    /// original geometry, poses, source records, layers, targets and scene header.
    pub fn clear_scene(&mut self) -> Result<bool, String> {
        self.finish_layer_stroke()?;
        if self.mesh.is_none() && self.subtool_count() == 0 {
            return Ok(false);
        }
        if self.snapshot_state().approximate_bytes() > HISTORY_BUDGET_BYTES {
            return Err(
                "Clear cannot retain the complete original scene within the history budget".into(),
            );
        }
        let previous = self.snapshot_state();
        let next_id = self.scene.as_ref().map_or(2, |s| s.next_id);
        self.mesh = None;
        self.active_metadata = None;
        self.multiresolution = None;
        self.layers = None;
        self.morph_target = None;
        let mut empty = Scene::empty();
        empty.next_id = next_id;
        self.scene = Some(empty);
        self.commit_previous(previous);
        Ok(true)
    }
    /// Consume a fully validated staging scene only after all output/storage
    /// checks pass. Caller-supplied staged IDs/order and every raw origin survive.
    /// Host camera/appearance application remains an explicit adapter obligation.
    pub fn install_staged_sgl_scene(
        &mut self,
        staged: crate::StagedSglScene,
    ) -> Result<(), String> {
        staged.validate()?;
        let mut imported = Self::empty();
        imported.name = self.name.clone();
        imported.scene_origin = Some(staged.origin.clone());
        let mut scene = Scene {
            active_id: staged.active_id,
            selected_ids: staged.selected_ids,
            next_id: staged.next_id,
            solo: false,
            objects: Vec::new(),
        };
        for object in staged.objects {
            let metadata = SceneObjectMetadata {
                source_center_initialized: object.source_center_initialized,
                symmetry_offset: 0.0,
                affine: object.affine,
                render: object.render,
                picking_policy: staged.picking_policy,
                origin: Some(crate::SceneObjectOrigin {
                    header: staged.origin.clone(),
                    object: object.origin,
                }),
            };
            metadata.validate(&object.mesh)?;
            let payload = if Some(object.id) == scene.active_id {
                imported.mesh = Some(object.mesh);
                imported.active_metadata = Some(metadata);
                None
            } else {
                Some(Arc::new(ObjectState::with_metadata(
                    object.mesh,
                    None,
                    None,
                    None,
                    Some(metadata),
                )))
            };
            scene.objects.push(Subtool {
                id: object.id,
                name: format!("SGL object {}", object.id),
                visible: true,
                payload,
            });
        }
        scene.validate_structure()?;
        imported.scene = Some(scene);
        self.replace_document(imported)
    }
    pub fn replace_sgl_bytes(
        &mut self,
        source: &[u8],
        settings: &crate::SglStageSettings,
    ) -> Result<(), String> {
        let staged = crate::StagedSglScene::from_sgl_bytes(source, settings)?;
        self.install_staged_sgl_scene(staged)
    }
    fn scene_v7_project(&self) -> Result<Option<SceneV7Project>, String> {
        let selected = self.selected_subtool_ids();
        let default_selected = self.active_subtool_id().into_iter().collect::<Vec<_>>();
        let mut objects = Vec::new();
        if let (Some(id), Some(metadata)) =
            (self.active_subtool_id(), self.active_metadata.as_ref())
        {
            objects.push((id, metadata));
        }
        if let Some(scene) = &self.scene {
            for object in &scene.objects {
                if let Some(metadata) = object.payload.as_ref().and_then(|p| p.metadata.as_ref()) {
                    objects.push((object.id, metadata));
                }
            }
        }
        if self.mesh.is_some()
            && objects.is_empty()
            && self.scene_origin.is_none()
            && selected == default_selected
        {
            return Ok(None);
        }
        // Preserve scene order rather than pool allocation/active ordering.
        if let Some(scene) = &self.scene {
            objects.sort_by_key(|(id, _)| {
                scene
                    .objects
                    .iter()
                    .position(|o| o.id == *id)
                    .unwrap_or(usize::MAX)
            });
        }
        Ok(Some(SceneV7Project::from_metadata(
            &selected,
            self.scene_origin.as_ref(),
            &objects,
        )?))
    }
    fn install_v7_project_metadata(&mut self, project: SceneV7Project) -> Result<(), String> {
        let (selected, header, objects) = project.into_metadata()?;
        let mut scene = self
            .scene
            .clone()
            .ok_or("v7 scene metadata requires an explicit scene")?;
        scene.selected_ids = selected;
        scene.validate_structure()?;
        for (id, metadata) in objects {
            let index = scene.index(id)?;
            if Some(id) == scene.active_id {
                metadata.validate(self.require_active_mesh()?)?;
                self.active_metadata = Some(metadata);
            } else {
                let prior = scene.objects[index]
                    .payload
                    .as_ref()
                    .ok_or("Inactive scene metadata requires payload")?;
                metadata.validate(&prior.mesh)?;
                scene.objects[index].payload = Some(Arc::new(ObjectState::with_metadata(
                    prior.mesh.snapshot(),
                    prior
                        .multiresolution
                        .as_ref()
                        .map(Multiresolution::snapshot),
                    prior.layers.clone(),
                    prior.morph_target.clone(),
                    Some(metadata),
                )));
            }
        }
        self.scene = Some(scene);
        self.scene_origin = header;
        Ok(())
    }
    fn validate_scene_metadata(&self) -> Result<(), String> {
        self.validate_scene_metadata_with_admitted_active(false)
    }

    /// End-stroke continuation only: the active mesh and its pose have already
    /// passed the full preflight under this same mutable Document borrow.
    fn validate_scene_metadata_with_admitted_active(
        &self,
        active_mesh_and_pose_validated: bool,
    ) -> Result<(), String> {
        if let Some(header) = &self.scene_origin {
            crate::scene_v7::validate_header(header)?;
        }
        if let Some(metadata) = &self.active_metadata {
            if !active_mesh_and_pose_validated {
                metadata.validate(self.require_active_mesh()?)?;
            }
        }
        let mut origins = Vec::new();
        let (mut raw_points, mut raw_faces, mut raw_uv, mut raw_bytes) =
            (0usize, 0usize, 0usize, 0usize);
        let mut consider = |metadata: &SceneObjectMetadata,
                            mesh: &Mesh,
                            already_validated: bool|
         -> Result<(), String> {
            if !already_validated {
                metadata.validate(mesh)?;
            }
            if let Some(origin) = &metadata.origin {
                let key = Arc::as_ptr(&origin.object.raw) as usize;
                if !origins.contains(&key) {
                    origins.push(key);
                    raw_points = raw_points
                        .checked_add(origin.object.raw.positions.len())
                        .ok_or("Origin point overflow")?;
                    for face in 0..origin.object.raw.faces.len() {
                        raw_faces = raw_faces
                            .checked_add(
                                origin.object.raw.face_arity(face, origin.object.version)? - 2,
                            )
                            .ok_or("Origin face overflow")?;
                    }
                    raw_uv = raw_uv
                        .checked_add(
                            origin
                                .object
                                .raw
                                .uv
                                .as_ref()
                                .map_or(0, |uv| uv.coordinates.len()),
                        )
                        .ok_or("Origin UV overflow")?;
                    raw_bytes = raw_bytes
                        .checked_add(origin.object.raw.storage_bytes()?)
                        .ok_or("Origin storage overflow")?;
                }
            }
            Ok(())
        };
        if let (Some(metadata), Some(mesh)) = (&self.active_metadata, &self.mesh) {
            consider(metadata, mesh, active_mesh_and_pose_validated)?;
        }
        if let Some(scene) = &self.scene {
            for object in &scene.objects {
                if let Some(payload) = &object.payload {
                    if let Some(metadata) = &payload.metadata {
                        consider(metadata, &payload.mesh, false)?;
                    }
                }
            }
        }
        let editable = self
            .mesh
            .as_ref()
            .map_or(0, |mesh| {
                ObjectBudget::for_state(
                    mesh,
                    self.multiresolution.as_ref(),
                    self.layers.as_ref(),
                    self.morph_target.as_ref(),
                )
                .bytes
            })
            .saturating_add(self.scene.as_ref().map_or(0, |s| s.inactive_budget().bytes));
        if raw_points > crate::MAX_VERTICES
            || raw_faces > crate::MAX_TRIANGLES
            || raw_uv > crate::MAX_UV_COORDINATES
            || editable.saturating_add(raw_bytes) > crate::resource_limits::MAX_SCENE_STORAGE_BYTES
        {
            return Err("Editable scene plus unique raw origins exceed aggregate limits".into());
        }
        Ok(())
    }
    fn validate_complete_state(&self) -> Result<(), String> {
        if self.name.len() > MAX_NAME_BYTES {
            return Err("Project name exceeds4096 bytes".into());
        }
        match &self.mesh {
            None => {
                if self.multiresolution.is_some()
                    || self.layers.is_some()
                    || self.morph_target.is_some()
                    || self.active_metadata.is_some()
                {
                    return Err("Empty scene cannot retain active editable payload".into());
                }
                let scene = self
                    .scene
                    .as_ref()
                    .ok_or("Empty document requires explicit zero-object Scene")?;
                scene.validate_structure()?;
                if !scene.objects.is_empty() {
                    return Err("Absent active mesh requires zero objects".into());
                }
            }
            Some(mesh) => {
                mesh.validate()?;
                let stack = self
                    .multiresolution
                    .as_ref()
                    .map(|s| s.reconciled(mesh))
                    .transpose()?;
                let layers = self
                    .layers
                    .as_ref()
                    .map(|l| l.reconciled(mesh))
                    .transpose()?;
                if let Some(target) = &self.morph_target {
                    Self::validate_morph_binding(target, mesh, stack.as_ref())?;
                }
                if self
                    .scene
                    .as_ref()
                    .is_some_and(|s| s.objects.is_empty() || s.active_id.is_none())
                {
                    return Err("Active mesh requires an active scene member".into());
                }
                self.check_scene_candidate(
                    mesh,
                    stack.as_ref(),
                    layers.as_ref(),
                    self.morph_target.as_ref(),
                    self.scene.as_ref(),
                )?;
                Self::check_output_budget(
                    mesh,
                    &self.name,
                    stack.as_ref(),
                    layers.as_ref(),
                    self.morph_target.as_ref(),
                )?;
            }
        }
        self.validate_scene_metadata()
    }
    fn snapshot_state(&self) -> DocumentState {
        DocumentState {
            mesh: self.mesh.as_ref().map(Mesh::snapshot),
            name: self.name.clone(),
            multiresolution: self.multiresolution.as_ref().map(Multiresolution::snapshot),
            layers: self.layers.clone(),
            morph_target: self.morph_target.clone(),
            scene: self.scene.clone(),
            active_metadata: self.active_metadata.clone(),
            scene_origin: self.scene_origin.clone(),
            source_geometry_states: self.source_geometry_states.clone(),
            source_history: None,
        }
    }

    fn exchange_state(&mut self, mut state: DocumentState) -> DocumentState {
        self.material_paint_stroke = false;
        self.source_geometry_active = None;
        if let Some(mesh) = &mut self.mesh {
            mesh.clear_cache();
        }
        std::mem::swap(&mut self.mesh, &mut state.mesh);
        std::mem::swap(&mut self.name, &mut state.name);
        std::mem::swap(&mut self.multiresolution, &mut state.multiresolution);
        std::mem::swap(&mut self.layers, &mut state.layers);
        std::mem::swap(&mut self.morph_target, &mut state.morph_target);
        std::mem::swap(&mut self.scene, &mut state.scene);
        std::mem::swap(&mut self.active_metadata, &mut state.active_metadata);
        std::mem::swap(&mut self.scene_origin, &mut state.scene_origin);
        std::mem::swap(
            &mut self.source_geometry_states,
            &mut state.source_geometry_states,
        );
        state
    }

    /// Count encoded bytes without retaining another full JSON allocation.
    fn check_output_budget(
        mesh: &Mesh,
        name: &str,
        stack: Option<&Multiresolution>,
        layers: Option<&LayerStack>,
        morph_target: Option<&MorphTarget>,
    ) -> Result<(), String> {
        if name.len() > MAX_NAME_BYTES {
            return Err("Project name exceeds 4096 bytes".into());
        }
        let mut upper = mesh_encoded_upper_bound(mesh).saturating_add(name.len() * 6 + 1024);
        if let Some(stack) = stack {
            upper = upper.saturating_add(1024);
            for (index, level) in stack.levels.iter().enumerate() {
                if index != stack.active {
                    upper = upper.saturating_add(mesh_encoded_upper_bound(level));
                }
            }
        }
        if let Some(layers) = layers {
            upper = upper.saturating_add(layers.encoded_upper_bound());
        }
        if let Some(target) = morph_target {
            upper = upper.saturating_add(target.encoded_upper_bound());
        }
        if upper <= MAX_INPUT_BYTES {
            return Ok(());
        }
        struct Counter {
            bytes: usize,
        }
        impl std::io::Write for Counter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.bytes = self
                    .bytes
                    .checked_add(bytes.len())
                    .ok_or_else(|| std::io::Error::other("Project output-size overflow"))?;
                if self.bytes > MAX_INPUT_BYTES {
                    return Err(std::io::Error::other(
                        "Project exceeds the encoded project output limit",
                    ));
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let project = ProjectRef {
            version: if state_has_source_masks(mesh, stack, None) {
                MASK_CONVENTION_PROJECT_VERSION
            } else if state_has_attributes(mesh, stack, None) {
                ATTRIBUTES_PROJECT_VERSION
            } else {
                project_version(layers.is_some(), morph_target.is_some())
            },
            name,
            mesh: Some(mesh),
            multiresolution: stack.map(Multiresolution::project_ref),
            layers: layers.map(LayerStack::project_ref),
            morph_target: morph_target.map(MorphTarget::project_ref),
            scene: None,
            scene_v7: None,
        };
        serde_json::to_writer(&mut Counter { bytes: 0 }, &project)
            .map_err(|error| format!("Cannot serialize project: {error}"))
    }

    fn trim_history(&mut self) {
        self.trim_history_to(HISTORY_BUDGET_BYTES, self.history_limit);
    }

    fn trim_history_to(&mut self, byte_budget: usize, max_entries: usize) {
        let mut bytes: usize = self
            .undo
            .iter()
            .chain(self.redo.iter())
            .map(DocumentState::approximate_bytes)
            .fold(0, usize::saturating_add);
        while bytes > byte_budget || self.undo.len() + self.redo.len() > max_entries {
            let dropped = if self.undo.len() >= self.redo.len() {
                self.undo.pop_front()
            } else {
                self.redo.pop_front()
            };
            let Some(dropped) = dropped else {
                break;
            };
            bytes = bytes.saturating_sub(dropped.approximate_bytes());
        }
    }
}

/// Each immutable original is charged once per complete history state. Repeated
/// object references to one Arc do not cause overflow or evict a valid own Undo.
fn state_origin_storage(active: Option<&SceneObjectMetadata>, scene: Option<&Scene>) -> usize {
    let mut seen = Vec::new();
    let mut bytes = 0usize;
    let mut consider = |metadata: &SceneObjectMetadata| {
        if let Some(origin) = &metadata.origin {
            let identity = Arc::as_ptr(&origin.object.raw) as usize;
            if !seen.contains(&identity) {
                seen.push(identity);
                bytes = bytes
                    .saturating_add(std::mem::size_of::<crate::SglMesh>())
                    .saturating_add(origin.object.raw.storage_bytes().unwrap_or(usize::MAX));
            }
        }
    };
    if let Some(active) = active {
        consider(active);
    }
    if let Some(scene) = scene {
        for object in &scene.objects {
            if let Some(metadata) = object.payload.as_ref().and_then(|p| p.metadata.as_ref()) {
                consider(metadata);
            }
        }
    }
    bytes
}

fn state_has_attributes(
    mesh: &Mesh,
    stack: Option<&Multiresolution>,
    scene: Option<&Scene>,
) -> bool {
    !mesh.mask_convention.is_default()
        || mesh.has_attributes()
        || stack.is_some_and(|s| {
            s.levels.len() == 1
                || s.scheme.is_static()
                || s.levels
                    .iter()
                    .any(|m| m.has_attributes() || !m.mask_convention.is_default())
        })
        || scene.is_some_and(|s| {
            s.objects
                .iter()
                .filter_map(|o| o.payload.as_ref())
                .any(|p| state_has_attributes(&p.mesh, p.multiresolution.as_ref(), None))
        })
}
fn project_has_attributes(project: &Project) -> bool {
    fn object(mesh: &Mesh, stack: Option<&crate::multires::MultiresProject>) -> bool {
        mesh.has_attributes()
            || stack.is_some_and(|s| {
                s.levels.len() == 1
                    || s.scheme.is_static()
                    || s.levels.iter().flatten().any(Mesh::has_attributes)
            })
    }
    project
        .mesh
        .as_ref()
        .is_some_and(|mesh| object(mesh, project.multiresolution.as_ref()))
        || project.scene.as_ref().is_some_and(|scene| {
            scene
                .objects
                .iter()
                .filter_map(|o| o.payload.as_ref())
                .any(|p| object(&p.mesh, p.multiresolution.as_ref()))
        })
}

fn state_has_source_masks(
    mesh: &Mesh,
    stack: Option<&Multiresolution>,
    scene: Option<&Scene>,
) -> bool {
    !mesh.mask_convention.is_default()
        || stack.is_some_and(|s| s.levels.iter().any(|m| !m.mask_convention.is_default()))
        || scene.is_some_and(|s| {
            s.objects
                .iter()
                .filter_map(|o| o.payload.as_ref())
                .any(|p| state_has_source_masks(&p.mesh, p.multiresolution.as_ref(), None))
        })
}
fn project_has_source_masks(project: &Project) -> bool {
    fn object(mesh: &Mesh, stack: Option<&crate::multires::MultiresProject>) -> bool {
        !mesh.mask_convention.is_default()
            || stack.is_some_and(|s| {
                s.levels
                    .iter()
                    .flatten()
                    .any(|m| !m.mask_convention.is_default())
            })
    }
    project
        .mesh
        .as_ref()
        .is_some_and(|mesh| object(mesh, project.multiresolution.as_ref()))
        || project.scene.as_ref().is_some_and(|s| {
            s.objects
                .iter()
                .filter_map(|o| o.payload.as_ref())
                .any(|p| object(&p.mesh, p.multiresolution.as_ref()))
        })
}

fn project_version(has_layers: bool, has_morph_target: bool) -> u32 {
    if has_morph_target {
        MORPH_PROJECT_VERSION
    } else if has_layers {
        LAYERS_PROJECT_VERSION
    } else {
        PROJECT_VERSION
    }
}

/// Allow 24 bytes per float32 scalar, including scientific notation. Per vertex,
/// three vec3 rows cost 3*(3*24 + 2 commas + 2 brackets + 1 row separator),
/// and the mask costs 24 + 1 separator: 256 bytes. A face row permits three
/// ten-digit u32 indices, two commas, two brackets and a separator: 35 bytes.
fn mesh_encoded_upper_bound(mesh: &Mesh) -> usize {
    mesh.positions
        .len()
        .saturating_mul(256)
        .saturating_add(mesh.triangles.len().saturating_mul(35))
        .saturating_add(512)
        .saturating_add(mesh.attribute_encoded_bound())
}

pub(crate) fn mesh_channels_equal_bits(a: &Mesh, b: &Mesh) -> bool {
    a.attributes_equal_bits(b)
        && a.positions
            .iter()
            .zip(&b.positions)
            .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits))
        && a.masks
            .iter()
            .zip(&b.masks)
            .all(|(a, b)| a.to_bits() == b.to_bits())
        && a.colors
            .iter()
            .zip(&b.colors)
            .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
}

#[cfg(test)]
mod memory_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn scene_stroke_end_reuses_one_active_mesh_validation_and_keeps_exact_history() {
        for rich in [false, true] {
            let mut doc = Document::new(Mesh::sphere(1));
            doc.add_subtool(Mesh::sphere(1), "detail").unwrap();
            if rich {
                let mesh = doc.mesh.as_mut().unwrap();
                mesh.mask_convention = crate::MaskConvention::UnprotectedF32V1;
                mesh.masks.fill(1.0);
            }
            let before = doc.to_json().unwrap();
            doc.begin_stroke();
            doc.mesh.as_mut().unwrap().positions[0].x += 0.001;
            doc.mesh.as_mut().unwrap().recompute_normals();
            crate::mesh::validation_probe::reset();
            doc.end_stroke();
            assert_eq!(crate::mesh::validation_probe::count(), 1, "rich={rich}");
            let after = doc.to_json().unwrap();
            assert_ne!(before, after);
            assert!(doc.undo());
            assert_eq!(doc.to_json().unwrap(), before);
            assert!(doc.redo());
            assert_eq!(doc.to_json().unwrap(), after);

            doc.begin_stroke();
            doc.mesh.as_mut().unwrap().positions[0].x = f32::NAN;
            crate::mesh::validation_probe::reset();
            doc.end_stroke();
            assert_eq!(crate::mesh::validation_probe::count(), 1, "rich={rich}");
            assert_eq!(doc.to_json().unwrap(), after);
        }
    }

    #[test]
    fn mask_conversion_keeps_derived_cache_separate_from_near_limit_static_storage() {
        use crate::{
            MaskConvention, MaskConversionPolicy, SourceClayOrderPolicy, SourceGeometryAlphaTracks,
            SourceGeometryControls, SourceGeometryOptions, SourceGeometryPass, SourceGeometryTool,
            Vec3,
        };
        let mut mesh = Mesh::new(vec![Vec3::ZERO, Vec3::X, Vec3::Y], vec![[0, 1, 2]]);
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let noncolor = mesh.approximate_bytes() - mesh.colors.capacity() * 12;
        let capacity = (crate::resource_limits::MAX_SCENE_STORAGE_BYTES - noncolor - 128) / 12;
        let mut colors = Vec::with_capacity(capacity);
        colors.extend_from_slice(&mesh.colors);
        mesh.colors = colors;
        mesh.validate().unwrap();
        let mut doc = Document::new(mesh);
        let options = SourceGeometryOptions {
            tool: SourceGeometryTool::Brush {
                clay: false,
                accumulate: false,
            },
            intensity: 0.25,
            prevent_folds: false,
            clay_order: SourceClayOrderPolicy::OrderedStaticOctree,
            ..Default::default()
        };
        let mut stroke = doc
            .begin_source_geometry_selection_history(options)
            .unwrap();
        let pass = SourceGeometryPass {
            center: [0.25, 0.25, 0.0],
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 4.0,
            original_face_index: 0,
        };
        let report = doc
            .apply_source_geometry_controlled_dab(
                &mut stroke,
                [Some(pass), None],
                0.5,
                true,
                &SourceGeometryControls::default(),
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        assert!(report.combined.changed_vertices > 0);
        doc.end_source_geometry_stroke(&stroke).unwrap();
        let static_bytes = doc.require_active_mesh().unwrap().approximate_bytes();
        let derived = doc.source_geometry_retained_bytes();
        assert!(static_bytes <= crate::resource_limits::MAX_SCENE_STORAGE_BYTES);
        assert!(static_bytes + derived > crate::resource_limits::MAX_SCENE_STORAGE_BYTES);
        assert!(static_bytes + derived <= crate::resource_limits::MAX_DOCUMENT_RETAINED_BYTES);
        let before = doc.to_json().unwrap();
        let converted = doc
            .convert_active_subtool_masks(
                MaskConvention::ProtectedF32V1,
                MaskConversionPolicy::RequireRoundtripExact,
            )
            .unwrap();
        assert!(converted.changed);
        let after = doc.to_json().unwrap();
        assert!(doc.undo());
        assert_eq!(doc.to_json().unwrap(), before);
        assert!(doc.redo());
        assert_eq!(doc.to_json().unwrap(), after);
    }

    #[test]
    fn bounded_json_writer_refuses_output_before_oversized_allocation() {
        let mut writer = LimitedJsonWriter {
            bytes: Vec::new(),
            limit: 8,
        };
        writer.write_all(b"12345678").unwrap();
        let capacity = writer.bytes.capacity();
        assert!(writer.write_all(b"more").is_err());
        assert_eq!(writer.bytes.len(), 8);
        assert_eq!(writer.bytes.capacity(), capacity);
    }

    #[test]
    fn json_output_non_power_limit_bounds_capacity_and_failed_write_is_atomic() {
        let mut writer = LimitedJsonWriter {
            bytes: Vec::new(),
            limit: 9,
        };
        writer.write_all(b"12345678").unwrap();
        writer.write_all(b"9").unwrap();
        assert_eq!(writer.bytes, b"123456789");
        assert!(writer.bytes.capacity() <= 9);
        let capacity = writer.bytes.capacity();
        assert_eq!(writer.write(&[]).unwrap(), 0);
        writer.flush().unwrap();
        assert!(writer.write(b"x").is_err());
        assert_eq!(writer.bytes, b"123456789");
        assert_eq!(writer.bytes.capacity(), capacity);
    }

    #[test]
    fn large_json_output_keeps_at_most_one_chunk_of_spare_capacity() {
        let length = 35 * 1024 * 1024 + 13;
        let mut writer = LimitedJsonWriter {
            bytes: Vec::new(),
            limit: 40 * 1024 * 1024 + 17,
        };
        let chunk = [0xab; 64 * 1024];
        while writer.bytes.len() < length {
            let count = (length - writer.bytes.len()).min(chunk.len());
            writer.write_all(&chunk[..count]).unwrap();
        }
        assert_eq!(writer.bytes.len(), length);
        assert!(writer.bytes.iter().all(|&b| b == 0xab));
        assert!(writer.bytes.capacity() <= length + JSON_OUTPUT_GROWTH_BYTES);
        assert!(writer.bytes.capacity() <= writer.limit);
    }

    #[test]
    fn bounded_json_output_matches_normal_serializer_with_utf8_and_signed_zero() {
        let value = serde_json::json!({
            "name": "Brush 🖌 / قلم",
            "values": [-0.0_f32, f32::MIN_POSITIVE, f32::MAX, 0.33333334_f32],
            "nested": {"escape": "line\nquote\""}
        });
        let expected = serde_json::to_vec(&value).unwrap();
        for limit in [
            0,
            1,
            expected.len() - 1,
            expected.len(),
            expected.len() + 17,
        ] {
            let mut writer = LimitedJsonWriter {
                bytes: Vec::new(),
                limit,
            };
            let result = serde_json::to_writer(&mut writer, &value);
            assert_eq!(result.is_ok(), limit >= expected.len());
            assert!(writer.bytes.capacity() <= limit);
            assert!(expected.starts_with(&writer.bytes));
            if result.is_ok() {
                assert_eq!(writer.bytes, expected);
            }
        }
    }

    #[test]
    fn history_budget_counts_reserved_storage_and_drops_oldest_snapshots() {
        let mut document = Document::new(Mesh::sphere(0));
        let snapshot = document.snapshot_state();
        let mut oversized = document.snapshot_state();
        // Retained capacity, even with short lengths, contributes to the budget.
        oversized
            .mesh
            .as_mut()
            .unwrap()
            .positions
            .reserve_exact(1024);
        assert!(oversized.approximate_bytes() > snapshot.approximate_bytes() + 10_000);
        let keep_budget = oversized.approximate_bytes() + 1;
        document.undo.push_back(snapshot);
        document.undo.push_back(oversized);
        let bytes: usize = document
            .undo
            .iter()
            .map(DocumentState::approximate_bytes)
            .fold(0, usize::saturating_add);
        assert!(bytes < HISTORY_BUDGET_BYTES);
        document.trim_history_to(keep_budget, MAX_HISTORY_ENTRIES);
        assert_eq!(document.undo.len(), 1);
        assert!(
            document
                .undo
                .front()
                .unwrap()
                .mesh
                .as_ref()
                .unwrap()
                .positions
                .capacity()
                >= 1024
        );
    }

    #[test]
    fn configurable_history_length_trims_oldest_edits_and_preserves_redo_order() {
        let mut document = Document::new(Mesh::sphere(0));
        let base = document.active_mesh().unwrap().positions[0].x;
        assert_eq!(document.history_limit(), 15);
        assert_eq!(document.history_depths(), (0, 0));
        document.set_history_limit(50).unwrap();
        let before = document.to_json().unwrap();
        for step in 1..=55 {
            document.begin_stroke();
            let mesh = document.active_mesh_mut().unwrap();
            mesh.positions[0].x = base + step as f32 * 0.001;
            mesh.recompute_normals();
            document.end_stroke();
        }
        assert_eq!(document.history_depths(), (50, 0));
        for _ in 0..50 {
            assert!(document.undo());
        }
        assert!(!document.undo());
        assert_eq!(document.history_depths(), (0, 50));
        assert_eq!(document.active_mesh().unwrap().positions[0].x, base + 0.005);

        document.set_history_limit(3).unwrap();
        assert_eq!(document.history_depths(), (0, 3));
        for step in 6..=8 {
            assert!(document.redo());
            assert_eq!(
                document.active_mesh().unwrap().positions[0].x,
                base + step as f32 * 0.001
            );
        }
        assert!(!document.redo());
        assert_eq!(document.history_depths(), (3, 0));
        for invalid in [0, 2, 51, usize::MAX] {
            assert!(document.set_history_limit(invalid).is_err());
            assert_eq!(document.history_limit(), 3);
            assert_eq!(document.history_depths(), (3, 0));
        }
        assert_ne!(document.to_json().unwrap(), before);
    }

    #[test]
    fn history_length_survives_whole_file_replacement_without_becoming_project_data() {
        let mut document = Document::new(Mesh::sphere(0));
        let same_project = document.to_json().unwrap();
        document.set_history_limit(3).unwrap();
        assert_eq!(document.to_json().unwrap(), same_project);
        document
            .replace_document(Document::new(Mesh::sphere(1)))
            .unwrap();
        assert_eq!(document.history_limit(), 3);
        assert_eq!(document.history_depths(), (1, 0));
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), same_project);
        assert!(document.redo());
        assert_eq!(document.history_limit(), 3);
    }

    #[test]
    fn lowering_history_length_prefers_nearest_undo_and_discards_far_redo() {
        let mut document = Document::new(Mesh::sphere(0));
        let base = document.active_mesh().unwrap().positions[0].x;
        document.set_history_limit(10).unwrap();
        for step in 1..=9 {
            document.begin_stroke();
            let mesh = document.active_mesh_mut().unwrap();
            mesh.positions[0].x = base + step as f32 * 0.001;
            mesh.recompute_normals();
            document.end_stroke();
        }
        for _ in 0..3 {
            assert!(document.undo());
        }
        assert_eq!(document.history_depths(), (6, 3));
        let current = document.to_json().unwrap();
        document.set_history_limit(3).unwrap();
        assert_eq!(document.history_depths(), (3, 0));
        assert_eq!(document.to_json().unwrap(), current);
        assert!(!document.redo());
        for _ in 0..3 {
            assert!(document.undo());
        }
        assert_eq!(document.active_mesh().unwrap().positions[0].x, base + 0.003);
        assert!(!document.undo());
    }

    #[test]
    fn conservative_encoded_bounds_cover_actual_mesh_and_layer_output() {
        let mut document = Document::new(Mesh::sphere(0));
        document.name = "\"\\\n\u{0001}".repeat(1024);
        document.active_mesh_mut().unwrap().positions[0].x = f32::from_bits(1);
        document.active_mesh_mut().unwrap().masks[0] = f32::from_bits(1);
        document.active_mesh_mut().unwrap().colors[0] = [f32::from_bits(1), 0.999_999_94, -0.0];
        document.active_mesh_mut().unwrap().recompute_normals();
        document.add_layer(&"\"\\".repeat(128)).unwrap();
        document.begin_stroke();
        document.active_mesh_mut().unwrap().positions[0].x = f32::from_bits(2);
        document.end_stroke();
        assert!(document.take_layer_error().is_none());
        let layers = document.layers.as_ref().unwrap();
        let actual_mesh = serde_json::to_vec(document.active_mesh().unwrap())
            .unwrap()
            .len();
        let actual_layers = serde_json::to_vec(&layers.project_ref()).unwrap().len();
        assert!(actual_mesh <= mesh_encoded_upper_bound(document.active_mesh().unwrap()));
        assert!(actual_layers <= layers.encoded_upper_bound());
        let complete_bound = mesh_encoded_upper_bound(document.active_mesh().unwrap())
            + document.name.len() * 6
            + 1024
            + layers.encoded_upper_bound();
        assert!(document.to_json().unwrap().len() <= complete_bound);
    }

    #[test]
    fn history_budget_counts_layer_buffers_even_when_shared() {
        let mut document = Document::new(Mesh::sphere(0));
        let plain_bytes = document.snapshot_state().approximate_bytes();
        document.add_layer("First").unwrap();
        let first = document.snapshot_state();
        let first_bytes = first.approximate_bytes();
        assert!(
            first_bytes >= plain_bytes + 24 * document.active_mesh().unwrap().positions.len() * 2
        );
        document.add_layer("Second").unwrap();
        let second = document.snapshot_state();
        assert!(
            second.approximate_bytes()
                >= first_bytes + 24 * document.active_mesh().unwrap().positions.len()
        );
        document.undo.clear();
        document.undo.push_back(first);
        document.undo.push_back(second);
        let newest_bytes = document.undo.back().unwrap().approximate_bytes();
        document.trim_history_to(newest_bytes, MAX_HISTORY_ENTRIES);
        assert_eq!(document.undo.len(), 1);
        assert_eq!(
            document
                .undo
                .front()
                .unwrap()
                .layers
                .as_ref()
                .unwrap()
                .count(),
            2
        );
    }

    #[test]
    fn morph_history_shares_target_buffers_and_charges_retained_capacity() {
        let mut document = Document::new(Mesh::sphere(1));
        let plain = document.snapshot_state().approximate_bytes();
        document.store_morph_target().unwrap();
        let first = document.snapshot_state();
        let second = document.snapshot_state();
        let a = first.morph_target.as_ref().unwrap();
        let b = second.morph_target.as_ref().unwrap();
        assert!(std::sync::Arc::ptr_eq(&a.positions, &b.positions));
        assert!(
            first.approximate_bytes()
                >= plain
                    + document.active_mesh().unwrap().positions.len() * 12
                    + document.active_mesh().unwrap().triangles.len() * 12
        );
        let target_bytes = serde_json::to_vec(&a.project_ref()).unwrap().len();
        assert!(target_bytes <= a.encoded_upper_bound());
        assert!(
            document.to_json().unwrap().len()
                <= mesh_encoded_upper_bound(document.active_mesh().unwrap())
                    + document.name.len() * 6
                    + 1024
                    + a.encoded_upper_bound()
        );
    }
}
