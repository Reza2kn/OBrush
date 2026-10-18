//! Compare source-selected order for an independently authored posed SGL pair.
use obrush_core::{Document, RemeshSettings, SglStageSettings, SubtoolMergePolicy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Pass paired SGL path")?;
    let bytes = std::fs::read(path)?;
    let mut cases = Vec::new();
    for order in [[2, 1], [1, 2]] {
        for relaxed in [false, true] {
            let mut document = Document::empty();
            document.replace_sgl_bytes(&bytes, &SglStageSettings::default())?;
            document.select_subtools(&order, Some(1))?;
            let report = document.remesh_selected_subtools(
                &RemeshSettings {
                    resolution: 16,
                    marching_cubes_mode: true,
                    smoothing_steps: u32::from(relaxed),
                    allow_source_pose_loss: true,
                    ..RemeshSettings::default()
                },
                SubtoolMergePolicy::RequirePlain,
            )?;
            let mesh = document.require_active_mesh()?;
            cases.push(serde_json::json!({
                "order": order,
                "relaxed": relaxed,
                "report": report,
                "positions": mesh.positions.iter().map(|p| [p.x,p.y,p.z]).collect::<Vec<_>>(),
                "triangles": mesh.triangles,
                "colors": mesh.colors,
                "materials": mesh.materials,
                "masks": mesh.masks,
                "mask_convention": mesh.mask_convention,
            }));
        }
    }
    serde_json::to_writer(std::io::stdout(), &cases)?;
    Ok(())
}
