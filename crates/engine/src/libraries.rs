//! The list of libraries, and which one is open.
//!
//! The names in `layout` are the address of a library: renaming one sends an installed copy
//! to an empty profile, with the notes still on disk and no way in. A library from before the
//! registry is moved once by `gather` into `libraries/<id>/`, so every library has the same
//! shape; the registry sits beside them and belongs to none.

#![allow(clippy::needless_pass_by_value)]

pub mod registry;

use std::path::PathBuf;

use crate::Engine;
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
pub fn list_libraries(engine: &Engine) -> Registry {
    registry_in(&engine.paths.profile)
}

/// Adds one, and leaves it closed: opening it is a second, deliberate gesture.
pub fn create_library(engine: &Engine, name: String) -> Result<LibraryEntry, AppError> {
    Ok(create_in(&engine.paths.profile, &name)?)
}

/// Closes whatever is open and points the registry at another one.
///
/// ⚠️ The connection `Mutex` is emptied under the lock every command takes: they all answer
/// `Locked` afterwards, which sends the interface back to the gate for the other passphrase.
pub fn open_library(engine: &Engine, id: String) -> Result<(), AppError> {
    let db = &engine.db;
    let mut open = db.lock().map_err(|_| StorageError::Unavailable)?;
    *open = None;
    engine.let_go_of_the_network();

    Ok(point_at(&engine.paths.profile, &id)?)
}

pub fn rename_library(engine: &Engine, id: String, name: String) -> Result<(), AppError> {
    Ok(rename_in(&engine.paths.profile, &id, &name)?)
}

/// Erases a library and everything in it.
///
/// Refused on the open one: deleting files under a live connection takes the process down.
pub fn delete_library(engine: &Engine, id: String) -> Result<(), AppError> {
    let db = &engine.db;
    let profile = &engine.paths.profile;
    if registry_in(profile).open.as_deref() == Some(id.as_str())
        && db.lock().map_err(|_| StorageError::Unavailable)?.is_some()
    {
        return Err(StorageError::LibraryOpen.into());
    }

    Ok(delete_in(profile, &id)?)
}
