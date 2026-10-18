//! Bounded object metadata and immutable inactive editable-state snapshots.
use crate::layers::{LayerStack, LayersProject, LayersProjectRef};
use crate::morph::{MorphProject, MorphProjectRef, MorphTarget};
use crate::multires::{MultiresProject, MultiresProjectRef, Multiresolution};
use crate::{
    Hit, Mesh, MAX_LAYER_STORAGE_BYTES, MAX_LAYER_VECTORS, MAX_MORPH_STORAGE_BYTES, MAX_TRIANGLES,
    MAX_VERTICES,
};
use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::sync::Arc;

pub const MAX_SUBTOOLS: usize = 32;
pub const MAX_SUBTOOL_NAME_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SubtoolSummary {
    pub id: u32,
    pub name: String,
    pub visible: bool,
    pub effective_visible: bool,
    pub active: bool,
    pub vertices: usize,
    pub triangles: usize,
}

#[derive(Clone, Debug)]
pub struct SubtoolRenderSnapshot {
    pub id: u32,
    pub active: bool,
    pub mesh: Arc<Mesh>,
    pub metadata: Option<crate::SceneObjectMetadata>,
}

#[derive(Clone, Copy, Debug)]
pub struct SubtoolHit {
    pub id: u32,
    pub hit: Hit,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ObjectBudget {
    pub(crate) vertices: usize,
    pub(crate) triangles: usize,
    pub(crate) uv_coordinates: usize,
    pub(crate) layer_vectors: usize,
    pub(crate) layer_bytes: usize,
    pub(crate) morph_vertices: usize,
    pub(crate) morph_triangles: usize,
    pub(crate) morph_bytes: usize,
    pub(crate) bytes: usize,
    pub(crate) encoded: usize,
}

impl ObjectBudget {
    pub(crate) fn for_state(
        mesh: &Mesh,
        levels: Option<&Multiresolution>,
        layers: Option<&LayerStack>,
        target: Option<&MorphTarget>,
    ) -> Self {
        let mut budget = Self {
            vertices: mesh.positions.len(),
            triangles: mesh.triangles.len(),
            uv_coordinates: mesh.uv_coordinate_count(),
            bytes: mesh.approximate_bytes(),
            encoded: mesh_bound(mesh),
            ..Self::default()
        };
        if let Some(levels) = levels {
            budget.bytes += levels.approximate_bytes();
            budget.encoded += 1024;
            for (index, level) in levels.levels.iter().enumerate() {
                if index != levels.active {
                    budget.vertices += level.positions.len();
                    budget.triangles += level.triangles.len();
                    budget.uv_coordinates += level.uv_coordinate_count();
                    budget.encoded += mesh_bound(level);
                }
            }
        }
        if let Some(layers) = layers {
            budget.layer_vectors = layers.vector_count();
            budget.layer_bytes = layers.approximate_bytes();
            budget.bytes += budget.layer_bytes;
            budget.encoded += layers.encoded_upper_bound();
        }
        if let Some(target) = target {
            let summary = target.summary(false);
            budget.morph_vertices = summary.vertices;
            budget.morph_triangles = summary.triangles;
            budget.morph_bytes = target.approximate_bytes();
            budget.bytes += budget.morph_bytes;
            budget.encoded += target.encoded_upper_bound();
        }
        budget
    }
    pub(crate) fn plus(self, other: Self) -> Self {
        Self {
            vertices: self.vertices.saturating_add(other.vertices),
            triangles: self.triangles.saturating_add(other.triangles),
            uv_coordinates: self.uv_coordinates.saturating_add(other.uv_coordinates),
            layer_vectors: self.layer_vectors.saturating_add(other.layer_vectors),
            layer_bytes: self.layer_bytes.saturating_add(other.layer_bytes),
            morph_vertices: self.morph_vertices.saturating_add(other.morph_vertices),
            morph_triangles: self.morph_triangles.saturating_add(other.morph_triangles),
            morph_bytes: self.morph_bytes.saturating_add(other.morph_bytes),
            bytes: self.bytes.saturating_add(other.bytes),
            encoded: self.encoded.saturating_add(other.encoded),
        }
    }
    /// Keep static arrays under their own bound before charging the separately
    /// bounded, nonserialized normal/tree producer. Counts are never bypassed.
    pub(crate) fn validate_with_derived_bytes(self, derived: usize) -> Result<(), String> {
        self.validate()?;
        if derived > crate::resource_limits::MAX_GEOMETRY_STATE_BYTES
            || self
                .bytes
                .checked_add(derived)
                .is_none_or(|bytes| bytes > crate::resource_limits::MAX_DOCUMENT_RETAINED_BYTES)
        {
            return Err("Scene and derived cache exceed the retained document budget".into());
        }
        Ok(())
    }

