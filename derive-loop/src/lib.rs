use indexmap::{IndexMap as HashMap, IndexSet as HashSet};
use proc_macro2::Span;
use syn::{parse::Parse, Block, Ident, Token};

#[derive(derive_builder::Builder)]
struct PmbLoop {
    backend_crate_name: Ident,
    coords_name: Ident,
    stroke_backend_name: Ident,

    window: Block,
    egui_ctx: Block,

    bindings: HashMap<Ident, (bool, Option<Block>)>,
    graphics_setup: HashMap<Ident, (bool, Option<Block>)>,

    per_event: Block,
    resize: Block,
    render: Block,
}

trait ErrorExt {
    fn combine(self, span: Span, message: &str) -> Self;
}

impl<T> ErrorExt for syn::Result<T> {
    fn combine(self, span: Span, message: &str) -> Self {
        self.map_err(|mut err| {
            err.combine(syn::Error::new(span, message));
            err
        })
    }
}

impl Parse for PmbLoop {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        use syn::Error;
        let mut builder = PmbLoopBuilder::default();

        macro_rules! build {
            ($($name:ident),* $(,)? ; $($bindings:ident),* $(,)?) => {
                let mut missing_fields = HashSet::new();
                $(missing_fields.insert(stringify!($name));)*
                $(missing_fields.insert(stringify!($bindings));)*

                while let Err(_) = builder.build() {
                    let field = input.parse::<Ident>().combine(
                        input.span(),
                        &format!("Expected more fields: missing {:?}", missing_fields),
                    )?;

                    let _colon = input.parse::<Token!(:)>()?;

                    match field.to_string().as_str() {
                        $(stringify!($name) => {
                            builder.$name(input.parse()?);
                            missing_fields.remove(stringify!($name));

                            let _comma = input
                                .parse::<Token!(,)>()
                                .combine(field.span(), "Expected comma after field")?;
                        })*

                        $(stringify!($bindings) => {
                            let mut bindings = HashMap::new();

                            while !input.peek(Token!(;)) {
                                let name = input.parse()?;
                                let _eq = input.parse::<Token!(=)>()?;

                                let mutable = input.peek(Token!(mut));
                                if mutable {
                                    let _mut = input.parse::<Token!(mut)>()?;
                                }

                                let uninit = input.peek(Ident) && input.parse::<Ident>()?.to_string().as_str() == "no_init";

                                let value = if uninit {
                                    None
                                } else {
                                    Some(input.parse()?)
                                };

                                bindings.insert(name, (mutable, value));
                            }
                            let _dcol = input.parse::<Token!(;)>()?;

                            builder.$bindings(bindings);
                            missing_fields.remove(stringify!($bindings));
                        })*

                        _ => return Err(Error::new(field.span(), "Unexpected field name")),
                    }
                }
            }
        }

        build!(
            backend_crate_name,
            coords_name,
            stroke_backend_name,
            window,
            egui_ctx,
            per_event,
            resize,
            render;
            bindings,
            graphics_setup,
        );

        Ok(builder.build().unwrap())
    }
}

