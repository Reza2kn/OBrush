//! Deterministic, conservative quadric-error edge collapse.
//!
//! This independently written implementation follows the plane-quadric idea
//! from Garland and Heckbert, *Surface Simplification Using Quadric Error
//! Metrics* (1997), https://www.cs.cmu.edu/~garland/Papers/quadrics.pdf .
//! It does not implement a reference application's decimator.
//! Boundary, masked and (optionally) sharp-feature vertices remain fixed.
//! Local orientation and link checks do not prove global intersection freedom.

use crate::Mesh;
use glam::{DMat3, DVec3};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

/// Operation limits are lower than the import limits to bound CPU working memory.
pub const MAX_DECIMATION_VERTICES: usize = 250_000;
pub const MAX_DECIMATION_TRIANGLES: usize = 500_000;
const MAX_VERTEX_FACES: usize = 256;
const MAX_CANDIDATE_POPS: usize = 10_000_000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecimateSettings {
    /// Desired fraction of the original triangles, in (0, 1].
    pub target_ratio: f32,
    /// World-space upper bound on distance from every original vertex in a
    /// collapsed cluster to its new representative. This conservative bound
    /// is not a surface Hausdorff-distance measurement.
    pub max_error: f32,
    pub preserve_sharp_features: bool,
    pub feature_angle_degrees: f32,
    /// Bounds heap work. The result may stop above the requested face count.
    pub max_candidate_pops: usize,
}

