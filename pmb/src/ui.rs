use winit::event::MouseButton;

use crate::{
    config::Config,
    error::{ErrorKind, PmbError, PmbErrorExt},
    s, CoordinateSystem, Sketch, StrokeBackend, Tool,
};
use std::path::{Path, PathBuf};

pub mod undo;
pub mod widget;

fn prompt_migrate() -> rfd::MessageDialogResult {
    rfd::MessageDialog::new()
        .set_title(s!(&MboxTitleMigrateWarning))
        .set_buttons(rfd::MessageButtons::YesNo)
        .set_description(s!(&MboxMessageMigrateWarning))
        .show()
}

pub fn error(text: &str) -> rfd::MessageDialogResult {
    rfd::MessageDialog::new()
        .set_title(s!(&MboxTitleError))
        .set_description(text)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show()
}

pub fn ask_to_save(why: &str) -> rfd::MessageDialogResult {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title(s!(&MboxTitleUnsavedChanges))
        .set_description(why)
        .set_buttons(rfd::MessageButtons::YesNoCancel)
        .show()
}

pub fn save_dialog(title: &str, filename: Option<&Path>) -> Option<PathBuf> {
    let filename = filename
        .and_then(|path| path.file_name())
        .and_then(|os| os.to_str())
        .unwrap_or("");

    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("PMB", &["pmb"])
        .set_file_name(filename)
        .save_file()
}

pub fn open_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title(s!(&MboxTitleOpen))
        .add_filter("PMB", &["pmb"])
        .pick_file()
}

fn settings_window<S: StrokeBackend>(
    ui: &egui::Ui,
    ctx: &egui::Context,
    settings_id: egui::Id,
    config: &mut Config,
    sketch: &mut Sketch<S>,
    mut settings_open: bool,
) {
    use egui::*;

    Window::new(s!(&WindowTitleConfig))
        .open(&mut settings_open)
        .show(ctx, |ui| {
            // if this were going to spawn a separate window, we would need an event loop
            // proxy to send configuration changes back to the main thread

            Grid::new("input settings").show(ui, |ui| {
                macro_rules! tfg {
                    ($num:literal) => {
                        paste::paste! {
                            ui.label(s!(&[<ConfigLabelToolForGesture $num>]));
                            ComboBox::new(concat!("tool for gesture ", stringify!($num)), "")
                                .selected_text(match config.[<tool_for_gesture_ $num>] {
                                    Tool::Pen => s!(&RadioLabelToolPen),
                                    Tool::Eraser => s!(&RadioLabelToolEraser),
                                    Tool::Pan => s!(&RadioLabelToolPan),
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut config.[<tool_for_gesture_ $num>],
                                        Tool::Pen,
                                        s!(&RadioLabelToolPen),
                                    );
                                    ui.selectable_value(
                                        &mut config.[<tool_for_gesture_ $num>],
                                        Tool::Eraser,
                                        s!(&RadioLabelToolEraser),
                                    );
                                    ui.selectable_value(
                                        &mut config.[<tool_for_gesture_ $num>],
                                        Tool::Pan,
                                        s!(&RadioLabelToolPan),
                                    );
                                });
                            ui.end_row();
                        }
                    };
                }

                tfg!(2);
                tfg!(3);
                tfg!(4);

                ui.label(s!(&ConfigLabelUseMouseForPen));
                ui.checkbox(&mut config.use_mouse_for_pen, "");
                ui.end_row();

                ui.label(s!(&ConfigLabelStylusMayBeInverted));
                ui.checkbox(&mut config.stylus_may_be_inverted, "");
                ui.end_row();

                ui.label(s!(&ConfigLabelPrimaryMouseButton));
                ComboBox::new("primary button", "")
                    .selected_text(match config.primary_button {
                        MouseButton::Left => s!(&ConfigOptionButtonLeftMouse),
                        MouseButton::Middle => s!(&ConfigOptionButtonMiddleMouse),
                        MouseButton::Right => s!(&ConfigOptionButtonRightMouse),
                        _ => s!(&Placeholder),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut config.primary_button,
                            MouseButton::Left,
                            s!(&ConfigOptionButtonLeftMouse),
                        );
                        ui.selectable_value(
                            &mut config.primary_button,
                            MouseButton::Middle,
                            s!(&ConfigOptionButtonMiddleMouse),
                        );
                        ui.selectable_value(
                            &mut config.primary_button,
                            MouseButton::Right,
                            s!(&ConfigOptionButtonRightMouse),
                        );
                    });
                ui.end_row();
            });

            ui.separator();

            Grid::new("visual fx").show(ui, |ui| {
                ui.label(s!(&ConfigLabelBackgroundColor));
                ui.color_edit_button_rgb(&mut sketch.bg_color);
                ui.end_row();

                ui.label(s!(&ConfigLabelDarkMode));
                let before = config.dark_mode;
                ui.checkbox(&mut config.dark_mode, "");
                if before != config.dark_mode {
                    ctx.set_visuals(if config.dark_mode {
                        egui::Visuals::dark()
                    } else {
                        egui::Visuals::light()
                    });
                }
                ui.end_row();

                ui.label(s!(&ConfigLabelStartMaximized));
                ui.checkbox(&mut config.window_start_maximized, "");
            });

            ui.separator();
            ctx.settings_ui(ui);
        });

    ui.memory().data.insert_temp(settings_id, settings_open);
}

