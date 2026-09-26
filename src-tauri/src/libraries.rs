//! The list of libraries, and which one is open.
//!
//! The names in `layout` are the address of a library: renaming one sends an installed copy
//! to an empty profile, with the notes still on disk and no way in. A library from before the
//! registry is moved once by `gather` into `libraries/<id>/`, so every library has the same
//! shape; the registry sits beside them and belongs to none.

#![allow(clippy::needless_pass_by_value)]

pub mod registry;

use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

use crate::db::blocking;
use crate::error::{AppError, FileContext, StorageError};
use registry::{
    LibraryEntry, Registry, create_in, delete_in, open_directory_in, point_at, registry_in,
    rename_in,
};

fn profile<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, StorageError> {
    app.path().app_data_dir().context("app_data_dir")
}

pub(crate) fn registry<R: Runtime>(app: &AppHandle<R>) -> Result<Registry, StorageError> {
    Ok(registry_in(&profile(app)?))
}

/// The directory of the library the registry points at, for what runs while none is open:
/// the gate, the first launch, the recovery commands.
///
/// ⚠️ An open library answers `Library::directory` instead, without reading this file. Never
/// `app_data_dir()` directly: that is the profile, which holds no library.
pub(crate) fn open_directory<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, StorageError> {
    open_directory_in(&profile(app)?)
}

/// The libraries, and which one is open.
#[tauri::command(async)]
#[specta::specta]
pub fn list_libraries<R: Runtime>(app: AppHandle<R>) -> Result<Registry, AppError> {
    Ok(registry(&app)?)
}

/// Adds one, and leaves it closed: opening it is a second, deliberate gesture.
#[tauri::command(async)]
#[specta::specta]
pub fn create_library<R: Runtime>(
    name: String,
    app: AppHandle<R>,
) -> Result<LibraryEntry, AppError> {
    Ok(create_in(&profile(&app)?, &name)?)
}

/// Closes whatever is open and points the registry at another one.
///
/// ⚠️ The connection `Mutex` is emptied under the lock every command takes: they all answer
/// `Locked` afterwards, which sends the interface back to the gate for the other passphrase.
#[tauri::command]
#[specta::specta]
pub async fn open_library<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), AppError> {
    blocking(app, move |app, db| {
        let profile = profile(app)?;

        let mut open = db.lock().map_err(|_| StorageError::Unavailable)?;
        *open = None;

        Ok(point_at(&profile, &id)?)
    })
    .await
}

#[tauri::command(async)]
#[specta::specta]
pub fn rename_library<R: Runtime>(
    id: String,
    name: String,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    Ok(rename_in(&profile(&app)?, &id, &name)?)
}

/// Erases a library and everything in it.
///
/// Refused on the open one: deleting files under a live connection takes the process down.
#[tauri::command]
#[specta::specta]
pub async fn delete_library<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), AppError> {
    blocking(app, move |app, db| {
        let profile = profile(app)?;
        if registry_in(&profile).open.as_deref() == Some(id.as_str())
            && db.lock().map_err(|_| StorageError::Unavailable)?.is_some()
        {
            return Err(StorageError::LibraryOpen.into());
        }

        Ok(delete_in(&profile, &id)?)
    })
    .await
}
