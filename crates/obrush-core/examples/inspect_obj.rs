//! Inspect an OBJ through OBrush's actual importer:
//! `cargo run -p obrush-core --example inspect_obj -- path/to/model.obj`.
use obrush_core::Mesh;
use std::io::Read;

fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Pass the path to an OBJ file")?;
    let file = std::fs::File::open(&path).map_err(|error| format!("Cannot read OBJ: {error}"))?;
    let mut source = String::new();
    file.take(128 * 1024 * 1024 + 1)
        .read_to_string(&mut source)
        .map_err(|error| format!("Cannot read OBJ: {error}"))?;
    let mesh = Mesh::from_obj(&source)?;
    let non_default = mesh
        .colors
        .iter()
        .filter(|&&color| color != [0.65, 0.69, 0.74])
        .count();
    println!(
        "{} vertices / {} triangles / {non_default} vertices with non-default colors",
        mesh.positions.len(),
        mesh.triangles.len()
    );
    let minimum: [f32; 3] = std::array::from_fn(|channel| {
        mesh.colors
            .iter()
            .map(|color| color[channel])
            .fold(1.0, f32::min)
    });
    let maximum: [f32; 3] = std::array::from_fn(|channel| {
        mesh.colors
            .iter()
            .map(|color| color[channel])
            .fold(0.0, f32::max)
    });
    println!("RGB minimum {minimum:?}; maximum {maximum:?}");
    if source
        .lines()
        .any(|line| line.trim_start().starts_with("#MRGB "))
    {
        println!("MRGB RGB bytes decoded; the leading mask byte is deliberately ignored.");
    }
    Ok(())
}
