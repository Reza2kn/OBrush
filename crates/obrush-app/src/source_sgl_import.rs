//! Separate source Add route: native-order policy is explicit and captured;
//! complete geometry/appearance preflight precedes any document/global change.
use crate::renderer::{Camera, CameraMode, ViewportStyle, consumer_style_for_mesh};
use obrush_core::{
    Document, SglAppendReport, SglByteOrder, SglDisplaySettings, SglSceneOrigin, SglStageSettings,
    SourceImportOptions, SourceImportOptionsReport, StagedSglScene,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ByteOrderPolicy {
    #[default]
    ReferenceNative,
    LittleEndian,
    BigEndian,
}
impl ByteOrderPolicy {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::ReferenceNative => "Reference native",
            Self::LittleEndian => "Little endian",
            Self::BigEndian => "Big endian",
        }
    }
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::ReferenceNative => "native",
            Self::LittleEndian => "little",
            Self::BigEndian => "big",
        }
    }
    pub(crate) fn resolve(self, native: SglByteOrder) -> SglByteOrder {
        match self {
            Self::ReferenceNative => native,
            Self::LittleEndian => SglByteOrder::LittleEndian,
            Self::BigEndian => SglByteOrder::BigEndian,
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn native_byte_order() -> SglByteOrder {
    if cfg!(target_endian = "little") {
        SglByteOrder::LittleEndian
    } else {
        SglByteOrder::BigEndian
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct FileRequest {
    pub id: u32,
    pub revision: u64,
    pub policy: ByteOrderPolicy,
    pub options: SourceImportOptions,
    pub native_order: Option<SglByteOrder>,
}
#[derive(Debug, Default)]
pub(crate) struct SourceSglImportState {
    pub policy: ByteOrderPolicy,
    pub options: SourceImportOptions,
    serial: u32,
    pub pending: Option<FileRequest>,
    /// View-only modern header, including unimplemented grid/contour requests.
    pub display_override: Option<SglDisplaySettings>,
    pub report: Option<SglAppendReport>,
    pub last_order: Option<SglByteOrder>,
    pub options_report: Option<SourceImportOptionsReport>,
}
impl SourceSglImportState {
    pub(crate) fn begin(&mut self, revision: u64) -> Result<FileRequest, String> {
        let id = self
            .serial
            .checked_add(1)
            .ok_or("Source Add request IDs are exhausted")?;
        self.serial = id;
        let request = FileRequest {
            id,
            revision,
            policy: self.policy,
            options: self.options,
            native_order: None,
        };
        self.pending = Some(request);
        Ok(request)
    }
    /// Actual picker entry captures the runtime order before opening or reading.
    pub(crate) fn begin_captured(
        &mut self,
        revision: u64,
        native: SglByteOrder,
    ) -> Result<FileRequest, String> {
        let mut request = self.begin(revision)?;
        request.native_order = Some(native);
        self.pending = Some(request);
        Ok(request)
    }
    pub(crate) fn accept_request(&mut self, id: u32, revision: u64) -> Result<FileRequest, String> {
        let request = self
            .pending
            .ok_or("Source Add request is no longer active")?;
        if request.id != id {
            return Err("A newer Source Add request replaced this read".into());
        }
        if request.revision != revision {
            self.pending = None;
            return Err("The document changed while the Source Add file was read".into());
        }
        self.pending = None;
        Ok(request)
    }
    pub(crate) fn cancel(&mut self, id: u32) {
        if self.pending.is_some_and(|p| p.id == id) {
            self.pending = None;
        }
    }
    pub(crate) fn reset_after_whole_open(&mut self) {
        self.pending = None;
        self.display_override = None;
        self.report = None;
        self.last_order = None;
        self.options_report = None;
    }
    /// Called after complete installation preflight, before inspecting added IDs.
    /// Old headers have no globals and leave existing viewer fields untouched.
    pub(crate) fn apply_header(&mut self, header: &SglSceneOrigin, camera: &mut Camera) {
        if let Some(display) = &header.display {
            self.display_override = Some(display.clone());
        }
        if let Some(input) = &header.camera {
            camera.orthographic = input.projection == 1;
            camera.mode = match input.mode {
                0 => CameraMode::Orbit,
                1 => CameraMode::Spherical,
                _ => CameraMode::Plane,
            };
            // Source setMode(Orbit) resets Front through a timed quaternion.
            // This host sets its final pose immediately; animation equality is open.
            if input.mode == 0 {
                camera.rotation = Some(glam::Quat::IDENTITY);
                camera.yaw = 0.0;
                camera.pitch = 0.0;
            }
            camera.fov_degrees = input.fov_degrees;
            camera.boot_source_fov = None;
            camera.source_vertical_fov = true;
            camera.use_pivot = input.use_pivot;
        }
    }
    /// Display8 coupling happens before the caller decides whether the input
    /// supplied any new mesh. The original scalar method remains unchanged.
    pub(crate) fn apply_header_and_display(
        &mut self,
        header: &SglSceneOrigin,
        camera: &mut Camera,
        style: &mut ViewportStyle,
    ) {
        self.apply_header(header, camera);
        style.source_display.import_header(header.display.as_ref());
    }
    pub(crate) fn display_for_export<'a>(
        &'a self,
        document: &'a Document,
    ) -> Option<&'a SglDisplaySettings> {
        self.display_override
            .as_ref()
            .or_else(|| document.scene_origin().and_then(|o| o.display.as_ref()))
    }
    pub(crate) fn metrics(&self) -> serde_json::Value {
        serde_json::json!({"policy":self.policy.name(),"options":self.options,"pending":self.pending.map(|p|serde_json::json!({"id":p.id,"revision":p.revision,"policy":p.policy.name(),"options":p.options,"native_order":p.native_order})),"last_order":self.last_order,"report":self.report,"options_report":self.options_report,"display_requested":self.display_override,"scope":"Source Add accepts multi-file SGL/OBJ/PLY/STL selections as independently validated, undoable appends and captures options for native/browser file and startup URL. OBJ o-records retain separate parts; wider object/group syntax and V8 pow/hypot exactness remain unverified. SGL native byte order and header application remain separate."})
    }
}
pub(crate) struct PreparedSourceSgl {
    pub staged: StagedSglScene,
    pub byte_order: SglByteOrder,
    pub options_report: Option<SourceImportOptionsReport>,
}
pub(crate) fn prepare(
    name: &str,
    bytes: &[u8],
    policy: ByteOrderPolicy,
    native: SglByteOrder,
    style: &ViewportStyle,
) -> Result<PreparedSourceSgl, String> {
    if !name
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("sgl"))
    {
        return Err("Source SGL Add accepts .sgl files".into());
    }
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("SGL file exceeds the128MiB import limit".into());
    }
    let byte_order = policy.resolve(native);
    let staged = StagedSglScene::from_sgl_bytes(
        bytes,
        &SglStageSettings {
            byte_order: Some(byte_order),
            ..Default::default()
        },
    )?;
    for object in &staged.objects {
        let metadata = obrush_core::SceneObjectMetadata {
            source_center_initialized: object.source_center_initialized,
            affine: object.affine.clone(),
            render: object.render.clone(),
            picking_policy: staged.picking_policy,
            origin: Some(obrush_core::SceneObjectOrigin {
                header: staged.origin.clone(),
                object: object.origin.clone(),
            }),
        };
        consumer_style_for_mesh(Some(&metadata), &object.mesh, style)?;
    }
    Ok(PreparedSourceSgl {
        staged,
        byte_order,
        options_report: None,
    })
}
/// Captured flags transform only owned incoming staging. Original prepare stays
/// the exact default-compatible preflight used by all prior SourceAdd tests.
pub(crate) fn prepare_with_options(
    name: &str,
    bytes: &[u8],
    policy: ByteOrderPolicy,
    native: SglByteOrder,
    options: SourceImportOptions,
    style: &ViewportStyle,
) -> Result<PreparedSourceSgl, String> {
    let original = prepare(name, bytes, policy, native, style)?;
    let applied = original.staged.prepare_source_import_options(options)?;
    for object in &applied.staged.objects {
        let metadata = obrush_core::SceneObjectMetadata {
            source_center_initialized: object.source_center_initialized,
            affine: object.affine.clone(),
            render: object.render.clone(),
            picking_policy: applied.staged.picking_policy,
            origin: Some(obrush_core::SceneObjectOrigin {
                header: applied.staged.origin.clone(),
                object: object.origin.clone(),
            }),
        };
        consumer_style_for_mesh(Some(&metadata), &object.mesh, style)?;
    }
    Ok(PreparedSourceSgl {
        staged: applied.staged,
        byte_order: original.byte_order,
        options_report: Some(applied.report),
    })
}
pub(crate) fn prepare_request(
    request: FileRequest,
    name: &str,
    bytes: &[u8],
    reported_native: SglByteOrder,
    style: &ViewportStyle,
) -> Result<PreparedSourceSgl, String> {
    let native = request
        .native_order
        .ok_or("Source SGL request has no pre-read runtime byte order")?;
    if native != reported_native {
        return Err(
            "Source SGL read reported a different runtime order than its captured picker".into(),
        );
    }
    prepare_with_options(name, bytes, request.policy, native, request.options, style)
}
pub(crate) fn install(
    document: &mut Document,
    prepared: PreparedSourceSgl,
) -> Result<(SglAppendReport, SglByteOrder), String> {
    let report = document.append_staged_sgl_scene(prepared.staged)?;
    Ok((report, prepared.byte_order))
}

