//! Checked, transient position/normal rows for a future Source Worker bridge.
//!
//! This codec deliberately does not publish into a `Document`: a host must
//! separately prove its one-use epoch, document revision, scene/selection
//! witness, retained Source normal state and history budget. Decoding only
//! constructs a validated candidate mesh. The bytes are not a project format.
use crate::{Mesh, MAX_VERTICES};
use glam::Vec3;

const MAGIC: &[u8; 8] = b"OBRSDEL1";
const VERSION: u32 = 1;
const HEADER_BYTES: usize = 56;
const ROW_BYTES: usize = 28;
const CHECKSUM_BYTES: usize = 32;
pub const MAX_SOURCE_WORKER_DELTA_BYTES: usize = 32 * 1024 * 1024;

fn geometry_digest(mesh: &Mesh) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"OBR-SOURCE-WORKER-GEOMETRY-1");
    hash.update(&(mesh.positions.len() as u64).to_le_bytes());
    for point in mesh.positions.iter().chain(&mesh.normals) {
        for word in point.to_array() {
            hash.update(&word.to_bits().to_le_bytes());
        }
    }
    *hash.finalize().as_bytes()
}

fn same_words(a: Vec3, b: Vec3) -> bool {
    a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits)
}

fn same_non_geometry(a: &Mesh, b: &Mesh) -> bool {
    a.triangles == b.triangles
        && a.attributes_equal_bits(b)
        && a.masks.len() == b.masks.len()
        && a.masks
            .iter()
            .zip(&b.masks)
            .all(|(a, b)| a.to_bits() == b.to_bits())
        && a.colors.len() == b.colors.len()
        && a.colors
            .iter()
            .zip(&b.colors)
            .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
}

/// Encode changed rows against the worker's pre-stroke public mesh. The
/// complete after-mesh is checked so topology or channel edits cannot hide in
/// a position-only transfer. There is no 100k trial cap: the byte limit admits
/// up to one million 28-byte rows while rejecting larger allocations.
pub fn encode_source_worker_mesh_delta(
    before: &Mesh,
    after: &Mesh,
    active_subtool_id: u32,
) -> Result<Vec<u8>, String> {
    before.validate()?;
    after.validate()?;
    if active_subtool_id == 0
        || before.positions.len() != after.positions.len()
        || before.normals.len() != after.normals.len()
        || !same_non_geometry(before, after)
    {
        return Err("Source worker changed the target or non-geometry mesh state".into());
    }
    let changed = before
        .positions
        .iter()
        .zip(&before.normals)
        .zip(after.positions.iter().zip(&after.normals))
        .filter(|((old_position, old_normal), (new_position, new_normal))| {
            !same_words(**old_position, **new_position) || !same_words(**old_normal, **new_normal)
        })
        .count();
    let total = changed
        .checked_mul(ROW_BYTES)
        .and_then(|size| size.checked_add(HEADER_BYTES + CHECKSUM_BYTES))
        .ok_or("Source worker delta size overflow")?;
    if total > MAX_SOURCE_WORKER_DELTA_BYTES {
        return Err("Source worker delta exceeds transfer limit".into());
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(total)
        .map_err(|_| "Cannot reserve Source worker delta")?;
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&VERSION.to_le_bytes());
    output.extend_from_slice(&(before.positions.len() as u32).to_le_bytes());
    output.extend_from_slice(&(changed as u32).to_le_bytes());
    output.extend_from_slice(&active_subtool_id.to_le_bytes());
    output.extend_from_slice(&geometry_digest(before));
    for (index, ((old_position, old_normal), (new_position, new_normal))) in before
        .positions
        .iter()
        .zip(&before.normals)
        .zip(after.positions.iter().zip(&after.normals))
        .enumerate()
    {
        if same_words(*old_position, *new_position) && same_words(*old_normal, *new_normal) {
            continue;
        }
        output.extend_from_slice(&(index as u32).to_le_bytes());
        for word in new_position
            .to_array()
            .into_iter()
            .chain(new_normal.to_array())
        {
            output.extend_from_slice(&word.to_bits().to_le_bytes());
        }
    }
    debug_assert_eq!(output.len() + CHECKSUM_BYTES, total);
    let checksum = blake3::hash(&output);
    output.extend_from_slice(checksum.as_bytes());
    Ok(output)
}

