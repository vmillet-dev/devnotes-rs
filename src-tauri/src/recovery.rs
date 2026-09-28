//! What to do with a library that will not open.
//!
//! The other half of the integrity check: without a way out, a library that will not open
//! leaves deleting a file by hand as the only move.

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::Text;
use tauri::{AppHandle, Runtime};

use crate::db::{Db, blocking};
use crate::error::{AppError, FileContext, StorageError};
use crate::layout::{self, ARCHIVED, ATTACHMENTS, DAMAGED, DATABASE, DATABASE_SIDECARS, KEY_FILE};

/// Why a library is being set aside, which is what decides what travels with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reason {
    /// SQLite says the file is corrupt. The phrase still works, so `vault.json` stays and the
    /// rescued copy opens under it.
    Damaged,
    /// The phrase was forgotten. `vault.json` travels: it is the only thing that phrase would
    /// ever open again. Nothing is rescued, since the file is sealed.
    Forgotten,
}

impl Reason {
    fn directory(self) -> &'static str {
        match self {
            Self::Damaged => DAMAGED,
            Self::Forgotten => ARCHIVED,
        }
    }
}

/// The name the rescued copy takes, beside the file it was rescued from.
const RESCUED: &str = "rescued.sqlite3";

/// Best effort: `VACUUM INTO` often rescues most of a partly readable file, and the original
/// is set aside either way.
fn rescue(damaged: &Path, into: &Path) -> Option<PathBuf> {
    let mut connection = SqliteConnection::establish(&damaged.to_string_lossy()).ok()?;
    let target = into.join(RESCUED);

    diesel::sql_query("VACUUM INTO ?")
        .bind::<Text, _>(target.to_string_lossy().to_string())
        .execute(&mut connection)
        .ok()?;

    Some(target)
}

/// Moves a library aside and leaves the directory ready for a fresh one.
///
/// The attachments go too, whatever the reason: a fresh library would call them orphans and
/// the startup sweep would delete them. ⚠️ `vault.json` is [`Reason`]'s to decide, and the two
/// answers are opposite: getting it wrong either way loses the library for good.
pub(crate) fn set_aside(
    directory: &Path,
    reason: Reason,
    now: DateTime<Utc>,
) -> Result<PathBuf, StorageError> {
    let database = directory.join(DATABASE);
    if !database.exists() {
        return Err(StorageError::NothingToSetAside(
            database.display().to_string(),
        ));
    }

    let target = directory.join(reason.directory()).join(layout::stamp(now));
    std::fs::create_dir_all(&target).context(target.display())?;

    if reason == Reason::Damaged {
        // Before the move, while SQLite still finds its sidecars beside the file.
        rescue(&database, &target);
    }

    std::fs::rename(&database, target.join(DATABASE)).context(database.display())?;

    for sidecar in DATABASE_SIDECARS {
        // Absent is the ordinary case: a clean shutdown leaves neither.
        let _ = std::fs::rename(directory.join(sidecar), target.join(sidecar));
    }

    if reason == Reason::Forgotten {
        std::fs::rename(directory.join(KEY_FILE), target.join(KEY_FILE)).context(KEY_FILE)?;
    }

    let attachments = directory.join(ATTACHMENTS);
    if attachments.is_dir() {
        std::fs::rename(&attachments, target.join(ATTACHMENTS)).context(attachments.display())?;
    }

    Ok(target)
}

/// Refused on an open library: moving a database under a live connection loses it.
fn set_aside_closed(directory: &Path, db: &Db, reason: Reason) -> Result<String, StorageError> {
    if db.lock().map_err(|_| StorageError::Unavailable)?.is_some() {
        return Err(StorageError::LibraryOpen);
    }

    let target = set_aside(directory, reason, Utc::now())?;

    Ok(target.to_string_lossy().to_string())
}

/// Sets the damaged library aside so the next unlock starts on a fresh one.
///
/// Answers the folder it moved everything into, so the interface can say where.
#[tauri::command]
#[specta::specta]
pub async fn set_aside_damaged_library<R: Runtime>(app: AppHandle<R>) -> Result<String, AppError> {
    blocking(app, move |app, db| {
        let directory = crate::libraries::open_directory(app)?;

        Ok(set_aside_closed(&directory, db, Reason::Damaged)?)
    })
    .await
}

