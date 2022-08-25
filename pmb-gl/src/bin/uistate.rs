use glutin::{
    dpi::PhysicalSize,
    event::{
        ElementState, Event as Gevent, KeyboardInput, MouseScrollDelta, Touch, TouchPhase,
        VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use pmb_gl::{GlBackend, GlStrokeBackend};
use powdermilk_biscuits::ui::{Config, Device, Event as Pevent, Sketch, Tool, Ui};

fn main() {
    let ev = EventLoop::new();
    let window = WindowBuilder::new().build(&ev).unwrap();
    let PhysicalSize { width, height } = window.inner_size();

    let mut sketch = Sketch::<GlStrokeBackend>::default();
    let mut ui = Ui::<GlBackend>::new(width, height);
    let mut config = Config::default();
    let mut cursor_visible = true;

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        println!("{:?}", ui);

        match event {
            Gevent::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    },
                ..
            }
            | Gevent::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *flow = ControlFlow::Exit;
            }

            Gevent::WindowEvent {
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
                let touch = pmb_gl::glutin_to_pmb_touch(touch);
                if config.stylus_may_be_inverted {
                    if pen_info.inverted {
                        config.active_tool = Tool::Eraser;
                    } else {
                        config.active_tool = Tool::Pen;
                    }
                }

                match phase {
                    TouchPhase::Started => ui.next(&config, &mut sketch, Pevent::PenDown(touch)),
                    TouchPhase::Moved => ui.next(&config, &mut sketch, Pevent::PenMove(touch)),
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        ui.next(&config, &mut sketch, Pevent::PenUp(touch))
                    }
                }

                config.prev_device = Device::Pen;

                if cursor_visible {
                    cursor_visible = false;
                    window.set_cursor_visible(false);
                }

                window.request_redraw();
            }

            Gevent::WindowEvent {
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
                let key = pmb_gl::glutin_to_pmb_keycode(key);
                match (key, state) {
                    (zoom, ElementState::Pressed)
                        if config.prev_device == Device::Pen && zoom == config.pen_zoom_key =>
                    {
                        ui.next(&config, &mut sketch, Pevent::StartZoom);
                        window.request_redraw();
                    }

                    (zoom, ElementState::Released) if zoom == config.pen_zoom_key => {
                        ui.next(&config, &mut sketch, Pevent::EndZoom);
                        window.request_redraw();
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
                        window.request_redraw();
                    }

                    (brush, ElementState::Pressed) if brush == config.brush_increase => {
                        ui.next(
                            &config,
                            &mut sketch,
                            Pevent::BrushSize(powdermilk_biscuits::BRUSH_DELTA as i32),
                        );

                        window.request_redraw();
                    }

                    (brush, ElementState::Pressed) if brush == config.brush_decrease => {
                        ui.next(
                            &config,
                            &mut sketch,
                            Pevent::BrushSize(-(powdermilk_biscuits::BRUSH_DELTA as i32)),
                        );

                        window.request_redraw();
                    }

                    _ => {}
                }
            }

            Gevent::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, delta) => {
                        ui.next(&config, &mut sketch, Pevent::ActiveZoom(delta as i32));
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        ui.next(&config, &mut sketch, Pevent::ActiveZoom(delta.y as i32));
                    }
                }

                window.request_redraw();
            }

            Gevent::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let button = pmb_gl::glutin_to_pmb_mouse_button(button);
                match (button, state) {
                    (primary, ElementState::Pressed) if primary == config.primary_button => {
                        ui.next(&config, &mut sketch, Pevent::MouseDown(button));
                        window.request_redraw();
                    }
                    (primary, ElementState::Released) if primary == config.primary_button => {
                        ui.next(&config, &mut sketch, Pevent::MouseUp(button));
                        window.request_redraw();
                    }
                    (pan, ElementState::Pressed) if pan == config.pan_button => {
                        ui.next(&config, &mut sketch, Pevent::StartPan);
                    }
                    (pan, ElementState::Released) if pan == config.pan_button => {
                        ui.next(&config, &mut sketch, Pevent::EndPan);
                    }
                    _ => {}
                }

                config.prev_device = Device::Mouse;
            }

            Gevent::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                ui.next(
                    &config,
                    &mut sketch,
                    Pevent::MouseMove(pmb_gl::physical_pos_to_pixel_pos(position)),
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
            }

            Gevent::WindowEvent {
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
                let touch = pmb_gl::glutin_to_pmb_touch(touch);
                ui.next(
                    &config,
                    &mut sketch,
                    match phase {
                        TouchPhase::Started => Pevent::Touch(touch),
                        TouchPhase::Moved => Pevent::PenMove(touch),
                        TouchPhase::Ended | TouchPhase::Cancelled => Pevent::Release(touch),
                    },
                );

                config.prev_device = Device::Touch;

                if cursor_visible && config.use_finger_for_pen {
                    cursor_visible = false;
                    window.set_cursor_visible(false);
                }

                window.request_redraw();
            }

            Gevent::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                ui.resize(new_size.width, new_size.height);
                window.request_redraw();
            }

            _ => {}
        }
    });
}
