use piet_common::{
    kurbo::{BezPath, Point},
    Color, ImageFormat, RenderContext, StrokeStyle,
};
use powdermilk_biscuits::{graphics::PixelPos, Backend, State};
use softbuffer::GraphicsContext;
use winit::{
    dpi::{LogicalPosition, PhysicalSize},
    event::{Event, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod backend {
    use powdermilk_biscuits::{
        graphics::{PixelPos, StrokePoint},
        input::{ElementState, Keycode, MouseButton},
        Backend, StrokeBackend,
    };
    use winit::event::{
        ElementState as WinitElementState, MouseButton as WinitMouseButton,
        VirtualKeyCode as WinitKeycode,
    };

    #[derive(Debug, Default, Clone, Copy)]
    pub struct PietBackend;

    impl Backend for PietBackend {
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
    pub struct PietStrokeBackend {
        pub image: piet_common::ImageBuf,
        pub dirty: bool,
    }

    impl StrokeBackend for PietStrokeBackend {
        fn is_dirty(&self) -> bool {
            self.dirty
        }

        fn make_dirty(&mut self) {
            self.dirty = true;
        }
    }

    pub fn winit_to_pmb_key_state(state: WinitElementState) -> ElementState {
        match state {
            WinitElementState::Pressed => ElementState::Pressed,
            WinitElementState::Released => ElementState::Released,
        }
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
}

fn main() {
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
    let mut screen_buffer = vec![0u8; size.width as usize * size.height as usize];

    let mut style = StrokeStyle::new();
    style.set_line_cap(piet_common::LineCap::Round);
    style.set_line_join(piet_common::LineJoin::Round);

    let backend = backend::PietBackend;
    let mut state: State<backend::PietBackend, backend::PietStrokeBackend> =
        if let Some(filename) = std::env::args().nth(1) {
            State::with_filename(std::path::PathBuf::from(filename))
        } else {
            State::default()
        };
    state.reset_view(size.width, size.height);

    let corner = PixelPos {
        x: size.width as f32 / 2.,
        y: size.height as f32 / 2.,
    };
    state.set_origin(
        size.width,
        size.height,
        backend.pixel_to_stroke(size.width, size.height, state.zoom, corner),
    );

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } if Some(VirtualKeyCode::Escape) == input.virtual_keycode => {
                *flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            winit::event::KeyboardInput {
                                state: key_state,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                let key = backend::winit_to_pmb_keycode(key);
                let key_state = backend::winit_to_pmb_key_state(key_state);
                if state.handle_key(key, key_state, size.width, size.height) {
                    gc.window().request_redraw();
                }
            }

            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: key_state,
                        button,
                        ..
                    },
                ..
            } => {
                let button = backend::winit_to_pmb_mouse_button(button);
                let key_state = backend::winit_to_pmb_key_state(key_state);
                state.handle_mouse_button(button, key_state);
            }

            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                let zoom_in = match delta {
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_positive() => true,
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_positive() => true,
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_negative() => false,
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_negative() => false,
                    _ => unreachable!(),
                };
                const ZOOM_SPEED: f32 = 4.25;

                let dzoom = if zoom_in { ZOOM_SPEED } else { -ZOOM_SPEED };
                state.change_zoom(dzoom, size.width, size.height);
                gc.window().request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                if state.handle_cursor_move(
                    size.width,
                    size.height,
                    backend::physical_pos_to_pixel_pos(position),
                ) {
                    gc.window().request_redraw();
                }
            }

            Event::WindowEvent {
                event: WindowEvent::Touch(touch),
                ..
            } => {
                let PhysicalSize { width, height } = size;
                let pixel_pos = PixelPos {
                    x: touch.location.x as f32,
                    y: touch.location.y as f32,
                };

                let ndc = backend.pixel_to_ndc(width, height, pixel_pos);
                let stroke_point = backend.pixel_to_stroke(width, height, state.zoom, pixel_pos);
                let stroke_pos =
                    backend.pixel_to_pos(width, height, state.zoom, state.origin, pixel_pos);
                print!(
                    "p={} n={} i={} o={}\r",
                    pixel_pos, ndc, stroke_point, stroke_pos
                );
                use std::io::Write;
                std::io::stdout().flush().unwrap();

                // TODO
                gc.window().request_redraw();
            }

            Event::RedrawRequested(_) => {
                let PhysicalSize { width, height } = size;

                for stroke in state.strokes.iter_mut().filter(|stroke| stroke.is_dirty()) {
                    let mut target = device
                        .bitmap_target(width as usize, height as usize, 1.0)
                        .unwrap();

                    {
                        let mut ctx = target.render_context();
                        let mut path = BezPath::new();

                        let first: Option<&powdermilk_biscuits::stroke::StrokeElement> =
                            stroke.points.first();
                        if let Some(first) = first {
                            let first = backend.pos_to_pixel(
                                width,
                                height,
                                state.zoom,
                                state.origin,
                                first.into(),
                            );

                            path.move_to(Point {
                                x: first.x as f64,
                                y: first.y as f64,
                            });
                        }

                        for point in stroke.points.iter() {
                            let point = backend.pos_to_pixel(
                                width,
                                height,
                                state.zoom,
                                state.origin,
                                point.into(),
                            );
                            path.line_to(Point {
                                x: point.x as f64,
                                y: point.y as f64,
                            });
                        }

                        ctx.stroke_styled(
                            path,
                            &Color::rgba8(stroke.color[0], stroke.color[1], stroke.color[2], 0xff),
                            (stroke.brush_size * state.zoom) as f64,
                            &style,
                        );

                        ctx.finish().unwrap();
                    }

                    stroke.replace_backend_with(|_points, _mesh, _mesh_len| {
                        backend::PietStrokeBackend {
                            dirty: false,
                            image: target.to_image_buf(ImageFormat::RgbaPremul).unwrap(),
                        }
                    });
                }

                'strokes: for stroke in state.strokes.iter() {
                    assert!(!stroke.is_dirty());
                    let top_left = backend.pos_to_pixel(
                        width,
                        height,
                        state.zoom,
                        state.origin,
                        stroke.top_left,
                    );
                    let top = top_left.y as usize;
                    let left = top_left.x as usize;

                    let image = &stroke.backend().unwrap().image;
                    for row in image.raw_pixels().chunks(image.width()) {
                        let start = top * image.width() + left;
                        let end = start + row.len();

                        if start <= end && end - start < row.len() && end < screen_buffer.len() {
                            screen_buffer[start..end].copy_from_slice(row)
                        } else {
                            println!("sad");
                            continue 'strokes;
                        }
                    }
                }

                gc.set_buffer(
                    bytemuck::cast_slice(&screen_buffer),
                    width as u16,
                    height as u16,
                );
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                if new_size.height < 10 || new_size.width < 10 {
                    gc.window().set_inner_size(size);
                } else {
                    size = new_size;
                    screen_buffer = vec![0; size.width as usize * size.height as usize];
                    state.change_zoom(0.0, size.width, size.height);
                    gc.window().request_redraw();
                }
            }

            _ => {}
        }
    });
}
