//! GPU-resident ray picking, shared by native Metal and browser WebGPU. A
//! conservative current-mesh bound rebases distant camera origins in f64
//! before the f32 GPU intersection; this is not a full studio pick policy.
use glam::DVec3;
use wgpu::util::DeviceExt;

pub const RESIDENT_BYTES: u64 = 56; // 48-byte query + 8-byte atomic result.
const MAX_DISTANCE: f32 = 8.0 / std::f32::consts::SQRT_2;

#[derive(Clone, Copy, Debug)]
pub struct LocalBounds {
    min: DVec3,
    max: DVec3,
}
impl LocalBounds {
    pub fn from_positions(positions: &[[f32; 4]]) -> Result<Self, String> {
        let mut min = DVec3::splat(f64::INFINITY);
        let mut max = DVec3::splat(f64::NEG_INFINITY);
        for row in positions {
            let point = DVec3::new(f64::from(row[0]), f64::from(row[1]), f64::from(row[2]));
            if !point.is_finite() || point.abs().max_element() > 1_000_000.0 {
                return Err("Resident pick bounds contain invalid position".into());
            }
            min = min.min(point);
            max = max.max(point);
        }
        if !min.is_finite() || !max.is_finite() {
            return Err("Resident pick bounds require nonempty positions".into());
        }
        Ok(Self { min, max })
    }
    /// Any committed candidate starts inside the radius and moves at most
    /// radius/10: intensity, mask and the radial polynomial each have magnitude
    /// at most one. A little f32 rounding allowance keeps the bound outward.
    pub fn include_point_program(&mut self, center: [f32; 3], radius: f32) {
        let center = DVec3::from_array(center.map(f64::from));
        let reach = f64::from(radius) * 1.100_001 + 1e-5;
        self.min = self
            .min
            .min((center - DVec3::splat(reach)).max(DVec3::splat(-1e6)));
        self.max = self
            .max
            .max((center + DVec3::splat(reach)).min(DVec3::splat(1e6)));
    }
    /// A GPU-picked center is not read back. The current surface is inside
    /// these bounds, so expanding them by the maximum point displacement
    /// remains conservative across the queued dab.
    pub fn include_unread_point_program(&mut self, radius: f32) {
        let reach = f64::from(radius) * 0.100_001 + 1e-5;
        self.min = (self.min - DVec3::splat(reach)).max(DVec3::splat(-1e6));
        self.max = (self.max + DVec3::splat(reach)).min(DVec3::splat(1e6));
    }
    fn entry(self, ray: obrush_core::SceneRay64, maximum: f64) -> Option<f64> {
        let mut near: f64 = 0.0;
        let mut far = maximum;
        for axis in 0..3 {
            let origin = ray.origin[axis];
            let direction = ray.direction[axis];
            if direction == 0.0 {
                if origin < self.min[axis] || origin > self.max[axis] {
                    return None;
                }
                continue;
            }
            let a = (self.min[axis] - origin) / direction;
            let b = (self.max[axis] - origin) / direction;
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return None;
            }
        }
        Some(near)
    }
}

