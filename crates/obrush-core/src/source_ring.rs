//! Reconstruct source-static fine rings from parent topology and UV seams.
//! Chained levels retain source child-face order and cyclic orientation for
//! the next ring while keeping all geometry/history decisions in the core.
use crate::{Mesh, Vec3, TRIANGLE_CORNER};
use glam::DVec3;
use std::collections::HashMap;

/// Source's post-transform face cache window. The UV pseudo-vertices affect
/// face visit only; geometric edge/face children remain welded.
const CACHE_SIZE: usize = 32;

#[derive(Clone, Debug)]
pub struct SourceStaticRingMap {
    /// Parent faces in the order visited by the source's UV-aware cache pass.
    parent_face_visit: Vec<u32>,
    /// Neighbor IDs are in the provided child mesh's physical vertex order.
    first_neighbor: Vec<u32>,
    /// Source child faces in source face order and cyclic orientation, but
    /// expressed with OBrush physical vertex IDs. The next static level uses
    /// these rows directly: MeshResolution does not re-optimize its faces.
    source_child_faces: Vec<[u32; 4]>,
    source_child_host_face_ids: Vec<u32>,
    /// Host face ID and cyclic start corner for each source parent face row.
    source_parent_order: Vec<[u32; 2]>,
}

fn faces(mesh: &Mesh) -> Vec<[u32; 4]> {
    mesh.surface.as_ref().map_or_else(
        || {
            mesh.triangles
                .iter()
                .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
                .collect()
        },
        |surface| surface.faces.clone(),
    )
}

fn arity(face: &[u32; 4]) -> usize {
    if face[3] == TRIANGLE_CORNER {
        3
    } else {
        4
    }
}

fn same_oriented_face(a: &[u32; 4], b: &[u32; 4]) -> bool {
    let n = arity(a);
    n == arity(b) && (0..n).any(|offset| (0..n).all(|corner| a[corner] == b[(corner + offset) % n]))
}

fn child_faces(parent: &[u32; 4], edges: &[u32; 4], center: u32) -> [[u32; 4]; 4] {
    let [a, b, c, d] = *parent;
    let [x, y, z, w] = *edges;
    if d == TRIANGLE_CORNER {
        [
            [x, y, z, TRIANGLE_CORNER],
            [a, x, z, TRIANGLE_CORNER],
            [x, b, y, TRIANGLE_CORNER],
            [y, c, z, TRIANGLE_CORNER],
        ]
    } else {
        [
            [a, x, center, w],
            [x, b, y, center],
            [center, y, c, z],
            [w, center, z, d],
        ]
    }
}

