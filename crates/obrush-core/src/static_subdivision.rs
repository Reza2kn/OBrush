//! Independent source-static triangle/quad subdivision. Legacy cosine Loop is
//! separate. A quad's render diagonal never participates in the control graph.
use crate::{
    CornerUv, Mesh, SurfaceTopology, Vec3, MAX_TRIANGLES, MAX_UV_COORDINATES, MAX_VERTICES,
    TRIANGLE_CORNER,
};
use std::collections::{HashMap, HashSet};

pub const MAX_STATIC_MAP_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StaticSubdivisionSettings {
    /// Midpoint odd edges; pinned boundary-even handling still applies.
    pub linear_edges: bool,
}

#[derive(Clone, Debug)]
pub struct StaticSubdivisionReport {
    pub input_vertices: usize,
    pub input_faces: usize,
    pub input_quads: usize,
    pub true_edges: usize,
    pub nonmanifold_edges: usize,
    pub output_vertices: usize,
    pub output_faces: usize,
    pub output_triangles: usize,
    pub output_uv_coordinates: usize,
    pub map_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct StaticSubdivisionOutcome {
    pub mesh: Mesh,
    pub forward_map: StaticSubdivisionMap,
    pub report: StaticSubdivisionReport,
}

#[derive(Clone, Copy, Debug)]
struct Corner {
    face: u32,
    corner: u32,
}
#[derive(Clone, Debug)]
struct Edge {
    vertices: [u32; 2],
    first: Corner,
    second: Corner,
    count: u32,
    output: u32,
}

/// Bounded sparse topology/origin map, regenerated rather than encoded as a
/// large floating weight matrix. Geometric edits may reuse it; UV/topology or
/// channel-presence edits require a new map. Even IDs are the original prefix.
#[derive(Clone, Debug)]
pub struct StaticSubdivisionMap {
    settings: StaticSubdivisionSettings,
    vertices: usize,
    faces: Vec<[u32; 4]>,
    face_edges: Vec<[u32; 4]>,
    centers: Vec<u32>,
    edges: Vec<Edge>,
    incident_offsets: Vec<u32>,
    incidents: Vec<Corner>,
    neighbor_offsets: Vec<u32>,
    neighbors: Vec<u32>,
    original_surface: bool,
    materials: bool,
    mask_convention: crate::MaskConvention,
    uv: Option<CornerUv>,
    output_vertices: usize,
    output_triangles: usize,
}

impl Mesh {
    pub fn subdivided_static(
        &self,
        settings: &StaticSubdivisionSettings,
    ) -> Result<StaticSubdivisionOutcome, String> {
        let forward_map = StaticSubdivisionMap::build(self, *settings)?;
        let mesh = forward_map.apply(self)?;
        let report = forward_map.report(&mesh);
        Ok(StaticSubdivisionOutcome {
            mesh,
            forward_map,
            report,
        })
    }
}

/// Reorder a retained static child after its parent is renumbered by Reverse.
/// The returned map relates each new child vertex to the old child vertex
/// carrying its sculpt/paint data. The caller keeps this mapping for the next
/// level; no coordinate-based matching is used, so overlapping points remain
/// unambiguous.
pub(crate) fn rebase_static_child(
    old_parent: &Mesh,
    old_child: &Mesh,
    new_parent: &Mesh,
    parent_to_old: &[u32],
) -> Result<(Mesh, Vec<u32>), String> {
    let parent_uv = old_parent.surface.as_ref().and_then(|s| s.uv.as_ref());
    let child_uv = old_child.surface.as_ref().and_then(|s| s.uv.as_ref());
    let rebased_parent_uv = new_parent.surface.as_ref().and_then(|s| s.uv.as_ref());
    if parent_uv.is_some() != child_uv.is_some()
        || parent_uv.is_some() != rebased_parent_uv.is_some()
    {
        return Err("Repeated Reverse parent/child UV channel presence differs".into());
    }
    if parent_to_old.len() != new_parent.positions.len()
        || old_parent.positions.len() != new_parent.positions.len()
        || old_child.materials.is_some() != new_parent.materials.is_some()
    {
        return Err("Repeated Reverse parent/channel correspondence changed size".into());
    }
    let settings = StaticSubdivisionSettings {
        linear_edges: false,
    };
    let old_map = StaticSubdivisionMap::build(old_parent, settings)?;
    let new_map = StaticSubdivisionMap::build(new_parent, settings)?;
    let old_prediction = old_map.apply(old_parent)?;
    let old_surface_matches = match (&old_prediction.surface, &old_child.surface) {
        (Some(expected), Some(actual)) if expected.faces == actual.faces => {
            match (&expected.uv, &actual.uv) {
                (Some(expected_uv), Some(actual_uv)) => {
                    crate::attributes::uv_corner_refinement_matches(expected_uv, actual_uv)
                }
                (None, None) => true,
                _ => false,
            }
        }
        (None, None) => true,
        _ => false,
    };
    if old_prediction.triangles != old_child.triangles
        || !old_surface_matches
        || old_map.output_vertices != old_child.positions.len()
        || new_map.output_vertices != old_child.positions.len()
    {
        return Err("Repeated Reverse child topology no longer matches its retained parent".into());
    }
    let mut seen_parent = vec![false; old_parent.positions.len()];
    for &old in parent_to_old {
        let Some(seen) = seen_parent.get_mut(old as usize) else {
            return Err("Repeated Reverse parent mapping is out of range".into());
        };
        if *seen {
            return Err("Repeated Reverse parent mapping is not bijective".into());
        }
        *seen = true;
    }
    if seen_parent.contains(&false) {
        return Err("Repeated Reverse parent mapping omits vertices".into());
    }
    let mut old_edges = HashMap::new();
    old_edges
        .try_reserve(old_map.edges.len())
        .map_err(|_| "Cannot allocate repeated Reverse edge correspondence")?;
    for edge in &old_map.edges {
        let [a, b] = edge.vertices;
        old_edges.insert((a.min(b), a.max(b)), edge.output);
    }
    let mut old_faces = HashMap::new();
    old_faces
        .try_reserve(old_map.faces.len())
        .map_err(|_| "Cannot allocate repeated Reverse face correspondence")?;
    for (id, face) in old_map.faces.iter().enumerate() {
        if old_faces.insert(cyclic_face_key(*face), id).is_some() {
            return Err("Repeated Reverse parent has duplicate oriented faces".into());
        }
    }
    let mut vertex_to_old = vec![TRIANGLE_CORNER; new_map.output_vertices];
    vertex_to_old[..parent_to_old.len()].copy_from_slice(parent_to_old);
    for edge in &new_map.edges {
        let [a, b] = edge.vertices.map(|id| parent_to_old[id as usize]);
        vertex_to_old[edge.output as usize] = *old_edges
            .get(&(a.min(b), a.max(b)))
            .ok_or("Repeated Reverse edge correspondence is missing")?;
    }
    for (id, &center) in new_map.centers.iter().enumerate() {
        if center == TRIANGLE_CORNER {
            continue;
        }
        let mut mapped = new_map.faces[id];
        let count = arity(&mapped);
        for vertex in mapped.iter_mut().take(count) {
            *vertex = parent_to_old[*vertex as usize];
        }
        let old_face = *old_faces
            .get(&cyclic_face_key(mapped))
            .ok_or("Repeated Reverse face correspondence is missing")?;
        vertex_to_old[center as usize] = old_map.centers[old_face];
    }
    let mut seen = HashSet::new();
    seen.try_reserve(vertex_to_old.len())
        .map_err(|_| "Cannot allocate repeated Reverse vertex validation")?;
    for &old in &vertex_to_old {
        if old as usize >= old_child.positions.len() || !seen.insert(old) {
            return Err("Repeated Reverse child mapping is not bijective".into());
        }
    }
    let mut child = new_map.apply(new_parent)?;
    child.positions = vertex_to_old
        .iter()
        .map(|&id| old_child.positions[id as usize])
        .collect();
    child.normals = vertex_to_old
        .iter()
        .map(|&id| old_child.normals[id as usize])
        .collect();
    child.colors = vertex_to_old
        .iter()
        .map(|&id| old_child.colors[id as usize])
        .collect();
    child.masks = vertex_to_old
        .iter()
        .map(|&id| old_child.masks[id as usize])
        .collect();
    child.materials = old_child.materials.as_ref().map(|materials| {
        vertex_to_old
            .iter()
            .map(|&id| materials[id as usize])
            .collect()
    });
    child.mask_convention = old_child.mask_convention;
    let mut old_faces = HashMap::new();
    old_faces
        .try_reserve(old_child.surface.as_ref().map_or(0, |s| s.faces.len()))
        .map_err(|_| "Cannot allocate repeated Reverse child face correspondence")?;
    for (id, face) in old_child
        .surface
        .as_ref()
        .ok_or("Repeated Reverse child has no control faces")?
        .faces
        .iter()
        .enumerate()
    {
        if old_faces.insert(cyclic_face_key(*face), id).is_some() {
            return Err("Repeated Reverse child has duplicate oriented faces".into());
        }
    }
    let new_faces = &child
        .surface
        .as_ref()
        .ok_or("Repeated Reverse rebased child has no control faces")?
        .faces;
    if old_faces.len() != new_faces.len() {
        return Err("Repeated Reverse child face correspondence changed orientation".into());
    }
    let mut remapped_uv_faces = Vec::new();
    if child_uv.is_some() {
        remapped_uv_faces
            .try_reserve_exact(new_faces.len())
            .map_err(|_| "Cannot allocate repeated Reverse UV faces")?;
    }
    for face in new_faces {
        let mut mapped = *face;
        let n = arity(face);
        for vertex in mapped.iter_mut().take(n) {
            *vertex = vertex_to_old[*vertex as usize];
        }
        let old_id = *old_faces
            .get(&cyclic_face_key(mapped))
            .ok_or("Repeated Reverse child face correspondence changed orientation")?;
        if let Some(old_uv) = child_uv {
            let old_face = old_child.surface.as_ref().unwrap().faces[old_id];
            let mut row = [TRIANGLE_CORNER; 4];
            let Some(rotation) = (0..n).find(|&offset| {
                (0..n).all(|corner| mapped[corner] == old_face[(corner + offset) % n])
            }) else {
                return Err("Repeated Reverse UV face rotation changed winding".into());
            };
            for (corner, id) in row.iter_mut().enumerate().take(n) {
                *id = old_uv.face_indices[old_id][(corner + rotation) % n];
            }
            remapped_uv_faces.push(row);
        }
    }
    if let Some(old_uv) = child_uv {
        let retained = crate::CornerUv {
            coordinates: old_uv.coordinates.clone(),
            face_indices: remapped_uv_faces,
        };
        let predicted = child
            .surface
            .as_ref()
            .and_then(|surface| surface.uv.as_ref())
            .ok_or("Repeated Reverse predicted UVs are missing")?;
        if !crate::attributes::uv_corner_refinement_matches(predicted, &retained) {
            return Err("Repeated Reverse fine UV values or aliases cannot be retained; source kept unchanged".into());
        }
        child.surface.as_mut().unwrap().uv = Some(retained);
    }
    child.validate()?;
    Ok((child, vertex_to_old))
}

fn cyclic_face_key(face: [u32; 4]) -> [u32; 4] {
    let n = arity(&face);
    let mut best = [TRIANGLE_CORNER; 4];
    for offset in 0..n {
        let mut candidate = [TRIANGLE_CORNER; 4];
        for corner in 0..n {
            candidate[corner] = face[(corner + offset) % n];
        }
        if offset == 0 || candidate < best {
            best = candidate;
        }
    }
    best
}

impl StaticSubdivisionMap {
    pub fn build(mesh: &Mesh, settings: StaticSubdivisionSettings) -> Result<Self, String> {
        mesh.validate()?;
        validate_areas(mesh, "source")?;
        let output_triangles = mesh
            .triangles
            .len()
            .checked_mul(4)
            .ok_or("Static triangle count overflow")?;
        if output_triangles > MAX_TRIANGLES || mesh.positions.len() > MAX_VERTICES - 3 {
            return Err("Static subdivision exceeds point/triangle limits".into());
        }
        let faces = match &mesh.surface {
            Some(surface) => surface.faces.clone(),
            None => mesh
                .triangles
                .iter()
                .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
                .collect(),
        };
        let mut edges: Vec<Edge> = Vec::new();
        let mut lookup = HashMap::<(u32, u32), u32>::new();
        let mut face_edges = reserved(faces.len(), "face edges")?;
        let mut centers = reserved(faces.len(), "quad centers")?;
        let mut counts = vec![0usize; mesh.positions.len()];
        let mut output_vertices = mesh.positions.len();
        for (face_id, face) in faces.iter().enumerate() {
            let arity = arity(face);
            let mut fe = [TRIANGLE_CORNER; 4];
            for corner in 0..arity {
                let a = face[corner];
                let b = face[(corner + 1) % arity];
                counts[a as usize] += 1;
                let key = (a.min(b), a.max(b));
                let incident = Corner {
                    face: face_id as u32,
                    corner: corner as u32,
                };
                fe[corner] = match lookup.get(&key) {
                    Some(&id) => {
                        let edge = &mut edges[id as usize];
                        if edge.count == 1 {
                            edge.second = incident;
                        }
                        edge.count += 1;
                        id
                    }
                    None => {
                        if output_vertices >= MAX_VERTICES {
                            return Err("Static subdivision exceeds point limit".into());
                        }
                        let id = edges.len() as u32;
                        edges
                            .try_reserve(1)
                            .map_err(|_| "Cannot allocate static edges")?;
                        lookup
                            .try_reserve(1)
                            .map_err(|_| "Cannot allocate static edge index")?;
                        edges.push(Edge {
                            vertices: [a, b],
                            first: incident,
                            second: incident,
                            count: 1,
                            output: output_vertices as u32,
                        });
                        output_vertices += 1;
                        lookup.insert(key, id);
                        id
                    }
                };
            }
            face_edges.push(fe);
            if arity == 4 {
                if output_vertices >= MAX_VERTICES {
                    return Err("Static subdivision exceeds point limit".into());
                }
                centers.push(output_vertices as u32);
                output_vertices += 1;
            } else {
                centers.push(TRIANGLE_CORNER);
            }
        }
        drop(lookup);
        if counts.contains(&0) {
            return Err("Static subdivision rejects isolated control vertices".into());
        }
        let mut incident_offsets = reserved(counts.len() + 1, "incident offsets")?;
        incident_offsets.push(0u32);
        for &count in &counts {
            incident_offsets.push(incident_offsets.last().unwrap() + count as u32);
        }
        let mut incidents =
            vec![Corner { face: 0, corner: 0 }; *incident_offsets.last().unwrap() as usize];
        let mut cursors = incident_offsets[..counts.len()].to_vec();
        for (face_id, face) in faces.iter().enumerate() {
            for (corner, &vertex) in face[..arity(face)].iter().enumerate() {
                incidents[cursors[vertex as usize] as usize] = Corner {
                    face: face_id as u32,
                    corner: corner as u32,
                };
                cursors[vertex as usize] += 1;
            }
        }
        drop(cursors);
        drop(counts);
        let mut neighbor_offsets = reserved(mesh.positions.len() + 1, "neighbor offsets")?;
        let mut neighbors = reserved(edges.len() * 2, "neighbor indices")?;
        let mut tags = vec![0u32; mesh.positions.len()];
        neighbor_offsets.push(0);
        for vertex in 0..mesh.positions.len() {
            for incident in
                &incidents[incident_offsets[vertex] as usize..incident_offsets[vertex + 1] as usize]
            {
                let face = &faces[incident.face as usize];
                let n = arity(face);
                let corner = incident.corner as usize;
                let prev = face[(corner + n - 1) % n];
                let next = face[(corner + 1) % n];
                // Match source initVertexRings order, including its tri corner2.
                let ordered = if corner < 2 {
                    [prev, next]
                } else {
                    [next, prev]
                };
                for neighbor in ordered {
                    if tags[neighbor as usize] != (vertex + 1) as u32 {
                        tags[neighbor as usize] = (vertex + 1) as u32;
                        neighbors.push(neighbor);
                    }
                }
            }
            neighbor_offsets.push(neighbors.len() as u32);
        }
        let uv = mesh.surface.as_ref().and_then(|s| s.uv.as_ref());
        let mut map = Self {
            settings,
            vertices: mesh.positions.len(),
            faces,
            face_edges,
            centers,
            edges,
            incident_offsets,
            incidents,
            neighbor_offsets,
            neighbors,
            original_surface: mesh.surface.is_some(),
            materials: mesh.materials.is_some(),
            mask_convention: mesh.mask_convention,
            uv: None,
            output_vertices,
            output_triangles,
        };
        let uv_bytes = uv.map_or(0, |u| u.coordinates.len() * 8 + u.face_indices.len() * 16);
        if map
            .approximate_bytes()
            .checked_add(uv_bytes)
            .is_none_or(|b| b > MAX_STATIC_MAP_BYTES)
        {
            return Err("Static forward map exceeds 64 MiB".into());
        }
        map.uv = uv.cloned();
        Ok(map)
    }

