//! A rolling copy of the library, taken at launch.
//!
//! The trash protects a note, not the file: this covers an emptied trash or a botched
//! update, not a dead disk. ⚠️ `VACUUM INTO` rather than a file copy: under WAL the database
//! file alone is not a consistent snapshot, and a copy can open short of what was written.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};

use chrono::{DateTime, TimeDelta, Utc};
use diesel::prelude::*;
use diesel::sql_types::Text;

use crate::db::{Db, Library, lock};
use crate::error::{AppError, FileContext, StorageError};
use crate::layout::{self, ATTACHMENTS, BACKUPS, DATABASE_SIDECARS, KEY_FILE, REPLACED};
use crate::vault::key::{Cost, Vault};

/// The key the front end writes this setting under, in the application's preferences.
pub(crate) const AUTOMATIC_BACKUPS_KEY: &str = "devnotes.automaticBackups";

/// Whether a copy carries `attachments/`, read the same way: a copy of the corpus is a
/// multiple of it, and some would rather keep the notes alone.
pub(crate) const BACKUP_ATTACHMENTS_KEY: &str = "devnotes.backupAttachments";

/// Enough to reach past the launch that went wrong without becoming a second library.
pub(crate) const KEEP: usize = 3;

/// At most one a day: five launches in an hour would rotate every older copy out.
const MIN_AGE: TimeDelta = TimeDelta::hours(24);

fn taken_at(entry: &Path) -> Option<DateTime<Utc>> {
    layout::parse_stamp(entry.file_name()?.to_str()?)
}

/// The copies on disk, newest first.
fn existing(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut taken: Vec<(DateTime<Utc>, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| taken_at(&path).map(|at| (at, path)))
        .collect();

    taken.sort_by_key(|(at, _)| std::cmp::Reverse(*at));

    taken.into_iter().map(|(_, path)| path).collect()
}

/// Takes one if the newest is older than a day, then prunes to [`KEEP`].
///
/// ⚠️ The key file travels with the database: without `vault.json` the copy is a sealed file
/// nobody can ever open. The attachments are copied as they are, sealed under that same key.
pub(crate) fn rotate(
    library: &Path,
    connection: &mut Library,
    now: DateTime<Utc>,
    with_attachments: bool,
) -> Result<Option<PathBuf>, StorageError> {
    let directory = library.join(BACKUPS);
    let taken = existing(&directory);

    if let Some(newest) = taken.first()
        && let Some(at) = taken_at(newest)
        && now.signed_duration_since(at) < MIN_AGE
    {
        return Ok(None);
    }

    let target = directory.join(layout::stamp(now));
    std::fs::create_dir_all(&target).context(target.display())?;

    let copy = target.join(layout::DATABASE);
    diesel::sql_query("VACUUM INTO ?")
        .bind::<Text, _>(copy.to_string_lossy().to_string())
        .execute(connection.db())
        .map_err(|error| {
            // A half-written copy would be the newest, and hold the next rotation off.
            let _ = std::fs::remove_dir_all(&target);
            StorageError::File(format!("{}: {error}", copy.display()))
        })?;

    std::fs::copy(library.join(KEY_FILE), target.join(KEY_FILE)).map_err(|error| {
        let _ = std::fs::remove_dir_all(&target);
        StorageError::File(format!("{KEY_FILE}: {error}"))
    })?;

    // Created even when the library has none: its presence is what says the copy carries them.
    if with_attachments {
        copy_files(&library.join(ATTACHMENTS), &target.join(ATTACHMENTS)).map_err(|error| {
            let _ = std::fs::remove_dir_all(&target);
            StorageError::File(format!("{ATTACHMENTS}: {error}"))
        })?;
    }

    for old in existing(&directory).into_iter().skip(KEEP) {
        // Best effort: a copy that resists deletion is not worth failing a launch over.
        let _ = std::fs::remove_dir_all(old);
    }

    Ok(Some(target))
}

