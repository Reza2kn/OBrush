use wgpu::util::DeviceExt;

pub struct ResidentSparse {
    pub counter: wgpu::Buffer,
    count_pipeline: wgpu::ComputePipeline,
    collect_pipeline: wgpu::ComputePipeline,
    count_group: wgpu::BindGroup,
    positions: wgpu::Buffer,
    checkpoint: wgpu::Buffer,
}

impl ResidentSparse {
    pub fn new(device: &wgpu::Device, positions: &wgpu::Buffer, checkpoint: &wgpu::Buffer) -> Self {
        let counter = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("resident sparse changed-row counter"),
            contents: &0u32.to_le_bytes(),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("resident sparse changed-row compaction"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sparse.wgsl").into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let count_pipeline = pipeline("count");
        let collect_pipeline = pipeline("collect");
        let count_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resident sparse count inputs"),
            layout: &count_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: positions.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: checkpoint.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: counter.as_entire_binding(),
                },
            ],
        });
        Self {
            counter,
            count_pipeline,
            collect_pipeline,
            count_group,
            positions: positions.clone(),
            checkpoint: checkpoint.clone(),
        }
    }

    pub fn encode_count(&self, encoder: &mut wgpu::CommandEncoder, vertices: u32) {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.count_pipeline);
        pass.set_bind_group(0, &self.count_group, &[]);
        pass.dispatch_workgroups(vertices.div_ceil(256), 1, 1);
    }

    pub fn encode_collect(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::Buffer,
        vertices: u32,
    ) {
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resident sparse collect output"),
            layout: &self.collect_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.positions.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.checkpoint.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.counter.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.collect_pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(vertices.div_ceil(256), 1, 1);
    }

    pub fn destroy(&self) {
        self.counter.destroy();
    }
}
