//! Bounded explicit static original-face IDW normal preparation.
//! Positions/raw normals read Float32; intersection, weights and output stay f64.
//! Source vec3.dist uses implementation-approximated Math.hypot: the shared
//! scaled compensated policy below requires a future paired distance gate.
use crate::{Mesh, SourceNormalField, SourceNormalPolicy, TRIANGLE_CORNER};
use serde::Serialize;

pub const MAX_SOURCE_PICK_COORDINATE: f64 = 1_000_000.0;

/// Causes which do not define an alpha frame, but do not affect alphaNone's
/// getAlpha==1. These are never replacement normal directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum SourcePickedNormalUndefined {
    ExactVertexContact { corner: usize, vertex_id: u32 },
    InverseDistanceOverflow { corner: usize, vertex_id: u32 },
    InverseSumOverflow,
    ZeroDirection,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePickedNormalRejection {
    StaleOrNonstaticField,
    InvalidContact,
    MissingOriginalFace,
    NonfiniteDistance,
    InvalidInverseArithmetic,
    NonfiniteInterpolation,
}
impl std::fmt::Display for SourcePickedNormalRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::StaleOrNonstaticField => {
                "Source picked normal requires a current static original-face field"
            }
            Self::InvalidContact => {
                "Source picked-normal intersection must be bounded finite Number coordinates"
            }
            Self::MissingOriginalFace => "Source picked original face is missing",
            Self::NonfiniteDistance => "Source picked-normal distance is nonfinite",
            Self::InvalidInverseArithmetic => {
                "Source picked-normal inverse-distance sum is undefined"
            }
            Self::NonfiniteInterpolation => {
                "Source picked-normal interpolation/normalization is nonfinite"
            }
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum SourcePickedNormalError {
    Undefined(SourcePickedNormalUndefined),
    Rejected(SourcePickedNormalRejection),
}
impl std::fmt::Display for SourcePickedNormalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected(reason) => std::fmt::Display::fmt(reason, f),
            Self::Undefined(SourcePickedNormalUndefined::ExactVertexContact { .. }) => f.write_str("Source picked normal is undefined at an exact/underflowed original-face vertex contact"),
            Self::Undefined(_) => f.write_str("Source picked-normal inverse-distance sum is undefined"),
        }
    }
}
impl std::error::Error for SourcePickedNormalError {}
impl std::error::Error for SourcePickedNormalRejection {}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SourcePickedNormalDisposition {
    Defined {
        report: SourcePickedNormalReport,
    },
    /// Explicit alphaNone-only admission. No fabricated picked normal/alpha frame.
    Undefined {
        reason: SourcePickedNormalUndefined,
        original_face_index: usize,
        local_point64: [f64; 3],
        /// Only ZeroDirection retains a finite raw zero report.
        raw_report: Option<SourcePickedNormalReport>,
    },
}
impl SourcePickedNormalDisposition {
    pub fn raw_report(self) -> Option<SourcePickedNormalReport> {
        match self {
            Self::Defined { report } => Some(report),
            Self::Undefined { raw_report, .. } => raw_report,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SourceOriginalFaceContact {
    /// Actual original face identity, not an arbitrary matching triangle search.
    pub face_index: usize,
    /// Source Picking._interPoint Number array, in local coordinates.
    pub local_point: [f64; 3],
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct SourcePickedNormalReport {
    pub policy: SourceNormalPolicy,
    pub original_face_index: usize,
    pub vertex_ids: [u32; 4],
    pub corner_count: usize,
    pub local_point64: [f64; 3],
    pub distances64: [Option<f64>; 4],
    pub inverse_distances64: [f64; 4],
    pub weights64: [f64; 4],
    pub interpolated64: [f64; 3],
    pub normal64: [f64; 3],
    pub normalization_length_squared64: f64,
    pub direction_defined: bool,
    /// Native/WASM deterministic policy, not verified V8 Math.hypot bit equality.
    pub distance_policy: &'static str,
}

pub fn prepare_source_picked_normal(
    mesh: &Mesh,
    field: &SourceNormalField,
    contact: SourceOriginalFaceContact,
) -> Result<SourcePickedNormalReport, String> {
    prepare_source_picked_normal_typed(mesh, field, contact).map_err(|error| error.to_string())
}

/// Typed version of the original strict query. Undefined states remain errors.
pub fn prepare_source_picked_normal_typed(
    mesh: &Mesh,
    field: &SourceNormalField,
    contact: SourceOriginalFaceContact,
) -> Result<SourcePickedNormalReport, SourcePickedNormalError> {
    if field.report().policy != SourceNormalPolicy::StaticOriginalFaces || !field.matches_mesh(mesh)
    {
        return Err(SourcePickedNormalError::Rejected(
            SourcePickedNormalRejection::StaleOrNonstaticField,
        ));
    }
    prepare_source_picked_normal_raw(mesh, field.vertex_normals(), field.report().policy, contact)
}

/// The existing strict/fresh API validates its own field first. The separate
/// geometry cache validates exact current words before entering this unchanged
/// scalar IDW routine with its explicit StaticSelectedRefresh policy.
pub(crate) fn prepare_source_picked_normal_raw(
    mesh: &Mesh,
    raw_normals: &[crate::Vec3],
    policy: SourceNormalPolicy,
    contact: SourceOriginalFaceContact,
) -> Result<SourcePickedNormalReport, SourcePickedNormalError> {
    if raw_normals.len() != mesh.positions.len() {
        return Err(SourcePickedNormalError::Rejected(
            SourcePickedNormalRejection::StaleOrNonstaticField,
        ));
    }
    if contact
        .local_point
        .iter()
        .any(|v| !v.is_finite() || v.abs() > MAX_SOURCE_PICK_COORDINATE)
    {
        return Err(SourcePickedNormalError::Rejected(
            SourcePickedNormalRejection::InvalidContact,
        ));
    }
    let face = if let Some(surface) = &mesh.surface {
        *surface
            .faces
            .get(contact.face_index)
            .ok_or(SourcePickedNormalError::Rejected(
                SourcePickedNormalRejection::MissingOriginalFace,
            ))?
    } else {
        let &[a, b, c] =
            mesh.triangles
                .get(contact.face_index)
                .ok_or(SourcePickedNormalError::Rejected(
                    SourcePickedNormalRejection::MissingOriginalFace,
                ))?;
        [a, b, c, TRIANGLE_CORNER]
    };
    let corner_count = if face[3] == TRIANGLE_CORNER { 3 } else { 4 };
    let mut distances = [None; 4];
    let mut inverse = [0.0; 4];
    for (corner, &vertex_id) in face[..corner_count].iter().enumerate() {
        let vertex = mesh.positions[vertex_id as usize].to_array().map(f64::from);
        // gl-matrix3.3.0 distance(a,b) evaluates b-a, then Math.hypot.
        let delta = std::array::from_fn(|i| vertex[i] - contact.local_point[i]);
        let distance = scaled_compensated_hypot3(delta);
        if distance == 0.0 {
            // Source inverse-distance Infinity and Infinity*0 weight are NaN.
            // Never silently snap to a corner or substitute barycentric/Legacy.
            return Err(SourcePickedNormalError::Undefined(
                SourcePickedNormalUndefined::ExactVertexContact { corner, vertex_id },
            ));
        }
        if !distance.is_finite() {
            return Err(SourcePickedNormalError::Rejected(
                SourcePickedNormalRejection::NonfiniteDistance,
            ));
        }
        distances[corner] = Some(distance);
        inverse[corner] = 1.0 / distance;
        if inverse[corner] == f64::INFINITY {
            return Err(SourcePickedNormalError::Undefined(
                SourcePickedNormalUndefined::InverseDistanceOverflow { corner, vertex_id },
            ));
        }
        if !inverse[corner].is_finite() {
            return Err(SourcePickedNormalError::Rejected(
                SourcePickedNormalRejection::InvalidInverseArithmetic,
            ));
        }
    }
    // Source includes len4==0 for triangles, in original corner order.
    let sum = ((inverse[0] + inverse[1]) + inverse[2]) + inverse[3];
    let reciprocal = 1.0 / sum;
    if sum == f64::INFINITY {
        return Err(SourcePickedNormalError::Undefined(
            SourcePickedNormalUndefined::InverseSumOverflow,
        ));
    }
    if !sum.is_finite() || sum <= 0.0 || !reciprocal.is_finite() {
        return Err(SourcePickedNormalError::Rejected(
            SourcePickedNormalRejection::InvalidInverseArithmetic,
        ));
    }
    let weights = inverse.map(|v| v * reciprocal);
    let mut interpolated = [0.0; 3];
    for (corner, &vertex_id) in face[..corner_count].iter().enumerate() {
        let normal = raw_normals[vertex_id as usize].to_array().map(f64::from);
        // Plain output arrays: no Float32 assignment between scaleAndAdd calls.
        for (out, value) in interpolated.iter_mut().zip(normal) {
            *out += value * weights[corner];
        }
    }
    let [x, y, z] = interpolated;
    let length_squared = (x * x + y * y) + z * z;
    // Preserve gl-matrix normalize's zero-length multiplication schedule.
    let factor = if length_squared > 0.0 {
        1.0 / length_squared.sqrt()
    } else {
        length_squared
    };
    let normal = interpolated.map(|v| v * factor);
    if weights
        .iter()
        .chain(&interpolated)
        .chain(&normal)
        .any(|v| !v.is_finite())
        || !length_squared.is_finite()
    {
        return Err(SourcePickedNormalError::Rejected(
            SourcePickedNormalRejection::NonfiniteInterpolation,
        ));
    }
    Ok(SourcePickedNormalReport {
        policy,
        original_face_index: contact.face_index,
        vertex_ids: face,
        corner_count,
        local_point64: contact.local_point,
        distances64: distances,
        inverse_distances64: inverse,
        weights64: weights,
        interpolated64: interpolated,
        normal64: normal,
        normalization_length_squared64: length_squared,
        direction_defined: length_squared > 0.0,
        distance_policy: "scaled-compensated-hypot3; Math.hypot bit equivalence unpaired",
    })
}
/// Only alphaNone may disregard an undefined alpha-frame normal. Real invalid
/// contact/cache/geometry arithmetic still rejects; no normal fallback is used.
pub fn prepare_source_picked_normal_alpha_none(
    mesh: &Mesh,
    field: &SourceNormalField,
    contact: SourceOriginalFaceContact,
) -> Result<SourcePickedNormalDisposition, SourcePickedNormalRejection> {
    match prepare_source_picked_normal_typed(mesh, field, contact) {
        Ok(report) if report.direction_defined => {
            Ok(SourcePickedNormalDisposition::Defined { report })
        }
        Ok(report) => Ok(SourcePickedNormalDisposition::Undefined {
            reason: SourcePickedNormalUndefined::ZeroDirection,
            original_face_index: contact.face_index,
            local_point64: contact.local_point,
            raw_report: Some(report),
        }),
        Err(SourcePickedNormalError::Undefined(reason)) => {
            Ok(SourcePickedNormalDisposition::Undefined {
                reason,
                original_face_index: contact.face_index,
                local_point64: contact.local_point,
                raw_report: None,
            })
        }
        Err(SourcePickedNormalError::Rejected(reason)) => Err(reason),
    }
}

pub(crate) fn scaled_compensated_hypot3(values: [f64; 3]) -> f64 {
    let values = values.map(f64::abs);
    let maximum = values[0].max(values[1]).max(values[2]);
    if maximum == 0.0 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for value in values {
        let ratio = value / maximum;
        let adjusted = ratio * ratio - compensation;
        let next = sum + adjusted;
        compensation = (next - sum) - adjusted;
        sum = next;
    }
    maximum * sum.sqrt()
}
