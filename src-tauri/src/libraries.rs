//! The list of libraries, and which one is open.
//!
//! ⚠️ There used to be exactly one, and its address was compiled in: `app_data_dir()` for
//! the directory, `devnotes.sqlite3` for the database, `attachments/` and `backups/`
//! beside it. Four names that are the **address of a library** rather than decoration —
//! renaming any of them sends an installed copy to a virgin profile with the notes still
//! on disk and no way in.
//!
//! ⚠️ So the library that was already there is **moved**, once, by [`adopt`] — database,
//! key file, attachments and every directory beside them — into `libraries/<id>/` with
//! the rest. Every library is then the same shape, which is what makes the first one
//! deletable like any other and keeps the profile root down to the registry and the
//! application's own preferences.
//!
//! ⚠️ The registry itself lives beside the libraries and **no library owns it**. A
//! library that is deleted, moved by hand or sealed under a forgotten passphrase takes
//! nothing else with it — which is exactly the situation #252 is about, seen from the
//! other end.

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, State};

use crate::db::{DB_FILE_NAME, Db};
use crate::error::{AppError, StorageError};
use crate::vault::file::FILE_NAME as VAULT_FILE_NAME;

/// The registry, beside the libraries rather than inside one of them.
const REGISTRY: &str = "libraries.json";

/// Where a library created after the first one goes.
const LIBRARIES: &str = "libraries";

/// A library's own preferences, beside its database. ⚠️ The same name the application's
/// file has, and that is fine: they are told apart by their directory, which only works
/// because no library lives at the profile root any more.
pub(crate) const LIBRARY_PREFERENCES: &str = "preferences.json";

/// What a library is made of, for the move that gathers an old one up.
const LIBRARY_FILES: [&str; 4] = [
    DB_FILE_NAME,
    "devnotes.sqlite3-wal",
    "devnotes.sqlite3-shm",
    VAULT_FILE_NAME,
];
const LIBRARY_DIRECTORIES: [&str; 5] =
    ["attachments", "backups", "damaged", "archived", "replaced"];

/// One library, as the interface lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub id: String,
    pub name: String,
    /// ⚠️ Relative to the profile, so a profile copied to another machine still resolves.
    pub directory: String,
    pub created_at: DateTime<Utc>,
}

/// What the File menu draws: the libraries, and which of them is open.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub libraries: Vec<LibraryEntry>,
    pub open: Option<String>,
}

impl Registry {
    fn entry(&self, id: &str) -> Option<&LibraryEntry> {
        self.libraries.iter().find(|entry| entry.id == id)
    }

    /// ⚠️ Always one, and never `None` once anything exists: the gate asks for *a*
    /// library's passphrase, so a registry with nothing open would need a third screen
    /// nobody asked for.
    fn opened(&self) -> Option<&LibraryEntry> {
        self.open
            .as_deref()
            .and_then(|id| self.entry(id))
            .or_else(|| self.libraries.first())
    }
}

fn profile(app: &AppHandle) -> Result<PathBuf, StorageError> {
    app.path()
        .app_data_dir()
        .map_err(|error| StorageError::File(format!("app_data_dir: {error}")))
}

fn directory_of(profile: &Path, entry: &LibraryEntry) -> PathBuf {
    profile.join(&entry.directory)
}

/// Whether a library has ever been written to this directory.
fn holds_a_library(directory: &Path) -> bool {
    directory.join(DB_FILE_NAME).is_file() || directory.join(VAULT_FILE_NAME).is_file()
}

/// Moves a library that predates the registry into a directory of its own.
///
/// ⚠️ Best effort, file by file, and never fatal. A profile where the move half finished
/// is still a profile whose registry names where the library went — and the half that
/// stayed behind is what the *next* launch picks up, because the same move runs again
/// over whatever is left.
///
/// ⚠️ Everything beside the database travels: the key file (without it the copy opens for
/// nobody), the attachments (a fresh library calls every one of them an orphan and the
/// startup sweep deletes them), and the four directories a library accumulates.
fn gather(profile: &Path, into: &Path) {
    for name in LIBRARY_FILES {
        let _ = std::fs::rename(profile.join(name), into.join(name));
    }

    for name in LIBRARY_DIRECTORIES {
        let from = profile.join(name);
        if from.is_dir() {
            let _ = std::fs::rename(&from, into.join(name));
        }
    }
}

