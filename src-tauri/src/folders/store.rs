//! ⚠️ `folders.name` is sealed, so uniqueness leaves SQL exactly as it did for
//! `spaces.name`: two seals of the same name differ, and a unique index on ciphertext
//! catches nothing. The order does not move — `created_at` stays in the clear, and
//! reading order is what the board lays zones out in.

pub mod board;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use super::model::{Folder, FolderColour, NoteFiling, NoteFolder};
use crate::db::schema::{folders, notes};
use crate::db::{Library, iso8601};
use crate::error::StorageError;
use crate::spaces::store as spaces;
use crate::vault::key::Vault;

#[derive(Queryable, Selectable)]
#[diesel(table_name = folders, check_for_backend(diesel::sqlite::Sqlite))]
struct FolderRow {
    id: String,
    space_id: String,
    name: String,
    colour: String,
    created_at: String,
}

impl FolderRow {
    fn open(self, vault: &Vault) -> Result<Folder, StorageError> {
        Ok(Folder {
            created_at: iso8601::parse(&self.created_at).map_err(|_| StorageError::CorruptRow {
                id: self.id.clone(),
                field: "createdAt",
            })?,
            name: vault.open(&self.name)?,
            // Degrades like `notes.language`: a newer build may have written a colour this
            // one cannot name, and a folder is worth more than its swatch.
            colour: self.colour.parse().unwrap_or_default(),
            id: self.id,
            space_id: self.space_id,
        })
    }
}

/// `None` = every space. Ordered by creation, which is the reading order the board
/// lays its zones out in.
pub fn list(connection: &mut Library, space_id: Option<&str>) -> Result<Vec<Folder>, StorageError> {
    let (db, vault) = connection.split();

    list_in(db, vault, space_id)
}

pub(crate) fn list_in(
    connection: &mut SqliteConnection,
    vault: &Vault,
    space_id: Option<&str>,
) -> Result<Vec<Folder>, StorageError> {
    let mut query = folders::table.select(FolderRow::as_select()).into_boxed();

    if let Some(space_id) = space_id {
        query = query.filter(folders::space_id.eq(space_id.to_string()));
    }

    query
        .order((folders::created_at.asc(), folders::id.asc()))
        .load::<FolderRow>(connection)?
        .into_iter()
        .map(|row| row.open(vault))
        .collect()
}

/// Keyed by folder id: the decoration pass holds notes carrying a `folder_id` and needs
/// nothing else to resolve them.
pub fn by_id(
    connection: &mut Library,
    space_id: Option<&str>,
) -> Result<HashMap<String, NoteFolder>, StorageError> {
    Ok(list(connection, space_id)?
        .into_iter()
        .map(|folder| {
            (
                folder.id.clone(),
                NoteFolder {
                    id: folder.id,
                    name: folder.name,
                    colour: folder.colour,
                },
            )
        })
        .collect())
}

fn find(
    connection: &mut SqliteConnection,
    vault: &Vault,
    id: &str,
) -> Result<Folder, StorageError> {
    folders::table
        .find(id)
        .select(FolderRow::as_select())
        .first::<FolderRow>(connection)
        .optional()?
        .map(|row| row.open(vault))
        .transpose()?
        .ok_or_else(|| StorageError::FolderNotFound(id.to_string()))
}

pub fn exists(connection: &mut SqliteConnection, id: &str) -> Result<bool, StorageError> {
    Ok(folders::table
        .find(id)
        .select(folders::id)
        .first::<String>(connection)
        .optional()?
        .is_some())
}

/// Unique within its space: two spaces may each hold a "Perf".
fn ensure_unique_name(
    connection: &mut SqliteConnection,
    vault: &Vault,
    space_id: &str,
    name: &str,
    except_id: Option<&str>,
) -> Result<(), StorageError> {
    let folders = list_in(connection, vault, Some(space_id))?;
    let held = folders
        .iter()
        .map(|folder| (folder.id.as_str(), folder.name.as_str()));

    if crate::name::is_taken(held, name, except_id) {
        return Err(StorageError::DuplicateFolderName(name.to_string()));
    }

    Ok(())
}

pub fn create(
    connection: &mut Library,
    space_id: &str,
    name: &str,
    now: DateTime<Utc>,
) -> Result<Folder, StorageError> {
    connection.transaction(|connection, vault| create_in(connection, vault, space_id, name, now))
}

/// For a caller already inside a transaction — an import creates the folders it needs.
pub(crate) fn create_in(
    connection: &mut SqliteConnection,
    vault: &Vault,
    space_id: &str,
    name: &str,
    now: DateTime<Utc>,
) -> Result<Folder, StorageError> {
    if !spaces::exists(connection, space_id)? {
        return Err(StorageError::SpaceNotFound(space_id.to_string()));
    }
    ensure_unique_name(connection, vault, space_id, name, None)?;

    let taken: i64 = folders::table
        .filter(folders::space_id.eq(space_id))
        .count()
        .get_result(connection)?;

    let folder = Folder {
        id: Uuid::new_v4().to_string(),
        space_id: space_id.to_string(),
        name: name.to_string(),
        colour: FolderColour::nth(usize::try_from(taken).unwrap_or(0)),
        created_at: now,
    };

    diesel::insert_into(folders::table)
        .values((
            folders::id.eq(&folder.id),
            folders::space_id.eq(&folder.space_id),
            folders::name.eq(vault.seal(&folder.name)?),
            folders::colour.eq(folder.colour.as_str()),
            folders::created_at.eq(iso8601::format(folder.created_at)),
        ))
        .execute(connection)?;

    Ok(folder)
}

