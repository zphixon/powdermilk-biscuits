use femtovg::{renderer::OpenGl, Canvas, Color, LineCap, LineJoin, Paint, Path};
use glutin::{
    event::{
        Event as GlutinEvent, KeyboardInput, MouseScrollDelta, Touch, TouchPhase, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use powdermilk_biscuits::{
    event::{ElementState, Event},
    ui::Ui,
    Config, Device, Sketch, Tool,
};

mod backend {
    use glutin::event::{
        ElementState as GlutinElementState, MouseButton as GlutinMouseButton,
        PenInfo as GlutinPenInfo, Touch as GlutinTouch, TouchPhase as GlutinTouchPhase,
        VirtualKeyCode as GlutinKeycode,
    };
    use powdermilk_biscuits::{
        event::{ElementState, Keycode, MouseButton, PenInfo, Touch, TouchPhase},
        graphics::{PixelPos, StrokePoint},
        CoordinateSystem, StrokeBackend,
    };

    #[derive(Debug, Default, Clone, Copy)]
    pub struct FemtovgCoords;

    impl CoordinateSystem for FemtovgCoords {
        type Ndc = PixelPos;

        fn pixel_to_ndc(&self, _width: u32, _height: u32, pos: PixelPos) -> Self::Ndc {
            pos
        }

        fn ndc_to_pixel(&self, _width: u32, _height: u32, pos: Self::Ndc) -> PixelPos {
            pos
        }

        fn ndc_to_stroke(&self, width: u32, height: u32, zoom: f32, ndc: Self::Ndc) -> StrokePoint {
            StrokePoint {
                x: (2. * (ndc.x - width as f32 / 2.)) / zoom,
                y: -((2. * (ndc.y - height as f32 / 2.)) / zoom),
            }
        }

        fn stroke_to_ndc(
            &self,
            width: u32,
            height: u32,
            zoom: f32,
            point: StrokePoint,
        ) -> Self::Ndc {
            PixelPos {
                x: (point.x * zoom / 2.) + width as f32 / 2.,
                y: (-point.y * zoom / 2.) + height as f32 / 2.,
            }
        }
    }

    #[derive(Debug)]
    pub struct FemtovgStrokeBackend;
    impl StrokeBackend for FemtovgStrokeBackend {
        fn is_dirty(&self) -> bool {
            false
        }

        fn make_dirty(&mut self) {}
    }

    pub fn glutin_to_pmb_mouse_button(button: GlutinMouseButton) -> MouseButton {
        match button {
            GlutinMouseButton::Left => MouseButton::Left,
            GlutinMouseButton::Right => MouseButton::Right,
            GlutinMouseButton::Middle => MouseButton::Middle,
            GlutinMouseButton::Other(b) => MouseButton::Other(b as usize),
        }
    }

    pub fn physical_pos_to_pixel_pos(pos: glutin::dpi::PhysicalPosition<f64>) -> PixelPos {
        PixelPos {
            x: pos.x as f32,
            y: pos.y as f32,
        }
    }

    pub fn glutin_to_pmb_key_state(state: GlutinElementState) -> ElementState {
        match state {
            GlutinElementState::Pressed => ElementState::Pressed,
            GlutinElementState::Released => ElementState::Released,
        }
    }

    pub fn glutin_to_pmb_keycode(code: GlutinKeycode) -> Keycode {
        macro_rules! codes {
            ($($code:ident),*) => {
                $(if code == GlutinKeycode::$code {
                    return Keycode::$code;
                })*
            };
        }

        #[rustfmt::skip]
        codes!(
            Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, Key0, A, B, C, D, E, F, G, H, I, J,
            K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Escape, F1, F2, F3, F4, F5, F6, F7, F8, F9,
            F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24, Snapshot,
            Scroll, Pause, Insert, Home, Delete, End, PageDown, PageUp, Left, Up, Right, Down, Back,
            Return, Space, Compose, Caret, Numlock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
            Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadDivide, NumpadDecimal,
            NumpadComma, NumpadEnter, NumpadEquals, NumpadMultiply, NumpadSubtract, AbntC1, AbntC2,
            Apostrophe, Apps, Asterisk, At, Ax, Backslash, Calculator, Capital, Colon, Comma, Convert,
            Equals, Grave, Kana, Kanji, LAlt, LBracket, LControl, LShift, LWin, Mail, MediaSelect,
            MediaStop, Minus, Mute, MyComputer, NavigateForward, NavigateBackward, NextTrack,
            NoConvert, OEM102, Period, PlayPause, Plus, Power, PrevTrack, RAlt, RBracket, RControl,
            RShift, RWin, Semicolon, Slash, Sleep, Stop, Sysrq, Tab, Underline, Unlabeled, VolumeDown,
            VolumeUp, Wake, WebBack, WebFavorites, WebForward, WebHome, WebRefresh, WebSearch, WebStop,
            Yen, Copy, Paste, Cut
        );

        panic!("unmatched keycode: {code:?}");
    }

    pub fn glutin_to_pmb_touch_phase(phase: GlutinTouchPhase) -> TouchPhase {
        match phase {
            GlutinTouchPhase::Started => TouchPhase::Start,
            GlutinTouchPhase::Moved => TouchPhase::Move,
            GlutinTouchPhase::Ended => TouchPhase::End,
            GlutinTouchPhase::Cancelled => TouchPhase::Cancel,
        }
    }

    pub fn glutin_to_pmb_pen_info(pen_info: GlutinPenInfo) -> PenInfo {
        PenInfo {
            barrel: pen_info.barrel,
            inverted: pen_info.inverted,
            eraser: pen_info.eraser,
        }
    }

    pub fn glutin_to_pmb_touch(touch: GlutinTouch) -> Touch {
        Touch {
            force: touch.force.map(|f| f.normalized()),
            phase: glutin_to_pmb_touch_phase(touch.phase),
            location: physical_pos_to_pixel_pos(touch.location),
            pen_info: touch.pen_info.map(glutin_to_pmb_pen_info),
        }
    }
}

fn main() {
    env_logger::init();

    let ev = EventLoop::new();
    let builder = WindowBuilder::new().with_position(glutin::dpi::LogicalPosition {
        x: 1920. / 2. - 800. / 2.,
        y: 1080. + 1080. / 2. - 600. / 2.,
    });

    let context = unsafe {
        ContextBuilder::new()
            .with_vsync(true)
            .with_gl(glutin::GlRequest::Latest)
            .with_multisampling(4)
            .build_windowed(builder, &ev)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let mut size = context.window().inner_size();
    let renderer = OpenGl::new_from_glutin_context(&context).unwrap();
    let mut canvas = Canvas::new(renderer).unwrap();
    canvas.set_size(
        size.width,
        size.height,
        context.window().scale_factor() as f32,
    );

    let coords = backend::FemtovgCoords;
    let mut cursor_visible = true;
    let mut config = Config::default();
    let mut ui = Ui::<backend::FemtovgCoords>::new(size.width, size.height);
    let mut sketch: Sketch<backend::FemtovgStrokeBackend> =
        if let Some(filename) = std::env::args().nth(1) {
            Sketch::with_filename(&mut ui, std::path::PathBuf::from(filename))
        } else {
            Sketch::default()
        };

    context.window().request_redraw();
    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        match event {
            GlutinEvent::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested,
                ..
            } => {
                *flow = ControlFlow::Exit;
            }

            GlutinEvent::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(key),
                                state,
                                ..
                            },
                        ..
                    },
                ..
            } => {
                let key = backend::glutin_to_pmb_keycode(key);
                let state = backend::glutin_to_pmb_key_state(state);
                ui.handle_key(
                    &mut config,
                    &mut sketch,
                    key,
                    state,
                    size.width,
                    size.height,
                );
                context.window().request_redraw();
            }

            GlutinEvent::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, delta) => {
                        ui.next(&config, &mut sketch, Event::ScrollZoom(delta));
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        ui.next(&config, &mut sketch, Event::ScrollZoom(delta.y as f32));
                    }
                }

                context.window().request_redraw();
            }

            GlutinEvent::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let button = backend::glutin_to_pmb_mouse_button(button);
                let state = backend::glutin_to_pmb_key_state(state);

                match (button, state) {
                    (primary, ElementState::Pressed) if primary == config.primary_button => {
                        ui.next(&config, &mut sketch, Event::MouseDown(button));
                    }
                    (primary, ElementState::Released) if primary == config.primary_button => {
                        ui.next(&config, &mut sketch, Event::MouseUp(button));
                    }
                    (pan, ElementState::Pressed) if pan == config.pan_button => {
                        ui.next(&config, &mut sketch, Event::StartPan);
                    }
                    (pan, ElementState::Released) if pan == config.pan_button => {
                        ui.next(&config, &mut sketch, Event::EndPan);
                    }
                    _ => {}
                }

                ui.prev_device = Device::Mouse;
                context.window().request_redraw();
            }

            GlutinEvent::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                ui.next(
                    &config,
                    &mut sketch,
                    Event::MouseMove(backend::physical_pos_to_pixel_pos(position)),
                );
                ui.prev_device = Device::Mouse;

                if config.use_mouse_for_pen {
                    if cursor_visible {
                        cursor_visible = false;
                        context.window().set_cursor_visible(false);
                    }
                    context.window().request_redraw();
                } else if !cursor_visible {
                    cursor_visible = true;
                    context.window().set_cursor_visible(true);
                }

                if ui.state.redraw() {
                    context.window().request_redraw();
                }
            }

            GlutinEvent::WindowEvent {
                event:
                    WindowEvent::Touch(
                        touch @ Touch {
                            phase,
                            pen_info: Some(_),
                            ..
                        },
                    ),
                ..
            } => {
                let touch = backend::glutin_to_pmb_touch(touch);

                match phase {
                    TouchPhase::Started => ui.next(&config, &mut sketch, Event::PenDown(touch)),
                    TouchPhase::Moved => ui.next(&config, &mut sketch, Event::PenMove(touch)),
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        ui.next(&config, &mut sketch, Event::PenUp(touch))
                    }
                }

                ui.prev_device = Device::Pen;

                if cursor_visible {
                    cursor_visible = false;
                    context.window().set_cursor_visible(false);
                }

                context.window().request_redraw();
            }

            GlutinEvent::WindowEvent {
                event:
                    WindowEvent::Touch(
                        touch @ Touch {
                            phase,
                            pen_info: None,
                            ..
                        },
                    ),
                ..
            } => {
                let touch = backend::glutin_to_pmb_touch(touch);
                ui.next(
                    &config,
                    &mut sketch,
                    match phase {
                        TouchPhase::Started => Event::Touch(touch),
                        TouchPhase::Moved => Event::TouchMove(touch),
                        TouchPhase::Ended | TouchPhase::Cancelled => Event::Release(touch),
                    },
                );

                ui.prev_device = Device::Touch;

                if cursor_visible {
                    cursor_visible = false;
                    context.window().set_cursor_visible(false);
                }

                context.window().request_redraw();
            }

            GlutinEvent::RedrawRequested(_) => {
                use powdermilk_biscuits::CoordinateSystem;
                canvas.clear_rect(0, 0, size.width, size.height, Color::black());

                // TODO
                // canvas.set_transform(1., 0., 0., 1., sketch.origin.x, sketch.origin.y);
                // canvas.get_image on stroke backend

                for stroke in sketch.visible_strokes() {
                    let mut paint = Paint::color(Color {
                        r: stroke.color[0] as f32 / 255.,
                        g: stroke.color[1] as f32 / 255.,
                        b: stroke.color[2] as f32 / 255.,
                        a: 1.0,
                    })
                    .with_line_cap(LineCap::Round)
                    .with_line_join(LineJoin::Round);

                    for pair in stroke.points.windows(2) {
                        if let [a, b] = pair {
                            let start = coords.pos_to_pixel(
                                size.width,
                                size.height,
                                sketch.zoom,
                                sketch.origin,
                                a.into(),
                            );
                            let end = coords.pos_to_pixel(
                                size.width,
                                size.height,
                                sketch.zoom,
                                sketch.origin,
                                b.into(),
                            );

                            let pressure = (a.pressure + b.pressure) / 2.;
                            let brush_size = (stroke.brush_size * sketch.zoom * pressure).max(1.0);

                            let mut path = Path::new();
                            paint.set_line_width(brush_size);
                            path.move_to(start.x, start.y);
                            path.line_to(end.x, end.y);
                            canvas.stroke_path(&mut path, paint);
                        }
                    }
                }

                if !cursor_visible {
                    let pos = coords.stroke_to_pixel(
                        size.width,
                        size.height,
                        sketch.zoom,
                        ui.stylus.point,
                    );

                    let color = match (ui.active_tool == Tool::Eraser, ui.stylus.down()) {
                        (true, true) => Color::rgbf(0.980, 0.200, 0.203),
                        (true, false) => Color::rgbf(0.325, 0.067, 0.067),
                        (false, true) => Color::white(),
                        (false, false) => Color::rgbf(0.333, 0.333, 0.333),
                    };

                    let mut circle = Path::new();
                    circle.circle(pos.x, pos.y, ui.brush_size as f32 / 2.);
                    canvas.stroke_path(&mut circle, Paint::color(color));
                }

                canvas.flush();
                context.swap_buffers().unwrap();
            }

            GlutinEvent::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                size = new_size;
                canvas.set_size(
                    size.width,
                    size.height,
                    context.window().scale_factor() as f32,
                );
                ui.resize(size.width, size.height, &mut sketch);
            }

            GlutinEvent::WindowEvent {
                event:
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    },
                ..
            } => {
                size = *new_inner_size;
                canvas.set_size(size.width, size.height, scale_factor as f32);
            }

            _ => {}
        }
    });
}