#[proc_macro]
pub fn pmb_loop(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let loop_ = syn::parse_macro_input!(input as PmbLoop);

    let PmbLoop {
        backend_crate_name,
        coords_name,
        stroke_backend_name,
        window,
        egui_ctx,
        bindings,
        graphics_setup,
        per_event,
        resize,
        render,
    } = loop_;

    let quoted_bindings = bindings
        .into_iter()
        .map(|(name, (mutable, value))| match (mutable, value) {
            (true, Some(value)) => quote::quote!(let mut #name = #value;),
            (false, Some(value)) => quote::quote!(let #name = #value;),
            (true, None) => quote::quote!(let mut #name;),
            (false, None) => quote::quote!(let #name;),
        })
        .collect::<Vec<_>>();

    let quoted_graphics_setup = graphics_setup
        .into_iter()
        .map(|(name, (mutable, value))| match (mutable, value) {
            (true, Some(value)) => quote::quote!(let mut #name = #value;),
            (false, Some(value)) => quote::quote!(let #name = #value;),
            (true, None) => quote::quote!(let mut #name;),
            (false, None) => quote::quote!(let #name;),
        })
        .collect::<Vec<_>>();

    quote::quote! {
        use powdermilk_biscuits::{
            config::Config,
            event::Event,
            gumdrop::Options,
            ui::widget::SketchWidget,
            Sketch,
            winit::{
                self,
                dpi::{PhysicalPosition, PhysicalSize},
                event::{
                    ElementState, Event as WinitEvent, KeyboardInput,
                    MouseScrollDelta, Touch, TouchPhase, VirtualKeyCode, WindowEvent,
                },
                event_loop::EventLoop,
                window::WindowBuilder,
            }
        };

        let args = powdermilk_biscuits::Args::parse_args_default_or_exit();

        if args.version {
            println!(
                "Powdermilk Biscuits ({} {}, file format version {})",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                powdermilk_biscuits::migrate::Version::CURRENT,
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
        } else {
            if cfg!(feature = "pmb-release") {
                use powdermilk_biscuits::error::PmbErrorExt;
                match Config::config_path().problem(format!("Couldn't open config dir")) {
                    Ok(path) => path,
                    Err(e) => {
                        e.display();
                        return;
                    }
                }
            } else {
                std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../config.ron"))
            }
        };

        let mut config = Config::from_disk(&config_path);
        let mut builder = WindowBuilder::new()
            .with_maximized(config.window_maximized)
            .with_title(powdermilk_biscuits::TITLE_UNMODIFIED);

        if let (Some(x), Some(y)) = config.start_pos() {
            builder = builder.with_position(PhysicalPosition { x, y });
        }

        if let (Some(width), Some(height)) = config.start_size() {
            builder = builder.with_inner_size(PhysicalSize { width, height });
        }

        let ev = EventLoop::new();

        #(#quoted_bindings)*

        let mut widget = {
            let PhysicalSize { width, height } = #window.inner_size();
            SketchWidget::<#backend_crate_name::#coords_name>::new(width, height)
        };
        let mut sketch: Sketch<#backend_crate_name::#stroke_backend_name> =
            if let Some(filename) = args.file {
                Sketch::with_filename(&mut widget, std::path::PathBuf::from(filename))
            } else {
                Sketch::default()
            };

        widget.force_update(&mut sketch);

        #(#quoted_graphics_setup)*

        let mut size = #window.inner_size();
        let mut cursor_visible = true;

        if let Ok(pos) = #window.outer_position() {
            config.move_window(pos.x, pos.y);
        }
        config.resize_window(size.width, size.height);

        ev.run(move |event, _, flow| {
            flow.set_wait();

            log::trace!("{:?} {:?}", widget.state, event);

            #per_event;

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
                        if powdermilk_biscuits::ui::ask_to_save_then_save(
                            &mut widget,
                            &sketch,
                            powdermilk_biscuits::s!(&AskToSaveBeforeClosing),
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
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
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
                    widget.handle_key(
                        &mut config,
                        &mut sketch,
                        key,
                        state,
                        size.width,
                        size.height,
                    );
                    #window.request_redraw();
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

                    #window.request_redraw();
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
                        (pan, ElementState::Pressed) if pan == config.pan_button => {
                            widget.next(&config, &mut sketch, Event::StartPan);
                        }
                        (pan, ElementState::Released) if pan == config.pan_button => {
                            widget.next(&config, &mut sketch, Event::EndPan);
                        }
                        _ => {}
                    }

                    widget.prev_device = powdermilk_biscuits::Device::Mouse;
                    #window.request_redraw();
                }

                WinitEvent::WindowEvent {
                    event: WindowEvent::CursorMoved { position, .. },
                    ..
                } => {
                    widget.next(
                        &config,
                        &mut sketch,
                        Event::MouseMove(#backend_crate_name::physical_pos_to_pixel_pos(position)),
                    );
                    widget.prev_device = powdermilk_biscuits::Device::Mouse;

                    if config.use_mouse_for_pen {
                        #window.request_redraw();
                    }

                    if widget.state.redraw() {
                        #window.request_redraw();
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

                    widget.prev_device = powdermilk_biscuits::Device::Pen;

                    #window.request_redraw();
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

                    widget.prev_device = powdermilk_biscuits::Device::Touch;

                    #window.request_redraw();
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
                    #resize
                    #window.request_redraw();
                },

                WinitEvent::MainEventsCleared => {
                    use powdermilk_biscuits::winit::event::VirtualKeyCode::*;

                    match (widget.path.as_ref(), widget.modified) {
                        (Some(path), true) => {
                            let title = format!("{} (modified)", path.display());
                            #window.set_title(title.as_str());
                        }
                        (Some(path), false) => #window.set_title(&path.display().to_string()),
                        (None, true) => #window.set_title(powdermilk_biscuits::TITLE_MODIFIED),
                        (None, false) => #window.set_title(powdermilk_biscuits::TITLE_UNMODIFIED),
                    }

                    if #egui_ctx.wants_pointer_input() {
                        if !cursor_visible {
                            #window.set_cursor_visible(true);
                            cursor_visible = true;
                        }
                    } else {
                        use powdermilk_biscuits::{Device, Tool};
                        let next_visible = widget.active_tool == Tool::Pan
                            || (widget.prev_device == Device::Mouse
                                && !config.use_mouse_for_pen);
                        if cursor_visible != next_visible {
                            #window.set_cursor_visible(next_visible);
                            cursor_visible = next_visible;
                        }
                    }
                }

                WinitEvent::RedrawRequested(_) => #render,

                _ => {}
            }

            log::trace!("{:?}", flow);
        });
    }.into()
}
