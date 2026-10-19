//! Exact, private ordered-tree transfer. This is intentionally stricter than
//! the tree's hot-path invariants because bytes may arrive from another realm.
use super::*;
use crate::source_worker_warm_io::{Reader, Writer};

fn push_words<const N: usize>(out: &mut Writer, values: [f64; N]) -> Result<(), String> {
    for value in values {
        out.u64(value.to_bits())?;
    }
    Ok(())
}
fn read_words<const N: usize>(input: &mut Reader<'_>) -> Result<[f64; N], String> {
    let mut words = [0.0; N];
    for value in &mut words {
        *value = f64::from_bits(input.u64()?);
    }
    Ok(words)
}
fn count(input: &mut Reader<'_>, maximum: usize) -> Result<usize, String> {
    let n = input.usize()?;
    if n > maximum {
        return Err("Source warm tree count exceeds bound".into());
    }
    Ok(n)
}
fn reserve<T>(n: usize) -> Result<Vec<T>, String> {
    let mut out = Vec::new();
    out.try_reserve_exact(n)
        .map_err(|_| "Cannot reserve Source warm tree")?;
    Ok(out)
}
fn flags(n: usize) -> Result<Vec<bool>, String> {
    let mut out = reserve(n)?;
    out.resize(n, false);
    Ok(out)
}
fn bounds_valid(bounds: [f64; 6], empty_allowed: bool) -> bool {
    (0..3).all(|axis| {
        (bounds[axis].is_finite()
            && bounds[axis + 3].is_finite()
            && bounds[axis] <= bounds[axis + 3])
            || (empty_allowed
                && bounds[axis] == f64::INFINITY
                && bounds[axis + 3] == f64::NEG_INFINITY)
    })
}

impl OrderedTree {
    pub(crate) fn encode_warm(&self, out: &mut Writer) -> Result<(), String> {
        out.u64(self.next_serial)?;
        out.u64(self.rebuilds)?;
        out.usize(self.cells.len())?;
        for cell in &self.cells {
            match cell {
                None => out.u8(0)?,
                Some(cell) => {
                    out.u8(1)?;
                    out.u64(cell.serial)?;
                    out.u32(cell.parent.map_or(u32::MAX, |p| p as u32))?;
                    out.u8(cell.depth)?;
                    push_words(out, cell.split)?;
                    push_words(out, cell.loose)?;
                    if let Some(children) = cell.children {
                        out.u8(1)?;
                        for child in children {
                            out.usize(child)?;
                        }
                    } else {
                        out.u8(0)?;
                    }
                    out.usize(cell.faces.len())?;
                    for &face in &cell.faces {
                        out.u32(face)?;
                    }
                }
            }
        }
        out.usize(self.free.len())?;
        for &slot in &self.free {
            out.usize(slot)?;
        }
        out.usize(self.face_boxes.len())?;
        for bounds in &self.face_boxes {
            for &word in bounds {
                out.u32(word.to_bits())?;
            }
        }
        out.usize(self.face_leaves.len())?;
        for &slot in &self.face_leaves {
            out.u32(slot)?;
        }
        out.usize(self.face_positions.len())?;
        for &position in &self.face_positions {
            out.u32(position)?;
        }
        out.usize(self.rings.starts.len())?;
        for &start in &self.rings.starts {
            out.u32(start)?;
        }
        out.usize(self.rings.faces.len())?;
        for &face in &self.rings.faces {
            out.u32(face)?;
        }
        Ok(())
    }

