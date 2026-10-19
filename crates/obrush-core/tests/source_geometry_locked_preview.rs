//! Independent Source Lock transaction controls: press, replacement, rejection.
use obrush_core::*;

fn input(screen: [f64; 2]) -> PaintPreparationInput {
    PaintPreparationInput {
        view: PaintViewCapture {
            world_to_clip: [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., -1., 0., 0., 0., 0., 1.,
            ],
            viewport: [0., 0., 1000., 1000.],
        },
        screen,
        screen_radius: 64.,
        pressure: 1.,
        radius_factor: 0.,
    }
}

fn fresh() -> (Document, SourceGeometryStroke, SourceGeometryAlphaTracks) {
    let mut mesh = Mesh::sphere(4);
    mesh.mask_convention = MaskConvention::UnprotectedF32V1;
    mesh.masks.fill(1.);
    let mut document = Document::new(mesh);
    let options = SourceGeometryOptions {
        tool: SourceGeometryTool::Brush {
            clay: false,
            accumulate: false,
        },
        intensity: 0.75,
        intensity_factor: 0.,
        prevent_folds: false,
        ..Default::default()
    };
    let stroke = document
        .begin_source_geometry_selection_history(options)
        .unwrap();
    let contacts = prepare_source_geometry_press_contacts(
        document.active_mesh().unwrap(),
        None,
        input([500., 500.]),
        None,
    )
    .unwrap();
    assert!(contacts.passes[0].is_some());
    let mut tracks = document
        .prepare_source_geometry_press_alpha(&stroke, contacts.passes, Default::default())
        .unwrap();
    tracks.primary.keep_origin = true;
    (document, stroke, tracks)
}

#[test]
fn held_previews_replace_not_accumulate_and_release_one_history_step() {
    let (mut document, mut stroke, tracks) = fresh();
    let before = document.to_json().unwrap();
    let controls = SourceGeometryControls::default();
    assert!(!document.can_undo());
    let first = document
        .replace_source_geometry_locked_preview(
            &mut stroke,
            [500., 500.],
            input([660., 500.]),
            &controls,
            tracks,
        )
        .unwrap();
    assert_eq!(first.radius_pixels, 160.);
    assert!(first.report.combined.changed_vertices > 0);
    assert!(matches!(
        stroke.options().tool,
        SourceGeometryTool::Brush {
            accumulate: true,
            ..
        }
    ));
    let first_json = document.to_json().unwrap();
    assert_ne!(first_json, before);
    let id = document.active_subtool_id().unwrap();
    let first_normals = document
        .source_geometry_state(id)
        .unwrap()
        .vertex_normals()
        .to_vec();

    let turned = document
        .replace_source_geometry_locked_preview(
            &mut stroke,
            [500., 500.],
            input([500., 340.]),
            &controls,
            tracks,
        )
        .unwrap();
    assert_eq!(turned.radius_pixels, 160.);
    assert_ne!(document.to_json().unwrap(), first_json);
    document
        .replace_source_geometry_locked_preview(
            &mut stroke,
            [500., 500.],
            input([660., 500.]),
            &controls,
            tracks,
        )
        .unwrap();
    assert_eq!(document.to_json().unwrap(), first_json);
    assert_eq!(
        document.source_geometry_state(id).unwrap().vertex_normals(),
        first_normals
    );
    assert_eq!(document.history_depths(), (0, 0));
    document.end_source_geometry_stroke(&stroke).unwrap();
    assert!(document.can_undo());
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), first_json);
}

#[test]
fn invalid_held_update_after_preview_rolls_back_and_cannot_commit() {
    let (mut document, original_stroke, tracks) = fresh();
    document
        .cancel_source_geometry_stroke(&original_stroke)
        .unwrap();
    let id = document.active_subtool_id().unwrap();
    document.set_subtool_name(id, "redo sentinel").unwrap();
    let redo_target = document.to_json().unwrap();
    assert!(document.undo());
    assert!(document.can_redo());
    let mut stroke = document
        .begin_source_geometry_selection_history(original_stroke.options())
        .unwrap();
    let before = document.to_json().unwrap();
    let controls = SourceGeometryControls::default();
    document
        .replace_source_geometry_locked_preview(
            &mut stroke,
            [500., 500.],
            input([660., 500.]),
            &controls,
            tracks,
        )
        .unwrap();
    assert_ne!(document.to_json().unwrap(), before);
    let mut invalid = input([500., 340.]);
    invalid.pressure = f64::NAN;
    assert_eq!(
        document
            .replace_source_geometry_locked_preview(
                &mut stroke,
                [500., 500.],
                invalid,
                &controls,
                tracks,
            )
            .unwrap_err(),
        SourceGeometryError::InvalidInput
    );
    assert_eq!(document.to_json().unwrap(), before);
    assert_eq!(document.source_geometry_active_claim(), None);
    assert_eq!(
        document.end_source_geometry_stroke(&stroke).unwrap_err(),
        SourceGeometryError::NoOpenStroke
    );
    assert!(!document.can_undo());
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(document.to_json().unwrap(), redo_target);
}

#[test]
fn zero_travel_restores_exact_press_surface_without_spending_history() {
    let (mut document, mut stroke, tracks) = fresh();
    let before = document.to_json().unwrap();
    let controls = SourceGeometryControls::default();
    document
        .replace_source_geometry_locked_preview(
            &mut stroke,
            [500., 500.],
            input([660., 500.]),
            &controls,
            tracks,
        )
        .unwrap();
    let restored = document
        .replace_source_geometry_locked_preview(
            &mut stroke,
            [500., 500.],
            input([500., 500.]),
            &controls,
            tracks,
        )
        .unwrap();
    assert_eq!(restored.radius_pixels, 0.);
    assert!(restored.report.combined.changed_vertices > 0);
    assert_eq!(document.to_json().unwrap(), before);
    document.end_source_geometry_stroke(&stroke).unwrap();
    assert!(!document.can_undo());
}
