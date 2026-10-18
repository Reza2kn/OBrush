//! One canonical f32 mask channel. Display conversion is never written back.
use crate::Mesh;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaskConvention {
    #[default]
    ProtectedF32V1,
    UnprotectedF32V1,
}

impl MaskConvention {
    pub fn is_default(&self) -> bool {
        *self == Self::ProtectedF32V1
    }
    pub fn clear_value(self) -> f32 {
        match self {
            Self::ProtectedF32V1 => 0.0,
            Self::UnprotectedF32V1 => 1.0,
        }
    }
    pub fn full_value(self) -> f32 {
        1.0 - self.clear_value()
    }
    /// Preserve legacy subtraction at exactly the caller's scalar precision.
    pub fn influence(self, raw: f32) -> f32 {
        let value = raw.clamp(0.0, 1.0);
        match self {
            Self::ProtectedF32V1 => 1.0 - value,
            Self::UnprotectedF32V1 => value,
        }
    }
    pub fn influence_f64(self, raw: f32) -> f64 {
        let value = f64::from(raw.clamp(0.0, 1.0));
        match self {
            Self::ProtectedF32V1 => 1.0 - value,
            Self::UnprotectedF32V1 => value,
        }
    }
    /// Temporary UI/GPU protection; rounding here does not alter canonical data.
    pub fn protection_for_display(self, raw: f32) -> f32 {
        let value = raw.clamp(0.0, 1.0);
        match self {
            Self::ProtectedF32V1 => value,
            Self::UnprotectedF32V1 => 1.0 - value,
        }
    }
    pub fn fully_protected(self, raw: f32) -> bool {
        match self {
            Self::ProtectedF32V1 => raw == 1.0,
            Self::UnprotectedF32V1 => raw == 0.0,
        }
    }
    pub fn has_protection(self, raw: f32) -> bool {
        match self {
            Self::ProtectedF32V1 => raw > 0.0,
            Self::UnprotectedF32V1 => raw < 1.0,
        }
    }
    /// No f32 complement is made before a protection threshold comparison.
    pub fn protection_at_least(self, raw: f32, threshold: f32) -> bool {
        match self {
            Self::ProtectedF32V1 => raw >= threshold,
            Self::UnprotectedF32V1 => {
                if threshold == 0.0 {
                    true
                } else if threshold == 1.0 {
                    raw == 0.0
                } else if raw == 1.0 {
                    false
                } else {
                    f64::from(raw) <= 1.0 - f64::from(threshold)
                }
            }
        }
    }
    pub fn protection_equals(self, raw: f32, threshold: f32) -> bool {
        match self {
            Self::ProtectedF32V1 => raw == threshold,
            Self::UnprotectedF32V1 => {
                if threshold == 0.0 {
                    raw == 1.0
                } else if threshold == 1.0 {
                    raw == 0.0
                } else {
                    f64::from(raw) == 1.0 - f64::from(threshold)
                }
            }
        }
    }
    pub fn add_protection(self, raw: f32, delta: f32) -> f32 {
        match self {
            Self::ProtectedF32V1 => (raw + delta).clamp(0.0, 1.0),
            Self::UnprotectedF32V1 => (raw - delta).clamp(0.0, 1.0),
        }
    }
    pub(crate) fn edge_cut(self, a: f32, b: f32, threshold: f32) -> f64 {
        match self {
            Self::ProtectedF32V1 => {
                (f64::from(threshold) - f64::from(a)) / (f64::from(b) - f64::from(a))
            }
            Self::UnprotectedF32V1 => {
                let (a, b, t) = (f64::from(a), f64::from(b), f64::from(threshold));
                let numerator = if a >= 0.5 {
                    t - (1.0 - a)
                } else {
                    a - (1.0 - t)
                };
                numerator / (a - b)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskConversionPolicy {
    RequireRoundtripExact,
    AllowAndReport,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MaskConversionReport {
    pub source: MaskConvention,
    pub destination: MaskConvention,
    pub vertices: usize,
    pub roundtrip_changed_words: usize,
    pub signed_zero_only_changes: usize,
    pub max_abs_roundtrip_error: f64,
    pub max_relative_roundtrip_error: f64,
}

impl Mesh {
    pub fn mask_influence(&self, vertex: usize) -> f32 {
        self.mask_convention.influence(self.masks[vertex])
    }
    pub fn mask_influence_f64(&self, vertex: usize) -> f64 {
        self.mask_convention.influence_f64(self.masks[vertex])
    }
    pub fn protection_for_display(&self, vertex: usize) -> f32 {
        self.mask_convention
            .protection_for_display(self.masks[vertex])
    }
    pub fn fully_protected(&self, vertex: usize) -> bool {
        self.mask_convention.fully_protected(self.masks[vertex])
    }
    pub fn has_protection(&self, vertex: usize) -> bool {
        self.mask_convention.has_protection(self.masks[vertex])
    }

    /// Fully validate before replacing either the channel or its convention.
    /// Document callers should use an undoable transaction rather than this API.
    pub fn install_mask_channel(
        &mut self,
        values: Vec<f32>,
        convention: MaskConvention,
    ) -> Result<(), String> {
        self.validate()?;
        if values.len() != self.positions.len() {
            return Err("Mask channel must match the vertex count".into());
        }
        if values.capacity() > crate::MAX_VERTICES {
            return Err("Mask channel allocation exceeds the vertex limit".into());
        }
        if values
            .capacity()
            .saturating_mul(std::mem::size_of::<f32>())
            .saturating_add(
                self.approximate_bytes().saturating_sub(
                    self.masks
                        .capacity()
                        .saturating_mul(std::mem::size_of::<f32>()),
                ),
            )
            > crate::decode_limits::MAX_SCENE_STORAGE_BYTES
        {
            return Err("Mask channel exceeds the mesh storage budget".into());
        }
        if values
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err("Mask channel values must be finite in 0–1".into());
        }
        self.masks = values;
        self.mask_convention = convention;
        Ok(())
    }

    /// An explicit numerical conversion, never an implicit export side effect.
    pub fn converted_mask_convention(
        &self,
        convention: MaskConvention,
        policy: MaskConversionPolicy,
    ) -> Result<(Self, MaskConversionReport), String> {
        self.validate()?;
        let mut report = MaskConversionReport {
            source: self.mask_convention,
            destination: convention,
            vertices: self.masks.len(),
            roundtrip_changed_words: 0,
            signed_zero_only_changes: 0,
            max_abs_roundtrip_error: 0.0,
            max_relative_roundtrip_error: 0.0,
        };
        if convention != self.mask_convention {
            for &old in &self.masks {
                let roundtrip = 1.0 - (1.0 - old);
                if old.to_bits() != roundtrip.to_bits() {
                    report.roundtrip_changed_words += 1;
                    if old == roundtrip {
                        report.signed_zero_only_changes += 1;
                    }
                    let delta = (f64::from(roundtrip) - f64::from(old)).abs();
                    report.max_abs_roundtrip_error = report.max_abs_roundtrip_error.max(delta);
                    if old != 0.0 {
                        report.max_relative_roundtrip_error = report
                            .max_relative_roundtrip_error
                            .max(delta / f64::from(old).abs());
                    }
                }
            }
        }
        if policy == MaskConversionPolicy::RequireRoundtripExact
            && report.roundtrip_changed_words != 0
        {
            return Err(format!(
                "Mask convention conversion would lose {} f32 words on roundtrip",
                report.roundtrip_changed_words
            ));
        }
        let mut output = self.snapshot();
        if convention != self.mask_convention {
            for value in &mut output.masks {
                *value = 1.0 - *value;
            }
            output.mask_convention = convention;
        }
        Ok((output, report))
    }
}
