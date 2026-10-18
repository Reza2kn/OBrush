//! Owned source-compatible linear viewport attachments, environment and merge.
//! The caller restores GL state and orders opaque objects before transparent ones.
use std::sync::{Arc, Weak};

use eframe::glow;
use glow::HasContext;

use crate::environment_glsl as shader;
use crate::environment_ibl::{EnvironmentAtlas, background_quad_half_extents};
use crate::uv_view::ViewTexture;

const MAX_LINEAR_TARGET_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BackgroundMode {
    #[default]
    Image,
    Environment,
    Ambient,
}

impl BackgroundMode {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Image => "Image / solid",
            Self::Environment => "Environment",
            Self::Ambient => "Ambient environment",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EnvironmentStyle {
    pub preset: u32,
    pub filmic: bool,
    pub darken_unselected: bool,
    pub custom_background: bool,
    pub background_mode: BackgroundMode,
    pub background_fill: bool,
    pub background_blur: f32,
    pub background_image: Option<Arc<ViewTexture>>,
}

impl Default for EnvironmentStyle {
    fn default() -> Self {
        Self {
            preset: 0,
            filmic: false,
            darken_unselected: true,
            custom_background: false,
            background_mode: BackgroundMode::Image,
            background_fill: true,
            background_blur: 0.0,
            background_image: None,
        }
    }
}

struct GpuImage {
    texture: glow::Texture,
    source: Weak<ViewTexture>,
}

struct GpuEnvironment {
    texture: glow::Texture,
    source: Weak<EnvironmentAtlas>,
}

pub(crate) struct LinearPipeline {
    merge: glow::Program,
    background: glow::Program,
    present: glow::Program,
    present_texture: Option<glow::UniformLocation>,
    present_size: Option<glow::UniformLocation>,
    present_antialias: Option<glow::UniformLocation>,
    target: Option<LinearTarget>,
    environment: Option<GpuEnvironment>,
    image: Option<GpuImage>,
    solid: glow::Texture,
    merge_opaque: Option<glow::UniformLocation>,
    merge_transparent: Option<glow::UniformLocation>,
    merge_filmic: Option<glow::UniformLocation>,
    bg_image: Option<glow::UniformLocation>,
    bg_type: Option<glow::UniformLocation>,
    bg_blur: Option<glow::UniformLocation>,
    bg_extents: Option<glow::UniformLocation>,
    bg_environment: EnvironmentUniforms,
}

pub(crate) struct EnvironmentUniforms {
    sampler: Option<glow::UniformLocation>,
    rotation: Option<glow::UniformLocation>,
    sh: Option<glow::UniformLocation>,
    size: Option<glow::UniformLocation>,
}

impl EnvironmentUniforms {
    pub(crate) unsafe fn new(gl: &glow::Context, program: glow::Program) -> Self {
        unsafe {
            Self {
                sampler: gl.get_uniform_location(program, "u_environment"),
                rotation: gl.get_uniform_location(program, "u_environment_rotation"),
                sh: gl.get_uniform_location(program, "u_environment_sh[0]"),
                size: gl.get_uniform_location(program, "u_environment_size"),
            }
        }
    }
    pub(crate) unsafe fn upload(
        &self,
        gl: &glow::Context,
        atlas: &EnvironmentAtlas,
        view_to_world: &[f32; 9],
    ) {
        unsafe {
            gl.uniform_1_i32(self.sampler.as_ref(), 2);
            gl.uniform_matrix_3_f32_slice(self.rotation.as_ref(), false, view_to_world);
            let coefficients: [f32; 27] =
                std::array::from_fn(|i| atlas.coefficients()[i / 3][i % 3] as f32);
            gl.uniform_3_f32_slice(self.sh.as_ref(), &coefficients);
            let dimensions = atlas.dimensions();
            gl.uniform_2_f32(
                self.size.as_ref(),
                dimensions[0] as f32,
                dimensions[1] as f32,
            );
        }
    }
}

impl LinearPipeline {
    pub(crate) unsafe fn new(gl: &glow::Context, header: &str) -> Result<Self, String> {
        unsafe {
            let merge_fragment = format!("{}{}", shader::COLOR, shader::MERGE);
            let merge = crate::renderer::compile_program(
                gl,
                header,
                shader::FULLSCREEN_VERTEX,
                &merge_fragment,
                "Linear merge",
            )?;
            let bg_fragment = format!("{}{}{}", shader::COLOR, shader::IBL, shader::BACKGROUND);
            let background = match crate::renderer::compile_program(
                gl,
                header,
                shader::BACKGROUND_VERTEX,
                &bg_fragment,
                "Linear background",
            ) {
                Ok(program) => program,
                Err(error) => {
                    gl.delete_program(merge);
                    return Err(error);
                }
            };
            let present = match crate::renderer::compile_program(
                gl,
                header,
                shader::FULLSCREEN_VERTEX,
                shader::PRESENT,
                "Display FXAA",
            ) {
                Ok(program) => program,
                Err(error) => {
                    gl.delete_program(merge);
                    gl.delete_program(background);
                    return Err(error);
                }
            };
            let solid = match upload_rgba(gl, [1, 1], &[50, 50, 50, 255], false, 3) {
                Ok(texture) => texture,
                Err(error) => {
                    gl.delete_program(merge);
                    gl.delete_program(background);
                    gl.delete_program(present);
                    return Err(error);
                }
            };
            Ok(Self {
                merge_opaque: gl.get_uniform_location(merge, "u_opaque"),
                merge_transparent: gl.get_uniform_location(merge, "u_transparent"),
                merge_filmic: gl.get_uniform_location(merge, "u_filmic"),
                bg_image: gl.get_uniform_location(background, "u_background_image"),
                bg_type: gl.get_uniform_location(background, "u_background_type"),
                bg_blur: gl.get_uniform_location(background, "u_background_blur"),
                bg_extents: gl.get_uniform_location(background, "u_background_extents"),
                bg_environment: EnvironmentUniforms::new(gl, background),
                present_texture: gl.get_uniform_location(present, "u_final_texture"),
                present_size: gl.get_uniform_location(present, "u_inverse_size"),
                present_antialias: gl.get_uniform_location(present, "u_antialias"),
                merge,
                background,
                present,
                target: None,
                environment: None,
                image: None,
                solid,
            })
        }
    }

