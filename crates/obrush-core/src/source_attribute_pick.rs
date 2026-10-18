//! Read-only original-face Paint sampling with an ordered Float32 output store.
//! The published SculptGL Paint/Picking contract is distinct from both Legacy
//! paint sampling and Source's plain-Number picked-normal interpolation.
//! Distance uses the existing portable policy; V8 Math.hypot bits are unpaired.
use crate::{
    MaskConvention, Mesh, SourceOriginalFaceContact, DEFAULT_VERTEX_MATERIAL,
    MAX_SOURCE_PICK_COORDINATE, TRIANGLE_CORNER,
};
use serde::Serialize;

pub const MAX_SOURCE_ATTRIBUTE_PICK_REPORT_BYTES: usize = 4096;
pub const MAX_SOURCE_ATTRIBUTE_PICK_DIAGNOSTIC_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAttributePickMaskPolicy {
    RawUnprotected,
    DerivedProtectionCompatibility,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAttributePickRejection {
    InvalidMesh,
    InvalidContact,
    MissingOriginalFace,
    InvalidDistance,
    InvalidInverseArithmetic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAttributePickUndefined {
    ExactOrUnderflowedZeroDistance,
    IndividualInverseDistanceOverflow,
    NonfiniteInterpolation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "reason", rename_all = "snake_case")]
pub enum SourceAttributePickFailure {
    Rejected(SourceAttributePickRejection),
    Undefined(SourceAttributePickUndefined),
}
/// Error diagnostics contain words, never an ordinary nonfinite JSON number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourceAttributePickError {
    pub failure: SourceAttributePickFailure,
    pub local_point64_words: [[u32; 2]; 3],
    pub corner: Option<usize>,
    pub vertex_id: Option<u32>,
    pub numeric64_word: Option<[u32; 2]>,
}
impl std::fmt::Display for SourceAttributePickError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self.failure {
            SourceAttributePickFailure::Rejected(SourceAttributePickRejection::InvalidMesh) => {
                "Source attribute sampling requires a valid bounded mesh and fields"
            }
            SourceAttributePickFailure::Rejected(SourceAttributePickRejection::InvalidContact) => {
                "Source attribute contact must be bounded finite local Number coordinates"
            }
            SourceAttributePickFailure::Rejected(
                SourceAttributePickRejection::MissingOriginalFace,
            ) => "Source attribute original face is missing",
            SourceAttributePickFailure::Rejected(SourceAttributePickRejection::InvalidDistance) => {
                "Source attribute distance is invalid"
            }
            SourceAttributePickFailure::Rejected(
                SourceAttributePickRejection::InvalidInverseArithmetic,
            ) => "Source attribute inverse-distance arithmetic is invalid",
            SourceAttributePickFailure::Undefined(
                SourceAttributePickUndefined::ExactOrUnderflowedZeroDistance,
            ) => "Source attribute sample is undefined at a zero-distance original corner",
            SourceAttributePickFailure::Undefined(
                SourceAttributePickUndefined::IndividualInverseDistanceOverflow,
            ) => "Source attribute sample is undefined after an individual reciprocal overflow",
            SourceAttributePickFailure::Undefined(
                SourceAttributePickUndefined::NonfiniteInterpolation,
            ) => "Source attribute ordered Float32 interpolation is nonfinite",
        };
        f.write_str(text)
    }
}
impl std::error::Error for SourceAttributePickError {}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SourceAttributePickInverseSum {
    Finite {
        value64: f64,
    },
    /// All individually finite reciprocals produce zero weights in this branch.
    PositiveInfinityZeroWeights,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SourceAttributePickSample {
    pub rgb: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
    /// Diagnostic only. The Paint picker never changes a mask target.
    pub sampled_unprotected_mask: f32,
}
/// Fixed-size diagnostic records retain every ordered input/store word.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SourceAttributePickReport {
    pub original_face_index: usize,
    pub vertex_ids: [u32; 4],
    pub corner_count: usize,
    pub uv_corner_ids: Option<[u32; 4]>,
    pub local_point64: [f64; 3],
    pub local_point64_words: [[u32; 2]; 3],
    pub positions_f32_words: [[u32; 3]; 4],
    pub rgb_input_f32_words: [[u32; 3]; 4],
    pub material_mask_input_f32_words: [[u32; 3]; 4],
    pub raw_mask_f32_words: [u32; 4],
    pub materials_explicit: bool,
    pub mask_policy: SourceAttributePickMaskPolicy,
    pub distances64: [Option<f64>; 4],
    pub distance64_words: [Option<[u32; 2]>; 4],
    pub inverse_distances64: [f64; 4],
    pub inverse_distance64_words: [[u32; 2]; 4],
    pub ordered_inverse_sum: SourceAttributePickInverseSum,
    pub ordered_inverse_sum64_word: [u32; 2],
    pub reciprocal64: f64,
    pub reciprocal64_word: [u32; 2],
    pub weights64: [f64; 4],
    pub weight64_words: [[u32; 2]; 4],
    pub material_stages_f32_words: [[u32; 3]; 4],
    pub rgb_stages_f32_words: [[u32; 3]; 4],
    pub material_final_f32_words: [u32; 3],
    pub rgb_final_f32_words: [u32; 3],
    pub sample: SourceAttributePickSample,
    /// An owner may reject an out-of-domain candidate, but cannot clamp this result.
    pub targets_within_normalized_domain: bool,
    pub distance_policy: &'static str,
    pub output_policy: &'static str,
}
const _: () = assert!(
    std::mem::size_of::<SourceAttributePickReport>() <= MAX_SOURCE_ATTRIBUTE_PICK_REPORT_BYTES
);
const _: () = assert!(std::mem::size_of::<SourceAttributePickError>() <= 128);

