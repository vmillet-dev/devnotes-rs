#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::error::AppError;
use engine::transfer::model::ExportReport;
use engine::transfer::model::ExportScope;
use engine::transfer::model::ImportReport;
use engine::{Engine, transfer};
use std::sync::Arc;
use tauri::State;

/// A `None` passphrase writes the file in the clear: the library's key protects what is
/// on this machine, never what leaves it.
#[tauri::command]
#[specta::specta]
pub async fn export_notes(
    path: String,
    scope: ExportScope,
    passphrase: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<ExportReport, AppError> {
    run(&engine, move |engine| {
        transfer::export_notes(engine, path, scope, passphrase)
    })
    .await
}

/// How many notes an export would carry, asked before the save dialog: none is said at once.
#[tauri::command]
#[specta::specta]
pub async fn count_export(
    scope: ExportScope,
    engine: State<'_, Arc<Engine>>,
) -> Result<u32, AppError> {
    run(&engine, move |engine| transfer::count_export(engine, scope)).await
}

/// The file is read before the lock is taken: parsing a large export while holding the
/// connection would block every other command for the length of it.
#[tauri::command]
#[specta::specta]
pub async fn import_notes(
    path: String,
    passphrase: Option<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<ImportReport, AppError> {
    run(&engine, move |engine| {
        transfer::import_notes(engine, path, passphrase)
    })
    .await
}

/// Whether an import will want a phrase, so the interface can ask before it starts.
#[tauri::command(async)]
#[specta::specta]
pub fn export_is_protected(path: String) -> Result<bool, AppError> {
    transfer::export_is_protected(path)
}

/// Nothing is sent anywhere: "share" stops at the clipboard.
#[tauri::command]
#[specta::specta]
pub async fn share_notes(
    ids: Vec<String>,
    engine: State<'_, Arc<Engine>>,
) -> Result<String, AppError> {
    run(&engine, move |engine| transfer::share_notes(engine, ids)).await
}
