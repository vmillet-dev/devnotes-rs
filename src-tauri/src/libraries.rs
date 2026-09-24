//! The list of libraries, and which one is open.
//!
//! The names in `layout` are the address of a library: renaming one sends an installed copy
//! to an empty profile, with the notes still on disk and no way in. A library from before the
//! registry is moved once by `gather` into `libraries/<id>/`, so every library has the same
//! shape; the registry sits beside them and belongs to none.

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
    /// Relative to the profile, so a profile copied to another machine still resolves.
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

    /// One is always open once anything exists: the gate asks for a library's passphrase,
    /// and a registry with nothing open would need a screen of its own.
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
/// Best effort, file by file, and never fatal: whatever stays behind, the next launch moves.
/// The key file and the attachments travel too — without them the copy opens for nobody, and
/// the startup sweep deletes every picture as an orphan.
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

/// Staged and renamed: `fs::write` truncates first, and a registry naming no libraries leaves
/// every one of them on disk with nothing pointing at them.
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
/// Gathering a library from before the registry is the migration. Takes a path rather than an
/// `AppHandle`, like every rule below: a Tauri handle cannot be built in a unit test.
fn registry_in(profile: &Path) -> Registry {
    let mut registry = read(profile);

    if registry.libraries.is_empty() {
        let entry = fresh(String::new());
        let directory = directory_of(profile, &entry);

        if std::fs::create_dir_all(&directory).is_ok() {
            // A virgin profile has nothing to move: its first library is created in place.
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

/// Public for the benchmark that says what an open library saves by not calling it.
pub fn open_directory_in(profile: &Path) -> Result<PathBuf, StorageError> {
    let registry = registry_in(profile);
    let Some(entry) = registry.opened() else {
        return Ok(profile.to_path_buf());
    };

    let directory = directory_of(profile, entry);
    std::fs::create_dir_all(&directory).context(directory.display())?;

    Ok(directory)
}

/// The directory of the library the registry points at, for what runs while none is open:
/// the gate, the first launch, the recovery commands.
///
/// ⚠️ An open library answers `Library::directory` instead, without reading this file. Never
/// `app_data_dir()` directly: that is the profile, which holds no library.
pub(crate) fn open_directory(app: &AppHandle) -> Result<PathBuf, StorageError> {
    open_directory_in(&profile(app)?)
}

/// Adds one to the registry, with a directory of its own.
///
/// Only the directory: a library is born when its passphrase is chosen, by `create_vault`,
/// the path a first launch takes too.
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
/// Never the last one: the gate would have nothing to offer. The registry is written first,
/// so a directory that resists deletion is left unlisted rather than listed and broken.
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
/// ⚠️ The connection `Mutex` is emptied under the lock every command takes: they all answer
/// `Locked` afterwards, which sends the interface back to the gate for the other passphrase.
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
/// Refused on the open one: deleting files under a live connection takes the process down.
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

    fn entry(id: &str, directory: &str) -> LibraryEntry {
        LibraryEntry {
            id: id.to_string(),
            name: String::new(),
            directory: directory.to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn a_library_sits_under_a_folder_of_its_own() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();

        assert_eq!(
            directory_of(&profile, &entry("b", "libraries/b")),
            profile.join("libraries").join("b")
        );
    }

    /// Without the key file the library opens for nobody, and without the attachments the next
    /// orphan sweep deletes the pictures.
    #[test]
    fn gathering_takes_the_key_and_the_attachments_with_the_database() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
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
    }

    /// A directory a library gains travels only once `layout` declares it.
    #[test]
    fn gathering_moves_every_entry_the_layout_declares() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
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
    }

    /// The application's preferences follow the person, not the corpus.
    #[test]
    fn gathering_leaves_the_application_preferences_where_they_are() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        std::fs::write(profile.join(DATABASE), b"a database").unwrap();
        std::fs::write(profile.join("preferences.json"), b"{}").unwrap();
        let into = profile.join("libraries").join("x");
        std::fs::create_dir_all(&into).unwrap();

        gather(&profile, &into);

        assert!(profile.join("preferences.json").is_file());
        assert!(!into.join("preferences.json").exists());
    }

    #[test]
    fn a_profile_with_no_library_in_it_holds_none() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        assert!(!holds_a_library(&profile));

        std::fs::write(profile.join(KEY_FILE), b"{}").unwrap();

        assert!(holds_a_library(&profile));
    }

    #[test]
    fn the_registry_is_staged_and_renamed_rather_than_truncated() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        let registry = Registry {
            libraries: vec![entry("a", "")],
            open: Some("a".to_string()),
        };

        write(&profile, &registry).unwrap();

        assert!(profile.join(REGISTRY).is_file());
        assert!(!profile.join(format!("{REGISTRY}.writing")).exists());
        assert_eq!(read(&profile).libraries, registry.libraries);
    }

    #[test]
    fn a_registry_that_cannot_be_read_answers_an_empty_one_rather_than_failing() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        std::fs::write(profile.join(REGISTRY), b"not json at all").unwrap();

        assert!(read(&profile).libraries.is_empty());
    }

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

    /// The migration, end to end.
    #[test]
    fn a_profile_from_before_the_registry_is_adopted_and_gathered() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
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
    }

    /// A first launch has nothing to gather, and still gets a library of its own.
    #[test]
    fn a_virgin_profile_gets_one_library_and_no_files_moved() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();

        let registry = registry_in(&profile);

        assert_eq!(registry.libraries.len(), 1);
        assert!(directory_of(&profile, &registry.libraries[0]).is_dir());
    }

    #[test]
    fn adopting_happens_once() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        let first = registry_in(&profile);

        let second = registry_in(&profile);

        assert_eq!(second.libraries.len(), 1);
        assert_eq!(second.libraries[0].id, first.libraries[0].id);
    }

    #[test]
    fn the_open_directory_is_the_one_the_registry_points_at() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        let created = create_in(&profile, "Boulot").unwrap();
        point_at(&profile, &created.id).unwrap();

        let directory = open_directory_in(&profile).unwrap();

        assert_eq!(directory, directory_of(&profile, &created));
    }

    #[test]
    fn creating_one_adds_it_to_the_registry_and_leaves_it_closed() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        let first = registry_in(&profile).open;

        let created = create_in(&profile, "  Boulot  ").unwrap();

        let registry = registry_in(&profile);
        assert_eq!(registry.libraries.len(), 2);
        assert_eq!(created.name, "Boulot", "the name is trimmed");
        assert_eq!(registry.open, first, "creating does not open");
        assert!(directory_of(&profile, &created).is_dir());
        assert!(!directory_of(&profile, &created).join(DATABASE).exists());
    }

    #[test]
    fn pointing_at_one_that_does_not_exist_says_so() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        registry_in(&profile);

        assert!(matches!(
            point_at(&profile, "nothing"),
            Err(StorageError::LibraryNotFound(_))
        ));
    }

    #[test]
    fn renaming_touches_the_name_and_nothing_else() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        let created = create_in(&profile, "Boulot").unwrap();
        let open = registry_in(&profile).open;

        rename_in(&profile, &created.id, "  Archives ").unwrap();

        let registry = registry_in(&profile);
        let renamed = registry.entry(&created.id).unwrap();
        assert_eq!(renamed.name, "Archives");
        assert_eq!(renamed.directory, created.directory);
        assert_eq!(registry.open, open);
    }

    #[test]
    fn renaming_one_that_does_not_exist_says_so() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        registry_in(&profile);

        assert!(matches!(
            rename_in(&profile, "nothing", "Archives"),
            Err(StorageError::LibraryNotFound(_))
        ));
    }

    #[test]
    fn deleting_takes_the_entry_and_the_files() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        let created = create_in(&profile, "Boulot").unwrap();
        std::fs::write(directory_of(&profile, &created).join(DATABASE), b"x").unwrap();

        delete_in(&profile, &created.id).unwrap();

        assert_eq!(registry_in(&profile).libraries.len(), 1);
        assert!(!directory_of(&profile, &created).exists());
    }

    #[test]
    fn the_last_library_cannot_be_deleted() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        let only = registry_in(&profile).libraries[0].clone();

        assert!(matches!(
            delete_in(&profile, &only.id),
            Err(StorageError::LastLibrary)
        ));
        assert_eq!(registry_in(&profile).libraries.len(), 1);
    }

    #[test]
    fn deleting_the_open_one_moves_the_mark_to_what_remains() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        let created = create_in(&profile, "Boulot").unwrap();
        point_at(&profile, &created.id).unwrap();

        delete_in(&profile, &created.id).unwrap();

        let registry = registry_in(&profile);
        assert_eq!(registry.libraries.len(), 1);
        assert_eq!(
            registry.open.as_deref(),
            Some(registry.libraries[0].id.as_str())
        );
    }

    #[test]
    fn deleting_one_that_does_not_exist_says_so() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        create_in(&profile, "Boulot").unwrap();

        assert!(matches!(
            delete_in(&profile, "nothing"),
            Err(StorageError::LibraryNotFound(_))
        ));
    }
}
