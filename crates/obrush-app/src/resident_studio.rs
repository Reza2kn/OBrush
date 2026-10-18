//! One-use bridge from a captured studio document to a GPU-produced sparse
//! result. The ordinary shared-core document remains the only history owner.
use obrush_core::Document;
use obrush_core::resident_point_program::{ResidentCommitTiming, ResidentDocumentCapture};

struct Pending {
    id: u64,
    captured: ResidentDocumentCapture,
}

#[derive(Default)]
pub struct ResidentStudioBridge {
    next_id: u64,
    pending: Option<Pending>,
    last_commit_timing: Option<ResidentCommitTiming>,
}

impl ResidentStudioBridge {
    /// Compact transient GPU upload for large meshes. The pending core capture
    /// still owns stale-result validation and the only Undo transaction.
    pub fn begin_upload(&mut self, document: &Document) -> Result<(u64, Vec<u8>), String> {
        if self.pending.is_some() {
            return Err("A resident studio capture is already active".into());
        }
        if document.can_redo() {
            return Err("Resident studio capture would replace an active redo branch".into());
        }
        self.last_commit_timing = None;
        let upload = obrush_core::resident_upload::encode_document(document)?;
        let captured = ResidentDocumentCapture::capture(document)?;
        let id = self
            .next_id
            .checked_add(1)
            .ok_or("Resident studio capture IDs exhausted")?;
        self.next_id = id;
        self.pending = Some(Pending { id, captured });
        Ok((id, upload))
    }

    pub fn begin(&mut self, document: &Document) -> Result<(u64, String), String> {
        if self.pending.is_some() {
            return Err("A resident studio capture is already active".into());
        }
        if document.can_redo() {
            return Err("Resident studio capture would replace an active redo branch".into());
        }
        self.last_commit_timing = None;
        let captured = ResidentDocumentCapture::capture(document)?;
        let project = document.to_json()?;
        let id = self
            .next_id
            .checked_add(1)
            .ok_or("Resident studio capture IDs exhausted")?;
        self.next_id = id;
        self.pending = Some(Pending { id, captured });
        Ok((id, project))
    }

    pub fn commit(
        &mut self,
        document: &mut Document,
        id: u64,
        sparse_bytes: &[u8],
    ) -> Result<bool, String> {
        let pending = self
            .pending
            .as_ref()
            .ok_or("No active resident studio capture")?;
        if pending.id != id {
            return Err("Resident studio capture token is stale".into());
        }
        let (changed, timing) = pending
            .captured
            .commit_sparse_position_bytes_timed(document, sparse_bytes)?;
        self.pending = None;
        self.last_commit_timing = Some(timing);
        Ok(changed)
    }

    pub fn cancel(&mut self, id: u64) -> Result<(), String> {
        if self.pending.as_ref().is_none_or(|pending| pending.id != id) {
            return Err("Resident studio capture token is stale".into());
        }
        self.pending = None;
        Ok(())
    }

    pub fn invalidate(&mut self) {
        self.pending = None;
    }

    pub fn last_commit_timing(&self) -> Option<ResidentCommitTiming> {
        self.last_commit_timing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use obrush_core::{Mesh, Vec3};

    fn sparse(index: u32, point: Vec3) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&index.to_le_bytes());
        for value in [point.x, point.y, point.z, 1.] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn captured_studio_result_is_one_undoable_edit_and_token_is_one_use() {
        let mut document = Document::new(Mesh::sphere(2));
        let before = document.to_json().unwrap();
        let mut bridge = ResidentStudioBridge::default();
        let (id, project) = bridge.begin(&document).unwrap();
        assert_eq!(project, before);
        assert!(bridge.begin(&document).is_err());
        let index = 25usize;
        let mut point = document.active_mesh().unwrap().positions[index];
        point += document.active_mesh().unwrap().normals[index] * 0.001;
        assert!(
            bridge
                .commit(&mut document, id, &sparse(index as u32, point))
                .unwrap()
        );
        let after = document.to_json().unwrap();
        assert_ne!(before, after);
        assert!(bridge.commit(&mut document, id, &[]).is_err());
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), after);
    }

    #[test]
    fn cancelled_stale_and_invalid_results_preserve_document() {
        let mut document = Document::new(Mesh::sphere(2));
        let mut bridge = ResidentStudioBridge::default();
        let (id, _) = bridge.begin(&document).unwrap();
        let before = document.to_json().unwrap();
        assert!(bridge.commit(&mut document, id + 1, &[]).is_err());
        assert!(bridge.commit(&mut document, id, &[1, 2]).is_err());
        assert_eq!(document.to_json().unwrap(), before);
        bridge.cancel(id).unwrap();
        assert!(bridge.commit(&mut document, id, &[]).is_err());
        let (new_id, _) = bridge.begin(&document).unwrap();
        assert!(new_id > id);
        bridge.invalidate();
        assert!(bridge.commit(&mut document, new_id, &[]).is_err());
        assert_eq!(document.to_json().unwrap(), before);
    }

    #[test]
    fn compact_capture_binds_the_same_one_use_document_transaction() {
        let mut document = Document::new(Mesh::sphere(2));
        let mut bridge = ResidentStudioBridge::default();
        let (id, upload) = bridge.begin_upload(&document).unwrap();
        let decoded = obrush_core::resident_upload::decode(&upload).unwrap();
        assert_eq!(decoded.subtool_id, document.active_subtool_id().unwrap());
        assert_eq!(
            decoded.input.positions().len(),
            document.active_mesh().unwrap().positions.len()
        );
        assert!(bridge.begin(&document).is_err());
        let before = document.to_json().unwrap();
        let point = document.active_mesh().unwrap().positions[25] + Vec3::Z * 0.001;
        assert!(
            bridge
                .commit(&mut document, id, &sparse(25, point))
                .unwrap()
        );
        let after = document.to_json().unwrap();
        assert_ne!(after, before);
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), before);
    }
}
