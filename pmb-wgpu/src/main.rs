use powdermilk_biscuits::{
    event::{ElementState, Event},
    ui::{self, Ui},
    Config, Device, Sketch, Tool,
};
use wgpu::SurfaceError;
use winit::{
    dpi::LogicalPosition,
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
    let ev = EventLoop::new();
    let window = WindowBuilder::new()
        .with_position(LogicalPosition {
            x: 1920. / 2. - 800. / 2.,
            y: 1080. + 1080. / 2. - 600. / 2.,
        })
        .with_title(powdermilk_biscuits::TITLE_UNMODIFIED)
        .build(&ev)
        .unwrap();

    let mut config = Config::default();
    let mut ui = {
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();
        Ui::<pmb_wgpu::WgpuBackend>::new(width, height)
    };
    let mut sketch: Sketch<pmb_wgpu::WgpuStrokeBackend> =
        if let Some(filename) = std::env::args().nth(1) {
            Sketch::with_filename(&mut ui, std::path::PathBuf::from(filename))
        } else {
            Sketch::default()
        };

    let mut graphics = pmb_wgpu::Graphics::new(&window).await;
    graphics.buffer_all_strokes(&mut sketch);

    let mut size = window.inner_size();
    let mut cursor_visible = true;

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        log::info!("{event:?}");

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
                    if ui
                        .ask_to_save_then_save(&sketch, "Would you like to save before exiting?")
                        .unwrap_or(false)
                    {
                        *flow = ControlFlow::Exit;
                    }
                } else {
                    *flow = ControlFlow::Exit;
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
                ui.handle_key(&mut sketch, key, state, size.width, size.height);

                match (key, state) {
                    (zoom, ElementState::Pressed)
                        if config.prev_device == Device::Pen && zoom == config.pen_zoom_key =>
                    {
                        ui.next(&config, &mut sketch, Event::StartZoom);
                    }

                    (zoom, ElementState::Released) if zoom == config.pen_zoom_key => {
                        ui.next(&config, &mut sketch, Event::EndZoom);
                    }

                    (mouse, ElementState::Pressed) if mouse == config.use_mouse_for_pen_key => {
                        config.use_mouse_for_pen = !config.use_mouse_for_pen;
                        println!("using mouse for pen? {}", config.use_mouse_for_pen);
                    }

                    (finger, ElementState::Pressed) if finger == config.use_finger_for_pen_key => {
                        config.use_finger_for_pen = !config.use_finger_for_pen;
                        println!("using finger for pen? {}", config.use_finger_for_pen);
                    }

                    (swap, ElementState::Pressed)
                        if (config.prev_device == Device::Mouse
                            || !config.stylus_may_be_inverted)
                            && swap == config.swap_eraser_key =>
                    {
                        if config.active_tool != Tool::Eraser {
                            config.active_tool = Tool::Eraser;
                        } else {
                            config.active_tool = Tool::Pen;
                        }
                        ui.next(&config, &mut sketch, Event::ToolChange);
                    }

                    (brush, ElementState::Pressed) if brush == config.brush_increase => {
                        ui.next(
                            &config,
                            &mut sketch,
                            Event::IncreaseBrush(powdermilk_biscuits::BRUSH_DELTA),
                        );
                    }

                    (brush, ElementState::Pressed) if brush == config.brush_decrease => {
                        ui.next(
                            &config,
                            &mut sketch,
                            Event::DecreaseBrush(powdermilk_biscuits::BRUSH_DELTA),
                        );
                    }

                    _ => {}
                }

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

                config.prev_device = Device::Mouse;
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
                config.prev_device = Device::Mouse;

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
                            pen_info: Some(pen_info),
                            ..
                        },
                    ),
                ..
            } => {
                let touch = pmb_wgpu::winit_to_pmb_touch(touch);
                if config.stylus_may_be_inverted {
                    if pen_info.inverted {
                        config.active_tool = Tool::Eraser;
                    } else {
                        config.active_tool = Tool::Pen;
                    }
                }

                match phase {
                    TouchPhase::Started => ui.next(&config, &mut sketch, Event::PenDown(touch)),
                    TouchPhase::Moved => ui.next(&config, &mut sketch, Event::PenMove(touch)),
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        ui.next(&config, &mut sketch, Event::PenUp(touch))
                    }
                }

                config.prev_device = Device::Pen;

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
                        TouchPhase::Moved => Event::PenMove(touch),
                        TouchPhase::Ended | TouchPhase::Cancelled => Event::Release(touch),
                    },
                );

                config.prev_device = Device::Touch;

                if cursor_visible && config.use_finger_for_pen {
                    cursor_visible = false;
                    window.set_cursor_visible(false);
                }

                window.request_redraw();
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

            _ => {}
        }
    });
}
