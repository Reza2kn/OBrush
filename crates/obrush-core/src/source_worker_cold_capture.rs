//! Fail-closed cross-realm seed for an ordinary Source worker gesture.
//!
//! A public project does not contain the retained Source normal/tree state or
//! live history claim. The active producer is transferred separately through
//! the bounded warm packet and remains witnessed by this one-use capture.

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
    active_mesh_addr: usize,
    retained: Option<Arc<crate::SourceGeometryNormalState>>,
    retained_packet: Vec<u8>,
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
        let active_subtool_id = document
            .active_subtool_id()
            .ok_or("Source worker capture has no active SubTool")?;
        let retained = document.source_geometry_state(active_subtool_id);
        // The public seed has no representation for idle other-part producers.
        // Do not silently drop one while copying the scene into the worker.
        if document.source_geometry_state_count() != usize::from(retained.is_some()) {
            return Err("Source worker capture has an untransferred other-part state".into());
        }
        let retained_packet = match retained.as_ref() {
            Some(state) => crate::encode_source_worker_retained_warm(
                state,
                document
                    .active_mesh()
                    .ok_or("Source worker has no active mesh")?,
                active_subtool_id,
            )?,
            None => Vec::new(),
        };
        let active_mesh_addr = document
            .active_mesh()
            .ok_or("Source worker has no active mesh")?
            as *const Mesh as usize;
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
            active_mesh_addr,
            project,
            retained,
            retained_packet,
        })
    }

    pub fn project_bytes(&self) -> &[u8] {
        &self.project
    }

    pub fn retained_packet(&self) -> &[u8] {
        &self.retained_packet
    }

    pub fn take_retained_packet(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.retained_packet)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn active_subtool_id(&self) -> u32 {
        self.active_subtool_id
    }

    /// Validate the complete public scene and transient producer at the final
    /// publication boundary. Preview uses only the cheap live-owner witness.
    pub fn verify_scene(
        &self,
        document: &Document,
        epoch: u64,
        revision: u64,
    ) -> Result<(), String> {
        self.verify_live_owner(document, epoch, revision)?;
        let (bytes, digest) = document.project_json_digest()?;
        if bytes != self.project.len() || digest != self.project_digest {
            return Err("Source worker reply has a stale complete scene".into());
        }
        Ok(())
    }

    /// Check immutable capture identity and the host's document revision
    /// without serializing the million-vertex scene for every Worker reply.
    pub fn verify_live_owner(
        &self,
        document: &Document,
        epoch: u64,
        revision: u64,
    ) -> Result<(), String> {
        if epoch != self.epoch
            || revision != self.revision
            || document.source_geometry_active_claim().is_some()
            || document.source_geometry_state_count() != usize::from(self.retained.is_some())
            || match (
                &self.retained,
                document.source_geometry_state(self.active_subtool_id),
            ) {
                (None, None) => false,
                (Some(expected), Some(current)) => !Arc::ptr_eq(expected, &current),
                _ => true,
            }
            || document.active_subtool_id() != Some(self.active_subtool_id)
            || document
                .active_mesh()
                .map(|mesh| mesh as *const Mesh as usize)
                != Some(self.active_mesh_addr)
            || document.current_level() != self.level
            || document.history_depths() != self.history_depths
        {
            return Err("Source worker reply has a stale owner or document revision".into());
        }
        Ok(())
    }

    /// Decode a packet against the original mesh. A successful result is
    /// still only a candidate: adoption and undo are separate steps.
    pub fn decode_candidate(
        &self,
        document: &Document,
        epoch: u64,
        revision: u64,
        delta: &[u8],
    ) -> Result<Option<Mesh>, String> {
        self.verify_scene(document, epoch, revision)?;
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
        self.commit_candidate_checked(document, epoch, revision, delta, retained_packet, |_, _| {
            Ok(())
        })
    }

    /// Check the decoded candidate against an additional host witness before
    /// adopting it. The complete scene and packet are each checked exactly
    /// once; the callback never receives a mutable document or an unchecked
    /// mesh. This keeps preview comparison inside the same one-use commit.
    pub fn commit_candidate_checked(
        self,
        document: &mut Document,
        epoch: u64,
        revision: u64,
        delta: &[u8],
        retained_packet: &[u8],
        check: impl FnOnce(&Mesh, Option<&Mesh>) -> Result<(), String>,
    ) -> Result<bool, String> {
        let id = self.active_subtool_id;
        let decoded = self.decode_candidate(document, epoch, revision, delta)?;
        let baseline = document
            .active_mesh()
            .ok_or("Source worker reply has no active mesh")?;
        check(baseline, decoded.as_ref())?;
        let candidate = match decoded {
            Some(mesh) => mesh,
            None if retained_packet.is_empty() => return Ok(false),
            None => baseline.try_snapshot()?,
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
    use std::cell::Cell;

    fn fixture() -> Document {
        let mut mesh = Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nf 1 2 3\n")
            .unwrap()
            .mesh;
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        Document::new(mesh)
    }

    #[test]
    fn streamed_scene_digest_matches_exact_public_json_bytes() {
        let mut plain = fixture();
        let mut rich = Document::new(Mesh::sphere(2));
        rich.name = "source witness · \"scene\"".into();
        rich.duplicate_subtool(1).unwrap();
        for document in [&mut plain, &mut rich] {
            let json = document.to_json().unwrap();
            let (bytes, digest) = document.project_json_digest().unwrap();
            assert_eq!(bytes, json.len());
            assert_eq!(digest, blake3::hash(json.as_bytes()));
        }
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
    fn in_place_mutation_can_pass_preview_owner_but_final_digest_rejects() {
        let mut doc = fixture();
        let capture =
            SourceWorkerColdCapture::capture(&doc, SourceGeometryOptions::inflate(), 8, 10)
                .unwrap();
        let baseline = doc.active_mesh().unwrap().snapshot();
        let delta = encode_source_worker_mesh_delta(&baseline, &baseline, 1).unwrap();
        doc.active_mesh_mut().unwrap().positions[0].z = 0.25;
        // The cheap pointer/revision check is intentionally provisional.
        assert!(capture.verify_live_owner(&doc, 8, 10).is_ok());
        assert!(capture.decode_candidate(&doc, 8, 10, &delta).is_err());
    }

    #[test]
    fn warm_retained_state_uses_separate_checked_seed() {
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
        let mut capture = SourceWorkerColdCapture::capture(&doc, options, 2, 3).unwrap();
        let packet = capture.take_retained_packet();
        assert!(!packet.is_empty());
        assert!(capture.retained_packet().is_empty());
        let mut worker = Document::from_json(&doc.to_json().unwrap()).unwrap();
        assert!(!worker.has_source_geometry_normal_state());
        worker.install_source_worker_warm_seed(&packet).unwrap();
        let expected = doc.source_geometry_state(1).unwrap();
        let actual = worker.source_geometry_state(1).unwrap();
        assert_eq!(expected.seed_fingerprint(), actual.seed_fingerprint());
        let before = doc.active_mesh().unwrap();
        let delta = encode_source_worker_mesh_delta(before, before, 1).unwrap();
        assert!(capture.decode_candidate(&doc, 2, 3, &delta).is_ok());
        doc.reset_source_geometry_normal_state().unwrap();
        assert!(capture.decode_candidate(&doc, 2, 3, &delta).is_err());
    }

    #[test]
    fn other_part_retained_state_is_not_dropped_from_worker_seed() {
        let mut doc = fixture();
        let other = doc.duplicate_subtool(1).unwrap();
        doc.select_subtool(other).unwrap();
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
        assert!(doc.source_geometry_state(other).is_some());
        doc.select_subtool(1).unwrap();
        assert!(SourceWorkerColdCapture::capture(&doc, options, 2, 3).is_err());
    }

    #[test]
    fn second_worker_stroke_uses_warm_state_and_one_new_history_step() {
        let mut main = fixture();
        let options = SourceGeometryOptions::inflate();
        let pass = SourceGeometryPass {
            center: [0.0; 3],
            eye_direction: [0.0, 0.0, -1.0],
            local_radius_squared: 4.0,
            original_face_index: 0,
        };
        let mut first = SourceWorkerTransaction::begin(
            Document::from_json(&main.to_json().unwrap()).unwrap(),
            options,
            61,
        )
        .unwrap();
        first
            .apply_controlled(
                61,
                1,
                [Some(pass), None],
                0.5,
                true,
                &SourceGeometryControls::default(),
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        let first = first.finish(61, 2).unwrap();
        let first_delta = encode_source_worker_mesh_delta(
            main.active_mesh().unwrap(),
            first.active_mesh().unwrap(),
            1,
        )
        .unwrap();
        let first_retained = encode_source_worker_retained_warm(
            &first.source_geometry_state(1).unwrap(),
            first.active_mesh().unwrap(),
            1,
        )
        .unwrap();
        let capture = SourceWorkerColdCapture::capture(&main, options, 61, 4).unwrap();
        assert!(capture
            .commit_candidate(&mut main, 61, 4, &first_delta, &first_retained)
            .unwrap());
        assert_eq!(main.history_depths(), (1, 0));

        let mut capture = SourceWorkerColdCapture::capture(&main, options, 62, 5).unwrap();
        let warm_seed = capture.take_retained_packet();
        assert!(!warm_seed.is_empty());
        let mut second = Document::from_json(&main.to_json().unwrap()).unwrap();
        second.install_source_worker_warm_seed(&warm_seed).unwrap();
        let mut second = SourceWorkerTransaction::begin(second, options, 62).unwrap();
        second
            .apply_controlled(
                62,
                1,
                [Some(pass), None],
                0.5,
                true,
                &SourceGeometryControls::default(),
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        let second = second.finish(62, 2).unwrap();
        let second_delta = encode_source_worker_mesh_delta(
            main.active_mesh().unwrap(),
            second.active_mesh().unwrap(),
            1,
        )
        .unwrap();
        let second_retained = encode_source_worker_retained_warm(
            &second.source_geometry_state(1).unwrap(),
            second.active_mesh().unwrap(),
            1,
        )
        .unwrap();
        assert!(capture
            .commit_candidate(&mut main, 62, 5, &second_delta, &second_retained)
            .unwrap());
        assert_eq!(main.history_depths(), (2, 0));
        assert_eq!(main.to_json().unwrap(), second.to_json().unwrap());
        assert_eq!(
            main.source_geometry_state(1).unwrap().seed_fingerprint(),
            second.source_geometry_state(1).unwrap().seed_fingerprint(),
        );
    }

    #[test]
    fn empty_worker_release_preserves_redo_branch() {
        let mut main = fixture();
        main.begin_stroke();
        main.active_mesh_mut().unwrap().positions[0].z = 0.25;
        main.end_stroke();
        assert!(main.undo());
        let depths = main.history_depths();
        assert_eq!(depths, (0, 1));
        let before = main.to_json().unwrap();
        let capture =
            SourceWorkerColdCapture::capture(&main, SourceGeometryOptions::inflate(), 73, 8)
                .unwrap();
        let empty = encode_source_worker_mesh_delta(
            main.active_mesh().unwrap(),
            main.active_mesh().unwrap(),
            1,
        )
        .unwrap();
        assert!(!capture
            .commit_candidate(&mut main, 73, 8, &empty, &[])
            .unwrap());
        assert_eq!(main.to_json().unwrap(), before);
        assert_eq!(main.history_depths(), depths);
        assert!(main.redo());
    }

    #[test]
    fn final_candidate_check_runs_once_and_failure_is_atomic() {
        let mut main = fixture();
        let before = main.to_json().unwrap();
        let depths = main.history_depths();
        let empty = encode_source_worker_mesh_delta(
            main.active_mesh().unwrap(),
            main.active_mesh().unwrap(),
            1,
        )
        .unwrap();
        let calls = Cell::new(0);
        let capture =
            SourceWorkerColdCapture::capture(&main, SourceGeometryOptions::inflate(), 74, 9)
                .unwrap();
        assert!(capture
            .commit_candidate_checked(&mut main, 74, 9, &empty, &[], |baseline, candidate| {
                calls.set(calls.get() + 1);
                assert_eq!(baseline.positions.len(), 3);
                assert!(candidate.is_none());
                Err("Final preview mismatch".into())
            })
            .is_err());
        assert_eq!(calls.get(), 1);
        assert_eq!(main.to_json().unwrap(), before);
        assert_eq!(main.history_depths(), depths);

        let capture =
            SourceWorkerColdCapture::capture(&main, SourceGeometryOptions::inflate(), 75, 9)
                .unwrap();
        let calls = Cell::new(0);
        assert!(capture
            .commit_candidate_checked(&mut main, 75, 10, &empty, &[], |_, _| {
                calls.set(calls.get() + 1);
                Ok(())
            })
            .is_err());
        assert_eq!(calls.get(), 0);
        assert_eq!(main.to_json().unwrap(), before);
        assert_eq!(main.history_depths(), depths);

        let capture =
            SourceWorkerColdCapture::capture(&main, SourceGeometryOptions::inflate(), 76, 9)
                .unwrap();
        let calls = Cell::new(0);
        assert!(capture
            .commit_candidate_checked(&mut main, 76, 9, &empty[..empty.len() - 1], &[], |_, _| {
                calls.set(calls.get() + 1);
                Ok(())
            })
            .is_err());
        assert_eq!(calls.get(), 0);
        assert_eq!(main.to_json().unwrap(), before);
        assert_eq!(main.history_depths(), depths);
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
