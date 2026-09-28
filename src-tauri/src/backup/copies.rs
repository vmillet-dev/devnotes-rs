//! The copies on disk: taking one, listing them, putting one back, and rewrapping their keys.

use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeDelta, Utc};
use diesel::prelude::*;
use diesel::sql_types::Text;

use crate::db::Library;
use crate::error::{FileContext, StorageError};
use crate::layout::{self, ATTACHMENTS, BACKUPS, DATABASE_SIDECARS, KEY_FILE, REPLACED};
use crate::vault::key::{Cost, Vault};

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
/// nobody can ever open. The attachments are linked as they are, sealed under that same key.
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
            // A half-written copy would be the newest, and hold the next rotation off.
            let _ = std::fs::remove_dir_all(&target);
            StorageError::File(format!("{}: {error}", copy.display()))
        })?;

    std::fs::copy(library.join(KEY_FILE), target.join(KEY_FILE)).map_err(|error| {
        let _ = std::fs::remove_dir_all(&target);
        StorageError::File(format!("{KEY_FILE}: {error}"))
    })?;

    link_files(&library.join(ATTACHMENTS), &target.join(ATTACHMENTS)).map_err(|error| {
        let _ = std::fs::remove_dir_all(&target);
        StorageError::File(format!("{ATTACHMENTS}: {error}"))
    })?;

    for old in existing(&directory).into_iter().skip(KEEP) {
        // Best effort: a copy that resists deletion is not worth failing a launch over.
        let _ = std::fs::remove_dir_all(old);
    }

    Ok(Some(target))
}

