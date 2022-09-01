use crate::{
    error::{ErrorKind, PmbError, PmbErrorExt},
    event::{ElementState, Event, InputHandler, Keycode, Touch, TouchPhase},
    graphics::{PixelPos, StrokePos},
    Backend, Config, Sketch, Stroke, StrokeBackend, Stylus, StylusPosition, StylusState, Tool,
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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum UiState {
    #[default]
    Ready,
    Pan,
    PreZoom,
    PenZoom,
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

impl UiState {
    pub fn redraw(&self) -> bool {
        use UiState::*;
        !matches!(self, Ready | OpenDialog | SaveDialog)
    }
}

#[derive(Debug)]
pub struct Ui<B: Backend> {
    pub state: UiState,
    pub stylus: Stylus,
    pub brush_size: usize,
    pub modified: bool,
    pub path: Option<std::path::PathBuf>,
    pub input: InputHandler,
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

    pub fn resize<S: StrokeBackend>(&mut self, width: u32, height: u32, sketch: &mut Sketch<S>) {
        self.width = width;
        self.height = height;
        self.update_visible_strokes(sketch);
        sketch.update_stroke_primitive();
    }

    fn start_stroke<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
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
            .push(Stroke::new(rand::random(), stroke_brush_size, true));
    }

    fn continue_stroke<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
        let stroke = sketch.strokes.last_mut().unwrap();
        stroke.add_point(&self.stylus);
    }

    fn end_stroke<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
        let stroke = sketch.strokes.last_mut().unwrap();
        stroke.finish();
    }

    fn update_stylus_from_mouse<S: StrokeBackend>(
        &mut self,
        config: &Config,
        sketch: &Sketch<S>,
        phase: TouchPhase,
    ) {
        let eraser = config.active_tool == Tool::Eraser;
        let pressure = if self.input.button_down(config.primary_button) {
            1.0
        } else {
            0.0
        };

        self.update_stylus(sketch, phase, self.input.cursor_pos(), eraser, pressure);
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

        let pressure = force.unwrap_or(1.0);

        let eraser = pen_info
            .map(|info| info.inverted || info.eraser)
            .unwrap_or(config.active_tool == Tool::Eraser);

        self.update_stylus(sketch, phase, location, eraser, pressure);
    }

    fn update_stylus<S: StrokeBackend>(
        &mut self,
        sketch: &Sketch<S>,
        phase: TouchPhase,
        location: PixelPos,
        eraser: bool,
        pressure: f64,
    ) {
        let point = self
            .backend
            .pixel_to_stroke(self.width, self.height, sketch.zoom, location);
        let pos = crate::graphics::xform_point_to_pos(sketch.origin, point);

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

    fn increase_brush(&mut self, by: usize) {
        self.brush_size += by;
        self.brush_size = self.brush_size.clamp(crate::MIN_BRUSH, crate::MAX_BRUSH);

        log::debug!("increase brush {}", self.brush_size);
    }

    fn decrease_brush(&mut self, by: usize) {
        self.brush_size -= by;
        self.brush_size = self.brush_size.clamp(crate::MIN_BRUSH, crate::MAX_BRUSH);

        log::debug!("decrease brush {}", self.brush_size);
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
            (S::Ready, E::ToolChange) => {
                match config.active_tool {
                    Tool::Pen => {
                        self.stylus.state.eraser = false;
                    }
                    Tool::Eraser => {
                        self.stylus.state.eraser = true;
                    }
                }
                S::Ready
            }

            (S::Ready, E::IncreaseBrush(change)) => {
                self.increase_brush(change);
                S::Ready
            }

            (S::Ready, E::DecreaseBrush(change)) => {
                self.decrease_brush(change);
                S::Ready
            }

            (S::Ready, E::ScrollZoom(change)) => {
                let next_zoom = sketch.zoom + change;
                sketch.zoom = if next_zoom < crate::MIN_ZOOM {
                    crate::MIN_ZOOM
                } else if next_zoom > crate::MAX_ZOOM {
                    crate::MAX_ZOOM
                } else {
                    next_zoom
                };

                if config.use_mouse_for_pen {
                    self.update_stylus_from_mouse(config, sketch, TouchPhase::Move);
                }

                sketch.update_stroke_primitive();
                self.update_visible_strokes(sketch);

                S::Ready
            }

            // pan handling
            (S::Ready, E::StartPan) => S::Pan,
            (S::PenZoom, E::EndZoom) => S::Pan,
            (S::Pan, E::EndPan) => S::Ready,

            (S::Ready, E::MouseDown(button)) => {
                self.input
                    .handle_mouse_button(button, ElementState::Pressed);
                if config.use_mouse_for_pen {
                    self.update_stylus_from_mouse(config, sketch, TouchPhase::Start);
                    match config.active_tool {
                        Tool::Pen => {
                            self.start_stroke(sketch);
                            S::MouseDraw
                        }
                        Tool::Eraser => S::MouseErase,
                    }
                } else {
                    S::Pan
                }
            }

            (S::Pan, E::MouseUp(button)) => {
                self.input
                    .handle_mouse_button(button, ElementState::Released);
                S::Ready
            }

            (S::Pan, E::PenMove(touch)) => {
                let prev = crate::graphics::xform_point_to_pos(sketch.origin, self.stylus.point);
                self.update_stylus_from_touch(config, sketch, touch);
                let next = crate::graphics::xform_point_to_pos(sketch.origin, self.stylus.point);
                self.move_origin(sketch, prev, next);
                S::Pan
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

                if config.use_mouse_for_pen {
                    self.update_stylus_from_mouse(config, sketch, TouchPhase::Move);
                }

                S::Pan
            }

            (S::Pan, E::TouchMove(touch)) => {
                let prev = self.backend.pixel_to_pos(
                    self.width,
                    self.height,
                    sketch.zoom,
                    sketch.origin,
                    touch.location,
                );

                self.input.handle_mouse_move(touch.location);

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

            // zoom handling
            (S::PenZoom, E::EndPan) => S::PreZoom,
            (S::Pan, E::StartZoom) => S::PenZoom,
            (S::PreZoom, E::StartPan) => S::PenZoom,
            (S::Ready, E::StartZoom) => S::PreZoom,
            (S::PreZoom, E::EndZoom) => S::Ready,

            (S::PenZoom, E::PenMove(touch)) => {
                let prev = self.stylus.pos;
                self.update_stylus_from_touch(config, sketch, touch);
                let next = self.stylus.pos;
                sketch.zoom += prev.y - next.y;
                sketch.update_stroke_primitive();
                self.update_visible_strokes(sketch);
                S::PenZoom
            }

            // pen draw/erase
            (S::Ready, E::PenMove(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                S::Ready
            }

            (S::Ready, E::PenDown(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                match config.active_tool {
                    Tool::Pen => {
                        self.start_stroke(sketch);
                        S::PenDraw
                    }
                    Tool::Eraser => S::PenErase,
                }
            }

            (S::PenDraw, E::PenMove(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                self.continue_stroke(sketch);
                S::PenDraw
            }

            (S::PenDraw, E::PenUp(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                self.end_stroke(sketch);
                S::Ready
            }

            (S::PenErase, E::PenMove(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                S::PenErase
            }

            (S::PenErase, E::PenUp(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                S::Ready
            }

            // mouse input
            (S::Ready, E::MouseMove(location)) => {
                self.input.handle_mouse_move(location);

                if config.use_mouse_for_pen {
                    self.update_stylus_from_mouse(config, sketch, TouchPhase::End);
                }

                S::Ready
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

            (S::MouseErase, E::MouseMove(location)) => {
                self.input.handle_mouse_move(location);
                self.update_stylus_from_mouse(config, sketch, TouchPhase::Move);
                S::MouseErase
            }

            (S::MouseErase, E::MouseUp(button)) => {
                self.input
                    .handle_mouse_button(button, ElementState::Released);
                self.update_stylus_from_mouse(config, sketch, TouchPhase::End);
                S::Ready
            }

            // TODO: touch input, pan & zoom
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

    pub fn handle_key<S: StrokeBackend>(
        &mut self,
        config: &Config,
        sketch: &mut Sketch<S>,
        key: Keycode,
        state: ElementState,
        _width: u32,
        _height: u32,
    ) {
        log::debug!("handle key {key:?} {state:?}");

        self.input.handle_key(key, state);

        macro_rules! just_pressed {
            ($action:ident) => {
                just_pressed!($action, false, false)
            };

            (ctrl + $action:ident) => {
                just_pressed!($action, true, false)
            };

            (shift + $action:ident) => {
                just_pressed!($action, false, true)
            };

            (ctrl + shift + $action:ident) => {
                just_pressed!($action, true, true)
            };

            ($action:ident, $ctrl:expr, $shift:expr) => {
                self.input.just_pressed(config.$action)
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

        if just_pressed!(brush_increase) {
            self.next(config, sketch, Event::IncreaseBrush(crate::BRUSH_DELTA));
        }

        if just_pressed!(brush_decrease) {
            self.next(config, sketch, Event::DecreaseBrush(crate::BRUSH_DELTA));
        }

        if just_pressed!(clear_strokes) {
            // TODO
            //sketch.clear_strokes();
        }

        if just_pressed!(debug_strokes) {
            for stroke in sketch.strokes.iter() {
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
            println!("zoom={:.02}", sketch.zoom);
            println!("origin={}", sketch.origin);
        }

        if just_pressed!(ctrl + undo) {
            // TODO
            //self.undo_stroke();
        }

        if just_pressed!(ctrl + save) {
            // TODO
            //self.save_file()
            //    .problem(format!("Could not save file"))
            //    .display();
        }

        if just_pressed!(reset_view) {
            // TODO
            //self.reset_view(width, height);
        }

        if just_pressed!(ctrl + open) {
            // TODO
            //self.read_file(Option::<&str>::None)
            //    .problem(format!("Could not open file"))
            //    .display();
        }

        if just_pressed!(ctrl + zoom_out) {
            // TODO
            //sketch.change_zoom(-4.25, width, height);
        }

        if just_pressed!(ctrl + zoom_in) {
            // TODO
            //self.change_zoom(4.25, width, height);
        }

        if just_pressed!(pen_zoom) && config.prev_device == crate::Device::Pen {
            self.next(config, sketch, Event::StartZoom);
        }

        if !self.input.is_down(config.pen_zoom) {
            self.next(config, sketch, Event::EndZoom);
        }

        if just_pressed!(toggle_eraser_pen)
            && (config.prev_device == crate::Device::Mouse || !config.stylus_may_be_inverted)
        {
            // TODO move active_tool to Ui
            //self.next(config, sketch, Event::ToolChange);
        }

        self.input.upstrokes();
    }

    pub fn read_file<S: StrokeBackend>(
        &mut self,
        path: Option<impl AsRef<std::path::Path>>,
        sketch: &mut Sketch<S>,
    ) -> Result<(), PmbError> {
        use crate::{
            migrate,
            migrate::{UpgradeType, Version},
        };

        // if we are modified
        if self.modified {
            // ask to save first
            if !self
                .ask_to_save_then_save(
                    sketch,
                    "Would you like to save before opening another file?",
                )
                .problem(String::from("Could not save file"))?
            {
                return Ok(());
            }
        }

        // if we were passed a path, use that, otherwise ask for one
        log::info!("finding where to read from");
        let path = match path
            .map(|path| path.as_ref().to_path_buf())
            .or_else(open_dialog)
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
                self.path = Some(path);
                self.modified = true;
                return Ok(());
            }
            Err(err) => Err(PmbError::from(err))?,
        };

        // read the new file
        let disk: Sketch<S> = match crate::read(file).problem(format!("{}", path.display())) {
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
                            sketch.update_from(state);
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

        sketch.update_from(disk);
        self.modified = false;
        self.path = Some(path);

        log::info!(
            "success, read from {}",
            self.path.as_ref().unwrap().display()
        );

        Ok(())
    }

    pub fn ask_to_save_then_save<S: StrokeBackend>(
        &mut self,
        sketch: &Sketch<S>,
        why: &str,
    ) -> Result<bool, PmbError> {
        log::info!("asking to save {why:?}");
        match (ask_to_save(why), self.path.as_ref()) {
            // if they say yes and the file we're editing has a path
            (rfd::MessageDialogResult::Yes, Some(path)) => {
                log::info!("writing as {}", path.display());
                crate::write(path, sketch).problem(format!("{}", path.display()))?;
                self.modified = false;
                Ok(true)
            }

            // they say yes and the file doesn't have a path yet
            (rfd::MessageDialogResult::Yes, None) => {
                log::info!("asking where to save");
                // ask where to save it
                match save_dialog("Save unnamed file", None) {
                    Some(new_filename) => {
                        log::info!("writing as {}", new_filename.display());
                        // try write to disk
                        crate::write(&new_filename, sketch)
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
}
