//! The side tables a note owns. All three are keyed on `note_id` and rewritten whole
//! rather than patched — the row set *is* the value.

use std::collections::{BTreeMap, HashMap};

use diesel::prelude::*;

use super::notes_of_space;
use crate::db::schema::{note_items, note_placeholders, note_tags, notes};
use crate::error::StorageError;
use crate::notes::checklist::ChecklistItem;
use crate::notes::model::Note;
use crate::vault::key::Vault;

/// Up to this many notes, their side tables are read by id; past it, by their space.
const BIND_AT_MOST: usize = 500;

/// Which notes a side-table read covers.
///
/// ⚠️ Bound ids for a few, the space's subquery for many: one parameter per note measured
/// slower than reading the table whole past a few thousand notes — but a filter that cut the
/// query to a hundred notes has no business paying for the space they sit in. Reading a
/// superset is harmless: `attach_related` only looks up the notes it holds.
enum Scope<'a> {
    Notes(&'a [String]),
    Space(Option<&'a str>),
}

impl<'a> Scope<'a> {
    fn of(ids: &'a [String], space_id: Option<&'a str>) -> Self {
        if ids.len() <= BIND_AT_MOST {
            Self::Notes(ids)
        } else {
            Self::Space(space_id)
        }
    }
}

fn all_tags(
    connection: &mut SqliteConnection,
    scope: &Scope<'_>,
) -> Result<HashMap<String, Vec<String>>, StorageError> {
    let mut query = note_tags::table
        .select((note_tags::note_id, note_tags::tag))
        .order(note_tags::tag.asc())
        .into_boxed();

    match scope {
        Scope::Notes(ids) => query = query.filter(note_tags::note_id.eq_any(*ids)),
        Scope::Space(Some(space_id)) => {
            query = query.filter(note_tags::note_id.eq_any(notes_of_space(space_id)));
        }
        Scope::Space(None) => {}
    }

    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    for (note_id, tag) in query.load::<(String, String)>(connection)? {
        grouped.entry(note_id).or_default().push(tag);
    }

    Ok(grouped)
}

/// The tags of the notes in the trash, and of no other note.
pub fn trashed_tags(
    connection: &mut SqliteConnection,
) -> Result<HashMap<String, Vec<String>>, StorageError> {
    let trashed = notes::table
        .filter(notes::deleted_at.is_not_null())
        .select(notes::id);
    let rows = note_tags::table
        .filter(note_tags::note_id.eq_any(trashed))
        .select((note_tags::note_id, note_tags::tag))
        .order(note_tags::tag.asc())
        .load::<(String, String)>(connection)?;

    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    for (note_id, tag) in rows {
        grouped.entry(note_id).or_default().push(tag);
    }

    Ok(grouped)
}

pub fn tags_of(
    connection: &mut SqliteConnection,
    note_id: &str,
) -> Result<Vec<String>, StorageError> {
    Ok(note_tags::table
        .filter(note_tags::note_id.eq(note_id))
        .select(note_tags::tag)
        .order(note_tags::tag.asc())
        .load::<String>(connection)?)
}

/// ⚠️ Re-read rather than sorted: `note_tags.tag` is `COLLATE NOCASE` and a read orders
/// in that collation, which a byte-wise `sort()` does not reproduce.
pub fn replace_tags(
    connection: &mut SqliteConnection,
    note_id: &str,
    tags: &[String],
) -> Result<Vec<String>, StorageError> {
    diesel::delete(note_tags::table.filter(note_tags::note_id.eq(note_id))).execute(connection)?;

    if !tags.is_empty() {
        let rows: Vec<_> = tags
            .iter()
            .map(|tag| (note_tags::note_id.eq(note_id), note_tags::tag.eq(tag)))
            .collect();
        diesel::insert_or_ignore_into(note_tags::table)
            .values(rows)
            .execute(connection)?;
    }

    tags_of(connection, note_id)
}

fn all_items(
    connection: &mut SqliteConnection,
    vault: &Vault,
    scope: &Scope<'_>,
) -> Result<HashMap<String, Vec<ChecklistItem>>, StorageError> {
    let mut query = note_items::table
        .select((note_items::note_id, note_items::text, note_items::done))
        .order((note_items::note_id.asc(), note_items::position.asc()))
        .into_boxed();

    match scope {
        Scope::Notes(ids) => query = query.filter(note_items::note_id.eq_any(*ids)),
        Scope::Space(Some(space_id)) => {
            query = query.filter(note_items::note_id.eq_any(notes_of_space(space_id)));
        }
        Scope::Space(None) => {}
    }

    let mut grouped: HashMap<String, Vec<ChecklistItem>> = HashMap::new();
    for (note_id, text, done) in query.load::<(String, String, bool)>(connection)? {
        grouped.entry(note_id).or_default().push(ChecklistItem {
            text: vault.open(&text)?,
            done,
        });
    }

    Ok(grouped)
}

pub fn items_of(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
) -> Result<Vec<ChecklistItem>, StorageError> {
    note_items::table
        .filter(note_items::note_id.eq(note_id))
        .select((note_items::text, note_items::done))
        .order(note_items::position.asc())
        .load::<(String, bool)>(connection)?
        .into_iter()
        .map(|(text, done)| {
            Ok(ChecklistItem {
                text: vault.open(&text)?,
                done,
            })
        })
        .collect()
}

/// Wiped then reinserted: the position is part of the key, so reordering would otherwise
/// move rows one at a time under a key that refuses duplicates.
pub fn replace_items(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
    items: &[ChecklistItem],
) -> Result<(), StorageError> {
    diesel::delete(note_items::table.filter(note_items::note_id.eq(note_id)))
        .execute(connection)?;

    if !items.is_empty() {
        let rows: Vec<_> = items
            .iter()
            .enumerate()
            .map(|(position, item)| {
                Ok((
                    note_items::note_id.eq(note_id),
                    note_items::position.eq(i32::try_from(position).unwrap_or(i32::MAX)),
                    note_items::text.eq(vault.seal(&item.text)?),
                    note_items::done.eq(item.done),
                ))
            })
            .collect::<Result<Vec<_>, StorageError>>()?;
        diesel::insert_into(note_items::table)
            .values(rows)
            .execute(connection)?;
    }

    Ok(())
}
pub fn attach_related(
    connection: &mut SqliteConnection,
    vault: &Vault,
    notes: &mut [Note],
    space_id: Option<&str>,
) -> Result<(), StorageError> {
    if notes.is_empty() {
        return Ok(());
    }

    let ids: Vec<String> = notes.iter().map(|note| note.id.clone()).collect();
    let scope = Scope::of(&ids, space_id);
    let mut tags = all_tags(connection, &scope)?;
    let mut items = all_items(connection, vault, &scope)?;
    let mut values = all_placeholder_values(connection, vault, &scope)?;

    for note in notes {
        note.tags = tags.remove(&note.id).unwrap_or_default();
        note.items = items.remove(&note.id).unwrap_or_default();
        note.placeholder_values = values.remove(&note.id).unwrap_or_default();
    }

    Ok(())
}

fn all_placeholder_values(
    connection: &mut SqliteConnection,
    vault: &Vault,
    scope: &Scope<'_>,
) -> Result<HashMap<String, BTreeMap<String, String>>, StorageError> {
    let mut query = note_placeholders::table
        .select((
            note_placeholders::note_id,
            note_placeholders::name,
            note_placeholders::value,
        ))
        .into_boxed();

    match scope {
        Scope::Notes(ids) => query = query.filter(note_placeholders::note_id.eq_any(*ids)),
        Scope::Space(Some(space_id)) => {
            query = query.filter(note_placeholders::note_id.eq_any(notes_of_space(space_id)));
        }
        Scope::Space(None) => {}
    }

    let mut grouped: HashMap<String, BTreeMap<String, String>> = HashMap::new();
    for (note_id, name, value) in query.load::<(String, String, String)>(connection)? {
        grouped
            .entry(note_id)
            .or_default()
            .insert(name, vault.open(&value)?);
    }

    Ok(grouped)
}

pub fn placeholder_values_of(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
) -> Result<BTreeMap<String, String>, StorageError> {
    note_placeholders::table
        .filter(note_placeholders::note_id.eq(note_id))
        .select((note_placeholders::name, note_placeholders::value))
        .load::<(String, String)>(connection)?
        .into_iter()
        .map(|(name, value)| Ok((name, vault.open(&value)?)))
        .collect::<Result<BTreeMap<_, _>, StorageError>>()
}

pub fn replace_placeholder_values(
    connection: &mut SqliteConnection,
    vault: &Vault,
    note_id: &str,
    values: &BTreeMap<String, String>,
) -> Result<(), StorageError> {
    diesel::delete(note_placeholders::table.filter(note_placeholders::note_id.eq(note_id)))
        .execute(connection)?;

    if !values.is_empty() {
        let rows: Vec<_> = values
            .iter()
            .map(|(name, value)| {
                Ok((
                    note_placeholders::note_id.eq(note_id),
                    note_placeholders::name.eq(name),
                    note_placeholders::value.eq(vault.seal(value)?),
                ))
            })
            .collect::<Result<Vec<_>, StorageError>>()?;
        diesel::insert_into(note_placeholders::table)
            .values(rows)
            .execute(connection)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(count: usize) -> Vec<String> {
        (0..count).map(|n| format!("n-{n}")).collect()
    }

    #[test]
    fn a_few_notes_are_read_by_their_ids() {
        let few = ids(BIND_AT_MOST);

        assert!(
            matches!(Scope::of(&few, Some("s-1")), Scope::Notes(bound) if bound.len() == BIND_AT_MOST)
        );
    }

    #[test]
    fn many_notes_are_read_by_their_space() {
        let many = ids(BIND_AT_MOST + 1);

        assert!(matches!(
            Scope::of(&many, Some("s-1")),
            Scope::Space(Some("s-1"))
        ));
        assert!(matches!(Scope::of(&many, None), Scope::Space(None)));
    }
}
