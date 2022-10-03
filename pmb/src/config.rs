use crate::{
    error::{PmbError, PmbErrorExt},
    event::Combination,
    s, Tool,
};
use std::path::{Path, PathBuf};
use winit::event::{MouseButton, VirtualKeyCode as Keycode};

macro_rules! config {
    ($($field:ident : $ty:ty $default:block),* $(,)?) => {
        paste::paste! {
            mod default {
                use super::*;
                use Keycode::*;
                $(pub fn $field() -> $ty $default)*
            }

            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            pub struct Config {
                $(
                    #[serde(default = "default::" $field)]
                    pub $field: $ty,
                )*

                #[serde(skip)]
                had_error_parsing: bool,
            }

            impl Config {
                pub fn new() -> Self {
                    Self {
                        $($field: default::$field(),)*
                        had_error_parsing: false,
                    }
                }
            }
        }
    };
}

config!(
    use_mouse_for_pen: bool { true },
    stylus_may_be_inverted: bool { true },
    primary_button: MouseButton { MouseButton::Left },
    pan_button: MouseButton { MouseButton::Middle },
    pen_zoom: Keycode { LControl },
    toggle_eraser_pen: Combination { E.into() },
    brush_increase: Combination { Combination::from(RBracket).repeatable() },
    brush_decrease: Combination { Combination::from(LBracket).repeatable() },
    undo: Combination { Combination::from(LControl).repeatable() | Z },
    redo: Combination { Combination::from(LControl).repeatable() | LShift | Z },
    save: Combination { Combination::from(LControl) | S },
    new: Combination { Combination::from(LControl) | N },
    reset_view: Combination { Z.into() },
    open: Combination { Combination::from(LControl) | O },
    zoom_out: Combination { Combination::from(LControl) | NumpadSubtract },
    zoom_in: Combination { Combination::from(LControl) | NumpadAdd },
    tool_for_gesture_1: Tool { Tool::Pan },
    tool_for_gesture_2: Tool { Tool::Pan },
    tool_for_gesture_3: Tool { Tool::Pan },
    tool_for_gesture_4: Tool { Tool::Pan },

    window_start_x: Option<i32> { None },
    window_start_y: Option<i32> { None },
    window_start_width: Option<u32> { None },
    window_start_height: Option<u32> { None },
    window_maximized: bool { false },

    debug_toggle_stylus_invertability: Combination { Combination::INACTIVE },
    debug_toggle_use_mouse_for_pen: Combination { Combination::INACTIVE },
    debug_toggle_use_finger_for_pen: Combination { Combination::INACTIVE },
    debug_clear_strokes: Combination { Combination::INACTIVE },
    debug_print_strokes: Combination { Combination::INACTIVE },
    debug_dirty_all_strokes: Combination { Combination::INACTIVE },
);

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
            debug_dirty_all_strokes: Combination::from(LControl) | D,
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
                Config::default().with_error()
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

        let contents = self.to_ron_string();
        if let Err(err) = std::fs::write(path, contents) {
            PmbError::from(err).display_with(s!(CouldNotOpenConfigFile));
        }
    }

    pub fn to_ron_string(&self) -> String {
        let contents = ron::ser::to_string_pretty(
            self,
            ron::ser::PrettyConfig::new()
                .new_line(String::from("\n"))
                .indentor(String::from("  "))
                .compact_arrays(true),
        )
        .unwrap();

        format!("// this file generated automatically.\n// do not edit while pmb is running!!\n{contents}")
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
