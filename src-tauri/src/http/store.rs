//! The collections' SQL. Every name, a request's document and a container's settings are
//! sealed; the parents, positions, kind and method stay in the clear, so SQL orders and
//! joins on them. Folders and requests under one parent share one order.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use super::model::{
    ContainerSettings, HttpCollection, HttpCollectionNode, HttpContents, HttpFolder, HttpItem,
    HttpItemKind, HttpNode, HttpPlace, HttpRequest, HttpRequestDraft, HttpRequestPatch,
    HttpRequestSummary, HttpTree,
};
use super::settings::{HttpOrigin, Inherited};
use crate::db::schema::{http_collections, http_folders, http_requests};
use crate::db::{Library, iso8601};
use crate::error::{StorageError, ValidationError};
use crate::vault::key::Vault;

#[derive(Queryable, Selectable)]
#[diesel(table_name = http_collections, check_for_backend(diesel::sqlite::Sqlite))]
struct CollectionRow {
    id: String,
    name: String,
    settings: String,
    position: i32,
    created_at: String,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = http_folders, check_for_backend(diesel::sqlite::Sqlite))]
struct FolderRow {
    id: String,
    collection_id: String,
    parent_id: Option<String>,
    name: String,
    settings: String,
    position: i32,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = http_requests, check_for_backend(diesel::sqlite::Sqlite))]
struct RequestRow {
    id: String,
    collection_id: String,
    folder_id: Option<String>,
    name: String,
    kind: String,
    method: String,
    document: String,
    position: i32,
    created_at: String,
    updated_at: String,
}

fn rank(position: i32) -> u32 {
    u32::try_from(position).unwrap_or(0)
}

fn instant(id: &str, field: &'static str, text: &str) -> Result<DateTime<Utc>, StorageError> {
    iso8601::parse(text).map_err(|_| StorageError::CorruptRow {
        id: id.to_string(),
        field,
    })
}

impl CollectionRow {
    fn open(self, vault: &Vault) -> Result<HttpCollection, StorageError> {
        Ok(HttpCollection {
            created_at: instant(&self.id, "createdAt", &self.created_at)?,
            name: vault.open(&self.name)?,
            position: rank(self.position),
            id: self.id,
        })
    }
}

impl FolderRow {
    fn open(self, vault: &Vault) -> Result<HttpFolder, StorageError> {
        Ok(HttpFolder {
            name: vault.open(&self.name)?,
            position: rank(self.position),
            id: self.id,
            collection_id: self.collection_id,
            parent_id: self.parent_id,
        })
    }
}

impl RequestRow {
    /// An unknown kind or method, written by a newer version, degrades to the default rather
    /// than hiding the request.
    fn summary(&self, vault: &Vault) -> Result<HttpRequestSummary, StorageError> {
        Ok(HttpRequestSummary {
            id: self.id.clone(),
            collection_id: self.collection_id.clone(),
            folder_id: self.folder_id.clone(),
            name: vault.open(&self.name)?,
            kind: self.kind.parse().unwrap_or_default(),
            method: self.method.parse().unwrap_or_default(),
            position: rank(self.position),
        })
    }

    fn open(self, vault: &Vault) -> Result<HttpRequest, StorageError> {
        let summary = self.summary(vault)?;
        let document = serde_json::from_str(&vault.open(&self.document)?).map_err(|_| {
            StorageError::CorruptRow {
                id: self.id.clone(),
                field: "document",
            }
        })?;
        Ok(HttpRequest {
            created_at: instant(&self.id, "createdAt", &self.created_at)?,
            updated_at: instant(&self.id, "updatedAt", &self.updated_at)?,
            id: summary.id,
            collection_id: summary.collection_id,
            folder_id: summary.folder_id,
            name: summary.name,
            kind: summary.kind,
            method: summary.method,
            document,
        })
    }
}

fn not_found(id: &str) -> StorageError {
    StorageError::HttpItemNotFound(id.to_string())
}