/// Lossless high32/low32 diagnostic order, including signedzero/nonfinite bits.
fn number_word(value: f64) -> [u32; 2] {
    let bits = value.to_bits();
    [(bits >> 32) as u32, bits as u32]
}

fn failure(
    contact: SourceOriginalFaceContact,
    reason: SourceAttributePickFailure,
    detail: Option<(usize, u32, f64)>,
) -> SourceAttributePickError {
    SourceAttributePickError {
        failure: reason,
        local_point64_words: contact.local_point.map(number_word),
        corner: detail.map(|v| v.0),
        vertex_id: detail.map(|v| v.1),
        numeric64_word: detail.map(|v| number_word(v.2)),
    }
}
fn rejected(
    contact: SourceOriginalFaceContact,
    reason: SourceAttributePickRejection,
) -> SourceAttributePickError {
    failure(contact, SourceAttributePickFailure::Rejected(reason), None)
}

/// Validate the entire mesh first (O(mesh size)), then sample at most4corners.
/// No mesh mutation, materialization, normal field, topology or history is owned.
/// With original metadata, face_index is its ordinal; otherwise it names a
/// canonical triangle. The caller supplies the actual precise local contact.
pub fn prepare_source_attribute_pick(
    mesh: &Mesh,
    contact: SourceOriginalFaceContact,
) -> Result<SourceAttributePickReport, SourceAttributePickError> {
    mesh.validate()
        .map_err(|_| rejected(contact, SourceAttributePickRejection::InvalidMesh))?;
    sample_validated_source_attributes(mesh, contact)
}

