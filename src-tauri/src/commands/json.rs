#![allow(clippy::needless_pass_by_value)]

use engine::error::AppError;
use engine::json;
use engine::json::model::JsonQuery;
use engine::json::model::JsonView;

/// No lock, but off the window's thread: a document of a few megabytes takes a while.
#[tauri::command]
#[specta::specta]
pub async fn explore_json(query: JsonQuery) -> Result<JsonView, AppError> {
    json::explore_json(query).await
}
