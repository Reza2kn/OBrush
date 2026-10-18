//! Private bounded original-face Smooth stencil and real pre-guard candidate.
//! Static current-ID rings only; no legacy adjacency or topology mutation.
use super::{SourceGeometryError, SourceGeometryNormalState};
use crate::{Mesh, Vec3, TRIANGLE_CORNER};
use std::mem::size_of;

pub(super) struct SmoothStencil {
    slots: Vec<u32>,
    selected: Vec<usize>,
    starts: Vec<usize>,
    neighbors: Vec<u32>,
    // 0 unknown, 1 interior, 2 boundary.
    boundary: Vec<u8>,
}

/// Both candidate buffers are admitted before the whole controlled dab writes.
/// The mirror recomputes means from live geometry in these same allocations.
pub(super) struct SmoothCandidateWorkspace {
    means: Vec<Vec3>,
    out: Vec<(usize, Vec3)>,
}
impl SmoothCandidateWorkspace {
    pub(super) fn reserve(
        mesh: &Mesh,
        stencil: &SmoothStencil,
        rows: usize,
        state_bytes: usize,
    ) -> Result<Self, SourceGeometryError> {
        Self::reserve_at_limit(
            mesh,
            stencil,
            rows,
            state_bytes,
            super::MAX_SOURCE_GEOMETRY_SCRATCH_BYTES,
        )
    }
    fn reserve_at_limit(
        mesh: &Mesh,
        stencil: &SmoothStencil,
        rows: usize,
        state_bytes: usize,
        limit: usize,
    ) -> Result<Self, SourceGeometryError> {
        let requested = rows
            .checked_mul(size_of::<Vec3>() + size_of::<(usize, Vec3)>())
            .and_then(|b| b.checked_add(size_of::<Self>()))
            .ok_or(SourceGeometryError::StorageLimit)?;
        stencil.check(mesh, state_bytes, requested, limit)?;
        let workspace = Self {
            means: super::reserved(rows)?,
            out: super::reserved(rows)?,
        };
        stencil.check(mesh, state_bytes, workspace.bytes(), limit)?;
        Ok(workspace)
    }
    pub(super) fn bytes(&self) -> usize {
        size_of::<Self>()
            .saturating_add(self.means.capacity().saturating_mul(size_of::<Vec3>()))
            .saturating_add(
                self.out
                    .capacity()
                    .saturating_mul(size_of::<(usize, Vec3)>()),
            )
    }
}

impl SmoothStencil {
    pub(super) fn build(
        mesh: &Mesh,
        state: &SourceGeometryNormalState,
        selections: &[&[usize]],
        state_bytes: usize,
    ) -> Result<Self, SourceGeometryError> {
        Self::build_at_limit(
            mesh,
            state,
            selections,
            state_bytes,
            super::MAX_SOURCE_GEOMETRY_SCRATCH_BYTES,
        )
    }

