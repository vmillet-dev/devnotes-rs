//! ⚠️ The bytes cross the bridge only on read, as a `data:` URI: the `WebView`'s CSP
//! forbids loading a local file, and opening the `asset:` protocol to show a screenshot
//! would be a wide door for a narrow need.

#![allow(clippy::needless_pass_by_value)]

pub mod model;
pub mod sealed;
pub mod store;

use std::io::Read;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::Utc;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::db::{Db, lock};
use crate::error::{AppError, StorageError};
use crate::vault::key::Vault;
use model::Attachment;

fn file_error(context: &str, error: &std::io::Error) -> StorageError {
    StorageError::File(format!("{context}: {error}"))
}

/// Created on demand, so an installation that never attached anything has none.
pub(crate) fn directory(app: &AppHandle) -> Result<PathBuf, StorageError> {
    let path = crate::libraries::open_directory(app)?.join(crate::layout::ATTACHMENTS);
    std::fs::create_dir_all(&path).map_err(|error| file_error("attachments directory", &error))?;

    Ok(path)
}

/// Deletes without reporting: a file already gone is the intended result.
pub(crate) fn remove_files(directory: &Path, stored_names: &[String]) {
    for name in stored_names {
        let path = directory.join(name);
        if let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!("Attachment file {} not removed: {error}", path.display());
        }
    }
}

/// ⚠️ The limit is enforced by the copy itself: reading `metadata().len()` first leaves
/// the two free to disagree, and a file growing between them lands whole.
/// ⚠️ The limit is still enforced by the read rather than by `metadata`: a file growing
/// between the two would land whole, whatever the limit said. What changed is that the
/// bytes are sealed before they touch the destination, so nothing readable is ever
/// written — not even briefly.
fn copy_within_limit(vault: &Vault, source: &str, destination: &Path) -> Result<u32, AppError> {
    let mut reader = std::fs::File::open(source).map_err(|error| file_error(source, &error))?;

    let mut plain = Vec::new();
    std::io::copy(&mut reader.by_ref().take(model::MAX_BYTES + 1), &mut plain)
        .map_err(|error| file_error(source, &error))?;

    let size = model::validate_size(plain.len() as u64)?;
    sealed::write_sealed(vault, destination, &plain)?;

    Ok(size)
}

#[tauri::command(async)]
#[specta::specta]
pub fn attach_file(
    note_id: String,
    path: String,
    app: AppHandle,
    db: State<'_, Db>,
) -> Result<Attachment, AppError> {
    let file_name = model::display_name(&path)?;

    let mut attachment = Attachment {
        id: Uuid::new_v4().to_string(),
        note_id,
        mime_type: model::mime_of(&file_name),
        file_name,
        byte_size: 0,
        created_at: Utc::now(),
    };

    let directory = directory(&app)?;
    let destination = directory.join(attachment.stored_name());

    let mut connection = lock(&db)?;
    let (db, vault) = connection.split();

    // ⚠️ Seal and copy before the database write: a record without a file shows a broken
    // thumbnail, where a file without a record is swept at startup.
    match copy_within_limit(vault, &path, &destination) {
        Ok(byte_size) => attachment.byte_size = byte_size,
        Err(error) => {
            remove_files(&directory, &[attachment.stored_name()]);
            return Err(error);
        }
    }

    if let Err(error) = store::create(db, vault, &attachment) {
        remove_files(&directory, &[attachment.stored_name()]);
        return Err(error.into());
    }

    Ok(attachment)
}

/// ⚠️ Checks the record exists first: opening a file nothing refers to would be a leak
/// out of the directory.
fn locate(id: &str, app: &AppHandle, db: &Db) -> Result<PathBuf, AppError> {
    let stored_name = {
        let mut connection = lock(db)?;
        store::find(&mut connection, id)?
            .ok_or_else(|| StorageError::AttachmentNotFound(id.to_string()))?
            .stored_name()
    };

    Ok(directory(app)?.join(stored_name))
}

