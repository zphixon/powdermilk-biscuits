use crate::{
    config::Config,
    error::PmbErrorExt,
    event::{ElementState, Event, InputHandler, Keycode, Touch, TouchPhase},
    graphics::{PixelPos, StrokePos},
    s,
    ui::undo::{Action, UndoStack},
    CoordinateSystem, Device, Sketch, Stroke, StrokeBackend, Stylus, StylusPosition, StylusState,
    Tool,
};
use lyon::{
    lyon_tessellation::{StrokeOptions, StrokeTessellator},
    path::{LineCap, LineJoin},
};
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SketchWidgetState {
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
    Gesture(u8),
    OpenDialog,
    SaveDialog,
}

impl SketchWidgetState {
    pub fn redraw(&self) -> bool {
        use SketchWidgetState::*;
        !matches!(self, Ready | OpenDialog | SaveDialog)
    }
}

pub struct SketchWidget<C: CoordinateSystem> {
    pub state: SketchWidgetState,
    pub modified: bool,
    pub path: Option<std::path::PathBuf>,

    pub input: InputHandler,
    pub prev_device: Device,

    pub stylus: Stylus,
    pub brush_size: usize,
    pub active_tool: Tool,
    pub undo_stack: UndoStack,
    pub color: [f32; 3],

    pub width: u32,
    pub height: u32,
    pub tesselator: StrokeTessellator,
    pub stroke_options: StrokeOptions,

    coords: PhantomData<C>,
}

