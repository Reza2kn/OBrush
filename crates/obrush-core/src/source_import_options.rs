//! Import-only source options. Owned staged objects are prepared before a
//! Document exchange; local geometry and the immutable decoded origin stay raw.
//! Portable scalar policy is shared by native/WASM. Exact V8 pow/hypot outcomes
//! require the separately registered reference checks, not a formula claim.
use crate::{
    Mesh, ObjectAffineF32V1, SceneObjectMetadata, SglPickingPolicy, StagedSglScene, Vec3,
    MAX_STAGED_SCENE_BYTES, MAX_SUBTOOLS,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourceImportOptions {
    pub scale_center: bool,
    /// True retains encoded sRGB values. False encodes incoming linear RGB.
    pub vertex_srgb: bool,
}
impl Default for SourceImportOptions {
    fn default() -> Self {
        Self {
            scale_center: false,
            vertex_srgb: true,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct SourceImportObjectReport {
    pub ordinal: usize,
    pub initial_local_bound: [f64; 6],
    pub initial_world_bound: [f64; 6],
    pub matrix_words: [u32; 16],
    pub local_center_words: [u32; 3],
    pub source_scale_squared_before: f64,
    pub source_scale_squared_after: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct SourceImportNormalization {
    pub incoming_world_bound: [f64; 6],
    pub diagonal: f64,
    pub scale_binary64: f64,
    pub normalization_matrix_words: [u32; 16],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SourceImportCenterReason {
    #[serde(rename = "source_initial_loose_referenced_f32_v1")]
    InitialLooseReferencedF32V1,
}
#[derive(Clone, Debug, Serialize)]
pub struct SourceImportCenterReport {
    pub ordinal: usize,
    pub referenced_triangle_corners: usize,
    /// Encoded incoming center is archival data; it is never overwritten.
    pub archived_center_words: [u32; 3],
    /// Editable current center before this option staging operation.
    pub previous_center_words: [u32; 3],
    pub center_words: [u32; 3],
    pub reason: SourceImportCenterReason,
}
#[derive(Clone, Debug, Serialize)]
pub struct SourceImportOptionsReport {
    pub options: SourceImportOptions,
    pub objects: usize,
    pub converted_explicit_rgb_vertices: usize,
    pub normalization: Option<SourceImportNormalization>,
    pub object_reports: Vec<SourceImportObjectReport>,
    pub initialized_centers: Vec<SourceImportCenterReport>,
    /// Explicit bounded option report/temp storage; not process/GPU memory.
    pub options_storage_bytes_bound: usize,
}
impl SourceImportOptionsReport {
    fn unchanged(options: SourceImportOptions, objects: usize) -> Self {
        Self {
            options,
            objects,
            converted_explicit_rgb_vertices: 0,
            normalization: None,
            object_reports: Vec::new(),
            initialized_centers: Vec::new(),
            options_storage_bytes_bound: 0,
        }
    }
}
#[derive(Debug)]
pub struct PreparedSourceImportOptions {
    pub staged: StagedSglScene,
    pub report: SourceImportOptionsReport,
}

/// Source File Add preparation for a single OBJ/PLY/STL mesh. The mesh retains
/// local positions; normalization is represented by the same editable affine
/// used for SGL objects. The caller must pass whether RGB was explicit in the
/// input, because plain mesh defaults must never be color-space converted.
pub struct PreparedSourceMeshImport {
    pub mesh: Mesh,
    pub metadata: SceneObjectMetadata,
    pub report: SourceImportOptionsReport,
}

pub struct PreparedSourceMeshGroup {
    pub parts: Vec<PreparedSourceMeshImport>,
    pub report: SourceImportOptionsReport,
}

/// Prepare one normalization matrix for the entire incoming OBJ/PLY/STL group,
/// matching the pinned `normalizeAndCenterMeshes(newMeshes)` boundary. All
/// source meshes remain local; only their checked metadata receives the matrix.
pub fn prepare_source_mesh_group(
    inputs: Vec<(Mesh, bool)>,
    options: SourceImportOptions,
) -> Result<PreparedSourceMeshGroup, String> {
    if inputs.is_empty() || inputs.len() > MAX_SUBTOOLS {
        return Err("Source mesh group requires 1–32 parts".into());
    }
    let count = inputs.len();
    let mut parts = Vec::new();
    parts
        .try_reserve_exact(count)
        .map_err(|_| "Cannot allocate bounded Source mesh group")?;
    let mut report = SourceImportOptionsReport::unchanged(options, count);
    report
        .object_reports
        .try_reserve_exact(count)
        .map_err(|_| "Cannot allocate bounded Source mesh option report")?;
    let mut bounds = [BoundInput::EMPTY; MAX_SUBTOOLS];
    for (ordinal, (mesh, explicit_rgb)) in inputs.into_iter().enumerate() {
        let part = prepare_source_mesh_import(
            mesh,
            explicit_rgb,
            SourceImportOptions {
                scale_center: false,
                vertex_srgb: options.vertex_srgb,
            },
        )?;
        bounds[ordinal] = BoundInput {
            local: part.report.object_reports[0].initial_local_bound,
            matrix: part.metadata.affine.local_to_world,
            visible: true,
        };
        report.converted_explicit_rgb_vertices = report
            .converted_explicit_rgb_vertices
            .checked_add(part.report.converted_explicit_rgb_vertices)
            .ok_or("Source RGB row count overflow")?;
        let mut object_report = part.report.object_reports[0].clone();
        object_report.ordinal = ordinal;
        report.object_reports.push(object_report);
        parts.push(part);
    }
    if options.scale_center {
        let bound = union_visible(&bounds[..count])?;
        let (matrix, diagonal, scale) = normalization_matrix(bound)?;
        report.normalization = Some(SourceImportNormalization {
            incoming_world_bound: bound,
            diagonal,
            scale_binary64: scale,
            normalization_matrix_words: matrix.map(f32::to_bits),
        });
        for (part, row) in parts.iter_mut().zip(&mut report.object_reports) {
            part.metadata.affine.local_to_world = matrix;
            part.metadata.validate(&part.mesh)?;
            row.matrix_words = matrix.map(f32::to_bits);
            row.source_scale_squared_after = part.metadata.affine.source_scale_squared();
        }
    }
    report.options_storage_bytes_bound = report
        .object_reports
        .capacity()
        .checked_mul(std::mem::size_of::<SourceImportObjectReport>())
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<[BoundInput; MAX_SUBTOOLS]>()))
        .ok_or("Source group report storage overflow")?;
    Ok(PreparedSourceMeshGroup { parts, report })
}

pub fn prepare_source_mesh_import(
    mut mesh: Mesh,
    explicit_rgb: bool,
    options: SourceImportOptions,
) -> Result<PreparedSourceMeshImport, String> {
    mesh.validate()?;
    let mut report = SourceImportOptionsReport::unchanged(options, 1);
    if !options.vertex_srgb && explicit_rgb {
        for rgb in &mut mesh.colors {
            for component in rgb {
                *component = encode_linear_rgb(*component)?;
            }
        }
        report.converted_explicit_rgb_vertices = mesh.colors.len();
    }
    let local = initial_local_bound(&mesh.positions)?;
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let (matrix, scale_before, scale_after) = if options.scale_center {
        let (matrix, diagonal, scale) = normalization_matrix(local)?;
        report.normalization = Some(SourceImportNormalization {
            incoming_world_bound: local,
            diagonal,
            scale_binary64: scale,
            normalization_matrix_words: matrix.map(f32::to_bits),
        });
        (matrix, 1.0, f64::from(matrix[0]).powi(2))
    } else {
        (identity, 1.0, 1.0)
    };
    let mut affine = ObjectAffineF32V1::from_local_points(matrix, &mesh.positions)?;
    let referenced = initial_referenced_bound(&mesh)?;
    affine.interaction_center =
        std::array::from_fn(|axis| ((referenced[axis] + referenced[axis + 3]) * 0.5) as f32);
    let metadata = SceneObjectMetadata {
        source_center_initialized: true,
        affine,
        render: None,
        picking_policy: SglPickingPolicy::SourceScaleRowV1,
        origin: None,
    };
    metadata.validate(&mesh)?;
    mesh.validate()?;
    report.object_reports.push(SourceImportObjectReport {
        ordinal: 0,
        initial_local_bound: local,
        initial_world_bound: local,
        matrix_words: matrix.map(f32::to_bits),
        local_center_words: metadata.affine.interaction_center.map(f32::to_bits),
        source_scale_squared_before: scale_before,
        source_scale_squared_after: scale_after,
    });
    report.options_storage_bytes_bound = std::mem::size_of::<SourceImportObjectReport>();
    Ok(PreparedSourceMeshImport {
        mesh,
        metadata,
        report,
    })
}

impl StagedSglScene {
    /// Consuming staging keeps every failure outside the receiving Document.
    /// Every new Source object initializes its editable center, even when no
    /// normalization/RGB conversion is requested. General affine/whole-Open
    /// constructors and immutable raw archives retain their previous policy.
    pub fn prepare_source_import_options(
        mut self,
        options: SourceImportOptions,
    ) -> Result<PreparedSourceImportOptions, String> {
        self.validate()?;
        let mut report = SourceImportOptionsReport::unchanged(options, self.objects.len());
        if self.objects.is_empty() {
            return Ok(PreparedSourceImportOptions {
                staged: self,
                report,
            });
        }
        report
            .initialized_centers
            .try_reserve_exact(self.objects.len())
            .map_err(|_| "Source center report allocation failed")?;
        let center_extra = report
            .initialized_centers
            .capacity()
            .checked_mul(std::mem::size_of::<SourceImportCenterReport>())
            .and_then(|n| n.checked_add(std::mem::size_of::<[[u32; 3]; MAX_SUBTOOLS]>() + 128))
            .ok_or("Source center storage accounting overflow")?;
        ensure_storage_bound(self.storage_bytes()?, center_extra)?;
        report.options_storage_bytes_bound = center_extra;
        let mut previous_centers = [[0u32; 3]; MAX_SUBTOOLS];
        for (ordinal, object) in self.objects.iter().enumerate() {
            previous_centers[ordinal] = object.affine.interaction_center.map(f32::to_bits);
        }
        if !options.vertex_srgb {
            for object in &mut self.objects {
                if object.origin.raw.colors.is_none() {
                    continue;
                }
                // Mesh RGB is an owned editable Vec. Its Arc raw origin is never
                // made mutable or cloned into a second full transformation buffer.
                for rgb in &mut object.mesh.colors {
                    for component in rgb {
                        *component = encode_linear_rgb(*component)?;
                    }
                }
                report.converted_explicit_rgb_vertices = report
                    .converted_explicit_rgb_vertices
                    .checked_add(object.mesh.colors.len())
                    .ok_or("Source RGB row count overflow")?;
            }
        }
        if options.scale_center {
            report
                .object_reports
                .try_reserve_exact(self.objects.len())
                .map_err(|_| "Source normalization report allocation failed")?;
            // SGL has no visibility words: every newly imported object is visible.
            // Existing receiving membership/visibility/Solo is deliberately absent.
            let mut bounds = [BoundInput::EMPTY; MAX_SUBTOOLS];
            for (ordinal, object) in self.objects.iter().enumerate() {
                let local = initial_local_bound(&object.mesh.positions)?;
                bounds[ordinal] = BoundInput {
                    local,
                    matrix: object.affine.local_to_world,
                    visible: true,
                };
                report.object_reports.push(SourceImportObjectReport {
                    ordinal,
                    initial_local_bound: local,
                    initial_world_bound: world_bound(local, &object.affine.local_to_world)?,
                    matrix_words: object.affine.local_to_world.map(f32::to_bits),
                    local_center_words: object.affine.interaction_center.map(f32::to_bits),
                    source_scale_squared_before: object.affine.source_scale_squared(),
                    source_scale_squared_after: object.affine.source_scale_squared(),
                });
            }
            // The stack descriptors and result report are accounted before any
            // matrix mutation. No unbounded second point/color/topology arrays.
            let extra = report
                .object_reports
                .capacity()
                .checked_mul(std::mem::size_of::<SourceImportObjectReport>())
                .and_then(|n| {
                    n.checked_add(std::mem::size_of::<[BoundInput; MAX_SUBTOOLS]>() + 512)
                })
                .ok_or("Source options storage accounting overflow")?;
            let combined = report
                .options_storage_bytes_bound
                .checked_add(extra)
                .ok_or("Source center/normalization accounting overflow")?;
            ensure_storage_bound(self.storage_bytes()?, combined)?;
            report.options_storage_bytes_bound = combined;
            let bound = union_visible(&bounds[..self.objects.len()])?;
            let (matrix, diagonal, scale) = normalization_matrix(bound)?;
            report.normalization = Some(SourceImportNormalization {
                incoming_world_bound: bound,
                diagonal,
                scale_binary64: scale,
                normalization_matrix_words: matrix.map(f32::to_bits),
            });
            for (object, row) in self.objects.iter_mut().zip(&mut report.object_reports) {
                object.affine.local_to_world = multiply_f32(matrix, object.affine.local_to_world);
                // Preserve the normalization report calculation here; the source
                // initial center is installed for every object below.
                object.affine.interaction_center = std::array::from_fn(|axis| {
                    ((row.initial_local_bound[axis] + row.initial_local_bound[axis + 3]) * 0.5)
                        as f32
                });
                object.affine.validate()?;
                for &point in &object.mesh.positions {
                    object.affine.world_point_validated(point)?;
                }
                row.matrix_words = object.affine.local_to_world.map(f32::to_bits);
                row.local_center_words = object.affine.interaction_center.map(f32::to_bits);
                row.source_scale_squared_after = object.affine.source_scale_squared();
            }
        }
        // This runs last so normalization cannot overwrite the referenced-center
        // semantics. It changes metadata only, never local/raw point words.
        for (ordinal, object) in self.objects.iter_mut().enumerate() {
            let bound = initial_referenced_bound(&object.mesh)?;
            let center = std::array::from_fn(|axis| ((bound[axis] + bound[axis + 3]) * 0.5) as f32);
            object.affine.interaction_center = center;
            object.source_center_initialized = true;
            object.affine.validate()?;
            if let Some(row) = report.object_reports.get_mut(ordinal) {
                row.local_center_words = center.map(f32::to_bits);
            }
            report.initialized_centers.push(SourceImportCenterReport {
                ordinal,
                referenced_triangle_corners: object
                    .mesh
                    .triangles
                    .len()
                    .checked_mul(3)
                    .ok_or("Source center corner count overflow")?,
                archived_center_words: object.origin.raw.center.map(f32::to_bits),
                previous_center_words: previous_centers[ordinal],
                center_words: center.map(f32::to_bits),
                reason: SourceImportCenterReason::InitialLooseReferencedF32V1,
            });
        }
        self.validate()?;
        ensure_storage_bound(self.storage_bytes()?, report.options_storage_bytes_bound)?;
        Ok(PreparedSourceImportOptions {
            staged: self,
            report,
        })
    }
}
fn ensure_storage_bound(staged: usize, extra: usize) -> Result<(), String> {
    if staged
        .checked_add(extra)
        .is_none_or(|total| total > MAX_STAGED_SCENE_BYTES)
    {
        return Err("Source import options exceed128MiB combined staged/options storage".into());
    }
    Ok(())
}
fn encode_linear_rgb(input: f32) -> Result<f32, String> {
    if !input.is_finite() || !(0.0..=1.0).contains(&input) {
        return Err("Source incoming RGB must be finite0..1 before conversion".into());
    }
    let x = f64::from(input);
    let output = if x < 0.0031308 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    let stored = output as f32;
    if !stored.is_finite() || !(0.0..=1.0).contains(&stored) {
        return Err("Source RGB conversion output exceeds finite0..1".into());
    }
    Ok(stored)
}
/// Source init follows optimized face membership, while this importer retains
/// raw/editable unused vertices. The scalar center is order-independent after
/// initial loose padding; this is not an optimizer/bound-endpoint parity claim.
fn initial_referenced_bound(mesh: &Mesh) -> Result<[f64; 6], String> {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for triangle in &mesh.triangles {
        for &index in triangle {
            let point = mesh
                .positions
                .get(index as usize)
                .ok_or("Source center referenced point is out of range")?;
            for (axis, input) in point.to_array().into_iter().enumerate() {
                let x = f64::from(input);
                if !x.is_finite() {
                    return Err("Source center requires finite points".into());
                }
                if x < b[axis] {
                    b[axis] = x;
                }
                if x > b[axis + 3] {
                    b[axis + 3] = x;
                }
            }
        }
    }
    if !b.iter().all(|v| v.is_finite()) {
        return Err("Source center requires nonempty referenced geometry".into());
    }
    let d: [f64; 3] = std::array::from_fn(|i| b[i + 3] - b[i]);
    let offset = ((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]).sqrt() * 0.2;
    for axis in 0..3 {
        if d[axis] < 1e-16 {
            b[axis] -= offset;
            b[axis + 3] += offset;
        }
    }
    if !b.iter().all(|v| v.is_finite()) {
        return Err("Source center initial loose bound overflow".into());
    }
    Ok(b)
}
fn initial_local_bound(points: &[Vec3]) -> Result<[f64; 6], String> {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for &point in points {
        for (axis, input) in point.to_array().into_iter().enumerate() {
            let x = f64::from(input);
            if !x.is_finite() {
                return Err("Nonfinite source bound point".into());
            }
            if x < b[axis] {
                b[axis] = x;
            }
            if x > b[axis + 3] {
                b[axis + 3] = x;
            }
        }
    }
    if !b.iter().all(|v| v.is_finite()) {
        return Err("Source bounds require nonempty finite points".into());
    }
    let d: [f64; 3] = std::array::from_fn(|i| b[i + 3] - b[i]);
    // Source initial loose padding uses sqrt(sum), unlike its hypot diagonal.
    let offset = ((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2]).sqrt() * 0.2;
    for axis in 0..3 {
        if d[axis] < 1e-16 {
            b[axis] -= offset;
            b[axis + 3] += offset;
        }
    }
    if !b.iter().all(|v| v.is_finite()) {
        return Err("Source initial loose bound overflow".into());
    }
    Ok(b)
}
#[derive(Clone, Copy)]
struct BoundInput {
    local: [f64; 6],
    matrix: [f32; 16],
    visible: bool,
}
impl BoundInput {
    const EMPTY: Self = Self {
        local: [0.0; 6],
        matrix: [0.0; 16],
        visible: false,
    };
}
fn world_bound(local: [f64; 6], matrix: &[f32; 16]) -> Result<[f64; 6], String> {
    let m = matrix.map(f64::from);
    let mut b = [m[12], m[13], m[14], m[12], m[13], m[14]];
    for axis in 0..3 {
        for component in 0..3 {
            let a = m[axis * 4 + component] * local[axis + 3];
            let z = m[axis * 4 + component] * local[axis];
            if a < z {
                b[component] += a;
                b[component + 3] += z;
            } else {
                b[component] += z;
                b[component + 3] += a;
            }
        }
    }
    if !b.iter().all(|v| v.is_finite()) {
        return Err("Source world bound overflow".into());
    }
    Ok(b)
}
fn union_visible(objects: &[BoundInput]) -> Result<[f64; 6], String> {
    if objects.len() > MAX_SUBTOOLS {
        return Err("Source bound group exceeds32 objects".into());
    }
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for object in objects.iter().filter(|o| o.visible) {
        let next = world_bound(object.local, &object.matrix)?;
        for axis in 0..3 {
            if next[axis] < b[axis] {
                b[axis] = next[axis];
            }
            if next[axis + 3] > b[axis + 3] {
                b[axis + 3] = next[axis + 3];
            }
        }
    }
    if !b.iter().all(|v| v.is_finite()) {
        return Err("No finite visible incoming source bound".into());
    }
    Ok(b)
}
/// Three-argument portable scaled/compensated norm. This is independently
/// written binary64 arithmetic, not an assertion of a particular V8 algorithm.
fn portable_hypot3(d: [f64; 3]) -> f64 {
    let maximum = d[0].abs().max(d[1].abs()).max(d[2].abs());
    if maximum == 0.0 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for x in d {
        let n = x / maximum;
        let adjusted = n * n - compensation;
        let total = sum + adjusted;
        compensation = (total - sum) - adjusted;
        sum = total;
    }
    maximum * sum.sqrt()
}
fn normalization_matrix(bound: [f64; 6]) -> Result<([f32; 16], f64, f64), String> {
    let diagonal = portable_hypot3(std::array::from_fn(|axis| bound[axis + 3] - bound[axis]));
    let scale = 100.0 / diagonal;
    if !diagonal.is_finite() || diagonal <= 0.0 || !scale.is_finite() || scale <= 0.0 {
        return Err("Source normalization needs a finite positive diagonal and scale".into());
    }
    let mut m = [0.0_f32; 16];
    for i in [0, 5, 10, 15] {
        m[i] = 1.0;
    }
    for column in 0..3 {
        for row in 0..4 {
            m[column * 4 + row] = (f64::from(m[column * 4 + row]) * scale) as f32;
        }
    }
    let center: [f64; 3] = std::array::from_fn(|axis| -(bound[axis] + bound[axis + 3]) * 0.5);
    for row in 0..4 {
        m[12 + row] = (((f64::from(m[row]) * center[0] + f64::from(m[row + 4]) * center[1])
            + f64::from(m[row + 8]) * center[2])
            + f64::from(m[row + 12])) as f32;
    }
    if m.iter().any(|v| !v.is_finite()) {
        return Err("Source normalization float32 matrix overflow".into());
    }
    Ok((m, diagonal, scale))
}
fn multiply_f32(lhs: [f32; 16], rhs: [f32; 16]) -> [f32; 16] {
    let a = lhs.map(f64::from);
    let b = rhs.map(f64::from);
    std::array::from_fn(|index| {
        let row = index % 4;
        let column = (index / 4) * 4;
        (((b[column] * a[row] + b[column + 1] * a[row + 4]) + b[column + 2] * a[row + 8])
            + b[column + 3] * a[row + 12]) as f32
    })
}

#[cfg(test)]
#[path = "source_import_options_tests.rs"]
mod tests;
