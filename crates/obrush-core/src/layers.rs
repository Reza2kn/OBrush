//! Fixed-topology additive geometry layers. Masks and RGB are document channels.
use crate::{Mesh, Vec3, MAX_TRIANGLES};
use glam::DVec3;
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::sync::Arc;

pub const MAX_SCULPT_LAYERS: usize = 16;
pub const MAX_LAYER_VECTORS: usize = 1_000_000;
pub const MAX_LAYER_STORAGE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_LAYER_NAME_BYTES: usize = 256;
const MAX_COORDINATE: f64 = 1_000_000.0;
const MAX_DELTA: f64 = 2_000_000.0;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LayerSummary {
    pub index: usize,
    pub name: String,
    pub intensity: f32,
    pub visible: bool,
    pub recording: bool,
    pub has_geometry: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct GeometryLayer {
    name: String,
    intensity: f32,
    visible: bool,
    deltas: Arc<Vec<DVec3>>,
    has_geometry: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayerStack {
    base_positions: Arc<Vec<DVec3>>,
    triangles: Arc<Vec<[u32; 3]>>,
    entries: Vec<GeometryLayer>,
    recording_layer: Option<usize>,
    /// Last committed composition; never serialized as another mesh channel.
    display_positions: Arc<Vec<Vec3>>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum LayerScheme {
    WorldOffsetsF64V1,
}

pub(crate) struct LayersProject {
    base_positions: Vec<DVec3>,
    triangles: Vec<[u32; 3]>,
    entries: Vec<LayerProject>,
    recording_layer: Option<usize>,
}

struct LayerProject {
    name: String,
    intensity: f32,
    visible: bool,
    deltas: Vec<DVec3>,
}

#[derive(Serialize)]
pub(crate) struct LayersProjectRef<'a> {
    scheme: LayerScheme,
    base_positions: &'a [DVec3],
    triangles: &'a [[u32; 3]],
    recording_layer: Option<usize>,
    entries: Vec<LayerProjectRef<'a>>,
}

#[derive(Serialize)]
struct LayerProjectRef<'a> {
    name: &'a str,
    intensity: f32,
    visible: bool,
    deltas: &'a [DVec3],
}

impl LayerStack {
    /// Affine all editable geometry, including hidden/faded layers. Visible
    /// f32 geometry comes from the new ordered f64 composition, not a silently
    /// corrected base or delta. Caller checks the visible rounding difference.
    pub(crate) fn transformed(
        &self,
        affine: &crate::transform::Affine,
        source: &Mesh,
        transformed: &Mesh,
    ) -> Result<(Self, Mesh), String> {
        self.validate_composition(source)?;
        let mut candidate = self.clone();
        let mut base = Vec::new();
        base.try_reserve_exact(self.base_positions.len())
            .map_err(|_| "Cannot allocate transformed layer base")?;
        for &point in self.base_positions.iter() {
            base.push(affine.point(point));
        }
        candidate.base_positions = Arc::new(base);
        for layer in &mut candidate.entries {
            let mut deltas = Vec::new();
            deltas
                .try_reserve_exact(layer.deltas.len())
                .map_err(|_| "Cannot allocate transformed layer offsets")?;
            for &delta in layer.deltas.iter() {
                deltas.push(affine.vector(delta));
            }
            layer.deltas = Arc::new(deltas);
        }
        let mesh = candidate.recomposed(transformed)?;
        affine.validate_faces(&source.positions, &mesh.positions, &mesh.triangles)?;
        candidate.validate_composition(&mesh)?;
        Ok((candidate, mesh))
    }
    pub(crate) fn vector_count(&self) -> usize {
        self.base_positions.len()
            + self
                .entries
                .iter()
                .map(|layer| layer.deltas.len())
                .sum::<usize>()
    }
    pub(crate) fn same_state(&self, other: &Self) -> bool {
        self.recording_layer == other.recording_layer
            && (Arc::ptr_eq(&self.base_positions, &other.base_positions)
                || dvecs_equal_bits(&self.base_positions, &other.base_positions))
            && (Arc::ptr_eq(&self.triangles, &other.triangles) || self.triangles == other.triangles)
            && self.entries.len() == other.entries.len()
            && self.entries.iter().zip(&other.entries).all(|(a, b)| {
                a.name == b.name
                    && a.intensity.to_bits() == b.intensity.to_bits()
                    && a.visible == b.visible
                    && a.has_geometry == b.has_geometry
                    && (Arc::ptr_eq(&a.deltas, &b.deltas) || dvecs_equal_bits(&a.deltas, &b.deltas))
            })
    }

    /// Deliberately generous finite-f64 width (32 bytes), brackets and commas,
    /// plus escaped names/metadata. Near the project limit the actual writer
    /// remains authoritative; this bound merely avoids ordinary-stroke encode.
    pub(crate) fn encoded_upper_bound(&self) -> usize {
        (self.base_positions.len() + self.entries.iter().map(|e| e.deltas.len()).sum::<usize>())
            .saturating_mul(100)
            .saturating_add(self.triangles.len().saturating_mul(35))
            .saturating_add(
                self.entries
                    .iter()
                    .map(|e| e.name.len() * 6 + 512)
                    .sum::<usize>(),
            )
            .saturating_add(1024)
    }
    pub(crate) fn new(mesh: &Mesh) -> Result<Self, String> {
        mesh.validate()?;
        validate_geometry(&mesh.positions, &mesh.triangles)?;
        let stack = Self {
            base_positions: Arc::new(mesh.positions.iter().map(|p| p.as_dvec3()).collect()),
            triangles: Arc::new(mesh.triangles.clone()),
            entries: Vec::new(),
            recording_layer: None,
            display_positions: Arc::new(mesh.positions.clone()),
        };
        stack.validate_budget()?;
        Ok(stack)
    }

    pub(crate) fn count(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn recording(&self) -> Option<usize> {
        self.recording_layer
    }
    pub(crate) fn summaries(&self) -> Vec<LayerSummary> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, layer)| LayerSummary {
                index,
                name: layer.name.clone(),
                intensity: layer.intensity,
                visible: layer.visible,
                recording: self.recording_layer == Some(index),
                has_geometry: layer.has_geometry,
            })
            .collect()
    }

    pub(crate) fn add(&mut self, name: &str) -> Result<usize, String> {
        validate_name(name)?;
        if self.entries.len() >= MAX_SCULPT_LAYERS {
            return Err("At most 16 geometry layers are supported".into());
        }
        let vertices = self.base_positions.len();
        if vertices
            .checked_mul(self.entries.len() + 2)
            .is_none_or(|n| n > MAX_LAYER_VECTORS)
        {
            return Err("Layers exceed the one-million combined base/delta vector limit".into());
        }
        let extra = vertices
            .checked_mul(std::mem::size_of::<DVec3>())
            .ok_or("Layer size overflow")?;
        if self
            .approximate_bytes()
            .checked_add(extra + name.len())
            .is_none_or(|n| n > MAX_LAYER_STORAGE_BYTES)
        {
            return Err("Layer storage exceeds 64 MiB".into());
        }
        let mut deltas = Vec::new();
        deltas
            .try_reserve_exact(vertices)
            .map_err(|_| "Cannot allocate geometry layer")?;
        deltas.resize(vertices, DVec3::ZERO);
        let index = self.entries.len();
        self.entries.push(GeometryLayer {
            name: name.into(),
            intensity: 1.0,
            visible: true,
            deltas: Arc::new(deltas),
            has_geometry: false,
        });
        self.recording_layer = Some(index);
        self.validate_budget()?;
        Ok(index)
    }

    pub(crate) fn set_recording(&mut self, index: Option<usize>) -> Result<(), String> {
        if let Some(index) = index {
            let layer = self
                .entries
                .get(index)
                .ok_or("Layer index is out of range")?;
            if !layer.visible || layer.intensity != 1.0 {
                return Err("Recording requires a visible layer at intensity 1".into());
            }
        }
        self.recording_layer = index;
        Ok(())
    }

    pub(crate) fn set_intensity(&mut self, index: usize, intensity: f32) -> Result<(), String> {
        validate_intensity(intensity)?;
        self.entries
            .get_mut(index)
            .ok_or("Layer index is out of range")?
            .intensity = intensity;
        if self.recording_layer == Some(index) && intensity != 1.0 {
            self.recording_layer = None;
        }
        Ok(())
    }

    pub(crate) fn set_visibility(&mut self, index: usize, visible: bool) -> Result<(), String> {
        self.entries
            .get_mut(index)
            .ok_or("Layer index is out of range")?
            .visible = visible;
        if self.recording_layer == Some(index) && !visible {
            self.recording_layer = None;
        }
        Ok(())
    }

    pub(crate) fn set_name(&mut self, index: usize, name: &str) -> Result<(), String> {
        validate_name(name)?;
        self.entries
            .get_mut(index)
            .ok_or("Layer index is out of range")?
            .name = name.into();
        Ok(())
    }

    pub(crate) fn delete(&mut self, index: usize) -> Result<(), String> {
        if index >= self.entries.len() {
            return Err("Layer index is out of range".into());
        }
        self.entries.remove(index);
        self.recording_layer = self.recording_layer.and_then(|recording| {
            if recording == index {
                None
            } else {
                Some(recording - usize::from(recording > index))
            }
        });
        Ok(())
    }

    /// Capture raw public-mesh edits without changing accepted visible f32 data.
    pub(crate) fn reconciled(&self, mesh: &Mesh) -> Result<Self, String> {
        self.validate_structure(mesh)?;
        let mut candidate = self.clone();
        if positions_equal_bits(&mesh.positions, &self.display_positions) {
            return Ok(candidate);
        }
        for (vertex, &position) in mesh.positions.iter().enumerate() {
            if position.to_array().map(f32::to_bits)
                == self.display_positions[vertex].to_array().map(f32::to_bits)
            {
                continue;
            }
            let target = position.as_dvec3();
            if let Some(recording) = self.recording_layer {
                let other =
                    self.contribution(vertex, Some(recording)) + self.base_positions[vertex];
                Arc::make_mut(&mut candidate.entries[recording].deltas)[vertex] = target - other;
            } else {
                Arc::make_mut(&mut candidate.base_positions)[vertex] =
                    target - self.contribution(vertex, None);
            }
        }
        candidate.validate_structure(mesh)?;
        if let Some(recording) = candidate.recording_layer {
            candidate.entries[recording].has_geometry = candidate.entries[recording]
                .deltas
                .iter()
                .any(|&d| d != DVec3::ZERO);
        }
        let composed = candidate.compose_positions()?;
        if !positions_equal_bits(&composed, &mesh.positions) {
            return Err("Layer offsets cannot represent the accepted geometry exactly at this coordinate scale".into());
        }
        candidate.display_positions = Arc::new(composed);
        candidate.validate_budget()?;
        Ok(candidate)
    }

    pub(crate) fn recomposed(&mut self, mesh: &Mesh) -> Result<Mesh, String> {
        self.validate_structure(mesh)?;
        let positions = self.compose_positions()?;
        validate_geometry(&positions, &self.triangles)?;
        let mut result = mesh.snapshot();
        if !positions_equal_bits(&result.positions, &positions) {
            result.positions.clone_from(&positions);
            result.recompute_normals();
        }
        self.display_positions = Arc::new(positions);
        self.validate_budget()?;
        Ok(result)
    }

    fn contribution(&self, vertex: usize, excluding: Option<usize>) -> DVec3 {
        let mut result = DVec3::ZERO;
        for (index, layer) in self.entries.iter().enumerate() {
            if Some(index) != excluding && layer.visible && layer.intensity != 0.0 {
                result += layer.deltas[vertex] * f64::from(layer.intensity);
            }
        }
        result
    }

    fn compose_positions(&self) -> Result<Vec<Vec3>, String> {
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(self.base_positions.len())
            .map_err(|_| "Cannot allocate layer composition")?;
        for (vertex, &base) in self.base_positions.iter().enumerate() {
            // The explicit base-plus-entry-order sum is part of the IO scheme.
            let mut position = base;
            for layer in &self.entries {
                if layer.visible && layer.intensity != 0.0 {
                    let contribution = layer.deltas[vertex] * f64::from(layer.intensity);
                    // A zero offset must preserve the base's signed zero.
                    for component in 0..3 {
                        if contribution[component] != 0.0 {
                            position[component] += contribution[component];
                        }
                    }
                }
            }
            if !position.is_finite() || position.abs().max_element() > MAX_COORDINATE {
                return Err("Layer composition exceeds finite world-coordinate limits".into());
            }
            positions.push(position.as_vec3());
        }
        Ok(positions)
    }

    fn validate_structure(&self, mesh: &Mesh) -> Result<(), String> {
        mesh.validate()?;
        if mesh.positions.len() != self.base_positions.len() || mesh.triangles != *self.triangles {
            return Err(
                "Bake geometry layers before changing their vertex or triangle correspondence"
                    .into(),
            );
        }
        self.validate_budget()?;
        if self
            .base_positions
            .iter()
            .any(|p| !p.is_finite() || p.abs().max_element() > MAX_COORDINATE)
        {
            return Err("Layer base contains invalid world coordinates".into());
        }
        for layer in &self.entries {
            validate_name(&layer.name)?;
            validate_intensity(layer.intensity)?;
            if layer.deltas.len() != self.base_positions.len()
                || layer
                    .deltas
                    .iter()
                    .any(|d| !d.is_finite() || d.abs().max_element() > MAX_DELTA)
            {
                return Err("Layer delta length or finite displacement limits are invalid".into());
            }
        }
        if let Some(index) = self.recording_layer {
            let layer = self
                .entries
                .get(index)
                .ok_or("Recording layer index is out of range")?;
            if !layer.visible || layer.intensity != 1.0 {
                return Err("Recording requires a visible layer at intensity 1".into());
            }
        }
        validate_geometry(&mesh.positions, &self.triangles)
    }

    pub(crate) fn validate_composition(&self, mesh: &Mesh) -> Result<(), String> {
        self.validate_structure(mesh)?;
        if !positions_equal_bits(&self.compose_positions()?, &mesh.positions) {
            return Err("Project mesh does not match its geometry-layer composition".into());
        }
        Ok(())
    }

    fn validate_budget(&self) -> Result<(), String> {
        if self.base_positions.is_empty() || self.entries.len() > MAX_SCULPT_LAYERS {
            return Err("Invalid geometry-layer vertex or layer count".into());
        }
        let count = self
            .entries
            .iter()
            .try_fold(self.base_positions.len(), |count, entry| {
                count.checked_add(entry.deltas.len())
            })
            .ok_or("Layer vector-count overflow")?;
        if count > MAX_LAYER_VECTORS {
            return Err("Layers exceed the one-million combined base/delta vector limit".into());
        }
        if self.approximate_bytes() > MAX_LAYER_STORAGE_BYTES {
            return Err("Layer storage exceeds 64 MiB".into());
        }
        Ok(())
    }

    pub(crate) fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.base_positions.capacity() * std::mem::size_of::<DVec3>()
            + self.triangles.capacity() * std::mem::size_of::<[u32; 3]>()
            + self.display_positions.capacity() * std::mem::size_of::<Vec3>()
            + self.entries.capacity() * std::mem::size_of::<GeometryLayer>()
            + self
                .entries
                .iter()
                .map(|e| e.name.capacity() + e.deltas.capacity() * std::mem::size_of::<DVec3>())
                .sum::<usize>()
    }

    pub(crate) fn project_ref(&self) -> LayersProjectRef<'_> {
        LayersProjectRef {
            scheme: LayerScheme::WorldOffsetsF64V1,
            base_positions: &self.base_positions,
            triangles: &self.triangles,
            recording_layer: self.recording_layer,
            entries: self
                .entries
                .iter()
                .map(|e| LayerProjectRef {
                    name: &e.name,
                    intensity: e.intensity,
                    visible: e.visible,
                    deltas: &e.deltas,
                })
                .collect(),
        }
    }

    pub(crate) fn from_project(project: LayersProject, mesh: &Mesh) -> Result<Self, String> {
        if project.entries.is_empty() {
            return Err("Layer-bearing projects require at least one layer".into());
        }
        crate::decode_limits::capacity(
            crate::decode_limits::Channel::LayerVectors,
            mesh.positions.len() * std::mem::size_of::<Vec3>(),
        )?;
        let stack = Self {
            base_positions: Arc::new(project.base_positions),
            triangles: Arc::new(project.triangles),
            recording_layer: project.recording_layer,
            entries: project
                .entries
                .into_iter()
                .map(|e| GeometryLayer {
                    name: e.name,
                    intensity: e.intensity,
                    visible: e.visible,
                    has_geometry: e.deltas.iter().any(|&d| d != DVec3::ZERO),
                    deltas: Arc::new(e.deltas),
                })
                .collect(),
            display_positions: Arc::new(mesh.positions.clone()),
        };
        stack.validate_composition(mesh)?;
        Ok(stack)
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_LAYER_NAME_BYTES || name.chars().any(char::is_control) {
        Err("Layer names require 1–256 UTF-8 bytes without control characters".into())
    } else {
        Ok(())
    }
}

