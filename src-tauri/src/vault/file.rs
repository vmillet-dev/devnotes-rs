//! The key file, beside the database.
//!
//! Readable before the library is: it holds the salt, the cost, and the library's key sealed
//! under the phrase. Never a key in the clear, and no check value — opening the wrapped key is
//! the check. Losing this file loses the library, as losing the passphrase does.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::key::{Cost, SALT_BYTES, Vault, fresh_salt};
use crate::error::StorageError;
use crate::layout::KEY_FILE;

/// Bumped when a file written today would stop being readable. Version 2 wraps the library's
/// key under the phrase, which is what lets the phrase change without touching a note.
const FORMAT_VERSION: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyFile {
    version: u32,
    kdf: Kdf,
    /// The library's key, sealed under the one derived from the passphrase: a wrong phrase
    /// fails to open it rather than unlocking onto a key nobody can reproduce.
    key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Kdf {
    algorithm: String,
    memory_kib: u32,
    passes: u32,
    lanes: u32,
    salt: String,
}

pub fn path_in(directory: &Path) -> PathBuf {
    directory.join(KEY_FILE)
}

pub fn exists(directory: &Path) -> bool {
    path_in(directory).is_file()
}

/// A library that has never been encrypted. Refuses to overwrite: the file there is the only
/// way into the notes beside it.
pub fn create(directory: &Path, passphrase: &str, cost: Cost) -> Result<Vault, StorageError> {
    let path = path_in(directory);
    if path.exists() {
        return Err(StorageError::Vault(
            "this library already has a key file".to_string(),
        ));
    }

    let vault = Vault::random()?;
    write_wrapped(&path, &vault, passphrase, cost)?;

    Ok(vault)
}

/// A new phrase over the same library: only the wrapping changes, so this cannot half-succeed.
///
/// ⚠️ Answers the key because the caller is not done: every retained backup holds a key file
/// still wrapped under the retired phrase. `backup::rewrap` is the other half, and
/// `vault::change` the one place that does both.
pub fn change_passphrase(
    directory: &Path,
    current: &str,
    next: &str,
    cost: Cost,
) -> Result<Vault, StorageError> {
    let vault = unlock(directory, current)?;
    write_wrapped(&path_in(directory), &vault, next, cost)?;

    Ok(vault)
}

/// A fresh salt every time: two phrases must not share a derivation.
pub(crate) fn write_wrapped(
    path: &Path,
    vault: &Vault,
    passphrase: &str,
    cost: Cost,
) -> Result<(), StorageError> {
    let salt = fresh_salt()?;
    let wrapping = Vault::derive(passphrase, &salt, cost)?;

    write_atomically(
        path,
        &KeyFile {
            version: FORMAT_VERSION,
            kdf: Kdf {
                algorithm: "argon2id".to_string(),
                memory_kib: cost.memory_kib,
                passes: cost.passes,
                lanes: cost.lanes,
                salt: BASE64.encode(salt),
            },
            key: BASE64.encode(vault.wrapped_with(&wrapping)?),
        },
    )
}

/// Answers [`StorageError::WrongPassphrase`] and nothing more detailed.
pub fn unlock(directory: &Path, passphrase: &str) -> Result<Vault, StorageError> {
    let path = path_in(directory);
    let json = std::fs::read_to_string(&path)
        .map_err(|error| StorageError::Vault(format!("{}: {error}", path.display())))?;

    let file: KeyFile = serde_json::from_str(&json)
        .map_err(|error| StorageError::Vault(format!("unreadable key file: {error}")))?;

    if file.version > FORMAT_VERSION {
        return Err(StorageError::Vault(format!(
            "key file version {}, this version of DevNotes reads up to {FORMAT_VERSION}",
            file.version
        )));
    }
    if file.kdf.algorithm != "argon2id" {
        return Err(StorageError::Vault(format!(
            "key derived with \"{}\", which this version of DevNotes cannot reproduce",
            file.kdf.algorithm
        )));
    }

    let salt = BASE64
        .decode(&file.kdf.salt)
        .map_err(|_| StorageError::Vault("the salt is not base64".to_string()))?;
    if salt.len() != SALT_BYTES {
        return Err(StorageError::Vault(
            "the salt is the wrong size".to_string(),
        ));
    }

    let cost = bounded(Cost {
        memory_kib: file.kdf.memory_kib,
        passes: file.kdf.passes,
        lanes: file.kdf.lanes,
    })?;
    let wrapping = Vault::derive(passphrase, &salt, cost)?;
    let wrapped = BASE64
        .decode(&file.key)
        .map_err(|_| StorageError::Vault("the wrapped key is not base64".to_string()))?;

    Vault::unwrapped_with(&wrapping, &wrapped).map_err(|_| StorageError::WrongPassphrase)
}

/// Sixteen times the shipped memory cost, far past any raise worth making.
const MAX_MEMORY_KIB: u32 = 1024 * 1024;
const MAX_PASSES: u32 = 16;
const MAX_LANES: u32 = 16;

/// ⚠️ The key file is the one input an attacker can write: an unbounded cost read out of it is
/// a gigabyte allocation, or an hour of hashing, at every unlock.
fn bounded(cost: Cost) -> Result<Cost, StorageError> {
    let fits = cost.memory_kib <= MAX_MEMORY_KIB
        && (1..=MAX_PASSES).contains(&cost.passes)
        && (1..=MAX_LANES).contains(&cost.lanes);

    if fits {
        Ok(cost)
    } else {
        Err(StorageError::Vault(format!(
            "key parameters out of range: {} KiB, {} passes, {} lanes",
            cost.memory_kib, cost.passes, cost.lanes
        )))
    }
}

/// Staged then renamed: a plain write truncates first, and a half-written key file is a
/// library nobody opens again.
fn write_atomically(path: &Path, file: &KeyFile) -> Result<(), StorageError> {
    let json = serde_json::to_string_pretty(file)
        .map_err(|error| StorageError::Vault(error.to_string()))?;

    let staged = path.with_file_name(format!(".{KEY_FILE}.{}.tmp", Uuid::new_v4()));
    std::fs::write(&staged, json)
        .map_err(|error| StorageError::Vault(format!("{}: {error}", staged.display())))?;

    if let Err(error) = std::fs::rename(&staged, path) {
        let _ = std::fs::remove_file(&staged);
        return Err(StorageError::Vault(format!("{}: {error}", path.display())));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_created_vault_opens_again_with_the_same_passphrase() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();

        let created = create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();
        let sealed = created.seal("a note").unwrap();

        let reopened = unlock(&directory, "correct horse").unwrap();
        assert_eq!(reopened.open(&sealed).unwrap(), "a note");
    }

    /// Otherwise every later write would seal notes under a key nobody can reproduce.
    #[test]
    fn a_wrong_passphrase_is_refused_rather_than_accepted_quietly() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();

        let error = unlock(&directory, "battery staple").unwrap_err();

        assert!(matches!(error, StorageError::WrongPassphrase));
    }

    /// The key it carries is sealed; the phrase that opens it is nowhere.
    #[test]
    fn the_key_file_holds_no_passphrase_and_no_key_in_the_clear() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let vault = create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();

        let written = std::fs::read_to_string(path_in(&directory)).unwrap();
        let sealed = vault.seal("a note").unwrap();

        assert!(!written.contains("correct horse"));
        assert!(written.contains("argon2id"));
        // Nothing in the file opens what the library sealed — only the phrase does.
        let file: serde_json::Value = serde_json::from_str(&written).unwrap();
        let wrapped = file["key"].as_str().unwrap().to_string();
        assert!(unlock(&directory, &wrapped).is_err());
        assert_eq!(
            unlock(&directory, "correct horse")
                .unwrap()
                .open(&sealed)
                .unwrap(),
            "a note"
        );
    }

    /// The notes stay sealed exactly as they were.
    #[test]
    fn a_changed_passphrase_opens_the_notes_the_old_one_sealed() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let sealed = create(&directory, "correct horse", Cost::FOR_TESTS)
            .unwrap()
            .seal("a note")
            .unwrap();

        change_passphrase(
            &directory,
            "correct horse",
            "battery staple",
            Cost::FOR_TESTS,
        )
        .unwrap();

        let reopened = unlock(&directory, "battery staple").unwrap();
        assert_eq!(reopened.open(&sealed).unwrap(), "a note");
    }

    #[test]
    fn the_old_passphrase_stops_opening_the_library() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();

        change_passphrase(
            &directory,
            "correct horse",
            "battery staple",
            Cost::FOR_TESTS,
        )
        .unwrap();

        assert!(matches!(
            unlock(&directory, "correct horse").unwrap_err(),
            StorageError::WrongPassphrase
        ));
    }

    /// Refused before anything is written, or the library would end up behind a phrase nobody chose.
    #[test]
    fn a_change_that_cannot_name_the_current_passphrase_writes_nothing() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();
        let before = std::fs::read_to_string(path_in(&directory)).unwrap();

        let error =
            change_passphrase(&directory, "not it", "battery staple", Cost::FOR_TESTS).unwrap_err();

        assert!(matches!(error, StorageError::WrongPassphrase));
        assert_eq!(
            std::fs::read_to_string(path_in(&directory)).unwrap(),
            before
        );
        assert!(unlock(&directory, "correct horse").is_ok());
    }

    /// Two phrases over one library must not share a derivation.
    #[test]
    fn a_change_draws_a_fresh_salt() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();
        let before: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path_in(&directory)).unwrap()).unwrap();

        change_passphrase(
            &directory,
            "correct horse",
            "battery staple",
            Cost::FOR_TESTS,
        )
        .unwrap();

        let after: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path_in(&directory)).unwrap()).unwrap();
        assert_ne!(before["kdf"]["salt"], after["kdf"]["salt"]);
        assert_ne!(before["key"], after["key"]);
    }

    /// Raising the default cost later must not lock an existing library out.
    #[test]
    fn a_library_reopens_at_the_cost_it_was_written_with() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let odd = Cost {
            memory_kib: 96,
            passes: 3,
            lanes: 1,
        };
        create(&directory, "correct horse", odd).unwrap();

        assert!(unlock(&directory, "correct horse").is_ok());
    }

    #[test]
    fn creating_over_an_existing_key_file_is_refused() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        create(&directory, "first", Cost::FOR_TESTS).unwrap();

        assert!(create(&directory, "second", Cost::FOR_TESTS).is_err());
        // And the first passphrase still works.
        assert!(unlock(&directory, "first").is_ok());
    }

    #[test]
    fn a_key_file_from_a_newer_version_says_so_rather_than_failing_on_serde() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();

        let path = path_in(&directory);
        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json["version"] = serde_json::json!(FORMAT_VERSION + 1);
        std::fs::write(&path, json.to_string()).unwrap();

        let error = unlock(&directory, "correct horse").unwrap_err();
        assert!(format!("{error}").contains("reads up to"));
    }

    /// A key file asking for four gigabytes is refused before anything is allocated.
    #[test]
    fn a_cost_out_of_range_is_refused_before_deriving() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();

        for (field, value) in [("memoryKib", 4 * 1024 * 1024), ("passes", 0), ("lanes", 64)] {
            let path = path_in(&directory);
            let mut json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let original = json["kdf"][field].clone();
            json["kdf"][field] = serde_json::json!(value);
            std::fs::write(&path, json.to_string()).unwrap();

            let error = unlock(&directory, "correct horse").unwrap_err();
            assert!(
                format!("{error}").contains("out of range"),
                "{field}: {error}"
            );

            json["kdf"][field] = original;
            std::fs::write(&path, json.to_string()).unwrap();
        }

        assert!(unlock(&directory, "correct horse").is_ok());
    }

    #[test]
    fn an_unknown_derivation_is_named_rather_than_guessed_at() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();

        let path = path_in(&directory);
        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json["kdf"]["algorithm"] = serde_json::json!("scrypt");
        std::fs::write(&path, json.to_string()).unwrap();

        let error = unlock(&directory, "correct horse").unwrap_err();
        assert!(format!("{error}").contains("scrypt"));
    }

    #[test]
    fn a_library_with_no_key_file_says_it_has_none() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();

        assert!(!exists(&directory));
        create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();
        assert!(exists(&directory));
    }

    #[test]
    fn creating_leaves_no_staging_file_behind() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        create(&directory, "correct horse", Cost::FOR_TESTS).unwrap();

        let left: Vec<_> = std::fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();

        assert_eq!(left, [KEY_FILE]);
    }
}
