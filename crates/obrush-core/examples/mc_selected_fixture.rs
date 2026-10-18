//! Paired diagnostic for independently authored selected solid sources.
use obrush_core::{Document, Mesh, RemeshSettings, SubtoolMergePolicy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let left = args.next().ok_or("Pass left OBJ path")?;
    let right = args.next().ok_or("Pass right OBJ path")?;
    let mode = args.next().unwrap_or_else(|| "mc".into());
    if !matches!(mode.as_str(), "mc" | "nets" | "block") {
        return Err("Expected optional mode: mc, nets or block".into());
    }
    let left = Mesh::import_obj(&std::fs::read(left)?)?.mesh;
    let right = Mesh::import_obj(&std::fs::read(right)?)?.mesh;
    let mut cases = Vec::new();
    let variants: &[bool] = if mode == "mc" {
        &[false, true]
    } else {
        &[false]
    };
    for &relaxed in variants {
        let mut document = Document::new(left.snapshot());
        let right_id = document.add_subtool(right.snapshot(), "Right")?;
        // The reference imports left then right, then multi-selects left while
        // right is active. Its selected array is [right, left].
        document.select_subtools(&[right_id, 1], Some(1))?;
        let report = document.remesh_selected_subtools(
            &RemeshSettings {
                resolution: 16,
                marching_cubes_mode: mode == "mc",
                surface_nets_mode: mode == "nets",
                block_mode: mode == "block",
                smoothing_steps: u32::from(relaxed),
                ..RemeshSettings::default()
            },
            SubtoolMergePolicy::RequirePlain,
        )?;
        let mesh = document.require_active_mesh()?;
        cases.push(serde_json::json!({
            "relaxed": relaxed,
            "mode": mode,
            "report": report,
            "positions": mesh.positions.iter().map(|p| [p.x,p.y,p.z]).collect::<Vec<_>>(),
            "triangles": mesh.triangles,
            "faces": mesh.surface.as_ref().map(|surface| &surface.faces),
            "colors": mesh.colors,
            "materials": mesh.materials,
            "masks": mesh.masks,
        }));
    }
    serde_json::to_writer(std::io::stdout(), &cases)?;
    Ok(())
}