fn positions_equal_bits(a: &[Vec3], b: &[Vec3]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits))
}

fn dvecs_equal_bits(a: &[DVec3], b: &[DVec3]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.to_array().map(f64::to_bits) == b.to_array().map(f64::to_bits))
}

fn validate_intensity(intensity: f32) -> Result<(), String> {
    if !intensity.is_finite() || !(0.0..=1.0).contains(&intensity) {
        Err("Layer intensity must be finite and between 0 and 1".into())
    } else {
        Ok(())
    }
}

fn validate_geometry(positions: &[Vec3], triangles: &[[u32; 3]]) -> Result<(), String> {
    for (face, ids) in triangles.iter().enumerate() {
        let [a, b, c] = ids.map(|id| positions[id as usize].as_dvec3());
        let area = (b - a).cross(c - a).length_squared();
        if !area.is_finite() || area == 0.0 {
            return Err(format!(
                "Geometry layers refuse a collapsed or nonfinite triangle at face {face}"
            ));
        }
    }
    Ok(())
}

/// All vector/triangle fields share this budget while decoding, regardless of
/// JSON field order. Capacity is charged before growth, not after all layers.
#[derive(Default)]
struct DecodeBudget {
    vectors: usize,
    triangles: usize,
    bytes: usize,
}

impl DecodeBudget {
    fn grow<T>(
        &mut self,
        values: &mut Vec<T>,
        maximum: usize,
        channel: crate::decode_limits::Channel,
    ) -> Result<(), String> {
        if values.len() < values.capacity() {
            return Ok(());
        }
        let desired = values
            .capacity()
            .saturating_mul(2)
            .max(8)
            .min(maximum)
            .min(values.len() + 1 + crate::decode_limits::remaining(channel));
        let available = (MAX_LAYER_STORAGE_BYTES - self.bytes) / std::mem::size_of::<T>();
        let extra = desired.saturating_sub(values.capacity()).min(available);
        if extra == 0 {
            return Err("Layer decode allocation exceeds 64 MiB".into());
        }
        let previous = values.capacity();
        crate::decode_limits::capacity(channel, extra * std::mem::size_of::<T>())?;
        values
            .try_reserve_exact(extra)
            .map_err(|_| "Cannot allocate layer project field")?;
        self.bytes = self
            .bytes
            .checked_add((values.capacity() - previous) * std::mem::size_of::<T>())
            .ok_or("Layer decode size overflow")?;
        if self.bytes > MAX_LAYER_STORAGE_BYTES {
            return Err("Layer decode allocation exceeds 64 MiB".into());
        }
        Ok(())
    }
}

