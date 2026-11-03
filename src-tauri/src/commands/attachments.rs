#![allow(clippy::needless_pass_by_value)]

use super::run;
use engine::attachments::model::Attachment;
use engine::error::AppError;
use engine::error::FileContext;
use engine::{Engine, attachments};
use std::sync::Arc;
use tauri::{AppHandle, State};

/// ⚠️ `path` is not checked, and cannot be: it is the native picker's answer. With
/// `read_attachment` this reads any file the account can read into the `WebView`, and what keeps
/// that harmless is the CSP — `script-src 'self'`, nothing remote — not this function.
#[tauri::command]
#[specta::specta]
pub async fn attach_file(
    note_id: String,
    path: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Attachment, AppError> {
    run(&engine, move |engine| {
        attachments::attach_file(engine, note_id, path)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_attachments(
    note_id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<Vec<Attachment>, AppError> {
    run(&engine, move |engine| {
        attachments::list_attachments(engine, note_id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn read_attachment(
    id: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<String, AppError> {
    run(&engine, move |engine| {
        attachments::read_attachment(engine, id)
    })
    .await
}

/// The path comes from a native picker; the write stays here.
#[tauri::command]
#[specta::specta]
pub async fn save_attachment(
    id: String,
    path: String,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    run(&engine, move |engine| {
        attachments::save_attachment(engine, id, path)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_attachment(id: String, engine: State<'_, Arc<Engine>>) -> Result<(), AppError> {
    run(&engine, move |engine| {
        attachments::delete_attachment(engine, id)
    })
    .await
}

/// The call starts from Rust: opening a path from the front end would mean allowing
/// `opener:allow-open-path` over a whole directory.
#[tauri::command]
#[specta::specta]
pub async fn open_attachment(
    id: String,
    app: AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<(), AppError> {
    let copy = run(&engine, move |engine| {
        attachments::decrypted_copy(engine, id)
    })
    .await?;

    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(copy.to_string_lossy(), None::<&str>)
        .context("open")?;
    Ok(())
}

/// The bytes do not cross the bridge: the clipboard is read natively, as raw RGBA.
#[tauri::command]
#[specta::specta]
pub async fn attach_clipboard_image(
    note_id: String,
    file_name: String,
    app: AppHandle,
    engine: State<'_, Arc<Engine>>,
) -> Result<Attachment, AppError> {
    let image = tauri_plugin_clipboard_manager::ClipboardExt::clipboard(&app)
        .read_image()
        .context("clipboard image")?;
    let size = (image.width(), image.height());
    let rgba = image.rgba().to_vec();

    run(&engine, move |engine| {
        attachments::attach_image(engine, note_id, file_name, size, &rgba)
    })
    .await
}
