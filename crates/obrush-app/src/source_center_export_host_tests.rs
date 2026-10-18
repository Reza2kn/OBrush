//! Normal shared-host preparation only; native/browser trusted UI/download proof remains future.
use super::*;
use obrush_core::{
    SglExportLossPolicy, SglExportScope, SglExportSettings, SglScene,
    baseline_source_render_defaults,
};
const INPUT: &[u8] =
    include_bytes!("../../obrush-core/tests/fixtures/independent-center-export-v3-le.sgl");
fn export(d: &Document) -> obrush_core::SglExport {
    let origin = d.scene_origin().unwrap();
    d.export_sgl(&SglExportSettings {
        scope: SglExportScope::AllScene,
        byte_order: SglByteOrder::LittleEndian,
        display: origin.display.clone().unwrap(),
        camera: origin.camera.clone().unwrap(),
        plain_render: baseline_source_render_defaults(),
        legacy_constructor_render: baseline_source_render_defaults(),
        plain_appearance_conversion: false,
        declared_display_defaults: false,
        loss_policy: SglExportLossPolicy::AllowAndReport,
    })
    .unwrap()
}
#[test]
fn normal_source_options_preflight_and_current_export_carry_center_policy() {
    let mut requests = SourceSglImportState::default();
    let request = requests
        .begin_captured(7, SglByteOrder::LittleEndian)
        .unwrap();
    let p = prepare_request(
        request,
        "Independent_center.sgl",
        INPUT,
        SglByteOrder::LittleEndian,
        &ViewportStyle::default(),
    )
    .unwrap();
    assert!(p.staged.objects[0].source_center_initialized);
    assert_eq!(
        p.options_report.unwrap().initialized_centers[0].center_words,
        [0; 3]
    );
    let mut d = Document::empty();
    d.append_staged_sgl_scene(p.staged).unwrap();
    let before = d.to_json().unwrap();
    let e = export(&d);
    let current = SglScene::from_bytes(&e.bytes).unwrap();
    assert!(
        d.active_object_metadata()
            .unwrap()
            .source_center_initialized
    );
    assert_eq!(current.meshes[0].center.map(f32::to_bits), [0; 3]);
    assert_eq!(current.meshes[0].scale.to_bits(), 0x40e00000);
    assert_eq!(e.report.changed_source_centers, 1);
    assert_eq!(e.report.changed_source_scales, 0);
    assert_eq!(d.to_json().unwrap(), before);
}
#[test]
fn ordinary_whole_sgl_preflight_preserves_archived_export_center() {
    let p = crate::scene_import::prepare(
        "Independent_center.sgl",
        INPUT,
        false,
        &ViewportStyle::default(),
    )
    .unwrap();
    let crate::scene_import::PreparedImport::Whole { document, .. } = p else {
        panic!("SGL wholeOpen must staywhole")
    };
    assert!(
        !document
            .active_object_metadata()
            .unwrap()
            .source_center_initialized
    );
    let before = document.to_json().unwrap();
    let e = export(&document);
    assert_eq!(e.bytes, INPUT);
    assert_eq!(e.report.changed_source_centers, 0);
    assert_eq!(document.to_json().unwrap(), before);
}
