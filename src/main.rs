use {
    pixels::{Pixels, SurfaceTexture},
    std::ffi::CString,
    tablet_thing::{
        graphics::{self, ScreenPos},
        State, StrokePos, StrokeStyle,
    },
    winit::{
        dpi::PhysicalSize,
        event::{
            device::{GamepadHandle, HidId, KeyboardId, MouseId},
            ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
            WindowEvent,
        },
        event_loop::{ControlFlow, EventLoop},
        platform::windows::DeviceExtWindows,
        window::{Window, WindowBuilder},
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

    let mut cursor_down = false;
    let mut cursor_visible = true;
    let mut cursor_pos = Default::default();
    let mut state = State::default();
    println!("stroke style {:?}", state.stroke_style);

    let magic_point = StrokePos { x: 2.0, y: 4.0 };
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
                state.key(key, key_state);

                if state.just_pressed(Escape) {
                    *control_flow = ControlFlow::Exit;
                }

                if state.just_pressed(C) {
                    state.clear_strokes();
                    window.request_redraw();
                }

                if state.just_pressed(D) {
                    println!("{device_str}");
                }

                if state.just_pressed(F) {
                    state.fill_brush_head = !state.fill_brush_head;
                    window.request_redraw();
                }

                if state.control() && state.just_pressed(Z) {
                    state.undo_stroke();
                    window.request_redraw();
                }

                if state.just_pressed(Key1)
                    || state.just_pressed(Key2)
                    || state.just_pressed(Key3)
                    || state.just_pressed(Key4)
                    || state.just_pressed(Key5)
                    || state.just_pressed(Key6)
                    || state.just_pressed(Key7)
                    || state.just_pressed(Key8)
                    || state.just_pressed(Key9)
                    || state.just_pressed(Key0)
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

                if state.just_pressed(R) {
                    state.use_individual_style = !state.use_individual_style;
                    window.request_redraw();
                }

                if state.just_pressed(S) {
                    let num_string = std::fs::read_to_string("img/num.txt").expect("read num.txt");
                    let num = num_string.trim().parse::<usize>().expect("parse num.txt");
                    let filename = format!("img/strokes{num}.png");

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

                    image::save_buffer(&filename, frame, width, height, image::ColorType::Rgba8)
                        .expect(&format!("save {filename}"));

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
                match delta {
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_positive() => {
                        zoom += 1.;
                    }
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_positive() => {
                        zoom += 1.;
                    }
                    MouseScrollDelta::LineDelta(_, y) if y.is_sign_negative() => {
                        zoom -= 1.;
                    }
                    MouseScrollDelta::PixelDelta(pos) if pos.y.is_sign_negative() => {
                        zoom -= 1.;
                    }
                    _ => unreachable!(),
                };
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
            } if button == MouseButton::Left => match state {
                ElementState::Pressed => cursor_down = true,
                ElementState::Released => cursor_down = false,
            },

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let prev_pos = cursor_pos;
                cursor_pos = position;

                if cursor_down {
                    let prev = StrokePos::from_physical_position(prev_pos, zoom, screen_in_paper);
                    let next = StrokePos::from_physical_position(cursor_pos, zoom, screen_in_paper);
                    let diff = prev - next;
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

                graphics::put_pixel_absolute(
                    frame,
                    width,
                    height,
                    ScreenPos::from_stroke(magic_point, zoom, screen_in_paper),
                    [0xff, 0x00, 0xff],
                );

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