    pub(crate) unsafe fn prepare(
        &mut self,
        gl: &glow::Context,
        size: [i32; 2],
        atlas: &Arc<EnvironmentAtlas>,
        image: Option<&Arc<ViewTexture>>,
    ) -> Result<(), String> {
        unsafe {
            if self
                .target
                .as_ref()
                .is_none_or(|target| target.size != size)
            {
                let retained_bytes = self.target.as_ref().map_or(0, |old| old.storage_bytes);
                let target = LinearTarget::new(gl, size, retained_bytes)?;
                if let Some(old) = self.target.replace(target) {
                    old.destroy(gl);
                }
            }
            if self
                .environment
                .as_ref()
                .is_none_or(|old| old.source.as_ptr() != Arc::as_ptr(atlas))
            {
                let dimensions = atlas.dimensions().map(|x| x as i32);
                let texture = upload_rgba(gl, dimensions, atlas.raw_rgba(), true, 2)?;
                if let Some(old) = self.environment.replace(GpuEnvironment {
                    texture,
                    source: Arc::downgrade(atlas),
                }) {
                    gl.delete_texture(old.texture);
                }
            }
            match image {
                Some(image)
                    if self
                        .image
                        .as_ref()
                        .is_none_or(|old| old.source.as_ptr() != Arc::as_ptr(image)) =>
                {
                    let texture = upload_rgba(
                        gl,
                        [image.width as i32, image.height as i32],
                        &image.pixels_bottom_up,
                        false,
                        3,
                    )?;
                    if let Some(old) = self.image.replace(GpuImage {
                        texture,
                        source: Arc::downgrade(image),
                    }) {
                        gl.delete_texture(old.texture);
                    }
                }
                None => {
                    if let Some(old) = self.image.take() {
                        gl.delete_texture(old.texture);
                    }
                }
                _ => {}
            }
            self.bind_environment(gl);
            Ok(())
        }
    }

