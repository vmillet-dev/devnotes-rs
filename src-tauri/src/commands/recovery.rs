#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::error::AppError;
use engine::{Engine, recovery};
use std::sync::Arc;
use tauri::State;

/// Sets the damaged library aside so the next unlock starts on a fresh one.
///
/// Answers the folder it moved everything into, so the interface can say where.
#[tauri::command]
#[specta::specta]
pub async fn set_aside_damaged_library(engine: State<'_, Arc<Engine>>) -> Result<String, AppError> {
    run(&engine, move |engine| {
        recovery::set_aside_damaged_library(engine)
    })
    .await
}

/// Archives a library whose passphrase was forgotten, so a fresh one can be started.
///
/// Nothing is recovered: the notes leave sealed, under the forgotten phrase, and `vault.json`
/// goes with them for the day the phrase comes back.
#[tauri::command]
#[specta::specta]
pub async fn archive_locked_library(engine: State<'_, Arc<Engine>>) -> Result<String, AppError> {
    run(&engine, move |engine| {
        recovery::archive_locked_library(engine)
    })
    .await
}
