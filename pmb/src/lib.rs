pub mod error;
pub mod event;
pub mod graphics;
pub mod migrate;
pub mod stroke;
pub mod ui;

pub extern crate bytemuck;
pub extern crate dirs;
pub extern crate egui;
pub extern crate gumdrop;
pub extern crate lyon;
pub extern crate rand;

use crate::{
    error::{PmbError, PmbErrorExt},
    event::{Keycode, MouseButton},
    graphics::{Color, ColorExt, PixelPos, StrokePoint, StrokePos},
    stroke::{Stroke, StrokeElement},
};
use event::Combination;
use lyon::lyon_tessellation::{StrokeOptions, StrokeTessellator};
use slotmap::{DefaultKey, SlotMap};
use std::path::{Path, PathBuf};

pub const TITLE_UNMODIFIED: &str = "hi! <3";
pub const TITLE_MODIFIED: &str = "hi! <3 (modified)";
pub const PMB_MAGIC: [u8; 3] = [b'P', b'M', b'B'];

pub const DEFAULT_ZOOM: f32 = 50.;
pub const MAX_ZOOM: f32 = 500.;
pub const MIN_ZOOM: f32 = 1.;

pub const DEFAULT_BRUSH: usize = 1;
pub const MAX_BRUSH: usize = 20;
pub const MIN_BRUSH: usize = 1;
pub const BRUSH_DELTA: usize = 1;

pub trait CoordinateSystem: std::fmt::Debug + Default + Clone + Copy {
    type Ndc: std::fmt::Display + Clone + Copy;

    fn pixel_to_ndc(width: u32, height: u32, pos: PixelPos) -> Self::Ndc;
    fn ndc_to_pixel(width: u32, height: u32, pos: Self::Ndc) -> PixelPos;

    fn ndc_to_stroke(width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint;
    fn stroke_to_ndc(width: u32, height: u32, zoom: f32, point: StrokePoint) -> Self::Ndc;

    fn pixel_to_stroke(width: u32, height: u32, zoom: f32, pos: PixelPos) -> StrokePoint {
        let ndc = Self::pixel_to_ndc(width, height, pos);
        Self::ndc_to_stroke(width, height, zoom, ndc)
    }

    fn stroke_to_pixel(width: u32, height: u32, zoom: f32, pos: StrokePoint) -> PixelPos {
        let ndc = Self::stroke_to_ndc(width, height, zoom, pos);
        Self::ndc_to_pixel(width, height, ndc)
    }

    fn pixel_to_pos(
        width: u32,
        height: u32,
        zoom: f32,
        origin: StrokePoint,
        pos: PixelPos,
    ) -> StrokePos {
        graphics::xform_point_to_pos(origin, Self::pixel_to_stroke(width, height, zoom, pos))
    }

    fn pos_to_pixel(
        width: u32,
        height: u32,
        zoom: f32,
        origin: StrokePoint,
        pos: StrokePos,
    ) -> PixelPos {
        Self::stroke_to_pixel(
            width,
            height,
            zoom,
            graphics::xform_pos_to_point(origin, pos),
        )
    }
}

pub trait StrokeBackend: std::fmt::Debug {
    fn make_dirty(&mut self);
    fn is_dirty(&self) -> bool;
}

#[derive(gumdrop::Options, Debug)]
pub struct Args {
    #[options(help = "Show this message")]
    help: bool,
    #[options(help = "Print the version", short = "V")]
    pub version: bool,
    #[options(help = "Config file location")]
    pub config: Option<PathBuf>,
    #[options(free, help = "File to open")]
    pub file: Option<PathBuf>,
}

#[derive(Default, PartialEq, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Tool {
    #[default]
    Pen,
    Eraser,
    Pan,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub enum Device {
    #[default]
    Mouse,
    Touch,
    Pen,
}

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
                PmbError::from(err).display_with(String::from("Couldn't read config file"));
                return Config::default().with_error();
            }
        };

        match ron::from_str(&file) {
            Ok(config) => config,
            Err(err) => {
                PmbError::from(err).display_with(String::from("Couldn't read config file"));
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
                PmbError::from(err).display_with(String::from("Couldn't read config file"));
            }
            _ => {}
        }
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

