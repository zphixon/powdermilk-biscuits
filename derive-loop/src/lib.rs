use indexmap::{IndexMap as HashMap, IndexSet as HashSet};
use proc_macro2::Span;
use syn::{parse::Parse, Block, Ident, Token};

#[derive(derive_builder::Builder)]
struct PmbLoop {
    loop_name: Ident,
    windowing_crate_name: Ident,

    backend_crate_name: Ident,
    coords_name: Ident,
    stroke_backend_name: Ident,
    keycode_translation: Ident,
    mouse_button_translation: Ident,
    key_state_translation: Ident,
    touch_translation: Ident,

    bindings: HashMap<Ident, (bool, Option<Block>)>,
    graphics_setup: HashMap<Ident, (bool, Option<Block>)>,

    window: Block,
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
            loop_name,
            windowing_crate_name,
            backend_crate_name,
            coords_name,
            stroke_backend_name,
            keycode_translation,
            mouse_button_translation,
            key_state_translation,
            touch_translation,
            window,
            per_event,
            resize,
            render;

            bindings,
            graphics_setup
        );

        Ok(builder.build().unwrap())
    }
}

#[proc_macro]
pub fn egui(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    quote::quote!(|ctx| {
        egui::SidePanel::left("side panel").show(ctx, |eui| {
            eui.heading("Real Hot Item");
            eui.color_edit_button_rgb(&mut ui.clear_color);
        });
    })
    .into()
}

