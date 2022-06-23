use {
    glutin::{
        dpi::PhysicalSize,
        event::{
            device::{GamepadHandle, HidId, KeyboardId, MouseId},
            Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
        },
        event_loop::{ControlFlow, EventLoop},
        platform::windows::DeviceExtWindows,
        window::{Window, WindowBuilder},
    },
    pixels::{Pixels, SurfaceTexture},
    std::ffi::CString,
    tablet_thing::{
        graphics::{
            self,
            coords::{ScreenPos, StrokePos},
        },
        input::InputHandler,
        State, StrokeStyle,
    },
};

fn print_human_info(identifier: &str) -> String {
    let identifier_cstr = CString::new(&identifier[..identifier.len() - 1]).expect("cstr");
    let api = hidapi::HidApi::new().unwrap();
    let device = api.open_path(&identifier_cstr).expect("open_path");
    let get_product_string = device.get_product_string();
    let get_manufacturer_string = device.get_manufacturer_string();
    let get_serial_number_string = device.get_serial_number_string();
    format!("product: {get_product_string:?}\nmanufacturer: {get_manufacturer_string:?}\nserial number: {get_serial_number_string:?}\n")
}

fn enumerate_devices<T>(ev: &EventLoop<T>) -> String {
    let mut devices = String::new();
    HidId::enumerate(ev).for_each(|id| {
        let identifier = id.persistent_identifier().unwrap();
        devices += &format!("{id:?} {identifier:?}\n");
        devices += &print_human_info(&identifier);
        devices += "\n";
    });
    KeyboardId::enumerate(ev).for_each(|id| {
        let identifier = id.persistent_identifier().unwrap();
        devices += &format!("{id:?} {identifier:?}\n");
        devices += &print_human_info(&identifier);
        devices += "\n";
    });
    MouseId::enumerate(ev).for_each(|id| {
        let identifier = id.persistent_identifier().unwrap();
        devices += &format!("{id:?} {identifier:?}\n");
        devices += &print_human_info(&identifier);
        devices += "\n";
    });
    GamepadHandle::enumerate(ev).for_each(|id| {
        let identifier = id.persistent_identifier().unwrap();
        devices += &format!("{id:?} {identifier:?}\n");
        devices += &print_human_info(&identifier);
        devices += "\n";
    });
    devices
}

fn new_pixels(window: &Window) -> Pixels {
    let size = window.inner_size();
    let tex = SurfaceTexture::new(size.width, size.height, &window);
    Pixels::new(size.width, size.height, tex).unwrap()
}

