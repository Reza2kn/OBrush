use obrush_core::{
    Document, MaskConvention, Mesh, ObjectAffineF32V1, SceneObjectMetadata, SglPickingPolicy,
};

fn independent_triangle() -> Mesh {
    let mut mesh = Mesh::from_obj("v 0 0 0\nv 2 0 0\nv 0 4 0\nf 1 2 3\n").unwrap();
    mesh.mask_convention = MaskConvention::UnprotectedF32V1;
    mesh
}

#[test]
fn view_only_source_guide_can_seed_an_imported_protected_mask_mesh() {
    let mut mesh = independent_triangle();
    let source_bounds = obrush_core::SourceGeometryNormalState::seed(&mesh)
        .unwrap()
        .root_loose();
    mesh.mask_convention = MaskConvention::ProtectedF32V1;
    assert!(obrush_core::SourceGeometryNormalState::seed(&mesh).is_err());
    let guide = obrush_core::SourceGeometryNormalState::seed_for_guide(&mesh).unwrap();
    assert_eq!(guide.root_loose(), source_bounds);
    assert_eq!(mesh.mask_convention, MaskConvention::ProtectedF32V1);
}

fn metadata(mesh: &Mesh) -> SceneObjectMetadata {
    SceneObjectMetadata {
        source_center_initialized: true,
        symmetry_offset: 0.0,
        affine: ObjectAffineF32V1::from_local_points(
            [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
            &mesh.positions,
        )
        .unwrap(),
        render: None,
        picking_policy: SglPickingPolicy::SourceScaleRowV1,
        origin: None,
    }
}

#[test]
fn source_mirror_plane_uses_normalized_offset_from_the_live_source_tree() {
    let mesh = independent_triangle();
    let document = Document::new(mesh);
    let center = document.source_geometry_interaction_center().unwrap();
    let half = document
        .source_geometry_mirror_plane(center, 0.5, [1., 0., 0.])
        .unwrap();
    let full = document
        .source_geometry_mirror_plane(center, 1.0, [1., 0., 0.])
        .unwrap();
    assert!(full.origin[0] > half.origin[0]);
    assert!(
        (f64::from(half.origin[0] - center[0]) * 2.0 - f64::from(full.origin[0] - center[0])).abs()
            < 1e-6
    );
    assert_eq!(half.origin[1..], center[1..]);
}

#[test]
fn per_mesh_offset_survives_project_and_undo_without_changing_peer() {
    let mesh = independent_triangle();
    let mut document = Document::empty();
    let first = document
        .add_subtool_with_metadata(mesh.snapshot(), "First", false, metadata(&mesh))
        .unwrap();
    let second = document
        .add_subtool_with_metadata(mesh.snapshot(), "Second", false, metadata(&mesh))
        .unwrap();
    assert_eq!(document.active_subtool_id(), Some(second));
    let before = document.to_json().unwrap();
    assert!(document.set_active_source_symmetry_offset(0.375).unwrap());
    assert!(!document.set_active_source_symmetry_offset(0.375).unwrap());
    assert_eq!(
        document.active_object_metadata().unwrap().symmetry_offset,
        0.375
    );
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert_eq!(
        document.active_object_metadata().unwrap().symmetry_offset,
        0.375
    );
    document.select_subtools(&[first], Some(first)).unwrap();
    assert_eq!(
        document.active_object_metadata().unwrap().symmetry_offset,
        0.0
    );
    document.select_subtools(&[second], Some(second)).unwrap();
    let saved = document.to_json().unwrap();
    let restored = Document::from_json(&saved).unwrap();
    assert_eq!(
        restored.active_object_metadata().unwrap().symmetry_offset,
        0.375
    );
    assert!(document
        .set_active_source_symmetry_offset(f64::NAN)
        .is_err());
    assert_eq!(document.to_json().unwrap(), saved);
}

#[test]
fn sgl_preflight_reports_the_nonrepresentable_per_mesh_offset() {
    use obrush_core::{
        baseline_source_render_defaults, SglByteOrder, SglCameraSettings, SglDisplaySettings,
        SglExportLossPolicy, SglExportScope, SglExportSettings, SglStageSettings,
    };
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(
            include_bytes!("fixtures/independent-affine-two-mesh.sgl"),
            &SglStageSettings::default(),
        )
        .unwrap();
    let settings = SglExportSettings {
        scope: SglExportScope::AllScene,
        byte_order: SglByteOrder::LittleEndian,
        display: SglDisplaySettings {
            show_grid: true,
            show_symmetry_line: false,
            show_contour: false,
        },
        camera: SglCameraSettings {
            projection: 0,
            mode: 0,
            fov_degrees: 40.0,
            use_pivot: true,
        },
        plain_render: baseline_source_render_defaults(),
        legacy_constructor_render: baseline_source_render_defaults(),
        plain_appearance_conversion: false,
        declared_display_defaults: false,
        loss_policy: SglExportLossPolicy::AllowAndReport,
    };
    let before = document.plan_sgl_export(&settings).unwrap();
    assert_eq!(before.omitted_symmetry_offset_objects, 0);
    document.set_active_source_symmetry_offset(0.25).unwrap();
    let report = document.plan_sgl_export(&settings).unwrap();
    assert_eq!(report.omitted_symmetry_offset_objects, 1);
    assert!(report.requires_allow());
}

#[test]
fn pinned_reference_mirror_ray_reaches_the_same_authored_face_at_half_offset() {
    use obrush_core::{
        prepare_source_geometry_press_contacts, MaskConversionPolicy, PaintPreparationInput,
        PaintViewCapture, SglStageSettings,
    };
    // Pinned SculptGL 8e45daf, C255 owned-browser receipt: the active triangle
    // has loose root [1,-2,-1,4,2,1], center [2.5,0,0], and at SymOffset .5
    // its plane is X=3.8462913036346436. The fixed local near/far ray at
    // X=5.192582607269287 reflects onto face 0 at [2.5,-2/3,0]; offset zero
    // misses. Use the same independent SGL bytes and pose, with an authored
    // orthographic camera chosen to produce that precise local near/far ray.
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(
            include_bytes!("fixtures/independent-affine-two-mesh.sgl"),
            &SglStageSettings::default(),
        )
        .unwrap();
    assert_eq!(document.active_mesh().unwrap().positions.len(), 3);
    document
        .convert_active_subtool_masks(
            MaskConvention::UnprotectedF32V1,
            MaskConversionPolicy::AllowAndReport,
        )
        .unwrap();
    let metadata = document.active_object_metadata().unwrap();
    let center = metadata.affine.interaction_center;
    assert_eq!(center, [2.5, 0.0, 0.0]);
    let half = document
        .source_geometry_mirror_plane(center, 0.5, [1.0, 0.0, 0.0])
        .unwrap();
    let zero = document
        .source_geometry_mirror_plane(center, 0.0, [1.0, 0.0, 0.0])
        .unwrap();
    assert!((f64::from(half.origin[0]) - 3.8462913036346436).abs() < 1e-6);
    let ray_x = 5.192582607269287;
    let input = PaintPreparationInput {
        view: PaintViewCapture {
            world_to_clip: [
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                -0.1,
                0.0,
                -ray_x,
                2.0 / 3.0,
                0.05,
                1.0,
            ],
            viewport: [0.0, 0.0, 100.0, 100.0],
        },
        screen: [50.0, 50.0],
        screen_radius: 10.0,
        pressure: 0.5,
        radius_factor: 0.0,
    };
    let half_contacts = prepare_source_geometry_press_contacts(
        document.active_mesh().unwrap(),
        Some(&metadata.affine),
        input,
        Some(half),
    )
    .unwrap();
    let zero_contacts = prepare_source_geometry_press_contacts(
        document.active_mesh().unwrap(),
        Some(&metadata.affine),
        input,
        Some(zero),
    )
    .unwrap();
    assert!(half_contacts.passes[0].is_none());
    assert!(zero_contacts.passes[1].is_none());
    let mirrored = half_contacts.passes[1].expect("half-offset mirror must hit triangle");
    assert_eq!(mirrored.original_face_index, 0);
    assert!((mirrored.center[0] - 2.5).abs() < 1e-6);
    assert!((mirrored.center[1] + 2.0 / 3.0).abs() < 1e-6);
    assert!(mirrored.center[2].abs() < 1e-6);
}
