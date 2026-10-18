//! Create an independently authored posed-source browser extraction fixture.
//! Usage: cargo run -p obrush-core --example affine_extract_fixture -- OUTPUT.obrush [OUTPUT.sgl]
use obrush_core::{
    baseline_source_render_defaults, Document, SglByteOrder, SglCameraSettings, SglDisplaySettings,
    SglExportLossPolicy, SglExportScope, SglExportSettings,
};

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("Pass output .obrush path")?;
    let mut document = Document::empty();
    document.replace_sgl_bytes(
        include_bytes!("../tests/fixtures/independent-affine-two-mesh.sgl"),
        &Default::default(),
    )?;
    document.select_subtool(1)?;
    document.begin_stroke();
    let mesh = document.require_active_mesh_mut()?;
    mesh.masks[0] = mesh.mask_convention.full_value();
    document.end_stroke();
    document.select_subtool(2)?;
    document.begin_stroke();
    let mesh = document.require_active_mesh_mut()?;
    mesh.masks[0] = mesh.mask_convention.full_value();
    document.end_stroke();
    let json = document.to_json()?;
    std::fs::write(path, json).map_err(|error| error.to_string())?;
    if let Some(sgl_path) = std::env::args().nth(2) {
        let render = baseline_source_render_defaults();
        let export = document.export_sgl(&SglExportSettings {
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
            plain_render: render.clone(),
            legacy_constructor_render: render,
            plain_appearance_conversion: false,
            declared_display_defaults: false,
            loss_policy: SglExportLossPolicy::AllowAndReport,
        })?;
        std::fs::write(sgl_path, export.bytes).map_err(|error| error.to_string())?;
    }
    Ok(())
}
