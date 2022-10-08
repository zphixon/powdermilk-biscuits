use egui_glow::glow::{NativeBuffer, NativeProgram, NativeUniformLocation, NativeVertexArray};
use ezgl::{gl, gl::HasContext};
use powdermilk_biscuits::{
    bytemuck,
    graphics::{PixelPos, StrokePoint},
    ui::widget::SketchWidget,
    winit::dpi::PhysicalSize,
    CoordinateSystem, Sketch,
};

pub const SAMPLE_COUNT: i32 = 4;

#[derive(Debug, Default, Clone, Copy)]
pub struct GlCoords {}

impl CoordinateSystem for GlCoords {
    type Ndc = GlPos;

    fn pixel_to_ndc(width: u32, height: u32, pos: PixelPos) -> Self::Ndc {
        GlPos {
            x: (2.0 * pos.x as f32) / width as f32 - 1.0,
            y: -((2.0 * pos.y as f32) / height as f32 - 1.0),
        }
    }

    fn ndc_to_pixel(width: u32, height: u32, pos: Self::Ndc) -> PixelPos {
        PixelPos {
            x: (pos.x + 1.0) * width as f32 / 2.0,
            y: (-pos.y + 1.0) * height as f32 / 2.0,
        }
    }

    fn ndc_to_stroke(width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint {
        StrokePoint {
            x: ndc.x * width as f32 / zoom,
            y: ndc.y * height as f32 / zoom,
        }
    }

    fn stroke_to_ndc(width: u32, height: u32, zoom: f32, point: StrokePoint) -> Self::Ndc {
        GlPos {
            x: point.x * zoom / width as f32,
            y: point.y * zoom / height as f32,
        }
    }
}

#[derive(Debug)]
pub struct GlStrokeBackend {
    line_vao: gl::VertexArray,
    line_vbo: gl::Buffer,
    line_len: i32,
    mesh_vaos: Vec<gl::VertexArray>,
    mesh_vbos: Vec<gl::Buffer>,
    mesh_ebos: Vec<gl::Buffer>,
    mesh_lens: Vec<i32>,
    dirty: bool,
}

impl powdermilk_biscuits::StrokeBackend for GlStrokeBackend {
    fn make_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

pub fn view_matrix(
    zoom: f32,
    scale: f32,
    size: PhysicalSize<u32>,
    origin: StrokePoint,
) -> glam::Mat4 {
    let PhysicalSize { width, height } = size;
    let xform = GlCoords::stroke_to_ndc(width, height, zoom, origin);
    glam::Mat4::from_scale_rotation_translation(
        glam::vec3(scale / width as f32, scale / height as f32, 1.0),
        glam::Quat::IDENTITY,
        glam::vec3(xform.x, xform.y, 0.0),
    )
}

#[derive(Debug, Clone, Copy)]
pub struct GlPos {
    pub x: f32,
    pub y: f32,
}

use std::fmt::{Display, Formatter};
impl Display for GlPos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.02},{:.02}", self.x, self.y)
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn compile_shader(
    gl: &gl::Context,
    shader_type: u32,
    source: &'static str,
) -> gl::NativeShader {
    let shader = gl.create_shader(shader_type).unwrap();
    gl.shader_source(shader, source);
    gl.compile_shader(shader);

    if !gl.get_shader_compile_status(shader) {
        panic!("{}", gl.get_shader_info_log(shader));
    }

    shader
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn compile_program(
    gl: &gl::Context,
    vert_src: &'static str,
    frag_src: &'static str,
) -> gl::NativeProgram {
    let program = gl.create_program().unwrap();

    let vert = compile_shader(gl, gl::VERTEX_SHADER, vert_src);
    let frag = compile_shader(gl, gl::FRAGMENT_SHADER, frag_src);

    gl.attach_shader(program, vert);
    gl.attach_shader(program, frag);

    gl.link_program(program);

    if !gl.get_program_link_status(program) {
        panic!("{}", gl.get_program_info_log(program));
    }

    gl.detach_shader(program, vert);
    gl.detach_shader(program, frag);
    gl.delete_shader(vert);
    gl.delete_shader(frag);

    program
}

pub struct Renderer {
    msaa_fbo: gl::Framebuffer,
    line_strokes_program: NativeProgram,
    mesh_strokes_program: NativeProgram,
    pen_cursor_program: NativeProgram,
    strokes_view: NativeUniformLocation,
    strokes_color: NativeUniformLocation,
    pen_cursor_view: NativeUniformLocation,
    pen_cursor_erasing: NativeUniformLocation,
    pen_cursor_pen_down: NativeUniformLocation,
    cursor_vao: NativeVertexArray,
    cursor_buffer: NativeBuffer,
}

impl Renderer {
    pub fn new(gl: &gl::Context, width: u32, height: u32) -> Self {
        unsafe {
            gl.enable(gl::SRGB8_ALPHA8);
            gl.enable(gl::FRAMEBUFFER_SRGB);
            gl.enable(gl::MULTISAMPLE);
            gl.enable(gl::VERTEX_PROGRAM_POINT_SIZE);
            gl.enable(gl::DEBUG_OUTPUT);
            gl.disable(gl::CULL_FACE);

            let pen_cursor_program = compile_program(
                gl,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/shaders/cursor.vert"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/shaders/cursor.frag"
                )),
            );

            let pen_cursor_erasing = gl
                .get_uniform_location(pen_cursor_program, "erasing")
                .unwrap();
            let pen_cursor_pen_down = gl
                .get_uniform_location(pen_cursor_program, "penDown")
                .unwrap();
            let pen_cursor_view = gl.get_uniform_location(pen_cursor_program, "view").unwrap();

            let line_strokes_program = compile_program(
                gl,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/shaders/stroke_line.vert"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/shaders/stroke_line.frag"
                )),
            );