    fn build_at_limit(
        mesh: &Mesh,
        state: &SourceGeometryNormalState,
        selections: &[&[usize]],
        state_bytes: usize,
        limit: usize,
    ) -> Result<Self, SourceGeometryError> {
        let n = mesh.positions.len();
        if n > u32::MAX as usize {
            return Err(SourceGeometryError::StorageLimit);
        }
        let upper = selections
            .iter()
            .try_fold(0usize, |a, rows| {
                a.checked_add(rows.len())
                    .ok_or(SourceGeometryError::StorageLimit)
            })?
            .min(n);
        let minimum = n
            .checked_mul(9)
            .and_then(|b| {
                upper
                    .checked_mul(2 * size_of::<usize>())
                    .and_then(|a| b.checked_add(a))
            })
            .and_then(|b| {
                b.checked_add(size_of::<usize>() + size_of::<Self>() + size_of::<Vec<u32>>())
            })
            .ok_or(SourceGeometryError::StorageLimit)?;
        if minimum > limit {
            return Err(SourceGeometryError::StorageLimit);
        }
        super::operation_budget_with_scratch(mesh, state_bytes, minimum)?;
        let mut stencil = Self {
            slots: super::filled(n, u32::MAX)?,
            selected: super::reserved(upper)?,
            starts: Vec::new(),
            neighbors: Vec::new(),
            boundary: super::filled(n, 0u8)?,
        };
        let mut tags = super::filled(n, 0u32)?;
        let tag_bytes = tags
            .capacity()
            .checked_mul(4)
            .and_then(|b| b.checked_add(size_of::<Vec<u32>>()))
            .ok_or(SourceGeometryError::StorageLimit)?;
        stencil.check(mesh, state_bytes, tag_bytes, limit)?;
        for &v in selections.iter().flat_map(|rows| rows.iter()) {
            let slot = stencil
                .slots
                .get_mut(v)
                .ok_or(SourceGeometryError::InvalidInput)?;
            if *slot == u32::MAX {
                *slot = u32::try_from(stencil.selected.len())
                    .map_err(|_| SourceGeometryError::StorageLimit)?;
                stencil.selected.push(v);
            }
        }
        let starts_count = stencil
            .selected
            .len()
            .checked_add(1)
            .ok_or(SourceGeometryError::StorageLimit)?;
        let starts_bytes = starts_count
            .checked_mul(size_of::<usize>())
            .ok_or(SourceGeometryError::StorageLimit)?;
        stencil.check(
            mesh,
            state_bytes,
            tag_bytes
                .checked_add(starts_bytes)
                .ok_or(SourceGeometryError::StorageLimit)?,
            limit,
        )?;
        stencil.starts = super::reserved(starts_count)?;
        stencil.starts.push(0);
        let mut epoch = 0u32;
        for row in 0..stencil.selected.len() {
            let v = stencil.selected[row];
            let incident = state.ordered_tree.incident_faces(v)?;
            let stamp = next_epoch(&mut epoch)?;
            let start = stencil.neighbors.len();
            for &face in incident {
                let face = *state
                    .faces
                    .get(face as usize)
                    .ok_or(SourceGeometryError::InvalidMesh)?;
                for neighbor in perimeter(face, v)? {
                    let tag = tags
                        .get_mut(neighbor)
                        .ok_or(SourceGeometryError::InvalidMesh)?;
                    if *tag != stamp {
                        *tag = stamp;
                        if stencil.neighbors.len() == stencil.neighbors.capacity() {
                            stencil.check(
                                mesh,
                                state_bytes,
                                tag_bytes
                                    .checked_add(4)
                                    .ok_or(SourceGeometryError::StorageLimit)?,
                                limit,
                            )?;
                            stencil
                                .neighbors
                                .try_reserve(1)
                                .map_err(|_| SourceGeometryError::Allocation)?;
                        }
                        stencil.neighbors.push(neighbor as u32);
                        stencil.check(mesh, state_bytes, tag_bytes, limit)?;
                    }
                }
            }
            stencil.boundary[v] = if incident.len() != stencil.neighbors.len() - start {
                2
            } else {
                1
            };
            stencil.starts.push(stencil.neighbors.len());
        }
        // Boundary decisions require the original ring degree of neighbors too.
        // Compute only missing flags; do not allocate a second full ring table.
        for index in 0..stencil.neighbors.len() {
            let v = stencil.neighbors[index] as usize;
            if stencil.boundary[v] != 0 {
                continue;
            }
            let incident = state.ordered_tree.incident_faces(v)?;
            let stamp = next_epoch(&mut epoch)?;
            let mut degree = 0usize;
            for &face in incident {
                let face = *state
                    .faces
                    .get(face as usize)
                    .ok_or(SourceGeometryError::InvalidMesh)?;
                for neighbor in perimeter(face, v)? {
                    let tag = tags
                        .get_mut(neighbor)
                        .ok_or(SourceGeometryError::InvalidMesh)?;
                    if *tag != stamp {
                        *tag = stamp;
                        degree += 1;
                    }
                }
            }
            stencil.boundary[v] = if incident.len() != degree { 2 } else { 1 };
        }
        stencil.check(mesh, state_bytes, tag_bytes, limit)?;
        Ok(stencil)
    }

