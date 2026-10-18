//! Bounded, independently written linear-mask clipping and signed shells.
//! Source winding defines outward. This checks local geometry and manifold
//! correspondence, not global intersections or reference extraction behavior.
use crate::{Mesh, MAX_TRIANGLES, MAX_VERTICES};
use glam::{DVec3, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_SOURCE_VERTICES: usize = 250_000;
const MAX_SOURCE_TRIANGLES: usize = 500_000;

/// Face admission policy. Clipped preserves OBrush's original interpolated
/// boundary; WholeIncidentFaces follows SculptGL's face-selection scope but
/// does not claim its offset, bridge, or smoothing response.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaskExtractSelection {
    #[default]
    Clipped,
    WholeIncidentFaces,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaskExtractSettings {
    pub threshold: f32,
    /// Signed world units from the original patch to the second skin.
    /// Zero produces an open surface; negative values extrude inward.
    pub thickness: f32,
}

impl Default for MaskExtractSettings {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            thickness: 0.02,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaskExtractReport {
    pub input_vertices: usize,
    pub input_triangles: usize,
    pub selected_source_triangles: usize,
    pub patch_vertices: usize,
    pub patch_triangles: usize,
    pub patch_components: usize,
    pub boundary_edges: usize,
    pub boundary_loops: usize,
    pub output_vertices: usize,
    pub output_triangles: usize,
    pub threshold: f32,
    pub thickness: f32,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum PointKey {
    Original(u32),
    Edge(u32, u32),
}

struct PatchPoint {
    position: Vec3,
    color: [f32; 3],
    material: Option<[f32; 2]>,
}

impl Mesh {
    /// Clip each ordered triangle where its linearly interpolated mask is
    /// `mask >= threshold`. Original IDs and canonical edge cuts are welded.
    /// Corner-angle-weighted geometric normals of the selected rounded patch
    /// determine extrusion; stored source normals do not influence the result.
    /// Positive thickness places a second skin outward; negative thickness
    /// places it inward. Zero returns the selected open patch only. Nonzero
    /// thickness bridges the boundary into a closed shell.
    /// Errors leave the source untouched. Global collisions remain unverified.
    pub fn extract_mask(
        &self,
        settings: &MaskExtractSettings,
    ) -> Result<(Self, MaskExtractReport), String> {
        self.extract_mask_with_selection(settings, MaskExtractSelection::Clipped)
    }

    pub fn extract_mask_with_selection(
        &self,
        settings: &MaskExtractSettings,
        selection: MaskExtractSelection,
    ) -> Result<(Self, MaskExtractReport), String> {
        self.extract_mask_with_surface_offset(settings, selection, false)
    }

    /// Reproduce SculptGL's face-averaged normal, signed 0.01-unit pre-offset
    /// and fresh appearance channels on a whole-face patch. Closed source
    /// surfaces also use a three-pass logical-quad border smooth when it keeps
    /// the output noncollapsed. Open sources retain OBrush's checked shell.
    pub fn extract_mask_with_surface_offset(
        &self,
        settings: &MaskExtractSettings,
        selection: MaskExtractSelection,
        sculptgl_surface_offset: bool,
    ) -> Result<(Self, MaskExtractReport), String> {
        self.extract_mask_with_affine(settings, selection, sculptgl_surface_offset, None)
    }

    pub(crate) fn extract_mask_with_affine(
        &self,
        settings: &MaskExtractSettings,
        selection: MaskExtractSelection,
        sculptgl_surface_offset: bool,
        affine: Option<&crate::ObjectAffineF32V1>,
    ) -> Result<(Self, MaskExtractReport), String> {
        if let Some(affine) = affine {
            affine.validate()?;
        }
        if sculptgl_surface_offset && selection != MaskExtractSelection::WholeIncidentFaces {
            return Err("SculptGL surface offset requires whole incident faces".into());
        }
        if !settings.threshold.is_finite() || !(0.01..=0.99).contains(&settings.threshold) {
            return Err("Mask extraction threshold must be finite in 0.01–0.99".into());
        }
        if !settings.thickness.is_finite() || !(-5.0..=5.0).contains(&settings.thickness) {
            return Err("Mask extraction thickness must be finite in -5–5 world units".into());
        }
        // Reject large public input before validation/topology scratch buffers.
        if self.positions.len() > MAX_SOURCE_VERTICES || self.triangles.len() > MAX_SOURCE_TRIANGLES
        {
            return Err("Mask extraction supports at most 250000 source vertices and 500000 source triangles".into());
        }
        self.validate()?;
        for (face, ids) in self.triangles.iter().enumerate() {
            face_normal(&self.positions, *ids)
                .map_err(|_| format!("Mask extraction source face {face} is collapsed"))?;
        }
        // Validate the complete source, including unselected regions.
        let source_topology = topology(self.positions.len(), &self.triangles)?;
        let surface_only = settings.thickness == 0.0;
        let patch_vertex_limit = if surface_only {
            MAX_VERTICES
        } else {
            MAX_VERTICES / 2
        };
        let patch_triangle_limit = if surface_only {
            MAX_TRIANGLES
        } else {
            MAX_TRIANGLES / 2
        };
        let mut points = Vec::new();
        let mut point_ids = HashMap::new();
        let mut faces = Vec::new();
        let mut selected_source_triangles = 0;
        let original_face_selection = if selection == MaskExtractSelection::WholeIncidentFaces {
            self.surface
                .as_ref()
                .map(|surface| {
                    let mut selected = reserved(self.triangles.len(), "selected original faces")?;
                    for face in &surface.faces {
                        let arity = if face[3] == crate::TRIANGLE_CORNER {
                            3
                        } else {
                            4
                        };
                        let included = face[..arity].iter().any(|&vertex| {
                            self.mask_convention.protection_at_least(
                                self.masks[vertex as usize],
                                settings.threshold,
                            )
                        });
                        selected.push(included);
                        if arity == 4 {
                            selected.push(included);
                        }
                    }
                    Ok::<_, String>(selected)
                })
                .transpose()?
        } else {
            None
        };
        for (source_face_index, &ids) in self.triangles.iter().enumerate() {
            let polygon = match selection {
                MaskExtractSelection::Clipped => clipped_keys(self, ids, settings.threshold),
                MaskExtractSelection::WholeIncidentFaces => {
                    let included = original_face_selection.as_ref().map_or_else(
                        || {
                            ids.iter().any(|&v| {
                                self.mask_convention
                                    .protection_at_least(self.masks[v as usize], settings.threshold)
                            })
                        },
                        |selected| selected[source_face_index],
                    );
                    if included {
                        ids.into_iter().map(PointKey::Original).collect()
                    } else {
                        Vec::new()
                    }
                }
            };
            // Exact-threshold isolated points/edges carry no selected area.
            if polygon.len() < 3 {
                continue;
            }
            let mut polygon_ids = [0; 4];
            for (slot, &key) in polygon.iter().enumerate() {
                polygon_ids[slot] = match point_ids.get(&key) {
                    Some(&id) => id,
                    None => {
                        let point = clipped_point(self, key, settings.threshold)?;
                        let id = points.len() as u32;
                        push_bounded(&mut points, point, patch_vertex_limit, "patch vertices")?;
                        point_ids
                            .try_reserve(1)
                            .map_err(|_| "Cannot allocate mask extraction weld map")?;
                        point_ids.insert(key, id);
                        id
                    }
                };
            }
            let original_normal = face_normal(&self.positions, ids)?;
            for i in 1..polygon.len() - 1 {
                let triangle = [polygon_ids[0], polygon_ids[i], polygon_ids[i + 1]];
                let [a, b, c] = triangle.map(|v| points[v as usize].position.as_dvec3());
                require_orientation((b - a).cross(c - a), original_normal, "clipped patch")?;
                push_bounded(
                    &mut faces,
                    triangle,
                    patch_triangle_limit,
                    "patch triangles",
                )?;
            }
            selected_source_triangles += 1;
        }
        if faces.is_empty() {
            return Err("The mask contains no selected surface area".into());
        }
        let mut patch = topology(points.len(), &faces)?;
        let normal_transform = if let Some(affine) = affine {
            let matrix = glam::DMat4::from_cols_array(&affine.local_to_world.map(f64::from));
            let linear = glam::DMat3::from_cols(
                matrix.x_axis.truncate(),
                matrix.y_axis.truncate(),
                matrix.z_axis.truncate(),
            );
            let normal = linear.inverse().transpose();
            if !normal.is_finite() {
                return Err("Mask extraction affine normal transform is not finite".into());
            }
            for point in &mut points {
                point.position = affine.world_point_validated(point.position)?.as_vec3();
            }
            if affine.determinant() < 0.0 {
                for face in &mut faces {
                    face.swap(1, 2);
                }
                patch = topology(points.len(), &faces)?;
            }
            Some(normal)
        } else {
            None
        };
        let mut positions = reserved(points.len(), "patch positions")?;
        positions.extend(points.iter().map(|p| p.position));
        if affine.is_some() {
            for (face_index, &face) in faces.iter().enumerate() {
                face_normal(&positions, face).map_err(|_| {
                    format!("Mask extraction affine collapses patch face {face_index}")
                })?;
            }
        }
        let mut source_normals = if sculptgl_surface_offset {
            Some(reserved(points.len(), "source normals")?)
        } else {
            None
        };
        if let Some(normals) = &mut source_normals {
            normals.resize(points.len(), DVec3::ZERO);
            let extraction_normals = sculptgl_face_averaged_normals(self)?;
            let epsilon = if settings.thickness < 0.0 {
                -0.01
            } else {
                0.01
            };
            for (key, &id) in &point_ids {
                let PointKey::Original(source_id) = key else {
                    return Err("SculptGL surface offset requires original patch vertices".into());
                };
                let normal = normal_transform
                    .map_or(extraction_normals[*source_id as usize], |transform| {
                        transform * extraction_normals[*source_id as usize]
                    });
                if !normal.is_finite() || normal.length_squared() == 0.0 {
                    return Err(
                        "SculptGL surface offset requires finite nonzero source normals".into(),
                    );
                }
                let normal = normal.normalize();
                let shifted = positions[id as usize].as_dvec3() + normal * epsilon;
                if !shifted.is_finite() || shifted.abs().max_element() > 1_000_000.0 {
                    return Err("SculptGL surface offset exceeds world-coordinate bounds".into());
                }
                positions[id as usize] = shifted.as_vec3();
                normals[id as usize] = normal;
            }
            for &ids in &faces {
                let original = ids.map(|v| points[v as usize].position);
                let original_normal = (original[1].as_dvec3() - original[0].as_dvec3())
                    .cross(original[2].as_dvec3() - original[0].as_dvec3());
                require_orientation(
                    face_normal(&positions, ids)?,
                    original_normal,
                    "surface offset",
                )?;
            }
        }
        if surface_only {
            let output_vertices = points.len();
            let output_triangles = faces.len();
            let mut mesh = Self::new(positions, faces);
            mesh.mask_convention = self.mask_convention;
            mesh.masks.fill(self.mask_convention.clear_value());
            if sculptgl_surface_offset {
                mesh.colors.fill([1.0; 3]);
                mesh.materials = Some(vec![[0.18, 0.08]; output_vertices]);
            } else {
                mesh.materials = self
                    .materials
                    .as_ref()
                    .map(|_| vec![[0.0; 2]; output_vertices]);
                for (i, point) in points.iter().enumerate() {
                    mesh.colors[i] = point.color;
                    if let (Some(output), Some(material)) = (&mut mesh.materials, point.material) {
                        output[i] = material;
                    }
                }
            }
            mesh.validate()?;
            let report = MaskExtractReport {
                input_vertices: self.positions.len(),
                input_triangles: self.triangles.len(),
                selected_source_triangles,
                patch_vertices: points.len(),
                patch_triangles: output_triangles,
                patch_components: patch.components,
                boundary_edges: patch.boundary.len(),
                boundary_loops: patch.boundary_loops,
                output_vertices,
                output_triangles,
                threshold: settings.threshold,
                thickness: settings.thickness,
            };
            return Ok((mesh, report));
        }
        let normals = match source_normals {
            Some(normals) => normals,
            None => patch_normals(&positions, &faces)?,
        };
        let output_vertices = points
            .len()
            .checked_mul(2)
            .filter(|&n| n <= MAX_VERTICES)
            .ok_or("Extracted shell exceeds the vertex limit")?;
        let output_triangles = faces
            .len()
            .checked_add(patch.boundary.len())
            .and_then(|n| n.checked_mul(2))
            .filter(|&n| n <= MAX_TRIANGLES)
            .ok_or("Extracted shell exceeds the triangle limit")?;
        let mut shell_positions = reserved(output_vertices, "shell positions")?;
        shell_positions.extend_from_slice(&positions);
        for (&point, &normal) in positions.iter().zip(&normals) {
            let outer = point.as_dvec3() + normal * f64::from(settings.thickness);
            if !outer.is_finite() || outer.abs().max_element() > 1_000_000.0 {
                return Err("Extracted shell exceeds finite world-coordinate bounds".into());
            }
            let outer = outer.as_vec3();
            if (outer.as_dvec3() - point.as_dvec3()).dot(normal)
                * f64::from(settings.thickness.signum())
                <= 0.0
            {
                return Err("Extraction thickness collapses at f32 coordinate precision".into());
            }
            shell_positions.push(outer);
        }
        let offset = points.len() as u32;
        let mut shell_faces = reserved(output_triangles, "shell triangles")?;
        for &ids in &faces {
            let expected = face_normal(&positions, ids)?;
            let outer = ids.map(|v| v + offset);
            require_orientation(
                face_normal(&shell_positions, outer)?,
                expected,
                "outer skin",
            )?;
            for &v in &ids {
                let displacement = shell_positions[v as usize + points.len()].as_dvec3()
                    - positions[v as usize].as_dvec3();
                if displacement.dot(expected) * f64::from(settings.thickness.signum()) <= 0.0 {
                    return Err(
                        "Extraction cannot preserve signed thickness on every incident face".into(),
                    );
                }
            }
            if settings.thickness > 0.0 {
                shell_faces.push([ids[0], ids[2], ids[1]]);
                shell_faces.push(outer);
            } else {
                shell_faces.push(ids);
                shell_faces.push([outer[0], outer[2], outer[1]]);
            }
        }
        for &[a, b] in &patch.boundary {
            let edge = positions[b as usize].as_dvec3() - positions[a as usize].as_dvec3();
            let da = shell_positions[(a + offset) as usize].as_dvec3()
                - positions[a as usize].as_dvec3();
            let db = shell_positions[(b + offset) as usize].as_dvec3()
                - positions[b as usize].as_dvec3();
            let expected = edge.cross(da + db) * f64::from(settings.thickness.signum());
            let side = if settings.thickness > 0.0 {
                [[a, b, b + offset], [a, b + offset, a + offset]]
            } else {
                [[a, b + offset, b], [a, a + offset, b + offset]]
            };
            for triangle in side {
                require_orientation(
                    face_normal(&shell_positions, triangle)?,
                    expected,
                    "boundary side",
                )?;
                shell_faces.push(triangle);
            }
        }
        let shell_topology = topology(shell_positions.len(), &shell_faces)?;
        if !shell_topology.boundary.is_empty() {
            return Err("Extracted shell unexpectedly has open boundary edges".into());
        }
        // The pinned quad reference bridges only part of this masked cube's
        // boundary and leaves an open ten-edge output. Keep the checked shell
        // until the quad bridge/neighbor policy is validated on healthy cases.
        if sculptgl_surface_offset
            && source_topology.boundary.is_empty()
            && !self
                .surface
                .as_ref()
                .is_some_and(|surface| surface.has_quads())
        {
            if let Some(smoothed) = smooth_reference_shell_border(
                &shell_positions,
                &shell_faces,
                &faces,
                &patch.boundary,
                points.len(),
            )? {
                shell_positions = smoothed;
            }
        }
        let mut mesh = Self::new(shell_positions, shell_faces);
        mesh.mask_convention = self.mask_convention;
        mesh.masks.fill(self.mask_convention.clear_value());
        if sculptgl_surface_offset {
            mesh.colors.fill([1.0; 3]);
            mesh.materials = Some(vec![[0.18, 0.08]; output_vertices]);
        } else {
            mesh.materials = self
                .materials
                .as_ref()
                .map(|_| vec![[0.0; 2]; output_vertices]);
            for (i, point) in points.iter().enumerate() {
                mesh.colors[i] = point.color;
                mesh.colors[i + points.len()] = point.color;
                if let (Some(output), Some(material)) = (&mut mesh.materials, point.material) {
                    output[i] = material;
                    output[i + points.len()] = material;
                }
            }
        }
        mesh.validate()?;
        let report = MaskExtractReport {
            input_vertices: self.positions.len(),
            input_triangles: self.triangles.len(),
            selected_source_triangles,
            patch_vertices: points.len(),
            patch_triangles: faces.len(),
            patch_components: patch.components,
            boundary_edges: patch.boundary.len(),
            boundary_loops: patch.boundary_loops,
            output_vertices,
            output_triangles,
            threshold: settings.threshold,
            thickness: settings.thickness,
        };
        Ok((mesh, report))
    }
}

/// SculptGL's source mesh uses the arithmetic mean of f32 raw incident-face
/// normals. A quad contributes one combined face normal, not two triangle
/// normals. The final extraction direction is normalized after f32 storage.
/// This is local to the opt-in extraction mode; ordinary OBrush normals keep
/// their angle-weighted policy.
fn sculptgl_face_averaged_normals(mesh: &Mesh) -> Result<Vec<DVec3>, String> {
    let mut sums = reserved(mesh.positions.len(), "reference source normal sums")?;
    sums.resize(mesh.positions.len(), DVec3::ZERO);
    let mut counts = reserved(mesh.positions.len(), "reference source normal counts")?;
    counts.resize(mesh.positions.len(), 0_u32);

    let mut add_face = |ids: &[u32]| -> Result<(), String> {
        if ids.len() < 3 {
            return Err("Extraction face has fewer than 3 vertices".into());
        }
        let [a, b, c] = [ids[0], ids[1], ids[2]];
        let [pa, pb, pc] = [a, b, c].map(|id| mesh.positions[id as usize].as_dvec3());
        let mut normal = (pb - pa).cross(pc - pa);
        if let Some(&d) = ids.get(3) {
            let pd = mesh.positions[d as usize].as_dvec3();
            normal += (pc - pd).cross(pc - pa);
        }
        if !normal.is_finite() || normal.length_squared() == 0.0 {
            return Err("SculptGL surface offset requires noncollapsed source faces".into());
        }
        let stored_face_normal = normal.as_vec3().as_dvec3();
        for &id in ids {
            sums[id as usize] += stored_face_normal;
            counts[id as usize] += 1;
        }
        Ok(())
    };

    if let Some(surface) = &mesh.surface {
        for face in &surface.faces {
            let arity = if face[3] == crate::TRIANGLE_CORNER {
                3
            } else {
                4
            };
            add_face(&face[..arity])?;
        }
    } else {
        for face in &mesh.triangles {
            add_face(face)?;
        }
    }
    for (normal, &count) in sums.iter_mut().zip(&counts) {
        if count == 0 {
            continue;
        }
        let averaged = (*normal / f64::from(count)).as_vec3().as_dvec3();
        if !averaged.is_finite() || averaged.length_squared() == 0.0 {
            return Err("SculptGL surface offset has a cancelled source normal".into());
        }
        *normal = averaged.normalize().as_vec3().as_dvec3();
    }
    Ok(sums)
}

/// Smooth the logical side-wall quad ring without treating each rendering
/// triangle's side-wall diagonal as a sculpting neighbor. A source with open
/// edges is excluded by the caller: its reference extraction can collapse
/// triangles. If the candidate collapses any triangle, preserve the original
/// checked shell instead.
fn smooth_reference_shell_border(
    positions: &[Vec3],
    shell_faces: &[[u32; 3]],
    patch_faces: &[[u32; 3]],
    boundary: &[[u32; 2]],
    patch_vertices: usize,
) -> Result<Option<Vec<Vec3>>, String> {
    if boundary.is_empty() {
        return Ok(None);
    }
    let shell_vertices = positions.len();
    let edges_limit = patch_faces
        .len()
        .checked_mul(6)
        .and_then(|n| boundary.len().checked_mul(2).and_then(|b| n.checked_add(b)))
        .ok_or("Mask extraction smoothing edge count overflows")?;
    let mut edges = reserved(edges_limit, "extraction smoothing edges")?;
    let offset = patch_vertices as u32;
    for face in patch_faces {
        for corner in 0..3 {
            let a = face[corner];
            let b = face[(corner + 1) % 3];
            edges.push((a.min(b), a.max(b)));
            edges.push(((a + offset).min(b + offset), (a + offset).max(b + offset)));
        }
    }
    let mut selected = reserved(shell_vertices, "extraction smoothing selection")?;
    selected.resize(shell_vertices, false);
    selected[patch_vertices..].fill(true);
    for &[a, b] in boundary {
        for vertex in [a, b] {
            let outer = vertex + offset;
            edges.push((vertex, outer));
            selected[vertex as usize] = true;
        }
    }
    edges.sort_unstable();
    edges.dedup();
    let mut start = reserved(shell_vertices + 1, "extraction smoothing offsets")?;
    start.resize(shell_vertices + 1, 0_usize);
    for &(a, b) in &edges {
        start[a as usize + 1] += 1;
        start[b as usize + 1] += 1;
    }
    for i in 0..shell_vertices {
        start[i + 1] += start[i];
    }
    let mut ring = reserved(start[shell_vertices], "extraction smoothing neighbors")?;
    ring.resize(start[shell_vertices], 0_u32);
    let mut next = reserved(shell_vertices, "extraction smoothing cursors")?;
    next.extend_from_slice(&start[..shell_vertices]);
    for &(a, b) in &edges {
        ring[next[a as usize]] = b;
        next[a as usize] += 1;
        ring[next[b as usize]] = a;
        next[b as usize] += 1;
    }
    let mut current = reserved(shell_vertices, "extraction smoothing positions")?;
    current.extend_from_slice(positions);
    let mut candidate = reserved(shell_vertices, "extraction smoothing candidate")?;
    candidate.extend_from_slice(positions);
    for _ in 0..3 {
        for (i, &included) in selected.iter().enumerate() {
            if !included || start[i + 1] - start[i] <= 2 {
                continue;
            }
            let mut sum = DVec3::ZERO;
            for &neighbor in &ring[start[i]..start[i + 1]] {
                sum += current[neighbor as usize].as_dvec3();
            }
            candidate[i] = (sum / (start[i + 1] - start[i]) as f64).as_vec3();
        }
        std::mem::swap(&mut current, &mut candidate);
    }
    for &face in shell_faces {
        if face_normal(&current, face).is_err() {
            return Ok(None);
        }
    }
    if current
        .iter()
        .any(|p| !p.is_finite() || p.abs().max_element() > 1_000_000.0)
    {
        return Ok(None);
    }
    Ok(Some(current))
}

fn clipped_keys(mesh: &Mesh, ids: [u32; 3], threshold: f32) -> Vec<PointKey> {
    let mut polygon = Vec::with_capacity(4);
    let mut previous = ids[2];
    for next in ids {
        let previous_inside = mesh
            .mask_convention
            .protection_at_least(mesh.masks[previous as usize], threshold);
        let next_inside = mesh
            .mask_convention
            .protection_at_least(mesh.masks[next as usize], threshold);
        if previous_inside != next_inside {
            let cut = if mesh
                .mask_convention
                .protection_equals(mesh.masks[previous as usize], threshold)
            {
                PointKey::Original(previous)
            } else if mesh
                .mask_convention
                .protection_equals(mesh.masks[next as usize], threshold)
            {
                PointKey::Original(next)
            } else {
                PointKey::Edge(previous.min(next), previous.max(next))
            };
            if polygon.last() != Some(&cut) {
                polygon.push(cut);
            }
        }
        if next_inside {
            let point = PointKey::Original(next);
            if polygon.last() != Some(&point) {
                polygon.push(point);
            }
        }
        previous = next;
    }
    if polygon.len() > 1 && polygon.first() == polygon.last() {
        polygon.pop();
    }
    polygon
}

fn clipped_point(mesh: &Mesh, key: PointKey, threshold: f32) -> Result<PatchPoint, String> {
    match key {
        PointKey::Original(v) => Ok(PatchPoint {
            position: mesh.positions[v as usize],
            color: mesh.colors[v as usize],
            material: mesh.materials.as_ref().map(|m| m[v as usize]),
        }),
        PointKey::Edge(a, b) => {
            let a = a as usize;
            let b = b as usize;
            let t = mesh
                .mask_convention
                .edge_cut(mesh.masks[a], mesh.masks[b], threshold);
            let point = mesh.positions[a].as_dvec3()
                + (mesh.positions[b].as_dvec3() - mesh.positions[a].as_dvec3()) * t;
            if !t.is_finite() || !(0.0..=1.0).contains(&t) || !point.is_finite() {
                return Err("Mask extraction has an invalid edge intersection".into());
            }
            Ok(PatchPoint {
                position: point.as_vec3(),
                color: std::array::from_fn(|channel| {
                    (f64::from(mesh.colors[a][channel])
                        + (f64::from(mesh.colors[b][channel]) - f64::from(mesh.colors[a][channel]))
                            * t) as f32
                }),
                material: mesh.materials.as_ref().map(|m| {
                    std::array::from_fn(|channel| {
                        (f64::from(m[a][channel])
                            + (f64::from(m[b][channel]) - f64::from(m[a][channel])) * t)
                            as f32
                    })
                }),
            })
        }
    }
}

fn face_normal(positions: &[Vec3], ids: [u32; 3]) -> Result<DVec3, String> {
    let [a, b, c] = ids.map(|v| positions[v as usize].as_dvec3());
    let normal = (b - a).cross(c - a);
    if !normal.is_finite() || normal.length_squared() == 0.0 {
        return Err("Mask extraction refuses collapsed geometry".into());
    }
    Ok(normal)
}

fn require_orientation(normal: DVec3, expected: DVec3, label: &str) -> Result<(), String> {
    if !normal.is_finite()
        || normal.length_squared() == 0.0
        || !expected.is_finite()
        || expected.length_squared() == 0.0
        || normal.dot(expected) <= 0.0
    {
        return Err(format!(
            "Mask extraction refuses collapsed/flipped {label} geometry"
        ));
    }
    Ok(())
}

fn patch_normals(positions: &[Vec3], triangles: &[[u32; 3]]) -> Result<Vec<DVec3>, String> {
    let mut normals = reserved(positions.len(), "patch normals")?;
    normals.resize(positions.len(), DVec3::ZERO);
    for &ids in triangles {
        let normal = face_normal(positions, ids)?.normalize();
        let points = ids.map(|v| positions[v as usize].as_dvec3());
        for corner in 0..3 {
            let u = points[(corner + 1) % 3] - points[corner];
            let v = points[(corner + 2) % 3] - points[corner];
            let angle = u.cross(v).length().atan2(u.dot(v));
            normals[ids[corner] as usize] += normal * angle;
        }
    }
    for normal in &mut normals {
        if !normal.is_finite() || normal.length_squared() == 0.0 {
            return Err("Selected patch has a zero/cancelled geometric vertex normal".into());
        }
        *normal = normal.normalize();
    }
    for &ids in triangles {
        let face = face_normal(positions, ids)?;
        if ids.iter().any(|&v| normals[v as usize].dot(face) <= 0.0) {
            return Err(
                "Selected patch cannot define outward normals on every incident face".into(),
            );
        }
    }
    Ok(normals)
}

struct Edge {
    start: u32,
    end: u32,
    face: u32,
    corner: u32,
}

impl Edge {
    fn key(&self) -> (u32, u32) {
        (self.start.min(self.end), self.start.max(self.end))
    }
}

struct Topology {
    boundary: Vec<[u32; 2]>,
    boundary_loops: usize,
    components: usize,
}

/// Ordered edge incidence plus connected vertex-corner fans. Work is bounded
/// by face sorting and linear union/find, rather than per-vertex face scans.
fn topology(vertices: usize, triangles: &[[u32; 3]]) -> Result<Topology, String> {
    let count = triangles
        .len()
        .checked_mul(3)
        .ok_or("Extraction topology overflows")?;
    let mut edges = reserved(count, "topology edges")?;
    let mut corners = UnionFind::new(count)?;
    let mut components = UnionFind::new(triangles.len())?;
    for (face, &ids) in triangles.iter().enumerate() {
        for corner in 0..3 {
            edges.push(Edge {
                start: ids[corner],
                end: ids[(corner + 1) % 3],
                face: face as u32,
                corner: corner as u32,
            });
        }
    }
    edges.sort_unstable_by_key(|edge| (edge.key(), edge.face));
    let mut boundary = Vec::new();
    let mut outgoing = reserved(vertices, "boundary outgoing")?;
    outgoing.resize(vertices, u32::MAX);
    let mut incoming = reserved(vertices, "boundary incoming")?;
    incoming.resize(vertices, u32::MAX);
    let mut cursor = 0;
    while cursor < edges.len() {
        let mut end = cursor + 1;
        while end < edges.len() && edges[end].key() == edges[cursor].key() {
            end += 1;
        }
        let edge = &edges[cursor];
        match end - cursor {
            1 => {
                if outgoing[edge.start as usize] != u32::MAX
                    || incoming[edge.end as usize] != u32::MAX
                {
                    return Err("Extraction refuses branching/ambiguous boundary vertices".into());
                }
                outgoing[edge.start as usize] = edge.end;
                incoming[edge.end as usize] = edge.start;
                push_bounded(
                    &mut boundary,
                    [edge.start, edge.end],
                    count,
                    "boundary edges",
                )?;
            }
            2 => {
                let other = &edges[cursor + 1];
                if edge.start != other.end || edge.end != other.start {
                    return Err("Extraction requires consistently oriented manifold edges".into());
                }
                components.join(edge.face, other.face);
                corners.join(
                    edge.face * 3 + edge.corner,
                    other.face * 3 + (other.corner + 1) % 3,
                );
                corners.join(
                    edge.face * 3 + (edge.corner + 1) % 3,
                    other.face * 3 + other.corner,
                );
            }
            _ => {
                return Err("Extraction refuses nonmanifold edges with more than two faces".into());
            }
        }
        cursor = end;
    }
    let mut vertex_fans = reserved(vertices, "vertex fans")?;
    vertex_fans.resize(vertices, u32::MAX);
    for (face, ids) in triangles.iter().enumerate() {
        for (corner, &vertex) in ids.iter().enumerate() {
            let root = corners.root((face * 3 + corner) as u32);
            let previous = &mut vertex_fans[vertex as usize];
            if *previous != u32::MAX && *previous != root {
                return Err("Extraction refuses disconnected vertex surface fans".into());
            }
            *previous = root;
        }
    }
    let mut visited = reserved(vertices, "boundary visits")?;
    visited.resize(vertices, false);
    let mut boundary_loops = 0;
    for &[start, _] in &boundary {
        if visited[start as usize] {
            continue;
        }
        boundary_loops += 1;
        let mut at = start;
        loop {
            if at == u32::MAX || incoming[at as usize] == u32::MAX {
                return Err("Extraction boundary is not a closed directed loop".into());
            }
            if visited[at as usize] {
                if at != start {
                    return Err("Extraction refuses intersecting boundary loops".into());
                }
                break;
            }
            visited[at as usize] = true;
            at = outgoing[at as usize];
        }
    }
    Ok(Topology {
        boundary,
        boundary_loops,
        components: (0..triangles.len())
            .filter(|&face| components.root(face as u32) == face as u32)
            .count(),
    })
}

struct UnionFind {
    parent: Vec<u32>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(count: usize) -> Result<Self, String> {
        let mut parent = reserved(count, "topology union parents")?;
        parent.extend((0..count).map(|v| v as u32));
        let mut rank = reserved(count, "topology union ranks")?;
        rank.resize(count, 0);
        Ok(Self { parent, rank })
    }
    fn root(&mut self, mut vertex: u32) -> u32 {
        while self.parent[vertex as usize] != vertex {
            let next = self.parent[vertex as usize];
            self.parent[vertex as usize] = self.parent[next as usize];
            vertex = next;
        }
        vertex
    }
    fn join(&mut self, a: u32, b: u32) {
        let mut a = self.root(a);
        let mut b = self.root(b);
        if a == b {
            return;
        }
        if self.rank[a as usize] < self.rank[b as usize] {
            std::mem::swap(&mut a, &mut b);
        }
        self.parent[b as usize] = a;
        if self.rank[a as usize] == self.rank[b as usize] {
            self.rank[a as usize] += 1;
        }
    }
}

fn reserved<T>(count: usize, label: &str) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| format!("Cannot allocate extraction {label}"))?;
    Ok(values)
}

fn push_bounded<T>(values: &mut Vec<T>, value: T, limit: usize, label: &str) -> Result<(), String> {
    if values.len() >= limit {
        return Err(format!("Extraction exceeds the {label} limit"));
    }
    if values.len() == values.capacity() {
        let next = (limit - values.len()).min(4096);
        values
            .try_reserve_exact(next)
            .map_err(|_| format!("Cannot allocate extraction {label}"))?;
    }
    values.push(value);
    Ok(())
}
