//! Shared, nonmutating topology jobs for native threads and browser workers.
use crate::{
    DecimateReport, DecimateSettings, Document, Mesh, RemeshReport, RemeshSettings,
    SelectedRemeshReport, SubtoolMergePolicy,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "settings",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TopologyOperation {
    Remesh(RemeshSettings),
    RemeshSelected {
        remesh: RemeshSettings,
        policy: SubtoolMergePolicy,
    },
    Decimate(DecimateSettings),
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "report", rename_all = "snake_case")]
pub enum TopologyReport {
    Remesh(RemeshReport),
    RemeshSelected(SelectedRemeshReport),
    Decimate(DecimateReport),
}

pub fn execute_topology(
    mesh: &Mesh,
    operation: &TopologyOperation,
) -> Result<(Mesh, TopologyReport), String> {
    execute_topology_with_cancel(mesh, operation, || false)
}

pub fn execute_topology_with_cancel(
    mesh: &Mesh,
    operation: &TopologyOperation,
    mut cancelled: impl FnMut() -> bool,
) -> Result<(Mesh, TopologyReport), String> {
    match operation {
        TopologyOperation::Remesh(settings) => mesh
            .remeshed_with_progress(settings, |_| cancelled())
            .map(|(mesh, report)| (mesh, TopologyReport::Remesh(report))),
        TopologyOperation::RemeshSelected { .. } => {
            Err("Selected remesh requires the complete scene project".into())
        }
        TopologyOperation::Decimate(settings) => mesh
            .decimated_with_progress(settings, |_| cancelled())
            .map(|(mesh, report)| (mesh, TopologyReport::Decimate(report))),
    }
}

/// The worker uses the existing bounded project decoder and encoder. The
/// caller keeps its original document/history until the result is accepted.
pub fn execute_topology_project(project: &str, operation: &str) -> Result<String, String> {
    execute_topology_project_with_cancel(project, operation, || false)
}

pub fn execute_topology_project_with_cancel(
    project: &str,
    operation: &str,
    mut cancelled: impl FnMut() -> bool,
) -> Result<String, String> {
    if operation.len() > 16 * 1024 {
        return Err("Topology settings exceed the 16 KiB limit".into());
    }
    let operation: TopologyOperation = serde_json::from_str(operation)
        .map_err(|error| format!("Invalid topology settings: {error}"))?;
    let mut result = Document::from_json(project)?;
    if result.layer_topology_locked()
        && !matches!(
            &operation,
            TopologyOperation::RemeshSelected {
                policy: SubtoolMergePolicy::FlattenDisplayed,
                ..
            }
        )
    {
        return Err("Bake sculpt layers before rebuilding or reducing the surface".into());
    }
    let report = match &operation {
        TopologyOperation::RemeshSelected { remesh, policy } => {
            let report =
                result.remesh_selected_subtools_with_progress(remesh, *policy, |_| cancelled())?;
            TopologyReport::RemeshSelected(report)
        }
        _ => {
            let (mesh, report) = execute_topology_with_cancel(
                result.require_active_mesh()?,
                &operation,
                &mut cancelled,
            )?;
            // A zero-collapse reduction is a no-op, including retained state.
            if !matches!(&report, TopologyReport::Decimate(report) if report.collapses == 0) {
                result.replace_mesh(mesh)?;
            }
            report
        }
    };
    let project = result.to_json()?;
    serde_json::to_string(&serde_json::json!({"project": project, "report": report}))
        .map_err(|error| format!("Cannot encode topology result: {error}"))
}
