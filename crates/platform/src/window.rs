//! The window's size and place, remembered between two launches. Never whether it was
//! visible: the window starts hidden and shows itself when its first frame is ready.

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// In points, the unit of egui's viewport.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Geometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
}

/// `None` for a first launch, and for a file that does not read: the default size is a fine
/// answer to either.
pub fn read(file: &Path) -> Option<Geometry> {
    let text = std::fs::read_to_string(file).ok()?;
    serde_json::from_str(&text)
        .inspect_err(|error| log::warn!("The window geometry is unreadable: {error}"))
        .ok()
}

/// Staged then renamed: a crash halfway leaves the previous geometry, never half of one.
pub fn write(file: &Path, geometry: &Geometry) -> io::Result<()> {
    if let Some(directory) = file.parent() {
        std::fs::create_dir_all(directory)?;
    }
    let staged = file.with_extension("json.part");
    std::fs::write(&staged, serde_json::to_vec_pretty(geometry)?)?;
    std::fs::rename(&staged, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLACED: Geometry = Geometry {
        x: 40.0,
        y: 30.0,
        width: 1_200.0,
        height: 800.0,
        maximized: false,
    };

    #[test]
    fn a_geometry_written_reads_back_the_same() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("nested").join("window.json");

        write(&file, &PLACED).unwrap();

        assert_eq!(read(&file), Some(PLACED));
        assert!(!file.with_extension("json.part").exists());
    }

    #[test]
    fn a_missing_or_damaged_file_reads_as_none() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("window.json");
        assert_eq!(read(&file), None);

        std::fs::write(&file, "{ not json").unwrap();
        assert_eq!(read(&file), None);
    }
}