    pub fn settings(&self) -> StaticSubdivisionSettings {
        self.settings
    }
    pub fn parent_vertex_count(&self) -> usize {
        self.vertices
    }
    pub fn child_vertex_count(&self) -> usize {
        self.output_vertices
    }
    pub fn parent_face_count(&self) -> usize {
        self.faces.len()
    }
    pub fn child_faces_for_parent(&self, parent: u32) -> Option<[u32; 4]> {
        ((parent as usize) < self.faces.len())
            .then(|| std::array::from_fn(|c| parent * 4 + c as u32))
    }

    pub fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.faces.capacity() * 16
            + self.face_edges.capacity() * 16
            + self.centers.capacity() * 4
            + self.edges.capacity() * std::mem::size_of::<Edge>()
            + self.incident_offsets.capacity() * 4
            + self.incidents.capacity() * std::mem::size_of::<Corner>()
            + self.neighbor_offsets.capacity() * 4
            + self.neighbors.capacity() * 4
            + self.uv.as_ref().map_or(0, |uv| {
                uv.coordinates.capacity() * 8 + uv.face_indices.capacity() * 16
            })
    }

    pub fn apply(&self, parent: &Mesh) -> Result<Mesh, String> {
        parent.validate()?;
        validate_areas(parent, "source")?;
        if !self.matches(parent) {
            return Err(
                "Static forward map is stale after topology/UV/channel-presence changes".into(),
            );
        }
        let child_faces = self
            .faces
            .len()
            .checked_mul(4)
            .ok_or("Static face count overflow")?;
        let quads = self
            .centers
            .iter()
            .filter(|&&c| c != TRIANGLE_CORNER)
            .count();
        let max_uv = self.uv.as_ref().map_or(Ok(0), |uv| {
            uv.coordinates
                .len()
                .checked_add(self.incidents.len() + quads)
                .ok_or("Static UV count overflow")
        })?;
        if max_uv > MAX_UV_COORDINATES {
            return Err("Static subdivision exceeds UV coordinate limit".into());
        }
        let output_bytes = self.output_vertices * (40 + if self.materials { 8 } else { 0 })
            + self.output_triangles * 12
            + child_faces * 16
            + if self.uv.is_some() {
                max_uv * 8 + child_faces * 16
            } else {
                0
            };
        if output_bytes > crate::decode_limits::MAX_SCENE_STORAGE_BYTES {
            return Err("Static subdivided attribute storage exceeds 128 MiB".into());
        }
        let mut positions = vec![Vec3::ZERO; self.output_vertices];
        let mut colors = vec![[0.0; 3]; self.output_vertices];
        colors[..self.vertices].copy_from_slice(&parent.colors);
        let mut masks = vec![0.0; self.output_vertices];
        masks[..self.vertices].copy_from_slice(&parent.masks);
        let mut materials = parent.materials.as_ref().map(|m| {
            let mut out = vec![[0.0; 2]; self.output_vertices];
            out[..self.vertices].copy_from_slice(m);
            out
        });
        for (vertex, position) in positions[..self.vertices].iter_mut().enumerate() {
            *position = Vec3::from_array(std::array::from_fn(|channel| {
                self.even_scalar(vertex, |v| parent.positions[v].to_array()[channel])
            }));
        }
        for edge in &self.edges {
            let index = edge.output as usize;
            positions[index] = Vec3::from_array(std::array::from_fn(|channel| {
                self.edge_scalar(edge, |v| parent.positions[v].to_array()[channel])
            }));
            colors[index] = std::array::from_fn(|channel| {
                self.edge_scalar(edge, |v| parent.colors[v][channel])
            });
            masks[index] = self.edge_scalar(edge, |v| parent.masks[v]);
            if let (Some(output), Some(source)) = (&mut materials, &parent.materials) {
                output[index] =
                    std::array::from_fn(|channel| self.edge_scalar(edge, |v| source[v][channel]));
            }
        }
        for (face, &index) in self.centers.iter().enumerate() {
            if index != TRIANGLE_CORNER {
                let ids = self.faces[face];
                let index = index as usize;
                positions[index] = Vec3::from_array(std::array::from_fn(|channel| {
                    quad_scalar(ids, |v| parent.positions[v].to_array()[channel])
                }));
                colors[index] =
                    std::array::from_fn(|channel| quad_scalar(ids, |v| parent.colors[v][channel]));
                masks[index] = quad_scalar(ids, |v| parent.masks[v]);
                if let (Some(output), Some(source)) = (&mut materials, &parent.materials) {
                    output[index] =
                        std::array::from_fn(|channel| quad_scalar(ids, |v| source[v][channel]));
                }
            }
        }
        let mut faces = reserved(child_faces, "child faces")?;
        for (id, face) in self.faces.iter().enumerate() {
            let mids = self.face_edges[id].map(|e| {
                if e == TRIANGLE_CORNER {
                    e
                } else {
                    self.edges[e as usize].output
                }
            });
            faces.extend(children(*face, mids, self.centers[id]));
        }
        let mut triangles = reserved(self.output_triangles, "render triangles")?;
        for &[a, b, c, d] in &faces {
            triangles.push([a, b, c]);
            if d != TRIANGLE_CORNER {
                triangles.push([a, c, d]);
            }
        }
        let uv = self.subdivided_uv()?;
        let mut mesh = Mesh::new(positions, triangles);
        mesh.colors = colors;
        mesh.masks = masks;
        mesh.mask_convention = parent.mask_convention;
        mesh.materials = materials;
        mesh.surface = Some(SurfaceTopology { faces, uv });
        mesh.validate()?;
        validate_areas(&mesh, "result")?;
        Ok(mesh)
    }

