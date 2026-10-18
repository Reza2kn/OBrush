use obrush_core::{MaskFilterOperation, Mesh};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: sculptgl_mask_filter_fixture <independent.obj>")?;
    let bytes = std::fs::read(path)?;
    let mesh = Mesh::import_obj(&bytes)?.mesh;
    let mut cases = Vec::new();
    for operation in [MaskFilterOperation::Blur, MaskFilterOperation::Sharpen] {
        let (masks, report) = mesh.sculptgl_filtered_masks(operation)?;
        cases.push(json!({"operation":operation,"masks":masks,"report":report}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "positions":mesh.positions,
            "input_masks":mesh.masks,
            "mask_convention":mesh.mask_convention,
            "faces":mesh.triangles,
            "cases":cases
        }))?
    );
    Ok(())
}