impl<C: CoordinateSystem> SketchWidget<C> {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            state: SketchWidgetState::default(),
            stylus: Stylus::default(),
            prev_device: Device::Mouse,
            active_tool: Tool::Pen,
            undo_stack: UndoStack::new(),
            brush_size: crate::DEFAULT_BRUSH,
            modified: false,
            path: None,
            input: InputHandler::default(),
            width,
            height,
            tesselator: StrokeTessellator::new(),
            stroke_options: StrokeOptions::default()
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round)
                .with_tolerance(0.001)
                .with_variable_line_width(0),
            color: [1., 1., 1.],
            coords: Default::default(),
        }
    }

    pub fn resize<S: StrokeBackend>(&mut self, width: u32, height: u32, sketch: &mut Sketch<S>) {
        self.width = width;
        self.height = height;
        sketch.update_visible_strokes::<C>(self.width, self.height);
        sketch.update_stroke_primitive();
    }

    pub fn force_update<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
        sketch.force_update::<C>(
            self.width,
            self.height,
            &mut self.tesselator,
            &self.stroke_options,
        );
    }

    fn start_stroke<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
        self.modified = true;
        let stroke_brush_size = self.brush_size as f32 / sketch.zoom;
        let key = sketch
            .strokes
            .insert(Stroke::new(self.color, stroke_brush_size, true));
        self.undo_stack.push(Action::DrawStroke(key));
    }

    fn continue_stroke<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
        if let Action::DrawStroke(key) = self.undo_stack.last().expect("empty undo stack") {
            let stroke = sketch.strokes.get_mut(key).unwrap();
            stroke.add_point(&self.stylus, &mut self.tesselator, &self.stroke_options);
        } else {
            panic!("last action not draw stroke in continue stroke");
        }
    }

    fn end_stroke<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
        if let Action::DrawStroke(key) = self.undo_stack.last().expect("empty undo stack") {
            let stroke = sketch.strokes.get_mut(key).unwrap();
            stroke.finish();
        } else {
            panic!("last action not draw stroke in end stroke");
        }
    }

    fn erase_strokes<S: StrokeBackend>(&mut self, sketch: &mut Sketch<S>) {
        let stylus_pos_pix = C::pos_to_pixel(
            self.width,
            self.height,
            sketch.zoom,
            sketch.origin,
            self.stylus.pos,
        );

        let top_left_cursor = C::pixel_to_pos(
            self.width,
            self.height,
            sketch.zoom,
            sketch.origin,
            PixelPos {
                x: stylus_pos_pix.x - self.brush_size as f32 / 2.,
                y: stylus_pos_pix.y - self.brush_size as f32 / 2.,
            },
        );

        let bottom_right_cursor = C::pixel_to_pos(
            self.width,
            self.height,
            sketch.zoom,
            sketch.origin,
            PixelPos {
                x: stylus_pos_pix.x + self.brush_size as f32 / 2.,
                y: stylus_pos_pix.y + self.brush_size as f32 / 2.,
            },
        );

        sketch
            .strokes
            .iter_mut()
            .filter(|(_, stroke)| {
                stroke.visible
                    && !stroke.erased
                    && stroke.aabb(top_left_cursor, bottom_right_cursor)
            })
            .for_each(|(key, stroke)| {
                // TODO lyon_path::builder::Flattened?
                if stroke.mesh.vertices.iter().any(|point| {
                    let point_pix = C::pos_to_pixel(
                        self.width,
                        self.height,
                        sketch.zoom,
                        sketch.origin,
                        StrokePos {
                            x: point.x,
                            y: point.y,
                        },
                    );

                    ((stylus_pos_pix.x - point_pix.x).powi(2)
                        + (stylus_pos_pix.y - point_pix.y).powi(2))
                    .sqrt()
                        <= self.brush_size as f32
                }) {
                    stroke.erase();
                    self.undo_stack.push(Action::EraseStroke(key));
                    self.modified = true;
                }
            });
    }

    fn update_stylus_from_mouse<S: StrokeBackend>(
        &mut self,
        config: &Config,
        sketch: &Sketch<S>,
        phase: TouchPhase,
    ) {
        let eraser = self.active_tool == Tool::Eraser;
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
        sketch: &mut Sketch<S>,
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

        if let Some(pen_info) = pen_info {
            if config.stylus_may_be_inverted {
                if pen_info.inverted || pen_info.eraser {
                    self.active_tool = Tool::Eraser;
                } else {
                    self.active_tool = Tool::Pen;
                }
            }
        }

        let eraser = pen_info
            .map(|info| info.inverted || info.eraser)
            .unwrap_or(self.active_tool == Tool::Eraser);

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
        let point = C::pixel_to_stroke(self.width, self.height, sketch.zoom, location);
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
        use SketchWidgetState as S;

        self.state = match (self.state, event) {
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
                sketch.update_zoom::<C>(self.width, self.height, next_zoom);

                if config.use_mouse_for_pen {
                    self.update_stylus_from_mouse(config, sketch, TouchPhase::Move);
                }

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
                    match self.active_tool {
                        Tool::Pen => {
                            self.start_stroke(sketch);
                            S::MouseDraw
                        }
                        Tool::Eraser => S::MouseErase,
                        Tool::Pan => S::Pan,
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
                sketch.move_origin::<C>(self.width, self.height, prev, next);
                S::Pan
            }

            (S::Pan, E::MouseMove(location)) => {
                let prev = C::pixel_to_pos(
                    self.width,
                    self.height,
                    sketch.zoom,
                    sketch.origin,
                    self.input.cursor_pos(),
                );

                self.input.handle_mouse_move(location);

                let next = C::pixel_to_pos(
                    self.width,
                    self.height,
                    sketch.zoom,
                    sketch.origin,
                    self.input.cursor_pos(),
                );

                sketch.move_origin::<C>(self.width, self.height, prev, next);

                if config.use_mouse_for_pen {
                    self.update_stylus_from_mouse(config, sketch, TouchPhase::Move);
                }

                S::Pan
            }

            // zoom handling
            (S::PenZoom, E::EndPan) => S::PreZoom,
            (S::Pan, E::StartZoom) => S::PenZoom,
            (S::PreZoom, E::StartPan) => S::PenZoom,
            (S::Ready, E::StartZoom) => S::PreZoom,
            (S::PreZoom, E::EndZoom) => S::Ready,

            (S::PreZoom, E::PenMove(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                S::PreZoom
            }

            (S::PenZoom, E::PenMove(touch)) => {
                let prev = self.stylus.pos;
                self.update_stylus_from_touch(config, sketch, touch);
                let next = self.stylus.pos;

                let next_zoom = sketch.zoom + (prev.y - next.y);
                sketch.update_zoom::<C>(self.width, self.height, next_zoom);

                S::PenZoom
            }

            // pen draw/erase
            (S::Ready, E::PenMove(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                S::Ready
            }

            (S::Ready, E::PenDown(touch)) => {
                self.update_stylus_from_touch(config, sketch, touch);
                match self.active_tool {
                    Tool::Pen => {
                        self.start_stroke(sketch);
                        S::PenDraw
                    }
                    Tool::Eraser => S::PenErase,
                    Tool::Pan => S::Pan,
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
                self.erase_strokes(sketch);
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
                self.erase_strokes(sketch);
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
                let tool = config.tool_for_gesture(1);
                self.active_tool = tool;
                match self.active_tool {
                    Tool::Pen => {
                        self.update_stylus_from_touch(config, sketch, touch);
                        self.start_stroke(sketch);
                    }
                    _ => {
                        // TODO
                        self.input.handle_mouse_move(touch.location);
                    }
                }

                S::Gesture(1)
            }

            (S::Gesture(i), E::Touch(touch)) => {
                // TODO dedup, more movement tolerance for gesture state transition
                let tool = config.tool_for_gesture(i + 1);
                self.active_tool = tool;
                match self.active_tool {
                    Tool::Pen => {
                        self.update_stylus_from_touch(config, sketch, touch);
                        self.start_stroke(sketch);
                    }
                    _ => {
                        // TODO
                        self.input.handle_mouse_move(touch.location);
                    }
                }

                S::Gesture(i + 1)
            }

            (S::Gesture(i), E::TouchMove(touch)) => {
                let tool = config.tool_for_gesture(i);
                self.active_tool = tool;

                match tool {
                    Tool::Pen => {
                        // TODO dedup, logic???
                        self.update_stylus_from_touch(config, sketch, touch);
                        self.continue_stroke(sketch);
                    }

                    Tool::Eraser => {
                        self.update_stylus_from_touch(config, sketch, touch);
                        self.erase_strokes(sketch);
                    }

                    Tool::Pan => {
                        let prev = C::pixel_to_pos(
                            self.width,
                            self.height,
                            sketch.zoom,
                            sketch.origin,
                            self.input.cursor_pos(),
                        );

                        self.input.handle_mouse_move(touch.location);

                        let next = C::pixel_to_pos(
                            self.width,
                            self.height,
                            sketch.zoom,
                            sketch.origin,
                            self.input.cursor_pos(),
                        );

                        sketch.move_origin::<C>(self.width, self.height, prev, next);
                    }
                }

                S::Gesture(i)
            }

            (S::Gesture(i), E::Release(_)) => {
                #[allow(clippy::single_match)]
                match self.active_tool {
                    Tool::Pen => {
                        self.end_stroke(sketch);
                    }

                    // TODO
                    _ => {}
                }

                if i == 1 {
                    S::Ready
                } else {
                    S::Gesture(i - 1)
                }
            }

            (any, _) => any,
        };
    }

    // TODO move this to InputHandler?
    pub fn handle_key<S: StrokeBackend>(
        &mut self,
        config: &mut Config,
        sketch: &mut Sketch<S>,
        key: Keycode,
        state: ElementState,
        width: u32,
        height: u32,
    ) {
        log::debug!("handle key {key:?} {state:?}");
        self.input.handle_key(key, state);

        if self.input.combo_just_pressed(&config.brush_increase) {
            self.next(config, sketch, Event::IncreaseBrush(crate::BRUSH_DELTA));
        }

        if self.input.combo_just_pressed(&config.brush_decrease) {
            self.next(config, sketch, Event::DecreaseBrush(crate::BRUSH_DELTA));
        }

        if self.input.combo_just_pressed(&config.debug_clear_strokes) {
            sketch.clear_strokes();
            self.undo_stack.clear();
            self.modified = true;
        }

        if self.input.combo_just_pressed(&config.debug_print_strokes)
            && !self
                .input
                .combo_just_pressed(&config.debug_dirty_all_strokes)
        {
            for stroke in sketch.strokes.values() {
                println!("stroke");
                for point in stroke.points().iter() {
                    println!("{},{},{}", point.x, point.y, point.pressure);
                }
                println!(
                    "{} points, {} vertices, {} size, {} visible, {:?} color, {} top left, {} bottom right",
                    stroke.points().len(),
                    stroke.mesh.vertices.len(),
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
            println!("undo_stack={:?}", self.undo_stack);
        }

        if self.input.combo_just_pressed(&config.undo) {
            match self.undo_stack.undo() {
                Some(Action::DrawStroke(stroke)) => sketch.strokes[stroke].erase(),
                Some(Action::EraseStroke(stroke)) => {
                    sketch.strokes[stroke].erased = false;
                    sketch.update_visible_strokes::<C>(width, height);
                }
                None => {}
            }

            self.modified = !self.undo_stack.at_saved_state();
        }

        if self.input.combo_just_pressed(&config.redo) {
            match self.undo_stack.redo() {
                Some(Action::DrawStroke(stroke)) => {
                    sketch.strokes[stroke].erased = false;
                    sketch.update_visible_strokes::<C>(width, height);
                }
                Some(Action::EraseStroke(stroke)) => sketch.strokes[stroke].erase(),
                None => {}
            }

            self.modified = !self.undo_stack.at_saved_state();
        }

        if self.input.combo_just_pressed(&config.save) {
            super::save_file(self, sketch)
                .problem(s!(CouldNotSaveFile))
                .map(|_| {
                    self.undo_stack.set_saved_state();
                })
                .display();
        }

        if self.input.combo_just_pressed(&config.reset_view) {
            sketch.update_zoom::<C>(self.width, self.height, crate::DEFAULT_ZOOM);
            sketch.move_origin::<C>(
                self.width,
                self.height,
                StrokePos {
                    x: sketch.origin.x,
                    y: sketch.origin.y,
                },
                Default::default(),
            );
        }

        if self.input.combo_just_pressed(&config.open) {
            super::read_file(self, None::<&str>, sketch)
                .problem(s!(CouldNotOpenFile))
                .display();
        }

        if self.input.combo_just_pressed(&config.zoom_out) {
            sketch.update_zoom::<C>(self.width, self.height, sketch.zoom - 4.25);
        }

        if self.input.combo_just_pressed(&config.zoom_in) {
            sketch.update_zoom::<C>(self.width, self.height, sketch.zoom + 4.25);
        }

        if self.input.just_pressed(config.pen_zoom) && self.prev_device == crate::Device::Pen {
            self.next(config, sketch, Event::StartZoom);
        }

        if !self.input.is_down(config.pen_zoom) {
            self.next(config, sketch, Event::EndZoom);
        }

        if self.input.combo_just_pressed(&config.toggle_eraser_pen)
            && (self.prev_device == crate::Device::Mouse || !config.stylus_may_be_inverted)
        {
            if self.active_tool == Tool::Eraser {
                // TODO use previous tool?
                self.active_tool = Tool::Pen;
            } else {
                self.active_tool = Tool::Eraser;
            }
        }

        if self
            .input
            .combo_just_pressed(&config.debug_toggle_use_mouse_for_pen)
        {
            config.use_mouse_for_pen = !config.use_mouse_for_pen;
            println!("using mouse for pen? {}", config.use_mouse_for_pen);
        }

        if self
            .input
            .combo_just_pressed(&config.debug_toggle_use_finger_for_pen)
        {
            if config.tool_for_gesture_1 != Tool::Pen {
                config.tool_for_gesture_1 = Tool::Pen;
            } else {
                config.tool_for_gesture_1 = Tool::Pan;
            }
            println!("tool for gesture 1: {:?}", config.tool_for_gesture_1);
        }

        if self
            .input
            .combo_just_pressed(&config.debug_toggle_stylus_invertability)
        {
            config.stylus_may_be_inverted = !config.stylus_may_be_inverted;
            println!("stylus invertable? {}", config.stylus_may_be_inverted);
        }

        if self
            .input
            .combo_just_pressed(&config.debug_dirty_all_strokes)
        {
            log::info!("debug dirty all strokes");
            self.force_update(sketch);
        }

        if self.input.combo_just_pressed(&Keycode::L.into()) {
            use crate::i18n::*;
            if get_lang() == "es" {
                set_lang("en");
            } else {
                set_lang("es-MX");
            }
        }

        self.input.pump_key_state();
    }
}
