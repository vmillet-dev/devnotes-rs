//! A rolling copy of the library, taken at launch.
//!
//! ⚠️ The trash protects a note from being deleted; nothing protected the **file**. A
//! dead disk, a botched migration or an emptied trash took the library with it, and an
//! export only helps the person who remembered to make one.
//!
//! ⚠️ `VACUUM INTO` rather than a file copy: under WAL the database file on its own is
//! not a consistent snapshot — the committed pages may still be in the write-ahead log —
//! so copying it can produce something that opens short of what was written.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};

use chrono::{DateTime, TimeDelta, Utc};
use diesel::prelude::*;
use diesel::sql_types::Text;

use crate::db::{Db, Library};
use crate::error::{AppError, FileContext, StorageError};
use crate::layout::{self, BACKUPS, DATABASE_SIDECARS, KEY_FILE, REPLACED};
use crate::vault::key::{Cost, Vault};

/// The key the front end writes this setting under, in the application's preferences.
pub(crate) const AUTOMATIC_BACKUPS_KEY: &str = "devnotes.automaticBackups";

/// How many are kept. Enough to reach past the launch that went wrong without turning the
/// data directory into a second library.
pub(crate) const KEEP: usize = 3;

/// ⚠️ At most one a day, not one per launch: five launches in an hour would otherwise
/// rotate every older copy out, which is exactly the history a backup is for.
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

    // Newest first, which is the order both the age check and the pruning want.
    taken.sort_by_key(|(at, _)| std::cmp::Reverse(*at));

    taken.into_iter().map(|(_, path)| path).collect()
}

/// Takes one if the newest is older than a day, then prunes to [`KEEP`].
///
/// ⚠️ The key file travels with the database, and must: the library is sealed, and a
/// copy of it without `vault.json` is a file nobody can ever open again.
pub(crate) fn rotate(
    library: &Path,
    connection: &mut Library,
    now: DateTime<Utc>,
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
            // A half-written copy is worse than none: it would be the newest, and would
            // hold the next day's rotation off.
            let _ = std::fs::remove_dir_all(&target);
            StorageError::File(format!("{}: {error}", copy.display()))
        })?;

    std::fs::copy(library.join(KEY_FILE), target.join(KEY_FILE)).map_err(|error| {
        let _ = std::fs::remove_dir_all(&target);
        StorageError::File(format!("{KEY_FILE}: {error}"))
    })?;

    for old in existing(&directory).into_iter().skip(KEEP) {
        // Best effort: a copy that resists deletion is not worth failing a launch over.
        let _ = std::fs::remove_dir_all(old);
    }

    Ok(Some(target))
}

/// One copy, as the interface lists it.
///
/// ⚠️ `bytes` is `f64` rather than `u64`: specta refuses the integer types JSON cannot
/// carry without losing precision, and a size is the one field where a float says the
/// same thing.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// The folder's name, which is its stamp — and what a restore is asked for by.
    pub id: String,
    pub taken_at: DateTime<Utc>,
    pub bytes: f64,
    /// ⚠️ Whether the key file travelled with it. Without one, the copy is a file nobody
    /// can open, and offering to restore it would be offering to lose the library.
    pub openable: bool,
}

