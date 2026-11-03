#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::backup::copies::Backup;
use engine::error::AppError;
use engine::{Engine, backup};
use std::sync::Arc;
use tauri::State;

/// The copies that exist, newest first, for the panel that lists them.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub async fn list_backups(engine: State<'_, Arc<Engine>>) -> Result<Vec<Backup>, AppError> {
    run(&engine, backup::list_backups).await
}

/// Puts a copy back, and answers where the library it replaced was moved to.
///
/// ⚠️ Closes the library first, under the lock every command takes: renaming a database out
/// from under a live connection loses it. Every command then answers `Locked`, which sends
/// the interface back to the gate to ask for the restored copy's phrase.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
#[specta::specta]
pub async fn restore_backup(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<String, AppError> {
    run(&engine, move |engine| backup::restore_backup(engine, id)).await
}

/// Where the library the last restore replaced went, once: the page it reloaded says it.
#[tauri::command]
#[specta::specta]
pub fn take_restored_aside(engine: State<'_, Arc<Engine>>) -> Option<String> {
    backup::take_restored_aside(&engine)
}