            let mesh_strokes_program = compile_program(
                gl,
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/shaders/stroke_line.vert"
                )),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/shaders/stroke_mesh.frag"
                )),
            );

            let strokes_view = gl
                .get_uniform_location(line_strokes_program, "view")
                .unwrap();
            let strokes_color = gl
                .get_uniform_location(line_strokes_program, "strokeColor")
                .unwrap();

            let cursor_vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(cursor_vao));
            let cursor_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(gl::ARRAY_BUFFER, Some(cursor_buffer));

            let float_size = std::mem::size_of::<f32>();
            let circle = powdermilk_biscuits::graphics::cursor_geometry(1., 50);
            let bytes =
                std::slice::from_raw_parts(circle.as_ptr() as *const u8, circle.len() * float_size);

            gl.buffer_data_u8_slice(gl::ARRAY_BUFFER, bytes, gl::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, gl::FLOAT, false, 2 * float_size as i32, 0);

            let msaa_fbo = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(gl::FRAMEBUFFER, Some(msaa_fbo));

            let tex = gl.create_texture().unwrap();
            gl.bind_texture(gl::TEXTURE_2D_MULTISAMPLE, Some(tex));
            gl.tex_image_2d_multisample(
                gl::TEXTURE_2D_MULTISAMPLE,
                4,
                gl::RGBA8 as i32,
                width as i32,
                height as i32,
                true,
            );
            gl.bind_texture(gl::TEXTURE_2D_MULTISAMPLE, None);
            gl.framebuffer_texture_2d(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D_MULTISAMPLE,
                Some(tex),
                0,
            );

            assert_eq!(
                gl.check_framebuffer_status(gl::FRAMEBUFFER),
                gl::FRAMEBUFFER_COMPLETE
            );

            gl.bind_framebuffer(gl::FRAMEBUFFER, None);

            Self {
                msaa_fbo,
                line_strokes_program,
                mesh_strokes_program,
                pen_cursor_program,
                strokes_view,
                strokes_color,
                pen_cursor_view,
                pen_cursor_erasing,
                pen_cursor_pen_down,
                cursor_vao,
                cursor_buffer,
            }
        }
    }

    pub fn resize(&self, new_size: PhysicalSize<u32>, gl: &gl::Context) {
        unsafe {
            gl.viewport(0, 0, new_size.width as i32, new_size.height as i32);
            gl.bind_framebuffer(gl::FRAMEBUFFER, Some(self.msaa_fbo));

            let tex = gl.create_texture().unwrap();
            gl.bind_texture(gl::TEXTURE_2D_MULTISAMPLE, Some(tex));
            gl.tex_image_2d_multisample(
                gl::TEXTURE_2D_MULTISAMPLE,
                4,
                gl::RGBA8 as i32,
                new_size.width as i32,
                new_size.height as i32,
                true,
            );
            gl.bind_texture(gl::TEXTURE_2D_MULTISAMPLE, None);
            gl.framebuffer_texture_2d(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D_MULTISAMPLE,
                Some(tex),
                0,
            );
        }
    }

    pub fn render(
        &self,
        gl: &gl::Context,
        sketch: &mut Sketch<GlStrokeBackend>,
        widget: &SketchWidget<GlCoords>,
        size: PhysicalSize<u32>,
        cursor_visible: bool,
    ) {
        use std::mem::size_of;

        sketch
            .strokes
            .values_mut()
            .filter(|stroke| stroke.is_dirty())
            .for_each(|stroke| {
                if let Some(backend) = stroke.backend() {
                    unsafe {
                        gl.delete_buffer(backend.line_vbo);
                        for (vao, (vbo, ebo)) in backend
                            .mesh_vaos
                            .iter()
                            .zip(backend.mesh_vbos.iter().zip(backend.mesh_ebos.iter()))
                        {
                            gl.delete_vertex_array(*vao);
                            gl.delete_buffer(*vbo);
                            gl.delete_buffer(*ebo);
                        }
                        gl.delete_vertex_array(backend.line_vao);
                    }
                }

                stroke.backend.replace(unsafe {
                    let f32_size = size_of::<f32>() as i32;

                    let line_vao = gl.create_vertex_array().unwrap();
                    gl.bind_vertex_array(Some(line_vao));

                    let line_vbo = gl.create_buffer().unwrap();
                    gl.bind_buffer(gl::ARRAY_BUFFER, Some(line_vbo));
                    gl.buffer_data_u8_slice(
                        gl::ARRAY_BUFFER,
                        bytemuck::cast_slice(&stroke.points),
                        gl::STATIC_DRAW,
                    );

                    gl.vertex_attrib_pointer_f32(0, 2, gl::FLOAT, false, f32_size * 3, 0);
                    gl.vertex_attrib_pointer_f32(
                        1,
                        1,
                        gl::FLOAT,
                        false,
                        f32_size * 3,
                        f32_size * 2,
                    );
                    gl.enable_vertex_attrib_array(0);
                    gl.enable_vertex_attrib_array(1);

                    let mut mesh_vaos = Vec::new();
                    let mut mesh_lens = Vec::new();
                    let mut mesh_vbos = Vec::new();
                    let mut mesh_ebos = Vec::new();
                    for mesh in stroke.meshes.iter() {
                        let mesh_vao = gl.create_vertex_array().unwrap();
                        gl.bind_vertex_array(Some(mesh_vao));

                        let mesh_vbo = gl.create_buffer().unwrap();
                        gl.bind_buffer(gl::ARRAY_BUFFER, Some(mesh_vbo));
                        gl.buffer_data_u8_slice(
                            gl::ARRAY_BUFFER,
                            bytemuck::cast_slice(mesh.vertices()),
                            gl::STATIC_DRAW,
                        );
                        gl.vertex_attrib_pointer_f32(0, 2, gl::FLOAT, false, f32_size * 2, 0);
                        gl.enable_vertex_attrib_array(0);
                        mesh_vbos.push(mesh_vbo);

                        let mesh_ebo = gl.create_buffer().unwrap();
                        gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, Some(mesh_ebo));
                        gl.buffer_data_u8_slice(
                            gl::ELEMENT_ARRAY_BUFFER,
                            bytemuck::cast_slice(mesh.indices()),
                            gl::STATIC_DRAW,
                        );

                        mesh_vaos.push(mesh_vao);
                        mesh_ebos.push(mesh_ebo);
                        mesh_lens.push(mesh.indices().len() as i32);
                    }

                    GlStrokeBackend {
                        line_vao,
                        line_vbo,
                        line_len: stroke.points.len() as i32,
                        mesh_vaos,
                        mesh_vbos,
                        mesh_ebos,
                        mesh_lens,
                        dirty: false,
                    }
                });
            });

        unsafe {
            gl.bind_framebuffer(gl::FRAMEBUFFER, Some(self.msaa_fbo));
            gl.clear_color(
                sketch.bg_color[0],
                sketch.bg_color[1],
                sketch.bg_color[2],
                1.,
            );
            gl.clear(gl::COLOR_BUFFER_BIT);
        }

        sketch.visible_strokes().for_each(|stroke| unsafe {
            gl.use_program(Some(self.line_strokes_program));
            let view = view_matrix(sketch.zoom, sketch.zoom, size, sketch.origin);
            gl.uniform_matrix_4_f32_slice(Some(&self.strokes_view), false, &view.to_cols_array());
            gl.uniform_3_f32(
                Some(&self.strokes_color),
                stroke.color[0],
                stroke.color[1],
                stroke.color[2],
            );

            let GlStrokeBackend {
                line_vao, line_len, ..
            } = stroke.backend().unwrap();
            gl.bind_vertex_array(Some(*line_vao));
            gl.draw_arrays(gl::LINE_STRIP, 0, *line_len);

            if stroke.draw_tesselated {
                gl.use_program(Some(self.mesh_strokes_program));
                gl.uniform_matrix_4_f32_slice(
                    Some(&self.strokes_view),
                    false,
                    &view.to_cols_array(),
                );
                gl.uniform_3_f32(
                    Some(&self.strokes_color),
                    stroke.color[0],
                    stroke.color[1],
                    stroke.color[2],
                );

                let GlStrokeBackend {
                    mesh_vaos,
                    mesh_lens,
                    ..
                } = stroke.backend().unwrap();
                for (mesh_vao, mesh_len) in mesh_vaos.iter().zip(mesh_lens.iter()) {
                    gl.bind_vertex_array(Some(*mesh_vao));
                    gl.draw_elements(gl::TRIANGLES, *mesh_len, gl::UNSIGNED_SHORT, 0);
                }
            }
        });

        if !cursor_visible {
            unsafe {
                gl.use_program(Some(self.pen_cursor_program));
                gl.bind_vertex_array(Some(self.cursor_vao));
                gl.bind_buffer(gl::ARRAY_BUFFER, Some(self.cursor_buffer));

                gl.uniform_1_f32(
                    Some(&self.pen_cursor_erasing),
                    if widget.active_tool == powdermilk_biscuits::Tool::Eraser {
                        1.0
                    } else {
                        0.0
                    },
                );
                gl.uniform_1_f32(
                    Some(&self.pen_cursor_pen_down),
                    if widget.stylus.down() { 1.0 } else { 0.0 },
                );

                let view = view_matrix(
                    sketch.zoom,
                    widget.brush_size as f32,
                    size,
                    widget.stylus.point,
                );

                gl.uniform_matrix_4_f32_slice(
                    Some(&self.pen_cursor_view),
                    false,
                    &view.to_cols_array(),
                );

                gl.draw_arrays(gl::LINES, 0, 50 * 2);
            }
        }

        unsafe {
            gl.bind_framebuffer(gl::READ_FRAMEBUFFER, Some(self.msaa_fbo));
            gl.bind_framebuffer(gl::DRAW_FRAMEBUFFER, None);
            gl.blit_framebuffer(
                0,
                0,
                size.width as i32,
                size.height as i32,
                0,
                0,
                size.width as i32,
                size.height as i32,
                gl::COLOR_BUFFER_BIT,
                gl::NEAREST,
            );
        }
    }
}