    pub(crate) unsafe fn bind_environment(&self, gl: &glow::Context) {
        unsafe {
            gl.active_texture(glow::TEXTURE2);
            gl.bind_texture(
                glow::TEXTURE_2D,
                self.environment.as_ref().map(|x| x.texture),
            );
        }
    }

    /// Both attachments share one depth buffer. Clear depth only on opaque start.
    pub(crate) unsafe fn begin_opaque(&self, gl: &glow::Context) {
        unsafe {
            let target = self.target.as_ref().expect("prepared linear target");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.opaque));
            gl.viewport(0, 0, target.size[0], target.size[1]);
            gl.disable(glow::SCISSOR_TEST);
            gl.color_mask(true, true, true, true);
            gl.depth_mask(true);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear_depth_f32(1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
    }

    pub(crate) unsafe fn draw_background(
        &self,
        gl: &glow::Context,
        style: &EnvironmentStyle,
        atlas: &EnvironmentAtlas,
        view_to_world: &[f32; 9],
    ) -> Result<(), String> {
        let target = self.target.as_ref().ok_or("Linear target not prepared")?;
        let image_size = if style.custom_background {
            style
                .background_image
                .as_ref()
                .map_or([1, 1], |x| [x.width as usize, x.height as usize])
        } else {
            [1, 1]
        };
        let mode = if style.custom_background {
            style.background_mode
        } else {
            BackgroundMode::Image
        };
        let extents = background_quad_half_extents(
            target.size.map(|x| x as usize),
            image_size,
            style.background_fill,
            mode != BackgroundMode::Image,
        )?;
        unsafe {
            // Source draws its far-plane background after opaque geometry.
            // Keep the shared depth contents and fill only uncovered pixels.
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.opaque));
            gl.viewport(0, 0, target.size[0], target.size[1]);
            gl.disable(glow::SCISSOR_TEST);
            gl.color_mask(true, true, true, true);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.depth_mask(true);
            gl.depth_range_f32(0.0, 1.0);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.use_program(Some(self.background));
            self.bind_environment(gl);
            self.bg_environment.upload(gl, atlas, view_to_world);
            gl.active_texture(glow::TEXTURE3);
            gl.bind_texture(
                glow::TEXTURE_2D,
                Some(if style.custom_background {
                    self.image.as_ref().map_or(self.solid, |x| x.texture)
                } else {
                    self.solid
                }),
            );
            gl.uniform_1_i32(self.bg_image.as_ref(), 3);
            gl.uniform_1_i32(self.bg_type.as_ref(), mode as i32);
            gl.uniform_1_f32(self.bg_blur.as_ref(), style.background_blur.clamp(0.0, 1.0));
            gl.uniform_2_f32(
                self.bg_extents.as_ref(),
                extents[0] as f32,
                extents[1] as f32,
            );
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
        Ok(())
    }

