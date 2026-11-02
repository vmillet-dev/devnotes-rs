#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::error::AppError;
use engine::libraries::registry::{LibraryEntry, Registry};
use engine::{Engine, libraries};
use std::sync::Arc;
use tauri::State;

/// Closes whatever is open and points the registry at another one.
///
/// ⚠️ The connection `Mutex` is emptied under the lock every command takes: they all answer
/// `Locked` afterwards, which sends the interface back to the gate for the other passphrase.
#[tauri::command]
#[specta::specta]
pub async fn open_library(id: String, engine: State<'_, Arc<Engine>>) -> Result<(), AppError> {
    run(&engine, move |engine| libraries::open_library(engine, id)).await
}

/// Erases a library and everything in it.
///
/// Refused on the open one: deleting files under a live connection takes the process down.
#[tauri::command]
#[specta::specta]
pub async fn delete_library(id: String, engine: State<'_, Arc<Engine>>) -> Result<(), AppError> {
    run(&engine, move |engine| libraries::delete_library(engine, id)).await
}

/// The libraries, and which one is open. Off the queue: a short read of a file.
#[tauri::command(async)]
#[specta::specta]
#[allow(
    clippy::unnecessary_wraps,
    reason = "the front reads a result, as it always has"
)]
pub fn list_libraries(engine: State<'_, Arc<Engine>>) -> Result<Registry, AppError> {
    Ok(libraries::list_libraries(&engine))
}

/// Adds one, and leaves it closed: opening it is a second, deliberate gesture.
#[tauri::command(async)]
#[specta::specta]
pub fn create_library(
    name: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<LibraryEntry, AppError> {
    libraries::create_library(&engine, name)
}

#[tauri::command(async)]
#[specta::specta]
pub fn rename_library(
    id: String,
    name: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    libraries::rename_library(&engine, id, name)
}
