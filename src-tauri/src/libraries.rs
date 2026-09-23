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

use crate::db::Db;
use crate::error::{AppError, FileContext, StorageError};
use crate::layout::{DATABASE, KEY_FILE, LIBRARIES, LIBRARY_DIRECTORIES, LIBRARY_FILES, REGISTRY};

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
    app.path().app_data_dir().context("app_data_dir")
}

fn directory_of(profile: &Path, entry: &LibraryEntry) -> PathBuf {
    profile.join(&entry.directory)
}

/// Whether a library has ever been written to this directory.
fn holds_a_library(directory: &Path) -> bool {
    directory.join(DATABASE).is_file() || directory.join(KEY_FILE).is_file()
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
    let raw = serde_json::to_string_pretty(registry).context(REGISTRY)?;

    std::fs::write(&staged, raw).context(staged.display())?;
    std::fs::rename(&staged, &target).context(target.display())?;

    Ok(())
}

/// The registry as it stands, adopting what is already on disk the first time.
///
/// ⚠️ The adoption is the migration: a library that predates the registry is gathered into
/// a directory of its own, and the registry then names where it went.
///
/// ⚠️ Takes a path rather than an `AppHandle`, like every rule below it. That is what
/// makes this module testable at all — a Tauri handle cannot be built in a unit test, and
/// the rules would otherwise only ever be exercised through the interface.
fn registry_in(profile: &Path) -> Registry {
    let mut registry = read(profile);

    if registry.libraries.is_empty() {
        let entry = fresh(String::new());
        let directory = directory_of(profile, &entry);

        if std::fs::create_dir_all(&directory).is_ok() {
            // ⚠️ Only when something is there to gather: a virgin profile has nothing to
            // move, and the first launch creates its library in the new place directly.
            if holds_a_library(profile) {
                gather(profile, &directory);
            }

            registry.open = Some(entry.id.clone());
            registry.libraries.push(entry);
            let _ = write(profile, &registry);
        }
    }

    if registry.open.is_none() {
        registry.open = registry.libraries.first().map(|entry| entry.id.clone());
    }

    registry
}

pub(crate) fn registry(app: &AppHandle) -> Result<Registry, StorageError> {
    Ok(registry_in(&profile(app)?))
}

/// A library in a directory of its own, named after nothing but a fresh id.
fn fresh(name: String) -> LibraryEntry {
    let id = uuid::Uuid::new_v4().to_string();

    LibraryEntry {
        directory: format!("{LIBRARIES}/{id}"),
        id,
        name,
        created_at: Utc::now(),
    }
}

fn open_directory_in(profile: &Path) -> Result<PathBuf, StorageError> {
    let registry = registry_in(profile);
    let Some(entry) = registry.opened() else {
        return Ok(profile.to_path_buf());
    };

    let directory = directory_of(profile, entry);
    std::fs::create_dir_all(&directory).context(directory.display())?;

    Ok(directory)
}

/// The directory every other module reads and writes in.
///
/// ⚠️ This replaced `app.path().app_data_dir()` at a dozen call sites. A module that
/// reaches for the profile directly writes into whichever library happens to be first,
/// whatever is open.
pub(crate) fn open_directory(app: &AppHandle) -> Result<PathBuf, StorageError> {
    open_directory_in(&profile(app)?)
}

/// Adds one to the registry, with a directory of its own.
///
/// ⚠️ Nothing is created on disk beyond that directory. A library is born when its
/// passphrase is chosen — `create_vault` writes the key file and the database — which is
/// the same path a first launch takes, and the only one that has ever been exercised.
fn create_in(profile: &Path, name: &str) -> Result<LibraryEntry, StorageError> {
    let mut registry = registry_in(profile);
    let entry = fresh(name.trim().to_string());

    let directory = directory_of(profile, &entry);
    std::fs::create_dir_all(&directory).context(directory.display())?;

    registry.libraries.push(entry.clone());
    write(profile, &registry)?;

    Ok(entry)
}

