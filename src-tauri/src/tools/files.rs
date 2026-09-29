//! A file a tool reads or writes: by its path, so its bytes never cross the bridge.

use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::Path;

use serde::Serialize;
use specta::Type;

/// Past this, a tool that has to send what it made of a file back across the bridge refuses it.
pub(crate) const MAX_SENT_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FileProblem {
    NotFound,
    TooLarge,
    Unreadable,
    Unwritable,
}

fn problem(error: &std::io::Error) -> FileProblem {
    match error.kind() {
        ErrorKind::NotFound => FileProblem::NotFound,
        _ => FileProblem::Unreadable,
    }
}

pub(crate) fn open(path: &str) -> Result<File, FileProblem> {
    File::open(path).map_err(|error| problem(&error))
}

/// Whole, and refused past `MAX_SENT_BYTES` rather than cut.
pub(crate) fn read_limited(path: &str) -> Result<Vec<u8>, FileProblem> {
    let mut bytes = Vec::new();
    open(path)?
        .take(MAX_SENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| problem(&error))?;
    if bytes.len() as u64 > MAX_SENT_BYTES {
        return Err(FileProblem::TooLarge);
    }
    Ok(bytes)
}

pub(crate) fn write(path: &str, bytes: &[u8]) -> Result<(), FileProblem> {
    std::fs::write(path, bytes).map_err(|_| FileProblem::Unwritable)
}

/// What the tool shows of the file: its name, never the folders above it.
pub(crate) fn name_of(path: &str) -> String {
    Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_read_whole_and_named_without_its_folders() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("clé.bin");
        std::fs::write(&path, [1, 2, 3]).unwrap();
        let path = path.to_string_lossy().into_owned();

        assert_eq!(read_limited(&path), Ok(vec![1, 2, 3]));
        assert_eq!(name_of(&path), "clé.bin");
    }

    #[test]
    fn a_missing_file_says_so() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("absent").to_string_lossy().into_owned();

        assert_eq!(read_limited(&path), Err(FileProblem::NotFound));
    }

    #[test]
    fn a_file_too_large_to_send_is_refused() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("big.bin");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_SENT_BYTES + 1).unwrap();

        assert_eq!(
            read_limited(&path.to_string_lossy()),
            Err(FileProblem::TooLarge)
        );
    }
}
