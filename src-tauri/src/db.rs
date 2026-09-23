pub mod migration;
pub mod schema;

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use diesel::connection::SimpleConnection;
use diesel::prelude::*;

use crate::error::StorageError;
use crate::vault::key::Vault;

/// The connection, and the key everything it holds is sealed with.
///
/// ⚠️ It derefs to the connection, so a caller keeps writing `&mut connection`. What it
/// does **not** do is stand in for one where Diesel expects it: `load` and its siblings
/// take their connection as a generic parameter, and a generic gets no deref coercion —
/// hence [`Library::db`] at every query.
pub struct Library {
    connection: SqliteConnection,
    vault: Vault,
    directory: PathBuf,
}

impl Library {
    /// The connection, for the Diesel call that needs it by that name.
    pub fn db(&mut self) -> &mut SqliteConnection {
        &mut self.connection
    }

    /// ⚠️ Both halves at once. Two calls would not do: one borrows mutably and the other
    /// shared, and the compiler cannot see they touch different fields until they are
    /// destructured together.
    pub fn split(&mut self) -> (&mut SqliteConnection, &Vault) {
        (&mut self.connection, &self.vault)
    }

    /// The key. Sealing is the caller's to do — this only hands it over.
    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    /// Where the database file sits, and everything that travels with it. ⚠️ Asked of the
    /// open library rather than of the registry, which is a file read per call.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// ⚠️ Hands the closure the connection **and** the key. `SqliteConnection::transaction`
    /// alone gives back a bare connection, which would leave a caller unable to seal
    /// anything inside the transaction it just opened.
    pub fn transaction<T, F>(&mut self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&mut SqliteConnection, &Vault) -> Result<T, StorageError>,
    {
        // Split borrows: the connection mutably, the key shared, and they are disjoint
        // fields — which is the whole reason this is destructured rather than chained.
        let Self {
            connection, vault, ..
        } = self;
        connection.transaction(|connection| f(connection, vault))
    }
}

impl Deref for Library {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl DerefMut for Library {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

/// ⚠️ Empty until the passphrase has been given. Every command runs behind the front
/// end's unlock gate, so `None` here is a caller that jumped the queue, not a state to
/// render.
pub type Db = Mutex<Option<Library>>;

/// A poisoned mutex means a command panicked while holding it.
pub(crate) fn lock(db: &Db) -> Result<LibraryGuard<'_>, StorageError> {
    let guard = db.lock().map_err(|_| StorageError::Unavailable)?;
    if guard.is_none() {
        return Err(StorageError::Locked);
    }

    Ok(LibraryGuard(guard))
}

/// The guard, narrowed to a library that is definitely there — [`lock`] refused otherwise.
pub(crate) struct LibraryGuard<'a>(MutexGuard<'a, Option<Library>>);

impl Deref for LibraryGuard<'_> {
    type Target = Library;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("a library checked by lock")
    }
}

impl DerefMut for LibraryGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("a library checked by lock")
    }
}

/// ⚠️ `quick_check` and not `integrity_check`: the quick one skips the most expensive
/// cross-checks and reads the file once, which is milliseconds on a library this size.
/// The full check belongs behind a button, never on the path to a window.
///
/// ⚠️ Run before anything writes. A damaged file discovered on the first failing query is
/// discovered too late — by then the launch copies are copies of a broken database, and
/// a backup that propagates the damage on a schedule is worse than none.
pub(crate) fn quick_check(connection: &mut SqliteConnection) -> Result<(), StorageError> {
    #[derive(QueryableByName)]
    struct Answer {
        #[diesel(sql_type = diesel::sql_types::Text)]
        quick_check: String,
    }

    let answers: Vec<Answer> = diesel::sql_query("PRAGMA quick_check")
        .load(connection)
        .map_err(|error| StorageError::Damaged(error.to_string()))?;

    // SQLite answers a single "ok", or one row per problem it found.
    if answers.len() == 1 && answers[0].quick_check == "ok" {
        return Ok(());
    }

    Err(StorageError::Damaged(
        answers
            .iter()
            .map(|answer| answer.quick_check.as_str())
            .collect::<Vec<_>>()
            .join("; "),
    ))
}

/// ⚠️ Every failure here names the file. This is the one error a user meets before any
/// window has anything in it, and "migration failed" without a path leaves them nowhere
/// to look — the database is in a directory they have never opened.
pub fn open(path: &Path, vault: Vault) -> Result<Library, StorageError> {
    let named = |error: &dyn std::fmt::Display| {
        StorageError::Migration(format!("{}: {error}", path.display()))
    };

    let mut connection =
        SqliteConnection::establish(&path.to_string_lossy()).map_err(|error| named(&error))?;
    configure(&mut connection).map_err(|error| named(&error))?;

    // ⚠️ Before the migrations, which write: running them over a damaged file is how a
    // salvageable database becomes an unsalvageable one.
    if let Err(error) = quick_check(&mut connection) {
        return Err(StorageError::Damaged(format!(
            "{}: {error}",
            path.display()
        )));
    }

    migration::run(&mut connection).map_err(|error| named(&error))?;

    Ok(Library {
        connection,
        vault,
        directory: path.parent().map(Path::to_path_buf).unwrap_or_default(),
    })
}

