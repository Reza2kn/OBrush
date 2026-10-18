//! Independent adapter gates. These use the actual combined scalar/display
//! method and portable decoder/install path; no display, GPU or fake WebHandle.
use super::*;
use crate::source_camera_math::{ProjectionKind, ViewportDimensions};
use obrush_core::{Mesh, SglCameraSettings};

fn words(version: u32, display: [u32; 3]) -> Vec<u8> {
    let mut words = vec![version];
    if version >= 2 {
        words.extend([
            display[0],
            display[1],
            display[2],
            1,
            2,
            57.25_f32.to_bits(),
            1,
        ]);
    }
    words.push(0);
    words.into_iter().flat_map(u32::to_le_bytes).collect()
}
fn document() -> Document {
    Document::new(Mesh::from_obj("v -1 -1 0\nv 1 -1 0\nv 0 1 0\nf 1 2 3\n").unwrap())
}
fn receiving_with_redo() -> Document {
    let mut document = document();
    document
        .add_subtool(Mesh::sphere(0), "Existing redo target")
        .unwrap();
    assert!(document.undo());
    document
}
fn viewport() -> ViewportDimensions {
    ViewportDimensions::checked([1200, 800], glam::DVec2::new(1200.0, 800.0)).unwrap()
}
fn source_camera(document: &Document) -> Camera {
    let mut camera = Camera::default();
    camera.set_source_framing(true, viewport()).unwrap();
    camera
        .prepare_source_frame(viewport(), &document.subtool_render_snapshots())
        .unwrap();
    camera
}
fn install_zero(document: &mut Document, version: u32, style: &ViewportStyle) -> SglAppendReport {
    let prepared = prepare(
        "IndependentZero.sgl",
        &words(version, [1, 1, 0]),
        ByteOrderPolicy::LittleEndian,
        SglByteOrder::LittleEndian,
        style,
    )
    .unwrap();
    let (report, _) = install(document, prepared).unwrap();
    report
}

#[test]
fn modern_zero_combines_display_and_camera_before_noop_decision() {
    let mut document = receiving_with_redo();
    let before = document.to_json().unwrap();
    let history = (document.can_undo(), document.can_redo());
    let mut state = SourceSglImportState::default();
    let mut style = ViewportStyle::default();
    let mut camera = Camera::default();
    let report = install_zero(&mut document, 3, &style);
    state.apply_header_and_display(&report.input_header, &mut camera, &mut style);
    assert!(!report.document_changed);
    assert!(report.added_ids.is_empty());
    assert!(camera.orthographic);
    assert_eq!(camera.mode, CameraMode::Plane);
    assert_eq!(camera.fov_degrees.to_bits(), 57.25_f32.to_bits());
    assert!(camera.use_pivot);
    assert!(style.source_display.show_symmetry_line);
    let requested = state.display_for_export(&document).unwrap();
    assert!(requested.show_grid && requested.show_symmetry_line && !requested.show_contour);
    assert_eq!(document.to_json().unwrap(), before);
    assert_eq!((document.can_undo(), document.can_redo()), history);
}

#[test]
fn legacy_zero_retains_existing_display_camera_and_redo() {
    let mut document = receiving_with_redo();
    let before = document.to_json().unwrap();
    let history = (document.can_undo(), document.can_redo());
    let mut state = SourceSglImportState::default();
    let mut style = ViewportStyle::default();
    let mut camera = Camera::default();
    let modern = install_zero(&mut document, 3, &style);
    state.apply_header_and_display(&modern.input_header, &mut camera, &mut style);
    let projection = camera
        .view_projection(1.5)
        .to_cols_array()
        .map(f32::to_bits);
    let display = state.display_override.clone();
    let old = install_zero(&mut document, 0, &style);
    state.apply_header_and_display(&old.input_header, &mut camera, &mut style);
    assert!(!old.document_changed);
    assert!(old.input_header.camera.is_none() && old.input_header.display.is_none());
    assert_eq!(state.display_override, display);
    assert!(style.source_display.show_symmetry_line);
    assert_eq!(
        camera
            .view_projection(1.5)
            .to_cols_array()
            .map(f32::to_bits),
        projection
    );
    assert_eq!(document.to_json().unwrap(), before);
    assert_eq!((document.can_undo(), document.can_redo()), history);
}

#[test]
fn source_framing_zero_publishes_new_projection_without_fitting_or_doc_edit() {
    let mut document = receiving_with_redo();
    let before = document.to_json().unwrap();
    let mut camera = source_camera(&document);
    let zoom = camera.source_projection_zoom().unwrap().to_bits();
    let target = camera.target.to_array().map(f32::to_bits);
    let previous = camera.source_frame().unwrap().projection64;
    let mut state = SourceSglImportState::default();
    let mut style = ViewportStyle::default();
    let report = install_zero(&mut document, 3, &style);
    state.apply_header_and_display(&report.input_header, &mut camera, &mut style);
    camera
        .prepare_source_frame(viewport(), &document.subtool_render_snapshots())
        .unwrap();
    let frame = camera.source_frame().unwrap();
    assert_eq!(frame.projection_kind, ProjectionKind::Orthographic);
    assert_eq!(frame.vertical_fov_degrees.to_bits(), 57.25_f64.to_bits());
    assert_ne!(frame.projection64, previous);
    assert!((frame.eye64.length() - 1000.0).abs() <= 1e-8);
    assert_eq!(camera.source_projection_zoom().unwrap().to_bits(), zoom);
    assert_eq!(camera.target.to_array().map(f32::to_bits), target);
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.can_redo());
    assert!(style.source_display.show_symmetry_line);
}

