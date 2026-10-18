//! Opt-in fixed-topology Paint stencil from the pinned public SculptGL source.
//! This is a prepared local-space kernel, not reference gesture/camera parity.
//! The established BrushSettings/legacy Paint implementation is unchanged.
use crate::{
    BrushKind, BrushResult, MaskConvention, Mesh, RasterAlpha, Vec3, DEFAULT_VERTEX_MATERIAL,
};
use serde::Serialize;

pub const MAX_PAINT_RESPONSE_SCRATCH_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaintResponseOperation {
    #[default]
    OrdinaryDab,
    LockedPreview,
    LockedAccumulation,
    Fill,
    Picker,
    DynamicTopology,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintTabletIntensity {
    /// Raw source Tablet.pressure; absent/mouse input uses 0.5, not one.
    pub pressure: f64,
    pub factor: f64,
}
impl Default for PaintTabletIntensity {
    fn default() -> Self {
        Self {
            pressure: 0.5,
            factor: 0.0,
        }
    }
}
impl PaintTabletIntensity {
    pub fn multiplier(self) -> Result<f64, String> {
        tablet_multiplier(self.pressure, self.factor)
    }
}

/// Explicit local-space linear projection adapter. The Paint kernel instead
/// receives Picking's already pressure/camera/scale-adjusted radius squared.
/// Actual camera unprojection/Float32 rounding must be supplied by the host.
pub fn paint_tablet_local_radius_squared(
    unpressured_local_squared: f64,
    tablet_pressure: f64,
    radius_factor: f64,
) -> Result<f64, String> {
    validate_radius(unpressured_local_squared)?;
    let multiplier = tablet_multiplier(tablet_pressure, radius_factor)?;
    let radius = unpressured_local_squared.sqrt() * multiplier;
    let result = radius * radius;
    validate_radius(result)?;
    Ok(result)
}
fn tablet_multiplier(pressure: f64, factor: f64) -> Result<f64, String> {
    if !pressure.is_finite()
        || !factor.is_finite()
        || !(0.0..=1.0).contains(&pressure)
        || !(0.0..=1.0).contains(&factor)
    {
        return Err("Source Tablet pressure/factor must be finite in 0–1".into());
    }
    Ok(1.0 + factor * (pressure * 2.0 - 1.0))
}

#[derive(Clone, Debug)]
pub struct PaintResponseOptions {
    pub intensity: f64,
    pub hardness: f64,
    pub color: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
    pub write_rgb: bool,
    pub write_roughness: bool,
    pub write_metallic: bool,
    pub culling: bool,
    pub tablet: PaintTabletIntensity,
    pub alpha: Option<RasterAlpha>,
}
impl Default for PaintResponseOptions {
    /// Source defaults are opt-in request defaults, not shipped OBrush defaults.
    fn default() -> Self {
        Self {
            intensity: 0.75,
            hardness: 0.75,
            color: [1.0, 0.766, 0.336],
            roughness: 0.3,
            metallic: 0.95,
            write_rgb: true,
            write_roughness: true,
            write_metallic: true,
            culling: false,
            tablet: PaintTabletIntensity::default(),
            alpha: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintResponseAlphaFrame {
    /// Source Picking._alphaLookAt Float32 matrix, column-major. Its generation
    /// and origin tracking are a separate host contract, never inferred here.
    pub look_at: [f32; 16],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintResponsePass {
    /// Source Picking._interPoint is a plain Number array, not Float32.
    pub center: [f64; 3],
    /// Prepared source Picking.getLocalRadius2(), including pressure and scale.
    pub local_radius_squared: f64,
    /// Source Picking eye direction points away from the camera. No fallback
    /// or renormalization changes its supplied Number-array scalar semantics.
    pub eye_direction: [f64; 3],
    /// Equivalent to source Picking._xSym for the alpha X lookup only. Mirrored
    /// center/eye/frame are prepared actual picks, not guessed reflections.
    pub mirrored_x: bool,
    pub alpha_frame: Option<PaintResponseAlphaFrame>,
}

#[derive(Clone, Debug)]
pub struct PaintResponseRequest {
    /// This implementation explicitly rejects every requested non-Paint tool.
    pub kind: BrushKind,
    pub operation: PaintResponseOperation,
    /// Contact eligibility is distinct from the raw source Tablet pressure.
    /// False preserves the OBrush no-contact/no-history invariant.
    pub in_contact: bool,
    pub options: PaintResponseOptions,
    pub first: PaintResponsePass,
    /// Ordered second source symmetry pass; overlapping vertices blend twice.
    pub mirrored: Option<PaintResponsePass>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct PaintResponseReport {
    pub changed_vertices: usize,
    pub changed_rgb_vertices: usize,
    pub changed_material_vertices: usize,
    pub materialized: bool,
    pub first_selected_vertices: usize,
    pub mirrored_selected_vertices: usize,
    pub maximum_weight: f64,
}
impl PaintResponseReport {
    pub fn brush_result(&self) -> BrushResult {
        BrushResult {
            changed_vertices: self.changed_vertices,
            displacement_scale: if self.changed_vertices > 0 { 1.0 } else { 0.0 },
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy)]
struct ProposedRow {
    color: [f32; 3],
    material: [f32; 2],
    selected: [bool; 2],
    maximum_weight: f64,
}

impl Mesh {
    /// Atomic opt-in prepared Paint response. Geometry, topology, normals, masks
    /// and UVs do not change. The caller owns one whole Document stroke; material
    /// channels require begin_material_paint_stroke on retained hierarchies.
    pub fn apply_paint_response(
        &mut self,
        request: &PaintResponseRequest,
    ) -> Result<PaintResponseReport, String> {
        self.apply_paint_response_normals(request, None)
    }

    /// Explicit source-profile consumer. A stale field rejects before mutation;
    /// the original prepared-input API retains its prescribed-normal contract.
    pub fn apply_paint_response_with_source_normals(
        &mut self,
        request: &PaintResponseRequest,
        normals: &crate::SourceNormalField,
    ) -> Result<PaintResponseReport, String> {
        if !normals.matches_mesh(self) {
            return Err("Source Paint normal field does not match actual geometry".into());
        }
        self.apply_paint_response_normals(request, Some(normals.vertex_normals()))
    }

    /// New explicit ordinary alphaNone consumer of a validated warm raw cache.
    /// The unchanged row kernel sees the exact same borrowed Vec3 dtype.
    pub fn apply_paint_response_with_retained_source_normals(
        &mut self,
        request: &PaintResponseRequest,
        normals: &crate::SourceGeometryNormalState,
    ) -> Result<PaintResponseReport, String> {
        if request.options.alpha.is_some() || request.mirrored.is_some() || request.first.mirrored_x
        {
            return Err(
                "Retained Source Paint admits alphaNone/unmirrored ordinary dabs only".into(),
            );
        }
        let view = crate::SourcePaintNormalView::RetainedGeometry(normals);
        view.validate(self)
            .map_err(|e| format!("Retained Source Paint normal producer rejected: {e:?}"))?;
        self.apply_paint_response_normals(request, Some(view.vertex_normals()))
    }

    /// Controlled ordinary Paint over the actual immutable warm producer.
    /// Shared request validation still requires finite alpha frames, correct
    /// primary/mirror ordering and equal radii. No field is recomputed or copied.
    /// The old alphaNone-only entry above retains its narrow admission contract.
    pub fn apply_paint_response_with_retained_source_controls(
        &mut self,
        request: &PaintResponseRequest,
        normals: &crate::SourceGeometryNormalState,
    ) -> Result<PaintResponseReport, String> {
        let view = crate::SourcePaintNormalView::RetainedGeometry(normals);
        view.validate(self)
            .map_err(|e| format!("Retained Source Paint control producer rejected: {e:?}"))?;
        self.apply_paint_response_normals(request, Some(view.vertex_normals()))
    }

    fn apply_paint_response_normals(
        &mut self,
        request: &PaintResponseRequest,
        source_normals: Option<&[crate::Vec3]>,
    ) -> Result<PaintResponseReport, String> {
        self.validate()?;
        validate_request(request)?;
        if self.mask_convention != MaskConvention::UnprotectedF32V1 {
            return Err(
                "Prepared source Paint requires canonical unprotected masks; convert explicitly"
                    .into(),
            );
        }
        let mut report = PaintResponseReport::default();
        if !request.in_contact
            || request.options.intensity == 0.0
            || (!request.options.write_rgb
                && !request.options.write_roughness
                && !request.options.write_metallic)
        {
            return Ok(report);
        }
        // Source sphere picking is face-derived. Isolated vertices are not
        // paint candidates; a bounded bit vector records actual connectivity.
        let mut used = Vec::new();
        used.try_reserve_exact(self.positions.len())
            .map_err(|_| "Cannot allocate bounded Paint selection")?;
        used.resize(self.positions.len(), false);
        for triangle in &self.triangles {
            for &vertex in triangle {
                used[vertex as usize] = true;
            }
        }
        // Complete numerical/attribute preflight before allocation or any write.
        for (vertex, &connected) in used.iter().enumerate() {
            if !connected {
                continue;
            }
            let row = proposed_row(self, vertex, request, source_normals)?;
            report.first_selected_vertices += usize::from(row.selected[0]);
            report.mirrored_selected_vertices += usize::from(row.selected[1]);
            report.maximum_weight = report.maximum_weight.max(row.maximum_weight);
            let rgb = row.color != self.colors[vertex];
            let old_material = self
                .materials
                .as_ref()
                .map_or(DEFAULT_VERTEX_MATERIAL, |m| m[vertex]);
            let material = row.material != old_material;
            report.changed_rgb_vertices += usize::from(rgb);
            report.changed_material_vertices += usize::from(material);
            report.changed_vertices += usize::from(rgb || material);
        }
        if report.changed_vertices == 0 {
            return Ok(report);
        }
        let colors_bytes = if report.changed_rgb_vertices > 0 {
            self.positions.len().checked_mul(12)
        } else {
            Some(0)
        };
        let materials_bytes = if report.changed_material_vertices > 0 {
            self.positions.len().checked_mul(8)
        } else {
            Some(0)
        };
        let scratch = colors_bytes
            .zip(materials_bytes)
            .and_then(|(colors, materials)| colors.checked_add(materials))
            .and_then(|bytes| bytes.checked_add(used.capacity()))
            .ok_or("Paint response allocation overflow")?;
        if scratch > MAX_PAINT_RESPONSE_SCRATCH_BYTES {
            return Err("Paint response scratch exceeds its bound".into());
        }
        if report.changed_material_vertices > 0 && self.materials.is_none() {
            let output_bytes = self
                .positions
                .len()
                .checked_mul(8)
                .and_then(|bytes| self.approximate_bytes().checked_add(bytes))
                .ok_or("Paint material allocation overflow")?;
            if output_bytes > crate::decode_limits::MAX_SCENE_STORAGE_BYTES {
                return Err("Paint materials exceed the mesh storage budget".into());
            }
        }
        let mut colors = if report.changed_rgb_vertices > 0 {
            let mut values = Vec::new();
            values
                .try_reserve_exact(self.colors.len())
                .map_err(|_| "Cannot allocate bounded Paint RGB output")?;
            values.extend_from_slice(&self.colors);
            Some(values)
        } else {
            None
        };
        let mut materials = if report.changed_material_vertices > 0 {
            let mut values = Vec::new();
            values
                .try_reserve_exact(self.positions.len())
                .map_err(|_| "Cannot allocate bounded Paint material output")?;
            if let Some(old) = &self.materials {
                values.extend_from_slice(old);
            } else {
                values.resize(self.positions.len(), DEFAULT_VERTEX_MATERIAL);
            }
            Some(values)
        } else {
            None
        };
        // Deterministic replay reads the unchanged original mesh. Even an
        // unexpected second-pass error would leave the original untouched.
        for (vertex, &connected) in used.iter().enumerate() {
            if !connected {
                continue;
            }
            let row = proposed_row(self, vertex, request, source_normals)?;
            if let Some(output) = colors.as_mut() {
                output[vertex] = row.color;
            }
            if let Some(output) = materials.as_mut() {
                output[vertex] = row.material;
            }
        }
        report.materialized = self.materials.is_none() && materials.is_some();
        if let Some(colors) = colors {
            self.colors = colors;
        }
        if let Some(materials) = materials {
            self.materials = Some(materials);
        }
        Ok(report)
    }
}

fn validate_request(request: &PaintResponseRequest) -> Result<(), String> {
    if request.kind != BrushKind::Paint || request.operation != PaintResponseOperation::OrdinaryDab
    {
        return Err("This response kernel supports ordinary fixed-topology Paint only".into());
    }
    let o = &request.options;
    if !o.intensity.is_finite()
        || !o.hardness.is_finite()
        || !(0.0..=1.0).contains(&o.intensity)
        || !(0.0..=1.0).contains(&o.hardness)
        || o.color
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        || !o.roughness.is_finite()
        || !o.metallic.is_finite()
        || !(0.0..=1.0).contains(&o.roughness)
        || !(0.0..=1.0).contains(&o.metallic)
    {
        return Err("Paint response options require finite normalized values".into());
    }
    o.tablet.multiplier()?;
    validate_pass(request.first, o.alpha.is_some())?;
    if let Some(mirrored) = request.mirrored {
        if request.first.mirrored_x || !mirrored.mirrored_x {
            return Err("Two Paint passes must be ordered primary then mirrored".into());
        }
        if mirrored.local_radius_squared != request.first.local_radius_squared {
            return Err("Source symmetry uses the primary prepared local radius squared".into());
        }
        validate_pass(mirrored, o.alpha.is_some())?;
    }
    Ok(())
}
fn validate_pass(pass: PaintResponsePass, alpha: bool) -> Result<(), String> {
    if pass
        .center
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
        || pass
            .eye_direction
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
        || pass.eye_direction.iter().all(|&v| v == 0.0)
    {
        return Err(
            "Prepared Paint center/eye must be finite and bounded; eye must be nonzero".into(),
        );
    }
    validate_radius(pass.local_radius_squared)?;
    if let Some(frame) = pass.alpha_frame {
        if !alpha || frame.look_at.iter().any(|v| !v.is_finite()) {
            return Err(
                "Paint alpha frame requires an active alpha and finite source matrix".into(),
            );
        }
    } else if alpha && pass.local_radius_squared > 0.0 {
        return Err("Source alpha lookup requires an explicit prepared look-at frame".into());
    }
    Ok(())
}
fn validate_radius(radius_squared: f64) -> Result<(), String> {
    if !radius_squared.is_finite() || !(0.0..=1_000_000_000_000.0).contains(&radius_squared) {
        return Err("Prepared Paint radius squared exceeds its finite local bound".into());
    }
    Ok(())
}

fn proposed_row(
    mesh: &Mesh,
    vertex: usize,
    request: &PaintResponseRequest,
    source_normals: Option<&[crate::Vec3]>,
) -> Result<ProposedRow, String> {
    let o = &request.options;
    let mut row = ProposedRow {
        color: mesh.colors[vertex],
        material: mesh
            .materials
            .as_ref()
            .map_or(DEFAULT_VERTEX_MATERIAL, |m| m[vertex]),
        selected: [false; 2],
        maximum_weight: 0.0,
    };
    for (side, pass) in [Some(request.first), request.mirrored]
        .into_iter()
        .enumerate()
    {
        let Some(pass) = pass else {
            continue;
        };
        let [x, y, z] = mesh.positions[vertex].to_array().map(f64::from);
        let [cx, cy, cz] = pass.center;
        let (dx, dy, dz) = (x - cx, y - cy, z - cz);
        let squared = (dx * dx + dy * dy) + dz * dz;
        if squared >= pass.local_radius_squared {
            continue;
        }
        row.selected[side] = true;
        if o.culling {
            let field = source_normals.unwrap_or(mesh.normals.as_slice());
            let [nx, ny, nz] = field[vertex].to_array().map(f64::from);
            let [ex, ey, ez] = pass.eye_direction;
            if (nx * ex + ny * ey) + nz * ez > 0.0 {
                continue;
            }
        }
        let distance = squared.sqrt() / pass.local_radius_squared.sqrt();
        let radial = (1.0 - distance.min(1.0)).powf(2.0 * (1.0 - o.hardness));
        let alpha = if let Some(alpha) = &o.alpha {
            source_alpha(alpha, pass, mesh.positions[vertex])?
        } else {
            1.0
        };
        let weight = radial
            * (((o.intensity * o.tablet.multiplier()?) * f64::from(mesh.masks[vertex])) * alpha);
        if !weight.is_finite() {
            return Err("Source Paint weight is nonfinite".into());
        }
        row.maximum_weight = row.maximum_weight.max(weight);
        if weight == 0.0 {
            continue;
        }
        if o.write_rgb {
            for (value, target) in row.color.iter_mut().zip(o.color) {
                *value = blend(*value, target, weight)?;
            }
        }
        for (channel, flag) in [o.write_roughness, o.write_metallic]
            .into_iter()
            .enumerate()
        {
            if flag {
                row.material[channel] = blend(
                    row.material[channel],
                    [o.roughness, o.metallic][channel],
                    weight,
                )?;
            }
        }
    }
    Ok(row)
}
fn blend(old: f32, target: f32, weight: f64) -> Result<f32, String> {
    let next = (f64::from(old) * (1.0 - weight) + f64::from(target) * weight) as f32;
    if !next.is_finite() || !(0.0..=1.0).contains(&next) {
        return Err("Source Paint extrapolation would violate normalized attribute bounds".into());
    }
    // Real no-ops retain existing signed-zero words and never manufacture undo.
    Ok(if next == old { old } else { next })
}
pub(crate) fn source_alpha(
    alpha: &RasterAlpha,
    pass: PaintResponsePass,
    position: Vec3,
) -> Result<f64, String> {
    let frame = pass.alpha_frame.ok_or("Missing source alpha frame")?;
    let m = frame.look_at;
    let [x, y, z] = position.to_array().map(f64::from);
    let side = pass.local_radius_squared.sqrt() * std::f64::consts::FRAC_1_SQRT_2;
    let ratio_x = (f64::from(alpha.width()) / f64::from(alpha.height())).max(1.0);
    let ratio_y = (f64::from(alpha.height()) / f64::from(alpha.width())).max(1.0);
    let sx = ((f64::from(m[0]) * x + f64::from(m[4]) * y) + f64::from(m[8]) * z) + f64::from(m[12]);
    let sy = ((f64::from(m[1]) * x + f64::from(m[5]) * y) + f64::from(m[9]) * z) + f64::from(m[13]);
    let xn = ratio_y * sx / if pass.mirrored_x { -side } else { side };
    let yn = ratio_x * sy / side;
    if !xn.is_finite() || !yn.is_finite() {
        return Err("Source alpha coordinates are nonfinite".into());
    }
    if xn.abs() > 1.0 || yn.abs() > 1.0 {
        return Ok(0.0);
    }
    let column = ((0.5 - xn * 0.5) * f64::from(alpha.width())) as usize;
    let row = ((0.5 - yn * 0.5) * f64::from(alpha.height())) as usize;
    // Preserve source flat-index row aliasing at exact X boundaries. A global
    // out-of-range source lookup would be undefined/NaN; reject atomically.
    let index = column
        .checked_add(
            (alpha.width() as usize)
                .checked_mul(row)
                .ok_or("Alpha index overflow")?,
        )
        .ok_or("Alpha index overflow")?;
    alpha
        .pixels()
        .get(index)
        .map(|&value| f64::from(value) / 255.0)
        .ok_or_else(|| "Source alpha lookup is undefined at this exact boundary".into())
}

/// Additive Source Picking _alphaSide entry; prior prescribed Paint sampler
/// remains byte exact. Only explicit side/frame inputs differ, ratio/index body
/// preserves original arithmetic/nearest/flat-edge alias semantics.
pub(crate) fn source_alpha_prepared_side(
    alpha: &RasterAlpha,
    m: [f32; 16],
    side: f64,
    mirrored: bool,
    position: Vec3,
) -> Result<f64, String> {
    let [x, y, z] = position.to_array().map(f64::from);
    let ratio_x = (f64::from(alpha.width()) / f64::from(alpha.height())).max(1.0);
    let ratio_y = (f64::from(alpha.height()) / f64::from(alpha.width())).max(1.0);
    let sx = ((f64::from(m[0]) * x + f64::from(m[4]) * y) + f64::from(m[8]) * z) + f64::from(m[12]);
    let sy = ((f64::from(m[1]) * x + f64::from(m[5]) * y) + f64::from(m[9]) * z) + f64::from(m[13]);
    let xn = ratio_y * sx / if mirrored { -side } else { side };
    let yn = ratio_x * sy / side;
    if !xn.is_finite() || !yn.is_finite() {
        return Err("Source alpha coordinates are nonfinite".into());
    }
    if xn.abs() > 1.0 || yn.abs() > 1.0 {
        return Ok(0.0);
    }
    let column = ((0.5 - xn * 0.5) * f64::from(alpha.width())) as usize;
    let row = ((0.5 - yn * 0.5) * f64::from(alpha.height())) as usize;
    // Preserve source flat-index row aliasing at exact X boundaries. A global
    // out-of-range source lookup would be undefined/NaN; reject atomically.
    let index = column
        .checked_add(
            (alpha.width() as usize)
                .checked_mul(row)
                .ok_or("Alpha index overflow")?,
        )
        .ok_or("Alpha index overflow")?;
    alpha
        .pixels()
        .get(index)
        .map(|&value| f64::from(value) / 255.0)
        .ok_or_else(|| "Source alpha lookup is undefined at this exact boundary".into())
}
