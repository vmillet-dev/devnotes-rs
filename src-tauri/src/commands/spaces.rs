#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::error::AppError;
use engine::spaces::model::Space;
use engine::spaces::model::SpaceContents;
use engine::spaces::model::SpaceDraft;
use engine::{Engine, spaces};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn list_spaces(engine: State<'_, Arc<Engine>>) -> Result<Vec<Space>, AppError> {
    run(&engine, spaces::list_spaces).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_space(
    draft: SpaceDraft,
    engine: State<'_, Arc<Engine>>,
) -> Result<Space, AppError> {
    run(&engine, move |engine| spaces::create_space(engine, draft)).await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_space(
    id: String,
    draft: SpaceDraft,
    engine: State<'_, Arc<Engine>>,
) -> Result<Space, AppError> {
    run(&engine, move |engine| {
        spaces::rename_space(engine, id, draft)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn pin_space(
    id: String,
    pinned: bool,
    engine: State<'_, Arc<Engine>>,
) -> Result<Space, AppError> {
    run(&engine, move |engine| spaces::pin_space(engine, id, pinned)).await
}

/// What deleting the space would move and take, for the confirmation that comes first.
#[tauri::command]
#[specta::specta]
pub async fn count_space_contents(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<SpaceContents, AppError> {
    run(&engine, move |engine| {
        spaces::count_space_contents(engine, id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_space(
    id: String,
    target_space_id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        spaces::delete_space(engine, id, target_space_id)
    })
    .await
}
