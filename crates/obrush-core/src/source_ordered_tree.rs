//! Independently authored bounded static ordered selection. The reference's
//! current original-face IDs are inputs; this module does not implement its
//! import optimizer, dynamic topology, or ray/contact tie ordering.
use crate::{SourceGeometryError, Vec3, TRIANGLE_CORNER};
use glam::DVec3;
use serde::Serialize;
use std::sync::Arc;

pub const MAX_SOURCE_ORDERED_TREE_BYTES: usize = crate::resource_limits::MAX_ORDERED_TREE_BYTES;
// The fixed preflight bounds face/corner counts, and bytes() bounds live cell
// slots and leaf-vector lengths. Even one byte per entry fits u32 under this
// cap, so compact private indices cannot truncate on native or WASM hosts.
const _: () = assert!(MAX_SOURCE_ORDERED_TREE_BYTES < u32::MAX as usize);
pub const SOURCE_ORDERED_TREE_MAX_DEPTH: u8 = 8;
pub const SOURCE_ORDERED_TREE_LEAF_FACES: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourceLeafHandle {
    pub slot: usize,
    pub serial: u64,
}
#[derive(Debug, Serialize)]
pub struct SourceOrderedSelection {
    pub candidate_faces: Vec<u32>,
    pub selected_vertices: Vec<usize>,
    pub visited_leaves: Vec<SourceLeafHandle>,
}
#[derive(Debug, Serialize)]
pub struct SourceOrderedCell {
    pub handle: SourceLeafHandle,
    pub parent: Option<SourceLeafHandle>,
    pub depth: u8,
    pub split: [f64; 6],
    pub loose: [f64; 6],
    pub children: Option<[SourceLeafHandle; 8]>,
    pub faces: Vec<u32>,
}
#[derive(Debug, Serialize)]
pub struct SourceOrderedTreeSnapshot {
    pub cells: Vec<SourceOrderedCell>,
    pub face_boxes: Vec<[f32; 6]>,
    pub face_centers: Vec<[f32; 3]>,
    pub face_leaves: Vec<SourceLeafHandle>,
    pub face_positions: Vec<usize>,
    pub root_rebuilds: u64,
}

