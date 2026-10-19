//! Fail-closed cross-realm seed for a *cold* Source worker experiment.
//!
//! A public project does not contain the retained Source normal/tree state or
//! live history claim. Consequently this boundary refuses any retained state
//! and only returns a validated mesh candidate; it never publishes to a
//! Document. The normal browser host still needs a retained-state wire format
//! and an atomic history/render commit before using a worker for sculpting.

use crate::{
    decode_source_worker_mesh_delta, resource_limits::MAX_PROJECT_BYTES, Document, Mesh,
    SourceGeometryOptions,
};
use std::sync::Arc;

/// One-use witness for the complete main-thread scene before any worker seed.
/// `revision` is assigned by the host and must advance on every document edit.
pub struct SourceWorkerColdCapture {
    epoch: u64,
    revision: u64,
    active_subtool_id: u32,
    level: usize,
    history_depths: (usize, usize),
    project: Vec<u8>,
    project_digest: blake3::Hash,
}

impl SourceWorkerColdCapture {
    pub fn capture(
        document: &Document,
        options: SourceGeometryOptions,
        epoch: u64,
        revision: u64,
    ) -> Result<Self, String> {
        if epoch == 0 || revision == 0 || document.source_geometry_active_claim().is_some() {
            return Err(
                "Source worker capture requires a nonzero identity and closed stroke".into(),
            );
        }
        document
            .source_geometry_blocker(options)
            .map_err(|error| error.to_string())?;
        // A public .obrush snapshot cannot reproduce this transient producer.
        // Reject *all* retained SubTool states, including an idle non-active one.
        if document.has_source_geometry_normal_state() {
            return Err("Source worker capture cannot transfer retained normal state".into());
        }
        let active_subtool_id = document
            .active_subtool_id()
            .ok_or("Source worker capture has no active SubTool")?;
        let project = document.to_json()?.into_bytes();
        if project.len() > MAX_PROJECT_BYTES {
            return Err("Source worker project exceeds the file limit".into());
        }
        Ok(Self {
            epoch,
            revision,
            active_subtool_id,
            level: document.current_level(),
            history_depths: document.history_depths(),
            project_digest: blake3::hash(&project),
            project,
        })
    }

