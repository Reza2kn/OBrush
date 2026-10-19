//! Versioned, bounded cross-realm transport for a warmed Source producer.
//! Neither a Document nor undo/redo ownership crosses this boundary.
use crate::source_worker_retained_seed::seed_mesh_digest;
use crate::source_worker_warm_io::{Reader, Writer};
use crate::{Mesh, SourceGeometryNormalState};

const MAGIC: &[u8; 8] = b"OBRSNWV1";
const VERSION: u32 = 1;
const MESH_DIGEST_AT: usize = 8 + 4 + 4;
const STATE_DIGEST_AT: usize = MESH_DIGEST_AT + 32;
const PAYLOAD_LEN_AT: usize = STATE_DIGEST_AT + 32;
const PAYLOAD_AT: usize = PAYLOAD_LEN_AT + 4;
const CHECKSUM_BYTES: usize = 32;
/// Separate from the 256 MiB retained-state cap: transfer buffers must have
/// an explicit bounded admission before clone/postMessage can allocate.
pub const MAX_SOURCE_WORKER_WARM_PACKET_BYTES: usize = 256 * 1024 * 1024;

/// Serialize exact selected-refresh normals and the historical ordered tree.
/// The mesh is the public geometry witness; ID, mesh digest, state digest and
/// packet checksum bind this private packet to one checked producer.
pub fn encode_source_worker_retained_warm(
    state: &SourceGeometryNormalState,
    mesh: &Mesh,
    subtool_id: u32,
) -> Result<Vec<u8>, String> {
    if subtool_id == 0 {
        return Err("Source warm target ID must be nonzero".into());
    }
    let mut out = Writer::new(MAX_SOURCE_WORKER_WARM_PACKET_BYTES - CHECKSUM_BYTES);
    out.raw(MAGIC)?;
    out.u32(VERSION)?;
    out.u32(subtool_id)?;
    out.raw(&seed_mesh_digest(mesh))?;
    out.raw(&state.seed_fingerprint())?;
    // Fill this field after the bounded payload has been emitted.
    out.u32(0)?;
    state.encode_warm(mesh, &mut out)?;
    let mut packet = out.finish();
    let payload_len = packet
        .len()
        .checked_sub(PAYLOAD_AT)
        .ok_or("Source warm payload underflow")?;
    packet[PAYLOAD_LEN_AT..PAYLOAD_AT].copy_from_slice(
        &u32::try_from(payload_len)
            .map_err(|_| "Source warm payload exceeds u32")?
            .to_le_bytes(),
    );
    let checksum = blake3::hash(&packet);
    packet
        .try_reserve_exact(CHECKSUM_BYTES)
        .map_err(|_| "Cannot reserve Source warm checksum")?;
    packet.extend_from_slice(checksum.as_bytes());
    Ok(packet)
}

