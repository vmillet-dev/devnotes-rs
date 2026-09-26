pub mod migration;
pub mod schema;

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use diesel::connection::SimpleConnection;
use diesel::prelude::*;

use tauri::{AppHandle, Manager};

use crate::error::{AppError, StorageError};
use crate::vault::key::Vault;

/// The connection, and the key everything it holds is sealed with. The connection is reached
/// through [`Library::db`] alone: Diesel's generic `load` gets no deref coercion anyway.
pub struct Library {
    connection: SqliteConnection,
    /// Shared, so a read can take the key past the lock: see [`Library::shared_vault`].
    vault: Arc<Vault>,
    directory: PathBuf,
}

impl Library {
    /// The connection, for the Diesel call that needs it by that name.
    pub fn db(&mut self) -> &mut SqliteConnection {
        &mut self.connection
    }

    /// Both halves at once: two calls, one mutable and one shared, would not borrow-check.
    pub fn split(&mut self) -> (&mut SqliteConnection, &Vault) {
        (&mut self.connection, &self.vault)
    }

    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    /// The key, for work that can finish once the lock is released: opening a file read off
    /// disk needs the key and nothing else the lock guards.
    pub fn shared_vault(&self) -> Arc<Vault> {
        Arc::clone(&self.vault)
    }

    /// An in-memory library that writes its files into `directory`.
    #[cfg(test)]
    pub(crate) fn with_directory(mut self, directory: PathBuf) -> Self {
        self.directory = directory;
        self
    }

    /// Where the database file sits, with everything that travels with it. Asked of the open
    /// library, where the registry would be a file read per call.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The closure gets the key as well: `SqliteConnection::transaction` alone hands back a
    /// bare connection, with nothing to seal with.
    pub fn transaction<T, F>(&mut self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&mut SqliteConnection, &Vault) -> Result<T, StorageError>,
    {
        // Destructured so the two disjoint fields borrow separately.
        let Self {
            connection, vault, ..
        } = self;
        connection.transaction(|connection| f(connection, vault))
    }
}

/// Empty until the passphrase is given. The front end gates every command on the unlock,
/// so `None` here is a caller that jumped the queue, not a state to render.
pub type Db = Mutex<Option<Library>>;

/// A poisoned mutex means a command panicked while holding it.
pub(crate) fn lock(db: &Db) -> Result<LibraryGuard<'_>, StorageError> {
    let guard = db.lock().map_err(|_| StorageError::Unavailable)?;
    if guard.is_none() {
        return Err(StorageError::Locked);
    }

    Ok(LibraryGuard(guard))
}

/// A command's body, on Tokio's blocking pool rather than on a runtime worker: waiting on the
/// lock, or holding it through a whole-corpus read, would park a worker the updater and the
/// plugins share. A body that panics answers as the lock it would have poisoned.
pub(crate) async fn blocking<T, F>(app: AppHandle, body: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&AppHandle, &Db) -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || body(&app, &app.state::<Db>()))
        .await
        .map_err(|_| StorageError::Unavailable)?
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

/// `quick_check` and not `integrity_check`: milliseconds on a library this size. The full
/// check belongs behind a button, never on the path to a window.
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

/// Every failure names the file: this is the error a user meets before any window has
/// anything in it, and the database sits in a directory they have never opened.
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
        vault: Arc::new(vault),
        directory: path.parent().map(Path::to_path_buf).unwrap_or_default(),
    })
}

/// For the integration tests, which see nothing of the crate but its API.
#[doc(hidden)]
pub fn open_in_memory() -> Result<Library, StorageError> {
    let mut connection = SqliteConnection::establish(":memory:")
        .map_err(|error| StorageError::Migration(error.to_string()))?;
    configure(&mut connection)?;
    migration::run(&mut connection)?;

    Ok(Library {
        connection,
        vault: Arc::new(test_vault()?),
        // Named, never created: a test writing beside the library fails loudly.
        directory: std::env::temp_dir()
            .join(format!("devnotes-in-memory-{}", uuid::Uuid::new_v4())),
    })
}

/// A key derived at a cost nobody would ship: the sealing path is the real one, which is
/// what the tests and the benchmarks need.
#[doc(hidden)]
pub fn test_vault() -> Result<Vault, StorageError> {
    Vault::derive(
        "in-memory",
        b"0123456789abcdef",
        crate::vault::key::Cost::FOR_TESTS,
    )
}

fn configure(connection: &mut SqliteConnection) -> Result<(), StorageError> {
    // ⚠️ `foreign_keys` is per connection and off by default: without it every `ON DELETE
    // CASCADE` is inert. `busy_timeout` rides out another process holding the file.
    // `synchronous = NORMAL` under WAL can lose the last commit to a power cut, never the file.
    connection.batch_execute(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;",
    )?;

    Ok(())
}

/// ⚠️ Milliseconds are always written: the TEXT columns sort lexicographically and `.`
/// precedes `Z`, so chrono's default `09:00:00.500Z` would sort before `09:00:00Z`.
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

    /// SQLite would otherwise meet the damage on whichever query reads the broken page, days
    /// later, with every launch copy a copy of the broken file.
    #[test]
    fn a_damaged_file_is_named_as_such_rather_than_opened() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let path = directory.join(crate::layout::DATABASE);

        // A real library, with enough in it to fill more than the first page.
        {
            let mut library = open(&path, test_vault().unwrap()).unwrap();
            for at in 0..40 {
                crate::spaces::store::create(&mut library, &format!("Space {at}")).unwrap();
            }
        }

        // Garbage past the header: a broken header fails to open rather than to check.
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
    }

    /// A sound library answers the check and says nothing about it.
    #[test]
    fn a_sound_library_passes_the_check_it_runs_at_every_open() {
        let mut library = open_in_memory().unwrap();

        assert!(quick_check(library.db()).is_ok());
    }

    #[test]
    fn a_database_that_will_not_open_says_which_file() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();

        // A directory fails to open the way a corrupt file or a permission would.
        // Destructured: `unwrap_err` would want `Library: Debug`, and a library holds the key.
        let Err(error) = open(&directory, test_vault().unwrap()) else {
            panic!("a directory opened as a database");
        };

        assert!(
            format!("{error}").contains(&directory.display().to_string()),
            "the failure does not name the file: {error}"
        );
    }

    /// SQLite's default of zero turns a file another process holds for a moment into a
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

    /// `NORMAL` can lose the last commit to a power cut, and cannot corrupt the file.
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

    /// A command that reached the library before the passphrase answers rather than unwrapping.
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

        // `unwrap_err()` would want the guard to be `Debug`.
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
