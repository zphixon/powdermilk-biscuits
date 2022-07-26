use std::path::{Path, PathBuf};

pub trait ToUi {
    fn error_dialog(self, text: &str) -> Self;
}

impl<T> ToUi for crate::Result<T> {
    fn error_dialog(self, text: &str) -> Self {
        match &self {
            Err(e) => {
                let text = format!("{text}\n{e}");
                error(&text);
            }
            _ => {}
        }

        self
    }
}

pub fn error(text: &str) -> rfd::MessageDialogResult {
    rfd::MessageDialog::new()
        .set_title("Error")
        .set_description(&text)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show()
}

pub fn ask_to_save(why: &str) -> rfd::MessageDialogResult {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Unsaved changes")
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
        .set_file_name(&filename)
        .save_file()
}

pub fn open_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Open file")
        .add_filter("PMB", &["pmb"])
        .pick_file()
}