/// `attachments/` is flat: a stored name is one path component.
fn copy_files(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry?.path();
        if let Some(name) = path.file_name().filter(|_| path.is_file()) {
            std::fs::copy(&path, to.join(name))?;
        }
    }

    Ok(())
}

fn size_of_files(directory: &Path) -> u64 {
    std::fs::read_dir(directory).map_or(0, |entries| {
        entries
            .flatten()
            .filter_map(|entry| entry.metadata().ok())
            .filter(std::fs::Metadata::is_file)
            .map(|meta| meta.len())
            .sum()
    })
}

/// One copy, as the interface lists it. `bytes` is `f64` because specta refuses the integer
/// types JSON cannot carry exactly.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// The folder's name, which is its stamp, and what a restore asks for.
    pub id: String,
    pub taken_at: DateTime<Utc>,
    pub bytes: f64,
    /// Whether the key file travelled with it: without one, restoring loses the library.
    pub openable: bool,
    /// Without them, a restore leaves the live `attachments/` where it is.
    pub attachments: bool,
}

#[allow(clippy::cast_precision_loss)]
fn describe(copy: &Path) -> Option<Backup> {
    let at = taken_at(copy)?;
    let database = copy.join(layout::DATABASE);
    let bytes = std::fs::metadata(&database).map_or(0, |meta| meta.len())
        + size_of_files(&copy.join(ATTACHMENTS));

    Some(Backup {
        id: copy.file_name()?.to_str()?.to_string(),
        taken_at: at,
        bytes: bytes as f64,
        openable: database.is_file() && copy.join(KEY_FILE).is_file(),
        attachments: copy.join(ATTACHMENTS).is_dir(),
    })
}

/// The copies that exist, newest first.
pub(crate) fn list(library: &Path) -> Vec<Backup> {
    existing(&library.join(BACKUPS))
        .iter()
        .filter_map(|copy| describe(copy))
        .collect()
}

/// Puts a copy back in place of the live library, and answers where the live one went.
///
/// Moved aside into `replaced/`, never deleted, and `vault.json` with it: the copy brings its
/// own, and a database and a key from two wrappings open nothing. `attachments/` goes too when
/// the copy carries its own; otherwise it stays, or every restored record would name a file
/// that left.
pub(crate) fn replace(
    library: &Path,
    id: &str,
    now: DateTime<Utc>,
) -> Result<PathBuf, StorageError> {
    // ⚠️ Matched against the listing, never joined: the id comes from the front end, and
    // `../2026-01-01_00-00-00` joins to a path outside `backups/` that still parses as a stamp.
    let copy = existing(&library.join(BACKUPS))
        .into_iter()
        .find(|path| path.file_name().and_then(std::ffi::OsStr::to_str) == Some(id))
        .ok_or_else(|| StorageError::BackupNotFound(id.to_string()))?;

    if !copy.join(layout::DATABASE).is_file() || !copy.join(KEY_FILE).is_file() {
        return Err(StorageError::BackupUnopenable(id.to_string()));
    }

    let aside = library.join(REPLACED).join(layout::stamp(now));
    std::fs::create_dir_all(&aside).context(aside.display())?;

    let carries = copy.join(ATTACHMENTS).is_dir();
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for name in [layout::DATABASE, KEY_FILE] {
        let from = library.join(name);
        let to = aside.join(name);
        if let Err(error) = std::fs::rename(&from, &to) {
            put_back(&moved, &aside);
            return Err(StorageError::File(format!("{}: {error}", from.display())));
        }
        moved.push((from, to));
    }
    // Absent is the ordinary case for all three: a clean shutdown leaves no sidecar, and a
    // library may hold no attachment at all.
    let optional = DATABASE_SIDECARS
        .into_iter()
        .chain(carries.then_some(ATTACHMENTS));
    for name in optional {
        let (from, to) = (library.join(name), aside.join(name));
        if std::fs::rename(&from, &to).is_ok() {
            moved.push((from, to));
        }
    }

    if let Err(error) = copy_back(&copy, library, carries) {
        // Back where they were: a failed restore must never leave no library at all.
        put_back(&moved, &aside);
        return Err(StorageError::File(error));
    }

    Ok(aside)
}