fn seal_json<T: serde::Serialize>(vault: &Vault, value: &T) -> Result<String, StorageError> {
    let json = serde_json::to_string(value)
        .map_err(|error| StorageError::Vault(format!("unwritable document: {error}")))?;
    vault.seal(&json)
}

fn find_collection(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<CollectionRow, StorageError> {
    http_collections::table
        .find(id)
        .select(CollectionRow::as_select())
        .first(connection)
        .optional()?
        .ok_or_else(|| not_found(id))
}

fn find_folder(connection: &mut SqliteConnection, id: &str) -> Result<FolderRow, StorageError> {
    http_folders::table
        .find(id)
        .select(FolderRow::as_select())
        .first(connection)
        .optional()?
        .ok_or_else(|| not_found(id))
}

fn find_request(connection: &mut SqliteConnection, id: &str) -> Result<RequestRow, StorageError> {
    http_requests::table
        .find(id)
        .select(RequestRow::as_select())
        .first(connection)
        .optional()?
        .ok_or_else(|| not_found(id))
}

/// A folder of `collection_id`, or a refusal naming the field: a request filed in another
/// collection's folder would sit in two places at once.
fn folder_of_collection(
    connection: &mut SqliteConnection,
    collection_id: &str,
    folder_id: Option<&str>,
) -> Result<(), StorageError> {
    find_collection(connection, collection_id)?;
    if let Some(folder_id) = folder_id
        && find_folder(connection, folder_id)?.collection_id != collection_id
    {
        return Err(
            ValidationError::new("folderId", "the folder belongs to another collection").into(),
        );
    }
    Ok(())
}

pub fn tree(connection: &mut Library) -> Result<HttpTree, StorageError> {
    let (connection, vault) = connection.split();

    let collections = http_collections::table
        .select(CollectionRow::as_select())
        .order((http_collections::position.asc(), http_collections::id.asc()))
        .load(connection)?;
    let folders = http_folders::table
        .select(FolderRow::as_select())
        .load(connection)?;
    let requests = http_requests::table
        .select(RequestRow::as_select())
        .load(connection)?;

    let mut nodes: Branches = HashMap::new();
    for row in folders {
        let folder = row.open(vault)?;
        nodes
            .entry((folder.collection_id.clone(), folder.parent_id.clone()))
            .or_default()
            .push((
                folder.position,
                0,
                folder.id.clone(),
                Branch::Folder(folder),
            ));
    }
    for row in &requests {
        let request = row.summary(vault)?;
        nodes
            .entry((request.collection_id.clone(), request.folder_id.clone()))
            .or_default()
            .push((
                request.position,
                1,
                request.id.clone(),
                Branch::Request(request),
            ));
    }
    for siblings in nodes.values_mut() {
        siblings.sort_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)));
    }

    let collections = collections
        .into_iter()
        .map(|row| {
            let collection = row.open(vault)?;
            let children = grow(&mut nodes, &collection.id, None);
            Ok(HttpCollectionNode {
                collection,
                children,
            })
        })
        .collect::<Result<_, StorageError>>()?;

    Ok(HttpTree { collections })
}

/// Under one parent, keyed by collection and folder, with what orders them.
type Branches = HashMap<(String, Option<String>), Vec<(u32, u8, String, Branch)>>;

enum Branch {
    Folder(HttpFolder),
    Request(HttpRequestSummary),
}

fn grow(nodes: &mut Branches, collection_id: &str, folder_id: Option<&str>) -> Vec<HttpNode> {
    let siblings = nodes
        .remove(&(collection_id.to_string(), folder_id.map(str::to_string)))
        .unwrap_or_default();
    siblings
        .into_iter()
        .map(|(_, _, id, branch)| match branch {
            Branch::Folder(folder) => HttpNode::Folder {
                children: grow(nodes, collection_id, Some(&id)),
                folder,
            },
            Branch::Request(request) => HttpNode::Request { request },
        })
        .collect()
}

