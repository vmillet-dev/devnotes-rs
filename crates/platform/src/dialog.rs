//! Choosing a file to read, or where to write one. An end-to-end scenario cannot reach the
//! system's dialog, so the `e2e` flavour answers the path the scenario fixed instead.

mod system;

use std::path::PathBuf;
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

/// What the screens ask: the system's dialog, or in the `e2e` flavour the scenario's.
/// `on_missing` hears a desktop with no dialog to show, which otherwise reads as a cancel.
pub fn system(on_missing: impl Fn() + Send + Sync + 'static) -> Arc<dyn FileDialog + Send + Sync> {
    #[cfg(feature = "e2e")]
    if let Some(fixed) = from_scenario() {
        return Arc::new(fixed);
    }
    system::system(on_missing)
}

/// Answers every question like a cancel: tests that ask none.
pub struct NoDialog;

impl FileDialog for NoDialog {
    fn open(&self) -> Option<PathBuf> {
        None
    }

    fn save(&self, _name: &str) -> Option<PathBuf> {
        None
    }
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
