//! One-use ordinary-studio witness for a cold Source Worker result.
//! The Worker cannot mutate the public document, renderer or history directly.

use obrush_core::{
    Document, Mesh, SourceGeometryOptions, SourceWorkerColdCapture, decode_source_worker_mesh_delta,
};
use std::sync::Arc;

struct Pending {
    epoch: u64,
    revision: u64,
    capture: SourceWorkerColdCapture,
    preview: Option<Arc<Mesh>>,
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
            preview: None,
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
        match (&pending.preview, final_mesh.as_ref()) {
            (None, None) => {}
            (Some(preview), Some(final_mesh)) if same_geometry_words(preview, final_mesh) => {}
            _ => {
                return Err(
                    "Source worker final geometry differs from the last checked preview".into(),
                );
            }
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
    ) -> Result<Option<Arc<Mesh>>, String> {
        let pending = self
            .pending
            .as_mut()
            .ok_or("No Source worker gesture is pending")?;
        pending.capture.verify_scene(document, epoch, revision)?;
        let before = pending
            .preview
            .as_deref()
            .or_else(|| document.active_mesh())
            .ok_or("Source worker preview has no active mesh")?;
        let candidate =
            decode_source_worker_mesh_delta(before, pending.capture.active_subtool_id(), delta)?;
        if let Some(candidate) = candidate {
            pending.preview = Some(Arc::new(candidate));
        }
        Ok(pending.preview.clone())
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

fn same_geometry_words(a: &Mesh, b: &Mesh) -> bool {
    a.positions.len() == b.positions.len()
        && a.normals.len() == b.normals.len()
        && a.positions
            .iter()
            .zip(&b.positions)
            .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits))
        && a.normals
            .iter()
            .zip(&b.normals)
            .all(|(a, b)| a.to_array().map(f32::to_bits) == b.to_array().map(f32::to_bits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use obrush_core::{
        MaskConvention, SourceGeometryAlphaTracks, SourceGeometryControls, SourceGeometryPass,
        SourceWorkerTransaction, encode_source_worker_mesh_delta,
        encode_source_worker_retained_warm,
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
        let preview_delta = encode_source_worker_mesh_delta(
            main.active_mesh().unwrap(),
            tx.document().active_mesh().unwrap(),
            1,
        )
        .unwrap();
        let preview = bridge
            .preview(&main, epoch, 7, &preview_delta)
            .unwrap()
            .unwrap();
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
        assert!(same_geometry_words(&preview, done.active_mesh().unwrap()));
        assert!(
            bridge
                .commit(&mut main, epoch, 7, &final_delta, &retained)
                .unwrap()
        );
        assert_eq!(main.history_depths(), (1, 0));
        assert!(main.source_geometry_state(1).is_some());
    }
}
