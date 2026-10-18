//! Explicit active-object mask representation transitions. These reports describe
//! Float32 precision; they do not claim reference brush or image equivalence.
use crate::{MaskConvention, MaskConversionReport, Mesh};
use serde::Serialize;

/// First raw-word witness of a lossy Float32 complement/complement roundtrip.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct MaskConversionWord {
    pub vertex: usize,
    pub source_bits: u32,
    pub converted_bits: u32,
    pub roundtrip_bits: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MaskLevelConversionReport {
    pub level: usize,
    pub active: bool,
    pub precision: MaskConversionReport,
    pub first_changed_word: Option<MaskConversionWord>,
}

/// Every effective retained level appears once, including the active level.
/// Scope is exactly the active SubTool, never inactive parts of its scene.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MaskHierarchyConversionReport {
    pub subtool_id: u32,
    pub retained_hierarchy: bool,
    pub active_level: usize,
    pub source: MaskConvention,
    pub destination: MaskConvention,
    pub changed: bool,
    pub vertices: usize,
    pub roundtrip_changed_words: usize,
    pub signed_zero_only_changes: usize,
    pub max_abs_roundtrip_error: f64,
    pub max_relative_roundtrip_error: f64,
    pub levels: Vec<MaskLevelConversionReport>,
}

impl MaskHierarchyConversionReport {
    pub(crate) fn new(
        subtool_id: u32,
        retained_hierarchy: bool,
        active_level: usize,
        source: MaskConvention,
        destination: MaskConvention,
    ) -> Self {
        Self {
            subtool_id,
            retained_hierarchy,
            active_level,
            source,
            destination,
            changed: source != destination,
            vertices: 0,
            roundtrip_changed_words: 0,
            signed_zero_only_changes: 0,
            max_abs_roundtrip_error: 0.0,
            max_relative_roundtrip_error: 0.0,
            levels: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, mesh: &Mesh, level: usize) -> Result<(), String> {
        if mesh.mask_convention != self.source {
            return Err("All effective levels must use the active mask convention".into());
        }
        let precision = level_report(mesh, self.destination, level, level == self.active_level);
        self.vertices = self
            .vertices
            .checked_add(precision.precision.vertices)
            .ok_or("Mask conversion vertex count overflow")?;
        if self.vertices > crate::MAX_VERTICES {
            return Err("Mask conversion exceeds the aggregate vertex limit".into());
        }
        self.roundtrip_changed_words += precision.precision.roundtrip_changed_words;
        self.signed_zero_only_changes += precision.precision.signed_zero_only_changes;
        self.max_abs_roundtrip_error = self
            .max_abs_roundtrip_error
            .max(precision.precision.max_abs_roundtrip_error);
        self.max_relative_roundtrip_error = self
            .max_relative_roundtrip_error
            .max(precision.precision.max_relative_roundtrip_error);
        self.levels.push(precision);
        Ok(())
    }
}

fn level_report(
    mesh: &Mesh,
    destination: MaskConvention,
    level: usize,
    active: bool,
) -> MaskLevelConversionReport {
    let mut precision = MaskConversionReport {
        source: mesh.mask_convention,
        destination,
        vertices: mesh.masks.len(),
        roundtrip_changed_words: 0,
        signed_zero_only_changes: 0,
        max_abs_roundtrip_error: 0.0,
        max_relative_roundtrip_error: 0.0,
    };
    let mut first_changed_word = None;
    if destination != mesh.mask_convention {
        for (vertex, &source) in mesh.masks.iter().enumerate() {
            let converted = 1.0_f32 - source;
            let roundtrip = 1.0_f32 - converted;
            if source.to_bits() == roundtrip.to_bits() {
                continue;
            }
            precision.roundtrip_changed_words += 1;
            if source == roundtrip {
                precision.signed_zero_only_changes += 1;
            }
            let delta = (f64::from(roundtrip) - f64::from(source)).abs();
            precision.max_abs_roundtrip_error = precision.max_abs_roundtrip_error.max(delta);
            if source != 0.0 {
                precision.max_relative_roundtrip_error = precision
                    .max_relative_roundtrip_error
                    .max(delta / f64::from(source).abs());
            }
            first_changed_word.get_or_insert(MaskConversionWord {
                vertex,
                source_bits: source.to_bits(),
                converted_bits: converted.to_bits(),
                roundtrip_bits: roundtrip.to_bits(),
            });
        }
    }
    MaskLevelConversionReport {
        level,
        active,
        precision,
        first_changed_word,
    }
}