    fn matches(&self, parent: &Mesh) -> bool {
        if parent.mask_convention != self.mask_convention
            || parent.positions.len() != self.vertices
            || parent.materials.is_some() != self.materials
            || parent.surface.is_some() != self.original_surface
        {
            return false;
        }
        let faces_match = parent.surface.as_ref().map_or_else(
            || {
                self.faces.len() == parent.triangles.len()
                    && self
                        .faces
                        .iter()
                        .zip(&parent.triangles)
                        .all(|(f, t)| *f == [t[0], t[1], t[2], TRIANGLE_CORNER])
            },
            |s| s.faces == self.faces,
        );
        let uv = parent.surface.as_ref().and_then(|s| s.uv.as_ref());
        faces_match
            && match (&self.uv, uv) {
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

    fn incident(&self, vertex: usize) -> &[Corner] {
        &self.incidents
            [self.incident_offsets[vertex] as usize..self.incident_offsets[vertex + 1] as usize]
    }
    fn neighbor(&self, vertex: usize) -> &[u32] {
        &self.neighbors
            [self.neighbor_offsets[vertex] as usize..self.neighbor_offsets[vertex + 1] as usize]
    }

    fn even_scalar(&self, vertex: usize, value: impl Fn(usize) -> f32) -> f32 {
        let incident = self.incident(vertex);
        let neighbors = self.neighbor(vertex);
        if incident.len() != neighbors.len() || self.settings.linear_edges {
            let mut total = 0.0f64;
            let mut count = 0;
            for corner in incident {
                let face = self.faces[corner.face as usize];
                let c = corner.corner as usize;
                let n = arity(&face);
                let fe = self.face_edges[corner.face as usize];
                // Match the actual source's forward/rear else-if ordering.
                let (first, second) = if c == 0 {
                    (c, (c + n - 1) % n)
                } else {
                    ((c + n - 1) % n, c)
                };
                let chosen = if self.edges[fe[first] as usize].count == 1 {
                    Some(first)
                } else if self.edges[fe[second] as usize].count == 1 {
                    Some(second)
                } else {
                    None
                };
                if let Some(edge) = chosen {
                    let next = if edge == c {
                        (c + 1) % n
                    } else {
                        (c + n - 1) % n
                    };
                    total += f64::from(value(face[next] as usize));
                    count += 1;
                }
            }
            return if count < 2 {
                value(vertex)
            } else {
                (f64::from(value(vertex)) * 0.75 + total * (0.25 / count as f64)) as f32
            };
        }
        let total: f64 = neighbors
            .iter()
            .map(|&v| f64::from(value(v as usize)))
            .sum();
        let mut opposite = 0.0;
        let mut quads = 0;
        for corner in incident {
            let face = self.faces[corner.face as usize];
            if face[3] != TRIANGLE_CORNER {
                quads += 1;
                opposite += f64::from(value(face[(corner.corner as usize + 2) % 4] as usize));
            }
        }
        let n = neighbors.len() as f64;
        let (alpha, beta, gamma) = if quads == 0 {
            if n == 3.0 {
                (0.4375, 0.1875, 0.0)
            } else {
                (0.625, 0.375 / n, 0.0)
            }
        } else if quads == incident.len() {
            if n == 4.0 {
                (0.5625, 0.09375, 0.015625)
            } else {
                let beta = 1.5 / (n * n);
                let gamma = 0.25 / (n * n);
                (1.0 - (beta + gamma) * n, beta, gamma)
            }
        } else {
            let alpha = 1.0 / (1.0 + n * 0.5 + quads as f64 * 0.25);
            (alpha, alpha * 0.5, alpha * 0.25)
        };
        (f64::from(value(vertex)) * alpha + total * beta + opposite * gamma) as f32
    }

    fn edge_scalar(&self, edge: &Edge, value: impl Fn(usize) -> f32) -> f32 {
        let endpoints = f64::from(value(edge.vertices[0] as usize))
            + f64::from(value(edge.vertices[1] as usize));
        if edge.count != 2 || self.settings.linear_edges {
            return (0.5 * endpoints) as f32;
        }
        let contribution = |corner: Corner| {
            let face = self.faces[corner.face as usize];
            let n = arity(&face);
            let c = corner.corner as usize;
            if n == 3 {
                0.125 * f64::from(value(face[(c + 2) % 3] as usize))
            } else {
                0.0625
                    * (f64::from(value(face[(c + 2) % 4] as usize))
                        + f64::from(value(face[(c + 3) % 4] as usize)))
            }
        };
        // Source typed-array += rounds the first incident contribution before
        // adding the second. This is intentionally not one fused f64 stencil.
        let first = (contribution(edge.first) + 0.375 * endpoints) as f32;
        (f64::from(first) + contribution(edge.second)) as f32
    }

    fn subdivided_uv(&self) -> Result<Option<CornerUv>, String> {
        let Some(source) = &self.uv else {
            return Ok(None);
        };
        let maximum = source.coordinates.len()
            + self.incidents.len()
            + self
                .centers
                .iter()
                .filter(|&&c| c != TRIANGLE_CORNER)
                .count();
        let mut coordinates = reserved(maximum, "UV coordinates")?;
        coordinates.extend_from_slice(&source.coordinates);
        let mut face_indices = reserved(self.faces.len() * 4, "UV child faces")?;
        let mut lookup = HashMap::<(u32, u32, u32), u32>::new();
        for (id, face) in self.faces.iter().enumerate() {
            let n = arity(face);
            let uv = source.face_indices[id];
            let mut mids = [TRIANGLE_CORNER; 4];
            for corner in 0..n {
                let next = (corner + 1) % n;
                let (a, b) = if face[corner] < face[next] {
                    (uv[corner], uv[next])
                } else {
                    (uv[next], uv[corner])
                };
                let key = (self.face_edges[id][corner], a, b);
                mids[corner] = match lookup.get(&key) {
                    Some(&i) => i,
                    None => {
                        let index = coordinates.len() as u32;
                        coordinates.push(std::array::from_fn(|channel| {
                            (0.5 * (f64::from(source.coordinates[a as usize][channel])
                                + f64::from(source.coordinates[b as usize][channel])))
                                as f32
                        }));
                        lookup
                            .try_reserve(1)
                            .map_err(|_| "Cannot allocate UV chart edges")?;
                        lookup.insert(key, index);
                        index
                    }
                };
            }
            let center = if n == 4 {
                let index = coordinates.len() as u32;
                coordinates.push(std::array::from_fn(|channel| {
                    quad_scalar(uv, |v| source.coordinates[v][channel])
                }));
                index
            } else {
                TRIANGLE_CORNER
            };
            face_indices.extend(children(uv, mids, center));
        }
        Ok(Some(CornerUv {
            coordinates,
            face_indices,
        }))
    }

    fn report(&self, mesh: &Mesh) -> StaticSubdivisionReport {
        StaticSubdivisionReport {
            input_vertices: self.vertices,
            input_faces: self.faces.len(),
            input_quads: self
                .centers
                .iter()
                .filter(|&&c| c != TRIANGLE_CORNER)
                .count(),
            true_edges: self.edges.len(),
            nonmanifold_edges: self.edges.iter().filter(|e| e.count > 2).count(),
            output_vertices: mesh.positions.len(),
            output_faces: self.faces.len() * 4,
            output_triangles: mesh.triangles.len(),
            output_uv_coordinates: mesh.uv_coordinate_count(),
            map_bytes: self.approximate_bytes(),
        }
    }
}

fn arity(face: &[u32; 4]) -> usize {
    if face[3] == TRIANGLE_CORNER {
        3
    } else {
        4
    }
}
fn children([a, b, c, d]: [u32; 4], [ab, bc, cd, da]: [u32; 4], center: u32) -> [[u32; 4]; 4] {
    if d == TRIANGLE_CORNER {
        [
            [ab, bc, cd, TRIANGLE_CORNER],
            [a, ab, cd, TRIANGLE_CORNER],
            [ab, b, bc, TRIANGLE_CORNER],
            [cd, bc, c, TRIANGLE_CORNER],
        ]
    } else {
        [
            [a, ab, center, da],
            [ab, b, bc, center],
            [center, bc, c, cd],
            [da, center, cd, d],
        ]
    }
}
fn quad_scalar(ids: [u32; 4], value: impl Fn(usize) -> f32) -> f32 {
    let [a, b, c, d] = ids.map(|v| f64::from(value(v as usize)));
    (0.25 * (a + b + c + d)) as f32
}
fn reserved<T>(count: usize, label: &str) -> Result<Vec<T>, String> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|_| format!("Cannot allocate bounded static {label}"))?;
    Ok(rows)
}
fn validate_areas(mesh: &Mesh, label: &str) -> Result<(), String> {
    for triangle in &mesh.triangles {
        let [a, b, c] = triangle.map(|v| mesh.positions[v as usize].as_dvec3());
        let area = (b - a).cross(c - a).length_squared();
        if !area.is_finite() || area == 0.0 {
            return Err(format!(
                "Static subdivision rejects collapsed/nonfinite {label} triangles"
            ));
        }
    }
    Ok(())
}
