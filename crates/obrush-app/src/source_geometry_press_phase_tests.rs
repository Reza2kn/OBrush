//! Actual normal host capture and queue over independently supplied affine SGL.
use crate::dab_queue::DabQueue;
use crate::source_geometry_host::{
    self as host, SourceGeometryMode, SourceGeometryPreferences, SourceGeometrySession,
};
use crate::source_paint_host::RawPressureSample;
use obrush_core::*;
use std::sync::{Arc, Mutex};

#[test]
fn captured_normal_press_owns_two_depth_radii_then_dab_copies_primary_without_history() {
    let raw = SglScene {
        version: 1,
        byte_order: SglByteOrder::LittleEndian,
        display: None,
        camera: None,
        meshes: vec![SglMesh {
            render: None,
            center: [0.0; 3],
            matrix: [
                0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            scale: 1.0,
            positions: vec![
                [-1.0, -2.0, -2.0],
                [-1.0, 2.0, -2.0],
                [-1.0, 2.0, 2.0],
                [-1.0, -2.0, 2.0],
                [1.0, -2.0, -2.0],
                [1.0, 2.0, -2.0],
                [1.0, 2.0, 2.0],
                [1.0, -2.0, 2.0],
            ],
            colors: None,
            materials: Some(vec![[0.18, 0.08, 1.0]; 8]),
            faces: vec![[0, 1, 2, 3], [4, 5, 6, 7]],
            uv: None,
        }],
    };
    let staged = raw
        .into_staged_editable(&SglStageSettings::default())
        .unwrap();
    let mut doc = Document::empty();
    doc.install_staged_sgl_scene(staged).unwrap();
    let before = doc.to_json().unwrap();
    let history = (doc.can_undo(), doc.can_redo());
    let matrix = glam::DMat4::perspective_rh_gl(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 100.0)
        * glam::DMat4::from_translation(glam::DVec3::new(0.0, 0.0, -10.0));
    let input = host::SourceGeometryInput {
        view: PaintViewCapture {
            world_to_clip: matrix.to_cols_array(),
            viewport: [0.0, 0.0, 100.0, 100.0],
        },
        screen: [50.0, 50.0],
        raw: RawPressureSample::default(),
        in_contact: true,
    };
    let mut prefs = SourceGeometryPreferences {
        mode: SourceGeometryMode::SourceOrdinary,
        radius: 10.0,
        radius_factor: 0.0,
        ..Default::default()
    };
    prefs.options.tool = SourceGeometryTool::Brush {
        clay: false,
        accumulate: false,
    };
    let settings = BrushSettings {
        kind: BrushKind::Draw,
        symmetry_x: true,
        adaptive_detail: false,
        alpha: None,
        ..Default::default()
    };
    let tracks: host::SharedSourceGeometryAlphaTracks = Arc::new(Mutex::new(Default::default()));
    let session = SourceGeometrySession::begin_with_controls(
        &mut doc,
        &settings,
        false,
        prefs,
        input,
        tracks.clone(),
    )
    .unwrap();
    let press = tracks.lock().unwrap().snapshot();
    assert!((press.primary.side * press.primary.side * 2.0 - 3.24).abs() <= 1e-12);
    assert!((press.mirror.side * press.mirror.side * 2.0 - 4.84).abs() <= 1e-12);
    assert!(
        (press.primary.origin[0] + 1.0).abs() <= 1e-12
            && (press.mirror.origin[0] - 1.0).abs() <= 1e-12
    );
    let mut queue = DabQueue::default();
    queue.source_geometry_dab(session.clone(), input);
    let progress = queue.release_source_geometry(&mut doc);
    assert!(progress.error.is_none());
    assert!(!progress.changed);
    assert_eq!(progress.dabs, 1);
    assert!(queue.is_empty());
    let dab = tracks.lock().unwrap().snapshot();
    assert_eq!(dab.primary.side.to_bits(), dab.mirror.side.to_bits());
    assert!((dab.mirror.side * dab.mirror.side * 2.0 - 3.24).abs() <= 1e-12);
    session.lock().unwrap().finish(&mut doc).unwrap();
    assert_eq!(doc.to_json().unwrap(), before);
    assert_eq!((doc.can_undo(), doc.can_redo()), history);
    assert_eq!(doc.source_geometry_retained_bytes(), 0);
    assert!(tracks.lock().unwrap().active_claim().is_none());
}
