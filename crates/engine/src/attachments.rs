//! The bytes cross the bridge only on read, as a `data:` URI: the CSP forbids loading a local
//! file, and opening the `asset:` protocol for a screenshot would be a wide door.

#![allow(clippy::needless_pass_by_value)]

pub mod files;
pub mod model;
pub mod sealed;
pub mod store;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::Engine;
use crate::db::{Db, lock};
use crate::error::{AppError, StorageError};
use files::{
    directory, read_outside_lock, read_within_limit, remove_files, store_new, write_plain,
    write_plaintext_copy,
};
use model::Attachment;

/// ⚠️ `path` is not checked, and cannot be: it is the native picker's answer. With
/// `read_attachment` this reads any file the account can read into the `WebView`, and what keeps
/// that harmless is the CSP — `script-src 'self'`, nothing remote — not this function.
pub fn attach_file(engine: &Engine, note_id: String, path: String) -> Result<Attachment, AppError> {
    let db = &engine.db;
    let file_name = model::display_name(&path)?;
    let bytes = read_within_limit(&path)?;
    model::validate_size(bytes.len() as u64, &file_name)?;
    Ok(store_new(note_id, file_name, &bytes, db)?)
}

pub fn list_attachments(engine: &Engine, note_id: String) -> Result<Vec<Attachment>, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(store::list(&mut connection, &note_id)?)
}

pub fn read_attachment(engine: &Engine, id: String) -> Result<String, AppError> {
    let db = &engine.db;
    let (attachment, bytes) = read_outside_lock(db, &id)?;

    Ok(format!(
        "data:{};base64,{}",
        attachment.mime_type,
        STANDARD.encode(bytes)
    ))
}

/// The decrypted copy of an attachment, for the desktop to open: the shell opens the path it
/// answers.
///
/// ⚠️ The one place a decrypted copy reaches the disk: the program the desktop picks reads a
/// path. It goes under the profile's `open/` ([`sealed::plaintext_directory`]) and is swept
/// on exit and at the next launch, since that program may still hold it on close.
pub fn decrypted_copy(engine: &Engine, id: String) -> Result<std::path::PathBuf, AppError> {
    let (attachment, bytes) = read_outside_lock(&engine.db, &id)?;
    let directory = sealed::plaintext_directory(&engine.paths);

    Ok(write_plaintext_copy(&directory, &attachment, &bytes)?)
}

/// The path comes from a native picker; the write stays here.
pub fn save_attachment(engine: &Engine, id: String, path: String) -> Result<(), AppError> {
    let db = &engine.db;
    let (_, bytes) = read_outside_lock(db, &id)?;

    Ok(write_plain(&path, &bytes)?)
}

/// An image the interface read, as raw RGBA, stored as a PNG.
pub fn attach_image(
    engine: &Engine,
    note_id: String,
    file_name: String,
    (width, height): (u32, u32),
    rgba: &[u8],
) -> Result<Attachment, AppError> {
    let png = model::encode_png(width, height, rgba)?;
    model::validate_size(png.len() as u64, &model::png_name(&file_name))?;
    Ok(store_new(
        note_id,
        model::png_name(&file_name),
        &png,
        &engine.db,
    )?)
}

pub fn delete_attachment(engine: &Engine, id: String) -> Result<(), AppError> {
    let db = &engine.db;
    Ok(delete(db, &id)?)
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
