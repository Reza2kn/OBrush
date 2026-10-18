use obrush_core::{Mesh, RemeshSettings};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: block_remesh_fixture <independent.obj> [resolution] [--smooth-nets]")?;
    let resolution = std::env::args()
        .nth(2)
        .map_or(Ok(64), |value| value.parse::<u32>())?;
    let mesh = Mesh::import_obj(&std::fs::read(path)?)?.mesh;
    let smooth_nets = std::env::args().any(|arg| arg == "--smooth-nets");
    let (output, report) = mesh.remeshed(&RemeshSettings {
        resolution,
        block_mode: !smooth_nets,
        surface_nets_mode: smooth_nets,
        ..RemeshSettings::default()
    })?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "positions": output.positions,
            "triangles": output.triangles,
            "report": report,
        }))?
    );
    Ok(())
}
