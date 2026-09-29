//! The Outils area: each tool a pure function of its module, a request in and an answer out.
//! A tool's own failure — a line that does not parse — is part of its answer; `AppError` is left
//! for the unexpected. No lock and no store: nothing here reads the library.

pub mod encoding;
pub(crate) mod files;
pub mod hash;
pub mod random;
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

#[tauri::command]
#[specta::specta]
pub async fn url_codec(
    request: encoding::UrlCodecRequest,
) -> Result<encoding::UrlCodecAnswer, AppError> {
    off_thread(move || encoding::url_codec(&request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn encode_base64(
    text: String,
    options: encoding::Base64Options,
) -> Result<String, AppError> {
    off_thread(move || encoding::encode_base64(&text, options)).await
}

/// By its path: the bytes are read here and never cross the bridge.
#[tauri::command]
#[specta::specta]
pub async fn encode_base64_file(
    path: String,
    options: encoding::Base64Options,
) -> Result<encoding::Base64FileAnswer, AppError> {
    off_thread(move || encoding::encode_base64_file(&path, options)).await
}

#[tauri::command]
#[specta::specta]
pub async fn decode_base64(
    text: String,
    options: encoding::Base64Options,
) -> Result<encoding::Base64Decoded, AppError> {
    off_thread(move || encoding::decode_base64(&text, options)).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_base64(
    text: String,
    options: encoding::Base64Options,
    path: String,
) -> Result<encoding::Base64Saved, AppError> {
    off_thread(move || encoding::save_base64(&text, options, &path)).await
}

#[tauri::command]
#[specta::specta]
pub async fn hash_input(request: hash::HashRequest) -> Result<hash::HashAnswer, AppError> {
    off_thread(move || hash::hash(request)).await
}

/// No randomness from the system is the unexpected: nothing weaker is drawn in its place.
#[tauri::command]
#[specta::specta]
pub async fn generate_passwords(
    request: random::PasswordRequest,
) -> Result<random::PasswordAnswer, AppError> {
    off_thread(move || random::passwords(&request))
        .await?
        .map_err(|_| StorageError::Unavailable.into())
}

#[tauri::command]
#[specta::specta]
pub async fn generate_uuids(request: random::UuidRequest) -> Result<Vec<String>, AppError> {
    off_thread(move || random::uuids(&request))
        .await?
        .map_err(|_| StorageError::Unavailable.into())
}

#[tauri::command]
#[specta::specta]
pub async fn inspect_uuid(text: String) -> Result<random::UuidInspection, AppError> {
    off_thread(move || random::inspect_uuid(&text)).await
}
