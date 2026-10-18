//! Emit an independently authored sphere after OBrush's cube-edge remesher.
//! This is a diagnostic fixture, not a SculptGL parity oracle.
use obrush_core::{Mesh, RemeshSettings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args().nth(1);
    let source_text = if let Some(path) = input.as_ref() {
        std::fs::read_to_string(path)?
    } else {
        include_str!("../tests/fixtures/independent-reference-sphere.obj").to_owned()
    };
    let source = if input.is_some() {
        Mesh::import_obj(source_text.as_bytes())?.mesh
    } else {
        Mesh::from_obj(&source_text)?
    };
    let allow_uv_loss = source
        .surface
        .as_ref()
        .is_some_and(|surface| surface.uv.is_some());
    let mut cases = Vec::new();
    for (relaxed, smoothing_steps) in [(false, 0), (true, 1)] {
        let (mesh, report) = source.remeshed(&RemeshSettings {
            resolution: 16,
            marching_cubes_mode: true,
            allow_uv_loss,
            smoothing_steps,
            ..RemeshSettings::default()
        })?;
        cases.push(serde_json::json!({
            "relaxed": relaxed,
            "smoothing_steps": smoothing_steps,
            "report": report,
            "positions": mesh.positions.iter().map(|p| [p.x, p.y, p.z]).collect::<Vec<_>>(),
            "triangles": mesh.triangles,
            "normals": mesh.normals.iter().map(|n| [n.x, n.y, n.z]).collect::<Vec<_>>(),
            "masks": mesh.masks,
            "colors": mesh.colors,
            "materials": mesh.materials,
        }));
    }
    serde_json::to_writer(std::io::stdout(), &cases)?;
    Ok(())
}
