//! Direct scene presentation; no offscreen-image blit or CPU image upload.
use wgpu::util::DeviceExt;
pub const READBACK_CHUNK: u64 = 1024 * 1024;
pub const FRAME_BYTES_PER_PIXEL: u64 = 12; // requested2surface images and one depth target
pub const VIEW_UNIFORM_BYTES: u64 = crate::view::VIEW_UNIFORM_BYTES;
#[derive(Clone, Copy)]
pub struct PresentationBudget {
    pub fixed: u64,
    pub cap: u64,
    pub max_dimension: u32,
}
impl PresentationBudget {
    pub fn admit(self, width: u32, height: u32) -> Result<u64, String> {
        if width > self.max_dimension || height > self.max_dimension {
            return Err("Surface dimensions exceed requested device limit".into());
        }
        let extra = if width == 0 || height == 0 {
            VIEW_UNIFORM_BYTES
        } else {
            u64::from(width)
                .checked_mul(u64::from(height))
                .and_then(|n| n.checked_mul(FRAME_BYTES_PER_PIXEL))
                .and_then(|n| n.checked_add(VIEW_UNIFORM_BYTES))
                .ok_or("Surface resource arithmetic overflow")?
        };
        if self
            .fixed
            .checked_add(READBACK_CHUNK)
            .and_then(|n| n.checked_add(extra))
            .is_none_or(|n| n > self.cap)
        {
            return Err("Surface exceeds existing256MiB explicit resource budget".into());
        }
        Ok(extra)
    }
}
pub fn aspect(width: u32, height: u32) -> [f32; 4] {
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    [(h / w).min(1.), (w / h).min(1.), 0., 0.]
}
pub fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("resident position draw"),
        source: wgpu::ShaderSource::Wgsl(include_str!("draw.wgsl").into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shared resident scene pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}
