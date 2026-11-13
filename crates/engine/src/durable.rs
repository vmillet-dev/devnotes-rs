//! A file that survives a power cut: written beside its place, flushed to the disk, renamed over
//! the old one, then its folder flushed so the rename itself is kept. After a cut the old file
//! or the new one is there, whole — without the flush, a renamed file can be found empty.
//!
//! ⚠️ Not for the preferences: rewritten at each change on the db queue, a flush each time
//! (30–190 ms on a CI disk) holds every job behind it, and an empty one reads as the defaults.

use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};

use uuid::Uuid;

/// `bytes` in `path`'s place. The staged file is removed when anything fails.
pub(crate) fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let staged = beside(path);
    std::fs::write(&staged, bytes)
        .and_then(|()| replace(&staged, path))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&staged);
        })
}

/// `from` copied into `to`'s place, as [`write`] would.
pub(crate) fn copy(from: &Path, to: &Path) -> io::Result<()> {
    let staged = beside(to);
    std::fs::copy(from, &staged)
        .and_then(|_| replace(&staged, to))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&staged);
        })
}

/// `staged`, already written in full, put in `path`'s place. The caller removes it on failure.
pub(crate) fn replace(staged: &Path, path: &Path) -> io::Result<()> {
    OpenOptions::new().write(true).open(staged)?.sync_all()?;
    std::fs::rename(staged, path)?;
    #[cfg(unix)]
    sync_folder(path)?;
    Ok(())
}

fn beside(path: &Path) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!(".{name}.{}.tmp", Uuid::new_v4()))
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

    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_file_is_replaced_whole_and_nothing_is_left_beside_it() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("vault.json");
        std::fs::write(&path, b"old").unwrap();

        write(&path, b"new").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(names(folder.path()), ["vault.json"]);
    }

    #[test]
    fn a_failed_rename_leaves_no_staged_file() {
        let folder = tempfile::tempdir().unwrap();
        let taken = folder.path().join("registry.json");
        std::fs::create_dir(&taken).unwrap();
        std::fs::write(taken.join("inside"), b"x").unwrap();
        let source = folder.path().join("source");
        std::fs::write(&source, b"copied").unwrap();

        assert!(write(&taken, b"new").is_err());
        assert!(copy(&source, &taken).is_err());

        assert_eq!(names(folder.path()), ["registry.json", "source"]);
    }

    #[test]
    fn a_copy_takes_the_place_of_the_file() {
        let folder = tempfile::tempdir().unwrap();
        let source = folder.path().join("library.db");
        let target = folder.path().join("restored.db");
        std::fs::write(&source, b"pages").unwrap();

        copy(&source, &target).unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"pages");
        assert_eq!(names(folder.path()), ["library.db", "restored.db"]);
    }
}
