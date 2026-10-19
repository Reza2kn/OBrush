//! One-use ordinary-studio witness for a cold Source Worker result.
//! The Worker cannot mutate the public document, renderer or history directly.

use obrush_core::{Document, SourceGeometryOptions, SourceWorkerColdCapture};

struct Pending {
    epoch: u64,
    revision: u64,
    capture: SourceWorkerColdCapture,
}

#[derive(Default)]
pub struct ColdSourceWorkerBridge {
    next_epoch: u64,
    pending: Option<Pending>,
}

impl ColdSourceWorkerBridge {
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