impl SourceStaticRingMap {
    pub fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.parent_face_visit.capacity() * std::mem::size_of::<u32>()
            + self.first_neighbor.capacity() * std::mem::size_of::<u32>()
            + self.source_child_faces.capacity() * std::mem::size_of::<[u32; 4]>()
            + self.source_child_host_face_ids.capacity() * std::mem::size_of::<u32>()
            + self.source_parent_order.capacity() * std::mem::size_of::<[u32; 2]>()
    }

    pub fn parent_face_visit(&self) -> &[u32] {
        &self.parent_face_visit
    }

    pub fn first_neighbor(&self) -> &[u32] {
        &self.first_neighbor
    }

    pub fn source_parent_order(&self) -> &[[u32; 2]] {
        &self.source_parent_order
    }

    pub fn build(parent: &Mesh, child: &Mesh) -> Result<Self, String> {
        parent.validate()?;
        child.validate()?;
        let parent_faces = faces(parent);
        let child_faces_actual = faces(child);
        if child_faces_actual.len() != parent_faces.len() * 4 {
            return Err("Static ring child face count does not match parent".into());
        }
        let vertices = parent.positions.len();
        let mut first_uv = vec![TRIANGLE_CORNER; vertices];
        let mut extra_uv: Vec<Vec<u32>> = vec![Vec::new(); vertices];
        let uv_faces = parent
            .surface
            .as_ref()
            .and_then(|surface| surface.uv.as_ref());
        for (face_id, face) in parent_faces.iter().enumerate() {
            for (corner, &vertex) in face[..arity(face)].iter().enumerate() {
                let uv = uv_faces.map_or(0, |data| data.face_indices[face_id][corner]);
                let first = &mut first_uv[vertex as usize];
                if *first == TRIANGLE_CORNER {
                    *first = uv;
                } else if *first != uv && !extra_uv[vertex as usize].contains(&uv) {
                    extra_uv[vertex as usize]
                        .try_reserve(1)
                        .map_err(|_| "Cannot allocate source ring seam IDs")?;
                    extra_uv[vertex as usize].push(uv);
                }
            }
        }
        if first_uv.contains(&TRIANGLE_CORNER) {
            return Err("Static ring rejects isolated parent vertices".into());
        }
        let mut extra_start = Vec::new();
        extra_start
            .try_reserve_exact(vertices)
            .map_err(|_| "Cannot allocate source ring seam offsets")?;
        let mut pseudo_count = vertices;
        for row in &extra_uv {
            extra_start.push(pseudo_count);
            pseudo_count = pseudo_count
                .checked_add(row.len())
                .ok_or("Static ring seam ID overflow")?;
        }
        // A pseudo-vertex cannot outnumber the bounded input's face corners.
        if pseudo_count > vertices + parent_faces.len() * 4 {
            return Err("Static ring seam allocation limit exceeded".into());
        }
        let mut owner = Vec::new();
        owner
            .try_reserve_exact(pseudo_count)
            .map_err(|_| "Cannot allocate source ring owners")?;
        owner.extend((0..vertices).map(|id| id as u32));
        for (vertex, row) in extra_uv.iter().enumerate() {
            owner.extend(std::iter::repeat_n(vertex as u32, row.len()));
        }
        let mut pseudo_faces = Vec::new();
        pseudo_faces
            .try_reserve_exact(parent_faces.len())
            .map_err(|_| "Cannot allocate source ring pseudo faces")?;
        let mut counts = vec![0usize; vertices];
        for (face_id, face) in parent_faces.iter().enumerate() {
            let mut pseudo = [TRIANGLE_CORNER; 4];
            for corner in 0..arity(face) {
                let vertex = face[corner] as usize;
                counts[vertex] += 1;
                let uv = uv_faces.map_or(0, |data| data.face_indices[face_id][corner]);
                pseudo[corner] = if uv == first_uv[vertex] {
                    vertex as u32
                } else {
                    let index = extra_uv[vertex]
                        .iter()
                        .position(|&item| item == uv)
                        .ok_or("Static ring seam ID missing")?;
                    (extra_start[vertex] + index) as u32
                };
            }
            pseudo_faces.push(pseudo);
        }
        let mut offsets = Vec::new();
        offsets
            .try_reserve_exact(vertices + 1)
            .map_err(|_| "Cannot allocate source ring incidence offsets")?;
        offsets.push(0usize);
        for count in &counts {
            offsets.push(offsets.last().copied().unwrap() + count);
        }
        let mut incidence = vec![0u32; *offsets.last().unwrap()];
        let mut cursor = offsets[..vertices].to_vec();
        for (face_id, face) in parent_faces.iter().enumerate() {
            for &vertex in &face[..arity(face)] {
                let index = &mut cursor[vertex as usize];
                incidence[*index] = face_id as u32;
                *index += 1;
            }
        }
        let mut live: Vec<usize> = owner.iter().map(|&id| counts[id as usize]).collect();
        let mut timestamp = vec![0usize; pseudo_count];
        let mut emitted = vec![false; parent_faces.len()];
        let mut dead_end = Vec::new();
        dead_end
            .try_reserve_exact(incidence.len())
            .map_err(|_| "Cannot allocate source ring cache stack")?;
        let mut visit = Vec::new();
        visit
            .try_reserve_exact(parent_faces.len())
            .map_err(|_| "Cannot allocate source ring face visit")?;
        let mut fan = Some(0usize);
        let mut clock = CACHE_SIZE + 1;
        let mut scan = 0usize;
        while let Some(current) = fan {
            let original = owner[current] as usize;
            let mut candidates = Vec::new();
            for &face_id in incidence[offsets[original]..offsets[original + 1]]
                .iter()
                .rev()
            {
                let face_id = face_id as usize;
                if emitted[face_id] {
                    continue;
                }
                emitted[face_id] = true;
                visit.push(face_id as u32);
                let pseudo = &pseudo_faces[face_id];
                candidates
                    .try_reserve(arity(pseudo))
                    .map_err(|_| "Cannot allocate source ring face candidates")?;
                for &id in &pseudo[..arity(pseudo)] {
                    let id = id as usize;
                    dead_end.push(id);
                    candidates.push(id);
                    live[id] = live[id].saturating_sub(1);
                    if clock - timestamp[id] > CACHE_SIZE {
                        timestamp[id] = clock;
                        clock += 1;
                    }
                }
            }
            fan = None;
            let mut best = None;
            for vertex in candidates {
                let remaining = live[vertex];
                if remaining == 0 {
                    continue;
                }
                let age = clock - timestamp[vertex];
                let priority = if age + 2 * remaining <= CACHE_SIZE {
                    age
                } else {
                    0
                };
                if best.is_none_or(|prior| priority > prior) {
                    fan = Some(vertex);
                    best = Some(priority);
                }
            }
            if fan.is_some() {
                continue;
            }
            while let Some(vertex) = dead_end.pop() {
                if live[vertex] > 0 {
                    fan = Some(vertex);
                    break;
                }
            }
            if fan.is_some() {
                continue;
            }
            while scan < pseudo_count {
                let vertex = scan;
                scan += 1;
                if live[vertex] > 0 {
                    fan = Some(vertex);
                    break;
                }
            }
        }
        if visit.len() != parent_faces.len() {
            return Err("Static ring face visit did not cover the mesh".into());
        }

        // OBrush's child ID allocation follows the original parent face order.
        // Reuse those IDs while replaying the reference's optimized face order.
        let mut edge_ids = HashMap::new();
        edge_ids
            .try_reserve(parent_faces.len() * 2)
            .map_err(|_| "Cannot allocate source ring edge IDs")?;
        let mut face_edges = Vec::new();
        let mut centers = Vec::new();
        face_edges
            .try_reserve_exact(parent_faces.len())
            .map_err(|_| "Cannot allocate source ring face edges")?;
        centers
            .try_reserve_exact(parent_faces.len())
            .map_err(|_| "Cannot allocate source ring face centers")?;
        let mut next_id = vertices;
        for face in &parent_faces {
            let mut edges = [TRIANGLE_CORNER; 4];
            for corner in 0..arity(face) {
                let a = face[corner];
                let b = face[(corner + 1) % arity(face)];
                let key = (a.min(b), a.max(b));
                edges[corner] = *edge_ids.entry(key).or_insert_with(|| {
                    let id = next_id as u32;
                    next_id += 1;
                    id
                });
            }
            face_edges.push(edges);
            let center = if arity(face) == 4 {
                let id = next_id as u32;
                next_id += 1;
                id
            } else {
                TRIANGLE_CORNER
            };
            centers.push(center);
        }
        if child.positions.len() != next_id {
            return Err("Static ring child vertex count does not match parent edges".into());
        }
        for (face_id, face) in parent_faces.iter().enumerate() {
            let expected = child_faces(face, &face_edges[face_id], centers[face_id]);
            for (quarter, row) in expected.iter().enumerate() {
                if !same_oriented_face(row, &child_faces_actual[face_id * 4 + quarter]) {
                    return Err("Static ring child topology does not match parent".into());
                }
            }
        }
        let mut first_neighbor = vec![TRIANGLE_CORNER; next_id];
        let mut source_child_faces = Vec::new();
        let mut source_child_host_face_ids = Vec::new();
        source_child_faces
            .try_reserve_exact(child_faces_actual.len())
            .map_err(|_| "Cannot allocate ordered static child faces")?;
        source_child_host_face_ids
            .try_reserve_exact(child_faces_actual.len())
            .map_err(|_| "Cannot allocate ordered static child face IDs")?;
        for &face_id in &visit {
            let face_id = face_id as usize;
            let children = child_faces(
                &parent_faces[face_id],
                &face_edges[face_id],
                centers[face_id],
            );
            for (quarter, child_face) in children.iter().enumerate() {
                source_child_faces.push(*child_face);
                source_child_host_face_ids.push(face_id as u32 * 4 + quarter as u32);
                let n = arity(child_face);
                for corner in 0..n {
                    let neighbor = match corner {
                        0 => child_face[n - 1],
                        1 => child_face[0],
                        2 if n == 4 => child_face[3],
                        _ => child_face[0],
                    };
                    first_neighbor[child_face[corner] as usize] = neighbor;
                }
            }
        }
        if first_neighbor.contains(&TRIANGLE_CORNER) {
            return Err("Static ring child has an isolated vertex".into());
        }
        let mut source_parent_order = Vec::new();
        source_parent_order
            .try_reserve_exact(visit.len())
            .map_err(|_| "Cannot allocate source parent face order")?;
        source_parent_order.extend(visit.iter().map(|&face| [face, 0]));
        Ok(Self {
            source_parent_order,
            parent_face_visit: visit,
            first_neighbor,
            source_child_faces,
            source_child_host_face_ids,
        })
    }

    /// A source-generated parent level is already in the previous map's face
    /// order. Triangle child rows can be cyclically rotated relative to the
    /// OBrush row even though the oriented surface is identical. Preserve the
    /// source row orientation before constructing the next vertex ring.
    pub fn build_after(parent: &Mesh, child: &Mesh, previous: &Self) -> Result<Self, String> {
        let parent_faces = faces(parent);
        if previous.source_child_faces.len() != parent_faces.len()
            || previous.source_child_host_face_ids.len() != parent_faces.len()
        {
            return Err("Static source parent face count does not match the retained map".into());
        }
        let mut order = Vec::new();
        order
            .try_reserve_exact(parent_faces.len())
            .map_err(|_| "Cannot allocate chained static face order")?;
        for (source_face, &host_id) in previous
            .source_child_faces
            .iter()
            .zip(&previous.source_child_host_face_ids)
        {
            let host = parent_faces
                .get(host_id as usize)
                .ok_or("Static source parent face ID is invalid")?;
            let n = arity(host);
            let rotation = (0..n)
                .find(|&offset| {
                    (0..n).all(|corner| source_face[corner] == host[(corner + offset) % n])
                })
                .ok_or("Static source parent face orientation is invalid")?;
            order.push([host_id, rotation as u32]);
        }
        Self::build_seeded(parent, child, &order)
    }

    /// Rebuild a saved hierarchy whose original lower level was deleted.
    /// The seed is validated against the installed parent before any transport.
    pub fn build_seeded(parent: &Mesh, child: &Mesh, order: &[[u32; 2]]) -> Result<Self, String> {
        let mut map = Self::build(parent, child)?;
        let parent_faces = faces(parent);
        let child_faces_actual = faces(child);
        if order.len() != parent_faces.len() {
            return Err("Static source parent face count does not match the retained map".into());
        }
        let mut seen = vec![false; parent_faces.len()];
        for &[host_id, rotation] in order {
            let id = host_id as usize;
            if id >= parent_faces.len() || seen[id] || rotation as usize >= arity(&parent_faces[id])
            {
                return Err("Static source parent face order is invalid".into());
            }
            seen[id] = true;
        }
        let mut edge_ids = HashMap::new();
        edge_ids
            .try_reserve(parent_faces.len() * 2)
            .map_err(|_| "Cannot allocate chained static edge IDs")?;
        let mut centers = Vec::new();
        centers
            .try_reserve_exact(parent_faces.len())
            .map_err(|_| "Cannot allocate chained static centers")?;
        let mut next_id = parent.positions.len();
        for face in &parent_faces {
            for corner in 0..arity(face) {
                let a = face[corner];
                let b = face[(corner + 1) % arity(face)];
                let key = (a.min(b), a.max(b));
                edge_ids.entry(key).or_insert_with(|| {
                    let id = next_id as u32;
                    next_id += 1;
                    id
                });
            }
            centers.push(if arity(face) == 4 {
                let id = next_id as u32;
                next_id += 1;
                id
            } else {
                TRIANGLE_CORNER
            });
        }
        if next_id != child.positions.len() {
            return Err("Chained static child vertex count does not match parent".into());
        }
        let mut source_child_faces = Vec::new();
        let mut source_child_host_face_ids = Vec::new();
        source_child_faces
            .try_reserve_exact(parent_faces.len() * 4)
            .map_err(|_| "Cannot allocate chained static child faces")?;
        source_child_host_face_ids
            .try_reserve_exact(parent_faces.len() * 4)
            .map_err(|_| "Cannot allocate chained static child face IDs")?;
        let mut first_neighbor = vec![TRIANGLE_CORNER; child.positions.len()];
        for &[parent_face_id, rotation] in order {
            let host = parent_faces[parent_face_id as usize];
            let n = arity(&host);
            let mut source_face = [TRIANGLE_CORNER; 4];
            for corner in 0..n {
                source_face[corner] = host[(corner + rotation as usize) % n];
            }
            let mut edges = [TRIANGLE_CORNER; 4];
            for corner in 0..n {
                let a = source_face[corner];
                let b = source_face[(corner + 1) % n];
                edges[corner] = *edge_ids
                    .get(&(a.min(b), a.max(b)))
                    .ok_or("Chained static source edge is absent")?;
            }
            let children = child_faces(&source_face, &edges, centers[parent_face_id as usize]);
            let mut matched_quarters = [false; 4];
            for child_face in &children {
                let quarter = (0..4)
                    .find(|&quarter| {
                        !matched_quarters[quarter]
                            && same_oriented_face(
                                child_face,
                                &child_faces_actual[parent_face_id as usize * 4 + quarter],
                            )
                    })
                    .ok_or("Chained static source child orientation is invalid")?;
                matched_quarters[quarter] = true;
                let host_id = parent_face_id as usize * 4 + quarter;
                source_child_faces.push(*child_face);
                source_child_host_face_ids.push(host_id as u32);
                let n = arity(child_face);
                for corner in 0..n {
                    let neighbor = match corner {
                        0 => child_face[n - 1],
                        1 => child_face[0],
                        2 if n == 4 => child_face[3],
                        _ => child_face[0],
                    };
                    first_neighbor[child_face[corner] as usize] = neighbor;
                }
            }
        }
        if first_neighbor.contains(&TRIANGLE_CORNER) {
            return Err("Chained static child has an isolated vertex".into());
        }
        let mut parent_face_visit = Vec::new();
        let mut source_parent_order = Vec::new();
        parent_face_visit
            .try_reserve_exact(order.len())
            .map_err(|_| "Cannot allocate source parent face visit")?;
        source_parent_order
            .try_reserve_exact(order.len())
            .map_err(|_| "Cannot allocate source parent face order")?;
        parent_face_visit.extend(order.iter().map(|row| row[0]));
        source_parent_order.extend_from_slice(order);
        map.parent_face_visit = parent_face_visit;
        map.source_parent_order = source_parent_order;
        map.first_neighbor = first_neighbor;
        map.source_child_faces = source_child_faces;
        map.source_child_host_face_ids = source_child_host_face_ids;
        Ok(map)
    }

    /// Transport retained fine detail through a changed parent prediction
    /// using the source's raw face-averaged normal and first-ring tangent.
    /// This is a pure calculation: callers own document/history transactions.
    pub fn transport_positions(
        &self,
        old_child: &Mesh,
        old_prediction: &Mesh,
        new_prediction: &Mesh,
    ) -> Result<Vec<Vec3>, String> {
        old_child.validate()?;
        old_prediction.validate()?;
        new_prediction.validate()?;
        let count = old_child.positions.len();
        if self.first_neighbor.len() != count
            || old_prediction.positions.len() != count
            || new_prediction.positions.len() != count
            || old_child.triangles != old_prediction.triangles
            || old_child.triangles != new_prediction.triangles
            || !old_child.surface_matches_bits(old_prediction)
            || !old_child.surface_matches_bits(new_prediction)
        {
            return Err("Static ring transport requires matching child topology".into());
        }
        let face_rows = faces(old_child);
        let mut normal_sums = Vec::new();
        normal_sums
            .try_reserve_exact(count)
            .map_err(|_| "Cannot allocate source ring normals")?;
        normal_sums.resize(count, DVec3::ZERO);
        let point = |vertex: u32| {
            let p = old_child.positions[vertex as usize];
            DVec3::new(p.x as f64, p.y as f64, p.z as f64)
        };
        for face in &face_rows {
            let a = point(face[0]);
            let b = point(face[1]);
            let c = point(face[2]);
            let mut area = (b - a).cross(c - a);
            if arity(face) == 4 {
                let d = point(face[3]);
                area += (c - d).cross(c - a);
            }
            for &vertex in &face[..arity(face)] {
                normal_sums[vertex as usize] += area;
            }
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(count)
            .map_err(|_| "Cannot allocate static ring transported positions")?;
        let read = |positions: &[Vec3], vertex: usize| {
            let p = positions[vertex];
            DVec3::new(p.x as f64, p.y as f64, p.z as f64)
        };
        for (vertex, &normal_sum) in normal_sums.iter().enumerate() {
            let neighbor = self.first_neighbor[vertex] as usize;
            if neighbor >= count || neighbor == vertex {
                return Err("Static ring first neighbor is invalid".into());
            }
            if old_prediction.positions[vertex] == new_prediction.positions[vertex]
                && old_prediction.positions[neighbor] == new_prediction.positions[neighbor]
            {
                output.push(old_child.positions[vertex]);
                continue;
            }
            let normal = normal_sum.normalize_or_zero();
            let old_base = read(&old_prediction.positions, vertex);
            let new_base = read(&new_prediction.positions, vertex);
            let residual = read(&old_child.positions, vertex) - old_base;
            let old_edge = read(&old_prediction.positions, neighbor) - old_base;
            let new_edge = read(&new_prediction.positions, neighbor) - new_base;
            let old_tangent = (old_edge - normal * old_edge.dot(normal)).normalize_or_zero();
            let new_tangent = (new_edge - normal * new_edge.dot(normal)).normalize_or_zero();
            let old_binormal = normal.cross(old_tangent);
            let new_binormal = normal.cross(new_tangent);
            let position = new_base
                + normal * normal.dot(residual)
                + new_tangent * old_tangent.dot(residual)
                + new_binormal * old_binormal.dot(residual);
            let position = Vec3::new(position.x as f32, position.y as f32, position.z as f32);
            if !position.is_finite() {
                return Err("Static ring transport produced a nonfinite point".into());
            }
            output.push(position);
        }
        Ok(output)
    }
}
