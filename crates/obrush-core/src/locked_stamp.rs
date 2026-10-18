//! Absolute one-dab stamp previews for the exposed lock-position brush workflow.
//! The engine owns no pointer queue, renderer, scene selection or history entry.
use crate::{BrushKind, BrushResult, BrushSettings, Mesh, Vec3, DEFAULT_VERTEX_MATERIAL};
use serde::{Deserialize, Serialize};

/// Input from a host's absolute pointer travel, already converted to world
/// radius and a stamp-frame angle. These values replace the preceding sample;
/// they are never incremental dabs or accumulated rotations.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedStampSample {
    pub radius: f32,
    pub alpha_rotation: f32,
    pub alpha_tangent: Option<Vec3>,
    pub pressure: f32,
}

impl Default for LockedStampSample {
    fn default() -> Self {
        Self {
            radius: 0.0,
            alpha_rotation: 0.0,
            alpha_tangent: None,
            pressure: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockedStampAction {
    Preview,
    Restore,
    Cancel,
}

/// Read-only provenance of the most recently processed sample. `changed()`
/// describes the real transition from the preceding preview, while dab fields
/// describe the single operation evaluated against the captured source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LockedStampReport {
    pub kind: BrushKind,
    pub action: LockedStampAction,
    pub sequence: u64,
    pub center: Vec3,
    pub normal: Vec3,
    pub radius: f32,
    pub effective_radius: Option<f32>,
    pub alpha_rotation: f32,
    pub alpha_tangent: Option<Vec3>,
    pub pressure: f32,
    pub effective_pressure: f32,
    pub at_origin: bool,
    pub changed_vertices: usize,
    pub topology_changed: bool,
    pub dab_changed_vertices: usize,
    pub dab_displacement_scale: f32,
    pub refined_vertices: usize,
    pub detail_limited: bool,
    pub original_vertices: usize,
    pub original_triangles: usize,
    pub current_vertices: usize,
    pub current_triangles: usize,
}

impl LockedStampReport {
    pub fn changed(&self) -> bool {
        self.changed_vertices != 0 || self.topology_changed
    }
}

/// Capture a source mesh and brush settings once at press. Every positive
/// contact preview starts from this same source, so radius, angle and detail
/// cannot accumulate while a pointer remains held. The host groups the gesture
/// with Document::begin_stroke/end_stroke and ends/cancels it before commands.
pub struct LockedStamp {
    original: Mesh,
    expected: Mesh,
    center: Vec3,
    normal: Vec3,
    settings: BrushSettings,
    sequence: u64,
    at_origin: bool,
    last_report: Option<LockedStampReport>,
}

impl LockedStamp {
    /// The pinned source exposes Lock Position for its ordinary Brush (including
    /// Clay option), Inflate, Smooth, Flatten, Crease, Pinch, Masking and Paint.
    /// Move, Drag, Twist, Local Scale, Morph and object transforms are excluded.
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

    pub fn prepare(
        mesh: &Mesh,
        center: Vec3,
        normal: Vec3,
        settings: &BrushSettings,
    ) -> Result<Self, String> {
        mesh.validate()?;
        if !Self::supports_kind(settings.kind) {
            return Err("Lock Position is unavailable for this brush kind".into());
        }
        if !center.is_finite()
            || center.abs().max_element() > 1_000_000.0
            || !normal.is_finite()
            || normal == Vec3::ZERO
            || !valid_captured_settings(settings)
        {
            return Err(
                "Locked stamp requires finite captured settings, a center and a surface normal"
                    .into(),
            );
        }
        let normal = normal.as_dvec3().normalize().as_vec3();
        Ok(Self {
            original: mesh.snapshot(),
            expected: mesh.snapshot(),
            center,
            normal,
            settings: settings.clone(),
            sequence: 0,
            at_origin: true,
            last_report: None,
        })
    }

    pub fn kind(&self) -> BrushKind {
        self.settings.kind
    }
    pub fn center(&self) -> Vec3 {
        self.center
    }
    pub fn normal(&self) -> Vec3 {
        self.normal
    }
    pub fn settings(&self) -> &BrushSettings {
        &self.settings
    }
    pub fn last_report(&self) -> Option<&LockedStampReport> {
        self.last_report.as_ref()
    }
    pub fn at_origin(&self) -> bool {
        self.at_origin
    }

    /// Radius zero is an explicit exact restoration. With a nonzero radius,
    /// zero/negative contact leaves the last preview unchanged. A cancelled
    /// contact restores through cancel(), independent of the last pressure.
    /// Every field is validated before mutation; stale/external edits reject.
    pub fn apply(
        &mut self,
        mesh: &mut Mesh,
        sample: LockedStampSample,
    ) -> Result<LockedStampReport, String> {
        self.require_expected(mesh)?;
        if !sample.radius.is_finite()
            || sample.radius < 0.0
            || sample.radius > 1_000_000.0
            || !sample.alpha_rotation.is_finite()
            || sample
                .alpha_tangent
                .is_some_and(|tangent| !tangent.is_finite())
            || !sample.pressure.is_finite()
        {
            return Err(
                "Locked stamp sample requires a bounded radius and finite alpha/contact inputs"
                    .into(),
            );
        }
        if sample.radius == 0.0 {
            return self.install(
                mesh,
                self.original.snapshot(),
                sample,
                LockedStampAction::Restore,
                BrushResult::default(),
            );
        }
        if sample.pressure <= 0.0 {
            return self.describe_no_contact(mesh, sample);
        }
        let mut candidate = self.original.snapshot();
        let mut settings = self.settings.clone();
        settings.radius = sample.radius;
        settings.alpha_rotation = sample.alpha_rotation;
        settings.alpha_tangent = sample.alpha_tangent;
        settings.pressure = sample.pressure.clamp(0.0, 1.0);
        // Lock previews are one ordinary dab. The Accumulate proxy is a separate
        // path stroke mode and cannot feed a previous locked preview into this.
        settings.draw_accumulate = true;
        let dab =
            candidate.apply_brush_with_result(self.center, self.normal, Vec3::ZERO, &settings);
        candidate.validate()?;
        self.install(mesh, candidate, sample, LockedStampAction::Preview, dab)
    }

    /// Explicit cancellation/restoration cannot overwrite an external edit.
    pub fn cancel(&mut self, mesh: &mut Mesh) -> Result<LockedStampReport, String> {
        self.require_expected(mesh)?;
        self.install(
            mesh,
            self.original.snapshot(),
            LockedStampSample::default(),
            LockedStampAction::Cancel,
            BrushResult::default(),
        )
    }

    fn require_expected(&self, mesh: &Mesh) -> Result<(), String> {
        if !same_exact(mesh, &self.expected) {
            Err("Locked stamp is stale after an external surface edit".into())
        } else if self.sequence == u64::MAX {
            Err("Locked stamp sample sequence is exhausted".into())
        } else {
            Ok(())
        }
    }

    fn describe_no_contact(
        &mut self,
        mesh: &Mesh,
        sample: LockedStampSample,
    ) -> Result<LockedStampReport, String> {
        let report = self.report(
            mesh,
            sample,
            LockedStampAction::Preview,
            0,
            false,
            BrushResult::default(),
        );
        self.sequence = report.sequence;
        self.at_origin = report.at_origin;
        self.last_report = Some(report.clone());
        Ok(report)
    }

    fn install(
        &mut self,
        mesh: &mut Mesh,
        candidate: Mesh,
        sample: LockedStampSample,
        action: LockedStampAction,
        dab: BrushResult,
    ) -> Result<LockedStampReport, String> {
        let changed_vertices = changed_point_count(mesh, &candidate);
        let topology_changed = mesh.triangles != candidate.triangles;
        let report = self.report(
            &candidate,
            sample,
            action,
            changed_vertices,
            topology_changed,
            dab,
        );
        if report.changed() {
            *mesh = candidate;
            self.expected = mesh.snapshot();
        }
        self.sequence = report.sequence;
        self.at_origin = report.at_origin;
        self.last_report = Some(report.clone());
        Ok(report)
    }

    fn report(
        &self,
        candidate: &Mesh,
        sample: LockedStampSample,
        action: LockedStampAction,
        changed_vertices: usize,
        topology_changed: bool,
        dab: BrushResult,
    ) -> LockedStampReport {
        let mut settings = self.settings.clone();
        settings.radius = sample.radius;
        settings.pressure = sample.pressure.clamp(0.0, 1.0);
        LockedStampReport {
            kind: self.kind(),
            action,
            sequence: self.sequence + 1,
            center: self.center,
            normal: self.normal,
            radius: sample.radius,
            effective_radius: settings.effective_radius(),
            alpha_rotation: sample.alpha_rotation,
            alpha_tangent: sample.alpha_tangent,
            pressure: sample.pressure,
            effective_pressure: sample.pressure.clamp(0.0, 1.0),
            at_origin: same_exact(candidate, &self.original),
            changed_vertices,
            topology_changed,
            dab_changed_vertices: dab.changed_vertices,
            dab_displacement_scale: dab.displacement_scale,
            refined_vertices: dab.refined_vertices,
            detail_limited: dab.detail_limited,
            original_vertices: self.original.positions.len(),
            original_triangles: self.original.triangles.len(),
            current_vertices: candidate.positions.len(),
            current_triangles: candidate.triangles.len(),
        }
    }
}

fn valid_captured_settings(settings: &BrushSettings) -> bool {
    settings.radius.is_finite()
        && settings.radius > 0.0
        && settings.radius <= 1_000_000.0
        && settings.strength.is_finite()
        && settings.pressure.is_finite()
        && settings.color.iter().all(|channel| channel.is_finite())
        && settings.paint_targets_are_valid()
        && settings.alpha_rotation.is_finite()
        && settings
            .alpha_tangent
            .is_none_or(|tangent| tangent.is_finite())
        && settings.hardness.is_finite()
        && settings.symmetry_offset.is_finite()
        && settings.symmetry_offset.abs() <= 1_000_000.0
        && settings
            .culling_axis
            .is_none_or(|axis| axis.is_finite() && axis != Vec3::ZERO)
}

fn point_same(first: &Mesh, second: &Mesh, vertex: usize) -> bool {
    first.mask_convention == second.mask_convention
        && first.positions[vertex].to_array().map(f32::to_bits)
            == second.positions[vertex].to_array().map(f32::to_bits)
        && first.normals[vertex].to_array().map(f32::to_bits)
            == second.normals[vertex].to_array().map(f32::to_bits)
        && first.masks[vertex].to_bits() == second.masks[vertex].to_bits()
        && first.colors[vertex].map(f32::to_bits) == second.colors[vertex].map(f32::to_bits)
        && first
            .materials
            .as_ref()
            .map_or(DEFAULT_VERTEX_MATERIAL, |values| values[vertex])
            .map(f32::to_bits)
            == second
                .materials
                .as_ref()
                .map_or(DEFAULT_VERTEX_MATERIAL, |values| values[vertex])
                .map(f32::to_bits)
}
fn changed_point_count(first: &Mesh, second: &Mesh) -> usize {
    (0..first.positions.len().min(second.positions.len()))
        .filter(|&vertex| !point_same(first, second, vertex))
        .count()
        + first.positions.len().abs_diff(second.positions.len())
}
fn same_exact(first: &Mesh, second: &Mesh) -> bool {
    first.attributes_equal_bits(second)
        && first.positions.len() == second.positions.len()
        && first.normals.len() == second.normals.len()
        && first.masks.len() == second.masks.len()
        && first.colors.len() == second.colors.len()
        && first.triangles == second.triangles
        && changed_point_count(first, second) == 0
}
