//! Selected-mesh mask and source-style Sobel contour for the normal GL/WebGL2
//! viewport. View-only: no mesh, file or history operation is performed here.
use eframe::glow;
use glow::HasContext;

use crate::renderer::compile_program;

const MAX_MASK_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) struct ContourMesh {
    pub vao: glow::VertexArray,
    pub triangles: glow::Buffer,
    pub count: i32,
    pub mvp: [f32; 16],
}

struct MaskTarget {
    framebuffer: glow::Framebuffer,
    texture: glow::Texture,
    size: [i32; 2],
}

impl MaskTarget {
    unsafe fn new(gl: &glow::Context, size: [i32; 2]) -> Result<Self, String> {
        if size[0] <= 0
            || size[1] <= 0
            || size[0] > unsafe { gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) }
            || size[1] > unsafe { gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE) }
            || (size[0] as u64)
                .checked_mul(size[1] as u64)
                .and_then(|pixels| pixels.checked_mul(4))
                .is_none_or(|bytes| bytes > MAX_MASK_BYTES)
        {
            return Err("Contour mask exceeds its bounded render target".into());
        }
        let framebuffer = unsafe { gl.create_framebuffer()? };
        let texture = match unsafe { gl.create_texture() } {
            Ok(texture) => texture,
            Err(error) => {
                unsafe { gl.delete_framebuffer(framebuffer) };
                return Err(error);
            }
        };
        let result = (|| unsafe {
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            for (name, value) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, name, value as i32);
            }
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                size[0],
                size[1],
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
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
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!(
                    "Contour mask framebuffer incomplete (0x{status:x})"
                ));
            }
            Ok(Self {
                framebuffer,
                texture,
                size,
            })
        })();
        if result.is_err() {
            unsafe {
                gl.delete_texture(texture);
                gl.delete_framebuffer(framebuffer);
            }
        }
        result
    }

    unsafe fn destroy(self, gl: &glow::Context) {
        unsafe {
            gl.delete_texture(self.texture);
            gl.delete_framebuffer(self.framebuffer);
        }
    }
}

pub(crate) struct ContourPipeline {
    mask_program: glow::Program,
    mask_mvp: Option<glow::UniformLocation>,
    edge_program: glow::Program,
    edge_texture: Option<glow::UniformLocation>,
    edge_inverse_size: Option<glow::UniformLocation>,
    edge_color: Option<glow::UniformLocation>,
    edge_linear: Option<glow::UniformLocation>,
    target: Option<MaskTarget>,
}

impl ContourPipeline {
    pub(crate) unsafe fn new(gl: &glow::Context, header: &str) -> Result<Self, String> {
        let mask_program = unsafe {
            compile_program(
                gl,
                header,
                MASK_VERTEX,
                MASK_FRAGMENT,
                "Selected contour mask",
            )?
        };
        let edge_program = match unsafe {
            compile_program(
                gl,
                header,
                EDGE_VERTEX,
                EDGE_FRAGMENT,
                "Selected contour edge",
            )
        } {
            Ok(program) => program,
            Err(error) => {
                unsafe { gl.delete_program(mask_program) };
                return Err(error);
            }
        };
        Ok(Self {
            mask_mvp: unsafe { gl.get_uniform_location(mask_program, "u_mvp") },
            edge_texture: unsafe { gl.get_uniform_location(edge_program, "u_texture") },
            edge_inverse_size: unsafe { gl.get_uniform_location(edge_program, "u_inv_size") },
            edge_color: unsafe { gl.get_uniform_location(edge_program, "u_color") },
            edge_linear: unsafe { gl.get_uniform_location(edge_program, "u_linear") },
            mask_program,
            edge_program,
            target: None,
        })
    }