/// Archives a library whose passphrase was forgotten, so a fresh one can be started.
///
/// Nothing is recovered: the notes leave sealed, under the forgotten phrase, and `vault.json`
/// goes with them for the day the phrase comes back.
#[tauri::command]
#[specta::specta]
pub async fn archive_locked_library<R: Runtime>(app: AppHandle<R>) -> Result<String, AppError> {
    blocking(app, move |app, db| {
        let directory = crate::libraries::open_directory(app)?;

        Ok(set_aside_closed(&directory, db, Reason::Forgotten)?)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn nothing_is_set_aside_under_an_open_library() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        std::fs::write(directory.join(DATABASE), b"a library").unwrap();
        let db: Db = std::sync::Mutex::new(Some(db::open_in_memory().unwrap()));

        let refused = set_aside_closed(&directory, &db, Reason::Forgotten);

        assert!(refused.is_err());
        assert!(directory.join(DATABASE).exists());
    }

    #[test]
    fn a_closed_library_is_set_aside_and_the_answer_says_where() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        std::fs::write(directory.join(DATABASE), b"a sealed library").unwrap();
        std::fs::write(directory.join(KEY_FILE), b"its key").unwrap();
        let db: Db = std::sync::Mutex::new(None);

        let target = set_aside_closed(&directory, &db, Reason::Forgotten).unwrap();

        assert!(Path::new(&target).join(DATABASE).exists());
        assert!(!directory.join(DATABASE).exists());
    }

    fn at() -> DateTime<Utc> {
        db::iso8601::parse("2026-07-25T09:00:00.000Z").unwrap()
    }

    /// A library as one really sits on disk: a database, its key file, and a picture.
    fn library(directory: &Path) -> String {
        let vault = crate::vault::file::create(
            directory,
            "a passphrase",
            crate::vault::key::Cost::FOR_TESTS,
        )
        .unwrap();

        let mut connection = db::open(&directory.join(DATABASE), vault).unwrap();
        let space = crate::spaces::store::create(&mut connection, "Perso")
            .unwrap()
            .id;

        std::fs::create_dir_all(directory.join(ATTACHMENTS)).unwrap();
        std::fs::write(directory.join(ATTACHMENTS).join("a-1.png"), b"\x89PNG").unwrap();

        space
    }

    #[test]
    fn the_database_leaves_and_the_directory_is_ready_for_a_new_one() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        library(&directory);

        let target = set_aside(&directory, Reason::Damaged, at()).unwrap();

        assert!(!directory.join(DATABASE).exists());
        assert!(target.join(DATABASE).is_file());
    }

    /// Or the next launch's orphan sweep deletes the pictures of the notes just set aside.
    #[test]
    fn the_attachments_go_with_the_database_they_belong_to() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        library(&directory);

        let target = set_aside(&directory, Reason::Damaged, at()).unwrap();

        assert!(!directory.join(ATTACHMENTS).exists());
        assert!(target.join(ATTACHMENTS).join("a-1.png").is_file());
    }

    /// The rescued copy needs that exact key.
    #[test]
    fn the_key_file_stays_where_it_was() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        library(&directory);

        set_aside(&directory, Reason::Damaged, at()).unwrap();

        assert!(directory.join(KEY_FILE).is_file());
    }

    #[test]
    fn what_could_be_read_is_rescued_beside_it() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let space = library(&directory);

        let target = set_aside(&directory, Reason::Damaged, at()).unwrap();

        let vault = crate::vault::file::unlock(&directory, "a passphrase").unwrap();
        let mut rescued = db::open(&target.join(RESCUED), vault).unwrap();
        let spaces = crate::spaces::store::list(&mut rescued).unwrap();

        assert_eq!(spaces.len(), 1);
        assert_eq!(spaces[0].id, space);
    }

    #[test]
    fn a_fresh_library_opens_in_its_place() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        library(&directory);
        set_aside(&directory, Reason::Damaged, at()).unwrap();

        let vault = crate::vault::file::unlock(&directory, "a passphrase").unwrap();
        let mut fresh = db::open(&directory.join(DATABASE), vault).unwrap();

        assert!(crate::spaces::store::list(&mut fresh).unwrap().is_empty());
    }

    #[test]
    fn setting_aside_nothing_says_so_rather_than_pretending() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();

        assert!(set_aside(&directory, Reason::Damaged, at()).is_err());
    }
    mod a_forgotten_passphrase {
        use super::*;

        /// The phrase would only ever open this file again.
        #[test]
        fn the_key_file_goes_with_the_library_it_seals() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            library(&directory);

            let target = set_aside(&directory, Reason::Forgotten, at()).unwrap();

            assert!(target.join(KEY_FILE).is_file());
            assert!(!directory.join(KEY_FILE).exists());
        }

        /// With no key file left, the gate asks for a new phrase.
        #[test]
        fn what_is_left_behind_is_a_directory_with_no_library_in_it() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            library(&directory);

            set_aside(&directory, Reason::Forgotten, at()).unwrap();

            assert!(!directory.join(DATABASE).exists());
            assert!(!directory.join(KEY_FILE).exists());
        }

        #[test]
        fn the_archived_copy_still_opens_on_the_day_the_phrase_comes_back() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let space = library(&directory);

            let target = set_aside(&directory, Reason::Forgotten, at()).unwrap();

            let vault = crate::vault::file::unlock(&target, "a passphrase").unwrap();
            let mut archived = db::open(&target.join(DATABASE), vault).unwrap();
            let spaces = crate::spaces::store::list(&mut archived).unwrap();

            assert_eq!(spaces.len(), 1);
            assert_eq!(spaces[0].id, space);
        }

        /// Sealed: SQLite hands over nothing without the key.
        #[test]
        fn nothing_is_rescued_beside_it() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            library(&directory);

            let target = set_aside(&directory, Reason::Forgotten, at()).unwrap();

            assert!(!target.join(RESCUED).exists());
        }

        #[test]
        fn it_is_filed_apart_from_a_damaged_one() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            library(&directory);

            let target = set_aside(&directory, Reason::Forgotten, at()).unwrap();

            assert!(target.starts_with(directory.join(ARCHIVED)));
            assert!(!directory.join(DAMAGED).exists());
        }

        #[test]
        fn the_attachments_go_with_it_too() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            library(&directory);

            let target = set_aside(&directory, Reason::Forgotten, at()).unwrap();

            assert!(target.join(ATTACHMENTS).join("a-1.png").is_file());
        }
    }
}