#[allow(unreachable_code)]
fn main() {
    let ev = EventLoop::new();
    let window = WindowBuilder::new().build(&ev).unwrap();
    let device_str = enumerate_devices(&ev);

    let mut pixels = new_pixels(&window);

    let mut cursor_visible = true;
    let mut input_handler = InputHandler::default();

    let mut state = State::default();
    println!("stroke style {:?}", state.stroke_style);

    let mut screen_in_paper = StrokePos { x: -2.0, y: 5.33 };
    let mut zoom = 150.;

    ev.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput(KeyboardInput {
                        virtual_keycode: Some(key),
                        state: key_state,
                        ..
                    }),
                ..
            } => {
                use VirtualKeyCode::*;
                input_handler.handle_key(key, key_state);

                if input_handler.just_pressed(Escape) {
                    *control_flow = ControlFlow::Exit;
                }

                if input_handler.just_pressed(C) {
                    state.clear_strokes();
                    window.request_redraw();
                }

                if input_handler.just_pressed(D) {
                    println!("{device_str}");
                    println!("zoom={zoom:.02}");
                    println!("screen_in_paper={screen_in_paper:?}");
                }

                if input_handler.just_pressed(F) {
                    state.fill_brush_head = !state.fill_brush_head;
                    window.request_redraw();
                }

                if input_handler.control() && input_handler.just_pressed(Z) {
                    state.undo_stroke();
                    window.request_redraw();
                }

                if input_handler.just_pressed(Key1)
                    || input_handler.just_pressed(Key2)
                    || input_handler.just_pressed(Key3)
                    || input_handler.just_pressed(Key4)
                    || input_handler.just_pressed(Key5)
                    || input_handler.just_pressed(Key6)
                    || input_handler.just_pressed(Key7)
                    || input_handler.just_pressed(Key8)
                    || input_handler.just_pressed(Key9)
                    || input_handler.just_pressed(Key0)
                {
                    state.stroke_style = unsafe {
                        std::mem::transmute(
                            match key {
                                Key1 => 0,
                                Key2 => 1,
                                Key3 => 2,
                                Key4 => 3,
                                Key5 => 4,
                                Key6 => 5,
                                Key7 => 6,
                                Key8 => 7,
                                Key9 => 8,
                                Key0 => 9,
                                _ => unreachable!(),
                            } % StrokeStyle::NUM_VARIANTS,
                        )
                    };
                    window.request_redraw();

                    println!("stroke style {:?}", state.stroke_style);
                }

                if input_handler.just_pressed(R) {
                    state.use_individual_style = !state.use_individual_style;
                    window.request_redraw();
                }

                if input_handler.just_pressed(S) {
                    let num_string = std::fs::read_to_string("img/num.txt").expect("read num.txt");
                    let num = num_string.trim().parse::<usize>().expect("parse num.txt");
                    let filename = format!("img/strokes{num}.png");

                    // when we render with a real graphics library, we'll compute the geometry of
                    // each stroke and just render it like a normal person. when we want the full
                    // overview like what we're trying to do here we'll render into an image
                    // target, mapping each sample so that the far bounds of the stroke space
                    // correspond to 1/-1.

                    if input_handler.shift() {
                        let mut min_x = f64::INFINITY;
                        let mut max_x = -f64::INFINITY;
                        let mut min_y = f64::INFINITY;
                        let mut max_y = -f64::INFINITY;
                        let mut max_rad = -f64::INFINITY;
                        for stroke in state.strokes.iter() {
                            if stroke.style == StrokeStyle::Circles && stroke.brush_size > max_rad {
                                max_rad = stroke.brush_size;
                            }

                            for point in stroke.points.iter() {
                                if point.pos.x > max_x {
                                    max_x = point.pos.x;
                                }
                                if point.pos.x < min_x {
                                    min_x = point.pos.x;
                                }
                                if point.pos.y > max_y {
                                    max_y = point.pos.y;
                                }
                                if point.pos.y < min_y {
                                    min_y = point.pos.y;
                                }
                            }
                        }

                        let margin = 20. + max_rad;

                        let top_left_stroke = StrokePos { x: min_x, y: max_y };
                        let bottom_right_stroke = StrokePos { x: max_x, y: min_y };
                        let bottom_right_screen =
                            ScreenPos::from_stroke(bottom_right_stroke, 150., top_left_stroke);
                        let width = bottom_right_screen.x + 2 * margin as isize;
                        let height = bottom_right_screen.y + 2 * margin as isize;
                        let diff = bottom_right_stroke - top_left_stroke;
                        let zoom_overview = width as f64 / diff.x;
                        let width = width.try_into().unwrap();
                        let height = height.try_into().unwrap();

                        let image = image::RgbaImage::new(width, height);

                        let mut container = image.into_raw();

                        graphics::clear(container.as_mut_slice());
                        state.draw_strokes(
                            container.as_mut_slice(),
                            width as usize,
                            height as usize,
                            zoom_overview,
                            top_left_stroke,
                        );

                        let image = image::RgbaImage::from_raw(width, height, container)
                            .expect("image from raw");
                        image.save(&filename).expect(&format!("save {filename}"));
                    } else {
                        let PhysicalSize { width, height } = window.inner_size();
                        let frame = pixels.get_frame();
                        graphics::clear(frame);
                        state.draw_strokes(
                            frame,
                            width as usize,
                            height as usize,
                            zoom,
                            screen_in_paper,
                        );

                        image::save_buffer(
                            &filename,
                            frame,
                            width,
                            height,
                            image::ColorType::Rgba8,
                        )
                        .expect(&format!("save {filename}"));
                    }

                    let next_num = num + 1;
                    std::fs::write("img/num.txt", format!("{next_num}")).expect("write num.txt");
                    println!("wrote image as {filename}");

                    window.request_redraw();
                }
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::Touch(touch),
                ..
            } => {
                cursor_visible = false;
                window.set_cursor_visible(cursor_visible);
                state.update(touch, zoom, screen_in_paper);
                window.request_redraw();
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
                const ZOOM_SPEED: f64 = 3.;

                let PhysicalSize { width, height } = window.inner_size();
                let dzoom = if zoom_in { ZOOM_SPEED } else { -ZOOM_SPEED };
                let dscreen_in_paper = if zoom_in {
                    let x = (width as f64 / 2.) / zoom;
                    let y = -(height as f64 / 2.) / zoom;
                    StrokePos { x, y }
                } else {
                    let x = -(width as f64 / 2.) / zoom;
                    let y = (height as f64 / 2.) / zoom;
                    StrokePos { x, y }
                };

                zoom += dzoom;
                let next_sip = screen_in_paper + (dscreen_in_paper * (1. / zoom));
                if next_sip.x.is_finite() && next_sip.y.is_finite() {
                    screen_in_paper = next_sip;
                }

                window.request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter(c @ ('[' | ']')),
                ..
            } => {
                match c {
                    '[' => state.decrease_brush(),
                    ']' => state.increase_brush(),
                    _ => unreachable!(),
                };
                window.request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                input_handler.handle_mouse_button(button, state);
            }

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let prev = input_handler.cursor_pos();
                input_handler.handle_mouse_move(position);

                if input_handler.button_down(MouseButton::Left) {
                    let next = input_handler.cursor_pos();
                    let diff = StrokePos::from_screen_pos(prev, zoom, screen_in_paper)
                        - StrokePos::from_screen_pos(next, zoom, screen_in_paper);
                    screen_in_paper = screen_in_paper + diff;
                    window.request_redraw();
                }

                if !cursor_visible {
                    cursor_visible = true;
                    window.set_cursor_visible(cursor_visible);
                    window.request_redraw();
                }
            }

            Event::RedrawRequested(_) => {
                let frame = pixels.get_frame();
                graphics::clear(frame);

                let PhysicalSize { width, height } = window.inner_size();
                let (width, height) = (width as usize, height as usize);

                state.draw_strokes(frame, width, height, zoom, screen_in_paper);

                if !cursor_visible {
                    graphics::put_circle_absolute(
                        frame,
                        width,
                        height,
                        ScreenPos::from_stroke(state.stylus.pos, zoom, screen_in_paper),
                        match (state.stylus.inverted(), state.stylus.down()) {
                            (true, true) => [0xfa, 0x34, 0x33],
                            (true, false) => [0x53, 0x11, 0x11],
                            (false, true) => [0xff, 0xff, 0xff],
                            (false, false) => [0x55, 0x55, 0x55],
                        },
                        state.brush_size,
                    );
                }

                pixels.render().unwrap();
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                pixels = new_pixels(&window);
                window.request_redraw();
            }

            _ => {}
        }
    });
}