    pub(crate) fn decode_warm(
        input: &mut Reader<'_>,
        positions: &[Vec3],
        faces: &[[u32; 4]],
    ) -> Result<Self, String> {
        let vertices = positions.len();
        let face_count = faces.len();
        let next_serial = input.u64()?;
        let rebuilds = input.u64()?;
        let max_cells = MAX_SOURCE_ORDERED_TREE_BYTES / std::mem::size_of::<Option<Cell>>();
        let cell_count = count(input, max_cells)?;
        if cell_count == 0
            || cell_count > input_size_cell_limit(face_count)?
            || cell_count > input.remaining_len()
        {
            return Err("Source warm tree cell count exceeds mesh bound".into());
        }
        let mut cells = reserve(cell_count)?;
        let mut total_face_rows = 0usize;
        let mut face_storage_bytes = 0usize;
        for _ in 0..cell_count {
            match input.u8()? {
                0 => cells.push(None),
                1 => {
                    let serial = input.u64()?;
                    let raw_parent = input.u32()?;
                    let parent = (raw_parent != u32::MAX).then_some(raw_parent as usize);
                    let depth = input.u8()?;
                    let split = read_words(input)?;
                    let loose = read_words(input)?;
                    let children = match input.u8()? {
                        0 => None,
                        1 => {
                            let mut children = [0usize; 8];
                            for slot in &mut children {
                                *slot = input.usize()?;
                            }
                            Some(children)
                        }
                        _ => return Err("Invalid Source warm tree children tag".into()),
                    };
                    let len = count(input, face_count)?;
                    total_face_rows = total_face_rows
                        .checked_add(len)
                        .ok_or("Source warm face count overflow")?;
                    if total_face_rows > face_count {
                        return Err("Duplicate Source warm face rows".into());
                    }
                    let mut rows = reserve(len)?;
                    for _ in 0..len {
                        rows.push(input.u32()?);
                    }
                    face_storage_bytes = face_storage_bytes
                        .checked_add(rows.capacity() * 4)
                        .ok_or("Source warm tree storage overflow")?;
                    cells.push(Some(Cell {
                        serial,
                        parent,
                        depth,
                        split,
                        loose,
                        children,
                        faces: rows,
                    }));
                }
                _ => return Err("Invalid Source warm tree cell tag".into()),
            }
        }
        let free_count = count(input, cell_count)?;
        let mut free = reserve(free_count)?;
        for _ in 0..free_count {
            free.push(input.usize()?);
        }
        if count(input, face_count)? != face_count {
            return Err("Source warm face-box count differs from mesh".into());
        }
        let mut face_boxes = reserve(face_count)?;
        for _ in 0..face_count {
            let mut bounds = [0.0f32; 6];
            for word in &mut bounds {
                *word = f32::from_bits(input.u32()?);
            }
            face_boxes.push(bounds);
        }
        if count(input, face_count)? != face_count {
            return Err("Source warm leaf count differs from mesh".into());
        }
        let mut face_leaves = reserve(face_count)?;
        for _ in 0..face_count {
            face_leaves.push(input.u32()?);
        }
        if count(input, face_count)? != face_count {
            return Err("Source warm face-position count differs from mesh".into());
        }
        let mut face_positions = reserve(face_count)?;
        for _ in 0..face_count {
            face_positions.push(input.u32()?);
        }
        let starts_count = vertices
            .checked_add(1)
            .ok_or("Source warm ring count overflow")?;
        if count(input, starts_count)? != starts_count {
            return Err("Source warm ring count differs from mesh".into());
        }
        let mut starts = reserve(starts_count)?;
        for _ in 0..starts_count {
            starts.push(input.u32()?);
        }
        let max_ring_rows = face_count
            .checked_mul(4)
            .ok_or("Source warm ring count overflow")?;
        let ring_count = count(input, max_ring_rows)?;
        let mut ring_faces = reserve(ring_count)?;
        for _ in 0..ring_count {
            ring_faces.push(input.u32()?);
        }
        let tree = Self {
            cells,
            free,
            face_storage_bytes,
            face_boxes,
            face_leaves,
            face_positions,
            rings: Arc::new(FaceRings {
                starts,
                faces: ring_faces,
            }),
            next_serial,
            rebuilds,
        };
        tree.validate_warm(positions, faces)?;
        Ok(tree)
    }

