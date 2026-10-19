//! Explicit whole-mesh channel fill and read-only paint sampling. Fill uses
//! mask weight only. Picker modes are actual distinct interpolation policies;
//! numerical source/runtime equivalence is not established by these APIs.
use crate::{
    BrushKind, BrushSettings, Hit, Mesh, PaintResponseOptions, DEFAULT_VERTEX_MATERIAL,
    TRIANGLE_CORNER,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaintFillReport {
    pub changed_vertices: usize,
    pub changed_rgb_vertices: usize,
    pub changed_material_vertices: usize,
    pub materialized: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintPickInterpolation {
    TriangleBarycentric,
    OriginalFaceInverseDistance,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintSample {
    pub rgb: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
    pub materials_explicit: bool,
    pub triangle_index: usize,
    pub original_face_index: Option<usize>,
    pub interpolation: PaintPickInterpolation,
}

impl Mesh {
    /// Fill selected channels by the unmasked fraction. Radius, intensity,
    /// pressure, alpha, hardness, culling and symmetry are irrelevant to a fill.
    /// Targets/kind/mesh are validated before writing; material allocation and
    /// storage limits are checked before any RGB or scalar edit.
    /// Group this operation through Document's material-paint transaction when
    /// either material channel is selected; Mesh alone does not own history.
    pub fn fill_paint_channels(
        &mut self,
        settings: &BrushSettings,
    ) -> Result<PaintFillReport, String> {
        if settings.kind != BrushKind::Paint
            || !settings.paint_targets_are_valid()
            || settings.color.iter().any(|value| !value.is_finite())
        {
            return Err(
                "Fill requires Paint with finite RGB and normalized material targets".into(),
            );
        }
        let rgb_target = if settings.invert {
            [0.65, 0.69, 0.74]
        } else {
            settings.color.map(|v| v.clamp(0.0, 1.0))
        };
        let material_target = if settings.invert {
            DEFAULT_VERTEX_MATERIAL
        } else {
            [settings.roughness, settings.metallic]
        };
        self.fill_paint_targets(
            rgb_target,
            material_target,
            [
                settings.paint_rgb,
                settings.paint_roughness,
                settings.paint_metallic,
            ],
        )
    }

    /// Source Paint All uses the active Source Paint targets and channel flags.
    /// It blends once by each vertex's unmasked fraction; brush intensity,
    /// radius, alpha, culling, tablet pressure and symmetry do not participate.
    pub fn fill_paint_channels_source(
        &mut self,
        options: &PaintResponseOptions,
    ) -> Result<PaintFillReport, String> {
        if options
            .color
            .iter()
            .chain([&options.roughness, &options.metallic])
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err("Source Paint All targets must be finite and normalized".into());
        }
        self.fill_paint_targets(
            options.color,
            [options.roughness, options.metallic],
            [
                options.write_rgb,
                options.write_roughness,
                options.write_metallic,
            ],
        )
    }

    fn fill_paint_targets(
        &mut self,
        rgb_target: [f32; 3],
        material_target: [f32; 2],
        channels: [bool; 3],
    ) -> Result<PaintFillReport, String> {
        self.validate()?;
        if !channels.iter().any(|enabled| *enabled) {
            return Ok(PaintFillReport::default());
        }
        let material_flags = [channels[1], channels[2]];
        let mut report = PaintFillReport::default();
        for vertex in 0..self.positions.len() {
            let weight = self.mask_influence_f64(vertex);
            let rgb_changed = channels[0]
                && self.colors[vertex]
                    .iter()
                    .zip(rgb_target)
                    .any(|(&old, target)| fill_scalar(old, target, weight) != old);
            let previous = self
                .materials
                .as_ref()
                .map_or(DEFAULT_VERTEX_MATERIAL, |m| m[vertex]);
            let material_changed = previous
                .iter()
                .zip(material_target)
                .zip(material_flags)
                .any(|((&old, target), enabled)| {
                    enabled && fill_scalar(old, target, weight) != old
                });
            report.changed_rgb_vertices += usize::from(rgb_changed);
            report.changed_material_vertices += usize::from(material_changed);
            report.changed_vertices += usize::from(rgb_changed || material_changed);
        }
        if report.changed_vertices == 0 {
            return Ok(report);
        }
        if report.changed_material_vertices > 0 && self.materials.is_none() {
            let storage = self
                .positions
                .len()
                .checked_mul(std::mem::size_of::<[f32; 2]>())
                .and_then(|bytes| self.approximate_bytes().checked_add(bytes))
                .ok_or("Fill material storage overflow")?;
            if storage > crate::decode_limits::MAX_SCENE_STORAGE_BYTES {
                return Err("Fill material channels exceed the mesh storage budget".into());
            }
            let mut values = Vec::new();
            values
                .try_reserve_exact(self.positions.len())
                .map_err(|_| "Cannot allocate fill material channels")?;
            values.resize(self.positions.len(), DEFAULT_VERTEX_MATERIAL);
            self.materials = Some(values);
            report.materialized = true;
        }
        for vertex in 0..self.positions.len() {
            let weight = self.mask_influence_f64(vertex);
            if channels[0] {
                for (value, target) in self.colors[vertex].iter_mut().zip(rgb_target) {
                    *value = fill_scalar(*value, target, weight);
                }
            }
            if let Some(materials) = &mut self.materials {
                for ((value, target), enabled) in materials[vertex]
                    .iter_mut()
                    .zip(material_target)
                    .zip(material_flags)
                {
                    if enabled {
                        *value = fill_scalar(*value, target, weight);
                    }
                }
            }
        }
        Ok(report)
    }

    /// Resolve a canonical picked triangle only when its indices identify one
    /// render row uniquely. Duplicate rows require explicit triangle identity.
    pub fn pick_paint_channels(
        &self,
        hit: &Hit,
        interpolation: PaintPickInterpolation,
    ) -> Result<PaintSample, String> {
        self.validate()?;
        let mut matches = self
            .triangles
            .iter()
            .enumerate()
            .filter(|(_, triangle)| **triangle == hit.triangle);
        let (index, _) = matches
            .next()
            .ok_or("Picked triangle no longer belongs to this mesh")?;
        if matches.next().is_some() {
            return Err(
                "Picked triangle identity is ambiguous; supply its explicit render index".into(),
            );
        }
        self.pick_paint_channels_validated(hit, index, interpolation)
    }

    /// An explicit render triangle index may disambiguate duplicated original
    /// faces; it must agree with the captured Hit's canonical ordered indices.
    pub fn pick_paint_channels_at_triangle(
        &self,
        hit: &Hit,
        triangle_index: usize,
        interpolation: PaintPickInterpolation,
    ) -> Result<PaintSample, String> {
        self.validate()?;
        self.pick_paint_channels_validated(hit, triangle_index, interpolation)
    }

    fn pick_paint_channels_validated(
        &self,
        hit: &Hit,
        triangle_index: usize,
        interpolation: PaintPickInterpolation,
    ) -> Result<PaintSample, String> {
        if self.triangles.get(triangle_index) != Some(&hit.triangle) {
            return Err("Explicit picked triangle index does not match its Hit".into());
        }
        if !hit.position.is_finite()
            || !hit.normal.is_finite()
            || !hit.distance.is_finite()
            || hit.distance < 0.0
        {
            return Err("Picked Hit contains nonfinite or invalid geometry metadata".into());
        }
        let tolerance = 8.0 * f64::from(f32::EPSILON);
        if hit
            .barycentric
            .iter()
            .any(|&v| !v.is_finite() || f64::from(v) < -tolerance || f64::from(v) > 1.0 + tolerance)
        {
            return Err("Picked barycentric weights are invalid".into());
        }
        let sum: f64 = hit.barycentric.iter().map(|&v| f64::from(v)).sum();
        if (sum - 1.0).abs() > tolerance {
            return Err("Picked barycentric weights do not sum to one".into());
        }
        let mut barycentric = hit.barycentric.map(|v| f64::from(v).clamp(0.0, 1.0));
        let sum: f64 = barycentric.iter().sum();
        for v in &mut barycentric {
            *v /= sum;
        }
        let positions = hit.triangle.map(|v| self.positions[v as usize].as_dvec3());
        if (positions[1] - positions[0])
            .cross(positions[2] - positions[0])
            .length_squared()
            == 0.0
        {
            return Err("Picked triangle is collapsed".into());
        }
        // Like Hit::surface_point, weights follow current vertex geometry.
        // The old ray distance/point do not silently redefine interpolation.
        let point = positions
            .into_iter()
            .zip(barycentric)
            .fold(glam::DVec3::ZERO, |sum, (p, w)| sum + p * w);
        let (face_index, face) = self.paint_original_face(triangle_index);
        let (vertices, weights, count) = match interpolation {
            PaintPickInterpolation::TriangleBarycentric => (
                [hit.triangle[0], hit.triangle[1], hit.triangle[2], 0],
                [barycentric[0], barycentric[1], barycentric[2], 0.0],
                3,
            ),
            PaintPickInterpolation::OriginalFaceInverseDistance => {
                let face = face.unwrap_or([
                    hit.triangle[0],
                    hit.triangle[1],
                    hit.triangle[2],
                    TRIANGLE_CORNER,
                ]);
                let count = if face[3] == TRIANGLE_CORNER { 3 } else { 4 };
                let mut distances = [0.0_f64; 4];
                for (distance, &vertex) in distances[..count].iter_mut().zip(&face[..count]) {
                    *distance = self.positions[vertex as usize].as_dvec3().distance(point);
                }
                let mut coincident = None;
                for (corner, &distance) in distances[..count].iter().enumerate() {
                    if distance == 0.0 && coincident.replace(corner).is_some() {
                        return Err("Original picked face has ambiguous coincident corners".into());
                    }
                }
                let mut weights = [0.0; 4];
                if let Some(corner) = coincident {
                    weights[corner] = 1.0;
                } else {
                    let smallest = distances[..count]
                        .iter()
                        .copied()
                        .fold(f64::INFINITY, f64::min);
                    for (weight, &distance) in weights[..count].iter_mut().zip(&distances[..count])
                    {
                        *weight = smallest / distance;
                    }
                    let sum: f64 = weights[..count].iter().sum();
                    for weight in &mut weights[..count] {
                        *weight /= sum;
                    }
                }
                (face, weights, count)
            }
        };
        let sample_scalar = |channel: usize, material: bool| {
            if let Some(corner) = weights[..count].iter().position(|&w| w == 1.0) {
                let vertex = vertices[corner] as usize;
                return if material {
                    self.materials
                        .as_ref()
                        .map_or(DEFAULT_VERTEX_MATERIAL, |m| m[vertex])[channel]
                } else {
                    self.colors[vertex][channel]
                };
            }
            let value: f64 = vertices[..count]
                .iter()
                .zip(&weights[..count])
                .map(|(&v, &w)| {
                    let v = v as usize;
                    let scalar = if material {
                        self.materials
                            .as_ref()
                            .map_or(DEFAULT_VERTEX_MATERIAL, |m| m[v])[channel]
                    } else {
                        self.colors[v][channel]
                    };
                    f64::from(scalar) * w
                })
                .sum();
            (value as f32).clamp(0.0, 1.0)
        };
        Ok(PaintSample {
            rgb: std::array::from_fn(|c| sample_scalar(c, false)),
            roughness: sample_scalar(0, true),
            metallic: sample_scalar(1, true),
            materials_explicit: self.materials.is_some(),
            triangle_index,
            original_face_index: face_index,
            interpolation,
        })
    }

    fn paint_original_face(&self, triangle_index: usize) -> (Option<usize>, Option<[u32; 4]>) {
        let Some(surface) = &self.surface else {
            return (None, None);
        };
        let mut first = 0;
        for (index, &face) in surface.faces.iter().enumerate() {
            let count = if face[3] == TRIANGLE_CORNER { 1 } else { 2 };
            if (first..first + count).contains(&triangle_index) {
                return (Some(index), Some(face));
            }
            first += count;
        }
        (None, None)
    }
}

fn fill_scalar(previous: f32, target: f32, weight: f64) -> f32 {
    if weight == 0.0 || previous == target {
        return previous;
    }
    if weight == 1.0 {
        return target;
    }
    let next = (f64::from(previous) * (1.0 - weight) + f64::from(target) * weight) as f32;
    if next == previous {
        previous
    } else {
        next
    }
}
