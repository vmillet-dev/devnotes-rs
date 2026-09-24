use std::collections::HashMap;

use diesel::prelude::*;

use super::model::{self, Attachment};
use crate::count::saturating_u32;
use crate::db::Library;
use crate::db::iso8601;
use crate::db::schema::{attachments, notes};
use crate::error::StorageError;
use crate::vault::key::Vault;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = attachments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct AttachmentRow {
    id: String,
    note_id: String,
    file_name: String,
    mime_type: String,
    byte_size: i64,
    created_at: String,
}

/// ⚠️ `mime_type` stays in the clear, deliberately: the file on disk is named
/// `{id}.{extension}`, so the type is already public. Sealing it would be theatre.
impl AttachmentRow {
    fn open(row: Self, vault: &Vault) -> Result<Attachment, StorageError> {
        let created_at = iso8601::parse(&row.created_at).map_err(|_| StorageError::CorruptRow {
            id: row.id.clone(),
            field: "createdAt",
        })?;

        Ok(Attachment {
            byte_size: saturating_u32(row.byte_size),
            file_name: vault.open(&row.file_name)?,
            id: row.id,
            note_id: row.note_id,
            mime_type: row.mime_type,
            created_at,
        })
    }

    fn seal(attachment: &Attachment, vault: &Vault) -> Result<Self, StorageError> {
        Ok(Self {
            id: attachment.id.clone(),
            note_id: attachment.note_id.clone(),
            file_name: vault.seal(&attachment.file_name)?,
            mime_type: attachment.mime_type.clone(),
            byte_size: i64::from(attachment.byte_size),
            created_at: iso8601::format(attachment.created_at),
        })
    }
}

/// The foreign key would refuse it too, but with a message the front cannot translate.
pub fn create(
    connection: &mut SqliteConnection,
    vault: &Vault,
    attachment: &Attachment,
) -> Result<(), StorageError> {
    connection.transaction(|connection| {
        let known = notes::table
            .find(&attachment.note_id)
            .filter(notes::deleted_at.is_null())
            .select(notes::id)
            .first::<String>(connection)
            .optional()?
            .is_some();
        if !known {
            return Err(StorageError::NoteNotFound(attachment.note_id.clone()));
        }

        diesel::insert_into(attachments::table)
            .values(AttachmentRow::seal(attachment, vault)?)
            .execute(connection)?;

        Ok(())
    })
}

pub fn list(connection: &mut Library, note_id: &str) -> Result<Vec<Attachment>, StorageError> {
    let (db, vault) = connection.split();

    attachments::table
        .filter(attachments::note_id.eq(note_id))
        .select(AttachmentRow::as_select())
        .order((attachments::created_at.asc(), attachments::id.asc()))
        .load::<AttachmentRow>(db)?
        .into_iter()
        .map(|row| AttachmentRow::open(row, vault))
        .collect()
}

pub fn find(connection: &mut Library, id: &str) -> Result<Option<Attachment>, StorageError> {
    let (db, vault) = connection.split();

    attachments::table
        .find(id)
        .select(AttachmentRow::as_select())
        .first::<AttachmentRow>(db)
        .optional()?
        .map(|row| AttachmentRow::open(row, vault))
        .transpose()
}

pub fn delete(connection: &mut SqliteConnection, id: &str) -> Result<(), StorageError> {
    let deleted = diesel::delete(attachments::table.find(id)).execute(connection)?;

    if deleted == 0 {
        return Err(StorageError::AttachmentNotFound(id.to_string()));
    }

    Ok(())
}

/// The records an export carries. The bytes are not here: the caller reads them from the
/// attachments directory by [`model::stored_name`].
pub fn for_notes(
    connection: &mut Library,
    note_ids: &[String],
) -> Result<Vec<Attachment>, StorageError> {
    if note_ids.is_empty() {
        return Ok(Vec::new());
    }

    let (db, vault) = connection.split();

    attachments::table
        .filter(attachments::note_id.eq_any(note_ids))
        .select(AttachmentRow::as_select())
        .order((attachments::created_at.asc(), attachments::id.asc()))
        .load::<AttachmentRow>(db)?
        .into_iter()
        .map(|row| AttachmentRow::open(row, vault))
        .collect()
}

