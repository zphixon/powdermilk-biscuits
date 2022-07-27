use powdermilk_biscuits::{ui, State};
use wgpu::SurfaceError;
use winit::{
    dpi::{LogicalPosition, PhysicalSize},
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    env_logger::init();
    futures::executor::block_on(run());
}

async fn run() {
    let ev = EventLoop::new();
    let window = WindowBuilder::new()
        .with_position(LogicalPosition {
            x: 1920. / 2. - 800. / 2.,
            y: 1080. + 1080. / 2. - 600. / 2.,
        })
        .with_title(powdermilk_biscuits::TITLE_UNMODIFIED)
        .build(&ev)
        .unwrap();

    let mut state: State<pmb_wgpu::WgpuBackend, pmb_wgpu::StrokeBackend> = if let Some(filename) =
        std::env::args()
            .nth(1)
            .map(|filename| std::path::PathBuf::from(filename))
    {
        State::with_filename(filename)
    } else {
        State::default()
    };

    let mut graphics = pmb_wgpu::Graphics::new(&window).await;
    graphics.buffer_all_strokes(&mut state);

    let mut input = pmb_wgpu::InputHandler::default();
    let mut cursor_visible = true;

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if state.modified {
                    if state
                        .ask_to_save_then_save("Would you like to save before exiting?")
                        .unwrap_or(false)
                    {
                        *flow = ControlFlow::Exit;
                    }
                } else {
                    *flow = ControlFlow::Exit;
                }
            }

            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                *flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event:
                    WindowEvent::Resized(new_size)
                    | WindowEvent::ScaleFactorChanged {
                        new_inner_size: &mut new_size,
                        ..
                    },
                ..
            } => {
                graphics.resize(new_size);
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
                state.change_zoom(dzoom);

                window.request_redraw();
            }

            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                input.handle_mouse_button(button, state);
            }

            Event::RedrawRequested(_) => {
                match graphics.render(&mut state, window.inner_size(), cursor_visible) {
                    Err(SurfaceError::Lost) => graphics.resize(graphics.size),
                    Err(SurfaceError::OutOfMemory) => {
                        ui::error("Out of memory!");
                        *flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }

            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                input.handle_key(key, state);
            }

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let prev = input.cursor_pos();
                input.handle_mouse_move(position);

                if input.button_down(MouseButton::Left) {
                    let next = input.cursor_pos();
                    let PhysicalSize { width, height } = window.inner_size();
                    state.move_origin(
                        width,
                        height,
                        pmb_wgpu::physical_pos_to_pixel_pos(prev),
                        pmb_wgpu::physical_pos_to_pixel_pos(next),
                    );
                    window.request_redraw();
                }

                if !cursor_visible {
                    cursor_visible = true;
                    window.set_cursor_visible(true);
                    window.request_redraw();
                }
            }

            Event::WindowEvent {
                event: WindowEvent::Touch(touch),
                ..
            } => {
                cursor_visible = false;
                window.set_cursor_visible(false);

                let prev_y = input.cursor_pos().y as f32;
                input.handle_mouse_move(touch.location);
                let next_y = input.cursor_pos().y as f32;
                let dy = next_y - prev_y;

                let PhysicalSize { width, height } = window.inner_size();
                let prev_ndc =
                    pmb_wgpu::stroke_to_ndc(width, height, state.settings.zoom, state.stylus.point);
                let prev_pix = pmb_wgpu::ndc_to_pixel(width, height, prev_ndc);

                state.update(width, height, pmb_wgpu::glutin_to_pmb_touch(touch));

                let next_ndc =
                    pmb_wgpu::stroke_to_ndc(width, height, state.settings.zoom, state.stylus.point);
                let next_pix = pmb_wgpu::ndc_to_pixel(width, height, next_ndc);

                match (input.button_down(MouseButton::Middle), input.control()) {
                    (true, false) => state.move_origin(width, height, prev_pix, next_pix),
                    (true, true) => state.change_zoom(dy),
                    _ => {}
                }
            }

            Event::MainEventsCleared => {
                use VirtualKeyCode::*;

                if input.just_pressed(D) {
                    for stroke in state.strokes.iter() {
                        println!("stroke");
                        for point in stroke.points().iter() {
                            let x = point.x;
                            let y = point.y;
                            let p = point.pressure;
                            println!("{x}, {y}, {p}");
                        }
                    }
                    println!("brush={}", state.settings.brush_size);
                    println!("zoom={:.02}", state.settings.zoom);
                    println!("origin={}", state.settings.origin);
                }

                match (input.control(), input.just_pressed(Z)) {
                    (true, true) => {
                        state.undo_stroke();
                        window.request_redraw();
                    }
                    (false, true) => {
                        state.settings.origin = Default::default();
                        state.settings.zoom = powdermilk_biscuits::DEFAULT_ZOOM;
                        window.request_redraw();
                    }
                    _ => {}
                }

                if input.just_pressed(LBracket) {
                    state.decrease_brush();
                }

                if input.just_pressed(RBracket) {
                    state.increase_brush();
                }

                match (state.path.as_ref(), state.modified) {
                    (Some(path), true) => {
                        let title = format!("{} (modified)", path.display());
                        window.set_title(title.as_str());
                    }
                    (Some(path), false) => window.set_title(&path.display().to_string()),
                    (None, true) => window.set_title(powdermilk_biscuits::TITLE_MODIFIED),
                    (None, false) => window.set_title(powdermilk_biscuits::TITLE_UNMODIFIED),
                }

                window.request_redraw();
            }

            _ => {}
        }
    });
}
