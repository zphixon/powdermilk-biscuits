use crate::{
    event::Touch,
    graphics::PixelPos,
    input::{ElementState, InputHandler, Keycode, MouseButton},
    Backend, Stroke, StrokeBackend, StrokePoint, Stylus,
};
use std::path::{Path, PathBuf};

pub fn error(text: &str) -> rfd::MessageDialogResult {
    rfd::MessageDialog::new()
        .set_title("Error")
        .set_description(text)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show()
}

pub fn ask_to_save(why: &str) -> rfd::MessageDialogResult {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Unsaved changes")
        .set_description(why)
        .set_buttons(rfd::MessageButtons::YesNoCancel)
        .show()
}

pub fn save_dialog(title: &str, filename: Option<&Path>) -> Option<PathBuf> {
    let filename = filename
        .and_then(|path| path.file_name())
        .and_then(|os| os.to_str())
        .unwrap_or("");

    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("PMB", &["pmb"])
        .set_file_name(filename)
        .save_file()
}

pub fn open_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Open file")
        .add_filter("PMB", &["pmb"])
        .pick_file()
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Touch(Touch),
    Release(Touch),
    PenDown(Touch),
    PenUp(Touch),
    MovePen(Touch),
    MoveMouse(PixelPos),
    MoveTouch(Touch),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    StartPan,
    EndPan,
    StartZoom,
    EndZoom,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum UiState {
    #[default]
    Ready,
    Pan,
    PreZoom,
    Zoom,
    Select,
    PenDraw,
    PenErase,
    MouseDraw,
    MouseErase,
    TouchDraw,
    TouchErase,
    Gesture(u8),
    OpenDialog,
    SaveDialog,
}

#[derive(Default, PartialEq, Debug)]
pub enum Tool {
    #[default]
    Pen,
    Eraser,
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum Device {
    #[default]
    Mouse,
    Touch,
    Pen,
}

pub struct Config {
    pub prev_device: Device,
    pub use_mouse_for_pen: bool,
    pub use_finger_for_pen: bool,
    pub stylus_may_be_inverted: bool,
    pub active_tool: Tool,
    pub primary_button: MouseButton,
    pub pan_button: MouseButton,
    pub pen_zoom_key: Keycode,
    pub use_mouse_for_pen_key: Keycode,
    pub use_finger_for_pen_key: Keycode,
    pub swap_eraser_key: Keycode,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prev_device: Device::Mouse,
            use_mouse_for_pen: false,
            use_finger_for_pen: true,
            active_tool: Tool::Pen,
            stylus_may_be_inverted: true,
            primary_button: MouseButton::Left,
            pan_button: MouseButton::Middle,
            pen_zoom_key: Keycode::LControl,
            use_mouse_for_pen_key: Keycode::M,
            use_finger_for_pen_key: Keycode::F,
            swap_eraser_key: Keycode::E,
        }
    }
}

pub struct Sketch<S: StrokeBackend> {
    pub strokes: Vec<Stroke<S>>,
    pub zoom: f32,
    pub origin: StrokePoint,
}

impl<S: StrokeBackend> Default for Sketch<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: StrokeBackend> Sketch<S> {
    pub fn new() -> Self {
        Self {
            strokes: Vec::new(),
            zoom: crate::DEFAULT_ZOOM,
            origin: StrokePoint::default(),
        }
    }
}

#[derive(Debug)]
pub struct Ui<B: Backend> {
    pub state: UiState,
    pub stylus: Stylus,
    pub brush_size: usize,
    pub modified: bool,
    pub path: Option<std::path::PathBuf>,
    pub input: crate::input::InputHandler,
    pub width: u32,
    pub height: u32,
    pub backend: B,
}

impl<B: Backend> Ui<B> {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            state: UiState::default(),
            stylus: Stylus::default(),
            brush_size: crate::DEFAULT_BRUSH,
            modified: false,
            path: None,
            input: InputHandler::default(),
            width,
            height,
            backend: B::default(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn start_stroke<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
        if false {
            let stroke_brush_size = self
                .backend
                .pixel_to_stroke(
                    self.width,
                    self.height,
                    sketch.zoom,
                    PixelPos {
                        x: ((self.width / 2) + self.brush_size as u32) as f32,
                        y: (self.height / 2) as f32,
                    },
                )
                .x
                / 2.0;

            sketch
                .strokes
                .push(Stroke::new(rand::random(), stroke_brush_size));
        }
    }