/// The folders and requests under one parent, in their order.
fn siblings(
    connection: &mut SqliteConnection,
    collection_id: &str,
    folder_id: Option<&str>,
) -> Result<Vec<HttpItem>, StorageError> {
    let mut folders = http_folders::table
        .filter(http_folders::collection_id.eq(collection_id))
        .select((http_folders::id, http_folders::position))
        .into_boxed();
    let mut requests = http_requests::table
        .filter(http_requests::collection_id.eq(collection_id))
        .select((http_requests::id, http_requests::position))
        .into_boxed();
    if let Some(folder_id) = folder_id {
        folders = folders.filter(http_folders::parent_id.eq(folder_id.to_string()));
        requests = requests.filter(http_requests::folder_id.eq(folder_id.to_string()));
    } else {
        folders = folders.filter(http_folders::parent_id.is_null());
        requests = requests.filter(http_requests::folder_id.is_null());
    }

    let mut ranked: Vec<(i32, u8, String)> = folders
        .load::<(String, i32)>(connection)?
        .into_iter()
        .map(|(id, position)| (position, 0, id))
        .chain(
            requests
                .load::<(String, i32)>(connection)?
                .into_iter()
                .map(|(id, position)| (position, 1, id)),
        )
        .collect();
    ranked.sort();

    Ok(ranked
        .into_iter()
        .map(|(_, kind, id)| HttpItem {
            kind: if kind == 0 {
                HttpItemKind::Folder
            } else {
                HttpItemKind::Request
            },
            id,
        })
        .collect())
}

fn renumber(connection: &mut SqliteConnection, items: &[HttpItem]) -> Result<(), StorageError> {
    for (position, item) in items.iter().enumerate() {
        let position = i32::try_from(position).unwrap_or(i32::MAX);
        match item.kind {
            HttpItemKind::Folder => {
                diesel::update(http_folders::table.find(&item.id))
                    .set(http_folders::position.eq(position))
                    .execute(connection)?;
            }
            HttpItemKind::Request => {
                diesel::update(http_requests::table.find(&item.id))
                    .set(http_requests::position.eq(position))
                    .execute(connection)?;
            }
            HttpItemKind::Collection => {
                diesel::update(http_collections::table.find(&item.id))
                    .set(http_collections::position.eq(position))
                    .execute(connection)?;
            }
        }
    }
    Ok(())
}

/// Puts `item` at `index` among the others: the end when the index is past them.
fn insert_at(
    connection: &mut SqliteConnection,
    mut others: Vec<HttpItem>,
    item: HttpItem,
    index: usize,
) -> Result<(), StorageError> {
    others.retain(|other| other != &item);
    let index = index.min(others.len());
    others.insert(index, item);
    renumber(connection, &others)
}

fn collections_in_order(connection: &mut SqliteConnection) -> Result<Vec<HttpItem>, StorageError> {
    Ok(http_collections::table
        .select(http_collections::id)
        .order((http_collections::position.asc(), http_collections::id.asc()))
        .load::<String>(connection)?
        .into_iter()
        .map(|id| HttpItem {
            kind: HttpItemKind::Collection,
            id,
        })
        .collect())
}

pub fn create_collection(
    connection: &mut Library,
    name: &str,
    now: DateTime<Utc>,
) -> Result<HttpCollection, StorageError> {
    connection.transaction(|connection, vault| {
        let order = collections_in_order(connection)?;
        let collection = HttpCollection {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            position: u32::try_from(order.len()).unwrap_or(u32::MAX),
            created_at: now,
        };
        diesel::insert_into(http_collections::table)
            .values((
                http_collections::id.eq(&collection.id),
                http_collections::name.eq(vault.seal(name)?),
                http_collections::settings.eq(seal_json(vault, &ContainerSettings::default())?),
                http_collections::position.eq(i32::try_from(order.len()).unwrap_or(i32::MAX)),
                http_collections::created_at.eq(iso8601::format(now)),
            ))
            .execute(connection)?;
        Ok(collection)
    })
}

