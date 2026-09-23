//! Encryption at rest: the key, what it seals, and the one gate that opens the library.
//!
//! ⚠️ The passphrase is never stored, anywhere, deliberately: this is the bargain a
//! password manager makes, not a keychain's. Losing it loses the library, and an export
//! is the only copy that does not depend on it.

#![allow(clippy::needless_pass_by_value)]

pub mod file;
pub mod key;

use std::path::Path;

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, State};

use zeroize::{Zeroize, Zeroizing};

use crate::count::saturating_u32;
use crate::db::{self, Db};
use crate::error::{AppError, FileContext, StorageError, ValidationError};
use key::Cost;

/// Short enough to be typed at every launch, long enough to be worth deriving from.
const MINIMUM_LENGTH: usize = 8;

/// Holds a passphrase for the rest of a command, and wipes it when the command returns —
/// whatever it returns, a panic included.
///
/// ⚠️ A passphrase arrives owned from the IPC payload, so the command holds the last copy —
/// and one left in freed memory is one in a crash dump.
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
pub fn create_vault(passphrase: String, app: AppHandle, db: State<'_, Db>) -> Result<(), AppError> {
    let passphrase = secret(passphrase);
    validate(&passphrase)?;

    let directory = crate::libraries::open_directory(&app)?;
    create(&passphrase, &directory, &db, Cost::default())?;
    crate::sweep(&app);

    Ok(())
}

fn create(passphrase: &str, directory: &Path, db: &Db, cost: Cost) -> Result<(), StorageError> {
    // ⚠️ The directory, not just the cause: this is the first thing a full disk or a
    // permissions problem reaches, and the user has never opened that folder.
    std::fs::create_dir_all(directory).context(directory.display())?;
    refuse_a_database_without_its_key(directory)?;

    let vault = file::create(directory, passphrase, cost)?;

    open_library(directory, db, vault)
}

/// ⚠️ A database already here without a key file is a library that lost its key — or one
/// written in the clear before 0.2.0, which this version no longer seals in place. A new key
/// over it would open nothing it holds, so it is answered as damaged, which the gate offers
/// to set aside.
fn refuse_a_database_without_its_key(directory: &Path) -> Result<(), StorageError> {
    if directory.join(crate::layout::DATABASE).exists() {
        return Err(StorageError::Damaged(format!(
            "{}: a library is here without its key file",
            directory.display()
        )));
    }

    Ok(())
}

/// ⚠️ Deliberately slow: deriving the key is the whole defence against someone trying
/// passphrases against a copied file. It is `(async)` for the same reason — a second on the
/// main thread would freeze the window over every attempt.
#[tauri::command(async)]
#[specta::specta]
pub fn unlock_vault(passphrase: String, app: AppHandle, db: State<'_, Db>) -> Result<(), AppError> {
    let directory = crate::libraries::open_directory(&app)?;
    unlock(&secret(passphrase), &directory, &db)?;
    crate::sweep(&app);

    Ok(())
}

fn unlock(passphrase: &str, directory: &Path, db: &Db) -> Result<(), StorageError> {
    let vault = file::unlock(directory, passphrase)?;

    open_library(directory, db, vault)
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
    current: String,
    next: String,
    db: State<'_, Db>,
) -> Result<PassphraseChange, AppError> {
    let (current, next) = (secret(current), secret(next));
    validate(&next)?;

    Ok(change(&current, &next, &db, Cost::default())?)
}

fn change(
    current: &str,
    next: &str,
    db: &Db,
    cost: Cost,
) -> Result<PassphraseChange, StorageError> {
    // ⚠️ Asked and released rather than held: the derivations below cost tens of
    // milliseconds each, and keeping the connection for them would freeze every other
    // command.
    let directory = db
        .lock()
        .map_err(|_| StorageError::Unavailable)?
        .as_ref()
        .ok_or(StorageError::Locked)?
        .directory()
        .to_path_buf();
    // ⚠️ The live file first, and the whole change fails here if it cannot be written: it
    // is the only one whose loss is fatal. The copies follow, and a copy that resists is
    // counted rather than fatal — see `backup::rewrap`.
    let vault = file::change_passphrase(&directory, current, next, cost)?;
    let copies = crate::backup::rewrap(&directory, &vault, next, cost);

    Ok(PassphraseChange {
        backups_rewrapped: saturating_u32(copies.done),
        backups_left: saturating_u32(copies.left),
    })
}

/// Opens the library under the key and hands it to the rest of the application. The
/// sweeps are the caller's to run, because a first launch has to seal what is there first.
///
/// ⚠️ `attachments/` is created here, once, and nowhere else: every writer and the
/// startup sweep assume it is there.
fn open_library(directory: &Path, db: &Db, vault: key::Vault) -> Result<(), StorageError> {
    let library = db::open(&directory.join(crate::layout::DATABASE), vault)?;
    let attachments = crate::attachments::directory(&library);
    std::fs::create_dir_all(&attachments).context(attachments.display())?;

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

    /// The hand-written `zeroize()` this replaced ran on the way out of a normal return
    /// only; a command that panicked halfway left the phrase where it was.
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

    fn cheap() -> Cost {
        Cost {
            memory_kib: 64,
            passes: 1,
            lanes: 1,
        }
    }

    fn scratch() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("devnotes-vault-{}", uuid::Uuid::new_v4()))
    }

    fn close(db: &Db) {
        *db.lock().unwrap() = None;
    }

    #[test]
    fn a_library_created_is_open_and_unlocks_again_with_its_phrase_alone() {
        let directory = scratch();
        let db: Db = std::sync::Mutex::new(None);

        create("a passphrase", &directory, &db, cheap()).unwrap();
        assert!(db.lock().unwrap().is_some());
        close(&db);

        assert!(unlock("not the phrase", &directory, &db).is_err());
        unlock("a passphrase", &directory, &db).unwrap();
        assert!(db.lock().unwrap().is_some());

        close(&db);
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_changed_phrase_opens_the_library_and_the_old_one_no_longer_does() {
        let directory = scratch();
        let db: Db = std::sync::Mutex::new(None);
        create("a passphrase", &directory, &db, cheap()).unwrap();

        let changed = change("a passphrase", "another phrase", &db, cheap()).unwrap();
        close(&db);

        assert_eq!(changed.backups_left, 0);
        assert!(unlock("a passphrase", &directory, &db).is_err());
        unlock("another phrase", &directory, &db).unwrap();

        close(&db);
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_phrase_is_not_changed_on_a_library_nobody_opened() {
        let db: Db = std::sync::Mutex::new(None);

        let refused = change("a passphrase", "another phrase", &db, cheap());

        assert!(matches!(refused, Err(StorageError::Locked)));
    }

    /// A key written over a database it did not seal would open nothing that database holds.
    #[test]
    fn a_database_left_without_its_key_is_answered_as_damaged() {
        let directory =
            std::env::temp_dir().join(format!("devnotes-keyless-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        assert!(refuse_a_database_without_its_key(&directory).is_ok());

        std::fs::write(directory.join(crate::layout::DATABASE), b"a sealed library").unwrap();

        assert!(matches!(
            refuse_a_database_without_its_key(&directory),
            Err(StorageError::Damaged(_))
        ));
        std::fs::remove_dir_all(&directory).ok();
    }

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