    fn continue_stroke<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
        if false {
            let stroke = sketch.strokes.last_mut().unwrap();
            stroke.add_point(&self.stylus);
        }
    }

    fn update_stylus_from_mouse<S: StrokeBackend>(
        &mut self,
        sketch: &Sketch<S>,
        location: PixelPos,
    ) {
    }

    fn update_stylus_from_touch<S: StrokeBackend>(&mut self, sketch: &Sketch<S>, touch: Touch) {
        use crate::event::TouchPhase;
        use crate::{StylusPosition, StylusState};

        let Touch {
            force,
            phase,
            location,
            pen_info,
            ..
        } = touch;

        let point = self
            .backend
            .pixel_to_stroke(self.width, self.height, sketch.zoom, location);
        let pos = crate::graphics::xform_point_to_pos(sketch.origin, point);
        let pressure = force.unwrap_or(1.0);

        let eraser = pen_info
            .map(|info| info.inverted || info.eraser)
            .unwrap_or(self.stylus.state.eraser);

        let state = match phase {
            TouchPhase::Start => StylusState {
                pos: StylusPosition::Down,
                eraser,
            },

            TouchPhase::Move => {
                self.stylus.state.eraser = eraser;
                self.stylus.state
            }

            TouchPhase::End | TouchPhase::Cancel => StylusState {
                pos: StylusPosition::Up,
                eraser,
            },
        };

        self.stylus.point = point;
        self.stylus.pos = pos;
        self.stylus.pressure = pressure as f32;
        self.stylus.state = state;
    }

    pub fn next<S: StrokeBackend>(
        &mut self,
        config: &Config,
        sketch: &mut Sketch<S>,
        event: Event,
    ) {
        use Event as E;
        use UiState as S;

        self.state = match (self.state, event) {
            // pen input
            (S::Ready, E::MovePen(touch)) => {
                self.update_stylus_from_touch(sketch, touch);
                S::Ready
            }

            (S::Ready, E::PenDown(touch)) => match config.active_tool {
                Tool::Pen => {
                    self.update_stylus_from_touch(sketch, touch);
                    self.start_stroke(sketch);
                    S::PenDraw
                }
                Tool::Eraser => S::PenErase,
            },

            (S::PenDraw, E::MovePen(touch)) => {
                self.update_stylus_from_touch(sketch, touch);
                self.continue_stroke(sketch);
                S::PenDraw
            }

            (S::PenDraw | S::PenErase, E::PenUp(touch)) => {
                self.update_stylus_from_touch(sketch, touch);
                S::Ready
            }
            (S::Ready, E::StartPan) => S::Pan,
            (
                S::Pan,
                E::MoveMouse(location)
                | E::MovePen(Touch { location, .. })
                | E::MoveTouch(Touch { location, .. }),
            ) => {
                // state.change_origin()
                S::Pan
            }
            (S::Pan, E::EndPan) => S::Ready,
            (S::Pan, E::StartZoom) => S::Zoom,
            (S::Zoom, E::EndZoom) => S::Pan,
            (S::Zoom, E::EndPan) => S::PreZoom,
            (S::Ready, E::StartZoom) => S::PreZoom,
            (S::PreZoom, E::EndZoom) => S::Ready,
            (S::PreZoom, E::StartPan) => S::Zoom,

            // mouse input
            (S::Ready, E::MouseDown(button)) => {
                self.input
                    .handle_mouse_button(button, ElementState::Pressed);
                if config.use_mouse_for_pen {
                    // update stylus
                    match config.active_tool {
                        Tool::Pen => S::MouseDraw,
                        Tool::Eraser => S::MouseErase,
                    }
                } else {
                    S::Pan
                }
            }
            (S::MouseDraw | S::MouseErase | S::Pan, E::MouseUp(_)) => S::Ready,

            // touch input
            (S::Ready, E::Touch(touch)) => {
                if config.use_finger_for_pen {
                    self.update_stylus_from_touch(sketch, touch);
                    match config.active_tool {
                        Tool::Pen => S::TouchDraw,
                        Tool::Eraser => S::TouchErase,
                    }
                } else {
                    S::Gesture(1)
                }
            }
            (S::TouchDraw | S::TouchErase, E::Touch(_)) => S::Gesture(2),
            (S::TouchDraw | S::TouchErase, E::Release(_)) => S::Ready,
            (S::Gesture(i), E::Touch(_)) => S::Gesture(i + 1),
            (S::Gesture(i), E::Release(_)) => {
                if i == 1 {
                    S::Ready
                } else {
                    S::Gesture(i - 1)
                }
            }

            (any, _) => any,
        };
    }
}
