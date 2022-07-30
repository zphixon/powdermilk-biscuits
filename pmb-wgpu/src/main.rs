use pmb_wgpu::WgslState as State;
use powdermilk_biscuits::ui;
use wgpu::SurfaceError;
use winit::{
    dpi::{LogicalPosition, PhysicalSize},
    event::{ElementState, Event, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent},
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

    let mut state: State =
        if let Some(filename) = std::env::args().nth(1).map(std::path::PathBuf::from) {
            State::with_filename(filename)
        } else {
            State::default()
        };

    let mut graphics = pmb_wgpu::Graphics::new(&window).await;
    graphics.buffer_all_strokes(&mut state);

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
                        .ask_to_save_then_save("save-before-exit")
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
                event:
                    WindowEvent::MouseInput {
                        state: key_state,
                        button,
                        ..
                    },
                ..
            } => {
                let button = pmb_wgpu::winit_to_pmb_mouse_button(button);
                let key_state = pmb_wgpu::winit_to_pmb_key_state(key_state);
                state.handle_mouse_button(button, key_state);
                window.request_redraw();
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
                                state: key_state,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                let key = pmb_wgpu::winit_to_pmb_keycode(key);
                let key_state = pmb_wgpu::winit_to_pmb_key_state(key_state);
                state.handle_key(key, key_state);
            }

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let PhysicalSize { width, height } = window.inner_size();
                state.handle_cursor_move(
                    width,
                    height,
                    pmb_wgpu::physical_pos_to_pixel_pos(position),
                );

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

                let PhysicalSize { width, height } = window.inner_size();
                state.handle_touch(pmb_wgpu::winit_to_pmb_touch(touch), width, height);
            }

            Event::MainEventsCleared => {
                if state
                    .input
                    .just_pressed(powdermilk_biscuits::input::Keycode::A)
                {
                    graphics.aa = !graphics.aa;
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
