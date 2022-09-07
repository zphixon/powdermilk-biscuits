pub mod error;
pub mod event;
pub mod graphics;
pub mod migrate;
pub mod stroke;
pub mod ui;

pub extern crate rand;

use crate::{
    error::{ErrorKind, PmbError, PmbErrorExt},
    event::{Keycode, MouseButton},
    graphics::{Color, ColorExt, PixelPos, StrokePoint, StrokePos},
    migrate::Version,
    stroke::{Stroke, StrokeElement},
};
use bincode::config::standard;
use event::Combination;
use std::io::{Read, Write};

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

pub fn read<S: StrokeBackend>(mut reader: impl Read) -> Result<Sketch<S>, PmbError> {
    let mut magic = [0; 3];
    reader.read_exact(&mut magic)?;

    if magic != PMB_MAGIC {
        return Err(PmbError::new(ErrorKind::MissingHeader));
    }

    let mut version_bytes = [0; std::mem::size_of::<u64>()];
    reader.read_exact(&mut version_bytes)?;
    let version = migrate::Version(u64::from_le_bytes(version_bytes));

    log::debug!("got version {}", version);
    if version != Version::CURRENT {
        return Err(PmbError::new(ErrorKind::VersionMismatch(version)));
    }

    log::debug!("inflating");
    let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
    Ok(bincode::decode_from_std_read(
        &mut deflate_reader,
        standard(),
    )?)
}

pub fn write<S: StrokeBackend>(
    path: impl AsRef<std::path::Path>,
    state: &Sketch<S>,
) -> Result<(), PmbError> {
    log::debug!("truncating {} and deflating", path.as_ref().display());

    let mut file = std::fs::File::create(&path)?;
    file.write_all(&PMB_MAGIC)?;
    file.write_all(&u64::to_le_bytes(Version::CURRENT.0))?;

    let mut deflate_writer = flate2::write::DeflateEncoder::new(file, flate2::Compression::fast());
    bincode::encode_into_std_write(state, &mut deflate_writer, standard())?;

    Ok(())
}

pub trait CoordinateSystem: std::fmt::Debug + Default + Clone + Copy {
    type Ndc: std::fmt::Display + Clone + Copy;

    fn pixel_to_ndc(&self, width: u32, height: u32, pos: PixelPos) -> Self::Ndc;
    fn ndc_to_pixel(&self, width: u32, height: u32, pos: Self::Ndc) -> PixelPos;

