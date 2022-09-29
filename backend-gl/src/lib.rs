use egui_glow::glow::{NativeBuffer, NativeProgram, NativeUniformLocation, NativeVertexArray};
use ezgl::{gl, gl::HasContext};
use powdermilk_biscuits::{
    bytemuck,
    graphics::{PixelPos, StrokePoint},
    ui::widget::SketchWidget,
    winit::dpi::{PhysicalPosition, PhysicalSize},
    Sketch,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct GlCoords {}

impl powdermilk_biscuits::CoordinateSystem for GlCoords {
    type Ndc = GlPos;

    fn pixel_to_ndc(width: u32, height: u32, pos: PixelPos) -> Self::Ndc {
        pixel_to_ndc(width, height, pos)
    }

    fn ndc_to_pixel(width: u32, height: u32, pos: Self::Ndc) -> PixelPos {
        ndc_to_pixel(width, height, pos)
    }

    fn ndc_to_stroke(width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint {
        ndc_to_stroke(width, height, zoom, ndc)
    }

    fn stroke_to_ndc(width: u32, height: u32, zoom: f32, point: StrokePoint) -> Self::Ndc {
        stroke_to_ndc(width, height, zoom, point)
    }
}

#[derive(Debug)]
pub struct GlStrokeBackend {
    pub line_vao: gl::VertexArray,
    pub line_len: i32,
    pub mesh_vao: gl::VertexArray,
    pub mesh_len: i32,
    pub dirty: bool,
}

impl powdermilk_biscuits::StrokeBackend for GlStrokeBackend {
    fn make_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

pub fn physical_pos_to_pixel_pos(pos: PhysicalPosition<f64>) -> PixelPos {
    PixelPos {
        x: pos.x as f32,
        y: pos.y as f32,
    }
}

pub fn view_matrix(
    zoom: f32,
    scale: f32,
    size: PhysicalSize<u32>,
    origin: StrokePoint,
) -> glam::Mat4 {
    let PhysicalSize { width, height } = size;
    let xform = stroke_to_ndc(width, height, zoom, origin);
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

pub fn pixel_to_ndc(width: u32, height: u32, pos: PixelPos) -> GlPos {
    GlPos {
        x: (2.0 * pos.x as f32) / width as f32 - 1.0,
        y: -((2.0 * pos.y as f32) / height as f32 - 1.0),
    }
}

pub fn ndc_to_pixel(width: u32, height: u32, pos: GlPos) -> PixelPos {
    PixelPos {
        x: (pos.x + 1.0) * width as f32 / 2.0,
        y: (-pos.y + 1.0) * height as f32 / 2.0,
    }
}

pub fn ndc_to_stroke(width: u32, height: u32, zoom: f32, gl: GlPos) -> StrokePoint {
    StrokePoint {
        x: gl.x * width as f32 / zoom,
        y: gl.y * height as f32 / zoom,
    }
}

pub fn stroke_to_ndc(width: u32, height: u32, zoom: f32, point: StrokePoint) -> GlPos {
    GlPos {
        x: point.x * zoom / width as f32,
        y: point.y * zoom / height as f32,
    }
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
    pub fn new(gl: &gl::Context) -> Self {
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
                &gl,
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
                &gl,
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

            Self {
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
                log::debug!("replace stroke with {} points", stroke.points.len());
                stroke.backend.replace(unsafe {
                    let f32_size = size_of::<f32>() as i32;

                    let line_vao = gl.create_vertex_array().unwrap();
                    gl.bind_vertex_array(Some(line_vao));

                    let points = gl.create_buffer().unwrap();
                    gl.bind_buffer(gl::ARRAY_BUFFER, Some(points));
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

                    let mesh_vao = gl.create_vertex_array().unwrap();
                    gl.bind_vertex_array(Some(mesh_vao));
                    let mesh = gl.create_buffer().unwrap();
                    gl.bind_buffer(gl::ARRAY_BUFFER, Some(mesh));
                    gl.buffer_data_u8_slice(
                        gl::ARRAY_BUFFER,
                        bytemuck::cast_slice(&stroke.mesh.vertices),
                        gl::STATIC_DRAW,
                    );
                    gl.vertex_attrib_pointer_f32(0, 2, gl::FLOAT, false, f32_size * 2, 0);
                    gl.enable_vertex_attrib_array(0);

                    let mesh_ebo = gl.create_buffer().unwrap();
                    gl.bind_buffer(gl::ELEMENT_ARRAY_BUFFER, Some(mesh_ebo));
                    gl.buffer_data_u8_slice(
                        gl::ELEMENT_ARRAY_BUFFER,
                        bytemuck::cast_slice(&stroke.mesh.indices),
                        gl::STATIC_DRAW,
                    );

                    GlStrokeBackend {
                        line_vao,
                        line_len: stroke.points.len() as i32,
                        mesh_vao,
                        mesh_len: stroke.mesh.indices.len() as i32,
                        dirty: false,
                    }
                });
            });

        unsafe {
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
                    mesh_vao, mesh_len, ..
                } = stroke.backend().unwrap();
                gl.bind_vertex_array(Some(*mesh_vao));
                gl.draw_elements(gl::TRIANGLES, *mesh_len, gl::UNSIGNED_SHORT, 0);
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
    }
}