struct VectorsSeed<'a>(&'a mut DecodeBudget);
impl<'de> DeserializeSeed<'de> for VectorsSeed<'_> {
    type Value = Vec<DVec3>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct VectorsVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for VectorsVisitor<'_> {
            type Value = Vec<DVec3>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bounded finite layer vectors")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                loop {
                    if self.0.vectors >= MAX_LAYER_VECTORS {
                        if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                            return Err(A::Error::custom(
                                "Layer aggregate vector allocation limit exceeded",
                            ));
                        }
                        break;
                    }
                    let Some(value) = seq.next_element::<DVec3>()? else {
                        break;
                    };
                    crate::decode_limits::element(crate::decode_limits::Channel::LayerVectors)
                        .map_err(A::Error::custom)?;
                    self.0
                        .grow(
                            &mut values,
                            MAX_LAYER_VECTORS,
                            crate::decode_limits::Channel::LayerVectors,
                        )
                        .map_err(A::Error::custom)?;
                    values.push(value);
                    self.0.vectors += 1;
                }
                Ok(values)
            }
        }
        deserializer.deserialize_seq(VectorsVisitor(self.0))
    }
}

struct TrianglesSeed<'a>(&'a mut DecodeBudget);
impl<'de> DeserializeSeed<'de> for TrianglesSeed<'_> {
    type Value = Vec<[u32; 3]>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct TrianglesVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for TrianglesVisitor<'_> {
            type Value = Vec<[u32; 3]>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bounded layer triangles")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                loop {
                    if self.0.triangles >= MAX_TRIANGLES {
                        if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                            return Err(A::Error::custom(
                                "Layer triangle allocation limit exceeded",
                            ));
                        }
                        break;
                    }
                    let Some(value) = seq.next_element::<[u32; 3]>()? else {
                        break;
                    };
                    crate::decode_limits::element(crate::decode_limits::Channel::LayerTriangles)
                        .map_err(A::Error::custom)?;
                    self.0
                        .grow(
                            &mut values,
                            MAX_TRIANGLES,
                            crate::decode_limits::Channel::LayerTriangles,
                        )
                        .map_err(A::Error::custom)?;
                    values.push(value);
                    self.0.triangles += 1;
                }
                Ok(values)
            }
        }
        deserializer.deserialize_seq(TrianglesVisitor(self.0))
    }
}

