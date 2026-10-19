//! Transient, checked per-dab display rows for an ordinary Source Worker.
//!
//! This packet cannot publish a document or authorize history. The final
//! stroke still uses the complete checked V1 worker transaction. A host must
//! hold the baseline position/normal slices stable for the overlay's epoch.

use crate::MAX_VERTICES;
use glam::Vec3;
use std::collections::HashMap;

const MAGIC: &[u8; 8] = b"OBRSPRV1";
const VERSION: u32 = 1;
const HEADER_BYTES: usize = 40;
const ROW_BYTES: usize = 52;
const CHECKSUM_BYTES: usize = 32;
// Match Mesh's public coordinate bound without a whole-mesh validation walk.
const MAX_PREVIEW_COORDINATE: f32 = 1_000_000.0;
pub const MAX_SOURCE_WORKER_PREVIEW_BYTES: usize = 64 * 1024 * 1024;
/// At most the project's one-million vertex ceiling can remain in the sparse
/// overlay, even across an unbounded number of small packets. Each retained
/// entry stores one u32 index plus six u32 words; HashMap bucket overhead is
/// implementation-dependent, so allocation is also fallibly reserved.
pub const MAX_SOURCE_WORKER_PREVIEW_OVERLAY_ROWS: usize = MAX_VERTICES;

/// The old and new display words of one vertex. Both normals and positions
/// are transported because a neighboring face can change an unmoved normal.
#[derive(Clone, Copy, Debug)]
pub struct SourceWorkerPreviewRow {
    pub index: u32,
    pub before_position: [f32; 3],
    pub before_normal: [f32; 3],
    pub after_position: [f32; 3],
    pub after_normal: [f32; 3],
}

impl SourceWorkerPreviewRow {
    fn old_words(self) -> [u32; 6] {
        let p = self.before_position.map(f32::to_bits);
        let n = self.before_normal.map(f32::to_bits);
        [p[0], p[1], p[2], n[0], n[1], n[2]]
    }

    fn new_words(self) -> [u32; 6] {
        let p = self.after_position.map(f32::to_bits);
        let n = self.after_normal.map(f32::to_bits);
        [p[0], p[1], p[2], n[0], n[1], n[2]]
    }
}

fn words_from_baseline(positions: &[Vec3], normals: &[Vec3], index: usize) -> [u32; 6] {
    let p = positions[index].to_array().map(f32::to_bits);
    let n = normals[index].to_array().map(f32::to_bits);
    [p[0], p[1], p[2], n[0], n[1], n[2]]
}

fn valid_geometry_words(words: &[u32; 6]) -> bool {
    let point = Vec3::new(
        f32::from_bits(words[0]),
        f32::from_bits(words[1]),
        f32::from_bits(words[2]),
    );
    let normal = Vec3::new(
        f32::from_bits(words[3]),
        f32::from_bits(words[4]),
        f32::from_bits(words[5]),
    );
    point.is_finite()
        && point.abs().max_element() <= MAX_PREVIEW_COORDINATE
        && normal.is_finite()
        && normal.length_squared().is_finite()
}

/// Encode preselected changed rows in ascending unique order. This function
/// examines only supplied rows; the producer is responsible for deriving a
/// certified affected-row set from its private geometry computation.
pub fn encode_source_worker_sparse_preview_rows(
    epoch: u64,
    sequence: u64,
    active_subtool_id: u32,
    vertex_count: usize,
    rows: &[SourceWorkerPreviewRow],
) -> Result<Vec<u8>, String> {
    if epoch == 0
        || sequence == 0
        || active_subtool_id == 0
        || vertex_count == 0
        || vertex_count > MAX_VERTICES
        || rows.len() > vertex_count
    {
        return Err("Source preview identity or row count is invalid".into());
    }
    let total = rows
        .len()
        .checked_mul(ROW_BYTES)
        .and_then(|size| size.checked_add(HEADER_BYTES + CHECKSUM_BYTES))
        .ok_or("Source preview packet size overflow")?;
    if total > MAX_SOURCE_WORKER_PREVIEW_BYTES {
        return Err("Source preview packet exceeds byte limit".into());
    }
    let mut packet = Vec::new();
    packet
        .try_reserve_exact(total)
        .map_err(|_| "Cannot reserve Source preview packet")?;
    packet.extend_from_slice(MAGIC);
    packet.extend_from_slice(&VERSION.to_le_bytes());
    packet.extend_from_slice(&epoch.to_le_bytes());
    packet.extend_from_slice(&sequence.to_le_bytes());
    packet.extend_from_slice(&active_subtool_id.to_le_bytes());
    packet.extend_from_slice(&(vertex_count as u32).to_le_bytes());
    packet.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    let mut previous = None;
    for row in rows {
        let old = row.old_words();
        let new = row.new_words();
        if row.index as usize >= vertex_count
            || previous.is_some_and(|index| row.index <= index)
            || !valid_geometry_words(&old)
            || !valid_geometry_words(&new)
            || old == new
        {
            return Err("Source preview row is invalid, unordered or unchanged".into());
        }
        previous = Some(row.index);
        packet.extend_from_slice(&row.index.to_le_bytes());
        for word in old.into_iter().chain(new) {
            packet.extend_from_slice(&word.to_le_bytes());
        }
    }
    let checksum = blake3::hash(&packet);
    packet.extend_from_slice(checksum.as_bytes());
    Ok(packet)
}