pub fn group(
    device: &wgpu::Device,
    pipeline: &wgpu::RenderPipeline,
    buffers: [&wgpu::Buffer; 4],
    label: &str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
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
}
pub struct SceneDraw<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub group: &'a wgpu::BindGroup,
    pub indices: &'a wgpu::Buffer,
    pub index_count: u32,
}
pub fn encode_scene(
    encoder: &mut wgpu::CommandEncoder,
    scene: SceneDraw<'_>,
    color: &wgpu::TextureView,
    depth: &wgpu::TextureView,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("draw canonical resident scene"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: color,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.02,
                    g: 0.025,
                    b: 0.035,
                    a: 1.,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    pass.set_pipeline(scene.pipeline);
    pass.set_bind_group(0, scene.group, &[]);
    pass.set_index_buffer(scene.indices.slice(..), wgpu::IndexFormat::Uint32);
    pass.draw_indexed(0..scene.index_count, 0, 0..1);
}
pub struct SurfaceSetup<'a> {
    pub device: &'a wgpu::Device,
    pub adapter: &'a wgpu::Adapter,
    pub buffers: [&'a wgpu::Buffer; 3],
    pub budget: PresentationBudget,
    pub extent: [u32; 2],
}
pub struct SurfacePresenter {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    group: wgpu::BindGroup,
    view: wgpu::Buffer,
    depth: Option<wgpu::Texture>,
    budget: PresentationBudget,
    requested: [u32; 2],
    suspended: bool,
    resource_bytes: u64,
    enqueued: u64,
    reconfigures: u64,
    needs_reconfigure: bool,
    camera_active: bool,
    camera_stale: bool,
}
impl SurfacePresenter {
    pub fn new(surface: wgpu::Surface<'static>, setup: SurfaceSetup<'_>) -> Result<Self, String> {
        let [width, height] = setup.extent;
        let bytes = setup.budget.admit(width, height)?;
        let caps = surface.get_capabilities(setup.adapter);
        let format = [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
        ]
        .into_iter()
        .find(|f| caps.formats.contains(f))
        .ok_or("Prototype requires supported rgba/bgra8unorm surface")?;
        let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            *caps
                .alpha_modes
                .first()
                .ok_or("No supported surface alpha mode")?
        };
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            *caps
                .present_modes
                .first()
                .ok_or("No surface presentation mode")?
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            desired_maximum_frame_latency: 2,
            present_mode,
            alpha_mode,
            view_formats: vec![],
        };
        let view = setup
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("surface camera/diagnostic uniform"),
                contents: bytemuck::cast_slice(&crate::view::diagnostic_uniform(aspect(
                    width, height,
                ))),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let pipeline = pipeline(setup.device, format);
        let group = group(
            setup.device,
            &pipeline,
            [setup.buffers[0], setup.buffers[1], setup.buffers[2], &view],
            "canonical resident surface draw",
        );
        let suspended = width == 0 || height == 0;
        let mut result = Self {
            surface,
            config,
            pipeline,
            group,
            view,
            depth: None,
            budget: setup.budget,
            requested: [width, height],
            suspended,
            resource_bytes: bytes,
            enqueued: 0,
            reconfigures: 0,
            needs_reconfigure: false,
            camera_active: false,
            camera_stale: false,
        };
        if !suspended {
            result.configure(setup.device);
        }
        Ok(result)
    }
    fn configure(&mut self, device: &wgpu::Device) {
        if let Some(depth) = self.depth.take() {
            depth.destroy();
        }
        self.surface.configure(device, &self.config);
        self.depth = Some(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("direct surface depth"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }));
        self.reconfigures += 1;
        self.needs_reconfigure = false;
    }
    pub fn validate(&self, width: u32, height: u32) -> Result<(), String> {
        self.budget.admit(width, height).map(|_| ())
    }
    pub fn set_camera(&mut self, queue: &wgpu::Queue, view: crate::view::ResidentViewMatrix) {
        queue.write_buffer(&self.view, 0, bytemuck::cast_slice(&view.uniform()));
        self.camera_active = true;
        self.camera_stale = false;
    }
    pub fn camera_stale(&self) -> bool {
        self.camera_stale
    }
    pub fn depth_view(&self) -> Result<wgpu::TextureView, String> {
        if self.suspended || self.camera_stale {
            return Err("Surface depth is unavailable until camera and frame are current".into());
        }
        Ok(self
            .depth
            .as_ref()
            .ok_or("Missing surface depth")?
            .create_view(&Default::default()))
    }
    pub fn camera_extent(&self) -> [u32; 2] {
        self.requested
    }
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let bytes = self.budget.admit(width, height)?;
        self.requested = [width, height];
        if width == 0 || height == 0 {
            self.suspended = true;
            return Ok(());
        }
        self.config.width = width;
        self.config.height = height;
        self.configure(device);
        if self.camera_active {
            self.camera_stale = true;
        } else {
            queue.write_buffer(
                &self.view,
                0,
                bytemuck::cast_slice(&crate::view::diagnostic_uniform(aspect(width, height))),
            );
        }
        self.resource_bytes = bytes;
        self.suspended = false;
        Ok(())
    }
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        indices: &wgpu::Buffer,
        index_count: u32,
    ) -> Result<bool, String> {
        if self.suspended {
            return Ok(false);
        }
        if self.camera_stale {
            return Err("Surface camera must be republished after resize".into());
        }
        if self.needs_reconfigure {
            self.configure(device);
        }
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Timeout) => return Ok(false),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.configure(device);
                self.surface
                    .get_current_texture()
                    .map_err(|e| e.to_string())?
            }
            Err(e) => return Err(e.to_string()),
        };
        let color = frame.texture.create_view(&Default::default());
        let depth = self
            .depth
            .as_ref()
            .ok_or("Missing surface depth")?
            .create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        encode_scene(
            &mut encoder,
            SceneDraw {
                pipeline: &self.pipeline,
                group: &self.group,
                indices,
                index_count,
            },
            &color,
            &depth,
        );
        queue.submit([encoder.finish()]);
        self.needs_reconfigure = frame.suboptimal;
        frame.present();
        self.enqueued += 1;
        Ok(true)
    }
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"requested":self.requested,"configured":[self.config.width,self.config.height],"suspended":self.suspended,"camera_active":self.camera_active,"camera_stale":self.camera_stale,"format":format!("{:?}",self.config.format),"resource_bytes":self.resource_bytes,"frames_enqueued":self.enqueued,"reconfigurations":self.reconfigures,"scope":"Enqueued surface frames, not physical display completion. Suspended surfaces retain last configured resource accounting."})
    }
    pub fn destroy(&mut self) {
        if let Some(depth) = self.depth.take() {
            depth.destroy();
        }
        self.view.destroy();
        self.suspended = true;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_mesh_hd_fits_without_increasing_cap_and_oversize_rejects() {
        let b = PresentationBudget {
            fixed: 241857504,
            cap: 256 * 1024 * 1024,
            max_dimension: 8192,
        };
        assert_eq!(
            b.admit(1920, 1080).unwrap(),
            1920 * 1080 * FRAME_BYTES_PER_PIXEL + VIEW_UNIFORM_BYTES
        );
        assert!(b.admit(2400, 1350).is_err());
        assert!(b.admit(u32::MAX, u32::MAX).is_err());
        assert_eq!(b.admit(0, 1080).unwrap(), VIEW_UNIFORM_BYTES);
        assert!(b.admit(0, 8193).is_err());
    }
    #[test]
    fn aspect_preserves_a_square_frame_in_wide_and_tall_targets() {
        assert_eq!(aspect(512, 512), [1., 1., 0., 0.]);
        assert_eq!(aspect(1920, 1080), [0.5625, 1., 0., 0.]);
        assert_eq!(aspect(1080, 1920), [1., 0.5625, 0., 0.]);
    }
}
