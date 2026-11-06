//! Choosing a file to read, or where to write one. An end-to-end scenario cannot reach the
//! system's dialog, so the `e2e` flavour answers the path the scenario fixed instead.

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub trait FileDialog {
    /// A file to read: `None` when the person cancelled.
    fn open(&self) -> Option<PathBuf>;
    /// A file to read among those of `extensions`.
    fn open_of(&self, _extensions: &[&str]) -> Option<PathBuf> {
        self.open()
    }
    /// Where to write a file proposed as `name`: `None` when the person cancelled.
    fn save(&self, name: &str) -> Option<PathBuf>;
}

/// The system's dialog. ⚠️ It blocks until answered: never called on the window's thread.
pub struct System;

impl FileDialog for System {
    fn open(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_file()
    }

    fn open_of(&self, extensions: &[&str]) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter(extensions[0], extensions)
            .pick_file()
    }

    fn save(&self, name: &str) -> Option<PathBuf> {
        let dialog = rfd::FileDialog::new().set_file_name(name);
        let extension = Path::new(name).extension().and_then(|it| it.to_str());
        match extension {
            Some(extension) => dialog.add_filter(extension, &[extension]).save_file(),
            None => dialog.save_file(),
        }
    }
}

/// What the screens ask: the system's dialog, or in the `e2e` flavour the scenario's.
pub fn system() -> Arc<dyn FileDialog + Send + Sync> {
    #[cfg(feature = "e2e")]
    if let Some(fixed) = from_scenario() {
        return Arc::new(fixed);
    }
    Arc::new(System)
}

/// Answers one path to whatever is asked: a scenario's file, or a test's.
pub struct Fixed(pub PathBuf);

impl FileDialog for Fixed {
    fn open(&self) -> Option<PathBuf> {
        Some(self.0.clone())
    }

    fn save(&self, _name: &str) -> Option<PathBuf> {
        Some(self.0.clone())
    }
}

/// Where a scenario names the file every dialog answers.
pub const E2E_DIALOG: &str = "DEVNOTES_E2E_DIALOG";

/// The `e2e` flavour's dialog, when the scenario fixed a path.
#[cfg(feature = "e2e")]
pub fn from_scenario() -> Option<Fixed> {
    std::env::var_os(E2E_DIALOG).map(|path| Fixed(PathBuf::from(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_dialog_answers_its_path_to_both_questions() {
        let dialog = Fixed(PathBuf::from("export.devnotes"));

        assert_eq!(dialog.open(), Some(PathBuf::from("export.devnotes")));
        assert_eq!(
            dialog.save("notes.devnotes"),
            Some(PathBuf::from("export.devnotes"))
        );
    }
}
