//! A file that survives a power cut, for what the shell writes beside the core: written beside
//! its place, flushed, renamed over the old one, its folder flushed. The core's twin is
//! `engine::durable`: neither crate depends on the other.

use std::fs::OpenOptions;
use std::io;
use std::path::Path;

/// `bytes` in `path`'s place, its folder made if need be. The staged file is removed when
/// anything fails.
pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let staged = path.with_file_name(format!(".{name}.part"));
    std::fs::write(&staged, bytes)
        .and_then(|()| OpenOptions::new().write(true).open(&staged)?.sync_all())
        .and_then(|()| std::fs::rename(&staged, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&staged);
        })?;
    #[cfg(unix)]
    sync_folder(path)?;
    Ok(())
}

/// Unix wants the folder flushed for the rename to be kept; Windows keeps it in the file
/// system's journal.
#[cfg(unix)]
fn sync_folder(path: &Path) -> io::Result<()> {
    match path.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => std::fs::File::open(folder)?.sync_all(),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_replaced_whole_and_a_failed_rename_leaves_nothing_beside_it() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("window.json");
        write(&path, b"old").unwrap();
        write(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");

        let taken = folder.path().join("taken.json");
        std::fs::create_dir(&taken).unwrap();
        std::fs::write(taken.join("inside"), b"x").unwrap();
        assert!(write(&taken, b"new").is_err());

        let mut names: Vec<String> = std::fs::read_dir(folder.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["taken.json", "window.json"]);
    }
}
