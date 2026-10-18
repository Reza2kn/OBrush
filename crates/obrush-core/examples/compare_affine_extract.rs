//! Pair the independently authored transformed-source extraction fixture.
//! Usage: cargo run -p obrush-core --example compare_affine_extract -- FIXTURE.obrush
use obrush_core::{Document, MaskExtractSelection, MaskExtractSettings};
use serde_json::json;

fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Pass a .obrush fixture path")?;
    let source = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    if source.len() > 1024 * 1024 {
        return Err("Diagnostic fixture exceeds 1 MiB".into());
    }
    let document = Document::from_json(&source)?;
    let input: serde_json::Value =
        serde_json::from_str(&document.to_json()?).map_err(|error| error.to_string())?;
    let cases = [0.0, -0.125, 0.125]
        .into_iter()
        .map(|thickness| {
            let mut trial = Document::from_json(&source)?;
            let (id, report) = trial.extract_mask_as_subtool_with_surface_options(
                &MaskExtractSettings {
                    threshold: 0.5,
                    thickness,
                },
                MaskExtractSelection::WholeIncidentFaces,
                true,
                true,
            )?;
            trial.select_subtool(id)?;
            let part = trial.require_active_mesh()?;
            Ok(json!({"thickness": thickness, "report": report, "part": part}))
        })
        .collect::<Result<Vec<_>, String>>()?;
    println!(
        "{}",
        serde_json::to_string(&json!({"source": path, "input": input, "cases": cases}))
            .map_err(|error| error.to_string())?
    );
    Ok(())
}