/// Public for the integration tests, which see nothing of the crate but its API.
pub fn open_in_memory() -> Result<Library, StorageError> {
    let mut connection = SqliteConnection::establish(":memory:")
        .map_err(|error| StorageError::Migration(error.to_string()))?;
    configure(&mut connection)?;
    migration::run(&mut connection)?;

    Ok(Library {
        connection,
        vault: test_vault()?,
        // ⚠️ Named, never created: a test that writes beside the library fails on a
        // missing directory rather than writing into whatever the working directory is.
        directory: std::env::temp_dir()
            .join(format!("devnotes-in-memory-{}", uuid::Uuid::new_v4())),
    })
}

/// The same key the in-memory libraries use, for the benchmarks, which open a file.
pub fn bench_vault() -> Result<Vault, StorageError> {
    test_vault()
}

/// ⚠️ A key of its own per in-memory library, derived at a cost nobody would ship. These
/// libraries exist for the length of a test and never reach a file, so what matters is
/// that the sealing path is the real one — not that the key is expensive to guess.
pub fn test_vault() -> Result<Vault, StorageError> {
    use crate::vault::key::Cost;

    Vault::derive(
        "in-memory",
        b"0123456789abcdef",
        Cost {
            memory_kib: 64,
            passes: 1,
            lanes: 1,
        },
    )
}

fn configure(connection: &mut SqliteConnection) -> Result<(), StorageError> {
    // ⚠️ `foreign_keys` is set per connection and is off by default: without it the
    // `ON DELETE CASCADE` clauses are inert. `busy_timeout` covers the window where a
    // second process still holds the file, where the default of zero surfaces
    // `SQLITE_BUSY` as a storage error on the very first write.
    //
    // ⚠️ `synchronous = NORMAL` is WAL's own default, written down because it is a
    // durability choice and not a detail: a power cut can cost the last committed
    // transaction, and cannot corrupt the file. `FULL` would fsync every commit for a
    // note the user can retype, on an application whose whole corpus is local.
    connection.batch_execute(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;",
    )?;

    Ok(())
}

/// ⚠️ Milliseconds are always written, even when zero. `created_at` and `updated_at` are
/// TEXT columns sorted lexicographically, and the canvas orders on them: `.` (0x2E)
/// precedes `Z` (0x5A), so `09:00:00.500Z` would sort before `09:00:00Z` — exactly what
/// chrono's default `SecondsFormat::AutoSi` produces.
pub mod iso8601 {
    use chrono::{DateTime, SecondsFormat, Utc};