/// Sparse display state layered on one immutable baseline. A rejected packet
/// poisons the overlay; its caller must cancel the display and start anew.
pub struct SourceWorkerSparsePreviewOverlay {
    epoch: u64,
    next_sequence: u64,
    active_subtool_id: u32,
    vertex_count: usize,
    changed: HashMap<u32, [u32; 6]>,
    closed: bool,
}

impl SourceWorkerSparsePreviewOverlay {
    pub fn new(vertex_count: usize, active_subtool_id: u32, epoch: u64) -> Result<Self, String> {
        if vertex_count == 0 || vertex_count > MAX_VERTICES || active_subtool_id == 0 || epoch == 0
        {
            return Err("Source preview baseline identity is invalid".into());
        }
        Ok(Self {
            epoch,
            next_sequence: 1,
            active_subtool_id,
            vertex_count,
            changed: HashMap::new(),
            closed: false,
        })
    }

    /// Validate an entire packet before any overlay mutation. The baseline
    /// slices must be the immutable source mesh captured when `new` was called.
    /// The returned sorted rows can patch a renderer without a full mesh copy.
    pub fn apply(
        &mut self,
        baseline_positions: &[Vec3],
        baseline_normals: &[Vec3],
        packet: &[u8],
    ) -> Result<Vec<SourceWorkerPreviewRow>, String> {
        if self.closed {
            return Err("Source preview is closed".into());
        }
        let result = self.decode_and_check(baseline_positions, baseline_normals, packet);
        match result {
            Ok(rows) => {
                self.next_sequence = self.next_sequence.checked_add(1).unwrap_or(0);
                for row in &rows {
                    let words = row.new_words();
                    let baseline = words_from_baseline(
                        baseline_positions,
                        baseline_normals,
                        row.index as usize,
                    );
                    if words == baseline {
                        self.changed.remove(&row.index);
                    } else {
                        self.changed.insert(row.index, words);
                    }
                }
                if self.next_sequence == 0 {
                    self.closed = true;
                }
                Ok(rows)
            }
            Err(error) => {
                self.cancel();
                Err(error)
            }
        }
    }