pub fn create_folder(
    connection: &mut Library,
    collection_id: &str,
    parent_id: Option<&str>,
    name: &str,
    now: DateTime<Utc>,
) -> Result<HttpFolder, StorageError> {
    connection.transaction(|connection, vault| {
        folder_of_collection(connection, collection_id, parent_id)?;
        let others = siblings(connection, collection_id, parent_id)?;
        let folder = HttpFolder {
            id: Uuid::new_v4().to_string(),
            collection_id: collection_id.to_string(),
            parent_id: parent_id.map(str::to_string),
            name: name.to_string(),
            position: u32::try_from(others.len()).unwrap_or(u32::MAX),
        };
        diesel::insert_into(http_folders::table)
            .values((
                http_folders::id.eq(&folder.id),
                http_folders::collection_id.eq(collection_id),
                http_folders::parent_id.eq(parent_id),
                http_folders::name.eq(vault.seal(name)?),
                http_folders::settings.eq(seal_json(vault, &ContainerSettings::default())?),
                http_folders::position.eq(i32::MAX),
                http_folders::created_at.eq(iso8601::format(now)),
            ))
            .execute(connection)?;
        insert_at(
            connection,
            others,
            HttpItem {
                kind: HttpItemKind::Folder,
                id: folder.id.clone(),
            },
            usize::MAX,
        )?;
        Ok(folder)
    })
}

pub fn create_request(
    connection: &mut Library,
    draft: &HttpRequestDraft,
    name: &str,
    now: DateTime<Utc>,
) -> Result<HttpRequest, StorageError> {
    connection.transaction(|connection, vault| {
        let folder_id = draft.folder_id.as_deref();
        folder_of_collection(connection, &draft.collection_id, folder_id)?;
        let others = siblings(connection, &draft.collection_id, folder_id)?;
        let id = Uuid::new_v4().to_string();
        diesel::insert_into(http_requests::table)
            .values((
                http_requests::id.eq(&id),
                http_requests::collection_id.eq(&draft.collection_id),
                http_requests::folder_id.eq(folder_id),
                http_requests::name.eq(vault.seal(name)?),
                http_requests::kind.eq(draft.kind.as_str()),
                http_requests::method.eq(draft.method.as_str()),
                http_requests::document.eq(seal_json(vault, &draft.document)?),
                http_requests::position.eq(i32::MAX),
                http_requests::created_at.eq(iso8601::format(now)),
                http_requests::updated_at.eq(iso8601::format(now)),
            ))
            .execute(connection)?;
        insert_at(
            connection,
            others,
            HttpItem {
                kind: HttpItemKind::Request,
                id: id.clone(),
            },
            usize::MAX,
        )?;
        find_request(connection, &id)?.open(vault)
    })
}

pub fn get_request(connection: &mut Library, id: &str) -> Result<HttpRequest, StorageError> {
    let (connection, vault) = connection.split();
    find_request(connection, id)?.open(vault)
}

pub fn save_request(
    connection: &mut Library,
    id: &str,
    patch: &HttpRequestPatch,
    name: Option<&str>,
    now: DateTime<Utc>,
) -> Result<HttpRequest, StorageError> {
    connection.transaction(|connection, vault| {
        find_request(connection, id)?;
        if let Some(name) = name {
            diesel::update(http_requests::table.find(id))
                .set(http_requests::name.eq(vault.seal(name)?))
                .execute(connection)?;
        }
        if let Some(kind) = patch.kind {
            diesel::update(http_requests::table.find(id))
                .set(http_requests::kind.eq(kind.as_str()))
                .execute(connection)?;
        }
        if let Some(method) = patch.method {
            diesel::update(http_requests::table.find(id))
                .set(http_requests::method.eq(method.as_str()))
                .execute(connection)?;
        }
        if let Some(document) = &patch.document {
            diesel::update(http_requests::table.find(id))
                .set(http_requests::document.eq(seal_json(vault, document)?))
                .execute(connection)?;
        }
        diesel::update(http_requests::table.find(id))
            .set(http_requests::updated_at.eq(iso8601::format(now)))
            .execute(connection)?;
        find_request(connection, id)?.open(vault)
    })
}

