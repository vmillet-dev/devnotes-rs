//! Choosing a file to read, or where to write one. The system's dialog arrives with the first
//! screen that asks for one; an end-to-end scenario cannot reach it, so the `e2e` flavour
//! answers the path the scenario fixed instead.

use std::path::PathBuf;

pub trait FileDialog {
    /// A file to read: `None` when the person cancelled.
    fn open(&self) -> Option<PathBuf>;
    /// Where to write a file proposed as `name`: `None` when the person cancelled.
    fn save(&self, name: &str) -> Option<PathBuf>;
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
