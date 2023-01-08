use winit::{event_loop::DeviceEventFilter, window::Window};

use crate::{
    config::Config,
    event::Event,
    gumdrop::Options,
    s,
    ui::widget::SketchWidget,
    winit::{
        self,
        dpi::{PhysicalPosition, PhysicalSize},
        event::{
            ElementState, Event as WinitEvent, KeyboardInput, MouseScrollDelta, Touch, TouchPhase,
            WindowEvent,
        },
        event_loop::EventLoop,
        window::WindowBuilder,
    },
    CoordinateSystem, Sketch, StrokeBackend,
};

pub enum PerEvent {
    ConsumedByEgui(bool),
    JustRedraw,
    Nothing,
}

pub enum RenderResult {
    Redraw,
    Nothing,
}

pub trait LoopContext<S: StrokeBackend, C: CoordinateSystem> {
    fn setup(ev: &EventLoop<()>, window: &Window, sketch: &mut Sketch<S>) -> Self;

    fn per_event(
        &mut self,
        event: &WinitEvent<()>,
        window: &Window,
        sketch: &mut Sketch<S>,
        widget: &mut SketchWidget<C>,
        config: &mut Config,
    ) -> PerEvent;

    fn egui_ctx(&self) -> &egui::Context;

    fn resize(&mut self, new_size: PhysicalSize<u32>);

    fn render(
        &mut self,
        window: &Window,
        sketch: &mut Sketch<S>,
        widget: &mut SketchWidget<C>,
        config: &mut Config,
        size: PhysicalSize<u32>,
        cursor_visible: bool,
    ) -> RenderResult;
}

