//! Ordinary static Flatten's original-order plane and pre-guard candidate.
//! Fixed stack storage only; shared transaction/guard own accepted publication.
use super::{SourceFlattenPlane, SourceGeometryError, SourceGeometryPass};
use crate::{Mesh, Vec3};

pub(super) fn plane(
    mesh: &Mesh,
    normals: &[Vec3],
    front: &[usize],
) -> Result<Option<SourceFlattenPlane>, SourceGeometryError> {
    let mut sum = [0.0; 3];
    let mut center = [0.0; 3];
    let mut total = 0.0;
    for &v in front {
        let u = f64::from(mesh.masks[v]);
        total += u;
        for c in 0..3 {
            sum[c] += f64::from(normals[v][c]) * u;
            center[c] += f64::from(mesh.positions[v][c]) * u;
        }
    }
    let length = ((sum[0] * sum[0] + sum[1] * sum[1]) + sum[2] * sum[2]).sqrt();
    if length == 0.0 {
        return Ok(None);
    }
    if !length.is_finite() || !total.is_finite() || total <= 0.0 {
        return Err(SourceGeometryError::NonfiniteCandidate);
    }
    let inverse = 1.0 / length;
    let normal = sum.map(|v| v * inverse);
    let center = center.map(|v| v / total);
    if normal.iter().chain(&center).any(|v| !v.is_finite()) {
        return Err(SourceGeometryError::NonfiniteCandidate);
    }
    Ok(Some(SourceFlattenPlane {
        center64: center,
        normal64: normal,
        weight_sum: total,
        front_vertices: front.len(),
    }))
}

/// Calls alpha only on an admitted unprotected side/radius row. Literal zero
/// components remain unchanged under the shared OBrush protected-word policy.
pub(super) fn candidate(
    point: Vec3,
    pass: SourceGeometryPass,
    intensity: f64,
    negative: bool,
    u: f64,
    plane: SourceFlattenPlane,
    alpha: impl FnOnce(Vec3) -> Result<f64, SourceGeometryError>,
) -> Result<Vec3, SourceGeometryError> {
    if u == 0.0 {
        return Ok(point);
    }
    let live = point.to_array().map(f64::from);
    let n = plane.normal64;
    let a = plane.center64;
    let distance = ((live[0] - a[0]) * n[0] + (live[1] - a[1]) * n[1]) + (live[2] - a[2]) * n[2];
    let comp = if negative { -1.0 } else { 1.0 };
    if distance * comp > 0.0 {
        return Ok(point);
    }
    let center = pass.center;
    let dx = live[0] - center[0];
    let dy = live[1] - center[1];
    let dz = live[2] - center[2];
    let d = ((dx * dx + dy * dy) + dz * dz).sqrt() / pass.local_radius_squared.sqrt();
    if d >= 1.0 {
        return Ok(point);
    }
    let q = d * d;
    let mut weight = (3.0 * q * q - 4.0 * q * d) + 1.0;
    weight *= ((distance * intensity) * u) * alpha(point)?;
    let mut result = point;
    for c in 0..3 {
        let motion = n[c] * weight;
        let value = (live[c] - motion) as f32;
        if !value.is_finite() || value.abs() > 1_000_000.0 {
            return Err(SourceGeometryError::NonfiniteCandidate);
        }
        if motion != 0.0 {
            result[c] = value;
        }
    }
    Ok(result)
}
