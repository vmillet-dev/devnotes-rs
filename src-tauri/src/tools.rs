//! The Outils area: each tool a pure function of its module, a request in and an answer out.
//! A tool's own failure — a line that does not parse — is part of its answer; `AppError` is left
//! for the unexpected. No lock and no store: nothing here reads the library.

pub mod text;
pub mod url_parts;

use crate::error::{AppError, StorageError};

/// Off the window's thread even for a tool that answers in a microsecond: a large paste does not.
async fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| StorageError::Unavailable.into())
}

#[tauri::command]
#[specta::specta]
pub async fn convert_case(text: String) -> Result<Vec<text::CaseConversion>, AppError> {
    off_thread(move || text::convert_case(&text)).await
}

#[tauri::command]
#[specta::specta]
pub async fn slugify(request: text::SlugRequest) -> Result<String, AppError> {
    off_thread(move || text::slugify(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn fix_line_breaks(
    request: text::LineBreaksRequest,
) -> Result<text::LineBreaksAnswer, AppError> {
    off_thread(move || text::fix_line_breaks(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn parse_url(text: String) -> Result<url_parts::UrlAnswer, AppError> {
    off_thread(move || url_parts::parse_url(&text)).await
}
