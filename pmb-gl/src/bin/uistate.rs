use glutin::{
    event::{
        ElementState, Event as Gevent, KeyboardInput, MouseButton, Touch, TouchPhase,
        VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use powdermilk_biscuits::ui::{Config, Device, Event as Pevent, Tool, UiState};

fn main() {
    let ev = EventLoop::new();
    let _window = WindowBuilder::new().build(&ev);

    let mut ui_state = UiState::default();
    let mut config = Config::default();

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        if !matches!(ui_state, UiState::Ready) {
            println!("{:?}", ui_state);
        }

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
                    WindowEvent::Touch(Touch {
                        phase,
                        pen_info: Some(pen_info),
                        ..
                    }),
                ..
            } => {
                if config.stylus_may_be_inverted {
                    if pen_info.inverted {
                        config.active_tool = Tool::Eraser;
                    } else {
                        config.active_tool = Tool::Pen;
                    }
                }

                match phase {
                    TouchPhase::Started => ui_state.next(&config, Pevent::PenDown),
                    TouchPhase::Moved => ui_state.next(&config, Pevent::MovePen),
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        ui_state.next(&config, Pevent::PenUp)
                    }
                }

                config.prev_device = Device::Pen;
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
            } => match (key, state) {
                (VirtualKeyCode::LControl, ElementState::Pressed)
                    if config.prev_device == Device::Pen =>
                {
                    ui_state.next(&config, Pevent::StartZoom)
                }

                (VirtualKeyCode::LControl, ElementState::Released) => {
                    ui_state.next(&config, Pevent::EndZoom)
                }

                (VirtualKeyCode::M, ElementState::Pressed) => {
                    config.use_mouse_for_pen = !config.use_mouse_for_pen;
                    println!("using mouse for pen? {}", config.use_mouse_for_pen);
                }

                (VirtualKeyCode::F, ElementState::Pressed) => {
                    config.use_finger_for_pen = !config.use_finger_for_pen;
                    println!("using finger for pen? {}", config.use_finger_for_pen);
                }

                (VirtualKeyCode::E, ElementState::Pressed)
                    if config.prev_device == Device::Mouse || !config.stylus_may_be_inverted =>
                {
                    if config.active_tool != Tool::Eraser {
                        config.active_tool = Tool::Eraser;
                    } else {
                        config.active_tool = Tool::Pen;
                    }
                }

                _ => {}
            },

            Gevent::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        ui_state.next(&config, Pevent::MouseDown)
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        ui_state.next(&config, Pevent::MouseUp)
                    }
                    (MouseButton::Middle, ElementState::Pressed) => {
                        ui_state.next(&config, Pevent::StartPan)
                    }
                    (MouseButton::Middle, ElementState::Released) => {
                        ui_state.next(&config, Pevent::EndPan)
                    }
                    _ => {}
                }

                config.prev_device = Device::Mouse;
            }

            Gevent::WindowEvent {
                event: WindowEvent::CursorMoved { .. },
                ..
            } => {
                ui_state.next(&config, Pevent::MoveMouse);
                config.prev_device = Device::Mouse;
            }

            Gevent::WindowEvent {
                event:
                    WindowEvent::Touch(Touch {
                        phase,
                        pen_info: None,
                        ..
                    }),
                ..
            } => {
                ui_state.next(
                    &config,
                    match phase {
                        TouchPhase::Started => Pevent::Touch,
                        TouchPhase::Moved => Pevent::MovePen,
                        TouchPhase::Ended | TouchPhase::Cancelled => Pevent::Release,
                    },
                );

                config.prev_device = Device::Touch;
            }

            _ => {}
        }
    });
}
