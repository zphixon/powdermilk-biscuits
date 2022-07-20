use std::mem::size_of;

use glow::{Context, HasContext};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use tablet_thing::{
    graphics::coords::{GlPos, StrokePos},
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

    let view_uniform;
    let proj_uniform;

    unsafe {
        gl.enable(glow::VERTEX_PROGRAM_POINT_SIZE);
        gl.disable(glow::CULL_FACE);

        let va = gl.create_vertex_array().expect("create vertex array");
        gl.bind_vertex_array(Some(va));
        let program = gl.create_program().expect("create program");
        let (vs_source, fs_source) = (
            include_str!("shaders/points.vert"),
            include_str!("shaders/points.frag"),
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
        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }

        gl.detach_shader(program, vs);
        gl.delete_shader(vs);
        gl.detach_shader(program, fs);
        gl.delete_shader(fs);

        gl.use_program(Some(program));

        proj_uniform = gl.get_uniform_location(program, "proj").unwrap();
        view_uniform = gl.get_uniform_location(program, "view").unwrap();

        gl.uniform_matrix_4_f32_slice(
            Some(&proj_uniform),
            false,
            &glam::Mat4::IDENTITY.to_cols_array(),
        );
        gl.uniform_matrix_4_f32_slice(
            Some(&view_uniform),
            false,
            &glam::Mat4::IDENTITY.to_cols_array(),
        );
    };

    let mut cursor_visible = true;
    let mut input_handler = InputHandler::default();

    let mut state = State::default();
    println!("stroke style {:?}", state.stroke_style);

    // gl origin in stroke space
    let mut gis = StrokePos { x: 0.0, y: 0.0 };
    let mut zoom = 50.;

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
                    for stroke in state.strokes.iter() {
                        println!("stroke");
                        for point in stroke.points.iter() {
                            println!("{}, {}, {}", point.pos.x, point.pos.y, point.pressure);
                        }
                    }
                    println!("zoom={zoom:.02}");
                    println!("gis={gis:?}");
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
                let PhysicalSize { width, height } = context.window().inner_size();
                state.update(gis, zoom, width, height, touch);
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
                const ZOOM_SPEED: f32 = 4.25;

                let dzoom = if zoom_in { ZOOM_SPEED } else { -ZOOM_SPEED };
                zoom += dzoom;
                zoom = zoom.clamp(1.0, 500.0);

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

                let PhysicalSize { width, height } = context.window().inner_size();
                //let gl = GlPos::from_pixel(width, height, prev);
                //let st = StrokePos::from_gl(gis, zoom, gl);
                //println!(
                //    "{},{} -> {:.02},{:.02} -> {:.02},{:.02}",
                //    prev.x, prev.y, gl.x, gl.y, st.x, st.y
                //);

                if input_handler.button_down(MouseButton::Left) {
                    let next = input_handler.cursor_pos();

                    let prev_gl = GlPos::from_pixel(width, height, prev);
                    let next_gl = GlPos::from_pixel(width, height, next);

                    let prev_stroke = StrokePos::from_gl(gis, zoom, prev_gl);
                    let next_stroke = StrokePos::from_gl(gis, zoom, next_gl);
                    let diff_stroke = next_stroke - prev_stroke;

                    gis = gis - diff_stroke;

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
                    //let points = state
                    //    .strokes
                    //    .iter()
                    //    .map(|stroke| {
                    //        stroke
                    //            .points
                    //            .iter()
                    //            .map(|point| [point.pos.x, point.pos.y, point.pressure])
                    //            .flatten()
                    //    })
                    //    .flatten()
                    //    .collect::<Vec<f32>>();

                    //let bytes = std::slice::from_raw_parts(
                    //    points.as_ptr() as *const u8,
                    //    points.len() * size_of::<f32>(),
                    //);

                    let points: &[f32] = &[-2., -2., 1., 0., 2., 1., 2., -2., 1.];
                    let bytes = std::slice::from_raw_parts(
                        points.as_ptr() as *const u8,
                        points.len() * size_of::<f32>(),
                    );

                    let vbo = gl.create_buffer().unwrap();
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);

                    let vao = gl.create_vertex_array().unwrap();
                    gl.bind_vertex_array(Some(vao));
                    gl.vertex_attrib_pointer_f32(
                        0,
                        2,
                        glow::FLOAT,
                        false,
                        size_of::<f32>() as i32 * 3,
                        0,
                    );
                    gl.enable_vertex_attrib_array(0);
                    gl.vertex_attrib_pointer_f32(
                        1,
                        1,
                        glow::FLOAT,
                        false,
                        size_of::<f32>() as i32 * 3,
                        size_of::<f32>() as i32 * 2,
                    );
                    gl.enable_vertex_attrib_array(1);

                    if state.stylus.inverted() {
                        gl.clear_color(1.0, 0.65, 0.65, 1.0);
                    } else {
                        gl.clear_color(0.65, 1.0, 0.75, 1.0);
                    }
                    gl.clear(glow::COLOR_BUFFER_BIT);

                    let PhysicalSize { width, height } = context.window().inner_size();
                    let (width, height) = (width as f32, height as f32);

                    let translate = GlPos::from_stroke(StrokePos { x: 0., y: 0. }, zoom, gis);
                    let view = glam::Mat4::from_scale_rotation_translation(
                        glam::vec3(zoom / width, zoom / height, 0.0),
                        glam::Quat::IDENTITY,
                        glam::vec3(-translate.x, -translate.y, 0.0),
                    );
                    let proj = glam::Mat4::IDENTITY;
                    // uhhhhehehhghhuhuhuh

                    gl.uniform_matrix_4_f32_slice(
                        Some(&view_uniform),
                        false,
                        &view.to_cols_array(),
                    );
                    gl.uniform_matrix_4_f32_slice(
                        Some(&proj_uniform),
                        false,
                        &proj.to_cols_array(),
                    );

                    gl.draw_arrays(glow::TRIANGLES, 0, 3);
                }
                context.swap_buffers().unwrap();
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                context.resize(size);
                unsafe {
                    gl.viewport(0, 0, size.width as i32, size.height as i32);
                };
            }

            _ => {}
        }
    });
}
