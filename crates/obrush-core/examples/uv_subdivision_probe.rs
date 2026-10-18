//! Emits exact core face/corner UV arrays for an independently authored OBJ.
//! Usage: cargo run -p obrush-core --example uv_subdivision_probe -- path.obj
use obrush_core::{Mesh, StaticSubdivisionSettings};
use serde_json::json;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("OBJ path is required")?;
    let source = std::fs::read(&path).map_err(|error| error.to_string())?;
    let parent = Mesh::import_obj(&source)?.mesh;
    let child = parent
        .subdivided_static(&StaticSubdivisionSettings::default())?
        .mesh;
    let face_data = |mesh: &Mesh| {
        let surface = mesh.surface.as_ref().ok_or("Missing original faces")?;
        let uv = surface.uv.as_ref().ok_or("Missing corner UVs")?;
        Ok::<_, String>(json!({
            "positions": mesh.positions.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
            "faces": surface.faces,
            "uvs": uv.coordinates,
            "face_uvs": uv.face_indices,
            "colors": mesh.colors,
            "materials": mesh.materials,
            "masks": mesh.masks,
        }))
    };
    println!(
        "{}",
        json!({"input": path, "parent": face_data(&parent)?, "child": face_data(&child)?})
    );
    Ok(())
}
