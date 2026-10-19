//! Fixed-size witness for a seed-equivalent retained Source producer.
//!
//! This deliberately refuses warm selected-refresh rows and historical tree
//! layouts. It does not transfer a Document, its history, or an active stroke.
use crate::{Mesh, SourceGeometryNormalState};

const MAGIC: &[u8; 8] = b"OBRSN0V1";
const VERSION: u32 = 1;
const MESH_DIGEST_AT: usize = 8 + 4 + 4;
const STATE_DIGEST_AT: usize = MESH_DIGEST_AT + 32;
const PAYLOAD_BYTES: usize = STATE_DIGEST_AT + 32;
const PACKET_BYTES: usize = PAYLOAD_BYTES + 32;

pub(crate) fn seed_mesh_digest(mesh: &Mesh) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"OBR-SOURCE-RETAINED-SEED-MESH-1");
    hash.update(&(mesh.positions.len() as u64).to_le_bytes());
    hash.update(&(mesh.triangles.len() as u64).to_le_bytes());
    hash.update(&[u8::from(mesh.surface.is_some())]);
    for point in &mesh.positions {
        for word in point.to_array() {
            hash.update(&word.to_bits().to_le_bytes());
        }
    }
    for triangle in &mesh.triangles {
        for index in triangle {
            hash.update(&index.to_le_bytes());
        }
    }
    if let Some(surface) = &mesh.surface {
        hash.update(&(surface.faces.len() as u64).to_le_bytes());
        for face in &surface.faces {
            for index in face {
                hash.update(&index.to_le_bytes());
            }
        }
    }
    *hash.finalize().as_bytes()
}

/// Encode one retained producer only when it is bit-for-bit equivalent to
/// reconstructing a fresh Source seed from this mesh. The fixed 112-byte
/// packet carries a target ID, exact mesh and rebuilt-state witnesses and
/// checksum, not the warm tree.
pub fn encode_source_worker_retained_seed(
    state: &SourceGeometryNormalState,
    mesh: &Mesh,
    subtool_id: u32,
) -> Result<Vec<u8>, String> {
    if subtool_id == 0
        || !state
            .matches_seed_reconstruction(mesh)
            .map_err(|e| e.to_string())?
    {
        return Err("Source retained state is warm, stale or not seed-equivalent".into());
    }
    let mut packet = Vec::new();
    packet
        .try_reserve_exact(PACKET_BYTES)
        .map_err(|_| "Cannot reserve Source seed packet")?;
    packet.extend_from_slice(MAGIC);
    packet.extend_from_slice(&VERSION.to_le_bytes());
    packet.extend_from_slice(&subtool_id.to_le_bytes());
    packet.extend_from_slice(&seed_mesh_digest(mesh));
    packet.extend_from_slice(&state.seed_fingerprint());
    packet.extend_from_slice(blake3::hash(&packet).as_bytes());
    debug_assert_eq!(packet.len(), PACKET_BYTES);
    Ok(packet)
}

