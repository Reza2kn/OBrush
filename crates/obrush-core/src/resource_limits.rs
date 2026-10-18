//! Shared bounds for distinct document lifetimes. Encoded JSON includes the
//! immutable SGL origin and editable channels; its size is not retained scene
//! storage. History and atomic geometry operations have separate checked bounds.
//! These limits are not a process RSS or GPU-memory guarantee. Callers may keep
//! smaller selected budgets (for example SglStageSettings::storage_limit).

pub const MIB: usize = 1024 * 1024;
pub const MAX_MESH_INPUT_BYTES: usize = 128 * MIB;
pub const MAX_PROJECT_BYTES: usize = 512 * MIB;
pub const MAX_SCENE_STORAGE_BYTES: usize = 256 * MIB;
pub const MAX_HISTORY_BYTES: usize = 512 * MIB;
/// Static scene plus the retained geometry cache, excluding history/output.
pub const MAX_DOCUMENT_RETAINED_BYTES: usize = 512 * MIB;
pub const MAX_ORDERED_TREE_BYTES: usize = 128 * MIB;
pub const MAX_GEOMETRY_STATE_BYTES: usize = 256 * MIB;
pub const MAX_GEOMETRY_SCRATCH_BYTES: usize = 768 * MIB;
pub const MAX_GEOMETRY_OPERATION_BYTES: usize = 2048 * MIB;
/// Conservative aggregate renderer source-normal cache/replacement peak.
/// Includes shared selected geometry states, old/new overlap and certificates;
/// independent of the smaller individual fresh-normal builder budget.
pub const MAX_RENDER_SOURCE_NORMAL_BYTES: usize = 512 * MIB;

// The same constants must be representable on the wasm32 target. Every summed
// budget is still checked for overflow at its existing allocation boundary.
const _: () = assert!(MAX_GEOMETRY_OPERATION_BYTES <= u32::MAX as usize);

/// Local project files retain the larger encoded-project budget. Mesh/SGL and
/// unknown-extension inputs keep their existing bounded input policy.
pub fn document_input_limit(name: &str) -> usize {
    if name
        .rsplit_once('.')
        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("obrush"))
    {
        MAX_PROJECT_BYTES
    } else {
        MAX_MESH_INPUT_BYTES
    }
}

/// Shared pre-read policy; validates a length without allocating the payload.
pub fn check_document_input_size(name: &str, bytes: usize) -> Result<(), String> {
    let limit = document_input_limit(name);
    if bytes > limit {
        return Err(format!("File exceeds the {} MiB import limit", limit / MIB));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_and_mesh_extensions_use_distinct_shared_budgets() {
        for name in ["sculpt.obrush", "SCULPT.ObRuSh", "قلم.obrush"] {
            assert_eq!(document_input_limit(name), MAX_PROJECT_BYTES);
        }
        for name in [
            "sculpt.obj",
            "sculpt.sgl",
            "sculpt.ply",
            "sculpt.stl",
            "sculpt.json",
            "sculpt.obrush.obj",
            "",
            "obrush",
        ] {
            assert_eq!(document_input_limit(name), MAX_MESH_INPUT_BYTES);
        }
    }

    #[test]
    fn input_size_boundaries_reject_before_payload_allocation() {
        for name in ["one.obrush", "one.OBRUSH", "one.obj", "one.sgl", "unknown"] {
            let limit = document_input_limit(name);
            assert!(check_document_input_size(name, 0).is_ok());
            assert!(check_document_input_size(name, limit).is_ok());
            assert!(check_document_input_size(name, limit + 1).is_err());
            assert!(check_document_input_size(name, usize::MAX).is_err());
        }
        assert!(check_document_input_size("one.obj", MAX_PROJECT_BYTES).is_err());
        assert!(check_document_input_size("one.obrush", MAX_PROJECT_BYTES).is_ok());
    }
}
