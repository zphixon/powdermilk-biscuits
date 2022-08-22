pub mod error;
pub mod event;
pub mod graphics;
pub mod input;
pub mod migrate;
pub mod stroke;
pub mod ui;

use crate::{
    error::{ErrorKind, PmbError, PmbErrorExt},
    event::{Touch, TouchPhase},
    graphics::{Color, ColorExt, PixelPos, StrokePoint, StrokePos},
    migrate::{UpgradeType, Version},
    stroke::{Stroke, StrokeElement},
};
use bincode::config::standard;
use std::{
    io::{Read, Write},
    path::PathBuf,
};

pub const TITLE_UNMODIFIED: &str = "hi! <3";
pub const TITLE_MODIFIED: &str = "hi! <3 (modified)";
pub const PMB_MAGIC: [u8; 3] = [b'P', b'M', b'B'];

pub fn read<B, S>(mut reader: impl Read) -> Result<State<B, S>, PmbError>
where
    B: Backend,
    S: StrokeBackend,
{
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

pub fn write<B, S>(path: impl AsRef<std::path::Path>, state: &State<B, S>) -> Result<(), PmbError>
where
    B: Backend,
    S: StrokeBackend,
{
    log::debug!("truncating {} and deflating", path.as_ref().display());

    let mut file = std::fs::File::create(&path)?;
    file.write_all(&PMB_MAGIC)?;
    file.write_all(&u64::to_le_bytes(Version::CURRENT.0))?;

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

    fn pixel_to_stroke(&self, width: u32, height: u32, zoom: f32, pos: PixelPos) -> StrokePoint {
        let ndc = self.pixel_to_ndc(width, height, pos);
        self.ndc_to_stroke(width, height, zoom, ndc)
    }

    fn stroke_to_pixel(&self, width: u32, height: u32, zoom: f32, pos: StrokePoint) -> PixelPos {
        let ndc = self.stroke_to_ndc(width, height, zoom, pos);
        self.ndc_to_pixel(width, height, ndc)
    }
}

pub trait StrokeBackend: std::fmt::Debug {
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
            log::debug!("do gesture {self:?}");
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
            log::debug!("do gesture {self:?}");
        }
    }
}

