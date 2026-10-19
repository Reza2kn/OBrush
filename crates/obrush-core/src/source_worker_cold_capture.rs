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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encode_source_worker_mesh_delta, MaskConvention, SourceGeometryPass};

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
}
