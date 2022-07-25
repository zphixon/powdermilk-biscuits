use crate::{
    event::{PenInfo, Touch, TouchPhase},
    graphics::{PixelPos, StrokePoint, StrokePos},
};
use glow::HasContext;
use glutin::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{PenInfo as GlutinPenInfo, Touch as GlutinTouch, TouchPhase as GlutinTouchPhase},
};

impl From<GlutinPenInfo> for PenInfo {
    fn from(pen_info: GlutinPenInfo) -> Self {
        PenInfo {
            barrel: pen_info.barrel,
            inverted: pen_info.inverted,
            eraser: pen_info.eraser,
        }
    }
}

impl From<GlutinTouchPhase> for TouchPhase {
    fn from(phase: GlutinTouchPhase) -> Self {
        match phase {
            GlutinTouchPhase::Started => TouchPhase::Start,
            GlutinTouchPhase::Moved => TouchPhase::Move,
            GlutinTouchPhase::Ended => TouchPhase::End,
            GlutinTouchPhase::Cancelled => TouchPhase::Cancel,
        }
    }
}

impl From<GlutinTouch> for Touch {
    fn from(touch: GlutinTouch) -> Self {
        Touch {
            force: touch.force.map(|f| f.normalized()),
            phase: touch.phase.into(),
            location: touch.location.into(),
            pen_info: touch.pen_info.map(|p| p.into()),
        }
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

impl From<PhysicalPosition<f64>> for PixelPos {
    fn from(pp: PhysicalPosition<f64>) -> Self {
        PixelPos {
            x: pp.x as f32,
            y: pp.y as f32,
        }
    }
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

pub fn xform_point_to_pos(gis: StrokePoint, stroke: StrokePoint) -> StrokePos {
    let x = stroke.x - gis.x;
    let y = stroke.y - gis.y;
    StrokePos { x, y }
}

use std::fmt::{Display, Formatter};
impl Display for GlPos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.02},{:.02}", self.x, self.y)
    }
}

pub unsafe fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    path: &'static str,
) -> glow::NativeShader {
    let source =
        std::fs::read_to_string(path).expect(&format!("could not read shader at path {path}"));

    let shader = gl.create_shader(shader_type).unwrap();
    gl.shader_source(shader, &source);
    gl.compile_shader(shader);

    if !gl.get_shader_compile_status(shader) {
        panic!("{}", gl.get_shader_info_log(shader));
    }

    shader
}

pub unsafe fn compile_program(
    gl: &glow::Context,
    vert_path: &'static str,
    frag_path: &'static str,
) -> glow::NativeProgram {
    let program = gl.create_program().unwrap();

    let vert = compile_shader(gl, glow::VERTEX_SHADER, vert_path);
    let frag = compile_shader(gl, glow::FRAGMENT_SHADER, frag_path);

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
