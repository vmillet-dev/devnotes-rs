//! Encryption at rest: the key, what it seals, and the one gate that opens the library.
//!
//! The passphrase is never stored, anywhere: losing it loses the library, and an export is
//! the only copy that does not depend on it.

#![allow(clippy::needless_pass_by_value)]

pub mod file;
pub(crate) mod gate;
pub mod key;

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, State};

use zeroize::{Zeroize, Zeroizing};

use crate::db::Db;
use crate::error::{AppError, StorageError, ValidationError};
use key::Cost;

/// In characters. A human-chosen eight carries 25–30 bits, which Argon2id at 64 MiB stretches
/// to days on one GPU, not years. `u32` because it crosses as `MINIMUM_PASSPHRASE_LENGTH`.
pub(crate) const MINIMUM_LENGTH: u32 = 12;

/// Wipes a passphrase when the command returns, a panic included. It arrives owned from the
/// IPC payload, so the command holds the last copy, and freed memory ends up in crash dumps.
pub(crate) fn secret<S: Zeroize>(value: S) -> Zeroizing<S> {
    Zeroizing::new(value)
}

/// What the front end renders before it renders anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum VaultState {
    /// A library that has never been encrypted: the first launch asks for a passphrase
    /// twice and creates one.
    Absent,
    /// A key file is there and the passphrase has not been given yet.
    Locked,
    /// Held in Rust, not in the front end: a page reload must not ask again for a library
    /// this process has open.
    Unlocked,
    /// ⚠️ A database is here and its key file is not. Not `Absent`: a new key over it would
    /// open nothing it holds, and asking for a new phrase would be the first step towards that.
    KeyMissing,
}

#[tauri::command(async)]
#[specta::specta]
pub fn vault_state(app: AppHandle, db: State<'_, Db>) -> Result<VaultState, AppError> {
    if db.lock().map_err(|_| StorageError::Unavailable)?.is_some() {
        return Ok(VaultState::Unlocked);
    }

    let directory = crate::libraries::open_directory(&app)?;

    Ok(gate::state_of(&directory))
}

/// The first launch. Refuses a library that already has a key file: that file is the only
/// way into the notes beside it.
#[tauri::command(async)]
#[specta::specta]
pub fn create_vault(passphrase: String, app: AppHandle, db: State<'_, Db>) -> Result<(), AppError> {
    let passphrase = secret(passphrase);
    validate(&passphrase)?;

    let directory = crate::libraries::open_directory(&app)?;
    gate::create(&passphrase, &directory, &db, Cost::default())?;
    crate::sweep(&app);

    Ok(())
}

/// `(async)` because deriving the key is slow on purpose, and would freeze the window over
/// every attempt on the main thread.
#[tauri::command(async)]
#[specta::specta]
pub fn unlock_vault(passphrase: String, app: AppHandle, db: State<'_, Db>) -> Result<(), AppError> {
    let directory = crate::libraries::open_directory(&app)?;
    gate::unlock(&secret(passphrase), &directory, &db)?;
    crate::sweep(&app);

    Ok(())
}

/// What a change reached. `backupsLeft` is the honest half: a key file copied somewhere else
/// still opens with the retired phrase.
#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PassphraseChange {
    pub backups_rewrapped: u32,
    pub backups_left: u32,
}

/// A new phrase over the same library, from the preferences panel.
///
/// Not a re-encryption: the key the notes are sealed with is rewrapped, and the library stays
/// open on it.
#[tauri::command(async)]
#[specta::specta]
pub fn change_passphrase(
    current: String,
    next: String,
    db: State<'_, Db>,
) -> Result<PassphraseChange, AppError> {
    let (current, next) = (secret(current), secret(next));
    validate(&next)?;

    Ok(gate::change(&current, &next, &db, Cost::default())?)
}

/// An export written in the clear has no phrase to hold to anything.
pub(crate) fn validate_protection(passphrase: Option<&str>) -> Result<(), ValidationError> {
    passphrase.map_or(Ok(()), validate)
}

/// A length and nothing else: rules about digits and symbols push people towards one
/// memorable pattern.
fn validate(passphrase: &str) -> Result<(), ValidationError> {
    if passphrase.chars().count() < MINIMUM_LENGTH as usize {
        return Err(ValidationError::new(
            "passphrase",
            format!("a passphrase of at least {MINIMUM_LENGTH} characters"),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    /// Records being wiped, which a `String` in freed memory cannot be asked about.
    struct Witness(Rc<Cell<bool>>);

    impl Zeroize for Witness {
        fn zeroize(&mut self) {
            self.0.set(true);
        }
    }

    #[test]
    fn a_secret_is_wiped_when_the_command_returns_and_not_before() {
        let wiped = Rc::new(Cell::new(false));

        {
            let held = secret(Witness(Rc::clone(&wiped)));
            assert!(!held.0.get(), "wiped before it was used");
        }

        assert!(wiped.get());
    }

    /// A command that panics halfway must not leave the phrase where it was.
    #[test]
    fn a_secret_is_wiped_when_the_command_panics_too() {
        let wiped = Rc::new(Cell::new(false));
        let witness = Witness(Rc::clone(&wiped));

        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _held = secret(witness);
            panic!("a command that failed halfway");
        }));
        std::panic::set_hook(hook);

        assert!(unwound.is_err());
        assert!(wiped.get());
    }

    /// An export is the one file meant to travel, so its phrase meets the same floor.
    #[test]
    fn an_export_phrase_meets_the_floor_and_no_phrase_is_no_protection() {
        assert!(validate_protection(Some("eleven char")).is_err());
        assert!(validate_protection(Some("twelve chars")).is_ok());
        assert!(validate_protection(None).is_ok());
    }

    #[test]
    fn a_passphrase_too_short_to_be_worth_deriving_is_refused() {
        assert!(validate("short").is_err());
        assert!(validate("1234567").is_err());
    }

    #[test]
    fn a_passphrase_of_the_minimum_length_is_accepted() {
        assert!(validate("123456789012").is_ok());
        assert!(validate("12345678901").is_err());
    }

    /// Six accented letters are twelve bytes: a byte count would let them pass.
    #[test]
    fn the_length_is_counted_in_characters() {
        assert!(validate("éàèùçâêîôûëï").is_ok());
        assert!(validate("éàèùçâ").is_err());
    }
}
