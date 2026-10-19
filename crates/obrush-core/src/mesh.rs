use glam::{Vec2, Vec3};
use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

/// Explicit limits apply to imports, deserialization, and topology changes.
pub const MAX_VERTICES: usize = 1_000_000;
pub const MAX_TRIANGLES: usize = 2_000_000;
pub(crate) const MAX_INPUT_BYTES: usize = crate::resource_limits::MAX_MESH_INPUT_BYTES;
const MAX_COORDINATE: f32 = 1_000_000.0;
const MAX_FACE_VERTICES: usize = 512;

/// Whether both triangles of a quad have a sound, consistently oriented 0–2
/// diagonal. Shared by OBJ and PLY import before retaining original faces.
pub(crate) fn canonical_quad_valid(polygon: &[u32], positions: &[Vec3]) -> bool {
    if polygon.len() != 4 {
        return false;
    }
    let p: [_; 4] = std::array::from_fn(|corner| positions[polygon[corner] as usize].as_dvec3());
    let first = (p[1] - p[0]).cross(p[2] - p[0]);
    let second = (p[2] - p[0]).cross(p[3] - p[0]);
    let normal = first + second;
    let scale = p
        .iter()
        .map(|&point| (point - p[0]).length_squared())
        .fold(0.0_f64, f64::max);
    let epsilon = scale * scale * 1e-12;
    first.dot(normal) > epsilon && second.dot(normal) > epsilon
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mesh {
    #[serde(deserialize_with = "deserialize_positions")]
    pub positions: Vec<Vec3>,
    #[serde(deserialize_with = "deserialize_normals")]
    pub normals: Vec<Vec3>,
    #[serde(deserialize_with = "deserialize_triangles")]
    pub triangles: Vec<[u32; 3]>,
    #[serde(deserialize_with = "deserialize_masks")]
    /// Canonical values interpreted through mask_convention.
    pub masks: Vec<f32>,
    #[serde(default, skip_serializing_if = "crate::MaskConvention::is_default")]
    pub mask_convention: crate::MaskConvention,
    #[serde(deserialize_with = "deserialize_colors")]
    pub colors: Vec<[f32; 3]>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::attributes::deserialize_materials"
    )]
    pub materials: Option<Vec<[f32; 2]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<crate::SurfaceTopology>,
    #[serde(skip)]
    adjacency: Option<Arc<Adjacency>>,
    #[serde(skip)]
    resident_normal_witness: Option<Arc<ResidentNormalWitness>>,
}

#[derive(Clone, Debug)]
struct ResidentNormalWitness {
    positions: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    normals: Vec<Vec3>,
}

#[derive(Clone, Debug)]
struct Adjacency {
    signature: u64,
    neighbor_offsets: Vec<usize>,
    neighbor_indices: Vec<usize>,
    vertex_face_offsets: Vec<usize>,
    vertex_faces: Vec<u32>,
    face_pairs: Vec<[u32; 2]>,
    face_pair_offsets: Vec<usize>,
    face_pair_indices: Vec<usize>,
    nonmanifold_faces: Vec<u32>,
    invalid_indices: bool,
    normal_only: bool,
}

struct SubdivisionEdge {
    vertices: [u32; 2],
    opposites: [u32; 2],
    faces: [usize; 2],
    face_count: usize,
    forward: bool,
}

struct SmoothSubdivisionTopology {
    edges: Vec<SubdivisionEdge>,
    neighbors: Vec<Vec<u32>>,
    boundary_neighbors: Vec<Vec<u32>>,
    triangle_edges: Vec<[usize; 3]>,
}

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub position: Vec3,
    pub normal: Vec3,
    /// World-space distance, independent of the input direction's length.
    pub distance: f32,
    /// Face and interpolation weights identify the picked material point while
    /// vertex-only edits deform the mesh. Topology edits invalidate this hit.
    pub triangle: [u32; 3],
    pub barycentric: [f32; 3],
}

impl Hit {
    /// Follow the actual deformed point, including masks, falloff and clipped
    /// steps. This does not re-pick a different point under the moving cursor.
    pub fn surface_point(&self, mesh: &Mesh) -> Option<(Vec3, Vec3)> {
        let mut position = Vec3::ZERO;
        let mut normal = Vec3::ZERO;
        for (&vertex, &weight) in self.triangle.iter().zip(&self.barycentric) {
            position += *mesh.positions.get(vertex as usize)? * weight;
            normal += *mesh.normals.get(vertex as usize)? * weight;
        }
        let normal = normal.normalize_or_zero();
        (position.is_finite() && normal.is_finite() && normal != Vec3::ZERO)
            .then_some((position, normal))
    }
}

