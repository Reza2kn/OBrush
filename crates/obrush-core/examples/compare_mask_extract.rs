//! Diagnostic for paired, independently authored OBJ mask-extraction fixtures.
//! Usage: cargo run -p obrush-core --example compare_mask_extract -- FILE
//! Output is JSON; the source is validated before any extraction result exists.
use obrush_core::{MaskExtractSelection, MaskExtractSettings, Mesh};
use serde_json::json;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("Pass one OBJ fixture path")?;
    let source =
        std::fs::read_to_string(&path).map_err(|error| format!("Cannot read fixture: {error}"))?;
    if source.len() > 1024 * 1024 {
        return Err("Diagnostic fixture exceeds 1 MiB".into());
    }
    let mesh = Mesh::import_obj(source.as_bytes())?.mesh;
    mesh.validate()?;
    let cases = [0.0, -0.125, 0.125]
        .into_iter()
        .map(|thickness| {
            let (part, report) = mesh.extract_mask_with_surface_offset(
                &MaskExtractSettings {
                    threshold: 0.5,
                    thickness,
                },
                MaskExtractSelection::WholeIncidentFaces,
                true,
            )?;
            Ok(json!({"thickness": thickness, "report": report, "part": part}))
        })
        .collect::<Result<Vec<_>, String>>()?;
    println!(
        "{}",
        serde_json::to_string(&json!({"source": path, "input": mesh, "cases": cases}))
            .map_err(|error| error.to_string())?
    );
    Ok(())
}
