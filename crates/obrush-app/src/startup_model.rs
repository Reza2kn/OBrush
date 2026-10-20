//! Bounded request ownership for normal startup model reads. No geometry math,
//! document installation or GPU. A completed read is accepted only against its
//! captured request identity/revision; transport errors never mutate a document.
use crate::startup_options::SourceStartupSettings;
use obrush_core::SglByteOrder;

pub(crate) const MAX_MODEL_BYTES: usize = 128 * 1024 * 1024;
#[derive(Clone, Debug)]
pub(crate) struct ModelRequest {
    pub id: u32,
    pub revision: u64,
    pub url: String,
    pub filename: String,
    pub options: SourceStartupSettings,
    pub native_order: SglByteOrder,
    pub sgl_request_id: Option<u32>,
}
#[derive(Default)]
pub(crate) struct ModelRequests {
    next_id: u32,
    pending: Option<ModelRequest>,
    pub last_result: Option<String>,
}
impl ModelRequests {
    pub fn begin(
        &mut self,
        revision: u64,
        url: &str,
        options: SourceStartupSettings,
        native_order: SglByteOrder,
    ) -> Result<ModelRequest, String> {
        let filename = model_filename(url)?;
        let id = self
            .next_id
            .checked_add(1)
            .ok_or("Startup model request IDs exhausted")?;
        let request = ModelRequest {
            id,
            revision,
            url: url.into(),
            filename,
            options,
            native_order,
            sgl_request_id: None,
        };
        self.next_id = id;
        self.pending = Some(request.clone());
        self.last_result = None;
        Ok(request)
    }
    pub fn attach_sgl(&mut self, id: u32, sgl_id: u32) -> Result<(), String> {
        let request = self
            .pending
            .as_mut()
            .filter(|p| p.id == id)
            .ok_or("Model request replaced")?;
        request.sgl_request_id = Some(sgl_id);
        Ok(())
    }
    pub fn accept(&mut self, id: u32, revision: u64) -> Result<ModelRequest, String> {
        let request = self
            .pending
            .as_ref()
            .ok_or("Startup model read is no longer active")?;
        if request.id != id {
            return Err("A newer model read replaced this request".into());
        }
        if request.revision != revision {
            self.pending = None;
            return Err("Document changed during model read".into());
        }
        Ok(self.pending.take().expect("Checked active request"))
    }
    pub fn cancel(&mut self) -> Option<ModelRequest> {
        self.pending.take()
    }
    pub fn pending(&self) -> Option<&ModelRequest> {
        self.pending.as_ref()
    }
    pub fn metrics(&self) -> serde_json::Value {
        serde_json::json!({"pending": self.pending.as_ref().map(|p|serde_json::json!({"id":p.id,
            "revision":p.revision,"url":"[redacted]","filename":p.filename,"native_order":p.native_order,
            "scale_center":p.options.scale_center,"sgl_request_id":p.sgl_request_id})),
            "last_result":self.last_result,"byte_limit":MAX_MODEL_BYTES})
    }
}
pub(crate) fn model_filename(url: &str) -> Result<String, String> {
    if url.is_empty()
        || url.len() > crate::startup_options::MAX_MODEL_URL_BYTES
        || url.contains('\0')
    {
        return Err("Model URL is empty or exceeds its finite input domain".into());
    }
    let path = url.split(['?', '#']).next().unwrap_or("");
    let filename = path
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or("Model URL needs a file name")?;
    let extension = filename
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(extension.as_str(), "obj" | "ply" | "stl" | "sgl") {
        return Err(
            "Startup model accepts OBJ, PLY, STL or SGL URLs with explicit extensions".into(),
        );
    }
    Ok(filename.to_owned())
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::io::Read;
    use std::process::{Child, Command, Stdio};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    type SharedChild = Arc<Mutex<Option<Child>>>;
    pub(crate) struct NativeModelJob {
        pub id: u32,
        pub receiver: mpsc::Receiver<Result<Vec<u8>, String>>,
        cancelled: Arc<AtomicBool>,
        child: SharedChild,
    }
    impl NativeModelJob {
        pub fn start(request: &ModelRequest, repaint: eframe::egui::Context) -> Self {
            let cancelled = Arc::new(AtomicBool::new(false));
            let child = Arc::new(Mutex::new(None));
            let (sender, receiver) = mpsc::channel();
            let url = request.url.clone();
            let token = cancelled.clone();
            let owned_child = child.clone();
            std::thread::spawn(move || {
                let result = read_url(&url, &token, &owned_child);
                let _ = sender.send(result);
                repaint.request_repaint();
            });
            Self {
                id: request.id,
                receiver,
                cancelled,
                child,
            }
        }
        pub fn cancel(&self) {
            self.cancelled.store(true, Ordering::SeqCst);
            if let Ok(mut child) = self.child.lock()
                && let Some(child) = child.as_mut()
            {
                let _ = child.kill();
            }
        }
    }
    impl Drop for NativeModelJob {
        fn drop(&mut self) {
            self.cancel();
        }
    }
    fn read_bounded(
        mut input: impl Read,
        cancelled: &AtomicBool,
        limit: usize,
    ) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            if cancelled.load(Ordering::SeqCst) {
                return Err("Model read cancelled".into());
            }
            let request = limit
                .saturating_sub(bytes.len())
                .saturating_add(1)
                .min(chunk.len());
            let size = input
                .read(&mut chunk[..request])
                .map_err(|e| e.to_string())?;
            if size == 0 {
                return Ok(bytes);
            }
            if size > limit.saturating_sub(bytes.len()) {
                return Err("Model exceeds128MiB".into());
            }
            bytes
                .try_reserve_exact(size)
                .map_err(|_| "Cannot allocate bounded model input")?;
            bytes.extend_from_slice(&chunk[..size]);
        }
    }
    fn read_url(url: &str, cancelled: &AtomicBool, child: &SharedChild) -> Result<Vec<u8>, String> {
        if cancelled.load(Ordering::SeqCst) {
            return Err("Model read cancelled".into());
        }
        if !url.starts_with("https://") && !url.starts_with("http://") {
            if url.contains("://") {
                return Err("Native model URL protocol must be HTTP or HTTPS".into());
            }
            let file = std::fs::File::open(url).map_err(|e| format!("Model file: {e}"))?;
            if file.metadata().map_err(|e| e.to_string())?.len() > MAX_MODEL_BYTES as u64 {
                return Err("Model exceeds128MiB".into());
            }
            return read_bounded(file, cancelled, MAX_MODEL_BYTES);
        }
        let mut process = Command::new("curl")
            .args([
                "--disable",
                "--globoff",
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--proto",
                "=http,https",
                "--proto-redir",
                "=http,https",
                "--max-redirs",
                "3",
                "--connect-timeout",
                "10",
                "--max-time",
                "30",
                "--max-filesize",
                "134217728",
                "--write-out",
                "%{stderr}\nOBRUSH_HTTP_STATUS=%{http_code}\n",
                "--url",
                url,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Native URL reads require the system curl executable: {e}"))?;
        let output = process.stdout.take().ok_or("CURL output unavailable")?;
        let error = process.stderr.take().ok_or("CURL error pipe unavailable")?;
        // Drain stderr concurrently. stdout must not block on a filled pipe,
        // and cancellation must not wait for a read lock. Curl diagnostics may
        // echo a URL containing user credentials, so retain only the bounded
        // status trailer for the HTTP 200 check and never publish raw stderr.
        let error_thread = std::thread::spawn(move || {
            let mut error = error;
            let mut tail = Vec::new();
            let mut chunk = [0u8; 4096];
            while let Ok(n) = error.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                tail.extend_from_slice(&chunk[..n]);
                if tail.len() > 256 {
                    tail.drain(..tail.len() - 256);
                }
            }
            tail
        });
        {
            let mut shared = child.lock().map_err(|_| "Model process lock poisoned")?;
            *shared = Some(process);
            if cancelled.load(Ordering::SeqCst)
                && let Some(p) = shared.as_mut()
            {
                let _ = p.kill();
            }
        }
        let read = read_bounded(output, cancelled, MAX_MODEL_BYTES);
        if (read.is_err() || cancelled.load(Ordering::SeqCst))
            && let Ok(mut p) = child.lock()
            && let Some(p) = p.as_mut()
        {
            let _ = p.kill();
        }
        // Never hold the process mutex across wait/read: the main UI cancellation
        // can kill its owned child while the worker is blocked on a pipe.
        let status = loop {
            let status = {
                let mut shared = child.lock().map_err(|_| "Model process lock poisoned")?;
                shared
                    .as_mut()
                    .ok_or("Model process lost ownership")?
                    .try_wait()
                    .map_err(|e| e.to_string())?
            };
            if let Some(status) = status {
                break status;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        child
            .lock()
            .map_err(|_| "Model process lock poisoned")?
            .take();
        let status_tail = error_thread.join().unwrap_or_default();
        let bytes = read?;
        if cancelled.load(Ordering::SeqCst) {
            return Err("Model read cancelled".into());
        }
        if !status.success() {
            return Err(format!("Model GET failed (curl exit {status})"));
        }
        if !String::from_utf8_lossy(&status_tail)
            .trim_end()
            .ends_with("OBRUSH_HTTP_STATUS=200")
        {
            return Err("Model GET requires HTTP200 and curl stderr write-out support".into());
        }
        Ok(bytes)
    }
    #[cfg(test)]
    mod bounded_tests {
        use super::*;
        #[test]
        fn native_reader_preserves_exact_bytes_and_rejects_one_over_limit() {
            let token = AtomicBool::new(false);
            let bytes = [0, 255, 0, 4, 5, 6, 7, 8];
            assert_eq!(read_bounded(&bytes[..], &token, 8).unwrap(), bytes);
            assert!(read_bounded(&[0u8; 9][..], &token, 8).is_err());
        }
        #[test]
        fn native_cancel_checked_before_read_or_allocation() {
            let token = AtomicBool::new(true);
            assert_eq!(
                read_bounded(&[][..], &token, 8).unwrap_err(),
                "Model read cancelled"
            );
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::NativeModelJob;

#[cfg(test)]
mod tests {
    use super::*;
    fn request(s: &mut ModelRequests, rev: u64) -> ModelRequest {
        s.begin(
            rev,
            "https://fixture.invalid/tetra.sgl",
            Default::default(),
            SglByteOrder::LittleEndian,
        )
        .unwrap()
    }
    #[test]
    fn request_captures_flags_before_transport() {
        let mut state = ModelRequests::default();
        let options = SourceStartupSettings {
            scale_center: true,
            ..Default::default()
        };
        let r = state
            .begin(7, "a.sgl", options, SglByteOrder::BigEndian)
            .unwrap();
        state.attach_sgl(r.id, 42).unwrap();
        let accepted = state.accept(r.id, 7).unwrap();
        assert!(accepted.options.scale_center);
        assert_eq!(accepted.native_order, SglByteOrder::BigEndian);
        assert_eq!(accepted.sgl_request_id, Some(42));
    }
    #[test]
    fn older_result_does_not_consume_newer_request() {
        let mut s = ModelRequests::default();
        let a = request(&mut s, 2);
        let b = request(&mut s, 2);
        assert!(s.accept(a.id, 2).is_err());
        assert_eq!(s.pending().unwrap().id, b.id);
    }
    #[test]
    fn revision_change_discards_and_cancel_is_idempotent() {
        let mut s = ModelRequests::default();
        let a = request(&mut s, 2);
        assert!(s.accept(a.id, 3).is_err());
        assert!(s.pending().is_none());
        assert!(s.cancel().is_none());
    }
    #[test]
    fn unsupported_extension_is_atomic() {
        let mut s = ModelRequests::default();
        let a = request(&mut s, 2);
        assert!(
            s.begin(2, "a.obrush", Default::default(), a.native_order)
                .is_err()
        );
        assert_eq!(s.pending().unwrap().id, a.id);
    }
    #[test]
    fn filename_drops_query_fragment_without_percent_decoding() {
        assert_eq!(
            model_filename("https://fixture.invalid/a%20b.obj?token=opaque#fragment").unwrap(),
            "a%20b.obj"
        );
    }
    #[test]
    fn missing_extension_and_overlong_url_rejected() {
        assert!(model_filename("https://fixture.invalid/").is_err());
        assert!(model_filename(&format!("{}.obj", "x".repeat(4097))).is_err());
    }
    #[test]
    fn request_metrics_never_publish_url_credentials() {
        let mut state = ModelRequests::default();
        let secret = "unique_token_do_not_expose_5749";
        let url = format!("https://user:{secret}@fixture.invalid/tetra.obj?token={secret}");
        state
            .begin(7, &url, Default::default(), SglByteOrder::LittleEndian)
            .unwrap();
        let metrics = state.metrics().to_string();
        assert!(!metrics.contains(secret));
        assert!(!metrics.contains("fixture.invalid"));
        assert_eq!(state.pending().unwrap().url, url);
        assert_eq!(state.metrics()["pending"]["filename"], "tetra.obj");
    }
}