#[proc_macro]
pub fn pmb_loop(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let loop_ = syn::parse_macro_input!(input as PmbLoop);

    let PmbLoop {
        loop_name,
        windowing_crate_name,
        backend_crate_name,
        coords_name,
        stroke_backend_name,
        keycode_translation,
        mouse_button_translation,
        key_state_translation,
        touch_translation,
        graphics_setup,
        window,
        per_event,
        resize,
        render,
        bindings,
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
        fn #loop_name() {
            use powdermilk_biscuits::gumdrop::Options;
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
                    let mut dir = powdermilk_biscuits::dirs::config_dir().unwrap();
                    dir.push("powdermilk-biscuits");
                    dir.push("config.ron");
                    dir
                } else {
                    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../config.ron"))
                }
            };

            let mut config = powdermilk_biscuits::Config::from_disk(&config_path);
            let mut builder = #windowing_crate_name::window::WindowBuilder::new()
                .with_maximized(config.window_maximized)
                .with_title(powdermilk_biscuits::TITLE_UNMODIFIED);

            if let (Some(x), Some(y)) = (config.window_start_x, config.window_start_y) {
                builder = builder.with_position(#windowing_crate_name::dpi::PhysicalPosition { x, y });
            }

            if let (Some(width), Some(height)) = (config.window_start_width, config.window_start_height) {
                builder = builder.with_inner_size(#windowing_crate_name::dpi::PhysicalSize { width, height });
            }

            let ev = #windowing_crate_name::event_loop::EventLoop::new();

            #(#quoted_bindings)*

            let mut ui = {
                let #windowing_crate_name::dpi::PhysicalSize { width, height } = #window.inner_size();
                powdermilk_biscuits::ui::Ui::<#backend_crate_name::#coords_name>::new(width, height)
            };
            let mut sketch: powdermilk_biscuits::Sketch<#backend_crate_name::#stroke_backend_name> =
                if let Some(filename) = args.file {
                    powdermilk_biscuits::Sketch::with_filename(&mut ui, std::path::PathBuf::from(filename))
                } else {
                    powdermilk_biscuits::Sketch::default()
                };

            ui.force_update(&mut sketch);

            #(#quoted_graphics_setup)*

            let mut size = #window.inner_size();
            let mut cursor_visible = true;

            if let Ok(pos) = #window.outer_position() {
                config.move_window(pos.x, pos.y);
            }
            config.resize_window(size.width, size.height);

            ev.run(move |event, _, flow| {
                flow.set_wait();

                log::trace!("{:?} {:?}", ui.state, event);

                #per_event;

                match event {
                    #windowing_crate_name::event::Event::WindowEvent {
                        event: #windowing_crate_name::event::WindowEvent::Focused(focused),
                        ..
                    } if !focused => {
                        ui.input.clear();
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event: #windowing_crate_name::event::WindowEvent::CloseRequested,
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
                                flow.set_exit();
                                config.save(&config_path);
                            }
                        } else {
                            flow.set_exit();
                            config.save(&config_path);
                        }
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event:
                            #windowing_crate_name::event::WindowEvent::KeyboardInput {
                                input:
                                    #windowing_crate_name::event::KeyboardInput {
                                        state: #windowing_crate_name::event::ElementState::Pressed,
                                        virtual_keycode: Some(#windowing_crate_name::event::VirtualKeyCode::Escape),
                                        ..
                                    },
                                ..
                            },
                        ..
                    } => {
                        flow.set_exit();
                        config.save(&config_path);
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event:
                            #windowing_crate_name::event::WindowEvent::KeyboardInput {
                                input:
                                    #windowing_crate_name::event::KeyboardInput {
                                        virtual_keycode: Some(key),
                                        state,
                                        ..
                                    },
                                ..
                            },
                        ..
                    } => {
                        let key = #backend_crate_name::#keycode_translation(key);
                        let state = #backend_crate_name::#key_state_translation(state);
                        ui.handle_key(
                            &mut config,
                            &mut sketch,
                            key,
                            state,
                            size.width,
                            size.height,
                        );
                        #window.request_redraw();
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event: #windowing_crate_name::event::WindowEvent::MouseWheel { delta, .. },
                        ..
                    } => {
                        match delta {
                            #windowing_crate_name::event::MouseScrollDelta::LineDelta(_, delta) => {
                                ui.next(&config, &mut sketch, powdermilk_biscuits::event::Event::ScrollZoom(delta));
                            }
                            #windowing_crate_name::event::MouseScrollDelta::PixelDelta(delta) => {
                                ui.next(&config, &mut sketch, powdermilk_biscuits::event::Event::ScrollZoom(delta.y as f32));
                            }
                        }

                        #window.request_redraw();
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event: #windowing_crate_name::event::WindowEvent::MouseInput { state, button, .. },
                        ..
                    } => {
                        let button = #backend_crate_name::#mouse_button_translation(button);
                        let state = #backend_crate_name::#key_state_translation(state);

                        match (button, state) {
                            (primary, powdermilk_biscuits::event::ElementState::Pressed) if primary == config.primary_button => {
                                ui.next(&config, &mut sketch, powdermilk_biscuits::event::Event::MouseDown(button));
                            }
                            (primary, powdermilk_biscuits::event::ElementState::Released) if primary == config.primary_button => {
                                ui.next(&config, &mut sketch, powdermilk_biscuits::event::Event::MouseUp(button));
                            }
                            (pan, powdermilk_biscuits::event::ElementState::Pressed) if pan == config.pan_button => {
                                ui.next(&config, &mut sketch, powdermilk_biscuits::event::Event::StartPan);
                            }
                            (pan, powdermilk_biscuits::event::ElementState::Released) if pan == config.pan_button => {
                                ui.next(&config, &mut sketch, powdermilk_biscuits::event::Event::EndPan);
                            }
                            _ => {}
                        }

                        ui.prev_device = powdermilk_biscuits::Device::Mouse;
                        #window.request_redraw();
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event: #windowing_crate_name::event::WindowEvent::CursorMoved { position, .. },
                        ..
                    } => {
                        ui.next(
                            &config,
                            &mut sketch,
                            powdermilk_biscuits::event::Event::MouseMove(#backend_crate_name::physical_pos_to_pixel_pos(position)),
                        );
                        ui.prev_device = powdermilk_biscuits::Device::Mouse;

                        if config.use_mouse_for_pen {
                            if cursor_visible {
                                cursor_visible = false;
                                #window.set_cursor_visible(false);
                            }
                            #window.request_redraw();
                        } else if !cursor_visible {
                            cursor_visible = true;
                            #window.set_cursor_visible(true);
                        }

                        if ui.state.redraw() {
                            #window.request_redraw();
                        }
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event:
                            #windowing_crate_name::event::WindowEvent::Touch(
                                touch @ #windowing_crate_name::event::Touch {
                                    phase,
                                    pen_info: Some(_),
                                    ..
                                },
                            ),
                        ..
                    } => {
                        let touch = #backend_crate_name::#touch_translation(touch);

                        match phase {
                            #windowing_crate_name::event::TouchPhase::Started => ui.next(&config, &mut sketch, powdermilk_biscuits::event::Event::PenDown(touch)),
                            #windowing_crate_name::event::TouchPhase::Moved => ui.next(&config, &mut sketch, powdermilk_biscuits::event::Event::PenMove(touch)),
                            #windowing_crate_name::event::TouchPhase::Ended | #windowing_crate_name::event::TouchPhase::Cancelled => {
                                ui.next(&config, &mut sketch, powdermilk_biscuits::event::Event::PenUp(touch))
                            }
                        }

                        ui.prev_device = powdermilk_biscuits::Device::Pen;

                        if cursor_visible {
                            cursor_visible = false;
                            #window.set_cursor_visible(false);
                        }

                        #window.request_redraw();
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event:
                            #windowing_crate_name::event::WindowEvent::Touch(
                                touch @ #windowing_crate_name::event::Touch {
                                    phase,
                                    pen_info: None,
                                    ..
                                },
                            ),
                        ..
                    } => {
                        let touch = #backend_crate_name::#touch_translation(touch);
                        ui.next(
                            &config,
                            &mut sketch,
                            match phase {
                                #windowing_crate_name::event::TouchPhase::Started => powdermilk_biscuits::event::Event::Touch(touch),
                                #windowing_crate_name::event::TouchPhase::Moved => powdermilk_biscuits::event::Event::TouchMove(touch),
                                #windowing_crate_name::event::TouchPhase::Ended | #windowing_crate_name::event::TouchPhase::Cancelled => powdermilk_biscuits::event::Event::Release(touch),
                            },
                        );

                        ui.prev_device = powdermilk_biscuits::Device::Touch;

                        if cursor_visible {
                            cursor_visible = false;
                            #window.set_cursor_visible(false);
                        }

                        #window.request_redraw();
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event: #windowing_crate_name::event::WindowEvent::Moved(location),
                        ..
                    } => {
                        config.move_window(location.x, location.y);
                    }

                    #windowing_crate_name::event::Event::WindowEvent {
                        event:
                            #windowing_crate_name::event::WindowEvent::Resized(new_size)
                            | #windowing_crate_name::event::WindowEvent::ScaleFactorChanged {
                                new_inner_size: &mut new_size,
                                ..
                            },
                        ..
                    } => #resize,

                    #windowing_crate_name::event::Event::MainEventsCleared => {
                        use powdermilk_biscuits::event::Keycode::*;

                        match (ui.path.as_ref(), ui.modified) {
                            (Some(path), true) => {
                                let title = format!("{} (modified)", path.display());
                                #window.set_title(title.as_str());
                            }
                            (Some(path), false) => #window.set_title(&path.display().to_string()),
                            (None, true) => #window.set_title(powdermilk_biscuits::TITLE_MODIFIED),
                            (None, false) => #window.set_title(powdermilk_biscuits::TITLE_UNMODIFIED),
                        }
                    }

                    #windowing_crate_name::event::Event::RedrawRequested(_) => #render,

                    _ => {}
                }

                log::trace!("{:?}", flow);
            });
        }
    }.into()
}