pub fn rename(connection: &mut Library, item: &HttpItem, name: &str) -> Result<(), StorageError> {
    let (connection, vault) = connection.split();
    let sealed = vault.seal(name)?;
    let changed = match item.kind {
        HttpItemKind::Collection => diesel::update(http_collections::table.find(&item.id))
            .set(http_collections::name.eq(sealed))
            .execute(connection)?,
        HttpItemKind::Folder => diesel::update(http_folders::table.find(&item.id))
            .set(http_folders::name.eq(sealed))
            .execute(connection)?,
        HttpItemKind::Request => diesel::update(http_requests::table.find(&item.id))
            .set(http_requests::name.eq(sealed))
            .execute(connection)?,
    };
    if changed == 0 {
        return Err(not_found(&item.id));
    }
    Ok(())
}

/// What goes with it is `ON DELETE CASCADE`: a collection's folders and requests, a folder's
/// folders and theirs. `PRAGMA foreign_keys` is what makes that true (`db::configure`).
pub fn delete(connection: &mut Library, item: &HttpItem) -> Result<(), StorageError> {
    let connection = connection.db();
    let deleted = match item.kind {
        HttpItemKind::Collection => {
            diesel::delete(http_collections::table.find(&item.id)).execute(connection)?
        }
        HttpItemKind::Folder => {
            diesel::delete(http_folders::table.find(&item.id)).execute(connection)?
        }
        HttpItemKind::Request => {
            diesel::delete(http_requests::table.find(&item.id)).execute(connection)?
        }
    };
    if deleted == 0 {
        return Err(not_found(&item.id));
    }
    Ok(())
}

/// Every folder below `folder_id`, at any depth, not counting it.
fn descendants(
    connection: &mut SqliteConnection,
    folder_id: &str,
) -> Result<Vec<String>, StorageError> {
    let mut found = Vec::new();
    let mut frontier = vec![folder_id.to_string()];
    while !frontier.is_empty() {
        let next: Vec<String> = http_folders::table
            .filter(http_folders::parent_id.eq_any(&frontier))
            .select(http_folders::id)
            .load(connection)?;
        found.extend(next.iter().cloned());
        frontier = next;
    }
    Ok(found)
}

fn count(rows: i64) -> u32 {
    u32::try_from(rows).unwrap_or(u32::MAX)
}

pub fn contents(connection: &mut Library, item: &HttpItem) -> Result<HttpContents, StorageError> {
    let connection = connection.db();
    match item.kind {
        HttpItemKind::Collection => {
            find_collection(connection, &item.id)?;
            let folders: i64 = http_folders::table
                .filter(http_folders::collection_id.eq(&item.id))
                .count()
                .get_result(connection)?;
            let requests: i64 = http_requests::table
                .filter(http_requests::collection_id.eq(&item.id))
                .count()
                .get_result(connection)?;
            Ok(HttpContents {
                folders: count(folders),
                requests: count(requests),
            })
        }
        HttpItemKind::Folder => {
            find_folder(connection, &item.id)?;
            let below = descendants(connection, &item.id)?;
            let mut held = below.clone();
            held.push(item.id.clone());
            let requests: i64 = http_requests::table
                .filter(http_requests::folder_id.eq_any(&held))
                .count()
                .get_result(connection)?;
            Ok(HttpContents {
                folders: u32::try_from(below.len()).unwrap_or(u32::MAX),
                requests: count(requests),
            })
        }
        HttpItemKind::Request => {
            find_request(connection, &item.id)?;
            Ok(HttpContents::default())
        }
    }
}

