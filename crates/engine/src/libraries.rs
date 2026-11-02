//! The list of libraries, and which one is open.
//!
//! The names in `layout` are the address of a library: renaming one sends an installed copy
//! to an empty profile, with the notes still on disk and no way in. A library from before the
//! registry is moved once by `gather` into `libraries/<id>/`, so every library has the same
//! shape; the registry sits beside them and belongs to none.

#![allow(clippy::needless_pass_by_value)]

pub mod registry;

use std::path::PathBuf;

use tauri::{AppHandle, Runtime};

use crate::Engine;
use crate::db::{blocking, engine};
use crate::error::{AppError, StorageError};
use registry::{
    LibraryEntry, Registry, create_in, delete_in, open_directory_in, point_at, registry_in,
    rename_in,
};

/// The directory of the library the registry points at, for what runs while none is open:
/// the gate, the first launch, the recovery commands.
///
/// ⚠️ An open library answers `Library::directory` instead, without reading this file. Never
/// the profile directly: it holds no library.
pub(crate) fn open_directory(engine: &Engine) -> Result<PathBuf, StorageError> {
    open_directory_in(&engine.paths.profile)
}

/// The libraries, and which one is open.
#[tauri::command(async)]
#[specta::specta]
pub fn list_libraries<R: Runtime>(app: AppHandle<R>) -> Result<Registry, AppError> {
    Ok(registry_in(&engine(&app).paths.profile))
}

/// Adds one, and leaves it closed: opening it is a second, deliberate gesture.
#[tauri::command(async)]
#[specta::specta]
pub fn create_library<R: Runtime>(
    name: String,
    app: AppHandle<R>,
) -> Result<LibraryEntry, AppError> {
    Ok(create_in(&engine(&app).paths.profile, &name)?)
}

/// Closes whatever is open and points the registry at another one.
///
/// ⚠️ The connection `Mutex` is emptied under the lock every command takes: they all answer
/// `Locked` afterwards, which sends the interface back to the gate for the other passphrase.
#[tauri::command]
#[specta::specta]
pub async fn open_library<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), AppError> {
    blocking(app, move |engine, db| {
        let mut open = db.lock().map_err(|_| StorageError::Unavailable)?;
        *open = None;
        engine.sockets.close_all();

        Ok(point_at(&engine.paths.profile, &id)?)
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
    Ok(rename_in(&engine(&app).paths.profile, &id, &name)?)
}

/// Erases a library and everything in it.
///
/// Refused on the open one: deleting files under a live connection takes the process down.
#[tauri::command]
#[specta::specta]
pub async fn delete_library<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), AppError> {
    blocking(app, move |engine, db| {
        let profile = &engine.paths.profile;
        if registry_in(profile).open.as_deref() == Some(id.as_str())
            && db.lock().map_err(|_| StorageError::Unavailable)?.is_some()
        {
            return Err(StorageError::LibraryOpen.into());
        }

        Ok(delete_in(profile, &id)?)
    })
    .await
}