pub fn egui<C: CoordinateSystem, S: StrokeBackend>(
    ctx: &egui::Context,
    sketch: &mut Sketch<S>,
    widget: &mut widget::SketchWidget<C>,
    config: &mut Config,
) {
    use egui::*;

    TopBottomPanel::top("top").resizable(false).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading(s!(&RealHotItem));

            let settings_id = ui.make_persistent_id("settings window");
            let mut settings_open = ui
                .memory()
                .data
                .get_temp::<bool>(settings_id)
                .unwrap_or(false);

            ui.menu_button(s!(&MenuLabelFile), |ui| {
                if ui.button(s!(&MenuItemFileNew)).clicked() {
                    new_file(widget, sketch);
                    ui.close_menu();
                }
                if ui.button(s!(&MenuItemFileOpen)).clicked() {
                    read_file(widget, None::<&str>, sketch);
                    ui.close_menu();
                }

                if if widget.path.is_none() {
                    ui.button(s!(&MenuItemFileSaveUnnamed)).clicked()
                } else {
                    ui.button(s!(&MenuItemFileSave)).clicked()
                } {
                    save_file(widget, sketch);
                    ui.close_menu();
                }

                ui.separator();

                if ui.button(s!(&MenuItemFileSettings)).clicked() {
                    settings_open = true;
                    ui.close_menu();
                }

                ui.separator();

                if ui
                    .button(if widget.modified {
                        s!(&MenuItemFileQuitModified)
                    } else {
                        s!(&MenuItemFileQuitUnmodified)
                    })
                    .clicked()
                {
                    // ehhh
                    widget.next(config, sketch, crate::event::Event::Exit);
                }
            });

            if settings_open {
                settings_window(ui, ctx, settings_id, config, sketch, settings_open);
            }

            ui.menu_button(s!(&MenuLabelEdit), |ui| {
                if ui.button(s!(&MenuItemEditUndo)).clicked() {
                    widget.undo(sketch);
                }

                if ui.button(s!(&MenuItemEditRedo)).clicked() {
                    widget.redo(sketch);
                }
            });

            ui.separator();

            ui.radio_value(&mut widget.active_tool, Tool::Pen, s!(&RadioLabelToolPen));
            ui.radio_value(
                &mut widget.active_tool,
                Tool::Eraser,
                s!(&RadioLabelToolEraser),
            );
            ui.radio_value(&mut widget.active_tool, Tool::Pan, s!(&RadioLabelToolPan));

            let brush_size_slider = ui.add(
                Slider::new(&mut widget.brush_size, crate::MIN_BRUSH..=crate::MAX_BRUSH)
                    .text(s!(&SliderLabelBrushSize)),
            );

            if brush_size_slider.hovered() || brush_size_slider.is_pointer_button_down_on() {
                egui::show_tooltip(ui.ctx(), Id::new("tt"), |ui| {
                    let size = widget.brush_size as f32;
                    let (_id, space) =
                        ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
                    ui.painter().circle_stroke(
                        space.rect.center(),
                        size / 2.,
                        egui::Stroke {
                            width: 0.5,
                            color: Color32::WHITE,
                        },
                    );
                });
            }

            ui.color_edit_button_rgb(&mut sketch.fg_color);
            ui.label(s!(&ColorPickerLabelStrokeColor));

            ui.separator();

            let slider = Slider::new(&mut sketch.zoom, crate::MIN_ZOOM..=crate::MAX_ZOOM)
                .text(s!(&SliderLabelZoom));

            if ui.add(slider).changed() {
                sketch.update_visible_strokes::<C>(widget.width, widget.height);
                sketch.update_stroke_primitive();
            };
        });
    });

    if config.debug_show_info {
        Window::new("debug info").show(ctx, |ui| {
            Grid::new("debug info grid").show(ui, |ui| {
                ui.label("mouse pos");
                ui.label(format!("{}", widget.input.cursor_pos()));
                ui.end_row();

                ui.label("stylus pos");
                ui.label(format!("{}", widget.stylus.pos));
                ui.end_row();

                ui.label("origin");
                ui.label(format!("{}", sketch.origin));
                ui.end_row();

                ui.label("state");
                ui.label(format!("{:?}", widget.state));
                ui.end_row();
            });
        });
    }
}