    fn validate_warm(&self, positions: &[Vec3], faces: &[[u32; 4]]) -> Result<(), String> {
        self.check_limit().map_err(|e| e.to_string())?;
        let Some(root) = self.cells.first().and_then(Option::as_ref) else {
            return Err("Missing Source warm root".into());
        };
        if root.parent.is_some() || root.depth != 0 || self.next_serial == 0 {
            return Err("Invalid Source warm root or serial".into());
        }
        let expected_rings = FaceRings::build(positions.len(), faces).map_err(|e| e.to_string())?;
        if self.rings.starts != expected_rings.starts || self.rings.faces != expected_rings.faces {
            return Err("Source warm incident rings differ from mesh".into());
        }
        for (index, &face) in faces.iter().enumerate() {
            let expected = face_bounds(positions, face).map_err(|e| e.to_string())?.0;
            if self.face_boxes[index].map(f32::to_bits) != expected.map(f32::to_bits) {
                return Err("Source warm face bounds differ from mesh".into());
            }
        }
        let mut free_seen = flags(self.cells.len())?;
        for &slot in &self.free {
            if slot == 0
                || slot >= self.cells.len()
                || free_seen[slot]
                || self.cells[slot].is_some()
            {
                return Err("Invalid Source warm free slot".into());
            }
            free_seen[slot] = true;
        }
        if self
            .cells
            .iter()
            .enumerate()
            .any(|(i, cell)| cell.is_none() != free_seen[i])
        {
            return Err("Source warm free list omits a vacant cell".into());
        }
        let mut serials = reserve::<u64>(self.cells.len())?;
        let mut seen_faces = flags(faces.len())?;
        let mut live_count = 0usize;
        for (slot, cell) in self.cells.iter().enumerate() {
            let Some(cell) = cell else {
                continue;
            };
            live_count += 1;
            if cell.serial == 0
                || cell.serial >= self.next_serial
                || cell.depth > SOURCE_ORDERED_TREE_MAX_DEPTH
                || !bounds_valid(cell.split, false)
                || !bounds_valid(cell.loose, true)
            {
                return Err("Invalid Source warm cell metadata".into());
            }
            serials.push(cell.serial);
            if slot != 0 {
                let Some(parent) = cell
                    .parent
                    .and_then(|p| self.cells.get(p))
                    .and_then(Option::as_ref)
                else {
                    return Err("Source warm cell has missing parent".into());
                };
                if parent.depth.checked_add(1) != Some(cell.depth) {
                    return Err("Invalid Source warm depth".into());
                }
            }
            if let Some(children) = cell.children {
                if cell.depth == SOURCE_ORDERED_TREE_MAX_DEPTH || !cell.faces.is_empty() {
                    return Err("Invalid Source warm branch".into());
                }
                let mut siblings = [usize::MAX; 8];
                for (i, &child) in children.iter().enumerate() {
                    if child >= self.cells.len()
                        || siblings[..i].contains(&child)
                        || self.cells[child]
                            .as_ref()
                            .is_none_or(|c| c.parent != Some(slot))
                    {
                        return Err("Invalid Source warm child link".into());
                    }
                    let expected = child_splits(cell.split)[i];
                    if self.cells[child].as_ref().unwrap().split.map(f64::to_bits)
                        != expected.map(f64::to_bits)
                    {
                        return Err("Invalid Source warm child split".into());
                    }
                    siblings[i] = child;
                }
            } else {
                for (position, &face) in cell.faces.iter().enumerate() {
                    let index = face as usize;
                    if index >= faces.len()
                        || seen_faces[index]
                        || self.face_leaves[index] as usize != slot
                        || self.face_positions[index] as usize != position
                    {
                        return Err("Invalid Source warm face membership".into());
                    }
                    let bounds = self.face_boxes[index];
                    if (0..3).any(|axis| {
                        cell.loose[axis] > f64::from(bounds[axis])
                            || cell.loose[axis + 3] < f64::from(bounds[axis + 3])
                    }) {
                        return Err("Source warm leaf bounds omit a face".into());
                    }
                    seen_faces[index] = true;
                }
            }
        }
        serials.sort_unstable();
        if serials.windows(2).any(|pair| pair[0] == pair[1]) || seen_faces.contains(&false) {
            return Err("Duplicate Source warm serial or missing face".into());
        }
        let mut visited = flags(self.cells.len())?;
        let mut stack = reserve::<usize>(live_count)?;
        stack.push(0);
        while let Some(slot) = stack.pop() {
            if visited[slot] {
                return Err("Source warm cell cycle or duplicate reachability".into());
            }
            visited[slot] = true;
            if let Some(children) = self.cells[slot].as_ref().unwrap().children {
                if stack.len().checked_add(8).is_none_or(|n| n > live_count) {
                    return Err("Source warm tree traversal exceeds live cells".into());
                }
                stack.extend(children);
            }
        }
        if visited.iter().filter(|&&v| v).count() != live_count {
            return Err("Unreachable Source warm cell".into());
        }
        Ok(())
    }
}