    pub(super) fn bytes(&self) -> usize {
        size_of::<Self>()
            .saturating_add(self.slots.capacity().saturating_mul(4))
            .saturating_add(self.selected.capacity().saturating_mul(size_of::<usize>()))
            .saturating_add(self.starts.capacity().saturating_mul(size_of::<usize>()))
            .saturating_add(self.neighbors.capacity().saturating_mul(4))
            .saturating_add(self.boundary.capacity())
    }

    fn check(
        &self,
        mesh: &Mesh,
        state_bytes: usize,
        extra: usize,
        limit: usize,
    ) -> Result<(), SourceGeometryError> {
        let bytes = self
            .bytes()
            .checked_add(extra)
            .ok_or(SourceGeometryError::StorageLimit)?;
        if bytes > limit {
            return Err(SourceGeometryError::StorageLimit);
        }
        super::operation_budget_with_scratch(mesh, state_bytes, bytes)
    }

    fn ring(&self, v: usize) -> Result<&[u32], SourceGeometryError> {
        let row = *self.slots.get(v).ok_or(SourceGeometryError::InvalidInput)? as usize;
        let start = *self
            .starts
            .get(row)
            .ok_or(SourceGeometryError::InvalidInput)?;
        let next = row
            .checked_add(1)
            .ok_or(SourceGeometryError::InvalidInput)?;
        let end = *self
            .starts
            .get(next)
            .ok_or(SourceGeometryError::InvalidInput)?;
        self.neighbors
            .get(start..end)
            .ok_or(SourceGeometryError::InvalidMesh)
    }

    fn mean(&self, positions: &[Vec3], v: usize) -> Result<Vec3, SourceGeometryError> {
        let ring = self.ring(v)?;
        if ring.len() <= 2 {
            return positions
                .get(v)
                .copied()
                .ok_or(SourceGeometryError::InvalidInput);
        }
        let mut sum = [0.0f64; 3];
        let mut count = 0usize;
        if self.boundary[v] == 2 {
            for &neighbor in ring {
                let neighbor = neighbor as usize;
                if self.boundary[neighbor] == 2 {
                    let p = positions
                        .get(neighbor)
                        .ok_or(SourceGeometryError::InvalidInput)?;
                    for c in 0..3 {
                        sum[c] += f64::from(p[c]);
                    }
                    count += 1;
                }
            }
        }
        if count < 2 {
            sum = [0.0; 3];
            count = ring.len();
            for &neighbor in ring {
                let p = positions
                    .get(neighbor as usize)
                    .ok_or(SourceGeometryError::InvalidInput)?;
                for c in 0..3 {
                    sum[c] += f64::from(p[c]);
                }
            }
        }
        let mean = Vec3::from_array(sum.map(|s| (s / count as f64) as f32));
        if !mean.is_finite() {
            return Err(SourceGeometryError::NonfiniteCandidate);
        }
        Ok(mean)
    }

