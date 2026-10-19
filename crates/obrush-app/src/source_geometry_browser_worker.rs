//! Display-free WASM owner for ordered ordinary Source geometry messages.
//!
//! This is deliberately separate from `WebHandle`: a Worker realm must not
//! construct an eframe runner, GPU device or second browser document UI.

use obrush_core::{
    Document, Mesh, PaintPreparationInput, PaintViewCapture, SOURCE_GEOMETRY_CHUNK_ROWS,
    SourceGeometryAlphaTracks, SourceGeometryControls, SourceGeometryOptions, SourceNormalField,
    SourceNormalPolicy, SourceWorkerPreviewRow, SourceWorkerTransaction,
    encode_source_worker_mesh_delta, encode_source_worker_sparse_preview_rows_mode,
};
use serde_json::Value;
use wasm_bindgen::prelude::*;

const MAX_WORKER_INPUT_BYTES: usize = 8 * 1024;
const MAX_WORKER_PROJECT_BYTES: usize = 512 * 1024 * 1024;

fn copy_display_normals(source: &[glam::Vec3]) -> Result<Vec<glam::Vec3>, JsValue> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(source.len())
        .map_err(|_| JsValue::from_str("Cannot reserve Source worker display normals"))?;
    copy.extend_from_slice(source);
    Ok(copy)
}

fn field_array<const N: usize>(value: &Value, key: &str) -> Result<[f64; N], String> {
    let words = value
        .get(key)
        .and_then(Value::as_array)
        .filter(|words| words.len() == N)
        .ok_or_else(|| format!("Source worker {key} shape is invalid"))?;
    let mut result = [0.0; N];
    for (out, input) in result.iter_mut().zip(words) {
        *out = input
            .as_f64()
            .filter(|word| word.is_finite())
            .ok_or_else(|| format!("Source worker {key} contains a nonfinite word"))?;
    }
    Ok(result)
}

fn field_f64(value: &Value, key: &str) -> Result<f64, String> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|word| word.is_finite())
        .ok_or_else(|| format!("Source worker {key} is invalid"))
}

fn input_from_json(source: &str) -> Result<(PaintPreparationInput, bool), String> {
    if source.len() > MAX_WORKER_INPUT_BYTES {
        return Err("Source worker input exceeds the message limit".into());
    }
    let value: Value = serde_json::from_str(source).map_err(|error| error.to_string())?;
    let input = PaintPreparationInput {
        view: PaintViewCapture {
            world_to_clip: field_array::<16>(&value, "world_to_clip")?,
            viewport: field_array::<4>(&value, "viewport")?,
        },
        screen: field_array::<2>(&value, "screen")?,
        screen_radius: field_f64(&value, "screen_radius")?,
        pressure: field_f64(&value, "pressure")?,
        radius_factor: field_f64(&value, "radius_factor")?,
    };
    input.view.validate()?;
    let in_contact = value
        .get("in_contact")
        .and_then(Value::as_bool)
        .ok_or("Source worker contact flag is invalid")?;
    Ok((input, in_contact))
}

fn parse_counter(source: &str) -> Result<u64, JsValue> {
    source
        .parse::<u64>()
        .ok()
        .filter(|counter| *counter != 0)
        .ok_or_else(|| JsValue::from_str("Source worker identity is invalid"))
}

