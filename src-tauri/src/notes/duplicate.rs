//! A note copied whole, attachments included, to start another from.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::model::Note;
use super::store;
use crate::attachments;
use crate::attachments::model::Attachment;
use crate::db::Library;
use crate::error::{FileContext, StorageError};

/// The files first, then the rows in one transaction, as for any attachment: a copy that fails
/// half way leaves files the startup sweep collects, never a record without its bytes. The
/// sealed bytes are copied as they are — the key that opens them is the library's.
pub(crate) fn duplicate(
    library: &mut Library,
    id: &str,
    title: String,
    now: DateTime<Utc>,
) -> Result<Note, StorageError> {
    let copy = store::get(library, id)?.duplicate(Uuid::new_v4().to_string(), title, now);

    let directory = attachments::directory(library);
    let mut carried = Vec::new();
    for attachment in attachments::store::list(library, id)? {
        let source = directory.join(attachment.stored_name());
        // Left behind, as an export leaves it: the original has no bytes to give either.
        if !source.is_file() {
            continue;
        }

        let twin = Attachment {
            id: Uuid::new_v4().to_string(),
            note_id: copy.id.clone(),
            ..attachment
        };
        std::fs::copy(&source, directory.join(twin.stored_name())).context(twin.stored_name())?;
        carried.push(twin);
    }

    library.transaction(|connection, vault| {
        store::insert_imported_in(connection, vault, &copy)?;
        for attachment in &carried {
            attachments::store::create(connection, vault, attachment)?;
        }

        Ok(())
    })?;

    Ok(copy)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::notes::checklist::{ChecklistItem, NoteKind};
    use crate::notes::fixtures::{NOW, at, note};

    fn library_with(original: impl FnOnce(String) -> Note) -> (tempfile::TempDir, Library, Note) {
        let scratch = tempfile::tempdir().unwrap();
        let mut library = crate::db::open_in_memory()
            .unwrap()
            .with_directory(scratch.path().to_path_buf());
        let space_id = crate::spaces::store::create(&mut library, "Perso")
            .unwrap()
            .id;
        let original = original(space_id);
        store::insert_imported(&mut library, &original).unwrap();

        (scratch, library, original)
    }

    fn attach(library: &mut Library, note_id: &str, id: &str) -> Attachment {
        let attachment = Attachment {
            id: id.to_string(),
            note_id: note_id.to_string(),
            file_name: "capture.png".to_string(),
            mime_type: "image/png".to_string(),
            byte_size: 4,
            created_at: at(NOW),
        };
        let directory = attachments::directory(library);
        std::fs::create_dir_all(&directory).unwrap();
        attachments::sealed::write_sealed(
            library.vault(),
            &directory.join(attachment.stored_name()),
            b"\x89PNG",
        )
        .unwrap();
        let (db, vault) = library.split();
        attachments::store::create(db, vault, &attachment).unwrap();

        attachment
    }

    #[test]
    fn the_copy_says_what_the_note_says_under_its_own_id_and_title() {
        let (_scratch, mut library, original) = library_with(|space_id| Note {
            space_id,
            pinned: true,
            content: "ssh {{user}}@{{host}}".to_string(),
            source: "OpenSSH".to_string(),
            placeholder_values: BTreeMap::from([("user".to_string(), "deploy".to_string())]),
            ..note()
        });
        let later = at("2026-07-26T09:00:00.000Z");

        let copy = duplicate(
            &mut library,
            &original.id,
            "Title (copy)".to_string(),
            later,
        )
        .unwrap();

        let read = store::get(&mut library, &copy.id).unwrap();
        assert_ne!(read.id, original.id);
        assert_eq!(read.title, "Title (copy)");
        assert_eq!(read.content, original.content);
        assert_eq!(read.source, original.source);
        assert_eq!(read.tags, original.tags);
        assert_eq!(read.placeholder_values, original.placeholder_values);
        assert_eq!(read.space_id, original.space_id);
        assert!(!read.pinned, "a copy is not pinned");
        assert_eq!((read.created_at, read.updated_at), (later, later));
        assert_eq!(
            store::get(&mut library, &original.id).unwrap().title,
            original.title
        );
    }

    #[test]
    fn a_todo_list_keeps_its_items() {
        let (_scratch, mut library, original) = library_with(|space_id| Note {
            space_id,
            kind: NoteKind::Checklist,
            content: String::new(),
            items: vec![
                ChecklistItem {
                    text: "Tag".to_string(),
                    done: true,
                },
                ChecklistItem {
                    text: "Publish".to_string(),
                    done: false,
                },
            ],
            ..note()
        });

        let copy = duplicate(&mut library, &original.id, "Copy".to_string(), at(NOW)).unwrap();

        assert_eq!(
            store::get(&mut library, &copy.id).unwrap().items,
            original.items
        );
    }

    #[test]
    fn the_attachments_are_copied_under_new_ids_and_still_open() {
        let (_scratch, mut library, original) =
            library_with(|space_id| Note { space_id, ..note() });
        let attachment = attach(&mut library, &original.id, "a-1");

        let copy = duplicate(&mut library, &original.id, "Copy".to_string(), at(NOW)).unwrap();

        let carried = attachments::store::list(&mut library, &copy.id).unwrap();
        assert_eq!(carried.len(), 1);
        assert_ne!(carried[0].id, attachment.id);
        assert_eq!(carried[0].file_name, attachment.file_name);
        let sealed =
            std::fs::read(attachments::directory(&library).join(carried[0].stored_name())).unwrap();
        assert_eq!(library.vault().open_bytes(&sealed).unwrap(), b"\x89PNG");
        assert_eq!(
            attachments::store::list(&mut library, &original.id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn an_attachment_whose_file_went_missing_stays_behind() {
        let (_scratch, mut library, original) =
            library_with(|space_id| Note { space_id, ..note() });
        let attachment = attach(&mut library, &original.id, "a-1");
        std::fs::remove_file(attachments::directory(&library).join(attachment.stored_name()))
            .unwrap();

        let copy = duplicate(&mut library, &original.id, "Copy".to_string(), at(NOW)).unwrap();

        assert!(
            attachments::store::list(&mut library, &copy.id)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn an_unknown_note_is_refused() {
        let (_scratch, mut library, _) = library_with(|space_id| Note { space_id, ..note() });

        assert!(matches!(
            duplicate(&mut library, "missing", "Copy".to_string(), at(NOW)),
            Err(StorageError::NoteNotFound(_))
        ));
    }
}