/// Static incident rings are built from ascending original rows using reverse
/// insertion per vertex. Duplicate original corners remain duplicate entries.
#[derive(Debug)]
struct FaceRings {
    starts: Vec<u32>,
    faces: Vec<u32>,
}
impl FaceRings {
    fn build(vertices: usize, faces: &[[u32; 4]]) -> Result<Self, SourceGeometryError> {
        let mut counts = filled(vertices, 0u32)?;
        for &face in faces {
            for v in corners(face) {
                counts[v] = counts[v]
                    .checked_add(1)
                    .ok_or(SourceGeometryError::StorageLimit)?;
            }
        }
        let mut starts = reserved(
            vertices
                .checked_add(1)
                .ok_or(SourceGeometryError::StorageLimit)?,
        )?;
        starts.push(0u32);
        for &count in &counts {
            starts.push(
                starts
                    .last()
                    .copied()
                    .unwrap()
                    .checked_add(count)
                    .ok_or(SourceGeometryError::StorageLimit)?,
            );
        }
        let mut rows = filled(*starts.last().unwrap() as usize, 0u32)?;
        for (i, &face) in faces.iter().enumerate() {
            for v in corners(face) {
                counts[v] -= 1;
                rows[(starts[v] + counts[v]) as usize] = i as u32;
            }
        }
        Ok(Self {
            starts,
            faces: rows,
        })
    }
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + 2 * std::mem::size_of::<usize>()
            + self.starts.capacity() * std::mem::size_of::<u32>()
            + self.faces.capacity() * 4
    }
}
#[derive(Debug)]
struct Cell {
    serial: u64,
    parent: Option<usize>,
    depth: u8,
    split: [f64; 6],
    loose: [f64; 6],
    children: Option<[usize; 8]>,
    faces: Vec<u32>,
}
#[derive(Debug)]
pub(crate) struct OrderedTree {
    cells: Vec<Option<Cell>>,
    free: Vec<usize>,
    // Sum of capacities of the live cells' face vectors, in bytes.
    // Only private tree mutations can change those allocations.
    face_storage_bytes: usize,
    face_boxes: Vec<[f32; 6]>,
    // Each face belongs to a live leaf. Its serial is authoritative in Cell;
    // only external visited handles need a retained serial for ABA rejection.
    face_leaves: Vec<u32>,
    face_positions: Vec<u32>,
    rings: Arc<FaceRings>,
    next_serial: u64,
    rebuilds: u64,
}
impl OrderedTree {
    pub(crate) fn build(
        positions: &[Vec3],
        faces: &[[u32; 4]],
    ) -> Result<Self, SourceGeometryError> {
        Self::build_at(positions, faces, 1, 0, None)
    }
    fn build_at(
        positions: &[Vec3],
        faces: &[[u32; 4]],
        serial: u64,
        rebuilds: u64,
        old_rings: Option<Arc<FaceRings>>,
    ) -> Result<Self, SourceGeometryError> {
        // All fixed arrays are charged before allocating. Node and face-vector
        // growth has a separate progressive check, rather than a guessed tree.
        let fixed = positions
            .len()
            .checked_mul(8)
            .and_then(|n| faces.len().checked_mul(56).and_then(|f| n.checked_add(f)))
            .ok_or(SourceGeometryError::StorageLimit)?;
        if fixed > MAX_SOURCE_ORDERED_TREE_BYTES {
            return Err(SourceGeometryError::StorageLimit);
        }
        let rings = match old_rings {
            Some(r) => r,
            None => Arc::new(FaceRings::build(positions.len(), faces)?),
        };
        let mut boxes = reserved(faces.len())?;
        for &face in faces {
            let (bounds, _) = face_bounds(positions, face)?;
            boxes.push(bounds);
        }
        let (loose, split) = root_bounds(positions)?;
        let mut tree = Self {
            cells: reserved(1)?,
            free: Vec::new(),
            face_storage_bytes: 0,
            face_boxes: boxes,
            face_leaves: filled(faces.len(), 0u32)?,
            face_positions: filled(faces.len(), 0u32)?,
            rings,
            next_serial: serial,
            rebuilds,
        };
        let mut initial = reserved(faces.len())?;
        initial.extend((0..faces.len()).map(|i| i as u32));
        let root = tree.allocate_cell(None, 0, split, loose, initial)?;
        debug_assert_eq!(root, 0);
        tree.build_subtree(root)?;
        tree.check_limit()?;
        Ok(tree)
    }
    pub(crate) fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            .saturating_add(
                self.cells
                    .capacity()
                    .saturating_mul(std::mem::size_of::<Option<Cell>>()),
            )
            .saturating_add(
                self.free
                    .capacity()
                    .saturating_mul(std::mem::size_of::<usize>()),
            )
            .saturating_add(self.face_boxes.capacity().saturating_mul(24))
            .saturating_add(self.face_leaves.capacity().saturating_mul(4))
            .saturating_add(self.face_positions.capacity().saturating_mul(4))
            .saturating_add(self.rings.bytes())
            .saturating_add(self.face_storage_bytes)
    }
    fn face_capacity_sum(cells: &[Option<Cell>]) -> usize {
        cells
            .iter()
            .flatten()
            .map(|c| {
                #[cfg(test)]
                capacity_scan_probe::record();
                c.faces.capacity().saturating_mul(4)
            })
            .fold(0, usize::saturating_add)
    }
    fn replace_face_capacity(&mut self, old: usize, new: usize) -> Result<(), SourceGeometryError> {
        self.face_storage_bytes = self
            .face_storage_bytes
            .checked_sub(
                old.checked_mul(4)
                    .ok_or(SourceGeometryError::StorageLimit)?,
            )
            .and_then(|n| new.checked_mul(4).and_then(|v| n.checked_add(v)))
            .ok_or(SourceGeometryError::StorageLimit)?;
        Ok(())
    }
    fn check_limit(&self) -> Result<(), SourceGeometryError> {
        #[cfg(test)]
        assert_eq!(
            self.face_storage_bytes,
            self.cells
                .iter()
                .flatten()
                .map(|c| c.faces.capacity() * 4)
                .sum::<usize>(),
            "live face allocation accounting"
        );
        if self.bytes() > MAX_SOURCE_ORDERED_TREE_BYTES {
            Err(SourceGeometryError::StorageLimit)
        } else {
            Ok(())
        }
    }
    fn growth(&self, bytes: usize) -> Result<(), SourceGeometryError> {
        if self
            .bytes()
            .checked_add(bytes)
            .is_none_or(|n| n > MAX_SOURCE_ORDERED_TREE_BYTES)
        {
            Err(SourceGeometryError::StorageLimit)
        } else {
            Ok(())
        }
    }
    fn cell(&self, id: usize) -> &Cell {
        self.cells[id].as_ref().expect("live ordered cell")
    }
    fn cell_mut(&mut self, id: usize) -> &mut Cell {
        self.cells[id].as_mut().expect("live ordered cell")
    }
    fn handle(&self, id: usize) -> SourceLeafHandle {
        SourceLeafHandle {
            slot: id,
            serial: self.cell(id).serial,
        }
    }
    fn face_leaf_slot(&self, face: usize) -> usize {
        self.face_leaves[face] as usize
    }
    fn face_leaf_handle(&self, face: usize) -> SourceLeafHandle {
        self.handle(self.face_leaf_slot(face))
    }
    // Derive from the stored historical bounds, never current Mesh positions.
    // Every bound is an exact f32 input endpoint; widening it to f64 therefore
    // reproduces the original midpoint calculation, including signed zero.
    fn face_center(&self, face: usize) -> [f32; 3] {
        bounds_center(self.face_boxes[face])
    }
    fn live(&self, h: SourceLeafHandle) -> bool {
        self.cells
            .get(h.slot)
            .and_then(Option::as_ref)
            .is_some_and(|c| c.serial == h.serial)
    }
    pub(crate) fn root_loose(&self) -> [f64; 6] {
        self.cell(0).loose
    }
    pub(crate) fn root_split(&self) -> [f64; 6] {
        self.cell(0).split
    }
    pub(crate) fn rebuilds(&self) -> u64 {
        self.rebuilds
    }
    pub(crate) fn unused_vertices(&self) -> usize {
        self.rings
            .starts
            .windows(2)
            .filter(|w| w[0] == w[1])
            .count()
    }
    fn allocate_cell(
        &mut self,
        parent: Option<usize>,
        depth: u8,
        split: [f64; 6],
        loose: [f64; 6],
        faces: Vec<u32>,
    ) -> Result<usize, SourceGeometryError> {
        self.growth(
            faces
                .capacity()
                .checked_mul(4)
                .ok_or(SourceGeometryError::StorageLimit)?,
        )?;
        let incoming_capacity = faces.capacity();
        let serial = self.next_serial;
        self.next_serial = serial
            .checked_add(1)
            .ok_or(SourceGeometryError::StorageLimit)?;
        let row = Cell {
            serial,
            parent,
            depth,
            split,
            loose,
            children: None,
            faces,
        };
        let id = if let Some(id) = self.free.pop() {
            self.cells[id] = Some(row);
            id
        } else {
            if self.cells.len() == self.cells.capacity() {
                self.growth(std::mem::size_of::<Option<Cell>>())?;
                self.cells
                    .try_reserve_exact(1)
                    .map_err(|_| SourceGeometryError::Allocation)?;
            }
            let id = self.cells.len();
            self.cells.push(Some(row));
            id
        };
        self.replace_face_capacity(0, incoming_capacity)?;
        self.check_limit()?;
        Ok(id)
    }
    fn build_subtree(&mut self, root: usize) -> Result<(), SourceGeometryError> {
        let mut stack = [0usize; 1 + 7 * SOURCE_ORDERED_TREE_MAX_DEPTH as usize];
        let mut count = 1;
        stack[0] = root;
        while count > 0 {
            count -= 1;
            let id = stack[count];
            if self.cell(id).faces.len() > SOURCE_ORDERED_TREE_LEAF_FACES
                && self.cell(id).depth < SOURCE_ORDERED_TREE_MAX_DEPTH
            {
                let child = self.split_leaf(id)?;
                stack[count..count + 8].copy_from_slice(&child);
                count += 8;
            } else if !self.cell(id).faces.is_empty() {
                let rows = self.cell(id).faces.len();
                let mut loose = empty_box();
                for index in 0..rows {
                    let f = self.cell(id).faces[index] as usize;
                    self.face_leaves[f] = id as u32;
                    self.face_positions[f] = index as u32;
                    expand(&mut loose, self.face_boxes[f].map(f64::from));
                }
                self.expand_to_parents(id, loose);
            }
        }
        Ok(())
    }
    fn split_leaf(&mut self, id: usize) -> Result<[usize; 8], SourceGeometryError> {
        let split = self.cell(id).split;
        let depth = self.cell(id).depth + 1;
        let midpoint = [
            (split[3] + split[0]) * 0.5,
            (split[4] + split[1]) * 0.5,
            (split[5] + split[2]) * 0.5,
        ];
        let mut counts = [0usize; 8];
        for &face in &self.cell(id).faces {
            counts[child_index(self.face_center(face as usize), midpoint)] += 1;
        }
        self.growth(
            self.cell(id)
                .faces
                .len()
                .checked_mul(4)
                .and_then(|n| n.checked_add(8 * std::mem::size_of::<Option<Cell>>()))
                .ok_or(SourceGeometryError::StorageLimit)?,
        )?;
        let mut rows: [Vec<u32>; 8] = std::array::from_fn(|_| Vec::new());
        for i in 0..8 {
            rows[i]
                .try_reserve_exact(counts[i])
                .map_err(|_| SourceGeometryError::Allocation)?;
        }
        for &face in &self.cell(id).faces {
            rows[child_index(self.face_center(face as usize), midpoint)].push(face);
        }
        let splits = child_splits(split);
        // Source construction empties the parent face list; its prior loose
        // box deliberately survives subdivision and later pruning.
        let old_capacity = self.cell(id).faces.capacity();
        self.cell_mut(id).faces = Vec::new();
        self.replace_face_capacity(old_capacity, 0)?;
        let mut children = [0usize; 8];
        for i in 0..8 {
            children[i] = self.allocate_cell(
                Some(id),
                depth,
                splits[i],
                empty_box(),
                std::mem::take(&mut rows[i]),
            )?;
        }
        self.cell_mut(id).children = Some(children);
        Ok(children)
    }
    fn expand_to_parents(&mut self, mut id: usize, b: [f64; 6]) {
        loop {
            if !expand(&mut self.cell_mut(id).loose, b) {
                break;
            }
            match self.cell(id).parent {
                Some(p) => id = p,
                None => break,
            }
        }
    }
    /// Upper bound for managed selection buffers and their fixed stack. The
    /// borrowed tree/state is retained input; no editing clone or rollback is
    /// allocated by this read-only operation.
    fn selection_storage_bytes(&self, vertices: usize) -> Result<usize, SourceGeometryError> {
        if vertices != self.rings.starts.len().saturating_sub(1) {
            return Err(SourceGeometryError::InvalidMesh);
        }
        self.face_boxes
            .len()
            .checked_mul(std::mem::size_of::<u32>())
            .and_then(|n| {
                self.cells
                    .len()
                    .checked_mul(std::mem::size_of::<SourceLeafHandle>())
                    .and_then(|v| n.checked_add(v))
            })
            .and_then(|n| {
                vertices
                    .checked_mul(std::mem::size_of::<usize>() + std::mem::size_of::<bool>())
                    .and_then(|v| n.checked_add(v))
            })
            .and_then(|n| {
                n.checked_add(
                    std::mem::size_of::<SourceOrderedSelection>()
                        + std::mem::size_of::<Vec<bool>>()
                        + std::mem::size_of::<
                            [usize; 1 + 7 * SOURCE_ORDERED_TREE_MAX_DEPTH as usize],
                        >()
                        + 128,
                )
            })
            .ok_or(SourceGeometryError::StorageLimit)
    }
    pub(crate) fn check_selection_budget(
        &self,
        vertices: usize,
        retained: usize,
    ) -> Result<(), SourceGeometryError> {
        self.check_selection_at_limits(
            vertices,
            retained,
            crate::MAX_SOURCE_GEOMETRY_SCRATCH_BYTES,
            crate::MAX_SOURCE_GEOMETRY_OPERATION_BYTES,
        )
    }
    fn check_selection_at_limits(
        &self,
        vertices: usize,
        retained: usize,
        scratch_limit: usize,
        operation_limit: usize,
    ) -> Result<(), SourceGeometryError> {
        let scratch = self.selection_storage_bytes(vertices)?;
        if scratch > scratch_limit
            || retained
                .checked_add(scratch)
                .is_none_or(|n| n > operation_limit)
        {
            Err(SourceGeometryError::StorageLimit)
        } else {
            Ok(())
        }
    }
    pub(crate) fn select(
        &self,
        positions: &[Vec3],
        faces: &[[u32; 4]],
        center: [f64; 3],
        radius_squared: f64,
    ) -> Result<SourceOrderedSelection, SourceGeometryError> {
        if faces.len() != self.face_boxes.len() {
            return Err(SourceGeometryError::InvalidMesh);
        }
        self.check_selection_budget(positions.len(), self.bytes())?;
        let mut candidates = reserved(faces.len())?;
        let mut visited = reserved(self.cells.len())?;
        let mut stack = [0usize; 1 + 7 * SOURCE_ORDERED_TREE_MAX_DEPTH as usize];
        let mut count = 1;
        stack[0] = 0;
        while count > 0 {
            count -= 1;
            let id = stack[count];
            let c = self.cell(id);
            if box_distance(c.loose, center) > radius_squared {
                continue;
            }
            if let Some(children) = c.children {
                stack[count..count + 8].copy_from_slice(&children);
                count += 8;
            } else {
                visited.push(self.handle(id));
                candidates.extend_from_slice(&c.faces);
            }
        }
        let mut seen = filled(positions.len(), false)?;
        let mut selected = reserved(positions.len())?;
        for &f in &candidates {
            for v in corners(faces[f as usize]) {
                if seen[v] {
                    continue;
                }
                seen[v] = true;
                let d = positions[v].to_array().map(f64::from);
                let dx = d[0] - center[0];
                let dy = d[1] - center[1];
                let dz = d[2] - center[2];
                if (dx * dx + dy * dy) + dz * dz < radius_squared {
                    selected.push(v);
                }
            }
        }
        Ok(SourceOrderedSelection {
            candidate_faces: candidates,
            selected_vertices: selected,
            visited_leaves: visited,
        })
    }
    /// Return current original-face rows in authoritative ascending order.
    /// The loose cells and face boxes are updated with every accepted Source
    /// geometry edit; the ray test only prunes impossible rows. Exact triangle
    /// intersection and original tie behavior remain in paint_preparation.
    pub(crate) fn ray_candidates(
        &self,
        origin: DVec3,
        direction: DVec3,
    ) -> Result<Vec<u32>, SourceGeometryError> {
        if !origin.is_finite() || !direction.is_finite() || direction == DVec3::ZERO {
            return Err(SourceGeometryError::InvalidInput);
        }
        let mut candidates = Vec::new();
        let mut stack = [0usize; 1 + 7 * SOURCE_ORDERED_TREE_MAX_DEPTH as usize];
        let mut count = 1;
        stack[0] = 0;
        while count > 0 {
            count -= 1;
            let cell = self.cell(stack[count]);
            if !ray_overlaps_box(origin, direction, cell.loose) {
                continue;
            }
            if let Some(children) = cell.children {
                stack[count..count + 8].copy_from_slice(&children);
                count += 8;
            } else {
                for &face in &cell.faces {
                    if ray_overlaps_box(
                        origin,
                        direction,
                        self.face_boxes[face as usize].map(f64::from),
                    ) {
                        if candidates.len() >= self.face_boxes.len() {
                            return Err(SourceGeometryError::StorageLimit);
                        }
                        candidates
                            .try_reserve(1)
                            .map_err(|_| SourceGeometryError::Allocation)?;
                        candidates.push(face);
                    }
                }
            }
        }
        candidates.sort_unstable();
        Ok(candidates)
    }
    /// Immutable descending current original-face rows; validation remains
    /// the public entry's responsibility. No allocation or mutable access.
    pub(crate) fn incident_faces(&self, vertex: usize) -> Result<&[u32], SourceGeometryError> {
        let next = vertex
            .checked_add(1)
            .ok_or(SourceGeometryError::InvalidInput)?;
        let start = *self
            .rings
            .starts
            .get(vertex)
            .ok_or(SourceGeometryError::InvalidInput)?;
        let end = *self
            .rings
            .starts
            .get(next)
            .ok_or(SourceGeometryError::InvalidInput)?;
        self.rings
            .faces
            .get(start as usize..end as usize)
            .ok_or(SourceGeometryError::InvalidMesh)
    }
    pub(crate) fn affected_faces(
        &self,
        selected: &[usize],
    ) -> Result<Vec<u32>, SourceGeometryError> {
        if selected
            .iter()
            .any(|&v| v >= self.rings.starts.len().saturating_sub(1))
        {
            return Err(SourceGeometryError::InvalidInput);
        }
        // A vertex can repeat in selected, and a face can repeat in its ring.
        // Saturating and clipping gives a safe unique-face upper bound without
        // allocating every face row for a small local operation.
        let upper = selected.iter().fold(0usize, |n, &v| {
            let count = (self.rings.starts[v + 1] - self.rings.starts[v]) as usize;
            n.saturating_add(count).min(self.face_boxes.len())
        });
        let mut seen = filled(self.face_boxes.len(), false)?;
        let mut out = reserved(upper)?;
        for &v in selected {
            for &f in
                &self.rings.faces[self.rings.starts[v] as usize..self.rings.starts[v + 1] as usize]
            {
                if !seen[f as usize] {
                    seen[f as usize] = true;
                    out.push(f);
                }
            }
        }
        Ok(out)
    }
    fn copy_fallible(&self) -> Result<Self, SourceGeometryError> {
        let mut cells = reserved(self.cells.len())?;
        for c in &self.cells {
            cells.push(match c {
                Some(c) => Some(Cell {
                    serial: c.serial,
                    parent: c.parent,
                    depth: c.depth,
                    split: c.split,
                    loose: c.loose,
                    children: c.children,
                    faces: copied(&c.faces)?,
                }),
                None => None,
            });
        }
        // A copy reserves each vector's length; spare capacity may differ.
        // Count its actual allocations once, rather than copying the old sum.
        let face_storage_bytes = Self::face_capacity_sum(&cells);
        let tree = Self {
            cells,
            free: copied(&self.free)?,
            face_storage_bytes,
            face_boxes: copied(&self.face_boxes)?,
            face_leaves: copied(&self.face_leaves)?,
            face_positions: copied(&self.face_positions)?,
            rings: self.rings.clone(),
            next_serial: self.next_serial,
            rebuilds: self.rebuilds,
        };
        tree.check_limit()?;
        Ok(tree)
    }
    pub(crate) fn refreshed(
        &self,
        positions: &[Vec3],
        faces: &[[u32; 4]],
        selected: &[usize],
    ) -> Result<Self, SourceGeometryError> {
        let affected = self.affected_faces(selected)?;
        let mut tree = self.copy_fallible()?;
        let mut moved = reserved(affected.len())?;
        for &face in &affected {
            let f = face as usize;
            let (bounds, center) = face_bounds(positions, faces[f])?;
            tree.face_boxes[f] = bounds;
            let old_leaf = tree.face_leaf_slot(f);
            if !contains_center(tree.cell(old_leaf).split, center) {
                moved.push(face);
                let index = tree.face_positions[f] as usize;
                let last = *tree
                    .cell(old_leaf)
                    .faces
                    .last()
                    .expect("face leaf membership");
                tree.cell_mut(old_leaf).faces[index] = last;
                tree.face_positions[last as usize] = index as u32;
                tree.cell_mut(old_leaf).faces.pop();
            } else {
                tree.expand_to_parents(old_leaf, bounds.map(f64::from));
            }
        }
        for &face in &moved {
            let f = face as usize;
            let b = tree.face_boxes[f];
            let split = tree.cell(0).split;
            // The primary source checks disjointness, not containment. A face
            // straddling the root edge alone does not rebuild the tree.
            if (0..3).any(|c| f64::from(b[c]) > split[c + 3] || f64::from(b[c + 3]) < split[c]) {
                return Self::build_at(
                    positions,
                    faces,
                    tree.next_serial,
                    tree.rebuilds
                        .checked_add(1)
                        .ok_or(SourceGeometryError::StorageLimit)?,
                    Some(tree.rings.clone()),
                );
            }
            let destination = tree.insert_destination(tree.face_center(f), b.map(f64::from));
            let id = destination.unwrap_or(tree.face_leaf_slot(f));
            tree.growth(4)?;
            let old_capacity = tree.cell(id).faces.capacity();
            tree.cell_mut(id)
                .faces
                .try_reserve_exact(1)
                .map_err(|_| SourceGeometryError::Allocation)?;
            tree.replace_face_capacity(old_capacity, tree.cell(id).faces.capacity())?;
            tree.face_positions[f] = tree.cell(id).faces.len() as u32;
            tree.cell_mut(id).faces.push(face);
            tree.face_leaves[f] = id as u32;
        }
        tree.check_limit()?;
        Ok(tree)
    }
    fn insert_destination(&mut self, center: [f32; 3], b: [f64; 6]) -> Option<usize> {
        let mut stack = [0usize; 1 + 7 * SOURCE_ORDERED_TREE_MAX_DEPTH as usize];
        let mut count = 1;
        stack[0] = 0;
        while count > 0 {
            count -= 1;
            let id = stack[count];
            if !contains_center(self.cell(id).split, center) {
                continue;
            }
            expand(&mut self.cell_mut(id).loose, b);
            if let Some(children) = self.cell(id).children {
                stack[count..count + 8].copy_from_slice(&children);
                count += 8;
            } else {
                return Some(id);
            }
        }
        None
    }
    pub(crate) fn balanced(
        &self,
        visited: &[SourceLeafHandle],
    ) -> Result<Option<Self>, SourceGeometryError> {
        let eligible = visited.iter().any(|&h| {
            self.live(h)
                && self.cell(h.slot).children.is_none()
                && (self.cell(h.slot).faces.is_empty()
                    || (self.cell(h.slot).faces.len() > SOURCE_ORDERED_TREE_LEAF_FACES
                        && self.cell(h.slot).depth < SOURCE_ORDERED_TREE_MAX_DEPTH))
        });
        if !eligible {
            return Ok(None);
        }
        let mut tree = self.copy_fallible()?;
        let mut seen = filled(tree.cells.len(), false)?;
        for &h in visited {
            if !tree.live(h)
                || h.slot >= seen.len()
                || seen[h.slot]
                || tree.cell(h.slot).children.is_some()
            {
                continue;
            }
            seen[h.slot] = true;
            if tree.cell(h.slot).faces.is_empty() {
                tree.prune(h.slot)?;
            } else if tree.cell(h.slot).faces.len() > SOURCE_ORDERED_TREE_LEAF_FACES
                && tree.cell(h.slot).depth < SOURCE_ORDERED_TREE_MAX_DEPTH
            {
                tree.build_subtree(h.slot)?;
            }
        }
        tree.check_limit()?;
        Ok((!tree.same_words(self)).then_some(tree))
    }
    fn prune(&mut self, mut id: usize) -> Result<(), SourceGeometryError> {
        while let Some(parent) = self.cell(id).parent {
            let Some(children) = self.cell(parent).children else {
                break;
            };
            if children
                .iter()
                .any(|&c| self.cell(c).children.is_some() || !self.cell(c).faces.is_empty())
            {
                break;
            }
            self.growth(8 * std::mem::size_of::<usize>())?;
            self.free
                .try_reserve_exact(8)
                .map_err(|_| SourceGeometryError::Allocation)?;
            for child in children {
                let old_capacity = self.cell(child).faces.capacity();
                self.cells[child] = None;
                self.replace_face_capacity(old_capacity, 0)?;
                self.free.push(child);
            }
            self.cell_mut(parent).children = None;
            id = parent;
        }
        Ok(())
    }
    pub(crate) fn same_words(&self, other: &Self) -> bool {
        self.next_serial == other.next_serial
            && self.rebuilds == other.rebuilds
            && self.free == other.free
            && self.face_leaves == other.face_leaves
            && self.face_positions == other.face_positions
            && self.face_boxes.len() == other.face_boxes.len()
            && self
                .face_boxes
                .iter()
                .zip(&other.face_boxes)
                .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
            && self.cells.len() == other.cells.len()
            && self
                .cells
                .iter()
                .zip(&other.cells)
                .all(|(a, b)| match (a, b) {
                    (None, None) => true,
                    (Some(a), Some(b)) => {
                        a.serial == b.serial
                            && a.parent == b.parent
                            && a.depth == b.depth
                            && a.split.map(f64::to_bits) == b.split.map(f64::to_bits)
                            && a.loose.map(f64::to_bits) == b.loose.map(f64::to_bits)
                            && a.children == b.children
                            && a.faces == b.faces
                    }
                    _ => false,
                })
    }
    pub(crate) fn snapshot(&self) -> Result<SourceOrderedTreeSnapshot, SourceGeometryError> {
        let mut cells = reserved(self.cells.iter().flatten().count())?;
        for (i, c) in self.cells.iter().enumerate() {
            if let Some(c) = c {
                cells.push(SourceOrderedCell {
                    handle: self.handle(i),
                    parent: c.parent.map(|p| self.handle(p)),
                    depth: c.depth,
                    split: c.split,
                    loose: c.loose,
                    children: c.children.map(|a| a.map(|j| self.handle(j))),
                    faces: copied(&c.faces)?,
                });
            }
        }
        Ok(SourceOrderedTreeSnapshot {
            cells,
            face_boxes: copied(&self.face_boxes)?,
            face_centers: collected(
                (0..self.face_boxes.len()).map(|f| self.face_center(f)),
                self.face_boxes.len(),
            )?,
            face_leaves: collected(
                (0..self.face_leaves.len()).map(|f| self.face_leaf_handle(f)),
                self.face_leaves.len(),
            )?,
            face_positions: collected(
                self.face_positions.iter().map(|&p| p as usize),
                self.face_positions.len(),
            )?,
            root_rebuilds: self.rebuilds,
        })
    }
}
fn child_index(c: [f32; 3], m: [f64; 3]) -> usize {
    match (
        f64::from(c[0]) > m[0],
        f64::from(c[1]) > m[1],
        f64::from(c[2]) > m[2],
    ) {
        (false, false, false) => 0,
        (true, false, false) => 1,
        (true, false, true) => 2,
        (false, false, true) => 3,
        (false, true, false) => 4,
        (true, true, false) => 5,
        (true, true, true) => 6,
        (false, true, true) => 7,
    }
}
fn child_splits(b: [f64; 6]) -> [[f64; 6]; 8] {
    let [x, y, z, max_x, max_y, max_z] = b;
    let dx = (max_x - x) * 0.5;
    let dy = (max_y - y) * 0.5;
    let dz = (max_z - z) * 0.5;
    let mx = (max_x + x) * 0.5;
    let my = (max_y + y) * 0.5;
    let mz = (max_z + z) * 0.5;
    [
        [x, y, z, mx, my, mz],
        [x + dx, y, z, mx + dx, my, mz],
        [mx, my - dy, mz, max_x, max_y - dy, max_z],
        [x, y, z + dz, mx, my, mz + dz],
        [x, y + dy, z, mx, my + dy, mz],
        [mx, my, mz - dz, max_x, max_y, max_z - dz],
        [mx, my, mz, max_x, max_y, max_z],
        [mx - dx, my, mz, max_x - dx, max_y, max_z],
    ]
}
fn contains_center(b: [f64; 6], c: [f32; 3]) -> bool {
    (0..3).all(|i| f64::from(c[i]) > b[i] && f64::from(c[i]) <= b[i + 3])
}
fn empty_box() -> [f64; 6] {
    [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ]
}
fn ray_overlaps_box(origin: DVec3, direction: DVec3, bounds: [f64; 6]) -> bool {
    if (0..3).any(|axis| bounds[axis] > bounds[axis + 3]) {
        return false;
    }
    let scale = origin.abs().max_element().max(
        bounds
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs())),
    );
    let slack = 1e-7 + scale * 1e-12;
    let mut near = 0.0_f64;
    let mut far = f64::INFINITY;
    for axis in 0..3 {
        let lower = bounds[axis] - slack;
        let upper = bounds[axis + 3] + slack;
        if direction[axis] == 0.0 {
            if origin[axis] < lower || origin[axis] > upper {
                return false;
            }
            continue;
        }
        let a = (lower - origin[axis]) / direction[axis];
        let b = (upper - origin[axis]) / direction[axis];
        near = near.max(a.min(b));
        far = far.min(a.max(b));
        if near > far {
            return false;
        }
    }
    true
}
fn expand(to: &mut [f64; 6], b: [f64; 6]) -> bool {
    let mut changed = false;
    for c in 0..3 {
        if b[c] < to[c] {
            to[c] = b[c];
            changed = true;
        }
        if b[c + 3] > to[c + 3] {
            to[c + 3] = b[c + 3];
            changed = true;
        }
    }
    changed
}
fn box_distance(b: [f64; 6], p: [f64; 3]) -> f64 {
    let delta: [f64; 3] = std::array::from_fn(|c| {
        if b[c] > p[c] {
            b[c] - p[c]
        } else if b[c + 3] < p[c] {
            b[c + 3] - p[c]
        } else {
            0.
        }
    });
    (delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]
}
fn root_bounds(p: &[Vec3]) -> Result<([f64; 6], [f64; 6]), SourceGeometryError> {
    let mut b = empty_box();
    for v in p {
        let v = v.to_array().map(f64::from);
        expand(&mut b, [v[0], v[1], v[2], v[0], v[1], v[2]]);
    }
    if b.iter().any(|x| !x.is_finite()) {
        return Err(SourceGeometryError::InvalidMesh);
    }
    let d = [b[3] - b[0], b[4] - b[1], b[5] - b[2]];
    let off = ((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]).sqrt() * 0.2;
    for c in 0..3 {
        if d[c] < 1e-16 {
            b[c] -= off;
            b[c + 3] += off;
        }
    }
    let mut split = b;
    for c in 0..3 {
        split[c] -= d[c] * 0.3;
        split[c + 3] += d[c] * 0.3;
    }
    Ok((b, split))
}
fn face_bounds(p: &[Vec3], face: [u32; 4]) -> Result<([f32; 6], [f32; 3]), SourceGeometryError> {
    let mut b = empty_box();
    for v in corners(face) {
        let a = p
            .get(v)
            .ok_or(SourceGeometryError::InvalidMesh)?
            .to_array()
            .map(f64::from);
        if a.iter().any(|n| !n.is_finite()) {
            return Err(SourceGeometryError::NonfiniteCandidate);
        }
        expand(&mut b, [a[0], a[1], a[2], a[0], a[1], a[2]]);
    }
    let center = std::array::from_fn(|c| ((b[c] + b[c + 3]) * 0.5) as f32);
    let b = b.map(|v| v as f32);
    if b.iter().chain(&center).any(|v| !v.is_finite()) {
        return Err(SourceGeometryError::NonfiniteCandidate);
    }
    Ok((b, center))
}
fn bounds_center(bounds: [f32; 6]) -> [f32; 3] {
    std::array::from_fn(|c| ((f64::from(bounds[c]) + f64::from(bounds[c + 3])) * 0.5) as f32)
}
fn collected<T>(
    values: impl Iterator<Item = T>,
    count: usize,
) -> Result<Vec<T>, SourceGeometryError> {
    let mut out = reserved(count)?;
    out.extend(values);
    Ok(out)
}
fn corners(f: [u32; 4]) -> impl Iterator<Item = usize> {
    f.into_iter()
        .filter(|&v| v != TRIANGLE_CORNER)
        .map(|v| v as usize)
}
fn reserved<T>(n: usize) -> Result<Vec<T>, SourceGeometryError> {
    let mut a = Vec::new();
    a.try_reserve_exact(n)
        .map_err(|_| SourceGeometryError::Allocation)?;
    Ok(a)
}
fn filled<T: Clone>(n: usize, v: T) -> Result<Vec<T>, SourceGeometryError> {
    let mut a = reserved(n)?;
    a.resize(n, v);
    Ok(a)
}
fn copied<T: Copy>(a: &[T]) -> Result<Vec<T>, SourceGeometryError> {
    let mut b = reserved(a.len())?;
    b.extend_from_slice(a);
    Ok(b)
}

