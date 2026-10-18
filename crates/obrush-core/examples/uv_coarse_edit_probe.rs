//! Controlled static-level response to the same single-vertex coarse edit used
//! in the pinned SculptGL diagnostic. Not a brush or GUI latency test.
//! Usage: cargo run -p obrush-core --example uv_coarse_edit_probe -- path.obj [coarse_vertex]
use obrush_core::{Document, Mesh, StaticSubdivisionSettings, Vec3};
use serde_json::json;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("OBJ path is required")?;
    let vertex: usize = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "0".into())
        .parse()
        .map_err(|_| "Invalid coarse vertex index")?;
    let source = std::fs::read(&path).map_err(|error| error.to_string())?;
    let mut doc = Document::new(Mesh::import_obj(&source)?.mesh);
    doc.add_source_static_subdivision_level(&StaticSubdivisionSettings::default())?;
    let fine_before = doc.require_active_mesh()?.snapshot();
    doc.select_level(0)?;
    let coarse_before = doc.require_active_mesh()?.snapshot();
    let old_prediction = coarse_before
        .subdivided_static(&StaticSubdivisionSettings::default())?
        .mesh;
    let delta = Vec3::new(0.125, -0.09375, 0.0625);
    let coarse = doc.require_active_mesh_mut()?;
    let point = coarse
        .positions
        .get_mut(vertex)
        .ok_or("Coarse vertex index is out of range")?;
    *point += delta;
    coarse.recompute_normals();
    let coarse_after = doc.require_active_mesh()?.snapshot();
    let new_prediction = coarse_after
        .subdivided_static(&StaticSubdivisionSettings::default())?
        .mesh;
    doc.select_level(1)?;
    let fine_after = doc.require_active_mesh()?.snapshot();
    println!(
        "{}",
        json!({
            "input": path,
            "vertex": vertex,
            "delta": delta.to_array(),
            "fine_before": fine_before.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "fine_faces": fine_before.surface.as_ref().map(|surface| &surface.faces),
            "coarse_before": coarse_before.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "old_prediction": old_prediction.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "new_prediction": new_prediction.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "fine_before_normals": fine_before.normals.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "coarse_after": coarse_after.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "fine_after": fine_after.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "fine_colors": fine_after.colors,
            "fine_materials": fine_after.materials,
            "fine_masks": fine_after.masks,
        })
    );
    Ok(())
}
