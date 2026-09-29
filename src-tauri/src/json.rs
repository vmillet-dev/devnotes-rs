//! The JSON visualiser's model: a text in, what to draw out. It takes a text and not a note,
//! so an HTTP response is explored the same way.

pub(crate) mod layout;
pub mod model;
pub mod parse;

use crate::error::{AppError, StorageError};
use model::{JsonQuery, JsonView};

/// No lock, but off the window's thread: a document of a few megabytes takes a while.
#[tauri::command]
#[specta::specta]
pub async fn explore_json(query: JsonQuery) -> Result<JsonView, AppError> {
    tauri::async_runtime::spawn_blocking(move || model::explore(&query))
        .await
        .map_err(|_| StorageError::Unavailable.into())
}
