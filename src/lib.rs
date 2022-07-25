pub mod backend;
pub mod event;
pub mod graphics;
pub mod ui;

use crate::{
    backend::backend_impl,
    event::{Touch, TouchPhase},
    graphics::{Color, ColorExt, PixelPos, StrokePoint, StrokePos},
    ui::ToUi,
};
use bspline::BSpline;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::PathBuf,
};

pub type Result<T> = core::result::Result<T, PmbError>;

#[derive(Debug)]
pub enum PmbError {
    MissingHeader,
    IoError(std::io::Error),
    BincodeError(bincode::Error),
}

impl From<std::io::Error> for PmbError {
    fn from(err: std::io::Error) -> Self {
        PmbError::IoError(err)
    }
}

impl From<bincode::Error> for PmbError {
    fn from(err: bincode::Error) -> Self {
        PmbError::BincodeError(err)
    }
}

impl std::fmt::Display for PmbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PmbError::MissingHeader => write!(f, "Missing PMB header"),
            PmbError::IoError(err) => write!(f, "{err}"),
            PmbError::BincodeError(err) => write!(f, "{err}"),
        }
    }
}

pub fn read(mut r: impl Read) -> Result<ToDisk> {
    let mut magic = [0; 3];
    r.read_exact(&mut magic)?;

    if magic != [b'P', b'M', b'B'] {
        return Result::Err(PmbError::MissingHeader);
    }

    let reader = flate2::read::DeflateDecoder::new(r);
    Ok(bincode::deserialize_from(reader)?)
}