/// ⚠️ Collected before a purge: the cascade takes the records, never the files.
pub fn stored_names_of(
    connection: &mut Library,
    note_ids: &[String],
) -> Result<Vec<String>, StorageError> {
    if note_ids.is_empty() {
        return Ok(Vec::new());
    }

    let (db, vault) = connection.split();

    attachments::table
        .filter(attachments::note_id.eq_any(note_ids))
        .select((attachments::id, attachments::file_name))
        .load::<(String, String)>(db)?
        .iter()
        .map(|(id, file_name)| Ok(model::stored_name(id, &vault.open(file_name)?)))
        .collect::<Result<Vec<_>, StorageError>>()
}

pub fn all_stored_names(connection: &mut Library) -> Result<Vec<String>, StorageError> {
    let (db, vault) = connection.split();

    attachments::table
        .select((attachments::id, attachments::file_name))
        .load::<(String, String)>(db)?
        .iter()
        .map(|(id, file_name)| Ok(model::stored_name(id, &vault.open(file_name)?)))
        .collect::<Result<Vec<_>, StorageError>>()
}

/// One note; [`counts`] answers for the whole corpus at once.
pub fn counts(connection: &mut SqliteConnection) -> Result<HashMap<String, u32>, StorageError> {
    let rows = attachments::table
        .group_by(attachments::note_id)
        .select((attachments::note_id, diesel::dsl::count_star()))
        .load::<(String, i64)>(connection)?;

    Ok(rows
        .into_iter()
        .map(|(note_id, count)| (note_id, saturating_u32(count)))
        .collect())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::db::open_in_memory;
    use crate::notes::checklist::NoteKind;
    use crate::notes::language::Language;
    use crate::notes::model::{NoteDraft, NoteLifecycle};

    fn note(connection: &mut Library) -> String {
        let space = crate::spaces::store::create(connection, "Personal").unwrap();
        crate::notes::store::create(
            connection,
            NoteDraft {
                space_id: space.id,
                folder_id: None,
                title: "T".to_string(),
                language: Language::Txt,
                content: String::new(),
                source: String::new(),
                tags: Vec::new(),
                pinned: false,
                lifecycle: NoteLifecycle::Permanent,
                kind: NoteKind::Snippet,
                items: Vec::new(),
            },
            Utc::now(),
        )
        .unwrap()
        .id
    }

    /// The tests hold a library; `create` takes the pair, as an import does.
    fn create_here(connection: &mut Library, attachment: &Attachment) -> Result<(), StorageError> {
        let (db, vault) = connection.split();
        create(db, vault, attachment)
    }

    fn sample(id: &str, note_id: &str) -> Attachment {
        Attachment {
            id: id.to_string(),
            note_id: note_id.to_string(),
            file_name: "capture.png".to_string(),
            mime_type: "image/png".to_string(),
            byte_size: 12,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn an_attachment_is_read_back_whole() {
        let mut connection = open_in_memory().unwrap();
        let note_id = note(&mut connection);

        create_here(&mut connection, &sample("a-1", &note_id)).unwrap();
        let listed = list(&mut connection, &note_id).unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].file_name, "capture.png");
        assert_eq!(listed[0].byte_size, 12);
    }

    #[test]
    fn attaching_to_an_unknown_note_is_refused() {
        let mut connection = open_in_memory().unwrap();

        let error = create_here(&mut connection, &sample("a-1", "ghost")).unwrap_err();

        assert!(matches!(error, StorageError::NoteNotFound(_)));
    }

    #[test]
    fn purging_a_note_takes_its_attachment_rows_with_it() {
        let mut connection = open_in_memory().unwrap();
        let note_id = note(&mut connection);
        create_here(&mut connection, &sample("a-1", &note_id)).unwrap();

        let files = stored_names_of(&mut connection, std::slice::from_ref(&note_id)).unwrap();
        crate::notes::store::trash::trash(&mut connection, &note_id, Utc::now()).unwrap();
        crate::notes::store::trash::purge(&mut connection, std::slice::from_ref(&note_id)).unwrap();

        assert_eq!(files, ["a-1.png"]);
        assert!(list(&mut connection, &note_id).unwrap().is_empty());
    }
}