#[rustfmt::skip]
#[derive(derive_disk::Disk)]
pub struct State<B, S>
where
    B: Backend,
    S: StrokeBackend,
{
    pub strokes: Vec<Stroke<S>>,
    pub brush_size: usize,
    pub zoom: f32,
    pub origin: StrokePoint,

    #[disk_skip] pub stylus: Stylus,
    #[disk_skip] pub gesture_state: GestureState,
    #[disk_skip] pub modified: bool,
    #[disk_skip] pub path: Option<PathBuf>,
    #[disk_skip] pub input: input::InputHandler,
    #[disk_skip] pub backend: B,
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
        let mut this = Self {
            strokes: grid(),
            brush_size: DEFAULT_BRUSH,
            zoom: DEFAULT_ZOOM,
            origin: StrokePoint::default(),
            stylus: Stylus::default(),
            gesture_state: GestureState::NoInput,
            modified: false,
            path: None,
            input: input::InputHandler::default(),
            backend: Default::default(),
        };

        for stroke in this.strokes.iter_mut() {
            stroke.draw_tesselated = stroke.brush_size * this.zoom > 1.0;
        }

        this
    }

    pub fn benchmark() -> Self {
        let mut this = Self::new();
        this.strokes = benchmark();
        this
    }

    pub fn modified() -> Self {
        let mut this = State::new();
        this.modified = true;
        this
    }

    pub fn update_from(&mut self, other: State<B, S>) {
        self.strokes = other.strokes;
        self.brush_size = other.brush_size;
        self.zoom = other.zoom;
        self.origin = other.origin;
        self.stylus = other.stylus;

        self.strokes.iter_mut().for_each(|stroke| {
            stroke.generate_full_mesh();
            stroke.update_bounding_box();
        });
    }

    pub fn with_filename(path: impl AsRef<std::path::Path>) -> Self {
        log::info!("create State from {}", path.as_ref().display());

        let mut this = State::new();
        this.read_file(Some(path))
            .problem(String::from("Could not open file"))
            .display();

        this
    }

    pub fn reset_view(&mut self) {
        self.zoom = DEFAULT_ZOOM;
        self.origin = Default::default();
        self.update_stroke_primitive();
    }

    pub fn handle_key(
        &mut self,
        key: input::Keycode,
        state: input::ElementState,
        width: u32,
        height: u32,
    ) -> bool {
        log::debug!("handle key {key:?} {state:?}");

        use input::Keycode::*;
        self.input.handle_key(key, state);
        let mut request_redraw = false;

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
            request_redraw = true;
        }

        if just_pressed!(LBracket) {
            self.decrease_brush();
            request_redraw = true;
        }

        if just_pressed!(C) {
            self.clear_strokes();
            request_redraw = true;
        }

        if just_pressed!(D) {
            for stroke in self.strokes.iter() {
                println!("stroke");
                for point in stroke.points().iter() {
                    println!("{},{},{}", point.x, point.y, point.pressure);
                }
                println!(
                    "{} points, {} vertices, {} size, {} visible, {:?} color, {} top left, {} bottom right",
                    stroke.points().len(),
                    stroke.mesh.len(),
                    stroke.brush_size(),
                    stroke.visible,
                    stroke.color(),
                    stroke.top_left,
                    stroke.bottom_right,
                );
            }
            println!("brush={}", self.brush_size);
            println!("zoom={:.02}", self.zoom);
            println!("origin={}", self.origin);
        }

        if just_pressed!(ctrl + Z) {
            self.undo_stroke();
            request_redraw = true;
        }

        if just_pressed!(ctrl + S) {
            self.save_file()
                .problem(format!("Could not save file"))
                .display();
        }

        if just_pressed!(Z) {
            self.reset_view();
            request_redraw = true;
        }

        if just_pressed!(E) {
            self.stylus.state.eraser = !self.stylus.state.eraser;
            request_redraw = true;
        }

        if just_pressed!(ctrl + O) {
            self.read_file(Option::<&str>::None)
                .problem(format!("Could not open file"))
                .display();
            request_redraw = true;
        }

        if just_pressed!(ctrl + NumpadSubtract) {
            self.change_zoom(-4.25, width, height);
            request_redraw = true;
        }

        if just_pressed!(ctrl + NumpadAdd) {
            self.change_zoom(4.25, width, height);
            request_redraw = true;
        }

        self.input.upstrokes();
        request_redraw
    }

    pub fn handle_touch(&mut self, touch: Touch, width: u32, height: u32) {
        log::trace!("handle touch {touch:?}");

        let prev_y = self.input.cursor_pos().y;
        self.input.handle_mouse_move(touch.location);
        let next_y = self.input.cursor_pos().y;
        let dy = next_y - prev_y;

        let prev_ndc = self
            .backend
            .stroke_to_ndc(width, height, self.zoom, self.stylus.point);
        let prev_stylus = self.backend.ndc_to_pixel(width, height, prev_ndc);

        self.update_stylus(width, height, touch);

        let next_ndc = self
            .backend
            .stroke_to_ndc(width, height, self.zoom, self.stylus.point);
        let next_stylus = self.backend.ndc_to_pixel(width, height, next_ndc);

        match (
            self.input.button_down(input::MouseButton::Middle),
            self.input.control(),
        ) {
            (true, false) => {
                self.move_origin(width, height, prev_stylus, next_stylus);
            }
            (true, true) => self.change_zoom(dy, width, height),
            _ => {}
        }
    }

    pub fn handle_cursor_move(&mut self, width: u32, height: u32, position: PixelPos) -> bool {
        log::trace!("handle cursor move {position:?}");
        let mut request_redraw = false;

        let prev = self.input.cursor_pos();
        self.input.handle_mouse_move(position);

        if self.input.button_down(input::MouseButton::Left) {
            let next = self.input.cursor_pos();
            self.move_origin(width, height, prev, next);
            request_redraw = true;
        }

        request_redraw
    }

    pub fn handle_mouse_button(&mut self, button: input::MouseButton, state: input::ElementState) {
        log::trace!("handle mouse button {button:?} {state:?}");
        self.input.handle_mouse_button(button, state);
    }

    pub fn increase_brush(&mut self) {
        self.brush_size += BRUSH_DELTA;
        self.brush_size = self.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);

        log::debug!("increase brush {}", self.brush_size);
    }

    pub fn decrease_brush(&mut self) {
        self.brush_size -= BRUSH_DELTA;
        self.brush_size = self.brush_size.clamp(MIN_BRUSH, MAX_BRUSH);

        log::debug!("decrease brush {}", self.brush_size);
    }

    fn move_origin(&mut self, width: u32, height: u32, prev: PixelPos, next: PixelPos) {
        use graphics::xform_point_to_pos as xform;

        let prev_xformed = xform(
            self.origin,
            self.backend.pixel_to_stroke(width, height, self.zoom, prev),
        );

        let next_xformed = xform(
            self.origin,
            self.backend.pixel_to_stroke(width, height, self.zoom, next),
        );

        let dx = next_xformed.x - prev_xformed.x;
        let dy = next_xformed.y - prev_xformed.y;
        self.origin.x += dx;
        self.origin.y += dy;

        self.update_stroke_visible(width, height);

        log::trace!("move origin {}", self.origin);
    }

    fn update_stroke_primitive(&mut self) {
        for stroke in self.strokes.iter_mut() {
            stroke.draw_tesselated = stroke.brush_size * self.zoom > 1.0;
        }
    }

    fn update_stroke_visible(&mut self, width: u32, height: u32) {
        for stroke in self.strokes.iter_mut() {
            stroke.update_visible(self.backend, self.origin, self.zoom, width, height);
        }
    }

    pub fn change_zoom(&mut self, dz: f32, width: u32, height: u32) {
        if (self.zoom + dz).is_finite() {
            self.zoom += dz;
        }

        self.zoom = self.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        self.update_stroke_primitive();
        self.update_stroke_visible(width, height);

        log::debug!("change zoom {}", self.zoom);
    }

    fn clear_strokes(&mut self) {
        log::debug!("clear strokes");
        self.modified = true;
        std::mem::take(&mut self.strokes);
    }

    pub fn undo_stroke(&mut self) {
        log::info!("undo stroke");
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

        let ndc_pos = self.backend.pixel_to_ndc(width, height, location);
        let point = self
            .backend
            .ndc_to_stroke(width, height, self.zoom, ndc_pos);
        let pos = graphics::xform_point_to_pos(self.origin, point);
        let pressure = force.unwrap_or(1.0);

        let eraser = pen_info
            .map(|info| info.inverted || info.eraser)
            .unwrap_or(self.stylus.state.eraser);

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
                    eraser,
                }
            }

            TouchPhase::Move => {
                if !self.gesture_state.active() && self.stylus.down() {
                    print!("\r{status}");
                }

                self.stylus.state.eraser = eraser;
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
                    eraser,
                }
            }
        };

        log::trace!(
            "update stylus {:?} {:?} {:?} {:?}",
            point,
            pos,
            pressure,
            state
        );

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
        log::trace!("handle update {phase:?}");

        if self.stylus.eraser() {
            let stylus_pix = self.backend.stroke_to_pixel(
                width,
                height,
                self.zoom,
                StrokePoint {
                    x: self.stylus.pos.x,
                    y: self.stylus.pos.y,
                },
            );

            let brush_size_stroke = self
                .backend
                .pixel_to_stroke(
                    width,
                    height,
                    self.zoom,
                    PixelPos {
                        x: self.brush_size as f32,
                        y: 0.,
                    },
                )
                .x;

            if phase == TouchPhase::Move && self.stylus.down() {
                for stroke in self.strokes.iter_mut() {
                    if stroke.erased() {
                        continue;
                    }

                    if !(stroke.top_left.x + brush_size_stroke <= self.stylus.pos.x
                        && self.stylus.pos.x <= stroke.bottom_right.x - brush_size_stroke
                        && stroke.bottom_right.y + brush_size_stroke <= self.stylus.pos.y
                        && self.stylus.pos.y <= stroke.top_left.y - brush_size_stroke)
                    {
                        continue;
                    }

                    'inner: for point in stroke.points().iter() {
                        let point_pix = self.backend.stroke_to_pixel(
                            width,
                            height,
                            self.zoom,
                            StrokePoint {
                                x: point.x,
                                y: point.y,
                            },
                        );

                        let dist = ((stylus_pix.x - point_pix.x).powi(2)
                            + (stylus_pix.y - point_pix.y).powi(2))
                        .sqrt()
                            * 2.0;

                        // TODO check geometry instead of stroke points
                        if dist < self.brush_size as f32 {
                            stroke.erase();
                            self.modified = true;
                            log::info!("erase stroke at {}", point_pix);
                            break 'inner;
                        }
                    }
                }
            }
        } else {
            match phase {
                TouchPhase::Start => {
                    self.modified = true;

                    let stroke_brush_size = self.backend.pixel_to_stroke(
                        width,
                        height,
                        self.zoom,
                        PixelPos {
                            x: ((width / 2) + self.brush_size as u32) as f32,
                            y: (height / 2) as f32,
                        },
                    );

                    self.strokes
                        .push(Stroke::new(rand::random(), stroke_brush_size.x / 2.));
                }

                TouchPhase::Move => {
                    if let Some(stroke) = self.strokes.last_mut() {
                        if self.stylus.down() {
                            stroke.add_point(&self.stylus);
                        }
                    }
                }

                TouchPhase::End | TouchPhase::Cancel => {
                    if let Some(stroke) = self.strokes.last_mut() {
                        stroke.finish();
                    }
                }
            };
        }
    }

    // returns whether to exit or overwrite state
    pub fn ask_to_save_then_save(&mut self, why: &str) -> Result<bool, PmbError> {
        log::info!("asking to save {why:?}");
        match (ui::ask_to_save(why), self.path.as_ref()) {
            // if they say yes and the file we're editing has a path
            (rfd::MessageDialogResult::Yes, Some(path)) => {
                log::info!("writing as {}", path.display());
                write(path, self).problem(format!("{}", path.display()))?;
                self.modified = false;
                Ok(true)
            }

            // they say yes and the file doesn't have a path yet
            (rfd::MessageDialogResult::Yes, None) => {
                log::info!("asking where to save");
                // ask where to save it
                match ui::save_dialog("Save unnamed file", None) {
                    Some(new_filename) => {
                        log::info!("writing as {}", new_filename.display());
                        // try write to disk
                        write(&new_filename, self)
                            .problem(format!("{}", new_filename.display()))?;
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

    pub fn read_file(&mut self, path: Option<impl AsRef<std::path::Path>>) -> Result<(), PmbError> {
        // if we are modified
        if self.modified {
            // ask to save first
            if !self
                .ask_to_save_then_save("Would you like to save before opening another file?")
                .problem(String::from("Could not save file"))?
            {
                return Ok(());
            }
        }

        // if we were passed a path, use that, otherwise ask for one
        log::info!("finding where to read from");
        let path = match path
            .map(|path| path.as_ref().to_path_buf())
            .or_else(ui::open_dialog)
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
                log::info!("using a new file");
                // if it doesn't exist don't try to read it
                *self = State::default();
                self.path = Some(path);
                self.modified = true;
                return Ok(());
            }
            Err(err) => Err(PmbError::from(err))?,
        };

        // read the new file
        let disk: Self = match read(file).problem(format!("{}", path.display())) {
            Ok(disk) => disk,

            Err(PmbError {
                kind: ErrorKind::VersionMismatch(version),
                ..
            }) => {
                log::warn!("version mismatch, got {version} want {}", Version::CURRENT);

                match Version::upgrade_type(version) {
                    UpgradeType::Smooth => migrate::from(version, &path)?,

                    UpgradeType::Rocky => match rfd::MessageDialog::new()
                        .set_title("Migrate version")
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .set_description("Significant internal changes have been made to Powdermilk Biscuits since you last opened this file. Although it has not been marked as significantly incompatible with the current version, you may still experience data loss by attempting to upgrade this file to the most recent version.\n\nNo changes will be made to the file as is, and you will be prompted to save the file in a new location instead of overwriting it.\n\nProceed?")
                        .show()
                    {
                        rfd::MessageDialogResult::Yes => {
                            let state = migrate::from(version, &path)?;
                            self.update_from(state);
                            self.modified = true;
                            self.path = None;

                            return Ok(());
                        },

                        _ => return Ok(()),
                    },

                    UpgradeType::Incompatible => {
                        return Err(PmbError::new(ErrorKind::IncompatibleVersion(version)));
                    }
                }
            }

            err => err?,
        };

        self.update_from(disk);
        self.modified = false;
        self.path = Some(path);

        log::info!(
            "success, read from {}",
            self.path.as_ref().unwrap().display()
        );

        Ok(())
    }

    pub fn save_file(&mut self) -> Result<(), PmbError> {
        if let Some(path) = self.path.as_ref() {
            write(path, self).problem(format!("{}", path.display()))?;
            self.modified = false;
        } else if let Some(path) = ui::save_dialog("Save unnamed file", None) {
            let problem = format!("{}", path.display());
            self.path = Some(path);
            write(self.path.as_ref().unwrap(), self).problem(problem)?;
            self.modified = false;
        }

        log::info!("saved file as {}", self.path.as_ref().unwrap().display());
        Ok(())
    }
}

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
