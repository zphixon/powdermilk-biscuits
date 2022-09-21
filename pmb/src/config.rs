use crate::{
    error::{PmbError, PmbErrorExt},
    event::{Combination, Keycode, MouseButton},
    s, Tool,
};
use std::path::{Path, PathBuf};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub use_mouse_for_pen: bool,
    pub stylus_may_be_inverted: bool,
    pub primary_button: MouseButton,
    pub pan_button: MouseButton,
    pub pen_zoom: Keycode,
    pub toggle_eraser_pen: Combination,
    pub brush_increase: Combination,
    pub brush_decrease: Combination,
    pub undo: Combination,
    pub redo: Combination,
    pub save: Combination,
    pub reset_view: Combination,
    pub open: Combination,
    pub zoom_out: Combination,
    pub zoom_in: Combination,
    pub tool_for_gesture_1: Tool,
    pub tool_for_gesture_2: Tool,
    pub tool_for_gesture_3: Tool,
    pub tool_for_gesture_4: Tool,

    pub window_start_x: Option<i32>,
    pub window_start_y: Option<i32>,
    pub window_start_width: Option<u32>,
    pub window_start_height: Option<u32>,
    pub window_maximized: bool,

    pub debug_toggle_stylus_invertability: Combination,
    pub debug_toggle_use_mouse_for_pen: Combination,
    pub debug_toggle_use_finger_for_pen: Combination,
    pub debug_clear_strokes: Combination,
    pub debug_print_strokes: Combination,
    pub debug_dirty_all_strokes: Combination,

    #[serde(skip)]
    pub had_error_parsing: bool,
}

impl Default for Config {
    fn default() -> Self {
        if cfg!(feature = "pmb-release") {
            Self::new()
        } else {
            Self::debug()
        }
    }
}

impl Config {
    pub fn new() -> Config {
        use Keycode::*;

        Config {
            use_mouse_for_pen: false,
            stylus_may_be_inverted: true,
            primary_button: MouseButton::Left,
            pan_button: MouseButton::Middle,
            pen_zoom: LControl,
            toggle_eraser_pen: E.into(),
            brush_increase: Combination::from(RBracket).repeatable(),
            brush_decrease: Combination::from(LBracket).repeatable(),
            undo: (LControl | Z).repeatable(),
            redo: (LControl | LShift | Z).repeatable(),
            save: LControl | S,
            reset_view: Z.into(),
            open: LControl | O,
            zoom_out: LControl | NumpadSubtract,
            zoom_in: LControl | NumpadAdd,
            tool_for_gesture_1: Tool::Pan,
            tool_for_gesture_2: Tool::Pan,
            tool_for_gesture_3: Tool::Pan,
            tool_for_gesture_4: Tool::Pan,

            window_start_x: None,
            window_start_y: None,
            window_start_width: None,
            window_start_height: None,
            window_maximized: false,

            debug_toggle_stylus_invertability: Combination::INACTIVE,
            debug_toggle_use_mouse_for_pen: Combination::INACTIVE,
            debug_toggle_use_finger_for_pen: Combination::INACTIVE,
            debug_clear_strokes: Combination::INACTIVE,
            debug_print_strokes: Combination::INACTIVE,
            debug_dirty_all_strokes: Combination::INACTIVE,

            had_error_parsing: false,
        }
    }

    fn with_error(self) -> Config {
        Config {
            had_error_parsing: true,
            ..self
        }
    }

    pub fn debug() -> Config {
        use Keycode::*;
        Config {
            debug_toggle_stylus_invertability: I.into(),
            debug_toggle_use_mouse_for_pen: M.into(),
            debug_toggle_use_finger_for_pen: F.into(),
            debug_clear_strokes: C.into(),
            debug_print_strokes: D.into(),
            debug_dirty_all_strokes: LControl | D,
            ..Config::new()
        }
    }

    pub fn config_path() -> Result<PathBuf, PmbError> {
        let mut path = dirs::config_dir().unwrap();
        path.push("powdermilk-biscuits");

        if !path.exists() {
            std::fs::create_dir(&path)?;
        }

        path.push("config.ron");
        Ok(path)
    }

    // TODO registry/gsettings or something, this is dumb
    pub fn from_disk(path: &Path) -> Config {
        log::info!("load config from {}", path.display());
        let file = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Config::default();
            }
            Err(err) => {
                PmbError::from(err).display_with(s!(CouldNotOpenConfigFile));
                return Config::default().with_error();
            }
        };

        match ron::from_str(&file) {
            Ok(config) => config,
            Err(err) => {
                PmbError::from(err).display_with(s!(CouldNotOpenConfigFile));
                return Config::default().with_error();
            }
        }
    }

    pub fn save(&self, path: &Path) {
        log::info!("save config to {}", path.display());

        if self.had_error_parsing {
            // don't overwrite broken configs
            log::error!("had error");
            return;
        }

        let contents = ron::ser::to_string_pretty(
            self,
            ron::ser::PrettyConfig::new()
                .new_line(String::from("\n"))
                .indentor(String::from("  "))
                .compact_arrays(true),
        )
        .unwrap();

        let contents =
            format!("// this file generated automatically.\n// do not edit while pmb is running!!\n{contents}");

        match std::fs::write(path, contents) {
            Err(err) => {
                PmbError::from(err).display_with(s!(CouldNotOpenConfigFile));
            }
            _ => {}
        }
    }

    pub fn start_pos(&self) -> (Option<i32>, Option<i32>) {
        (self.window_start_x, self.window_start_y)
    }

    pub fn start_size(&self) -> (Option<u32>, Option<u32>) {
        (self.window_start_width, self.window_start_height)
    }

    pub fn tool_for_gesture(&self, i: u8) -> Tool {
        match i {
            1 => self.tool_for_gesture_1,
            2 => self.tool_for_gesture_2,
            3 => self.tool_for_gesture_3,
            4 => self.tool_for_gesture_4,
            _ => Tool::Pan,
        }
    }

    pub fn resize_window(&mut self, width: u32, height: u32) {
        self.window_start_width.replace(width);
        self.window_start_height.replace(height);
    }

    pub fn move_window(&mut self, x: i32, y: i32) {
        self.window_start_x.replace(x);
        self.window_start_y.replace(y);
    }
}
