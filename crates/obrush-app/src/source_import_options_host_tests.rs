//! Twelve preregistered host contract preparations. No display/GUI/GPU is run.
//! H01/H08 normal checkbox tracking, H11 actual GPU/framing, and H12 real
//! native/WASM targets still need their separately frozen consumer checks.
use super::*;
use obrush_core::{Mesh, SourceNormalField, SourceNormalPolicy};
const TETRA: &[u8] =
    include_bytes!("../../obrush-core/tests/fixtures/source-import-options/tetra-13-v3-le.sgl");
const TRANSFER: &[u8] = include_bytes!(
    "../../obrush-core/tests/fixtures/source-import-options/transfer-scalar-v3-le.sgl"
);
const ZERO: &[u8] =
    include_bytes!("../../obrush-core/tests/fixtures/source-import-options/empty-v3-le.sgl");
fn captured(options: SourceImportOptions) -> FileRequest {
    let mut state = SourceSglImportState {
        options,
        ..Default::default()
    };
    state
        .begin_captured(11, SglByteOrder::LittleEndian)
        .unwrap()
}
fn receiving() -> Document {
    let mut d = Document::new(Mesh::sphere(0));
    d.add_subtool(Mesh::sphere(0), "Kept redo").unwrap();
    assert!(d.undo());
    d
}
fn checked(request: FileRequest, data: &[u8]) -> PreparedSourceSgl {
    prepare_request(
        request,
        "Independent.sgl",
        data,
        SglByteOrder::LittleEndian,
        &ViewportStyle::default(),
    )
    .unwrap()
}
#[test]
fn h01_default_controls_guard32() {
    let state = SourceSglImportState::default();
    assert!(!state.options.scale_center && state.options.vertex_srgb);
    let original = prepare(
        "Input.sgl",
        TRANSFER,
        ByteOrderPolicy::ReferenceNative,
        SglByteOrder::LittleEndian,
        &ViewportStyle::default(),
    )
    .unwrap();
    let current = checked(captured(state.options), TRANSFER);
    assert_eq!(
        serde_json::to_vec(&original.staged).unwrap(),
        serde_json::to_vec(&current.staged).unwrap()
    );
    assert!(current.options_report.unwrap().normalization.is_none());
    assert_eq!(
        state.metrics()["options"],
        serde_json::json!({"scale_center":false,"vertex_srgb":true})
    );
}
#[test]
fn h02_captured_options_order_before_read() {
    let mut state = SourceSglImportState {
        options: SourceImportOptions {
            scale_center: true,
            vertex_srgb: false,
        },
        policy: ByteOrderPolicy::BigEndian,
        ..Default::default()
    };
    let request = state
        .begin_captured(11, SglByteOrder::LittleEndian)
        .unwrap();
    state.options = SourceImportOptions::default();
    state.policy = ByteOrderPolicy::LittleEndian;
    let accepted = state.accept_request(request.id, 11).unwrap();
    assert_eq!(accepted.native_order, Some(SglByteOrder::LittleEndian));
    assert!(accepted.options.scale_center && !accepted.options.vertex_srgb);
    assert_eq!(accepted.policy, ByteOrderPolicy::BigEndian);
    let be =
        include_bytes!("../../obrush-core/tests/fixtures/source-import-options/tetra-13-v0-be.sgl");
    let p = checked(accepted, be);
    assert_eq!(p.byte_order, SglByteOrder::BigEndian);
    assert_eq!(
        p.staged.objects[0].affine.local_to_world[13].to_bits(),
        0xc1762762
    );
}
#[test]
fn h03_newer_request_preserved() {
    let mut state = SourceSglImportState::default();
    let old = state
        .begin_captured(11, SglByteOrder::LittleEndian)
        .unwrap();
    state.options.scale_center = true;
    let new = state.begin_captured(11, SglByteOrder::BigEndian).unwrap();
    assert!(state.accept_request(old.id, 11).is_err());
    state.cancel(old.id);
    assert_eq!(state.pending.unwrap().id, new.id);
    assert!(state.pending.unwrap().options.scale_center);
    assert_eq!(
        state.pending.unwrap().native_order,
        Some(SglByteOrder::BigEndian)
    );
    state.cancel(new.id);
    assert!(state.pending.is_none());
}
#[test]
fn h04_stale_revision_atomic() {
    let d = receiving();
    let before = d.to_json().unwrap();
    let history = (d.can_undo(), d.can_redo());
    let mut state = SourceSglImportState::default();
    let request = state
        .begin_captured(11, SglByteOrder::LittleEndian)
        .unwrap();
    let camera = Camera {
        fov_degrees: 61.,
        ..Default::default()
    };
    let projection = camera.view_projection(1.25).to_cols_array();
    assert!(state.accept_request(request.id, 12).is_err());
    assert!(state.pending.is_none());
    assert!(
        state.report.is_none()
            && state.options_report.is_none()
            && state.display_override.is_none()
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert_eq!((d.can_undo(), d.can_redo()), history);
    assert_eq!(camera.view_projection(1.25).to_cols_array(), projection);
}
#[test]
fn h05_zero_modern_header_display8() {
    let mut d = receiving();
    let before = d.to_json().unwrap();
    let mut p = checked(
        captured(SourceImportOptions {
            scale_center: true,
            vertex_srgb: false,
        }),
        ZERO,
    );
    let opts = p.options_report.take().unwrap();
    assert!(opts.normalization.is_none());
    assert_eq!(opts.options_storage_bytes_bound, 0);
    let (r, _) = install(&mut d, p).unwrap();
    let mut state = SourceSglImportState::default();
    let mut style = ViewportStyle::default();
    let mut camera = Camera::default();
    state.apply_header_and_display(&r.input_header, &mut camera, &mut style);
    assert!(!r.document_changed);
    assert!(camera.orthographic);
    assert_eq!(camera.fov_degrees.to_bits(), 45.0_f32.to_bits());
    assert!(style.source_display.show_symmetry_line);
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn h06_stroke_boundary_rejection() {
    let mut d = receiving();
    d.begin_stroke();
    let before = d.to_json().unwrap();
    let old = captured(SourceImportOptions {
        scale_center: true,
        vertex_srgb: false,
    });
    let p = checked(old, TETRA);
    assert!(install(&mut d, p).is_err());
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    d.end_stroke();
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn h07_route_isolation() {
    let style = ViewportStyle::default();
    let mut whole = Document::empty();
    let p = crate::scene_import::prepare("Input.sgl", TETRA, false, &style).unwrap();
    crate::scene_import::install(&mut whole, p).unwrap();
    assert_eq!(
        whole
            .active_object_metadata()
            .unwrap()
            .affine
            .local_to_world[0]
            .to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(whole.active_mesh().unwrap().colors[0], [0., 0.25, 0.5]);
    let obj = b"v 0 0 0\nv 2 0 0\nv 0 1 0\nf 1 2 3\n";
    let p = crate::scene_import::prepare("Input.obj", obj, true, &style).unwrap();
    crate::scene_import::install(&mut whole, p).unwrap();
    assert_eq!(
        whole.active_mesh().unwrap().positions[1].x.to_bits(),
        2.0_f32.to_bits()
    );
    assert!(crate::scene_import::prepare("Input.sgl", TETRA, true, &style).is_err());
}
#[test]
fn h08_readonly_report_actual_controls_contract() {
    let d = receiving();
    let before = d.to_json().unwrap();
    let mut state = SourceSglImportState {
        options: SourceImportOptions {
            scale_center: true,
            vertex_srgb: false,
        },
        ..Default::default()
    };
    let request = state
        .begin_captured(11, SglByteOrder::LittleEndian)
        .unwrap();
    let metrics = state.metrics();
    assert_eq!(metrics["pending"]["id"], request.id);
    assert_eq!(
        metrics["pending"]["options"],
        serde_json::json!({"scale_center":true,"vertex_srgb":false})
    );
    assert_eq!(metrics["pending"]["native_order"], "little_endian");
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    // Source-only actual UI paths are checked in preservation/binding proof;
    // a tracked response is not an executed browser checkbox.
}
#[test]
fn h09_whole_open_reset_preferences() {
    let mut state = SourceSglImportState {
        options: SourceImportOptions {
            scale_center: true,
            vertex_srgb: false,
        },
        policy: ByteOrderPolicy::BigEndian,
        ..Default::default()
    };
    let request = state
        .begin_captured(11, SglByteOrder::LittleEndian)
        .unwrap();
    let p = checked(captured(state.options), ZERO);
    state.options_report = p.options_report;
    state.last_order = Some(SglByteOrder::BigEndian);
    state.reset_after_whole_open();
    assert!(
        state.pending.is_none()
            && state.options_report.is_none()
            && state.report.is_none()
            && state.last_order.is_none()
    );
    assert!(state.accept_request(request.id, 11).is_err());
    assert_eq!(state.policy, ByteOrderPolicy::BigEndian);
    assert!(state.options.scale_center && !state.options.vertex_srgb);
}
#[test]
fn h10_appearance_and_storage_preflight() {
    let d = receiving();
    let before = d.to_json().unwrap();
    let mut unsupported = TETRA.to_vec();
    unsupported[36..40].copy_from_slice(&12_u32.to_le_bytes());
    assert!(
        prepare_request(
            captured(SourceImportOptions {
                scale_center: true,
                vertex_srgb: false
            }),
            "Unsupported.sgl",
            &unsupported,
            SglByteOrder::LittleEndian,
            &ViewportStyle::default()
        )
        .is_err()
    );
    let tiny = include_bytes!(
        "../../obrush-core/tests/fixtures/source-import-options/tiny-normalized-overlimit-v3-le.sgl"
    );
    assert!(
        prepare_request(
            captured(SourceImportOptions {
                scale_center: true,
                vertex_srgb: true
            }),
            "Tiny.sgl",
            tiny,
            SglByteOrder::LittleEndian,
            &ViewportStyle::default()
        )
        .is_err()
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
}
#[test]
fn h11_normal_saved_gpu_contract_inputs_only() {
    let mut p = checked(
        captured(SourceImportOptions {
            scale_center: true,
            vertex_srgb: false,
        }),
        TETRA,
    );
    let report = p.options_report.take().unwrap();
    let mut d = Document::empty();
    install(&mut d, p).unwrap();
    let saved = d.to_json().unwrap();
    assert_eq!(
        Document::from_json(&saved).unwrap().to_json().unwrap(),
        saved
    );
    let m = d.active_mesh().unwrap();
    let normal = SourceNormalField::compute(m, SourceNormalPolicy::StaticOriginalFaces).unwrap();
    assert_eq!(
        normal
            .vertex_normals()
            .iter()
            .map(|v| v.to_array())
            .collect::<Vec<_>>(),
        vec![
            [-16., -12., -4.],
            [16., 0., 0.],
            [0., 12., 0.],
            [0., 0., 4.]
        ]
    );
    assert_eq!(
        d.active_object_metadata()
            .unwrap()
            .affine
            .local_to_world
            .map(f32::to_bits),
        report.object_reports[0].matrix_words
    );
    assert_eq!(
        d.active_object_metadata()
            .unwrap()
            .origin
            .as_ref()
            .unwrap()
            .object
            .raw
            .colors
            .as_ref()
            .unwrap()[0],
        [0., 0.25, 0.5]
    );
    // These are the CPU inputs for actual GPU/framing oracle, not that oracle.
}
#[test]
fn h12_runtime_order_mismatch_rejected_before_prepare() {
    let request = captured(SourceImportOptions {
        scale_center: true,
        vertex_srgb: false,
    });
    let d = receiving();
    let before = d.to_json().unwrap();
    assert!(
        prepare_request(
            request,
            "Input.sgl",
            TETRA,
            SglByteOrder::BigEndian,
            &ViewportStyle::default()
        )
        .is_err()
    );
    assert_eq!(d.to_json().unwrap(), before);
    assert!(d.can_redo());
    // Real strict native/WASM/consumer checks remain pending; no target is run.
}
