use glow::{Context, HasContext};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use tablet_thing::{
    graphics::{
        self,
        coords::{ScreenPos, StrokePos},
    },
    input::InputHandler,
    State, StrokeStyle,
};

#[allow(unreachable_code)]
fn main() {
    let (gl, context, ev) = {
        let event_loop = EventLoop::new();
        let builder = WindowBuilder::new().with_title("hi! <3");
        let context = unsafe {
            ContextBuilder::new()
                .with_vsync(true)
                .with_gl(glutin::GlRequest::Latest)
                .build_windowed(builder, &event_loop)
                .unwrap()
                .make_current()
                .unwrap()
        };
        let gl = unsafe {
            Context::from_loader_function(|name| context.get_proc_address(name) as *const _)
        };

        (gl, context, event_loop)
    };

    unsafe {
        let va = gl.create_vertex_array().expect("create vertex array");
        gl.bind_vertex_array(Some(va));
        let program = gl.create_program().expect("create program");
        let (vs_source, fs_source) = (
            include_str!("shaders/triangle.vert"),
            include_str!("shaders/triangle.frag"),
        );

        let vs = gl
            .create_shader(glow::VERTEX_SHADER)
            .expect("create shader");
        gl.shader_source(vs, vs_source);
        gl.compile_shader(vs);
        assert!(gl.get_shader_compile_status(vs));
        gl.attach_shader(program, vs);

        let fs = gl
            .create_shader(glow::FRAGMENT_SHADER)
            .expect("create shader");
        gl.shader_source(fs, fs_source);
        gl.compile_shader(fs);
        assert!(gl.get_shader_compile_status(fs));

        gl.attach_shader(program, fs);
        gl.link_program(program);
        assert!(gl.get_program_link_status(program));

        gl.detach_shader(program, vs);
        gl.delete_shader(vs);
        gl.detach_shader(program, fs);
        gl.delete_shader(fs);

        gl.use_program(Some(program));

        let screen_pos_uniform = gl.get_uniform_location(program, "screenPos").unwrap();
        let zoom_x_uniform = gl.get_uniform_location(program, "zoomX").unwrap();
        let zoom_y_uniform = gl.get_uniform_location(program, "zoomY").unwrap();
        gl.uniform_2_f32(Some(&screen_pos_uniform), 0.0, 0.0);
        gl.uniform_1_f32(Some(&zoom_x_uniform), 1.0);
        gl.uniform_1_f32(Some(&zoom_y_uniform), -1.0);

        gl.clear_color(0.1, 0.2, 0.3, 1.0);
    };

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
                    context.window().request_redraw();
                }

                if input_handler.just_pressed(D) {
                    println!("zoom={zoom:.02}");
                    println!("screen_in_paper={screen_in_paper:?}");
                }

                if input_handler.just_pressed(F) {
                    state.fill_brush_head = !state.fill_brush_head;
                    context.window().request_redraw();
                }

                if input_handler.control() && input_handler.just_pressed(Z) {
                    state.undo_stroke();
                    context.window().request_redraw();
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
                    context.window().request_redraw();

                    println!("stroke style {:?}", state.stroke_style);
                }

                if input_handler.just_pressed(R) {
                    state.use_individual_style = !state.use_individual_style;
                    context.window().request_redraw();
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
                        // let PhysicalSize { width, height } = window.inner_size();
                    }

                    let next_num = num + 1;
                    std::fs::write("img/num.txt", format!("{next_num}")).expect("write num.txt");
                    println!("wrote image as {filename}");

                    context.window().request_redraw();
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
                context.window().set_cursor_visible(cursor_visible);
                state.update(touch, zoom, screen_in_paper);
                context.window().request_redraw();
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

                let PhysicalSize { width, height } = context.window().inner_size();
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

                context.window().request_redraw();
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
                context.window().request_redraw();
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
                    context.window().request_redraw();
                }

                if !cursor_visible {
                    cursor_visible = true;
                    context.window().set_cursor_visible(cursor_visible);
                    context.window().request_redraw();
                }
            }

            Event::RedrawRequested(_) => {
                unsafe {
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    gl.draw_arrays(glow::TRIANGLES, 0, 3);
                }
                context.swap_buffers().unwrap();
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                context.resize(size);
            }

            _ => {}
        }
    });
}