impl Default for DecimateSettings {
    fn default() -> Self {
        Self {
            target_ratio: 0.5,
            max_error: 0.05,
            preserve_sharp_features: true,
            feature_angle_degrees: 45.0,
            max_candidate_pops: 2_000_000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecimateReport {
    pub input_vertices: usize,
    pub input_triangles: usize,
    pub output_vertices: usize,
    pub output_triangles: usize,
    pub target_triangles: usize,
    pub collapses: usize,
    pub candidate_pops: usize,
    pub reached_target: bool,
    pub max_vertex_displacement_bound: f64,
    /// Maximum accumulated, area-weighted plane error divided by cluster plane
    /// weight. This is an internal QEM diagnostic, not measured surface error.
    pub max_quadric_error: f64,
    pub protected_vertices: usize,
    pub boundary_vertices: usize,
    pub termination: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum DecimatePhase {
    Validate,
    Build,
    Collapse,
    Compact,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DecimateProgress {
    pub phase: DecimatePhase,
    pub completed: usize,
    pub total: usize,
}

fn checkpoint(
    cancel: &mut impl FnMut(DecimateProgress) -> bool,
    phase: DecimatePhase,
    completed: usize,
    total: usize,
) -> Result<(), String> {
    if cancel(DecimateProgress {
        phase,
        completed,
        total,
    }) {
        Err("Decimation cancelled".into())
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Default)]
struct Quadric {
    // Symmetric 4x4: xx, xy, xz, xw, yy, yz, yw, zz, zw, ww.
    coefficients: [f64; 10],
    weight: f64,
}

impl Quadric {
    fn plane(normal: DVec3, offset: f64, weight: f64) -> Self {
        let [x, y, z] = normal.to_array();
        Self {
            coefficients: [
                x * x,
                x * y,
                x * z,
                x * offset,
                y * y,
                y * z,
                y * offset,
                z * z,
                z * offset,
                offset * offset,
            ]
            .map(|value| value * weight),
            weight,
        }
    }

    fn add(self, other: Self) -> Self {
        let mut result = self;
        for (value, addition) in result.coefficients.iter_mut().zip(other.coefficients) {
            *value += addition;
        }
        result.weight += other.weight;
        result
    }

    fn error(self, point: DVec3) -> f64 {
        let [a, b, c, d, e, f, g, h, i, j] = self.coefficients;
        let [x, y, z] = point.to_array();
        (a * x * x
            + 2.0 * b * x * y
            + 2.0 * c * x * z
            + 2.0 * d * x
            + e * y * y
            + 2.0 * f * y * z
            + 2.0 * g * y
            + h * z * z
            + 2.0 * i * z
            + j)
            .max(0.0)
    }

    fn optimum(self) -> Option<DVec3> {
        let [a, b, c, d, e, f, g, h, i, _] = self.coefficients;
        let matrix = DMat3::from_cols(
            DVec3::new(a, b, c),
            DVec3::new(b, e, f),
            DVec3::new(c, f, h),
        );
        let scale = a.abs().max(e.abs()).max(h.abs());
        if scale == 0.0 || matrix.determinant().abs() < 1.0e-12 * scale.powi(3) {
            return None;
        }
        let point = matrix.inverse() * -DVec3::new(d, g, i);
        point.is_finite().then_some(point)
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    error: f64,
    vertices: [usize; 2],
    generations: [u32; 2],
    point: DVec3,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max heap: reverse for minimum error, then stable IDs.
        other
            .error
            .total_cmp(&self.error)
            .then_with(|| other.vertices.cmp(&self.vertices))
            .then_with(|| other.generations.cmp(&self.generations))
    }
}

struct Decimator {
    positions: Vec<DVec3>,
    colors: Vec<[f32; 3]>,
    masks: Vec<f32>,
    mask_convention: crate::MaskConvention,
    materials: Option<Vec<[f32; 2]>>,
    faces: Vec<Option<[usize; 3]>>,
    incident: Vec<Vec<usize>>,
    quadrics: Vec<Quadric>,
    alive: Vec<bool>,
    pinned: Vec<bool>,
    generations: Vec<u32>,
    radii: Vec<f64>,
    heap: BinaryHeap<Candidate>,
    face_count: usize,
    heap_limit: usize,
    settings: DecimateSettings,
}

impl Mesh {
    /// Produce a new mesh without changing the source. All validation and
    /// simplification finish before a host can replace its document/undo state.
    pub fn decimated(&self, settings: &DecimateSettings) -> Result<(Mesh, DecimateReport), String> {
        self.decimated_with_progress(settings, |_| false)
    }

    /// The callback returns true to cancel. Calls occur before allocation,
    /// between setup phases and at most every 256 candidate pops; setup and
    /// heap rebuilds are bounded atomic phases rather than real-time slices.
    pub fn decimated_with_progress(
        &self,
        settings: &DecimateSettings,
        mut cancel: impl FnMut(DecimateProgress) -> bool,
    ) -> Result<(Mesh, DecimateReport), String> {
        checkpoint(&mut cancel, DecimatePhase::Validate, 0, 1)?;
        self.validate()?;
        self.require_plain_topology("Decimation")?;
        validate_settings(settings)?;
        if self.positions.len() > MAX_DECIMATION_VERTICES
            || self.triangles.len() > MAX_DECIMATION_TRIANGLES
        {
            return Err(format!(
                "Decimation currently supports at most {MAX_DECIMATION_VERTICES} vertices and {MAX_DECIMATION_TRIANGLES} triangles"
            ));
        }
        checkpoint(&mut cancel, DecimatePhase::Build, 0, 1)?;
        let (mut decimator, boundary_vertices) = Decimator::new(self, *settings)?;
        let target =
            ((self.triangles.len() as f64 * settings.target_ratio as f64).ceil() as usize).max(1);
        let mut report = DecimateReport {
            input_vertices: self.positions.len(),
            input_triangles: self.triangles.len(),
            output_vertices: self.positions.len(),
            output_triangles: self.triangles.len(),
            target_triangles: target,
            collapses: 0,
            candidate_pops: 0,
            reached_target: false,
            max_vertex_displacement_bound: 0.0,
            max_quadric_error: 0.0,
            protected_vertices: decimator.pinned.iter().filter(|&&pin| pin).count(),
            boundary_vertices,
            termination: String::new(),
        };
        if decimator.face_count > target {
            decimator.rebuild_heap();
        }
        let mut rebuild_collapses = 0;
        while decimator.face_count > target && report.candidate_pops < settings.max_candidate_pops {
            if report.candidate_pops.is_multiple_of(256) {
                checkpoint(
                    &mut cancel,
                    DecimatePhase::Collapse,
                    report.candidate_pops,
                    settings.max_candidate_pops,
                )?;
            }
            let Some(candidate) = decimator.heap.pop() else {
                // Links can become legal after a neighboring collapse. Retry
                // those edges once after making progress, rather than adding
                // duplicate unchanged candidates at every local edit.
                if report.collapses > rebuild_collapses {
                    rebuild_collapses = report.collapses;
                    decimator.rebuild_heap();
                    continue;
                }
                break;
            };
            report.candidate_pops += 1;
            let [a, b] = candidate.vertices;
            if !decimator.alive[a]
                || !decimator.alive[b]
                || candidate.generations != [decimator.generations[a], decimator.generations[b]]
            {
                continue;
            }
            if let Some((radius, error)) = decimator.collapse(candidate) {
                report.collapses += 1;
                report.max_vertex_displacement_bound =
                    report.max_vertex_displacement_bound.max(radius);
                report.max_quadric_error = report.max_quadric_error.max(error);
            }
            if decimator.heap.len() > decimator.heap_limit {
                decimator.rebuild_heap();
            }
        }
        report.reached_target = decimator.face_count <= target;
        report.termination = if report.reached_target {
            "Target reached"
        } else if report.candidate_pops == settings.max_candidate_pops {
            "Work limit reached"
        } else {
            "No remaining collapse satisfies topology, protection and error limits"
        }
        .into();
        checkpoint(&mut cancel, DecimatePhase::Compact, 0, 1)?;
        let result = if report.collapses == 0 {
            self.clone()
        } else {
            decimator.compact()
        };
        result.validate()?;
        report.output_vertices = result.positions.len();
        report.output_triangles = result.triangles.len();
        Ok((result, report))
    }
}

fn validate_settings(settings: &DecimateSettings) -> Result<(), String> {
    if !settings.target_ratio.is_finite()
        || settings.target_ratio <= 0.0
        || settings.target_ratio > 1.0
    {
        return Err("Decimation target ratio must be greater than zero and at most one".into());
    }
    if !settings.max_error.is_finite() || settings.max_error < 0.0 {
        return Err("Decimation error bound must be finite and nonnegative".into());
    }
    if !settings.feature_angle_degrees.is_finite()
        || !(0.0..180.0).contains(&settings.feature_angle_degrees)
    {
        return Err("Feature angle must be finite and in 0–180 degrees".into());
    }
    if settings.max_candidate_pops == 0 || settings.max_candidate_pops > MAX_CANDIDATE_POPS {
        return Err(format!(
            "Decimation work limit must be in 1–{MAX_CANDIDATE_POPS}"
        ));
    }
    Ok(())
}

impl Decimator {
    fn new(mesh: &Mesh, settings: DecimateSettings) -> Result<(Self, usize), String> {
        let count = mesh.positions.len();
        let positions: Vec<_> = mesh.positions.iter().map(|p| p.as_dvec3()).collect();
        let mut incident = vec![Vec::new(); count];
        let mut quadrics = vec![Quadric::default(); count];
        let mut pinned: Vec<_> = (0..mesh.positions.len())
            .map(|vertex| mesh.has_protection(vertex))
            .collect();
        let mut normals = Vec::with_capacity(mesh.triangles.len());
        let mut edge_faces = Vec::with_capacity(mesh.triangles.len() * 3);
        let mut unique_faces = HashSet::with_capacity(mesh.triangles.len());
        for (face, triangle) in mesh.triangles.iter().enumerate() {
            let vertices = triangle.map(|v| v as usize);
            let mut key = vertices;
            key.sort_unstable();
            if !unique_faces.insert(key) {
                return Err(format!("Decimation rejects duplicate triangle {face}"));
            }
            let [a, b, c] = vertices.map(|v| positions[v]);
            let cross = (b - a).cross(c - a);
            let double_area = cross.length();
            if !double_area.is_finite() || triangle_quality([a, b, c]) < 1.0e-10 {
                return Err(format!("Decimation rejects degenerate triangle {face}"));
            }
            let normal = cross / double_area;
            normals.push(normal);
            let plane = Quadric::plane(normal, -normal.dot(a), double_area * 0.5);
            for vertex in vertices {
                incident[vertex].push(face);
                if incident[vertex].len() > MAX_VERTEX_FACES {
                    return Err(format!(
                        "Decimation vertex {vertex} exceeds {MAX_VERTEX_FACES} incident triangles"
                    ));
                }
                quadrics[vertex] = quadrics[vertex].add(plane);
            }
            for edge in [
                [vertices[0], vertices[1]],
                [vertices[1], vertices[2]],
                [vertices[2], vertices[0]],
            ] {
                let [lo, hi] = [edge[0].min(edge[1]), edge[0].max(edge[1])];
                edge_faces.push((((lo as u64) << 32) | hi as u64, face, edge[0] == lo));
            }
        }
        edge_faces.sort_unstable();
        let mut boundary = vec![false; count];
        let sharp_cosine = (settings.feature_angle_degrees as f64).to_radians().cos();
        let mut index = 0;
        while index < edge_faces.len() {
            let key = edge_faces[index].0;
            let first = index;
            while index < edge_faces.len() && edge_faces[index].0 == key {
                index += 1;
            }
            let [a, b] = [(key >> 32) as usize, key as u32 as usize];
            match index - first {
                1 => {
                    boundary[a] = true;
                    boundary[b] = true;
                    pinned[a] = true;
                    pinned[b] = true;
                }
                2 => {
                    if edge_faces[first].2 == edge_faces[first + 1].2 {
                        return Err("Decimation requires consistent adjacent face winding".into());
                    }
                    if settings.preserve_sharp_features
                        && normals[edge_faces[first].1].dot(normals[edge_faces[first + 1].1])
                            < sharp_cosine
                    {
                        pinned[a] = true;
                        pinned[b] = true;
                    }
                }
                _ => return Err("Decimation requires manifold edges".into()),
            }
        }
        for (vertex, faces) in incident.iter().enumerate() {
            validate_vertex_fan(vertex, faces, &mesh.triangles)?;
        }
        let boundary_count = boundary.iter().filter(|&&value| value).count();
        Ok((
            Self {
                positions,
                colors: mesh.colors.clone(),
                masks: mesh.masks.clone(),
                mask_convention: mesh.mask_convention,
                materials: mesh.materials.clone(),
                faces: mesh
                    .triangles
                    .iter()
                    .map(|triangle| Some(triangle.map(|vertex| vertex as usize)))
                    .collect(),
                incident,
                quadrics,
                alive: vec![true; count],
                pinned,
                generations: vec![0; count],
                radii: vec![0.0; count],
                heap: BinaryHeap::new(),
                face_count: mesh.triangles.len(),
                heap_limit: (mesh.triangles.len() * 6).max(1024),
                settings,
            },
            boundary_count,
        ))
    }

    fn neighbors(&self, vertex: usize) -> Vec<usize> {
        let mut neighbors = Vec::with_capacity(self.incident[vertex].len());
        for &face in &self.incident[vertex] {
            if let Some(triangle) = self.faces[face] {
                neighbors.extend(triangle.into_iter().filter(|&v| v != vertex));
            }
        }
        neighbors.sort_unstable();
        neighbors.dedup();
        neighbors
    }

    fn candidate(&self, a: usize, b: usize) -> Option<Candidate> {
        if self.pinned[a] || self.pinned[b] || !self.alive[a] || !self.alive[b] {
            return None;
        }
        let quadric = self.quadrics[a].add(self.quadrics[b]);
        let mut points = vec![
            self.positions[a],
            self.positions[b],
            (self.positions[a] + self.positions[b]) * 0.5,
        ];
        if let Some(optimum) = quadric.optimum() {
            points.push(optimum);
        }
        let mut best: Option<(f64, DVec3)> = None;
        for point in points {
            // Geometry guards examine precisely the f32 position that is stored.
            let point = point.as_vec3().as_dvec3();
            let radius = (self.radii[a] + point.distance(self.positions[a]))
                .max(self.radii[b] + point.distance(self.positions[b]));
            if !point.is_finite() || radius > self.settings.max_error as f64 {
                continue;
            }
            let error = quadric.error(point);
            if error.is_finite() && best.is_none_or(|(old, _)| error < old) {
                best = Some((error, point));
            }
        }
        best.map(|(error, point)| Candidate {
            error,
            vertices: [a.min(b), a.max(b)],
            generations: [self.generations[a.min(b)], self.generations[a.max(b)]],
            point,
        })
    }

    fn rebuild_heap(&mut self) {
        let mut candidates = Vec::with_capacity(self.face_count * 3 / 2);
        for vertex in 0..self.positions.len() {
            if self.alive[vertex] && !self.pinned[vertex] {
                for neighbor in self.neighbors(vertex) {
                    if vertex < neighbor {
                        if let Some(candidate) = self.candidate(vertex, neighbor) {
                            candidates.push(candidate);
                        }
                    }
                }
            }
        }
        self.heap = BinaryHeap::from(candidates);
    }

    fn collapse(&mut self, candidate: Candidate) -> Option<(f64, f64)> {
        let [a, b] = candidate.vertices;
        let neighbors_a = self.neighbors(a);
        let neighbors_b = self.neighbors(b);
        if !neighbors_a.contains(&b) {
            return None;
        }
        let common: Vec<_> = neighbors_a
            .iter()
            .copied()
            .filter(|vertex| neighbors_b.binary_search(vertex).is_ok())
            .collect();
        let mut affected = self.incident[a].clone();
        affected.extend(&self.incident[b]);
        affected.sort_unstable();
        affected.dedup();
        let mut opposites = Vec::new();
        let mut replacements = Vec::with_capacity(affected.len());
        let mut face_keys = HashSet::with_capacity(affected.len());
        for &face in &affected {
            let triangle = self.faces[face]?;
            if triangle.contains(&a) && triangle.contains(&b) {
                opposites.extend(triangle.into_iter().filter(|v| *v != a && *v != b));
                replacements.push((face, None));
                continue;
            }
            let replacement = triangle.map(|vertex| if vertex == b { a } else { vertex });
            let mut key = replacement;
            key.sort_unstable();
            if !face_keys.insert(key) {
                return None;
            }
            let old = triangle.map(|vertex| self.positions[vertex]);
            let new = replacement.map(|vertex| {
                if vertex == a {
                    candidate.point
                } else {
                    self.positions[vertex]
                }
            });
            let old_cross = (old[1] - old[0]).cross(old[2] - old[0]);
            let new_cross = (new[1] - new[0]).cross(new[2] - new[0]);
            if old_cross.dot(new_cross) < old_cross.length() * new_cross.length() * 0.1
                || triangle_quality(new) + 1.0e-14 < triangle_quality(old).min(0.05)
            {
                return None;
            }
            replacements.push((face, Some(replacement)));
        }
        opposites.sort_unstable();
        // Pinned boundary vertices mean all candidate edges must be interior.
        if opposites.len() != 2 || opposites != common {
            return None;
        }
        let new_valence = replacements
            .iter()
            .filter(|(_, face)| face.is_some())
            .count();
        if new_valence > MAX_VERTEX_FACES {
            return None;
        }
        let radius = (self.radii[a] + candidate.point.distance(self.positions[a]))
            .max(self.radii[b] + candidate.point.distance(self.positions[b]));
        if radius > self.settings.max_error as f64 {
            return None;
        }
        let segment = self.positions[b] - self.positions[a];
        let mix = if segment.length_squared() > 0.0 {
            ((candidate.point - self.positions[a]).dot(segment) / segment.length_squared())
                .clamp(0.0, 1.0) as f32
        } else {
            0.5
        };
        self.colors[a] = std::array::from_fn(|channel| {
            self.colors[a][channel] * (1.0 - mix) + self.colors[b][channel] * mix
        });
        self.masks[a] = self.masks[a] * (1.0 - mix) + self.masks[b] * mix;
        if let Some(materials) = &mut self.materials {
            materials[a] = std::array::from_fn(|channel| {
                materials[a][channel] * (1.0 - mix) + materials[b][channel] * mix
            });
        }
        for &(face, _) in &replacements {
            for vertex in self.faces[face]? {
                let incident = &mut self.incident[vertex];
                if let Some(index) = incident.iter().position(|&value| value == face) {
                    incident.swap_remove(index);
                }
            }
        }
        for &(face, replacement) in &replacements {
            self.faces[face] = replacement;
            if let Some(triangle) = replacement {
                for vertex in triangle {
                    self.incident[vertex].push(face);
                }
            } else {
                self.face_count -= 1;
            }
        }
        self.positions[a] = candidate.point;
        self.radii[a] = radius;
        self.quadrics[a] = self.quadrics[a].add(self.quadrics[b]);
        self.alive[b] = false;
        self.generations[a] += 1;
        self.generations[b] += 1;
        // Only endpoint quadrics/positions changed. Existing heap entries for
        // other edges remain useful; link checks run again when they are popped.
        for neighbor in self.neighbors(a) {
            if let Some(candidate) = self.candidate(a, neighbor) {
                self.heap.push(candidate);
            }
        }
        let error = (self.quadrics[a].error(candidate.point) / self.quadrics[a].weight).sqrt();
        Some((radius, error))
    }

    fn compact(self) -> Mesh {
        let mut mapping = vec![u32::MAX; self.positions.len()];
        let mut positions = Vec::new();
        let mut masks = Vec::new();
        let mut colors = Vec::new();
        let mut materials = self.materials.as_ref().map(|_| Vec::new());
        for (vertex, &alive) in self.alive.iter().enumerate() {
            if alive {
                mapping[vertex] = positions.len() as u32;
                positions.push(self.positions[vertex].as_vec3());
                masks.push(self.masks[vertex]);
                colors.push(self.colors[vertex]);
                if let (Some(output), Some(source)) = (&mut materials, &self.materials) {
                    output.push(source[vertex]);
                }
            }
        }
        let triangles = self
            .faces
            .into_iter()
            .flatten()
            .map(|face| face.map(|v| mapping[v]))
            .collect();
        let mut result = Mesh::new(positions, triangles);
        result.masks = masks;
        result.mask_convention = self.mask_convention;
        result.colors = colors;
        result.materials = materials;
        result
    }
}

fn triangle_quality(points: [DVec3; 3]) -> f64 {
    let [a, b, c] = points;
    let longest_squared = a
        .distance_squared(b)
        .max(b.distance_squared(c))
        .max(c.distance_squared(a));
    if longest_squared == 0.0 {
        0.0
    } else {
        (b - a).cross(c - a).length() / longest_squared
    }
}

fn validate_vertex_fan(
    vertex: usize,
    faces: &[usize],
    triangles: &[[u32; 3]],
) -> Result<(), String> {
    if faces.is_empty() {
        return Err(format!("Decimation rejects isolated vertex {vertex}"));
    }
    let mut seen = vec![false; faces.len()];
    seen[0] = true;
    let mut pending = vec![0];
    while let Some(index) = pending.pop() {
        let triangle = triangles[faces[index]];
        for (other, &face) in faces.iter().enumerate() {
            if !seen[other]
                && triangles[face]
                    .iter()
                    .any(|&neighbor| neighbor as usize != vertex && triangle.contains(&neighbor))
            {
                seen[other] = true;
                pending.push(other);
            }
        }
    }
    if seen.iter().any(|&value| !value) {
        Err(format!(
            "Decimation rejects disconnected face fan at vertex {vertex}"
        ))
    } else {
        Ok(())
    }
}