    pub(crate) unsafe fn begin_transparent(&self, gl: &glow::Context) {
        unsafe {
            let target = self.target.as_ref().expect("prepared linear target");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.transparent));
            gl.disable(glow::SCISSOR_TEST);
            gl.color_mask(true, true, true, true);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            // Opaque depth remains. Transparent objects do not write depth.
            gl.depth_mask(false);
        }
    }

    pub(crate) fn transparent_framebuffer(&self) -> Option<glow::Framebuffer> {
        self.target.as_ref().map(|target| target.transparent)
    }

    /// Transfer exactly once into bounded RGBA8 merge storage before FXAA.
    pub(crate) unsafe fn merge(&self, gl: &glow::Context, filmic: bool) {
        unsafe {
            let target = self.target.as_ref().expect("prepared linear target");
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.merged));
            gl.viewport(0, 0, target.size[0], target.size[1]);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.use_program(Some(self.merge));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(target.opaque_texture));
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(target.transparent_texture));
            gl.uniform_1_i32(self.merge_opaque.as_ref(), 0);
            gl.uniform_1_i32(self.merge_transparent.as_ref(), 1);
            gl.uniform_1_i32(self.merge_filmic.as_ref(), i32::from(filmic));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }

    /// Caller supplies the destination viewport/scissor and fullscreen VAO.
    pub(crate) unsafe fn present(&self, gl: &glow::Context, antialias: bool) {
        unsafe {
            let target = self.target.as_ref().expect("prepared linear target");
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.use_program(Some(self.present));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(target.merged_texture));
            gl.uniform_1_i32(self.present_texture.as_ref(), 0);
            gl.uniform_2_f32(
                self.present_size.as_ref(),
                1.0 / target.size[0] as f32,
                1.0 / target.size[1] as f32,
            );
            gl.uniform_1_i32(self.present_antialias.as_ref(), i32::from(antialias));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }

    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"size":self.target.as_ref().map(|x|x.size),"transparent_format":self.target.as_ref().map(|x|x.format_name),"fallback_reasons":self.target.as_ref().map(|x|&x.fallback_reasons),"attachment_storage_bytes":self.target.as_ref().map(|x|x.storage_bytes),"creation_peak_attachment_bytes":self.target.as_ref().map(|x|x.creation_peak_bytes),"attachment_peak_budget_bytes":MAX_LINEAR_TARGET_BYTES,"attachment_budget_scope":"Opaque, transparent, merge and shared depth, including a retained resize target; geometry and asset textures have separate bounds", "shared_depth":self.target.is_some(),"environment_texture":self.environment.is_some(),"background_texture":self.image.is_some(),"transfer":"RGBM opaque plus premultiplied linear transparent; one final source sRGB/Hejl transfer","scope":"GPU execution/reference pixels require separate evidence"})
    }

    pub(crate) unsafe fn destroy(self, gl: &glow::Context) {
        unsafe {
            if let Some(target) = self.target {
                target.destroy(gl);
            }
            if let Some(image) = self.image {
                gl.delete_texture(image.texture);
            }
            if let Some(environment) = self.environment {
                gl.delete_texture(environment.texture);
            }
            gl.delete_texture(self.solid);
            gl.delete_program(self.merge);
            gl.delete_program(self.background);
            gl.delete_program(self.present);
        }
    }
}

struct LinearTarget {
    size: [i32; 2],
    opaque: glow::Framebuffer,
    transparent: glow::Framebuffer,
    merged: glow::Framebuffer,
    opaque_texture: glow::Texture,
    transparent_texture: glow::Texture,
    merged_texture: glow::Texture,
    depth: glow::Renderbuffer,
    format_name: &'static str,
    /// Reasons earlier attachment candidates were rejected, including a
    /// successful lower-precision fallback. A complete FBO is not sufficient
    /// evidence that WebGL can blend and linearly sample an RGBA32F target.
    fallback_reasons: Vec<String>,
    storage_bytes: usize,
    creation_peak_bytes: usize,
}

fn target_bytes(
    size: [i32; 2],
    transparent_bytes_per_pixel: usize,
    max_side: i32,
) -> Option<usize> {
    if size.into_iter().any(|x| x <= 0 || x > max_side) {
        return None;
    }
    let bytes = (size[0] as usize)
        .checked_mul(size[1] as usize)?
        .checked_mul(12 + transparent_bytes_per_pixel)?;
    (bytes <= MAX_LINEAR_TARGET_BYTES).then_some(bytes)
}

fn replacement_target_storage(
    size: [i32; 2],
    transparent_bytes_per_pixel: usize,
    max_side: i32,
    retained_bytes: usize,
) -> Option<(usize, usize)> {
    let replacement_bytes = target_bytes(size, transparent_bytes_per_pixel, max_side)?;
    let peak_bytes = retained_bytes.checked_add(replacement_bytes)?;
    (peak_bytes <= MAX_LINEAR_TARGET_BYTES).then_some((replacement_bytes, peak_bytes))
}

