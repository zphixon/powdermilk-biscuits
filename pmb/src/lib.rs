pub mod error;
pub mod event;
pub mod graphics;
pub mod input;
pub mod stroke;
pub mod ui;

use crate::{
    error::{PmbError, Result},
    event::{Touch, TouchPhase},
    graphics::{Color, ColorExt, PixelPos, StrokePoint, StrokePos},
    stroke::{Stroke, StrokeElement, StrokeStyle},
    ui::ToUi,
};
use bincode::config::standard;
use std::{
    io::{Read, Write},
    path::PathBuf,
};

pub const TITLE_UNMODIFIED: &'static str = "hi! <3";
pub const TITLE_MODIFIED: &'static str = "hi! <3 (modified)";
pub const PMB_MAGIC: [u8; 3] = [b'P', b'M', b'B'];

pub fn read<B, S>(mut reader: impl Read) -> Result<State<B, S>>
where
    B: Backend,
    S: StrokeBackend,
{
    let mut magic = [0; 3];
    reader.read_exact(&mut magic)?;

    if magic != PMB_MAGIC {
        return Err(PmbError::MissingHeader);
    }

    let mut deflate_reader = flate2::read::DeflateDecoder::new(reader);
    Ok(bincode::decode_from_std_read(
        &mut deflate_reader,
        standard(),
    )?)
}

pub fn write<B, S>(path: impl AsRef<std::path::Path>, state: &State<B, S>) -> Result<()>
where
    B: Backend,
    S: StrokeBackend,
{
    let mut file = std::fs::File::create(&path)?;
    file.write_all(&PMB_MAGIC)?;

    let mut deflate_writer = flate2::write::DeflateEncoder::new(file, flate2::Compression::fast());
    bincode::encode_into_std_write(state, &mut deflate_writer, standard())?;

    Ok(())
}

pub trait Backend: std::fmt::Debug + Default + Clone + Copy {
    type Ndc: std::fmt::Display + Clone + Copy;

    fn pixel_to_ndc(&self, width: u32, height: u32, pos: PixelPos) -> Self::Ndc;
    fn ndc_to_pixel(&self, width: u32, height: u32, pos: Self::Ndc) -> PixelPos;

    fn ndc_to_stroke(&self, width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint;
    fn stroke_to_ndc(&self, width: u32, height: u32, zoom: f32, point: StrokePoint) -> Self::Ndc;
}

pub trait StrokeBackend: std::fmt::Debug + Default {
    fn make_dirty(&mut self);
    fn is_dirty(&self) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub enum StylusPosition {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy)]
pub struct StylusState {
    pub pos: StylusPosition,
    pub inverted: bool,
}

impl Default for StylusState {
    fn default() -> Self {
        StylusState {
            pos: StylusPosition::Up,
            inverted: false,
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

    pub fn inverted(&self) -> bool {
        self.state.inverted
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum GestureState {
    #[default]
    NoInput,
    Stroke,
    Active(usize),
}

impl GestureState {
    pub fn active(&self) -> bool {
        use GestureState::*;
        !matches!(self, NoInput | Stroke)
    }

    // returns should delete last stroke
    pub fn touch(&mut self) -> bool {
        use GestureState::*;

        let prev = *self;
        *self = match *self {
            NoInput => Stroke,
            Stroke => Active(2),
            Active(num) => Active(num + 1),
        };

        if self.active() {
            println!("do gesture {self:?}");
        }

        matches!(prev, Stroke) && matches!(self, Active(2))
    }

    pub fn release(&mut self) {
        use GestureState::*;
        *self = match *self {
            NoInput | Stroke | Active(1) => NoInput,
            Active(num) => Active(num - 1),
        };

        if self.active() {
            println!("do gesture {self:?}");
        }
    }
}

#[derive(pmb_derive_disk::Disk)]
pub struct State<B, S>
where
    B: Backend,
    S: StrokeBackend,
{
    pub strokes: Vec<Stroke<S>>,
    pub brush_size: usize,
    pub stroke_style: StrokeStyle,
    pub use_individual_style: bool,
    pub zoom: f32,
    pub origin: StrokePoint,

    #[disk_skip]
    pub stylus: Stylus,
    #[disk_skip]
    pub gesture_state: GestureState,
    #[disk_skip]
    pub modified: bool,
    #[disk_skip]
    pub path: Option<PathBuf>,
    #[disk_skip]
    pub input: input::InputHandler,
    #[disk_skip]
    pub backend: Option<B>,
}

impl<B, S> Default for State<B, S>
where
    B: Backend,
    S: StrokeBackend,
{
    fn default() -> Self {
        Self::new()
    }
}

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
                pressure: 1.0,
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
                    pressure: 1.0,
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
                    pressure: 1.0,
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
                pressure: 1.0,
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
                pressure: 1.0,
            })
            .collect(),
        Color::grey(0.3),
    ));