fn write_attachment(
    note_id: String,
    file_name: String,
    bytes: Vec<u8>,
    app: &AppHandle,
    db: &Db,
) -> Result<Attachment, AppError> {
    let byte_size = model::validate_size(bytes.len() as u64)?;

    let attachment = Attachment {
        id: Uuid::new_v4().to_string(),
        note_id,
        mime_type: model::mime_of(&file_name),
        file_name,
        byte_size,
        created_at: Utc::now(),
    };

    let directory = directory(app)?;
    let destination = directory.join(attachment.stored_name());

    let mut connection = lock(db)?;
    let (db, vault) = connection.split();

    // Sealed on the way in, like a copied file: a pasted screenshot of a credentials page
    // has no business being the one attachment left readable.
    sealed::write_sealed(vault, &destination, &bytes)?;

    if let Err(error) = store::create(db, vault, &attachment) {
        remove_files(&directory, &[attachment.stored_name()]);
        return Err(error.into());
    }

    Ok(attachment)
}

#[tauri::command(async)]
#[specta::specta]
pub fn list_attachments(note_id: String, db: State<'_, Db>) -> Result<Vec<Attachment>, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::list(&mut connection, &note_id)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn read_attachment(id: String, app: AppHandle, db: State<'_, Db>) -> Result<String, AppError> {
    let attachment = {
        let mut connection = lock(&db)?;
        store::find(&mut connection, &id)?
            .ok_or_else(|| StorageError::AttachmentNotFound(id.clone()))?
    };

    let path = directory(&app)?.join(attachment.stored_name());
    let bytes = {
        let connection = lock(&db)?;
        sealed::read_sealed(connection.vault(), &path)?
    };

    Ok(format!(
        "data:{};base64,{}",
        attachment.mime_type,
        STANDARD.encode(bytes)
    ))
}

/// ⚠️ The call starts from Rust: opening a path from the front end would mean allowing
/// `opener:allow-open-path` over a whole directory.
///
/// ⚠️ **This is the one place a decrypted copy reaches the disk.** Handing a file to the
/// application the desktop chose for it means handing over a path, and that file has to
/// be readable. The copy goes under a directory of ours in the OS temporary folder and is
/// swept at the next launch — it cannot be deleted on close, because the application that
/// opened it still holds it. The README says so; replacing this with "save as" was the
/// alternative and was turned down, one click being the point.
#[tauri::command(async)]
#[specta::specta]
pub fn open_attachment(id: String, app: AppHandle, db: State<'_, Db>) -> Result<(), AppError> {
    let attachment = {
        let mut connection = lock(&db)?;
        store::find(&mut connection, &id)?
            .ok_or_else(|| StorageError::AttachmentNotFound(id.clone()))?
    };

    let stored = directory(&app)?.join(attachment.stored_name());
    let bytes = {
        let connection = lock(&db)?;
        sealed::read_sealed(connection.vault(), &stored)?
    };

    let directory = sealed::plaintext_directory(&app)?;
    std::fs::create_dir_all(&directory)
        .map_err(|error| file_error("a directory for decrypted copies", &error))?;

    // Named after the record, not after what the user called it: two `capture.png` must
    // not overwrite each other here either.
    let copy = directory.join(attachment.stored_name());
    std::fs::write(&copy, &bytes).map_err(|error| file_error(&attachment.file_name, &error))?;

    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(copy.to_string_lossy(), None::<&str>)
        .map_err(|error| StorageError::File(format!("open: {error}")))?;

    Ok(())
}

/// The path comes from a native picker; the write stays here.
#[tauri::command(async)]
#[specta::specta]
pub fn save_attachment(
    id: String,
    path: String,
    app: AppHandle,
    db: State<'_, Db>,
) -> Result<(), AppError> {
    let source = locate(&id, &app, &db)?;
    let bytes = {
        let connection = lock(&db)?;
        sealed::read_sealed(connection.vault(), &source)?
    };

    // In the clear, where the user chose: that is what "save as" means, and it is an
    // explicit gesture rather than something the application does behind them.
    std::fs::write(&path, &bytes).map_err(|error| file_error(&path, &error))?;

    Ok(())
}

