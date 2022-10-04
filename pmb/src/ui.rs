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
        .set_title(s!(&MigrateWarningTitle))
        .set_buttons(rfd::MessageButtons::YesNo)
        .set_description(s!(&MigrateWarningMessage))
        .show()
}

pub fn error(text: &str) -> rfd::MessageDialogResult {
    rfd::MessageDialog::new()
        .set_title(s!(&ErrorTitle))
        .set_description(text)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show()
}

pub fn ask_to_save(why: &str) -> rfd::MessageDialogResult {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title(s!(&UnsavedChangesTitle))
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
        .set_title(s!(&OpenTitle))
        .add_filter("PMB", &["pmb"])
        .pick_file()
}

pub fn egui<C: CoordinateSystem, S: StrokeBackend>(
    ctx: &egui::Context,
    sketch: &mut Sketch<S>,
    widget: &mut widget::SketchWidget<C>,
    config: &mut Config,
) {
    use egui::{Color32, ComboBox, Grid, Id, Sense, SidePanel, Slider, TopBottomPanel};

    TopBottomPanel::top("menu bar")
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(s!(&RealHotItem));
                ui.menu_button(s!(&FileMenu), |ui| {
                    let _ = ui.button(s!(&FileSave));
                    let _ = ui.button(s!(&FileOpen));
                    let _ = ui.button(s!(&FileNew));
                });
                ui.menu_button(s!(&EditMenu), |ui| {
                    let _ = ui.button(s!(&EditUndo));
                    let _ = ui.button(s!(&EditRedo));
                });
            });
        });

    SidePanel::left("side").resizable(false).show(ctx, |ui| {
        Grid::new("colors").show(ui, |ui| {
            ui.label(s!(&ClearColor));
            ui.color_edit_button_rgb(&mut sketch.bg_color);
            ui.end_row();
            ui.label(s!(&StrokeColor));
            ui.color_edit_button_rgb(&mut sketch.fg_color);
            ui.end_row();
        });

        ui.label(s!(&BrushSize));

        let brush_size_slider = ui.add(Slider::new(
            &mut widget.brush_size,
            crate::MIN_BRUSH..=crate::MAX_BRUSH,
        ));

        if brush_size_slider.hovered() || brush_size_slider.is_pointer_button_down_on() {
            egui::show_tooltip(ui.ctx(), Id::new("tt"), |ui| {
                let size = widget.brush_size as f32;
                let (_id, space) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
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
    });

    TopBottomPanel::top("top").resizable(false).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.radio_value(&mut widget.active_tool, Tool::Pen, s!(&Pen));
            ui.radio_value(&mut widget.active_tool, Tool::Eraser, s!(&Eraser));
            ui.radio_value(&mut widget.active_tool, Tool::Pan, s!(&Pan));
            ui.checkbox(&mut config.use_mouse_for_pen, s!(&UseMouseForPen));

            ComboBox::from_label(s!(&ToolForGesture1))
                .selected_text(match config.tool_for_gesture_1 {
                    Tool::Pen => s!(&Pen), // TODO helper for this?
                    Tool::Pan => s!(&Pan),
                    Tool::Eraser => s!(&Eraser),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut config.tool_for_gesture_1, Tool::Pen, s!(&Pen));
                    ui.selectable_value(&mut config.tool_for_gesture_1, Tool::Eraser, s!(&Eraser));
                    ui.selectable_value(&mut config.tool_for_gesture_1, Tool::Pan, s!(&Pan));
                });

            let slider =
                Slider::new(&mut sketch.zoom, crate::MIN_ZOOM..=crate::MAX_ZOOM).text(s!(&Zoom));

            if ui.add(slider).changed() {
                sketch.update_visible_strokes::<C>(widget.width, widget.height);
                sketch.update_stroke_primitive();
            };
        });
    });
}

