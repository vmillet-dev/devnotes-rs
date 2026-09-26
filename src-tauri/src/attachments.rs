//! The bytes cross the bridge only on read, as a `data:` URI: the CSP forbids loading a local
//! file, and opening the `asset:` protocol for a screenshot would be a wide door.

#![allow(clippy::needless_pass_by_value)]

pub mod files;
pub mod model;
pub mod sealed;
pub mod store;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use tauri::{AppHandle, Runtime};

use crate::db::{Db, blocking, lock};
use crate::error::{AppError, FileContext, StorageError};
use files::{directory, read_outside_lock, read_within_limit, remove_files, store_new};
use model::Attachment;

/// ⚠️ `path` is not checked, and cannot be: it is the native picker's answer. With
/// `read_attachment` this reads any file the account can read into the `WebView`, and what keeps
/// that harmless is the CSP — `script-src 'self'`, nothing remote — not this function.
#[tauri::command]
#[specta::specta]
pub async fn attach_file<R: Runtime>(
    note_id: String,
    path: String,
    app: AppHandle<R>,
) -> Result<Attachment, AppError> {
    blocking(app, move |_, db| {
        let file_name = model::display_name(&path)?;
        let bytes = read_within_limit(&path)?;
        model::validate_size(bytes.len() as u64)?;
        Ok(store_new(note_id, file_name, &bytes, db)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_attachments<R: Runtime>(
    note_id: String,
    app: AppHandle<R>,
) -> Result<Vec<Attachment>, AppError> {
    blocking(app, move |_, db| {
        let mut connection = lock(db)?;

        Ok(store::list(&mut connection, &note_id)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn read_attachment<R: Runtime>(
    id: String,
    app: AppHandle<R>,
) -> Result<String, AppError> {
    blocking(app, move |_, db| {
        let (attachment, bytes) = read_outside_lock(db, &id)?;

        Ok(format!(
            "data:{};base64,{}",
            attachment.mime_type,
            STANDARD.encode(bytes)
        ))
    })
    .await
}

/// The call starts from Rust: opening a path from the front end would mean allowing
/// `opener:allow-open-path` over a whole directory.
///
/// ⚠️ The one place a decrypted copy reaches the disk: the program the desktop picks reads a
/// path. It goes under the profile's `open/` ([`sealed::plaintext_directory`]) and is swept
/// on exit and at the next launch, since that program may still hold it on close.
#[tauri::command]
#[specta::specta]
pub async fn open_attachment<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), AppError> {
    blocking(app, move |app, db| {
        let (attachment, bytes) = read_outside_lock(db, &id)?;

        let directory = sealed::plaintext_directory(app)?;
        std::fs::create_dir_all(&directory).context("a directory for decrypted copies")?;

        // Named after the record: two `capture.png` must not overwrite each other here either.
        let copy = directory.join(attachment.stored_name());
        std::fs::write(&copy, &bytes).context(attachment.file_name)?;

        tauri_plugin_opener::OpenerExt::opener(app)
            .open_path(copy.to_string_lossy(), None::<&str>)
            .context("open")?;

        Ok(())
    })
    .await
}

/// The path comes from a native picker; the write stays here.
#[tauri::command]
#[specta::specta]
pub async fn save_attachment<R: Runtime>(
    id: String,
    path: String,
    app: AppHandle<R>,
) -> Result<(), AppError> {
    blocking(app, move |_, db| {
        let (_, bytes) = read_outside_lock(db, &id)?;

        // In the clear, where the user chose: that is what "save as" means.
        std::fs::write(&path, &bytes).context(path)?;

        Ok(())
    })
    .await
}

/// The bytes do not cross the bridge: the clipboard is read natively, as raw RGBA.
#[tauri::command]
#[specta::specta]
pub async fn attach_clipboard_image<R: Runtime>(
    note_id: String,
    file_name: String,
    app: AppHandle<R>,
) -> Result<Attachment, AppError> {
    blocking(app, move |app, db| {
        let image = tauri_plugin_clipboard_manager::ClipboardExt::clipboard(app)
            .read_image()
            .context("clipboard image")?;

        let png = model::encode_png(image.width(), image.height(), image.rgba())?;
        model::validate_size(png.len() as u64)?;
        Ok(store_new(note_id, model::png_name(&file_name), &png, db)?)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_attachment<R: Runtime>(id: String, app: AppHandle<R>) -> Result<(), AppError> {
    blocking(app, move |_, db| Ok(delete(db, &id)?)).await
}

/// The record, then the file, which is removed outside the lock.
fn delete(db: &Db, id: &str) -> Result<(), StorageError> {
    let mut connection = lock(db)?;
    let directory = directory(&connection);
    let stored_name = store::find(&mut connection, id)?
        .ok_or_else(|| StorageError::AttachmentNotFound(id.to_string()))?
        .stored_name();
    store::delete(connection.db(), id)?;
    drop(connection);

    remove_files(&directory, &[stored_name]);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use files::tests::{a_library_with_a_note, attachments_of};

    #[test]
    fn deleting_an_attachment_takes_its_file_with_it() {
        let (_scratch, db, note_id) = a_library_with_a_note();
        let directory = attachments_of(&db);
        let stored = store_new(note_id.clone(), "capture.png".to_string(), b"png", &db).unwrap();

        delete(&db, &stored.id).unwrap();

        assert!(!directory.join(stored.stored_name()).exists());
        assert!(
            store::list(&mut lock(&db).unwrap(), &note_id)
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            delete(&db, &stored.id),
            Err(StorageError::AttachmentNotFound(_))
        ));
    }
}
