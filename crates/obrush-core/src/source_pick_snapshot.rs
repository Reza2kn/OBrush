//! A sealed immutable publication, incrementally admitted without a display.
//! This first implementation completes a bounded linear ray query or rejects it
//! before scanning. It is not an octree/BVH or a dense-mesh latency claim.
use crate::{
    Mesh, ObjectAffineF32V1, PreparedSourcePaintHit, SceneRay64, SourceAttributePickError,
    SourceAttributePickReport, SourceOriginalFaceContact, MAX_TRIANGLES, MAX_UV_COORDINATES,
    MAX_VERTICES, TRIANGLE_CORNER,
};
use glam::{DMat4, DVec3};
use serde::Serialize;
use std::sync::Arc;

pub const MAX_SOURCE_PICK_ADMISSION_OPERATIONS: usize = 16_384;
pub const MAX_SOURCE_PICK_FACE_TESTS: usize = 8_192;
pub const MAX_SOURCE_PICK_TRIANGLE_TESTS: usize = 8_192;
pub const MAX_SOURCE_PICK_CORNER_READS: usize = 1_024;
/// Dense original-face count ceiling; admission also enforces the stricter actual
/// 64 MiB index and 128 MiB mesh-plus-index storage budgets.
pub const MAX_SOURCE_PICK_INDEX_FACES: usize = 393_216;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePickAdmissionError {
    Header,
    Storage,
    Vertex,
    Triangle,
    Face,
    Uv,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SourcePickAdmissionProgress {
    pub mesh_storage_bytes: usize,
    pub operations: usize,
    pub total_operations: usize,
    pub vertex_rows: usize,
    pub triangle_rows: usize,
    pub face_rows: usize,
    pub uv_coordinate_rows: usize,
    pub uv_face_rows: usize,
    pub ready: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Vertices,
    Triangles,
    Faces,
    UvCoordinates,
    UvFaces,
    Ready,
}

/// Its only mesh is a retained Arc. Even mutation through another Arc owner
/// must copy on write and cannot change the admitted allocation.
pub struct SourcePickSnapshotAdmission {
    mesh: Arc<Mesh>,
    phase: Phase,
    row: usize,
    next_triangle: usize,
    progress: SourcePickAdmissionProgress,
    rejection: Option<SourcePickAdmissionError>,
    local_bounds: [f64; 6],
}
pub struct ValidatedSourcePickSnapshot {
    mesh: Arc<Mesh>,
    local_bounds: [f64; 6],
    progress: SourcePickAdmissionProgress,
}
pub struct SourceAttributePickView<'a> {
    mesh: &'a Mesh,
}
const _: () = assert!(std::mem::size_of::<SourcePickSnapshotAdmission>() <= 4096);
const _: () = assert!(std::mem::size_of::<ValidatedSourcePickSnapshot>() <= 4096);
const _: () = assert!(std::mem::size_of::<SourceAttributePickView<'static>>() <= 64);

fn normalized(v: f32) -> bool {
    v.is_finite() && (0.0..=1.0).contains(&v)
}
fn storage(mesh: &Mesh) -> Option<usize> {
    let mut bytes = std::mem::size_of::<Mesh>();
    let mut add = |count: usize, size: usize| -> Option<()> {
        bytes = bytes.checked_add(count.checked_mul(size)?)?;
        Some(())
    };
    add(mesh.positions.capacity(), std::mem::size_of::<glam::Vec3>())?;
    add(mesh.normals.capacity(), std::mem::size_of::<glam::Vec3>())?;
    add(mesh.triangles.capacity(), 12)?;
    add(mesh.masks.capacity(), 4)?;
    add(mesh.colors.capacity(), 12)?;
    if let Some(rows) = &mesh.materials {
        add(rows.capacity(), 8)?;
    }
    if let Some(surface) = &mesh.surface {
        add(surface.faces.capacity(), 16)?;
        if let Some(uv) = &surface.uv {
            add(uv.coordinates.capacity(), 8)?;
            add(uv.face_indices.capacity(), 16)?;
        }
    }
    Some(bytes)
}
impl SourcePickSnapshotAdmission {
    /// Constant-sized header checks only. A charged operation is one row check
    /// with at most four corners/two triangles, not a wall-clock estimate.
    pub fn new(mesh: Arc<Mesh>) -> Result<Self, SourcePickAdmissionError> {
        let n = mesh.positions.len();
        if n == 0
            || n > MAX_VERTICES
            || mesh.triangles.is_empty()
            || mesh.triangles.len() > MAX_TRIANGLES
            || mesh.normals.len() != n
            || mesh.masks.len() != n
            || mesh.colors.len() != n
            || mesh.materials.as_ref().is_some_and(|rows| rows.len() != n)
        {
            return Err(SourcePickAdmissionError::Header);
        }
        let mesh_storage_bytes = storage(&mesh)
            .filter(|&bytes| bytes <= crate::decode_limits::MAX_SCENE_STORAGE_BYTES)
            .ok_or(SourcePickAdmissionError::Storage)?;
        if let Some(surface) = &mesh.surface {
            if surface.faces.is_empty() || surface.faces.len() > MAX_TRIANGLES {
                return Err(SourcePickAdmissionError::Header);
            }
            if surface.uv.as_ref().is_some_and(|uv| {
                uv.coordinates.is_empty()
                    || uv.coordinates.len() > MAX_UV_COORDINATES
                    || uv.face_indices.len() != surface.faces.len()
            }) {
                return Err(SourcePickAdmissionError::Header);
            }
        }
        Ok(Self {
            mesh,
            phase: Phase::Vertices,
            row: 0,
            next_triangle: 0,
            progress: SourcePickAdmissionProgress {
                mesh_storage_bytes,
                ..Default::default()
            },
            rejection: None,
            local_bounds: [
                f64::INFINITY,
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
        })
    }
    pub fn matches(&self, published: &Arc<Mesh>) -> bool {
        Arc::ptr_eq(&self.mesh, published)
    }
    pub fn progress(&self) -> SourcePickAdmissionProgress {
        self.progress
    }
    /// No allocation, final sort, normal reconstruction or whole-array scan.
    pub fn step(
        &mut self,
        allowance: usize,
    ) -> Result<SourcePickAdmissionProgress, SourcePickAdmissionError> {
        if let Some(error) = self.rejection {
            return Err(error);
        }
        self.progress.operations = 0;
        let limit = allowance.min(MAX_SOURCE_PICK_ADMISSION_OPERATIONS);
        while self.progress.operations < limit && self.phase != Phase::Ready {
            if let Err(error) = self.check_row() {
                self.rejection = Some(error);
                return Err(error);
            }
            self.progress.operations += 1;
            self.progress.total_operations += 1;
        }
        self.progress.ready = self.phase == Phase::Ready;
        Ok(self.progress)
    }
    fn advance(&mut self, phase: Phase) {
        self.phase = phase;
        self.row = 0;
    }
    fn check_row(&mut self) -> Result<(), SourcePickAdmissionError> {
        let i = self.row;
        match self.phase {
            Phase::Vertices => {
                let p = self.mesh.positions[i];
                let n = self.mesh.normals[i];
                if !p.is_finite()
                    || p.abs().max_element() > 1_000_000.0
                    || !n.is_finite()
                    || !n.length_squared().is_finite()
                    || !normalized(self.mesh.masks[i])
                    || !self.mesh.colors[i].into_iter().all(normalized)
                    || self
                        .mesh
                        .materials
                        .as_ref()
                        .is_some_and(|r| !r[i].into_iter().all(normalized))
                {
                    return Err(SourcePickAdmissionError::Vertex);
                }
                for (axis, value) in p.to_array().map(f64::from).into_iter().enumerate() {
                    self.local_bounds[axis] = self.local_bounds[axis].min(value);
                    self.local_bounds[axis + 3] = self.local_bounds[axis + 3].max(value);
                }
                self.progress.vertex_rows += 1;
                self.row += 1;
                if self.row == self.mesh.positions.len() {
                    self.advance(Phase::Triangles);
                }
            }
            Phase::Triangles => {
                let t = self.mesh.triangles[i];
                if t.into_iter()
                    .any(|v| v as usize >= self.mesh.positions.len())
                    || t[0] == t[1]
                    || t[1] == t[2]
                    || t[2] == t[0]
                {
                    return Err(SourcePickAdmissionError::Triangle);
                }
                self.progress.triangle_rows += 1;
                self.row += 1;
                if self.row == self.mesh.triangles.len() {
                    self.advance(if self.mesh.surface.is_some() {
                        Phase::Faces
                    } else {
                        Phase::Ready
                    });
                }
            }
            Phase::Faces => {
                let surface = self
                    .mesh
                    .surface
                    .as_ref()
                    .ok_or(SourcePickAdmissionError::Face)?;
                let f = surface.faces[i];
                let arity = if f[3] == TRIANGLE_CORNER { 3 } else { 4 };
                for (corner, &v) in f[..arity].iter().enumerate() {
                    if v as usize >= self.mesh.positions.len() || f[..corner].contains(&v) {
                        return Err(SourcePickAdmissionError::Face);
                    }
                }
                if self.mesh.triangles.get(self.next_triangle) != Some(&[f[0], f[1], f[2]]) {
                    return Err(SourcePickAdmissionError::Face);
                }
                self.next_triangle += 1;
                if arity == 4 {
                    if self.mesh.triangles.get(self.next_triangle) != Some(&[f[0], f[2], f[3]]) {
                        return Err(SourcePickAdmissionError::Face);
                    }
                    self.next_triangle += 1;
                }
                self.progress.face_rows += 1;
                self.row += 1;
                if self.row == surface.faces.len() {
                    if self.next_triangle != self.mesh.triangles.len() {
                        return Err(SourcePickAdmissionError::Face);
                    }
                    self.advance(if surface.uv.is_some() {
                        Phase::UvCoordinates
                    } else {
                        Phase::Ready
                    });
                }
            }
            Phase::UvCoordinates => {
                let uv = self
                    .mesh
                    .surface
                    .as_ref()
                    .and_then(|s| s.uv.as_ref())
                    .ok_or(SourcePickAdmissionError::Uv)?;
                if uv.coordinates[i]
                    .into_iter()
                    .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
                {
                    return Err(SourcePickAdmissionError::Uv);
                }
                self.progress.uv_coordinate_rows += 1;
                self.row += 1;
                if self.row == uv.coordinates.len() {
                    self.advance(Phase::UvFaces);
                }
            }
            Phase::UvFaces => {
                let surface = self
                    .mesh
                    .surface
                    .as_ref()
                    .ok_or(SourcePickAdmissionError::Uv)?;
                let uv = surface.uv.as_ref().ok_or(SourcePickAdmissionError::Uv)?;
                let triangle = surface.faces[i][3] == TRIANGLE_CORNER;
                let f = uv.face_indices[i];
                if (f[3] == TRIANGLE_CORNER) != triangle
                    || f[..if triangle { 3 } else { 4 }]
                        .iter()
                        .any(|&v| v as usize >= uv.coordinates.len())
                {
                    return Err(SourcePickAdmissionError::Uv);
                }
                self.progress.uv_face_rows += 1;
                self.row += 1;
                if self.row == uv.face_indices.len() {
                    self.advance(Phase::Ready);
                }
            }
            Phase::Ready => {}
        }
        Ok(())
    }
    /// A partially checked or rejected snapshot can never produce a view.
    pub fn finish(self) -> Result<ValidatedSourcePickSnapshot, SourcePickAdmissionError> {
        if let Some(error) = self.rejection {
            return Err(error);
        }
        if self.phase != Phase::Ready {
            return Err(SourcePickAdmissionError::Header);
        }
        Ok(ValidatedSourcePickSnapshot {
            mesh: self.mesh,
            local_bounds: self.local_bounds,
            progress: self.progress,
        })
    }
}
impl ValidatedSourcePickSnapshot {
    /// Exact finite vertex extrema collected inside incremental validation.
    /// Applies the same zero-axis padding as the Source ordered root; no scan,
    /// allocation, mesh mutation or derived normal field is required at hover.
    pub fn source_root_loose_bounds(&self) -> [f64; 6] {
        let mut b = self.local_bounds;
        let d = [b[3] - b[0], b[4] - b[1], b[5] - b[2]];
        let off = ((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]).sqrt() * 0.2;
        for axis in 0..3 {
            if d[axis] < 1e-16 {
                b[axis] -= off;
                b[axis + 3] += off;
            }
        }
        b
    }
    pub fn matches(&self, published: &Arc<Mesh>) -> bool {
        Arc::ptr_eq(&self.mesh, published)
    }
    pub fn progress(&self) -> SourcePickAdmissionProgress {
        self.progress
    }
    pub fn borrow(&self) -> SourceAttributePickView<'_> {
        SourceAttributePickView { mesh: &self.mesh }
    }
    pub fn complete_query_batch_supported(&self, count: usize) -> bool {
        (1..=2).contains(&count)
            && self
                .mesh
                .triangles
                .len()
                .checked_mul(count)
                .is_some_and(|n| n <= MAX_SOURCE_PICK_TRIANGLE_TESTS)
            && self
                .mesh
                .surface
                .as_ref()
                .map_or(self.mesh.triangles.len(), |s| s.faces.len())
                .checked_mul(count)
                .is_some_and(|n| n <= MAX_SOURCE_PICK_FACE_TESTS)
    }
    pub fn complete_query_supported(&self) -> bool {
        self.mesh.triangles.len() <= MAX_SOURCE_PICK_TRIANGLE_TESTS
            && self
                .mesh
                .surface
                .as_ref()
                .map_or(self.mesh.triangles.len(), |s| s.faces.len())
                <= MAX_SOURCE_PICK_FACE_TESTS
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourcePickWorkBudget {
    pub face_tests: usize,
    pub triangle_tests: usize,
    pub corner_reads: usize,
}
impl Default for SourcePickWorkBudget {
    fn default() -> Self {
        Self {
            face_tests: MAX_SOURCE_PICK_FACE_TESTS,
            triangle_tests: MAX_SOURCE_PICK_TRIANGLE_TESTS,
            corner_reads: MAX_SOURCE_PICK_CORNER_READS,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSnapshotRayError {
    InvalidRay,
    InvalidPose,
    InvalidLocalRay,
    WorkBudget,
}
impl SourceAttributePickView<'_> {
    /// At most four rows. This immutable view has no revalidation or normals work.
    pub fn sample(
        &self,
        contact: SourceOriginalFaceContact,
    ) -> Result<SourceAttributePickReport, SourceAttributePickError> {
        crate::source_attribute_pick::sample_validated_source_attributes(self.mesh, contact)
    }
    pub fn sample_with_budget(
        &self,
        contact: SourceOriginalFaceContact,
        budget: &mut SourcePickWorkBudget,
    ) -> Result<Result<SourceAttributePickReport, SourceAttributePickError>, SourceSnapshotRayError>
    {
        let corners = self
            .mesh
            .surface
            .as_ref()
            .and_then(|s| s.faces.get(contact.face_index))
            .map_or(3, |f| if f[3] == TRIANGLE_CORNER { 3 } else { 4 });
        if budget.corner_reads < corners {
            return Err(SourceSnapshotRayError::WorkBudget);
        }
        budget.corner_reads -= corners;
        Ok(self.sample(contact))
    }
    /// None means a *completed* miss. Insufficient work is a separate error and
    /// never continues on another frame, including after key/button release.
    pub fn ray(
        &self,
        pose: Option<&ObjectAffineF32V1>,
        ray: SceneRay64,
        budget: &mut SourcePickWorkBudget,
    ) -> Result<Option<PreparedSourcePaintHit>, SourceSnapshotRayError> {
        let faces = self
            .mesh
            .surface
            .as_ref()
            .map_or(self.mesh.triangles.len(), |s| s.faces.len());
        let triangles = self.mesh.triangles.len();
        if faces > budget.face_tests || triangles > budget.triangle_tests {
            return Err(SourceSnapshotRayError::WorkBudget);
        }
        ray.validate()
            .map_err(|_| SourceSnapshotRayError::InvalidRay)?;
        let matrix = if let Some(pose) = pose {
            pose.validate()
                .map_err(|_| SourceSnapshotRayError::InvalidPose)?;
            DMat4::from_cols_array(&pose.local_to_world.map(f64::from))
        } else {
            DMat4::IDENTITY
        };
        let inverse = matrix.inverse();
        let origin = inverse.transform_point3(ray.origin);
        let direction = inverse.transform_vector3(ray.direction).normalize();
        if !origin.is_finite() || !direction.is_finite() || direction == DVec3::ZERO {
            return Err(SourceSnapshotRayError::InvalidLocalRay);
        }
        // Charge the entire worst-case query, even when a quad's first half hits.
        budget.face_tests -= faces;
        budget.triangle_tests -= triangles;
        let mut best: Option<(usize, Option<usize>, f64)> = None;
        let mut row = 0;
        if let Some(surface) = &self.mesh.surface {
            for (face, ids) in surface.faces.iter().enumerate() {
                let first = [ids[0], ids[1], ids[2]];
                let mut chosen = intersect(self.mesh, origin, direction, first).map(|t| (row, t));
                row += 1;
                if ids[3] != TRIANGLE_CORNER {
                    if chosen.is_none() {
                        chosen = intersect(self.mesh, origin, direction, [ids[0], ids[2], ids[3]])
                            .map(|t| (row, t));
                    }
                    row += 1;
                }
                if let Some((triangle, t)) = chosen {
                    if best.as_ref().is_none_or(|(_, _, distance)| t < *distance) {
                        best = Some((triangle, Some(face), t));
                    }
                }
            }
        } else {
            for (row, &ids) in self.mesh.triangles.iter().enumerate() {
                if let Some(t) = intersect(self.mesh, origin, direction, ids) {
                    if best.as_ref().is_none_or(|(_, _, distance)| t < *distance) {
                        best = Some((row, None, t));
                    }
                }
            }
        }
        Ok(best.map(|(triangle_index, original_face_index, t)| {
            let point = origin + direction * t;
            PreparedSourcePaintHit {
                triangle_index,
                original_face_index,
                local_position: point.to_array(),
                local_eye_direction: direction.to_array(),
                world_position: matrix.transform_point3(point).to_array(),
            }
        }))
    }
}

// Predicate copied verbatim from the unchanged paint_preparation.rs. No pruning
// interval or new epsilon is introduced by the complete bounded linear path.
fn intersect(mesh: &Mesh, origin: DVec3, direction: DVec3, ids: [u32; 3]) -> Option<f64> {
    const EPS: f64 = 1e-15;
    let [a, b, c] = ids.map(|i| mesh.positions[i as usize].as_dvec3());
    let e1 = b - a;
    let e2 = c - a;
    let p = direction.cross(e2);
    let determinant = e1.dot(p);
    if determinant > -EPS && determinant < EPS {
        return None;
    }
    let inv = 1. / determinant;
    let offset = origin - a;
    let u = offset.dot(p) * inv;
    if !(-EPS..=1. + EPS).contains(&u) {
        return None;
    }
    let q = offset.cross(e1);
    let v = direction.dot(q) * inv;
    if v < -EPS || u + v > 1. + EPS {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t.is_finite() && t >= 0.).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn quad() -> Arc<Mesh> {
        let mut mesh = Mesh::new(
            vec![
                glam::Vec3::new(-1., 0., 0.),
                glam::Vec3::new(0., -1., 0.),
                glam::Vec3::new(1., 0., 0.),
                glam::Vec3::new(0., 1., 0.),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        );
        mesh.surface = Some(crate::SurfaceTopology {
            faces: vec![[0, 1, 2, 3]],
            uv: Some(crate::CornerUv {
                coordinates: vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
                face_indices: vec![[0, 1, 2, 3]],
            }),
        });
        mesh.colors = vec![
            [0.5, 0., 0.],
            [f32::from_bits(0x33000000), 0., 0.],
            [f32::from_bits(0x33000000), 0., 0.],
            [0.; 3],
        ];
        Arc::new(mesh)
    }
    #[test]
    fn sealed_bounds_are_incremental_exact_and_use_source_flat_axis_padding() {
        let mut m = Mesh::from_obj("v -1 0 0.5\nv 1 0 0.5\nv 0 1 0.5\nf 1 2 3\n").unwrap();
        let published = Arc::new(m.snapshot());
        let mut admission = SourcePickSnapshotAdmission::new(published.clone()).unwrap();
        let mut total = 0;
        loop {
            let p = admission.step(1).unwrap();
            assert!(p.operations <= 1);
            total += p.operations;
            if p.ready {
                break;
            }
        }
        let sealed = admission.finish().unwrap();
        let b = sealed.source_root_loose_bounds();
        let off = 5f64.sqrt() * 0.2;
        assert_eq!(b, [-1., 0., 0.5 - off, 1., 1., 0.5 + off]);
        assert!(sealed.matches(&published));
        assert!(total > 3);
        m.positions[0].x = -4.;
        assert_eq!(sealed.source_root_loose_bounds(), b);
        assert!(!sealed.matches(&Arc::new(m)));
        assert!(sealed.complete_query_batch_supported(2));
        assert!(!sealed.complete_query_batch_supported(0));
        assert!(!sealed.complete_query_batch_supported(3));
    }
    fn ready(mesh: Arc<Mesh>) -> ValidatedSourcePickSnapshot {
        let mut a = SourcePickSnapshotAdmission::new(mesh).unwrap();
        while !a.step(7).unwrap().ready {}
        a.finish().unwrap()
    }
    fn contact() -> SourceOriginalFaceContact {
        SourceOriginalFaceContact {
            face_index: 0,
            local_point: [0.; 3],
        }
    }
    fn ray(x: f64, y: f64) -> SceneRay64 {
        SceneRay64 {
            origin: DVec3::new(x, y, 2.),
            direction: -DVec3::Z,
        }
    }
    #[test]
    fn b01_borrowed_report_keeps_all_scalar_words() {
        let m = quad();
        let expected = crate::prepare_source_attribute_pick(&m, contact()).unwrap();
        let r = ready(m).borrow().sample(contact()).unwrap();
        assert_eq!(r, expected);
        assert_eq!(r.rgb_final_f32_words[0], 0x3e000000);
    }
    #[test]
    fn b02_invalid_mask_rejected_incrementally() {
        let mut m = quad();
        Arc::make_mut(&mut m).masks[3] = 2.;
        assert!(m.validate().is_err());
        let mut a = SourcePickSnapshotAdmission::new(m).unwrap();
        assert_eq!(a.step(16384), Err(SourcePickAdmissionError::Vertex));
        assert!(a.finish().is_err());
    }
    #[test]
    fn b02_invalid_uv_never_seals() {
        let mut m = quad();
        Arc::make_mut(&mut m)
            .surface
            .as_mut()
            .unwrap()
            .uv
            .as_mut()
            .unwrap()
            .face_indices[0][3] = 4;
        let mut a = SourcePickSnapshotAdmission::new(m).unwrap();
        assert_eq!(a.step(16384), Err(SourcePickAdmissionError::Uv));
    }
    #[test]
    fn b02_invalid_original_diagonal_never_seals() {
        let mut m = quad();
        Arc::make_mut(&mut m).triangles[1] = [0, 3, 2];
        let mut a = SourcePickSnapshotAdmission::new(m).unwrap();
        assert_eq!(a.step(16384), Err(SourcePickAdmissionError::Face));
    }
    #[test]
    fn b02_invalid_material_never_seals() {
        let mut m = quad();
        Arc::make_mut(&mut m).materials = Some(vec![[0.18, f32::NAN]; 4]);
        let mut a = SourcePickSnapshotAdmission::new(m).unwrap();
        assert_eq!(a.step(16384), Err(SourcePickAdmissionError::Vertex));
    }
    #[test]
    fn b03_copy_on_write_cannot_inherit_identity() {
        let mut m = quad();
        let v = ready(m.clone());
        assert!(v.matches(&m));
        Arc::make_mut(&mut m).colors[0] = [1.; 3];
        assert!(!v.matches(&m));
        assert_eq!(
            v.borrow().sample(contact()).unwrap().rgb_final_f32_words[0],
            0x3e000000
        );
    }
    #[test]
    fn b03_equal_foreign_allocation_is_not_same_publication() {
        let m = quad();
        let v = ready(m.clone());
        let foreign = Arc::new((*m).clone());
        assert!(!v.matches(&foreign));
    }
    #[test]
    fn b04_owned_queries_do_not_readmission() {
        let v = ready(quad());
        let before = v.progress();
        let mut budget = SourcePickWorkBudget::default();
        for _ in 0..256 {
            let h = v
                .borrow()
                .ray(None, ray(0., 0.), &mut budget)
                .unwrap()
                .unwrap();
            v.borrow()
                .sample_with_budget(
                    SourceOriginalFaceContact {
                        face_index: h.original_face_index.unwrap(),
                        local_point: h.local_position,
                    },
                    &mut budget,
                )
                .unwrap()
                .unwrap();
        }
        assert_eq!(v.progress(), before);
        assert_eq!(budget.corner_reads, 0);
        assert_eq!(budget.triangle_tests, 8192 - 512);
    }
    #[test]
    fn b05_each_step_is_bounded_and_all_rows_counted_once() {
        let mut a = SourcePickSnapshotAdmission::new(quad()).unwrap();
        while !a.step(2).unwrap().ready {
            assert!(a.progress().operations <= 2);
        }
        let p = a.progress();
        assert_eq!(
            (
                p.vertex_rows,
                p.triangle_rows,
                p.face_rows,
                p.uv_coordinate_rows,
                p.uv_face_rows
            ),
            (4, 2, 1, 4, 1)
        );
        assert_eq!(p.total_operations, 12);
    }
    #[test]
    fn b05_zero_allowance_never_readies() {
        let mut a = SourcePickSnapshotAdmission::new(quad()).unwrap();
        assert_eq!(a.step(0).unwrap().total_operations, 0);
        assert!(a.finish().is_err());
    }
    #[test]
    fn b06_first_and_second_quad_branch_match_old_full_scan() {
        let m = quad();
        let v = ready(m.clone());
        for r in [ray(0.5, -0.5), ray(-0.5, 0.5), ray(0., 0.)] {
            let old = crate::prepare_source_paint_hit(&m, None, r)
                .unwrap()
                .unwrap();
            let new = v
                .borrow()
                .ray(None, r, &mut SourcePickWorkBudget::default())
                .unwrap()
                .unwrap();
            assert_eq!(new.triangle_index, old.triangle_index);
            assert_eq!(new.original_face_index, old.original_face_index);
            assert_eq!(
                new.local_position.map(f64::to_bits),
                old.local_position.map(f64::to_bits)
            );
        }
    }
    #[test]
    fn b06_completed_miss_is_distinct_from_budget() {
        let v = ready(quad());
        assert!(v
            .borrow()
            .ray(None, ray(3., 3.), &mut SourcePickWorkBudget::default())
            .unwrap()
            .is_none());
        let mut b = SourcePickWorkBudget {
            triangle_tests: 1,
            ..Default::default()
        };
        let old = b;
        assert_eq!(
            v.borrow().ray(None, ray(3., 3.), &mut b).unwrap_err(),
            SourceSnapshotRayError::WorkBudget
        );
        assert_eq!(b, old);
    }
    #[test]
    fn b08_exact_corner_budget_has_no_partial_sample() {
        let v = ready(quad());
        let mut b = SourcePickWorkBudget {
            corner_reads: 3,
            ..Default::default()
        };
        let old = b;
        assert_eq!(
            v.borrow()
                .sample_with_budget(contact(), &mut b)
                .unwrap_err(),
            SourceSnapshotRayError::WorkBudget
        );
        assert_eq!(b, old);
    }
    #[test]
    fn b09_dense_unsupported_before_any_scan() {
        let mut m = quad();
        let mesh = Arc::make_mut(&mut m);
        mesh.surface = None;
        mesh.triangles = vec![[0, 1, 2]; 8193];
        let v = ready(m);
        assert!(!v.complete_query_supported());
        let mut b = SourcePickWorkBudget::default();
        let old = b;
        assert_eq!(
            v.borrow().ray(None, ray(0., 0.), &mut b).unwrap_err(),
            SourceSnapshotRayError::WorkBudget
        );
        assert_eq!(b, old);
    }
    #[test]
    fn b10_admission_foreign_pointer_does_not_match() {
        let m = quad();
        let a = SourcePickSnapshotAdmission::new(m.clone()).unwrap();
        assert!(a.matches(&m));
        assert!(!a.matches(&quad()));
    }
    #[test]
    fn b11_rejected_admission_never_recovers() {
        let mut m = quad();
        Arc::make_mut(&mut m).normals[0] = glam::Vec3::splat(f32::MAX);
        let mut a = SourcePickSnapshotAdmission::new(m).unwrap();
        assert_eq!(a.step(1), Err(SourcePickAdmissionError::Vertex));
        assert_eq!(a.step(16384), Err(SourcePickAdmissionError::Vertex));
    }
    #[test]
    fn bounded_header_storage_and_error_records() {
        assert!(std::mem::size_of::<SourcePickAdmissionError>() <= 128);
        assert!(std::mem::size_of::<SourceSnapshotRayError>() <= 128);
        let m = quad();
        assert_eq!(storage(&m).unwrap(), m.approximate_bytes());
    }
    #[test]
    fn original_validators_agree_for_valid_and_invalid_rows() {
        for slot in 0..5 {
            let mut m = quad();
            let mm = Arc::make_mut(&mut m);
            match slot {
                0 => {}
                1 => mm.positions[0].x = f32::INFINITY,
                2 => mm.triangles[0] = [0, 0, 1],
                3 => mm.colors[0][1] = -0.1,
                _ => {
                    mm.masks.pop();
                }
            };
            let expected = m.validate().is_ok();
            let actual = SourcePickSnapshotAdmission::new(m)
                .and_then(|mut a| {
                    a.step(16384)?;
                    a.finish()
                })
                .is_ok();
            assert_eq!(actual, expected);
        }
    }
    #[test]
    fn b06_equal_distance_prefers_lower_original_ordinal() {
        let mut m = quad();
        let mm = Arc::make_mut(&mut m);
        mm.triangles.extend([[0, 1, 2], [0, 2, 3]]);
        let surface = mm.surface.as_mut().unwrap();
        surface.faces.push([0, 1, 2, 3]);
        surface.uv.as_mut().unwrap().face_indices.push([0, 1, 2, 3]);
        let v = ready(m);
        let h = v
            .borrow()
            .ray(None, ray(0., 0.), &mut SourcePickWorkBudget::default())
            .unwrap()
            .unwrap();
        assert_eq!(h.original_face_index, Some(0));
        assert_eq!(h.triangle_index, 0);
    }
    #[test]
    fn b06_nonuniform_affine_retains_exact_local_contact() {
        let m = quad();
        let matrix = [
            2., 0., 0., 0., 0., 3., 0., 0., 0., 0., 4., 0., 10., 20., 30., 1.,
        ];
        let pose = ObjectAffineF32V1::from_local_points(matrix, &m.positions).unwrap();
        let ray = SceneRay64 {
            origin: DVec3::new(10., 20., 32.),
            direction: -DVec3::Z,
        };
        let expected = crate::prepare_source_paint_hit(&m, Some(&pose), ray)
            .unwrap()
            .unwrap();
        let h = ready(m)
            .borrow()
            .ray(Some(&pose), ray, &mut SourcePickWorkBudget::default())
            .unwrap()
            .unwrap();
        assert_eq!(
            h.local_position.map(f64::to_bits),
            expected.local_position.map(f64::to_bits)
        );
        assert_eq!(h.world_position, [10., 20., 30.]);
        assert_eq!(h.original_face_index, Some(0));
    }
    #[test]
    fn b08_larger_query_accepts_exact_prefix_then_rejects_unscanned_tail() {
        let mut m = quad();
        let mm = Arc::make_mut(&mut m);
        mm.surface = None;
        mm.triangles = vec![[0, 1, 2]; 1024];
        let v = ready(m);
        let mut b = SourcePickWorkBudget::default();
        for _ in 0..8 {
            assert!(v
                .borrow()
                .ray(None, ray(0., -0.5), &mut b)
                .unwrap()
                .is_some());
        }
        let before = b;
        assert_eq!(
            v.borrow().ray(None, ray(0., -0.5), &mut b).unwrap_err(),
            SourceSnapshotRayError::WorkBudget
        );
        assert_eq!(before, b);
        assert_eq!(b.triangle_tests, 0);
    }
    #[test]
    fn b09_independent_hundred_thousand_overlap_faces_are_unavailable() {
        let mut m = quad();
        let mm = Arc::make_mut(&mut m);
        mm.surface = None;
        mm.triangles = vec![[0, 1, 2]; 100_000];
        let mut a = SourcePickSnapshotAdmission::new(m).unwrap();
        loop {
            let p = a.step(usize::MAX).unwrap();
            assert!(p.operations <= 16384);
            if p.ready {
                break;
            }
        }
        let v = a.finish().unwrap();
        assert_eq!(v.progress().triangle_rows, 100_000);
        assert!(!v.complete_query_supported());
        let mut b = SourcePickWorkBudget::default();
        let original = b;
        assert_eq!(
            v.borrow().ray(None, ray(0., 0.), &mut b).unwrap_err(),
            SourceSnapshotRayError::WorkBudget
        );
        assert_eq!(b, original);
    }
    #[test]
    fn b05_capacity_not_only_length_is_charged() {
        let mut m = quad();
        let old = storage(&m).unwrap();
        Arc::make_mut(&mut m).positions.reserve_exact(64);
        assert!(storage(&m).unwrap() > old);
        assert_eq!(storage(&m).unwrap(), m.approximate_bytes());
        assert!(SourcePickSnapshotAdmission::new(m).is_ok());
    }
}

#[cfg(test)]
mod borrowed_original24 {
    use crate::source_attribute_pick::*;
    use crate::{CornerUv, Hit, PaintPickInterpolation, SurfaceTopology, Vec3, MAX_VERTICES};
    use crate::{MaskConvention, Mesh, SourceOriginalFaceContact, TRIANGLE_CORNER};

    fn prepare_source_attribute_pick(
        mesh: &Mesh,
        contact: SourceOriginalFaceContact,
    ) -> Result<SourceAttributePickReport, SourceAttributePickError> {
        let expected = crate::prepare_source_attribute_pick(mesh, contact);
        let mut admission =
            match super::SourcePickSnapshotAdmission::new(std::sync::Arc::new(mesh.clone())) {
                Ok(a) => a,
                Err(_) => {
                    assert_eq!(
                        expected.as_ref().unwrap_err().failure,
                        SourceAttributePickFailure::Rejected(
                            SourceAttributePickRejection::InvalidMesh
                        )
                    );
                    return expected;
                }
            };
        loop {
            match admission.step(16384) {
                Ok(p) if p.ready => break,
                Ok(_) => {}
                Err(_) => {
                    assert_eq!(
                        expected.as_ref().unwrap_err().failure,
                        SourceAttributePickFailure::Rejected(
                            SourceAttributePickRejection::InvalidMesh
                        )
                    );
                    return expected;
                }
            }
        }
        let ready = admission.finish().unwrap();
        let actual = ready.borrow().sample(contact);
        assert_eq!(actual, expected);
        actual
    }

    fn from_words(words: [u32; 3]) -> [f32; 3] {
        words.map(f32::from_bits)
    }
    fn bits64(pair: [u32; 2]) -> u64 {
        (u64::from(pair[0]) << 32) | u64::from(pair[1])
    }
    fn contact() -> SourceOriginalFaceContact {
        SourceOriginalFaceContact {
            face_index: 0,
            local_point: [0.0; 3],
        }
    }
    fn quad() -> Mesh {
        let mut mesh = Mesh::new(
            vec![
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::new(0.0, -1.0, 0.0),
                Vec3::X,
                Vec3::Y,
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        );
        mesh.surface = Some(SurfaceTopology {
            faces: vec![[0, 1, 2, 3]],
            uv: None,
        });
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks = [0x00000000, 0x3f800000, 0x3f000000, 0x00000001]
            .map(f32::from_bits)
            .to_vec();
        mesh.colors = [
            [0x3f000000, 0x00000000, 0x3f000000],
            [0x33000000, 0x3f800000, 0x3f000000],
            [0x33000000, 0x00000000, 0x3f000000],
            [0x00000000, 0x00000000, 0x3f000000],
        ]
        .map(from_words)
        .to_vec();
        mesh.materials = Some(vec![
            [f32::from_bits(0x3f000000), 0.5],
            [f32::from_bits(0x33000000), 0.5],
            [f32::from_bits(0x33000000), 0.5],
            [0.0, 0.5],
        ]);
        mesh
    }
    fn sample(mesh: &Mesh) -> SourceAttributePickReport {
        prepare_source_attribute_pick(mesh, contact()).unwrap()
    }
    fn word_snapshot(mesh: &Mesh) -> Vec<u32> {
        let mut words = Vec::new();
        for row in &mesh.positions {
            words.extend(row.to_array().map(f32::to_bits));
        }
        for row in &mesh.normals {
            words.extend(row.to_array().map(f32::to_bits));
        }
        words.extend(mesh.masks.iter().map(|v| v.to_bits()));
        for row in &mesh.colors {
            words.extend(row.map(f32::to_bits));
        }
        if let Some(rows) = &mesh.materials {
            for row in rows {
                words.extend(row.map(f32::to_bits));
            }
        }
        words
    }
    fn undefined(mesh: &Mesh, point: [f64; 3]) -> SourceAttributePickError {
        prepare_source_attribute_pick(
            mesh,
            SourceOriginalFaceContact {
                face_index: 0,
                local_point: point,
            },
        )
        .unwrap_err()
    }

    #[test]
    fn c01_quarter_quad_per_corner_store_discriminates_one_final_cast() {
        let r = sample(&quad());
        assert_eq!(
            r.rgb_stages_f32_words,
            [
                [0x3e000000, 0x00000000, 0x3e000000],
                [0x3e000000, 0x3e800000, 0x3e800000],
                [0x3e000000, 0x3e800000, 0x3ec00000],
                [0x3e000000, 0x3e800000, 0x3f000000],
            ]
        );
        assert_eq!(
            r.material_stages_f32_words,
            [
                [0x3e000000, 0x3e000000, 0x00000000],
                [0x3e000000, 0x3e800000, 0x3e800000],
                [0x3e000000, 0x3ec00000, 0x3ec00000],
                [0x3e000000, 0x3f000000, 0x3ec00000],
            ]
        );
        assert_eq!(r.weight64_words.map(bits64), [0x3fd0000000000000; 4]);
        assert_eq!(r.rgb_final_f32_words, [0x3e000000, 0x3e800000, 0x3f000000]);
        assert_eq!(
            r.material_final_f32_words,
            [0x3e000000, 0x3f000000, 0x3ec00000]
        );
        assert_ne!(r.sample.rgb[0].to_bits(), 0x3e000001);
    }
    #[test]
    fn c02_cyclic_original_corner_order_changes_the_registered_word() {
        let mut mesh = quad();
        mesh.surface.as_mut().unwrap().faces = vec![[1, 2, 3, 0]];
        mesh.triangles = vec![[1, 2, 3], [1, 3, 0]];
        let r = sample(&mesh);
        assert_eq!(
            r.rgb_stages_f32_words.map(|v| v[0]),
            [0x32000000, 0x32800000, 0x32800000, 0x3e000001]
        );
        assert_eq!(
            r.material_stages_f32_words.map(|v| v[0]),
            [0x32000000, 0x32800000, 0x32800000, 0x3e000001]
        );
        assert_eq!(r.rgb_final_f32_words, [0x3e000001, 0x3e800000, 0x3f000000]);
        assert_eq!(
            r.material_final_f32_words,
            [0x3e000001, 0x3f000000, 0x3ec00000]
        );
    }
    #[test]
    fn c03_material_pass_is_retained_before_rgb_output_resets() {
        let r = sample(&quad());
        assert_eq!(r.sample.roughness.to_bits(), 0x3e000000);
        assert_eq!(r.sample.metallic.to_bits(), 0x3f000000);
        assert_eq!(r.sample.sampled_unprotected_mask.to_bits(), 0x3ec00000);
        assert_eq!(
            r.sample.rgb.map(f32::to_bits),
            [0x3e000000, 0x3e800000, 0x3f000000]
        );
        assert_eq!(r.material_mask_input_f32_words[3][2], 0x00000001);
    }
    #[test]
    fn c04_rational_axis_triangle_keeps_fourth_zero_without_dereferencing_sentinel() {
        let mut mesh = Mesh::new(vec![Vec3::X, Vec3::Y * 2.0, Vec3::Z * 2.0], vec![[0, 1, 2]]);
        mesh.surface = Some(SurfaceTopology {
            faces: vec![[0, 1, 2, TRIANGLE_CORNER]],
            uv: None,
        });
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.colors = vec![[1.0, 0.0, 0.5], [0.0, 1.0, 0.5], [0.0, 0.0, 0.5]];
        mesh.materials = Some(vec![[1.0, 0.0], [0.0, 1.0], [0.0, 1.0]]);
        mesh.masks = vec![0.0, 1.0, 0.5];
        let r = sample(&mesh);
        assert_eq!(
            r.weight64_words.map(bits64),
            [
                0x3fe0000000000000,
                0x3fd0000000000000,
                0x3fd0000000000000,
                0
            ]
        );
        assert_eq!(
            r.inverse_distance64_words.map(bits64),
            [
                0x3ff0000000000000,
                0x3fe0000000000000,
                0x3fe0000000000000,
                0
            ]
        );
        assert_eq!(r.corner_count, 3);
        assert_eq!(r.vertex_ids[3], TRIANGLE_CORNER);
        assert_eq!(r.distances64[3], None);
        assert_eq!(r.rgb_stages_f32_words[3], [0; 3]);
        assert_eq!(r.rgb_final_f32_words, [0x3f000000, 0x3e800000, 0x3f000000]);
        assert_eq!(
            r.material_final_f32_words,
            [0x3f000000, 0x3f000000, 0x3ec00000]
        );
    }
    #[test]
    fn c05_equal_triangle_uses_number_reciprocal_third_then_f32_stores() {
        let mut mesh = Mesh::new(vec![-Vec3::X, Vec3::X, Vec3::Y], vec![[0, 1, 2]]);
        mesh.colors = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        mesh.materials = Some(vec![[1.0, 0.0], [0.0, 1.0], [0.0, 0.0]]);
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks = vec![0.0, 0.0, 1.0];
        let r = sample(&mesh);
        assert_eq!(
            r.weight64_words.map(bits64),
            [
                0x3fd5555555555555,
                0x3fd5555555555555,
                0x3fd5555555555555,
                0
            ]
        );
        assert_eq!(r.rgb_final_f32_words, [0x3eaaaaab; 3]);
        assert_eq!(r.material_final_f32_words, [0x3eaaaaab; 3]);
        assert_eq!(r.vertex_ids, [0, 1, 2, TRIANGLE_CORNER]);
    }
    #[test]
    fn c06_implicit_default_materials_have_literal_stores_and_stay_none() {
        let mut mesh = quad();
        mesh.materials = None;
        mesh.masks = vec![1.0; 4];
        let r = sample(&mesh);
        assert_eq!(
            r.material_stages_f32_words,
            [
                [0x3d3851ec, 0x3ca3d70a, 0x3e800000],
                [0x3db851ec, 0x3d23d70a, 0x3f000000],
                [0x3e0a3d71, 0x3d75c28f, 0x3f400000],
                [0x3e3851ec, 0x3da3d70a, 0x3f800000],
            ]
        );
        assert_eq!(
            r.material_final_f32_words,
            [0x3e3851ec, 0x3da3d70a, 0x3f800000]
        );
        assert!(!r.materials_explicit);
        assert!(mesh.materials.is_none());
    }
    #[test]
    fn c07_corner_uv_ids_are_provenance_not_attribute_row_ids() {
        let mut mesh = quad();
        mesh.surface.as_mut().unwrap().uv = Some(CornerUv {
            coordinates: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.5, 0.5]],
            face_indices: vec![[4, 2, 0, 1]],
        });
        let r = sample(&mesh);
        assert_eq!(r.uv_corner_ids, Some([4, 2, 0, 1]));
        assert_eq!(r.rgb_final_f32_words, [0x3e000000, 0x3e800000, 0x3f000000]);
        assert_eq!(
            r.material_final_f32_words,
            [0x3e000000, 0x3f000000, 0x3ec00000]
        );
    }
    #[test]
    fn c08_two_shared_geometry_faces_keep_explicit_order_and_uv_seams() {
        let mut mesh = quad();
        mesh.triangles = vec![[0, 1, 2], [0, 2, 3], [1, 2, 3], [1, 3, 0]];
        mesh.surface = Some(SurfaceTopology {
            faces: vec![[0, 1, 2, 3], [1, 2, 3, 0]],
            uv: Some(CornerUv {
                coordinates: vec![
                    [0.0, 0.0],
                    [1.0, 0.0],
                    [1.0, 1.0],
                    [0.0, 1.0],
                    [0.25, 0.25],
                    [0.75, 0.25],
                    [0.75, 0.75],
                    [0.25, 0.75],
                ],
                face_indices: vec![[0, 1, 2, 3], [4, 5, 6, 7]],
            }),
        });
        let first = sample(&mesh);
        let second = prepare_source_attribute_pick(
            &mesh,
            SourceOriginalFaceContact {
                face_index: 1,
                local_point: [0.0; 3],
            },
        )
        .unwrap();
        assert_eq!(first.uv_corner_ids, Some([0, 1, 2, 3]));
        assert_eq!(second.uv_corner_ids, Some([4, 5, 6, 7]));
        assert_eq!(first.vertex_ids, [0, 1, 2, 3]);
        assert_eq!(second.vertex_ids, [1, 2, 3, 0]);
        assert_eq!(first.rgb_final_f32_words[0], 0x3e000000);
        assert_eq!(second.rgb_final_f32_words[0], 0x3e000001);
    }
    #[test]
    fn c09_both_explicit_canonical_diagonal_contacts_name_the_same_quad() {
        let mesh = quad();
        for (index, barycentric) in [[0.5, 0.0, 0.5], [0.5, 0.5, 0.0]].into_iter().enumerate() {
            let hit = Hit {
                position: Vec3::ZERO,
                normal: Vec3::Z,
                distance: 1.0,
                triangle: mesh.triangles[index],
                barycentric,
            };
            let legacy = mesh
                .pick_paint_channels_at_triangle(
                    &hit,
                    index,
                    PaintPickInterpolation::OriginalFaceInverseDistance,
                )
                .unwrap();
            assert_eq!(legacy.original_face_index, Some(0));
            let r = prepare_source_attribute_pick(
                &mesh,
                SourceOriginalFaceContact {
                    face_index: 0,
                    local_point: [0.0; 3],
                },
            )
            .unwrap();
            assert_eq!(r.vertex_ids, [0, 1, 2, 3]);
            assert_eq!(r.rgb_final_f32_words, [0x3e000000, 0x3e800000, 0x3f000000]);
        }
    }
    #[test]
    fn c10_fully_protected_unprotected_zero_rows_do_not_weight_sampling() {
        let mut mesh = quad();
        mesh.masks = vec![0.0; 4];
        let r = sample(&mesh);
        assert_eq!(r.rgb_final_f32_words, [0x3e000000, 0x3e800000, 0x3f000000]);
        assert_eq!(r.material_final_f32_words, [0x3e000000, 0x3f000000, 0]);
    }
    #[test]
    fn c11_legacy_protection_conversion_is_temporary_f32_and_reported() {
        let mut mesh = quad();
        mesh.mask_convention = MaskConvention::ProtectedF32V1;
        mesh.masks = [0x3f800000, 0x3f400000, 0x3e800000, 0x00000000]
            .map(f32::from_bits)
            .to_vec();
        let r = sample(&mesh);
        assert_eq!(
            r.mask_policy,
            SourceAttributePickMaskPolicy::DerivedProtectionCompatibility
        );
        assert_eq!(
            r.raw_mask_f32_words,
            [0x3f800000, 0x3f400000, 0x3e800000, 0]
        );
        assert_eq!(
            r.material_mask_input_f32_words.map(|v| v[2]),
            [0, 0x3e800000, 0x3f400000, 0x3f800000]
        );
        assert_eq!(r.material_final_f32_words[2], 0x3f000000);
        assert_eq!(
            mesh.masks.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            r.raw_mask_f32_words
        );
        assert_eq!(mesh.mask_convention, MaskConvention::ProtectedF32V1);
    }
    #[test]
    fn c12_signed_zero_input_words_are_retained_but_output_starts_positive_zero() {
        let mut mesh = quad();
        mesh.colors = vec![[f32::from_bits(0x80000000); 3]; 4];
        mesh.materials = Some(vec![[f32::from_bits(0x80000000); 2]; 4]);
        mesh.masks = vec![f32::from_bits(0x80000000); 4];
        let r = sample(&mesh);
        assert_eq!(r.rgb_input_f32_words, [[0x80000000; 3]; 4]);
        assert_eq!(r.material_mask_input_f32_words, [[0x80000000; 3]; 4]);
        assert_eq!(r.raw_mask_f32_words, [0x80000000; 4]);
        assert_eq!(r.rgb_stages_f32_words, [[0; 3]; 4]);
        assert_eq!(r.material_stages_f32_words, [[0; 3]; 4]);
    }
    #[test]
    fn c13_tiny_channel_product_rounds_at_first_f32_assignment() {
        let mut mesh = quad();
        mesh.colors = vec![[0.0; 3]; 4];
        mesh.colors[0][0] = f32::from_bits(1);
        let r = sample(&mesh);
        assert_eq!(r.rgb_input_f32_words[0][0], 1);
        assert_eq!(r.rgb_stages_f32_words, [[0; 3]; 4]);
    }
    #[test]
    fn c14_coincident_positions_away_from_contact_remain_defined() {
        let mut mesh = quad();
        mesh.positions[1] = mesh.positions[0];
        let r = sample(&mesh);
        assert_eq!(r.weight64_words.map(bits64), [0x3fd0000000000000; 4]);
        assert_eq!(r.rgb_final_f32_words, [0x3e000000, 0x3e800000, 0x3f000000]);
    }
    #[test]
    fn c15_precise_number_contact_is_not_first_rounded_to_f32() {
        let mesh = Mesh::new(
            vec![Vec3::ZERO, Vec3::X * 2.0, Vec3::X * 2.0],
            vec![[0, 1, 2]],
        );
        let point = f64::from_bits(0x3ff0000008000000);
        let r = prepare_source_attribute_pick(
            &mesh,
            SourceOriginalFaceContact {
                face_index: 0,
                local_point: [point, 0.0, 0.0],
            },
        )
        .unwrap();
        assert_eq!(bits64(r.local_point64_words[0]), 0x3ff0000008000000);
        assert_eq!(
            r.distance64_words.map(|v| v.map(bits64)),
            [
                Some(0x3ff0000008000000),
                Some(0x3feffffff0000000),
                Some(0x3feffffff0000000),
                None
            ]
        );
        assert_eq!((point as f32).to_bits(), 0x3f800000);
    }
    #[test]
    fn c16_exact_vertex_is_explicitly_undefined_without_one_hot_sample() {
        let error = undefined(&quad(), [-1.0, 0.0, 0.0]);
        assert_eq!(
            error.failure,
            SourceAttributePickFailure::Undefined(
                SourceAttributePickUndefined::ExactOrUnderflowedZeroDistance
            )
        );
        assert_eq!(error.corner, Some(0));
        assert_eq!(error.vertex_id, Some(0));
        assert_eq!(error.numeric64_word.map(bits64), Some(0));
    }
    #[test]
    fn c17_coincident_corner_contact_keeps_first_original_undefined_identity() {
        let mut mesh = quad();
        mesh.positions[1] = mesh.positions[0];
        let error = undefined(&mesh, [-1.0, 0.0, 0.0]);
        assert_eq!(
            error.failure,
            SourceAttributePickFailure::Undefined(
                SourceAttributePickUndefined::ExactOrUnderflowedZeroDistance
            )
        );
        assert_eq!(error.corner, Some(0));
        assert_eq!(error.vertex_id, Some(0));
    }
    #[test]
    fn c18_individual_subnormal_reciprocal_overflow_is_undefined() {
        let mut mesh = quad();
        mesh.positions = vec![Vec3::ZERO; 4];
        let error = undefined(&mesh, [f64::from_bits(1), 0.0, 0.0]);
        assert_eq!(
            error.failure,
            SourceAttributePickFailure::Undefined(
                SourceAttributePickUndefined::IndividualInverseDistanceOverflow
            )
        );
        assert_eq!(error.corner, Some(0));
        assert_eq!(error.numeric64_word.map(bits64), Some(0x7ff0000000000000));
    }
    #[test]
    fn c19_finite_inverse_sum_overflow_has_positive_zero_attribute_sample() {
        let mut mesh = quad();
        mesh.positions = vec![Vec3::ZERO; 4];
        let r = prepare_source_attribute_pick(
            &mesh,
            SourceOriginalFaceContact {
                face_index: 0,
                local_point: [f64::from_bits(0x0008000000000000), 0.0, 0.0],
            },
        )
        .unwrap();
        assert_eq!(
            r.inverse_distance64_words.map(bits64),
            [0x7fe0000000000000; 4]
        );
        assert_eq!(bits64(r.ordered_inverse_sum64_word), 0x7ff0000000000000);
        assert_eq!(
            r.ordered_inverse_sum,
            SourceAttributePickInverseSum::PositiveInfinityZeroWeights
        );
        assert_eq!(bits64(r.reciprocal64_word), 0);
        assert_eq!(r.weight64_words.map(bits64), [0; 4]);
        assert_eq!(r.rgb_stages_f32_words, [[0; 3]; 4]);
        assert_eq!(r.material_stages_f32_words, [[0; 3]; 4]);
        assert_eq!(r.rgb_final_f32_words, [0; 3]);
        assert_eq!(r.material_final_f32_words, [0; 3]);
        let json = serde_json::to_value(r).unwrap();
        assert_eq!(json["ordered_inverse_sum64_word"][0], 0x7ff00000_u32);
        assert_eq!(json["ordered_inverse_sum64_word"][1], 0_u32);
        assert!(json["ordered_inverse_sum"]["value64"].is_null());
    }
    #[test]
    fn c20_invalid_contact_words_are_rejected_and_raw64_pairs_are_lossless() {
        let mesh = quad();
        for word in [0x7ff8000000000001, 0x7ff0000000000000, 0xfff0000000000000] {
            let error = undefined(&mesh, [f64::from_bits(word), 0.0, 0.0]);
            assert_eq!(
                error.failure,
                SourceAttributePickFailure::Rejected(SourceAttributePickRejection::InvalidContact)
            );
            assert_eq!(bits64(error.local_point64_words[0]), word);
            let json = serde_json::to_value(error).unwrap();
            assert_eq!(json["local_point64_words"][0][0].as_u64(), Some(word >> 32));
            assert_eq!(
                json["local_point64_words"][0][1].as_u64(),
                Some(word & 0xffffffff)
            );
        }
        let error = undefined(&mesh, [1_000_001.0, 0.0, 0.0]);
        assert_eq!(
            error.failure,
            SourceAttributePickFailure::Rejected(SourceAttributePickRejection::InvalidContact)
        );
    }
    #[test]
    fn c21_mesh_face_field_uv_and_capacity_errors_reject_before_reads() {
        let mesh = quad();
        let error = prepare_source_attribute_pick(
            &mesh,
            SourceOriginalFaceContact {
                face_index: 1,
                local_point: [0.0; 3],
            },
        )
        .unwrap_err();
        assert_eq!(
            error.failure,
            SourceAttributePickFailure::Rejected(SourceAttributePickRejection::MissingOriginalFace)
        );
        let mut variants = Vec::new();
        let mut bad = mesh.clone();
        bad.surface.as_mut().unwrap().faces[0][0] = 4;
        variants.push(bad);
        let mut bad = mesh.clone();
        bad.surface.as_mut().unwrap().faces[0][1] = 0;
        variants.push(bad);
        let mut bad = mesh.clone();
        bad.surface.as_mut().unwrap().faces[0][2] = TRIANGLE_CORNER;
        variants.push(bad);
        let mut bad = mesh.clone();
        bad.colors.pop();
        variants.push(bad);
        let mut bad = mesh.clone();
        bad.materials.as_mut().unwrap().pop();
        variants.push(bad);
        let mut bad = mesh.clone();
        bad.masks.pop();
        variants.push(bad);
        let mut bad = mesh.clone();
        bad.colors[0][0] = f32::from_bits(0x7fc00001);
        variants.push(bad);
        let mut bad = mesh.clone();
        bad.colors[0][0] = f32::from_bits(0x3f800001);
        variants.push(bad);
        let mut bad = mesh.clone();
        bad.materials.as_mut().unwrap()[0][0] = f32::from_bits(0x80000001);
        variants.push(bad);
        for ids in [[0, 1, 2, TRIANGLE_CORNER], [0, 1, 2, 4]] {
            let mut bad = mesh.clone();
            bad.surface.as_mut().unwrap().uv = Some(CornerUv {
                coordinates: vec![[0.0; 2]; 4],
                face_indices: vec![ids],
            });
            variants.push(bad);
        }
        let mut bad = mesh.clone();
        bad.surface.as_mut().unwrap().uv = Some(CornerUv {
            coordinates: vec![[0.0; 2]; 4],
            face_indices: vec![],
        });
        variants.push(bad);
        for bad in variants {
            let before = word_snapshot(&bad);
            assert_eq!(
                undefined(&bad, [0.0; 3]).failure,
                SourceAttributePickFailure::Rejected(SourceAttributePickRejection::InvalidMesh)
            );
            assert_eq!(word_snapshot(&bad), before);
        }
        let mut bad = mesh;
        bad.positions = vec![Vec3::ZERO; MAX_VERTICES + 1];
        assert_eq!(
            undefined(&bad, [0.0; 3]).failure,
            SourceAttributePickFailure::Rejected(SourceAttributePickRejection::InvalidMesh)
        );
    }
    #[test]
    fn c22_read_only_query_retains_all_mesh_channel_and_metadata_words() {
        let mut mesh = quad();
        mesh.surface.as_mut().unwrap().uv = Some(CornerUv {
            coordinates: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            face_indices: vec![[0, 1, 2, 3]],
        });
        let words = word_snapshot(&mesh);
        let json = serde_json::to_vec(&mesh).unwrap();
        let _ = sample(&mesh);
        let _ = undefined(&mesh, [-1.0, 0.0, 0.0]);
        assert_eq!(word_snapshot(&mesh), words);
        assert_eq!(serde_json::to_vec(&mesh).unwrap(), json);
        assert_eq!(mesh.mask_convention, MaskConvention::UnprotectedF32V1);
        assert!(mesh.materials.is_some());
    }
    #[test]
    fn c23_actual_fixed_report_and_lossless_json_remain_within_registered_caps() {
        let report = sample(&quad());
        assert!(std::mem::size_of_val(&report) <= MAX_SOURCE_ATTRIBUTE_PICK_REPORT_BYTES);
        let mut mesh = quad();
        let error = undefined(&mesh, [-1.0, 0.0, 0.0]);
        assert!(std::mem::size_of_val(&error) <= 128);
        mesh.positions = vec![Vec3::ZERO; 4];
        for point in [
            [0.0; 3],
            [1_000_000.0, -1_000_000.0, 1_000_000.0],
            [f64::from_bits(0x0008000000000000), 0.0, 0.0],
        ] {
            let value = prepare_source_attribute_pick(
                &mesh,
                SourceOriginalFaceContact {
                    face_index: 0,
                    local_point: point,
                },
            );
            let bytes = match value {
                Ok(report) => serde_json::to_vec(&report).unwrap(),
                Err(error) => serde_json::to_vec(&error).unwrap(),
            };
            assert!(bytes.len() <= MAX_SOURCE_ATTRIBUTE_PICK_DIAGNOSTIC_BYTES);
        }
        let mesh = quad();
        let r = sample(&mesh);
        assert_eq!(r.corner_count, 4);
        assert_eq!(r.positions_f32_words.len(), 4);
        assert_eq!(r.material_stages_f32_words.len(), 4);
        assert!(r.targets_within_normalized_domain);
    }
    #[test]
    fn c24_existing_legacy_picker_stays_a_distinct_one_final_cast_policy() {
        let mesh = quad();
        let hit = Hit {
            position: Vec3::ZERO,
            normal: Vec3::Z,
            distance: 1.0,
            triangle: [0, 1, 2],
            barycentric: [0.5, 0.0, 0.5],
        };
        let legacy = mesh
            .pick_paint_channels_at_triangle(
                &hit,
                0,
                PaintPickInterpolation::OriginalFaceInverseDistance,
            )
            .unwrap();
        assert_eq!(legacy.rgb[0].to_bits(), 0x3e000001);
        assert_eq!(legacy.roughness.to_bits(), 0x3e000001);
        assert_eq!(sample(&mesh).rgb_final_f32_words[0], 0x3e000000);
        assert_eq!(legacy.original_face_index, Some(0));
        // All pre-existing source/assertions are additionally bound by donor
        // preservation metadata; their required workspace run remains pending.
    }
}

#[path = "source_pick_dense.rs"]
mod dense;
pub use dense::{
    SourcePickQueryAdmission, SourcePickQueryAdmissionError, SourcePickQueryBudget,
    SourcePickQueryError, SourcePickQueryProgress, SourcePreparedPickQuery,
};
