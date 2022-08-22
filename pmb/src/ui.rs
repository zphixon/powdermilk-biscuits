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
    Touch,
    Release,
    PenDown,
    PenUp,
    MovePen,
    MoveMouse,
    MoveTouch,
    MouseDown,
    MouseUp,
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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prev_device: Device::Mouse,
            use_mouse_for_pen: false,
            use_finger_for_pen: true,
            active_tool: Tool::Pen,
            stylus_may_be_inverted: true,
        }
    }
}

impl UiState {
    pub fn next(&mut self, config: &Config, event: Event) {
        use Event::*;
        use UiState::*;

        *self = match (*self, event) {
            // pen input
            (Ready, PenDown) => match config.active_tool {
                Tool::Pen => PenDraw,
                Tool::Eraser => PenErase,
            },
            (PenDraw, MovePen) => {
                // state.last_stroke.add_point()
                PenDraw
            }
            (PenDraw | PenErase, PenUp) => Ready,
            (Ready, StartPan) => Pan,
            (Pan, MoveMouse | MovePen | MoveTouch) => {
                // state.change_origin()
                Pan
            }
            (Pan, EndPan) => Ready,
            (Pan, StartZoom) => Zoom,
            (Zoom, EndZoom) => Pan,
            (Zoom, EndPan) => PreZoom,
            (Ready, StartZoom) => PreZoom,
            (PreZoom, EndZoom) => Ready,
            (PreZoom, StartPan) => Zoom,

            // mouse input
            (Ready, MouseDown) => {
                if config.use_mouse_for_pen {
                    match config.active_tool {
                        Tool::Pen => MouseDraw,
                        Tool::Eraser => MouseErase,
                    }
                } else {
                    Pan
                }
            }
            (MouseDraw | MouseErase | Pan, MouseUp) => Ready,

            // touch input
            (Ready, Touch) => {
                if config.use_finger_for_pen {
                    match config.active_tool {
                        Tool::Pen => TouchDraw,
                        Tool::Eraser => TouchErase,
                    }
                } else {
                    Gesture(1)
                }
            }
            (TouchDraw | TouchErase, Touch) => Gesture(2),
            (TouchDraw | TouchErase, Release) => Ready,
            (Gesture(i), Touch) => Gesture(i + 1),
            (Gesture(i), Release) => {
                if i == 1 {
                    Ready
                } else {
                    Gesture(i - 1)
                }
            }

            (any, _) => any,
        };
    }
}