impl LinearTarget {
    unsafe fn new(
        gl: &glow::Context,
        size: [i32; 2],
        retained_bytes: usize,
    ) -> Result<Self, String> {
        let max_side = unsafe {
            gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE)
                .min(gl.get_parameter_i32(glow::MAX_RENDERBUFFER_SIZE))
        };
        let mut failures = Vec::new();
        for (internal, ty, bytes, name) in [
            (glow::RGBA16F, glow::HALF_FLOAT, 8, "RGBA16F"),
            (glow::RGBA32F, glow::FLOAT, 16, "RGBA32F"),
            (
                glow::RGBA8,
                glow::UNSIGNED_BYTE,
                4,
                "RGBA8 bounded fallback",
            ),
        ] {
            let Some((storage_bytes, creation_peak_bytes)) =
                replacement_target_storage(size, bytes, max_side, retained_bytes)
            else {
                failures.push(format!(
                    "{name}: attachment dimension/storage or old-plus-replacement peak exceeds 256 MiB; retained {retained_bytes} bytes, candidate {} bytes",
                    target_bytes(size, bytes, max_side).map_or_else(|| "invalid".into(), |bytes| bytes.to_string())
                ));
                continue;
            };
            #[cfg(target_arch = "wasm32")]
            if internal == glow::RGBA32F {
                let extensions = gl.supported_extensions();
                let missing: Vec<_> = ["EXT_float_blend", "OES_texture_float_linear"]
                    .into_iter()
                    .filter(|extension| !extensions.contains(*extension))
                    .collect();
                if !missing.is_empty() {
                    // Glow enables these exact WebGL extension names when
                    // available. EXT_color_buffer_float/FBO completeness
                    // alone does not permit 32-bit blending or LINEAR sampling.
                    failures.push(format!(
                        "{name}: missing WebGL blend/filter capability: {}",
                        missing.join(", ")
                    ));
                    continue;
                }
            }
            match unsafe { Self::allocate(gl, size, internal, ty, name) } {
                Ok(mut target) => {
                    target.fallback_reasons = failures;
                    target.storage_bytes = storage_bytes;
                    target.creation_peak_bytes = creation_peak_bytes;
                    return Ok(target);
                }
                Err(error) => failures.push(error),
            }
        }
        Err(format!(
            "Linear viewport allocation rejected: {}",
            failures.join("; ")
        ))
    }

    unsafe fn allocate(
        gl: &glow::Context,
        size: [i32; 2],
        internal: u32,
        ty: u32,
        format_name: &'static str,
    ) -> Result<Self, String> {
        let (
            mut opaque,
            mut transparent,
            mut merged,
            mut opaque_texture,
            mut transparent_texture,
            mut merged_texture,
            mut depth,
        ) = (None, None, None, None, None, None, None);
        let result = (|| unsafe {
            let a = gl.create_framebuffer()?;
            opaque = Some(a);
            let b = gl.create_framebuffer()?;
            transparent = Some(b);
            let merge_buffer = gl.create_framebuffer()?;
            merged = Some(merge_buffer);
            let c = gl.create_texture()?;
            opaque_texture = Some(c);
            let d = gl.create_texture()?;
            transparent_texture = Some(d);
            let merge_image = gl.create_texture()?;
            merged_texture = Some(merge_image);
            let z = gl.create_renderbuffer()?;
            depth = Some(z);
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(z));
            gl.renderbuffer_storage(
                glow::RENDERBUFFER,
                glow::DEPTH_COMPONENT24,
                size[0],
                size[1],
            );
            let _unpack = ClientUnpackState::normalize(gl);
            for (framebuffer, texture, storage, element) in [
                (a, c, glow::RGBA8, glow::UNSIGNED_BYTE),
                (b, d, internal, ty),
                (merge_buffer, merge_image, glow::RGBA8, glow::UNSIGNED_BYTE),
            ] {
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                texture_parameters(gl, false);
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    storage as i32,
                    size[0],
                    size[1],
                    0,
                    glow::RGBA,
                    element,
                    glow::PixelUnpackData::Slice(None),
                );
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(texture),
                    0,
                );
                gl.framebuffer_renderbuffer(
                    glow::FRAMEBUFFER,
                    glow::DEPTH_ATTACHMENT,
                    glow::RENDERBUFFER,
                    if framebuffer == merge_buffer {
                        None
                    } else {
                        Some(z)
                    },
                );
                let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
                if status != glow::FRAMEBUFFER_COMPLETE {
                    return Err(format!(
                        "{format_name}: incomplete framebuffer 0x{status:x}"
                    ));
                }
            }
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                return Err(format!("{format_name}: allocation GL error0x{error:x}"));
            }
            Ok(Self {
                size,
                opaque: a,
                transparent: b,
                merged: merge_buffer,
                opaque_texture: c,
                transparent_texture: d,
                merged_texture: merge_image,
                depth: z,
                format_name,
                fallback_reasons: Vec::new(),
                storage_bytes: 0, // Set by the admitted replacement constructor.
                creation_peak_bytes: 0,
            })
        })();
        if result.is_err() {
            unsafe {
                for texture in [opaque_texture, transparent_texture, merged_texture]
                    .into_iter()
                    .flatten()
                {
                    gl.delete_texture(texture);
                }
                for framebuffer in [opaque, transparent, merged].into_iter().flatten() {
                    gl.delete_framebuffer(framebuffer);
                }
                if let Some(depth) = depth {
                    gl.delete_renderbuffer(depth);
                }
                for _ in 0..8 {
                    if gl.get_error() == glow::NO_ERROR {
                        break;
                    }
                }
            }
        }
        result
    }
    unsafe fn destroy(self, gl: &glow::Context) {
        unsafe {
            gl.delete_texture(self.opaque_texture);
            gl.delete_texture(self.transparent_texture);
            gl.delete_texture(self.merged_texture);
            gl.delete_framebuffer(self.opaque);
            gl.delete_framebuffer(self.transparent);
            gl.delete_framebuffer(self.merged);
            gl.delete_renderbuffer(self.depth);
        }
    }
}

