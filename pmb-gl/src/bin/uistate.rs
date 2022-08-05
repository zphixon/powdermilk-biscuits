use glutin::{
    event::{
        ElementState, Event as Gevent, KeyboardInput, MouseButton, Touch, TouchPhase,
        VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use powdermilk_biscuits::ui::{Config, Event as Pevent, Tool, UiState};

fn main() {
    let ev = EventLoop::new();
    let _window = WindowBuilder::new().build(&ev);

    let mut ui_state = UiState::default();
    let mut config = Config::default();

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        println!("{:?}", ui_state);

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
                if pen_info.inverted {
                    config.active_tool = Tool::Eraser;
                } else {
                    config.active_tool = Tool::Pen;
                }

                match phase {
                    TouchPhase::Started => ui_state.next(&config, Pevent::PenDown),
                    TouchPhase::Moved => ui_state.next(&config, Pevent::MovePen),
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        ui_state.next(&config, Pevent::PenUp)
                    }
                }
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
                (VirtualKeyCode::LControl, ElementState::Pressed) => {
                    ui_state.next(&config, Pevent::StartZoom)
                }
                (VirtualKeyCode::LControl, ElementState::Released) => {
                    ui_state.next(&config, Pevent::EndZoom)
                }
                _ => {}
            },

            Gevent::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => match (button, state) {
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
            },

            Gevent::WindowEvent {
                event: WindowEvent::CursorMoved { .. },
                ..
            } => {
                ui_state.next(&config, Pevent::MoveMouse);
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
                todo!();
            }

            _ => {}
        }
    });
}
