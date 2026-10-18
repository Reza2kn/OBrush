//! Shared native/WebGPU residency prototype. No production document mutation.
use obrush_core::resident_point_program::{
    PointBrushParameters, PointInflateParameters, RESIDENT_BRUSH_PICK_WGSL,
    RESIDENT_POINT_COMMIT_WGSL, RESIDENT_POINT_WGSL, ResidentPointInput,
};
use std::sync::{Arc, Mutex};
mod normals;
mod picker;
mod sparse;
mod surface;
mod view;
use normals::RetainedNormals;
pub use obrush_core::resident_normal_program::ResidentNormalReference;
pub use picker::{PickHit, PickRay};
pub use view::ResidentViewMatrix;
use wgpu::util::DeviceExt;
const SIZE: u32 = 512;
const MAX_GPU_BYTES: u64 = 256 * 1024 * 1024;
#[derive(Clone, Copy)]
enum CandidateSource {
    Center,
    BrushCenter,
    BrushRay,
    Ray,
    Depth,
}
pub struct ResidentViewport {
    device: wgpu::Device,
    queue: wgpu::Queue,
    positions: wgpu::Buffer,
    checkpoint: wgpu::Buffer,
    candidates: wgpu::Buffer,
    normal_u: wgpu::Buffer,
    normals: RetainedNormals,
    picker: picker::ResidentPicker,
    sparse: sparse::ResidentSparse,
    bounds: picker::LocalBounds,
    normal_readbacks: u64,
    pick_readbacks: u64,
    pick_hits: u64,
    colors: wgpu::Buffer,
    indices: wgpu::Buffer,
    failure: wgpu::Buffer,
    parameters: wgpu::Buffer,
    depth_query: wgpu::Buffer,
    candidate_pipeline: wgpu::ComputePipeline,
    brush_candidate_pipeline: wgpu::ComputePipeline,
    brush_pick_direction_pipeline: wgpu::ComputePipeline,
    brush_picked_candidate_pipeline: wgpu::ComputePipeline,
    picked_candidate_pipeline: wgpu::ComputePipeline,
    depth_candidate_pipeline: wgpu::ComputePipeline,
    commit_pipeline: wgpu::ComputePipeline,
    candidate_group: wgpu::BindGroup,
    brush_candidate_group: wgpu::BindGroup,
    brush_pick_direction_group: wgpu::BindGroup,
    brush_picked_candidate_group: wgpu::BindGroup,
    picked_brush_direction: wgpu::Buffer,
    picked_candidate_group: wgpu::BindGroup,
    depth_candidate_group: Option<wgpu::BindGroup>,
    commit_group: wgpu::BindGroup,
    draw_pipeline: wgpu::RenderPipeline,
    draw_group: wgpu::BindGroup,
    offscreen_view: wgpu::Buffer,
    surface: Option<surface::SurfacePresenter>,
    camera_active: bool,
    camera_inverse: Option<[f32; 16]>,
    depth_ready: bool,
    color: wgpu::Texture,
    depth: wgpu::Texture,
    vertices: u32,
    index_count: u32,
    position_bytes: u64,
    resident_bytes: u64,
    checkpoint_valid: bool,
    destroyed: bool,
    position_readbacks: u64,
    sparse_readbacks: u64,
    sparse_payload_bytes: u64,
    pixel_readbacks: u64,
    draws: u64,
    point_passes: u64,
    pick_queued: bool,
    errors: Arc<Mutex<Vec<String>>>,
    device_lost: Arc<Mutex<Option<String>>>,
    adapter: wgpu::AdapterInfo,
}
impl ResidentViewport {
    pub async fn new(input: &ResidentPointInput) -> Result<Self, String> {
        Self::create(input, None, None).await
    }
    async fn create(
        input: &ResidentPointInput,
        target: Option<wgpu::SurfaceTarget<'static>>,
        extent: Option<[u32; 2]>,
    ) -> Result<Self, String> {
        let bounds = picker::LocalBounds::from_positions(input.positions())?;
        let vertex_count = input.positions().len();
        let position_bytes = (vertex_count as u64)
            .checked_mul(16)
            .ok_or("GPU position size overflow")?;
        let index_bytes = (input.triangles().len() as u64)
            .checked_mul(12)
            .ok_or("GPU index size overflow")?;
        let resident_bytes = position_bytes
            .checked_mul(5)
            .and_then(|n| n.checked_add(index_bytes))
            .and_then(|n| {
                n.checked_add(
                    u64::from(SIZE) * u64::from(SIZE) * 8
                        + 132
                        + surface::VIEW_UNIFORM_BYTES
                        + picker::RESIDENT_BYTES
                        + 20,
                )
            })
            .ok_or("GPU storage size overflow")?;
        let resident_bytes = resident_bytes
            .checked_add(RetainedNormals::extra_bytes(input)?)
            .ok_or("GPU combined residency overflow")?;
        if resident_bytes
            .checked_add(surface::READBACK_CHUNK)
            .is_none_or(|n| n > MAX_GPU_BYTES)
        {
            return Err("GPU residency exceeds bounded256MiB allocation".into());
        }
        let presentation_budget = surface::PresentationBudget {
            fixed: resident_bytes,
            cap: MAX_GPU_BYTES,
            max_dimension: wgpu::Limits::default().max_texture_dimension_2d,
        };
        if let Some([width, height]) = extent {
            presentation_budget.admit(width, height)?;
        }
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = target
            .map(|target| instance.create_surface(target).map_err(|e| e.to_string()))
            .transpose()?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                force_fallback_adapter: false,
                compatible_surface: surface.as_ref(),
            })
            .await
            .map_err(|e| e.to_string())?;
        let adapter_info = adapter.get_info();
        if let Some([width, height]) = extent {
            surface::PresentationBudget {
                max_dimension: adapter.limits().max_texture_dimension_2d,
                ..presentation_budget
            }
            .admit(width, height)?;
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("OBrush experimental resident device"),
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errors_sink = errors.clone();
        device.on_uncaptured_error(Arc::new(move |error| {
            if let Ok(mut rows) = errors_sink.lock() {
                rows.push(format!("{error}"));
            }
        }));
        let device_lost = Arc::new(Mutex::new(None));
        let lost_sink = device_lost.clone();
        device.set_device_lost_callback(move |reason, message| {
            if let Ok(mut state) = lost_sink.lock() {
                *state = Some(format!("{reason:?}: {message}"));
            }
        });
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let storage = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST;
        let upload = |label, contents: &[u8], usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        let positions = upload(
            "owned canonical resident positions",
            bytemuck::cast_slice(input.positions()),
            storage,
        );
        let normal_u = upload(
            "retained normal U",
            bytemuck::cast_slice(input.normal_u()),
            storage,
        );
        let colors = upload(
            "captured original RGB",
            bytemuck::cast_slice(input.colors()),
            wgpu::BufferUsages::STORAGE,
        );
        let indices = upload(
            "original canonical indices",
            bytemuck::cast_slice(input.triangles()),
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::STORAGE,
        );
        let checkpoint = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("owned GPU checkpoint"),
            size: position_bytes,
            usage: wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let candidates = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atomic candidate workspace"),
            size: position_bytes,
            usage: storage,
            mapped_at_creation: false,
        });
        let failure = upload(
            "atomic failure vertex",
            bytemuck::bytes_of(&u32::MAX),
            storage,
        );
        let parameters = upload(
            "finite bounded point parameters",
            &[0u8; 48],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let depth_query = upload(
            "current-frame depth pointer and inverse camera",
            &[0u8; 80],
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let picked_brush_direction = upload("resident picked Brush direction", &[0u8; 16], storage);
        let compute = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("core point candidate"),
            source: wgpu::ShaderSource::Wgsl(RESIDENT_POINT_WGSL.into()),
        });
        let commit = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("core atomic commit"),
            source: wgpu::ShaderSource::Wgsl(RESIDENT_POINT_COMMIT_WGSL.into()),
        });
        let brush_pick = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("original-face resident Brush direction"),
            source: wgpu::ShaderSource::Wgsl(RESIDENT_BRUSH_PICK_WGSL.into()),
        });
        let pipeline = |module: &wgpu::ShaderModule, entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let candidate_pipeline = pipeline(&compute, "candidate");
        let brush_candidate_pipeline = pipeline(&compute, "candidate_brush");
        let brush_picked_candidate_pipeline = pipeline(&compute, "candidate_brush_picked");
        let brush_pick_direction_pipeline = pipeline(&brush_pick, "brush_pick_direction");
        let picked_candidate_pipeline = pipeline(&compute, "candidate_picked");
        let depth_candidate_pipeline = pipeline(&compute, "candidate_depth");
        let commit_pipeline = pipeline(&commit, "commit");
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
        let normals =
            RetainedNormals::new(&device, input, &candidates, &normal_u, &indices, &failure);
        let picker = picker::ResidentPicker::new(&device, &positions, &indices, input)?;
        let sparse = sparse::ResidentSparse::new(&device, &positions, &checkpoint);
        let candidate_group = group(
            &candidate_pipeline,
            &[
                &positions,
                &normal_u,
                &candidates,
                &failure,
                &parameters,
                &normals.selected,
            ],
        );
        let brush_candidate_group = group(
            &brush_candidate_pipeline,
            &[
                &positions,
                &normal_u,
                &candidates,
                &failure,
                &parameters,
                &normals.selected,
            ],
        );
        let brush_pick_direction_group = group(
            &brush_pick_direction_pipeline,
            &[
                &positions,
                &normal_u,
                &indices,
                &normals.headers,
                picker.query(),
                picker.result(),
                &picked_brush_direction,
                &failure,
            ],
        );
        let mut brush_picked_entries = [
            &positions,
            &normal_u,
            &candidates,
            &failure,
            &parameters,
            &normals.selected,
            picker.query(),
            picker.result(),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, buffer)| wgpu::BindGroupEntry {
            binding: index as u32,
            resource: buffer.as_entire_binding(),
        })
        .collect::<Vec<_>>();
        brush_picked_entries.push(wgpu::BindGroupEntry {
            binding: 10,
            resource: picked_brush_direction.as_entire_binding(),
        });
        let brush_picked_candidate_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GPU-picked ordinary Brush candidate"),
            layout: &brush_picked_candidate_pipeline.get_bind_group_layout(0),
            entries: &brush_picked_entries,
        });
        let picked_candidate_group = group(
            &picked_candidate_pipeline,
            &[
                &positions,
                &normal_u,
                &candidates,
                &failure,
                &parameters,
                &normals.selected,
                picker.query(),
                picker.result(),
            ],
        );
        let commit_group = group(&commit_pipeline, &[&positions, &candidates, &failure]);
        let offscreen_view = upload(
            "offscreen camera/diagnostic view uniform",
            bytemuck::cast_slice(&view::diagnostic_uniform(surface::aspect(SIZE, SIZE))),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let draw_pipeline = surface::pipeline(&device, wgpu::TextureFormat::Rgba8Unorm);
        let draw_group = surface::group(
            &device,
            &draw_pipeline,
            [&positions, &normal_u, &colors, &offscreen_view],
            "canonical positions used directly by vertex stage",
        );
        let texture = |label, format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: SIZE,
                    height: SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let color = texture(
            "bounded offscreen viewport",
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let depth = texture(
            "bounded depth",
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let surface = match (surface, extent) {
            (Some(surface), Some(extent)) => Some(surface::SurfacePresenter::new(
                surface,
                surface::SurfaceSetup {
                    device: &device,
                    adapter: &adapter,
                    buffers: [&positions, &normal_u, &colors],
                    budget: surface::PresentationBudget {
                        max_dimension: adapter.limits().max_texture_dimension_2d,
                        ..presentation_budget
                    },
                    extent,
                },
            )?),
            (None, None) => None,
            _ => return Err("Surface target and dimensions must both be present".into()),
        };
        if let Some(error) = device.pop_error_scope().await {
            return Err(error.to_string());
        }
        Ok(Self {
            device,
            queue,
            positions,
            checkpoint,
            candidates,
            normal_u,
            normals,
            picker,
            sparse,
            bounds,
            normal_readbacks: 0,
            pick_readbacks: 0,
            pick_hits: 0,
            colors,
            indices,
            failure,
            parameters,
            depth_query,
            candidate_pipeline,
            brush_candidate_pipeline,
            brush_pick_direction_pipeline,
            brush_picked_candidate_pipeline,
            picked_candidate_pipeline,
            depth_candidate_pipeline,
            commit_pipeline,
            candidate_group,
            brush_candidate_group,
            brush_pick_direction_group,
            brush_picked_candidate_group,
            picked_brush_direction,
            picked_candidate_group,
            depth_candidate_group: None,
            commit_group,
            draw_pipeline,
            draw_group,
            offscreen_view,
            surface,
            camera_active: false,
            camera_inverse: None,
            depth_ready: false,
            color,
            depth,
            vertices: vertex_count as u32,
            index_count: (input.triangles().len() * 3) as u32,
            position_bytes,
            resident_bytes,
            checkpoint_valid: false,
            destroyed: false,
            position_readbacks: 0,
            sparse_readbacks: 0,
            sparse_payload_bytes: 0,
            pixel_readbacks: 0,
            draws: 0,
            point_passes: 0,
            pick_queued: false,
            errors,
            device_lost,
            adapter: adapter_info,
        })
    }
    fn ready(&self) -> Result<(), String> {
        if self.destroyed {
            return Err("Resident device destroyed".into());
        }
        if let Some(reason) = self
            .device_lost
            .lock()
            .map_err(|_| "GPU loss state poisoned")?
            .as_ref()
        {
            return Err(format!("Resident device lost: {reason}"));
        }
        let errors = self.errors.lock().map_err(|_| "GPU error state poisoned")?;
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        Ok(())
    }
    pub fn checkpoint(&mut self) -> Result<(), String> {
        self.ready()?;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.positions, 0, &self.checkpoint, 0, self.position_bytes);
        self.normals.checkpoint(&mut encoder, &self.normal_u);
        self.queue.submit([encoder.finish()]);
        self.checkpoint_valid = true;
        Ok(())
    }
    pub fn cancel(&mut self) -> Result<(), String> {
        self.ready()?;
        if !self.checkpoint_valid {
            return Err("No owned resident checkpoint".into());
        }
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.checkpoint, 0, &self.positions, 0, self.position_bytes);
        self.normals.cancel(&mut encoder, &self.normal_u);
        self.queue.submit([encoder.finish()]);
        self.pick_queued = false;
        self.depth_ready = false;
        Ok(())
    }
    pub fn inflate(
        &mut self,
        parameters: PointInflateParameters,
        passes: u32,
    ) -> Result<(), String> {
        self.inflate_with_candidate(parameters, passes, CandidateSource::Center)
    }
    /// Explicit-direction ordinary Brush candidate. The caller owns the
    /// picked-normal policy; this path does not infer a Source contact.
    pub fn brush(&mut self, parameters: PointBrushParameters, passes: u32) -> Result<(), String> {
        self.run_candidate(
            parameters.uniform()?.map(f32::to_bits),
            parameters.center,
            parameters.radius,
            passes,
            CandidateSource::BrushCenter,
        )
    }
    /// Resident pick and original-face direction remain GPU-owned through the
    /// ordinary Brush point pass; the document still checks the sparse result.
    pub fn brush_picked(
        &mut self,
        ray: PickRay,
        radius: f32,
        intensity: f32,
    ) -> Result<(), String> {
        self.queue_picked_ray(ray)?;
        self.brush_last_pick(radius, intensity)
    }
    pub fn brush_last_pick(&mut self, radius: f32, intensity: f32) -> Result<(), String> {
        if !self.pick_queued {
            return Err("No queued GPU pick for Brush deformation".into());
        }
        let parameters = PointInflateParameters {
            center: [0.0; 3],
            radius,
            intensity,
        };
        let words = parameters.uniform()?.map(f32::to_bits);
        self.pick_queued = false;
        self.run_candidate(words, [0.0; 3], radius, 1, CandidateSource::BrushRay)
    }
    /// Pick on the current resident surface, then deform at the result
    /// without a CPU readback between the GPU passes.
    pub fn inflate_picked(
        &mut self,
        ray: PickRay,
        radius: f32,
        intensity: f32,
    ) -> Result<(), String> {
        PointInflateParameters {
            center: [0.0; 3],
            radius,
            intensity,
        }
        .uniform()?;
        self.queue_picked_ray(ray)?;
        self.inflate_last_pick(radius, intensity)
    }
    pub fn queue_picked_ray(&mut self, ray: PickRay) -> Result<(), String> {
        self.ready()?;
        self.picker.encode(&self.device, &self.queue, ray)?;
        self.pick_queued = true;
        Ok(())
    }
    pub fn inflate_last_pick(&mut self, radius: f32, intensity: f32) -> Result<(), String> {
        if !self.pick_queued {
            return Err("No queued GPU pick for point deformation".into());
        }
        let parameters = PointInflateParameters {
            center: [0.0; 3],
            radius,
            intensity,
        };
        parameters.uniform()?;
        self.pick_queued = false;
        self.inflate_with_candidate(parameters, 1, CandidateSource::Ray)
    }
    /// Use the stored current frame depth at an integer canvas pixel. The
    /// inverse is of the exact f32 projection used for the GPU draw.
    pub fn inflate_depth_pixel(
        &mut self,
        x: u32,
        y: u32,
        radius: f32,
        intensity: f32,
    ) -> Result<(), String> {
        self.ready()?;
        if !self.depth_ready {
            return Err("Depth pick requires a freshly presented frame".into());
        }
        let inverse = self
            .camera_inverse
            .ok_or("Depth pick needs a camera matrix")?;
        let extent = self
            .surface
            .as_ref()
            .map_or([SIZE, SIZE], |surface| surface.camera_extent());
        if x >= extent[0] || y >= extent[1] || extent.contains(&0) {
            return Err("Depth pick pixel is outside the current canvas".into());
        }
        let parameters = PointInflateParameters {
            center: [0.0; 3],
            radius,
            intensity,
        };
        parameters.uniform()?;
        let mut query = [0u32; 20];
        for (word, value) in query[..16].iter_mut().zip(inverse) {
            *word = value.to_bits();
        }
        query[16..20].copy_from_slice(&[x, y, extent[0], extent[1]]);
        self.queue
            .write_buffer(&self.depth_query, 0, bytemuck::cast_slice(&query));
        let depth_view = self.surface.as_ref().map_or_else(
            || Ok(self.depth.create_view(&Default::default())),
            |surface| surface.depth_view(),
        )?;
        let buffers = [
            &self.positions,
            &self.normal_u,
            &self.candidates,
            &self.failure,
            &self.parameters,
            &self.normals.selected,
        ];
        let mut entries = buffers
            .iter()
            .enumerate()
            .map(|(index, buffer)| wgpu::BindGroupEntry {
                binding: index as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        entries.push(wgpu::BindGroupEntry {
            binding: 8,
            resource: wgpu::BindingResource::TextureView(&depth_view),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 9,
            resource: self.depth_query.as_entire_binding(),
        });
        self.depth_candidate_group =
            Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("current-frame depth candidate"),
                layout: &self.depth_candidate_pipeline.get_bind_group_layout(0),
                entries: &entries,
            }));
        self.inflate_with_candidate(parameters, 1, CandidateSource::Depth)
    }
    /// Backpressure without mapping any pick or position bytes. The browser
    /// can coalesce incoming pointer events until the preceding dab and draw
    /// have actually completed on the GPU.
    pub async fn wait_submitted(&self) -> Result<(), String> {
        self.ready()?;
        let (sender, receiver) = futures_channel::oneshot::channel();
        self.queue.on_submitted_work_done(move || {
            let _ = sender.send(());
        });
        receiver
            .await
            .map_err(|_| "GPU completion callback was dropped")?;
        self.ready()
    }
    fn inflate_with_candidate(
        &mut self,
        parameters: PointInflateParameters,
        passes: u32,
        source: CandidateSource,
    ) -> Result<(), String> {
        self.run_candidate(
            parameters.uniform()?.map(f32::to_bits),
            parameters.center,
            parameters.radius,
            passes,
            source,
        )
    }
    fn run_candidate(
        &mut self,
        words: [u32; 8],
        center: [f32; 3],
        radius: f32,
        passes: u32,
        source: CandidateSource,
    ) -> Result<(), String> {
        self.ready()?;
        let mut uniform = [0u32; 12];
        uniform[..8].copy_from_slice(&words);
        if passes > 256 {
            return Err("Resident point pass limit exceeded".into());
        }
        if passes == 0 {
            return Ok(());
        }
        uniform[8] = self.vertices;
        self.queue
            .write_buffer(&self.parameters, 0, bytemuck::cast_slice(&uniform));
        self.queue
            .write_buffer(&self.failure, 0, bytemuck::bytes_of(&u32::MAX));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        if matches!(source, CandidateSource::BrushRay) {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.brush_pick_direction_pipeline);
            pass.set_bind_group(0, &self.brush_pick_direction_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let (candidate_pipeline, candidate_group) = match source {
            CandidateSource::Center => (&self.candidate_pipeline, &self.candidate_group),
            CandidateSource::BrushCenter => {
                (&self.brush_candidate_pipeline, &self.brush_candidate_group)
            }
            CandidateSource::BrushRay => (
                &self.brush_picked_candidate_pipeline,
                &self.brush_picked_candidate_group,
            ),
            CandidateSource::Ray => (
                &self.picked_candidate_pipeline,
                &self.picked_candidate_group,
            ),
            CandidateSource::Depth => (
                &self.depth_candidate_pipeline,
                self.depth_candidate_group
                    .as_ref()
                    .ok_or("Missing depth candidate binding")?,
            ),
        };
        for _ in 0..passes {
            for (pipeline, group, count) in [
                (candidate_pipeline, candidate_group, self.vertices),
                (
                    &self.normals.face_candidate_pipeline,
                    &self.normals.face_candidate_group,
                    self.normals.face_count,
                ),
                (
                    &self.normals.mean_preflight_pipeline,
                    &self.normals.mean_preflight_group,
                    self.vertices,
                ),
                (&self.commit_pipeline, &self.commit_group, self.vertices),
                (
                    &self.normals.face_commit_pipeline,
                    &self.normals.face_commit_group,
                    self.normals.face_count,
                ),
                (
                    &self.normals.vertex_commit_pipeline,
                    &self.normals.vertex_commit_group,
                    self.vertices,
                ),
            ] {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.dispatch_workgroups(count.div_ceil(256), 1, 1);
            }
        }
        self.queue.submit([encoder.finish()]);
        self.point_passes += u64::from(passes);
        if !matches!(
            source,
            CandidateSource::Center | CandidateSource::BrushCenter
        ) {
            self.bounds.include_unread_point_program(radius);
        } else {
            self.bounds.include_point_program(center, radius);
        }
        self.depth_ready = false;
        Ok(())
    }
    pub fn draw(&mut self) -> Result<(), String> {
        self.ready()?;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let color = self.color.create_view(&Default::default());
        let depth = self.depth.create_view(&Default::default());
        surface::encode_scene(
            &mut encoder,
            surface::SceneDraw {
                pipeline: &self.draw_pipeline,
                group: &self.draw_group,
                indices: &self.indices,
                index_count: self.index_count,
            },
            &color,
            &depth,
        );
        self.queue.submit([encoder.finish()]);
        self.draws += 1;
        self.depth_ready = true;
        Ok(())
    }
    pub fn present(&mut self) -> Result<bool, String> {
        self.ready()?;
        let surface = self.surface.as_mut().ok_or("No canvas surface")?;
        let presented = surface.draw(&self.device, &self.queue, &self.indices, self.index_count)?;
        self.depth_ready = presented;
        Ok(presented)
    }
    /// Caller supplies the actual OBrush local-to-GL-clip matrix. Checked
    /// conversion changes only clip Z for WebGPU; both draw targets use it.
    pub fn set_camera_matrix(
        &mut self,
        local_to_gl_clip: [f64; 16],
        extent: [u32; 2],
    ) -> Result<(), String> {
        let view = ResidentViewMatrix::checked(local_to_gl_clip)?;
        self.publish_view(view, extent)
    }
    pub fn set_document_camera_matrix(
        &mut self,
        local_to_gl_clip: [f64; 16],
        local_to_world: [f32; 16],
        extent: [u32; 2],
    ) -> Result<(), String> {
        let view =
            ResidentViewMatrix::checked(local_to_gl_clip)?.with_model_pose(local_to_world)?;
        self.publish_view(view, extent)
    }
    fn publish_view(&mut self, view: ResidentViewMatrix, extent: [u32; 2]) -> Result<(), String> {
        self.ready()?;
        let expected = self
            .surface
            .as_ref()
            .map_or([SIZE, SIZE], |s| s.camera_extent());
        if extent != expected || extent.contains(&0) {
            return Err("Camera viewport extent does not match GPU draw target".into());
        }
        self.queue.write_buffer(
            &self.offscreen_view,
            0,
            bytemuck::cast_slice(&view.uniform()),
        );
        if let Some(surface) = self.surface.as_mut() {
            surface.set_camera(&self.queue, view);
        }
        self.camera_active = true;
        self.camera_inverse = Some(view.gpu_clip_to_local);
        self.depth_ready = false;
        Ok(())
    }
    pub fn validate_surface(&self, width: u32, height: u32) -> Result<(), String> {
        self.ready()?;
        self.surface
            .as_ref()
            .ok_or("No canvas surface")?
            .validate(width, height)
    }
    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.ready()?;
        self.depth_ready = false;
        self.surface.as_mut().ok_or("No canvas surface")?.resize(
            &self.device,
            &self.queue,
            width,
            height,
        )
    }
    pub async fn pick(&mut self, ray: PickRay) -> Result<Option<PickHit>, String> {
        self.ready()?;
        if self.surface.as_ref().is_some_and(|s| s.camera_stale()) {
            return Err("Surface camera must be republished after resize".into());
        }
        self.pick_queued = false;
        self.picker.encode(&self.device, &self.queue, ray)?;
        let bytes = self.read_buffer(self.picker.result(), 8).await?;
        let hit = self.picker.decode(ray, &bytes)?;
        self.pick_readbacks += 1;
        self.pick_hits += u64::from(hit.is_some());
        Ok(hit)
    }
    pub fn prepare_local_ray(
        &self,
        ray: obrush_core::SceneRay64,
        max_distance: f64,
    ) -> Result<PickRay, String> {
        self.ready()?;
        PickRay::from_local_scene(ray, max_distance, self.bounds)
    }
    pub async fn pick_pixel(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<Option<PickHit>, String> {
        if self.camera_active {
            return Err("Camera mode requires the actual local-space camera ray".into());
        }
        let ray = PickRay::diagnostic_pixel(x, y, width, height)?;
        self.pick(ray).await
    }
    async fn read_buffer(&self, source: &wgpu::Buffer, size: u64) -> Result<Vec<u8>, String> {
        self.ready()?;
        if size == 0 || !size.is_multiple_of(4) {
            return Err("Diagnostic buffer size must be nonzero and four-byte aligned".into());
        }
        let length = usize::try_from(size).map_err(|_| "Diagnostic output too large")?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(length)
            .map_err(|_| "Diagnostic output allocation refused")?;
        let chunk = size.min(surface::READBACK_CHUNK);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bounded reusable diagnostic readback"),
            size: chunk,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut offset = 0;
        while offset < size {
            let count = chunk.min(size - offset);
            let mut encoder = self.device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(source, offset, &buffer, 0, count);
            self.queue.submit([encoder.finish()]);
            let (sender, receiver) = futures_channel::oneshot::channel();
            buffer.map_async(wgpu::MapMode::Read, ..count, move |result| {
                let _ = sender.send(result.map_err(|e| e.to_string()));
            });
            #[cfg(not(target_arch = "wasm32"))]
            self.device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .map_err(|e| e.to_string())?;
            receiver.await.map_err(|_| "GPU mapping callback lost")??;
            output.extend_from_slice(&buffer.get_mapped_range(..count));
            buffer.unmap();
            self.ready()?;
            offset += count;
        }
        buffer.destroy();
        if output.len() != length {
            return Err("Incomplete diagnostic GPU readback".into());
        }
        Ok(output)
    }
    async fn map(&self, buffer: &wgpu::Buffer) -> Result<Vec<u8>, String> {
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = sender.send(result.map_err(|e| e.to_string()));
        });
        #[cfg(not(target_arch = "wasm32"))]
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .map_err(|e| e.to_string())?;
        receiver.await.map_err(|_| "GPU mapping callback lost")??;
        let bytes = buffer.get_mapped_range(..).to_vec();
        buffer.unmap();
        self.ready()?;
        Ok(bytes)
    }
    pub async fn positions(&mut self) -> Result<Vec<u8>, String> {
        let bytes = self
            .read_buffer(&self.positions, self.position_bytes)
            .await?;
        self.position_readbacks += 1;
        Ok(bytes)
    }
    /// Compact vertices changed since the owned checkpoint on the GPU. Only
    /// a four-byte count and the changed ID/position records cross to Rust.
    /// Atomic append order is normalized before passing records to the core.
    pub async fn sparse_positions(&mut self) -> Result<Vec<u8>, String> {
        self.ready()?;
        if !self.checkpoint_valid {
            return Err("No owned resident checkpoint for sparse positions".into());
        }
        self.queue
            .write_buffer(&self.sparse.counter, 0, &0u32.to_le_bytes());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.sparse.encode_count(&mut encoder, self.vertices);
        self.queue.submit([encoder.finish()]);
        let count_bytes = self.read_buffer(&self.sparse.counter, 4).await?;
        let count = u32::from_le_bytes(count_bytes[..4].try_into().unwrap());
        if count > self.vertices {
            return Err("Sparse GPU count exceeds source vertex count".into());
        }
        if count == 0 {
            self.sparse_readbacks += 1;
            return Ok(Vec::new());
        }
        let payload_bytes = u64::from(count)
            .checked_mul(20)
            .ok_or("Sparse GPU payload size overflow")?;
        if self
            .resident_bytes
            .checked_add(payload_bytes)
            .and_then(|n| n.checked_add(surface::READBACK_CHUNK))
            .is_none_or(|n| n > MAX_GPU_BYTES)
            || payload_bytes > u64::from(self.device.limits().max_storage_buffer_binding_size)
        {
            return Err("Sparse GPU payload exceeds bounded residency".into());
        }
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("resident sparse changed-row records"),
            size: payload_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        self.queue
            .write_buffer(&self.sparse.counter, 0, &0u32.to_le_bytes());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.sparse
            .encode_collect(&self.device, &mut encoder, &output, self.vertices);
        self.queue.submit([encoder.finish()]);
        let result: Result<Vec<u8>, String> = async {
            let bytes = self.read_buffer(&output, payload_bytes).await?;
            let actual_bytes = self.read_buffer(&self.sparse.counter, 4).await?;
            let actual = u32::from_le_bytes(actual_bytes[..4].try_into().unwrap());
            if actual != count {
                return Err("Sparse GPU count changed during collection".into());
            }
            let mut rows = Vec::new();
            rows.try_reserve_exact(count as usize)
                .map_err(|_| "Cannot reserve sparse GPU result rows")?;
            for row in bytes.chunks_exact(20) {
                let id = u32::from_le_bytes(row[..4].try_into().unwrap());
                if id >= self.vertices {
                    return Err("Sparse GPU result vertex ID is out of range".into());
                }
                rows.push((id, row.try_into().unwrap()));
            }
            rows.sort_unstable_by_key(|(id, _): &(u32, [u8; 20])| *id);
            if rows.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return Err("Sparse GPU result duplicated a vertex ID".into());
            }
            let mut sorted = Vec::new();
            sorted
                .try_reserve_exact(bytes.len())
                .map_err(|_| "Cannot reserve sorted sparse GPU result")?;
            for (_, row) in rows {
                sorted.extend_from_slice(&row);
            }
            Ok(sorted)
        }
        .await;
        output.destroy();
        let sorted = result?;
        self.sparse_readbacks += 1;
        self.sparse_payload_bytes += payload_bytes;
        Ok(sorted)
    }
    /// Failure words encode point vertex, original face, or vertex mean domains.
    pub async fn normal_state(&mut self, channel: u32) -> Result<Vec<u8>, String> {
        let bytes = match channel {
            0 => {
                self.read_buffer(&self.normal_u, self.position_bytes)
                    .await?
            }
            1 => {
                self.read_buffer(&self.normals.faces, self.normals.face_bytes)
                    .await?
            }
            2 => {
                self.read_buffer(&self.normals.visited, u64::from(self.vertices) * 4)
                    .await?
            }
            3 => {
                self.read_buffer(&self.normals.selected, u64::from(self.vertices) * 4)
                    .await?
            }
            _ => return Err("Unknown retained normal channel".into()),
        };
        self.normal_readbacks += 1;
        Ok(bytes)
    }
    pub async fn failure_vertex(&mut self) -> Result<Option<u32>, String> {
        let bytes = self.read_buffer(&self.failure, 4).await?;
        let word = u32::from_le_bytes(bytes.try_into().map_err(|_| "GPU failure word length")?);
        Ok((word != u32::MAX).then_some(word))
    }
    pub async fn pixels(&mut self) -> Result<Vec<u8>, String> {
        self.ready()?;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("complete viewport readback"),
            size: u64::from(SIZE) * u64::from(SIZE) * 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.color,
                mip_level: 0,
                origin: Default::default(),
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SIZE * 4),
                    rows_per_image: Some(SIZE),
                },
            },
            wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let bytes = self.map(&buffer).await?;
        self.pixel_readbacks += 1;
        Ok(bytes)
    }
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"adapter":format!("{:?}",self.adapter),"vertices":self.vertices,"triangles":self.index_count/3,"resident_bytes":self.resident_bytes,"surface":self.surface.as_ref().map(|s|s.report()),"camera_active":self.camera_active,"position_readbacks":self.position_readbacks,"sparse_readbacks":self.sparse_readbacks,"sparse_payload_bytes":self.sparse_payload_bytes,"normal_state_readbacks":self.normal_readbacks,"pick_readbacks":self.pick_readbacks,"pick_hits":self.pick_hits,"original_faces":self.normals.face_count,"pixel_readbacks":self.pixel_readbacks,"draws_enqueued":self.draws,"point_passes_enqueued":self.point_passes,"checkpoint_enqueued":self.checkpoint_valid,"destroyed":self.destroyed,"device_lost":self.device_lost.lock().map(|x|x.clone()).unwrap_or_else(|_|Some("poisoned".into())),"errors":self.errors.lock().map(|e|e.clone()).unwrap_or_else(|_|vec!["poisoned".into()]),"scope":"Enqueued work counters only; actual mapped outputs establish completion. Retained selected normals and GPU ray pick; point selection is not production Source brush/contact/proxy/history ownership."})
    }
    pub fn destroy(&mut self) {
        if !self.destroyed {
            for buffer in [
                &self.positions,
                &self.checkpoint,
                &self.candidates,
                &self.normal_u,
                &self.colors,
                &self.indices,
                &self.failure,
                &self.parameters,
                &self.picked_brush_direction,
                &self.offscreen_view,
            ] {
                buffer.destroy();
            }
            self.normals.destroy();
            self.picker.destroy();
            self.sparse.destroy();
            if let Some(surface) = self.surface.as_mut() {
                surface.destroy();
            }
            self.color.destroy();
            self.depth.destroy();
            self.device.destroy();
            self.destroyed = true;
        }
    }
}
impl Drop for ResidentViewport {
    fn drop(&mut self) {
        self.destroy();
    }
}

