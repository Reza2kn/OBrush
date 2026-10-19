use obrush_core::{
    baseline_source_render_defaults, Document, Mesh, SglByteOrder, SglCameraSettings,
    SglDisplaySettings, SglExportLossPolicy, SglExportScope, SglExportSettings, SglScene,
    SglStageSettings,
};

const TWO_PARTS: &[u8] = include_bytes!("fixtures/independent-affine-two-mesh.sgl");
const LEGACY_ABSENT_RENDER: &[u8] =
    include_bytes!("fixtures/independent-legacy1-absent-fields.sgl");

fn wireframe(document: &Document, id: u32) -> bool {
    document
        .subtool_object_metadata(id)
        .unwrap()
        .unwrap()
        .render
        .as_ref()
        .unwrap()
        .show_wireframe
}

#[test]
fn selected_wireframe_is_one_undoable_scene_edit() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(TWO_PARTS, &SglStageSettings::default())
        .unwrap();
    document.select_subtools(&[1, 2], Some(1)).unwrap();
    let before = document.to_json().unwrap();
    let positions = [1, 2].map(|id| document.subtool_mesh(id).unwrap().positions.clone());

    assert!(document.set_selected_source_wireframe(true).unwrap());
    assert!(wireframe(&document, 1) && wireframe(&document, 2));
    let saved = document.to_json().unwrap();
    let restored = Document::from_json(&saved).unwrap();
    assert!(wireframe(&restored, 1) && wireframe(&restored, 2));
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
        .all(|mesh| mesh.render.as_ref().is_some_and(|r| r.show_wireframe)));
    assert!(!document.set_selected_source_wireframe(true).unwrap());
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert!(wireframe(&document, 1) && wireframe(&document, 2));
    for (index, id) in [1, 2].into_iter().enumerate() {
        assert_eq!(
            document.subtool_mesh(id).unwrap().positions,
            positions[index]
        );
    }
}

#[test]
fn selection_scope_leaves_other_part_and_mixed_values_set_uniformly() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(TWO_PARTS, &SglStageSettings::default())
        .unwrap();
    document.select_subtools(&[1], Some(1)).unwrap();
    assert!(document.set_selected_source_wireframe(true).unwrap());
    assert!(wireframe(&document, 1));
    assert!(!wireframe(&document, 2));
    document.select_subtools(&[1, 2], Some(1)).unwrap();
    assert!(document.set_selected_source_wireframe(false).unwrap());
    assert!(!wireframe(&document, 1));
    assert!(!wireframe(&document, 2));
}

#[test]
fn mixed_source_and_plain_selection_rejects_without_partial_edit() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(TWO_PARTS, &SglStageSettings::default())
        .unwrap();
    let plain = document.add_subtool(Mesh::sphere(0), "Plain").unwrap();
    document.select_subtools(&[1, plain], Some(1)).unwrap();
    let before = document.to_json().unwrap();
    assert!(document.set_selected_source_wireframe(true).is_err());
    assert_eq!(document.to_json().unwrap(), before);
}

#[test]
fn legacy_source_without_render_record_uses_editable_baseline() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(LEGACY_ABSENT_RENDER, &SglStageSettings::default())
        .unwrap();
    let id = document.active_subtool_id().unwrap();
    assert!(document.active_object_metadata().unwrap().render.is_none());
    assert!(document.set_selected_source_wireframe(true).unwrap());
    assert!(wireframe(&document, id));
    assert!(document.undo());
    assert!(document.active_object_metadata().unwrap().render.is_none());
}
