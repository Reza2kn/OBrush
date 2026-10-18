//! Portable surface attributes. UVs belong to face corners, not welded vertices.
//! Original quad faces use the explicit (0,1,2)/(0,2,3) render diagonal.
use crate::{Mesh, MAX_TRIANGLES, MAX_VERTICES};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

pub const MAX_UV_COORDINATES: usize = MAX_TRIANGLES * 3;
pub const TRIANGLE_CORNER: u32 = u32::MAX;
/// Used only when concatenating an implicit legacy material with explicit materials.
/// These are the pinned SculptGL defaults, not a claim about viewport appearance.
pub const DEFAULT_VERTEX_MATERIAL: [f32; 2] = [0.18, 0.08];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributeExportPolicy {
    RejectLoss,
    AllowAndReport,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttributeExportReport {
    pub omitted_material_vertices: usize,
    /// Nondefault canonical mask data cannot be preserved by these geometry formats.
    pub omitted_source_mask_vertices: usize,
    pub omitted_original_faces: usize,
    pub omitted_uv_coordinates: usize,
    pub omitted_uv_faces: usize,
}

#[derive(Clone, Debug)]
pub struct AttributeExport {
    pub bytes: Vec<u8>,
    pub report: AttributeExportReport,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CornerUv {
    #[serde(deserialize_with = "deserialize_uv_coordinates")]
    pub coordinates: Vec<[f32; 2]>,
    #[serde(deserialize_with = "deserialize_uv_faces")]
    pub face_indices: Vec<[u32; 4]>,
}

/// A saved child may retain extra source UV IDs after Reverse renumbers its
/// geometry. Every physical corner must keep the predicted value, and one
/// saved ID may never merge two distinct predicted chart IDs. Splitting a
/// predicted ID into several saved aliases is lossless for that child level.
pub(crate) fn uv_corner_refinement_matches(predicted: &CornerUv, saved: &CornerUv) -> bool {
    if predicted.face_indices.len() != saved.face_indices.len() {
        return false;
    }
    let mut saved_to_predicted = vec![TRIANGLE_CORNER; saved.coordinates.len()];
    for (expected, actual) in predicted.face_indices.iter().zip(&saved.face_indices) {
        let count = if expected[3] == TRIANGLE_CORNER { 3 } else { 4 };
        if (actual[3] == TRIANGLE_CORNER) != (count == 3) {
            return false;
        }
        for corner in 0..count {
            let expected_id = expected[corner] as usize;
            let actual_id = actual[corner] as usize;
            let (Some(expected_uv), Some(actual_uv), Some(prior)) = (
                predicted.coordinates.get(expected_id),
                saved.coordinates.get(actual_id),
                saved_to_predicted.get_mut(actual_id),
            ) else {
                return false;
            };
            if expected_uv.map(f32::to_bits) != actual_uv.map(f32::to_bits)
                || (*prior != TRIANGLE_CORNER && *prior != expected_id as u32)
            {
                return false;
            }
            *prior = expected_id as u32;
        }
    }
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceTopology {
    #[serde(deserialize_with = "deserialize_faces")]
    pub faces: Vec<[u32; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uv: Option<CornerUv>,
}

impl SurfaceTopology {
    pub fn validate(&self, vertices: usize, triangles: &[[u32; 3]]) -> Result<(), String> {
        if self.faces.is_empty() || self.faces.len() > MAX_TRIANGLES {
            return Err("Original face metadata requires 1–2000000 faces".into());
        }
        let mut next = 0;
        for face in &self.faces {
            let arity = if face[3] == TRIANGLE_CORNER { 3 } else { 4 };
            for (corner, &index) in face[..arity].iter().enumerate() {
                if index as usize >= vertices || face[..corner].contains(&index) {
                    return Err("Original face has a missing or repeated vertex".into());
                }
            }
            let first = [face[0], face[1], face[2]];
            if triangles.get(next) != Some(&first) {
                return Err("Original faces do not match the canonical triangle array".into());
            }
            next += 1;
            if arity == 4 {
                let second = [face[0], face[2], face[3]];
                if triangles.get(next) != Some(&second) {
                    return Err("Quad metadata requires the explicit 0–2 diagonal".into());
                }
                next += 1;
            }
        }
        if next != triangles.len() {
            return Err("Original face metadata does not cover every triangle".into());
        }
        if let Some(uv) = &self.uv {
            if uv.coordinates.is_empty() || uv.coordinates.len() > MAX_UV_COORDINATES {
                return Err("UV coordinate allocation limit exceeded or empty UV channel".into());
            }
            if uv.face_indices.len() != self.faces.len() {
                return Err("UV face indices must cover every original face".into());
            }
            if uv
                .coordinates
                .iter()
                .flatten()
                .any(|&v| !v.is_finite() || v.abs() > 1_000_000.0)
            {
                return Err("UV coordinates must be finite within ±1000000".into());
            }
            for (face, indices) in self.faces.iter().zip(&uv.face_indices) {
                let arity = if face[3] == TRIANGLE_CORNER { 3 } else { 4 };
                if (indices[3] == TRIANGLE_CORNER) != (arity == 3)
                    || indices[..arity]
                        .iter()
                        .any(|&i| i as usize >= uv.coordinates.len())
                {
                    return Err("UV corner index or face arity is invalid".into());
                }
            }
        }
        Ok(())
    }

    pub fn has_quads(&self) -> bool {
        self.faces.iter().any(|face| face[3] != TRIANGLE_CORNER)
    }

    pub(crate) fn approximate_bytes(&self) -> usize {
        self.faces.capacity() * std::mem::size_of::<[u32; 4]>()
            + self.uv.as_ref().map_or(0, |uv| {
                uv.coordinates.capacity() * std::mem::size_of::<[f32; 2]>()
                    + uv.face_indices.capacity() * std::mem::size_of::<[u32; 4]>()
            })
    }

    /// UV edge identity includes geometry and both corner UV IDs. This retains
    /// equal-valued but distinct seams, including edges with reversed winding.
    pub(crate) fn subdivided_triangles(&self) -> Result<Self, String> {
        if self.has_quads() {
            return Err("Triangle subdivision cannot preserve original quads; use a quad subdivision path or explicitly detach original face/UV metadata".into());
        }
        let face_count = self
            .faces
            .len()
            .checked_mul(4)
            .ok_or("UV face count overflow")?;
        if face_count > MAX_TRIANGLES {
            return Err("UV subdivision exceeds the face limit".into());
        }
        let Some(uv) = &self.uv else {
            // Geometry faces are assigned from the actual subdivision result.
            return Ok(Self {
                faces: Vec::new(),
                uv: None,
            });
        };
        // A worst-case bound ensures seam scratch/output is bounded before allocation.
        let maximum = uv
            .coordinates
            .len()
            .checked_add(self.faces.len().checked_mul(3).ok_or("UV count overflow")?)
            .ok_or("UV count overflow")?;
        if maximum > MAX_UV_COORDINATES {
            return Err("UV subdivision exceeds the coordinate limit".into());
        }
        let mut coordinates = uv.coordinates.clone();
        coordinates
            .try_reserve_exact(maximum - coordinates.len())
            .map_err(|_| "Cannot allocate bounded UV coordinates")?;
        let mut edges = HashMap::<((u32, u32), (u32, u32)), u32>::new();
        let mut face_indices = Vec::new();
        face_indices
            .try_reserve_exact(face_count)
            .map_err(|_| "Cannot allocate subdivided UV faces")?;
        for (face, indices) in self.faces.iter().zip(&uv.face_indices) {
            let mut midpoint = |a: usize, b: usize| -> Result<u32, String> {
                let geometry = (face[a].min(face[b]), face[a].max(face[b]));
                let texture = if face[a] < face[b] {
                    (indices[a], indices[b])
                } else {
                    (indices[b], indices[a])
                };
                if let Some(&index) = edges.get(&(geometry, texture)) {
                    return Ok(index);
                }
                coordinates
                    .try_reserve(1)
                    .map_err(|_| "Cannot allocate subdivided UV coordinates")?;
                edges
                    .try_reserve(1)
                    .map_err(|_| "Cannot allocate UV edge map")?;
                let index = coordinates.len() as u32;
                coordinates.push(std::array::from_fn(|channel| {
                    (uv.coordinates[indices[a] as usize][channel]
                        + uv.coordinates[indices[b] as usize][channel])
                        * 0.5
                }));
                edges.insert((geometry, texture), index);
                Ok(index)
            };
            let [a, b, c, _] = *indices;
            let ab = midpoint(0, 1)?;
            let bc = midpoint(1, 2)?;
            let ca = midpoint(2, 0)?;
            face_indices.extend([
                [a, ab, ca, TRIANGLE_CORNER],
                [b, bc, ab, TRIANGLE_CORNER],
                [c, ca, bc, TRIANGLE_CORNER],
                [ab, bc, ca, TRIANGLE_CORNER],
            ]);
        }
        Ok(Self {
            faces: Vec::new(),
            uv: Some(CornerUv {
                coordinates,
                face_indices,
            }),
        })
    }
}

impl Mesh {
    /// A deliberate export loss policy never removes attributes from the source
    /// mesh or consumes an editing/history step.
    pub fn export_stl_with_policy(
        &self,
        encoding: crate::StlEncoding,
        policy: AttributeExportPolicy,
    ) -> Result<AttributeExport, String> {
        self.export_attributes_with_policy(policy, false, |m| m.to_stl(encoding))
    }
    pub fn export_ply_with_policy(
        &self,
        encoding: crate::PlyEncoding,
        policy: AttributeExportPolicy,
    ) -> Result<AttributeExport, String> {
        self.export_attributes_with_policy(policy, false, |m| m.to_ply(encoding))
    }
    pub fn export_obj_with_policy(
        &self,
        policy: AttributeExportPolicy,
    ) -> Result<AttributeExport, String> {
        self.export_attributes_with_policy(policy, true, |m| {
            m.export_obj_checked().map(String::into_bytes)
        })
    }
    fn export_attributes_with_policy(
        &self,
        policy: AttributeExportPolicy,
        preserve_surface: bool,
        write: impl FnOnce(&Mesh) -> Result<Vec<u8>, String>,
    ) -> Result<AttributeExport, String> {
        self.validate()?;
        let report = AttributeExportReport {
            omitted_material_vertices: self.materials.as_ref().map_or(0, Vec::len),
            omitted_source_mask_vertices: if self.mask_convention.is_default() {
                0
            } else {
                self.masks.len()
            },
            omitted_original_faces: if preserve_surface {
                0
            } else {
                self.surface.as_ref().map_or(0, |s| s.faces.len())
            },
            omitted_uv_coordinates: if preserve_surface {
                0
            } else {
                self.uv_coordinate_count()
            },
            omitted_uv_faces: if preserve_surface {
                0
            } else {
                self.surface
                    .as_ref()
                    .and_then(|s| s.uv.as_ref())
                    .map_or(0, |uv| uv.face_indices.len())
            },
        };
        if policy == AttributeExportPolicy::RejectLoss && report != AttributeExportReport::default()
        {
            return Err("Selected format cannot preserve all mesh attributes; choose an explicit allow-and-report export policy".into());
        }
        if report == AttributeExportReport::default() {
            return Ok(AttributeExport {
                bytes: write(self)?,
                report,
            });
        }
        let mut portable = self.snapshot();
        portable.materials = None;
        // These formats omit masks; clearing only this temporary export view
        // lets their checked writer run without converting canonical words.
        portable.mask_convention = crate::MaskConvention::ProtectedF32V1;
        portable.masks.fill(0.0);
        if !preserve_surface {
            portable.surface = None;
        }
        Ok(AttributeExport {
            bytes: write(&portable)?,
            report,
        })
    }

    /// Validate all new channels before installing either; failures are atomic.
    pub fn install_attributes(
        &mut self,
        materials: Option<Vec<[f32; 2]>>,
        surface: Option<SurfaceTopology>,
    ) -> Result<(), String> {
        self.validate()?;
        validate_materials(materials.as_deref(), self.positions.len())?;
        if let Some(surface) = &surface {
            surface.validate(self.positions.len(), &self.triangles)?;
        }
        let bytes = self
            .approximate_bytes()
            .saturating_sub(self.attribute_bytes())
            .saturating_add(materials.as_ref().map_or(0, |m| m.capacity() * 8))
            .saturating_add(
                surface
                    .as_ref()
                    .map_or(0, SurfaceTopology::approximate_bytes),
            );
        if bytes > crate::decode_limits::MAX_SCENE_STORAGE_BYTES {
            return Err("Mesh attribute storage exceeds the scene allocation budget".into());
        }
        self.materials = materials;
        self.surface = surface;
        Ok(())
    }

    /// Explicitly detach original face/UV metadata, retaining geometry/materials.
    /// Hosts must expose this as a deliberate undoable choice before unsupported topology edits.
    pub fn detach_surface_topology(&mut self) -> Option<SurfaceTopology> {
        self.surface.take()
    }

    pub fn has_attributes(&self) -> bool {
        !self.mask_convention.is_default() || self.materials.is_some() || self.surface.is_some()
    }
    pub fn supports_adaptive_detail(&self) -> bool {
        self.surface.is_none()
    }

    pub(crate) fn require_plain_topology(&self, operation: &str) -> Result<(), String> {
        if self.surface.is_some() {
            Err(format!(
                "{operation} cannot preserve original face/UV metadata; explicitly detach it before changing topology"
            ))
        } else {
            Ok(())
        }
    }
    pub(crate) fn attribute_encoded_bound(&self) -> usize {
        self.materials
            .as_ref()
            .map_or(0, |m| m.len().saturating_mul(52).saturating_add(32))
            .saturating_add(self.surface.as_ref().map_or(0, |s| {
                s.faces
                    .len()
                    .saturating_mul(46)
                    .saturating_add(64)
                    .saturating_add(s.uv.as_ref().map_or(0, |uv| {
                        uv.coordinates
                            .len()
                            .saturating_mul(52)
                            .saturating_add(uv.face_indices.len().saturating_mul(46))
                            .saturating_add(64)
                    }))
            }))
    }
    pub(crate) fn uv_coordinate_count(&self) -> usize {
        self.surface
            .as_ref()
            .and_then(|s| s.uv.as_ref())
            .map_or(0, |uv| uv.coordinates.len())
    }
    pub(crate) fn attribute_bytes(&self) -> usize {
        self.materials.as_ref().map_or(0, |m| m.capacity() * 8)
            + self
                .surface
                .as_ref()
                .map_or(0, SurfaceTopology::approximate_bytes)
    }
    pub(crate) fn attributes_equal_bits(&self, other: &Self) -> bool {
        let materials = match (&self.materials, &other.materials) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
            }
            _ => false,
        };
        self.mask_convention == other.mask_convention
            && materials
            && self.surface_matches_bits(other)
    }
    pub(crate) fn surface_matches_bits(&self, other: &Self) -> bool {
        match (&self.surface, &other.surface) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.faces == b.faces
                    && match (&a.uv, &b.uv) {
                        (None, None) => true,
                        (Some(a), Some(b)) => {
                            a.face_indices == b.face_indices
                                && a.coordinates.len() == b.coordinates.len()
                                && a.coordinates
                                    .iter()
                                    .zip(&b.coordinates)
                                    .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
                        }
                        _ => false,
                    }
            }
            _ => false,
        }
    }
}

