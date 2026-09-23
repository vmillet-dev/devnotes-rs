//! Encryption at rest: the key, what it seals, and the one gate that opens the library.
//!
//! ⚠️ The passphrase is never stored, anywhere, deliberately: this is the bargain a
//! password manager makes, not a keychain's. Losing it loses the library, and an export
//! is the only copy that does not depend on it.

#![allow(clippy::needless_pass_by_value)]

pub mod file;
pub mod key;
pub mod migrate;

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, State};

use zeroize::Zeroize;

use crate::count::saturating_u32;
use crate::db::{self, Db};
use crate::error::{AppError, StorageError, ValidationError};
use key::Cost;

/// Short enough to be typed at every launch, long enough to be worth deriving from.
const MINIMUM_LENGTH: usize = 8;

/// What the front end renders before it renders anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum VaultState {
    /// A library that has never been encrypted: the first launch asks for a passphrase
    /// twice and creates one.
    Absent,
    /// A key file is there and the passphrase has not been given yet.
    Locked,
    /// ⚠️ Held in Rust, never in the front end: a page reload must not ask again for a
    /// library this process already has open — which is also what keeps `reopenSession`
    /// working in the end-to-end suite.
    Unlocked,
}

#[tauri::command(async)]
#[specta::specta]
pub fn vault_state(app: AppHandle, db: State<'_, Db>) -> Result<VaultState, AppError> {
    if db.lock().map_err(|_| StorageError::Unavailable)?.is_some() {
        return Ok(VaultState::Unlocked);
    }

    let directory = crate::libraries::open_directory(&app)?;

    Ok(if file::exists(&directory) {
        VaultState::Locked
    } else {
        VaultState::Absent
    })
}

/// The first launch. ⚠️ Refuses a library that already has a key file rather than
/// replacing it: that file is the only way into the notes beside it.
#[tauri::command(async)]
#[specta::specta]
pub fn create_vault(
    mut passphrase: String,
    app: AppHandle,
    db: State<'_, Db>,
) -> Result<(), AppError> {
    // ⚠️ Wiped before this returns, whatever it returns. The string arrives owned from the
    // IPC payload, so this is the last reference to it — and a passphrase left in freed
    // memory is a passphrase in a crash dump.
    let result = create_with(&passphrase, &app, &db);
    passphrase.zeroize();

    result
}

fn create_with(passphrase: &str, app: &AppHandle, db: &State<'_, Db>) -> Result<(), AppError> {
    validate(passphrase)?;

    let directory = crate::libraries::open_directory(app)?;
    // ⚠️ The directory, not just the cause: this is the first thing a full disk or a
    // permissions problem reaches, and the user has never opened that folder.
    std::fs::create_dir_all(&directory)
        .map_err(|error| storage_msg(&format!("{}: {error}", directory.display())))?;

    let vault = file::create(&directory, passphrase, Cost::default())?;

    open_library(app, db, vault)?;

    // ⚠️ Between opening and sweeping, never after. The orphan-file sweep reads attachment
    // records, and on a library that predates the passphrase those are still in the clear —
    // it would fail to open every one of them and log a warning for nothing.
    seal_what_was_there(app, db)?;

    crate::sweep(app);

    Ok(())
}

/// ⚠️ The rows first, in one transaction, and the files after it commits. A file write
/// does not roll back — a file left readable is recoverable, a row sealed twice is not.
fn seal_what_was_there(app: &AppHandle, db: &State<'_, Db>) -> Result<(), AppError> {
    let stored_names = {
        let mut connection = crate::db::lock(db)?;
        let done = {
            let (connection, vault) = connection.split();
            migrate::seal_existing(connection, vault)?
        };

        if done.is_empty() {
            return Ok(());
        }

        log::info!(
            "Sealed an existing library: {} note(s), {} space(s), {} item(s), {} value(s), {} attachment record(s)",
            done.notes,
            done.spaces,
            done.items,
            done.values,
            done.attachments
        );

        crate::attachments::store::all_stored_names(&mut connection)?
    };

    let directory = crate::attachments::directory(app)?;
    let connection = crate::db::lock(db)?;
    let vault = connection.vault();

    for name in stored_names {
        let path = directory.join(&name);
        // ⚠️ Best effort, one file at a time, and never fatal: a library whose notes are
        // sealed is worth keeping even if one screenshot resisted. The alternative is
        // refusing to start over a file nobody may ever open.
        if let Err(error) = crate::attachments::sealed::seal_in_place(vault, &path) {
            log::warn!("Attachment {name} left as it was: {error}");
        }
    }

    Ok(())
}

