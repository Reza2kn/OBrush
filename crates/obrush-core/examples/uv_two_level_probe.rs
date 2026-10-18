//! Controlled two-level source-static response to one direct coarse edit.
//! Usage: cargo run -p obrush-core --example uv_two_level_probe -- path.obj [coarse_vertex] [levels=2]
//! This isolates hierarchy transport; it does not represent a brush stroke.
use obrush_core::{
    source_ring::SourceStaticRingMap, Document, Mesh, StaticSubdivisionSettings, Vec3,
};
use serde_json::json;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("OBJ path is required")?;
    let vertex: usize = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "0".into())
        .parse()
        .map_err(|_| "Invalid coarse vertex index")?;
    let levels: usize = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "2".into())
        .parse()
        .map_err(|_| "Invalid level count")?;
    if !(2..=3).contains(&levels) {
        return Err("Level count must be two or three".into());
    }
    let source = std::fs::read(&path).map_err(|error| error.to_string())?;
    let mut document = Document::new(Mesh::import_obj(&source)?.mesh);
    let settings = StaticSubdivisionSettings::default();
    document.add_source_static_subdivision_level(&settings)?;
    let first_before = document.require_active_mesh()?.snapshot();
    document.add_source_static_subdivision_level(&settings)?;
    let second_before = document.require_active_mesh()?.snapshot();
    let third_before = if levels == 3 {
        document.add_source_static_subdivision_level(&settings)?;
        Some(document.require_active_mesh()?.snapshot())
    } else {
        None
    };
    let original = Mesh::import_obj(&source)?.mesh;
    let first_ring = SourceStaticRingMap::build(&original, &first_before)?;
    let second_ring = SourceStaticRingMap::build_after(&first_before, &second_before, &first_ring)?;
    let third_ring = third_before
        .as_ref()
        .map(|mesh| SourceStaticRingMap::build_after(&second_before, mesh, &second_ring))
        .transpose()?;
    document.select_level(0)?;
    let coarse_before = document.require_active_mesh()?.snapshot();
    let point = document
        .require_active_mesh_mut()?
        .positions
        .get_mut(vertex)
        .ok_or("Coarse vertex index is out of range")?;
    *point += Vec3::new(0.125, -0.09375, 0.0625);
    document.require_active_mesh_mut()?.recompute_normals();
    document.select_level(1)?;
    let first_after = document.require_active_mesh()?.snapshot();
    document.select_level(2)?;
    let second_after = document.require_active_mesh()?.snapshot();
    let third_after = if levels == 3 {
        document.select_level(3)?;
        Some(document.require_active_mesh()?.snapshot())
    } else {
        None
    };
    println!(
        "{}",
        json!({
            "input": path,
            "vertex": vertex,
            "delta": [0.125, -0.09375, 0.0625],
            "coarse_before": coarse_before.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "first_before": first_before.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "second_before": second_before.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "first_after": first_after.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "second_after": second_after.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>(),
            "third_before": third_before.as_ref().map(|mesh|mesh.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>()),
            "third_after": third_after.as_ref().map(|mesh|mesh.positions.iter().map(|p|p.to_array()).collect::<Vec<_>>()),
            "first_faces": first_before.surface.as_ref().map(|surface| &surface.faces),
            "second_faces": second_before.surface.as_ref().map(|surface| &surface.faces),
            "second_first_neighbor": second_ring.first_neighbor(),
            "third_first_neighbor": third_ring.as_ref().map(SourceStaticRingMap::first_neighbor),
        })
    );
    Ok(())
}
