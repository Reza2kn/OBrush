use obrush_core::resident_normal_program::*;
use obrush_core::resident_point_program::ResidentPointInput;
use wgpu::util::DeviceExt;
pub struct RetainedNormals {
    pub headers: wgpu::Buffer,
    pub offsets: wgpu::Buffer,
    pub rings: wgpu::Buffer,
    pub faces: wgpu::Buffer,
    pub face_work: wgpu::Buffer,
    pub checkpoint_faces: wgpu::Buffer,
    pub checkpoint_vertices: wgpu::Buffer,
    pub selected: wgpu::Buffer,
    pub visited: wgpu::Buffer,
    pub checkpoint_visited: wgpu::Buffer,
    pub face_candidate_pipeline: wgpu::ComputePipeline,
    pub mean_preflight_pipeline: wgpu::ComputePipeline,
    pub face_commit_pipeline: wgpu::ComputePipeline,
    pub vertex_commit_pipeline: wgpu::ComputePipeline,
    pub face_candidate_group: wgpu::BindGroup,
    pub mean_preflight_group: wgpu::BindGroup,
    pub face_commit_group: wgpu::BindGroup,
    pub vertex_commit_group: wgpu::BindGroup,
    pub face_count: u32,
    pub face_bytes: u64,
    pub vertices: u32,
}
impl RetainedNormals {
    pub fn extra_bytes(input: &ResidentPointInput) -> Result<u64, String> {
        let t = input.normal_topology();
        let n = input.positions().len() as u64;
        let topology = (t.headers().len() as u64)
            .checked_add(t.offsets().len() as u64)
            .and_then(|v| v.checked_add(t.rings().len() as u64))
            .and_then(|v| v.checked_mul(4))
            .ok_or("GPU normal topology bytes overflow")?;
        topology
            .checked_add(t.face_normals().len() as u64 * 12 * 3)
            .and_then(|v| v.checked_add(n * 16))
            .and_then(|v| v.checked_add(n * 4 * 3))
            .ok_or_else(|| "GPU normal bytes overflow".into())
    }
    pub fn new(
        device: &wgpu::Device,
        input: &ResidentPointInput,
        candidates: &wgpu::Buffer,
        normal_u: &wgpu::Buffer,
        indices: &wgpu::Buffer,
        failure: &wgpu::Buffer,
    ) -> Self {
        let topology = input.normal_topology();
        let vertices = input.positions().len() as u32;
        let face_count = topology.headers().len() as u32;
        let face_bytes = u64::from(face_count) * 12;
        let usage = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST;
        let upload = |label, contents: &[u8]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        let zero = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let headers = upload(
            "original face headers",
            bytemuck::cast_slice(topology.headers()),
        );
        let offsets = upload(
            "descending ring offsets",
            bytemuck::cast_slice(topology.offsets()),
        );
        let rings = upload(
            "descending original face rings",
            bytemuck::cast_slice(topology.rings()),
        );
        let faces = upload(
            "retained original face cache",
            bytemuck::cast_slice(topology.face_normals()),
        );
        let face_work = zero("candidate original face cache", face_bytes);
        let checkpoint_faces = zero("GPU checkpoint retained face cache", face_bytes);
        let checkpoint_vertices = zero("GPU checkpoint normal U", u64::from(vertices) * 16);
        let selected = zero("current selected vertices", u64::from(vertices) * 4);
        let visited = zero("retained selected vertex IDs", u64::from(vertices) * 4);
        let checkpoint_visited = zero("GPU checkpoint selected IDs", u64::from(vertices) * 4);
        let pipeline = |label, parts: &[&str], entry| {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(parts.join("\n").into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: None,
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let face_candidate_pipeline = pipeline(
            "selected face candidate",
            &[
                NORMAL_FACE_FUNCTIONS,
                NORMAL_CROSS_FUNCTIONS,
                NORMAL_PREFLIGHT,
            ],
            "face_candidate",
        );
        let mean_preflight_pipeline = pipeline(
            "selected mean preflight",
            &[NORMAL_MEAN_FUNCTIONS, NORMAL_MEAN_PREFLIGHT],
            "mean_preflight",
        );
        let face_commit_pipeline = pipeline(
            "selected face commit",
            &[NORMAL_FACE_FUNCTIONS, NORMAL_FACE_COMMIT],
            "face_commit",
        );
        let vertex_commit_pipeline = pipeline(
            "selected vertex commit",
            &[NORMAL_MEAN_FUNCTIONS, NORMAL_VERTEX_COMMIT],
            "vertex_commit",
        );
        let group = |pipeline: &wgpu::ComputePipeline, buffers: &[&wgpu::Buffer]| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &buffers
                    .iter()
                    .enumerate()
                    .map(|(i, b)| wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: b.as_entire_binding(),
                    })
                    .collect::<Vec<_>>(),
            })
        };
        let face_candidate_group = group(
            &face_candidate_pipeline,
            &[
                candidates, &headers, indices, &selected, &face_work, failure,
            ],
        );
        let mean_preflight_group = group(
            &mean_preflight_pipeline,
            &[&face_work, &offsets, &rings, &selected, failure],
        );
        let face_commit_group = group(
            &face_commit_pipeline,
            &[&faces, &face_work, &headers, indices, &selected, failure],
        );
        let vertex_commit_group = group(
            &vertex_commit_pipeline,
            &[
                normal_u, &faces, &offsets, &rings, &selected, failure, &visited,
            ],
        );
        Self {
            headers,
            offsets,
            rings,
            faces,
            face_work,
            checkpoint_faces,
            checkpoint_vertices,
            selected,
            visited,
            checkpoint_visited,
            face_candidate_pipeline,
            mean_preflight_pipeline,
            face_commit_pipeline,
            vertex_commit_pipeline,
            face_candidate_group,
            mean_preflight_group,
            face_commit_group,
            vertex_commit_group,
            face_count,
            face_bytes,
            vertices,
        }
    }
    pub fn checkpoint(&self, encoder: &mut wgpu::CommandEncoder, normal_u: &wgpu::Buffer) {
        encoder.copy_buffer_to_buffer(
            normal_u,
            0,
            &self.checkpoint_vertices,
            0,
            u64::from(self.vertices) * 16,
        );
        encoder.copy_buffer_to_buffer(&self.faces, 0, &self.checkpoint_faces, 0, self.face_bytes);
        encoder.copy_buffer_to_buffer(
            &self.visited,
            0,
            &self.checkpoint_visited,
            0,
            u64::from(self.vertices) * 4,
        );
    }
    pub fn cancel(&self, encoder: &mut wgpu::CommandEncoder, normal_u: &wgpu::Buffer) {
        encoder.copy_buffer_to_buffer(
            &self.checkpoint_vertices,
            0,
            normal_u,
            0,
            u64::from(self.vertices) * 16,
        );
        encoder.copy_buffer_to_buffer(&self.checkpoint_faces, 0, &self.faces, 0, self.face_bytes);
        encoder.copy_buffer_to_buffer(
            &self.checkpoint_visited,
            0,
            &self.visited,
            0,
            u64::from(self.vertices) * 4,
        );
    }
    pub fn destroy(&self) {
        for buffer in [
            &self.headers,
            &self.offsets,
            &self.rings,
            &self.faces,
            &self.face_work,
            &self.checkpoint_faces,
            &self.checkpoint_vertices,
            &self.selected,
            &self.visited,
            &self.checkpoint_visited,
        ] {
            buffer.destroy();
        }
    }
}
