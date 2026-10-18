//! Bounded structural recognition, independent of a stored hierarchy or IDs.
//! Coordinates are copied from recognized even vertices, not inverse-smoothed.
use crate::{CornerUv, Mesh, StaticSubdivisionSettings, SurfaceTopology, TRIANGLE_CORNER};
use std::collections::{HashMap, HashSet};

pub const MAX_REVERSE_WORK: usize = 32_000_000;
pub const MAX_REVERSE_STORAGE_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct ReverseSubdivisionSettings {
    /// Optional imported fine vertex IDs known to be even. Regular components
    /// need an anchor; this is not an assumed original-prefix mapping.
    pub even_seed_vertices: Vec<u32>,
    /// A smaller deterministic limit may be used for interactive cancellation.
    pub work_limit: usize,
}
impl Default for ReverseSubdivisionSettings {
    fn default() -> Self {
        Self {
            even_seed_vertices: Vec::new(),
            work_limit: MAX_REVERSE_WORK,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ReverseSubdivisionReport {
    pub fine_vertices: usize,
    pub fine_faces: usize,
    pub coarse_vertices: usize,
    pub coarse_faces: usize,
    pub coarse_quads: usize,
    pub components: usize,
    pub candidate_patches: usize,
    pub work: usize,
    /// Fine UV coordinates unused by recovered coarse corners stay in the
    /// untouched fine source, not copied into the smaller coarse UV table.
    pub fine_uv_coordinates_not_in_coarse: usize,
}
#[derive(Clone, Debug)]
pub struct ReverseSubdivisionCorrespondence {
    pub coarse_vertex_to_fine: Vec<u32>,
    pub predicted_vertex_to_fine: Vec<u32>,
    pub coarse_face_children: Vec<[u32; 4]>,
    pub predicted_face_to_fine: Vec<u32>,
    /// Fine corner index matching predicted corner0; winding is preserved.
    pub predicted_face_rotation: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct ReverseSubdivisionOutcome {
    pub coarse: Mesh,
    pub correspondence: ReverseSubdivisionCorrespondence,
    pub report: ReverseSubdivisionReport,
}
#[derive(Clone, Copy)]
struct Incident {
    face: u32,
    corner: u8,
}
#[derive(Clone)]
struct Patch {
    faces: [u32; 4],
    corners: [u32; 4],
    mids: [u32; 4],
    center: u32,
    uv: [u32; 4],
}
impl Patch {
    fn arity(&self) -> usize {
        if self.center == TRIANGLE_CORNER {
            3
        } else {
            4
        }
    }
    fn assignments(&self) -> Vec<(u32, u8, [u32; 2])> {
        let n = self.arity();
        let mut a = Vec::with_capacity(n * 2 + usize::from(n == 4));
        for i in 0..n {
            a.push((self.corners[i], 1, [0; 2]));
            let x = self.corners[i];
            let y = self.corners[(i + 1) % n];
            a.push((self.mids[i], 2, [x.min(y), x.max(y)]));
        }
        if n == 4 {
            a.push((self.center, 3, [0; 2]));
        }
        a
    }
}
struct Work {
    used: usize,
    limit: usize,
}
impl Work {
    fn add(&mut self, count: usize) -> Result<(), String> {
        self.used = self
            .used
            .checked_add(count)
            .ok_or("Reverse work counter overflow")?;
        if self.used > self.limit {
            return Err("Structural Reverse work limit exceeded".into());
        }
        Ok(())
    }
}
struct Graph {
    faces: Vec<[u32; 4]>,
    neighbors: Vec<Vec<u32>>,
    incidents: Vec<Vec<Incident>>,
    adjacent: Vec<[u32; 4]>,
    extraordinary: Vec<bool>,
    components: Vec<Vec<u32>>,
}
fn arity(face: &[u32; 4]) -> usize {
    if face[3] == TRIANGLE_CORNER {
        3
    } else {
        4
    }
}
impl Graph {
    fn build(mesh: &Mesh, work: &mut Work) -> Result<Self, String> {
        let face_count = mesh
            .surface
            .as_ref()
            .map_or(mesh.triangles.len(), |s| s.faces.len());
        if !face_count.is_multiple_of(4) {
            return Err("Reverse requires four fine faces per recovered parent".into());
        }
        // Conservative bounded control-table estimate, separate from the input
        // Mesh's own 128 MiB storage bound. This is not a process RSS guarantee.
        let bytes = face_count
            .checked_mul(1024)
            .and_then(|b| b.checked_add(mesh.positions.len() * 512))
            .ok_or("Reverse table size overflow")?;
        if bytes > MAX_REVERSE_STORAGE_BYTES {
            return Err("Reverse control tables exceed 128 MiB".into());
        }
        let faces: Vec<_> = mesh.surface.as_ref().map_or_else(
            || {
                mesh.triangles
                    .iter()
                    .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
                    .collect()
            },
            |s| s.faces.clone(),
        );
        let mut neighbors = vec![Vec::new(); mesh.positions.len()];
        let mut incidents = vec![Vec::new(); mesh.positions.len()];
        let mut adjacent = vec![[TRIANGLE_CORNER; 4]; faces.len()];
        let mut edges = HashMap::<(u32, u32), Incident>::new();
        let mut completed = HashSet::new();
        edges
            .try_reserve(faces.len() * 2)
            .map_err(|_| "Cannot allocate Reverse edges")?;
        for (fi, face) in faces.iter().enumerate() {
            let n = arity(face);
            for i in 0..n {
                work.add(1)?;
                let a = face[i];
                let b = face[(i + 1) % n];
                let key = (a.min(b), a.max(b));
                if completed.contains(&key) {
                    return Err("Structural Reverse rejects nonmanifold true edges".into());
                }
                incidents[a as usize].push(Incident {
                    face: fi as u32,
                    corner: i as u8,
                });
                if let Some(old) = edges.remove(&key) {
                    let old_face = faces[old.face as usize];
                    let old_n = arity(&old_face);
                    if old_face[old.corner as usize] != b
                        || old_face[(old.corner as usize + 1) % old_n] != a
                    {
                        return Err(
                            "Structural Reverse requires consistent oriented face winding".into(),
                        );
                    }
                    adjacent[fi][i] = old.face;
                    adjacent[old.face as usize][old.corner as usize] = fi as u32;
                    completed.insert(key);
                } else {
                    edges.insert(
                        key,
                        Incident {
                            face: fi as u32,
                            corner: i as u8,
                        },
                    );
                    neighbors[a as usize].push(b);
                    neighbors[b as usize].push(a);
                }
            }
        }
        let mut extraordinary = vec![false; mesh.positions.len()];
        for v in 0..mesh.positions.len() {
            if incidents[v].is_empty() {
                return Err("Structural Reverse rejects isolated vertices".into());
            }
            let n = neighbors[v].len();
            let count = incidents[v].len();
            let border = count != n;
            let quads = incidents[v]
                .iter()
                .filter(|i| arity(&faces[i.face as usize]) == 4)
                .count();
            extraordinary[v] = if quads == 0 {
                n != if border { 4 } else { 6 }
            } else if quads == count {
                n != if border { 3 } else { 4 }
            } else {
                border || n != 5
            };
        }
        let mut components = Vec::new();
        let mut seen = vec![false; mesh.positions.len()];
        for seed in 0..mesh.positions.len() {
            if seen[seed] {
                continue;
            }
            let mut component = vec![seed as u32];
            seen[seed] = true;
            let mut at = 0;
            while at < component.len() {
                let v = component[at];
                at += 1;
                for &other in &neighbors[v as usize] {
                    work.add(1)?;
                    if !seen[other as usize] {
                        seen[other as usize] = true;
                        component.push(other);
                    }
                }
            }
            components.push(component);
        }
        Ok(Self {
            faces,
            neighbors,
            incidents,
            adjacent,
            extraordinary,
            components,
        })
    }
    fn corner_uv(mesh: &Mesh, face: u32, vertex: u32, faces: &[[u32; 4]]) -> u32 {
        mesh.surface
            .as_ref()
            .and_then(|s| s.uv.as_ref())
            .map_or(TRIANGLE_CORNER, |uv| {
                let i = faces[face as usize]
                    .iter()
                    .position(|&v| v == vertex)
                    .unwrap();
                uv.face_indices[face as usize][i]
            })
    }
    fn triangle_patch(&self, mesh: &Mesh, center: usize) -> Option<Patch> {
        let face = self.faces[center];
        if arity(&face) != 3 {
            return None;
        }
        let adjacent = self.adjacent[center];
        let mut opp = [0u32; 3];
        for i in 0..3 {
            let next = *self.faces.get(adjacent[i] as usize)?;
            if arity(&next) != 3 {
                return None;
            }
            opp[i] = *next[..3]
                .iter()
                .find(|&&v| v != face[i] && v != face[(i + 1) % 3])?;
        }
        let corners = [opp[2], opp[0], opp[1], TRIANGLE_CORNER];
        let faces = [center as u32, adjacent[2], adjacent[0], adjacent[1]];
        let mut uv = [TRIANGLE_CORNER; 4];
        for i in 0..3 {
            uv[i] = Self::corner_uv(mesh, faces[i + 1], corners[i], &self.faces);
        }
        let patch = Patch {
            faces,
            corners,
            mids: face,
            center: TRIANGLE_CORNER,
            uv,
        };
        distinct_patch(&patch).then_some(patch)
    }
    fn quad_patch(&self, mesh: &Mesh, center: usize) -> Option<Patch> {
        let incident = &self.incidents[center];
        if incident.len() != 4 || self.neighbors[center].len() != 4 {
            return None;
        }
        let mut entries = Vec::with_capacity(4);
        for i in incident {
            let f = self.faces[i.face as usize];
            if arity(&f) != 4 {
                return None;
            }
            let k = i.corner as usize;
            entries.push((f[(k + 2) % 4], f[(k + 3) % 4], f[(k + 1) % 4], i.face));
        }
        let mut order = [0usize; 4];
        for i in 1..4 {
            let mid = entries[order[i - 1]].1;
            order[i] = entries.iter().position(|e| e.2 == mid)?;
            if order[..i].contains(&order[i]) {
                return None;
            }
        }
        if entries[order[3]].1 != entries[order[0]].2 {
            return None;
        }
        let corners = order.map(|i| entries[i].0);
        let mids = order.map(|i| entries[i].1);
        let faces = order.map(|i| entries[i].3);
        let uv = std::array::from_fn(|i| Self::corner_uv(mesh, faces[i], corners[i], &self.faces));
        let patch = Patch {
            faces,
            corners,
            mids,
            center: center as u32,
            uv,
        };
        distinct_patch(&patch).then_some(patch)
    }
}
fn distinct_patch(p: &Patch) -> bool {
    let n = p.arity();
    let ids = p.assignments();
    let unique: HashSet<_> = ids.iter().map(|a| a.0).collect();
    unique.len() == n * 2 + usize::from(n == 4)
        && p.faces.iter().copied().collect::<HashSet<_>>().len() == 4
}
fn compatible(p: &Patch, used: &[bool], roles: &[u8], endpoints: &[[u32; 2]]) -> bool {
    !p.faces.iter().any(|&f| used[f as usize])
        && p.assignments().iter().all(|&(v, r, e)| {
            roles[v as usize] == 0
                || (roles[v as usize] == r && (r != 2 || endpoints[v as usize] == e))
        })
}
struct Frame {
    face: usize,
    next: usize,
    active: Option<usize>,
    changed: Vec<u32>,
    cursor: usize,
}
fn exact_partition(
    graph: &Graph,
    patches: &[Patch],
    forced: &[bool],
    work: &mut Work,
) -> Result<Vec<usize>, String> {
    let mut by_face = vec![Vec::new(); graph.faces.len()];
    for (i, p) in patches.iter().enumerate() {
        for &f in &p.faces {
            by_face[f as usize].push(i);
        }
    }
    if by_face.iter().any(Vec::is_empty) {
        return Err("Fine faces do not form complete static child patches".into());
    }
    let mut priority: Vec<_> = (0..graph.faces.len()).collect();
    priority.sort_unstable_by_key(|&f| (by_face[f].len(), f));
    let mut roles: Vec<_> = forced.iter().map(|&b| u8::from(b)).collect();
    let mut endpoints = vec![[0; 2]; roles.len()];
    let mut used = vec![false; graph.faces.len()];
    let mut frames = vec![Frame {
        face: priority[0],
        next: 0,
        active: None,
        changed: Vec::new(),
        cursor: 0,
    }];
    let mut solution: Option<Vec<usize>> = None;
    while !frames.is_empty() {
        let top = frames.len() - 1;
        if let Some(p) = frames[top].active.take() {
            for &f in &patches[p].faces {
                used[f as usize] = false;
            }
            for v in frames[top].changed.drain(..) {
                roles[v as usize] = 0;
                endpoints[v as usize] = [0; 2];
            }
        }
        let face = frames[top].face;
        let mut selected = None;
        while frames[top].next < by_face[face].len() {
            let p = by_face[face][frames[top].next];
            frames[top].next += 1;
            work.add(1 + patches[p].arity() * 2)?;
            if compatible(&patches[p], &used, &roles, &endpoints) {
                selected = Some(p);
                break;
            }
        }
        let Some(p) = selected else {
            frames.pop();
            continue;
        };
        for &f in &patches[p].faces {
            used[f as usize] = true;
        }
        for (v, r, e) in patches[p].assignments() {
            if roles[v as usize] == 0 {
                frames[top].changed.push(v);
                roles[v as usize] = r;
                endpoints[v as usize] = e;
            }
        }
        frames[top].active = Some(p);
        let mut cursor = frames[top].cursor;
        while cursor < priority.len() && used[priority[cursor]] {
            work.add(1)?;
            cursor += 1;
        }
        if cursor == priority.len() {
            let current: Vec<_> = frames.iter().map(|f| f.active.unwrap()).collect();
            if solution.is_some() {
                return Err("Static Reverse has multiple valid structural partitions; provide unambiguous even seeds".into());
            }
            solution = Some(current);
        } else {
            frames.push(Frame {
                face: priority[cursor],
                next: 0,
                active: None,
                changed: Vec::new(),
                cursor,
            });
        }
    }
    solution.ok_or_else(|| "Fine topology/seed constraints do not form static subdivision".into())
}

impl Mesh {
    pub fn reversed_static(
        &self,
        settings: &ReverseSubdivisionSettings,
    ) -> Result<ReverseSubdivisionOutcome, String> {
        crate::morph::validate_source(self)?;
        if settings.work_limit == 0
            || settings.work_limit > MAX_REVERSE_WORK
            || settings.even_seed_vertices.len() > self.positions.len()
        {
            return Err("Invalid structural Reverse work/seed limit".into());
        }
        let mut work = Work {
            used: 0,
            limit: settings.work_limit,
        };
        let graph = Graph::build(self, &mut work)?;
        let mut forced = graph.extraordinary.clone();
        let mut seeds = HashSet::new();
        for &v in &settings.even_seed_vertices {
            if v as usize >= forced.len() || !seeds.insert(v) {
                return Err("Reverse even seeds must be distinct valid fine vertex IDs".into());
            }
            forced[v as usize] = true;
        }
        for component in &graph.components {
            if !component.iter().any(|&v| forced[v as usize]) {
                return Err(
                    "Regular Reverse component is ambiguous without an explicit even seed".into(),
                );
            }
        }
        let mut patches = Vec::new();
        for face in 0..graph.faces.len() {
            work.add(1)?;
            if let Some(p) = graph.triangle_patch(self, face) {
                patches.push(p);
            }
        }
        for v in 0..self.positions.len() {
            work.add(1)?;
            if let Some(p) = graph.quad_patch(self, v) {
                patches.push(p);
            }
        }
        let empty_used = vec![false; graph.faces.len()];
        let forced_roles: Vec<_> = forced.iter().map(|&b| u8::from(b)).collect();
        let empty_endpoints = vec![[0; 2]; forced.len()];
        patches.retain(|p| compatible(p, &empty_used, &forced_roles, &empty_endpoints));
        let mut selected = exact_partition(&graph, &patches, &forced, &mut work)?;
        selected.sort_unstable_by_key(|&p| *patches[p].faces.iter().min().unwrap());
        assemble(self, &graph, &patches, &selected, &mut work)
    }
}
fn assemble(
    fine: &Mesh,
    graph: &Graph,
    patches: &[Patch],
    selected: &[usize],
    work: &mut Work,
) -> Result<ReverseSubdivisionOutcome, String> {
    work.add(
        fine.positions
            .len()
            .checked_mul(8)
            .and_then(|n| n.checked_add(graph.faces.len() * 16))
            .ok_or("Reverse correspondence work overflow")?,
    )?;
    let mut even = Vec::new();
    for &p in selected {
        even.extend_from_slice(&patches[p].corners[..patches[p].arity()]);
    }
    even.sort_unstable();
    even.dedup();
    let index: HashMap<_, _> = even
        .iter()
        .enumerate()
        .map(|(i, &v)| (v, i as u32))
        .collect();
    let mut faces = Vec::with_capacity(selected.len());
    let mut uv_ids = HashMap::new();
    let mut coordinates = Vec::new();
    let mut uv_faces = Vec::new();
    let fine_uv = fine.surface.as_ref().and_then(|s| s.uv.as_ref());
    for &p in selected {
        let patch = &patches[p];
        let n = patch.arity();
        let mut f = [TRIANGLE_CORNER; 4];
        let mut u = [TRIANGLE_CORNER; 4];
        for i in 0..n {
            f[i] = index[&patch.corners[i]];
            if let Some(uv) = fine_uv {
                u[i] = *uv_ids.entry(patch.uv[i]).or_insert_with(|| {
                    let id = coordinates.len() as u32;
                    coordinates.push(uv.coordinates[patch.uv[i] as usize]);
                    id
                });
            }
        }
        faces.push(f);
        if fine_uv.is_some() {
            uv_faces.push(u);
        }
    }
    let mut triangles = Vec::new();
    for &[a, b, c, d] in &faces {
        triangles.push([a, b, c]);
        if d != TRIANGLE_CORNER {
            triangles.push([a, c, d]);
        }
    }
    let mut coarse = Mesh::new(
        even.iter().map(|&v| fine.positions[v as usize]).collect(),
        triangles,
    );
    coarse.colors = even.iter().map(|&v| fine.colors[v as usize]).collect();
    coarse.masks = even.iter().map(|&v| fine.masks[v as usize]).collect();
    coarse.mask_convention = fine.mask_convention;
    coarse.materials = fine
        .materials
        .as_ref()
        .map(|m| even.iter().map(|&v| m[v as usize]).collect());
    coarse.surface = Some(SurfaceTopology {
        faces,
        uv: fine_uv.map(|_| CornerUv {
            coordinates,
            face_indices: uv_faces,
        }),
    });
    coarse.validate()?;
    crate::morph::validate_source(&coarse)?;
    let predicted = coarse
        .subdivided_static(&StaticSubdivisionSettings {
            linear_edges: false,
        })?
        .mesh;
    let mut mapping = even.clone();
    let mut edges = HashMap::new();
    for &p in selected {
        let patch = &patches[p];
        let n = patch.arity();
        for i in 0..n {
            let a = index[&patch.corners[i]];
            let b = index[&patch.corners[(i + 1) % n]];
            let key = (a.min(b), a.max(b));
            if let Some(&old) = edges.get(&key) {
                if old != patch.mids[i] {
                    return Err("Reverse edge has inconsistent fine midpoint correspondence".into());
                }
            } else {
                edges.insert(key, patch.mids[i]);
                mapping.push(patch.mids[i]);
            }
        }
        if n == 4 {
            mapping.push(patch.center);
        }
    }
    if mapping.len() != fine.positions.len()
        || mapping.len() != predicted.positions.len()
        || mapping.iter().copied().collect::<HashSet<_>>().len() != mapping.len()
    {
        return Err("Reverse fine-vertex correspondence is not bijective".into());
    }
    let predicted_faces = &predicted.surface.as_ref().unwrap().faces;
    let mut lookup = HashMap::new();
    for (f, face) in graph.faces.iter().enumerate() {
        let key = cycle_key(*face);
        if lookup.insert(key, f as u32).is_some() {
            return Err("Reverse rejects duplicate oriented fine faces".into());
        }
    }
    let mut face_map = Vec::new();
    let mut rotations = Vec::new();
    let mut seen = HashSet::new();
    let mut uv_reverse = HashMap::new();
    for (pf, face) in predicted_faces.iter().enumerate() {
        let n = arity(face);
        let mut mapped = [TRIANGLE_CORNER; 4];
        for i in 0..n {
            mapped[i] = mapping[face[i] as usize];
        }
        let f = *lookup
            .get(&cycle_key(mapped))
            .ok_or("Reverse reconstruction does not match oriented fine faces")?;
        if !seen.insert(f) {
            return Err("Reverse fine face was consumed more than once".into());
        }
        let source = graph.faces[f as usize];
        let rotation = source[..n].iter().position(|&v| v == mapped[0]).unwrap();
        if let Some(uv) = fine_uv {
            let puv = predicted.surface.as_ref().unwrap().uv.as_ref().unwrap();
            for i in 0..n {
                let a = puv.face_indices[pf][i];
                let b = uv.face_indices[f as usize][(rotation + i) % n];
                if puv.coordinates[a as usize].map(f32::to_bits)
                    != uv.coordinates[b as usize].map(f32::to_bits)
                {
                    return Err(
                        "Reverse UV midpoint/center is not the pinned linear corner subdivision"
                            .into(),
                    );
                }
                // A retained fine level may introduce a UV seam inside a
                // parent chart. One predicted ID can therefore have several
                // equal-valued fine aliases; the inverse must stay unique so
                // distinct parent charts cannot be silently merged.
                if uv_reverse.get(&b).is_some_and(|&old| old != a) {
                    return Err(
                        "Reverse UV seam equivalence is inconsistent with parent corners".into(),
                    );
                }
                uv_reverse.insert(b, a);
            }
        }
        face_map.push(f);
        rotations.push(rotation as u8);
    }
    if seen.len() != graph.faces.len() {
        return Err("Reverse does not cover every fine face".into());
    }
    let children = face_map
        .chunks_exact(4)
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect();
    let report = ReverseSubdivisionReport {
        fine_vertices: fine.positions.len(),
        fine_faces: graph.faces.len(),
        coarse_vertices: coarse.positions.len(),
        coarse_faces: selected.len(),
        coarse_quads: selected
            .iter()
            .filter(|&&p| patches[p].arity() == 4)
            .count(),
        components: graph.components.len(),
        candidate_patches: patches.len(),
        work: work.used,
        fine_uv_coordinates_not_in_coarse: fine_uv
            .map_or(0, |uv| uv.coordinates.len() - uv_ids.len()),
    };
    Ok(ReverseSubdivisionOutcome {
        coarse,
        correspondence: ReverseSubdivisionCorrespondence {
            coarse_vertex_to_fine: even,
            predicted_vertex_to_fine: mapping,
            coarse_face_children: children,
            predicted_face_to_fine: face_map,
            predicted_face_rotation: rotations,
        },
        report,
    })
}
fn cycle_key(face: [u32; 4]) -> [u32; 4] {
    let n = arity(&face);
    let start = face[..n]
        .iter()
        .enumerate()
        .min_by_key(|(_, v)| **v)
        .unwrap()
        .0;
    let mut out = [TRIANGLE_CORNER; 4];
    for i in 0..n {
        out[i] = face[(start + i) % n];
    }
    out
}