fn input_size_cell_limit(face_count: usize) -> Result<usize, String> {
    // A build starts with one root and adds eight cells only when splitting a
    // leaf with >100 faces. Historical free slots can remain, but no accepted
    // operation may exceed the explicit tree allocation cap.
    face_count
        .checked_mul(8)
        .and_then(|n| n.checked_add(1))
        .map(|n| n.min(MAX_SOURCE_ORDERED_TREE_BYTES / std::mem::size_of::<Option<Cell>>()))
        .ok_or_else(|| "Source warm cell count overflow".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Vec<Vec3>, Vec<[u32; 4]>, OrderedTree) {
        let mut positions = Vec::new();
        let mut faces = Vec::new();
        for i in 0..108 {
            let x = (i % 12) as f32;
            let y = (i / 12) as f32;
            let first = positions.len() as u32;
            positions.extend([
                Vec3::new(x, y, 0.0),
                Vec3::new(x + 0.1, y, 0.0),
                Vec3::new(x, y + 0.1, 0.0),
            ]);
            faces.push([first, first + 1, first + 2, TRIANGLE_CORNER]);
        }
        let tree = OrderedTree::build(&positions, &faces).unwrap();
        (positions, faces, tree)
    }

    #[test]
    fn exact_private_cells_free_serial_bounds_and_rings_round_trip() {
        let (positions, faces, tree) = fixture();
        assert!(tree.cells.len() > 1);
        let mut out = Writer::new(1024 * 1024);
        tree.encode_warm(&mut out).unwrap();
        let packet = out.finish();
        let mut reader = Reader::new(&packet);
        let restored = OrderedTree::decode_warm(&mut reader, &positions, &faces).unwrap();
        assert!(reader.done());
        assert!(tree.same_words(&restored));
        assert_eq!(tree.rings.starts, restored.rings.starts);
        assert_eq!(tree.rings.faces, restored.rings.faces);
    }

    #[test]
    fn malformed_free_duplicate_faces_cycles_and_bounds_are_rejected() {
        let (positions, faces, tree) = fixture();
        let mut missing_free = tree.copy_fallible().unwrap();
        missing_free.cells.push(None);
        assert!(missing_free
            .validate_warm(&positions, &faces)
            .unwrap_err()
            .contains("free"));

        let mut duplicate = tree.copy_fallible().unwrap();
        let leaf = duplicate.face_leaves[0] as usize;
        duplicate.cells[leaf].as_mut().unwrap().faces.push(0);
        duplicate.face_storage_bytes = OrderedTree::face_capacity_sum(&duplicate.cells);
        assert!(duplicate.validate_warm(&positions, &faces).is_err());

        let mut cycle = tree.copy_fallible().unwrap();
        let root = cycle.cells[0].as_mut().unwrap();
        root.children.as_mut().unwrap()[0] = 0;
        assert!(cycle.validate_warm(&positions, &faces).is_err());

        let mut wrong_bounds = tree.copy_fallible().unwrap();
        wrong_bounds.face_boxes[0][0] = 100.0;
        assert!(wrong_bounds
            .validate_warm(&positions, &faces)
            .unwrap_err()
            .contains("bounds"));

        let mut wrong_rings = tree.copy_fallible().unwrap();
        wrong_rings.rings = Arc::new(FaceRings {
            starts: wrong_rings.rings.starts.clone(),
            faces: wrong_rings.rings.faces.iter().rev().copied().collect(),
        });
        assert!(wrong_rings
            .validate_warm(&positions, &faces)
            .unwrap_err()
            .contains("rings"));
    }
}
