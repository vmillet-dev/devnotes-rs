//! `folders.name` is sealed, so uniqueness is decided in Rust, as for `spaces.name`: an index
//! on ciphertext catches nothing. `created_at` stays in the clear, and orders the zones.

pub mod board;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use super::model::{Folder, FolderColour, FolderMove, Lineage, NoteFiling, NoteFolder};
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
    parent_id: Option<String>,
}

impl FolderRow {
    fn open(self, vault: &Vault) -> Result<Folder, StorageError> {
        Ok(Folder {
            created_at: iso8601::parse(&self.created_at).map_err(|_| StorageError::CorruptRow {
                id: self.id.clone(),
                field: "createdAt",
            })?,
            name: vault.open(&self.name)?,
            // Degrades like `notes.language`: a folder is worth more than its swatch.
            colour: self.colour.parse().unwrap_or_default(),
            id: self.id,
            space_id: self.space_id,
            parent_id: self.parent_id,
        })
    }
}

/// `None` = every space. Ordered by creation, the order the board lays zones out in.
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

/// One space's tree in creation order, from ids and parents: no name is opened.
pub fn lineage(connection: &mut SqliteConnection, space_id: &str) -> Result<Lineage, StorageError> {
    Ok(Lineage::new(
        folders::table
            .filter(folders::space_id.eq(space_id))
            .order((folders::created_at.asc(), folders::id.asc()))
            .select((folders::id, folders::parent_id))
            .load::<(String, Option<String>)>(connection)?,
    ))
}

/// The tree of the folder's own space, which every rule about one folder reads.
fn lineage_of(connection: &mut SqliteConnection, id: &str) -> Result<Lineage, StorageError> {
    let space_id = folders::table
        .find(id)
        .select(folders::space_id)
        .first::<String>(connection)
        .optional()?
        .ok_or_else(|| StorageError::FolderNotFound(id.to_string()))?;

    lineage(connection, &space_id)
}

/// The folder and every folder under it: what a zone, an opened folder and a filter gather.
pub fn subtree(connection: &mut SqliteConnection, id: &str) -> Result<Vec<String>, StorageError> {
    Ok(lineage_of(connection, id)?.subtree(id))
}