    /// Production and raw fixtures share this exact pre-guard producer.
    /// Every mean is stored as f32 before any candidate is formed. Capacity is
    /// checked before clearing; evaluation performs no fallible allocation.
    #[allow(clippy::too_many_arguments)] // Distinct validated source-stage inputs.
    pub(super) fn candidates_into<'a>(
        &self,
        mesh: &Mesh,
        normals: &[Vec3],
        write: &[usize],
        intensity: f64,
        tangent: bool,
        state_bytes: usize,
        workspace: &'a mut SmoothCandidateWorkspace,
        mut sample: impl FnMut(usize, Vec3) -> Result<f64, SourceGeometryError>,
    ) -> Result<&'a [(usize, Vec3)], SourceGeometryError> {
        if write.len() > workspace.means.capacity() || write.len() > workspace.out.capacity() {
            return Err(SourceGeometryError::StorageLimit);
        }
        self.check(
            mesh,
            state_bytes,
            workspace.bytes(),
            super::MAX_SOURCE_GEOMETRY_SCRATCH_BYTES,
        )?;
        workspace.means.clear();
        workspace.out.clear();
        for &v in write {
            workspace.means.push(self.mean(&mesh.positions, v)?);
        }
        for (&v, mean) in write.iter().zip(&workspace.means) {
            let live = *mesh
                .positions
                .get(v)
                .ok_or(SourceGeometryError::InvalidInput)?;
            let u = f64::from(*mesh.masks.get(v).ok_or(SourceGeometryError::InvalidInput)?);
            if u == 0.0 || intensity == 0.0 {
                workspace.out.push((v, live));
                continue;
            }
            let p = live.to_array().map(f64::from);
            let mean = mean.to_array().map(f64::from);
            let next = if tangent {
                let raw = normals
                    .get(v)
                    .ok_or(SourceGeometryError::InvalidInput)?
                    .to_array()
                    .map(f64::from);
                let length_squared = super::dot(raw, raw);
                if length_squared == 0.0 {
                    workspace.out.push((v, live));
                    continue;
                }
                if !length_squared.is_finite() {
                    return Err(SourceGeometryError::NonfiniteCandidate);
                }
                let reciprocal = 1.0 / length_squared.sqrt();
                let n = raw.map(|c| c * reciprocal);
                let dot =
                    (n[0] * (mean[0] - p[0]) + n[1] * (mean[1] - p[1])) + n[2] * (mean[2] - p[2]);
                let m = (intensity * u) * sample(v, live)?;
                std::array::from_fn(|c| (p[c] + (mean[c] - n[c] * dot - p[c]) * m) as f32)
            } else {
                let m = (intensity * u) * sample(v, live)?;
                let complement = 1.0 - m;
                std::array::from_fn(|c| (p[c] * complement + mean[c] * m) as f32)
            };
            let next = Vec3::from_array(next);
            if !next.is_finite() || next.abs().max_element() > 1_000_000.0 {
                return Err(SourceGeometryError::NonfiniteCandidate);
            }
            workspace.out.push((v, next));
        }
        Ok(&workspace.out)
    }

    /// Preserve the original private raw-fixture entry. It delegates to the
    /// production producer and moves the already reserved output without copy.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn candidates(
        &self,
        mesh: &Mesh,
        normals: &[Vec3],
        write: &[usize],
        intensity: f64,
        tangent: bool,
        state_bytes: usize,
        sample: impl FnMut(usize, Vec3) -> Result<f64, SourceGeometryError>,
    ) -> Result<Vec<(usize, Vec3)>, SourceGeometryError> {
        let mut workspace =
            SmoothCandidateWorkspace::reserve(mesh, self, write.len(), state_bytes)?;
        self.candidates_into(
            mesh,
            normals,
            write,
            intensity,
            tangent,
            state_bytes,
            &mut workspace,
            sample,
        )?;
        Ok(workspace.out)
    }
}

fn next_epoch(epoch: &mut u32) -> Result<u32, SourceGeometryError> {
    *epoch = epoch
        .checked_add(1)
        .ok_or(SourceGeometryError::StorageLimit)?;
    Ok(*epoch)
}

fn perimeter([a, b, c, d]: [u32; 4], v: usize) -> Result<[usize; 2], SourceGeometryError> {
    let pair = if v == a as usize {
        [if d == TRIANGLE_CORNER { c } else { d }, b]
    } else if v == b as usize {
        [a, c]
    } else if v == c as usize {
        [if d == TRIANGLE_CORNER { a } else { d }, b]
    } else if d != TRIANGLE_CORNER && v == d as usize {
        [a, c]
    } else {
        return Err(SourceGeometryError::InvalidMesh);
    };
    Ok(pair.map(|v| v as usize))
}

#[cfg(test)]
#[path = "source_smooth_unit_tests.rs"]
mod tests;
