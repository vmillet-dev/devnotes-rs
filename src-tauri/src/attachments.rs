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

use crate::count::saturating_u32;
use crate::db::{Db, Library, lock};
use crate::error::{AppError, FileContext, StorageError};
use model::Attachment;

/// Created when the library opens (`vault::open_library`), so every writer can assume it.
pub fn directory(library: &Library) -> PathBuf {
    library.directory().join(crate::layout::ATTACHMENTS)
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

/// ⚠️ The limit is enforced by the read rather than by `metadata`: a file growing between
/// the two would land whole, whatever the limit said. One byte past it is enough to refuse.
fn read_within_limit(source: &str) -> Result<Vec<u8>, StorageError> {
    let reader = std::fs::File::open(source).context(source)?;

    let mut bytes = Vec::new();
    std::io::copy(&mut reader.take(model::MAX_BYTES + 1), &mut bytes).context(source)?;

    Ok(bytes)
}

/// A new attachment of `note_id`: `bytes` sealed beside the library, then recorded. Their
/// size is the caller's to have validated.
///
/// ⚠️ The file before the record, and the file removed again when the record cannot be
/// written: a record without a file shows a broken thumbnail, where a file without a record
/// is swept at startup. Sealed on the way in, so nothing readable is ever written.
fn store_new(
    note_id: String,
    file_name: String,
    bytes: &[u8],
    db: &Db,
) -> Result<Attachment, StorageError> {
    let attachment = Attachment {
        id: Uuid::new_v4().to_string(),
        note_id,
        mime_type: model::mime_of(&file_name),
        file_name,
        byte_size: saturating_u32(bytes.len()),
        created_at: Utc::now(),
    };

    let mut connection = lock(db)?;
    let directory = directory(&connection);
    let destination = directory.join(attachment.stored_name());
    let (db, vault) = connection.split();

    let stored = sealed::write_sealed(vault, &destination, bytes)
        .and_then(|_| store::create(db, vault, &attachment));
    if stored.is_err() {
        remove_files(&directory, &[attachment.stored_name()]);
    }

    stored.map(|()| attachment)
}

#[tauri::command(async)]
#[specta::specta]
pub fn attach_file(
    note_id: String,
    path: String,
    db: State<'_, Db>,
) -> Result<Attachment, AppError> {
    let file_name = model::display_name(&path)?;
    let bytes = read_within_limit(&path)?;
    model::validate_size(bytes.len() as u64)?;
    Ok(store_new(note_id, file_name, &bytes, &db)?)
}

/// The record and its bytes, opened. What the three read commands share.
///
/// ⚠️ The record is looked up first: opening a file nothing refers to would be a leak out
/// of the directory.
pub fn read_plain(library: &mut Library, id: &str) -> Result<(Attachment, Vec<u8>), StorageError> {
    let attachment = store::find(library, id)?
        .ok_or_else(|| StorageError::AttachmentNotFound(id.to_string()))?;
    let path = directory(library).join(attachment.stored_name());
    let bytes = sealed::read_sealed(library.vault(), &path)?;

    Ok((attachment, bytes))
}

#[tauri::command(async)]
#[specta::specta]
pub fn list_attachments(note_id: String, db: State<'_, Db>) -> Result<Vec<Attachment>, AppError> {
    let mut connection = lock(&db)?;

    Ok(store::list(&mut connection, &note_id)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn read_attachment(id: String, db: State<'_, Db>) -> Result<String, AppError> {
    let (attachment, bytes) = read_plain(&mut *lock(&db)?, &id)?;

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
    let (attachment, bytes) = read_plain(&mut *lock(&db)?, &id)?;

    let directory = sealed::plaintext_directory(&app)?;
    std::fs::create_dir_all(&directory).context("a directory for decrypted copies")?;

    // Named after the record, not after what the user called it: two `capture.png` must
    // not overwrite each other here either.
    let copy = directory.join(attachment.stored_name());
    std::fs::write(&copy, &bytes).context(attachment.file_name)?;

    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_path(copy.to_string_lossy(), None::<&str>)
        .context("open")?;

    Ok(())
}

/// The path comes from a native picker; the write stays here.
#[tauri::command(async)]
#[specta::specta]
pub fn save_attachment(id: String, path: String, db: State<'_, Db>) -> Result<(), AppError> {
    let (_, bytes) = read_plain(&mut *lock(&db)?, &id)?;

    // In the clear, where the user chose: that is what "save as" means, and it is an
    // explicit gesture rather than something the application does behind them.
    std::fs::write(&path, &bytes).context(path)?;

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
        .context("clipboard image")?;

    let png = model::encode_png(image.width(), image.height(), image.rgba())?;
    model::validate_size(png.len() as u64)?;
    Ok(store_new(note_id, model::png_name(&file_name), &png, &db)?)
}

#[tauri::command(async)]
#[specta::specta]
pub fn delete_attachment(id: String, db: State<'_, Db>) -> Result<(), AppError> {
    Ok(delete(&db, &id)?)
}

/// The record, then the file, which is removed outside the lock.
fn delete(db: &Db, id: &str) -> Result<(), StorageError> {
    let mut connection = lock(db)?;
    let directory = directory(&connection);
    let stored_name = store::find(&mut connection, id)?
        .ok_or_else(|| StorageError::AttachmentNotFound(id.to_string()))?
        .stored_name();
    store::delete(&mut connection, id)?;
    drop(connection);

    remove_files(&directory, &[stored_name]);

    Ok(())
}

/// Files no record claims any more: a copy interrupted between `fs::copy` and the
/// insert leaves one, and so does a purge that fails in between.
pub fn sweep_orphan_files(db: &Db) -> Result<usize, StorageError> {
    let (directory, known) = {
        let mut connection = lock(db)?;
        (
            directory(&connection),
            store::all_stored_names(&mut connection)?,
        )
    };

    let entries = std::fs::read_dir(&directory).context("attachments sweep")?;

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

    fn scratch() -> PathBuf {
        let directory = std::env::temp_dir().join(format!("devnotes-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    fn a_library_with_a_note() -> (Db, String) {
        use crate::notes::model::NoteDraft;

        let mut library = crate::db::open_in_memory().unwrap();
        let space = crate::spaces::store::create(&mut library, "Personal").unwrap();
        let note = crate::notes::store::create(
            &mut library,
            NoteDraft {
                space_id: space.id,
                folder_id: None,
                title: "T".to_string(),
                language: crate::notes::language::Language::Txt,
                content: String::new(),
                source: String::new(),
                tags: Vec::new(),
                pinned: false,
                lifecycle: crate::notes::model::NoteLifecycle::Permanent,
                kind: crate::notes::checklist::NoteKind::Snippet,
                items: Vec::new(),
            },
            Utc::now(),
        )
        .unwrap();

        (std::sync::Mutex::new(Some(library)), note.id)
    }

    /// An in-memory library names a directory it never creates; opening one creates it.
    fn attachments_of(db: &Db) -> PathBuf {
        let directory = directory(&lock(db).unwrap());
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    #[test]
    fn a_new_attachment_is_sealed_beside_the_library_then_recorded() {
        let (db, note_id) = a_library_with_a_note();
        let directory = attachments_of(&db);
        let attachment =
            store_new(note_id.clone(), "capture.png".to_string(), b"png", &db).unwrap();

        let written = std::fs::read(directory.join(attachment.stored_name())).unwrap();
        assert_ne!(written, b"png");
        let listed = store::list(&mut lock(&db).unwrap(), &note_id).unwrap();
        assert_eq!(listed[0].mime_type, "image/png");
        std::fs::remove_dir_all(&directory).ok();
    }

    /// A file without a record would sit there until the next launch's sweep.
    #[test]
    fn an_attachment_that_cannot_be_recorded_leaves_no_file_behind() {
        let (db, _) = a_library_with_a_note();
        let directory = attachments_of(&db);
        let refused = store_new("ghost".to_string(), "capture.png".to_string(), b"png", &db);

        assert!(matches!(refused, Err(StorageError::NoteNotFound(_))));
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn an_attachment_reads_back_as_the_bytes_it_was_given() {
        let (db, note_id) = a_library_with_a_note();
        let directory = attachments_of(&db);
        let stored = store_new(note_id, "capture.png".to_string(), b"png", &db).unwrap();

        let (attachment, bytes) = read_plain(&mut lock(&db).unwrap(), &stored.id).unwrap();

        assert_eq!(attachment.id, stored.id);
        assert_eq!(bytes, b"png");
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn an_attachment_nobody_recorded_is_not_read() {
        let (db, _) = a_library_with_a_note();

        let refused = read_plain(&mut lock(&db).unwrap(), "../vault");

        assert!(matches!(refused, Err(StorageError::AttachmentNotFound(_))));
    }

    #[test]
    fn deleting_an_attachment_takes_its_file_with_it() {
        let (db, note_id) = a_library_with_a_note();
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
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn the_sweep_removes_the_files_no_record_claims_and_only_those() {
        let (db, note_id) = a_library_with_a_note();
        let directory = attachments_of(&db);
        let kept = store_new(note_id, "capture.png".to_string(), b"png", &db).unwrap();
        std::fs::write(directory.join("orphan.png"), b"left behind").unwrap();

        assert_eq!(sweep_orphan_files(&db).unwrap(), 1);

        assert!(directory.join(kept.stored_name()).exists());
        assert!(!directory.join("orphan.png").exists());
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_file_within_the_limit_is_read_whole() {
        let directory = scratch();
        let source = directory.join("capture.png");
        std::fs::write(&source, vec![7u8; 2048]).unwrap();

        let bytes = read_within_limit(&source.to_string_lossy()).unwrap();

        assert_eq!(bytes, vec![7u8; 2048]);
        std::fs::remove_dir_all(&directory).ok();
    }

    /// The limit is applied by the read, so a file that grew past it is still refused — and
    /// no more than one byte past it is ever held.
    #[test]
    fn a_file_over_the_limit_is_read_one_byte_past_it_and_refused() {
        let directory = scratch();
        let source = directory.join("huge.bin");
        let limit = usize::try_from(model::MAX_BYTES).unwrap();
        std::fs::write(&source, vec![0u8; limit + 10]).unwrap();

        let bytes = read_within_limit(&source.to_string_lossy()).unwrap();

        assert_eq!(bytes.len(), limit + 1);
        assert_eq!(
            model::validate_size(bytes.len() as u64).unwrap_err().field,
            "byteSize"
        );
        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_missing_source_is_reported_rather_than_panicking() {
        let error = read_within_limit("no-such-file.png").unwrap_err();

        assert!(matches!(error, StorageError::File(_)), "{error}");
    }
}