    pub(crate) unsafe fn draw_mask(
        &mut self,
        gl: &glow::Context,
        size: [i32; 2],
        meshes: &[ContourMesh],
    ) -> Result<(), String> {
        if self
            .target
            .as_ref()
            .is_none_or(|target| target.size != size)
        {
            if let Some(target) = self.target.take() {
                unsafe { target.destroy(gl) };
            }
            self.target = Some(unsafe { MaskTarget::new(gl, size)? });
        }
        let target = self.target.as_ref().expect("contour target prepared");
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.framebuffer));
            gl.viewport(0, 0, size[0], size[1]);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::BLEND);
            gl.color_mask(true, true, true, true);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(self.mask_program));
            for mesh in meshes {
                gl.uniform_matrix_4_f32_slice(self.mask_mvp.as_ref(), false, &mesh.mvp);
                gl.bind_vertex_array(Some(mesh.vao));
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(mesh.triangles));
                gl.draw_elements(glow::TRIANGLES, mesh.count, glow::UNSIGNED_INT, 0);
            }
        }
        Ok(())
    }

    pub(crate) unsafe fn discard_target(&mut self, gl: &glow::Context) {
        if let Some(target) = self.target.take() {
            unsafe { target.destroy(gl) };
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn composite(
        &self,
        gl: &glow::Context,
        destination: Option<glow::Framebuffer>,
        viewport: [i32; 4],
        clip: [i32; 4],
        empty_vao: glow::VertexArray,
        color: [f32; 4],
        linear: bool,
    ) {
        let Some(target) = self.target.as_ref() else {
            return;
        };
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, destination);
            gl.viewport(viewport[0], viewport[1], viewport[2], viewport[3]);
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(clip[0], clip[1], clip[2], clip[3]);
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::POLYGON_OFFSET_FILL);
            gl.enable(glow::BLEND);
            gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
            gl.blend_func_separate(
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
            gl.use_program(Some(self.edge_program));
            gl.bind_vertex_array(Some(empty_vao));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(target.texture));
            gl.uniform_1_i32(self.edge_texture.as_ref(), 0);
            gl.uniform_2_f32(
                self.edge_inverse_size.as_ref(),
                1.0 / target.size[0] as f32,
                1.0 / target.size[1] as f32,
            );
            gl.uniform_4_f32(
                self.edge_color.as_ref(),
                color[0],
                color[1],
                color[2],
                color[3],
            );
            gl.uniform_1_i32(self.edge_linear.as_ref(), i32::from(linear));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }

    pub(crate) unsafe fn destroy(mut self, gl: &glow::Context) {
        unsafe {
            self.discard_target(gl);
            gl.delete_program(self.mask_program);
            gl.delete_program(self.edge_program);
        }
    }
}

const MASK_VERTEX: &str = r#"
in vec3 a_position;
uniform mat4 u_mvp;
void main() { gl_Position = u_mvp * vec4(a_position, 1.0); }
"#;

const MASK_FRAGMENT: &str = r#"
out vec4 out_color;
// SculptGL RenderData defaults each ordinary mesh's flat color to red;
// ShaderFlat draws that RGB with alpha one into the contour mask.
void main() { out_color = vec4(1.0, 0.0, 0.0, 1.0); }
"#;

const EDGE_VERTEX: &str = r#"
out vec2 v_uv;
void main() {
    vec2 position = vec2((gl_VertexID == 1) ? 3.0 : -1.0,
                         (gl_VertexID == 2) ? 3.0 : -1.0);
    v_uv = position * 0.5 + 0.5;
    gl_Position = vec4(position, 0.0, 1.0);
}
"#;

// Same 8-neighbor weighted Sobel expression and 1.5 threshold as the pinned
// SculptGL outline.glsl. Source scenes composite into the linear transparent
// target before merge/FXAA; legacy studio scenes composite in display space.
const EDGE_FRAGMENT: &str = r#"
in vec2 v_uv;
uniform sampler2D u_texture;
uniform vec2 u_inv_size;
uniform vec4 u_color;
uniform bool u_linear;
out vec4 out_color;
void main() {
    float ox = u_inv_size.x;
    float oy = u_inv_size.y;
    vec4 t0 = texture(u_texture, v_uv + vec2( ox,  oy));
    vec4 t1 = texture(u_texture, v_uv + vec2( ox, 0.0));
    vec4 t2 = texture(u_texture, v_uv + vec2( ox, -oy));
    vec4 t3 = texture(u_texture, v_uv + vec2(0.0, -oy));
    vec4 t4 = texture(u_texture, v_uv + vec2(-ox, -oy));
    vec4 t5 = texture(u_texture, v_uv + vec2(-ox, 0.0));
    vec4 t6 = texture(u_texture, v_uv + vec2(-ox,  oy));
    vec4 t7 = texture(u_texture, v_uv + vec2(0.0,  oy));
    vec4 row_x = -2.0*t5 + 2.0*t1 - t6 + t0 - t4 + t2;
    vec4 row_y = -2.0*t3 + 2.0*t7 - t4 + t6 - t2 + t0;
    float magnitude = dot(row_y, row_y) + dot(row_x, row_x);
    if (magnitude < 1.5) discard;
    float alpha = clamp(u_color.a, 0.0, 1.0);
    vec3 rgb = clamp(u_color.rgb, 0.0, 1.0);
    if (u_linear) {
        rgb = mix(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3(2.4)), step(vec3(0.04045), rgb));
    }
    out_color = vec4(rgb * alpha, alpha);
}
"#;