fn copy_back(copy: &Path, library: &Path, carries: bool) -> Result<(), String> {
    for name in [layout::DATABASE, KEY_FILE] {
        std::fs::copy(copy.join(name), library.join(name))
            .map_err(|error| format!("{name}: {error}"))?;
    }
    if carries {
        copy_files(&copy.join(ATTACHMENTS), &library.join(ATTACHMENTS))
            .map_err(|error| format!("{ATTACHMENTS}: {error}"))?;
    }

    Ok(())
}

fn put_back(moved: &[(PathBuf, PathBuf)], aside: &Path) {
    for (from, to) in moved {
        let _ = if from.is_dir() {
            std::fs::remove_dir_all(from)
        } else {
            std::fs::remove_file(from)
        };
        let _ = std::fs::rename(to, from);
    }
    let _ = std::fs::remove_dir_all(aside);
}

/// What [`rewrap`] managed. `left` is counted rather than failed: a locked backup must not
/// block the revocation at the moment it is asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Rewrapped {
    pub(crate) done: usize,
    pub(crate) left: usize,
}

/// Rewraps every retained copy's key file under the new phrase.
///
/// ⚠️ What makes a new passphrase revoke anything: `backups/` sits inside the profile, and each
/// copy's key file still wraps the one master key under a retired phrase. Written from the key
/// it is given, so every retired phrase goes, not only the last. `damaged/` has no key file of
/// its own (`recovery::set_aside` leaves `vault.json` in place).
pub(crate) fn rewrap(library: &Path, vault: &Vault, passphrase: &str, cost: Cost) -> Rewrapped {
    let mut tally = Rewrapped::default();

    for copy in existing(&library.join(BACKUPS)) {
        let path = copy.join(KEY_FILE);
        if !path.is_file() {
            continue;
        }

        // Staged and renamed by `write_wrapped`: a half-written key file is a backup lost.
        match crate::vault::file::write_wrapped(&path, vault, passphrase, cost) {
            Ok(()) => tally.done += 1,
            Err(error) => {
                log::warn!(
                    "{}: still opens with the old passphrase: {error}",
                    path.display()
                );
                tally.left += 1;
            }
        }
    }

    tally
}

/// Anything but a plain `"false"` keeps the copies: a missing or unreadable preference must
/// not switch a safety net off.
pub(crate) fn wanted(stored: Option<&str>) -> bool {
    stored != Some("false")
}

/// Read from the preferences file: the copy is taken at unlock, before the front end has
/// booted far enough to say anything.
fn wanted_by_preference(app: &AppHandle, key: &str) -> bool {
    use tauri_plugin_store::StoreExt;

    let stored = app
        .store(layout::PREFERENCES)
        .ok()
        .and_then(|store| store.get(key))
        .and_then(|value| value.as_str().map(str::to_owned));

    wanted(stored.as_deref())
}

/// The launch copy. Never fatal: a library that cannot be copied still has to open.
pub(crate) fn take(app: &AppHandle, db: &Db) {
    if wanted_by_preference(app, AUTOMATIC_BACKUPS_KEY) {
        copy(db, wanted_by_preference(app, BACKUP_ATTACHMENTS_KEY));
    }
}