    pub(crate) fn validate(self) -> Result<(), String> {
        if self.vertices > MAX_VERTICES || self.triangles > MAX_TRIANGLES {
            return Err("SubTools and retained levels exceed the aggregate one-million point/two-million face limit".into());
        }
        if self.uv_coordinates > crate::MAX_UV_COORDINATES {
            return Err(
                "SubTools and retained levels exceed the aggregate UV coordinate limit".into(),
            );
        }
        if self.layer_vectors > MAX_LAYER_VECTORS || self.layer_bytes > MAX_LAYER_STORAGE_BYTES {
            return Err("SubTool layers exceed aggregate vector/64 MiB storage limits".into());
        }
        if self.morph_vertices > MAX_VERTICES
            || self.morph_triangles > MAX_TRIANGLES
            || self.morph_bytes > MAX_MORPH_STORAGE_BYTES
        {
            return Err(
                "SubTool Morph Targets exceed aggregate point/face/64 MiB storage limits".into(),
            );
        }
        if self.bytes > crate::decode_limits::MAX_SCENE_STORAGE_BYTES {
            return Err("Retained scene storage exceeds the scene allocation budget".into());
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ObjectState {
    pub(crate) mesh: Arc<Mesh>,
    pub(crate) metadata: Option<crate::SceneObjectMetadata>,
    pub(crate) multiresolution: Option<Multiresolution>,
    pub(crate) layers: Option<LayerStack>,
    pub(crate) morph_target: Option<MorphTarget>,
    pub(crate) budget: ObjectBudget,
}

impl ObjectState {
    pub(crate) fn new(
        mesh: Mesh,
        multiresolution: Option<Multiresolution>,
        layers: Option<LayerStack>,
        morph_target: Option<MorphTarget>,
    ) -> Self {
        Self::with_metadata(mesh, multiresolution, layers, morph_target, None)
    }
    pub(crate) fn with_metadata(
        mesh: Mesh,
        multiresolution: Option<Multiresolution>,
        layers: Option<LayerStack>,
        morph_target: Option<MorphTarget>,
        metadata: Option<crate::SceneObjectMetadata>,
    ) -> Self {
        let mut budget = ObjectBudget::for_state(
            &mesh,
            multiresolution.as_ref(),
            layers.as_ref(),
            morph_target.as_ref(),
        );
        budget.bytes = budget.bytes.saturating_add(
            metadata
                .as_ref()
                .map_or(0, |_| std::mem::size_of::<crate::SceneObjectMetadata>()),
        );
        Self {
            mesh: Arc::new(mesh),
            metadata,
            multiresolution,
            layers,
            morph_target,
            budget,
        }
    }
    pub(crate) fn same_state(&self, other: &Self) -> bool {
        crate::scene_v7::optional_metadata_equal(self.metadata.as_ref(), other.metadata.as_ref())
            && self.mesh.same_surface(&other.mesh)
            && crate::document::mesh_channels_equal_bits(&self.mesh, &other.mesh)
            && match (&self.multiresolution, &other.multiresolution) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_surface(b),
                _ => false,
            }
            && match (&self.layers, &other.layers) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_state(b),
                _ => false,
            }
            && match (&self.morph_target, &other.morph_target) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_state(b),
                _ => false,
            }
    }
    fn project_ref(&self) -> ObjectProjectRef<'_> {
        ObjectProjectRef {
            mesh: &self.mesh,
            multiresolution: self
                .multiresolution
                .as_ref()
                .map(Multiresolution::project_ref),
            layers: self.layers.as_ref().map(LayerStack::project_ref),
            morph_target: self.morph_target.as_ref().map(MorphTarget::project_ref),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Subtool {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) visible: bool,
    pub(crate) payload: Option<Arc<ObjectState>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Scene {
    pub(crate) active_id: Option<u32>,
    pub(crate) selected_ids: Vec<u32>,
    pub(crate) next_id: u32,
    pub(crate) solo: bool,
    pub(crate) objects: Vec<Subtool>,
}

impl Scene {
    pub(crate) fn empty() -> Self {
        Self {
            active_id: None,
            selected_ids: Vec::new(),
            next_id: 1,
            solo: false,
            objects: Vec::new(),
        }
    }
    pub(crate) fn initial(name: &str) -> Self {
        let name = if validate_name(name).is_ok() {
            name
        } else {
            "Sculpture"
        };
        Self {
            active_id: Some(1),
            selected_ids: vec![1],
            next_id: 2,
            solo: false,
            objects: vec![Subtool {
                id: 1,
                name: name.into(),
                visible: true,
                payload: None,
            }],
        }
    }
    pub(crate) fn index(&self, id: u32) -> Result<usize, String> {
        self.objects
            .iter()
            .position(|o| o.id == id)
            .ok_or_else(|| "SubTool ID is not in this scene".into())
    }
    pub(crate) fn effectively_visible(&self, object: &Subtool) -> bool {
        object.visible && (!self.solo || Some(object.id) == self.active_id)
    }
    pub(crate) fn same_state(&self, other: &Self) -> bool {
        self.selected_ids == other.selected_ids
            && self.active_id == other.active_id
            && self.next_id == other.next_id
            && self.solo == other.solo
            && self.objects.len() == other.objects.len()
            && self.objects.iter().zip(&other.objects).all(|(a, b)| {
                a.id == b.id
                    && a.name == b.name
                    && a.visible == b.visible
                    && match (&a.payload, &b.payload) {
                        (None, None) => true,
                        (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a.same_state(b),
                        _ => false,
                    }
            })
    }
    pub(crate) fn inactive_budget(&self) -> ObjectBudget {
        self.objects
            .iter()
            .filter_map(|o| o.payload.as_ref())
            .fold(ObjectBudget::default(), |b, s| b.plus(s.budget))
    }
    pub(crate) fn approximate_bytes(&self) -> usize {
        let metadata = self
            .objects
            .iter()
            .map(|o| {
                o.name
                    .capacity()
                    .saturating_add(o.payload.as_ref().map_or(0, |state| state.budget.bytes))
            })
            .fold(0, usize::saturating_add);
        std::mem::size_of::<Self>()
            .saturating_add(
                self.selected_ids
                    .capacity()
                    .saturating_mul(std::mem::size_of::<u32>()),
            )
            .saturating_add(
                self.objects
                    .capacity()
                    .saturating_mul(std::mem::size_of::<Subtool>()),
            )
            .saturating_add(metadata)
    }
    pub(crate) fn encoded_upper_bound(&self) -> usize {
        self.inactive_budget()
            .encoded
            .saturating_add(
                self.objects
                    .iter()
                    .map(|o| o.name.len() * 6 + 512)
                    .sum::<usize>(),
            )
            .saturating_add(1024)
    }
    pub(crate) fn validate_structure(&self) -> Result<(), String> {
        if self.objects.len() > MAX_SUBTOOLS {
            return Err("Scenes require at most32 SubTools".into());
        }
        if self.objects.is_empty() {
            if self.active_id.is_some()
                || !self.selected_ids.is_empty()
                || self.next_id == 0
                || self.solo
            {
                return Err(
                    "Empty Scene must have no active object/Solo and a valid next ID".into(),
                );
            }
            return Ok(());
        }
        if self.selected_ids.is_empty() || self.selected_ids.len() > MAX_SUBTOOLS {
            return Err("Nonempty scene requires bounded selection".into());
        }
        let mut selected = std::collections::HashSet::new();
        for &id in &self.selected_ids {
            if !selected.insert(id) || !self.objects.iter().any(|o| o.id == id) {
                return Err("Invalid/duplicate selected scene ID".into());
            }
        }
        if self.active_id.is_none_or(|id| !selected.contains(&id)) {
            return Err("Scene active object must be selected".into());
        }
        let mut ids = std::collections::HashSet::new();
        let mut maximum = 0;
        let mut active = 0;
        for object in &self.objects {
            validate_name(&object.name)?;
            if object.id == 0 || !ids.insert(object.id) {
                return Err("SubTool IDs must be unique positive u32 values".into());
            }
            maximum = maximum.max(object.id);
            if Some(object.id) == self.active_id {
                active += 1;
                if object.payload.is_some() {
                    return Err("Active SubTool payload must be null".into());
                }
            } else if object.payload.is_none() {
                return Err("Inactive SubTools require complete payloads".into());
            }
        }
        if active != 1 || self.next_id <= maximum {
            return Err("Scene active/next ID state is invalid".into());
        }
        Ok(())
    }
    pub(crate) fn project_ref(&self) -> SceneProjectRef<'_> {
        SceneProjectRef {
            scheme: SceneScheme::SubtoolsV1,
            active_id: self.active_id,
            next_id: self.next_id,
            solo: self.solo,
            objects: self
                .objects
                .iter()
                .map(|o| SubtoolProjectRef {
                    id: o.id,
                    name: &o.name,
                    visible: o.visible,
                    payload: o.payload.as_ref().map(|p| p.project_ref()),
                })
                .collect(),
        }
    }
}

pub(crate) fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_SUBTOOL_NAME_BYTES || name.chars().any(char::is_control)
    {
        Err("SubTool names require 1–256 UTF-8 bytes without control characters".into())
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum SceneScheme {
    SubtoolsV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SceneProject {
    #[serde(rename = "scheme")]
    _scheme: SceneScheme,
    pub(crate) active_id: Option<u32>,
    pub(crate) next_id: u32,
    pub(crate) solo: bool,
    #[serde(deserialize_with = "deserialize_objects")]
    pub(crate) objects: Vec<SubtoolProject>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SubtoolProject {
    pub(crate) id: u32,
    #[serde(deserialize_with = "deserialize_name")]
    pub(crate) name: String,
    pub(crate) visible: bool,
    #[serde(deserialize_with = "deserialize_payload")]
    pub(crate) payload: Option<ObjectProject>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObjectProject {
    pub(crate) mesh: Mesh,
    #[serde(default)]
    pub(crate) multiresolution: Option<MultiresProject>,
    #[serde(default)]
    pub(crate) layers: Option<LayersProject>,
    #[serde(default)]
    pub(crate) morph_target: Option<MorphProject>,
}

#[derive(Serialize)]
pub(crate) struct SceneProjectRef<'a> {
    scheme: SceneScheme,
    active_id: Option<u32>,
    next_id: u32,
    solo: bool,
    objects: Vec<SubtoolProjectRef<'a>>,
}
#[derive(Serialize)]
struct SubtoolProjectRef<'a> {
    id: u32,
    name: &'a str,
    visible: bool,
    payload: Option<ObjectProjectRef<'a>>,
}
#[derive(Serialize)]
struct ObjectProjectRef<'a> {
    mesh: &'a Mesh,
    #[serde(skip_serializing_if = "Option::is_none")]
    multiresolution: Option<MultiresProjectRef<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    layers: Option<LayersProjectRef<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    morph_target: Option<MorphProjectRef<'a>>,
}

fn deserialize_objects<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<SubtoolProject>, D::Error> {
    struct ObjectsVisitor;
    impl<'de> Visitor<'de> for ObjectsVisitor {
        type Value = Vec<SubtoolProject>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("0–32 bounded SubTool objects")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            loop {
                if values.len() >= MAX_SUBTOOLS {
                    if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                        return Err(A::Error::custom("Too many SubTools"));
                    }
                    return Ok(values);
                }
                let Some(value) = seq.next_element::<SubtoolProject>()? else {
                    return Ok(values);
                };
                validate_name(&value.name).map_err(A::Error::custom)?;
                values.push(value);
            }
        }
    }
    d.deserialize_seq(ObjectsVisitor)
}

fn deserialize_payload<'de, D: Deserializer<'de>>(d: D) -> Result<Option<ObjectProject>, D::Error> {
    Option::deserialize(d)
}
fn deserialize_name<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    struct NameVisitor;
    impl Visitor<'_> for NameVisitor {
        type Value = String;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a bounded SubTool name")
        }
        fn visit_str<E: serde::de::Error>(self, name: &str) -> Result<String, E> {
            validate_name(name).map_err(E::custom)?;
            Ok(name.into())
        }
    }
    d.deserialize_str(NameVisitor)
}