#[test]
fn source_framing_legacy_zero_keeps_accepted_view_and_zoom() {
    let mut document = receiving_with_redo();
    let before = document.to_json().unwrap();
    let mut camera = source_camera(&document);
    let view = camera.source_frame().unwrap().view64;
    let projection = camera.source_frame().unwrap().projection64;
    let zoom = camera.source_projection_zoom().unwrap().to_bits();
    let mut state = SourceSglImportState::default();
    let mut style = ViewportStyle::default();
    style.source_display.show_symmetry_line = true;
    let report = install_zero(&mut document, 0, &style);
    state.apply_header_and_display(&report.input_header, &mut camera, &mut style);
    camera
        .prepare_source_frame(viewport(), &document.subtool_render_snapshots())
        .unwrap();
    assert_eq!(camera.source_frame().unwrap().view64, view);
    assert_eq!(camera.source_frame().unwrap().projection64, projection);
    assert_eq!(camera.source_projection_zoom().unwrap().to_bits(), zoom);
    assert!(style.source_display.show_symmetry_line);
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.can_redo());
}

#[test]
fn zero_override_export_uses_current_guide_toggle_and_retains_other_flags() {
    let mut document = receiving_with_redo();
    let before = document.to_json().unwrap();
    let mut state = SourceSglImportState::default();
    let mut style = ViewportStyle::default();
    let mut camera = Camera::default();
    let report = install_zero(&mut document, 3, &style);
    state.apply_header_and_display(&report.input_header, &mut camera, &mut style);
    style.source_display.show_symmetry_line = false;
    let header = style
        .source_display
        .export_header(state.display_for_export(&document));
    assert!(header.show_grid && !header.show_symmetry_line && !header.show_contour);
    assert!(state.display_override.as_ref().unwrap().show_symmetry_line);
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.can_redo());
}

#[test]
fn whole_open_reset_uses_new_archive_display_after_old_zero_override() {
    let mut document = document();
    let mut state = SourceSglImportState::default();
    let mut style = ViewportStyle::default();
    let mut camera = Camera::default();
    let report = install_zero(&mut document, 3, &style);
    state.apply_header_and_display(&report.input_header, &mut camera, &mut style);
    state.begin(10).unwrap();
    let prepared =
        crate::scene_import::prepare("NewWholeScene.sgl", &words(3, [0, 0, 1]), false, &style)
            .unwrap();
    crate::scene_import::install(&mut document, prepared).unwrap();
    state.reset_after_whole_open();
    style
        .source_display
        .import_header(document.scene_origin().and_then(|h| h.display.as_ref()));
    let current = style
        .source_display
        .export_header(state.display_for_export(&document));
    assert!(!current.show_grid && !current.show_symmetry_line && current.show_contour);
    assert!(state.pending.is_none() && state.report.is_none() && state.last_order.is_none());
    assert!(state.display_override.is_none());
    assert!(document.active_mesh().is_none());
}

#[test]
fn ordinary_camera_default_does_not_opt_into_source_framing() {
    let mut document = document();
    let mut state = SourceSglImportState::default();
    let mut style = ViewportStyle::default();
    let mut camera = Camera::default();
    let report = install_zero(&mut document, 3, &style);
    state.apply_header_and_display(&report.input_header, &mut camera, &mut style);
    assert!(!camera.source_enabled());
    assert!(camera.source_frame().is_none());
    assert!(camera.source_vertical_fov);
    assert_eq!(camera.fov_degrees.to_bits(), 57.25_f32.to_bits());
    assert!(report.input_header.camera.as_ref().is_some_and(|c| c
        == &SglCameraSettings {
            projection: 1,
            mode: 2,
            fov_degrees: 57.25,
            use_pivot: true
        }));
}

#[test]
fn failed_source_prepare_keeps_display_camera_controller_doc_and_redo() {
    let document = receiving_with_redo();
    let before = document.to_json().unwrap();
    let camera = source_camera(&document);
    let view = camera.source_frame().unwrap().view64;
    let projection = camera.source_frame().unwrap().projection64;
    let zoom = camera.source_projection_zoom().unwrap().to_bits();
    let mut style = ViewportStyle::default();
    style.source_display.show_symmetry_line = true;
    let state = SourceSglImportState::default();
    let mut malformed = words(3, [0, 0, 1]);
    malformed[20..24].copy_from_slice(&99_u32.to_le_bytes());
    assert!(
        prepare(
            "BadHeader.sgl",
            &malformed,
            ByteOrderPolicy::LittleEndian,
            SglByteOrder::LittleEndian,
            &style
        )
        .is_err()
    );
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.can_redo());
    assert!(style.source_display.show_symmetry_line);
    assert!(state.display_override.is_none());
    assert_eq!(camera.source_frame().unwrap().view64, view);
    assert_eq!(camera.source_frame().unwrap().projection64, projection);
    assert_eq!(camera.source_projection_zoom().unwrap().to_bits(), zoom);
}
