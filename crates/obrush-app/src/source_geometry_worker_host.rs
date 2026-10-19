//! One-use ordinary-studio witness for a cold Source Worker result.
//! The Worker cannot mutate the public document, renderer or history directly.

use obrush_core::{
    Document, Mesh, SourceGeometryOptions, SourceWorkerColdCapture, SourceWorkerPreviewRow,
    SourceWorkerSparsePreviewOverlay,
};
use std::collections::HashMap;

struct Pending {
    epoch: u64,
    revision: u64,
    capture: SourceWorkerColdCapture,
    preview: SourceWorkerSparsePreviewOverlay,
    preview_packets: u64,
}

#[derive(Default)]
pub struct ColdSourceWorkerBridge {
    next_epoch: u64,
    pending: Option<Pending>,
}

impl ColdSourceWorkerBridge {
    pub fn take_seed_packet(&mut self, epoch: u64) -> Result<Vec<u8>, String> {
        let pending = self
            .pending
            .as_mut()
            .ok_or("No Source worker gesture is pending")?;
        if pending.epoch != epoch {
            return Err("Source worker seed has a stale epoch".into());
        }
        Ok(pending.capture.take_retained_packet())
    }

    pub fn begin(
        &mut self,
        document: &Document,
        options: SourceGeometryOptions,
        revision: u64,
    ) -> Result<(u64, String, String), String> {
        if self.pending.is_some() {
            return Err("A Source worker gesture is already pending".into());
        }
        let epoch = self
            .next_epoch
            .checked_add(1)
            .ok_or("Source worker epochs exhausted")?;
        let capture = SourceWorkerColdCapture::capture(document, options, epoch, revision)?;
        let mesh = document
            .active_mesh()
            .ok_or("Source Worker has no active preview mesh")?;
        let preview = SourceWorkerSparsePreviewOverlay::new(
            mesh.positions.len(),
            capture.active_subtool_id(),
            epoch,
        )?;
        let mut project_bytes = Vec::new();
        project_bytes
            .try_reserve_exact(capture.project_bytes().len())
            .map_err(|_| "Cannot duplicate bounded Source worker project")?;
        project_bytes.extend_from_slice(capture.project_bytes());
        let project =
            String::from_utf8(project_bytes).map_err(|_| "Source worker project is not UTF-8")?;
        let options = serde_json::to_string(&options).map_err(|error| error.to_string())?;
        self.next_epoch = epoch;
        self.pending = Some(Pending {
            epoch,
            revision,
            capture,
            preview,
            preview_packets: 0,
        });
        Ok((epoch, project, options))
    }

    pub fn commit(
        &mut self,
        document: &mut Document,
        epoch: u64,
        revision: u64,
        delta: &[u8],
        retained: &[u8],
    ) -> Result<bool, String> {
        let pending = self
            .pending
            .take()
            .ok_or("No Source worker gesture is pending")?;
        if pending.epoch != epoch || pending.revision != revision {
            return Err("Source worker reply has stale epoch or revision".into());
        }
        let final_mesh = pending
            .capture
            .decode_candidate(document, epoch, revision, delta)?;
        if !final_matches_preview(
            document
                .active_mesh()
                .ok_or("Source Worker lost its baseline mesh")?,
            final_mesh.as_ref(),
            &pending.preview,
        )? {
            return Err("Source worker final geometry differs from checked sparse previews".into());
        }
        pending
            .capture
            .commit_candidate(document, epoch, revision, delta, retained)
    }

    pub fn preview(
        &mut self,
        document: &Document,
        epoch: u64,
        revision: u64,
        delta: &[u8],
    ) -> Result<Vec<SourceWorkerPreviewRow>, String> {
        let pending = self
            .pending
            .as_mut()
            .ok_or("No Source worker gesture is pending")?;
        pending
            .capture
            .verify_live_owner(document, epoch, revision)?;
        let baseline = document
            .active_mesh()
            .ok_or("Source worker preview has no active mesh")?;
        let rows = pending
            .preview
            .apply(&baseline.positions, &baseline.normals, delta)?;
        pending.preview_packets = pending
            .preview_packets
            .checked_add(1)
            .ok_or("Source worker preview sequence exhausted")?;
        Ok(rows)
    }