    pub fn format(instant: DateTime<Utc>) -> String {
        instant.to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    pub fn parse(value: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
        DateTime::parse_from_rfc3339(value).map(|instant| instant.with_timezone(&Utc))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn milliseconds_are_written_even_when_they_are_zero() {
            let instant = parse("2026-07-25T09:00:00Z").unwrap();

            assert_eq!(format(instant), "2026-07-25T09:00:00.000Z");
        }

        #[test]
        fn the_written_form_sorts_the_way_the_column_does() {
            let plain = format(parse("2026-07-25T09:00:00Z").unwrap());
            let with_millis = format(parse("2026-07-25T09:00:00.500Z").unwrap());

            assert!(plain < with_millis);
        }

        #[test]
        fn an_offset_instant_is_normalized_to_utc() {
            let instant = parse("2026-07-25T11:00:00+02:00").unwrap();

            assert_eq!(format(instant), "2026-07-25T09:00:00.000Z");
        }

        #[test]
        fn a_round_trip_keeps_the_instant() {
            let written = "2026-07-25T09:12:34.567Z";

            assert_eq!(format(parse(written).unwrap()), written);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{AppError, ErrorCode};
    use diesel::sql_types::Integer;

    fn in_memory() -> Db {
        Mutex::new(Some(open_in_memory().unwrap()))
    }

    /// Stays here rather than in `tests/`: `configure` is private.
    #[test]
    fn foreign_keys_are_enforced() {
        #[derive(QueryableByName)]
        struct ForeignKeys {
            #[diesel(sql_type = Integer)]
            foreign_keys: i32,
        }

        let mut connection = open_in_memory().unwrap();

        let enabled = diesel::sql_query("PRAGMA foreign_keys")
            .get_result::<ForeignKeys>(connection.db())
            .unwrap()
            .foreign_keys;

        assert_eq!(enabled, 1);
    }

    /// ⚠️ The point of checking at all: SQLite discovers this on the first query that
    /// happens to read the broken page, which can be days later — and by then every
    /// launch copy is a copy of a broken file.
    #[test]
    fn a_damaged_file_is_named_as_such_rather_than_opened() {
        let directory =
            std::env::temp_dir().join(format!("devnotes-damaged-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(crate::layout::DATABASE);

        // A real library, with enough in it to fill more than the first page.
        {
            let mut library = open(&path, test_vault().unwrap()).unwrap();
            for at in 0..40 {
                crate::spaces::store::create(&mut library, &format!("Space {at}")).unwrap();
            }
        }

        // Garbage over a page that is not the header: the header alone would fail to
        // open rather than fail to check, which is a different story.
        let mut bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 4096, "the corpus did not reach a second page");
        for byte in bytes.iter_mut().skip(4096).take(512) {
            *byte = 0x42;
        }
        std::fs::write(&path, &bytes).unwrap();

        let Err(error) = open(&path, test_vault().unwrap()) else {
            panic!("a damaged database opened as if it were sound");
        };

        assert!(matches!(error, StorageError::Damaged(_)), "{error}");
        assert!(
            format!("{error}").contains(&path.display().to_string()),
            "the failure does not name the file: {error}"
        );

        std::fs::remove_dir_all(&directory).ok();
    }

    /// A sound library answers the check and says nothing about it.
    #[test]
    fn a_sound_library_passes_the_check_it_runs_at_every_open() {
        let mut library = open_in_memory().unwrap();

        assert!(quick_check(library.db()).is_ok());
    }

    /// ⚠️ The one storage error a user can meet with an empty window: a message without
    /// the path leaves them nowhere to look, since the database sits in a directory they
    /// have never opened.
    #[test]
    fn a_database_that_will_not_open_says_which_file() {
        let directory =
            std::env::temp_dir().join(format!("devnotes-unopenable-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();

        // A directory is not a database file, so establishing it fails the way a corrupt
        // file or a bad permission would. ⚠️ Destructured rather than `unwrap_err`, which
        // would want `Library: Debug` — and a library carries the key.
        let Err(error) = open(&directory, test_vault().unwrap()) else {
            panic!("a directory opened as a database");
        };

        assert!(
            format!("{error}").contains(&directory.display().to_string()),
            "the failure does not name the file: {error}"
        );

        std::fs::remove_dir_all(&directory).ok();
    }

    /// ⚠️ Zero is SQLite's own default, and it turns a database another process holds for
    /// twenty milliseconds — a checkpoint, an antivirus, a second instance — into a
    /// storage error on the first write.
    #[test]
    fn a_locked_database_is_waited_on_rather_than_reported() {
        #[derive(QueryableByName)]
        struct BusyTimeout {
            #[diesel(sql_type = Integer)]
            timeout: i32,
        }

        let mut connection = open_in_memory().unwrap();

        let timeout = diesel::sql_query("PRAGMA busy_timeout")
            .get_result::<BusyTimeout>(connection.db())
            .unwrap()
            .timeout;

        assert_eq!(timeout, 5000);
    }

    /// The durability trade-off is chosen, not inherited: `NORMAL` can lose the last
    /// commit to a power cut and cannot corrupt the file.
    #[test]
    fn commits_are_not_fsynced_one_by_one() {
        #[derive(QueryableByName)]
        struct Synchronous {
            #[diesel(sql_type = Integer)]
            synchronous: i32,
        }

        let mut connection = open_in_memory().unwrap();

        let level = diesel::sql_query("PRAGMA synchronous")
            .get_result::<Synchronous>(connection.db())
            .unwrap()
            .synchronous;

        assert_eq!(level, 1);
    }

    #[test]
    fn a_healthy_connection_is_handed_over() {
        let db = in_memory();

        assert!(lock(&db).is_ok());
    }

    /// ⚠️ A command that reached the library before the passphrase did. The front gates
    /// on the unlock screen, so this only catches a caller that jumped the queue — but it
    /// answers rather than unwrapping a `None`.
    #[test]
    fn a_library_still_locked_is_reported_rather_than_unwrapped() {
        let db: Db = Mutex::new(None);

        let Err(error) = lock(&db) else {
            panic!("a locked library must be reported, not handed over");
        };

        assert!(matches!(error, StorageError::Locked));
    }

    #[test]
    fn a_poisoned_connection_is_reported_instead_of_panicking_again() {
        let db = in_memory();

        // Poison it the way production would: a panic while the guard is held.
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = db.lock().unwrap();
            panic!("a command panicked while holding the connection");
        }));
        std::panic::set_hook(hook);

        // `unwrap_err()` would need the guard to be `Debug`, which `SqliteConnection`
        // is not.
        let Err(error) = lock(&db) else {
            panic!("a poisoned mutex must be reported, not returned");
        };

        assert!(matches!(error, StorageError::Unavailable));
        assert!(matches!(
            AppError::from(error).code,
            ErrorCode::StorageUnavailable
        ));
    }
}