/// Moves a folder or a request under another parent, or to another rank under its own. A
/// folder takes everything below it, into another collection if need be.
pub fn move_item(
    connection: &mut Library,
    item: &HttpItem,
    place: &HttpPlace,
) -> Result<(), StorageError> {
    connection.transaction(|connection, _| {
        let target_folder = place.folder_id.as_deref();
        folder_of_collection(connection, &place.collection_id, target_folder)?;
        let index = usize::try_from(place.index).unwrap_or(usize::MAX);

        let (from_collection, from_folder) = match item.kind {
            HttpItemKind::Collection => {
                return Err(
                    ValidationError::new("kind", "a collection is reordered, not moved").into(),
                );
            }
            HttpItemKind::Folder => {
                let folder = find_folder(connection, &item.id)?;
                let below = descendants(connection, &item.id)?;
                if target_folder
                    .is_some_and(|target| target == item.id || below.iter().any(|id| id == target))
                {
                    return Err(ValidationError::new(
                        "folderId",
                        "a folder cannot move into itself",
                    )
                    .into());
                }
                if folder.collection_id != place.collection_id {
                    let mut subtree = below;
                    subtree.push(item.id.clone());
                    diesel::update(http_folders::table.filter(http_folders::id.eq_any(&subtree)))
                        .set(http_folders::collection_id.eq(&place.collection_id))
                        .execute(connection)?;
                    diesel::update(
                        http_requests::table.filter(http_requests::folder_id.eq_any(&subtree)),
                    )
                    .set(http_requests::collection_id.eq(&place.collection_id))
                    .execute(connection)?;
                }
                diesel::update(http_folders::table.find(&item.id))
                    .set(http_folders::parent_id.eq(target_folder))
                    .execute(connection)?;
                (folder.collection_id, folder.parent_id)
            }
            HttpItemKind::Request => {
                let request = find_request(connection, &item.id)?;
                diesel::update(http_requests::table.find(&item.id))
                    .set((
                        http_requests::collection_id.eq(&place.collection_id),
                        http_requests::folder_id.eq(target_folder),
                    ))
                    .execute(connection)?;
                (request.collection_id, request.folder_id)
            }
        };

        let left = siblings(connection, &from_collection, from_folder.as_deref())?;
        renumber(connection, &left)?;
        let others = siblings(connection, &place.collection_id, target_folder)?;
        insert_at(connection, others, item.clone(), index)
    })
}

pub fn reorder_collection(
    connection: &mut Library,
    id: &str,
    index: u32,
) -> Result<(), StorageError> {
    connection.transaction(|connection, _| {
        find_collection(connection, id)?;
        let order = collections_in_order(connection)?;
        insert_at(
            connection,
            order,
            HttpItem {
                kind: HttpItemKind::Collection,
                id: id.to_string(),
            },
            usize::try_from(index).unwrap_or(usize::MAX),
        )
    })
}

/// A copy named `name`, right after the original; a collection or a folder is copied with
/// everything in it. Answers the copy.
pub fn duplicate(
    connection: &mut Library,
    item: &HttpItem,
    name: &str,
    now: DateTime<Utc>,
) -> Result<HttpItem, StorageError> {
    connection.transaction(|connection, vault| {
        let sealed = vault.seal(name)?;
        let copy = match item.kind {
            HttpItemKind::Collection => {
                let source = find_collection(connection, &item.id)?;
                let id = Uuid::new_v4().to_string();
                diesel::insert_into(http_collections::table)
                    .values((
                        http_collections::id.eq(&id),
                        http_collections::name.eq(&sealed),
                        http_collections::settings.eq(&source.settings),
                        http_collections::position.eq(i32::MAX),
                        http_collections::created_at.eq(iso8601::format(now)),
                    ))
                    .execute(connection)?;
                copy_children(connection, &item.id, None, &id, None, now)?;
                let copy = HttpItem {
                    kind: HttpItemKind::Collection,
                    id,
                };
                let order = collections_in_order(connection)?;
                place_after(connection, order, item, copy.clone())?;
                copy
            }
            HttpItemKind::Folder => {
                let source = find_folder(connection, &item.id)?;
                let renamed = FolderRow {
                    name: sealed,
                    ..source.clone()
                };
                let id = copy_folder(
                    connection,
                    &renamed,
                    &source.collection_id,
                    source.parent_id.as_deref(),
                    now,
                )?;
                let copy = HttpItem {
                    kind: HttpItemKind::Folder,
                    id,
                };
                let others = siblings(
                    connection,
                    &source.collection_id,
                    source.parent_id.as_deref(),
                )?;
                place_after(connection, others, item, copy.clone())?;
                copy
            }
            HttpItemKind::Request => {
                let source = find_request(connection, &item.id)?;
                let renamed = RequestRow {
                    name: sealed,
                    ..source.clone()
                };
                let id = copy_request(
                    connection,
                    &renamed,
                    &source.collection_id,
                    source.folder_id.as_deref(),
                    now,
                )?;
                let copy = HttpItem {
                    kind: HttpItemKind::Request,
                    id,
                };
                let others = siblings(
                    connection,
                    &source.collection_id,
                    source.folder_id.as_deref(),
                )?;
                place_after(connection, others, item, copy.clone())?;
                copy
            }
        };
        Ok(copy)
    })
}

