//! Additive, fixed-topology locked dabs against the current surface.
//! This supplies a workflow engine using legacy OBrush brush response; precise
//! source picking/alpha/kernel preparation remains a separate host contract.
use crate::{BrushKind, BrushResult, BrushSettings, Mesh, Vec3, DEFAULT_VERTEX_MATERIAL};
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub struct LockedAccumulationSample {
    /// Actual fresh pick on the evolving surface, not the original press hit.
    pub center: Vec3,
    pub normal: Vec3,
    pub radius: f32,
    pub pressure: f32,
    pub alpha_rotation: f32,
    pub alpha_tangent: Option<Vec3>,
    pub culling_axis: Option<Vec3>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum LockedAccumulationAction {
    Accumulate,
    Hold,
    Cancel,
}

#[derive(Clone, Debug, Serialize)]
pub struct LockedAccumulationReport {
    pub kind: BrushKind,
    pub sequence: u64,
    pub action: LockedAccumulationAction,
    pub center: Vec3,
    pub radius: f32,
    pub pressure: f32,
    pub changed_vertices: usize,
    pub materialized: bool,
    pub at_original: bool,
    pub displacement_scale: f32,
}

/// Owns exact original/last accepted data, no pointer timer or Document history.
/// The host owns a single begin/end transaction and consumes one current tick.
/// Returning the pointer to zero radius *holds* accumulated work; cancellation
/// is separate and restores exact original data. Existing LockedStamp is intact.
pub struct LockedAccumulator {
    original: Mesh,
    expected: Mesh,
    settings: BrushSettings,
    sequence: u64,
}

impl LockedAccumulator {
    pub const fn supports_kind(kind: BrushKind) -> bool {
        matches!(
            kind,
            BrushKind::Draw
                | BrushKind::Clay
                | BrushKind::Inflate
                | BrushKind::Smooth
                | BrushKind::Flatten
                | BrushKind::Crease
                | BrushKind::Pinch
                | BrushKind::Mask
                | BrushKind::Paint
        )
    }

    pub fn prepare(mesh: &Mesh, settings: &BrushSettings) -> Result<Self, String> {
        mesh.validate()?;
        if !Self::supports_kind(settings.kind) || settings.adaptive_detail {
            return Err(
                "Locked accumulation requires an exposed ordinary tool and fixed topology".into(),
            );
        }
        if !valid_settings(settings) {
            return Err(
                "Locked accumulation requires bounded finite captured brush settings".into(),
            );
        }
        // Own the snapshot budget in core, where all attribute allocation is known.
        if mesh
            .approximate_bytes()
            .checked_mul(2)
            .is_none_or(|bytes| bytes > 128 * 1024 * 1024)
        {
            return Err(
                "Continuous Lock original/expected snapshots exceed their 128 MiB bound".into(),
            );
        }
        let mut settings = settings.clone();
        // Source lock bypasses the non-accumulating proxy, even if its saved
        // ordinary Brush option is false. This does not edit saved host prefs.
        settings.draw_accumulate = true;
        Ok(Self {
            original: mesh.snapshot(),
            expected: mesh.snapshot(),
            settings,
            sequence: 0,
        })
    }

    pub fn kind(&self) -> BrushKind {
        self.settings.kind
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn settings(&self) -> &BrushSettings {
        &self.settings
    }
    pub fn at_original(&self) -> bool {
        same_exact(&self.expected, &self.original)
    }

    pub fn apply(
        &mut self,
        mesh: &mut Mesh,
        sample: LockedAccumulationSample,
    ) -> Result<LockedAccumulationReport, String> {
        self.require_expected(mesh)?;
        if !valid_sample(sample) {
            return Err(
                "Locked accumulation sample has invalid radius/contact/pick/frame values".into(),
            );
        }
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or("Locked accumulation sequence overflow")?;
        // Source zero travel creates a 0/0 screen offset. Explicitly skipping
        // that undefined sample preserves prior work rather than restoring it.
        if sample.radius == 0.0 || sample.pressure == 0.0 {
            self.sequence = sequence;
            return Ok(self.report(
                sample,
                LockedAccumulationAction::Hold,
                BrushResult::default(),
                0,
                false,
            ));
        }
        let mut settings = self.settings.clone();
        settings.radius = sample.radius;
        settings.pressure = sample.pressure;
        settings.alpha_rotation = sample.alpha_rotation;
        settings.alpha_tangent = sample.alpha_tangent;
        settings.culling_axis = sample.culling_axis;
        // A private current-surface candidate guarantees atomic rejection and
        // unchanged external state. No source/rest preview is reapplied here.
        let mut candidate = mesh.snapshot();
        let dab =
            candidate.apply_brush_with_result(sample.center, sample.normal, Vec3::ZERO, &settings);
        candidate.validate()?;
        if candidate.triangles != mesh.triangles
            || candidate.positions.len() != mesh.positions.len()
        {
            return Err("Locked accumulation cannot change topology".into());
        }
        let changed = changed_points(mesh, &candidate);
        let materialized = mesh.materials.is_none() && candidate.materials.is_some();
        if !same_exact(mesh, &candidate) {
            self.expected = candidate.snapshot();
            *mesh = candidate;
        }
        self.sequence = sequence;
        Ok(self.report(
            sample,
            LockedAccumulationAction::Accumulate,
            dab,
            changed,
            materialized,
        ))
    }

    pub fn cancel(&mut self, mesh: &mut Mesh) -> Result<LockedAccumulationReport, String> {
        self.require_expected(mesh)?;
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or("Locked accumulation sequence overflow")?;
        let changed = changed_points(mesh, &self.original);
        let sample = LockedAccumulationSample {
            center: Vec3::ZERO,
            normal: Vec3::Z,
            radius: 0.0,
            pressure: 0.0,
            alpha_rotation: self.settings.alpha_rotation,
            alpha_tangent: self.settings.alpha_tangent,
            culling_axis: self.settings.culling_axis,
        };
        if !same_exact(mesh, &self.original) {
            *mesh = self.original.snapshot();
            self.expected = self.original.snapshot();
        }
        self.sequence = sequence;
        Ok(self.report(
            sample,
            LockedAccumulationAction::Cancel,
            BrushResult::default(),
            changed,
            false,
        ))
    }

    fn require_expected(&self, mesh: &Mesh) -> Result<(), String> {
        mesh.validate()?;
        if !same_exact(mesh, &self.expected) {
            return Err("External mesh/attribute edits invalidated locked accumulation".into());
        }
        Ok(())
    }

    fn report(
        &self,
        sample: LockedAccumulationSample,
        action: LockedAccumulationAction,
        dab: BrushResult,
        changed_vertices: usize,
        materialized: bool,
    ) -> LockedAccumulationReport {
        LockedAccumulationReport {
            kind: self.kind(),
            sequence: self.sequence,
            action,
            center: sample.center,
            radius: sample.radius,
            pressure: sample.pressure,
            changed_vertices,
            materialized,
            at_original: self.at_original(),
            displacement_scale: dab.displacement_scale,
        }
    }
}

fn normalized(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
fn valid_axis(axis: Option<Vec3>) -> bool {
    axis.is_none_or(|v| v.is_finite() && v.length_squared() > 0.0 && v.length_squared().is_finite())
}
fn valid_settings(s: &BrushSettings) -> bool {
    s.radius.is_finite()
        && s.radius > 0.0
        && s.radius <= 1_000_000.0
        && normalized(s.strength)
        && normalized(s.pressure)
        && normalized(s.hardness)
        && s.color.iter().copied().all(normalized)
        && s.paint_targets_are_valid()
        && s.alpha_rotation.is_finite()
        && valid_axis(s.alpha_tangent)
        && valid_axis(s.culling_axis)
        && s.symmetry_offset.is_finite()
        && s.symmetry_offset.abs() <= 1_000_000.0
}
fn valid_sample(s: LockedAccumulationSample) -> bool {
    s.center.is_finite()
        && s.center.abs().max_element() <= 1_000_000.0
        && s.normal.is_finite()
        && s.normal.length_squared() > 0.0
        && s.normal.length_squared().is_finite()
        && s.radius.is_finite()
        && s.radius >= 0.0
        && s.radius <= 1_000_000.0
        && normalized(s.pressure)
        && s.alpha_rotation.is_finite()
        && valid_axis(s.alpha_tangent)
        && valid_axis(s.culling_axis)
}
fn point_same(a: &Mesh, b: &Mesh, i: usize) -> bool {
    a.positions[i].to_array().map(f32::to_bits) == b.positions[i].to_array().map(f32::to_bits)
        && a.normals[i].to_array().map(f32::to_bits) == b.normals[i].to_array().map(f32::to_bits)
        && a.masks[i].to_bits() == b.masks[i].to_bits()
        && a.colors[i].map(f32::to_bits) == b.colors[i].map(f32::to_bits)
        && a.materials
            .as_ref()
            .map_or(DEFAULT_VERTEX_MATERIAL, |v| v[i])
            .map(f32::to_bits)
            == b.materials
                .as_ref()
                .map_or(DEFAULT_VERTEX_MATERIAL, |v| v[i])
                .map(f32::to_bits)
}
fn changed_points(a: &Mesh, b: &Mesh) -> usize {
    (0..a.positions.len().min(b.positions.len()))
        .filter(|&i| !point_same(a, b, i))
        .count()
        + a.positions.len().abs_diff(b.positions.len())
}
fn same_exact(a: &Mesh, b: &Mesh) -> bool {
    a.attributes_equal_bits(b)
        && a.triangles == b.triangles
        && a.positions.len() == b.positions.len()
        && a.normals.len() == b.normals.len()
        && a.masks.len() == b.masks.len()
        && a.colors.len() == b.colors.len()
        && changed_points(a, b) == 0
}
