use obrush_core::{
    baseline_source_render_defaults, Document, Mesh, SglByteOrder, SglCameraSettings,
    SglDisplaySettings, SglExportLossPolicy, SglExportScope, SglExportSettings, SglScene,
    SglStageSettings,
};

const TWO_PARTS: &[u8] = include_bytes!("fixtures/independent-affine-two-mesh.sgl");
const LEGACY_ABSENT_RENDER: &[u8] =
    include_bytes!("fixtures/independent-legacy1-absent-fields.sgl");

fn flat(document: &Document, id: u32) -> bool {
    document
        .subtool_object_metadata(id)
        .unwrap()
        .unwrap()
        .render
        .as_ref()
        .unwrap()
        .flat_shading
}

fn export_scene(document: &Document) -> SglScene {
    let baseline = baseline_source_render_defaults();
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
            plain_render: baseline.clone(),
            legacy_constructor_render: baseline,
            plain_appearance_conversion: false,
            declared_display_defaults: false,
            loss_policy: SglExportLossPolicy::AllowAndReport,
        })
        .unwrap();
    SglScene::from_bytes(&sgl.bytes).unwrap()
}

#[test]
fn selected_flat_shading_roundtrips_without_geometry_change() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(TWO_PARTS, &SglStageSettings::default())
        .unwrap();
    document.select_subtools(&[1, 2], Some(1)).unwrap();
    let before = document.to_json().unwrap();
    let positions = [1, 2].map(|id| document.subtool_mesh(id).unwrap().positions.clone());

    assert!(document.set_selected_source_flat_shading(true).unwrap());
    assert!(flat(&document, 1) && flat(&document, 2));
    let saved = document.to_json().unwrap();
    let restored = Document::from_json(&saved).unwrap();
    assert!(flat(&restored, 1) && flat(&restored, 2));
    assert!(export_scene(&document)
        .meshes
        .iter()
        .all(|mesh| mesh.render.as_ref().is_some_and(|r| r.flat_shading)));
    assert!(!document.set_selected_source_flat_shading(true).unwrap());
    assert!(document.undo());
    assert_eq!(document.to_json().unwrap(), before);
    assert!(document.redo());
    assert!(flat(&document, 1) && flat(&document, 2));
    for (index, id) in [1, 2].into_iter().enumerate() {
        assert_eq!(
            document.subtool_mesh(id).unwrap().positions,
            positions[index]
        );
    }
}

#[test]
fn selected_scope_sets_mixed_values_uniformly() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(TWO_PARTS, &SglStageSettings::default())
        .unwrap();
    document.select_subtools(&[1], Some(1)).unwrap();
    assert!(document.set_selected_source_flat_shading(true).unwrap());
    assert!(flat(&document, 1));
    assert!(!flat(&document, 2));
    document.select_subtools(&[1, 2], Some(1)).unwrap();
    assert!(document.set_selected_source_flat_shading(false).unwrap());
    assert!(!flat(&document, 1));
    assert!(!flat(&document, 2));
}

#[test]
fn mixed_plain_part_rejects_atomically() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(TWO_PARTS, &SglStageSettings::default())
        .unwrap();
    let plain = document.add_subtool(Mesh::sphere(0), "Plain").unwrap();
    document.select_subtools(&[1, plain], Some(1)).unwrap();
    let before = document.to_json().unwrap();
    assert!(document.set_selected_source_flat_shading(true).is_err());
    assert_eq!(document.to_json().unwrap(), before);
}

#[test]
fn legacy_source_without_render_record_gets_editable_flat_flag() {
    let mut document = Document::empty();
    document
        .replace_sgl_bytes(LEGACY_ABSENT_RENDER, &SglStageSettings::default())
        .unwrap();
    let id = document.active_subtool_id().unwrap();
    assert!(document.active_object_metadata().unwrap().render.is_none());
    assert!(document.set_selected_source_flat_shading(true).unwrap());
    assert!(flat(&document, id));
    assert!(document.undo());
    assert!(document.active_object_metadata().unwrap().render.is_none());
}