#[cfg(test)]
mod tests {
    use super::*;
    use obrush_core::{Mesh, SglCameraSettings};
    fn words(version: u32, order: SglByteOrder) -> Vec<u8> {
        let mut words = vec![version];
        if version >= 2 {
            words.extend([1, 1, 0, 1, 2, 57.25_f32.to_bits(), 1]);
        }
        words.push(0);
        words
            .into_iter()
            .flat_map(|word| match order {
                SglByteOrder::LittleEndian => word.to_le_bytes(),
                SglByteOrder::BigEndian => word.to_be_bytes(),
            })
            .collect()
    }
    fn doc() -> Document {
        Document::new(Mesh::sphere(0))
    }
    fn header(mode: u32) -> SglSceneOrigin {
        SglSceneOrigin {
            version: 3,
            byte_order: SglByteOrder::LittleEndian,
            display: Some(SglDisplaySettings {
                show_grid: true,
                show_symmetry_line: true,
                show_contour: false,
            }),
            camera: Some(SglCameraSettings {
                projection: 1,
                mode,
                fov_degrees: 57.25,
                use_pivot: true,
            }),
        }
    }
    #[test]
    fn source_sgl_import_native_policy_resolves_explicitly_for_each_runtime_order() {
        for order in [SglByteOrder::LittleEndian, SglByteOrder::BigEndian] {
            let prepared = prepare(
                "Empty.sgl",
                &words(0, order),
                ByteOrderPolicy::ReferenceNative,
                order,
                &ViewportStyle::default(),
            )
            .unwrap();
            assert_eq!(prepared.byte_order, order);
            assert_eq!(prepared.staged.origin.version, 0);
            assert!(prepared.staged.origin.camera.is_none());
        }
    }
    #[test]
    fn source_sgl_import_version_zero_ordinary_open_stays_rejected() {
        let b = words(0, SglByteOrder::LittleEndian);
        assert!(
            crate::scene_import::prepare("v0.sgl", &b, false, &ViewportStyle::default()).is_err()
        );
        assert!(
            prepare(
                "v0.sgl",
                &b,
                ByteOrderPolicy::ReferenceNative,
                SglByteOrder::LittleEndian,
                &ViewportStyle::default()
            )
            .is_ok()
        );
    }
    #[test]
    fn source_sgl_import_explicit_policy_can_override_runtime_order() {
        let prepared = prepare(
            "Empty.SGL",
            &words(3, SglByteOrder::BigEndian),
            ByteOrderPolicy::BigEndian,
            SglByteOrder::LittleEndian,
            &ViewportStyle::default(),
        )
        .unwrap();
        assert_eq!(prepared.byte_order, SglByteOrder::BigEndian);
        assert!(
            prepare(
                "Empty.sgl",
                &words(3, SglByteOrder::BigEndian),
                ByteOrderPolicy::ReferenceNative,
                SglByteOrder::LittleEndian,
                &ViewportStyle::default()
            )
            .is_err()
        );
    }
    #[test]
    fn source_sgl_import_modern_zero_changes_globals_but_not_document_history() {
        let mut d = doc();
        d.add_subtool(Mesh::sphere(0), "Redo target").unwrap();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let history = (d.can_undo(), d.can_redo());
        let prepared = prepare(
            "Empty.sgl",
            &words(3, SglByteOrder::LittleEndian),
            ByteOrderPolicy::ReferenceNative,
            SglByteOrder::LittleEndian,
            &ViewportStyle::default(),
        )
        .unwrap();
        let (report, order) = install(&mut d, prepared).unwrap();
        let mut state = SourceSglImportState::default();
        let mut camera = Camera::default();
        state.apply_header(&report.input_header, &mut camera);
        assert!(!report.document_changed);
        assert_eq!(order, SglByteOrder::LittleEndian);
        assert!(camera.orthographic);
        assert_eq!(camera.mode, CameraMode::Plane);
        assert_eq!(camera.fov_degrees.to_bits(), 57.25_f32.to_bits());
        assert!(camera.use_pivot);
        assert!(state.display_override.as_ref().unwrap().show_symmetry_line);
        assert_eq!(d.to_json().unwrap(), before);
        assert_eq!((d.can_undo(), d.can_redo()), history);
    }
    #[test]
    fn source_sgl_import_legacy_zero_retains_prior_globals_and_pose() {
        let mut state = SourceSglImportState::default();
        let mut camera = Camera::default();
        state.apply_header(&header(1), &mut camera);
        let projection = camera.view_projection(1.5).to_cols_array();
        let display = state.display_override.clone();
        let old = SglSceneOrigin {
            version: 0,
            byte_order: SglByteOrder::BigEndian,
            display: None,
            camera: None,
        };
        state.apply_header(&old, &mut camera);
        assert_eq!(state.display_override, display);
        assert_eq!(camera.view_projection(1.5).to_cols_array(), projection);
        assert_eq!(camera.mode, CameraMode::Spherical);
    }
    #[test]
    fn source_sgl_import_orbit_applies_final_front_pose_without_toggling() {
        let mut camera = Camera {
            rotation: Some(glam::Quat::from_rotation_y(1.2)),
            ..Camera::default()
        };
        let mut state = SourceSglImportState::default();
        state.apply_header(&header(0), &mut camera);
        assert_eq!(camera.rotation, Some(glam::Quat::IDENTITY));
        assert_eq!(camera.yaw, 0.0);
        assert_eq!(camera.pitch, 0.0);
        state.apply_header(&header(0), &mut camera);
        assert_eq!(camera.rotation, Some(glam::Quat::IDENTITY));
    }
    #[test]
    fn source_sgl_import_malformed_header_keeps_scene_camera_and_redo() {
        let mut d = doc();
        d.add_subtool(Mesh::sphere(0), "Redo").unwrap();
        assert!(d.undo());
        let before = d.to_json().unwrap();
        let camera = Camera::default();
        let matrix = camera.view_projection(1.0).to_cols_array();
        let mut bad = words(3, SglByteOrder::LittleEndian);
        bad[24..28].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        assert!(
            prepare(
                "Bad.sgl",
                &bad,
                ByteOrderPolicy::ReferenceNative,
                SglByteOrder::LittleEndian,
                &ViewportStyle::default()
            )
            .is_err()
        );
        assert_eq!(d.to_json().unwrap(), before);
        assert!(d.can_redo());
        assert_eq!(camera.view_projection(1.0).to_cols_array(), matrix);
    }
    #[test]
    fn source_sgl_import_request_captures_policy_before_later_ui_change() {
        let mut state = SourceSglImportState::default();
        let request = state.begin(12).unwrap();
        state.policy = ByteOrderPolicy::BigEndian;
        let accepted = state.accept_request(request.id, 12).unwrap();
        assert_eq!(accepted.policy, ByteOrderPolicy::ReferenceNative);
        assert!(state.pending.is_none());
    }
    #[test]
    fn source_sgl_import_stale_read_does_not_consume_newer_request() {
        let mut state = SourceSglImportState::default();
        let old = state.begin(3).unwrap();
        let current = state.begin(3).unwrap();
        assert!(state.accept_request(old.id, 3).is_err());
        assert_eq!(state.pending.unwrap().id, current.id);
        assert!(state.accept_request(current.id, 3).is_ok());
    }
    #[test]
    fn source_sgl_import_changed_doc_and_matching_cancel_invalidate_read() {
        let mut state = SourceSglImportState::default();
        let request = state.begin(3).unwrap();
        assert!(state.accept_request(request.id, 4).is_err());
        assert!(state.pending.is_none());
        let request = state.begin(4).unwrap();
        state.cancel(request.id + 1);
        assert!(state.pending.is_some());
        state.cancel(request.id);
        assert!(state.pending.is_none());
    }
    #[test]
    fn source_sgl_import_export_can_use_zero_header_without_writing_doc() {
        let d = doc();
        let before = d.to_json().unwrap();
        let mut state = SourceSglImportState::default();
        let mut camera = Camera::default();
        state.apply_header(&header(2), &mut camera);
        assert_eq!(state.display_for_export(&d), header(2).display.as_ref());
        assert_eq!(d.to_json().unwrap(), before);
        state.reset_after_whole_open();
        assert!(state.display_for_export(&d).is_none());
    }
    #[test]
    fn source_sgl_import_rejects_wrong_extension_and_short_buffer_before_install() {
        for (name, b) in [
            ("Not.obj", words(3, SglByteOrder::LittleEndian)),
            ("Short.sgl", vec![0, 0, 0]),
        ] {
            assert!(
                prepare(
                    name,
                    &b,
                    ByteOrderPolicy::ReferenceNative,
                    SglByteOrder::LittleEndian,
                    &ViewportStyle::default()
                )
                .is_err()
            );
        }
    }
}

#[cfg(test)]
mod request_limit_tests {
    use super::*;
    #[test]
    fn source_sgl_import_request_id_exhaustion_preserves_existing_request() {
        let mut state = SourceSglImportState::default();
        let request = state.begin(7).unwrap();
        state.serial = u32::MAX;
        assert!(state.begin(7).is_err());
        assert_eq!(state.serial, u32::MAX);
        assert_eq!(state.pending.unwrap().id, request.id);
    }
}

#[cfg(test)]
#[path = "source_sgl_adapter_contract.rs"]
mod adapter_contract;

#[cfg(test)]
#[path = "source_import_options_host_tests.rs"]
mod import_options_contract;

#[cfg(test)]
#[path = "source_center_export_host_tests.rs"]
mod center_export_contract;