#[allow(clippy::cast_precision_loss)]
fn describe(copy: &Path) -> Option<Backup> {
    let at = taken_at(copy)?;
    let database = copy.join(layout::DATABASE);
    let bytes = std::fs::metadata(&database).map_or(0, |meta| meta.len());

    Some(Backup {
        id: copy.file_name()?.to_str()?.to_string(),
        taken_at: at,
        bytes: bytes as f64,
        openable: database.is_file() && copy.join(KEY_FILE).is_file(),
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
/// ⚠️ The library being replaced is **moved aside, never deleted**. This is the one
/// gesture in the application that can lose a whole corpus, and a folder with a date on
/// it is the difference between a mistake and a loss.
///
/// ⚠️ `vault.json` moves with it, unlike the damaged case: the copy brings its own key
/// file, and the two must not be mixed — a database from one wrapping and a key from
/// another opens nothing.
///
/// ⚠️ `attachments/` stays where it is. The copies do not carry it — it is the bulk of a
/// profile — so moving it aside would point every restored record at a file that left.
/// The next launch's orphan sweep then collects whatever the restored library no longer
/// names, which is the right answer: those files belong to notes it does not have.
pub(crate) fn replace(
    library: &Path,
    id: &str,
    now: DateTime<Utc>,
) -> Result<PathBuf, StorageError> {
    // ⚠️ Matched against the listing rather than joined onto the directory: an id comes
    // from the front end, and `../2026-01-01_00-00-00` joins to a path outside
    // `backups/` whose file name still parses as a stamp. A name from outside has no
    // business deciding which directory this reads.
    let copy = existing(&library.join(BACKUPS))
        .into_iter()
        .find(|path| path.file_name().and_then(std::ffi::OsStr::to_str) == Some(id))
        .ok_or_else(|| StorageError::File(format!("{id}: no such copy")))?;

    if !copy.join(layout::DATABASE).is_file() || !copy.join(KEY_FILE).is_file() {
        return Err(StorageError::File(format!(
            "{id}: not a copy that can be opened"
        )));
    }

    let aside = library.join(REPLACED).join(layout::stamp(now));
    std::fs::create_dir_all(&aside).context(aside.display())?;

    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for name in [layout::DATABASE, KEY_FILE] {
        let from = library.join(name);
        let to = aside.join(name);
        std::fs::rename(&from, &to).context(from.display())?;
        moved.push((from, to));
    }
    for sidecar in DATABASE_SIDECARS {
        // Absent is the ordinary case: a clean shutdown leaves neither.
        let _ = std::fs::rename(library.join(sidecar), aside.join(sidecar));
    }

    for name in [layout::DATABASE, KEY_FILE] {
        if let Err(error) = std::fs::copy(copy.join(name), library.join(name)) {
            // ⚠️ Back where they were, or a failed restore leaves no library at all —
            // which is the exact outcome this whole function exists to avoid.
            for (from, to) in &moved {
                let _ = std::fs::remove_file(from);
                let _ = std::fs::rename(to, from);
            }
            let _ = std::fs::remove_dir_all(&aside);

            return Err(StorageError::File(format!("{name}: {error}")));
        }
    }

    Ok(aside)
}

/// What [`rewrap`] managed. ⚠️ `left` is not a failure to report as one: refusing to
/// rotate the phrase because a backup's file is locked would block the revocation at the
/// moment it is asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Rewrapped {
    pub(crate) done: usize,
    pub(crate) left: usize,
}

/// Rewraps every retained copy's key file under the new phrase.
///
/// ⚠️ This is what makes changing the passphrase revoke anything. `backups/` sits **inside
/// the profile it copies**, so whoever copies the profile copies every phrase the user has
/// ever retired — and the envelope means one master key for the life of the library, so any
/// key file ever written is a permanent escrow for it (#157).
///
/// ⚠️ It writes the key it is **given** rather than opening each copy with the old phrase:
/// a backup's file wraps that same master key whatever phrase was current when it was
/// taken, so this revokes *every* retired phrase rather than only the last one. A copy
/// with no key file predates the library being sealed and is left alone.
///
/// `damaged/` is deliberately absent: `recovery::set_aside` leaves `vault.json` where it
/// is, so a set-aside library has no wrapping of its own to retire.
pub(crate) fn rewrap(library: &Path, vault: &Vault, passphrase: &str, cost: Cost) -> Rewrapped {
    let mut tally = Rewrapped::default();

    for copy in existing(&library.join(BACKUPS)) {
        let path = copy.join(KEY_FILE);
        if !path.is_file() {
            continue;
        }

        // Staged and renamed by `write_wrapped`: a key file half written is a backup lost.
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

/// ⚠️ Anything but a plain `"false"` keeps the copies. A preferences file that is
/// missing, truncated, or written by a version that spells this differently must not
/// silently switch a safety net off — the only thing that turns it off is somebody
/// turning it off.
pub(crate) fn wanted(stored: Option<&str>) -> bool {
    stored != Some("false")
}

/// ⚠️ Read from the preferences file rather than handed over by the front end: the copy
/// is taken at unlock, before the front has booted far enough to tell anyone anything.
fn wanted_by_preference(app: &AppHandle) -> bool {
    use tauri_plugin_store::StoreExt;

    let stored = app
        .store(layout::PREFERENCES)
        .ok()
        .and_then(|store| store.get(AUTOMATIC_BACKUPS_KEY))
        .and_then(|value| value.as_str().map(str::to_owned));

    wanted(stored.as_deref())
}

/// The launch copy. ⚠️ Never fatal and never in the way: a library that cannot be
/// copied still has to open.
pub(crate) fn take(app: &AppHandle, db: &crate::db::Db) {
    if !wanted_by_preference(app) {
        return;
    }

    let Ok(directory) = crate::libraries::open_directory(app) else {
        return;
    };

    let mut connection = match crate::db::lock(db) {
        Ok(connection) => connection,
        Err(error) => {
            log::warn!("No backup taken: {error}");
            return;
        }
    };

    match rotate(&directory, &mut connection, Utc::now()) {
        Ok(Some(target)) => log::info!("Library copied to {}", target.display()),
        Ok(None) => {}
        Err(error) => log::warn!("No backup taken: {error}"),
    }
}

fn library_directory(app: &AppHandle) -> Result<PathBuf, StorageError> {
    crate::libraries::open_directory(app)
}

/// The copies that exist, newest first, for the panel that lists them.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn list_backups(app: AppHandle) -> Result<Vec<Backup>, AppError> {
    Ok(list(&library_directory(&app)?))
}

/// Puts a copy back, and answers where the library it replaced was moved to.
///
/// ⚠️ It **closes the library** first, under the same lock that guards every other
/// command: renaming a database file out from under a live connection is how a working
/// library becomes a lost one. Every command answers `Locked` afterwards, which is what
/// sends the interface back to the gate — the restored copy needs a passphrase, and
/// asking for it is the only proof the right file is in place.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command(async)]
#[specta::specta]
pub fn restore_backup(id: String, app: AppHandle, db: State<'_, Db>) -> Result<String, AppError> {
    let directory = library_directory(&app)?;

    let mut open = db.lock().map_err(|_| StorageError::Unavailable)?;
    // Dropped before a single file moves, and held for the whole swap so nothing can
    // reopen it halfway through.
    *open = None;

    let aside = replace(&directory, &id, Utc::now())?;

    Ok(aside.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::notes::store as notes;
    use crate::spaces::store as spaces;

    fn scratch() -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("devnotes-backup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    /// ⚠️ The shipped cost is ~52 ms a derivation and these tests derive a dozen times.
    fn cheap() -> Cost {
        Cost {
            memory_kib: 64,
            passes: 1,
            lanes: 1,
        }
    }

    fn at(offset_hours: i64) -> DateTime<Utc> {
        db::iso8601::parse("2026-07-25T09:00:00.000Z").unwrap() + TimeDelta::hours(offset_hours)
    }

    /// A library with one note in it, and the key file beside it, as a real one is.
    fn library(directory: &Path) -> (Library, String) {
        let vault = crate::vault::file::create(
            directory,
            "a passphrase",
            crate::vault::key::Cost {
                memory_kib: 64,
                passes: 1,
                lanes: 1,
            },
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

    /// ⚠️ The only test that matters: a copy that cannot be opened is not a backup.
    #[test]
    fn a_copy_opens_as_a_library_and_holds_the_notes() {
        let directory = scratch();
        let (mut connection, note_id) = library(&directory);

        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

        let reopened = crate::vault::file::unlock(&target, "a passphrase").unwrap();
        let mut copy = db::open(&target.join(layout::DATABASE), reopened).unwrap();
        let written = notes::all(&mut copy, None).unwrap();

        assert_eq!(written.len(), 1);
        assert_eq!(written[0].id, note_id);
        assert_eq!(written[0].title, "À sauvegarder");
        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ Without the key file the copy is a file nobody can ever open again.
    #[test]
    fn the_key_file_travels_with_the_database() {
        let directory = scratch();
        let (mut connection, _) = library(&directory);

        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

        assert!(target.join(KEY_FILE).is_file());
        assert!(target.join(layout::DATABASE).is_file());
        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ Five launches in an hour must not rotate the history out.
    #[test]
    fn a_second_launch_the_same_day_takes_nothing() {
        let directory = scratch();
        let (mut connection, _) = library(&directory);
        rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

        let again = rotate(&directory, &mut connection, at(3)).unwrap();

        assert!(again.is_none());
        assert_eq!(existing(&directory.join(BACKUPS)).len(), 1);
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_launch_the_next_day_takes_another() {
        let directory = scratch();
        let (mut connection, _) = library(&directory);
        rotate(&directory, &mut connection, at(0)).unwrap();

        rotate(&directory, &mut connection, at(25))
            .unwrap()
            .unwrap();

        assert_eq!(existing(&directory.join(BACKUPS)).len(), 2);
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn only_the_last_few_are_kept() {
        let directory = scratch();
        let (mut connection, _) = library(&directory);

        for day in 0..6 {
            rotate(&directory, &mut connection, at(day * 25)).unwrap();
        }

        let kept = existing(&directory.join(BACKUPS));
        assert_eq!(kept.len(), KEEP);
        // The newest survive, not the first ones taken.
        assert_eq!(taken_at(&kept[0]).unwrap(), at(5 * 25));
        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ Fail safe: only a deliberate "false" stops the copies.
    #[test]
    fn only_a_preference_turning_them_off_turns_them_off() {
        assert!(wanted(Some("true")));
        assert!(wanted(None), "a missing preference keeps the copies");
        assert!(wanted(Some("")), "a truncated value keeps them too");
        assert!(wanted(Some("False")), "and anything this build cannot read");

        assert!(!wanted(Some("false")));
    }

    /// ⚠️ The point of #157. Changing the passphrase is the gesture somebody makes when
    /// they believe the old one leaked, and every retained copy kept a key file still
    /// wrapped under it — in the same profile directory. It revoked nothing.
    #[test]
    fn changing_the_passphrase_stops_the_old_one_opening_a_backup() {
        let directory = scratch();
        let (mut connection, _) = library(&directory);
        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

        let vault =
            crate::vault::file::change_passphrase(&directory, "a passphrase", "a new one", cheap())
                .unwrap();

        // The gap itself, kept in the test: rewrapping the live file alone revokes nothing.
        crate::vault::file::unlock(&target, "a passphrase")
            .expect("the copy still opens with the retired phrase before the rewrap");

        let tally = rewrap(&directory, &vault, "a new one", cheap());

        assert_eq!(tally, Rewrapped { done: 1, left: 0 });
        assert!(
            crate::vault::file::unlock(&target, "a passphrase").is_err(),
            "the retired passphrase still opens the backup"
        );
        // And the copy is not merely shut: it opens under the new one, on the same key.
        crate::vault::file::unlock(&target, "a new one").unwrap();
        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ Every retired phrase, not only the last: a copy is rewrapped from the key the
    /// live file just gave up, so one taken two changes ago is reached as well. Opening
    /// each copy with the phrase being retired would have missed exactly this one.
    #[test]
    fn a_copy_left_over_from_an_older_phrase_is_reached_too() {
        let directory = scratch();
        let (mut connection, _) = library(&directory);
        let first = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

        let vault = crate::vault::file::change_passphrase(
            &directory,
            "a passphrase",
            "the second",
            cheap(),
        )
        .unwrap();
        // ⚠️ No rewrap here, so the first copy stays under the very first phrase.
        let second = rotate(&directory, &mut connection, at(25))
            .unwrap()
            .unwrap();
        crate::vault::file::write_wrapped(&second.join(KEY_FILE), &vault, "the second", cheap())
            .unwrap();

        let vault =
            crate::vault::file::change_passphrase(&directory, "the second", "the third", cheap())
                .unwrap();
        let tally = rewrap(&directory, &vault, "the third", cheap());

        assert_eq!(tally.done, 2);
        for copy in [&first, &second] {
            assert!(crate::vault::file::unlock(copy, "a passphrase").is_err());
            assert!(crate::vault::file::unlock(copy, "the second").is_err());
            crate::vault::file::unlock(copy, "the third").unwrap();
        }
        std::fs::remove_dir_all(&directory).ok();
    }

    /// A copy taken before the library was sealed has no key file to retire.
    #[test]
    fn a_copy_with_no_key_file_is_left_alone_rather_than_counted() {
        let directory = scratch();
        let (mut connection, _) = library(&directory);
        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
        std::fs::remove_file(target.join(KEY_FILE)).unwrap();

        let vault = crate::vault::file::unlock(&directory, "a passphrase").unwrap();
        let tally = rewrap(&directory, &vault, "a new one", cheap());

        assert_eq!(tally, Rewrapped { done: 0, left: 0 });
        std::fs::remove_dir_all(&directory).ok();
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

        /// ⚠️ The whole point: the copy is what opens afterwards, not the live file.
        #[test]
        fn the_copy_takes_the_place_of_the_live_library() {
            let directory = scratch();
            let (mut connection, note) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            retitle(&mut connection, &note, "écrit après la copie");
            drop(connection);

            replace(&directory, &id, at(30)).unwrap();

            assert_eq!(title_in(&directory, "a passphrase"), "À sauvegarder");
            std::fs::remove_dir_all(&directory).ok();
        }

        /// ⚠️ Moved aside, never deleted: this is the one gesture that can lose a corpus.
        #[test]
        fn the_library_it_replaced_is_still_readable_where_it_was_put() {
            let directory = scratch();
            let (mut connection, note) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            retitle(&mut connection, &note, "écrit après la copie");
            drop(connection);

            let aside = replace(&directory, &id, at(30)).unwrap();

            assert!(aside.starts_with(directory.join(REPLACED)));
            assert_eq!(title_in(&aside, "a passphrase"), "écrit après la copie");
            std::fs::remove_dir_all(&directory).ok();
        }

        /// ⚠️ A database from one wrapping and a key file from another opens nothing.
        #[test]
        fn the_key_file_goes_one_way_and_comes_back_the_other() {
            let directory = scratch();
            let (mut connection, _) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            drop(connection);

            let aside = replace(&directory, &id, at(30)).unwrap();

            assert!(directory.join(KEY_FILE).is_file());
            assert!(aside.join(KEY_FILE).is_file());
            std::fs::remove_dir_all(&directory).ok();
        }

        /// ⚠️ They are not in the copies, so moving them aside would point every restored
        /// record at a file that left.
        #[test]
        fn the_attachments_stay_where_they_are() {
            let directory = scratch();
            let (mut connection, _) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            drop(connection);
            std::fs::create_dir_all(directory.join("attachments")).unwrap();
            std::fs::write(directory.join("attachments").join("a-1.png"), b"\x89PNG").unwrap();

            replace(&directory, &id, at(30)).unwrap();

            assert!(directory.join("attachments").join("a-1.png").is_file());
            std::fs::remove_dir_all(&directory).ok();
        }

        /// ⚠️ An id comes from the front end: `../…` joins to a path outside `backups/`
        /// whose file name still parses as a stamp.
        #[test]
        fn an_id_cannot_name_a_directory_outside_the_copies() {
            let directory = scratch();
            let (mut connection, _) = library(&directory);
            rotate(&directory, &mut connection, at(0)).unwrap();
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
            std::fs::remove_dir_all(&directory).ok();
        }

        /// ⚠️ Offering to restore it would be offering to lose the library for nothing.
        #[test]
        fn a_copy_with_no_key_file_is_refused_rather_than_swapped_in() {
            let directory = scratch();
            let (mut connection, _) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            drop(connection);
            std::fs::remove_file(copy.join(KEY_FILE)).unwrap();

            let refused = replace(&directory, &id, at(30));

            assert!(refused.is_err());
            assert!(directory.join(layout::DATABASE).is_file());
            assert!(!directory.join(REPLACED).exists(), "nothing was set aside");
            std::fs::remove_dir_all(&directory).ok();
        }

        #[test]
        fn a_listing_says_when_each_copy_was_taken_and_whether_it_opens() {
            let directory = scratch();
            let (mut connection, _) = library(&directory);
            rotate(&directory, &mut connection, at(0)).unwrap();
            rotate(&directory, &mut connection, at(25)).unwrap();

            let copies = list(&directory);

            assert_eq!(copies.len(), 2);
            // Newest first, which is the order the panel wants and the pruning uses.
            assert_eq!(copies[0].taken_at, at(25));
            assert!(copies[0].bytes > 0.0);
            assert!(copies.iter().all(|copy| copy.openable));
            std::fs::remove_dir_all(&directory).ok();
        }
    }

    /// A directory somebody dropped in there is not a backup, and must not hold the
    /// rotation off by looking like the newest one.
    #[test]
    fn something_that_is_not_a_copy_is_ignored() {
        let directory = scratch();
        let (mut connection, _) = library(&directory);
        std::fs::create_dir_all(directory.join(BACKUPS).join("notes de Valentin")).unwrap();

        let target = rotate(&directory, &mut connection, at(0)).unwrap();

        assert!(target.is_some());
        std::fs::remove_dir_all(&directory).ok();
    }
}
