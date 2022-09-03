use piet_common::{
    kurbo::{BezPath, Point},
    Color, ImageFormat, RenderContext, StrokeStyle,
};
use powdermilk_biscuits::{
    event::{ElementState, Event},
    ui::Ui,
    Config, CoordinateSystem, Device, Sketch,
};
use softbuffer::GraphicsContext;
use winit::{
    dpi::{LogicalPosition, PhysicalSize},
    event::{
        Event as WinitEvent, KeyboardInput, MouseScrollDelta, Touch, TouchPhase, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod backend {
    use powdermilk_biscuits::{
        event::{ElementState, Keycode, MouseButton, PenInfo, Touch, TouchPhase},
        graphics::{PixelPos, StrokePoint},
        CoordinateSystem, StrokeBackend,
    };
    use winit::event::{
        ElementState as WinitElementState, MouseButton as WinitMouseButton,
        PenInfo as WinitPenInfo, Touch as WinitTouch, TouchPhase as WinitTouchPhase,
        VirtualKeyCode as WinitKeycode,
    };

    #[derive(Debug, Default, Clone, Copy)]
    pub struct PietCoords;

    impl CoordinateSystem for PietCoords {
        type Ndc = PixelPos;

        fn pixel_to_ndc(&self, _width: u32, _height: u32, pos: PixelPos) -> Self::Ndc {
            pos
        }

        fn ndc_to_pixel(&self, _width: u32, _height: u32, pos: Self::Ndc) -> PixelPos {
            pos
        }

        fn ndc_to_stroke(
            &self,
            _width: u32,
            _height: u32,
            zoom: f32,
            ndc: Self::Ndc,
        ) -> StrokePoint {
            StrokePoint {
                x: (2. * ndc.x) / zoom,
                y: -((2. * ndc.y) / zoom),
            }
        }

        fn stroke_to_ndc(
            &self,
            _width: u32,
            _height: u32,
            zoom: f32,
            point: StrokePoint,
        ) -> Self::Ndc {
            PixelPos {
                x: point.x * zoom / 2.,
                y: -point.y * zoom / 2.,
            }
        }
    }

    #[derive(Debug)]
    pub struct PietStrokeBackend;
    impl StrokeBackend for PietStrokeBackend {
        fn is_dirty(&self) -> bool {
            false
        }

        fn make_dirty(&mut self) {}
    }

    pub fn winit_to_pmb_mouse_button(button: WinitMouseButton) -> MouseButton {
        match button {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            WinitMouseButton::Other(b) => MouseButton::Other(b as usize),
        }
    }

    pub fn physical_pos_to_pixel_pos(pos: winit::dpi::PhysicalPosition<f64>) -> PixelPos {
        PixelPos {
            x: pos.x as f32,
            y: pos.y as f32,
        }
    }

    pub fn winit_to_pmb_key_state(state: WinitElementState) -> ElementState {
        match state {
            WinitElementState::Pressed => ElementState::Pressed,
            WinitElementState::Released => ElementState::Released,
        }
    }

    pub fn winit_to_pmb_keycode(code: WinitKeycode) -> Keycode {
        macro_rules! codes {
            ($($code:ident),*) => {
                $(if code == WinitKeycode::$code {
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

    pub fn winit_to_pmb_touch_phase(phase: WinitTouchPhase) -> TouchPhase {
        match phase {
            WinitTouchPhase::Started => TouchPhase::Start,
            WinitTouchPhase::Moved => TouchPhase::Move,
            WinitTouchPhase::Ended => TouchPhase::End,
            WinitTouchPhase::Cancelled => TouchPhase::Cancel,
        }
    }

    pub fn winit_to_pmb_pen_info(pen_info: WinitPenInfo) -> PenInfo {
        PenInfo {
            barrel: pen_info.barrel,
            inverted: pen_info.inverted,
            eraser: pen_info.eraser,
        }
    }

    pub fn winit_to_pmb_touch(touch: WinitTouch) -> Touch {
        Touch {
            force: touch.force.map(|f| f.normalized()),
            phase: winit_to_pmb_touch_phase(touch.phase),
            location: physical_pos_to_pixel_pos(touch.location),
            pen_info: touch.pen_info.map(winit_to_pmb_pen_info),
        }
    }
}

fn main() {
    env_logger::init();
    if cfg!(debug_assertions) && cfg!(windows) {
        println!("\u{1b}[93mThe program was built in debug mode, which enables certain Direct2D debugging features that have a significantly negative impact on performance.\u{1b}[m");
    }

    let ev = EventLoop::new();
    let window = WindowBuilder::new()
        .with_position(LogicalPosition {
            x: 1920. / 2. - 800. / 2.,
            y: 1080. + 1080. / 2. - 600. / 2.,
        })
        .build(&ev)
        .unwrap();

    let mut gc = unsafe { GraphicsContext::new(window) }.unwrap();

    let mut size = gc.window().inner_size();
    let mut device = piet_common::Device::new().unwrap();

    let mut style = StrokeStyle::new();
    style.set_line_cap(piet_common::LineCap::Round);
    style.set_line_join(piet_common::LineJoin::Round);

    let coords = backend::PietCoords;

    let mut cursor_visible = true;
    let mut config = Config::default();
    let mut ui = Ui::<backend::PietCoords>::new(800, 600);
    let mut sketch: Sketch<backend::PietStrokeBackend> =
        if let Some(filename) = std::env::args().nth(1) {
            Sketch::with_filename(&mut ui, std::path::PathBuf::from(filename))
        } else {
            Sketch::default()
        };

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        match event {
            WinitEvent::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *flow = ControlFlow::Exit;
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } if Some(VirtualKeyCode::Escape) == input.virtual_keycode => {
                *flow = ControlFlow::Exit;
            }

            WinitEvent::WindowEvent {
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
                let key = backend::winit_to_pmb_keycode(key);
                let state = backend::winit_to_pmb_key_state(state);
                ui.handle_key(
                    &mut config,
                    &mut sketch,
                    key,
                    state,
                    size.width,
                    size.height,
                );
                gc.window().request_redraw();
            }

            WinitEvent::WindowEvent {
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

                gc.window().request_redraw();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let button = backend::winit_to_pmb_mouse_button(button);
                let state = backend::winit_to_pmb_key_state(state);

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
                gc.window().request_redraw();
            }

            WinitEvent::WindowEvent {
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
                        gc.window().set_cursor_visible(false);
                    }
                    gc.window().request_redraw();
                } else if !cursor_visible {
                    cursor_visible = true;
                    gc.window().set_cursor_visible(true);
                }

                if ui.state.redraw() {
                    gc.window().request_redraw();
                }
            }

            WinitEvent::WindowEvent {
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
                let touch = backend::winit_to_pmb_touch(touch);

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
                    gc.window().set_cursor_visible(false);
                }

                gc.window().request_redraw();
            }

            WinitEvent::WindowEvent {
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
                let touch = backend::winit_to_pmb_touch(touch);
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

                if cursor_visible && config.use_finger_for_pen {
                    cursor_visible = false;
                    gc.window().set_cursor_visible(false);
                }

                gc.window().request_redraw();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                if new_size.height < 10 || new_size.width < 10 {
                    gc.window().set_inner_size(size);
                } else {
                    size = new_size;
                    ui.resize(new_size.width, new_size.height, &mut sketch);
                    gc.window().request_redraw();
                }
            }

            WinitEvent::RedrawRequested(_) => {
                let PhysicalSize { width, height } = size;
                let mut target = device
                    .bitmap_target(width as usize, height as usize, 1.0)
                    .unwrap();

                {
                    let mut ctx = target.render_context();

                    for stroke in sketch.strokes.iter() {
                        if !stroke.visible || stroke.erased {
                            continue;
                        }

                        for pair in stroke.points.windows(2) {
                            if let [a, b] = pair {
                                let start = coords.pos_to_pixel(
                                    width,
                                    height,
                                    sketch.zoom,
                                    sketch.origin,
                                    a.into(),
                                );
                                let end = coords.pos_to_pixel(
                                    width,
                                    height,
                                    sketch.zoom,
                                    sketch.origin,
                                    b.into(),
                                );

                                let mut path = BezPath::new();
                                path.move_to(Point {
                                    x: start.x as f64,
                                    y: start.y as f64,
                                });
                                path.line_to(Point {
                                    x: end.x as f64,
                                    y: end.y as f64,
                                });

                                ctx.stroke_styled(
                                    path,
                                    &Color::rgba8(
                                        stroke.color[0],
                                        stroke.color[1],
                                        stroke.color[2],
                                        0xff,
                                    ),
                                    (stroke.brush_size * sketch.zoom * a.pressure) as f64,
                                    &style,
                                );
                            }
                        }
                    }

                    ctx.finish().unwrap();
                }

                gc.set_buffer(
                    bytemuck::cast_slice(
                        target
                            .to_image_buf(ImageFormat::RgbaPremul)
                            .unwrap()
                            .raw_pixels(),
                    ),
                    width as u16,
                    height as u16,
                );
            }

            _ => {}
        }
    });
}