#[derive(Clone, Copy, Debug)]
struct SourceRay {
    original: obrush_core::SceneRay64,
    distance_offset: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct PickRay {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub max_distance: f32,
    source: Option<SourceRay>,
}
impl PickRay {
    pub fn source_distance_offset(self) -> Option<f64> {
        self.source.map(|source| source.distance_offset)
    }
    /// Receives the actual camera's already-local ray. Object/world transforms
    /// remain the caller's responsibility; never infer them from a mesh label.
    pub fn from_local_scene(
        ray: obrush_core::SceneRay64,
        max_distance: f64,
        bounds: LocalBounds,
    ) -> Result<Self, String> {
        ray.validate()?;
        if !max_distance.is_finite() || max_distance <= 0.0 || max_distance > 2_000_000.0 {
            return Err("Local camera pick distance exceeds bounded domain".into());
        }
        let coordinate_scale = bounds.min.abs().max(bounds.max.abs()).max_element();
        let margin = ((bounds.max - bounds.min).max_element() * 1e-4)
            .max(coordinate_scale * f64::from(f32::EPSILON) * 8.0)
            .max(1e-3);
        let offset = bounds
            .entry(ray, max_distance)
            .map_or(0.0, |entry| (entry - margin).max(0.0));
        let rebased = ray.origin + ray.direction * offset;
        let value = Self {
            origin: rebased.to_array().map(|v| v as f32),
            direction: ray.direction.to_array().map(|v| v as f32),
            max_distance: (max_distance - offset) as f32,
            source: Some(SourceRay {
                original: ray,
                distance_offset: offset,
            }),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(self) -> Result<(), String> {
        if self
            .origin
            .into_iter()
            .chain(self.direction)
            .any(|value| !value.is_finite() || value.abs() > 1_000_000.0)
            || !self.max_distance.is_finite()
            || !(0.0..=2_000_000.0).contains(&self.max_distance)
            || self.max_distance == 0.0
        {
            return Err("GPU pick ray exceeds finite bounded domain".into());
        }
        let length_squared = self.direction.iter().map(|v| v * v).sum::<f32>();
        if !(0.999..=1.001).contains(&length_squared) {
            return Err("GPU pick ray direction is not unit length".into());
        }
        Ok(())
    }
    /// Matches the staged scene's orthographic clip transform at a pixel
    /// center, with origin on clip depth zero and direction toward depth one.
    pub fn diagnostic_pixel(x: u32, y: u32, width: u32, height: u32) -> Result<Self, String> {
        if width == 0 || height == 0 || x >= width || y >= height {
            return Err("GPU pick pixel is outside the canvas".into());
        }
        let aspect = crate::surface::aspect(width, height);
        let nx = (2.0 * (x as f64 + 0.5) / f64::from(width) - 1.0) as f32;
        let ny = (1.0 - 2.0 * (y as f64 + 0.5) / f64::from(height)) as f32;
        let difference = 2.0 * nx / aspect[0];
        let y_world = ny / (std::f32::consts::FRAC_1_SQRT_2 * aspect[1]);
        let ray = Self {
            origin: [(4.0 + difference) * 0.5, y_world, (4.0 - difference) * 0.5],
            direction: [
                -std::f32::consts::FRAC_1_SQRT_2,
                0.0,
                -std::f32::consts::FRAC_1_SQRT_2,
            ],
            max_distance: MAX_DISTANCE,
            source: None,
        };
        ray.validate()?;
        Ok(ray)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PickHit {
    pub face: u32,
    pub distance: f64,
    pub position: [f32; 3],
}

pub struct ResidentPicker {
    result: wgpu::Buffer,
    query: wgpu::Buffer,
    nearest: wgpu::ComputePipeline,
    lowest_face: wgpu::ComputePipeline,
    nearest_group: wgpu::BindGroup,
    lowest_face_group: wgpu::BindGroup,
    faces: u32,
}
impl ResidentPicker {
    pub fn new(
        device: &wgpu::Device,
        positions: &wgpu::Buffer,
        triangles: &wgpu::Buffer,
        input: &obrush_core::resident_point_program::ResidentPointInput,
    ) -> Result<Self, String> {
        let faces = input.triangles().len() as u32;
        let result = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("8-byte current-surface pick result"),
            contents: &[255; 8],
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });
        let query = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("48-byte current-surface pick ray"),
            size: 48,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("current resident triangle ray query"),
            source: wgpu::ShaderSource::Wgsl(include_str!("pick.wgsl").into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let nearest = pipeline("nearest");
        let lowest_face = pipeline("lowest_face");
        let group = |pipeline: &wgpu::ComputePipeline| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("same resident position/index buffers for pick"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[positions, triangles, &query, &result]
                    .into_iter()
                    .enumerate()
                    .map(|(index, buffer)| wgpu::BindGroupEntry {
                        binding: index as u32,
                        resource: buffer.as_entire_binding(),
                    })
                    .collect::<Vec<_>>(),
            })
        };
        let nearest_group = group(&nearest);
        let lowest_face_group = group(&lowest_face);
        Ok(Self {
            result,
            query,
            nearest,
            lowest_face,
            nearest_group,
            lowest_face_group,
            faces,
        })
    }
    pub fn encode(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ray: PickRay,
    ) -> Result<(), String> {
        ray.validate()?;
        let mut words = [0u32; 12];
        for (word, value) in words[..3].iter_mut().zip(ray.origin) {
            *word = value.to_bits();
        }
        for (word, value) in words[4..7].iter_mut().zip(ray.direction) {
            *word = value.to_bits();
        }
        words[8] = self.faces;
        words[9] = ray.max_distance.to_bits();
        queue.write_buffer(&self.query, 0, bytemuck::cast_slice(&words));
        queue.write_buffer(&self.result, 0, &[255; 8]);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("two-pass nearest current-surface pick"),
        });
        for (pipeline, group) in [
            (&self.nearest, &self.nearest_group),
            (&self.lowest_face, &self.lowest_face_group),
        ] {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("current triangle query"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(self.faces.div_ceil(256), 1, 1);
        }
        queue.submit([encoder.finish()]);
        Ok(())
    }
    pub fn result(&self) -> &wgpu::Buffer {
        &self.result
    }
    pub fn query(&self) -> &wgpu::Buffer {
        &self.query
    }
    pub fn decode(&self, ray: PickRay, bytes: &[u8]) -> Result<Option<PickHit>, String> {
        if bytes.len() != 8 {
            return Err("GPU pick result has wrong length".into());
        }
        let depth = u32::from_le_bytes(bytes[..4].try_into().map_err(|_| "pick depth")?);
        let face = u32::from_le_bytes(bytes[4..].try_into().map_err(|_| "pick face")?);
        if depth == u32::MAX && face == u32::MAX {
            return Ok(None);
        }
        let local_distance = f32::from_bits(depth);
        if face >= self.faces
            || !local_distance.is_finite()
            || local_distance < 0.0
            || local_distance > ray.max_distance
        {
            return Err("GPU pick returned an invalid face or distance".into());
        }
        let (distance, position) = if let Some(source) = ray.source {
            let distance = source.distance_offset + f64::from(local_distance);
            let point = source.original.origin + source.original.direction * distance;
            (distance, point.to_array().map(|value| value as f32))
        } else {
            (
                f64::from(local_distance),
                std::array::from_fn(|axis| ray.origin[axis] + ray.direction[axis] * local_distance),
            )
        };
        if position.iter().any(|v| !v.is_finite()) {
            return Err("GPU pick produced nonfinite position".into());
        }
        Ok(Some(PickHit {
            face,
            distance,
            position,
        }))
    }
    pub fn destroy(&self) {
        self.result.destroy();
        self.query.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_ray_preserves_oblique_projection_and_bounds() {
        for (w, h, x, y) in [(512, 512, 256, 256), (1920, 1080, 960, 540)] {
            let ray = PickRay::diagnostic_pixel(x, y, w, h).unwrap();
            let aspect = crate::surface::aspect(w, h);
            let clip_x = 0.5 * (ray.origin[0] - ray.origin[2]) * aspect[0];
            let clip_y = std::f32::consts::FRAC_1_SQRT_2 * ray.origin[1] * aspect[1];
            let wanted_x = 2.0 * (x as f32 + 0.5) / w as f32 - 1.0;
            let wanted_y = 1.0 - 2.0 * (y as f32 + 0.5) / h as f32;
            assert!((clip_x - wanted_x).abs() < 1e-6);
            assert!((clip_y - wanted_y).abs() < 1e-6);
        }
        assert!(PickRay::diagnostic_pixel(512, 0, 512, 512).is_err());
        assert!(PickRay::diagnostic_pixel(0, 0, 0, 512).is_err());
        assert!(
            PickRay {
                origin: [f32::NAN, 0., 0.],
                direction: [0., 0., 1.],
                max_distance: 1.,
                source: None,
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn distant_local_ray_rebases_before_conservative_current_bounds() {
        let mut bounds =
            LocalBounds::from_positions(&[[-1., -1., -1., 1.], [1., 1., 1., 1.]]).unwrap();
        let ray = obrush_core::SceneRay64 {
            origin: DVec3::new(0., 0., 2000.),
            direction: -DVec3::Z,
        };
        let pick = PickRay::from_local_scene(ray, 5000., bounds).unwrap();
        assert!((pick.origin[2] - 1.001).abs() < 1e-5);
        assert!((pick.source.unwrap().distance_offset - 1998.999).abs() < 1e-8);
        bounds.include_point_program([0., 0., 1.], 0.5);
        let pick = PickRay::from_local_scene(ray, 5000., bounds).unwrap();
        assert!(pick.origin[2] >= 1.550_01);
        assert!(pick.origin[2] < 1.552);
        let inside = obrush_core::SceneRay64 {
            origin: DVec3::ZERO,
            direction: DVec3::Z,
        };
        assert_eq!(
            PickRay::from_local_scene(inside, 5., bounds)
                .unwrap()
                .source
                .unwrap()
                .distance_offset,
            0.
        );
    }
}
