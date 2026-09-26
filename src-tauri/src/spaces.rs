#![allow(clippy::needless_pass_by_value)]

pub mod model;
pub mod store;

use tauri::{AppHandle, Runtime};

use crate::db::{blocking, lock};
use crate::error::AppError;
use model::{Space, SpaceDraft};

#[tauri::command]
#[specta::specta]
pub async fn list_spaces<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Space>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::list(&mut connection)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_space<R: Runtime>(
    draft: SpaceDraft,
    app: AppHandle<R>,
) -> Result<Space, AppError> {
    blocking(app, move |_, db| {
        let name = draft.validated_name()?;

        let mut connection = lock(db)?;

        Ok(store::create(&mut connection, &name)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_space<R: Runtime>(
    id: String,
    draft: SpaceDraft,
    app: AppHandle<R>,
) -> Result<Space, AppError> {
    blocking(app, move |_, db| {
        let name = draft.validated_name()?;

        let mut connection = lock(db)?;

        Ok(store::rename(&mut connection, &id, &name)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn pin_space<R: Runtime>(
    id: String,
    pinned: bool,
    app: AppHandle<R>,
) -> Result<Space, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::set_pinned(&mut connection, &id, pinned)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_space<R: Runtime>(
    id: String,
    target_space_id: String,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        // A space as its own refuge would see its notes swept away by the cascade right
        // after the transfer.
        model::validate_move_target(&id, &target_space_id)?;

        let mut connection = lock(db)?;

        Ok(store::delete(&mut connection, &id, &target_space_id)?)
    })
    .await
}