pub fn capture_sgl(bytes: &[u8]) -> Result<ResidentPointInput, String> {
    let mut document = obrush_core::Document::new(obrush_core::Mesh::sphere(0));
    document.replace_sgl_bytes(bytes, &obrush_core::SglStageSettings::default())?;
    let mesh = document.active_mesh_mut().ok_or("Missing imported mesh")?;
    if mesh.positions.len() != 999002 || mesh.triangles.len() != 1998000 {
        return Err("Prototype requires original complete million-point fixture".into());
    }
    mesh.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
    mesh.masks.fill(1.);
    ResidentPointInput::capture(mesh)
}
pub fn parameters() -> PointInflateParameters {
    PointInflateParameters {
        center: [0., 0., 1.],
        radius: 0.5,
        intensity: 0.25,
    }
}
pub fn png(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut output = Vec::new();
    image::codecs::png::PngEncoder::new(&mut output)
        .write_image(bytes, SIZE, SIZE, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(output)
}
#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use wasm_bindgen::prelude::*;
    /// An isolated browser-side document authority for testing the same
    /// checked result/undo boundary as the native Metal probe. The viewport
    /// remains separate and is not synchronized by undo or redo here.
    #[wasm_bindgen]
    pub struct ResidentDocumentSession {
        document: obrush_core::Document,
        captured: obrush_core::resident_point_program::ResidentDocumentInput,
    }
    #[wasm_bindgen]
    impl ResidentDocumentSession {
        pub fn load_project(bytes: Vec<u8>) -> Result<ResidentDocumentSession, JsValue> {
            let source = std::str::from_utf8(&bytes)
                .map_err(|_| JsValue::from_str("Project is not UTF-8"))?;
            let document =
                obrush_core::Document::from_json(source).map_err(|e| JsValue::from_str(&e))?;
            let captured =
                obrush_core::resident_point_program::ResidentDocumentInput::capture(&document)
                    .map_err(|e| JsValue::from_str(&e))?;
            Ok(Self { document, captured })
        }
        pub fn source_vertices(&self) -> usize {
            self.captured.points().positions().len()
        }
        pub fn project_json(&self) -> Result<String, JsValue> {
            self.document.to_json().map_err(|e| JsValue::from_str(&e))
        }
        pub fn commit_position_bytes(&mut self, bytes: &[u8]) -> Result<bool, JsValue> {
            self.captured
                .commit_position_bytes(&mut self.document, bytes)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn commit_sparse_position_bytes(&mut self, bytes: &[u8]) -> Result<bool, JsValue> {
            self.captured
                .commit_sparse_position_bytes(&mut self.document, bytes)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn undo(&mut self) -> bool {
            self.document.undo()
        }
        pub fn redo(&mut self) -> bool {
            self.document.redo()
        }
    }
    #[wasm_bindgen]
    pub struct ResidentProbe {
        viewport: ResidentViewport,
        input: ResidentPointInput,
        reference: Option<[Vec<u8>; 4]>,
        canvas: Option<web_sys::HtmlCanvasElement>,
        project_pose: Option<[f32; 16]>,
        project_subtool_id: Option<u32>,
    }
    #[wasm_bindgen]
    impl ResidentProbe {
        pub async fn load(bytes: Vec<u8>) -> Result<ResidentProbe, JsValue> {
            let input = capture_sgl(&bytes).map_err(|e| JsValue::from_str(&e))?;
            let viewport = ResidentViewport::new(&input)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            Ok(Self {
                viewport,
                input,
                reference: None,
                canvas: None,
                project_pose: None,
                project_subtool_id: None,
            })
        }
        pub async fn load_canvas(
            bytes: Vec<u8>,
            canvas: web_sys::HtmlCanvasElement,
        ) -> Result<ResidentProbe, JsValue> {
            let input = capture_sgl(&bytes).map_err(|e| JsValue::from_str(&e))?;
            let extent = [canvas.width(), canvas.height()];
            let viewport = ResidentViewport::create(
                &input,
                Some(wgpu::SurfaceTarget::Canvas(canvas.clone())),
                Some(extent),
            )
            .await
            .map_err(|e| JsValue::from_str(&e))?;
            Ok(Self {
                viewport,
                input,
                reference: None,
                canvas: Some(canvas),
                project_pose: None,
                project_subtool_id: None,
            })
        }
        /// Load the ordinary versioned .obrush format into an isolated canvas.
        /// The saved document is decoded and validated but never replaced or
        /// mutated by this diagnostic GPU owner.
        pub async fn load_project_canvas(
            bytes: Vec<u8>,
            canvas: web_sys::HtmlCanvasElement,
        ) -> Result<ResidentProbe, JsValue> {
            let snapshot =
                obrush_core::resident_point_program::ResidentDocumentInput::from_project_bytes(
                    &bytes,
                )
                .map_err(|e| JsValue::from_str(&e))?;
            let pose = *snapshot.local_to_world();
            let subtool_id = snapshot.subtool_id();
            let input = snapshot.into_points();
            let extent = [canvas.width(), canvas.height()];
            let viewport = ResidentViewport::create(
                &input,
                Some(wgpu::SurfaceTarget::Canvas(canvas.clone())),
                Some(extent),
            )
            .await
            .map_err(|e| JsValue::from_str(&e))?;
            Ok(Self {
                viewport,
                input,
                reference: None,
                canvas: Some(canvas),
                project_pose: Some(pose),
                project_subtool_id: Some(subtool_id),
            })
        }
        /// Decode only the active mesh/pose handoff, without a project JSON
        /// copy or authority to edit the studio Document.
        pub async fn load_upload_canvas(
            bytes: Vec<u8>,
            canvas: web_sys::HtmlCanvasElement,
        ) -> Result<ResidentProbe, JsValue> {
            let upload =
                obrush_core::resident_upload::decode(&bytes).map_err(|e| JsValue::from_str(&e))?;
            let extent = [canvas.width(), canvas.height()];
            let viewport = ResidentViewport::create(
                &upload.input,
                Some(wgpu::SurfaceTarget::Canvas(canvas.clone())),
                Some(extent),
            )
            .await
            .map_err(|e| JsValue::from_str(&e))?;
            Ok(Self {
                viewport,
                input: upload.input,
                reference: None,
                canvas: Some(canvas),
                project_pose: Some(upload.local_to_world),
                project_subtool_id: Some(upload.subtool_id),
            })
        }
        pub fn project_subtool_id(&self) -> Option<u32> {
            self.project_subtool_id
        }
        pub fn present(&mut self) -> Result<bool, JsValue> {
            self.viewport.present().map_err(|e| JsValue::from_str(&e))
        }
        pub fn set_camera_gl_matrix(
            &mut self,
            columns: Vec<f64>,
            width: u32,
            height: u32,
        ) -> Result<(), JsValue> {
            let matrix: [f64; 16] = columns
                .try_into()
                .map_err(|_| JsValue::from_str("Camera matrix needs exactly 16 f64 entries"))?;
            self.viewport
                .set_camera_matrix(matrix, [width, height])
                .map_err(|e| JsValue::from_str(&e))
        }
        /// Accept the studio's world-to-GL-clip matrix. The saved object's
        /// local-to-world pose is composed in f64 before WebGPU f32 upload.
        pub fn set_world_camera_gl_matrix(
            &mut self,
            columns: Vec<f64>,
            width: u32,
            height: u32,
        ) -> Result<(), JsValue> {
            let pose = self
                .project_pose
                .ok_or_else(|| JsValue::from_str("No project pose; use load_project_canvas"))?;
            let world: [f64; 16] = columns
                .try_into()
                .map_err(|_| JsValue::from_str("Camera matrix needs exactly 16 f64 entries"))?;
            let local = obrush_core::resident_point_program::compose_document_camera(pose, world)
                .map_err(|e| JsValue::from_str(&e))?;
            self.viewport
                .set_document_camera_matrix(local, pose, [width, height])
                .map_err(|e| JsValue::from_str(&e))
        }
        pub async fn pick_local_ray(
            &mut self,
            components: Vec<f64>,
            max_distance: f64,
        ) -> Result<String, JsValue> {
            let [
                origin_x,
                origin_y,
                origin_z,
                direction_x,
                direction_y,
                direction_z,
            ]: [f64; 6] = components.try_into().map_err(|_| {
                JsValue::from_str("Local pick ray needs exactly six f64 components")
            })?;
            let ray = self
                .viewport
                .prepare_local_ray(
                    obrush_core::SceneRay64 {
                        origin: glam::DVec3::new(origin_x, origin_y, origin_z),
                        direction: glam::DVec3::new(direction_x, direction_y, direction_z),
                    },
                    max_distance,
                )
                .map_err(|e| JsValue::from_str(&e))?;
            let hit = self
                .viewport
                .pick(ray)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            Ok(hit
                .map_or_else(
                    || serde_json::Value::Null,
                    |hit| {
                        serde_json::json!({"face":hit.face,"distance":hit.distance,"position":hit.position})
                    },
                )
                .to_string())
        }
        pub async fn pick_world_ray(
            &mut self,
            components: Vec<f64>,
            max_world_distance: f64,
        ) -> Result<String, JsValue> {
            let pose = self
                .project_pose
                .ok_or_else(|| JsValue::from_str("No project pose; use load_project_canvas"))?;
            let [
                origin_x,
                origin_y,
                origin_z,
                direction_x,
                direction_y,
                direction_z,
            ]: [f64; 6] = components.try_into().map_err(|_| {
                JsValue::from_str("World pick ray needs exactly six f64 components")
            })?;
            let world_ray = obrush_core::SceneRay64 {
                origin: glam::DVec3::new(origin_x, origin_y, origin_z),
                direction: glam::DVec3::new(direction_x, direction_y, direction_z),
            };
            let local = obrush_core::resident_point_program::document_local_ray(
                pose,
                world_ray,
                max_world_distance,
            )
            .map_err(|e| JsValue::from_str(&e))?;
            let ray = self
                .viewport
                .prepare_local_ray(local.ray, local.max_distance)
                .map_err(|e| JsValue::from_str(&e))?;
            let hit = self
                .viewport
                .pick(ray)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            Ok(hit
                .map_or_else(
                    || serde_json::Value::Null,
                    |hit| {
                        let matrix = glam::DMat4::from_cols_array(&pose.map(f64::from));
                        let world_position = matrix
                            .transform_point3(glam::DVec3::from_array(hit.position.map(f64::from)));
                        let world_distance =
                            (world_position - world_ray.origin).dot(world_ray.direction);
                        serde_json::json!({
                            "face":hit.face,
                            "local_distance":hit.distance,
                            "local_position":hit.position,
                            "world_distance":world_distance,
                            "world_position":world_position.to_array(),
                        })
                    },
                )
                .to_string())
        }
        /// A GPU-owned pick/deform dab. The browser need not map the 8-byte
        /// pick result before dispatching the point pass.
        pub fn advance_world_ray(
            &mut self,
            components: Vec<f64>,
            max_world_distance: f64,
            radius: f32,
            intensity: f32,
        ) -> Result<(), JsValue> {
            let pose = self
                .project_pose
                .ok_or_else(|| JsValue::from_str("No project pose; use load_project_canvas"))?;
            let [
                origin_x,
                origin_y,
                origin_z,
                direction_x,
                direction_y,
                direction_z,
            ]: [f64; 6] = components.try_into().map_err(|_| {
                JsValue::from_str("World pick ray needs exactly six f64 components")
            })?;
            let local = obrush_core::resident_point_program::document_local_ray(
                pose,
                obrush_core::SceneRay64 {
                    origin: glam::DVec3::new(origin_x, origin_y, origin_z),
                    direction: glam::DVec3::new(direction_x, direction_y, direction_z),
                },
                max_world_distance,
            )
            .map_err(|e| JsValue::from_str(&e))?;
            let ray = self
                .viewport
                .prepare_local_ray(local.ray, local.max_distance)
                .map_err(|e| JsValue::from_str(&e))?;
            self.viewport
                .inflate_picked(ray, radius, intensity)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn advance_depth_pixel(
            &mut self,
            x: u32,
            y: u32,
            radius: f32,
            intensity: f32,
        ) -> Result<(), JsValue> {
            self.viewport
                .inflate_depth_pixel(x, y, radius, intensity)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn queue_world_ray_pick(
            &mut self,
            components: Vec<f64>,
            max_world_distance: f64,
        ) -> Result<(), JsValue> {
            let pose = self
                .project_pose
                .ok_or_else(|| JsValue::from_str("No project pose; use load_project_canvas"))?;
            let [
                origin_x,
                origin_y,
                origin_z,
                direction_x,
                direction_y,
                direction_z,
            ]: [f64; 6] = components.try_into().map_err(|_| {
                JsValue::from_str("World pick ray needs exactly six f64 components")
            })?;
            let local = obrush_core::resident_point_program::document_local_ray(
                pose,
                obrush_core::SceneRay64 {
                    origin: glam::DVec3::new(origin_x, origin_y, origin_z),
                    direction: glam::DVec3::new(direction_x, direction_y, direction_z),
                },
                max_world_distance,
            )
            .map_err(|e| JsValue::from_str(&e))?;
            let ray = self
                .viewport
                .prepare_local_ray(local.ray, local.max_distance)
                .map_err(|e| JsValue::from_str(&e))?;
            self.viewport
                .queue_picked_ray(ray)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn advance_last_pick(&mut self, radius: f32, intensity: f32) -> Result<(), JsValue> {
            self.viewport
                .inflate_last_pick(radius, intensity)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn advance_brush_last_pick(
            &mut self,
            radius: f32,
            intensity: f32,
        ) -> Result<(), JsValue> {
            self.viewport
                .brush_last_pick(radius, intensity)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn advance_brush_world_ray(
            &mut self,
            components: Vec<f64>,
            max_world_distance: f64,
            radius: f32,
            intensity: f32,
        ) -> Result<(), JsValue> {
            let pose = self
                .project_pose
                .ok_or_else(|| JsValue::from_str("No project pose; use load_project_canvas"))?;
            let [
                origin_x,
                origin_y,
                origin_z,
                direction_x,
                direction_y,
                direction_z,
            ]: [f64; 6] = components.try_into().map_err(|_| {
                JsValue::from_str("World pick ray needs exactly six f64 components")
            })?;
            let local = obrush_core::resident_point_program::document_local_ray(
                pose,
                obrush_core::SceneRay64 {
                    origin: glam::DVec3::new(origin_x, origin_y, origin_z),
                    direction: glam::DVec3::new(direction_x, direction_y, direction_z),
                },
                max_world_distance,
            )
            .map_err(|e| JsValue::from_str(&e))?;
            let ray = self
                .viewport
                .prepare_local_ray(local.ray, local.max_distance)
                .map_err(|e| JsValue::from_str(&e))?;
            self.viewport
                .brush_picked(ray, radius, intensity)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub async fn wait_submitted(&self) -> Result<(), JsValue> {
            self.viewport
                .wait_submitted()
                .await
                .map_err(|e| JsValue::from_str(&e))
        }
        pub async fn pick_pixel(&mut self, x: u32, y: u32) -> Result<String, JsValue> {
            let canvas = self
                .canvas
                .as_ref()
                .ok_or_else(|| JsValue::from_str("No canvas surface"))?;
            let hit = self
                .viewport
                .pick_pixel(x, y, canvas.width(), canvas.height())
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            Ok(hit.map_or_else(|| serde_json::Value::Null, |hit| {
                serde_json::json!({"face":hit.face,"distance":hit.distance,"position":hit.position})
            }).to_string())
        }
        pub fn advance_at(
            &mut self,
            x: f32,
            y: f32,
            z: f32,
            radius: f32,
            intensity: f32,
            passes: u32,
        ) -> Result<(), JsValue> {
            self.viewport
                .inflate(
                    PointInflateParameters {
                        center: [x, y, z],
                        radius,
                        intensity,
                    },
                    passes,
                )
                .map_err(|e| JsValue::from_str(&e))
        }
        /// Diagnostic explicit-direction ordinary Brush point pass. The
        /// caller must provide a checked picked direction; this does not
        /// replace the studio's pointer/alpha/proxy brush transaction.
        pub fn advance_brush_at(
            &mut self,
            center: Vec<f32>,
            direction: Vec<f32>,
            radius: f32,
            intensity: f32,
            passes: u32,
        ) -> Result<(), JsValue> {
            let center: [f32; 3] = center
                .try_into()
                .map_err(|_| JsValue::from_str("Brush center needs exactly three components"))?;
            let direction: [f32; 3] = direction
                .try_into()
                .map_err(|_| JsValue::from_str("Brush direction needs exactly three components"))?;
            self.viewport
                .brush(
                    PointBrushParameters {
                        center,
                        direction,
                        radius,
                        intensity,
                    },
                    passes,
                )
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
            self.viewport
                .validate_surface(width, height)
                .map_err(|e| JsValue::from_str(&e))?;
            let canvas = self
                .canvas
                .as_ref()
                .ok_or_else(|| JsValue::from_str("No canvas surface"))?;
            canvas.set_width(width);
            canvas.set_height(height);
            self.viewport
                .resize_surface(width, height)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn checkpoint(&mut self) -> Result<(), JsValue> {
            self.viewport
                .checkpoint()
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn advance(&mut self, passes: u32) -> Result<(), JsValue> {
            self.viewport
                .inflate(parameters(), passes)
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn cancel(&mut self) -> Result<(), JsValue> {
            self.viewport.cancel().map_err(|e| JsValue::from_str(&e))
        }
        pub fn draw(&mut self) -> Result<(), JsValue> {
            self.viewport.draw().map_err(|e| JsValue::from_str(&e))
        }
        pub fn invalid_parameters_rejected(&mut self) -> bool {
            self.viewport
                .inflate(
                    PointInflateParameters {
                        radius: f32::NAN,
                        ..parameters()
                    },
                    1,
                )
                .is_err()
        }
        pub async fn positions(&mut self) -> Result<Vec<u8>, JsValue> {
            self.viewport
                .positions()
                .await
                .map_err(|e| JsValue::from_str(&e))
        }
        pub async fn sparse_positions(&mut self) -> Result<Vec<u8>, JsValue> {
            self.viewport
                .sparse_positions()
                .await
                .map_err(|e| JsValue::from_str(&e))
        }
        pub async fn pixels_png(&mut self) -> Result<Vec<u8>, JsValue> {
            let bytes = self
                .viewport
                .pixels()
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            png(&bytes).map_err(|e| JsValue::from_str(&e))
        }
        pub async fn failure_vertex(&mut self) -> Result<JsValue, JsValue> {
            let vertex = self
                .viewport
                .failure_vertex()
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            Ok(vertex.map_or(JsValue::NULL, JsValue::from))
        }
        pub fn prepare_reference(&mut self, passes: u32) -> Result<(), JsValue> {
            if passes > 256 {
                return Err(JsValue::from_str("Reference pass cap"));
            }
            let mut state =
                ResidentNormalReference::new(&self.input).map_err(|e| JsValue::from_str(&e))?;
            for _ in 0..passes {
                state
                    .point_pass(parameters())
                    .map_err(|e| JsValue::from_str(&e))?;
            }
            self.reference = Some([
                bytemuck::cast_slice(state.positions()).to_vec(),
                bytemuck::cast_slice(state.normal_u()).to_vec(),
                bytemuck::cast_slice(state.face_normals()).to_vec(),
                bytemuck::cast_slice(state.visited()).to_vec(),
            ]);
            Ok(())
        }
        pub fn reference_bytes(&self, channel: u32) -> Result<Vec<u8>, JsValue> {
            self.reference
                .as_ref()
                .and_then(|r| r.get(channel as usize))
                .cloned()
                .ok_or_else(|| JsValue::from_str("Reference not prepared or invalid channel"))
        }
        pub async fn normal_state(&mut self, channel: u32) -> Result<Vec<u8>, JsValue> {
            self.viewport
                .normal_state(channel)
                .await
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn input_bytes(&self, channel: u32) -> Result<Vec<u8>, JsValue> {
            Ok(match channel {
                0 => bytemuck::cast_slice(self.input.positions()),
                1 => bytemuck::cast_slice(self.input.normal_u()),
                2 => bytemuck::cast_slice(self.input.colors()),
                3 => bytemuck::cast_slice(self.input.triangles()),
                4 => bytemuck::cast_slice(self.input.normal_topology().headers()),
                5 => bytemuck::cast_slice(self.input.normal_topology().offsets()),
                6 => bytemuck::cast_slice(self.input.normal_topology().rings()),
                7 => bytemuck::cast_slice(self.input.normal_topology().face_normals()),
                _ => return Err(JsValue::from_str("Unknown immutable input channel")),
            }
            .to_vec())
        }
        pub fn report(&self) -> String {
            self.viewport.report().to_string()
        }
        pub async fn normal_case_trials() -> Result<String, JsValue> {
            super::normal_case_trials()
                .await
                .map(|v| v.to_string())
                .map_err(|e| JsValue::from_str(&e))
        }
        pub async fn invalid_candidate_trial() -> Result<String, JsValue> {
            super::invalid_candidate_trial()
                .await
                .map(|v| v.to_string())
                .map_err(|e| JsValue::from_str(&e))
        }
        pub fn destroy(&mut self) {
            self.viewport.destroy();
        }
    }
}

pub async fn invalid_candidate_trial() -> Result<serde_json::Value, String> {
    let mut m = obrush_core::Mesh::sphere(0);
    m.positions.clear();
    m.triangles.clear();
    for i in 0..256u32 {
        let at = m.positions.len() as u32;
        m.positions.extend_from_slice(&[
            obrush_core::Vec3::new(999999., 0., 0.),
            obrush_core::Vec3::new(999999., 1., 0.),
            obrush_core::Vec3::new(999999., 0., 1.),
        ]);
        m.triangles.push(if i < 128 {
            [at, at + 1, at + 2]
        } else {
            [at, at + 2, at + 1]
        });
    }
    m.normals = vec![obrush_core::Vec3::X; m.positions.len()];
    m.colors = vec![[0.5; 3]; m.positions.len()];
    m.masks = vec![1.; m.positions.len()];
    m.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
    m.materials = None;
    m.surface = None;
    let input = ResidentPointInput::capture(&m)?;
    let p = PointInflateParameters {
        center: [999999., 0., 0.],
        radius: 100.,
        intensity: 1.,
    };
    if input.reference_positions(p, 1).is_ok() {
        return Err("Bad candidate fixture did not reject scalar output".into());
    }
    let mut gpu = ResidentViewport::new(&input).await?;
    gpu.checkpoint()?;
    gpu.inflate(p, 3)?;
    let failure = gpu.failure_vertex().await?;
    let actual = gpu.positions().await?;
    let normals = gpu.normal_state(0).await?;
    let faces = gpu.normal_state(1).await?;
    let visited = gpu.normal_state(2).await?;
    let unchanged = actual == bytemuck::cast_slice::<[f32; 4], u8>(input.positions())
        && normals == bytemuck::cast_slice::<[f32; 4], u8>(input.normal_u())
        && faces == bytemuck::cast_slice::<[f32; 3], u8>(input.normal_topology().face_normals())
        && visited.iter().all(|v| *v == 0);
    let report = gpu.report();
    gpu.destroy();
    if failure.is_none() || !unchanged {
        return Err("Atomic retained-state candidate rejection failed".into());
    }
    Ok(
        serde_json::json!({"first_failure_vertex":failure,"all_retained_words_unchanged":unchanged,"vertices":input.positions().len(),"before":{"positions":words(bytemuck::cast_slice(input.positions())),"normal_u":words(bytemuck::cast_slice(input.normal_u())),"faces":words(bytemuck::cast_slice(input.normal_topology().face_normals()))},"after":{"positions":words(&actual),"normal_u":words(&normals),"faces":words(&faces),"visited":words(&visited)},"report":report}),
    )
}
fn words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect()
}

pub async fn normal_case_trials() -> Result<serde_json::Value, String> {
    let mut cases = Vec::new();
    for mode in 0..4 {
        let mut mesh = obrush_core::Mesh::sphere(2);
        if mode == 2 {
            mesh.positions = vec![
                obrush_core::Vec3::new(-0.2, -0.2, 0.8),
                obrush_core::Vec3::new(0.2, -0.2, 0.9),
                obrush_core::Vec3::new(0.2, 0.2, 1.05),
                obrush_core::Vec3::new(-0.2, 0.2, 0.95),
                obrush_core::Vec3::new(0., 0.8, 0.7),
                obrush_core::Vec3::new(-0.2, -0.2, -1.),
                obrush_core::Vec3::new(0.2, -0.2, -1.),
                obrush_core::Vec3::new(0., 0.2, -1.),
            ];
            mesh.triangles = vec![[0, 1, 2], [0, 2, 3], [3, 2, 4], [5, 6, 7]];
            mesh.surface = Some(obrush_core::SurfaceTopology {
                faces: vec![[0, 1, 2, 3], [3, 2, 4, u32::MAX], [5, 6, 7, u32::MAX]],
                uv: None,
            });
            mesh.normals = vec![obrush_core::Vec3::Z; 8];
            mesh.colors = vec![[0.5; 3]; 8];
            mesh.materials = None;
        }
        mesh.mask_convention = obrush_core::MaskConvention::UnprotectedF32V1;
        mesh.masks = (0..mesh.positions.len())
            .map(|i| if mode == 3 { 0. } else { [0., 0.25, 1.][i % 3] })
            .collect();
        let input = ResidentPointInput::capture(&mesh)?;
        let params = PointInflateParameters {
            intensity: if mode == 1 { -0.25 } else { 0.25 },
            ..parameters()
        };
        let mut reference = ResidentNormalReference::new(&input)?;
        for _ in 0..3 {
            reference.point_pass(params)?;
        }
        let mut gpu = ResidentViewport::new(&input).await?;
        gpu.checkpoint()?;
        gpu.inflate(params, 3)?;
        if gpu.failure_vertex().await?.is_some() {
            return Err("Unexpected small normal-case GPU failure".into());
        }
        let positions = gpu.positions().await?;
        let normals = gpu.normal_state(0).await?;
        let faces = gpu.normal_state(1).await?;
        let visited = gpu.normal_state(2).await?;
        let selection = gpu.normal_state(3).await?;
        gpu.cancel()?;
        let cancel_positions = gpu.positions().await?;
        let cancel_normals = gpu.normal_state(0).await?;
        let cancel_faces = gpu.normal_state(1).await?;
        let cancel_visited = gpu.normal_state(2).await?;
        let report = gpu.report();
        gpu.destroy();
        if cancel_positions != bytemuck::cast_slice::<[f32; 4], u8>(input.positions())
            || cancel_normals != bytemuck::cast_slice::<[f32; 4], u8>(input.normal_u())
            || cancel_faces
                != bytemuck::cast_slice::<[f32; 3], u8>(input.normal_topology().face_normals())
            || cancel_visited.iter().any(|v| *v != 0)
        {
            return Err("Small selected-normal checkpoint mismatch".into());
        }
        let case_name = [
            "fractional_mask",
            "negative",
            "mixed_quad_triangle",
            "all_protected",
        ][mode];
        cases.push(serde_json::json!({"name":case_name,"vertices":input.positions().len(),"passes":3,"center":params.center,"radius":params.radius,"intensity":params.intensity,"input":{"positions":words(bytemuck::cast_slice(input.positions())),"normal_u":words(bytemuck::cast_slice(input.normal_u())),"faces":words(bytemuck::cast_slice(input.normal_topology().face_normals())),"headers":input.normal_topology().headers(),"triangles":input.triangles(),"offsets":input.normal_topology().offsets(),"rings":input.normal_topology().rings()},"reference":{"positions":words(bytemuck::cast_slice(reference.positions())),"normal_u":words(bytemuck::cast_slice(reference.normal_u())),"faces":words(bytemuck::cast_slice(reference.face_normals())),"visited":reference.visited()},"actual":{"positions":words(&positions),"normal_u":words(&normals),"faces":words(&faces),"visited":words(&visited),"selection":words(&selection)},"cancel":{"positions":words(&cancel_positions),"normal_u":words(&cancel_normals),"faces":words(&cancel_faces),"visited":words(&cancel_visited)},"report":report}));
    }
    Ok(
        serde_json::json!({"status":"Completed4actualGPU selected-normal cases; independent cache/policy/numerical audit required","cases":cases}),
    )
}