fn place_after(
    connection: &mut SqliteConnection,
    mut order: Vec<HttpItem>,
    original: &HttpItem,
    copy: HttpItem,
) -> Result<(), StorageError> {
    // The copy holds its original's position until now: where it sorts says nothing.
    order.retain(|item| item != &copy);
    let index = order
        .iter()
        .position(|item| item == original)
        .map_or(usize::MAX, |index| index + 1);
    insert_at(connection, order, copy, index)
}

fn copy_request(
    connection: &mut SqliteConnection,
    source: &RequestRow,
    collection_id: &str,
    folder_id: Option<&str>,
    now: DateTime<Utc>,
) -> Result<String, StorageError> {
    let id = Uuid::new_v4().to_string();
    diesel::insert_into(http_requests::table)
        .values((
            http_requests::id.eq(&id),
            http_requests::collection_id.eq(collection_id),
            http_requests::folder_id.eq(folder_id),
            http_requests::name.eq(&source.name),
            http_requests::kind.eq(&source.kind),
            http_requests::method.eq(&source.method),
            http_requests::document.eq(&source.document),
            http_requests::position.eq(source.position),
            http_requests::created_at.eq(iso8601::format(now)),
            http_requests::updated_at.eq(iso8601::format(now)),
        ))
        .execute(connection)?;
    Ok(id)
}

/// Inserts a copy of `source` and of everything below it, parents before their children.
fn copy_folder(
    connection: &mut SqliteConnection,
    source: &FolderRow,
    collection_id: &str,
    parent_id: Option<&str>,
    now: DateTime<Utc>,
) -> Result<String, StorageError> {
    let id = Uuid::new_v4().to_string();
    diesel::insert_into(http_folders::table)
        .values((
            http_folders::id.eq(&id),
            http_folders::collection_id.eq(collection_id),
            http_folders::parent_id.eq(parent_id),
            http_folders::name.eq(&source.name),
            http_folders::settings.eq(&source.settings),
            http_folders::position.eq(source.position),
            http_folders::created_at.eq(iso8601::format(now)),
        ))
        .execute(connection)?;
    copy_children(
        connection,
        &source.collection_id,
        Some(&source.id),
        collection_id,
        Some(&id),
        now,
    )?;
    Ok(id)
}

fn copy_children(
    connection: &mut SqliteConnection,
    from_collection: &str,
    from_folder: Option<&str>,
    to_collection: &str,
    to_folder: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), StorageError> {
    let mut requests = http_requests::table
        .filter(http_requests::collection_id.eq(from_collection))
        .select(RequestRow::as_select())
        .into_boxed();
    let mut folders = http_folders::table
        .filter(http_folders::collection_id.eq(from_collection))
        .select(FolderRow::as_select())
        .into_boxed();
    if let Some(folder) = from_folder {
        requests = requests.filter(http_requests::folder_id.eq(folder.to_string()));
        folders = folders.filter(http_folders::parent_id.eq(folder.to_string()));
    } else {
        requests = requests.filter(http_requests::folder_id.is_null());
        folders = folders.filter(http_folders::parent_id.is_null());
    }
    for request in requests.load(connection)? {
        copy_request(connection, &request, to_collection, to_folder, now)?;
    }
    for folder in folders.load(connection)? {
        copy_folder(connection, &folder, to_collection, to_folder, now)?;
    }
    Ok(())
}