// Only the validating convenience API and a privately sealed immutable snapshot
// may enter this unchanged scalar kernel. No public validation boolean exists.
pub(crate) fn sample_validated_source_attributes(
    mesh: &Mesh,
    contact: SourceOriginalFaceContact,
) -> Result<SourceAttributePickReport, SourceAttributePickError> {
    if contact
        .local_point
        .iter()
        .any(|v| !v.is_finite() || v.abs() > MAX_SOURCE_PICK_COORDINATE)
    {
        return Err(rejected(
            contact,
            SourceAttributePickRejection::InvalidContact,
        ));
    }
    let (face, uv_corner_ids) =
        if let Some(surface) = &mesh.surface {
            let face = *surface.faces.get(contact.face_index).ok_or_else(|| {
                rejected(contact, SourceAttributePickRejection::MissingOriginalFace)
            })?;
            (
                face,
                surface
                    .uv
                    .as_ref()
                    .map(|uv| uv.face_indices[contact.face_index]),
            )
        } else {
            let &[a, b, c] = mesh.triangles.get(contact.face_index).ok_or_else(|| {
                rejected(contact, SourceAttributePickRejection::MissingOriginalFace)
            })?;
            ([a, b, c, TRIANGLE_CORNER], None)
        };
    let corner_count = if face[3] == TRIANGLE_CORNER { 3 } else { 4 };
    let mut positions = [[0_u32; 3]; 4];
    let mut rgb_rows = [[0.0_f32; 3]; 4];
    let mut material_rows = [[0.0_f32; 3]; 4];
    let mut raw_masks = [0_u32; 4];
    let mut distances = [None; 4];
    let mut inverses = [0.0_f64; 4];
    for (corner, &id) in face[..corner_count].iter().enumerate() {
        let index = id as usize;
        let position = mesh.positions[index].to_array();
        positions[corner] = position.map(f32::to_bits);
        rgb_rows[corner] = mesh.colors[index];
        raw_masks[corner] = mesh.masks[index].to_bits();
        let unprotected: f32 = match mesh.mask_convention {
            MaskConvention::UnprotectedF32V1 => mesh.masks[index],
            MaskConvention::ProtectedF32V1 => 1.0 - mesh.masks[index],
        };
        let [roughness, metallic] = mesh
            .materials
            .as_ref()
            .map_or(DEFAULT_VERTEX_MATERIAL, |rows| rows[index]);
        material_rows[corner] = [roughness, metallic, unprotected];
        let delta = std::array::from_fn(|channel| {
            f64::from(position[channel]) - contact.local_point[channel]
        });
        let distance = crate::source_picked_normal::scaled_compensated_hypot3(delta);
        if distance == 0.0 {
            return Err(failure(
                contact,
                SourceAttributePickFailure::Undefined(
                    SourceAttributePickUndefined::ExactOrUnderflowedZeroDistance,
                ),
                Some((corner, id, distance)),
            ));
        }
        if !distance.is_finite() || distance < 0.0 {
            return Err(rejected(
                contact,
                SourceAttributePickRejection::InvalidDistance,
            ));
        }
        distances[corner] = Some(distance);
        inverses[corner] = 1.0 / distance;
        if inverses[corner] == f64::INFINITY {
            return Err(failure(
                contact,
                SourceAttributePickFailure::Undefined(
                    SourceAttributePickUndefined::IndividualInverseDistanceOverflow,
                ),
                Some((corner, id, inverses[corner])),
            ));
        }
        if !inverses[corner].is_finite() || inverses[corner] <= 0.0 {
            return Err(rejected(
                contact,
                SourceAttributePickRejection::InvalidInverseArithmetic,
            ));
        }
    }
    // Preserve the fourth +0 operation for triangles and original corner order.
    let sum = ((inverses[0] + inverses[1]) + inverses[2]) + inverses[3];
    if sum.is_nan() || sum <= 0.0 {
        return Err(rejected(
            contact,
            SourceAttributePickRejection::InvalidInverseArithmetic,
        ));
    }
    let reciprocal = 1.0 / sum;
    let weights = inverses.map(|inverse| inverse * reciprocal);
    if !reciprocal.is_finite() || weights.iter().any(|v| !v.is_finite()) {
        return Err(rejected(
            contact,
            SourceAttributePickRejection::InvalidInverseArithmetic,
        ));
    }
    // Material pass first, into a Float32 output; RGB then starts from +0.
    let (materials, material_stages) =
        ordered_stores(contact, face, corner_count, &material_rows, weights)?;
    let (rgb, rgb_stages) = ordered_stores(contact, face, corner_count, &rgb_rows, weights)?;
    let target_values = [rgb[0], rgb[1], rgb[2], materials[0], materials[1]];
    Ok(SourceAttributePickReport {
        original_face_index: contact.face_index,
        vertex_ids: face,
        corner_count,
        uv_corner_ids,
        local_point64: contact.local_point,
        local_point64_words: contact.local_point.map(number_word),
        positions_f32_words: positions,
        rgb_input_f32_words: rgb_rows.map(|v| v.map(f32::to_bits)),
        material_mask_input_f32_words: material_rows.map(|v| v.map(f32::to_bits)),
        raw_mask_f32_words: raw_masks,
        materials_explicit: mesh.materials.is_some(),
        mask_policy: match mesh.mask_convention {
            MaskConvention::UnprotectedF32V1 => SourceAttributePickMaskPolicy::RawUnprotected,
            MaskConvention::ProtectedF32V1 => SourceAttributePickMaskPolicy::DerivedProtectionCompatibility,
        },
        distances64: distances,
        distance64_words: distances.map(|v| v.map(number_word)),
        inverse_distances64: inverses,
        inverse_distance64_words: inverses.map(number_word),
        ordered_inverse_sum: if sum == f64::INFINITY {
            SourceAttributePickInverseSum::PositiveInfinityZeroWeights
        } else {
            SourceAttributePickInverseSum::Finite { value64: sum }
        },
        ordered_inverse_sum64_word: number_word(sum),
        reciprocal64: reciprocal,
        reciprocal64_word: number_word(reciprocal),
        weights64: weights,
        weight64_words: weights.map(number_word),
        material_stages_f32_words: material_stages,
        rgb_stages_f32_words: rgb_stages,
        material_final_f32_words: materials.map(f32::to_bits),
        rgb_final_f32_words: rgb.map(f32::to_bits),
        sample: SourceAttributePickSample {
            rgb,
            roughness: materials[0],
            metallic: materials[1],
            sampled_unprotected_mask: materials[2],
        },
        targets_within_normalized_domain: target_values.iter().all(|v| (0.0..=1.0).contains(v)),
        distance_policy: "scaled-compensated-hypot3; V8 Math.hypot bit equivalence unpaired",
        output_policy: "original-corner order; separate Number multiply/add; Float32 store each corner; material then RGB",
    })
}

fn ordered_stores(
    contact: SourceOriginalFaceContact,
    face: [u32; 4],
    corner_count: usize,
    rows: &[[f32; 3]; 4],
    weights: [f64; 4],
) -> Result<([f32; 3], [[u32; 3]; 4]), SourceAttributePickError> {
    let mut out = [0.0_f32; 3];
    let mut stages = [[0_u32; 3]; 4];
    for corner in 0..corner_count {
        for (channel, target) in out.iter_mut().enumerate() {
            let product = f64::from(rows[corner][channel]) * weights[corner];
            let sum = f64::from(*target) + product;
            // No mul_add, f32 product, reduction, clamp or carried f64 output.
            *target = sum as f32;
            if !target.is_finite() {
                return Err(failure(
                    contact,
                    SourceAttributePickFailure::Undefined(
                        SourceAttributePickUndefined::NonfiniteInterpolation,
                    ),
                    Some((corner, face[corner], sum)),
                ));
            }
        }
        stages[corner] = out.map(f32::to_bits);
    }
    Ok((out, stages))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CornerUv, Hit, PaintPickInterpolation, SurfaceTopology, Vec3, MAX_VERTICES};

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