struct LayerSeed<'a>(&'a mut DecodeBudget);
impl<'de> DeserializeSeed<'de> for LayerSeed<'_> {
    type Value = LayerProject;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct LayerVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for LayerVisitor<'_> {
            type Value = LayerProject;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a geometry layer")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let (mut name, mut intensity, mut visible, mut deltas) = (None, None, None, None);
                while let Some(key) = map.next_key::<LayerKey>()?.map(|key| key.0) {
                    match key.as_str() {
                        "name" => {
                            if name.is_some() {
                                return Err(A::Error::duplicate_field("name"));
                            }
                            let value = map.next_value::<LayerName>()?;
                            name = Some(value.0);
                        }
                        "intensity" => {
                            if intensity.is_some() {
                                return Err(A::Error::duplicate_field("intensity"));
                            }
                            intensity = Some(map.next_value()?);
                        }
                        "visible" => {
                            if visible.is_some() {
                                return Err(A::Error::duplicate_field("visible"));
                            }
                            visible = Some(map.next_value()?);
                        }
                        "deltas" => {
                            if deltas.is_some() {
                                return Err(A::Error::duplicate_field("deltas"));
                            }
                            deltas = Some(map.next_value_seed(VectorsSeed(self.0))?);
                        }
                        _ => {
                            return Err(A::Error::unknown_field(
                                &key,
                                &["name", "intensity", "visible", "deltas"],
                            ));
                        }
                    }
                }
                Ok(LayerProject {
                    name: name.ok_or_else(|| A::Error::missing_field("name"))?,
                    intensity: intensity.ok_or_else(|| A::Error::missing_field("intensity"))?,
                    visible: visible.ok_or_else(|| A::Error::missing_field("visible"))?,
                    deltas: deltas.ok_or_else(|| A::Error::missing_field("deltas"))?,
                })
            }
        }
        deserializer.deserialize_map(LayerVisitor(self.0))
    }
}