#[cfg(test)]
mod ray_candidate_tests {
    use super::*;

    #[test]
    fn current_face_bounds_keep_deformed_hits_and_original_order() {
        let mut positions = Vec::new();
        let mut faces = Vec::new();
        for y in 0..20 {
            for x in 0..20 {
                let start = positions.len() as u32;
                let base = Vec3::new(x as f32 * 2.0, y as f32 * 2.0, 0.0);
                positions.extend([base, base + Vec3::X, base + Vec3::Y]);
                faces.push([start, start + 1, start + 2, TRIANGLE_CORNER]);
            }
        }
        let tree = OrderedTree::build(&positions, &faces).unwrap();
        for face in [0, 37, 119, 201, 399] {
            let base = positions[faces[face][0] as usize].as_dvec3();
            let origin = base + DVec3::new(0.2, 0.2, 10.0);
            let candidates = tree.ray_candidates(origin, -DVec3::Z).unwrap();
            assert!(candidates.binary_search(&(face as u32)).is_ok());
            assert!(candidates.len() < faces.len());
            assert!(candidates.windows(2).all(|pair| pair[0] < pair[1]));
        }
        positions[0].z = 5.0;
        let current = tree.refreshed(&positions, &faces, &[0]).unwrap();
        let candidates = current
            .ray_candidates(DVec3::new(0.1, 0.1, 10.0), -DVec3::Z)
            .unwrap();
        assert!(candidates.binary_search(&0).is_ok());
        assert!(current.root_loose()[5] >= 5.0);
        assert!(current
            .ray_candidates(DVec3::new(100., 100., 10.), -DVec3::Z)
            .unwrap()
            .is_empty());
    }
}