/// The bytes do not cross the bridge: the clipboard is read natively, as raw RGBA.
#[tauri::command(async)]
#[specta::specta]
pub fn attach_clipboard_image(
    note_id: String,
    file_name: String,
    app: AppHandle,
    db: State<'_, Db>,
) -> Result<Attachment, AppError> {
    let image = tauri_plugin_clipboard_manager::ClipboardExt::clipboard(&app)
        .read_image()
        .map_err(|error| StorageError::File(format!("clipboard image: {error}")))?;

    let png = model::encode_png(image.width(), image.height(), image.rgba())?;

    write_attachment(note_id, model::png_name(&file_name), png, &app, &db)
}

#[tauri::command(async)]
#[specta::specta]
pub fn delete_attachment(id: String, app: AppHandle, db: State<'_, Db>) -> Result<(), AppError> {
    let directory = directory(&app)?;

    let mut connection = lock(&db)?;
    let stored_name = store::find(&mut connection, &id)?
        .ok_or_else(|| StorageError::AttachmentNotFound(id.clone()))?
        .stored_name();
    store::delete(&mut connection, &id)?;
    drop(connection);

    remove_files(&directory, &[stored_name]);

    Ok(())
}

/// Files no record claims any more: a copy interrupted between `fs::copy` and the
/// insert leaves one, and so does a purge that fails in between.
pub fn sweep_orphan_files(app: &AppHandle, db: &Db) -> Result<usize, StorageError> {
    let directory = directory(app)?;

    let known = {
        let mut connection = lock(db)?;
        store::all_stored_names(&mut connection)?
    };

    let entries =
        std::fs::read_dir(&directory).map_err(|error| file_error("attachments sweep", &error))?;

    let orphans: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !known.contains(name))
        .collect();

    remove_files(&directory, &orphans);

    Ok(orphans.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault() -> Vault {
        crate::db::test_vault().expect("a key")
    }
    use crate::error::ErrorCode;

    fn scratch() -> PathBuf {
        let directory = std::env::temp_dir().join(format!("devnotes-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    #[test]
    fn a_file_within_the_limit_is_copied_whole() {
        let directory = scratch();
        let source = directory.join("capture.png");
        std::fs::write(&source, vec![7u8; 2048]).unwrap();
        let destination = directory.join("a-1.png");

        let vault = test_vault();
        let copied = copy_within_limit(&vault, &source.to_string_lossy(), &destination).unwrap();

        // The size reported is the plaintext's: it is what the interface shows.
        assert_eq!(copied, 2048);

        let written = std::fs::read(&destination).unwrap();
        assert!(written.len() > 2048, "the file carries a nonce and a tag");
        assert_ne!(
            written[..2048],
            [7u8; 2048],
            "and none of the bytes as given"
        );
        assert_eq!(
            sealed::read_sealed(&vault, &destination).unwrap(),
            vec![7u8; 2048]
        );
        std::fs::remove_dir_all(&directory).ok();
    }

    /// The limit is applied by the copy, so a file that grew past it is still refused.
    #[test]
    fn a_file_over_the_limit_is_refused_and_leaves_nothing_behind() {
        let directory = scratch();
        let source = directory.join("huge.bin");
        std::fs::write(
            &source,
            vec![0u8; usize::try_from(model::MAX_BYTES).unwrap() + 1],
        )
        .unwrap();
        let destination = directory.join("a-1.bin");

        let error =
            copy_within_limit(&test_vault(), &source.to_string_lossy(), &destination).unwrap_err();

        assert!(matches!(error.code, ErrorCode::InvalidInput));
        assert_eq!(
            error.params.get("field").map(String::as_str),
            Some("byteSize")
        );
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_missing_source_is_reported_rather_than_panicking() {
        let directory = scratch();

        let error = copy_within_limit(
            &test_vault(),
            "no-such-file.png",
            &directory.join("a-1.png"),
        )
        .unwrap_err();

        assert!(matches!(error.code, ErrorCode::FileAccess));
        std::fs::remove_dir_all(&directory).ok();
    }
}
