//! Fixed, context-owned post-presentation cursor resources. No hidden renders.
use crate::background_cursor::{COLOR, CursorFrame, CursorMatrices, circle_vertices, dot_vertices};
use eframe::glow;
use glow::HasContext;

pub(crate) const VERTEX_SHADER: &str = r#"
in vec3 a_position;
uniform mat4 uMVP;
void main() {
    gl_Position = uMVP * vec4(a_position, 1.0);
}
"#;
pub(crate) const FRAGMENT_SHADER: &str = r#"
uniform vec3 uColor;
out vec4 cursor_frag;
void main() {
    cursor_frag = vec4(uColor, 1.0);
}
"#;

pub(crate) struct BackgroundCursorGl {
    context: usize,
    program: glow::Program,
    vao: glow::VertexArray,
    circle: glow::Buffer,
    dot: glow::Buffer,
    mvp: glow::UniformLocation,
    color: glow::UniformLocation,
}
impl BackgroundCursorGl {
    /// Caller already captured actual callback state and restores it on all exits.
    pub(crate) unsafe fn new(gl: &glow::Context, header: &str) -> Result<Self, String> {
        unsafe {
            let program = crate::renderer::compile_program(
                gl,
                header,
                VERTEX_SHADER,
                FRAGMENT_SHADER,
                "Source background cursor",
            )?;
            let Some(mvp) = gl.get_uniform_location(program, "uMVP") else {
                gl.delete_program(program);
                return Err("Cursor MVP uniform unavailable".into());
            };
            let Some(color) = gl.get_uniform_location(program, "uColor") else {
                gl.delete_program(program);
                return Err("Cursor color uniform unavailable".into());
            };
            let vao = match gl.create_vertex_array() {
                Ok(v) => v,
                Err(e) => {
                    gl.delete_program(program);
                    return Err(format!("Cursor vertex array: {e}"));
                }
            };
            let circle = match gl.create_buffer() {
                Ok(v) => v,
                Err(e) => {
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return Err(format!("Cursor circle buffer: {e}"));
                }
            };
            let dot = match gl.create_buffer() {
                Ok(v) => v,
                Err(e) => {
                    gl.delete_buffer(circle);
                    gl.delete_vertex_array(vao);
                    gl.delete_program(program);
                    return Err(format!("Cursor dot buffer: {e}"));
                }
            };
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(circle));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&circle_vertices()),
                glow::STATIC_DRAW,
            );
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 12, 0);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(dot));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&dot_vertices()),
                glow::STATIC_DRAW,
            );
            Ok(Self {
                context: gl as *const glow::Context as usize,
                program,
                vao,
                circle,
                dot,
                mvp,
                color,
            })
        }
    }

    pub(crate) unsafe fn draw(
        &self,
        gl: &glow::Context,
        frame: CursorFrame,
        matrices: CursorMatrices,
    ) -> Result<u32, String> {
        if self.context != gl as *const glow::Context as usize {
            return Err("Cursor belongs to a different GL context".into());
        }
        // Actual destination, measured viewport and callback clip are supplied
        // by Renderer, after real presentation and before GlState::restore.
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.depth_mask(true);
            gl.depth_range_f32(0.0, 1.0);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::POLYGON_OFFSET_FILL);
            gl.color_mask(true, true, true, true);
            gl.line_width(1.0);
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.uniform_3_f32_slice(Some(&self.color), &COLOR);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.circle));
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 12, 0);
            gl.uniform_matrix_4_f32_slice(Some(&self.mvp), false, &matrices.circle);
            gl.draw_arrays(glow::LINE_LOOP, 0, 50);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.dot));
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 12, 0);
            gl.uniform_matrix_4_f32_slice(Some(&self.mvp), false, &matrices.dot);
            gl.draw_arrays(glow::TRIANGLE_FAN, 0, 12);
            if frame.symmetry {
                gl.uniform_matrix_4_f32_slice(Some(&self.mvp), false, &matrices.symmetry);
                gl.draw_arrays(glow::TRIANGLE_FAN, 0, 12);
            }
        }
        Ok(if frame.symmetry { 3 } else { 2 })
    }

    pub(crate) unsafe fn destroy(self, gl: &glow::Context) -> Result<(), String> {
        if self.context != gl as *const glow::Context as usize {
            // Never delete another context's names. The originating context
            // owns any unavailable resource lifetime; no successful-drop claim.
            return Err("Cursor destruction requires its originating GL context".into());
        }
        unsafe {
            gl.delete_buffer(self.circle);
            gl.delete_buffer(self.dot);
            gl.delete_vertex_array(self.vao);
            gl.delete_program(self.program);
        }
        Ok(())
    }
}
