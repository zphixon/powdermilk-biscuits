use powdermilk_biscuits::{backend::wgpu as backend, ui};
use wgpu::SurfaceError;
use winit::{
    dpi::{LogicalPosition, PhysicalSize},
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub fn main() {
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

    let mut state = if let Some(filename) = std::env::args()
        .nth(1)
        .map(|filename| std::path::PathBuf::from(filename))
    {
        powdermilk_biscuits::State::with_filename(filename)
    } else {
        powdermilk_biscuits::State::default()
    };

    let mut graphics = backend::Graphics::new(&window).await;
    window.request_redraw();

    ev.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
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
                    WindowEvent::Resized(new_size)
                    | WindowEvent::ScaleFactorChanged {
                        new_inner_size: &mut new_size,
                        ..
                    },
                ..
            } => {
                graphics.resize(new_size);
            }

            Event::RedrawRequested(_) => match graphics.render() {
                Err(SurfaceError::Lost) => graphics.resize(graphics.size),
                Err(SurfaceError::OutOfMemory) => {
                    ui::error("Out of memory!");
                    *flow = ControlFlow::Exit;
                }
                _ => {}
            },

            Event::WindowEvent {
                event: WindowEvent::Touch(touch),
                ..
            } => {
                let PhysicalSize { width, height } = window.inner_size();
                state.update(width, height, touch.into());
            }

            Event::MainEventsCleared => {
                window.request_redraw();
            }

            _ => {}
        }
    });
}