impl Mesh {
    /// Number of editable polygon faces. Retained source quads count once,
    /// while meshes without original face topology count rendered triangles.
    pub fn face_count(&self) -> usize {
        self.surface
            .as_ref()
            .map_or(self.triangles.len(), |surface| surface.faces.len())
    }
    pub(crate) fn from_resident_upload_parts(
        positions: Vec<Vec3>,
        normals: Vec<Vec3>,
        triangles: Vec<[u32; 3]>,
        masks: Vec<f32>,
        colors: Vec<[f32; 3]>,
        surface: Option<crate::SurfaceTopology>,
    ) -> Self {
        Self {
            positions,
            normals,
            triangles,
            masks,
            mask_convention: crate::MaskConvention::UnprotectedF32V1,
            colors,
            materials: None,
            surface,
            adjacency: None,
            resident_normal_witness: None,
        }
    }
    /// Unit-radius icosphere. Levels above eight are clamped to bound allocation.
    pub fn sphere(subdivisions: u32) -> Self {
        let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let positions = vec![
            Vec3::new(-1.0, t, 0.0),
            Vec3::new(1.0, t, 0.0),
            Vec3::new(-1.0, -t, 0.0),
            Vec3::new(1.0, -t, 0.0),
            Vec3::new(0.0, -1.0, t),
            Vec3::new(0.0, 1.0, t),
            Vec3::new(0.0, -1.0, -t),
            Vec3::new(0.0, 1.0, -t),
            Vec3::new(t, 0.0, -1.0),
            Vec3::new(t, 0.0, 1.0),
            Vec3::new(-t, 0.0, -1.0),
            Vec3::new(-t, 0.0, 1.0),
        ]
        .into_iter()
        .map(Vec3::normalize)
        .collect();
        let triangles = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];
        let mut mesh = Self::new(positions, triangles);
        for _ in 0..subdivisions.min(8) {
            if mesh.subdivide().is_err() {
                break;
            }
            for p in &mut mesh.positions {
                *p = p.normalize_or_zero();
            }
        }
        mesh.recompute_normals();
        mesh
    }

    /// A unit half-extent cube, subdivided into a useful starting sculpt mesh.
    pub fn cube() -> Self {
        let positions = vec![
            Vec3::new(-1., -1., -1.),
            Vec3::new(1., -1., -1.),
            Vec3::new(1., 1., -1.),
            Vec3::new(-1., 1., -1.),
            Vec3::new(-1., -1., 1.),
            Vec3::new(1., -1., 1.),
            Vec3::new(1., 1., 1.),
            Vec3::new(-1., 1., 1.),
        ];
        let triangles = vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [3, 7, 6],
            [3, 6, 2],
            [0, 4, 7],
            [0, 7, 3],
            [1, 2, 6],
            [1, 6, 5],
        ];
        let mut mesh = Self::new(positions, triangles);
        for _ in 0..4 {
            mesh.subdivide().expect("bounded cube subdivision");
        }
        mesh
    }

    pub(crate) fn new(positions: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Self {
        let count = positions.len();
        let mut mesh = Self {
            positions,
            triangles,
            normals: vec![Vec3::ZERO; count],
            masks: vec![0.0; count],
            mask_convention: crate::MaskConvention::default(),
            colors: vec![[0.65, 0.69, 0.74]; count],
            materials: None,
            surface: None,
            adjacency: None,
            resident_normal_witness: None,
        };
        mesh.recompute_normals();
        mesh
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_with_retained_geometry(false, false)
    }

    /// The caller must hold a current retained geometry witness built from a
    /// previously validated mesh. Only position and triangle scans are elided;
    /// mutable attributes, surface metadata and allocation limits are checked
    /// again on every public contact preparation.
    pub(crate) fn validate_retained_geometry(&self) -> Result<(), String> {
        self.validate_with_retained_geometry(true, false)
    }

    /// Only for an exclusive borrow with both exact retained geometry and an
    /// unchanged, previously validated stroke-domain witness. Still rechecks
    /// lengths and allocation limits, which can change without changing words.
    pub(crate) fn validate_retained_exact_domain(&self) -> Result<(), String> {
        self.validate_with_retained_geometry(true, true)
    }

    /// Resident sparse replacement only: the caller must have captured a
    /// fully validated source, matched its exact serialized mesh fingerprint,
    /// cloned that source, and changed only decoded finite position rows under
    /// the fold guard. Recheck lengths and storage limits before publication.
    pub(crate) fn validate_resident_sparse_replacement(&self) -> Result<(), String> {
        self.validate_with_retained_geometry(true, true)
    }

    fn validate_with_retained_geometry(
        &self,
        retained_geometry: bool,
        exact_domain: bool,
    ) -> Result<(), String> {
        #[cfg(test)]
        validation_probe::record();
        let count = self.positions.len();
        if count == 0 || count > MAX_VERTICES {
            return Err(format!("Mesh must have 1–{MAX_VERTICES} vertices"));
        }
        if self.triangles.is_empty() || self.triangles.len() > MAX_TRIANGLES {
            return Err(format!("Mesh must have 1–{MAX_TRIANGLES} triangles"));
        }
        if self.normals.len() != count || self.masks.len() != count || self.colors.len() != count {
            return Err("Normals, masks, and colors must match the vertex count".into());
        }
        if !retained_geometry {
            for (i, p) in self.positions.iter().enumerate() {
                if !p.is_finite() || p.abs().max_element() > MAX_COORDINATE {
                    return Err(format!(
                        "Vertex {i} has a non-finite or excessive coordinate"
                    ));
                }
            }
        }
        if !exact_domain {
            for (i, n) in self.normals.iter().enumerate() {
                if !n.is_finite() || !n.length_squared().is_finite() {
                    return Err(format!("Normal {i} is not finite"));
                }
            }
            for (i, m) in self.masks.iter().enumerate() {
                if !m.is_finite() || !(0.0..=1.0).contains(m) {
                    return Err(format!("Mask {i} must be in 0–1"));
                }
            }
            for (i, color) in self.colors.iter().enumerate() {
                if color
                    .iter()
                    .any(|c| !c.is_finite() || !(0.0..=1.0).contains(c))
                {
                    return Err(format!("Color {i} must have finite channels in 0–1"));
                }
            }
        }
        if !retained_geometry {
            for (i, tri) in self.triangles.iter().enumerate() {
                if tri.iter().any(|&v| v as usize >= count) {
                    return Err(format!("Triangle {i} refers to a missing vertex"));
                }
                if tri[0] == tri[1] || tri[1] == tri[2] || tri[2] == tri[0] {
                    return Err(format!("Triangle {i} repeats a vertex"));
                }
            }
        }
        if !exact_domain {
            crate::attributes::validate_materials(self.materials.as_deref(), count)?;
            if let Some(surface) = &self.surface {
                surface.validate(count, &self.triangles)?;
            }
        }
        if self.approximate_bytes() > crate::decode_limits::MAX_SCENE_STORAGE_BYTES {
            return Err("Mesh storage exceeds the scene allocation budget".into());
        }
        Ok(())
    }

    /// Corner-angle-weighted unit face normals. Splitting a flat face preserves
    /// its contribution at existing corners, avoiding shading changes caused
    /// solely by local refinement. Invalid public topology is skipped safely.
    /// Corner angles use the same portable atan2 on native/WASM. Platform
    /// atan2 differences can survive cancellation at a symmetric high-valence
    /// pole and change serialized cache words after otherwise identical edits.
    pub fn recompute_normals(&mut self) {
        self.resident_normal_witness = None;
        if self.positions.len() > MAX_VERTICES || self.triangles.len() > MAX_TRIANGLES {
            return;
        }
        self.normals.resize(self.positions.len(), Vec3::ZERO);
        let mut normals = vec![glam::DVec3::ZERO; self.positions.len()];
        for &[a, b, c] in &self.triangles {
            let (a, b, c) = (a as usize, b as usize, c as usize);
            if a >= self.positions.len() || b >= self.positions.len() || c >= self.positions.len() {
                continue;
            }
            let points = [a, b, c].map(|vertex| self.positions[vertex].as_dvec3());
            let n = (points[1] - points[0])
                .cross(points[2] - points[0])
                .normalize_or_zero();
            if !n.is_finite() || n == glam::DVec3::ZERO {
                continue;
            }
            for (corner, vertex) in [a, b, c].into_iter().enumerate() {
                let u = points[(corner + 1) % 3] - points[corner];
                let v = points[(corner + 2) % 3] - points[corner];
                let angle = libm::atan2(u.cross(v).length(), u.dot(v));
                if angle.is_finite() {
                    normals[vertex] += n * angle;
                }
            }
        }
        for (normal, average) in self.normals.iter_mut().zip(normals) {
            *normal = average.normalize_or_zero().as_vec3();
        }
    }

    /// Rebuild the normals influenced by changed vertices, retaining the same
    /// corner weighting and face summation order as a complete rebuild. A face
    /// changed at one corner changes the normal at all three corners, and each
    /// of those normals must include its unchanged incident faces too.
    pub(crate) fn recompute_normals_near_vertices(&mut self, dirty: &[usize]) {
        if self.positions.len() > MAX_VERTICES || self.triangles.len() > MAX_TRIANGLES {
            return;
        }
        self.ensure_normal_incidence();
        self.recompute_normals_near_vertices_cached(dirty);
    }

    /// Internal dab continuation: the caller has already fingerprinted this
    /// topology and has only changed positions since then. A topology-changing
    /// candidate must use the fingerprinted wrapper above instead.
    pub(crate) fn recompute_normals_near_vertices_cached(&mut self, dirty: &[usize]) {
        if self.positions.len() > MAX_VERTICES || self.triangles.len() > MAX_TRIANGLES {
            return;
        }
        if self.normals.len() != self.positions.len() {
            self.recompute_normals();
            return;
        }
        if dirty.is_empty() {
            return;
        }
        if self.adjacency.is_none() {
            self.ensure_normal_incidence();
        }
        if self.cached_invalid_indices() {
            self.recompute_normals();
            return;
        }
        let changed_faces = self.cached_faces_for_vertices(dirty);
        let mut affected_vertices: Vec<_> = dirty
            .iter()
            .copied()
            .filter(|&vertex| vertex < self.positions.len())
            .collect();
        for face in changed_faces {
            affected_vertices.extend(self.triangles[face].map(|vertex| vertex as usize));
        }
        affected_vertices.sort_unstable();
        affected_vertices.dedup();
        let incident_faces = self.cached_faces_for_vertices(&affected_vertices);
        let mut affected = vec![false; self.positions.len()];
        for &vertex in &affected_vertices {
            affected[vertex] = true;
        }
        let mut averages = vec![glam::DVec3::ZERO; self.positions.len()];
        // Ascending face IDs retain the exact f64 summation order of the full
        // normal rebuild, while the incident list avoids two whole-mesh scans.
        for face in incident_faces {
            let triangle = self.triangles[face];
            let vertices = triangle.map(|v| v as usize);
            let points = vertices.map(|vertex| self.positions[vertex].as_dvec3());
            let normal = (points[1] - points[0])
                .cross(points[2] - points[0])
                .normalize_or_zero();
            if !normal.is_finite() || normal == glam::DVec3::ZERO {
                continue;
            }
            for (corner, vertex) in vertices.into_iter().enumerate() {
                if !affected[vertex] {
                    continue;
                }
                let u = points[(corner + 1) % 3] - points[corner];
                let v = points[(corner + 2) % 3] - points[corner];
                let angle = libm::atan2(u.cross(v).length(), u.dot(v));
                if angle.is_finite() {
                    averages[vertex] += normal * angle;
                }
            }
        }
        for vertex in affected_vertices {
            self.normals[vertex] = averages[vertex].normalize_or_zero().as_vec3();
        }
    }

    /// Four-way midpoint subdivision preserves masks and vertex color by interpolation.
    pub fn subdivide(&mut self) -> Result<(), String> {
        self.validate()?;
        let mut surface = self
            .surface
            .as_ref()
            .map(crate::SurfaceTopology::subdivided_triangles)
            .transpose()?;
        let triangle_count = self
            .triangles
            .len()
            .checked_mul(4)
            .ok_or("Topology limit exceeded")?;
        if triangle_count > MAX_TRIANGLES {
            return Err("Subdivision exceeds the two-million triangle limit".into());
        }
        let mut positions = self.positions.clone();
        let mut masks = self.masks.clone();
        let mut colors = self.colors.clone();
        let mut materials = self.materials.clone();
        let mut edges = HashMap::<(u32, u32), u32>::new();
        let mut triangles = Vec::with_capacity(triangle_count);
        for &[a, b, c] in &self.triangles {
            let mut midpoint = |a: u32, b: u32| -> Result<u32, String> {
                let edge = (a.min(b), a.max(b));
                if let Some(&index) = edges.get(&edge) {
                    return Ok(index);
                }
                if positions.len() >= MAX_VERTICES {
                    return Err("Subdivision exceeds the one-million vertex limit".into());
                }
                let index = positions.len() as u32;
                let (a, b) = (a as usize, b as usize);
                positions.push((positions[a] + positions[b]) * 0.5);
                masks.push((masks[a] + masks[b]) * 0.5);
                colors.push(std::array::from_fn(|channel| {
                    (colors[a][channel] + colors[b][channel]) * 0.5
                }));
                if let Some(materials) = &mut materials {
                    materials.push(std::array::from_fn(|channel| {
                        (materials[a][channel] + materials[b][channel]) * 0.5
                    }));
                }
                edges.insert(edge, index);
                Ok(index)
            };
            let ab = midpoint(a, b)?;
            let bc = midpoint(b, c)?;
            let ca = midpoint(c, a)?;
            triangles.extend_from_slice(&[[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        if let Some(surface) = &mut surface {
            surface.faces = triangles
                .iter()
                .map(|&[a, b, c]| [a, b, c, crate::TRIANGLE_CORNER])
                .collect();
        }
        let mut candidate = Self::new(positions, triangles);
        candidate.masks = masks;
        candidate.mask_convention = self.mask_convention;
        candidate.colors = colors;
        candidate.materials = materials;
        candidate.surface = surface;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// Smooth triangular surfaces using one Loop subdivision step.
    ///
    /// Interior edge points use both incident faces; existing vertices use the
    /// valence-dependent Loop weights. Boundary edges remain midpoints and
    /// boundary vertices use the cubic boundary rule. Masks and RGB follow the
    /// same convex interpolation as positions. This rounds corners and can
    /// shrink a surface; it does not create persistent subdivision levels.
    /// Unsupported topology and mesh-limit failures leave the mesh unchanged.
    pub fn subdivide_smooth(&mut self) -> Result<(), String> {
        self.validate()?;
        let mut surface = self
            .surface
            .as_ref()
            .map(crate::SurfaceTopology::subdivided_triangles)
            .transpose()?;
        let triangle_count = self
            .triangles
            .len()
            .checked_mul(4)
            .ok_or("Topology limit exceeded")?;
        if triangle_count > MAX_TRIANGLES {
            return Err("Subdivision exceeds the two-million triangle limit".into());
        }
        if self.positions.len() == MAX_VERTICES {
            return Err("Subdivision exceeds the one-million vertex limit".into());
        }
        let topology = self.smooth_subdivision_topology()?;
        let vertex_count = self.positions.len() + topology.edges.len();
        let mut positions = Vec::with_capacity(vertex_count);
        let mut masks = Vec::with_capacity(vertex_count);
        let mut colors = Vec::with_capacity(vertex_count);
        let mut materials = self
            .materials
            .as_ref()
            .map(|_| Vec::with_capacity(vertex_count));
        for (vertex, neighbors) in topology.neighbors.iter().enumerate() {
            let boundary = &topology.boundary_neighbors[vertex];
            let (neighbors, neighbor_weight) = if boundary.is_empty() {
                let valence = neighbors.len() as f32;
                let term = 0.375 + 0.25 * (std::f32::consts::TAU / valence).cos();
                (neighbors, (0.625 - term * term) / valence)
            } else {
                (boundary, 0.125)
            };
            let center_weight = 1.0 - neighbors.len() as f32 * neighbor_weight;
            let mut position = self.positions[vertex] * center_weight;
            let mut mask = self.masks[vertex] * center_weight;
            let mut color = self.colors[vertex].map(|value| value * center_weight);
            let mut material = self
                .materials
                .as_ref()
                .map(|m| m[vertex].map(|v| v * center_weight));
            for &neighbor in neighbors {
                let neighbor = neighbor as usize;
                position += self.positions[neighbor] * neighbor_weight;
                mask += self.masks[neighbor] * neighbor_weight;
                if let (Some(material), Some(source)) = (&mut material, &self.materials) {
                    for (channel, value) in material.iter_mut().enumerate() {
                        *value += source[neighbor][channel] * neighbor_weight;
                    }
                }
                for (channel, value) in color.iter_mut().enumerate() {
                    *value += self.colors[neighbor][channel] * neighbor_weight;
                }
            }
            positions.push(position);
            masks.push(mask.clamp(0.0, 1.0));
            colors.push(color.map(|value| value.clamp(0.0, 1.0)));
            if let (Some(output), Some(material)) = (&mut materials, material) {
                output.push(material.map(|v| v.clamp(0.0, 1.0)));
            }
        }
        for edge in &topology.edges {
            let [a, b] = edge.vertices.map(|vertex| vertex as usize);
            let endpoint_weight = if edge.face_count == 1 { 0.5 } else { 0.375 };
            let mut position = (self.positions[a] + self.positions[b]) * endpoint_weight;
            let mut mask = (self.masks[a] + self.masks[b]) * endpoint_weight;
            let mut color = std::array::from_fn::<_, 3, _>(|channel| {
                (self.colors[a][channel] + self.colors[b][channel]) * endpoint_weight
            });
            let mut material = self.materials.as_ref().map(|m| {
                std::array::from_fn::<_, 2, _>(|channel| {
                    (m[a][channel] + m[b][channel]) * endpoint_weight
                })
            });
            if edge.face_count == 2 {
                for opposite in edge.opposites {
                    let opposite = opposite as usize;
                    position += self.positions[opposite] * 0.125;
                    mask += self.masks[opposite] * 0.125;
                    if let (Some(material), Some(source)) = (&mut material, &self.materials) {
                        for (channel, value) in material.iter_mut().enumerate() {
                            *value += source[opposite][channel] * 0.125;
                        }
                    }
                    for (channel, value) in color.iter_mut().enumerate() {
                        *value += self.colors[opposite][channel] * 0.125;
                    }
                }
            }
            positions.push(position);
            masks.push(mask.clamp(0.0, 1.0));
            colors.push(color.map(|value| value.clamp(0.0, 1.0)));
            if let (Some(output), Some(material)) = (&mut materials, material) {
                output.push(material.map(|v| v.clamp(0.0, 1.0)));
            }
        }
        let old_count = self.positions.len();
        let mut triangles = Vec::with_capacity(triangle_count);
        for (&[a, b, c], &edges) in self.triangles.iter().zip(&topology.triangle_edges) {
            let [ab, bc, ca] = edges.map(|edge| (old_count + edge) as u32);
            triangles.extend_from_slice(&[[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        if let Some(surface) = &mut surface {
            surface.faces = triangles
                .iter()
                .map(|&[a, b, c]| [a, b, c, crate::TRIANGLE_CORNER])
                .collect();
        }
        let mut refined = Self {
            positions,
            triangles,
            normals: vec![Vec3::ZERO; vertex_count],
            masks,
            mask_convention: self.mask_convention,
            colors,
            materials,
            surface,
            adjacency: None,
            resident_normal_witness: None,
        };
        refined.recompute_normals();
        refined.validate()?;
        *self = refined;
        Ok(())
    }

    fn smooth_subdivision_topology(&self) -> Result<SmoothSubdivisionTopology, String> {
        let count = self.positions.len();
        let mut edges: Vec<SubdivisionEdge> = Vec::new();
        let mut edge_indices = HashMap::<(u32, u32), usize>::new();
        let mut faces = HashSet::new();
        let mut neighbors = vec![Vec::new(); count];
        let mut incident_faces = vec![Vec::new(); count];
        let mut triangle_edges = Vec::with_capacity(self.triangles.len());
        for (face, &[a, b, c]) in self.triangles.iter().enumerate() {
            let mut sorted = [a, b, c];
            sorted.sort_unstable();
            if !faces.insert(sorted) {
                return Err(format!(
                    "Smooth subdivision refuses duplicate triangle {face}"
                ));
            }
            // Double precision avoids overflow/underflow for valid f32 meshes.
            let [p, q, r] = [a, b, c].map(|vertex| self.positions[vertex as usize].as_dvec3());
            if (q - p).cross(r - p).length_squared() == 0.0 {
                return Err(format!(
                    "Smooth subdivision refuses degenerate triangle {face}"
                ));
            }
            for vertex in [a, b, c] {
                incident_faces[vertex as usize].push(face);
            }
            let mut indices = [0; 3];
            for (slot, (a, b, opposite)) in
                [(a, b, c), (b, c, a), (c, a, b)].into_iter().enumerate()
            {
                let key = (a.min(b), a.max(b));
                let index = if let Some(&index) = edge_indices.get(&key) {
                    let edge = &mut edges[index];
                    if edge.face_count == 2 {
                        return Err(
                            "Smooth subdivision requires manifold edges (at most two faces)".into(),
                        );
                    }
                    if edge.forward == (a < b) {
                        return Err(
                            "Smooth subdivision requires consistent triangle winding".into()
                        );
                    }
                    edge.opposites[1] = opposite;
                    edge.faces[1] = face;
                    edge.face_count = 2;
                    index
                } else {
                    if count + edges.len() >= MAX_VERTICES {
                        return Err("Subdivision exceeds the one-million vertex limit".into());
                    }
                    let index = edges.len();
                    edges.push(SubdivisionEdge {
                        vertices: [key.0, key.1],
                        opposites: [opposite, opposite],
                        faces: [face, face],
                        face_count: 1,
                        forward: a < b,
                    });
                    edge_indices.insert(key, index);
                    neighbors[a as usize].push(b);
                    neighbors[b as usize].push(a);
                    index
                };
                indices[slot] = index;
            }
            triangle_edges.push(indices);
        }
        let mut boundary_neighbors = vec![Vec::new(); count];
        for edge in &edges {
            if edge.face_count == 1 {
                let [a, b] = edge.vertices;
                boundary_neighbors[a as usize].push(b);
                boundary_neighbors[b as usize].push(a);
            }
        }
        let mut visited_faces = vec![usize::MAX; self.triangles.len()];
        let mut pending = Vec::new();
        for (vertex, incident) in incident_faces.iter().enumerate() {
            if incident.is_empty() {
                return Err(format!(
                    "Smooth subdivision refuses isolated vertex {vertex}"
                ));
            }
            let boundary_count = boundary_neighbors[vertex].len();
            if boundary_count != 0 && boundary_count != 2 {
                return Err(format!(
                    "Smooth subdivision requires one boundary fan at vertex {vertex}"
                ));
            }
            if boundary_count == 0 && neighbors[vertex].len() < 3 {
                return Err(format!(
                    "Smooth subdivision requires at least three interior neighbors at vertex {vertex}"
                ));
            }
            // Edge counts alone miss two disjoint surface fans meeting at one
            // vertex. Traverse its incident faces across the two local edges.
            pending.push(incident[0]);
            visited_faces[incident[0]] = vertex;
            let mut reached = 0;
            while let Some(face) = pending.pop() {
                reached += 1;
                for &edge in &triangle_edges[face] {
                    let edge = &edges[edge];
                    if !edge.vertices.contains(&(vertex as u32)) {
                        continue;
                    }
                    for &next in &edge.faces[..edge.face_count] {
                        if visited_faces[next] != vertex {
                            visited_faces[next] = vertex;
                            pending.push(next);
                        }
                    }
                }
            }
            if reached != incident.len() {
                return Err(format!(
                    "Smooth subdivision refuses disconnected surface fans at vertex {vertex}"
                ));
            }
        }
        Ok(SmoothSubdivisionTopology {
            edges,
            neighbors,
            boundary_neighbors,
            triangle_edges,
        })
    }

    pub fn raycast(&self, ray: Ray) -> Option<Hit> {
        self.raycast_indexed(ray).map(|(_, hit)| hit)
    }

    /// Preserve the authoritative render row when equal vertex triples belong
    /// to different original faces. Equal-distance hits keep the first row,
    /// exactly as raycast does; no duplicate-face inference is required.
    pub fn raycast_indexed(&self, ray: Ray) -> Option<(usize, Hit)> {
        if self.positions.len() > MAX_VERTICES
            || self.triangles.len() > MAX_TRIANGLES
            || !ray.origin.is_finite()
            || !ray.direction.is_finite()
        {
            return None;
        }
        let direction = ray.direction.normalize_or_zero();
        if direction == Vec3::ZERO {
            return None;
        }
        let mut closest: Option<(usize, Hit)> = None;
        for (row, &[a, b, c]) in self.triangles.iter().enumerate() {
            let (a, b, c) = (a as usize, b as usize, c as usize);
            let (Some(&p0), Some(&p1), Some(&p2)) = (
                self.positions.get(a),
                self.positions.get(b),
                self.positions.get(c),
            ) else {
                continue;
            };
            let e1 = p1 - p0;
            let e2 = p2 - p0;
            let p = direction.cross(e2);
            let determinant = e1.dot(p);
            let parallel_tolerance = f32::EPSILON * e1.length() * e2.length();
            if !determinant.is_finite() || determinant.abs() <= parallel_tolerance {
                continue;
            }
            let inverse = determinant.recip();
            let t = ray.origin - p0;
            let u = t.dot(p) * inverse;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = t.cross(e1);
            let v = direction.dot(q) * inverse;
            if v < 0.0 || u + v > 1.0 {
                continue;
            }
            let distance = e2.dot(q) * inverse;
            if !distance.is_finite()
                || distance < 0.0
                || closest.is_some_and(|(_, h)| h.distance <= distance)
            {
                continue;
            }
            let mut normal = match (
                self.normals.get(a),
                self.normals.get(b),
                self.normals.get(c),
            ) {
                (Some(na), Some(nb), Some(nc)) => {
                    (*na * (1.0 - u - v) + *nb * u + *nc * v).normalize_or_zero()
                }
                _ => Vec3::ZERO,
            };
            if normal == Vec3::ZERO || !normal.is_finite() {
                normal = e1.cross(e2).normalize_or_zero();
            }
            closest = Some((
                row,
                Hit {
                    position: ray.origin + direction * distance,
                    normal,
                    distance,
                    triangle: [a as u32, b as u32, c as u32],
                    barycentric: [1.0 - u - v, u, v],
                },
            ));
        }
        closest
    }

    pub fn from_obj(source: &str) -> Result<Self, String> {
        if source.len() > MAX_INPUT_BYTES {
            return Err("OBJ exceeds the 128 MiB input limit".into());
        }
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        let mut mrgb_colors = Vec::new();
        let mut has_mrgb = false;
        let mut mat_rows = Vec::new();
        let mut has_mat = false;
        let mut triangles = Vec::new();
        for (line_number, raw_line) in source.lines().enumerate() {
            let failure = |message: &str| format!("OBJ line {}: {message}", line_number + 1);
            let mut comment_tokens = raw_line.split_whitespace();
            if comment_tokens.next() == Some("#MRGB") {
                has_mrgb = true;
                let mut records = 0;
                for token in comment_tokens {
                    if token.len() % 8 != 0 {
                        return Err(failure(
                            "MRGB requires complete eight-digit hexadecimal records",
                        ));
                    }
                    for record in token.as_bytes().chunks_exact(8) {
                        if mrgb_colors.len() >= MAX_VERTICES {
                            return Err(failure("MRGB vertex color limit exceeded"));
                        }
                        let mut bytes = [0_u8; 4];
                        for (channel, pair) in record.chunks_exact(2).enumerate() {
                            bytes[channel] = hex_digit(pair[0])
                                .and_then(|high| hex_digit(pair[1]).map(|low| high * 16 + low))
                                .ok_or_else(|| failure("MRGB contains a non-hexadecimal digit"))?;
                        }
                        // MMRRGGBB: validate the leading byte, but do not infer an
                        // undocumented mapping between ZBrush masks and OBrush masks.
                        mrgb_colors.push([
                            bytes[1] as f32 / 255.0,
                            bytes[2] as f32 / 255.0,
                            bytes[3] as f32 / 255.0,
                        ]);
                        records += 1;
                    }
                }
                if records == 0 {
                    return Err(failure("MRGB row has no color records"));
                }
                continue;
            }
            let mut comment_tokens = raw_line.split_whitespace();
            if comment_tokens.next() == Some("#MAT") {
                has_mat = true;
                let mut records = 0;
                for token in comment_tokens {
                    if token.len() % 6 != 0 {
                        return Err(failure(
                            "MAT requires complete six-digit hexadecimal records",
                        ));
                    }
                    for record in token.as_bytes().chunks_exact(6) {
                        if mat_rows.len() >= MAX_VERTICES {
                            return Err(failure("MAT vertex material limit exceeded"));
                        }
                        let mut bytes = [0_u8; 3];
                        for (channel, pair) in record.chunks_exact(2).enumerate() {
                            bytes[channel] = hex_digit(pair[0])
                                .and_then(|high| hex_digit(pair[1]).map(|low| high * 16 + low))
                                .ok_or_else(|| failure("MAT contains a non-hexadecimal digit"))?;
                        }
                        mat_rows.push(bytes.map(|channel| channel as f32 / 255.0));
                        records += 1;
                    }
                }
                if records == 0 {
                    return Err(failure("MAT row has no material records"));
                }
                continue;
            }
            let line = raw_line.split('#').next().unwrap_or("").trim();
            let mut tokens = line.split_whitespace();
            let Some(kind) = tokens.next() else {
                continue;
            };
            match kind {
                "v" => {
                    if positions.len() >= MAX_VERTICES {
                        return Err(failure("vertex limit exceeded"));
                    }
                    let values = tokens
                        .take(8)
                        .map(str::parse::<f32>)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| failure("invalid vertex number"))?;
                    if !matches!(values.len(), 3 | 4 | 6 | 7) {
                        return Err(failure(
                            "vertex requires x y z, optional w, and optional RGB",
                        ));
                    }
                    let mut p = Vec3::new(values[0], values[1], values[2]);
                    let color_offset = if values.len() == 4 || values.len() == 7 {
                        if !values[3].is_finite() || values[3].abs() < 1e-12 {
                            return Err(failure("invalid homogeneous coordinate"));
                        }
                        p /= values[3];
                        4
                    } else {
                        3
                    };
                    if !p.is_finite() || p.abs().max_element() > MAX_COORDINATE {
                        return Err(failure("non-finite or excessive vertex coordinate"));
                    }
                    let color = if values.len() >= 6 {
                        [
                            values[color_offset],
                            values[color_offset + 1],
                            values[color_offset + 2],
                        ]
                    } else {
                        [0.65, 0.69, 0.74]
                    };
                    if color
                        .iter()
                        .any(|c| !c.is_finite() || !(0.0..=1.0).contains(c))
                    {
                        return Err(failure("RGB channels must be in 0–1"));
                    }
                    positions.push(p);
                    colors.push(color);
                }
                "f" => {
                    let mut polygon = Vec::new();
                    for token in tokens {
                        if polygon.len() >= MAX_FACE_VERTICES {
                            return Err(failure("face exceeds 512 vertices"));
                        }
                        let components: Vec<_> = token.split('/').collect();
                        if components.len() > 3 {
                            return Err(failure("invalid face index"));
                        }
                        let index = components[0]
                            .parse::<i64>()
                            .map_err(|_| failure("invalid face vertex index"))?;
                        for component in &components[1..] {
                            if !component.is_empty()
                                && component.parse::<i64>().map_or(true, |n| n == 0)
                            {
                                return Err(failure("invalid texture or normal index"));
                            }
                        }
                        let index = if index > 0 {
                            index - 1
                        } else if index < 0 {
                            positions.len() as i64 + index
                        } else {
                            -1
                        };
                        if index < 0 || index >= positions.len() as i64 {
                            return Err(failure("face refers to a missing vertex"));
                        }
                        polygon.push(index as u32);
                    }
                    if polygon.len() < 3 {
                        return Err(failure("face needs at least three vertices"));
                    }
                    if polygon.iter().copied().collect::<HashSet<_>>().len() != polygon.len() {
                        return Err(failure("face repeats a vertex"));
                    }
                    if triangles.len() + polygon.len() - 2 > MAX_TRIANGLES {
                        return Err(failure("triangle limit exceeded"));
                    }
                    triangles.extend(
                        triangulate_polygon(&polygon, &positions)
                            .map_err(|error| failure(&error))?,
                    );
                }
                _ => {} // OBJ groups, material, normal, and texture records do not alter surface topology.
            }
        }
        // A complete dedicated MRGB stream takes precedence over inline v RGB.
        // Reject a partial stream rather than silently mixing two color sources.
        if has_mrgb {
            if mrgb_colors.len() != positions.len() {
                return Err(format!(
                    "OBJ MRGB color count {} does not match vertex count {}",
                    mrgb_colors.len(),
                    positions.len()
                ));
            }
            colors = mrgb_colors;
        }
        if has_mat && mat_rows.len() != positions.len() {
            return Err(format!(
                "OBJ MAT material count {} does not match vertex count {}",
                mat_rows.len(),
                positions.len()
            ));
        }
        let mut mesh = Self::new(positions, triangles);
        mesh.colors = colors;
        if has_mat {
            mesh.materials = Some(mat_rows.iter().map(|row| [row[0], row[1]]).collect());
            mesh.masks = mat_rows.iter().map(|row| 1.0 - row[2]).collect();
        }
        mesh.validate()?;
        Ok(mesh)
    }

    /// Checked portable OBJ export retains original faces and corner UVs. Vertex
    /// roughness/metallic requires material files, which this path does not invent.
    pub fn export_obj_checked(&self) -> Result<String, String> {
        self.validate()?;
        if !self.mask_convention.is_default() {
            return Err(
                "OBJ writer cannot preserve source mask data; use an explicit export loss policy"
                    .into(),
            );
        }
        if self.materials.is_some() {
            return Err("OBJ roughness/metallic export needs an explicit material-file workflow; export a version6 project instead".into());
        }
        struct BoundedObjOutput(String);
        impl std::fmt::Write for BoundedObjOutput {
            fn write_str(&mut self, text: &str) -> std::fmt::Result {
                let required = self
                    .0
                    .len()
                    .checked_add(text.len())
                    .ok_or(std::fmt::Error)?;
                if required > MAX_INPUT_BYTES {
                    return Err(std::fmt::Error);
                }
                if required > self.0.capacity() {
                    let desired = self
                        .0
                        .capacity()
                        .saturating_mul(2)
                        .max(required)
                        .min(MAX_INPUT_BYTES);
                    self.0
                        .try_reserve_exact(desired - self.0.len())
                        .map_err(|_| std::fmt::Error)?;
                }
                self.0.push_str(text);
                Ok(())
            }
        }
        let mut output = BoundedObjOutput(String::new());
        self.write_obj(&mut output)
            .map_err(|_| "Cannot allocate bounded OBJ output within 128 MiB")?;
        Ok(output.0)
    }

    /// Legacy infallible geometry/RGB exporter. Rich hosts must call
    /// `export_obj_checked` to validate and reject unrepresentable materials.
    pub fn to_obj(&self) -> String {
        let mut output = String::new();
        self.write_obj(&mut output).expect("writing to string");
        output
    }

    fn write_obj(&self, output: &mut impl std::fmt::Write) -> std::fmt::Result {
        output.write_str("# OBrush mesh — vertex RGB colors included\n")?;
        for (i, p) in self.positions.iter().enumerate() {
            let color = self.colors.get(i).copied().unwrap_or([0.65, 0.69, 0.74]);
            writeln!(
                output,
                "v {} {} {} {} {} {}",
                p.x, p.y, p.z, color[0], color[1], color[2]
            )?;
        }
        for n in &self.normals {
            writeln!(output, "vn {} {} {}", n.x, n.y, n.z)?;
        }
        if let Some(surface) = &self.surface {
            if let Some(uv) = &surface.uv {
                for &[u, v] in &uv.coordinates {
                    writeln!(output, "vt {u} {v}")?;
                }
            }
            for (face_index, face) in surface.faces.iter().enumerate() {
                output.write_char('f')?;
                let arity = if face[3] == crate::TRIANGLE_CORNER {
                    3
                } else {
                    4
                };
                for (corner, &vertex) in face[..arity].iter().enumerate() {
                    let index = u64::from(vertex) + 1;
                    if let Some(uv) = &surface.uv {
                        let texture = u64::from(uv.face_indices[face_index][corner]) + 1;
                        write!(output, " {index}/{texture}/{index}")?;
                    } else {
                        write!(output, " {index}//{index}")?;
                    }
                }
                output.write_char('\n')?;
            }
        } else {
            for &[a, b, c] in &self.triangles {
                let (a, b, c) = (u64::from(a) + 1, u64::from(b) + 1, u64::from(c) + 1);
                writeln!(output, "f {a}//{a} {b}//{b} {c}//{c}")?;
            }
        }
        Ok(())
    }

    fn adjacency_signature(&self) -> u64 {
        // Preserve the existing public-topology fingerprint policy.
        let mut signature = self.positions.len() as u64;
        for tri in &self.triangles {
            for index in tri {
                signature = (signature ^ *index as u64).wrapping_mul(0x100000001b3);
            }
        }
        signature
    }

    fn build_normal_incidence(&self) -> (Vec<usize>, Vec<u32>, bool) {
        let mut vertex_face_counts = vec![0_usize; self.positions.len()];
        let mut invalid_indices = false;
        for &[a, b, c] in &self.triangles {
            let vertices = [a as usize, b as usize, c as usize];
            if vertices.iter().any(|&v| v >= self.positions.len()) {
                invalid_indices = true;
                continue;
            }
            for vertex in vertices {
                vertex_face_counts[vertex] += 1;
            }
        }
        let vertex_face_offsets = offsets_from_counts(vertex_face_counts);
        let mut vertex_faces = vec![0; *vertex_face_offsets.last().unwrap()];
        let mut cursors = vertex_face_offsets[..self.positions.len()].to_vec();
        for (face, &triangle) in self.triangles.iter().enumerate() {
            if triangle.iter().any(|&v| v as usize >= self.positions.len()) {
                continue;
            }
            for vertex in triangle {
                let vertex = vertex as usize;
                vertex_faces[cursors[vertex]] = face as u32;
                cursors[vertex] += 1;
            }
        }
        (vertex_face_offsets, vertex_faces, invalid_indices)
    }

    fn ensure_normal_incidence(&mut self) {
        let signature = self.adjacency_signature();
        if self
            .adjacency
            .as_ref()
            .is_none_or(|a| a.signature != signature)
        {
            // A stale cache is derived data. Release it before rebuilding.
            self.adjacency = None;
            let (vertex_face_offsets, vertex_faces, invalid_indices) =
                self.build_normal_incidence();
            self.adjacency = Some(Arc::new(Adjacency {
                signature,
                vertex_face_offsets,
                vertex_faces,
                invalid_indices,
                normal_only: true,
                neighbor_offsets: Vec::new(),
                neighbor_indices: Vec::new(),
                face_pairs: Vec::new(),
                face_pair_offsets: Vec::new(),
                face_pair_indices: Vec::new(),
                nonmanifold_faces: Vec::new(),
            }));
        }
    }

    pub(crate) fn ensure_adjacency(&mut self) {
        let signature = self.adjacency_signature();
        if self
            .adjacency
            .as_ref()
            .is_none_or(|a| a.signature != signature || a.normal_only)
        {
            let previous = self.adjacency.take();
            let (vertex_face_offsets, vertex_faces, invalid_indices) =
                match previous.filter(|a| a.signature == signature && a.normal_only) {
                    Some(a) => {
                        let a = Arc::unwrap_or_clone(a);
                        (a.vertex_face_offsets, a.vertex_faces, a.invalid_indices)
                    }
                    None => self.build_normal_incidence(),
                };
            let mut neighbor_counts = vec![0_usize; self.positions.len()];
            let mut face_pairs = Vec::new();
            let mut nonmanifold_faces = Vec::new();
            // Each edge belongs to its smaller endpoint. A sculpt mesh has
            // small vertex valence, so only tiny incident lists are sorted.
            // Ascending owner/neighbor/face order exactly matches the old global
            // packed-edge sort, including degenerate/non-manifold incidences.
            let mut unique_edges = Vec::new();
            let mut local_edges = Vec::new();
            for vertex in 0..self.positions.len() {
                local_edges.clear();
                let mut previous_face = None;
                for &face in
                    &vertex_faces[vertex_face_offsets[vertex]..vertex_face_offsets[vertex + 1]]
                {
                    if previous_face == Some(face) {
                        continue;
                    }
                    previous_face = Some(face);
                    let [a, b, c] = self.triangles[face as usize];
                    if a != b && b != c && c != a {
                        let others = if a as usize == vertex {
                            [b, c]
                        } else if b as usize == vertex {
                            [a, c]
                        } else {
                            [a, b]
                        };
                        for neighbor in others {
                            if neighbor as usize > vertex {
                                local_edges.push((neighbor, face));
                            }
                        }
                    } else {
                        // Invalid repeated corners still retain the old exact
                        // incidence counts for conservative topology guards.
                        for (a, b) in [(a, b), (b, c), (c, a)] {
                            if a.min(b) as usize == vertex {
                                local_edges.push((a.max(b), face));
                            }
                        }
                    }
                }
                local_edges.sort_unstable();
                let mut start = 0;
                while start < local_edges.len() {
                    let mut end = start + 1;
                    while end < local_edges.len() && local_edges[end].0 == local_edges[start].0 {
                        end += 1;
                    }
                    let neighbor = local_edges[start].0 as usize;
                    let first = local_edges[start].1;
                    unique_edges.push((vertex, neighbor));
                    neighbor_counts[vertex] += 1;
                    if vertex != neighbor {
                        neighbor_counts[neighbor] += 1;
                    }
                    for &(_, face) in &local_edges[start + 1..end] {
                        face_pairs.push([first, face]);
                    }
                    if end - start > 2 {
                        nonmanifold_faces
                            .extend(local_edges[start..end].iter().map(|&(_, face)| face));
                    }
                    start = end;
                }
            }
            let neighbor_offsets = offsets_from_counts(neighbor_counts);
            let mut neighbor_indices = vec![0; *neighbor_offsets.last().unwrap()];
            let mut cursors = neighbor_offsets[..self.positions.len()].to_vec();
            for (a, b) in unique_edges {
                neighbor_indices[cursors[a]] = b;
                cursors[a] += 1;
                if a != b {
                    neighbor_indices[cursors[b]] = a;
                    cursors[b] += 1;
                }
            }
            nonmanifold_faces.sort_unstable();
            nonmanifold_faces.dedup();
            let mut face_pair_counts = vec![0; self.triangles.len()];
            for &[a, b] in &face_pairs {
                face_pair_counts[a as usize] += 1;
                face_pair_counts[b as usize] += 1;
            }
            let face_pair_offsets = offsets_from_counts(face_pair_counts);
            let mut face_pair_indices = vec![0; *face_pair_offsets.last().unwrap()];
            let mut cursors = face_pair_offsets[..self.triangles.len()].to_vec();
            for (index, pair) in face_pairs.iter().enumerate() {
                for &face in pair {
                    let face = face as usize;
                    face_pair_indices[cursors[face]] = index;
                    cursors[face] += 1;
                }
            }
            self.adjacency = Some(Arc::new(Adjacency {
                signature,
                neighbor_offsets,
                neighbor_indices,
                vertex_face_offsets,
                vertex_faces,
                face_pairs,
                face_pair_offsets,
                face_pair_indices,
                nonmanifold_faces,
                invalid_indices,
                normal_only: false,
            }));
        }
    }

    /// Call `ensure_adjacency` first to validate the public topology fingerprint.
    pub(crate) fn cached_face_pairs(&self) -> &[[u32; 2]] {
        let adjacency = self.adjacency.as_ref().expect("adjacency built");
        assert!(!adjacency.normal_only, "full adjacency required");
        &adjacency.face_pairs
    }

    /// Call `ensure_adjacency` first to check the public topology fingerprint.
    pub(crate) fn cached_neighbors(&self, vertex: usize) -> &[usize] {
        let adjacency = self.adjacency.as_ref().expect("adjacency built");
        assert!(!adjacency.normal_only, "full adjacency required");
        &adjacency.neighbor_indices
            [adjacency.neighbor_offsets[vertex]..adjacency.neighbor_offsets[vertex + 1]]
    }

    pub(crate) fn cached_nonmanifold_faces(&self) -> &[u32] {
        let adjacency = self.adjacency.as_ref().expect("adjacency built");
        assert!(!adjacency.normal_only, "full adjacency required");
        &adjacency.nonmanifold_faces
    }

    /// Call `ensure_adjacency` first; public out-of-range indices must never be
    /// hidden by a local incidence query used for geometry safety checks.
    pub(crate) fn cached_invalid_indices(&self) -> bool {
        self.adjacency
            .as_ref()
            .expect("adjacency built")
            .invalid_indices
    }

    /// Incident faces in ascending global face order. Missing vertex IDs are
    /// ignored; invalid public topology is reported separately above.
    pub(crate) fn cached_faces_for_vertices(&self, vertices: &[usize]) -> Vec<usize> {
        let adjacency = self.adjacency.as_ref().expect("adjacency built");
        let mut faces = Vec::new();
        for &vertex in vertices {
            if vertex < adjacency.vertex_face_offsets.len().saturating_sub(1) {
                faces.extend(
                    adjacency.vertex_faces[adjacency.vertex_face_offsets[vertex]
                        ..adjacency.vertex_face_offsets[vertex + 1]]
                        .iter()
                        .map(|&face| face as usize),
                );
            }
        }
        faces.sort_unstable();
        faces.dedup();
        faces
    }

    /// Shared-edge pair IDs in their original deterministic order, including
    /// pairs to unchanged neighboring faces at the edge of the edited patch.
    pub(crate) fn cached_pairs_for_faces(&self, faces: &[usize]) -> Vec<usize> {
        let adjacency = self.adjacency.as_ref().expect("adjacency built");
        assert!(!adjacency.normal_only, "full adjacency required");
        let mut pairs = Vec::new();
        for &face in faces {
            if face < adjacency.face_pair_offsets.len().saturating_sub(1) {
                pairs.extend_from_slice(
                    &adjacency.face_pair_indices
                        [adjacency.face_pair_offsets[face]..adjacency.face_pair_offsets[face + 1]],
                );
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }

    /// Copy the surface without derived topology caches, for history/rendering.
    pub fn snapshot(&self) -> Self {
        Self {
            positions: self.positions.clone(),
            normals: self.normals.clone(),
            triangles: self.triangles.clone(),
            masks: self.masks.clone(),
            mask_convention: self.mask_convention,
            colors: self.colors.clone(),
            materials: self.materials.clone(),
            surface: self.surface.clone(),
            adjacency: None,
            resident_normal_witness: None,
        }
    }

    /// Fallible snapshot for bounded callers, including the display-free Worker.
    pub fn try_snapshot(&self) -> Result<Self, String> {
        fn copy<T: Copy>(values: &[T]) -> Result<Vec<T>, String> {
            let mut out = Vec::new();
            out.try_reserve_exact(values.len())
                .map_err(|_| "Cannot reserve Source Paint Lock snapshot")?;
            out.extend_from_slice(values);
            Ok(out)
        }
        let surface = self
            .surface
            .as_ref()
            .map(|surface| {
                Ok::<_, String>(crate::SurfaceTopology {
                    faces: copy(&surface.faces)?,
                    uv: surface
                        .uv
                        .as_ref()
                        .map(|uv| {
                            Ok::<_, String>(crate::CornerUv {
                                coordinates: copy(&uv.coordinates)?,
                                face_indices: copy(&uv.face_indices)?,
                            })
                        })
                        .transpose()?,
                })
            })
            .transpose()?;
        Ok(Self {
            positions: copy(&self.positions)?,
            normals: copy(&self.normals)?,
            triangles: copy(&self.triangles)?,
            masks: copy(&self.masks)?,
            mask_convention: self.mask_convention,
            colors: copy(&self.colors)?,
            materials: self.materials.as_ref().map(|m| copy(m)).transpose()?,
            surface,
            adjacency: None,
            resident_normal_witness: None,
        })
    }

    /// Resident position commits retain immutable, signature-checked topology
    /// across same-topology strokes. The ordinary history snapshot still drops
    /// derived caches, and a public topology edit invalidates this on use.
    pub(crate) fn try_resident_candidate(&self) -> Result<Self, String> {
        let mut candidate = self.try_snapshot()?;
        candidate.adjacency = self.adjacency.clone();
        candidate.resident_normal_witness = self.resident_normal_witness.clone();
        Ok(candidate)
    }

    /// An exact, bounded witness that a previous resident commit left the
    /// complete normal field canonical. Public mesh arrays can be modified
    /// directly, so reuse requires comparing every source word, including
    /// signed zero, before a later sparse normal refresh.
    pub(crate) fn resident_normals_match_witness(&self) -> bool {
        let Some(witness) = &self.resident_normal_witness else {
            return false;
        };
        self.positions.len() == witness.positions.len()
            && self.normals.len() == witness.normals.len()
            && self.triangles == witness.triangles
            && bytemuck::cast_slice::<_, u8>(&self.positions)
                == bytemuck::cast_slice::<_, u8>(&witness.positions)
            && bytemuck::cast_slice::<_, u8>(&self.normals)
                == bytemuck::cast_slice::<_, u8>(&witness.normals)
    }

    pub(crate) fn refresh_resident_normal_witness(&mut self) -> bool {
        // At the public million-vertex cap this retains at most 48 MB of
        // derived data. Failure only disables the optimization; the committed
        // mesh and history remain valid.
        const MAX_WITNESS_BYTES: usize = 48 * 1024 * 1024;
        let bytes = self
            .positions
            .len()
            .saturating_mul(std::mem::size_of::<Vec3>())
            .saturating_add(
                self.normals
                    .len()
                    .saturating_mul(std::mem::size_of::<Vec3>()),
            )
            .saturating_add(
                self.triangles
                    .len()
                    .saturating_mul(std::mem::size_of::<[u32; 3]>()),
            );
        if bytes > MAX_WITNESS_BYTES {
            self.resident_normal_witness = None;
            return false;
        }
        fn copy<T: Copy>(source: &[T]) -> Option<Vec<T>> {
            let mut values = Vec::new();
            values.try_reserve_exact(source.len()).ok()?;
            values.extend_from_slice(source);
            Some(values)
        }
        self.resident_normal_witness = (|| {
            Some(Arc::new(ResidentNormalWitness {
                positions: copy(&self.positions)?,
                triangles: copy(&self.triangles)?,
                normals: copy(&self.normals)?,
            }))
        })();
        self.resident_normal_witness.is_some()
    }

    pub(crate) fn clear_cache(&mut self) {
        self.adjacency = None;
        self.resident_normal_witness = None;
    }

    pub(crate) fn same_surface(&self, other: &Self) -> bool {
        self.positions == other.positions
            && self.triangles == other.triangles
            && self.masks == other.masks
            && (self.mask_convention.is_default()
                || self
                    .masks
                    .iter()
                    .zip(&other.masks)
                    .all(|(a, b)| a.to_bits() == b.to_bits()))
            && self.colors == other.colors
            && self.attributes_equal_bits(other)
    }

    pub(crate) fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.positions.capacity() * std::mem::size_of::<Vec3>()
            + self.normals.capacity() * std::mem::size_of::<Vec3>()
            + self.triangles.capacity() * std::mem::size_of::<[u32; 3]>()
            + self.masks.capacity() * std::mem::size_of::<f32>()
            + self.colors.capacity() * std::mem::size_of::<[f32; 3]>()
            + self.attribute_bytes()
    }
}

fn offsets_from_counts(counts: Vec<usize>) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(counts.len() + 1);
    offsets.push(0);
    for count in counts {
        offsets.push(offsets.last().unwrap() + count);
    }
    offsets
}

#[cfg(test)]
mod incidence_regressions {
    use super::*;

    #[test]
    fn incident_edge_order_matches_global_reference_on_mixed_public_topology() {
        let mut mesh = Mesh::sphere(3);
        mesh.triangles.extend([
            [0, 1, 2],
            [0, 1, 3],
            [0, 1, 4],
            [2, 2, 3],
            [5, 5, 5],
            [0, 1, u32::MAX],
        ]);
        mesh.ensure_adjacency();
        let mut edges = Vec::new();
        for (face, &[a, b, c]) in mesh.triangles.iter().enumerate() {
            if [a, b, c]
                .iter()
                .any(|&v| v as usize >= mesh.positions.len())
            {
                continue;
            }
            for (a, b) in [(a, b), (b, c), (c, a)] {
                edges.push((
                    ((u64::from(a.min(b))) << 32) | u64::from(a.max(b)),
                    face as u32,
                ));
            }
        }
        edges.sort_unstable();
        let mut neighbors = vec![Vec::new(); mesh.positions.len()];
        let mut pairs = Vec::new();
        let mut nonmanifold = Vec::new();
        let mut start = 0;
        while start < edges.len() {
            let mut end = start + 1;
            while end < edges.len() && edges[end].0 == edges[start].0 {
                end += 1;
            }
            let key = edges[start].0;
            let a = (key >> 32) as usize;
            let b = key as u32 as usize;
            neighbors[a].push(b);
            if a != b {
                neighbors[b].push(a);
            }
            for &(_, face) in &edges[start + 1..end] {
                pairs.push([edges[start].1, face]);
            }
            if end - start > 2 {
                nonmanifold.extend(edges[start..end].iter().map(|&(_, face)| face));
            }
            start = end;
        }
        nonmanifold.sort_unstable();
        nonmanifold.dedup();
        assert_eq!(mesh.cached_face_pairs(), pairs);
        assert_eq!(mesh.cached_nonmanifold_faces(), nonmanifold);
        assert!(mesh.cached_invalid_indices());
        for (vertex, expected) in neighbors.iter().enumerate() {
            assert_eq!(mesh.cached_neighbors(vertex), expected);
        }
    }

    #[test]
    fn incidence_gathers_all_neighbor_guards_and_detects_public_topology_edits() {
        let mut mesh = Mesh::sphere(3);
        for edited in [false, true] {
            if edited {
                mesh.triangles[3] = [0, 7, 9];
            }
            mesh.ensure_adjacency();
            let dirty = [0, 7, 9, usize::MAX];
            let expected_faces: Vec<_> = mesh
                .triangles
                .iter()
                .enumerate()
                .filter_map(|(face, triangle)| {
                    triangle
                        .iter()
                        .any(|&vertex| dirty.contains(&(vertex as usize)))
                        .then_some(face)
                })
                .collect();
            let faces = mesh.cached_faces_for_vertices(&dirty);
            assert_eq!(faces, expected_faces);
            let pairs: Vec<_> = mesh
                .cached_face_pairs()
                .iter()
                .enumerate()
                .filter_map(|(index, pair)| {
                    pair.iter()
                        .any(|&face| faces.contains(&(face as usize)))
                        .then_some(index)
                })
                .collect();
            assert_eq!(mesh.cached_pairs_for_faces(&faces), pairs);
            assert!(!mesh.cached_invalid_indices());
        }
        mesh.triangles[5][0] = mesh.positions.len() as u32;
        mesh.ensure_adjacency();
        assert!(mesh.cached_invalid_indices());
        let snapshot = mesh.snapshot();
        assert!(snapshot.adjacency.is_none());
    }

    #[test]
    fn local_normals_match_full_rebuild_after_many_disconnected_and_isolated_edits() {
        let mut mesh = Mesh::sphere(4);
        mesh.positions.push(Vec3::new(3.0, 0.0, 0.0));
        mesh.normals.push(Vec3::Z);
        mesh.masks.push(0.0);
        mesh.colors.push([0.3; 3]);
        mesh.ensure_adjacency();
        let dirty = [0, 19, 71, 607, mesh.positions.len() - 1];
        for &vertex in &dirty {
            mesh.positions[vertex] += Vec3::new(0.006, -0.008, 0.01);
        }
        let mut reference = mesh.snapshot();
        reference.recompute_normals();
        mesh.recompute_normals_near_vertices_cached(&dirty);
        assert_eq!(mesh.normals, reference.normals);
        assert_eq!(*mesh.normals.last().unwrap(), Vec3::ZERO);
    }

    #[test]
    fn fingerprinted_normals_refresh_incidence_after_same_size_public_face_edit() {
        let mut mesh = Mesh::sphere(3);
        mesh.ensure_adjacency();
        let mut dirty = mesh.triangles[0].map(|vertex| vertex as usize).to_vec();
        mesh.triangles[0] = [0, 7, 9];
        dirty.extend([0, 7, 9]);
        let mut reference = mesh.snapshot();
        reference.recompute_normals();
        mesh.recompute_normals_near_vertices(&dirty);
        assert_eq!(mesh.normals, reference.normals);
    }
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn triangulate_polygon(
    polygon: &[u32],
    positions: &[Vec3],
) -> Result<Vec<[u32; 3]>, String> {
    if polygon.len() == 3 {
        return Ok(vec![[polygon[0], polygon[1], polygon[2]]]);
    }
    let anchor = positions[polygon[0] as usize];
    let scale = polygon
        .iter()
        .map(|&index| (positions[index as usize] - anchor).abs().max_element())
        .fold(0.0_f32, f32::max);
    if !scale.is_finite() || scale <= 0.0 {
        return Err("face is degenerate".into());
    }
    let local: Vec<Vec3> = polygon
        .iter()
        .map(|&index| (positions[index as usize] - anchor) / scale)
        .collect();
    let mut normal = Vec3::ZERO;
    for i in 0..polygon.len() {
        normal += local[i].cross(local[(i + 1) % polygon.len()]);
    }
    if !normal.is_finite() || normal.length_squared() < 1e-20 {
        return Err("face is degenerate or self-intersecting".into());
    }
    let axis = if normal.x.abs() > normal.y.abs() && normal.x.abs() > normal.z.abs() {
        0
    } else if normal.y.abs() > normal.z.abs() {
        1
    } else {
        2
    };
    let points: Vec<Vec2> = local
        .iter()
        .map(|&p| match axis {
            0 => Vec2::new(p.y, p.z),
            1 => Vec2::new(p.z, p.x),
            _ => Vec2::new(p.x, p.y),
        })
        .collect();
    for a in 0..points.len() {
        let b = (a + 1) % points.len();
        for c in a + 1..points.len() {
            let d = (c + 1) % points.len();
            if a == c || a == d || b == c || b == d {
                continue;
            }
            if segments_intersect(points[a], points[b], points[c], points[d]) {
                return Err("face edges intersect".into());
            }
        }
    }
    let area: f32 = (0..points.len())
        .map(|i| points[i].perp_dot(points[(i + 1) % points.len()]))
        .sum();
    let winding = area.signum();
    let mut remaining: Vec<usize> = (0..polygon.len()).collect();
    let mut triangles = Vec::with_capacity(polygon.len() - 2);
    while remaining.len() > 3 {
        let mut ear = None;
        for i in 0..remaining.len() {
            let a = remaining[(i + remaining.len() - 1) % remaining.len()];
            let b = remaining[i];
            let c = remaining[(i + 1) % remaining.len()];
            if (points[b] - points[a]).perp_dot(points[c] - points[a]) * winding <= 1e-12 {
                continue;
            }
            let contains = remaining.iter().any(|&p| {
                if p == a || p == b || p == c {
                    return false;
                }
                (points[b] - points[a]).perp_dot(points[p] - points[a]) * winding >= -1e-12
                    && (points[c] - points[b]).perp_dot(points[p] - points[b]) * winding >= -1e-12
                    && (points[a] - points[c]).perp_dot(points[p] - points[c]) * winding >= -1e-12
            });
            if !contains {
                ear = Some((i, [polygon[a], polygon[b], polygon[c]]));
                break;
            }
        }
        let Some((index, triangle)) = ear else {
            return Err(
                "face could not be triangulated; check for intersecting or duplicate edges".into(),
            );
        };
        triangles.push(triangle);
        remaining.remove(index);
    }
    triangles.push([
        polygon[remaining[0]],
        polygon[remaining[1]],
        polygon[remaining[2]],
    ]);
    Ok(triangles)
}

fn segments_intersect(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let epsilon = 1e-10;
    let ab_c = (b - a).perp_dot(c - a);
    let ab_d = (b - a).perp_dot(d - a);
    let cd_a = (d - c).perp_dot(a - c);
    let cd_b = (d - c).perp_dot(b - c);
    let opposite = |x: f32, y: f32| (x > epsilon && y < -epsilon) || (x < -epsilon && y > epsilon);
    if opposite(ab_c, ab_d) && opposite(cd_a, cd_b) {
        return true;
    }
    let on_segment = |p: Vec2, x: Vec2, y: Vec2| {
        p.cmpge(x.min(y) - Vec2::splat(epsilon)).all()
            && p.cmple(x.max(y) + Vec2::splat(epsilon)).all()
    };
    (ab_c.abs() <= epsilon && on_segment(c, a, b))
        || (ab_d.abs() <= epsilon && on_segment(d, a, b))
        || (cd_a.abs() <= epsilon && on_segment(a, c, d))
        || (cd_b.abs() <= epsilon && on_segment(b, c, d))
}

#[cfg(test)]
fn deserialize_vertices<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    bounded_vec::<D, T, MAX_VERTICES>(deserializer, None)
}
fn deserialize_positions<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Vec3>, D::Error> {
    bounded_vec::<D, Vec3, MAX_VERTICES>(d, Some(crate::decode_limits::Channel::Positions))
}
fn deserialize_normals<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Vec3>, D::Error> {
    bounded_vec::<D, Vec3, MAX_VERTICES>(d, Some(crate::decode_limits::Channel::Normals))
}
fn deserialize_masks<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<f32>, D::Error> {
    bounded_vec::<D, f32, MAX_VERTICES>(d, Some(crate::decode_limits::Channel::Masks))
}
fn deserialize_colors<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<[f32; 3]>, D::Error> {
    bounded_vec::<D, [f32; 3], MAX_VERTICES>(d, Some(crate::decode_limits::Channel::Colors))
}
fn deserialize_triangles<'de, D>(deserializer: D) -> Result<Vec<[u32; 3]>, D::Error>
where
    D: Deserializer<'de>,
{
    bounded_vec::<D, [u32; 3], MAX_TRIANGLES>(
        deserializer,
        Some(crate::decode_limits::Channel::Triangles),
    )
}
pub(crate) fn bounded_vec<'de, D, T, const LIMIT: usize>(
    deserializer: D,
    channel: Option<crate::decode_limits::Channel>,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct BoundedVisitor<T, const LIMIT: usize> {
        marker: std::marker::PhantomData<T>,
        channel: Option<crate::decode_limits::Channel>,
    }
    impl<'de, T: Deserialize<'de>, const LIMIT: usize> Visitor<'de> for BoundedVisitor<T, LIMIT> {
        type Value = Vec<T>;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "an array with at most {LIMIT} elements")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Vec<T>, A::Error> {
            let maximum = self
                .channel
                .map_or(LIMIT, crate::decode_limits::remaining)
                .min(LIMIT);
            if sequence.size_hint().is_some_and(|size| size > maximum) {
                return Err(A::Error::custom("mesh allocation limit exceeded"));
            }
            let mut items = Vec::new();
            while let Some(item) = sequence.next_element()? {
                if items.len() >= LIMIT {
                    return Err(A::Error::custom("mesh allocation limit exceeded"));
                }
                if let Some(channel) = self.channel {
                    crate::decode_limits::element(channel).map_err(A::Error::custom)?;
                }
                if items.len() == items.capacity() {
                    let remaining = self.channel.map_or(LIMIT, crate::decode_limits::remaining);
                    let desired = items
                        .capacity()
                        .saturating_mul(2)
                        .max(8)
                        .min(LIMIT)
                        .min(items.len() + 1 + remaining);
                    let extra = desired - items.len();
                    if let Some(channel) = self.channel {
                        crate::decode_limits::capacity(channel, extra * std::mem::size_of::<T>())
                            .map_err(A::Error::custom)?;
                    }
                    items
                        .try_reserve_exact(extra)
                        .map_err(|_| A::Error::custom("Cannot allocate bounded mesh field"))?;
                }
                items.push(item);
            }
            Ok(items)
        }
    }
    deserializer.deserialize_seq(BoundedVisitor::<T, LIMIT> {
        marker: std::marker::PhantomData,
        channel,
    })
}

#[cfg(test)]
mod allocation_tests {
    use super::*;
    #[test]
    fn bounded_deserializer_refuses_excessive_size_hints_before_allocation() {
        let sequence = serde::de::value::SeqDeserializer::<_, serde::de::value::Error>::new(
            0..MAX_VERTICES + 1,
        );
        assert!(deserialize_vertices::<_, usize>(sequence).is_err());
    }
}

#[cfg(test)]
impl Mesh {
    fn ensure_adjacency_c52_reference(&mut self) {
        // Public topology can be edited by a caller; fingerprint before reusing the cache.
        let mut signature = self.positions.len() as u64;
        for tri in &self.triangles {
            for index in tri {
                signature = (signature ^ *index as u64).wrapping_mul(0x100000001b3);
            }
        }
        if self
            .adjacency
            .as_ref()
            .is_none_or(|a| a.signature != signature)
        {
            let mut neighbor_counts = vec![0_usize; self.positions.len()];
            let mut vertex_face_counts = vec![0_usize; self.positions.len()];
            let mut face_pairs = Vec::new();
            let mut nonmanifold_faces = Vec::new();
            let mut invalid_indices = false;
            for &[a, b, c] in &self.triangles {
                let vertices = [a as usize, b as usize, c as usize];
                if vertices.iter().any(|&v| v >= self.positions.len()) {
                    invalid_indices = true;
                    continue;
                }
                for vertex in vertices {
                    vertex_face_counts[vertex] += 1;
                }
            }
            let vertex_face_offsets = offsets_from_counts(vertex_face_counts);
            let mut vertex_faces = vec![0; *vertex_face_offsets.last().unwrap()];
            let mut cursors = vertex_face_offsets[..self.positions.len()].to_vec();
            for (face, &triangle) in self.triangles.iter().enumerate() {
                if triangle.iter().any(|&v| v as usize >= self.positions.len()) {
                    continue;
                }
                for vertex in triangle {
                    let vertex = vertex as usize;
                    vertex_faces[cursors[vertex]] = face as u32;
                    cursors[vertex] += 1;
                }
            }
            // Each edge belongs to its smaller endpoint. A sculpt mesh has
            // small vertex valence, so only tiny incident lists are sorted.
            // Ascending owner/neighbor/face order exactly matches the old global
            // packed-edge sort, including degenerate/non-manifold incidences.
            let mut unique_edges = Vec::new();
            let mut local_edges = Vec::new();
            for vertex in 0..self.positions.len() {
                local_edges.clear();
                let mut previous_face = None;
                for &face in
                    &vertex_faces[vertex_face_offsets[vertex]..vertex_face_offsets[vertex + 1]]
                {
                    if previous_face == Some(face) {
                        continue;
                    }
                    previous_face = Some(face);
                    let [a, b, c] = self.triangles[face as usize];
                    if a != b && b != c && c != a {
                        let others = if a as usize == vertex {
                            [b, c]
                        } else if b as usize == vertex {
                            [a, c]
                        } else {
                            [a, b]
                        };
                        for neighbor in others {
                            if neighbor as usize > vertex {
                                local_edges.push((neighbor, face));
                            }
                        }
                    } else {
                        // Invalid repeated corners still retain the old exact
                        // incidence counts for conservative topology guards.
                        for (a, b) in [(a, b), (b, c), (c, a)] {
                            if a.min(b) as usize == vertex {
                                local_edges.push((a.max(b), face));
                            }
                        }
                    }
                }
                local_edges.sort_unstable();
                let mut start = 0;
                while start < local_edges.len() {
                    let mut end = start + 1;
                    while end < local_edges.len() && local_edges[end].0 == local_edges[start].0 {
                        end += 1;
                    }
                    let neighbor = local_edges[start].0 as usize;
                    let first = local_edges[start].1;
                    unique_edges.push((vertex, neighbor));
                    neighbor_counts[vertex] += 1;
                    if vertex != neighbor {
                        neighbor_counts[neighbor] += 1;
                    }
                    for &(_, face) in &local_edges[start + 1..end] {
                        face_pairs.push([first, face]);
                    }
                    if end - start > 2 {
                        nonmanifold_faces
                            .extend(local_edges[start..end].iter().map(|&(_, face)| face));
                    }
                    start = end;
                }
            }
            let neighbor_offsets = offsets_from_counts(neighbor_counts);
            let mut neighbor_indices = vec![0; *neighbor_offsets.last().unwrap()];
            let mut cursors = neighbor_offsets[..self.positions.len()].to_vec();
            for (a, b) in unique_edges {
                neighbor_indices[cursors[a]] = b;
                cursors[a] += 1;
                if a != b {
                    neighbor_indices[cursors[b]] = a;
                    cursors[b] += 1;
                }
            }
            nonmanifold_faces.sort_unstable();
            nonmanifold_faces.dedup();
            let mut face_pair_counts = vec![0; self.triangles.len()];
            for &[a, b] in &face_pairs {
                face_pair_counts[a as usize] += 1;
                face_pair_counts[b as usize] += 1;
            }
            let face_pair_offsets = offsets_from_counts(face_pair_counts);
            let mut face_pair_indices = vec![0; *face_pair_offsets.last().unwrap()];
            let mut cursors = face_pair_offsets[..self.triangles.len()].to_vec();
            for (index, pair) in face_pairs.iter().enumerate() {
                for &face in pair {
                    let face = face as usize;
                    face_pair_indices[cursors[face]] = index;
                    cursors[face] += 1;
                }
            }
            self.adjacency = Some(Arc::new(Adjacency {
                signature,
                neighbor_offsets,
                neighbor_indices,
                vertex_face_offsets,
                vertex_faces,
                face_pairs,
                face_pair_offsets,
                face_pair_indices,
                nonmanifold_faces,
                invalid_indices,
                normal_only: false,
            }));
        }
    }
}

#[cfg(test)]
mod lazy_normal_incidence_regressions {
    use super::*;
    #[allow(dead_code)] // Frozen layout witness includes every C52 private cache field.
    struct FrozenAdjacencyLayout {
        signature: u64,
        neighbor_offsets: Vec<usize>,
        neighbor_indices: Vec<usize>,
        vertex_face_offsets: Vec<usize>,
        vertex_faces: Vec<u32>,
        face_pairs: Vec<[u32; 2]>,
        face_pair_offsets: Vec<usize>,
        face_pair_indices: Vec<usize>,
        nonmanifold_faces: Vec<u32>,
        invalid_indices: bool,
    }
    const _: () =
        assert!(std::mem::size_of::<Adjacency>() == std::mem::size_of::<FrozenAdjacencyLayout>());
    const _: () = assert!(
        std::mem::size_of::<Option<Adjacency>>()
            == std::mem::size_of::<Option<FrozenAdjacencyLayout>>()
    );
    fn words(values: &[Vec3]) -> Vec<[u32; 3]> {
        values
            .iter()
            .map(|v| v.to_array().map(f32::to_bits))
            .collect()
    }
    fn graph(mesh: &Mesh) -> serde_json::Value {
        let a = mesh.adjacency.as_ref().unwrap();
        serde_json::json!({"s":a.signature,"n":a.neighbor_offsets,"ni":a.neighbor_indices,"v":a.vertex_face_offsets,"vf":a.vertex_faces,"p":a.face_pairs,"po":a.face_pair_offsets,"pi":a.face_pair_indices,"bad":a.nonmanifold_faces,"invalid":a.invalid_indices,"caps":[a.neighbor_offsets.capacity(),a.neighbor_indices.capacity(),a.vertex_face_offsets.capacity(),a.vertex_faces.capacity(),a.face_pairs.capacity(),a.face_pair_offsets.capacity(),a.face_pair_indices.capacity(),a.nonmanifold_faces.capacity()]})
    }
    fn mixed() -> Mesh {
        let mut m = Mesh::sphere(2);
        m.triangles.extend([
            [0, 1, 2],
            [0, 1, 3],
            [0, 1, 4],
            [2, 2, 3],
            [5, 5, 5],
            [0, 1, u32::MAX],
        ]);
        m
    }
    #[test]
    fn normal_only_updates_match_frozen_full_incidence_every_word_without_other_graphs() {
        for mut mesh in [Mesh::sphere(3), Mesh::cube(), mixed()] {
            let dirty = [0, 1, 4];
            for &v in &dirty {
                mesh.positions[v] += Vec3::new(0.012, -0.023, 0.031);
            }
            let mut old = mesh.snapshot();
            old.ensure_adjacency_c52_reference();
            old.recompute_normals_near_vertices_cached(&dirty);
            mesh.recompute_normals_near_vertices(&dirty);
            assert_eq!(words(&mesh.normals), words(&old.normals));
            assert_eq!(
                serde_json::to_vec(&mesh).unwrap(),
                serde_json::to_vec(&old).unwrap()
            );
            let a = mesh.adjacency.as_ref().unwrap();
            let b = old.adjacency.as_ref().unwrap();
            assert!(a.normal_only);
            assert_eq!(a.vertex_face_offsets, b.vertex_face_offsets);
            assert_eq!(a.vertex_faces, b.vertex_faces);
            assert_eq!(a.invalid_indices, b.invalid_indices);
            assert!(
                a.neighbor_offsets.is_empty()
                    && a.neighbor_indices.is_empty()
                    && a.face_pairs.is_empty()
                    && a.face_pair_offsets.is_empty()
                    && a.face_pair_indices.is_empty()
                    && a.nonmanifold_faces.is_empty()
            );
        }
    }
    #[test]
    fn full_upgrade_reuses_incidence_and_matches_frozen_graph_capacities_and_order() {
        for mut mesh in [Mesh::sphere(3), Mesh::cube(), mixed()] {
            let mut old = mesh.snapshot();
            old.ensure_adjacency_c52_reference();
            mesh.ensure_normal_incidence();
            let pointers = {
                let a = mesh.adjacency.as_ref().unwrap();
                (a.vertex_face_offsets.as_ptr(), a.vertex_faces.as_ptr())
            };
            mesh.ensure_adjacency();
            let a = mesh.adjacency.as_ref().unwrap();
            assert!(!a.normal_only);
            assert_eq!(
                (a.vertex_face_offsets.as_ptr(), a.vertex_faces.as_ptr()),
                pointers
            );
            assert_eq!(graph(&mesh), graph(&old));
        }
    }
    #[test]
    fn normal_incidence_rebuilds_after_same_size_topology_and_vertex_count_edits() {
        let mut mesh = Mesh::new(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE],
            vec![[3, 0, 1], [0, 4, 1], [2, 0, 4]],
        );
        mesh.ensure_normal_incidence();
        for step in 0..3 {
            if step == 0 {
                mesh.triangles[2] = [2, 3, 4];
            } else if step == 1 {
                mesh.positions.push(Vec3::new(0.25, 0.5, 0.75));
                mesh.normals.push(Vec3::ZERO);
                mesh.masks.push(0.);
                mesh.colors.push([0.5; 3]);
            } else {
                mesh.triangles[0] = [0, 1, u32::MAX];
            }
            let mut old = mesh.snapshot();
            old.ensure_adjacency_c52_reference();
            old.recompute_normals_near_vertices_cached(&[0, 2]);
            mesh.recompute_normals_near_vertices(&[0, 2]);
            assert_eq!(words(&mesh.normals), words(&old.normals));
            assert_eq!(
                mesh.adjacency.as_ref().unwrap().vertex_faces,
                old.adjacency.as_ref().unwrap().vertex_faces
            );
            mesh.ensure_adjacency();
            assert_eq!(graph(&mesh), graph(&old));
        }
    }
    #[test]
    fn source_fold_guard_upgrades_partial_cache_and_matches_frozen_protection() {
        let mut mesh = Mesh::sphere(2);
        mesh.ensure_normal_incidence();
        let mut old = mesh.snapshot();
        old.ensure_adjacency_c52_reference();
        let before = mesh.positions.clone();
        mesh.positions[0] += Vec3::new(0., 0., 3.);
        old.positions[0] = mesh.positions[0];
        let actual = mesh.accept_source_geometry_candidate(&before);
        let expected = old.accept_source_geometry_candidate(&before);
        assert_eq!(actual.to_bits(), expected.to_bits());
        assert_eq!(words(&mesh.positions), words(&old.positions));
        assert!(!mesh.adjacency.as_ref().unwrap().normal_only);
        assert_eq!(graph(&mesh), graph(&old));
    }
    #[test]
    fn snapshots_drop_partial_cache_and_clones_preserve_its_capability() {
        let mut mesh = Mesh::sphere(2);
        mesh.ensure_normal_incidence();
        let serialized = serde_json::to_vec(&mesh).unwrap();
        let snapshot = mesh.snapshot();
        assert!(snapshot.adjacency.is_none());
        assert_eq!(serde_json::to_vec(&snapshot).unwrap(), serialized);
        let mut cloned = mesh.clone();
        assert!(cloned.adjacency.as_ref().unwrap().normal_only);
        assert_eq!(graph(&cloned), graph(&mesh));
        cloned.ensure_adjacency();
        assert!(!cloned.adjacency.as_ref().unwrap().normal_only);
        assert!(mesh.adjacency.as_ref().unwrap().normal_only);
        assert_eq!(serde_json::to_vec(&cloned).unwrap(), serialized);
    }

    #[test]
    fn resident_candidate_reuses_immutable_full_topology_and_rechecks_public_edits() {
        let mut mesh = Mesh::sphere(2);
        mesh.ensure_adjacency();
        let mut candidate = mesh.try_resident_candidate().unwrap();
        assert!(Arc::ptr_eq(
            mesh.adjacency.as_ref().unwrap(),
            candidate.adjacency.as_ref().unwrap()
        ));
        candidate.positions[0] += Vec3::new(0.01, 0.02, 0.03);
        candidate.ensure_adjacency();
        assert!(Arc::ptr_eq(
            mesh.adjacency.as_ref().unwrap(),
            candidate.adjacency.as_ref().unwrap()
        ));
        assert!(mesh.snapshot().adjacency.is_none());

        candidate.triangles[0].swap(1, 2);
        candidate.ensure_adjacency();
        assert!(!Arc::ptr_eq(
            mesh.adjacency.as_ref().unwrap(),
            candidate.adjacency.as_ref().unwrap()
        ));
        assert_ne!(
            mesh.adjacency.as_ref().unwrap().signature,
            candidate.adjacency.as_ref().unwrap().signature
        );
    }
}

#[cfg(test)]
pub(crate) mod validation_probe {
    use std::cell::Cell;
    thread_local! { static CALLS: Cell<usize> = const { Cell::new(0) }; }
    pub fn record() {
        CALLS.with(|v| v.set(v.get() + 1));
    }
    pub fn reset() {
        CALLS.with(|v| v.set(0));
    }
    pub fn count() -> usize {
        CALLS.with(Cell::get)
    }
}