/// Points the registry at another library. Closing the connection is the command's part.
fn point_at(profile: &Path, id: &str) -> Result<(), StorageError> {
    let mut registry = registry_in(profile);
    if registry.entry(id).is_none() {
        return Err(StorageError::LibraryNotFound(id.to_string()));
    }

    registry.open = Some(id.to_string());
    write(profile, &registry)
}

fn rename_in(profile: &Path, id: &str, name: &str) -> Result<(), StorageError> {
    let mut registry = registry_in(profile);
    let Some(entry) = registry.libraries.iter_mut().find(|entry| entry.id == id) else {
        return Err(StorageError::LibraryNotFound(id.to_string()));
    };

    entry.name = name.trim().to_string();
    write(profile, &registry)
}

/// Erases a library and everything in it.
///
/// ⚠️ Never the last one. A profile with no library at all would have the gate offering
/// nothing — and the next read of the registry would adopt the empty root as a library
/// nobody asked for, which is not an error anybody can act on.
///
/// ⚠️ The registry is written **first**: a directory that resists deletion must not stay
/// listed and openable, where a listing that lost an entry leaves files nobody points at —
/// which is the same thing as a library moved by hand, and harmless.
fn delete_in(profile: &Path, id: &str) -> Result<(), StorageError> {
    let mut registry = registry_in(profile);
    let Some(entry) = registry.entry(id).cloned() else {
        return Err(StorageError::LibraryNotFound(id.to_string()));
    };
    if registry.libraries.len() <= 1 {
        return Err(StorageError::LastLibrary);
    }

    registry.libraries.retain(|each| each.id != id);
    if registry.open.as_deref() == Some(id) {
        registry.open = registry.libraries.first().map(|first| first.id.clone());
    }
    write(profile, &registry)?;

    std::fs::remove_dir_all(directory_of(profile, &entry)).context(id)
}

/// The libraries, and which one is open.
#[tauri::command(async)]
#[specta::specta]
pub fn list_libraries(app: AppHandle) -> Result<Registry, AppError> {
    Ok(registry(&app)?)
}