struct EntriesSeed<'a>(&'a mut DecodeBudget);
impl<'de> DeserializeSeed<'de> for EntriesSeed<'_> {
    type Value = Vec<LayerProject>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct EntriesVisitor<'a>(&'a mut DecodeBudget);
        impl<'de> Visitor<'de> for EntriesVisitor<'_> {
            type Value = Vec<LayerProject>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("at most sixteen geometry layers")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                loop {
                    if entries.len() >= MAX_SCULPT_LAYERS {
                        if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                            return Err(A::Error::custom("Too many geometry layers"));
                        }
                        break;
                    }
                    let Some(entry) = seq.next_element_seed(LayerSeed(self.0))? else {
                        break;
                    };
                    entries.push(entry);
                }
                Ok(entries)
            }
        }
        deserializer.deserialize_seq(EntriesVisitor(self.0))
    }
}

struct LayerName(String);
impl<'de> Deserialize<'de> for LayerName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NameVisitor;
        impl Visitor<'_> for NameVisitor {
            type Value = LayerName;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a bounded layer name")
            }
            fn visit_str<E: serde::de::Error>(self, name: &str) -> Result<Self::Value, E> {
                validate_name(name).map_err(E::custom)?;
                Ok(LayerName(name.into()))
            }
        }
        deserializer.deserialize_str(NameVisitor)
    }
}