pub fn loop_<S, C, L>()
where
    S: StrokeBackend + 'static,
    C: CoordinateSystem + 'static,
    L: LoopContext<S, C> + 'static,
{
    env_logger::init();
    let args = crate::Args::parse_args_default_or_exit();

    if args.version {
        println!(
            "Powdermilk Biscuits ({} {}, file format version {})",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            crate::migrate::Version::CURRENT,
        );
        return;
    }

    if cfg!(unix) {
        let var = std::env::var("WINIT_UNIX_BACKEND");
        match var.as_ref().map(|s| s.as_str()) {
            Ok("x11") => {}
            Ok("wayland") => {
                let msg = "WINIT_UNIX_BACKEND=wayland is not recommended. Due to a bug in winit power consumption will suffer.";
                log::warn!("{}", msg);
                eprintln!("{}", msg);
            }
            _ => {
                let msg  = "Environment variable WINIT_UNIX_BACKEND=x11 is not set. If you're using Wayland power consumption may suffer.";
                log::warn!("{}", msg);
                eprintln!("{}", msg);
            }
        }
    }

    let config_path = if let Some(config_path) = args.config {
        config_path
    } else if cfg!(feature = "pmb-release") {
        use crate::error::PmbErrorExt;
        match Config::config_path().problem(s!(MboxMessageCouldNotOpenConfigFile)) {
            Ok(path) => path,
            Err(e) => {
                e.display();
                return;
            }
        }
    } else {
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../config.ron"))
    };

    let mut config = Config::from_disk(&config_path);
    let mut builder = WindowBuilder::new()
        .with_maximized(config.window_start_maximized)
        .with_title(format!(
            "{} ({})",
            s!(&WindowTitleNoFile),
            s!(&WindowTitleModifiedSign)
        ));

    if let (Some(x), Some(y)) = config.start_pos() {
        builder = builder.with_position(PhysicalPosition { x, y });
    }

    if let (Some(width), Some(height)) = config.start_size() {
        builder = builder.with_inner_size(PhysicalSize { width, height });
    }

    let ev = EventLoop::new();
    let window = builder.build(&ev).unwrap();
    ev.set_device_event_filter(DeviceEventFilter::Always);

    let mut widget = {
        let PhysicalSize { width, height } = window.inner_size();
        SketchWidget::<C>::new(width, height)
    };
    let mut sketch: Sketch<S> = if let Some(filename) = args.file {
        Sketch::with_filename(&mut widget, filename)
    } else {
        Sketch::default()
    };

    let mut size = window.inner_size();
    let mut cursor_visible = true;

    if let Ok(pos) = window.outer_position() {
        config.move_window(pos.x, pos.y);
    }
    config.resize_window(size.width, size.height);

    let mut ctx = L::setup(&ev, &window, &mut sketch);

    ev.run(move |event, _, flow| {
        flow.set_wait();

        log::trace!("{:?} {:?}", widget.state, event);

        let per_event = ctx.per_event(&event, &window, &mut sketch, &mut widget, &mut config);

        match per_event {
            PerEvent::ConsumedByEgui(redraw) => {
                if redraw {
                    window.request_redraw();
                }

                return;
            }

            PerEvent::JustRedraw => {
                window.request_redraw();
            }

            _ => {}
        }

        match event {
            WinitEvent::WindowEvent {
                event: WindowEvent::Focused(focused),
                ..
            } if !focused => {
                widget.input.clear();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if widget.modified {
                    if crate::ui::ask_to_save_then_save(
                        &mut widget,
                        &sketch,
                        s!(&MboxMessageAskToSaveBeforeClosing),
                    )
                    .unwrap_or(false)
                    {
                        flow.set_exit();
                        config.save(&config_path);
                    }
                } else {
                    flow.set_exit();
                    config.save(&config_path);
                }
            }

            #[cfg(not(feature = "pmb-release"))]
            WinitEvent::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                flow.set_exit();
                config.save(&config_path);
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
                widget.handle_key(&mut config, &mut sketch, key, state);
                window.request_redraw();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, delta) => {
                        widget.next(&config, &mut sketch, Event::ScrollZoom(delta));
                    }
                    MouseScrollDelta::PixelDelta(delta) => {
                        widget.next(&config, &mut sketch, Event::ScrollZoom(delta.y as f32));
                    }
                }

                window.request_redraw();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                match (button, state) {
                    (primary, ElementState::Pressed) if primary == config.primary_button => {
                        widget.next(&config, &mut sketch, Event::MouseDown(button));
                    }
                    (primary, ElementState::Released) if primary == config.primary_button => {
                        widget.next(&config, &mut sketch, Event::MouseUp(button));
                    }
                    (pan, ElementState::Pressed) if pan == config.pen_pan_button => {
                        widget.next(&config, &mut sketch, Event::StartPan);
                    }
                    (pan, ElementState::Released) if pan == config.pen_pan_button => {
                        widget.next(&config, &mut sketch, Event::EndPan);
                    }
                    _ => {}
                }

                widget.prev_device = crate::Device::Mouse;
                window.request_redraw();
            }

            WinitEvent::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                widget.next(&config, &mut sketch, Event::MouseMove(position.into()));
                widget.prev_device = crate::Device::Mouse;

                if config.use_mouse_for_pen || widget.state.redraw() {
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
                match phase {
                    TouchPhase::Started => widget.next(&config, &mut sketch, Event::PenDown(touch)),
                    TouchPhase::Moved => widget.next(&config, &mut sketch, Event::PenMove(touch)),
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        widget.next(&config, &mut sketch, Event::PenUp(touch))
                    }
                }

                widget.prev_device = crate::Device::Pen;

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
                widget.next(
                    &config,
                    &mut sketch,
                    match phase {
                        TouchPhase::Started => Event::Touch(touch),
                        TouchPhase::Moved => Event::TouchMove(touch),
                        TouchPhase::Ended | TouchPhase::Cancelled => Event::Release(touch),
                    },
                );

                widget.prev_device = crate::Device::Touch;

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
                widget.resize(new_size.width, new_size.height, &mut sketch);
                config.resize_window(new_size.width, new_size.height);
                ctx.resize(new_size);
                window.request_redraw();
            }

            WinitEvent::MainEventsCleared => {
                match (widget.path.as_ref(), widget.modified) {
                    (Some(path), true) => {
                        let title =
                            format!("{} ({})", path.display(), s!(&WindowTitleModifiedSign));
                        window.set_title(title.as_str());
                    }
                    (Some(path), false) => window.set_title(&path.display().to_string()),
                    (None, true) => window.set_title(&format!(
                        "{} ({})",
                        s!(&WindowTitleNoFile),
                        s!(&WindowTitleModifiedSign)
                    )),
                    (None, false) => window.set_title(s!(&WindowTitleNoFile)),
                }

                if ctx.egui_ctx().wants_pointer_input() {
                    if !cursor_visible {
                        window.set_cursor_visible(true);
                        cursor_visible = true;
                    }
                } else {
                    use crate::{Device, Tool};
                    let next_visible = widget.active_tool == Tool::Pan
                        || (widget.prev_device == Device::Mouse && !config.use_mouse_for_pen);
                    if cursor_visible != next_visible {
                        window.set_cursor_visible(next_visible);
                        cursor_visible = next_visible;
                    }
                }
            }

            WinitEvent::RedrawRequested(_) => match ctx.render(
                &window,
                &mut sketch,
                &mut widget,
                &mut config,
                size,
                cursor_visible,
            ) {
                RenderResult::Redraw => {
                    window.request_redraw();
                }

                RenderResult::Nothing => {}
            },

            _ => {}
        }

        log::trace!("{:?}", flow);
    });
}
