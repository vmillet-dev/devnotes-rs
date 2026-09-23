//! The key file, beside the database.
//!
//! ⚠️ Outside the library on purpose: it carries what is needed to derive the key that
//! opens it, so it has to be readable before anything else can be. What it holds is a
//! salt, the cost, and the library's own key **sealed under the phrase** — never a key in
//! the clear, and no separate check value: opening the wrapped key is the check.
//!
//! ⚠️ Losing this file loses the library, exactly as losing the passphrase does. An export
//! is the only copy that does not depend on it.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::key::{Cost, SALT_BYTES, Vault, fresh_salt};
use crate::error::StorageError;
use crate::layout::KEY_FILE;

/// Bumped when a file written today would stop being readable. 2 wraps the library key
/// under the phrase where 1 derived the library key from it — which is what lets the
/// phrase change without touching a single note.
const FORMAT_VERSION: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyFile {
    version: u32,
    kdf: Kdf,
    /// ⚠️ The library's key, sealed under the one derived from the passphrase. A wrong
    /// phrase fails to open it, which is what stops an unlock from succeeding on a key
    /// nobody can reproduce and sealing real notes under it.
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

/// A library that has never been encrypted. ⚠️ Refuses to overwrite: the file that is
/// there is the only way into the notes beside it.
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

/// A new phrase over the same library. ⚠️ Nothing is re-encrypted: the key the notes are
/// sealed with does not change, only what wraps it — so this cannot half-succeed and
/// leave some notes unreadable, and it costs one derivation rather than a full rewrite.
///
/// ⚠️ Answers the library's key because the caller is not finished: every retained backup
/// holds a key file of its own, still wrapped under the phrase being retired, and a change
/// that leaves those revokes nothing (#157). `backup::rewrap` is the other half, and
/// `vault::change_with` is the one place that has both.
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

/// ⚠️ A fresh salt every time, change included: two phrases must not share a derivation,
/// or knowing one would say something about the other.
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

/// ⚠️ Answers [`StorageError::WrongPassphrase`] and nothing more detailed: which of the
/// two the caller got wrong is not something to help with.
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

    let cost = Cost {
        memory_kib: file.kdf.memory_kib,
        passes: file.kdf.passes,
        lanes: file.kdf.lanes,
    };
    let wrapping = Vault::derive(passphrase, &salt, cost)?;
    let wrapped = BASE64
        .decode(&file.key)
        .map_err(|_| StorageError::Vault("the wrapped key is not base64".to_string()))?;

    Vault::unwrapped_with(&wrapping, &wrapped).map_err(|_| StorageError::WrongPassphrase)
}

