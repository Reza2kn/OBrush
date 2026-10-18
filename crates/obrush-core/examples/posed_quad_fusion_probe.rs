//! Capture the shared-core output for the independently authored posed SGL pair.
//! `cargo run -p obrush-core --example posed_quad_fusion_probe -- fixture.sgl`
use obrush_core::{Document, RemeshSettings, SglStageSettings, SubtoolMergePolicy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Pass paired SGL path")?;
    let bytes = std::fs::read(path)?;
    let mut cases = Vec::new();
    for mode in ["nets", "block"] {
        let mut document = Document::empty();
        document.replace_sgl_bytes(&bytes, &SglStageSettings::default())?;
        document.select_subtools(&[2, 1], Some(1))?;
        let report = document.remesh_selected_subtools(
            &RemeshSettings {
                resolution: 16,
                surface_nets_mode: mode == "nets",
                block_mode: mode == "block",
                allow_source_pose_loss: true,
                ..RemeshSettings::default()
            },
            SubtoolMergePolicy::RequirePlain,
        )?;
        let mesh = document.require_active_mesh()?;
        let surface = mesh.surface.as_ref().ok_or("Missing quad surface")?;
        cases.push(serde_json::json!({
            "mode": mode,
            "report": report,
            "positions": mesh.positions.iter().map(|p| [p.x, p.y, p.z]).collect::<Vec<_>>(),
            "faces": surface.faces,
            "colors": mesh.colors,
            "materials": mesh.materials,
            "masks": mesh.masks,
            "mask_convention": mesh.mask_convention,
        }));
    }
    serde_json::to_writer(std::io::stdout(), &cases)?;
    Ok(())
}