#[derive(derive_disk::Disk)]
pub struct Sketch<S: StrokeBackend> {
    #[custom_codec(to_vec, map_from_vec)]
    pub strokes: SlotMap<DefaultKey, Stroke<S>>,
    pub zoom: f32,
    pub origin: StrokePoint,
}

pub fn map_from_vec<S: StrokeBackend>(strokes: Vec<Stroke<S>>) -> SlotMap<DefaultKey, Stroke<S>> {
    strokes
        .into_iter()
        .fold(SlotMap::default(), |mut map, stroke| {
            map.insert(stroke);
            map
        })
}

impl<S: StrokeBackend> Default for Sketch<S> {
    fn default() -> Self {
        Self::new(grid())
    }
}

impl<S: StrokeBackend> Sketch<S> {
    pub fn new(strokes: Vec<Stroke<S>>) -> Self {
        Self {
            strokes: map_from_vec(strokes),
            zoom: crate::DEFAULT_ZOOM,
            origin: StrokePoint::default(),
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn with_filename<C: CoordinateSystem>(
        ui: &mut ui::Ui<C>,
        path: impl AsRef<std::path::Path>,
    ) -> Self {
        log::info!("create State from {}", path.as_ref().display());

        let mut this = Sketch::new(grid());
        ui::read_file(ui, Some(path), &mut this)
            .problem(String::from("Could not open file"))
            .display();

        this
    }

    fn to_vec(&self) -> Vec<Stroke<S>> {
        self.strokes
            .values()
            .map(|stroke| Stroke {
                points: stroke.points.clone(),
                color: stroke.color,
                brush_size: stroke.brush_size,
                erased: stroke.erased,
                ..Default::default()
            })
            .collect()
    }

    fn screen_rect<C: CoordinateSystem>(&self, width: u32, height: u32) -> (StrokePos, StrokePos) {
        let top_left = C::pixel_to_pos(width, height, self.zoom, self.origin, PixelPos::default());

        let bottom_right = C::pixel_to_pos(
            width,
            height,
            self.zoom,
            self.origin,
            PixelPos {
                x: width as f32,
                y: height as f32,
            },
        );

        (top_left, bottom_right)
    }

    pub fn update_visible_strokes<C: CoordinateSystem>(&mut self, width: u32, height: u32) {
        let (top_left, bottom_right) = self.screen_rect::<C>(width, height);
        for stroke in self.strokes.values_mut() {
            stroke.update_visible(top_left, bottom_right);
        }
    }

    fn update_stroke_primitive(&mut self) {
        for stroke in self.strokes.values_mut() {
            stroke.draw_tesselated = stroke.brush_size * self.zoom > 1.0;
        }
    }

    pub fn update_from<C: CoordinateSystem>(
        &mut self,
        width: u32,
        height: u32,
        tessellator: &mut StrokeTessellator,
        options: &StrokeOptions,
        other: Sketch<S>,
    ) {
        self.strokes = other.strokes;
        self.update_zoom::<C>(width, height, other.zoom);
        self.move_origin::<C>(
            width,
            height,
            Default::default(),
            StrokePos {
                x: other.origin.x, // kill me :)
                y: other.origin.y,
            },
        );
        self.force_update::<C>(width, height, tessellator, options);
    }

    pub fn clear_strokes(&mut self) {
        self.strokes.clear();
    }

    pub fn visible_strokes(&self) -> impl Iterator<Item = &Stroke<S>> {
        self.strokes
            .values()
            .filter(|stroke| stroke.visible && !stroke.erased)
    }

    pub fn update_zoom<C: CoordinateSystem>(&mut self, width: u32, height: u32, next_zoom: f32) {
        self.zoom = if next_zoom < crate::MIN_ZOOM {
            crate::MIN_ZOOM
        } else if next_zoom > crate::MAX_ZOOM {
            crate::MAX_ZOOM
        } else {
            next_zoom
        };

        self.update_visible_strokes::<C>(width, height);
        self.update_stroke_primitive();
    }

    pub fn move_origin<C: CoordinateSystem>(
        &mut self,
        width: u32,
        height: u32,
        prev: StrokePos,
        next: StrokePos,
    ) {
        let dx = next.x - prev.x;
        let dy = next.y - prev.y;
        self.origin.x += dx;
        self.origin.y += dy;
        self.update_visible_strokes::<C>(width, height);
    }

    pub fn force_update<C: CoordinateSystem>(
        &mut self,
        width: u32,
        height: u32,
        tessellator: &mut StrokeTessellator,
        options: &StrokeOptions,
    ) {
        self.strokes
            .values_mut()
            .flat_map(|stroke| {
                stroke.rebuild_mesh(tessellator, options);
                stroke.backend_mut()
            })
            .for_each(|backend| backend.make_dirty());
        self.update_visible_strokes::<C>(width, height);
        self.update_stroke_primitive();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StylusPosition {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy)]
pub struct StylusState {
    pub pos: StylusPosition,
    pub eraser: bool,
}

impl Default for StylusState {
    fn default() -> Self {
        StylusState {
            pos: StylusPosition::Up,
            eraser: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Stylus {
    pub state: StylusState,
    pub pressure: f32,
    pub point: StrokePoint,
    pub pos: StrokePos,
}

impl Stylus {
    pub fn down(&self) -> bool {
        matches!(self.state.pos, StylusPosition::Down)
    }

    pub fn eraser(&self) -> bool {
        self.state.eraser
    }
}

#[allow(dead_code)]
fn benchmark<S: StrokeBackend>() -> Vec<Stroke<S>> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let strokes = (-100..=100)
        .map(|x| {
            (-100..=100)
                .map(|y| {
                    Stroke::with_points(
                        (1..=100)
                            .map(|_| StrokeElement {
                                x: rng.gen::<f32>() + x as f32,
                                y: rng.gen::<f32>() + y as f32,
                                pressure: rng.gen(),
                            })
                            .collect(),
                        rng.gen(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();

    strokes
}

#[allow(dead_code)]
fn grid<S>() -> Vec<Stroke<S>>
where
    S: StrokeBackend,
{
    use std::iter::repeat;
    let mut strokes = vec![Stroke::with_points(
        graphics::circle_points(1.0, 50)
            .chunks_exact(2)
            .map(|arr| StrokeElement {
                x: arr[0],
                y: arr[1],
                pressure: 1.,
            })
            .collect(),
        Color::WHITE,
    )];

    strokes.extend(repeat(-25.0).take(50).enumerate().map(|(i, x)| {
        Stroke::with_points(
            repeat(-25.0)
                .take(50)
                .enumerate()
                .map(|(j, y)| StrokeElement {
                    x: i as f32 + x,
                    y: j as f32 + y,
                    pressure: 1.,
                })
                .collect(),
            Color::grey(0.1),
        )
    }));

    strokes.extend(repeat(-25.0).take(50).enumerate().map(|(i, y)| {
        Stroke::with_points(
            repeat(-25.0)
                .take(50)
                .enumerate()
                .map(|(j, x)| StrokeElement {
                    x: j as f32 + x,
                    y: i as f32 + y,
                    pressure: 1.,
                })
                .collect(),
            Color::grey(0.1),
        )
    }));

    strokes.push(Stroke::with_points(
        repeat(-25.0)
            .take(50)
            .enumerate()
            .map(|(i, x)| StrokeElement {
                x: i as f32 + x,
                y: 0.0,
                pressure: 1.,
            })
            .collect(),
        Color::grey(0.3),
    ));

    strokes.push(Stroke::with_points(
        repeat(-25.0)
            .take(50)
            .enumerate()
            .map(|(i, y)| StrokeElement {
                x: 0.0,
                y: i as f32 + y,
                pressure: 1.,
            })
            .collect(),
        Color::grey(0.3),
    ));

    strokes
}