/// ⚠️ Staged then renamed. A key file half-written is a library nobody opens again, and
/// a plain write truncates before it fills.
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

    /// ⚠️ Cheap parameters: the real ones cost about a second a derivation, and these tests derive
    /// a dozen times. What they assert on is the file, not Argon2id's strength.
    fn cheap() -> Cost {
        Cost {
            memory_kib: 64,
            passes: 1,
            lanes: 1,
        }
    }

    fn scratch() -> PathBuf {
        let directory = std::env::temp_dir().join(format!("devnotes-vault-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    #[test]
    fn a_created_vault_opens_again_with_the_same_passphrase() {
        let directory = scratch();

        let created = create(&directory, "correct horse", cheap()).unwrap();
        let sealed = created.seal("a note").unwrap();

        let reopened = unlock(&directory, "correct horse").unwrap();
        assert_eq!(reopened.open(&sealed).unwrap(), "a note");

        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ Without the wrapped key to open, this would succeed and every later write
    /// would seal real notes under a key nobody can reproduce.
    #[test]
    fn a_wrong_passphrase_is_refused_rather_than_accepted_quietly() {
        let directory = scratch();
        create(&directory, "correct horse", cheap()).unwrap();

        let error = unlock(&directory, "battery staple").unwrap_err();

        assert!(matches!(error, StorageError::WrongPassphrase));
        std::fs::remove_dir_all(&directory).ok();
    }

    /// The key it carries is sealed; the phrase that opens it is nowhere.
    #[test]
    fn the_key_file_holds_no_passphrase_and_no_key_in_the_clear() {
        let directory = scratch();
        let vault = create(&directory, "correct horse", cheap()).unwrap();

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

        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ The point of wrapping a random key rather than deriving one: the notes stay
    /// sealed exactly as they were, and a change cannot half-rewrite a library.
    #[test]
    fn a_changed_passphrase_opens_the_notes_the_old_one_sealed() {
        let directory = scratch();
        let sealed = create(&directory, "correct horse", cheap())
            .unwrap()
            .seal("a note")
            .unwrap();

        change_passphrase(&directory, "correct horse", "battery staple", cheap()).unwrap();

        let reopened = unlock(&directory, "battery staple").unwrap();
        assert_eq!(reopened.open(&sealed).unwrap(), "a note");

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn the_old_passphrase_stops_opening_the_library() {
        let directory = scratch();
        create(&directory, "correct horse", cheap()).unwrap();

        change_passphrase(&directory, "correct horse", "battery staple", cheap()).unwrap();

        assert!(matches!(
            unlock(&directory, "correct horse").unwrap_err(),
            StorageError::WrongPassphrase
        ));
        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ Refused *before* anything is written: a change that took a wrong current phrase
    /// on trust would lock the library behind a phrase nobody chose.
    #[test]
    fn a_change_that_cannot_name_the_current_passphrase_writes_nothing() {
        let directory = scratch();
        create(&directory, "correct horse", cheap()).unwrap();
        let before = std::fs::read_to_string(path_in(&directory)).unwrap();

        let error = change_passphrase(&directory, "not it", "battery staple", cheap()).unwrap_err();

        assert!(matches!(error, StorageError::WrongPassphrase));
        assert_eq!(
            std::fs::read_to_string(path_in(&directory)).unwrap(),
            before
        );
        assert!(unlock(&directory, "correct horse").is_ok());

        std::fs::remove_dir_all(&directory).ok();
    }

    /// Two phrases over one library must not share a derivation.
    #[test]
    fn a_change_draws_a_fresh_salt() {
        let directory = scratch();
        create(&directory, "correct horse", cheap()).unwrap();
        let before: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path_in(&directory)).unwrap()).unwrap();

        change_passphrase(&directory, "correct horse", "battery staple", cheap()).unwrap();

        let after: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path_in(&directory)).unwrap()).unwrap();
        assert_ne!(before["kdf"]["salt"], after["kdf"]["salt"]);
        assert_ne!(before["key"], after["key"]);

        std::fs::remove_dir_all(&directory).ok();
    }

    /// The cost travels with the file, so raising the default later does not lock an
    /// existing library out.
    #[test]
    fn a_library_reopens_at_the_cost_it_was_written_with() {
        let directory = scratch();
        let odd = Cost {
            memory_kib: 96,
            passes: 3,
            lanes: 1,
        };
        create(&directory, "correct horse", odd).unwrap();

        // `unlock` reads the parameters rather than assuming today's defaults.
        assert!(unlock(&directory, "correct horse").is_ok());
        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ Overwriting would throw away the only way into the notes sitting beside it.
    #[test]
    fn creating_over_an_existing_key_file_is_refused() {
        let directory = scratch();
        create(&directory, "first", cheap()).unwrap();

        assert!(create(&directory, "second", cheap()).is_err());
        // And the first passphrase still works.
        assert!(unlock(&directory, "first").is_ok());

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_key_file_from_a_newer_version_says_so_rather_than_failing_on_serde() {
        let directory = scratch();
        create(&directory, "correct horse", cheap()).unwrap();

        let path = path_in(&directory);
        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json["version"] = serde_json::json!(FORMAT_VERSION + 1);
        std::fs::write(&path, json.to_string()).unwrap();

        let error = unlock(&directory, "correct horse").unwrap_err();
        assert!(format!("{error}").contains("reads up to"));

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn an_unknown_derivation_is_named_rather_than_guessed_at() {
        let directory = scratch();
        create(&directory, "correct horse", cheap()).unwrap();

        let path = path_in(&directory);
        let mut json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json["kdf"]["algorithm"] = serde_json::json!("scrypt");
        std::fs::write(&path, json.to_string()).unwrap();

        let error = unlock(&directory, "correct horse").unwrap_err();
        assert!(format!("{error}").contains("scrypt"));

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_library_with_no_key_file_says_it_has_none() {
        let directory = scratch();

        assert!(!exists(&directory));
        create(&directory, "correct horse", cheap()).unwrap();
        assert!(exists(&directory));

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn creating_leaves_no_staging_file_behind() {
        let directory = scratch();
        create(&directory, "correct horse", cheap()).unwrap();

        let left: Vec<_> = std::fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();

        assert_eq!(left, [KEY_FILE]);
        std::fs::remove_dir_all(&directory).ok();
    }
}
