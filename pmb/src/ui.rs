use std::path::{Path, PathBuf};

use crate::input::{ElementState, Keycode};

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

#[derive(Default)]
pub enum Tool {
    #[default]
    Pen,
    Eraser,
}

#[derive(Default)]
pub struct Config {
    pub use_mouse_for_pen: bool,
    pub use_finger_for_pen: bool,
    pub active_tool: Tool,
}

impl UiState {
    pub fn bound_next(&mut self, config: &Config, key: Keycode, state: ElementState) {
        match (key, state) {
            _ => todo!(),
        }
    }

    pub fn next(&mut self, config: &Config, event: Event) {
        use Event::*;
        use UiState::*;

        *self = match (*self, event) {
            // pen input
            (Ready, PenDown) => match config.active_tool {
                Tool::Pen => PenDraw,
                Tool::Eraser => PenErase,
            },
            (PenDraw | PenErase, PenUp) => Ready,
            (Ready, StartPan) => Pan,
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

            (any, MoveMouse | MovePen | MoveTouch) => any,
            (this, event) => {
                println!("invalid state: {:?}, {:?}", this, event);
                this
            }
        };
    }
}
