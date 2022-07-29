use glow::HasContext;
use glutin::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        ElementState as GlutinElementState, MouseButton as GlutinMouseButton,
        PenInfo as GlutinPenInfo, Touch as GlutinTouch, TouchPhase as GlutinTouchPhase,
        VirtualKeyCode as GlutinKeycode,
    },
};
use powdermilk_biscuits::{
    event::{PenInfo, Touch, TouchPhase},
    graphics::{PixelPos, StrokePoint},
    input::{ElementState, Keycode, MouseButton},
};

pub type GlState = powdermilk_biscuits::State<GlBackend, StrokeBackend>;

#[derive(Debug, Default, Clone, Copy)]
pub struct GlBackend {}

impl powdermilk_biscuits::Backend for GlBackend {
    type Ndc = GlPos;

    fn pixel_to_ndc(&self, width: u32, height: u32, pos: PixelPos) -> Self::Ndc {
        pixel_to_ndc(width, height, pos)
    }

    fn ndc_to_pixel(&self, width: u32, height: u32, pos: Self::Ndc) -> PixelPos {
        ndc_to_pixel(width, height, pos)
    }

    fn ndc_to_stroke(&self, width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint {
        ndc_to_stroke(width, height, zoom, ndc)
    }

    fn stroke_to_ndc(&self, width: u32, height: u32, zoom: f32, point: StrokePoint) -> Self::Ndc {
        stroke_to_ndc(width, height, zoom, point)
    }
}

#[derive(Debug)]
pub struct StrokeBackend {
    pub vbo: glow::Buffer,
    pub vao: glow::VertexArray,
    pub dirty: bool,
}

impl powdermilk_biscuits::StrokeBackend for StrokeBackend {
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

pub fn glutin_to_pmb_pen_info(pen_info: GlutinPenInfo) -> PenInfo {
    PenInfo {
        barrel: pen_info.barrel,
        inverted: pen_info.inverted,
        eraser: pen_info.eraser,
    }
}

pub fn glutin_to_pmb_touch_phase(phase: GlutinTouchPhase) -> TouchPhase {
    match phase {
        GlutinTouchPhase::Started => TouchPhase::Start,
        GlutinTouchPhase::Moved => TouchPhase::Move,
        GlutinTouchPhase::Ended => TouchPhase::End,
        GlutinTouchPhase::Cancelled => TouchPhase::Cancel,
    }
}

pub fn glutin_to_pmb_touch(touch: GlutinTouch) -> Touch {
    Touch {
        force: touch.force.map(|f| f.normalized()),
        phase: glutin_to_pmb_touch_phase(touch.phase),
        location: physical_pos_to_pixel_pos(touch.location),
        pen_info: touch.pen_info.map(glutin_to_pmb_pen_info),
    }
}

pub fn glutin_to_pmb_keycode(code: GlutinKeycode) -> Keycode {
    macro_rules! codes {
        ($($code:ident),*) => {
            $(if code == GlutinKeycode::$code {
                return Keycode::$code;
            })*
        };
    }

    #[rustfmt::skip]
    codes!(
        Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Key0, A, B, C, D, E, F, G, H, I, J,
        K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Escape, F1, F2, F3, F4, F5, F6, F7, F8, F9,
        F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24, Snapshot,
        Scroll, Pause, Insert, Home, Delete, End, PageDown, PageUp, Left, Up, Right, Down, Back,
        Return, Space, Compose, Caret, Numlock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
        Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadDivide, NumpadDecimal,
        NumpadComma, NumpadEnter, NumpadEquals, NumpadMultiply, NumpadSubtract, AbntC1, AbntC2,
        Apostrophe, Apps, Asterisk, At, Ax, Backslash, Calculator, Capital, Colon, Comma, Convert,
        Equals, Grave, Kana, Kanji, LAlt, LBracket, LControl, LShift, LWin, Mail, MediaSelect,
        MediaStop, Minus, Mute, MyComputer, NavigateForward, NavigateBackward, NextTrack,
        NoConvert, OEM102, Period, PlayPause, Plus, Power, PrevTrack, RAlt, RBracket, RControl,
        RShift, RWin, Semicolon, Slash, Sleep, Stop, Sysrq, Tab, Underline, Unlabeled, VolumeDown,
        VolumeUp, Wake, WebBack, WebFavorites, WebForward, WebHome, WebRefresh, WebSearch, WebStop,
        Yen, Copy, Paste, Cut
    );

    panic!("unmatched keycode: {code:?}");
}

pub fn glutin_to_pmb_key_state(state: GlutinElementState) -> ElementState {
    match state {
        GlutinElementState::Pressed => ElementState::Pressed,
        GlutinElementState::Released => ElementState::Released,
    }
}

pub fn glutin_to_pmb_mouse_button(button: GlutinMouseButton) -> MouseButton {
    match button {
        GlutinMouseButton::Left => MouseButton::Left,
        GlutinMouseButton::Right => MouseButton::Right,
        GlutinMouseButton::Middle => MouseButton::Middle,
        GlutinMouseButton::Other(b) => MouseButton::Other(b as usize),
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
