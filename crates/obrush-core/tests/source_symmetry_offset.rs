use obrush_core::{
    Document, MaskConvention, Mesh, ObjectAffineF32V1, SceneObjectMetadata, SglPickingPolicy,
};

fn independent_triangle() -> Mesh {
    let mut mesh = Mesh::from_obj("v 0 0 0\nv 2 0 0\nv 0 4 0\nf 1 2 3\n").unwrap();
    mesh.mask_convention = MaskConvention::UnprotectedF32V1;
    mesh
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