/// Keyed by folder id, for the decoration pass; each chip carries its path for the hover.
pub fn by_id(
    connection: &mut Library,
    space_id: Option<&str>,
) -> Result<HashMap<String, NoteFolder>, StorageError> {
    let folders = list(connection, space_id)?;
    let lineage = Lineage::of(&folders);
    let names: HashMap<&str, &str> = folders
        .iter()
        .map(|folder| (folder.id.as_str(), folder.name.as_str()))
        .collect();

    Ok(folders
        .iter()
        .map(|folder| {
            let path = lineage
                .path(&folder.id)
                .iter()
                .filter_map(|id| names.get(id.as_str()).map(|name| (*name).to_string()))
                .collect();
            (
                folder.id.clone(),
                NoteFolder {
                    id: folder.id.clone(),
                    name: folder.name.clone(),
                    colour: folder.colour,
                    path,
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

/// Whether the folder still exists, in that space: what an undo checks before filing back into it.
pub fn belongs_to(
    connection: &mut SqliteConnection,
    id: &str,
    space_id: &str,
) -> Result<bool, StorageError> {
    Ok(folders::table
        .find(id)
        .filter(folders::space_id.eq(space_id))
        .select(folders::id)
        .first::<String>(connection)
        .optional()?
        .is_some())
}

/// Unique among its siblings: two folders may each hold a "Perf", as two spaces may.
fn ensure_unique_name(
    connection: &mut SqliteConnection,
    vault: &Vault,
    space_id: &str,
    parent_id: Option<&str>,
    name: &str,
    except_id: Option<&str>,
) -> Result<(), StorageError> {
    let folders = list_in(connection, vault, Some(space_id))?;
    let held = folders
        .iter()
        .filter(|folder| folder.parent_id.as_deref() == parent_id)
        .map(|folder| (folder.id.as_str(), folder.name.as_str()));

    if crate::name::is_taken(held, name, except_id) {
        return Err(StorageError::DuplicateFolderName(name.to_string()));
    }

    Ok(())
}

pub fn create(
    connection: &mut Library,
    space_id: &str,
    parent_id: Option<&str>,
    name: &str,
    now: DateTime<Utc>,
) -> Result<Folder, StorageError> {
    connection.transaction(|connection, vault| {
        create_in(connection, vault, space_id, parent_id, name, now)
    })
}

/// For a caller already inside a transaction — an import creates the folders it needs.
pub(crate) fn create_in(
    connection: &mut SqliteConnection,
    vault: &Vault,
    space_id: &str,
    parent_id: Option<&str>,
    name: &str,
    now: DateTime<Utc>,
) -> Result<Folder, StorageError> {
    if !spaces::exists(connection, space_id)? {
        return Err(StorageError::SpaceNotFound(space_id.to_string()));
    }
    lineage(connection, space_id)?.check_child(parent_id)?;
    ensure_unique_name(connection, vault, space_id, parent_id, name, None)?;

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
        parent_id: parent_id.map(str::to_string),
    };

    diesel::insert_into(folders::table)
        .values((
            folders::id.eq(&folder.id),
            folders::space_id.eq(&folder.space_id),
            folders::name.eq(vault.seal(&folder.name)?),
            folders::colour.eq(folder.colour.as_str()),
            folders::created_at.eq(iso8601::format(folder.created_at)),
            folders::parent_id.eq(&folder.parent_id),
        ))
        .execute(connection)?;

    Ok(folder)
}

pub fn rename(connection: &mut Library, id: &str, name: &str) -> Result<Folder, StorageError> {
    connection.transaction(|connection, vault| {
        let folder = find(connection, vault, id)?;
        ensure_unique_name(
            connection,
            vault,
            &folder.space_id,
            folder.parent_id.as_deref(),
            name,
            Some(id),
        )?;

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

/// Under another folder of its space, or at its top: what it holds goes with it. Its zone's
/// frame is let go of, so it takes a free slot on its new parent's board; the answer is where
/// it was, for the undo.
pub fn move_to(
    connection: &mut Library,
    id: &str,
    parent_id: Option<&str>,
) -> Result<FolderMove, StorageError> {
    connection.transaction(|connection, vault| {
        let folder = find(connection, vault, id)?;
        let was = FolderMove {
            folder_id: folder.id.clone(),
            parent_id: folder.parent_id.clone(),
            frame: board::frame_of(connection, id)?,
        };
        if folder.parent_id.as_deref() == parent_id {
            return Ok(was);
        }

        lineage(connection, &folder.space_id)?.check_move(id, parent_id)?;
        ensure_unique_name(
            connection,
            vault,
            &folder.space_id,
            parent_id,
            &folder.name,
            Some(id),
        )?;
        place(connection, id, parent_id, None)?;

        Ok(was)
    })
}

/// The undo of [`move_to`]: back under the parent it left, in the frame it had. A parent
/// deleted since, or one it now holds, leaves it where it is.
pub fn move_back(connection: &mut Library, was: &FolderMove) -> Result<bool, StorageError> {
    connection.transaction(|connection, vault| {
        let folder = find(connection, vault, &was.folder_id)?;
        let lineage = lineage(connection, &folder.space_id)?;
        let parent = was.parent_id.as_deref();
        if lineage.check_move(&folder.id, parent).is_err()
            || ensure_unique_name(
                connection,
                vault,
                &folder.space_id,
                parent,
                &folder.name,
                Some(&folder.id),
            )
            .is_err()
        {
            return Ok(false);
        }

        place(connection, &folder.id, parent, was.frame)?;
        Ok(true)
    })
}

/// A frame is a place on one parent's board: it never follows the folder to another.
fn place(
    connection: &mut SqliteConnection,
    id: &str,
    parent_id: Option<&str>,
    frame: Option<crate::folders::board::BoardFrame>,
) -> Result<(), StorageError> {
    diesel::update(folders::table.find(id))
        .set((
            folders::parent_id.eq(parent_id),
            folders::x.eq(frame.map(|frame| frame.x)),
            folders::y.eq(frame.map(|frame| frame.y)),
            folders::w.eq(frame.map(|frame| frame.width)),
            folders::h.eq(frame.map(|frame| frame.height)),
        ))
        .execute(connection)?;

    Ok(())
}

/// What it held goes to its parent — its notes, filed there, and its sub-folders, each to a
/// free slot on that board — or loose at the top of the space. `updated_at` is not touched:
/// the user aimed at the folder.
pub fn delete(connection: &mut Library, id: &str) -> Result<(), StorageError> {
    connection.transaction(|connection, _vault| {
        let lineage = lineage_of(connection, id)?;
        let parent = lineage.parent(id).map(str::to_string);

        for child in lineage.children(Some(id)) {
            place(connection, child, parent.as_deref(), None)?;
        }

        let held: Vec<String> = notes::table
            .filter(notes::folder_id.eq(id))
            .select(notes::id)
            .load::<String>(connection)?;
        // A place on the deleted folder's board means nothing on its parent's.
        board::forget_positions(connection, &held)?;
        diesel::update(notes::table.filter(notes::folder_id.eq(id)))
            .set(notes::folder_id.eq(parent.as_deref()))
            .execute(connection)?;

        diesel::delete(folders::table.find(id)).execute(connection)?;

        if let Some(parent) = &parent {
            let after = lineage_of(connection, parent)?;
            grow_with_ancestors(connection, &after, parent)?;
        }
        Ok(())
    })
}

/// Every zone that shows a folder's notes grows to fit them: its own, and its ancestors',
/// which gather what is under them.
fn grow_with_ancestors(
    connection: &mut SqliteConnection,
    lineage: &Lineage,
    folder_id: &str,
) -> Result<(), StorageError> {
    for zone in lineage.path(folder_id) {
        let held = count_filed(connection, &lineage.subtree(&zone))?;
        let nested = !lineage.children(Some(&zone)).is_empty();
        board::grow_to_fit(connection, &zone, held, nested)?;
    }
    Ok(())
}

/// Answers where each note was filed, for the undo; `None` unfiles. Only into a folder of
/// the note's own space: a cross-space filing would show a chip the space switcher cannot
/// reach.
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

        // Read before the update, which erases where each note came from.
        let filed: Vec<NoteFiling> = query
            .select((notes::id, notes::folder_id))
            .load::<(String, Option<String>)>(connection)?
            .into_iter()
            .filter(|(_, current)| current.as_deref() != folder_id)
            .map(|(note_id, folder_id)| NoteFiling { note_id, folder_id })
            .collect();

        let touched: Vec<String> = filed.iter().map(|filing| filing.note_id.clone()).collect();
        // A place is on the board of the note's own folder: it means nothing on another's.
        board::forget_positions(connection, &touched)?;

        diesel::update(notes::table.filter(notes::id.eq_any(&touched)))
            .set((
                notes::folder_id.eq(folder_id),
                notes::updated_at.eq(iso8601::format(now)),
            ))
            .execute(connection)?;

        // The gesture that filled the zone is the one that should make room in it.
        if let Some(folder_id) = folder_id {
            let lineage = lineage_of(connection, folder_id)?;
            grow_with_ancestors(connection, &lineage, folder_id)?;
        }

        Ok(filed)
    })
}

/// How many live notes these folders hold, which is what a zone has to be tall enough for.
fn count_filed(
    connection: &mut SqliteConnection,
    folder_ids: &[String],
) -> Result<usize, StorageError> {
    let count: i64 = notes::table
        .filter(notes::folder_id.eq_any(folder_ids))
        .filter(notes::deleted_at.is_null())
        .count()
        .get_result(connection)?;

    Ok(usize::try_from(count).unwrap_or(0))
}

/// Puts filed notes back where they were, leaving `updated_at` alone: undoing is not editing.
pub fn restore_filings(
    connection: &mut Library,
    filings: &[NoteFiling],
) -> Result<usize, StorageError> {
    if filings.is_empty() {
        return Ok(0);
    }

    connection.transaction(|connection, _vault| {
        let mut restored = 0;
        let ids: Vec<String> = filings
            .iter()
            .map(|filing| filing.note_id.clone())
            .collect();
        board::forget_positions(connection, &ids)?;

        for filing in filings {
            // A folder deleted since is not an error: that note simply stays loose.
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
            let lineage = lineage_of(connection, folder_id)?;
            grow_with_ancestors(connection, &lineage, folder_id)?;
        }

        Ok(restored)
    })
}
