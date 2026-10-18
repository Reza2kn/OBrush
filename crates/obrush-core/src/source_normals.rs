//! Optional original-face raw normals. Legacy Mesh.normals is never modified.
//! Number arithmetic has two explicit Float32 assignment boundaries: face cache,
//! then incident-face mean. Source optimizer and dynamic ring ordering are gaps.
use crate::{Mesh, Vec3, MAX_TRIANGLES, MAX_VERTICES, TRIANGLE_CORNER};
use serde::Serialize;
use std::mem::size_of;

pub const MAX_SOURCE_NORMAL_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourceNormalPolicy {
    StaticOriginalFaces,
    /// Dynamic mutation-dependent rings are deliberately unavailable.
    DynamicFaces,
    /// Retained static raw rows refreshed only for the selected operation set.
    /// Constructed by SourceGeometryNormalState, never by fresh compute().
    StaticSelectedRefresh,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourceNormalFaceOrigin {
    StoredOriginalFaces,
    TriangleFaces,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourceNormalStorage {
    pub retained_bytes: usize,
    pub peak_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourceNormalReport {
    pub policy: SourceNormalPolicy,
    pub face_origin: SourceNormalFaceOrigin,
    pub faces: usize,
    pub quads: usize,
    pub vertices: usize,
    pub unused_vertices: usize,
    pub zero_vertex_normals: usize,
    pub zero_face_normals: usize,
    /// Rows are used as supplied; actual Source optimize/remapping is unpaired.
    pub supplied_face_order_preserved: bool,
    pub storage: SourceNormalStorage,
}

/// Immutable derived values plus exact input words, avoiding hash collisions in
/// cache validation and detecting signed-zero/same-size public topology edits.
pub struct SourceNormalField {
    face_normals: Vec<Vec3>,
    vertex_normals: Vec<Vec3>,
    position_words: Vec<[u32; 3]>,
    face_rows: Vec<[u32; 4]>,
    report: SourceNormalReport,
}
/// Owned immutable rows transferred into the retained geometry producer.
/// Keeping the exact allocations also preserves position and face witnesses;
/// this transfer does not compute a new field or refresh selected rows.
pub(crate) struct SourceNormalParts {
    pub(crate) face_normals: Vec<Vec3>,
    pub(crate) vertex_normals: Vec<Vec3>,
    pub(crate) position_words: Vec<[u32; 3]>,
    pub(crate) face_rows: Vec<[u32; 4]>,
}
impl SourceNormalField {
    pub(crate) fn into_geometry_parts(self) -> SourceNormalParts {
        SourceNormalParts {
            face_normals: self.face_normals,
            vertex_normals: self.vertex_normals,
            position_words: self.position_words,
            face_rows: self.face_rows,
        }
    }
    pub fn compute(mesh: &Mesh, policy: SourceNormalPolicy) -> Result<Self, String> {
        Self::compute_with_budget(mesh, policy, MAX_SOURCE_NORMAL_BYTES)
    }
    pub fn storage_requirement(mesh: &Mesh) -> Result<SourceNormalStorage, String> {
        // Refuse oversized public arrays before any topology-validation scratch.
        if mesh.positions.len() > MAX_VERTICES || mesh.triangles.len() > MAX_TRIANGLES {
            return Err("Source normals exceed vertex/triangle limits".into());
        }
        mesh.validate()?;
        let faces = mesh
            .surface
            .as_ref()
            .map_or(mesh.triangles.len(), |s| s.faces.len());
        let vertices = mesh.positions.len();
        let retained_bytes = vertices
            .checked_mul(size_of::<Vec3>() + size_of::<[u32; 3]>())
            .and_then(|n| {
                faces
                    .checked_mul(size_of::<Vec3>() + size_of::<[u32; 4]>())
                    .and_then(|f| n.checked_add(f))
            })
            .and_then(|n| n.checked_add(size_of::<Self>()))
            .ok_or("Source normal storage overflow")?;
        let peak_bytes = vertices
            .checked_mul(size_of::<[f64; 3]>() + size_of::<u32>())
            .and_then(|scratch| retained_bytes.checked_add(scratch))
            .ok_or("Source normal scratch overflow")?;
        Ok(SourceNormalStorage {
            retained_bytes,
            peak_bytes,
        })
    }
    pub fn compute_with_budget(
        mesh: &Mesh,
        policy: SourceNormalPolicy,
        budget: usize,
    ) -> Result<Self, String> {
        if policy == SourceNormalPolicy::StaticSelectedRefresh {
            return Err("Retained selected-refresh normals require SourceGeometryNormalState, not fresh whole-field compute".into());
        }
        if policy != SourceNormalPolicy::StaticOriginalFaces {
            return Err("Dynamic source normal ring ordering is not implemented".into());
        }
        let storage = Self::storage_requirement(mesh)?;
        if storage.peak_bytes > budget.min(MAX_SOURCE_NORMAL_BYTES) {
            return Err("Source normal scratch/cache exceeds its bounded allocation budget".into());
        }
        let face_origin = if mesh.surface.is_some() {
            SourceNormalFaceOrigin::StoredOriginalFaces
        } else {
            SourceNormalFaceOrigin::TriangleFaces
        };
        let faces = mesh
            .surface
            .as_ref()
            .map_or(mesh.triangles.len(), |s| s.faces.len());
        let vertices = mesh.positions.len();
        let mut position_words = reserved(vertices)?;
        position_words.extend(
            mesh.positions
                .iter()
                .map(|p| p.to_array().map(f32::to_bits)),
        );
        let mut face_rows = reserved(faces)?;
        if let Some(surface) = &mesh.surface {
            face_rows.extend_from_slice(&surface.faces);
        } else {
            face_rows.extend(
                mesh.triangles
                    .iter()
                    .map(|&[a, b, c]| [a, b, c, TRIANGLE_CORNER]),
            );
        }
        let mut face_normals = reserved(faces)?;
        let mut quads = 0;
        let mut zero_face_normals = 0;
        for &[a, b, c, d] in &face_rows {
            let p = |i: u32| mesh.positions[i as usize].to_array().map(f64::from);
            let [a, b, c] = [p(a), p(b), p(c)];
            let edge = sub(c, a);
            let mut normal = cross(sub(b, a), edge);
            if d != TRIANGLE_CORNER {
                quads += 1;
                let other = cross(sub(c, p(d)), edge);
                for (component, value) in normal.iter_mut().zip(other) {
                    *component += value;
                }
            }
            let normal = Vec3::from_array(normal.map(|x| x as f32));
            if !normal.is_finite() || !normal.length_squared().is_finite() {
                return Err("Source face normal Float32 cache overflowed".into());
            }
            zero_face_normals += usize::from(normal == Vec3::ZERO);
            face_normals.push(normal);
        }
        let mut sums = reserved(vertices)?;
        sums.resize(vertices, [0.0_f64; 3]);
        let mut counts = reserved(vertices)?;
        counts.resize(vertices, 0_u32);
        // Scatter descending faces reproduces every static vertex ring's order.
        for (face, ids) in face_rows.iter().enumerate().rev() {
            let arity = if ids[3] == TRIANGLE_CORNER { 3 } else { 4 };
            let values = face_normals[face].to_array().map(f64::from);
            for &vertex in &ids[..arity] {
                let index = vertex as usize;
                counts[index] = counts[index]
                    .checked_add(1)
                    .ok_or("Source normal valence overflow")?;
                for (sum, value) in sums[index].iter_mut().zip(values) {
                    *sum += value;
                }
            }
        }
        let mut vertex_normals = reserved(vertices)?;
        let mut unused_vertices = 0;
        let mut zero_vertex_normals = 0;
        for (sum, count) in sums.into_iter().zip(counts) {
            let reciprocal = if count == 0 {
                0.0
            } else {
                1.0 / f64::from(count)
            };
            unused_vertices += usize::from(count == 0);
            let normal = Vec3::from_array(sum.map(|x| (x * reciprocal) as f32));
            if !normal.is_finite() || !normal.length_squared().is_finite() {
                return Err("Source vertex normal Float32 mean overflowed".into());
            }
            zero_vertex_normals += usize::from(normal == Vec3::ZERO);
            vertex_normals.push(normal);
        }
        Ok(Self {
            face_normals,
            vertex_normals,
            position_words,
            face_rows,
            report: SourceNormalReport {
                policy,
                face_origin,
                faces,
                quads,
                vertices,
                unused_vertices,
                zero_vertex_normals,
                zero_face_normals,
                supplied_face_order_preserved: true,
                storage,
            },
        })
    }
    pub fn face_normals(&self) -> &[Vec3] {
        &self.face_normals
    }
    pub fn vertex_normals(&self) -> &[Vec3] {
        &self.vertex_normals
    }
    pub fn report(&self) -> SourceNormalReport {
        self.report
    }
    pub fn matches_mesh(&self, mesh: &Mesh) -> bool {
        if mesh.positions.len() != self.position_words.len()
            || mesh
                .positions
                .iter()
                .zip(&self.position_words)
                .any(|(p, w)| p.to_array().map(f32::to_bits) != *w)
            || mesh.surface.is_some()
                != (self.report.face_origin == SourceNormalFaceOrigin::StoredOriginalFaces)
        {
            return false;
        }
        if let Some(surface) = &mesh.surface {
            if surface.faces != self.face_rows {
                return false;
            }
        } else if mesh.triangles.len() != self.face_rows.len()
            || mesh
                .triangles
                .iter()
                .zip(&self.face_rows)
                .any(|(t, f)| *t != [f[0], f[1], f[2]])
        {
            return false;
        }
        // Validate exact original→canonical binding without allocating scratch.
        let mut row = 0;
        for face in &self.face_rows {
            if mesh.triangles.get(row) != Some(&[face[0], face[1], face[2]]) {
                return false;
            }
            row += 1;
            if face[3] != TRIANGLE_CORNER {
                if mesh.triangles.get(row) != Some(&[face[0], face[2], face[3]]) {
                    return false;
                }
                row += 1;
            }
        }
        row == mesh.triangles.len()
    }
    /// Replacement is atomic: failed construction retains the prior cache but
    /// returns an error, never a stale field or a legacy fallback.
    pub fn refresh<'a>(
        cache: &'a mut Option<Self>,
        mesh: &Mesh,
        policy: SourceNormalPolicy,
    ) -> Result<&'a Self, String> {
        if cache
            .as_ref()
            .is_none_or(|field| field.report.policy != policy || !field.matches_mesh(mesh))
        {
            let old_bytes = cache
                .as_ref()
                .map_or(0, |field| field.report.storage.retained_bytes);
            let available = MAX_SOURCE_NORMAL_BYTES
                .checked_sub(old_bytes)
                .ok_or("Existing source normal cache exceeds its budget")?;
            let next = Self::compute_with_budget(mesh, policy, available)?;
            *cache = Some(next);
        }
        cache
            .as_ref()
            .ok_or("Source normal cache was not constructed".into())
    }
}
fn reserved<T>(count: usize) -> Result<Vec<T>, String> {
    let mut out = Vec::new();
    out.try_reserve_exact(count)
        .map_err(|_| "Cannot allocate bounded source normal field")?;
    Ok(out)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod ownership_regressions {
    use super::*;
    #[test]
    fn geometry_parts_reuse_all_four_allocations_and_capacities() {
        let mut mesh = Mesh::sphere(2);
        mesh.positions[0].x = -0.0;
        let field =
            SourceNormalField::compute(&mesh, SourceNormalPolicy::StaticOriginalFaces).unwrap();
        let pointers = (
            field.face_normals.as_ptr(),
            field.vertex_normals.as_ptr(),
            field.position_words.as_ptr(),
            field.face_rows.as_ptr(),
        );
        let capacities = (
            field.face_normals.capacity(),
            field.vertex_normals.capacity(),
            field.position_words.capacity(),
            field.face_rows.capacity(),
        );
        let parts = field.into_geometry_parts();
        assert_eq!(
            pointers,
            (
                parts.face_normals.as_ptr(),
                parts.vertex_normals.as_ptr(),
                parts.position_words.as_ptr(),
                parts.face_rows.as_ptr()
            )
        );
        assert_eq!(
            capacities,
            (
                parts.face_normals.capacity(),
                parts.vertex_normals.capacity(),
                parts.position_words.capacity(),
                parts.face_rows.capacity()
            )
        );
        assert_eq!(parts.position_words[0][0], (-0.0f32).to_bits());
        assert_eq!(
            parts.face_rows[0],
            [
                mesh.triangles[0][0],
                mesh.triangles[0][1],
                mesh.triangles[0][2],
                TRIANGLE_CORNER
            ]
        );
    }
    #[test]
    fn original_quad_and_unused_vertex_rows_survive_transfer() {
        let mut mesh = Mesh::cube();
        mesh.positions = vec![Vec3::ZERO, Vec3::X, Vec3::ONE, Vec3::Y, Vec3::Z];
        mesh.triangles = vec![[0, 1, 2], [0, 2, 3]];
        mesh.normals = vec![Vec3::ZERO; 5];
        mesh.masks = vec![0.0; 5];
        mesh.colors = vec![[0.0; 3]; 5];
        mesh.surface = Some(crate::SurfaceTopology {
            faces: vec![[0, 1, 2, 3]],
            uv: None,
        });
        let field =
            SourceNormalField::compute(&mesh, SourceNormalPolicy::StaticOriginalFaces).unwrap();
        assert_eq!(field.report().unused_vertices, 1);
        let normal = field.face_normals[0].to_array().map(f32::to_bits);
        let parts = field.into_geometry_parts();
        assert_eq!(parts.face_rows, [[0, 1, 2, 3]]);
        assert_eq!(parts.face_normals[0].to_array().map(f32::to_bits), normal);
        assert_eq!(parts.vertex_normals[4], Vec3::ZERO);
        assert_eq!(
            parts.position_words[4],
            Vec3::Z.to_array().map(f32::to_bits)
        );
    }
}
