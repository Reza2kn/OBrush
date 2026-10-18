//! Owned incremental dense hierarchy. Production entry applies an explicit finite local-ray domain.
//! Scalar leaves call the unchanged ancestor `intersect`, not a new predicate.
use super::{intersect, SourcePickWorkBudget, ValidatedSourcePickSnapshot};
use crate::source_pick_interval::{certificate, Descriptor, Interval, Verdict};
use crate::{Mesh, ObjectAffineF32V1, PreparedSourcePaintHit, SceneRay64, TRIANGLE_CORNER};
use glam::{DMat4, DVec3};
use std::sync::Arc;

const MAX_FACES: usize = super::MAX_SOURCE_PICK_INDEX_FACES;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_COMBINED: usize = 128 * 1024 * 1024;
const STEP: usize = 16_384;
const LEAF: usize = 8;
const NONE: u32 = u32::MAX;

#[derive(Clone, Copy, Debug)]
struct Face {
    ordinal: u32,
    triangle: u32,
    halves: u32,
    key: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Node {
    pairs: [[[f64; 2]; 3]; 5],
    links: [u32; 4],
}
const _: () = assert!(std::mem::size_of::<Node>() == 256);
const _: () = assert!(std::mem::size_of::<Face>() == 16);
impl Node {
    fn pack(d: Descriptor, links: [u32; 4]) -> Self {
        let vectors = [d.a, d.e1, d.e2, d.normal_real, d.vertex_box];
        Self {
            pairs: vectors.map(|v| {
                v.map(|x| {
                    x.bounds()
                        .map_or([f64::NEG_INFINITY, f64::INFINITY], |(a, b)| [a, b])
                })
            }),
            links,
        }
    }
    fn descriptor(self) -> Descriptor {
        let v = self
            .pairs
            .map(|v| v.map(|[lo, hi]| Interval::range(lo, hi)));
        // Every stored edge is a rounded subtraction. Its adjacent-gap bound
        // includes the exact subtraction residual; hull gaps include all rows.
        let errors =
            std::array::from_fn(|i| edge_rounding_gap(v[1][i]).plus(edge_rounding_gap(v[2][i])));
        Descriptor {
            a: v[0],
            e1: v[1],
            e2: v[2],
            normal_real: v[3],
            vertex_box: v[4],
            edge_reconstruction_error: errors,
        }
    }
}

// Conservative true residual bound for every finite rounded edge in a hull.
// This test-only copy uses public interval construction; the original proof
// prototype stays unchanged and its private helpers are not exposed.
fn edge_rounding_gap(value: Interval) -> Interval {
    let Some((lo, hi)) = value.bounds() else {
        return Interval::Whole;
    };
    let magnitude = lo.abs().max(hi.abs());
    if !magnitude.is_finite() {
        return Interval::Whole;
    }
    let gap = (magnitude.next_up() - magnitude).max(magnitude - magnitude.next_down());
    Interval::range(0.0, gap.next_up())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BuildError {
    Unsealed,
    Faces,
    Storage,
    Allocation,
    NotReady,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Bounds,
    Records,
    Scratch,
    Reset,
    Histogram,
    Prefix,
    Scatter,
    Leaves,
    PublishLeaf,
    Merge,
    Ready,
}
#[derive(Clone, Copy, Debug, Default)]
struct Progress {
    operations: usize,
    total: usize,
    phases: [usize; 10],
    ready: bool,
    peak_bytes: usize,
    capacities: [usize; 4],
}
struct Builder {
    sealed: ValidatedSourcePickSnapshot,
    mesh: Arc<Mesh>,
    faces: Vec<Face>,
    order: Vec<u32>,
    scratch: Vec<u32>,
    nodes: Vec<Node>,
    bins: [u32; 256],
    bounds: [[f64; 2]; 3],
    phase: Phase,
    row: usize,
    pass: usize,
    prefix: u32,
    triangle: u32,
    leaf_start: usize,
    accumulated: Option<Descriptor>,
    leaf_count: usize,
    front_base: usize,
    next_base: usize,
    front_len: usize,
    front_row: usize,
    next_len: usize,
    root: u32,
    progress: Progress,
}
const _: () = assert!(std::mem::size_of::<Builder>() <= 4096);

fn checked_bytes(faces: usize, mesh: usize) -> Result<(usize, usize, usize), BuildError> {
    if faces == 0 || faces > MAX_FACES {
        return Err(BuildError::Faces);
    }
    let leaves = faces.div_ceil(LEAF);
    let nodes = leaves
        .checked_mul(2)
        .and_then(|x| x.checked_sub(1))
        .ok_or(BuildError::Storage)?;
    let scratch = faces.max(leaves.checked_mul(2).ok_or(BuildError::Storage)?);
    let bytes = std::mem::size_of::<Builder>()
        .checked_add(faces.checked_mul(20).ok_or(BuildError::Storage)?)
        .and_then(|x| x.checked_add(scratch.checked_mul(4)?))
        .and_then(|x| x.checked_add(nodes.checked_mul(256)?))
        .ok_or(BuildError::Storage)?;
    if bytes > MAX_BYTES || mesh.checked_add(bytes).is_none_or(|n| n > MAX_COMBINED) {
        return Err(BuildError::Storage);
    }
    Ok((nodes, scratch, bytes))
}
impl Builder {
    fn new(sealed: ValidatedSourcePickSnapshot, published: &Arc<Mesh>) -> Result<Self, BuildError> {
        if !sealed.matches(published) {
            return Err(BuildError::Unsealed);
        }
        let count = published
            .surface
            .as_ref()
            .map_or(published.triangles.len(), |s| s.faces.len());
        let mesh_bytes = sealed.progress().mesh_storage_bytes;
        let (max_nodes, scratch_count, _) = checked_bytes(count, mesh_bytes)?;
        let mut faces = Vec::new();
        let mut order = Vec::new();
        let mut scratch = Vec::new();
        let mut nodes = Vec::new();
        faces
            .try_reserve_exact(count)
            .map_err(|_| BuildError::Allocation)?;
        order
            .try_reserve_exact(count)
            .map_err(|_| BuildError::Allocation)?;
        scratch
            .try_reserve_exact(scratch_count)
            .map_err(|_| BuildError::Allocation)?;
        nodes
            .try_reserve_exact(max_nodes)
            .map_err(|_| BuildError::Allocation)?;
        let peak = std::mem::size_of::<Self>()
            .checked_add(
                faces
                    .capacity()
                    .checked_mul(16)
                    .ok_or(BuildError::Storage)?,
            )
            .and_then(|n| n.checked_add(order.capacity().checked_mul(4)?))
            .and_then(|n| n.checked_add(scratch.capacity().checked_mul(4)?))
            .and_then(|n| n.checked_add(nodes.capacity().checked_mul(256)?))
            .ok_or(BuildError::Storage)?;
        if peak > MAX_BYTES
            || mesh_bytes
                .checked_add(peak)
                .is_none_or(|n| n > MAX_COMBINED)
        {
            return Err(BuildError::Storage);
        }
        let capacities = [
            faces.capacity(),
            order.capacity(),
            scratch.capacity(),
            nodes.capacity(),
        ];
        Ok(Self {
            sealed,
            mesh: Arc::clone(published),
            faces,
            order,
            scratch,
            nodes,
            bins: [0; 256],
            bounds: [[f64::INFINITY, f64::NEG_INFINITY]; 3],
            phase: Phase::Bounds,
            row: 0,
            pass: 0,
            prefix: 0,
            triangle: 0,
            leaf_start: 0,
            accumulated: None,
            leaf_count: 0,
            front_base: 0,
            next_base: 0,
            front_len: 0,
            front_row: 0,
            next_len: 0,
            root: NONE,
            progress: Progress {
                peak_bytes: peak,
                capacities,
                ..Progress::default()
            },
        })
    }
    fn count(&self) -> usize {
        self.mesh
            .surface
            .as_ref()
            .map_or(self.mesh.triangles.len(), |s| s.faces.len())
    }
    fn advance(&mut self, phase: Phase) {
        self.phase = phase;
        self.row = 0;
    }
    fn step(&mut self, allowance: usize) -> Progress {
        self.progress.operations = 0;
        while self.progress.operations < allowance.min(STEP) && self.phase != Phase::Ready {
            // Charge before each bounded row/bin/descriptor/publication/merge.
            self.progress.operations += 1;
            self.progress.total += 1;
            let slot = match self.phase {
                Phase::Bounds => 0,
                Phase::Records => 1,
                Phase::Scratch => 2,
                Phase::Reset => 3,
                Phase::Histogram => 4,
                Phase::Prefix => 5,
                Phase::Scatter => 6,
                Phase::Leaves => 7,
                Phase::PublishLeaf => 8,
                Phase::Merge => 9,
                Phase::Ready => unreachable!(),
            };
            self.progress.phases[slot] += 1;
            self.one();
        }
        self.progress.ready = self.phase == Phase::Ready;
        self.progress
    }
    fn one(&mut self) {
        match self.phase {
            Phase::Bounds => {
                for (i, x) in self.mesh.positions[self.row]
                    .as_dvec3()
                    .to_array()
                    .into_iter()
                    .enumerate()
                {
                    if x < self.bounds[i][0] {
                        self.bounds[i][0] = x;
                    }
                    if x > self.bounds[i][1] {
                        self.bounds[i][1] = x;
                    }
                }
                self.row += 1;
                if self.row == self.mesh.positions.len() {
                    self.advance(Phase::Records);
                }
            }
            Phase::Records => {
                let ids = face_ids(&self.mesh, self.row);
                let halves = if ids[3] == TRIANGLE_CORNER { 1 } else { 2 };
                let mut center = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
                for id in ids.into_iter().take(if halves == 1 { 3 } else { 4 }) {
                    for (i, x) in self.mesh.positions[id as usize]
                        .as_dvec3()
                        .to_array()
                        .into_iter()
                        .enumerate()
                    {
                        center[i][0] = center[i][0].min(x);
                        center[i][1] = center[i][1].max(x);
                    }
                }
                let key = morton(
                    std::array::from_fn(|i| (center[i][0] + center[i][1]) * 0.5),
                    self.bounds,
                );
                self.faces.push(Face {
                    ordinal: self.row as u32,
                    triangle: self.triangle,
                    halves,
                    key,
                });
                self.order.push(self.row as u32);
                self.triangle += halves;
                self.row += 1;
                if self.row == self.count() {
                    self.advance(Phase::Scratch);
                }
            }
            Phase::Scratch => {
                self.scratch.push(0);
                self.row += 1;
                if self.row == self.count().max(2 * self.count().div_ceil(LEAF)) {
                    self.advance(Phase::Reset);
                }
            }
            Phase::Reset => {
                self.bins[self.row] = 0;
                self.row += 1;
                if self.row == 256 {
                    self.advance(Phase::Histogram);
                }
            }
            Phase::Histogram => {
                let key = ((self.faces[self.order[self.row] as usize].key >> (self.pass * 8)) & 255)
                    as usize;
                self.bins[key] += 1;
                self.row += 1;
                if self.row == self.count() {
                    self.prefix = 0;
                    self.advance(Phase::Prefix);
                }
            }
            Phase::Prefix => {
                let count = self.bins[self.row];
                self.bins[self.row] = self.prefix;
                self.prefix += count;
                self.row += 1;
                if self.row == 256 {
                    self.advance(Phase::Scatter);
                }
            }
            Phase::Scatter => {
                let id = self.order[self.row];
                let key = ((self.faces[id as usize].key >> (self.pass * 8)) & 255) as usize;
                let target = self.bins[key] as usize;
                self.scratch[target] = id;
                self.bins[key] += 1;
                self.row += 1;
                if self.row == self.count() {
                    std::mem::swap(&mut self.order, &mut self.scratch);
                    self.pass += 1;
                    if self.pass == 4 {
                        self.advance(Phase::Leaves);
                    } else {
                        self.advance(Phase::Reset);
                    }
                }
            }
            Phase::Leaves => {
                let record = self.faces[self.order[self.row] as usize];
                let d = face_descriptor(&self.mesh, record.ordinal as usize);
                self.accumulated = Some(self.accumulated.map_or(d, |old| old.hull(d)));
                self.row += 1;
                if self.row - self.leaf_start == LEAF || self.row == self.count() {
                    self.phase = Phase::PublishLeaf;
                }
            }
            Phase::PublishLeaf => {
                let id = self.nodes.len() as u32;
                let d = self.accumulated.take().expect("charged nonempty leaf");
                self.nodes.push(Node::pack(
                    d,
                    [
                        NONE,
                        NONE,
                        self.leaf_start as u32,
                        (self.row - self.leaf_start) as u32,
                    ],
                ));
                self.scratch[self.leaf_count] = id;
                self.leaf_count += 1;
                self.leaf_start = self.row;
                if self.row == self.count() {
                    self.front_len = self.leaf_count;
                    self.next_base = self.leaf_count;
                    if self.front_len == 1 {
                        self.root = id;
                        self.phase = Phase::Ready;
                    } else {
                        self.phase = Phase::Merge;
                    }
                } else {
                    self.phase = Phase::Leaves;
                }
            }
            Phase::Merge => {
                let left = self.scratch[self.front_base + self.front_row];
                let id = if self.front_row + 1 < self.front_len {
                    let right = self.scratch[self.front_base + self.front_row + 1];
                    let l = self.nodes[left as usize];
                    let r = self.nodes[right as usize];
                    let id = self.nodes.len() as u32;
                    self.nodes.push(Node::pack(
                        l.descriptor().hull(r.descriptor()),
                        [left, right, l.links[2], l.links[3] + r.links[3]],
                    ));
                    self.front_row += 2;
                    id
                } else {
                    self.front_row += 1;
                    left
                };
                self.scratch[self.next_base + self.next_len] = id;
                self.next_len += 1;
                if self.front_row == self.front_len {
                    if self.next_len == 1 {
                        self.root = id;
                        self.phase = Phase::Ready;
                    } else {
                        std::mem::swap(&mut self.front_base, &mut self.next_base);
                        self.front_len = self.next_len;
                        self.front_row = 0;
                        self.next_len = 0;
                    }
                }
            }
            Phase::Ready => unreachable!("step never schedules uncharged ready work"),
        }
    }
    fn finish(self) -> Result<Index, BuildError> {
        if self.phase != Phase::Ready {
            return Err(BuildError::NotReady);
        }
        Ok(Index {
            sealed: self.sealed,
            mesh: self.mesh,
            faces: self.faces,
            order: self.order,
            nodes: self.nodes,
            root: self.root,
            progress: self.progress,
        })
    }
}
fn morton(point: [f64; 3], bounds: [[f64; 2]; 3]) -> u32 {
    let q: [u32; 3] = std::array::from_fn(|i| {
        let extent = bounds[i][1] - bounds[i][0];
        if extent == 0.0 {
            0
        } else {
            (((point[i] - bounds[i][0]) / extent).clamp(0.0, 1.0) * 1023.0).floor() as u32
        }
    });
    let mut key = 0;
    for bit in 0..10 {
        for (axis, value) in q.into_iter().enumerate() {
            key |= ((value >> bit) & 1) << (3 * bit + axis);
        }
    }
    key
}
fn face_ids(mesh: &Mesh, face: usize) -> [u32; 4] {
    mesh.surface.as_ref().map_or_else(
        || {
            let t = mesh.triangles[face];
            [t[0], t[1], t[2], TRIANGLE_CORNER]
        },
        |s| s.faces[face],
    )
}
fn face_descriptor(mesh: &Mesh, face: usize) -> Descriptor {
    let ids = face_ids(mesh, face);
    let tri = |t: [u32; 3]| {
        Descriptor::triangle(t.map(|id| mesh.positions[id as usize].as_dvec3().to_array()))
    };
    let first = tri([ids[0], ids[1], ids[2]]);
    if ids[3] == TRIANGLE_CORNER {
        first
    } else {
        first.hull(tri([ids[0], ids[2], ids[3]]))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    revision: u64,
    part: u64,
    epoch: u64,
    frame: u64,
    pose: [u32; 16],
}
impl Identity {
    fn new(
        revision: u64,
        part: u64,
        epoch: u64,
        frame: u64,
        pose: Option<&ObjectAffineF32V1>,
    ) -> Self {
        Self {
            revision,
            part,
            epoch,
            frame,
            pose: pose
                .map_or(DMat4::IDENTITY.to_cols_array().map(|x| x as f32), |p| {
                    p.local_to_world
                })
                .map(f32::to_bits),
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct Budget {
    nodes: usize,
    work: SourcePickWorkBudget,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            nodes: 4096,
            work: SourcePickWorkBudget::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct Counts {
    nodes: usize,
    faces: usize,
    triangles: usize,
    proved_miss: usize,
    possible: usize,
    unknown: usize,
    distance_pruned: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QueryError {
    Stale,
    InvalidRay,
    InvalidPose,
    InvalidLocalRay,
    WorkBudget,
}
#[derive(Clone, Debug)]
struct Hit {
    prepared: PreparedSourcePaintHit,
    #[cfg(test)]
    t: f64,
    #[cfg(test)]
    ordinal: usize,
}
struct Index {
    sealed: ValidatedSourcePickSnapshot,
    mesh: Arc<Mesh>,
    faces: Vec<Face>,
    order: Vec<u32>,
    nodes: Vec<Node>,
    root: u32,
    progress: Progress,
}
const _: () = assert!(std::mem::size_of::<Index>() <= 4096);
const _: () = assert!(
    std::mem::size_of::<Budget>()
        + std::mem::size_of::<Counts>()
        + std::mem::size_of::<[u32; 64]>()
        <= 4096
);
fn local_ray(
    pose: Option<&ObjectAffineF32V1>,
    ray: SceneRay64,
) -> Result<(DMat4, DVec3, DVec3), QueryError> {
    ray.validate().map_err(|_| QueryError::InvalidRay)?;
    let matrix = if let Some(pose) = pose {
        pose.validate().map_err(|_| QueryError::InvalidPose)?;
        DMat4::from_cols_array(&pose.local_to_world.map(f64::from))
    } else {
        DMat4::IDENTITY
    };
    let inverse = matrix.inverse();
    let origin = inverse.transform_point3(ray.origin);
    let direction = inverse.transform_vector3(ray.direction).normalize();
    if !origin.is_finite() || !direction.is_finite() || direction == DVec3::ZERO {
        return Err(QueryError::InvalidLocalRay);
    }
    Ok((matrix, origin, direction))
}
fn chosen(mesh: &Mesh, face: Face, origin: DVec3, direction: DVec3) -> Option<(usize, f64)> {
    let ids = face_ids(mesh, face.ordinal as usize);
    intersect(mesh, origin, direction, [ids[0], ids[1], ids[2]])
        .map(|t| (face.triangle as usize, t))
        .or_else(|| {
            if face.halves == 2 {
                intersect(mesh, origin, direction, [ids[0], ids[2], ids[3]])
                    .map(|t| (face.triangle as usize + 1, t))
            } else {
                None
            }
        })
}
fn make_hit(
    mesh: &Mesh,
    matrix: DMat4,
    origin: DVec3,
    direction: DVec3,
    best: (usize, usize, f64),
) -> Hit {
    let (ordinal, triangle_index, t) = best;
    let point = origin + direction * t;
    Hit {
        prepared: PreparedSourcePaintHit {
            triangle_index,
            original_face_index: mesh.surface.as_ref().map(|_| ordinal),
            local_position: point.to_array(),
            local_eye_direction: direction.to_array(),
            world_position: matrix.transform_point3(point).to_array(),
        },
        #[cfg(test)]
        t,
        #[cfg(test)]
        ordinal,
    }
}
impl Index {
    #[expect(
        clippy::too_many_arguments,
        reason = "Keep publication, captured/current identity, affine ray and shared budgets explicit in this owned query"
    )]
    fn query(
        &self,
        published: &Arc<Mesh>,
        captured: Identity,
        current: Identity,
        pose: Option<&ObjectAffineF32V1>,
        ray: SceneRay64,
        budget: &mut Budget,
        counts: &mut Counts,
    ) -> Result<Option<Hit>, QueryError> {
        if !self.sealed.matches(published)
            || captured != current
            || captured.pose
                != Identity::new(
                    current.revision,
                    current.part,
                    current.epoch,
                    current.frame,
                    pose,
                )
                .pose
        {
            return Err(QueryError::Stale);
        }
        let (matrix, origin, direction) = local_ray(pose, ray)?;
        let mut best: Option<(usize, usize, f64)> = None;
        let mut stack = [0_u32; 64];
        let mut len = 1;
        stack[0] = self.root;
        while len > 0 {
            if budget.nodes == 0 {
                return Err(QueryError::WorkBudget);
            }
            budget.nodes -= 1;
            counts.nodes += 1;
            len -= 1;
            let node = self.nodes[stack[len] as usize];
            let cert = certificate(&node.descriptor(), origin.to_array(), direction.to_array());
            match cert.verdict {
                Verdict::ProvedMiss => {
                    counts.proved_miss += 1;
                    continue;
                }
                Verdict::Possible => counts.possible += 1,
                Verdict::Unknown => counts.unknown += 1,
            }
            if best.is_some_and(|(_, _, t)| {
                cert.replay
                    .t
                    .floating
                    .bounds()
                    .is_some_and(|(lo, _)| lo > t)
            }) {
                counts.distance_pruned += 1;
                continue;
            }
            if node.links[0] == NONE {
                for row in node.links[2] as usize..(node.links[2] + node.links[3]) as usize {
                    let face = self.faces[self.order[row] as usize];
                    let halves = face.halves as usize;
                    if budget.work.face_tests == 0 || budget.work.triangle_tests < halves {
                        return Err(QueryError::WorkBudget);
                    }
                    budget.work.face_tests -= 1;
                    budget.work.triangle_tests -= halves;
                    counts.faces += 1;
                    counts.triangles += halves;
                    if let Some((triangle, t)) = chosen(&self.mesh, face, origin, direction) {
                        let ordinal = face.ordinal as usize;
                        if best.is_none_or(|(old, _, distance)| {
                            t < distance || (t == distance && ordinal < old)
                        }) {
                            best = Some((ordinal, triangle, t));
                        }
                    }
                }
            } else {
                if len + 2 > stack.len() {
                    return Err(QueryError::WorkBudget);
                }
                stack[len] = node.links[1];
                stack[len + 1] = node.links[0];
                len += 2;
            }
        }
        Ok(best.map(|b| make_hit(&self.mesh, matrix, origin, direction, b)))
    }
}

// An intentionally complete test oracle. Its predicate is the actual unchanged
// private ancestor function, and ordinal scan/quad precedence matches ray().
#[cfg(test)]
fn linear_authority(
    mesh: &Mesh,
    pose: Option<&ObjectAffineF32V1>,
    ray: SceneRay64,
) -> Result<Option<Hit>, QueryError> {
    let (matrix, origin, direction) = local_ray(pose, ray)?;
    let mut best: Option<(usize, usize, f64)> = None;
    let mut row = 0;
    let count = mesh
        .surface
        .as_ref()
        .map_or(mesh.triangles.len(), |s| s.faces.len());
    for ordinal in 0..count {
        let halves = if face_ids(mesh, ordinal)[3] == TRIANGLE_CORNER {
            1
        } else {
            2
        };
        let face = Face {
            ordinal: ordinal as u32,
            triangle: row,
            halves,
            key: 0,
        };
        row += halves;
        if let Some((triangle, t)) = chosen(mesh, face, origin, direction) {
            if best.is_none_or(|(_, _, distance)| t < distance) {
                best = Some((ordinal, triangle, t));
            }
        }
    }
    Ok(best.map(|b| make_hit(mesh, matrix, origin, direction, b)))
}

#[cfg(test)]
#[path = "source_pick_dense_tests.rs"]
mod tests;

/// Bounded preparation state; no input event is held for later execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum SourcePickQueryAdmissionError {
    Snapshot(super::SourcePickAdmissionError),
    IndexFaces,
    IndexStorage,
    IndexAllocation,
    Identity,
    NotReady,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct SourcePickQueryProgress {
    #[serde(flatten)]
    pub snapshot: super::SourcePickAdmissionProgress,
    pub index_operations: usize,
    pub index_total_operations: usize,
    pub index_peak_bytes: usize,
    pub index_capacities: [usize; 4],
    pub indexed: bool,
}
impl SourcePickQueryProgress {
    pub fn ready(&self) -> bool {
        self.snapshot.ready
    }
}
#[expect(
    clippy::large_enum_variant,
    reason = "Bounded inline owner avoids an infallible uncharged heap box; holder sizes and combined storage are checked"
)]
enum AdmissionStage {
    Validate(super::SourcePickSnapshotAdmission),
    Index(Builder),
    Ready(SourcePreparedPickQuery),
    Rejected,
}
pub struct SourcePickQueryAdmission {
    published: Arc<Mesh>,
    batch_queries: usize,
    stage: AdmissionStage,
    progress: SourcePickQueryProgress,
}
#[cfg_attr(
    target_pointer_width = "64",
    expect(
        clippy::large_enum_variant,
        reason = "Keep the bounded prepared owner inline; all index buffers use fallible charged allocation"
    )
)]
enum PreparedKind {
    Linear(ValidatedSourcePickSnapshot),
    Indexed(Index),
}
/// Owns immutable geometry; a different published allocation needs preparation.
/// Small meshes preserve the original complete linear-query API and budgets.
pub struct SourcePreparedPickQuery {
    kind: PreparedKind,
    progress: SourcePickQueryProgress,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum SourcePickQueryError {
    Ray(super::SourceSnapshotRayError),
    OutsideDenseLocalDomain,
    Identity,
}
/// Shared across an ordered input batch, including attribute corner reads.
#[derive(Clone, Copy, Debug)]
pub struct SourcePickQueryBudget {
    pub attributes: SourcePickWorkBudget,
    nodes: usize,
}
impl Default for SourcePickQueryBudget {
    fn default() -> Self {
        Self {
            attributes: SourcePickWorkBudget::default(),
            nodes: 4096,
        }
    }
}
impl SourcePickQueryBudget {
    pub fn remaining_node_certificates(&self) -> usize {
        self.nodes
    }
}
const QUERY_OWNER_CHARGE_BYTES: usize = std::mem::size_of::<SourcePickQueryAdmission>()
    + std::mem::size_of::<SourcePreparedPickQuery>()
    + std::mem::size_of::<SourcePickQueryBudget>()
    + 256;
const _: () = assert!(QUERY_OWNER_CHARGE_BYTES <= 8192);
fn owned_index_peak(raw: usize, mesh: usize) -> Result<usize, SourcePickQueryAdmissionError> {
    let peak = raw
        .checked_add(QUERY_OWNER_CHARGE_BYTES)
        .ok_or(SourcePickQueryAdmissionError::IndexStorage)?;
    if peak > MAX_BYTES || mesh.checked_add(peak).is_none_or(|n| n > MAX_COMBINED) {
        return Err(SourcePickQueryAdmissionError::IndexStorage);
    }
    Ok(peak)
}
fn admission_error(e: BuildError) -> SourcePickQueryAdmissionError {
    match e {
        BuildError::Unsealed => SourcePickQueryAdmissionError::Identity,
        BuildError::Faces => SourcePickQueryAdmissionError::IndexFaces,
        BuildError::Storage => SourcePickQueryAdmissionError::IndexStorage,
        BuildError::Allocation => SourcePickQueryAdmissionError::IndexAllocation,
        BuildError::NotReady => SourcePickQueryAdmissionError::NotReady,
    }
}
fn query_error(e: QueryError) -> SourcePickQueryError {
    use super::SourceSnapshotRayError as E;
    match e {
        QueryError::Stale => SourcePickQueryError::Identity,
        QueryError::InvalidRay => SourcePickQueryError::Ray(E::InvalidRay),
        QueryError::InvalidPose => SourcePickQueryError::Ray(E::InvalidPose),
        QueryError::InvalidLocalRay => SourcePickQueryError::Ray(E::InvalidLocalRay),
        QueryError::WorkBudget => SourcePickQueryError::Ray(E::WorkBudget),
    }
}
impl SourcePickQueryAdmission {
    pub fn new(published: Arc<Mesh>) -> Result<Self, SourcePickQueryAdmissionError> {
        Self::new_for_batch(published, 1)
    }
    /// Chooses an index when two complete linear rays would exceed the shared
    /// existing query budget. The original one-ray constructor stays unchanged.
    pub fn new_for_batch(
        published: Arc<Mesh>,
        batch_queries: usize,
    ) -> Result<Self, SourcePickQueryAdmissionError> {
        if !(1..=2).contains(&batch_queries) {
            return Err(SourcePickQueryAdmissionError::IndexFaces);
        }
        let admission = super::SourcePickSnapshotAdmission::new(Arc::clone(&published))
            .map_err(SourcePickQueryAdmissionError::Snapshot)?;
        Ok(Self {
            published,
            batch_queries,
            stage: AdmissionStage::Validate(admission),
            progress: SourcePickQueryProgress::default(),
        })
    }
    pub fn matches(&self, published: &Arc<Mesh>) -> bool {
        Arc::ptr_eq(&self.published, published) && !matches!(self.stage, AdmissionStage::Rejected)
    }
    pub fn progress(&self) -> SourcePickQueryProgress {
        self.progress
    }
    /// At most 16,384 validation rows or index operations; never both in one call.
    /// Reserving the bounded buffers occurs once after successful validation.
    pub fn step(
        &mut self,
        allowance: usize,
    ) -> Result<SourcePickQueryProgress, SourcePickQueryAdmissionError> {
        self.progress.snapshot.operations = 0;
        self.progress.index_operations = 0;
        let state = std::mem::replace(&mut self.stage, AdmissionStage::Rejected);
        self.stage = match state {
            AdmissionStage::Validate(mut admission) => {
                let p = admission
                    .step(allowance)
                    .map_err(SourcePickQueryAdmissionError::Snapshot)?;
                self.progress.snapshot = p;
                // Snapshot validation is not complete query preparation.
                self.progress.snapshot.ready = false;
                if p.ready {
                    let sealed = admission
                        .finish()
                        .map_err(SourcePickQueryAdmissionError::Snapshot)?;
                    if sealed.complete_query_batch_supported(self.batch_queries) {
                        self.progress.snapshot.ready = true;
                        AdmissionStage::Ready(SourcePreparedPickQuery {
                            kind: PreparedKind::Linear(sealed),
                            progress: self.progress,
                        })
                    } else {
                        let builder =
                            Builder::new(sealed, &self.published).map_err(admission_error)?;
                        self.progress.indexed = true;
                        self.progress.index_peak_bytes = owned_index_peak(
                            builder.progress.peak_bytes,
                            builder.sealed.progress().mesh_storage_bytes,
                        )?;
                        self.progress.index_capacities = builder.progress.capacities;
                        self.progress.snapshot.ready = false;
                        AdmissionStage::Index(builder)
                    }
                } else {
                    AdmissionStage::Validate(admission)
                }
            }
            AdmissionStage::Index(mut builder) => {
                let p = builder.step(allowance);
                self.progress.index_operations = p.operations;
                self.progress.index_total_operations = p.total;
                self.progress.index_peak_bytes =
                    owned_index_peak(p.peak_bytes, builder.sealed.progress().mesh_storage_bytes)?;
                self.progress.index_capacities = p.capacities;
                self.progress.snapshot.operations = p.operations;
                self.progress.snapshot.total_operations = builder
                    .sealed
                    .progress()
                    .total_operations
                    .checked_add(p.total)
                    .ok_or(SourcePickQueryAdmissionError::IndexStorage)?;
                self.progress.snapshot.ready = p.ready;
                if p.ready {
                    let index = builder.finish().map_err(admission_error)?;
                    self.progress.index_peak_bytes = owned_index_peak(
                        index.progress.peak_bytes,
                        index.sealed.progress().mesh_storage_bytes,
                    )?;
                    self.progress.index_capacities = index.progress.capacities;
                    AdmissionStage::Ready(SourcePreparedPickQuery {
                        kind: PreparedKind::Indexed(index),
                        progress: self.progress,
                    })
                } else {
                    AdmissionStage::Index(builder)
                }
            }
            AdmissionStage::Ready(prepared) => AdmissionStage::Ready(prepared),
            AdmissionStage::Rejected => return Err(SourcePickQueryAdmissionError::NotReady),
        };
        Ok(self.progress)
    }
    pub fn finish(self) -> Result<SourcePreparedPickQuery, SourcePickQueryAdmissionError> {
        match self.stage {
            AdmissionStage::Ready(prepared) => Ok(prepared),
            _ => Err(SourcePickQueryAdmissionError::NotReady),
        }
    }
}
impl SourcePreparedPickQuery {
    /// Reuses the sealed incremental bounds for either query representation.
    pub fn source_root_loose_bounds(&self) -> [f64; 6] {
        match &self.kind {
            PreparedKind::Linear(sealed) => sealed.source_root_loose_bounds(),
            PreparedKind::Indexed(index) => index.sealed.source_root_loose_bounds(),
        }
    }
    pub fn matches(&self, published: &Arc<Mesh>) -> bool {
        match &self.kind {
            PreparedKind::Linear(sealed) => sealed.matches(published),
            PreparedKind::Indexed(index) => index.sealed.matches(published),
        }
    }
    pub fn progress(&self) -> SourcePickQueryProgress {
        self.progress
    }
    fn attributes(&self) -> super::SourceAttributePickView<'_> {
        match &self.kind {
            PreparedKind::Linear(sealed) => sealed.borrow(),
            PreparedKind::Indexed(index) => index.sealed.borrow(),
        }
    }
    pub fn sample_with_budget(
        &self,
        contact: crate::SourceOriginalFaceContact,
        budget: &mut SourcePickQueryBudget,
    ) -> Result<
        Result<crate::SourceAttributePickReport, crate::SourceAttributePickError>,
        SourcePickQueryError,
    > {
        budget.attributes.corner_reads = budget
            .attributes
            .corner_reads
            .min(super::MAX_SOURCE_PICK_CORNER_READS);
        self.attributes()
            .sample_with_budget(contact, &mut budget.attributes)
            .map_err(SourcePickQueryError::Ray)
    }
    /// Completed miss is distinct from unsupported domain or exhausted work.
    /// There is no partial-hit result and no next-frame query continuation.
    pub fn ray(
        &self,
        pose: Option<&ObjectAffineF32V1>,
        ray: SceneRay64,
        budget: &mut SourcePickQueryBudget,
    ) -> Result<Option<PreparedSourcePaintHit>, SourcePickQueryError> {
        match &self.kind {
            PreparedKind::Linear(sealed) => sealed
                .borrow()
                .ray(pose, ray, &mut budget.attributes)
                .map_err(SourcePickQueryError::Ray),
            PreparedKind::Indexed(index) => {
                let (_, origin, direction) = local_ray(pose, ray).map_err(query_error)?;
                // The finite-intermediate proof uses these local bounds. The
                // existing small linear API intentionally retains its domain.
                if origin.abs().max_element() > 1_000_000.0 || direction.abs().max_element() > 2.0 {
                    return Err(SourcePickQueryError::OutsideDenseLocalDomain);
                }
                let mut bounded = Budget {
                    nodes: budget.nodes.min(4096),
                    work: SourcePickWorkBudget {
                        face_tests: budget.attributes.face_tests.min(8192),
                        triangle_tests: budget.attributes.triangle_tests.min(8192),
                        corner_reads: budget.attributes.corner_reads,
                    },
                };
                let mut counts = Counts::default();
                let identity = Identity::new(0, 0, 0, 0, pose);
                let result = index.query(
                    &index.mesh,
                    identity,
                    identity,
                    pose,
                    ray,
                    &mut bounded,
                    &mut counts,
                );
                budget.nodes = bounded.nodes;
                budget.attributes = bounded.work;
                result
                    .map(|hit| hit.map(|h| h.prepared))
                    .map_err(query_error)
            }
        }
    }
}

#[cfg(test)]
#[path = "source_pick_query_tests.rs"]
mod query_tests;
