//! Move-owned Source transaction boundary for a future ordinary browser worker.
//!
//! This is a same-runtime ownership contract, not a wire format or a worker
//! implementation. A host must transfer a complete live Document by a separate
//! checked mechanism before it can instantiate this value in another WASM realm.
use crate::{
    Document, PaintPreparationInput, SourceGeometryAlphaTracks, SourceGeometryControlledReport,
    SourceGeometryControls, SourceGeometryError, SourceGeometryOptions, SourceGeometryPass,
    SourceGeometryStroke, SourceMirrorPlane,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceWorkerTransactionError {
    StaleEpoch,
    OutOfOrder { expected: u64, received: u64 },
    SequenceExhausted,
    Geometry(SourceGeometryError),
}

impl From<SourceGeometryError> for SourceWorkerTransactionError {
    fn from(value: SourceGeometryError) -> Self {
        Self::Geometry(value)
    }
}

/// Owns the only editable Document and its Source claim for one gesture.
/// Neither document nor stroke can be separately borrowed mutably by a host.
pub struct SourceWorkerTransaction {
    document: Document,
    stroke: SourceGeometryStroke,
    epoch: u64,
    next_sequence: u64,
    closed: bool,
}

impl std::fmt::Debug for SourceWorkerTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceWorkerTransaction")
            .field("epoch", &self.epoch)
            .field("next_sequence", &self.next_sequence)
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

impl SourceWorkerTransaction {
    /// `epoch` is assigned by the host and must be unique for its live worker
    /// incarnation. A zero epoch is reserved for uninitialized messages.
    /// On admission failure, the complete unchanged Document is returned.
    #[allow(clippy::result_large_err)] // Return the owned document without allocating on failure.
    pub fn begin(
        mut document: Document,
        options: SourceGeometryOptions,
        epoch: u64,
    ) -> Result<Self, (Document, SourceWorkerTransactionError)> {
        if epoch == 0 {
            return Err((document, SourceWorkerTransactionError::StaleEpoch));
        }
        let stroke = match document.begin_source_geometry_selection_history(options) {
            Ok(stroke) => stroke,
            Err(error) => return Err((document, error.into())),
        };
        Ok(Self {
            document,
            stroke,
            epoch,
            next_sequence: 1,
            closed: false,
        })
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    fn check_message(&self, epoch: u64, sequence: u64) -> Result<(), SourceWorkerTransactionError> {
        if self.closed || epoch != self.epoch {
            return Err(SourceWorkerTransactionError::StaleEpoch);
        }
        if sequence != self.next_sequence {
            return Err(SourceWorkerTransactionError::OutOfOrder {
                expected: self.next_sequence,
                received: sequence,
            });
        }
        Ok(())
    }

    /// Apply one already prepared contact pair through the ordinary shared-core
    /// admission. Validation failures do not consume the sequence or edit state.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_controlled(
        &mut self,
        epoch: u64,
        sequence: u64,
        contacts: [Option<SourceGeometryPass>; 2],
        pressure: f64,
        in_contact: bool,
        controls: &SourceGeometryControls,
        tracks: SourceGeometryAlphaTracks,
    ) -> Result<SourceGeometryControlledReport, SourceWorkerTransactionError> {
        self.check_message(epoch, sequence)?;
        let next = self
            .next_sequence
            .checked_add(1)
            .ok_or(SourceWorkerTransactionError::SequenceExhausted)?;
        let report = self.document.apply_source_geometry_controlled_dab(
            &mut self.stroke,
            contacts,
            pressure,
            in_contact,
            controls,
            tracks,
        )?;
        self.next_sequence = next;
        Ok(report)
    }

    /// Keep the ordinary screen/ray contact preparation inside the same owner
    /// as the editable mesh. Input is copied across the future host boundary;
    /// all pick, retained-state, mask and geometry checks still run in core.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_input(
        &mut self,
        epoch: u64,
        sequence: u64,
        input: PaintPreparationInput,
        mirror: Option<SourceMirrorPlane>,
        in_contact: bool,
        controls: &SourceGeometryControls,
        tracks: SourceGeometryAlphaTracks,
    ) -> Result<SourceGeometryControlledReport, SourceWorkerTransactionError> {
        self.check_message(epoch, sequence)?;
        let next = self
            .next_sequence
            .checked_add(1)
            .ok_or(SourceWorkerTransactionError::SequenceExhausted)?;
        let prepared = self.document.prepare_source_geometry_controlled_dab(
            &mut self.stroke,
            input,
            mirror,
        )?;
        let report = prepared.apply(input.pressure, in_contact, controls, tracks)?;
        self.next_sequence = next;
        Ok(report)
    }

    /// Finish exactly once. The shared core preserves Source selection history
    /// and cancels its whole-state snapshot on failed release. A stale release
    /// leaves the transaction available for a correctly ordered retry.
    #[allow(clippy::result_large_err)] // Preserve move ownership even if allocation is exhausted.
    pub fn finish(
        mut self,
        epoch: u64,
        sequence: u64,
    ) -> Result<Document, (Self, SourceWorkerTransactionError)> {
        if let Err(error) = self.check_message(epoch, sequence) {
            return Err((self, error));
        }
        match self.document.end_source_geometry_stroke(&self.stroke) {
            Ok(()) => Ok(self.document),
            Err(error) => {
                self.closed = true;
                Err((self, error.into()))
            }
        }
    }

    /// Cancel leaves public history and redo untouched. A stale cancellation
    /// cannot retire a newer owner's transaction.
    #[allow(clippy::result_large_err)] // A rejected cancellation must retain the entire owner.
    pub fn cancel(
        mut self,
        epoch: u64,
        sequence: u64,
    ) -> Result<Document, (Self, SourceWorkerTransactionError)> {
        if let Err(error) = self.check_message(epoch, sequence) {
            return Err((self, error));
        }
        match self.document.cancel_source_geometry_stroke(&self.stroke) {
            Ok(()) => Ok(self.document),
            Err(error) => Err((self, error.into())),
        }
    }

    /// Recover the complete Document after a failed release, which has already
    /// consumed/canceled the core claim. An open transaction cannot be extracted.
    #[allow(clippy::result_large_err)] // Rejection returns the owner without a fallible box.
    pub fn into_canceled_document(self) -> Result<Document, Self> {
        if self.closed {
            Ok(self.document)
        } else {
            Err(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MaskConvention, Mesh, PaintViewCapture};
    use glam::{DMat4, DVec3};

    fn fixture() -> Document {
        let mut mesh = Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nf 1 2 3\n")
            .unwrap()
            .mesh;
        mesh.mask_convention = MaskConvention::UnprotectedF32V1;
        mesh.masks.fill(1.0);
        Document::new(mesh)
    }

    fn options() -> SourceGeometryOptions {
        SourceGeometryOptions {
            intensity: 0.625,
            ..SourceGeometryOptions::inflate()
        }
    }

    fn contacts() -> [Option<SourceGeometryPass>; 2] {
        [
            Some(SourceGeometryPass {
                center: [0.0; 3],
                eye_direction: [0.0, 0.0, -1.0],
                local_radius_squared: 4.0,
                original_face_index: 0,
            }),
            None,
        ]
    }

    fn dab(
        tx: &mut SourceWorkerTransaction,
        epoch: u64,
        sequence: u64,
        pressure: f64,
    ) -> Result<SourceGeometryControlledReport, SourceWorkerTransactionError> {
        tx.apply_controlled(
            epoch,
            sequence,
            contacts(),
            pressure,
            true,
            &SourceGeometryControls::default(),
            SourceGeometryAlphaTracks::default(),
        )
    }

    #[test]
    fn first_cold_controlled_dab_exposes_selected_chunk_envelope() {
        let document = fixture();
        assert!(document.source_geometry_state(1).is_none());
        let mut tx = SourceWorkerTransaction::begin(document, options(), 93).unwrap();
        let report = dab(&mut tx, 93, 1, 0.5).unwrap();
        assert!(report.combined.changed_vertices > 0);
        let state = tx.document().source_geometry_state(1).unwrap();
        let chunks = state
            .render_selected_chunks_after_new_worker_state()
            .unwrap();
        assert_ne!(chunks, [0; 8]);
        assert_ne!(chunks[0] & 1, 0);
    }

    #[test]
    fn ordered_owner_preserves_one_history_event_and_rejects_stale_messages() {
        let document = fixture();
        let original = document.to_json().unwrap();
        let mut tx = SourceWorkerTransaction::begin(document, options(), 93).unwrap();
        assert_eq!(tx.epoch(), 93);
        assert_eq!(tx.next_sequence(), 1);
        assert!(dab(&mut tx, 93, 1, 0.5).unwrap().combined.changed_vertices > 0);
        let edited = tx.document().to_json().unwrap();
        assert_ne!(edited, original);
        assert_eq!(
            dab(&mut tx, 93, 1, 0.5).unwrap_err(),
            SourceWorkerTransactionError::OutOfOrder {
                expected: 2,
                received: 1,
            }
        );
        assert_eq!(
            dab(&mut tx, 94, 2, 0.5).unwrap_err(),
            SourceWorkerTransactionError::StaleEpoch
        );
        assert_eq!(tx.document().to_json().unwrap(), edited);
        assert_eq!(
            dab(&mut tx, 93, 2, f64::NAN).unwrap_err(),
            SourceWorkerTransactionError::Geometry(SourceGeometryError::InvalidInput)
        );
        assert_eq!(tx.next_sequence(), 2);
        assert!(dab(&mut tx, 93, 2, 0.5).unwrap().combined.changed_vertices > 0);
        let edited = tx.document().to_json().unwrap();
        let (tx, error) = tx.finish(93, 4).unwrap_err();
        assert_eq!(
            error,
            SourceWorkerTransactionError::OutOfOrder {
                expected: 3,
                received: 4
            }
        );
        let mut document = tx.finish(93, 3).unwrap();
        assert_eq!(document.to_json().unwrap(), edited);

        let mut baseline = fixture();
        let mut baseline_stroke = baseline
            .begin_source_geometry_selection_history(options())
            .unwrap();
        baseline
            .apply_source_geometry_controlled_dab(
                &mut baseline_stroke,
                contacts(),
                0.5,
                true,
                &SourceGeometryControls::default(),
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        baseline
            .apply_source_geometry_controlled_dab(
                &mut baseline_stroke,
                contacts(),
                0.5,
                true,
                &SourceGeometryControls::default(),
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        baseline
            .end_source_geometry_stroke(&baseline_stroke)
            .unwrap();
        assert_eq!(document.to_json().unwrap(), baseline.to_json().unwrap());
        let worker_normals = document.source_geometry_state(1).unwrap();
        let direct_normals = baseline.source_geometry_state(1).unwrap();
        assert_eq!(
            worker_normals
                .vertex_normals()
                .iter()
                .map(|v| v.to_array().map(f32::to_bits))
                .collect::<Vec<_>>(),
            direct_normals
                .vertex_normals()
                .iter()
                .map(|v| v.to_array().map(f32::to_bits))
                .collect::<Vec<_>>()
        );
        assert!(document.undo());
        assert_eq!(document.to_json().unwrap(), original);
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), edited);
    }

    #[test]
    fn canceled_owner_restores_geometry_cache_and_existing_redo() {
        let mut document = fixture();
        document.begin_stroke();
        document.name = "future".into();
        document.end_stroke();
        let future = document.to_json().unwrap();
        assert!(document.undo());
        let original = document.to_json().unwrap();
        let mut tx = SourceWorkerTransaction::begin(document, options(), 9).unwrap();
        dab(&mut tx, 9, 1, 0.5).unwrap();
        let (tx, error) = tx.cancel(8, 2).unwrap_err();
        assert_eq!(error, SourceWorkerTransactionError::StaleEpoch);
        let mut document = tx.cancel(9, 2).unwrap();
        assert_eq!(document.to_json().unwrap(), original);
        assert!(document.source_geometry_state(1).is_none());
        assert!(document.redo());
        assert_eq!(document.to_json().unwrap(), future);
    }

    #[test]
    fn rejected_begin_returns_complete_document() {
        let document = fixture();
        let original = document.to_json().unwrap();
        let (document, error) = SourceWorkerTransaction::begin(document, options(), 0).unwrap_err();
        assert_eq!(error, SourceWorkerTransactionError::StaleEpoch);
        assert_eq!(document.to_json().unwrap(), original);

        let mut mesh = Mesh::import_obj(b"v 0 0 0\nv 4 0 0\nv 0 4 0\nf 1 2 3\n")
            .unwrap()
            .mesh;
        mesh.masks.fill(1.0);
        let document = Document::new(mesh);
        let original = document.to_json().unwrap();
        let (document, error) = SourceWorkerTransaction::begin(document, options(), 1).unwrap_err();
        assert_eq!(
            error,
            SourceWorkerTransactionError::Geometry(SourceGeometryError::MaskConvention)
        );
        assert_eq!(document.to_json().unwrap(), original);
    }

    #[test]
    fn input_preparation_runs_inside_owner_and_matches_direct_core() {
        let input = PaintPreparationInput {
            view: PaintViewCapture {
                world_to_clip: (DMat4::orthographic_rh_gl(-2.0, 2.0, -2.0, 2.0, 0.1, 20.0)
                    * DMat4::look_at_rh(DVec3::new(0.0, 0.0, 4.0), DVec3::ZERO, DVec3::Y))
                .to_cols_array(),
                viewport: [0.0, 0.0, 512.0, 512.0],
            },
            screen: [320.0, 192.0],
            screen_radius: 220.0,
            pressure: 0.5,
            radius_factor: 1.0,
        };
        let mut tx = SourceWorkerTransaction::begin(fixture(), options(), 7).unwrap();
        let controlled = tx
            .apply_input(
                7,
                1,
                input,
                None,
                true,
                &SourceGeometryControls::default(),
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        assert!(controlled.combined.changed_vertices > 0);
        let from_owner = tx.finish(7, 2).unwrap();

        let mut direct = fixture();
        let mut stroke = direct
            .begin_source_geometry_selection_history(options())
            .unwrap();
        let prepared = direct
            .prepare_source_geometry_controlled_dab(&mut stroke, input, None)
            .unwrap();
        let ordinary = prepared
            .apply(
                input.pressure,
                true,
                &SourceGeometryControls::default(),
                SourceGeometryAlphaTracks::default(),
            )
            .unwrap();
        direct.end_source_geometry_stroke(&stroke).unwrap();
        assert_eq!(
            controlled.combined.changed_vertices,
            ordinary.combined.changed_vertices
        );
        assert_eq!(from_owner.to_json().unwrap(), direct.to_json().unwrap());
    }
}
