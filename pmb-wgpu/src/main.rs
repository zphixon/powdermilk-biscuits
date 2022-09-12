#![cfg_attr(all(windows, feature = "pmb-release"), windows_subsystem = "windows")]

use powdermilk_biscuits::{
    event::{ElementState, Event},
    ui::{self, Ui},
    Config, Device, Sketch,
};
use wgpu::SurfaceError;
use winit::{
    event::{
        ElementState as WinitElementState, Event as WinitEvent, KeyboardInput, MouseScrollDelta,
        Touch, TouchPhase, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    env_logger::init();
    futures::executor::block_on(run());
}

async fn run() {
    let mut config = Config::from_disk();
    let mut builder = WindowBuilder::new()
        .with_maximized(config.window_maximized)
        .with_title(powdermilk_biscuits::TITLE_UNMODIFIED);

    if let (Some(x), Some(y)) = (config.window_start_x, config.window_start_y) {
        builder = builder.with_position(winit::dpi::PhysicalPosition { x, y });
    }

    if let (Some(width), Some(height)) = (config.window_start_width, config.window_start_height) {
        builder = builder.with_inner_size(winit::dpi::PhysicalSize { width, height });
    }

    let ev = EventLoop::new();
    let window = builder.build(&ev).unwrap();

    let mut ui = {
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();
        Ui::<pmb_wgpu::WgpuCoords>::new(width, height)
    };
    let mut sketch: Sketch<pmb_wgpu::WgpuStrokeBackend> =
        if let Some(filename) = std::env::args().nth(1) {
            Sketch::with_filename(&mut ui, std::path::PathBuf::from(filename))
        } else {
            Sketch::default()
        };

    ui.force_update(&mut sketch);

    let mut graphics = pmb_wgpu::Graphics::new(&window).await;
    graphics.buffer_all_strokes(&mut sketch);

    let mut size = window.inner_size();
    let mut cursor_visible = true;

    if let Ok(pos) = window.outer_position() {
        config.move_window(pos.x, pos.y);
    }
    config.resize_window(size.width, size.height);

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        log::trace!("{event:?}");

        match event {
            WinitEvent::WindowEvent {
                event: WindowEvent::Focused(focused),
                ..
            } if !focused => {
                ui.input.clear();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if ui.modified {
                    if powdermilk_biscuits::ui::ask_to_save_then_save(
                        &mut ui,
                        &sketch,
                        "Would you like to save before exiting?",
                    )
                    .unwrap_or(false)
                    {
                        *flow = ControlFlow::Exit;
                        config.save();
                    }
                } else {
                    *flow = ControlFlow::Exit;
                    config.save();
                }
            }

            WinitEvent::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: WinitElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                *flow = ControlFlow::Exit;
                config.save();
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
                let key = pmb_wgpu::winit_to_pmb_keycode(key);
                let state = pmb_wgpu::winit_to_pmb_key_state(state);
                ui.handle_key(
                    &mut config,
                    &mut sketch,
                    key,
                    state,
                    size.width,
                    size.height,
                );
                window.request_redraw();
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

                window.request_redraw();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let button = pmb_wgpu::winit_to_pmb_mouse_button(button);
                let state = pmb_wgpu::winit_to_pmb_key_state(state);

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
                window.request_redraw();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                ui.next(
                    &config,
                    &mut sketch,
                    Event::MouseMove(pmb_wgpu::physical_pos_to_pixel_pos(position)),
                );
                ui.prev_device = Device::Mouse;

                if config.use_mouse_for_pen {
                    if cursor_visible {
                        cursor_visible = false;
                        window.set_cursor_visible(false);
                    }
                    window.request_redraw();
                } else if !cursor_visible {
                    cursor_visible = true;
                    window.set_cursor_visible(true);
                }

                if ui.state.redraw() {
                    window.request_redraw();
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
                let touch = pmb_wgpu::winit_to_pmb_touch(touch);

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
                    window.set_cursor_visible(false);
                }

                window.request_redraw();
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
                let touch = pmb_wgpu::winit_to_pmb_touch(touch);
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
                    window.set_cursor_visible(false);
                }

                window.request_redraw();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::Moved(location),
                ..
            } => {
                config.move_window(location.x, location.y);
            }

            WinitEvent::WindowEvent {
                event:
                    WindowEvent::Resized(new_size)
                    | WindowEvent::ScaleFactorChanged {
                        new_inner_size: &mut new_size,
                        ..
                    },
                ..
            } => {
                size = new_size;
                ui.resize(new_size.width, new_size.height, &mut sketch);
                graphics.resize(new_size);
                window.request_redraw();
                config.resize_window(new_size.width, new_size.height);
            }

            WinitEvent::MainEventsCleared => {
                use powdermilk_biscuits::event::Keycode::*;

                if ui.input.just_pressed(A) {
                    graphics.aa = !graphics.aa;
                    window.request_redraw();
                }

                match (ui.path.as_ref(), ui.modified) {
                    (Some(path), true) => {
                        let title = format!("{} (modified)", path.display());
                        window.set_title(title.as_str());
                    }
                    (Some(path), false) => window.set_title(&path.display().to_string()),
                    (None, true) => window.set_title(powdermilk_biscuits::TITLE_MODIFIED),
                    (None, false) => window.set_title(powdermilk_biscuits::TITLE_UNMODIFIED),
                }
            }

            WinitEvent::RedrawRequested(_) => {
                match graphics.render(&mut sketch, &ui, size, cursor_visible) {
                    Err(SurfaceError::Lost) => graphics.resize(graphics.size),
                    Err(SurfaceError::OutOfMemory) => {
                        ui::error("Out of memory!");
                        *flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }

            _ => {}
        }
    });
}