    fn decode_and_check(
        &mut self,
        positions: &[Vec3],
        normals: &[Vec3],
        packet: &[u8],
    ) -> Result<Vec<SourceWorkerPreviewRow>, String> {
        if positions.len() != self.vertex_count || normals.len() != self.vertex_count {
            return Err("Source preview baseline shape changed".into());
        }
        if packet.len() < HEADER_BYTES + CHECKSUM_BYTES
            || packet.len() > MAX_SOURCE_WORKER_PREVIEW_BYTES
            || &packet[..8] != MAGIC
            || u32::from_le_bytes(packet[8..12].try_into().unwrap()) != VERSION
        {
            return Err("Source preview packet header or size is invalid".into());
        }
        let epoch = u64::from_le_bytes(packet[12..20].try_into().unwrap());
        let sequence = u64::from_le_bytes(packet[20..28].try_into().unwrap());
        let target = u32::from_le_bytes(packet[28..32].try_into().unwrap());
        let count = u32::from_le_bytes(packet[32..36].try_into().unwrap()) as usize;
        let row_count = u32::from_le_bytes(packet[36..40].try_into().unwrap()) as usize;
        if epoch != self.epoch
            || sequence != self.next_sequence
            || target != self.active_subtool_id
            || count != self.vertex_count
            || row_count > count
            || row_count
                .checked_mul(ROW_BYTES)
                .and_then(|size| size.checked_add(HEADER_BYTES + CHECKSUM_BYTES))
                != Some(packet.len())
        {
            return Err("Source preview epoch, sequence, target or shape is invalid".into());
        }
        let checksum_at = packet.len() - CHECKSUM_BYTES;
        if blake3::hash(&packet[..checksum_at]).as_bytes() != &packet[checksum_at..] {
            return Err("Source preview checksum mismatch".into());
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(row_count)
            .map_err(|_| "Cannot reserve Source preview rows")?;
        let mut previous = None;
        let mut additional = 0_usize;
        for bytes in packet[HEADER_BYTES..checksum_at].chunks_exact(ROW_BYTES) {
            let index = u32::from_le_bytes(bytes[..4].try_into().unwrap());
            if index as usize >= count || previous.is_some_and(|prev| index <= prev) {
                return Err("Source preview rows are out of range, duplicate or unordered".into());
            }
            previous = Some(index);
            let mut words = [0_u32; 12];
            for (out, source) in words.iter_mut().zip(bytes[4..].chunks_exact(4)) {
                *out = u32::from_le_bytes(source.try_into().unwrap());
            }
            let old: [u32; 6] = words[..6].try_into().unwrap();
            let new: [u32; 6] = words[6..].try_into().unwrap();
            if !valid_geometry_words(&old) || !valid_geometry_words(&new) || old == new {
                return Err("Source preview row geometry is invalid or unchanged".into());
            }
            let baseline = words_from_baseline(positions, normals, index as usize);
            let current = self.changed.get(&index).copied().unwrap_or(baseline);
            if current != old {
                return Err("Source preview previous row words differ".into());
            }
            if !self.changed.contains_key(&index) && new != baseline {
                additional += 1;
            }
            rows.push(SourceWorkerPreviewRow {
                index,
                before_position: [old[0], old[1], old[2]].map(f32::from_bits),
                before_normal: [old[3], old[4], old[5]].map(f32::from_bits),
                after_position: [new[0], new[1], new[2]].map(f32::from_bits),
                after_normal: [new[3], new[4], new[5]].map(f32::from_bits),
            });
        }
        if self
            .changed
            .len()
            .checked_add(additional)
            .is_none_or(|rows| rows > MAX_SOURCE_WORKER_PREVIEW_OVERLAY_ROWS)
        {
            return Err("Source preview overlay exceeds row limit".into());
        }
        self.changed
            .try_reserve(additional)
            .map_err(|_| "Cannot reserve Source preview overlay")?;
        Ok(rows)
    }

    /// Current changed rows relative to the stable baseline. Iteration order
    /// is unspecified; per-packet `apply` rows remain sorted.
    pub fn iter_changed(&self) -> impl Iterator<Item = (u32, [f32; 3], [f32; 3])> + '_ {
        self.changed.iter().map(|(&index, &words)| {
            (
                index,
                [words[0], words[1], words[2]].map(f32::from_bits),
                [words[3], words[4], words[5]].map(f32::from_bits),
            )
        })
    }

    pub fn cancel(&mut self) {
        self.changed.clear();
        self.closed = true;
    }