    pub fn project_bytes(&self) -> &[u8] {
        &self.project
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Consumes the one-use witness. A successful result is still only a
    /// candidate: adopting Source normal state and undo are separate steps.
    pub fn decode_candidate(
        self,
        document: &Document,
        epoch: u64,
        revision: u64,
        delta: &[u8],
    ) -> Result<Option<Mesh>, String> {
        if epoch != self.epoch
            || revision != self.revision
            || document.source_geometry_active_claim().is_some()
            || document.has_source_geometry_normal_state()
            || document.active_subtool_id() != Some(self.active_subtool_id)
            || document.current_level() != self.level
            || document.history_depths() != self.history_depths
        {
            return Err("Source worker reply has a stale owner or document revision".into());
        }
        let current = document.to_json()?;
        if current.len() != self.project.len()
            || blake3::hash(current.as_bytes()) != self.project_digest
        {
            return Err("Source worker reply has a stale complete scene".into());
        }
        let mesh = document
            .active_mesh()
            .ok_or("Source worker reply has no active mesh")?;
        decode_source_worker_mesh_delta(mesh, self.active_subtool_id, delta)
    }

    /// Consume the complete-scene witness and atomically publish a checked
    /// worker result with its exact retained Source state. The retained packet
    /// is authenticated and decoded against the checked candidate *inside*
    /// this one-use publication boundary. An empty delta and empty packet are
    /// a no-op; every other result needs a valid retained packet. A failed
    /// check leaves public state/history intact.
    pub fn commit_candidate(
        self,
        document: &mut Document,
        epoch: u64,
        revision: u64,
        delta: &[u8],
        retained_packet: &[u8],
    ) -> Result<bool, String> {
        let id = self.active_subtool_id;
        let candidate = match self.decode_candidate(document, epoch, revision, delta)? {
            Some(mesh) => mesh,
            None if retained_packet.is_empty() => return Ok(false),
            None => return Err("Source worker no-op unexpectedly included retained state".into()),
        };
        let retained = crate::decode_source_worker_retained_warm(&candidate, id, retained_packet)?;
        document.publish_cold_source_worker_result(candidate, Arc::new(retained))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        encode_source_worker_mesh_delta, encode_source_worker_retained_warm, MaskConvention,
        SourceGeometryAlphaTracks, SourceGeometryControls, SourceGeometryPass,
        SourceWorkerTransaction,
    };

    fn fixture() -> Document {
        let mut mesh = Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nf 1 2 3\n")
            .unwrap()
            .mesh;
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        Document::new(mesh)
    }

    #[test]
    fn cold_capture_checks_complete_scene_and_exact_candidate_words() {
        let doc = fixture();
        let capture =
            SourceWorkerColdCapture::capture(&doc, SourceGeometryOptions::inflate(), 41, 17)
                .unwrap();
        assert_eq!(capture.epoch(), 41);
        assert_eq!(capture.revision(), 17);
        assert_eq!(capture.project_bytes(), doc.to_json().unwrap().as_bytes());
        let before = doc.active_mesh().unwrap();
        let mut after = before.snapshot();
        after.positions[0].z = 0.25;
        after.normals[0].z = 0.75;
        let delta = encode_source_worker_mesh_delta(before, &after, 1).unwrap();
        let candidate = capture
            .decode_candidate(&doc, 41, 17, &delta)
            .unwrap()
            .unwrap();
        assert_eq!(candidate.positions[0].z.to_bits(), 0.25f32.to_bits());
        assert_eq!(candidate.normals[0].z.to_bits(), 0.75f32.to_bits());
    }

    #[test]
    fn stale_revision_and_non_geometry_scene_change_reject_reply() {
        let doc = fixture();
        let capture =
            SourceWorkerColdCapture::capture(&doc, SourceGeometryOptions::inflate(), 8, 10)
                .unwrap();
        let delta = encode_source_worker_mesh_delta(
            doc.active_mesh().unwrap(),
            doc.active_mesh().unwrap(),
            1,
        )
        .unwrap();
        assert!(capture.decode_candidate(&doc, 8, 11, &delta).is_err());
        let capture =
            SourceWorkerColdCapture::capture(&doc, SourceGeometryOptions::inflate(), 8, 10)
                .unwrap();
        let mut changed = fixture();
        changed.name.push_str(" changed");
        assert!(capture.decode_candidate(&changed, 8, 10, &delta).is_err());
    }

    #[test]
    fn warm_retained_state_is_never_seeded_from_public_json() {
        let mut doc = fixture();
        let options = SourceGeometryOptions::inflate();
        let mut stroke = doc.begin_source_geometry_stroke(options).unwrap();
        doc.apply_source_geometry_dab(
            &mut stroke,
            SourceGeometryPass {
                center: [0.0; 3],
                eye_direction: [0.0, 0.0, -1.0],
                local_radius_squared: 4.0,
                original_face_index: 0,
            },
            0.5,
            true,
        )
        .unwrap();
        doc.end_source_geometry_stroke(&stroke).unwrap();
        assert!(doc.has_source_geometry_normal_state());
        assert!(SourceWorkerColdCapture::capture(&doc, options, 2, 3).is_err());
    }

    #[test]
    fn completed_worker_result_publishes_exact_mesh_state_and_one_history_step() {
        let mut main = fixture();
        let before = main.to_json().unwrap();
        let capture =
            SourceWorkerColdCapture::capture(&main, SourceGeometryOptions::inflate(), 47, 5)
                .unwrap();
        let worker = Document::from_json(&before).unwrap();
        let mut transaction =
            SourceWorkerTransaction::begin(worker, SourceGeometryOptions::inflate(), 47).unwrap();
        transaction
            .apply_controlled(
                47,
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
        let mut worker = transaction.finish(47, 2).unwrap();
        let expected = worker.to_json().unwrap();
        let state = worker.source_geometry_state(1).unwrap();
        let retained =
            encode_source_worker_retained_warm(&state, worker.active_mesh().unwrap(), 1).unwrap();
        let delta = encode_source_worker_mesh_delta(
            main.active_mesh().unwrap(),
            worker.active_mesh().unwrap(),
            1,
        )
        .unwrap();
        let mut corrupt = retained.clone();
        corrupt[12] ^= 1;
        assert!(capture
            .commit_candidate(&mut main, 47, 5, &delta, &corrupt)
            .is_err());
        assert_eq!(main.to_json().unwrap(), before);
        assert_eq!(main.history_depths(), (0, 0));
        let capture =
            SourceWorkerColdCapture::capture(&main, SourceGeometryOptions::inflate(), 47, 5)
                .unwrap();
        assert!(capture
            .commit_candidate(&mut main, 47, 5, &delta, &retained)
            .unwrap());
        assert_eq!(main.to_json().unwrap(), expected);
        assert_eq!(main.history_depths(), (1, 0));
        assert!(main
            .source_geometry_state(1)
            .unwrap()
            .matches_mesh(main.active_mesh().unwrap()));
        assert!(main.undo());
        assert_eq!(main.to_json().unwrap(), before);
        assert!(main.redo());
        assert_eq!(main.to_json().unwrap(), expected);
        assert!(main
            .source_geometry_state(1)
            .unwrap()
            .matches_mesh(main.active_mesh().unwrap()));

        let next = SourceGeometryPass {
            center: [0.3, 0.2, 0.0],
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 4.0,
            original_face_index: 0,
        };
        for document in [&mut main, &mut worker] {
            let mut stroke = document
                .begin_source_geometry_selection_history(SourceGeometryOptions::inflate())
                .unwrap();
            document
                .apply_source_geometry_controlled_dab(
                    &mut stroke,
                    [Some(next), None],
                    0.5,
                    true,
                    &SourceGeometryControls::default(),
                    SourceGeometryAlphaTracks::default(),
                )
                .unwrap();
            document.end_source_geometry_stroke(&stroke).unwrap();
        }
        assert_eq!(main.to_json().unwrap(), worker.to_json().unwrap());
    }

    #[test]
    fn rejected_worker_publication_preserves_redo_and_complete_scene() {
        let mut main = fixture();
        let mut edit = main.active_mesh().unwrap().snapshot();
        edit.positions[0].z = 0.5;
        main.replace_mesh(edit).unwrap();
        assert!(main.undo());
        let before = main.to_json().unwrap();
        let depths = main.history_depths();
        let capture =
            SourceWorkerColdCapture::capture(&main, SourceGeometryOptions::inflate(), 9, 10)
                .unwrap();
        let mesh = main.active_mesh().unwrap();
        let delta = encode_source_worker_mesh_delta(mesh, mesh, 1).unwrap();
        assert!(capture
            .commit_candidate(&mut main, 9, 11, &delta, &[])
            .is_err());
        assert_eq!(main.to_json().unwrap(), before);
        assert_eq!(main.history_depths(), depths);

        let capture =
            SourceWorkerColdCapture::capture(&main, SourceGeometryOptions::inflate(), 9, 10)
                .unwrap();
        assert!(!capture
            .commit_candidate(&mut main, 9, 10, &delta, &[])
            .unwrap());
        assert_eq!(main.to_json().unwrap(), before);
        assert_eq!(main.history_depths(), depths);

        let capture =
            SourceWorkerColdCapture::capture(&main, SourceGeometryOptions::inflate(), 9, 10)
                .unwrap();
        assert!(capture
            .commit_candidate(&mut main, 9, 10, &delta, &[1, 2, 3])
            .is_err());
        assert_eq!(main.to_json().unwrap(), before);
        assert_eq!(main.history_depths(), depths);
    }
}
