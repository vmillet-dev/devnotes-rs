//! A rolling copy of the library, taken at launch.
//!
//! The trash protects a note, not the file: this covers an emptied trash or a botched
//! update, not a dead disk. ⚠️ `VACUUM INTO` rather than a file copy: under WAL the database
//! file alone is not a consistent snapshot, and a copy can open short of what was written.

pub mod copies;

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Runtime, State};

use chrono::{DateTime, Utc};

use crate::Paths;
use crate::db::{Db, blocking, lock};
use crate::error::{AppError, StorageError};
use crate::layout;
use copies::{Backup, list, replace, rotate, wanted};

/// Where the library the last restore replaced went, kept for the page the restore reloads:
/// nothing in the page survives the reload on every platform.
#[derive(Default)]
pub struct Restored(Mutex<Option<String>>);

impl Restored {
    fn keep(&self, aside: String) {
        if let Ok(mut kept) = self.0.lock() {
            *kept = Some(aside);
        }
    }

    fn take(&self) -> Option<String> {
        self.0.lock().ok().and_then(|mut kept| kept.take())
    }
}

/// The key the front end writes this setting under, in the application's preferences.
pub const AUTOMATIC_BACKUPS_KEY: &str = "devnotes.automaticBackups";

/// Read from the application's `preferences.json` itself, as the front's store writes it: the copy
/// is taken at unlock, before the front end has booted far enough to say anything. A file
/// missing or unreadable leaves the default, which takes the copy.
fn wanted_by_preference(paths: &Paths) -> bool {
    let stored = std::fs::read_to_string(paths.profile.join(layout::PREFERENCES))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|preferences| {
            preferences
                .get(AUTOMATIC_BACKUPS_KEY)?
                .as_str()
                .map(str::to_owned)
        });

    wanted(stored.as_deref())
}

/// The launch copy. Never fatal: a library that cannot be copied still has to open.
pub(crate) fn take(paths: &Paths, db: &Db) {
    if wanted_by_preference(paths) {
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
#[tauri::command]
#[specta::specta]
pub async fn list_backups<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Backup>, AppError> {
    blocking(app, move |_, db| Ok(list(lock(db)?.directory()))).await
}

/// Puts a copy back, and answers where the library it replaced was moved to.
///
/// ⚠️ Closes the library first, under the lock every command takes: renaming a database out
/// from under a live connection loses it. Every command then answers `Locked`, which sends
/// the interface back to the gate to ask for the restored copy's phrase.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub async fn restore_backup<R: Runtime>(id: String, app: AppHandle<R>) -> Result<String, AppError> {
    blocking(app, move |engine, db| {
        let aside = restore(db, &id, Utc::now())?.to_string_lossy().to_string();
        engine.restored.keep(aside.clone());

        Ok(aside)
    })
    .await
}

/// Where the library the last restore replaced went, once: the page it reloaded says it.
#[tauri::command]
#[specta::specta]
#[allow(clippy::needless_pass_by_value)]
pub fn take_restored_aside(engine: State<'_, std::sync::Arc<crate::Engine>>) -> Option<String> {
    engine.restored.take()
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
    fn the_launch_copy_follows_the_application_preference() {
        let profile = tempfile::tempdir().unwrap();
        let paths = Paths {
            profile: profile.path().to_path_buf(),
            resources: profile.path().to_path_buf(),
        };
        assert!(wanted_by_preference(&paths), "no file: the copy is taken");

        let file = profile.path().join(layout::PREFERENCES);
        std::fs::write(&file, r#"{ "devnotes.automaticBackups": "false" }"#).unwrap();
        assert!(!wanted_by_preference(&paths));

        std::fs::write(&file, r#"{ "devnotes.automaticBackups": "true" }"#).unwrap();
        assert!(wanted_by_preference(&paths));

        std::fs::write(&file, "not json").unwrap();
        assert!(wanted_by_preference(&paths), "unreadable: the default");
    }

    #[test]
    fn where_a_restore_set_the_library_aside_is_told_once() {
        let restored = Restored::default();
        assert_eq!(restored.take(), None);

        restored.keep("replaced/2026-07-25_09-00-00".to_string());

        assert_eq!(
            restored.take().as_deref(),
            Some("replaced/2026-07-25_09-00-00")
        );
        assert_eq!(restored.take(), None);
    }

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