unsafe fn texture_parameters(gl: &glow::Context, repeat: bool) {
    unsafe {
        for (parameter, value) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (
                glow::TEXTURE_WRAP_S,
                if repeat {
                    glow::REPEAT
                } else {
                    glow::CLAMP_TO_EDGE
                },
            ),
            (
                glow::TEXTURE_WRAP_T,
                if repeat {
                    glow::REPEAT
                } else {
                    glow::CLAMP_TO_EDGE
                },
            ),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, value as i32);
        }
    }
}

/// A callback inherits pixel-store fields, so tightly packed client images
/// must normalize them and restore the exact incoming values on every return.
/// Native GL additionally interprets client pointers through a bound unpack PBO.
/// WebGL object getters are deliberately avoided (null bindings panic in glow).
struct ClientUnpackState<'a> {
    gl: &'a glow::Context,
    fields: [i32; 4],
    #[cfg(not(target_arch = "wasm32"))]
    pixel_unpack_buffer: Option<glow::Buffer>,
}

impl<'a> ClientUnpackState<'a> {
    unsafe fn normalize(gl: &'a glow::Context) -> Self {
        unsafe {
            let fields = [
                glow::UNPACK_ROW_LENGTH,
                glow::UNPACK_SKIP_ROWS,
                glow::UNPACK_SKIP_PIXELS,
                glow::UNPACK_ALIGNMENT,
            ]
            .map(|field| gl.get_parameter_i32(field));
            #[cfg(not(target_arch = "wasm32"))]
            let pixel_unpack_buffer = gl.get_parameter_buffer(glow::PIXEL_UNPACK_BUFFER_BINDING);
            for (field, value) in [
                glow::UNPACK_ROW_LENGTH,
                glow::UNPACK_SKIP_ROWS,
                glow::UNPACK_SKIP_PIXELS,
                glow::UNPACK_ALIGNMENT,
            ]
            .into_iter()
            .zip([0, 0, 0, 1])
            {
                gl.pixel_store_i32(field, value);
            }
            #[cfg(not(target_arch = "wasm32"))]
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
            Self {
                gl,
                fields,
                #[cfg(not(target_arch = "wasm32"))]
                pixel_unpack_buffer,
            }
        }
    }
}

