//! The bytes on disk, sealed like the rows beside them.
//!
//! ⚠️ A sealed file is read whole to be opened: AES-GCM authenticates the message, and a
//! message is only authentic once all of it has been seen. That is the price of knowing a
//! file was not tampered with, and the 10 MiB cap on an attachment is what keeps it
//! bounded.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::error::{FileContext, StorageError};
use crate::vault::key::Vault;

pub fn seal_into(vault: &Vault, source: &Path, destination: &Path) -> Result<u64, StorageError> {
    let plain = std::fs::read(source).context(source.display())?;

    write_sealed(vault, destination, &plain)
}

pub fn write_sealed(vault: &Vault, destination: &Path, bytes: &[u8]) -> Result<u64, StorageError> {
    let sealed = vault.seal_bytes(bytes)?;
    std::fs::write(destination, &sealed).context(destination.display())?;

    // ⚠️ The plaintext length, not the file's: the record is what the interface shows, and
    // a size inflated by the nonce and the tag would be a lie the user could measure.
    Ok(bytes.len() as u64)
}

pub fn read_sealed(vault: &Vault, path: &Path) -> Result<Vec<u8>, StorageError> {
    let sealed = std::fs::read(path).context(path.display())?;

    vault.open_bytes(&sealed)
}

/// Where a decrypted copy goes so the desktop can open it.
///
/// ⚠️ The application's own data directory, deliberately, and **not** the OS temporary
/// one: that is a namespace shared with every account on the machine, where the copy would
/// be readable by all of them and where a directory somebody else created first would be
/// theirs rather than ours. This one sits inside the user's profile.
///
/// ⚠️ The **profile**, not the open library, which is the one path here that stayed put.
/// These copies are ephemeral and swept wholesale; one directory means one sweep catches
/// every library's leftovers, where a directory per library would leave the ones nobody
/// opened again untouched for good.
pub fn plaintext_directory(app: &AppHandle) -> Result<PathBuf, StorageError> {
    Ok(app
        .path()
        .app_data_dir()
        .context("app_data_dir")?
        .join(crate::layout::PLAINTEXT_COPIES))
}

/// ⚠️ Run on the way out *and* at every launch. A copy handed to another application
/// cannot be deleted while that application holds it, and a crash reaches neither path —
/// so the guarantee is "gone by the next launch", with the exit sweep narrowing the
/// window to the session for everything not still open.
pub fn sweep_plaintext(app: &AppHandle) {
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
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use crate::vault::key::Cost;

    fn vault() -> Vault {
        Vault::derive(
            "a passphrase",
            b"0123456789abcdef",
            Cost {
                memory_kib: 64,
                passes: 1,
                lanes: 1,
            },
        )
        .unwrap()
    }

    /// ⚠️ A counter, not the clock. `SystemTime::now()` is coarse enough on Windows that
    /// two of these tests — they run on parallel threads — drew the same nanosecond, took
    /// the same directory, and the first to finish removed it from under the second, which
    /// then failed to write into a path that no longer existed. The process id keeps two
    /// test binaries apart; the counter keeps two threads apart.
    fn scratch() -> std::path::PathBuf {
        static NEXT: AtomicU32 = AtomicU32::new(0);

        let directory = std::env::temp_dir().join(format!(
            "devnotes-sealed-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    /// The property the name has to have, asserted without waiting for a coincidence.
    #[test]
    fn two_scratch_directories_are_never_the_same_one() {
        let first = scratch();
        let second = scratch();

        assert_ne!(first, second);
        std::fs::remove_dir_all(&first).ok();
        std::fs::remove_dir_all(&second).ok();
    }

    #[test]
    fn a_sealed_file_reads_back_byte_for_byte() {
        let directory = scratch();
        let vault = vault();
        let target = directory.join("capture.png");

        let bytes = b"\x89PNG\r\n\x1a\n and whatever follows";
        write_sealed(&vault, &target, bytes).unwrap();

        assert_eq!(read_sealed(&vault, &target).unwrap(), bytes);
        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ The point: a screenshot of a credentials page must not be readable beside a
    /// database that is.
    #[test]
    fn the_file_on_disk_carries_none_of_the_bytes_it_was_given() {
        let directory = scratch();
        let vault = vault();
        let target = directory.join("capture.png");

        write_sealed(&vault, &target, b"SECRET-TOKEN-abc123").unwrap();

        let raw = std::fs::read(&target).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("SECRET-TOKEN"));
        std::fs::remove_dir_all(&directory).ok();
    }

    /// The record shows what the user attached, not what the cipher added to it.
    #[test]
    fn the_size_reported_is_the_plaintext_size() {
        let directory = scratch();
        let vault = vault();

        let written = write_sealed(&vault, &directory.join("f"), &[0u8; 1000]).unwrap();

        assert_eq!(written, 1000);
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_file_sealed_under_another_key_will_not_open() {
        let directory = scratch();
        let target = directory.join("capture.png");
        write_sealed(&vault(), &target, b"bytes").unwrap();

        let other = Vault::derive(
            "another passphrase",
            b"0123456789abcdef",
            Cost {
                memory_kib: 64,
                passes: 1,
                lanes: 1,
            },
        )
        .unwrap();

        assert!(read_sealed(&other, &target).is_err());
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_truncated_file_is_refused_rather_than_half_read() {
        let directory = scratch();
        let vault = vault();
        let target = directory.join("capture.png");
        write_sealed(&vault, &target, b"a reasonably long run of bytes").unwrap();

        let sealed = std::fs::read(&target).unwrap();
        std::fs::write(&target, &sealed[..sealed.len() - 4]).unwrap();

        assert!(read_sealed(&vault, &target).is_err());
        std::fs::remove_dir_all(&directory).ok();
    }
}
