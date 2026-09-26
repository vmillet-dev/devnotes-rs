//! The bytes on disk beside their records: where they live, how they arrive and how they go.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::count::saturating_u32;
use crate::db::{Db, Library, lock};
use crate::error::{FileContext, StorageError};
use crate::vault::key::Vault;

use super::model::{self, Attachment};
use super::{sealed, store};

/// Created when the library opens (`vault::gate::install`), so every writer can assume it.
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

/// The limit is enforced by the read rather than by `metadata`, which a growing file would
/// outrun. One byte past it is enough to refuse.
pub(crate) fn read_within_limit(source: &str) -> Result<Vec<u8>, StorageError> {
    let reader = std::fs::File::open(source).context(source)?;

    let mut bytes = Vec::new();
    std::io::copy(&mut reader.take(model::MAX_BYTES + 1), &mut bytes).context(source)?;

    Ok(bytes)
}

/// A new attachment of `note_id`: `bytes` sealed beside the library, then recorded; their size
/// is the caller's to have validated. The file before the record, removed again if the
/// record fails: a record without a file is a broken thumbnail, a file without one is swept.
pub(crate) fn store_new(
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

/// The record and its bytes, opened. The record is looked up first: opening a file nothing
/// refers to would read outside what the library knows.
pub fn read_plain(library: &mut Library, id: &str) -> Result<(Attachment, Vec<u8>), StorageError> {
    let (attachment, path, vault) = locate(library, id)?;

    Ok((attachment, sealed::read_sealed(&vault, &path)?))
}

/// The same for the read commands, with the lock held for the lookup alone: up to 10 MiB read
/// and opened is time every other command would spend waiting.
pub(crate) fn read_outside_lock(db: &Db, id: &str) -> Result<(Attachment, Vec<u8>), StorageError> {
    let (attachment, path, vault) = locate(&mut *lock(db)?, id)?;

    Ok((attachment, sealed::read_sealed(&vault, &path)?))
}

fn locate(
    library: &mut Library,
    id: &str,
) -> Result<(Attachment, PathBuf, Arc<Vault>), StorageError> {
    let attachment = store::find(library, id)?
        .ok_or_else(|| StorageError::AttachmentNotFound(id.to_string()))?;
    let path = directory(library).join(attachment.stored_name());

    Ok((attachment, path, library.shared_vault()))
}

/// Files no record claims: an interrupted copy or a purge failing halfway leaves one.
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
pub(crate) mod tests {
    use super::*;

    pub(crate) fn a_library_with_a_note() -> (tempfile::TempDir, Db, String) {
        use crate::notes::model::NoteDraft;

        let scratch = tempfile::tempdir().unwrap();
        let mut library = crate::db::open_in_memory()
            .unwrap()
            .with_directory(scratch.path().to_path_buf());
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

        (scratch, std::sync::Mutex::new(Some(library)), note.id)
    }

    pub(crate) fn attachments_of(db: &Db) -> PathBuf {
        let directory = directory(&lock(db).unwrap());
        std::fs::create_dir_all(&directory).unwrap();

        directory
    }

    #[test]
    fn a_new_attachment_is_sealed_beside_the_library_then_recorded() {
        let (_scratch, db, note_id) = a_library_with_a_note();
        let directory = attachments_of(&db);
        let attachment =
            store_new(note_id.clone(), "capture.png".to_string(), b"png", &db).unwrap();

        let written = std::fs::read(directory.join(attachment.stored_name())).unwrap();
        assert_ne!(written, b"png");
        let listed = store::list(&mut lock(&db).unwrap(), &note_id).unwrap();
        assert_eq!(listed[0].mime_type, "image/png");
    }

    /// A file without a record would sit there until the next launch's sweep.
    #[test]
    fn an_attachment_that_cannot_be_recorded_leaves_no_file_behind() {
        let (_scratch, db, _) = a_library_with_a_note();
        let directory = attachments_of(&db);
        let refused = store_new("ghost".to_string(), "capture.png".to_string(), b"png", &db);

        assert!(matches!(refused, Err(StorageError::NoteNotFound(_))));
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    }

    #[test]
    fn an_attachment_reads_back_as_the_bytes_it_was_given() {
        let (_scratch, db, note_id) = a_library_with_a_note();
        attachments_of(&db);
        let stored = store_new(note_id, "capture.png".to_string(), b"png", &db).unwrap();

        let (attachment, bytes) = read_plain(&mut lock(&db).unwrap(), &stored.id).unwrap();

        assert_eq!(attachment.id, stored.id);
        assert_eq!(bytes, b"png");
    }

    /// The read commands' path: the same bytes, and the lock free again once they are in hand.
    #[test]
    fn an_attachment_read_outside_the_lock_leaves_it_free() {
        let (_scratch, db, note_id) = a_library_with_a_note();
        attachments_of(&db);
        let stored = store_new(note_id, "capture.png".to_string(), b"png", &db).unwrap();

        let (attachment, bytes) = read_outside_lock(&db, &stored.id).unwrap();

        assert_eq!(
            (attachment.id, bytes.as_slice()),
            (stored.id, b"png".as_slice())
        );
        assert!(db.try_lock().is_ok());
        assert!(matches!(
            read_outside_lock(&db, "unknown"),
            Err(StorageError::AttachmentNotFound(_))
        ));
    }

    #[test]
    fn an_attachment_nobody_recorded_is_not_read() {
        let (_scratch, db, _) = a_library_with_a_note();

        let refused = read_plain(&mut lock(&db).unwrap(), "../vault");

        assert!(matches!(refused, Err(StorageError::AttachmentNotFound(_))));
    }

    #[test]
    fn the_sweep_removes_the_files_no_record_claims_and_only_those() {
        let (_scratch, db, note_id) = a_library_with_a_note();
        let directory = attachments_of(&db);
        let kept = store_new(note_id, "capture.png".to_string(), b"png", &db).unwrap();
        std::fs::write(directory.join("orphan.png"), b"left behind").unwrap();

        assert_eq!(sweep_orphan_files(&db).unwrap(), 1);

        assert!(directory.join(kept.stored_name()).exists());
        assert!(!directory.join("orphan.png").exists());
    }

    #[test]
    fn a_file_within_the_limit_is_read_whole() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let source = directory.join("capture.png");
        std::fs::write(&source, vec![7u8; 2048]).unwrap();

        let bytes = read_within_limit(&source.to_string_lossy()).unwrap();

        assert_eq!(bytes, vec![7u8; 2048]);
    }

    /// A file that grew past the limit is still refused, and no more than one byte past it is
    /// ever held.
    #[test]
    fn a_file_over_the_limit_is_read_one_byte_past_it_and_refused() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let source = directory.join("huge.bin");
        let limit = usize::try_from(model::MAX_BYTES).unwrap();
        std::fs::write(&source, vec![0u8; limit + 10]).unwrap();

        let bytes = read_within_limit(&source.to_string_lossy()).unwrap();

        assert_eq!(bytes.len(), limit + 1);
        assert_eq!(
            model::validate_size(bytes.len() as u64).unwrap_err().field,
            "byteSize"
        );
    }

    #[test]
    fn a_missing_source_is_reported_rather_than_panicking() {
        let error = read_within_limit("no-such-file.png").unwrap_err();

        assert!(matches!(error, StorageError::File(_)), "{error}");
    }
}