    /// The older debug-only cross-realm diagnostic has no live V2 previews.
    /// Its final V1 packet still receives the full scene/mesh/retained checks;
    /// never use this path for an ordinary held-pointer gesture.
    #[cfg(debug_assertions)]
    pub fn commit_diagnostic_without_preview(
        &mut self,
        document: &mut Document,
        epoch: u64,
        revision: u64,
        delta: &[u8],
        retained: &[u8],
    ) -> Result<bool, String> {
        let pending = self
            .pending
            .take()
            .ok_or("No Source worker gesture is pending")?;
        if pending.epoch != epoch || pending.revision != revision || pending.preview_packets != 0 {
            return Err("Source worker diagnostic has a stale or previewed owner".into());
        }
        pending
            .capture
            .commit_candidate(document, epoch, revision, delta, retained)
    }

    pub fn cancel(&mut self, epoch: u64) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.epoch == epoch)
        {
            self.pending = None;
            true
        } else {
            false
        }
    }
}

fn final_matches_preview(
    baseline: &Mesh,
    final_mesh: Option<&Mesh>,
    preview: &SourceWorkerSparsePreviewOverlay,
) -> Result<bool, String> {
    let mut changed = HashMap::new();
    changed
        .try_reserve(preview.changed_row_count())
        .map_err(|_| "Cannot reserve final Source Worker preview comparison")?;
    for (index, point, normal) in preview.iter_changed() {
        changed.insert(index as usize, (point, normal));
    }
    let final_mesh = final_mesh.unwrap_or(baseline);
    if baseline.positions.len() != final_mesh.positions.len()
        || baseline.normals.len() != final_mesh.normals.len()
    {
        return Ok(false);
    }
    for index in 0..baseline.positions.len() {
        let (point, normal) = changed.get(&index).copied().unwrap_or_else(|| {
            (
                baseline.positions[index].to_array(),
                baseline.normals[index].to_array(),
            )
        });
        if point.map(f32::to_bits) != final_mesh.positions[index].to_array().map(f32::to_bits)
            || normal.map(f32::to_bits) != final_mesh.normals[index].to_array().map(f32::to_bits)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use obrush_core::{
        MaskConvention, SourceGeometryAlphaTracks, SourceGeometryControls, SourceGeometryPass,
        SourceWorkerTransaction, encode_source_worker_mesh_delta,
        encode_source_worker_retained_warm, encode_source_worker_sparse_preview_rows,
    };

    #[test]
    fn preview_is_not_public_and_final_must_match_last_checked_preview() {
        let mut mesh = Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nf 1 2 3\n")
            .unwrap()
            .mesh;
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let mut main = Document::new(mesh);
        let original = main.to_json().unwrap();
        let mut bridge = ColdSourceWorkerBridge::default();
        let (epoch, _, _) = bridge
            .begin(&main, SourceGeometryOptions::inflate(), 7)
            .unwrap();
        let worker = Document::from_json(&original).unwrap();
        let mut tx =
            SourceWorkerTransaction::begin(worker, SourceGeometryOptions::inflate(), epoch)
                .unwrap();
        tx.apply_controlled(
            epoch,
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
        let before = main.active_mesh().unwrap();
        let after = tx.document().active_mesh().unwrap();
        let rows: Vec<_> = (0..before.positions.len())
            .filter_map(|index| {
                let bp = before.positions[index].to_array();
                let bn = before.normals[index].to_array();
                let ap = after.positions[index].to_array();
                let an = after.normals[index].to_array();
                if bp.map(f32::to_bits) == ap.map(f32::to_bits)
                    && bn.map(f32::to_bits) == an.map(f32::to_bits)
                {
                    return None;
                }
                Some(SourceWorkerPreviewRow {
                    index: index as u32,
                    before_position: bp,
                    before_normal: bn,
                    after_position: ap,
                    after_normal: an,
                })
            })
            .collect();
        let preview_delta =
            encode_source_worker_sparse_preview_rows(epoch, 1, 1, before.positions.len(), &rows)
                .unwrap();
        let preview = bridge.preview(&main, epoch, 7, &preview_delta).unwrap();
        assert_eq!(main.to_json().unwrap(), original);
        assert_eq!(main.history_depths(), (0, 0));
        let done = tx.finish(epoch, 2).unwrap();
        let state = done.source_geometry_state(1).unwrap();
        let retained =
            encode_source_worker_retained_warm(&state, done.active_mesh().unwrap(), 1).unwrap();
        let final_delta = encode_source_worker_mesh_delta(
            main.active_mesh().unwrap(),
            done.active_mesh().unwrap(),
            1,
        )
        .unwrap();
        assert_eq!(preview.len(), rows.len());
        for (actual, expected) in preview.iter().zip(&rows) {
            assert_eq!(actual.index, expected.index);
            assert_eq!(
                actual.after_position.map(f32::to_bits),
                expected.after_position.map(f32::to_bits)
            );
            assert_eq!(
                actual.after_normal.map(f32::to_bits),
                expected.after_normal.map(f32::to_bits)
            );
        }
        assert!(
            bridge
                .commit(&mut main, epoch, 7, &final_delta, &retained)
                .unwrap()
        );
        assert_eq!(main.history_depths(), (1, 0));
        assert!(main.source_geometry_state(1).is_some());

        // The preexisting debug transport intentionally has no per-dab V2
        // preview; keep its fully checked final V1 publication working.
        let mut diagnostic = Document::from_json(&original).unwrap();
        let mut diagnostic_bridge = ColdSourceWorkerBridge::default();
        let (diagnostic_epoch, _, _) = diagnostic_bridge
            .begin(&diagnostic, SourceGeometryOptions::inflate(), 7)
            .unwrap();
        assert_eq!(diagnostic_epoch, epoch);
        assert!(
            diagnostic_bridge
                .commit_diagnostic_without_preview(
                    &mut diagnostic,
                    diagnostic_epoch,
                    7,
                    &final_delta,
                    &retained,
                )
                .unwrap()
        );
        assert_eq!(diagnostic.to_json().unwrap(), main.to_json().unwrap());
        assert_eq!(diagnostic.history_depths(), (1, 0));
    }

    #[test]
    fn sparse_preview_cannot_commit_a_different_final_mesh() {
        let mut mesh = Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nf 1 2 3\n")
            .unwrap()
            .mesh;
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        let mut document = Document::new(mesh);
        let original = document.to_json().unwrap();
        let mut bridge = ColdSourceWorkerBridge::default();
        let (epoch, _, _) = bridge
            .begin(&document, SourceGeometryOptions::inflate(), 7)
            .unwrap();
        let baseline = document.active_mesh().unwrap();
        let old = baseline.positions[0].to_array();
        let mut new = old;
        new[2] += 0.25;
        let row = SourceWorkerPreviewRow {
            index: 0,
            before_position: old,
            before_normal: baseline.normals[0].to_array(),
            after_position: new,
            after_normal: baseline.normals[0].to_array(),
        };
        let packet =
            encode_source_worker_sparse_preview_rows(epoch, 1, 1, baseline.positions.len(), &[row])
                .unwrap();
        assert_eq!(
            bridge.preview(&document, epoch, 7, &packet).unwrap().len(),
            1
        );
        let final_delta = encode_source_worker_mesh_delta(baseline, baseline, 1).unwrap();
        assert!(
            bridge
                .commit(&mut document, epoch, 7, &final_delta, &[])
                .is_err()
        );
        assert_eq!(document.to_json().unwrap(), original);
        assert_eq!(document.history_depths(), (0, 0));
    }
}