struct LayerKey(String);
impl<'de> Deserialize<'de> for LayerKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeyVisitor;
        impl Visitor<'_> for KeyVisitor {
            type Value = LayerKey;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a short layer metadata field name")
            }
            fn visit_str<E: serde::de::Error>(self, key: &str) -> Result<Self::Value, E> {
                if key.len() > 32 {
                    return Err(E::custom("Layer metadata field name is too long"));
                }
                Ok(LayerKey(key.into()))
            }
        }
        deserializer.deserialize_identifier(KeyVisitor)
    }
}

impl<'de> Deserialize<'de> for LayersProject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ProjectVisitor;
        impl<'de> Visitor<'de> for ProjectVisitor {
            type Value = LayersProject;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bounded geometry-layer state")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut budget = DecodeBudget::default();
                let (mut scheme, mut base, mut triangles, mut entries, mut recording) =
                    (None, None, None, None, None);
                while let Some(key) = map.next_key::<LayerKey>()?.map(|key| key.0) {
                    match key.as_str() {
                        "scheme" => {
                            if scheme.is_some() {
                                return Err(A::Error::duplicate_field("scheme"));
                            }
                            scheme = Some(map.next_value::<LayerScheme>()?);
                        }
                        "base_positions" => {
                            if base.is_some() {
                                return Err(A::Error::duplicate_field("base_positions"));
                            }
                            base = Some(map.next_value_seed(VectorsSeed(&mut budget))?);
                        }
                        "triangles" => {
                            if triangles.is_some() {
                                return Err(A::Error::duplicate_field("triangles"));
                            }
                            triangles = Some(map.next_value_seed(TrianglesSeed(&mut budget))?);
                        }
                        "entries" => {
                            if entries.is_some() {
                                return Err(A::Error::duplicate_field("entries"));
                            }
                            entries = Some(map.next_value_seed(EntriesSeed(&mut budget))?);
                        }
                        "recording_layer" => {
                            if recording.is_some() {
                                return Err(A::Error::duplicate_field("recording_layer"));
                            }
                            recording = Some(map.next_value::<Option<usize>>()?);
                        }
                        _ => {
                            return Err(A::Error::unknown_field(
                                &key,
                                &[
                                    "scheme",
                                    "base_positions",
                                    "triangles",
                                    "entries",
                                    "recording_layer",
                                ],
                            ));
                        }
                    }
                }
                scheme.ok_or_else(|| A::Error::missing_field("scheme"))?;
                Ok(LayersProject {
                    base_positions: base
                        .ok_or_else(|| A::Error::missing_field("base_positions"))?,
                    triangles: triangles.ok_or_else(|| A::Error::missing_field("triangles"))?,
                    entries: entries.ok_or_else(|| A::Error::missing_field("entries"))?,
                    recording_layer: recording.unwrap_or(None),
                })
            }
        }
        deserializer.deserialize_map(ProjectVisitor)
    }
}