#[cfg(test)]
mod ordered_update_tests {
    use super::*;
    fn fixture(centers: &[[f32; 3]]) -> (Vec<Vec3>, Vec<[u32; 4]>) {
        let mut p = Vec::new();
        let mut f = Vec::new();
        for &[x, y, z] in centers {
            let i = p.len() as u32;
            p.extend([
                Vec3::new(x - 0.25, y - 0.25, z),
                Vec3::new(x + 0.25, y - 0.25, z),
                Vec3::new(x, y + 0.25, z),
            ]);
            f.push([i, i + 1, i + 2, TRIANGLE_CORNER]);
        }
        (p, f)
    }
    fn micro() -> (Vec<Vec3>, Vec<[u32; 4]>) {
        let mut centers = vec![[-4., -4., -4.]; 3];
        centers.extend([[4., 4., 4.]; 2]);
        centers.extend([[-4., 4., 4.]; 96]);
        fixture(&centers)
    }
    fn translate(p: &mut [Vec3], f: usize, delta: Vec3) {
        for v in &mut p[3 * f..3 * f + 3] {
            *v += delta;
        }
    }
    #[test]
    fn o08_live_swap_last_and_ordered_append_literal() {
        let (mut p, f) = micro();
        let tree = OrderedTree::build(&p, &f).unwrap();
        let old = tree.face_leaf_slot(0);
        let dest = tree.face_leaf_slot(3);
        assert_eq!(tree.cell(old).faces, [0, 1, 2]);
        assert_eq!(tree.cell(dest).faces, [3, 4]);
        translate(&mut p, 0, Vec3::splat(8.));
        translate(&mut p, 1, Vec3::splat(8.));
        let next = tree.refreshed(&p, &f, &[0, 3]).unwrap();
        assert_eq!(next.cell(old).faces, [2]);
        assert_eq!(next.cell(dest).faces, [3, 4, 0, 1]);
        assert_eq!(next.face_positions[2], 0);
        assert_eq!(next.face_positions[0], 2);
        assert_eq!(next.face_positions[1], 3);
        assert_eq!(tree.cell(old).faces, [0, 1, 2]);
        assert_eq!(tree.cell(dest).faces, [3, 4]);
    }
    #[test]
    fn o09_straddling_moved_face_does_not_rebuild_but_disjoint_does() {
        let (mut p, f) = micro();
        let tree = OrderedTree::build(&p, &f).unwrap();
        let old_split = tree.root_split();
        p[0].x = 10.;
        let next = tree.refreshed(&p, &f, &[0]).unwrap();
        assert_eq!(next.rebuilds, 0);
        assert_eq!(next.root_split(), old_split);
        assert!(next.cell(0).loose[3] >= 10.);
        assert_ne!(next.face_leaves[0], tree.face_leaves[0]);
        let (mut p, f) = micro();
        for v in &mut p[0..3] {
            v.x += 20.;
        }
        let rebuilt = tree.refreshed(&p, &f, &[0]).unwrap();
        assert_eq!(rebuilt.rebuilds, 1);
        assert_ne!(rebuilt.root_split(), old_split);
        assert!(rebuilt.next_serial > tree.next_serial);
        assert_eq!(
            rebuilt.face_boxes[0],
            [15.75, -4.25, -4., 16.25, -3.75, -4.]
        );
    }
    #[test]
    fn o10_loose_bounds_expand_without_shrink_and_root_stays() {
        let (mut p, f) = micro();
        let tree = OrderedTree::build(&p, &f).unwrap();
        let root = tree.cell(0).loose;
        p[0].x -= 0.5;
        let wide = tree.refreshed(&p, &f, &[0]).unwrap();
        assert!(wide.cell(0).loose[0] < root[0]);
        p[0].x += 0.5;
        let back = wide.refreshed(&p, &f, &[0]).unwrap();
        assert_eq!(back.cell(0).loose, wide.cell(0).loose);
        assert_eq!(back.root_split(), tree.root_split());
        assert_eq!(back.rebuilds, 0);
    }
    #[test]
    fn o11_overfull_destination_splits_only_on_visited_end_balance() {
        let mut centers = vec![[-4., -4., -4.]; 60];
        centers.extend([[4., 4., 4.]; 41]);
        let (mut p, f) = fixture(&centers);
        let tree = OrderedTree::build(&p, &f).unwrap();
        let visits = tree.select(&p, &f, [0.; 3], 400.).unwrap().visited_leaves;
        let dest = tree.face_leaf_slot(60);
        let selected: Vec<usize> = (0..60).map(|i| 3 * i).collect();
        for i in 0..60 {
            translate(&mut p, i, Vec3::splat(8.));
        }
        let next = tree.refreshed(&p, &f, &selected).unwrap();
        assert_eq!(next.cell(dest).faces.len(), 101);
        assert!(next.cell(dest).children.is_none());
        assert!(next.balanced(&[]).unwrap().is_none());
        let balanced = next.balanced(&visits).unwrap().unwrap();
        assert!(balanced.cell(dest).children.is_some());
        assert_eq!(
            balanced.cells.iter().flatten().map(|c| c.depth).max(),
            Some(8)
        );
        assert_eq!(balanced.rebuilds, 0);
        assert_eq!(next.cell(dest).faces.len(), 101); // borrowed retained state was not changed.
    }
    #[test]
    fn o11_empty_pruning_keeps_loose_history_and_deleted_handle_is_not_live() {
        let mut centers = vec![[-8., -8., -8.]; 101];
        centers.push([8., 8., 8.]);
        let (mut p, f) = fixture(&centers);
        let tree = OrderedTree::build(&p, &f).unwrap();
        let visits = tree.select(&p, &f, [0.; 3], 1600.).unwrap().visited_leaves;
        let old_handle = tree.face_leaf_handle(0);
        let root_loose = tree.cell(0).loose;
        let selected: Vec<usize> = (0..101).map(|i| 3 * i).collect();
        for i in 0..101 {
            translate(&mut p, i, Vec3::splat(16.));
        }
        let next = tree.refreshed(&p, &f, &selected).unwrap();
        let balanced = next.balanced(&visits).unwrap().unwrap();
        assert!(!balanced.live(old_handle));
        assert_eq!(balanced.cell(0).loose, root_loose);
        assert_eq!(balanced.rebuilds, 0);
        assert!(balanced.cells.iter().flatten().all(|c| c.depth <= 8));
        assert!(balanced.balanced(&[old_handle]).unwrap().is_none());
    }
    #[test]
    fn lower_split_boundary_failed_reinsert_appends_to_old_leaf() {
        let (mut p, f) = micro();
        // Unused bounding witnesses give exact [-8,8] root X split; the lower
        // boundary itself is representable in both Number and Float32.
        p.extend([Vec3::new(-5., 0., 0.), Vec3::new(5., 0., 0.)]);
        let tree = OrderedTree::build(&p, &f).unwrap();
        let old = tree.face_leaf_slot(0);
        assert_eq!(tree.root_split()[0], -8.);
        let min = -8f32;
        // Center is exactly root lower boundary; wider face straddles it, so
        // disjoint rebuild is false while strict (lower,upper] insertion fails.
        p[0].x = min - 0.25;
        p[1].x = min + 0.25;
        p[2].x = min;
        let next = tree.refreshed(&p, &f, &[0]).unwrap();
        assert_eq!(next.rebuilds, 0);
        assert_eq!(next.cell(old).faces, [2, 1, 0]);
        assert_eq!(next.face_leaf_slot(0), old);
    }
    #[test]
    fn nonfinite_refresh_rejects_without_changing_original_tree() {
        let (mut p, f) = micro();
        let tree = OrderedTree::build(&p, &f).unwrap();
        let before = tree.snapshot().unwrap();
        p[0].x = f32::NAN;
        assert_eq!(
            tree.refreshed(&p, &f, &[0]).unwrap_err(),
            SourceGeometryError::NonfiniteCandidate
        );
        assert_eq!(tree.face_boxes, before.face_boxes);
        assert_eq!(tree.snapshot().unwrap().face_leaves, before.face_leaves);
        assert_eq!(
            tree.snapshot().unwrap().face_positions,
            before.face_positions
        );
    }
    #[test]
    fn actual_node_face_ring_capacities_and_arc_are_charged() {
        let (p, f) = micro();
        let tree = OrderedTree::build(&p, &f).unwrap();
        let minimum = std::mem::size_of::<OrderedTree>()
            + tree.cells.capacity() * std::mem::size_of::<Option<Cell>>()
            + f.len() * (24 + 4 + 4)
            + tree.rings.bytes()
            + f.len() * 4;
        assert!(tree.bytes() >= minimum);
        assert!(tree.bytes() <= MAX_SOURCE_ORDERED_TREE_BYTES);
        assert_eq!(
            tree.growth(MAX_SOURCE_ORDERED_TREE_BYTES).unwrap_err(),
            SourceGeometryError::StorageLimit
        );
        let clone = tree.copy_fallible().unwrap();
        assert!(tree.same_words(&clone));
        assert!(Arc::ptr_eq(&tree.rings, &clone.rings));
    }
}

