use crate::{
    event::{Touch, TouchPhase},
    graphics::{PixelPos, StrokePos},
    input::{ElementState, InputHandler, Keycode, MouseButton},
    Backend, Stroke, StrokeBackend, StrokePoint, Stylus, StylusPosition, StylusState,
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
    TouchMove(Touch),
    Release(Touch),

    PenDown(Touch),
    PenMove(Touch),
    PenUp(Touch),

    MouseDown(MouseButton),
    MouseMove(PixelPos),
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

    pub fn update_visible_strokes(&mut self, top_left: StrokePos, bottom_right: StrokePos) {
        for stroke in self.strokes.iter_mut() {
            stroke.update_visible(top_left, bottom_right);
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
        config: &Config,
        sketch: &Sketch<S>,
        phase: TouchPhase,
    ) {
        let location = self.input.cursor_pos();
        let point = self
            .backend
            .pixel_to_stroke(self.width, self.height, sketch.zoom, location);
        let pos = crate::graphics::xform_point_to_pos(sketch.origin, point);
        let eraser = config.active_tool == Tool::Eraser;
        let pressure = if self.input.button_down(config.primary_button) {
            1.0
        } else {
            0.0
        };
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
        self.stylus.pressure = pressure;
        self.stylus.state = state;
    }

    fn update_stylus_from_touch<S: StrokeBackend>(
        &mut self,
        config: &Config,
        sketch: &Sketch<S>,
        touch: Touch,
    ) {
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
            .unwrap_or(config.active_tool == Tool::Eraser);

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

    fn update_visible_strokes<S: StrokeBackend>(&self, sketch: &mut Sketch<S>) {
        let top_left = self.backend.pixel_to_pos(
            self.width,
            self.height,
            sketch.zoom,
            sketch.origin,
            PixelPos::default(),
        );

        let bottom_right = self.backend.pixel_to_pos(
            self.width,
            self.height,
            sketch.zoom,
            sketch.origin,
            PixelPos {
                x: self.width as f32,
                y: self.height as f32,
            },
        );

        sketch.update_visible_strokes(top_left, bottom_right);
    }

    fn move_origin<S: StrokeBackend>(
        &mut self,
        sketch: &mut Sketch<S>,
        prev: StrokePos,
        next: StrokePos,
    ) {
        let dx = next.x - prev.x;
        let dy = next.y - prev.y;
        sketch.origin.x += dx;
        sketch.origin.y += dy;
        self.update_visible_strokes(sketch);
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
            (S::Ready, E::PenMove(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                S::Ready
            }

            (S::Ready, E::PenDown(touch)) => match config.active_tool {
                Tool::Pen => {
                    self.update_stylus_from_touch(config, sketch, touch);
                    self.start_stroke(sketch);
                    S::PenDraw
                }
                Tool::Eraser => S::PenErase,
            },

            (S::PenDraw, E::PenMove(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                self.continue_stroke(sketch);
                S::PenDraw
            }

            (S::PenDraw | S::PenErase, E::PenUp(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                S::Ready
            }
            (S::Ready, E::StartPan) => S::Pan,
            (S::Pan, E::PenMove(touch)) => {
                let prev = self.stylus.pos;
                self.update_stylus_from_touch(config, sketch, touch);
                self.move_origin(sketch, prev, self.stylus.pos);
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
            (S::Ready, E::MouseMove(location)) => {
                self.input.handle_mouse_move(location);

                if config.use_mouse_for_pen {
                    self.update_stylus_from_mouse(config, sketch, TouchPhase::End);
                }

                S::Ready
            }

            (S::Ready, E::MouseDown(button)) => {
                self.input
                    .handle_mouse_button(button, ElementState::Pressed);
                if config.use_mouse_for_pen {
                    self.update_stylus_from_mouse(config, sketch, TouchPhase::Start);
                    // update stylus
                    match config.active_tool {
                        Tool::Pen => S::MouseDraw,
                        Tool::Eraser => S::MouseErase,
                    }
                } else {
                    S::Pan
                }
            }

            (S::MouseDraw, E::MouseMove(location)) => {
                self.input.handle_mouse_move(location);
                self.update_stylus_from_mouse(config, sketch, TouchPhase::Move);
                self.continue_stroke(sketch);
                S::MouseDraw
            }

            (S::MouseDraw, E::MouseUp(button)) => {
                self.input
                    .handle_mouse_button(button, ElementState::Released);
                self.update_stylus_from_mouse(config, sketch, TouchPhase::End);
                S::Ready
            }

            (S::Pan, E::MouseMove(location)) => {
                let prev = self.backend.pixel_to_pos(
                    self.width,
                    self.height,
                    sketch.zoom,
                    sketch.origin,
                    self.input.cursor_pos(),
                );

                self.input.handle_mouse_move(location);

                let next = self.backend.pixel_to_pos(
                    self.width,
                    self.height,
                    sketch.zoom,
                    sketch.origin,
                    self.input.cursor_pos(),
                );

                self.move_origin(sketch, prev, next);

                S::Pan
            }

            (S::Pan, E::MouseUp(button)) => {
                self.input
                    .handle_mouse_button(button, ElementState::Released);
                S::Ready
            }

            // TODO: touch input
            (S::Ready, E::Touch(touch)) => {
                if config.use_finger_for_pen {
                    self.update_stylus_from_touch(config, sketch, touch);
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