pub fn read_file<S: StrokeBackend, C: CoordinateSystem>(
    widget: &mut widget::SketchWidget<C>,
    path: Option<impl AsRef<std::path::Path>>,
    sketch: &mut Sketch<S>,
) {
    use crate::{
        migrate,
        migrate::{UpgradeType, Version},
    };

    // if we are modified
    if widget.modified {
        // ask to save first
        match ask_to_save_then_save(widget, sketch, s!(&MboxMessageAskToSaveBeforeOpening))
            .problem(s!(MboxMessageCouldNotSaveFile))
        {
            Ok(should_continue) => {
                if !should_continue {
                    return;
                }
            }

            err => err.problem(s!(MboxMessageCouldNotOpenFile)).display(),
        }
    }

    // if we were passed a path, use that, otherwise ask for one
    tracing::info!("finding where to read from");
    let path = match path
        .map(|path| path.as_ref().to_path_buf())
        .or_else(open_dialog)
    {
        Some(path) => path,
        None => {
            return;
        }
    };

    // open the new file
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tracing::info!("using a new file");
            // if it doesn't exist don't try to read it
            widget.path = Some(path);
            widget.modified = true;
            return;
        }
        Err(err) => {
            PmbError::from(err).display();
            return;
        }
    };

    // read the new file
    let disk: Sketch<S> = match migrate::read(file).problem(format!("{}", path.display())) {
        Ok(disk) => disk,

        Err(PmbError {
            kind: ErrorKind::VersionMismatch(version),
            ..
        }) => {
            tracing::warn!("version mismatch, got {version} want {}", Version::CURRENT);

            match Version::upgrade_type(version) {
                UpgradeType::Smooth => match migrate::from(version, &path) {
                    Ok(sketch) => sketch,
                    err => {
                        err.display();
                        return;
                    }
                },

                UpgradeType::Rocky => match prompt_migrate() {
                    rfd::MessageDialogResult::Yes => {
                        let disk = match migrate::from(version, &path) {
                            Ok(disk) => disk,
                            err => {
                                err.display();
                                return;
                            }
                        };

                        *sketch = disk;
                        sketch.force_update::<C>(
                            widget.width,
                            widget.height,
                            &mut widget.tesselator,
                            &widget.stroke_options,
                        );

                        // set the path to none so the user is prompted to save elsewhere
                        widget.path = None;
                        widget.modified = true;

                        return;
                    }

                    _ => Sketch::default(),
                },

                UpgradeType::Incompatible => {
                    PmbError::new(ErrorKind::IncompatibleVersion(version)).display();
                    return;
                }
            }
        }

        err => {
            err.display();
            return;
        }
    };

    *sketch = disk;
    sketch.force_update::<C>(
        widget.width,
        widget.height,
        &mut widget.tesselator,
        &widget.stroke_options,
    );

    widget.modified = false;
    widget.path = Some(path);
    widget.undo_stack.clear();

    tracing::info!(
        "success, read from {}",
        widget.path.as_ref().unwrap().display()
    );
}