    strokes
}

pub const DEFAULT_ZOOM: f32 = 50.;
pub const MAX_ZOOM: f32 = 500.;
pub const MIN_ZOOM: f32 = 1.;

pub const DEFAULT_BRUSH: usize = 1;
pub const MAX_BRUSH: usize = 20;
pub const MIN_BRUSH: usize = 1;
pub const BRUSH_DELTA: usize = 1;

impl<B, S> State<B, S>
where
    B: Backend,
    S: StrokeBackend,
{
    pub fn new() -> Self {
        Self {
            strokes: grid(),
            brush_size: DEFAULT_BRUSH,
            stroke_style: StrokeStyle::default(),
            use_individual_style: false,
            zoom: DEFAULT_ZOOM,
            origin: StrokePoint::default(),
            stylus: Stylus::default(),
            gesture_state: GestureState::NoInput,
            modified: false,
            path: None,
            input: input::InputHandler::default(),
            backend: Some(Default::default()),
        }
    }

    fn backend(&self) -> B {
        self.backend.unwrap()
    }

    pub fn modified() -> Self {
        let mut this = State::new();
        this.modified = true;
        this
    }

    pub fn geedis(&self) {
        let bin = bincode::encode_to_vec(&self, standard()).unwrap();
        let (this, _): (Self, _) = bincode::decode_from_slice(&bin, standard()).unwrap();

        assert_eq!(self.strokes.len(), this.strokes.len());
        for (a, b) in self.strokes.iter().zip(this.strokes.iter()) {
            assert_eq!(a.points().len(), b.points().len());
            for (ae, be) in a.points().iter().zip(b.points().iter()) {
                let (ax, bx) = (ae.x, be.x);
                let (ay, by) = (ae.y, be.y);
                let (ap, bp) = (ae.pressure, be.pressure);
                assert_eq!(ax, bx);
                assert_eq!(ay, by);
                assert_eq!(ap, bp);
            }
        }
    }

    pub fn with_filename(path: impl AsRef<std::path::Path>) -> Self {
        let mut this = State::new();
        let message = format!("Could not open {}", path.as_ref().display());
        let _ = this.read_file(Some(path)).error_dialog(&message);
        this
    }

    pub fn handle_key(&mut self, key: input::Keycode, state: input::ElementState) {
        use input::Keycode::*;
        self.input.handle_key(key, state);

        macro_rules! just_pressed {
            ($key:ident) => {
                just_pressed!($key, false, false)
            };

            (ctrl + $key:ident) => {
                just_pressed!($key, true, false)
            };

            (shift + $key:ident) => {
                just_pressed!($key, false, true)
            };

            (ctrl + shift + $key:ident) => {
                just_pressed!($key, true, true)
            };

            ($key:ident, $ctrl:expr, $shift:expr) => {
                self.input.just_pressed($key)
                    && if $ctrl {
                        self.input.control()
                    } else {
                        !self.input.control()
                    }
                    && if $shift {
                        self.input.shift()
                    } else {
                        !self.input.shift()
                    }
            };
        }

        if just_pressed!(RBracket) {
            self.increase_brush();
        }

        if just_pressed!(LBracket) {
            self.decrease_brush();
        }

        if just_pressed!(C) {
            self.clear_strokes();
        }

        if just_pressed!(D) {
            for stroke in self.strokes.iter() {
                println!("stroke");
                for point in stroke.points().iter() {
                    let x = point.x;
                    let y = point.y;
                    let p = point.pressure;
                    println!("{x}, {y}, {p}");
                }
            }
            println!("brush={}", self.brush_size);
            println!("zoom={:.02}", self.zoom);
            println!("origin={}", self.origin);
        }

        if just_pressed!(ctrl + Z) {
            self.undo_stroke();
            self.input.clear();
        }

        if just_pressed!(ctrl + S) {
            let _ = self.save_file();
            self.input.clear();
        }

        if just_pressed!(Z) {
            self.reset_view();
        }

        if just_pressed!(E) {
            self.stylus.state.inverted = !self.stylus.state.inverted;
        }

        if just_pressed!(ctrl + O) {
            let _ = self.read_file(Option::<&str>::None);
            self.input.clear();
        }
    }

    pub fn reset_view(&mut self) {
        self.zoom = DEFAULT_ZOOM;
        self.origin = Default::default();
    }

    pub fn handle_mouse_move(&mut self, location: PixelPos) {
        self.input.handle_mouse_move(location);
    }

    pub fn handle_touch(&mut self, touch: Touch, width: u32, height: u32) {
        let prev_y = self.input.cursor_pos().y;
        self.handle_mouse_move(touch.location);
        let next_y = self.input.cursor_pos().y;
        let dy = next_y - prev_y;

        let prev_ndc = self
            .backend()
            .stroke_to_ndc(width, height, self.zoom, self.stylus.point);
        let prev_stylus = self.backend().ndc_to_pixel(width, height, prev_ndc);

        self.update_stylus(width, height, touch);

        let next_ndc = self
            .backend()
            .stroke_to_ndc(width, height, self.zoom, self.stylus.point);
        let next_stylus = self.backend().ndc_to_pixel(width, height, next_ndc);

        match (
            self.input.button_down(input::MouseButton::Middle),
            self.input.control(),
        ) {
            (true, false) => {
                self.move_origin(width, height, prev_stylus, next_stylus);
            }
            (true, true) => self.change_zoom(dy),
            _ => {}
        }
    }

    pub fn handle_cursor_move(&mut self, width: u32, height: u32, position: PixelPos) {
        let prev = self.input.cursor_pos();
        self.input.handle_mouse_move(position);

        if self.input.button_down(input::MouseButton::Left) {
            let next = self.input.cursor_pos();
            self.move_origin(width, height, prev, next);
        }
    }

    pub fn handle_mouse_button(&mut self, button: input::MouseButton, state: input::ElementState) {
        self.input.handle_mouse_button(button, state);
    }

    // returns whether to exit or overwrite state
    pub fn ask_to_save_then_save(&mut self, why: &str) -> Result<bool> {
        match (ui::ask_to_save(why), self.path.as_ref()) {
            // if they say yes and the file we're editing has a path
            (rfd::MessageDialogResult::Yes, Some(path)) => {
                let message = format!("Could not save file as {}", path.display());
                write(path, &self).error_dialog(&message)?;
                self.modified = false;
                Ok(true)
            }

            // they say yes and the file doesn't have a path yet
            (rfd::MessageDialogResult::Yes, None) => {
                // ask where to save it
                match ui::save_dialog("Save unnamed file", None) {
                    Some(new_filename) => {
                        // try write to disk
                        let message = format!("Could not save file as {}", new_filename.display());
                        write(new_filename, &self).error_dialog(&message)?;
                        self.modified = false;
                        Ok(true)
                    }

                    None => Ok(false),
                }
            }

            // they say no, don't write changes
            (rfd::MessageDialogResult::No, _) => Ok(true),

            _ => Ok(false),
        }
    }

    pub fn read_file(&mut self, path: Option<impl AsRef<std::path::Path>>) -> Result<()> {
        // if we are modified
        if self.modified {
            // ask to save first
            if !self.ask_to_save_then_save("Would you like to save before opening another file?")? {
                return Ok(());
            }
        }

        // if we were passed a path, use that, otherwise ask for one
        let path = match path
            .map(|path| path.as_ref().to_path_buf())
            .or_else(|| ui::open_dialog())
        {
            Some(path) => path,
            None => {
                return Ok(());
            }
        };

        // open the new file
        let file = match std::fs::File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // if it doesn't exist don't try to read it
                *self = State::default();
                self.path = Some(path);
                self.modified = true;
                return Ok(());
            }
            Err(err) => return Err(PmbError::from(err)),
        };

        // read the new file
        let mut disk: Self = read(file)?;
        disk.strokes.iter_mut().for_each(Stroke::calculate_spline);

        self.strokes = disk.strokes;
        self.brush_size = disk.brush_size;
        self.stroke_style = disk.stroke_style;
        self.use_individual_style = disk.use_individual_style;
        self.zoom = disk.zoom;
        self.origin = disk.origin;
        self.stylus = disk.stylus;

        self.modified = false;
        self.path = Some(path);

        Ok(())
    }

    pub fn save_file(&mut self) -> Result<()> {
        if let Some(path) = self.path.as_ref() {
            let message = format!("Could not save file {}", path.display());
            write(path, &self).error_dialog(&message)?;
            self.modified = false;
        } else if let Some(path) = ui::save_dialog("Save unnamed file", None) {
            let message = format!("Could not save file {}", path.display());
            self.path = Some(path);
            write(self.path.as_ref().unwrap(), &self).error_dialog(&message)?;
            self.modified = false;
        }

        Ok(())
    }

    pub fn increase_brush(&mut self) {
        self.brush_size += BRUSH_DELTA;
        self.brush_size = self.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);
    }

    pub fn decrease_brush(&mut self) {
        self.brush_size -= BRUSH_DELTA;
        self.brush_size = self.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);
    }

    fn move_origin(&mut self, width: u32, height: u32, prev: PixelPos, next: PixelPos) {
        let prev_ndc = self.backend().pixel_to_ndc(width, height, prev);
        let prev_stroke = self
            .backend()
            .ndc_to_stroke(width, height, self.zoom, prev_ndc);
        let prev_xformed = graphics::xform_point_to_pos(self.origin, prev_stroke);

        let next_ndc = self.backend().pixel_to_ndc(width, height, next);
        let next_stroke = self
            .backend()
            .ndc_to_stroke(width, height, self.zoom, next_ndc);
        let next_xformed = graphics::xform_point_to_pos(self.origin, next_stroke);

        let dx = next_xformed.x - prev_xformed.x;
        let dy = next_xformed.y - prev_xformed.y;
        self.origin.x += dx;
        self.origin.y += dy;
    }

    pub fn change_zoom(&mut self, dz: f32) {
        if (self.zoom + dz).is_finite() {
            self.zoom += dz;
        }

        self.zoom = self.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    fn clear_strokes(&mut self) {
        self.modified = true;
        std::mem::take(&mut self.strokes);
    }

    pub fn undo_stroke(&mut self) {
        self.modified = true;
        self.strokes.pop();
    }

    fn update_stylus(&mut self, width: u32, height: u32, touch: Touch) {
        let Touch {
            force,
            phase,
            location,
            pen_info,
            ..
        } = touch;

        let ndc_pos = self.backend().pixel_to_ndc(width, height, location);
        let point = self
            .backend()
            .ndc_to_stroke(width, height, self.zoom, ndc_pos);
        let pos = graphics::xform_point_to_pos(self.origin, point);
        let pressure = force.unwrap_or(1.0);

        let inverted = pen_info
            .map(|info| info.inverted)
            .unwrap_or(self.stylus.state.inverted);

        let status =
            format!("{pressure:.02} pix={location} -> ndc={ndc_pos} -> str={pos}                ");

        let state = match phase {
            TouchPhase::Start => {
                if pen_info.is_none() && self.gesture_state.touch() {
                    self.undo_stroke();
                }

                if !self.gesture_state.active() {
                    print!("{status}");
                    std::io::stdout().flush().unwrap();
                }

                StylusState {
                    pos: StylusPosition::Down,
                    inverted,
                }
            }

            TouchPhase::Move => {
                if !self.gesture_state.active() && self.stylus.down() {
                    print!("\r{status}");
                }

                self.stylus.state.inverted = inverted;
                self.stylus.state
            }

            TouchPhase::End | TouchPhase::Cancel => {
                if !self.gesture_state.active() {
                    println!();
                }

                if pen_info.is_none() {
                    self.gesture_state.release();
                }

                StylusState {
                    pos: StylusPosition::Up,
                    inverted,
                }
            }
        };

        self.stylus.point = point;
        self.stylus.pos = pos;
        self.stylus.pressure = pressure as f32;
        self.stylus.state = state;

        if pen_info.is_none() && self.gesture_state.active() {
            self.stylus.state.pos = StylusPosition::Up;
            return;
        }

        self.handle_update(width, height, phase);
    }

    fn handle_update(&mut self, width: u32, height: u32, phase: TouchPhase) {
        if self.stylus.inverted() {
            let stylus_ndc = self.backend().stroke_to_ndc(
                width,
                height,
                self.zoom,
                StrokePoint {
                    x: self.stylus.pos.x,
                    y: self.stylus.pos.y,
                },
            );
            let stylus_pix = self.backend().ndc_to_pixel(width, height, stylus_ndc);
            let stylus_pix_x = stylus_pix.x as f32;
            let stylus_pix_y = stylus_pix.y as f32;

            if phase == TouchPhase::Move && self.stylus.down() {
                for stroke in self.strokes.iter_mut() {
                    if stroke.erased() {
                        continue;
                    }

                    'inner: for point in stroke.points().iter() {
                        let point_ndc = self.backend.unwrap().stroke_to_ndc(
                            width,
                            height,
                            self.zoom,
                            StrokePoint {
                                x: point.x,
                                y: point.y,
                            },
                        );
                        let point_pix =
                            self.backend.unwrap().ndc_to_pixel(width, height, point_ndc);
                        let point_pix_x = point_pix.x as f32;
                        let point_pix_y = point_pix.y as f32;

                        let dist = ((stylus_pix_x - point_pix_x).powi(2)
                            + (stylus_pix_y - point_pix_y).powi(2))
                        .sqrt()
                            * 2.0;

                        if dist < self.brush_size as f32 {
                            stroke.erase();
                            self.modified = true;
                            break 'inner;
                        }
                    }
                }
            }
        } else {
            match phase {
                TouchPhase::Start => {
                    self.modified = true;
                    self.strokes
                        .push(Stroke::new(rand::random(), self.brush_size as f32));
                }

                TouchPhase::Move => {
                    if let Some(stroke) = self.strokes.last_mut() {
                        if self.stylus.down() {
                            stroke.points_mut().push(StrokeElement {
                                x: self.stylus.pos.x,
                                y: self.stylus.pos.y,
                                pressure: self.stylus.pressure,
                            });
                            stroke.backend_mut().map(|backend| backend.make_dirty());
                            stroke.calculate_spline();
                        }
                    }
                }

                TouchPhase::End | TouchPhase::Cancel => {
                    if let Some(stroke) = self.strokes.last_mut() {
                        stroke.calculate_spline();
                    }
                }
            };
        }
    }
}