fn fail(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// One worker-owned Document and Source claim. `apply` is synchronous only
/// within the dedicated Worker; the caller sends each input as an ordered
/// browser message and transfers the returned sparse packet to the studio.
#[wasm_bindgen]
pub struct BrowserSourceWorkerStroke {
    transaction: Option<SourceWorkerTransaction>,
    initial_mesh: Mesh,
    // One constructor snapshot is updated only inside certified selected
    // chunks. A held dab never clones or validates the complete mesh.
    last_mesh: Mesh,
    last_display_normals: Option<Vec<glam::Vec3>>,
    subtool_id: u32,
    failed: bool,
    accepted_dabs: u64,
    last_geometry_ms: f64,
    last_delta_encode_ms: f64,
    last_snapshot_copy_ms: f64,
}

#[wasm_bindgen]
impl BrowserSourceWorkerStroke {
    #[wasm_bindgen(constructor)]
    pub fn new(
        project: &str,
        options_json: &str,
        epoch: &str,
        warm_seed: &[u8],
    ) -> Result<Self, JsValue> {
        if project.len() > MAX_WORKER_PROJECT_BYTES || options_json.len() > 2048 {
            return Err(JsValue::from_str(
                "Source worker seed exceeds its input limit",
            ));
        }
        let epoch = parse_counter(epoch)?;
        let options: SourceGeometryOptions = serde_json::from_str(options_json).map_err(fail)?;
        options.validate().map_err(fail)?;
        let mut document = Document::from_json(project).map_err(fail)?;
        document
            .install_source_worker_warm_seed(warm_seed)
            .map_err(fail)?;
        let subtool_id = document
            .active_subtool_id()
            .ok_or_else(|| JsValue::from_str("Source worker seed has no active part"))?;
        let initial_mesh = document
            .active_mesh()
            .ok_or_else(|| JsValue::from_str("Source worker seed has no active mesh"))?
            .try_snapshot()
            .map_err(fail)?;
        let last_mesh = initial_mesh.try_snapshot().map_err(fail)?;
        let last_display_normals = if document.active_object_metadata().is_some() {
            Some(match document.source_geometry_state(subtool_id) {
                Some(state) => {
                    if !state.matches_mesh(&initial_mesh) {
                        return Err(JsValue::from_str(
                            "Source worker display-normal seed is stale",
                        ));
                    }
                    copy_display_normals(state.vertex_normals())?
                }
                None => SourceNormalField::compute(
                    &initial_mesh,
                    SourceNormalPolicy::StaticOriginalFaces,
                )
                .map_err(fail)
                .and_then(|field| copy_display_normals(field.vertex_normals()))?,
            })
        } else {
            None
        };
        let transaction = SourceWorkerTransaction::begin(document, options, epoch)
            .map_err(|(_, error)| fail(format!("{error:?}")))?;
        Ok(Self {
            transaction: Some(transaction),
            initial_mesh,
            last_mesh,
            last_display_normals,
            subtool_id,
            failed: false,
            accepted_dabs: 0,
            last_geometry_ms: 0.0,
            last_delta_encode_ms: 0.0,
            last_snapshot_copy_ms: 0.0,
        })
    }

    pub fn apply(
        &mut self,
        epoch: &str,
        sequence: &str,
        input_json: &str,
    ) -> Result<Vec<u8>, JsValue> {
        if self.failed {
            return Err(JsValue::from_str(
                "Source worker stroke was already rejected",
            ));
        }
        let epoch = parse_counter(epoch)?;
        let sequence = parse_counter(sequence)?;
        let (input, in_contact) = input_from_json(input_json).map_err(fail)?;
        let transaction = self
            .transaction
            .as_mut()
            .ok_or_else(|| JsValue::from_str("Source worker stroke is closed"))?;
        let predecessor = transaction
            .document()
            .source_geometry_state(self.subtool_id);
        let started = web_time::Instant::now();
        let report = match transaction.apply_input(
            epoch,
            sequence,
            input,
            None,
            in_contact,
            &SourceGeometryControls::default(),
            SourceGeometryAlphaTracks::default(),
        ) {
            Ok(report) => report,
            Err(error) => {
                self.failed = true;
                return Err(fail(format!("{error:?}")));
            }
        };
        self.last_geometry_ms = started.elapsed().as_secs_f64() * 1_000.0;
        let current_state = transaction
            .document()
            .source_geometry_state(self.subtool_id);
        if self.last_display_normals.is_some()
            && current_state.is_none()
            && report.combined.changed_vertices > 0
        {
            self.failed = true;
            return Err(JsValue::from_str(
                "Source worker display normals disappeared",
            ));
        }
        let chunks = match (predecessor.as_ref(), current_state.as_ref()) {
            (Some(before), Some(after)) => after.render_selected_chunks_from(before),
            (None, Some(after)) => after.render_selected_chunks_after_new_worker_state(),
            _ => None,
        };
        if chunks.is_none() && report.combined.changed_vertices > 0 {
            self.failed = true;
            return Err(JsValue::from_str(
                "Source worker changed geometry without a certified sparse row envelope",
            ));
        }
        let current = transaction
            .document()
            .active_mesh()
            .ok_or_else(|| JsValue::from_str("Source worker active mesh disappeared"))?;
        if current.positions.len() != self.last_mesh.positions.len()
            || current.normals.len() != self.last_mesh.normals.len()
        {
            self.failed = true;
            return Err(JsValue::from_str(
                "Source worker mesh shape changed during a dab",
            ));
        }
        let started = web_time::Instant::now();
        let mut rows = Vec::new();
        if let Some(chunks) = chunks {
            for (word_index, word) in chunks.into_iter().enumerate() {
                for bit in 0..64 {
                    if word & (1_u64 << bit) == 0 {
                        continue;
                    }
                    let start = (word_index * 64 + bit) * SOURCE_GEOMETRY_CHUNK_ROWS;
                    let end = (start + SOURCE_GEOMETRY_CHUNK_ROWS).min(current.positions.len());
                    rows.try_reserve(end - start).map_err(|_| {
                        self.failed = true;
                        JsValue::from_str("Cannot reserve Source worker sparse preview rows")
                    })?;
                    for index in start..end {
                        let old_position = self.last_mesh.positions[index].to_array();
                        let old_normal = self.last_mesh.normals[index].to_array();
                        let new_position = current.positions[index].to_array();
                        let new_normal = current.normals[index].to_array();
                        let display_normals = self.last_display_normals.as_ref().map(|last| {
                            (
                                last[index].to_array(),
                                current_state
                                    .as_ref()
                                    .map_or(last[index], |state| state.vertex_normals()[index])
                                    .to_array(),
                            )
                        });
                        if old_position.map(f32::to_bits) != new_position.map(f32::to_bits)
                            || old_normal.map(f32::to_bits) != new_normal.map(f32::to_bits)
                            || display_normals.is_some_and(|(before, after)| {
                                before.map(f32::to_bits) != after.map(f32::to_bits)
                            })
                        {
                            rows.push(SourceWorkerPreviewRow {
                                index: index as u32,
                                before_position: old_position,
                                before_normal: old_normal,
                                after_position: new_position,
                                after_normal: new_normal,
                                display_normals,
                            });
                        }
                    }
                }
            }
        }
        let delta = encode_source_worker_sparse_preview_rows_mode(
            epoch,
            sequence,
            self.subtool_id,
            current.positions.len(),
            &rows,
            self.last_display_normals.is_some(),
        )
        .map_err(|error| {
            self.failed = true;
            fail(error)
        })?;
        self.last_delta_encode_ms = started.elapsed().as_secs_f64() * 1_000.0;
        let started = web_time::Instant::now();
        for row in &rows {
            let index = row.index as usize;
            self.last_mesh.positions[index] = row.after_position.into();
            self.last_mesh.normals[index] = row.after_normal.into();
            if let (Some(display), Some((_, after))) =
                (&mut self.last_display_normals, row.display_normals)
            {
                display[index] = after.into();
            }
        }
        self.last_snapshot_copy_ms = started.elapsed().as_secs_f64() * 1_000.0;
        self.accepted_dabs += 1;
        Ok(delta)
    }

    pub fn timing_json(&self) -> String {
        serde_json::json!({
            "accepted_dabs":self.accepted_dabs,
            "geometry_ms":self.last_geometry_ms,
            "delta_encode_ms":self.last_delta_encode_ms,
            "snapshot_copy_ms":self.last_snapshot_copy_ms,
        })
        .to_string()
    }

    /// The completed stroke is still private to this realm. The output must
    /// carry a validated warm retained-state packet before the main studio can
    /// commit it; geometry-only publication is intentionally unavailable.
    pub fn finish(
        &mut self,
        epoch: &str,
        sequence: &str,
    ) -> Result<BrowserSourceWorkerFinish, JsValue> {
        if self.failed {
            return Err(JsValue::from_str("Source worker stroke was rejected"));
        }
        let epoch = parse_counter(epoch)?;
        let sequence = parse_counter(sequence)?;
        let transaction = self
            .transaction
            .take()
            .ok_or_else(|| JsValue::from_str("Source worker stroke is closed"))?;
        let document = match transaction.finish(epoch, sequence) {
            Ok(document) => document,
            Err((transaction, error)) => {
                self.transaction = Some(transaction);
                return Err(fail(format!("{error:?}")));
            }
        };
        let current = document
            .active_mesh()
            .ok_or_else(|| JsValue::from_str("Source worker release has no active mesh"))?;
        let delta = encode_source_worker_mesh_delta(&self.initial_mesh, current, self.subtool_id)
            .map_err(fail)?;
        let retained = match document.source_geometry_state(self.subtool_id) {
            Some(state) => {
                obrush_core::encode_source_worker_retained_warm(&state, current, self.subtool_id)
                    .map_err(fail)?
            }
            None if delta[16..20] == [0; 4] => Vec::new(),
            None => {
                return Err(JsValue::from_str(
                    "Source worker changed geometry without retained state",
                ));
            }
        };
        Ok(BrowserSourceWorkerFinish {
            delta: Some(delta),
            retained: Some(retained),
        })
    }
}

#[wasm_bindgen]
pub struct BrowserSourceWorkerFinish {
    delta: Option<Vec<u8>>,
    retained: Option<Vec<u8>>,
}

#[wasm_bindgen]
impl BrowserSourceWorkerFinish {
    pub fn take_delta(&mut self) -> Vec<u8> {
        self.delta.take().unwrap_or_default()
    }
    pub fn take_retained(&mut self) -> Vec<u8> {
        self.retained.take().unwrap_or_default()
    }
}