/// Restore a fresh retained producer, with its deterministic ordered tree,
/// only against the exact checked seed mesh. No Document/history is modified.
pub fn decode_source_worker_retained_seed(
    mesh: &Mesh,
    subtool_id: u32,
    packet: &[u8],
) -> Result<SourceGeometryNormalState, String> {
    if subtool_id == 0
        || packet.len() != PACKET_BYTES
        || &packet[..8] != MAGIC
        || u32::from_le_bytes(packet[8..12].try_into().unwrap()) != VERSION
        || u32::from_le_bytes(packet[12..16].try_into().unwrap()) != subtool_id
        || blake3::hash(&packet[..PAYLOAD_BYTES]).as_bytes() != &packet[PAYLOAD_BYTES..]
    {
        return Err("Source retained seed packet header, target or checksum is invalid".into());
    }
    mesh.validate()?;
    if seed_mesh_digest(mesh).as_slice() != &packet[MESH_DIGEST_AT..STATE_DIGEST_AT] {
        return Err("Source retained seed packet has a stale mesh baseline".into());
    }
    let restored = SourceGeometryNormalState::seed(mesh).map_err(|e| e.to_string())?;
    if restored.seed_fingerprint().as_slice() != &packet[STATE_DIGEST_AT..PAYLOAD_BYTES] {
        return Err("Source retained seed packet reconstructed different state words".into());
    }
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Document, MaskConvention, SourceGeometryOptions, SourceGeometryPass, SourceGeometryStroke,
    };

    fn fixture() -> Mesh {
        let mut mesh = Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nf 1 2 3\n")
            .unwrap()
            .mesh;
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        mesh
    }

    #[test]
    fn seed_round_trip_preserves_later_dab_mesh_and_retained_report() {
        let mesh = fixture();
        let state = SourceGeometryNormalState::seed(&mesh).unwrap();
        let packet = encode_source_worker_retained_seed(&state, &mesh, 7).unwrap();
        assert_eq!(packet.len(), PACKET_BYTES);
        // Recreate the receiver mesh through the public project path rather
        // than borrowing the sender allocation across a simulated realm.
        let project = Document::new(mesh.snapshot()).to_json().unwrap();
        let receiver = Document::from_json(&project).unwrap();
        let received_mesh = receiver.active_mesh().unwrap();
        let restored = decode_source_worker_retained_seed(received_mesh, 7, &packet).unwrap();
        let pass = SourceGeometryPass {
            center: [0.0, 0.0, 0.0],
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 4.0,
            original_face_index: 0,
        };
        let mut a_mesh = mesh.snapshot();
        let mut b_mesh = received_mesh.snapshot();
        let mut a_stroke = SourceGeometryStroke::new(SourceGeometryOptions::inflate()).unwrap();
        let mut b_stroke = SourceGeometryStroke::new(SourceGeometryOptions::inflate()).unwrap();
        let (a_report, a_next) = a_stroke
            .apply(&mut a_mesh, &state, pass, 0.5, true)
            .unwrap();
        let (b_report, b_next) = b_stroke
            .apply(&mut b_mesh, &restored, pass, 0.5, true)
            .unwrap();
        assert!(a_report.changed_vertices > 0);
        assert_eq!(a_report.changed_vertices, b_report.changed_vertices);
        assert_eq!(
            serde_json::to_vec(&a_mesh).unwrap(),
            serde_json::to_vec(&b_mesh).unwrap()
        );
        let a_next = a_next.unwrap();
        let b_next = b_next.unwrap();
        assert_eq!(a_next.generation(), b_next.generation());
        assert_eq!(
            serde_json::to_vec(&a_next.ordered_tree_snapshot().unwrap()).unwrap(),
            serde_json::to_vec(&b_next.ordered_tree_snapshot().unwrap()).unwrap()
        );
        assert_eq!(a_next.normal_report(), b_next.normal_report());
        let (a_second, a_next) = a_stroke
            .apply(&mut a_mesh, &a_next, pass, 0.5, true)
            .unwrap();
        let (b_second, b_next) = b_stroke
            .apply(&mut b_mesh, &b_next, pass, 0.5, true)
            .unwrap();
        assert_eq!(a_second.changed_vertices, b_second.changed_vertices);
        assert_eq!(
            serde_json::to_vec(&a_mesh).unwrap(),
            serde_json::to_vec(&b_mesh).unwrap()
        );
        assert_eq!(a_next.unwrap().generation(), b_next.unwrap().generation());
    }

    #[test]
    fn warm_state_stale_mesh_target_and_corruption_fail_closed() {
        let mut mesh = fixture();
        let seed = SourceGeometryNormalState::seed(&mesh).unwrap();
        let packet = encode_source_worker_retained_seed(&seed, &mesh, 7).unwrap();
        assert!(decode_source_worker_retained_seed(&mesh, 8, &packet).is_err());
        let mut corrupt = packet.clone();
        corrupt[24] ^= 1;
        assert!(decode_source_worker_retained_seed(&mesh, 7, &corrupt).is_err());
        let resealed = blake3::hash(&corrupt[..PAYLOAD_BYTES]);
        corrupt[PAYLOAD_BYTES..].copy_from_slice(resealed.as_bytes());
        assert!(decode_source_worker_retained_seed(&mesh, 7, &corrupt).is_err());
        let mut changed_state = packet.clone();
        changed_state[56] ^= 1;
        let resealed = blake3::hash(&changed_state[..PAYLOAD_BYTES]);
        changed_state[PAYLOAD_BYTES..].copy_from_slice(resealed.as_bytes());
        assert!(decode_source_worker_retained_seed(&mesh, 7, &changed_state).is_err());
        mesh.positions[0].x = -0.0;
        assert!(decode_source_worker_retained_seed(&mesh, 7, &packet).is_err());
        assert!(encode_source_worker_retained_seed(&seed, &mesh, 7).is_err());
        let mut topology = fixture();
        topology.surface = None;
        topology.triangles[0] = [0, 2, 1];
        topology.recompute_normals();
        assert!(decode_source_worker_retained_seed(&topology, 7, &packet).is_err());
        let mut mesh = fixture();
        let mut stroke = SourceGeometryStroke::new(SourceGeometryOptions::inflate()).unwrap();
        let pass = SourceGeometryPass {
            center: [0.0, 0.0, 0.0],
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 4.0,
            original_face_index: 0,
        };
        let (_, warm) = stroke.apply(&mut mesh, &seed, pass, 0.5, true).unwrap();
        assert!(encode_source_worker_retained_seed(&warm.unwrap(), &mesh, 7).is_err());
    }
}
