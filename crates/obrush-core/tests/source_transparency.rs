use glam::DVec3;
use obrush_core::{
    baseline_source_render_defaults, Document, Mesh, SceneRay64, SglByteOrder, SglCameraSettings,
    SglDisplaySettings, SglExportLossPolicy, SglExportScope, SglExportSettings, SglPickingPolicy,
    SglScene, SglStageSettings,
};

const TWO_PARTS: &[u8] = include_bytes!("fixtures/independent-affine-two-mesh.sgl");
const LEGACY_ABSENT_RENDER: &[u8] =
    include_bytes!("fixtures/independent-legacy1-absent-fields.sgl");

fn opacity(document: &Document, id: u32) -> f32 {
    document
        .subtool_object_metadata(id)
        .unwrap()
        .unwrap()
        .render
        .as_ref()
        .unwrap()
        .opacity
}

#[test]
fn transparency_applies_to_selected_parts_and_roundtrips_one_history_step() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(TWO_PARTS, &SglStageSettings::default())
        .unwrap();
    document.select_subtools(&[1, 2], Some(1)).unwrap();
    let positions = [1, 2].map(|id| document.subtool_mesh(id).unwrap().positions.clone());

    for (percent, expected) in [(0, 1.0), (50, 0.5), (100, 0.0)] {
        document.set_selected_source_transparency(percent).unwrap();
        for id in [1, 2] {
            assert_eq!(opacity(&document, id), expected);
        }
        let restored = Document::from_json(&document.to_json().unwrap()).unwrap();
        for id in [1, 2] {
            assert_eq!(opacity(&restored, id), expected);
        }
    }
    assert!(!document.set_selected_source_transparency(100).unwrap());
    for (index, id) in [1, 2].into_iter().enumerate() {
        assert_eq!(
            document.subtool_mesh(id).unwrap().positions,
            positions[index]
        );
    }
    assert!(document.undo());
    assert_eq!(opacity(&document, 1), 0.5);
    assert_eq!(opacity(&document, 2), 0.5);
    assert!(document.redo());
    assert_eq!(opacity(&document, 1), 0.0);
    assert_eq!(opacity(&document, 2), 0.0);

    let sgl = document
        .export_sgl(&SglExportSettings {
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
        })
        .unwrap();
    assert!(SglScene::from_bytes(&sgl.bytes)
        .unwrap()
        .meshes
        .iter()
        .all(|mesh| mesh
            .render
            .as_ref()
            .is_some_and(|render| render.opacity == 0.0)));
}

#[test]
fn selection_scope_and_failure_are_atomic() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(TWO_PARTS, &SglStageSettings::default())
        .unwrap();
    let other_opacity = opacity(&document, 2);
    document.select_subtools(&[1], Some(1)).unwrap();
    assert!(document.set_selected_source_transparency(50).unwrap());
    assert_eq!(opacity(&document, 1), 0.5);
    assert_eq!(opacity(&document, 2), other_opacity);

    let before = document.to_json().unwrap();
    assert!(document.set_selected_source_transparency(101).is_err());
    assert_eq!(document.to_json().unwrap(), before);
    let plain = document.add_subtool(Mesh::sphere(0), "Plain").unwrap();
    document.select_subtools(&[1, plain], Some(1)).unwrap();
    let before = document.to_json().unwrap();
    assert!(document.set_selected_source_transparency(100).is_err());
    assert_eq!(document.to_json().unwrap(), before);
}

#[test]
fn old_source_without_render_block_acquires_editable_opacity() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(LEGACY_ABSENT_RENDER, &SglStageSettings::default())
        .unwrap();
    let id = document.active_subtool_id().unwrap();
    assert!(document.active_object_metadata().unwrap().render.is_none());
    assert!(document.set_selected_source_transparency(50).unwrap());
    assert_eq!(opacity(&document, id), 0.5);
    assert!(document.undo());
    assert!(document.active_object_metadata().unwrap().render.is_none());
}

#[test]
fn transparent_parts_keep_the_same_scene_pick_and_wire_flags() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(TWO_PARTS, &SglStageSettings::default())
        .unwrap();
    document.select_subtools(&[1, 2], Some(1)).unwrap();
    document.set_selected_source_wireframe(true).unwrap();
    let ray = SceneRay64 {
        origin: DVec3::new(2.5, -0.5, 3.0),
        direction: -DVec3::Z,
    };
    for percent in [0, 50, 100] {
        document.set_selected_source_transparency(percent).unwrap();
        assert_eq!(
            document
                .raycast_scene(ray, SglPickingPolicy::SourceScaleRowV1)
                .unwrap()
                .unwrap()
                .id,
            2
        );
        for id in [1, 2] {
            assert!(
                document
                    .subtool_object_metadata(id)
                    .unwrap()
                    .unwrap()
                    .render
                    .as_ref()
                    .unwrap()
                    .show_wireframe
            );
        }
    }
}
