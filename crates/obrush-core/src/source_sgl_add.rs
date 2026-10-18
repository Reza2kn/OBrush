//! Append-only source SGL transaction. Complete staging/scene/output validation
//! precedes exchange. Global view headers are returned separately from history.
use super::*;

#[derive(Clone, Debug, Serialize)]
pub struct SglAppendReport {
    pub added_ids: Vec<u32>,
    pub active_id: Option<u32>,
    pub selected_ids: Vec<u32>,
    pub input_header: crate::SglSceneOrigin,
    pub document_changed: bool,
}
impl Document {
    /// A source Add appends, selects the final incoming object and commits one
    /// complete history entry. A valid zero-mesh file changes no document field,
    /// ID allocator, selection, pending stroke, Undo or Redo. Its view header is
    /// still returned so the host can apply modern globals before that decision.
    /// The host must complete its current stroke before a nonempty append.
    pub fn append_staged_sgl_scene(
        &mut self,
        staged: crate::StagedSglScene,
    ) -> Result<SglAppendReport, String> {
        staged.validate()?;
        let header = staged.origin.clone();
        if staged.objects.is_empty() {
            return Ok(SglAppendReport {
                added_ids: Vec::new(),
                active_id: self.active_subtool_id(),
                selected_ids: self.selected_subtool_ids(),
                input_header: header,
                document_changed: false,
            });
        }
        if self.stroke_start.is_some() || self.layer_error.is_some() {
            return Err(
                "Finish the current stroke or restored layer edit before adding an SGL scene"
                    .into(),
            );
        }
        if self
            .subtool_count()
            .checked_add(staged.objects.len())
            .is_none_or(|n| n > crate::MAX_SUBTOOLS)
        {
            return Err("Source SGL Add exceeds the32-part scene limit".into());
        }
        let previous = self.snapshot_state();
        if previous.approximate_bytes() > HISTORY_BUDGET_BYTES {
            return Err(
                "Source SGL Add cannot retain its complete original scene within the history budget".into(),
            );
        }
        let mut candidate = Document::empty();
        candidate.exchange_state(previous);
        let mut scene = candidate.scene.clone().unwrap_or_else(|| {
            if candidate.mesh.is_some() {
                Scene::initial(&candidate.name)
            } else {
                Scene::empty()
            }
        });
        let end_id = scene
            .next_id
            .checked_add(
                u32::try_from(staged.objects.len())
                    .map_err(|_| "SGL object count exceeds stable ID range")?,
            )
            .ok_or("SubTool ID allocator is exhausted")?;
        scene
            .objects
            .try_reserve_exact(staged.objects.len())
            .map_err(|_| "SGL scene membership allocation failed")?;
        if let Some(active) = scene.active_id {
            let index = scene.index(active)?;
            scene.objects[index].payload = Some(candidate.capture_object()?);
        }
        let mut added_ids = Vec::new();
        added_ids
            .try_reserve_exact(staged.objects.len())
            .map_err(|_| "SGL added-ID allocation failed")?;
        let count = staged.objects.len();
        for (ordinal, object) in staged.objects.into_iter().enumerate() {
            let id = scene.next_id;
            scene.next_id = id
                .checked_add(1)
                .ok_or("SubTool ID allocator is exhausted")?;
            let metadata = crate::SceneObjectMetadata {
                source_center_initialized: object.source_center_initialized,
                affine: object.affine,
                render: object.render,
                picking_policy: staged.picking_policy,
                origin: Some(crate::SceneObjectOrigin {
                    header: header.clone(),
                    object: object.origin,
                }),
            };
            metadata.validate(&object.mesh)?;
            let payload = if ordinal + 1 == count {
                candidate.mesh = Some(object.mesh);
                candidate.active_metadata = Some(metadata);
                None
            } else {
                Some(Arc::new(ObjectState::with_metadata(
                    object.mesh,
                    None,
                    None,
                    None,
                    Some(metadata),
                )))
            };
            let name = format!("SGL object {id}");
            crate::subtools::validate_name(&name)?;
            scene.objects.push(Subtool {
                id,
                name,
                visible: true,
                payload,
            });
            added_ids.push(id);
        }
        debug_assert_eq!(scene.next_id, end_id);
        scene.active_id = added_ids.last().copied();
        scene.selected_ids = scene.active_id.into_iter().collect();
        // Existing Solo remains an explicit OBrush view preference. Incoming
        // local data, poses and raw origins are never normalized or baked here.
        candidate.multiresolution = None;
        candidate.layers = None;
        candidate.morph_target = None;
        if header.version >= 2 || candidate.scene_origin.is_none() {
            candidate.scene_origin = Some(header.clone());
        }
        scene.validate_structure()?;
        candidate.scene = Some(scene);
        candidate.validate_complete_state()?;
        candidate.to_json()?;
        let report = SglAppendReport {
            added_ids,
            active_id: candidate.active_subtool_id(),
            selected_ids: candidate.selected_subtool_ids(),
            input_header: header,
            document_changed: true,
        };
        self.replace_document(candidate)?;
        Ok(report)
    }
    pub fn append_sgl_bytes(
        &mut self,
        bytes: &[u8],
        settings: &crate::SglStageSettings,
    ) -> Result<SglAppendReport, String> {
        let staged = crate::StagedSglScene::from_sgl_bytes(bytes, settings)?;
        self.append_staged_sgl_scene(staged)
    }
}