fn mesh_bound(mesh: &Mesh) -> usize {
    mesh.positions
        .len()
        .saturating_mul(256)
        .saturating_add(mesh.triangles.len().saturating_mul(35))
        .saturating_add(512)
        .saturating_add(mesh.attribute_encoded_bound())
}

#[cfg(test)]
mod derived_resource_budget_tests {
    use super::*;

    #[test]
    fn derived_cache_has_its_own_bound_without_replacing_static_storage_bound() {
        let scene = ObjectBudget {
            bytes: crate::resource_limits::MAX_SCENE_STORAGE_BYTES,
            ..Default::default()
        };
        assert!(scene
            .validate_with_derived_bytes(crate::resource_limits::MAX_GEOMETRY_STATE_BYTES)
            .is_ok());
        assert!(scene
            .validate_with_derived_bytes(crate::resource_limits::MAX_GEOMETRY_STATE_BYTES + 1)
            .is_err());
        assert!(scene.validate_with_derived_bytes(usize::MAX).is_err());
        assert!(ObjectBudget {
            bytes: scene.bytes + 1,
            ..scene
        }
        .validate_with_derived_bytes(0)
        .is_err());
    }

    #[test]
    fn charging_derived_cache_still_checks_original_count_and_layer_limits() {
        let oversize = [
            ObjectBudget {
                vertices: MAX_VERTICES + 1,
                ..Default::default()
            },
            ObjectBudget {
                triangles: MAX_TRIANGLES + 1,
                ..Default::default()
            },
            ObjectBudget {
                uv_coordinates: crate::MAX_UV_COORDINATES + 1,
                ..Default::default()
            },
            ObjectBudget {
                layer_bytes: MAX_LAYER_STORAGE_BYTES + 1,
                ..Default::default()
            },
            ObjectBudget {
                morph_bytes: MAX_MORPH_STORAGE_BYTES + 1,
                ..Default::default()
            },
        ];
        for budget in oversize {
            assert!(budget.validate_with_derived_bytes(0).is_err());
            assert!(budget.validate_with_derived_bytes(1).is_err());
        }
    }
}