pub(crate) fn validate_materials(values: Option<&[[f32; 2]]>, count: usize) -> Result<(), String> {
    if let Some(values) = values {
        if values.len() != count || values.len() > MAX_VERTICES {
            return Err("Material channels must match the bounded vertex count".into());
        }
        if values
            .iter()
            .flatten()
            .any(|&v| !v.is_finite() || !(0.0..=1.0).contains(&v))
        {
            return Err("Roughness and metallic must be finite within 0–1".into());
        }
    }
    Ok(())
}

pub(crate) fn deserialize_materials<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<Vec<[f32; 2]>>, D::Error> {
    struct MaterialRows(Vec<[f32; 2]>);
    impl<'de> Deserialize<'de> for MaterialRows {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            crate::mesh::bounded_vec::<D, [f32; 2], MAX_VERTICES>(
                d,
                Some(crate::decode_limits::Channel::Materials),
            )
            .map(Self)
        }
    }
    Option::<MaterialRows>::deserialize(d).map(|value| value.map(|v| v.0))
}
fn deserialize_faces<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<[u32; 4]>, D::Error> {
    crate::mesh::bounded_vec::<D, [u32; 4], MAX_TRIANGLES>(
        d,
        Some(crate::decode_limits::Channel::SurfaceFaces),
    )
}
fn deserialize_uv_faces<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<[u32; 4]>, D::Error> {
    crate::mesh::bounded_vec::<D, [u32; 4], MAX_TRIANGLES>(
        d,
        Some(crate::decode_limits::Channel::UvFaces),
    )
}
fn deserialize_uv_coordinates<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<[f32; 2]>, D::Error> {
    crate::mesh::bounded_vec::<D, [f32; 2], MAX_UV_COORDINATES>(
        d,
        Some(crate::decode_limits::Channel::UvCoordinates),
    )
}