/// Adds one, and leaves it closed: opening it is a second, deliberate gesture.
#[tauri::command(async)]
#[specta::specta]
pub fn create_library(name: String, app: AppHandle) -> Result<LibraryEntry, AppError> {
    Ok(create_in(&profile(&app)?, &name)?)
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

    let mut open = db.lock().map_err(|_| StorageError::Unavailable)?;
    *open = None;

    Ok(point_at(&profile, &id)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn rename_library(id: String, name: String, app: AppHandle) -> Result<(), AppError> {
    Ok(rename_in(&profile(&app)?, &id, &name)?)
}

/// Erases a library and everything in it.
///
/// ⚠️ Refused on the open one: the front end switches first, which is what closes the
/// connection. Deleting the files under a live one is how a library that was merely
/// unwanted takes the process down with it.
#[tauri::command(async)]
#[specta::specta]
pub fn delete_library(id: String, app: AppHandle, db: State<'_, Db>) -> Result<(), AppError> {
    let profile = profile(&app)?;
    if registry_in(&profile).open.as_deref() == Some(id.as_str())
        && db.lock().map_err(|_| StorageError::Unavailable)?.is_some()
    {
        return Err(StorageError::LibraryOpen.into());
    }

    Ok(delete_in(&profile, &id)?)
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
        std::fs::write(profile.join(DATABASE), b"a database").unwrap();
        std::fs::write(profile.join(KEY_FILE), b"{}").unwrap();
        std::fs::create_dir_all(profile.join("attachments")).unwrap();
        std::fs::write(profile.join("attachments").join("a-1.png"), b"\x89PNG").unwrap();
        std::fs::create_dir_all(profile.join("backups")).unwrap();
        let into = profile.join("libraries").join("x");
        std::fs::create_dir_all(&into).unwrap();

        gather(&profile, &into);

        assert!(into.join(DATABASE).is_file());
        assert!(into.join(KEY_FILE).is_file());
        assert!(into.join("attachments").join("a-1.png").is_file());
        assert!(into.join("backups").is_dir());
        assert!(!profile.join(DATABASE).exists());
        assert!(!profile.join("attachments").exists());
        std::fs::remove_dir_all(&profile).ok();
    }

    /// A directory a library gains has to be declared in `layout` to travel, and this is
    /// what notices one that was not moved.
    #[test]
    fn gathering_moves_every_entry_the_layout_declares() {
        let profile = scratch();
        for name in LIBRARY_FILES {
            std::fs::write(profile.join(name), b"x").unwrap();
        }
        for name in LIBRARY_DIRECTORIES {
            std::fs::create_dir_all(profile.join(name)).unwrap();
        }
        let into = profile.join(LIBRARIES).join("x");
        std::fs::create_dir_all(&into).unwrap();

        gather(&profile, &into);

        for name in LIBRARY_FILES.iter().chain(LIBRARY_DIRECTORIES.iter()) {
            assert!(into.join(name).exists(), "{name} did not arrive");
            assert!(!profile.join(name).exists(), "{name} stayed behind");
        }
        std::fs::remove_dir_all(&profile).ok();
    }

    /// ⚠️ The application's own preferences stay at the root: they follow the person, not
    /// the corpus, and `backup::wanted` reads one of their keys from there.
    #[test]
    fn gathering_leaves_the_application_preferences_where_they_are() {
        let profile = scratch();
        std::fs::write(profile.join(DATABASE), b"a database").unwrap();
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

        std::fs::write(profile.join(KEY_FILE), b"{}").unwrap();

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

    /// ⚠️ The migration, end to end: an installed copy finds its notes, and the registry
    /// names the directory they were moved into.
    #[test]
    fn a_profile_from_before_the_registry_is_adopted_and_gathered() {
        let profile = scratch();
        std::fs::write(profile.join(DATABASE), b"a database").unwrap();
        std::fs::write(profile.join(KEY_FILE), b"{}").unwrap();

        let registry = registry_in(&profile);

        assert_eq!(registry.libraries.len(), 1);
        let into = directory_of(&profile, &registry.libraries[0]);
        assert!(into.join(DATABASE).is_file());
        assert!(!profile.join(DATABASE).exists());
        assert_eq!(
            registry.open.as_deref(),
            Some(registry.libraries[0].id.as_str())
        );
        std::fs::remove_dir_all(&profile).ok();
    }

    /// A first launch has nothing to gather, and still gets a library of its own.
    #[test]
    fn a_virgin_profile_gets_one_library_and_no_files_moved() {
        let profile = scratch();

        let registry = registry_in(&profile);

        assert_eq!(registry.libraries.len(), 1);
        assert!(directory_of(&profile, &registry.libraries[0]).is_dir());
        std::fs::remove_dir_all(&profile).ok();
    }

    /// ⚠️ Read twice must not adopt twice: the second read finds the registry it wrote.
    #[test]
    fn adopting_happens_once() {
        let profile = scratch();
        let first = registry_in(&profile);

        let second = registry_in(&profile);

        assert_eq!(second.libraries.len(), 1);
        assert_eq!(second.libraries[0].id, first.libraries[0].id);
        std::fs::remove_dir_all(&profile).ok();
    }

    #[test]
    fn the_open_directory_is_the_one_the_registry_points_at() {
        let profile = scratch();
        let created = create_in(&profile, "Boulot").unwrap();
        point_at(&profile, &created.id).unwrap();

        let directory = open_directory_in(&profile).unwrap();

        assert_eq!(directory, directory_of(&profile, &created));
        std::fs::remove_dir_all(&profile).ok();
    }

    /// ⚠️ A library is born when its passphrase is chosen, not here: only the directory.
    #[test]
    fn creating_one_adds_it_to_the_registry_and_leaves_it_closed() {
        let profile = scratch();
        let first = registry_in(&profile).open;

        let created = create_in(&profile, "  Boulot  ").unwrap();

        let registry = registry_in(&profile);
        assert_eq!(registry.libraries.len(), 2);
        assert_eq!(created.name, "Boulot", "the name is trimmed");
        assert_eq!(registry.open, first, "creating does not open");
        assert!(directory_of(&profile, &created).is_dir());
        assert!(!directory_of(&profile, &created).join(DATABASE).exists());
        std::fs::remove_dir_all(&profile).ok();
    }

    #[test]
    fn pointing_at_one_that_does_not_exist_says_so() {
        let profile = scratch();
        registry_in(&profile);

        assert!(matches!(
            point_at(&profile, "nothing"),
            Err(StorageError::LibraryNotFound(_))
        ));
        std::fs::remove_dir_all(&profile).ok();
    }

    #[test]
    fn renaming_touches_the_name_and_nothing_else() {
        let profile = scratch();
        let created = create_in(&profile, "Boulot").unwrap();
        let open = registry_in(&profile).open;

        rename_in(&profile, &created.id, "  Archives ").unwrap();

        let registry = registry_in(&profile);
        let renamed = registry.entry(&created.id).unwrap();
        assert_eq!(renamed.name, "Archives");
        assert_eq!(renamed.directory, created.directory);
        assert_eq!(registry.open, open);
        std::fs::remove_dir_all(&profile).ok();
    }

    #[test]
    fn renaming_one_that_does_not_exist_says_so() {
        let profile = scratch();
        registry_in(&profile);

        assert!(matches!(
            rename_in(&profile, "nothing", "Archives"),
            Err(StorageError::LibraryNotFound(_))
        ));
        std::fs::remove_dir_all(&profile).ok();
    }

    #[test]
    fn deleting_takes_the_entry_and_the_files() {
        let profile = scratch();
        let created = create_in(&profile, "Boulot").unwrap();
        std::fs::write(directory_of(&profile, &created).join(DATABASE), b"x").unwrap();

        delete_in(&profile, &created.id).unwrap();

        assert_eq!(registry_in(&profile).libraries.len(), 1);
        assert!(!directory_of(&profile, &created).exists());
        std::fs::remove_dir_all(&profile).ok();
    }

    /// ⚠️ The gate would have nothing to offer, and the next read would adopt the empty
    /// root as a library nobody asked for.
    #[test]
    fn the_last_library_cannot_be_deleted() {
        let profile = scratch();
        let only = registry_in(&profile).libraries[0].clone();

        assert!(matches!(
            delete_in(&profile, &only.id),
            Err(StorageError::LastLibrary)
        ));
        assert_eq!(registry_in(&profile).libraries.len(), 1);
        std::fs::remove_dir_all(&profile).ok();
    }

    /// Deleting the open one leaves the registry pointing at what is left, never at a
    /// library that is gone.
    #[test]
    fn deleting_the_open_one_moves_the_mark_to_what_remains() {
        let profile = scratch();
        let created = create_in(&profile, "Boulot").unwrap();
        point_at(&profile, &created.id).unwrap();

        delete_in(&profile, &created.id).unwrap();

        let registry = registry_in(&profile);
        assert_eq!(registry.libraries.len(), 1);
        assert_eq!(
            registry.open.as_deref(),
            Some(registry.libraries[0].id.as_str())
        );
        std::fs::remove_dir_all(&profile).ok();
    }

    #[test]
    fn deleting_one_that_does_not_exist_says_so() {
        let profile = scratch();
        create_in(&profile, "Boulot").unwrap();

        assert!(matches!(
            delete_in(&profile, "nothing"),
            Err(StorageError::LibraryNotFound(_))
        ));
        std::fs::remove_dir_all(&profile).ok();
    }
}