/// Hard links, copied only where the file system refuses one: an attachment is never rewritten
/// — a new one is a new id — and a purge only unlinks, so every copy shares the same bytes.
///
/// ⚠️ A name already there is left alone: it is the same attachment, often the very same file,
/// and copying over it would truncate the source being read. `attachments/` is flat.
fn link_files(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry?.path();
        let Some(name) = path.file_name().filter(|_| path.is_file()) else {
            continue;
        };
        let target = to.join(name);
        match std::fs::hard_link(&path, &target) {
            Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
                std::fs::copy(&path, &target)?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// One copy, as the interface lists it. `bytes` is `f64` because specta refuses the integer
/// types JSON cannot carry exactly.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// The folder's name, which is its stamp, and what a restore asks for.
    pub id: String,
    pub taken_at: DateTime<Utc>,
    /// The database alone: the attachments are shared with the library and the other copies.
    pub bytes: f64,
    /// Whether the key file travelled with it: without one, restoring loses the library.
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
/// Moved aside into `replaced/`, never deleted, and `vault.json` with it: the copy brings its
/// own, and a database and a key from two wrappings open nothing. `attachments/` goes too, and
/// the copy's is linked back: an attachment added since is kept aside, not lost.
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
    for name in DATABASE_SIDECARS.into_iter().chain([ATTACHMENTS]) {
        let (from, to) = (library.join(name), aside.join(name));
        if std::fs::rename(&from, &to).is_ok() {
            moved.push((from, to));
        }
    }

    if let Err(error) = copy_back(&copy, library) {
        // Back where they were: a failed restore must never leave no library at all.
        put_back(&moved, &aside);
        return Err(StorageError::File(error));
    }

    Ok(aside)
}

fn copy_back(copy: &Path, library: &Path) -> Result<(), String> {
    for name in [layout::DATABASE, KEY_FILE] {
        std::fs::copy(copy.join(name), library.join(name))
            .map_err(|error| format!("{name}: {error}"))?;
    }

    link_files(&copy.join(ATTACHMENTS), &library.join(ATTACHMENTS))
        .map_err(|error| format!("{ATTACHMENTS}: {error}"))
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db;
    use crate::notes::store as notes;
    use crate::spaces::store as spaces;

    pub(crate) fn at(offset_hours: i64) -> DateTime<Utc> {
        db::iso8601::parse("2026-07-25T09:00:00.000Z").unwrap() + TimeDelta::hours(offset_hours)
    }

    /// A library with one note in it, and the key file beside it, as a real one is.
    pub(crate) fn library(directory: &Path) -> (Library, String) {
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

    /// A copy that cannot be opened is not a backup.
    #[test]
    fn a_copy_opens_as_a_library_and_holds_the_notes() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, note_id) = library(&directory);

        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

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

        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

        assert!(target.join(KEY_FILE).is_file());
        assert!(target.join(layout::DATABASE).is_file());
    }

    fn attach(directory: &Path, name: &str, bytes: &[u8]) {
        std::fs::create_dir_all(directory.join(ATTACHMENTS)).unwrap();
        std::fs::write(directory.join(ATTACHMENTS).join(name), bytes).unwrap();
    }

    /// Purged from the library afterwards, an attachment is still whole in the copy: the link
    /// is what goes, not the bytes.
    #[test]
    fn the_attachments_travel_with_the_copy_and_outlive_a_purge() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        attach(&directory, "a-1.png", b"\x89PNG sealed");

        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
        std::fs::remove_file(directory.join(ATTACHMENTS).join("a-1.png")).unwrap();

        assert_eq!(
            std::fs::read(target.join(ATTACHMENTS).join("a-1.png")).unwrap(),
            b"\x89PNG sealed"
        );
    }

    /// One file under two names, not two files: what is written through one reads through the
    /// other. Nothing in the application writes an attachment twice; this shows the copy is free.
    #[test]
    fn the_copy_shares_the_attachments_rather_than_duplicating_them() {
        use std::io::Write;

        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        attach(&directory, "a-1.png", b"linked");

        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(directory.join(ATTACHMENTS).join("a-1.png"))
            .unwrap()
            .write_all(b", once")
            .unwrap();

        assert_eq!(
            std::fs::read(target.join(ATTACHMENTS).join("a-1.png")).unwrap(),
            b"linked, once"
        );
    }

    /// Counting the shared attachments in each copy would make three copies look like three
    /// times the disk.
    #[test]
    fn a_copy_is_listed_at_the_size_of_its_database() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        attach(&directory, "a-1.png", &[0u8; 4096]);

        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

        let database = std::fs::metadata(target.join(layout::DATABASE))
            .unwrap()
            .len();
        #[allow(clippy::cast_precision_loss)]
        let expected = database as f64;
        assert!((list(&directory)[0].bytes - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn linking_onto_a_name_already_there_leaves_both_whole() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        attach(&directory, "a-1.png", b"the bytes");
        let into = directory.join("into");

        link_files(&directory.join(ATTACHMENTS), &into).unwrap();
        link_files(&directory.join(ATTACHMENTS), &into).unwrap();

        assert_eq!(std::fs::read(into.join("a-1.png")).unwrap(), b"the bytes");
        assert_eq!(
            std::fs::read(directory.join(ATTACHMENTS).join("a-1.png")).unwrap(),
            b"the bytes"
        );
    }

    #[test]
    fn a_second_launch_the_same_day_takes_nothing() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

        let again = rotate(&directory, &mut connection, at(3)).unwrap();

        assert!(again.is_none());
        assert_eq!(existing(&directory.join(BACKUPS)).len(), 1);
    }

    #[test]
    fn a_launch_the_next_day_takes_another() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let (mut connection, _) = library(&directory);
        rotate(&directory, &mut connection, at(0)).unwrap();

        rotate(&directory, &mut connection, at(25))
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
            rotate(&directory, &mut connection, at(day * 25)).unwrap();
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
        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

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
        let first = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();

        let vault = crate::vault::file::change_passphrase(
            &directory,
            "a passphrase",
            "the second",
            Cost::FOR_TESTS,
        )
        .unwrap();
        // No rewrap here: the first copy stays under the very first phrase.
        let second = rotate(&directory, &mut connection, at(25))
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
        let target = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
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
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
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
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
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
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
            let id = copy.file_name().unwrap().to_str().unwrap().to_string();
            drop(connection);

            let aside = replace(&directory, &id, at(30)).unwrap();

            assert!(directory.join(KEY_FILE).is_file());
            assert!(aside.join(KEY_FILE).is_file());
        }

        /// The live ones go aside with the database: one written since the copy is not lost,
        /// and the restored records name only what the copy brought.
        #[test]
        fn the_attachments_come_back_with_the_copy() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, _) = library(&directory);
            attach(&directory, "a-1.png", b"as copied");
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
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

        #[test]
        fn an_id_cannot_name_a_directory_outside_the_copies() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
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
        }

        #[test]
        fn a_copy_with_no_key_file_is_refused_rather_than_swapped_in() {
            let scratch = tempfile::tempdir().unwrap();
            let directory = scratch.path().to_path_buf();
            let (mut connection, _) = library(&directory);
            let copy = rotate(&directory, &mut connection, at(0)).unwrap().unwrap();
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
            rotate(&directory, &mut connection, at(0)).unwrap();
            rotate(&directory, &mut connection, at(25)).unwrap();

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

        let target = rotate(&directory, &mut connection, at(0)).unwrap();

        assert!(target.is_some());
    }
}