pub fn write(path: impl AsRef<std::path::Path>, disk: ToDisk) -> Result<()> {
    let mut file = std::fs::File::create(&path)?;
    file.write_all(&[b'P', b'M', b'B'])?;

    let writer = flate2::write::DeflateEncoder::new(file, flate2::Compression::fast());
    bincode::serialize_into(writer, &disk)?;

    let filename = path.as_ref().display();
    println!("saved as {filename}");

    Ok(())
}

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(packed)]
pub struct StrokeElement {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

impl std::ops::Add for StrokeElement {
    type Output = StrokeElement;
    fn add(self, rhs: Self) -> Self::Output {
        StrokeElement {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            pressure: self.pressure,
        }
    }
}

impl std::ops::Mul<f32> for StrokeElement {
    type Output = StrokeElement;
    fn mul(self, rhs: f32) -> Self::Output {
        StrokeElement {
            x: self.x * rhs,
            y: self.y * rhs,
            pressure: self.pressure,
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Stroke {
    pub points: Vec<StrokeElement>,
    pub color: Color,
    pub brush_size: f32,
    pub style: StrokeStyle,
    pub erased: bool,
    #[serde(skip)]
    pub spline: Option<BSpline<StrokeElement, f32>>,

    #[cfg(feature = "gl")]
    #[serde(skip)]
    pub vbo: Option<glow::Buffer>,
    #[cfg(feature = "gl")]
    #[serde(skip)]
    pub vao: Option<glow::VertexArray>,
}

impl Stroke {
    pub const DEGREE: usize = 3;

    pub fn calculate_spline(&mut self) {
        if self.points.len() > Stroke::DEGREE {
            let points = [self.points.first().cloned().unwrap(); Stroke::DEGREE]
                .into_iter()
                .chain(self.points.iter().cloned())
                .chain([self.points.last().cloned().unwrap(); Stroke::DEGREE])
                .map(|point| point.into())
                .collect::<Vec<StrokeElement>>();

            let knots = std::iter::repeat(())
                .take(points.len() + Stroke::DEGREE + 1)
                .enumerate()
                .map(|(i, ())| i as f32)
                .collect::<Vec<_>>();

            self.spline = Some(BSpline::new(Stroke::DEGREE, points, knots));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, evc_derive::EnumVariantCount, Serialize, Deserialize)]
#[repr(usize)]
#[allow(dead_code)]
pub enum StrokeStyle {
    Lines,
    Circles,
    CirclesPressure,
    Points,
    Spline,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        StrokeStyle::Lines
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

#[derive(Debug, Clone, Copy)]
pub enum GestureState {
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

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub brush_size: usize,
    pub stroke_style: StrokeStyle,
    pub use_individual_style: bool,
    pub zoom: f32,
    pub origin: StrokePoint,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            brush_size: DEFAULT_BRUSH,
            stroke_style: StrokeStyle::Lines,
            use_individual_style: false,
            zoom: DEFAULT_ZOOM,
            origin: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ToDisk {
    pub strokes: Vec<Stroke>,
    pub settings: Settings,
}

pub struct State {
    pub stylus: Stylus,
    pub strokes: Vec<Stroke>,
    pub gesture_state: GestureState,
    pub settings: Settings,
    pub modified: bool,
    pub path: Option<PathBuf>,
}

// Default for State {{{
impl Default for State {
    fn default() -> Self {
        use std::iter::repeat;
        let mut strokes = vec![Stroke {
            points: graphics::circle_points(1.0, 50)
                .chunks_exact(2)
                .map(|arr| StrokeElement {
                    x: arr[0],
                    y: arr[1],
                    pressure: 1.0,
                })
                .collect(),
            color: Color::WHITE,
            ..Default::default()
        }];

        strokes.extend(repeat(-25.0).take(50).enumerate().map(|(i, x)| {
            Stroke {
                points: repeat(-25.0)
                    .take(50)
                    .enumerate()
                    .map(|(j, y)| StrokeElement {
                        x: i as f32 + x,
                        y: j as f32 + y,
                        pressure: 1.0,
                    })
                    .collect(),
                color: Color::grey(0.1),
                ..Default::default()
            }
        }));

        strokes.extend(repeat(-25.0).take(50).enumerate().map(|(i, y)| {
            Stroke {
                points: repeat(-25.0)
                    .take(50)
                    .enumerate()
                    .map(|(j, x)| StrokeElement {
                        x: j as f32 + x,
                        y: i as f32 + y,
                        pressure: 1.0,
                    })
                    .collect(),
                color: Color::grey(0.1),
                ..Default::default()
            }
        }));

        strokes.push(Stroke {
            points: repeat(-25.0)
                .take(50)
                .enumerate()
                .map(|(i, x)| StrokeElement {
                    x: i as f32 + x,
                    y: 0.0,
                    pressure: 1.0,
                })
                .collect(),
            color: Color::grey(0.3),
            ..Default::default()
        });

        strokes.push(Stroke {
            points: repeat(-25.0)
                .take(50)
                .enumerate()
                .map(|(i, y)| StrokeElement {
                    x: 0.0,
                    y: i as f32 + y,
                    pressure: 1.0,
                })
                .collect(),
            color: Color::grey(0.3),
            ..Default::default()
        });

        State {
            stylus: Default::default(),
            strokes,
            gesture_state: GestureState::NoInput,
            settings: Default::default(),
            modified: false,
            path: None,
        }
    }
}
// }}}

pub const DEFAULT_ZOOM: f32 = 50.;
pub const MAX_ZOOM: f32 = 500.;
pub const MIN_ZOOM: f32 = 1.;

pub const DEFAULT_BRUSH: usize = 1;
pub const MAX_BRUSH: usize = 20;
pub const MIN_BRUSH: usize = 1;
pub const BRUSH_DELTA: usize = 1;

impl State {
    pub fn modified() -> State {
        let mut this = State::default();
        this.modified = true;
        this
    }

    pub fn with_filename(path: impl AsRef<std::path::Path>) -> State {
        let mut this = State::default();
        this.path = Some(path.as_ref().to_path_buf());
        this
    }

    // returns whether to exit or overwrite state
    pub fn ask_to_save_then_save(&mut self, why: &str) -> Result<bool> {
        match (ui::ask_to_save(why), self.path.as_ref()) {
            // if they say yes and the file we're editing has a path
            (rfd::MessageDialogResult::Yes, Some(path)) => {
                // ask where to save it
                // TODO a setting for this?
                match ui::save_dialog("Save changes", Some(path.as_path())) {
                    Some(new_filename) => {
                        // try write to disk
                        let message = format!("Could not save file as {}", new_filename.display());
                        write(new_filename, self.to_disk()).error_dialog(&message)?;
                        self.modified = false;
                        Ok(true)
                    }

                    None => Ok(false),
                }
            }

            // they say yes and the file doesn't have a path yet
            (rfd::MessageDialogResult::Yes, None) => {
                // ask where to save it
                match ui::save_dialog("Save unnamed file", None) {
                    Some(new_filename) => {
                        // try write to disk
                        let message = format!("Could not save file as {}", new_filename.display());
                        write(new_filename, self.to_disk()).error_dialog(&message)?;
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
                *self = State::with_filename(path);
                self.modified = true;
                return Ok(());
            }
            Err(err) => return Err(PmbError::from(err)),
        };

        // read the new file
        let mut disk = read(file)?;

        // if successful, update the state
        for stroke in disk.strokes.iter_mut() {
            stroke.calculate_spline();
        }

        self.strokes = disk.strokes;
        self.settings = disk.settings;
        self.modified = false;
        self.path = Some(path);

        Ok(())
    }

    pub fn save_file(&mut self) -> Result<()> {
        if let Some(path) = self.path.as_ref() {
            let message = format!("Could not save file {}", path.display());
            write(path, self.to_disk()).error_dialog(&message)?;
            self.modified = false;
        } else if let Some(path) = ui::save_dialog("Save unnamed file", None) {
            let message = format!("Could not save file {}", path.display());
            self.path = Some(path);
            write(self.path.as_ref().unwrap(), self.to_disk()).error_dialog(&message)?;
            self.modified = false;
        }

        Ok(())
    }

    pub fn to_disk(&self) -> ToDisk {
        ToDisk {
            strokes: self.strokes.clone(),
            settings: self.settings.clone(),
        }
    }

    pub fn increase_brush(&mut self) {
        self.settings.brush_size += BRUSH_DELTA;
        self.settings.brush_size = self.settings.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);
    }

    pub fn decrease_brush(&mut self) {
        self.settings.brush_size -= BRUSH_DELTA;
        self.settings.brush_size = self.settings.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);
    }

    pub fn move_origin(&mut self, width: u32, height: u32, prev: PixelPos, next: PixelPos) {
        use backend_impl::*;

        let prev_ndc = pixel_to_ndc(width, height, prev);
        let prev_stroke = ndc_to_stroke(width, height, self.settings.zoom, prev_ndc);
        let prev_xformed = xform_point_to_pos(self.settings.origin, prev_stroke);

        let next_ndc = pixel_to_ndc(width, height, next);
        let next_stroke = ndc_to_stroke(width, height, self.settings.zoom, next_ndc);
        let next_xformed = xform_point_to_pos(self.settings.origin, next_stroke);

        let dx = next_xformed.x - prev_xformed.x;
        let dy = next_xformed.y - prev_xformed.y;
        self.settings.origin.x += dx;
        self.settings.origin.y += dy;
    }

    pub fn change_zoom(&mut self, dz: f32) {
        if (self.settings.zoom + dz).is_finite() {
            self.settings.zoom += dz;
        }

        self.settings.zoom = self.settings.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    pub fn clear_strokes(&mut self) {
        self.modified = true;
        std::mem::take(&mut self.strokes);
    }

    pub fn undo_stroke(&mut self) {
        self.modified = true;
        self.strokes.pop();
    }

    pub fn update(&mut self, width: u32, height: u32, touch: Touch) {
        let Touch {
            force,
            phase,
            location,
            pen_info,
            ..
        } = touch;

        let ndc_pos = backend_impl::pixel_to_ndc(width, height, location);
        let point = backend_impl::ndc_to_stroke(width, height, self.settings.zoom, ndc_pos);
        let pressure = force.unwrap_or(0.0);

        let inverted = pen_info
            .map(|info| info.inverted)
            .unwrap_or(self.stylus.state.inverted);

        let state = match phase {
            TouchPhase::Start => {
                if pen_info.is_none() && self.gesture_state.touch() {
                    self.undo_stroke();
                }

                StylusState {
                    pos: StylusPosition::Down,
                    inverted,
                }
            }

            TouchPhase::Move => {
                self.stylus.state.inverted = inverted;
                self.stylus.state
            }

            TouchPhase::End | TouchPhase::Cancel => {
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
        self.stylus.pos = backend_impl::xform_point_to_pos(self.settings.origin, self.stylus.point);
        self.stylus.pressure = pressure as f32;
        self.stylus.state = state;

        if pen_info.is_none() && self.gesture_state.active() {
            self.stylus.state.pos = StylusPosition::Up;
            return;
        }

        self.handle_update(width, height, phase);
    }

    fn handle_update(&mut self, width: u32, height: u32, phase: TouchPhase) {
        use backend_impl::*;

        if self.stylus.inverted() {
            let stylus_ndc = stroke_to_ndc(
                width,
                height,
                self.settings.zoom,
                StrokePoint {
                    x: self.stylus.pos.x,
                    y: self.stylus.pos.y,
                },
            );
            let stylus_pix = ndc_to_pixel(width, height, stylus_ndc);
            let stylus_pix_x = stylus_pix.x as f32;
            let stylus_pix_y = stylus_pix.y as f32;

            if phase == TouchPhase::Move && self.stylus.down() {
                for stroke in self.strokes.iter_mut() {
                    if stroke.erased {
                        continue;
                    }

                    'inner: for point in stroke.points.iter() {
                        let point_ndc = stroke_to_ndc(
                            width,
                            height,
                            self.settings.zoom,
                            StrokePoint {
                                x: point.x,
                                y: point.y,
                            },
                        );
                        let point_pix = ndc_to_pixel(width, height, point_ndc);
                        let point_pix_x = point_pix.x as f32;
                        let point_pix_y = point_pix.y as f32;

                        let dist = ((stylus_pix_x - point_pix_x).powi(2)
                            + (stylus_pix_y - point_pix_y).powi(2))
                        .sqrt()
                            * 2.0;

                        if dist < self.settings.brush_size as f32 {
                            stroke.erased = true;
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
                    self.strokes.push(Stroke {
                        points: Vec::new(),
                        color: rand::random(),
                        brush_size: self.settings.brush_size as f32,
                        style: self.settings.stroke_style,
                        erased: false,
                        spline: None,

                        #[cfg(feature = "gl")]
                        vbo: None,
                        #[cfg(feature = "gl")]
                        vao: None,
                    });
                }

                TouchPhase::Move => {
                    if let Some(stroke) = self.strokes.last_mut() {
                        if self.stylus.down() {
                            stroke.points.push(StrokeElement {
                                x: self.stylus.pos.x,
                                y: self.stylus.pos.y,
                                pressure: self.stylus.pressure,
                            });

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