/// returns whether you should continue with whatever state-destroying operation you want to do
pub fn ask_to_save_then_save<S: StrokeBackend, C: CoordinateSystem>(
    widget: &mut widget::SketchWidget<C>,
    sketch: &Sketch<S>,
    why: &str,
) -> Result<bool, PmbError> {
    use crate::migrate;

    tracing::info!("asking to save {why:?}");
    match (ask_to_save(why), widget.path.as_ref()) {
        // if they say yes and the file we're editing has a path
        (rfd::MessageDialogResult::Yes, Some(path)) => {
            tracing::info!("writing as {}", path.display());
            migrate::write(path, sketch).problem(format!("{}", path.display()))?;
            widget.modified = false;
            Ok(true)
        }

        // they say yes and the file doesn't have a path yet
        (rfd::MessageDialogResult::Yes, None) => {
            tracing::info!("asking where to save");
            // ask where to save it
            match save_dialog(s!(&MboxTitleSaveUnnamedFile), None) {
                Some(new_filename) => {
                    tracing::info!("writing as {}", new_filename.display());
                    // try write to disk
                    migrate::write(&new_filename, sketch)
                        .problem(format!("{}", new_filename.display()))?;
                    widget.modified = false;
                    Ok(true)
                }

                None => Ok(false),
            }
        }

        // they say no, don't write changes
        (rfd::MessageDialogResult::No, _) => Ok(true),

        _ => Ok(false),
    }
}

fn save_file<C: CoordinateSystem, S: StrokeBackend>(
    widget: &mut widget::SketchWidget<C>,
    sketch: &Sketch<S>,
) {
    use crate::migrate;

    if let Some(path) = widget.path.as_ref() {
        match migrate::write(path, sketch) {
            Ok(()) => {}
            err => {
                err.problem(format!("{}", path.display())).display();
                return;
            }
        }
        widget.modified = false;
    } else if let Some(path) = save_dialog(s!(&MboxTitleSaveUnnamedFile), None) {
        let problem = format!("{}", path.display());
        widget.path = Some(path);
        match migrate::write(widget.path.as_ref().unwrap(), sketch) {
            Ok(()) => {}
            err => {
                err.problem(problem).display();
                return;
            }
        }
        widget.modified = false;
    }

    tracing::info!("saved file as {}", widget.path.as_ref().unwrap().display());
}

fn new_file<C: CoordinateSystem, S: StrokeBackend>(
    widget: &mut widget::SketchWidget<C>,
    sketch: &mut Sketch<S>,
) {
    if widget.modified {
        match ask_to_save_then_save(widget, sketch, s!(&MboxMessageAskToSaveBeforeOpening)) {
            Ok(should_continue) => {
                if !should_continue {
                    return;
                }
            }

            err => err.problem(s!(MboxMessageCouldNotSaveFile)).display(),
        }
    }

    *sketch = Sketch::empty();
    widget.path = None;
    widget.modified = false;
}