    pub fn changed_row_count(&self) -> usize {
        self.changed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> (Vec<Vec3>, Vec<Vec3>) {
        (
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![Vec3::Z, Vec3::Z, Vec3::Z],
        )
    }

    fn row(index: u32, before: f32, after: f32) -> SourceWorkerPreviewRow {
        SourceWorkerPreviewRow {
            index,
            before_position: [before, 0.0, 0.0],
            before_normal: [0.0, 0.0, 1.0],
            after_position: [after, 0.0, 0.0],
            after_normal: [0.0, 0.0, 1.0],
        }
    }

    fn packet(sequence: u64, rows: &[SourceWorkerPreviewRow]) -> Vec<u8> {
        encode_source_worker_sparse_preview_rows(9, sequence, 7, 3, rows).unwrap()
    }

    fn reseal(packet: &mut [u8]) {
        let end = packet.len() - CHECKSUM_BYTES;
        let hash = blake3::hash(&packet[..end]);
        packet[end..].copy_from_slice(hash.as_bytes());
    }

    #[test]
    fn exact_multi_dab_overlay_reverts_to_baseline_without_full_mesh() {
        let (positions, normals) = baseline();
        let mut overlay = SourceWorkerSparsePreviewOverlay::new(3, 7, 9).unwrap();
        let first = packet(1, &[row(0, 0.0, 0.25), row(1, 1.0, 1.5)]);
        let applied = overlay.apply(&positions, &normals, &first).unwrap();
        assert_eq!(applied.len(), 2);
        assert_eq!(overlay.changed_row_count(), 2);
        let second = packet(2, &[row(0, 0.25, 0.0), row(1, 1.5, 2.0)]);
        overlay.apply(&positions, &normals, &second).unwrap();
        assert_eq!(overlay.changed_row_count(), 1);
        let changed: Vec<_> = overlay.iter_changed().collect();
        assert_eq!(changed, vec![(1, [2.0, 0.0, 0.0], [0.0, 0.0, 1.0])]);
        overlay.cancel();
        assert_eq!(overlay.changed_row_count(), 0);
        assert!(overlay
            .apply(&positions, &normals, &packet(3, &[]))
            .is_err());
    }

    #[test]
    fn malformed_packet_is_atomic_and_closes_overlay() {
        let (positions, normals) = baseline();
        let mut overlay = SourceWorkerSparsePreviewOverlay::new(3, 7, 9).unwrap();
        overlay
            .apply(&positions, &normals, &packet(1, &[row(0, 0.0, 0.25)]))
            .unwrap();
        let mut bad = packet(2, &[row(0, 0.25, 0.5), row(1, 1.0, 1.5)]);
        // Second row has a stale previous word; the first valid row must not land.
        let second_old_x = HEADER_BYTES + ROW_BYTES + 4;
        bad[second_old_x..second_old_x + 4].copy_from_slice(&0.0_f32.to_bits().to_le_bytes());
        reseal(&mut bad);
        assert!(overlay.apply(&positions, &normals, &bad).is_err());
        assert_eq!(overlay.changed_row_count(), 0);
        assert!(overlay
            .apply(&positions, &normals, &packet(2, &[]))
            .is_err());
    }

    #[test]
    fn rejects_epoch_sequence_replay_target_checksum_and_shape() {
        let (positions, normals) = baseline();
        let original = packet(1, &[row(0, 0.0, 0.25)]);
        let mut wrong = original.clone();
        wrong[12..20].copy_from_slice(&10_u64.to_le_bytes());
        reseal(&mut wrong);
        assert!(SourceWorkerSparsePreviewOverlay::new(3, 7, 9)
            .unwrap()
            .apply(&positions, &normals, &wrong)
            .is_err());
        let mut wrong = original.clone();
        wrong[28..32].copy_from_slice(&8_u32.to_le_bytes());
        reseal(&mut wrong);
        assert!(SourceWorkerSparsePreviewOverlay::new(3, 7, 9)
            .unwrap()
            .apply(&positions, &normals, &wrong)
            .is_err());
        let mut wrong = original.clone();
        wrong[HEADER_BYTES + 8] ^= 1;
        assert!(SourceWorkerSparsePreviewOverlay::new(3, 7, 9)
            .unwrap()
            .apply(&positions, &normals, &wrong)
            .is_err());
        let mut overlay = SourceWorkerSparsePreviewOverlay::new(3, 7, 9).unwrap();
        overlay.apply(&positions, &normals, &original).unwrap();
        assert!(overlay.apply(&positions, &normals, &original).is_err());
        assert!(SourceWorkerSparsePreviewOverlay::new(3, 7, 9)
            .unwrap()
            .apply(&positions[..2], &normals, &original)
            .is_err());
    }

    #[test]
    fn rejects_duplicate_unsorted_nonfinite_and_signed_zero_previous_words() {
        let (positions, normals) = baseline();
        assert!(encode_source_worker_sparse_preview_rows(
            9,
            1,
            7,
            3,
            &[row(1, 1.0, 2.0), row(1, 1.0, 3.0)]
        )
        .is_err());
        assert!(encode_source_worker_sparse_preview_rows(
            9,
            1,
            7,
            3,
            &[row(1, 1.0, 2.0), row(0, 0.0, 1.0)]
        )
        .is_err());
        let mut duplicate = packet(1, &[row(0, 0.0, 0.25), row(1, 1.0, 1.5)]);
        duplicate[HEADER_BYTES + ROW_BYTES..HEADER_BYTES + ROW_BYTES + 4]
            .copy_from_slice(&0_u32.to_le_bytes());
        reseal(&mut duplicate);
        assert!(SourceWorkerSparsePreviewOverlay::new(3, 7, 9)
            .unwrap()
            .apply(&positions, &normals, &duplicate)
            .is_err());
        let mut nonfinite = packet(1, &[row(0, 0.0, 0.25)]);
        nonfinite[HEADER_BYTES + 28..HEADER_BYTES + 32]
            .copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        reseal(&mut nonfinite);
        assert!(SourceWorkerSparsePreviewOverlay::new(3, 7, 9)
            .unwrap()
            .apply(&positions, &normals, &nonfinite)
            .is_err());
        assert!(
            encode_source_worker_sparse_preview_rows(9, 1, 7, 3, &[row(0, 0.0, 1_000_001.0)])
                .is_err()
        );
        // The baseline is +0.0; a claimed -0.0 predecessor is stale even
        // though floating point equality would say they match.
        let mut signed_zero = packet(1, &[row(0, 0.0, 0.25)]);
        signed_zero[HEADER_BYTES + 4..HEADER_BYTES + 8]
            .copy_from_slice(&(-0.0_f32).to_bits().to_le_bytes());
        reseal(&mut signed_zero);
        assert!(SourceWorkerSparsePreviewOverlay::new(3, 7, 9)
            .unwrap()
            .apply(&positions, &normals, &signed_zero)
            .is_err());
    }
}