#[cfg(test)]
mod limits {
    use super::*;
    use serde::de::{DeserializeSeed, SeqAccess, Visitor};

    struct Hint(usize);
    impl<'de> Deserializer<'de> for Hint {
        type Error = serde::de::value::Error;
        fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            visitor.visit_seq(self)
        }
        fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            visitor.visit_some(self)
        }
        serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
    }
    impl<'de> SeqAccess<'de> for Hint {
        type Error = serde::de::value::Error;
        fn next_element_seed<T: DeserializeSeed<'de>>(
            &mut self,
            _: T,
        ) -> Result<Option<T::Value>, Self::Error> {
            panic!("Hostile length hint must fail before reading or allocating an element")
        }
        fn size_hint(&self) -> Option<usize> {
            Some(self.0)
        }
    }

    #[test]
    fn oversized_material_hint_fails_before_allocation() {
        assert!(deserialize_materials(Hint(MAX_VERTICES + 1)).is_err());
    }
    #[test]
    fn oversized_original_face_hint_fails_before_allocation() {
        assert!(deserialize_faces(Hint(MAX_TRIANGLES + 1)).is_err());
    }
    #[test]
    fn oversized_uv_coordinate_hint_fails_before_allocation() {
        assert!(deserialize_uv_coordinates(Hint(MAX_UV_COORDINATES + 1)).is_err());
    }
    #[test]
    fn oversized_uv_face_hint_fails_before_allocation() {
        assert!(deserialize_uv_faces(Hint(MAX_TRIANGLES + 1)).is_err());
    }
    #[test]
    fn attributes_share_decoded_capacity_budget_with_positions_and_layers() {
        let _scope = crate::decode_limits::DecodeScope::enter();
        crate::decode_limits::capacity(
            crate::decode_limits::Channel::Positions,
            crate::decode_limits::MAX_SCENE_STORAGE_BYTES,
        )
        .unwrap();
        let mut input = serde_json::Deserializer::from_str("[[0.2,0.3]]");
        assert!(deserialize_materials(&mut input).is_err());
        let mut input = serde_json::Deserializer::from_str("[[0.2,0.3]]");
        assert!(deserialize_uv_coordinates(&mut input).is_err());
    }
    #[test]
    fn topology_metadata_bytes_and_reserved_capacity_are_counted() {
        let mut mesh = Mesh::sphere(0);
        let before = mesh.approximate_bytes();
        mesh.materials = Some(vec![[0.2, 0.3]; mesh.positions.len()]);
        assert_eq!(mesh.approximate_bytes() - before, mesh.positions.len() * 8);
        let before = mesh.approximate_bytes();
        mesh.materials.as_mut().unwrap().reserve_exact(4096);
        assert!(mesh.approximate_bytes() >= before + 4096 * 8);
        let surface = SurfaceTopology {
            faces: vec![[0, 1, 2, TRIANGLE_CORNER]],
            uv: Some(CornerUv {
                coordinates: vec![[0., 0.]; 3],
                face_indices: vec![[0, 1, 2, TRIANGLE_CORNER]],
            }),
        };
        assert_eq!(surface.approximate_bytes(), 16 + 3 * 8 + 16);
    }
    #[test]
    fn uv_aggregate_budget_counts_all_mesh_and_level_rows() {
        let _scope = crate::decode_limits::DecodeScope::enter();
        for _ in 0..MAX_UV_COORDINATES {
            crate::decode_limits::element(crate::decode_limits::Channel::UvCoordinates).unwrap();
        }
        assert!(deserialize_uv_coordinates(Hint(1)).is_err());
    }
    #[test]
    fn material_budget_scope_restores_after_a_failed_decode() {
        {
            let _scope = crate::decode_limits::DecodeScope::enter();
            crate::decode_limits::capacity(
                crate::decode_limits::Channel::Colors,
                crate::decode_limits::MAX_SCENE_STORAGE_BYTES,
            )
            .unwrap();
            let mut input = serde_json::Deserializer::from_str("[[0.2,0.3]]");
            assert!(deserialize_materials(&mut input).is_err());
        }
        let _scope = crate::decode_limits::DecodeScope::enter();
        let mut input = serde_json::Deserializer::from_str("[[0.2,0.3]]");
        assert_eq!(
            deserialize_materials(&mut input).unwrap(),
            Some(vec![[0.2, 0.3]])
        );
    }
}
