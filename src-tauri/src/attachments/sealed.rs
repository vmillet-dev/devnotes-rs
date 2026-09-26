//! The bytes on disk, sealed like the rows beside them.
//!
//! A sealed file is read whole to be opened: GCM authenticates a message only once all of it
//! has been seen. The 10 MiB cap on an attachment keeps that bounded.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

use crate::error::{FileContext, StorageError};
use crate::vault::key::Vault;

pub fn write_sealed(vault: &Vault, destination: &Path, bytes: &[u8]) -> Result<u64, StorageError> {
    let sealed = vault.seal_bytes(bytes)?;
    std::fs::write(destination, &sealed).context(destination.display())?;

    // The plaintext length: the record is what the interface shows.
    Ok(bytes.len() as u64)
}

pub fn read_sealed(vault: &Vault, path: &Path) -> Result<Vec<u8>, StorageError> {
    let sealed = std::fs::read(path).context(path.display())?;

    vault.open_bytes(&sealed)
}

/// Where a decrypted copy goes so the desktop can open it: inside the user's profile, never the
/// OS temporary directory, which every account on the machine shares. The profile rather than
/// the library, so one sweep catches every library's copies.
pub fn plaintext_directory<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, StorageError> {
    Ok(app
        .path()
        .app_data_dir()
        .context("app_data_dir")?
        .join(crate::layout::PLAINTEXT_COPIES))
}

/// Run on the way out and at every launch: a copy another application still holds cannot be
/// deleted, and a crash reaches neither path, so the guarantee is "gone by the next launch".
pub fn sweep_plaintext<R: Runtime>(app: &AppHandle<R>) {
    let Ok(directory) = plaintext_directory(app) else {
        return;
    };

    if let Err(error) = std::fs::remove_dir_all(&directory)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!(
            "Decrypted copies not swept from {}: {error}",
            directory.display()
        );
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::vault::key::Cost;

    fn vault() -> Vault {
        Vault::derive("a passphrase", b"0123456789abcdef", Cost::FOR_TESTS).unwrap()
    }

    /// The property the name has to have, asserted without waiting for a coincidence.
    #[test]
    fn two_scratch_directories_are_never_the_same_one() {
        let scratch = tempfile::tempdir().unwrap();
        let first = scratch.path().to_path_buf();
        let scratch = tempfile::tempdir().unwrap();
        let second = scratch.path().to_path_buf();

        assert_ne!(first, second);
    }

    #[test]
    fn a_sealed_file_reads_back_byte_for_byte() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let vault = vault();
        let target = directory.join("capture.png");

        let bytes = b"\x89PNG\r\n\x1a\n and whatever follows";
        write_sealed(&vault, &target, bytes).unwrap();

        assert_eq!(read_sealed(&vault, &target).unwrap(), bytes);
    }

    /// A screenshot of a credentials page must not be readable beside a sealed database.
    #[test]
    fn the_file_on_disk_carries_none_of_the_bytes_it_was_given() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let vault = vault();
        let target = directory.join("capture.png");

        write_sealed(&vault, &target, b"SECRET-TOKEN-abc123").unwrap();

        let raw = std::fs::read(&target).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("SECRET-TOKEN"));
    }

    /// The record shows what the user attached, not what the cipher added to it.
    #[test]
    fn the_size_reported_is_the_plaintext_size() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let vault = vault();

        let written = write_sealed(&vault, &directory.join("f"), &[0u8; 1000]).unwrap();

        assert_eq!(written, 1000);
    }

    #[test]
    fn a_file_sealed_under_another_key_will_not_open() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("capture.png");
        write_sealed(&vault(), &target, b"bytes").unwrap();

        let other =
            Vault::derive("another passphrase", b"0123456789abcdef", Cost::FOR_TESTS).unwrap();

        assert!(read_sealed(&other, &target).is_err());
    }

    #[test]
    fn a_truncated_file_is_refused_rather_than_half_read() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let vault = vault();
        let target = directory.join("capture.png");
        write_sealed(&vault, &target, b"a reasonably long run of bytes").unwrap();

        let sealed = std::fs::read(&target).unwrap();
        std::fs::write(&target, &sealed[..sealed.len() - 4]).unwrap();

        assert!(read_sealed(&vault, &target).is_err());
    }
}
