//! The one way in: a library opened under its key and handed to the connection `Mutex`.

use std::path::Path;

use crate::count::saturating_u32;
use crate::db::{self, Db};
use crate::error::{FileContext, StorageError};

use super::key::{Cost, Vault};
use super::{PassphraseChange, VaultState, file};

pub(super) fn state_of(directory: &Path) -> VaultState {
    if file::exists(directory) {
        VaultState::Locked
    } else if directory.join(crate::layout::DATABASE).exists() {
        VaultState::KeyMissing
    } else {
        VaultState::Absent
    }
}

pub(super) fn create(
    passphrase: &str,
    directory: &Path,
    db: &Db,
    cost: Cost,
) -> Result<(), StorageError> {
    // The directory in the error: a full disk or a permission reaches this first, in a folder
    // the user has never opened.
    std::fs::create_dir_all(directory).context(directory.display())?;
    refuse_a_database_without_its_key(directory)?;

    let vault = file::create(directory, passphrase, cost)?;

    install(directory, db, vault)
}

/// A database here without a key file lost it, or predates encryption (before 0.2.0). A new
/// key over it would open nothing it holds, so it is answered as damaged, which the gate
/// offers to set aside.
fn refuse_a_database_without_its_key(directory: &Path) -> Result<(), StorageError> {
    if directory.join(crate::layout::DATABASE).exists() {
        return Err(StorageError::Damaged(format!(
            "{}: a library is here without its key file",
            directory.display()
        )));
    }

    Ok(())
}

pub(super) fn unlock(passphrase: &str, directory: &Path, db: &Db) -> Result<(), StorageError> {
    let vault = file::unlock(directory, passphrase)?;

    install(directory, db, vault)
}

pub(super) fn change(
    current: &str,
    next: &str,
    db: &Db,
    cost: Cost,
) -> Result<PassphraseChange, StorageError> {
    // Asked and released: the derivations below cost tens of milliseconds each.
    let directory = db
        .lock()
        .map_err(|_| StorageError::Unavailable)?
        .as_ref()
        .ok_or(StorageError::Locked)?
        .directory()
        .to_path_buf();
    // The live file first, and the change fails here if it cannot be written: it is the only
    // one whose loss is fatal. A copy that resists is counted instead (`backup::copies::rewrap`).
    let vault = file::change_passphrase(&directory, current, next, cost)?;
    let copies = crate::backup::copies::rewrap(&directory, &vault, next, cost);

    Ok(PassphraseChange {
        backups_rewrapped: saturating_u32(copies.done),
        backups_left: saturating_u32(copies.left),
    })
}

/// Opens the library under the key and hands it over; the caller runs the sweeps. Creates
/// `attachments/`, which every writer and the startup sweep then assume.
fn install(directory: &Path, db: &Db, vault: Vault) -> Result<(), StorageError> {
    let library = db::open(&directory.join(crate::layout::DATABASE), vault)?;
    let attachments = crate::attachments::files::directory(&library);
    std::fs::create_dir_all(&attachments).context(attachments.display())?;

    {
        let mut held = db.lock().map_err(|_| StorageError::Unavailable)?;
        // The front gates on `vault_state`, so a second unlock is a race: keep the first.
        if held.is_none() {
            *held = Some(library);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(db: &Db) {
        *db.lock().unwrap() = None;
    }

    #[test]
    fn a_library_created_is_open_and_unlocks_again_with_its_phrase_alone() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let db: Db = std::sync::Mutex::new(None);

        create("a passphrase", &directory, &db, Cost::FOR_TESTS).unwrap();
        assert!(db.lock().unwrap().is_some());
        close(&db);

        assert!(unlock("not the phrase", &directory, &db).is_err());
        unlock("a passphrase", &directory, &db).unwrap();
        assert!(db.lock().unwrap().is_some());

        close(&db);
    }

    #[test]
    fn a_changed_phrase_opens_the_library_and_the_old_one_no_longer_does() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let db: Db = std::sync::Mutex::new(None);
        create("a passphrase", &directory, &db, Cost::FOR_TESTS).unwrap();

        let changed = change("a passphrase", "another phrase", &db, Cost::FOR_TESTS).unwrap();
        close(&db);

        assert_eq!(changed.backups_left, 0);
        assert!(unlock("a passphrase", &directory, &db).is_err());
        unlock("another phrase", &directory, &db).unwrap();

        close(&db);
    }

    #[test]
    fn a_phrase_is_not_changed_on_a_library_nobody_opened() {
        let db: Db = std::sync::Mutex::new(None);

        let refused = change("a passphrase", "another phrase", &db, Cost::FOR_TESTS);

        assert!(matches!(refused, Err(StorageError::Locked)));
    }

    /// A key written over a database it did not seal would open nothing that database holds.
    #[test]
    fn a_database_left_without_its_key_is_answered_as_damaged() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        assert!(refuse_a_database_without_its_key(&directory).is_ok());

        std::fs::write(directory.join(crate::layout::DATABASE), b"a sealed library").unwrap();

        assert!(matches!(
            refuse_a_database_without_its_key(&directory),
            Err(StorageError::Damaged(_))
        ));
    }

    /// A database left without its key file is neither new nor locked.
    #[test]
    fn a_database_without_its_key_file_is_named_as_such() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        assert_eq!(state_of(&directory), VaultState::Absent);

        std::fs::write(directory.join(crate::layout::DATABASE), b"a sealed library").unwrap();
        assert_eq!(state_of(&directory), VaultState::KeyMissing);

        file::create(&directory, "a passphrase", Cost::FOR_TESTS).unwrap();
        assert_eq!(state_of(&directory), VaultState::Locked);
    }
}