/// ⚠️ Deliberately slow: deriving the key is the whole defence against someone trying
/// passphrases against a copied file. It is `(async)` for the same reason — a second on the
/// main thread would freeze the window over every attempt.
#[tauri::command(async)]
#[specta::specta]
pub fn unlock_vault(
    mut passphrase: String,
    app: AppHandle,
    db: State<'_, Db>,
) -> Result<(), AppError> {
    // ⚠️ Wiped before this returns, whatever it returns — see `create_vault`.
    let result = unlock_with(&passphrase, &app, &db);
    passphrase.zeroize();

    result
}

fn unlock_with(passphrase: &str, app: &AppHandle, db: &State<'_, Db>) -> Result<(), AppError> {
    let directory = crate::libraries::open_directory(app)?;
    let vault = file::unlock(&directory, passphrase)?;

    open_library(app, db, vault)?;
    crate::sweep(app);

    Ok(())
}

/// What a change reached, so the interface can say it. ⚠️ `backupsLeft` is the honest half:
/// the application can only speak for the copies it knows about, and a key file the user
/// put somewhere else still opens with the retired phrase.
#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PassphraseChange {
    pub backups_rewrapped: u32,
    pub backups_left: u32,
}

/// A new phrase over the same library, from the preferences panel.
///
/// ⚠️ Not a re-encryption: the key the notes are sealed with is the one being rewrapped,
/// so nothing in the database moves and the library stays open on the key it already had.
/// The consequence is worth knowing — this answers a phrase somebody else learned, never
/// a key somebody else got hold of.
#[tauri::command(async)]
#[specta::specta]
pub fn change_passphrase(
    mut current: String,
    mut next: String,
    app: AppHandle,
    db: State<'_, Db>,
) -> Result<PassphraseChange, AppError> {
    // ⚠️ Wiped before this returns, whatever it returns — see `create_vault`.
    let result = change_with(&current, &next, &app, &db);
    current.zeroize();
    next.zeroize();

    result
}

fn change_with(
    current: &str,
    next: &str,
    app: &AppHandle,
    db: &State<'_, Db>,
) -> Result<PassphraseChange, AppError> {
    validate(next)?;

    // ⚠️ Asked and released rather than held: the derivations below cost tens of
    // milliseconds each, and keeping the connection for them would freeze every other
    // command.
    if db.lock().map_err(|_| StorageError::Unavailable)?.is_none() {
        return Err(StorageError::Locked.into());
    }

    let directory = crate::libraries::open_directory(app)?;
    // ⚠️ The live file first, and the whole change fails here if it cannot be written: it
    // is the only one whose loss is fatal. The copies follow, and a copy that resists is
    // counted rather than fatal — see `backup::rewrap`.
    let vault = file::change_passphrase(&directory, current, next, Cost::default())?;
    let copies = crate::backup::rewrap(&directory, &vault, next, Cost::default());

    Ok(PassphraseChange {
        backups_rewrapped: saturating_u32(copies.done),
        backups_left: saturating_u32(copies.left),
    })
}

/// Opens the library under the key and hands it to the rest of the application. The
/// sweeps are the caller's to run, because a first launch has to seal what is there first.
fn open_library(app: &AppHandle, db: &State<'_, Db>, vault: key::Vault) -> Result<(), AppError> {
    let directory = crate::libraries::open_directory(app)?;
    let library = db::open(&directory.join(crate::layout::DATABASE), vault)?;

    {
        let mut held = db.lock().map_err(|_| StorageError::Unavailable)?;
        // ⚠️ A second unlock would drop the library the first one opened, and with it any
        // connection state. The front gates on `vault_state`, so this only catches a race.
        if held.is_none() {
            *held = Some(library);
        }
    }

    Ok(())
}

/// ⚠️ A length, and nothing else. A rule about digits and symbols pushes people towards
/// one memorable pattern, and the cost of guessing is Argon2id's to carry.
fn validate(passphrase: &str) -> Result<(), ValidationError> {
    if passphrase.chars().count() < MINIMUM_LENGTH {
        return Err(ValidationError::new(
            "passphrase",
            "a passphrase of at least 8 characters",
        ));
    }

    Ok(())
}

fn storage_msg(detail: &str) -> StorageError {
    StorageError::Vault(detail.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_passphrase_too_short_to_be_worth_deriving_is_refused() {
        assert!(validate("short").is_err());
        assert!(validate("1234567").is_err());
    }

    #[test]
    fn a_passphrase_of_the_minimum_length_is_accepted() {
        assert!(validate("12345678").is_ok());
    }

    /// ⚠️ Counted in characters, not bytes: "clé-privée" is ten characters and twelve
    /// bytes, and a byte count would accept a shorter one through an accent.
    #[test]
    fn the_length_is_counted_in_characters() {
        assert!(validate("éàèùçâêîô").is_ok());
        assert!(validate("éàèùç").is_err());
    }
}