impl Drop for ClientUnpackState<'_> {
    fn drop(&mut self) {
        // The guard lives only inside a current-context renderer callback.
        unsafe {
            for (field, value) in [
                glow::UNPACK_ROW_LENGTH,
                glow::UNPACK_SKIP_ROWS,
                glow::UNPACK_SKIP_PIXELS,
                glow::UNPACK_ALIGNMENT,
            ]
            .into_iter()
            .zip(self.fields)
            {
                self.gl.pixel_store_i32(field, value);
            }
            #[cfg(not(target_arch = "wasm32"))]
            self.gl
                .bind_buffer(glow::PIXEL_UNPACK_BUFFER, self.pixel_unpack_buffer);
        }
    }
}

unsafe fn upload_rgba(
    gl: &glow::Context,
    size: [i32; 2],
    pixels: &[u8],
    repeat: bool,
    unit: u32,
) -> Result<glow::Texture, String> {
    let length = size
        .into_iter()
        .try_fold(4_usize, |n, x| {
            usize::try_from(x)
                .ok()
                .filter(|x| *x > 0)
                .and_then(|x| n.checked_mul(x))
        })
        .ok_or("GPU image dimensions overflow")?;
    if length != pixels.len()
        || length > 16 * 1024 * 1024
        || unit > 3
        || size
            .into_iter()
            .any(|x| x > unsafe { gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) })
    {
        return Err("GPU image exceeds its bounded dimensions/bytes/unit".into());
    }
    unsafe {
        let texture = gl.create_texture()?;
        gl.active_texture(glow::TEXTURE0 + unit);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        texture_parameters(gl, repeat);
        let unpack = ClientUnpackState::normalize(gl);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            size[0],
            size[1],
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(pixels)),
        );
        drop(unpack); // Restore the caller's actual unpack fields and desktop PBO.
        let error = gl.get_error();
        if error != glow::NO_ERROR {
            gl.delete_texture(texture);
            return Err(format!("GPU image upload rejected GL error0x{error:x}"));
        }
        Ok(texture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_resize_uses_actual_peak_and_can_admit_lower_precision_only() {
        // Independently counted attachments: old 4000x3000 has RGBA8 opaque,
        // RGBA16F transparent, RGBA8 merge and depth24 padded to 4 bytes:
        // 12,000,000 pixels * 20 bytes = 240,000,000 retained bytes.
        // A 1200x1200 replacement needs 28,800,000 /40,320,000 /23,040,000
        // bytes in 16F / 32F / 8-bit mode. Only the last peak fits 256 MiB.
        let retained_bytes = 240_000_000;
        assert_eq!(
            replacement_target_storage([1200, 1200], 8, 4096, retained_bytes),
            None
        );
        assert_eq!(
            replacement_target_storage([1200, 1200], 16, 4096, retained_bytes),
            None
        );
        assert_eq!(
            replacement_target_storage([1200, 1200], 4, 4096, retained_bytes),
            Some((23_040_000, 263_040_000))
        );
        assert_eq!(
            replacement_target_storage([1, 1], 4, 4096, 268_435_456),
            None
        );
        assert_eq!(
            replacement_target_storage([1, 1], 4, 4096, usize::MAX),
            None
        );
        assert_eq!(replacement_target_storage([0, 1], 4, 4096, 0), None);
        assert_eq!(
            replacement_target_storage([1200, 1200], 8, 4096, 0),
            Some((28_800_000, 28_800_000))
        );
    }

    #[test]
    fn attachment_budget_counts_shared_depth_and_three_color_targets() {
        assert_eq!(target_bytes([1600, 1000], 8, 4096), Some(32_000_000));
        assert_eq!(target_bytes([1600, 1000], 16, 4096), Some(44_800_000));
        assert_eq!(target_bytes([1600, 1000], 4, 4096), Some(25_600_000));
        assert_eq!(
            target_bytes([4096, 4096], 4, 4096),
            Some(MAX_LINEAR_TARGET_BYTES)
        );
        assert_eq!(target_bytes([4096, 4096], 8, 4096), None);
        assert_eq!(target_bytes([4096, 4096], 16, 4096), None);
        for size in [[0, 2], [-1, 2], [2, i32::MAX], [5000, 1]] {
            assert_eq!(target_bytes(size, 4, 4096), None);
        }
    }
}