/// Decode into an uninstalled producer. A caller may publish it only after
/// checking the owning Document revision and preserving its own history.
pub fn decode_source_worker_retained_warm(
    mesh: &Mesh,
    subtool_id: u32,
    packet: &[u8],
) -> Result<SourceGeometryNormalState, String> {
    if subtool_id == 0
        || packet.len() < PAYLOAD_AT + CHECKSUM_BYTES
        || packet.len() > MAX_SOURCE_WORKER_WARM_PACKET_BYTES
        || &packet[..8] != MAGIC
        || u32::from_le_bytes(packet[8..12].try_into().unwrap()) != VERSION
        || u32::from_le_bytes(packet[12..16].try_into().unwrap()) != subtool_id
    {
        return Err("Invalid Source warm packet header, target or size".into());
    }
    let payload_len =
        u32::from_le_bytes(packet[PAYLOAD_LEN_AT..PAYLOAD_AT].try_into().unwrap()) as usize;
    if payload_len.checked_add(PAYLOAD_AT + CHECKSUM_BYTES) != Some(packet.len())
        || blake3::hash(&packet[..packet.len() - CHECKSUM_BYTES]).as_bytes()
            != &packet[packet.len() - CHECKSUM_BYTES..]
    {
        return Err("Invalid Source warm packet length or checksum".into());
    }
    mesh.validate()?;
    if seed_mesh_digest(mesh).as_slice() != &packet[MESH_DIGEST_AT..STATE_DIGEST_AT] {
        return Err("Source warm packet has a stale mesh baseline".into());
    }
    let mut input = Reader::new(&packet[PAYLOAD_AT..packet.len() - CHECKSUM_BYTES]);
    let state = SourceGeometryNormalState::decode_warm(mesh, &mut input)?;
    if !input.done()
        || state.seed_fingerprint().as_slice() != &packet[STATE_DIGEST_AT..PAYLOAD_LEN_AT]
    {
        return Err("Source warm packet reconstructed different state words".into());
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Document, MaskConvention, SourceGeometryOptions, SourceGeometryPass, SourceGeometryStroke,
    };

    // Authored independently of the Source primitive generator and the seed
    // witness. More than 100 original faces force a branched ordered tree.
    fn grid() -> Mesh {
        let mut obj = String::new();
        for y in 0..13 {
            for x in 0..13 {
                obj.push_str(&format!(
                    "v {} {} 0\n",
                    (x as f32 - 6.0) * 0.2,
                    (y as f32 - 6.0) * 0.2
                ));
            }
        }
        for y in 0..12 {
            for x in 0..12 {
                let a = 1 + y * 13 + x;
                obj.push_str(&format!("f {a} {} {} {}\n", a + 1, a + 14, a + 13));
            }
        }
        let mut mesh = Mesh::import_obj(obj.as_bytes()).unwrap().mesh;
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        mesh
    }
    fn pass(center: [f64; 3]) -> SourceGeometryPass {
        SourceGeometryPass {
            center,
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 0.45,
            original_face_index: 65,
        }
    }
    fn reseal(packet: &mut [u8]) {
        let len = packet.len() - CHECKSUM_BYTES;
        let digest = blake3::hash(&packet[..len]);
        packet[len..].copy_from_slice(digest.as_bytes());
    }
    fn tree_offset(mesh: &Mesh) -> usize {
        PAYLOAD_AT
            + 8
            + 1
            + 12
            + 4
            + 12 * mesh.positions.len()
            + 4
            + 12 * mesh.surface.as_ref().unwrap().faces.len()
            + 4
            + 12 * mesh.positions.len()
            + 8 * 8
    }
    fn semantic_normal_report(state: &SourceGeometryNormalState) -> serde_json::Value {
        let mut report = serde_json::to_value(state.normal_report()).unwrap();
        report.as_object_mut().unwrap().remove("storage");
        report
    }

    #[test]
    fn warm_cross_realm_second_and_third_dab_are_bit_exact() {
        let mut mesh = grid();
        let seed = SourceGeometryNormalState::seed(&mesh).unwrap();
        let options = SourceGeometryOptions::inflate();
        let mut first = SourceGeometryStroke::new(options).unwrap();
        let (first_report, warm) = first
            .apply(&mut mesh, &seed, pass([0.0, 0.0, 0.0]), 0.8, true)
            .unwrap();
        assert!(first_report.changed_vertices > 0);
        let warm = warm.unwrap();
        assert!(warm.generation() > 0);
        assert!(!warm.matches_seed_reconstruction(&mesh).unwrap());
        let packet = encode_source_worker_retained_warm(&warm, &mesh, 7).unwrap();
        eprintln!(
            "warm-fixture mesh={} packet={} packet_bytes={} first_changed={}",
            blake3::hash(&serde_json::to_vec(&mesh).unwrap()),
            blake3::hash(&packet),
            packet.len(),
            first_report.changed_vertices
        );
        assert!(packet.len() < MAX_SOURCE_WORKER_WARM_PACKET_BYTES);
        let project = Document::new(mesh.snapshot()).to_json().unwrap();
        let receiver = Document::from_json(&project).unwrap();
        let mut received_mesh = receiver.active_mesh().unwrap().snapshot();
        let received = decode_source_worker_retained_warm(&received_mesh, 7, &packet).unwrap();
        assert_eq!(warm.seed_fingerprint(), received.seed_fingerprint());
        assert_eq!(
            semantic_normal_report(&warm),
            semantic_normal_report(&received)
        );
        let mut baseline_stroke = SourceGeometryStroke::new(options).unwrap();
        let mut receiver_stroke = SourceGeometryStroke::new(options).unwrap();
        let mut baseline_state = warm;
        let mut receiver_state = received;
        for center in [[0.2, 0.0, 0.0], [-0.2, 0.2, 0.0]] {
            let (a_report, a_next) = baseline_stroke
                .apply(&mut mesh, &baseline_state, pass(center), 0.65, true)
                .unwrap();
            let (b_report, b_next) = receiver_stroke
                .apply(
                    &mut received_mesh,
                    &receiver_state,
                    pass(center),
                    0.65,
                    true,
                )
                .unwrap();
            assert!(a_report.changed_vertices > 0);
            assert_eq!(a_report.changed_vertices, b_report.changed_vertices);
            eprintln!("warm-fixture later_changed={}", a_report.changed_vertices);
            assert_eq!(
                serde_json::to_vec(&mesh).unwrap(),
                serde_json::to_vec(&received_mesh).unwrap()
            );
            baseline_state = a_next.unwrap();
            receiver_state = b_next.unwrap();
            assert_eq!(
                baseline_state.seed_fingerprint(),
                receiver_state.seed_fingerprint()
            );
            assert_eq!(
                semantic_normal_report(&baseline_state),
                semantic_normal_report(&receiver_state)
            );
        }
    }

    #[test]
    fn stale_target_checksum_counts_and_resealed_malformed_tree_fail_closed() {
        let mut mesh = grid();
        let seed = SourceGeometryNormalState::seed(&mesh).unwrap();
        let mut stroke = SourceGeometryStroke::new(SourceGeometryOptions::inflate()).unwrap();
        let (_, warm) = stroke
            .apply(&mut mesh, &seed, pass([0.0, 0.0, 0.0]), 0.8, true)
            .unwrap();
        let packet = encode_source_worker_retained_warm(&warm.unwrap(), &mesh, 7).unwrap();
        assert!(decode_source_worker_retained_warm(&mesh, 8, &packet).is_err());
        let mut changed = mesh.snapshot();
        changed.positions[0].x = -0.0;
        assert!(decode_source_worker_retained_warm(&changed, 7, &packet).is_err());
        let mut corrupt = packet.clone();
        corrupt[PAYLOAD_AT + 4] ^= 1;
        assert!(decode_source_worker_retained_warm(&mesh, 7, &corrupt).is_err());
        let mut bad_count = packet.clone();
        let tree = tree_offset(&mesh);
        bad_count[tree + 16..tree + 20].copy_from_slice(&u32::MAX.to_le_bytes());
        reseal(&mut bad_count);
        let count_error = decode_source_worker_retained_warm(&mesh, 7, &bad_count).unwrap_err();
        assert!(count_error.contains("count exceeds bound"), "{count_error}");
        let mut bad_parent = packet.clone();
        bad_parent[tree + 20 + 1 + 8..tree + 20 + 1 + 8 + 4].copy_from_slice(&0u32.to_le_bytes());
        reseal(&mut bad_parent);
        assert!(decode_source_worker_retained_warm(&mesh, 7, &bad_parent)
            .unwrap_err()
            .contains("root"));
        let mut cycle = packet.clone();
        let root_first_child = tree + 20 + 1 + 8 + 4 + 1 + 48 + 48 + 1;
        cycle[root_first_child..root_first_child + 4].copy_from_slice(&0u32.to_le_bytes());
        reseal(&mut cycle);
        assert!(decode_source_worker_retained_warm(&mesh, 7, &cycle)
            .unwrap_err()
            .contains("child link"));
    }
}