fn open_settings(vault: &Vault, id: &str, sealed: &str) -> Result<ContainerSettings, StorageError> {
    serde_json::from_str(&vault.open(sealed)?).map_err(|_| StorageError::CorruptRow {
        id: id.to_string(),
        field: "settings",
    })
}

/// A collection's or a folder's own settings, what its requests inherit.
pub fn settings(
    connection: &mut Library,
    item: &HttpItem,
) -> Result<ContainerSettings, StorageError> {
    let (connection, vault) = connection.split();
    let sealed = match item.kind {
        HttpItemKind::Collection => find_collection(connection, &item.id)?.settings,
        HttpItemKind::Folder => find_folder(connection, &item.id)?.settings,
        HttpItemKind::Request => {
            return Err(
                ValidationError::new("kind", "a request has no settings of its own").into(),
            );
        }
    };
    open_settings(vault, &item.id, &sealed)
}

pub fn save_settings(
    connection: &mut Library,
    item: &HttpItem,
    settings: &ContainerSettings,
) -> Result<(), StorageError> {
    let (connection, vault) = connection.split();
    let sealed = seal_json(vault, settings)?;
    let changed = match item.kind {
        HttpItemKind::Collection => diesel::update(http_collections::table.find(&item.id))
            .set(http_collections::settings.eq(sealed))
            .execute(connection)?,
        HttpItemKind::Folder => diesel::update(http_folders::table.find(&item.id))
            .set(http_folders::settings.eq(sealed))
            .execute(connection)?,
        HttpItemKind::Request => {
            return Err(
                ValidationError::new("kind", "a request has no settings of its own").into(),
            );
        }
    };
    if changed == 0 {
        return Err(not_found(&item.id));
    }
    Ok(())
}

/// What a request in `folder_id` of `collection_id` inherits: the collection, then each folder
/// down to its own.
pub fn inherited(
    connection: &mut Library,
    collection_id: &str,
    folder_id: Option<&str>,
) -> Result<Inherited, StorageError> {
    let (connection, vault) = connection.split();
    let collection = find_collection(connection, collection_id)?;
    let mut folders = Vec::new();
    let mut next = folder_id.map(str::to_string);
    while let Some(id) = next.take() {
        let folder = find_folder(connection, &id)?;
        next.clone_from(&folder.parent_id);
        folders.push(folder);
    }

    let mut levels = vec![(
        HttpOrigin {
            kind: HttpItemKind::Collection,
            id: collection.id.clone(),
            name: vault.open(&collection.name)?,
        },
        open_settings(vault, &collection.id, &collection.settings)?,
    )];
    for folder in folders.into_iter().rev() {
        levels.push((
            HttpOrigin {
                kind: HttpItemKind::Folder,
                id: folder.id.clone(),
                name: vault.open(&folder.name)?,
            },
            open_settings(vault, &folder.id, &folder.settings)?,
        ));
    }
    Ok(super::settings::inherit(&levels))
}

#[cfg(test)]
mod tests {
    use super::super::model::{HttpMethod, RequestDocument, RequestKind};
    use super::*;

    #[test]
    fn a_position_out_of_range_reads_as_the_first() {
        assert_eq!(rank(-3), 0);
        assert_eq!(rank(7), 7);
    }

    #[test]
    fn an_unknown_kind_or_method_degrades_to_the_default() {
        assert_eq!(
            "TRACE".parse::<HttpMethod>().unwrap_or_default(),
            HttpMethod::Get
        );
        assert_eq!(
            "grpc".parse::<RequestKind>().unwrap_or_default(),
            RequestKind::Http
        );
        assert!(serde_json::from_str::<RequestDocument>("{}").is_ok());
    }
}
