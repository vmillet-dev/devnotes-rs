//! What to do with a library that will not open.
//!
//! ⚠️ Detection without a way out is a loop: an application that refuses to start and
//! tells you why, every time, leaves deleting a file by hand as the only move. So this
//! is the other half of the check — set the damaged library aside, rescue what SQLite
//! will still hand over, and let the next launch start clean.

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::Text;
use tauri::{AppHandle, State};

use crate::db::Db;
use crate::error::{AppError, FileContext, StorageError};
use crate::layout::{self, ARCHIVED, ATTACHMENTS, DAMAGED, DATABASE, DATABASE_SIDECARS, KEY_FILE};

/// Why a library is being set aside, which is what decides what travels with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reason {
    /// SQLite says the file is corrupt. The phrase still works, so `vault.json` **stays**
    /// and the rescued copy opens under it — asking someone to choose a new passphrase in
    /// the middle of losing their library would be its own small cruelty.
    Damaged,
    /// The phrase was forgotten. ⚠️ `vault.json` **travels**: it is the only thing that
    /// phrase would ever open again, and leaving it behind would turn a library that is
    /// merely locked into one that is gone. Nothing is rescued either — the file is
    /// sealed, and SQLite has nothing to hand over without the key.
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

/// ⚠️ Best effort and deliberately so: `VACUUM INTO` on a partly readable database often
/// rescues most of it, and when it cannot, the damaged original is still set aside. A
/// failure here must not stop the user getting a working application back.
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
/// ⚠️ The attachments go with it, whatever the reason. They are files the database points
/// at, and a fresh library would call every one of them an orphan — the startup sweep
/// would then delete the pictures belonging to the notes just set aside.
///
/// ⚠️ What happens to `vault.json` is [`Reason`]'s to decide, and the two answers are
/// opposite. Getting it wrong either way loses the library for good.
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
        // Before the move, while the file is still where SQLite expects its sidecars.
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

/// ⚠️ Refused on an open library, for both commands below: they only answer the case
/// where opening failed, and moving a database under a live connection is how a library
/// that was merely shut becomes a lost one.
fn set_aside_closed(directory: &Path, db: &Db, reason: Reason) -> Result<String, StorageError> {
    if db.lock().map_err(|_| StorageError::Unavailable)?.is_some() {
        return Err(StorageError::LibraryOpen);
    }

    let target = set_aside(directory, reason, Utc::now())?;

    Ok(target.to_string_lossy().to_string())
}

/// Sets the damaged library aside so the next unlock starts on a fresh one.
///
/// ⚠️ Answers the folder it moved everything into, and the interface says it out loud:
/// "set aside" is only true if the user can be told where.
#[tauri::command(async)]
#[specta::specta]
pub fn set_aside_damaged_library(app: AppHandle, db: State<'_, Db>) -> Result<String, AppError> {
    let directory = crate::libraries::open_directory(&app)?;

    Ok(set_aside_closed(&directory, &db, Reason::Damaged)?)
}

/// Archives a library whose passphrase was forgotten, so a fresh one can be started.
///
/// ⚠️ Nothing is recovered and nothing is meant to be: the notes leave **sealed**, under
/// the phrase nobody remembers. What this buys is a way out of the gate that does not
/// require knowing where `%APPDATA%` is — and a copy still standing on the day the phrase
/// comes back, which is why `vault.json` goes with it.
#[tauri::command(async)]
#[specta::specta]
pub fn archive_locked_library(app: AppHandle, db: State<'_, Db>) -> Result<String, AppError> {
    let directory = crate::libraries::open_directory(&app)?;

    Ok(set_aside_closed(&directory, &db, Reason::Forgotten)?)
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

    /// ⚠️ Or the next launch's orphan sweep deletes the pictures of the notes just set
    /// aside — the one way this recovery could destroy what it was meant to save.
    #[test]
    fn the_attachments_go_with_the_database_they_belong_to() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        library(&directory);

        let target = set_aside(&directory, Reason::Damaged, at()).unwrap();

        assert!(!directory.join(ATTACHMENTS).exists());
        assert!(target.join(ATTACHMENTS).join("a-1.png").is_file());
    }

    /// ⚠️ The passphrase is unchanged, and the rescued copy needs that exact key.
    #[test]
    fn the_key_file_stays_where_it_was() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        library(&directory);

        set_aside(&directory, Reason::Damaged, at()).unwrap();

        assert!(directory.join(KEY_FILE).is_file());
    }

    /// What is rescued is a real library: the point of trying `VACUUM INTO` at all.
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

        /// ⚠️ The opposite of the damaged case, and the whole point: the phrase would
        /// only ever open this file again, so leaving it behind turns a library that is
        /// merely locked into one that is gone.
        #[test]
        fn the_key_file_goes_with_the_library_it_seals() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            library(&directory);

            let target = set_aside(&directory, Reason::Forgotten, at()).unwrap();

            assert!(target.join(KEY_FILE).is_file());
            assert!(!directory.join(KEY_FILE).exists());
        }

        /// ⚠️ With no key file left, `vault_state` answers `absent` and the gate asks for
        /// a new phrase rather than one nobody has.
        #[test]
        fn what_is_left_behind_is_a_directory_with_no_library_in_it() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            library(&directory);

            set_aside(&directory, Reason::Forgotten, at()).unwrap();

            assert!(!directory.join(DATABASE).exists());
            assert!(!directory.join(KEY_FILE).exists());
        }

        /// Sealed is the promise: the copy is still openable, by whoever remembers.
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

        /// ⚠️ Nothing to rescue: the file is sealed, and SQLite hands over nothing
        /// without the key. A `rescued.sqlite3` here would be an empty promise.
        #[test]
        fn nothing_is_rescued_beside_it() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            library(&directory);

            let target = set_aside(&directory, Reason::Forgotten, at()).unwrap();

            assert!(!target.join(RESCUED).exists());
        }

        /// The two reasons must not land in the same folder: one is recoverable, one is not.
        #[test]
        fn it_is_filed_apart_from_a_damaged_one() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            library(&directory);

            let target = set_aside(&directory, Reason::Forgotten, at()).unwrap();

            assert!(target.starts_with(directory.join(ARCHIVED)));
            assert!(!directory.join(DAMAGED).exists());
        }

        /// ⚠️ Or the next launch's orphan sweep deletes the pictures of notes that are
        /// only sealed, not gone.
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
