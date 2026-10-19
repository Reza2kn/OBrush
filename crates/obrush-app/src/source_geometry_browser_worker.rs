//! Display-free WASM owner for ordered ordinary Source geometry messages.
//!
//! This is deliberately separate from `WebHandle`: a Worker realm must not
//! construct an eframe runner, GPU device or second browser document UI.

use obrush_core::{
    Document, Mesh, PaintPreparationInput, PaintViewCapture, SourceGeometryAlphaTracks,
    SourceGeometryControls, SourceGeometryOptions, SourceWorkerTransaction,
    encode_source_worker_mesh_delta,
};
use serde_json::Value;
use wasm_bindgen::prelude::*;

const MAX_WORKER_INPUT_BYTES: usize = 8 * 1024;
const MAX_WORKER_PROJECT_BYTES: usize = 512 * 1024 * 1024;

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
    last_mesh: Mesh,
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
    pub fn new(project: &str, options_json: &str, epoch: &str) -> Result<Self, JsValue> {
        if project.len() > MAX_WORKER_PROJECT_BYTES || options_json.len() > 2048 {
            return Err(JsValue::from_str(
                "Source worker seed exceeds its input limit",
            ));
        }
        let epoch = parse_counter(epoch)?;
        let options: SourceGeometryOptions = serde_json::from_str(options_json).map_err(fail)?;
        options.validate().map_err(fail)?;
        let document = Document::from_json(project).map_err(fail)?;
        if document.has_source_geometry_normal_state() {
            return Err(JsValue::from_str("Source worker seed is not cold"));
        }
        let subtool_id = document
            .active_subtool_id()
            .ok_or_else(|| JsValue::from_str("Source worker seed has no active part"))?;
        let initial_mesh = document
            .active_mesh()
            .ok_or_else(|| JsValue::from_str("Source worker seed has no active mesh"))?
            .try_snapshot()
            .map_err(fail)?;
        let last_mesh = initial_mesh.try_snapshot().map_err(fail)?;
        let transaction = SourceWorkerTransaction::begin(document, options, epoch)
            .map_err(|(_, error)| fail(format!("{error:?}")))?;
        Ok(Self {
            transaction: Some(transaction),
            initial_mesh,
            last_mesh,
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
        let started = web_time::Instant::now();
        if let Err(error) = transaction.apply_input(
            epoch,
            sequence,
            input,
            None,
            in_contact,
            &SourceGeometryControls::default(),
            SourceGeometryAlphaTracks::default(),
        ) {
            self.failed = true;
            return Err(fail(format!("{error:?}")));
        }
        self.last_geometry_ms = started.elapsed().as_secs_f64() * 1_000.0;
        let current = transaction
            .document()
            .active_mesh()
            .ok_or_else(|| JsValue::from_str("Source worker active mesh disappeared"))?;
        let started = web_time::Instant::now();
        let delta = encode_source_worker_mesh_delta(&self.last_mesh, current, self.subtool_id)
            .map_err(|error| {
                self.failed = true;
                fail(error)
            })?;
        self.last_delta_encode_ms = started.elapsed().as_secs_f64() * 1_000.0;
        let started = web_time::Instant::now();
        self.last_mesh = current.try_snapshot().map_err(|error| {
            self.failed = true;
            fail(error)
        })?;
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