#[cfg(test)]
mod capacity_scan_probe {
    std::thread_local! { static VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
    pub(super) fn record() {
        VISITS.with(|v| v.set(v.get() + 1));
    }
    pub(super) fn reset() {
        VISITS.with(|v| v.set(0));
    }
    pub(super) fn visits() -> usize {
        VISITS.with(|v| v.get())
    }
}
#[cfg(test)]
mod accounting_performance_regression {
    use super::*;
    #[test]
    fn retained_bytes_and_growth_do_not_walk_existing_cells() {
        let mesh = crate::Mesh::sphere(3);
        let faces: Vec<_> = mesh
            .triangles
            .iter()
            .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
            .collect();
        let tree = OrderedTree::build(&mesh.positions, &faces).unwrap();
        assert!(tree.cells.len() > 8);
        capacity_scan_probe::reset();
        for _ in 0..128 {
            assert!(tree.bytes() > 0);
            tree.growth(0).unwrap();
        }
        assert_eq!(
            capacity_scan_probe::visits(),
            0,
            "Repeated storage queries must not rescan all live cells"
        );
    }
}

#[cfg(test)]
mod capacity_accounting_regressions {
    use super::*;
    fn assert_exact(tree: &OrderedTree) {
        let actual = tree
            .cells
            .iter()
            .flatten()
            .map(|c| c.faces.capacity() * 4)
            .sum::<usize>();
        assert_eq!(tree.face_storage_bytes, actual);
        let complete = std::mem::size_of::<OrderedTree>()
            + tree.cells.capacity() * std::mem::size_of::<Option<Cell>>()
            + tree.free.capacity() * std::mem::size_of::<usize>()
            + tree.face_boxes.capacity() * 24
            + tree.face_leaves.capacity() * 4
            + tree.face_positions.capacity() * 4
            + tree.rings.bytes()
            + actual;
        assert_eq!(tree.bytes(), complete);
        assert!(tree
            .growth(MAX_SOURCE_ORDERED_TREE_BYTES - complete)
            .is_ok());
        assert_eq!(
            tree.growth(MAX_SOURCE_ORDERED_TREE_BYTES - complete + 1),
            Err(SourceGeometryError::StorageLimit)
        );
        assert_eq!(
            tree.growth(usize::MAX),
            Err(SourceGeometryError::StorageLimit)
        );
    }
    #[test]
    fn exact_allocations_survive_copy_migration_balance_and_rebuild() {
        let mut mesh = crate::Mesh::sphere(3);
        let faces: Vec<_> = mesh
            .triangles
            .iter()
            .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
            .collect();
        let original = OrderedTree::build(&mesh.positions, &faces).unwrap();
        assert_exact(&original);
        let mut tree = original.copy_fallible().unwrap();
        assert_exact(&tree);
        assert!(original.same_words(&tree));
        let selected: Vec<_> = (0..mesh.positions.len()).filter(|i| i % 3 == 0).collect();
        for offset in [0.01, 0.2, -0.4, 0.7, -0.5, 0.0] {
            let visits = tree
                .select(&mesh.positions, &faces, [0.; 3], 100.)
                .unwrap()
                .visited_leaves;
            for &v in &selected {
                mesh.positions[v].x += offset;
            }
            tree = tree.refreshed(&mesh.positions, &faces, &selected).unwrap();
            assert_exact(&tree);
            if let Some(next) = tree.balanced(&visits).unwrap() {
                tree = next;
                assert_exact(&tree);
            }
        }
        for v in &mut mesh.positions {
            v.x += 10.;
        }
        let all: Vec<_> = (0..mesh.positions.len()).collect();
        let rebuilt = tree.refreshed(&mesh.positions, &faces, &all).unwrap();
        assert!(rebuilt.rebuilds > tree.rebuilds);
        assert_exact(&rebuilt);
    }
}

#[cfg(test)]
mod compact_storage_regressions {
    use super::*;
    #[test]
    fn historical_bound_centers_match_original_number_midpoints_including_zero_bits() {
        let fixed = [
            0.0,
            -0.0,
            f32::from_bits(1),
            -f32::from_bits(1),
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::MAX,
            -f32::MAX,
            1.0,
            -1.0,
        ];
        let mut seed = 0x7e5a_931du32;
        for i in 0..4096 {
            let mut points = [Vec3::ZERO; 4];
            for p in &mut points {
                for c in 0..3 {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let value = if i < fixed.len() {
                        fixed[i]
                    } else {
                        f32::from_bits(seed)
                    };
                    p[c] = if value.is_finite() { value } else { -0.0 };
                }
            }
            for face in [
                [0, 1, 2, TRIANGLE_CORNER],
                [0, 1, 2, 3],
                [3, 3, 3, TRIANGLE_CORNER],
            ] {
                let (bounds, original) = face_bounds(&points, face).unwrap();
                assert_eq!(
                    bounds_center(bounds).map(f32::to_bits),
                    original.map(f32::to_bits)
                );
            }
        }
    }
    #[test]
    fn partial_history_refresh_preserves_unselected_historical_boxes_and_centers() {
        let mut points = vec![
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            Vec3::new(10., 0., 0.),
            Vec3::new(11., 0., 0.),
            Vec3::new(10., 1., 0.),
        ];
        let faces = [[0, 1, 2, TRIANGLE_CORNER], [3, 4, 5, TRIANGLE_CORNER]];
        let tree = OrderedTree::build(&points, &faces).unwrap();
        let before = tree.snapshot().unwrap();
        points[0].z = 0.25;
        for point in &mut points[3..] {
            point.z += 2.0;
        }
        let next = tree.refreshed(&points, &faces, &[0]).unwrap();
        let after = next.snapshot().unwrap();
        assert_eq!(
            after.face_boxes[1].map(f32::to_bits),
            before.face_boxes[1].map(f32::to_bits)
        );
        assert_eq!(
            after.face_centers[1].map(f32::to_bits),
            before.face_centers[1].map(f32::to_bits)
        );
        assert_ne!(
            after.face_boxes[1],
            face_bounds(&points, faces[1]).unwrap().0
        );
        assert_eq!(
            after.face_centers[0].map(f32::to_bits),
            face_bounds(&points, faces[0]).unwrap().1.map(f32::to_bits)
        );
        for h in after.face_leaves {
            assert!(next.live(h));
        }
        assert_eq!(tree.snapshot().unwrap().face_boxes, before.face_boxes);
    }
    #[test]
    fn compact_incident_offsets_keep_duplicate_corners_and_large_face_ids() {
        let faces = vec![[0, 1, 2, 0]; 70_001];
        let rings = FaceRings::build(4, &faces).unwrap();
        assert_eq!(rings.starts, [0, 140_002, 210_003, 280_004, 280_004]);
        assert_eq!(
            &rings.faces[..6],
            [70_000, 70_000, 69_999, 69_999, 69_998, 69_998]
        );
        assert_eq!(&rings.faces[140_002..140_005], [70_000, 69_999, 69_998]);
        assert_eq!(rings.faces[280_003], 0);
        assert_eq!(
            rings.bytes(),
            std::mem::size_of::<FaceRings>()
                + 2 * std::mem::size_of::<usize>()
                + (rings.starts.capacity() + rings.faces.capacity()) * 4
        );
    }
}

#[cfg(test)]
mod selection_workspace_regressions {
    use super::*;
    #[test]
    fn read_selection_has_exact_separate_workspace_and_combined_limit_boundaries() {
        let mesh = crate::Mesh::sphere(2);
        let faces: Vec<_> = mesh
            .triangles
            .iter()
            .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
            .collect();
        let tree = OrderedTree::build(&mesh.positions, &faces).unwrap();
        let scratch = tree.selection_storage_bytes(mesh.positions.len()).unwrap();
        let retained = tree.bytes();
        assert!(tree
            .check_selection_at_limits(mesh.positions.len(), retained, scratch, retained + scratch)
            .is_ok());
        assert_eq!(
            tree.check_selection_at_limits(
                mesh.positions.len(),
                retained,
                scratch - 1,
                retained + scratch
            ),
            Err(SourceGeometryError::StorageLimit)
        );
        assert_eq!(
            tree.check_selection_at_limits(
                mesh.positions.len(),
                retained,
                scratch,
                retained + scratch - 1
            ),
            Err(SourceGeometryError::StorageLimit)
        );
        assert_eq!(
            tree.check_selection_at_limits(mesh.positions.len(), usize::MAX, scratch, usize::MAX),
            Err(SourceGeometryError::StorageLimit)
        );
        assert_eq!(
            tree.selection_storage_bytes(mesh.positions.len() + 1),
            Err(SourceGeometryError::InvalidMesh)
        );
        assert_eq!(
            tree.select(&mesh.positions, &faces[..faces.len() - 1], [0.; 3], 1.)
                .unwrap_err(),
            SourceGeometryError::InvalidMesh
        );
    }
    #[test]
    fn local_face_rows_reserve_incident_upper_bound_instead_of_all_faces() {
        let mesh = crate::Mesh::sphere(3);
        let faces: Vec<_> = mesh
            .triangles
            .iter()
            .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER])
            .collect();
        let tree = OrderedTree::build(&mesh.positions, &faces).unwrap();
        let expected = tree.incident_faces(0).unwrap();
        let rows = tree.affected_faces(&[0]).unwrap();
        assert_eq!(rows, expected);
        assert!(rows.capacity() <= expected.len() && rows.capacity() < faces.len());
        let repeated = tree.affected_faces(&[0, 0, 0]).unwrap();
        assert_eq!(repeated, expected);
        assert!(repeated.capacity() <= expected.len() * 3);
        assert_eq!(tree.affected_faces(&[]).unwrap().capacity(), 0);
        assert_eq!(
            tree.affected_faces(&[usize::MAX]).unwrap_err(),
            SourceGeometryError::InvalidInput
        );
    }
}