    fn ndc_to_stroke(&self, width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint;
    fn stroke_to_ndc(&self, width: u32, height: u32, zoom: f32, point: StrokePoint) -> Self::Ndc;

    fn pixel_to_stroke(&self, width: u32, height: u32, zoom: f32, pos: PixelPos) -> StrokePoint {
        let ndc = self.pixel_to_ndc(width, height, pos);
        self.ndc_to_stroke(width, height, zoom, ndc)
    }

    fn stroke_to_pixel(&self, width: u32, height: u32, zoom: f32, pos: StrokePoint) -> PixelPos {
        let ndc = self.stroke_to_ndc(width, height, zoom, pos);
        self.ndc_to_pixel(width, height, ndc)
    }

    fn pixel_to_pos(
        &self,
        width: u32,
        height: u32,
        zoom: f32,
        origin: StrokePoint,
        pos: PixelPos,
    ) -> StrokePos {
        graphics::xform_point_to_pos(origin, self.pixel_to_stroke(width, height, zoom, pos))
    }

    fn pos_to_pixel(
        &self,
        width: u32,
        height: u32,
        zoom: f32,
        origin: StrokePoint,
        pos: StrokePos,
    ) -> PixelPos {
        self.stroke_to_pixel(
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

#[derive(Default, PartialEq, Debug, Clone, Copy)]
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

// TODO handle key combinations
#[derive(Debug)]
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
    pub save: Combination,
    pub reset_view: Combination,
    pub open: Combination,
    pub zoom_out: Combination,
    pub zoom_in: Combination,
    pub tool_for_gesture_1: Tool,
    pub tool_for_gesture_2: Tool,
    pub tool_for_gesture_3: Tool,
    pub tool_for_gesture_4: Tool,

    pub debug_toggle_stylus_invertability: Combination,
    pub debug_toggle_use_mouse_for_pen: Combination,
    pub debug_toggle_use_finger_for_pen: Combination,
    pub debug_clear_strokes: Combination,
    pub debug_print_strokes: Combination,
    pub debug_dirty_all_strokes: Combination,
}

impl Default for Config {
    #[cfg(feature = "pmb-release")]
    fn default() -> Self {
        Self::new()
    }

    #[cfg(not(feature = "pmb-release"))]
    fn default() -> Self {
        Self::debug()
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
            undo: LControl | Z,
            save: LControl | S,
            reset_view: Z.into(),
            open: LControl | O,
            zoom_out: LControl | NumpadSubtract,
            zoom_in: LControl | NumpadAdd,
            tool_for_gesture_1: Tool::Pan,
            tool_for_gesture_2: Tool::Pan,
            tool_for_gesture_3: Tool::Pan,
            tool_for_gesture_4: Tool::Pan,

            debug_toggle_stylus_invertability: Combination::INACTIVE,
            debug_toggle_use_mouse_for_pen: Combination::INACTIVE,
            debug_toggle_use_finger_for_pen: Combination::INACTIVE,
            debug_clear_strokes: Combination::INACTIVE,
            debug_print_strokes: Combination::INACTIVE,
            debug_dirty_all_strokes: Combination::INACTIVE,
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

    pub fn tool_for_gesture(&self, i: u8) -> Tool {
        match i {
            1 => self.tool_for_gesture_1,
            2 => self.tool_for_gesture_2,
            3 => self.tool_for_gesture_3,
            4 => self.tool_for_gesture_4,
            _ => Tool::Pan,
        }
    }
}

#[derive(derive_disk::Disk)]
pub struct Sketch<S: StrokeBackend> {
    pub strokes: Vec<Stroke<S>>,
    pub zoom: f32,
    pub origin: StrokePoint,
}

impl<S: StrokeBackend> Default for Sketch<S> {
    fn default() -> Self {
        Self::new(grid())
    }
}

impl<S: StrokeBackend> Sketch<S> {
    pub fn new(strokes: Vec<Stroke<S>>) -> Self {
        Self {
            strokes,
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

    pub fn update_visible_strokes(&mut self, top_left: StrokePos, bottom_right: StrokePos) {
        for stroke in self.strokes.iter_mut() {
            stroke.update_visible(top_left, bottom_right);
        }
    }

    fn update_stroke_primitive(&mut self) {
        for stroke in self.strokes.iter_mut() {
            stroke.draw_tesselated = stroke.brush_size * self.zoom > 1.0;
        }
    }

    pub fn update_from(&mut self, other: Sketch<S>, top_left: StrokePos, bottom_right: StrokePos) {
        self.strokes = other.strokes;
        self.update_zoom(other.zoom, top_left, bottom_right);
        self.move_origin(
            Default::default(),
            StrokePos {
                x: other.origin.x, // kill me :)
                y: other.origin.y,
            },
            top_left,
            bottom_right,
        );
    }

    pub fn clear_strokes(&mut self) {
        self.strokes.clear();
    }

    pub fn visible_strokes(&self) -> impl Iterator<Item = &Stroke<S>> {
        self.strokes
            .iter()
            .filter(|stroke| stroke.visible && !stroke.erased)
    }

    pub fn update_zoom(&mut self, next_zoom: f32, top_left: StrokePos, bottom_right: StrokePos) {
        self.zoom = if next_zoom < crate::MIN_ZOOM {
            crate::MIN_ZOOM
        } else if next_zoom > crate::MAX_ZOOM {
            crate::MAX_ZOOM
        } else {
            next_zoom
        };

        self.update_visible_strokes(top_left, bottom_right);
        self.update_stroke_primitive();
    }

    pub fn move_origin(
        &mut self,
        prev: StrokePos,
        next: StrokePos,
        top_left: StrokePos,
        bottom_right: StrokePos,
    ) {
        let dx = next.x - prev.x;
        let dy = next.y - prev.y;
        self.origin.x += dx;
        self.origin.y += dy;
        self.update_visible_strokes(top_left, bottom_right)
    }

    pub fn force_update(&mut self, top_left: StrokePos, bottom_right: StrokePos) {
        self.strokes
            .iter_mut()
            .flat_map(|stroke| stroke.backend_mut())
            .for_each(|backend| backend.make_dirty());
        self.update_visible_strokes(top_left, bottom_right);
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
    let mut strokes = (-100..=100)
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

    for stroke in strokes.iter_mut() {
        stroke.generate_full_mesh();
    }

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
