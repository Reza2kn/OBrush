//! Experimental retained selected-row normal program. The scalar oracle uses
//! the existing f64 face/mean grammar. WGSL uses f32 and requires comparison;
//! matching refresh sets/order does not establish numerical or brush parity.
use crate::resident_point_program::{PointInflateParameters, ResidentPointInput};
use crate::{Mesh, SourceNormalField, TRIANGLE_CORNER};

pub const NORMAL_FACE_FUNCTIONS: &str = include_str!("resident_normal_face.wgsl");
pub const NORMAL_CROSS_FUNCTIONS: &str = include_str!("resident_normal_cross.wgsl");
pub const NORMAL_MEAN_FUNCTIONS: &str = include_str!("resident_normal_mean.wgsl");
pub const NORMAL_MEAN_PREFLIGHT: &str = include_str!("resident_normal_mean_preflight.wgsl");
pub const NORMAL_PREFLIGHT: &str = include_str!("resident_normal_preflight.wgsl");
pub const NORMAL_FACE_COMMIT: &str = include_str!("resident_normal_face_commit.wgsl");
pub const NORMAL_VERTEX_COMMIT: &str = include_str!("resident_normal_vertex_commit.wgsl");
pub struct ResidentNormalTopology {
    headers: Vec<u32>,
    offsets: Vec<u32>,
    rings: Vec<u32>,
    face_normals: Vec<[f32; 3]>,
}
fn reserved<T>(count: usize) -> Result<Vec<T>, String> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|_| "Resident normal allocation failed")?;
    Ok(rows)
}
fn copy<T: Copy>(rows: &[T]) -> Result<Vec<T>, String> {
    let mut out = reserved(rows.len())?;
    out.extend_from_slice(rows);
    Ok(out)
}
impl ResidentNormalTopology {
    pub(crate) fn capture(mesh: &Mesh, normals: &SourceNormalField) -> Result<Self, String> {
        let count = mesh.positions.len();
        let face_count = normals.face_normals().len();
        let mut headers = reserved(face_count)?;
        let mut offsets = reserved::<u32>(count + 1)?;
        offsets.resize(count + 1, 0);
        let mut triangle = 0u32;
        for face in 0..face_count {
            let ids = mesh.surface.as_ref().map_or_else(
                || {
                    let t = mesh.triangles[face];
                    [t[0], t[1], t[2], TRIANGLE_CORNER]
                },
                |s| s.faces[face],
            );
            let quad = ids[3] != TRIANGLE_CORNER;
            headers.push((triangle << 1) | u32::from(quad));
            triangle += if quad { 2 } else { 1 };
            for &vertex in &ids[..if quad { 4 } else { 3 }] {
                offsets[vertex as usize + 1] = offsets[vertex as usize + 1]
                    .checked_add(1)
                    .ok_or("Resident incident count overflow")?;
            }
        }
        if triangle as usize != mesh.triangles.len() {
            return Err("Resident original-face coverage mismatch".into());
        }
        for i in 1..offsets.len() {
            offsets[i] = offsets[i]
                .checked_add(offsets[i - 1])
                .ok_or("Resident incident prefix overflow")?;
        }
        let corners = offsets[count] as usize;
        if corners > crate::MAX_TRIANGLES * 3 {
            return Err("Resident corner work exceeds canonical triangle bound".into());
        }
        let mut rings = reserved(corners)?;
        rings.resize(corners, 0);
        let mut cursor = copy(&offsets[..count])?;
        for face in (0..face_count).rev() {
            let h = headers[face];
            let at = (h >> 1) as usize;
            let t = mesh.triangles[at];
            let ids = [
                t[0],
                t[1],
                t[2],
                if h & 1 != 0 {
                    mesh.triangles[at + 1][2]
                } else {
                    TRIANGLE_CORNER
                },
            ];
            for &v in &ids[..if h & 1 != 0 { 4 } else { 3 }] {
                let slot = &mut cursor[v as usize];
                rings[*slot as usize] = face as u32;
                *slot += 1;
            }
        }
        let mut face_normals = reserved(face_count)?;
        face_normals.extend(normals.face_normals().iter().map(|v| v.to_array()));
        Ok(Self {
            headers,
            offsets,
            rings,
            face_normals,
        })
    }
    pub fn headers(&self) -> &[u32] {
        &self.headers
    }
    pub fn offsets(&self) -> &[u32] {
        &self.offsets
    }
    pub fn rings(&self) -> &[u32] {
        &self.rings
    }
    pub fn face_normals(&self) -> &[[f32; 3]] {
        &self.face_normals
    }
    fn ids(&self, face: usize, triangles: &[[u32; 3]]) -> [u32; 4] {
        let h = self.headers[face];
        let at = (h >> 1) as usize;
        let t = triangles[at];
        [
            t[0],
            t[1],
            t[2],
            if h & 1 != 0 {
                triangles[at + 1][2]
            } else {
                TRIANGLE_CORNER
            },
        ]
    }
}
pub struct ResidentNormalReference<'a> {
    input: &'a ResidentPointInput,
    positions: Vec<[f32; 4]>,
    normal_u: Vec<[f32; 4]>,
    faces: Vec<[f32; 3]>,
    visited: Vec<u32>,
}
impl<'a> ResidentNormalReference<'a> {
    pub fn new(input: &'a ResidentPointInput) -> Result<Self, String> {
        let mut visited = reserved(input.positions().len())?;
        visited.resize(input.positions().len(), 0);
        Ok(Self {
            input,
            positions: copy(input.positions())?,
            normal_u: copy(input.normal_u())?,
            faces: copy(input.normal_topology().face_normals())?,
            visited,
        })
    }
    pub fn positions(&self) -> &[[f32; 4]] {
        &self.positions
    }
    pub fn normal_u(&self) -> &[[f32; 4]] {
        &self.normal_u
    }
    pub fn face_normals(&self) -> &[[f32; 3]] {
        &self.faces
    }
    pub fn visited(&self) -> &[u32] {
        &self.visited
    }
    /// Update only incident face cache rows and selected vertex means. Empty
    /// or duplicate selections preserve the old end state. Validate all input
    /// and reserve candidates before replacing any retained field.
    pub fn refresh_selected(
        &mut self,
        positions: &[[f32; 4]],
        selected: &[u32],
    ) -> Result<(), String> {
        if positions.len() != self.positions.len() || selected.len() > crate::MAX_VERTICES * 4 {
            return Err("Resident normal refresh count bound".into());
        }
        if positions.iter().any(|p| {
            p[3].to_bits() != 1f32.to_bits()
                || p[..3]
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
        }) {
            return Err("Resident normal coordinates invalid".into());
        }
        if selected.iter().any(|&v| v as usize >= positions.len()) {
            return Err("Resident normal selected vertex absent".into());
        }
        let mut flags = reserved(positions.len())?;
        flags.resize(positions.len(), false);
        for &v in selected {
            flags[v as usize] = true;
        }
        let mut faces = copy(&self.faces)?;
        let mut vertices = copy(&self.normal_u)?;
        let mut visited = copy(&self.visited)?;
        let topology = self.input.normal_topology();
        for (v, &selected) in flags.iter().enumerate() {
            if !selected {
                continue;
            }
            for &face in
                &topology.rings[topology.offsets[v] as usize..topology.offsets[v + 1] as usize]
            {
                faces[face as usize] = raw_face(
                    positions,
                    topology.ids(face as usize, self.input.triangles()),
                )?;
            }
        }
        for (v, &selected) in flags.iter().enumerate() {
            if !selected {
                continue;
            }
            let incident =
                &topology.rings[topology.offsets[v] as usize..topology.offsets[v + 1] as usize];
            let mut sum = [0f64; 3];
            for &face in incident {
                for (c, value) in faces[face as usize].iter().enumerate() {
                    sum[c] += f64::from(*value);
                }
            }
            let factor = if incident.is_empty() {
                0.
            } else {
                1. / incident.len() as f64
            };
            for c in 0..3 {
                vertices[v][c] = (sum[c] * factor) as f32;
                if !vertices[v][c].is_finite() {
                    return Err("Resident selected mean not finite".into());
                }
            }
            visited[v] = 1;
        }
        let positions = copy(positions)?;
        self.positions = positions;
        self.normal_u = vertices;
        self.faces = faces;
        self.visited = visited;
        Ok(())
    }
    pub fn point_pass(&mut self, parameters: PointInflateParameters) -> Result<(), String> {
        parameters.validate()?;
        let mut positions = copy(&self.positions)?;
        let mut selected = reserved(positions.len())?;
        let center = parameters.center.map(f64::from);
        let radius = f64::from(parameters.radius);
        let amplitude = f64::from(parameters.intensity) * radius * 0.1;
        for (i, (p, n)) in positions.iter_mut().zip(&self.normal_u).enumerate() {
            let live = [p[0], p[1], p[2]].map(f64::from);
            let delta = std::array::from_fn::<_, 3, _>(|c| live[c] - center[c]);
            let d =
                ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]).sqrt() / radius;
            if d >= 1. {
                continue;
            }
            selected.push(i as u32);
            let u = f64::from(n[3]);
            if u == 0. {
                continue;
            }
            let n = [n[0], n[1], n[2]].map(f64::from);
            let length = ((n[0] * n[0] + n[1] * n[1]) + n[2] * n[2]).sqrt();
            if length == 0. {
                return Err("Resident selected point normal is zero".into());
            }
            let q = d * d;
            let weight = ((amplitude * (3. * q * q - 4. * q * d + 1.)) / length) * u;
            for c in 0..3 {
                p[c] = (live[c] + n[c] * weight) as f32;
            }
        }
        self.refresh_selected(&positions, &selected)
    }
}
fn raw_face(positions: &[[f32; 4]], ids: [u32; 4]) -> Result<[f32; 3], String> {
    let p = |i: u32| {
        [
            positions[i as usize][0],
            positions[i as usize][1],
            positions[i as usize][2],
        ]
        .map(f64::from)
    };
    let [a, b, c] = [p(ids[0]), p(ids[1]), p(ids[2])];
    let sub = |a: [f64; 3], b: [f64; 3]| std::array::from_fn::<_, 3, _>(|c| a[c] - b[c]);
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let edge = sub(c, a);
    let mut n = cross(sub(b, a), edge);
    if ids[3] != TRIANGLE_CORNER {
        let other = cross(sub(c, p(ids[3])), edge);
        for c in 0..3 {
            n[c] += other[c];
        }
    }
    let n = n.map(|v| v as f32);
    if n.iter().any(|v| !v.is_finite()) {
        return Err("Resident face normal not finite".into());
    }
    Ok(n)
}