pub fn rename(connection: &mut Library, id: &str, name: &str) -> Result<Folder, StorageError> {
    connection.transaction(|connection, vault| {
        let folder = find(connection, vault, id)?;
        ensure_unique_name(connection, vault, &folder.space_id, name, Some(id))?;

        diesel::update(folders::table.find(id))
            .set(folders::name.eq(vault.seal(name)?))
            .execute(connection)?;

        Ok(Folder {
            name: name.to_string(),
            ..folder
        })
    })
}

pub fn recolour(
    connection: &mut Library,
    id: &str,
    colour: FolderColour,
) -> Result<Folder, StorageError> {
    connection.transaction(|connection, vault| {
        diesel::update(folders::table.find(id))
            .set(folders::colour.eq(colour.as_str()))
            .execute(connection)?;

        find(connection, vault, id)
    })
}

/// ⚠️ The notes are not touched here and must not be: `notes.folder_id` carries
/// `ON DELETE SET NULL`, so they come out loose. Nor is `updated_at` refreshed — the
/// user aimed at the folder, and the canvas sorts on that column.
pub fn delete(connection: &mut Library, id: &str) -> Result<(), StorageError> {
    if diesel::delete(folders::table.find(id)).execute(connection.db())? == 0 {
        return Err(StorageError::FolderNotFound(id.to_string()));
    }

    Ok(())
}

/// Answers where each note was filed, which is what putting the filing back needs.
/// `folder_id` of `None` unfiles.
///
/// ⚠️ A note is only filed into a folder of its own space: the two are joined by the
/// space, and a cross-space filing would show a chip the space switcher can never reach.
pub fn file_many(
    connection: &mut Library,
    ids: &[String],
    folder_id: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Vec<NoteFiling>, StorageError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    connection.transaction(|connection, vault| {
        let space_id = match folder_id {
            Some(id) => Some(find(connection, vault, id)?.space_id),
            None => None,
        };

        let mut query = notes::table
            .filter(notes::id.eq_any(ids))
            .filter(notes::deleted_at.is_null())
            .into_boxed();
        if let Some(space_id) = &space_id {
            query = query.filter(notes::space_id.eq(space_id.clone()));
        }

        // ⚠️ Read before the update: afterwards every one of them says `folder_id`, and
        // where each came from is gone.
        let filed: Vec<NoteFiling> = query
            .select((notes::id, notes::folder_id))
            .load::<(String, Option<String>)>(connection)?
            .into_iter()
            .filter(|(_, current)| current.as_deref() != folder_id)
            .map(|(note_id, folder_id)| NoteFiling { note_id, folder_id })
            .collect();

        let touched: Vec<String> = filed.iter().map(|filing| filing.note_id.clone()).collect();
        if folder_id.is_some() {
            // ⚠️ A position row means "loose": a filed card flows inside its zone and has
            // no place of its own to keep consistent.
            board::forget_positions(connection, &touched)?;
        }

        diesel::update(notes::table.filter(notes::id.eq_any(&touched)))
            .set((
                notes::folder_id.eq(folder_id),
                notes::updated_at.eq(iso8601::format(now)),
            ))
            .execute(connection)?;

        // The gesture that filled the zone is the one that should make room in it.
        if let Some(folder_id) = folder_id {
            let held = count_filed(connection, folder_id)?;
            board::grow_to_fit(connection, folder_id, held)?;
        }

        Ok(filed)
    })
}

/// How many live notes a folder holds, which is what its zone has to be tall enough for.
fn count_filed(connection: &mut SqliteConnection, folder_id: &str) -> Result<usize, StorageError> {
    let count: i64 = notes::table
        .filter(notes::folder_id.eq(folder_id))
        .filter(notes::deleted_at.is_null())
        .count()
        .get_result(connection)?;

    Ok(usize::try_from(count).unwrap_or(0))
}

/// Puts filed notes back where they were.
///
/// ⚠️ `updated_at` is left alone, like restoring from the trash: undoing is not editing.
pub fn restore_filings(
    connection: &mut Library,
    filings: &[NoteFiling],
) -> Result<usize, StorageError> {
    if filings.is_empty() {
        return Ok(0);
    }

    connection.transaction(|connection, _vault| {
        let mut restored = 0;

        for filing in filings {
            // A folder deleted since the batch is not an error: the rest still goes back,
            // and that note simply stays loose.
            if let Some(folder_id) = &filing.folder_id
                && !exists(connection, folder_id)?
            {
                continue;
            }

            restored += diesel::update(
                notes::table
                    .find(&filing.note_id)
                    .filter(notes::deleted_at.is_null()),
            )
            .set(notes::folder_id.eq(filing.folder_id.as_deref()))
            .execute(connection)?;
        }

        // Undoing puts notes back into zones that may no longer be tall enough for them.
        for folder_id in filings
            .iter()
            .filter_map(|filing| filing.folder_id.as_deref())
            .collect::<std::collections::BTreeSet<_>>()
        {
            let held = count_filed(connection, folder_id)?;
            board::grow_to_fit(connection, folder_id, held)?;
        }

        Ok(restored)
    })
}
