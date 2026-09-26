//! A rolling copy of the library, taken at launch.
//!
//! The trash protects a note, not the file: this covers an emptied trash or a botched
//! update, not a dead disk. ⚠️ `VACUUM INTO` rather than a file copy: under WAL the database
//! file alone is not a consistent snapshot, and a copy can open short of what was written.

pub mod copies;

use std::path::PathBuf;

use tauri::{AppHandle, State};

use chrono::{DateTime, Utc};

use crate::db::{Db, lock};
use crate::error::{AppError, StorageError};
use crate::layout;
use copies::{Backup, list, replace, rotate, wanted};

/// The key the front end writes this setting under, in the application's preferences.
pub(crate) const AUTOMATIC_BACKUPS_KEY: &str = "devnotes.automaticBackups";

/// Read from the preferences file: the copy is taken at unlock, before the front end has
/// booted far enough to say anything.
fn wanted_by_preference(app: &AppHandle) -> bool {
    use tauri_plugin_store::StoreExt;

    let stored = app
        .store(layout::PREFERENCES)
        .ok()
        .and_then(|store| store.get(AUTOMATIC_BACKUPS_KEY))
        .and_then(|value| value.as_str().map(str::to_owned));

    wanted(stored.as_deref())
}

/// The launch copy. Never fatal: a library that cannot be copied still has to open.
pub(crate) fn take(app: &AppHandle, db: &Db) {
    if wanted_by_preference(app) {
        copy(db);
    }
}

fn copy(db: &Db) {
    let mut connection = match crate::db::lock(db) {
        Ok(connection) => connection,
        Err(error) => {
            log::warn!("No backup taken: {error}");
            return;
        }
    };
    let directory = connection.directory().to_path_buf();

    match rotate(&directory, &mut connection, Utc::now()) {
        Ok(Some(target)) => log::info!("Library copied to {}", target.display()),
        Ok(None) => {}
        Err(error) => log::warn!("No backup taken: {error}"),
    }
}

/// The copies that exist, newest first, for the panel that lists them.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn list_backups(db: State<'_, Db>) -> Result<Vec<Backup>, AppError> {
    Ok(list(lock(&db)?.directory()))
}

/// Puts a copy back, and answers where the library it replaced was moved to.
///
/// ⚠️ Closes the library first, under the lock every command takes: renaming a database out
/// from under a live connection loses it. Every command then answers `Locked`, which sends
/// the interface back to the gate to ask for the restored copy's phrase.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn restore_backup(id: String, db: State<'_, Db>) -> Result<String, AppError> {
    let aside = restore(&db, &id, Utc::now())?;

    Ok(aside.to_string_lossy().to_string())
}

fn restore(db: &Db, id: &str, now: DateTime<Utc>) -> Result<PathBuf, StorageError> {
    let mut open = db.lock().map_err(|_| StorageError::Unavailable)?;
    let directory = open
        .as_ref()
        .ok_or(StorageError::Locked)?
        .directory()
        .to_path_buf();
    // Dropped before a single file moves, and the lock held for the whole swap.
    *open = None;

    replace(&directory, id, now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use copies::tests::{at, library};

    #[test]
    fn the_launch_copy_lands_beside_the_open_library() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (connection, _) = library(&directory);
        let db: Db = std::sync::Mutex::new(Some(connection));

        copy(&db);

        assert_eq!(list(&directory).len(), 1);
        drop(db);
    }

    /// Closed before a single file moves: every command answers `Locked` afterwards.
    #[test]
    fn restoring_closes_the_library_and_says_where_the_replaced_one_went() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
        let id = copy.file_name().unwrap().to_string_lossy().to_string();
        let db: Db = std::sync::Mutex::new(Some(connection));

        let aside = restore(&db, &id, at(1)).unwrap();

        assert!(db.lock().unwrap().is_none());
        assert!(aside.join(layout::DATABASE).exists());
        assert!(matches!(
            restore(&db, &id, at(2)),
            Err(StorageError::Locked)
        ));
    }
}