/// Return a validated mesh candidate, or `None` for an empty packet. This
/// never changes the caller's document, history, or retained Source state.
pub fn decode_source_worker_mesh_delta(
    before: &Mesh,
    active_subtool_id: u32,
    bytes: &[u8],
) -> Result<Option<Mesh>, String> {
    if bytes.len() < HEADER_BYTES + CHECKSUM_BYTES
        || bytes.len() > MAX_SOURCE_WORKER_DELTA_BYTES
        || &bytes[..8] != MAGIC
        || u32::from_le_bytes(bytes[8..12].try_into().unwrap()) != VERSION
    {
        return Err("Source worker delta header or size is invalid".into());
    }
    let count = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let rows = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    let target = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
    if target == 0
        || target != active_subtool_id
        || count == 0
        || count > MAX_VERTICES
        || count != before.positions.len()
        || count != before.normals.len()
        || rows > count
        || rows
            .checked_mul(ROW_BYTES)
            .and_then(|size| size.checked_add(HEADER_BYTES + CHECKSUM_BYTES))
            != Some(bytes.len())
    {
        return Err("Source worker delta shape or target is invalid".into());
    }
    let checksum_at = bytes.len() - CHECKSUM_BYTES;
    if blake3::hash(&bytes[..checksum_at]).as_bytes() != &bytes[checksum_at..] {
        return Err("Source worker delta checksum mismatch".into());
    }
    before.validate()?;
    if geometry_digest(before).as_slice() != &bytes[24..HEADER_BYTES] {
        return Err("Source worker delta has a stale mesh baseline".into());
    }
    if rows == 0 {
        return Ok(None);
    }
    let mut candidate = before.try_snapshot()?;
    let mut last = None;
    for row in bytes[HEADER_BYTES..checksum_at].chunks_exact(ROW_BYTES) {
        let index = u32::from_le_bytes(row[..4].try_into().unwrap()) as usize;
        if index >= count || last.is_some_and(|previous| index <= previous) {
            return Err("Source worker delta rows must be sorted, unique and in range".into());
        }
        last = Some(index);
        let mut words = [0.0_f32; 6];
        for (out, encoded) in words.iter_mut().zip(row[4..].chunks_exact(4)) {
            *out = f32::from_bits(u32::from_le_bytes(encoded.try_into().unwrap()));
        }
        let position = Vec3::from_array([words[0], words[1], words[2]]);
        let normal = Vec3::from_array([words[3], words[4], words[5]]);
        if same_words(candidate.positions[index], position)
            && same_words(candidate.normals[index], normal)
        {
            return Err("Source worker delta contains an unchanged row".into());
        }
        candidate.positions[index] = position;
        candidate.normals[index] = normal;
    }
    candidate.validate()?;
    Ok(Some(candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Document, MaskConvention, SourceGeometryAlphaTracks, SourceGeometryControls,
        SourceGeometryOptions, SourceGeometryPass, SourceWorkerTransaction,
    };

    fn mesh() -> Mesh {
        Mesh::import_obj(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")
            .unwrap()
            .mesh
    }

    fn changed() -> (Mesh, Mesh) {
        let before = mesh();
        let mut after = before.snapshot();
        after.positions[0].z = 0.25;
        after.normals[0] = Vec3::new(0.0, 0.5, 0.5);
        (before, after)
    }

    fn reseal(bytes: &mut [u8]) {
        let split = bytes.len() - CHECKSUM_BYTES;
        let digest = blake3::hash(&bytes[..split]);
        bytes[split..].copy_from_slice(digest.as_bytes());
    }

    #[test]
    fn changed_rows_round_trip_exact_position_and_normal_words() {
        let (before, after) = changed();
        let bytes = encode_source_worker_mesh_delta(&before, &after, 7).unwrap();
        assert_eq!(bytes.len(), HEADER_BYTES + ROW_BYTES + CHECKSUM_BYTES);
        let result = decode_source_worker_mesh_delta(&before, 7, &bytes)
            .unwrap()
            .unwrap();
        for (actual, expected) in result.positions.iter().zip(&after.positions) {
            assert!(same_words(*actual, *expected));
        }
        for (actual, expected) in result.normals.iter().zip(&after.normals) {
            assert!(same_words(*actual, *expected));
        }
        assert_eq!(result.triangles, before.triangles);
    }

    #[test]
    fn rejects_stale_checksum_target_rows_nonfinite_and_non_geometry_edit() {
        let (before, after) = changed();
        let bytes = encode_source_worker_mesh_delta(&before, &after, 7).unwrap();
        let mut stale = before.snapshot();
        stale.positions[2].z = 0.125;
        assert!(decode_source_worker_mesh_delta(&stale, 7, &bytes).is_err());
        assert!(decode_source_worker_mesh_delta(&before, 8, &bytes).is_err());
        let mut bad = bytes.clone();
        bad[HEADER_BYTES + 5] ^= 1;
        assert!(decode_source_worker_mesh_delta(&before, 7, &bad).is_err());
        let mut bad = bytes.clone();
        bad[HEADER_BYTES..HEADER_BYTES + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        reseal(&mut bad);
        assert!(decode_source_worker_mesh_delta(&before, 7, &bad).is_err());
        let mut bad = bytes.clone();
        bad[HEADER_BYTES + 4..HEADER_BYTES + 8].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        reseal(&mut bad);
        assert!(decode_source_worker_mesh_delta(&before, 7, &bad).is_err());
        let mut altered = after;
        altered.masks[0] = 0.5;
        assert!(encode_source_worker_mesh_delta(&before, &altered, 7).is_err());
    }

    #[test]
    fn unchanged_result_has_checked_empty_packet() {
        let baseline = mesh();
        let packet = encode_source_worker_mesh_delta(&baseline, &baseline, 1).unwrap();
        assert_eq!(packet.len(), HEADER_BYTES + CHECKSUM_BYTES);
        assert!(decode_source_worker_mesh_delta(&baseline, 1, &packet)
            .unwrap()
            .is_none());
    }

    #[test]
    fn signed_zero_is_a_changed_word() {
        let baseline = mesh();
        let mut after = baseline.snapshot();
        after.positions[0].z = -0.0;
        let packet = encode_source_worker_mesh_delta(&baseline, &after, 1).unwrap();
        assert_eq!(packet.len(), HEADER_BYTES + ROW_BYTES + CHECKSUM_BYTES);
        let candidate = decode_source_worker_mesh_delta(&baseline, 1, &packet)
            .unwrap()
            .unwrap();
        assert_eq!(candidate.positions[0].z.to_bits(), (-0.0f32).to_bits());
    }

    #[test]
    fn rejects_duplicate_or_unsorted_rows_after_valid_checksum() {
        let before = mesh();
        let mut after = before.snapshot();
        after.positions[0].z = 0.25;
        after.positions[1].z = 0.5;
        let packet = encode_source_worker_mesh_delta(&before, &after, 1).unwrap();
        assert_eq!(packet.len(), HEADER_BYTES + 2 * ROW_BYTES + CHECKSUM_BYTES);
        let mut duplicate = packet.clone();
        duplicate[HEADER_BYTES + ROW_BYTES..HEADER_BYTES + ROW_BYTES + 4]
            .copy_from_slice(&0u32.to_le_bytes());
        reseal(&mut duplicate);
        assert!(decode_source_worker_mesh_delta(&before, 1, &duplicate).is_err());
        let mut unsorted = packet;
        unsorted[HEADER_BYTES..HEADER_BYTES + 4].copy_from_slice(&1u32.to_le_bytes());
        unsorted[HEADER_BYTES + ROW_BYTES..HEADER_BYTES + ROW_BYTES + 4]
            .copy_from_slice(&0u32.to_le_bytes());
        reseal(&mut unsorted);
        assert!(decode_source_worker_mesh_delta(&before, 1, &unsorted).is_err());
    }

    #[test]
    fn completed_source_owner_produces_checked_mesh_candidate() {
        let mut baseline = Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nf 1 2 3\n")
            .unwrap()
            .mesh;
        baseline.mask_convention = MaskConvention::UnprotectedF32V1;
        baseline.masks.fill(1.0);
        let mut tx = SourceWorkerTransaction::begin(
            Document::new(baseline.snapshot()),
            SourceGeometryOptions {
                intensity: 0.625,
                ..SourceGeometryOptions::inflate()
            },
            14,
        )
        .unwrap();
        let report = tx
            .apply_controlled(
                14,
                1,
                [
                    Some(SourceGeometryPass {
                        center: [0.0; 3],
                        eye_direction: [0.0, 0.0, -1.0],
                        local_radius_squared: 4.0,
                        original_face_index: 0,
                    }),
                    None,
                ],
                0.5,
                true,
                &SourceGeometryControls::default(),
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        assert!(report.combined.changed_vertices > 0);
        let finished = tx.finish(14, 2).unwrap();
        let after = finished.require_active_mesh().unwrap();
        let packet = encode_source_worker_mesh_delta(&baseline, after, 1).unwrap();
        let candidate = decode_source_worker_mesh_delta(&baseline, 1, &packet)
            .unwrap()
            .unwrap();
        for (candidate, expected) in candidate.positions.iter().zip(&after.positions) {
            assert!(same_words(*candidate, *expected));
        }
        for (candidate, expected) in candidate.normals.iter().zip(&after.normals) {
            assert!(same_words(*candidate, *expected));
        }
        assert!(finished.source_geometry_state(1).is_some());
        // A candidate Mesh is not the complete retained normal state.
    }
}