pub fn read_file<S: StrokeBackend, C: CoordinateSystem>(
    widget: &mut widget::SketchWidget<C>,
    path: Option<impl AsRef<std::path::Path>>,
    sketch: &mut Sketch<S>,
) -> Result<(), PmbError> {
    use crate::{
        migrate,
        migrate::{UpgradeType, Version},
    };

    // if we are modified
    if widget.modified {
        // ask to save first
        if !ask_to_save_then_save(widget, sketch, s!(&AskToSaveBeforeOpening))
            .problem(s!(CouldNotSaveFile))?
        {
            return Ok(());
        }
    }

    // if we were passed a path, use that, otherwise ask for one
    log::info!("finding where to read from");
    let path = match path
        .map(|path| path.as_ref().to_path_buf())
        .or_else(open_dialog)
    {
        Some(path) => path,
        None => {
            return Ok(());
        }
    };

    // open the new file
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            log::info!("using a new file");
            // if it doesn't exist don't try to read it
            widget.path = Some(path);
            widget.modified = true;
            return Ok(());
        }
        Err(err) => Err(PmbError::from(err))?,
    };

    // read the new file
    let disk: Sketch<S> = match migrate::read(file).problem(format!("{}", path.display())) {
        Ok(disk) => disk,

        Err(PmbError {
            kind: ErrorKind::VersionMismatch(version),
            ..
        }) => {
            log::warn!("version mismatch, got {version} want {}", Version::CURRENT);

            match Version::upgrade_type(version) {
                UpgradeType::Smooth => migrate::from(version, &path)?,

                UpgradeType::Rocky => match prompt_migrate() {
                    rfd::MessageDialogResult::Yes => {
                        let disk = migrate::from(version, &path)?;

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

                        return Ok(());
                    }

                    _ => Sketch::default(),
                },

                UpgradeType::Incompatible => {
                    return Err(PmbError::new(ErrorKind::IncompatibleVersion(version)));
                }
            }
        }

        err => err?,
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

    log::info!(
        "success, read from {}",
        widget.path.as_ref().unwrap().display()
    );

    Ok(())
}

/// returns whether you should continue with whatever state-destroying operation you want to do
pub fn ask_to_save_then_save<S: StrokeBackend, C: CoordinateSystem>(
    widget: &mut widget::SketchWidget<C>,
    sketch: &Sketch<S>,
    why: &str,
) -> Result<bool, PmbError> {
    use crate::migrate;

    log::info!("asking to save {why:?}");
    match (ask_to_save(why), widget.path.as_ref()) {
        // if they say yes and the file we're editing has a path
        (rfd::MessageDialogResult::Yes, Some(path)) => {
            log::info!("writing as {}", path.display());
            migrate::write(path, sketch).problem(format!("{}", path.display()))?;
            widget.modified = false;
            Ok(true)
        }

        // they say yes and the file doesn't have a path yet
        (rfd::MessageDialogResult::Yes, None) => {
            log::info!("asking where to save");
            // ask where to save it
            match save_dialog(s!(&SaveUnnamedFile), None) {
                Some(new_filename) => {
                    log::info!("writing as {}", new_filename.display());
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
) -> Result<(), PmbError> {
    use crate::migrate;

    if let Some(path) = widget.path.as_ref() {
        migrate::write(path, sketch).problem(format!("{}", path.display()))?;
        widget.modified = false;
    } else if let Some(path) = save_dialog(s!(&SaveUnnamedFile), None) {
        let problem = format!("{}", path.display());
        widget.path = Some(path);
        migrate::write(widget.path.as_ref().unwrap(), sketch).problem(problem)?;
        widget.modified = false;
    }

    log::info!("saved file as {}", widget.path.as_ref().unwrap().display());
    Ok(())
}

fn new_file<C: CoordinateSystem, S: StrokeBackend>(
    widget: &mut widget::SketchWidget<C>,
    sketch: &mut Sketch<S>,
) -> Result<(), PmbError> {
    if widget.modified && !ask_to_save_then_save(widget, sketch, s!(&AskToSaveBeforeOpening))? {
        return Ok(());
    }

    *sketch = Sketch::empty();
    widget.path = None;
    widget.modified = false;

    Ok(())
}