fn read(profile: &Path) -> Registry {
    std::fs::read_to_string(profile.join(REGISTRY))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// ⚠️ Staged and renamed. `fs::write` truncates first, so a disk that fills mid-write
/// leaves a registry naming no libraries at all — with every one of them still on disk
/// and nothing pointing at them.
fn write(profile: &Path, registry: &Registry) -> Result<(), StorageError> {
    let target = profile.join(REGISTRY);
    let staged = profile.join(format!("{REGISTRY}.writing"));
    let raw = serde_json::to_string_pretty(registry)
        .map_err(|error| StorageError::File(format!("{REGISTRY}: {error}")))?;

    std::fs::write(&staged, raw)
        .map_err(|error| StorageError::File(format!("{}: {error}", staged.display())))?;
    std::fs::rename(&staged, &target)
        .map_err(|error| StorageError::File(format!("{}: {error}", target.display())))?;

    Ok(())
}

/// The registry as it stands, adopting what is already on disk the first time.
///
/// ⚠️ The adoption is the migration, and it is why the first library never moves: an
/// installed copy finds its notes exactly where it left them, and gains a registry naming
/// the directory they are already in.
pub(crate) fn registry(app: &AppHandle) -> Result<Registry, StorageError> {
    let profile = profile(app)?;
    let mut registry = read(&profile);

    if registry.libraries.is_empty() {
        let id = uuid::Uuid::new_v4().to_string();
        let entry = LibraryEntry {
            directory: format!("{LIBRARIES}/{id}"),
            id,
            name: String::new(),
            created_at: Utc::now(),
        };

        let directory = directory_of(&profile, &entry);
        if std::fs::create_dir_all(&directory).is_ok() {
            // ⚠️ Only when something is there to gather: a virgin profile has nothing to
            // move, and the first launch creates its library in the new place directly.
            if holds_a_library(&profile) {
                gather(&profile, &directory);
            }

            registry.open = Some(entry.id.clone());
            registry.libraries.push(entry);
            let _ = write(&profile, &registry);
        }
    }

    if registry.open.is_none() {
        registry.open = registry.libraries.first().map(|entry| entry.id.clone());
    }

    Ok(registry)
}

/// The directory every other module reads and writes in.
///
/// ⚠️ This replaced `app.path().app_data_dir()` at a dozen call sites. A module that
/// reaches for the profile directly writes into whichever library happens to be first,
/// whatever is open.
pub(crate) fn open_directory(app: &AppHandle) -> Result<PathBuf, StorageError> {
    let profile = profile(app)?;
    let registry = registry(app)?;
    let Some(entry) = registry.opened() else {
        return Ok(profile);
    };

    let directory = directory_of(&profile, entry);
    std::fs::create_dir_all(&directory)
        .map_err(|error| StorageError::File(format!("{}: {error}", directory.display())))?;

    Ok(directory)
}

/// The libraries, and which one is open.
#[tauri::command(async)]
#[specta::specta]
pub fn list_libraries(app: AppHandle) -> Result<Registry, AppError> {
    Ok(registry(&app)?)
}

/// Adds one, and leaves it closed: opening it is a second, deliberate gesture.
///
/// ⚠️ Nothing is created on disk here beyond the directory. A library is born when its
/// passphrase is chosen — `create_vault` writes the key file and the database — which is
/// the same path a first launch takes, and the only one that has ever been exercised.
#[tauri::command(async)]
#[specta::specta]
pub fn create_library(name: String, app: AppHandle) -> Result<LibraryEntry, AppError> {
    let profile = profile(&app)?;
    let mut registry = registry(&app)?;

    let id = uuid::Uuid::new_v4().to_string();
    let entry = LibraryEntry {
        directory: format!("{LIBRARIES}/{id}"),
        id,
        name: name.trim().to_string(),
        created_at: Utc::now(),
    };

    let directory = directory_of(&profile, &entry);
    std::fs::create_dir_all(&directory)
        .map_err(|error| StorageError::File(format!("{}: {error}", directory.display())))?;

    registry.libraries.push(entry.clone());
    write(&profile, &registry)?;

    Ok(entry)
}

/// Closes whatever is open and points the registry at another one.
///
/// ⚠️ The connection `Mutex` is emptied under the same lock every other command takes:
/// every one of them then answers `Locked`, which is what sends the interface back to the
/// gate. The new library has its own passphrase, and asking for it is the only proof the
/// right one is open.
#[tauri::command(async)]
#[specta::specta]
pub fn open_library(id: String, app: AppHandle, db: State<'_, Db>) -> Result<(), AppError> {
    let profile = profile(&app)?;
    let mut registry = registry(&app)?;
    if registry.entry(&id).is_none() {
        return Err(StorageError::File(format!("{id}: no such library")).into());
    }

    let mut open = db.lock().map_err(|_| StorageError::Unavailable)?;
    *open = None;

    registry.open = Some(id);
    write(&profile, &registry)?;

    Ok(())
}

#[tauri::command(async)]
#[specta::specta]
pub fn rename_library(id: String, name: String, app: AppHandle) -> Result<(), AppError> {
    let profile = profile(&app)?;
    let mut registry = registry(&app)?;
    let Some(entry) = registry.libraries.iter_mut().find(|entry| entry.id == id) else {
        return Err(StorageError::File(format!("{id}: no such library")).into());
    };

    entry.name = name.trim().to_string();
    write(&profile, &registry)?;

    Ok(())
}

/// Erases a library and everything in it.
///
/// ⚠️ Irreversible, and the only thing in the application that erases a corpus outright —
/// the interface gives it the treatment emptying the trash gets: a sentence naming what
/// goes, and a confirm somewhere other than the button that fired it.
///
/// ⚠️ The **adopted** library cannot be deleted. Its directory is the profile itself, so
/// erasing it would take the registry, the application's preferences and every other
/// library with it. Refused here rather than hidden in the interface, because a command
/// is reachable from more than the interface.
///
/// ⚠️ The open one cannot be deleted either: the front end switches first, which is what
/// closes the connection. Deleting the files under a live one is how a library that was
/// merely unwanted takes the process down with it.
#[tauri::command(async)]
#[specta::specta]
pub fn delete_library(id: String, app: AppHandle, db: State<'_, Db>) -> Result<(), AppError> {
    let profile = profile(&app)?;
    let mut registry = registry(&app)?;

    let Some(entry) = registry.entry(&id).cloned() else {
        return Err(StorageError::File(format!("{id}: no such library")).into());
    };
    // ⚠️ Never the last one. A profile with no library at all would have the gate
    // offering nothing — and the registry would adopt the empty root on the next read,
    // which is a library nobody asked for rather than an error anybody can act on.
    if registry.libraries.len() <= 1 {
        return Err(StorageError::File("the last library cannot be deleted".to_string()).into());
    }
    if registry.open.as_deref() == Some(id.as_str())
        && db.lock().map_err(|_| StorageError::Unavailable)?.is_some()
    {
        return Err(StorageError::File("the library is open".to_string()).into());
    }

    // ⚠️ The registry first: a directory that resists deletion must not stay listed and
    // openable, where a listing that lost an entry leaves files nobody points at — which
    // is the same thing as a library moved by hand, and harmless.
    registry.libraries.retain(|each| each.id != id);
    if registry.open.as_deref() == Some(id.as_str()) {
        registry.open = registry.libraries.first().map(|first| first.id.clone());
    }
    write(&profile, &registry)?;

    std::fs::remove_dir_all(directory_of(&profile, &entry))
        .map_err(|error| StorageError::File(format!("{id}: {error}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("devnotes-libraries-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    fn entry(id: &str, directory: &str) -> LibraryEntry {
        LibraryEntry {
            id: id.to_string(),
            name: String::new(),
            directory: directory.to_string(),
            created_at: Utc::now(),
        }
    }

    /// ⚠️ Every library is the same shape now, which is what makes the first one
    /// deletable like any other.
    #[test]
    fn a_library_sits_under_a_folder_of_its_own() {
        let profile = scratch();

        assert_eq!(
            directory_of(&profile, &entry("b", "libraries/b")),
            profile.join("libraries").join("b")
        );
        std::fs::remove_dir_all(&profile).ok();
    }

    /// ⚠️ Without the key file the gathered library is one nobody can ever open again,
    /// and without the attachments the next launch's orphan sweep deletes the pictures
    /// of notes that are merely somewhere else.
    #[test]
    fn gathering_takes_the_key_and_the_attachments_with_the_database() {
        let profile = scratch();
        std::fs::write(profile.join(DB_FILE_NAME), b"a database").unwrap();
        std::fs::write(profile.join(VAULT_FILE_NAME), b"{}").unwrap();
        std::fs::create_dir_all(profile.join("attachments")).unwrap();
        std::fs::write(profile.join("attachments").join("a-1.png"), b"\x89PNG").unwrap();
        std::fs::create_dir_all(profile.join("backups")).unwrap();
        let into = profile.join("libraries").join("x");
        std::fs::create_dir_all(&into).unwrap();

        gather(&profile, &into);

        assert!(into.join(DB_FILE_NAME).is_file());
        assert!(into.join(VAULT_FILE_NAME).is_file());
        assert!(into.join("attachments").join("a-1.png").is_file());
        assert!(into.join("backups").is_dir());
        assert!(!profile.join(DB_FILE_NAME).exists());
        assert!(!profile.join("attachments").exists());
        std::fs::remove_dir_all(&profile).ok();
    }

    /// ⚠️ The application's own preferences stay at the root: they follow the person, not
    /// the corpus, and `backup::wanted` reads one of their keys from there.
    #[test]
    fn gathering_leaves_the_application_preferences_where_they_are() {
        let profile = scratch();
        std::fs::write(profile.join(DB_FILE_NAME), b"a database").unwrap();
        std::fs::write(profile.join("preferences.json"), b"{}").unwrap();
        let into = profile.join("libraries").join("x");
        std::fs::create_dir_all(&into).unwrap();

        gather(&profile, &into);

        assert!(profile.join("preferences.json").is_file());
        assert!(!into.join("preferences.json").exists());
        std::fs::remove_dir_all(&profile).ok();
    }

    /// Nothing there is nothing to move, and must not leave a half-made library behind.
    #[test]
    fn a_profile_with_no_library_in_it_holds_none() {
        let profile = scratch();
        assert!(!holds_a_library(&profile));

        std::fs::write(profile.join(VAULT_FILE_NAME), b"{}").unwrap();

        assert!(holds_a_library(&profile));
        std::fs::remove_dir_all(&profile).ok();
    }

    /// ⚠️ `fs::write` truncates first: a disk that fills mid-write would otherwise leave a
    /// registry naming no libraries at all, with every one of them still on disk.
    #[test]
    fn the_registry_is_staged_and_renamed_rather_than_truncated() {
        let profile = scratch();
        let registry = Registry {
            libraries: vec![entry("a", "")],
            open: Some("a".to_string()),
        };

        write(&profile, &registry).unwrap();

        assert!(profile.join(REGISTRY).is_file());
        assert!(!profile.join(format!("{REGISTRY}.writing")).exists());
        assert_eq!(read(&profile).libraries, registry.libraries);
        std::fs::remove_dir_all(&profile).ok();
    }

    #[test]
    fn a_registry_that_cannot_be_read_answers_an_empty_one_rather_than_failing() {
        let profile = scratch();
        std::fs::write(profile.join(REGISTRY), b"not json at all").unwrap();

        assert!(read(&profile).libraries.is_empty());
        std::fs::remove_dir_all(&profile).ok();
    }

    /// ⚠️ A gate that asks for *a* library's passphrase needs one to be current; a
    /// registry with nothing open would want a third screen nobody asked for.
    #[test]
    fn the_first_one_stands_in_when_nothing_is_marked_open() {
        let registry = Registry {
            libraries: vec![entry("a", ""), entry("b", "libraries/b")],
            open: None,
        };

        assert_eq!(registry.opened().map(|entry| entry.id.as_str()), Some("a"));
    }

    #[test]
    fn an_open_id_naming_nothing_falls_back_rather_than_answering_none() {
        let registry = Registry {
            libraries: vec![entry("a", "")],
            open: Some("gone".to_string()),
        };

        assert_eq!(registry.opened().map(|entry| entry.id.as_str()), Some("a"));
    }
}