fn copy(db: &Db, with_attachments: bool) {
    let mut connection = match crate::db::lock(db) {
        Ok(connection) => connection,
        Err(error) => {
            log::warn!("No backup taken: {error}");
            return;
        }
    };
    let directory = connection.directory().to_path_buf();

    match rotate(&directory, &mut connection, Utc::now(), with_attachments) {
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
    use crate::db;
    use crate::notes::store as notes;
    use crate::spaces::store as spaces;

    fn at(offset_hours: i64) -> DateTime<Utc> {
        db::iso8601::parse("2026-07-25T09:00:00.000Z").unwrap() + TimeDelta::hours(offset_hours)
    }

    /// A library with one note in it, and the key file beside it, as a real one is.
    fn library(directory: &Path) -> (Library, String) {
        let vault = crate::vault::file::create(
            directory,
            "a passphrase",
            crate::vault::key::Cost::FOR_TESTS,
        )
        .unwrap();

        let mut connection = db::open(&directory.join(layout::DATABASE), vault).unwrap();
        let space = spaces::create(&mut connection, "Perso").unwrap().id;
        let note = notes::create(
            &mut connection,
            crate::notes::model::NoteDraft {
                space_id: space,
                folder_id: None,
                title: "À sauvegarder".to_string(),
                language: crate::notes::language::Language::Txt,
                content: "psql -h prod".to_string(),
                source: String::new(),
                tags: Vec::new(),
                pinned: false,
                lifecycle: crate::notes::model::NoteLifecycle::Permanent,
                kind: crate::notes::checklist::NoteKind::Snippet,
                items: Vec::new(),
            },
            at(0),
        )
        .unwrap()
        .id;

        (connection, note)
    }

    #[test]
    fn the_launch_copy_lands_beside_the_open_library() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (connection, _) = library(&directory);
        let db: Db = std::sync::Mutex::new(Some(connection));

        copy(&db, true);

        assert_eq!(list(&directory).len(), 1);
        drop(db);
    }

    /// Closed before a single file moves: every command answers `Locked` afterwards.
    #[test]
    fn restoring_closes_the_library_and_says_where_the_replaced_one_went() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        let copy = rotate(&directory, &mut connection, at(0), false)
            .unwrap()
            .unwrap();
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

    /// A copy that cannot be opened is not a backup.
    #[test]
    fn a_copy_opens_as_a_library_and_holds_the_notes() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, note_id) = library(&directory);

        let target = rotate(&directory, &mut connection, at(0), false)
            .unwrap()
            .unwrap();

        let reopened = crate::vault::file::unlock(&target, "a passphrase").unwrap();
        let mut copy = db::open(&target.join(layout::DATABASE), reopened).unwrap();
        let written = notes::all(&mut copy, None).unwrap();

        assert_eq!(written.len(), 1);
        assert_eq!(written[0].id, note_id);
        assert_eq!(written[0].title, "À sauvegarder");
    }

    #[test]
    fn the_key_file_travels_with_the_database() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);

        let target = rotate(&directory, &mut connection, at(0), false)
            .unwrap()
            .unwrap();

        assert!(target.join(KEY_FILE).is_file());
        assert!(target.join(layout::DATABASE).is_file());
    }

    fn attach(directory: &Path, name: &str, bytes: &[u8]) {
        std::fs::create_dir_all(directory.join(ATTACHMENTS)).unwrap();
        std::fs::write(directory.join(ATTACHMENTS).join(name), bytes).unwrap();
    }

    #[test]
    fn the_attachments_travel_with_the_copy_when_asked() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        let sealed = b"\x89PNG sealed";
        attach(&directory, "a-1.png", sealed);

        let target = rotate(&directory, &mut connection, at(0), true)
            .unwrap()
            .unwrap();

        assert_eq!(
            std::fs::read(target.join(ATTACHMENTS).join("a-1.png")).unwrap(),
            sealed
        );
        let listed = &list(&directory)[0];
        assert!(listed.attachments);
        let database = std::fs::metadata(target.join(layout::DATABASE))
            .unwrap()
            .len();
        #[allow(clippy::cast_precision_loss)]
        let expected = (database + sealed.len() as u64) as f64;
        assert!(
            (listed.bytes - expected).abs() < f64::EPSILON,
            "the size counts them"
        );
    }

    /// The directory is what says a copy carries them, so it exists even with nothing in it.
    #[test]
    fn a_library_with_no_attachment_still_takes_a_copy_that_carries_them() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);

        let target = rotate(&directory, &mut connection, at(0), true)
            .unwrap()
            .unwrap();

        assert!(target.join(ATTACHMENTS).is_dir());
        assert!(list(&directory)[0].attachments);
    }

    #[test]
    fn a_copy_taken_without_them_says_so() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        attach(&directory, "a-1.png", b"\x89PNG");

        let target = rotate(&directory, &mut connection, at(0), false)
            .unwrap()
            .unwrap();

        assert!(!target.join(ATTACHMENTS).exists());
        assert!(!list(&directory)[0].attachments);
    }

    #[test]
    fn a_second_launch_the_same_day_takes_nothing() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        rotate(&directory, &mut connection, at(0), false)
            .unwrap()
            .unwrap();

        let again = rotate(&directory, &mut connection, at(3), false).unwrap();

        assert!(again.is_none());
        assert_eq!(existing(&directory.join(BACKUPS)).len(), 1);
    }

    #[test]
    fn a_launch_the_next_day_takes_another() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        rotate(&directory, &mut connection, at(0), false).unwrap();

        rotate(&directory, &mut connection, at(25), false)
            .unwrap()
            .unwrap();

        assert_eq!(existing(&directory.join(BACKUPS)).len(), 2);
    }

    #[test]
    fn only_the_last_few_are_kept() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);

        for day in 0..6 {
            rotate(&directory, &mut connection, at(day * 25), false).unwrap();
        }

        let kept = existing(&directory.join(BACKUPS));
        assert_eq!(kept.len(), KEEP);
        // The newest survive, not the first ones taken.
        assert_eq!(taken_at(&kept[0]).unwrap(), at(5 * 25));
    }

    #[test]
    fn only_a_preference_turning_them_off_turns_them_off() {
        assert!(wanted(Some("true")));
        assert!(wanted(None), "a missing preference keeps the copies");
        assert!(wanted(Some("")), "a truncated value keeps them too");
        assert!(wanted(Some("False")), "and anything this build cannot read");

        assert!(!wanted(Some("false")));
    }

    /// Changing the phrase is what somebody does when they believe the old one leaked, and each
    /// copy in the same profile holds a key file wrapped under it.
    #[test]
    fn changing_the_passphrase_stops_the_old_one_opening_a_backup() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        let target = rotate(&directory, &mut connection, at(0), false)
            .unwrap()
            .unwrap();

        let vault = crate::vault::file::change_passphrase(
            &directory,
            "a passphrase",
            "a new one",
            Cost::FOR_TESTS,
        )
        .unwrap();

        // Rewrapping the live file alone revokes nothing.
        crate::vault::file::unlock(&target, "a passphrase")
            .expect("the copy still opens with the retired phrase before the rewrap");

        let tally = rewrap(&directory, &vault, "a new one", Cost::FOR_TESTS);

        assert_eq!(tally, Rewrapped { done: 1, left: 0 });
        assert!(
            crate::vault::file::unlock(&target, "a passphrase").is_err(),
            "the retired passphrase still opens the backup"
        );
        // And the copy is not merely shut: it opens under the new one, on the same key.
        crate::vault::file::unlock(&target, "a new one").unwrap();
    }

    /// A copy is rewrapped from the key, not opened with the phrase being retired, so one taken
    /// two changes ago is reached as well.
    #[test]
    fn a_copy_left_over_from_an_older_phrase_is_reached_too() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        let first = rotate(&directory, &mut connection, at(0), false)
            .unwrap()
            .unwrap();

        let vault = crate::vault::file::change_passphrase(
            &directory,
            "a passphrase",
            "the second",
            Cost::FOR_TESTS,
        )
        .unwrap();
        // No rewrap here: the first copy stays under the very first phrase.
        let second = rotate(&directory, &mut connection, at(25), false)
            .unwrap()
            .unwrap();
        crate::vault::file::write_wrapped(
            &second.join(KEY_FILE),
            &vault,
            "the second",
            Cost::FOR_TESTS,
        )
        .unwrap();

        let vault = crate::vault::file::change_passphrase(
            &directory,
            "the second",
            "the third",
            Cost::FOR_TESTS,
        )
        .unwrap();
        let tally = rewrap(&directory, &vault, "the third", Cost::FOR_TESTS);

        assert_eq!(tally.done, 2);
        for copy in [&first, &second] {
            assert!(crate::vault::file::unlock(copy, "a passphrase").is_err());
            assert!(crate::vault::file::unlock(copy, "the second").is_err());
            crate::vault::file::unlock(copy, "the third").unwrap();
        }
    }

    /// A copy taken before the library was sealed has no key file to retire.
    #[test]
    fn a_copy_with_no_key_file_is_left_alone_rather_than_counted() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        let target = rotate(&directory, &mut connection, at(0), false)
            .unwrap()
            .unwrap();
        std::fs::remove_file(target.join(KEY_FILE)).unwrap();

        let vault = crate::vault::file::unlock(&directory, "a passphrase").unwrap();
        let tally = rewrap(&directory, &vault, "a new one", Cost::FOR_TESTS);

        assert_eq!(tally, Rewrapped { done: 0, left: 0 });
    }

    mod restoring {
        use super::*;

        /// Reads the one note's title straight out of a library on disk.
        fn title_in(directory: &Path, passphrase: &str) -> String {
            let vault = crate::vault::file::unlock(directory, passphrase).unwrap();
            let mut connection = db::open(&directory.join(layout::DATABASE), vault).unwrap();

            notes::all(&mut connection, None).unwrap()[0].title.clone()
        }

        fn retitle(connection: &mut Library, id: &str, title: &str) {
            notes::update(
                connection,
                id,
                &crate::notes::model::NotePatch {
                    title: Some(title.to_string()),
                    ..Default::default()
                },
                at(1),
            )
            .unwrap();
        }

        #[test]
        fn the_copy_takes_the_place_of_the_live_library() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, note) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0), false)
                .unwrap()
                .unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            retitle(&mut connection, &note, "écrit après la copie");
            drop(connection);

            replace(&directory, &id, at(30)).unwrap();

            assert_eq!(title_in(&directory, "a passphrase"), "À sauvegarder");
        }

        #[test]
        fn the_library_it_replaced_is_still_readable_where_it_was_put() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, note) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0), false)
                .unwrap()
                .unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            retitle(&mut connection, &note, "écrit après la copie");
            drop(connection);

            let aside = replace(&directory, &id, at(30)).unwrap();

            assert!(aside.starts_with(directory.join(REPLACED)));
            assert_eq!(title_in(&aside, "a passphrase"), "écrit après la copie");
        }

        /// A database from one wrapping and a key file from another open nothing.
        #[test]
        fn the_key_file_goes_one_way_and_comes_back_the_other() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, _) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0), false)
                .unwrap()
                .unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            drop(connection);

            let aside = replace(&directory, &id, at(30)).unwrap();

            assert!(directory.join(KEY_FILE).is_file());
            assert!(aside.join(KEY_FILE).is_file());
        }

        /// The live ones go aside with the database: one written since the copy is not lost,
        /// and the restored records name only what the copy brought.
        #[test]
        fn the_attachments_come_back_with_a_copy_that_carries_them() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, _) = library(&directory);
            attach(&directory, "a-1.png", b"as copied");
            let copy = rotate(&directory, &mut connection, at(0), true)
                .unwrap()
                .unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            drop(connection);
            std::fs::remove_file(directory.join(ATTACHMENTS).join("a-1.png")).unwrap();
            attach(&directory, "a-2.png", b"added since");

            let aside = replace(&directory, &id, at(30)).unwrap();

            assert_eq!(
                std::fs::read(directory.join(ATTACHMENTS).join("a-1.png")).unwrap(),
                b"as copied"
            );
            assert!(!directory.join(ATTACHMENTS).join("a-2.png").exists());
            assert!(aside.join(ATTACHMENTS).join("a-2.png").is_file());
        }

        /// A failed copy leaves both halves as they were, directories included.
        #[test]
        fn putting_back_undoes_a_half_finished_restore() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            attach(&directory, "a-1.png", b"live");
            std::fs::write(directory.join(KEY_FILE), b"live key").unwrap();
            let aside = directory.join(REPLACED).join("2026-07-25_09-00-00");
            std::fs::create_dir_all(&aside).unwrap();
            let mut moved = Vec::new();
            for name in [KEY_FILE, ATTACHMENTS] {
                std::fs::rename(directory.join(name), aside.join(name)).unwrap();
                moved.push((directory.join(name), aside.join(name)));
            }
            std::fs::write(directory.join(KEY_FILE), b"half copied").unwrap();
            attach(&directory, "from-the-copy.png", b"half");

            put_back(&moved, &aside);

            assert_eq!(
                std::fs::read(directory.join(KEY_FILE)).unwrap(),
                b"live key"
            );
            assert!(directory.join(ATTACHMENTS).join("a-1.png").is_file());
            assert!(
                !directory
                    .join(ATTACHMENTS)
                    .join("from-the-copy.png")
                    .exists()
            );
            assert!(!aside.exists());
        }

        /// Not in this copy: moving them aside would point every restored record at a file that left.
        #[test]
        fn the_attachments_stay_where_they_are_when_the_copy_has_none() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, _) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0), false)
                .unwrap()
                .unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            drop(connection);
            std::fs::create_dir_all(directory.join("attachments")).unwrap();
            std::fs::write(directory.join("attachments").join("a-1.png"), b"\x89PNG").unwrap();

            replace(&directory, &id, at(30)).unwrap();

            assert!(directory.join("attachments").join("a-1.png").is_file());
        }

        #[test]
        fn an_id_cannot_name_a_directory_outside_the_copies() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, _) = library(&directory);
            rotate(&directory, &mut connection, at(0), false).unwrap();
            drop(connection);
            let outside = directory.join("2026-07-25_09-00-00");
            std::fs::create_dir_all(&outside).unwrap();
            std::fs::write(outside.join(layout::DATABASE), b"not a library").unwrap();
            std::fs::write(outside.join(KEY_FILE), b"{}").unwrap();

            let refused = replace(&directory, "../2026-07-25_09-00-00", at(30));

            assert!(refused.is_err());
            assert!(
                directory.join(layout::DATABASE).is_file(),
                "the library is still there"
            );
        }

        #[test]
        fn a_copy_with_no_key_file_is_refused_rather_than_swapped_in() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, _) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0), false)
                .unwrap()
                .unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            drop(connection);
            std::fs::remove_file(copy.join(KEY_FILE)).unwrap();

            let refused = replace(&directory, &id, at(30));

            assert!(refused.is_err());
            assert!(directory.join(layout::DATABASE).is_file());
            assert!(!directory.join(REPLACED).exists(), "nothing was set aside");
        }

        #[test]
        fn a_listing_says_when_each_copy_was_taken_and_whether_it_opens() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, _) = library(&directory);
            rotate(&directory, &mut connection, at(0), false).unwrap();
            rotate(&directory, &mut connection, at(25), false).unwrap();

            let copies = list(&directory);

            assert_eq!(copies.len(), 2);
            // Newest first, the order the panel wants and the pruning uses.
            assert_eq!(copies[0].taken_at, at(25));
            assert!(copies[0].bytes > 0.0);
            assert!(copies.iter().all(|copy| copy.openable));
        }
    }

    /// A stray directory must not hold the rotation off by looking like the newest copy.
    #[test]
    fn something_that_is_not_a_copy_is_ignored() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        std::fs::create_dir_all(directory.join(BACKUPS).join("notes de Valentin")).unwrap();

        let target = rotate(&directory, &mut connection, at(0), false).unwrap();

        assert!(target.is_some());
    }
}
