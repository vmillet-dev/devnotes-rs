//! The environments' SQL. A set is sealed whole, so its name is compared in Rust and the order
//! is the one column SQL reads; the globals are the one row of their kind.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use super::{Environment, EnvironmentDocument, EnvironmentPatch, HttpEnvironments, Variable};
use crate::db::schema::http_environments;
use crate::db::{Library, iso8601};
use crate::error::StorageError;
use crate::folders::model::FolderColour;
use crate::vault::key::Vault;

const GLOBALS: &str = "globals";
const ENVIRONMENT: &str = "environment";

#[derive(Queryable, Selectable)]
#[diesel(table_name = http_environments, check_for_backend(diesel::sqlite::Sqlite))]
struct EnvironmentRow {
    id: String,
    kind: String,
    position: i32,
    document: String,
}

impl EnvironmentRow {
    fn document(&self, vault: &Vault) -> Result<EnvironmentDocument, StorageError> {
        serde_json::from_str(&vault.open(&self.document)?).map_err(|_| StorageError::CorruptRow {
            id: self.id.clone(),
            field: "document",
        })
    }

    fn open(self, vault: &Vault) -> Result<Environment, StorageError> {
        let document = self.document(vault)?;
        Ok(Environment {
            name: document.name,
            colour: document.colour,
            protected: document.protected,
            variables: document.variables,
            position: u32::try_from(self.position).unwrap_or(0),
            id: self.id,
        })
    }
}

fn seal(vault: &Vault, document: &EnvironmentDocument) -> Result<String, StorageError> {
    let json = serde_json::to_string(document)
        .map_err(|error| StorageError::Vault(format!("unwritable environment: {error}")))?;
    vault.seal(&json)
}

fn rows(connection: &mut SqliteConnection) -> Result<Vec<EnvironmentRow>, StorageError> {
    Ok(http_environments::table
        .select(EnvironmentRow::as_select())
        .order((http_environments::position, http_environments::created_at))
        .load(connection)?)
}

fn environments(
    connection: &mut SqliteConnection,
    vault: &Vault,
) -> Result<Vec<Environment>, StorageError> {
    rows(connection)?
        .into_iter()
        .filter(|row| row.kind == ENVIRONMENT)
        .map(|row| row.open(vault))
        .collect()
}

fn find(connection: &mut SqliteConnection, id: &str) -> Result<EnvironmentRow, StorageError> {
    http_environments::table
        .find(id)
        .filter(http_environments::kind.eq(ENVIRONMENT))
        .select(EnvironmentRow::as_select())
        .first(connection)
        .optional()?
        .ok_or_else(|| StorageError::HttpEnvironmentNotFound(id.to_string()))
}

/// Decided here, not by a unique index: the names are sealed.
fn refuse_taken(
    held: &[Environment],
    name: &str,
    except_id: Option<&str>,
) -> Result<(), StorageError> {
    let pairs = held
        .iter()
        .map(|environment| (environment.id.as_str(), environment.name.as_str()));
    if crate::name::is_taken(pairs, name, except_id) {
        return Err(StorageError::DuplicateEnvironmentName(name.to_string()));
    }
    Ok(())
}

pub fn list(connection: &mut Library) -> Result<HttpEnvironments, StorageError> {
    let (connection, vault) = connection.split();
    let mut globals = Vec::new();
    let mut environments = Vec::new();
    for row in rows(connection)? {
        if row.kind == GLOBALS {
            globals = row.document(vault)?.variables;
        } else if row.kind == ENVIRONMENT {
            environments.push(row.open(vault)?);
        }
    }
    Ok(HttpEnvironments {
        globals,
        environments,
    })
}

/// One environment, for the send that resolves from it.
pub fn get(connection: &mut Library, id: &str) -> Result<Environment, StorageError> {
    let (connection, vault) = connection.split();
    find(connection, id)?.open(vault)
}

fn insert(
    connection: &mut SqliteConnection,
    vault: &Vault,
    document: &EnvironmentDocument,
    position: i32,
    now: DateTime<Utc>,
) -> Result<String, StorageError> {
    let id = Uuid::new_v4().to_string();
    diesel::insert_into(http_environments::table)
        .values((
            http_environments::id.eq(&id),
            http_environments::kind.eq(ENVIRONMENT),
            http_environments::position.eq(position),
            http_environments::created_at.eq(iso8601::format(now)),
            http_environments::document.eq(seal(vault, document)?),
        ))
        .execute(connection)?;
    Ok(id)
}

/// Last, in the next of the five colours: two made one after the other differ.
pub fn create(
    connection: &mut Library,
    name: &str,
    now: DateTime<Utc>,
) -> Result<Environment, StorageError> {
    connection.transaction(|connection, vault| {
        let held = environments(connection, vault)?;
        refuse_taken(&held, name, None)?;
        let position = held
            .iter()
            .map(|environment| i32::try_from(environment.position).unwrap_or(0) + 1)
            .max()
            .unwrap_or(0);
        let document = EnvironmentDocument {
            name: name.to_string(),
            colour: FolderColour::nth(held.len()),
            ..EnvironmentDocument::default()
        };
        let id = insert(connection, vault, &document, position, now)?;
        find(connection, &id)?.open(vault)
    })
}

pub fn save(
    connection: &mut Library,
    id: &str,
    patch: &EnvironmentPatch,
    name: Option<&str>,
) -> Result<Environment, StorageError> {
    connection.transaction(|connection, vault| {
        let row = find(connection, id)?;
        if let Some(name) = name {
            refuse_taken(&environments(connection, vault)?, name, Some(id))?;
        }
        let document = row.document(vault)?.patched(patch, name);
        diesel::update(http_environments::table.find(id))
            .set(http_environments::document.eq(seal(vault, &document)?))
            .execute(connection)?;
        find(connection, id)?.open(vault)
    })
}

/// Written whole: the row is made the first time anything is.
pub fn save_globals(
    connection: &mut Library,
    variables: &[Variable],
    now: DateTime<Utc>,
) -> Result<Vec<Variable>, StorageError> {
    connection.transaction(|connection, vault| {
        let document = seal(
            vault,
            &EnvironmentDocument {
                variables: variables.to_vec(),
                ..EnvironmentDocument::default()
            },
        )?;
        let changed =
            diesel::update(http_environments::table.filter(http_environments::kind.eq(GLOBALS)))
                .set(http_environments::document.eq(&document))
                .execute(connection)?;
        if changed == 0 {
            diesel::insert_into(http_environments::table)
                .values((
                    http_environments::id.eq(Uuid::new_v4().to_string()),
                    http_environments::kind.eq(GLOBALS),
                    http_environments::position.eq(-1),
                    http_environments::created_at.eq(iso8601::format(now)),
                    http_environments::document.eq(&document),
                ))
                .execute(connection)?;
        }
        Ok(variables.to_vec())
    })
}

/// Right after its original, secrets included — the copy stays in the library — under the
/// name the front gives it: « (copie) » is a word, never Rust's.
pub fn duplicate(
    connection: &mut Library,
    id: &str,
    name: &str,
    now: DateTime<Utc>,
) -> Result<Environment, StorageError> {
    connection.transaction(|connection, vault| {
        let original = find(connection, id)?;
        refuse_taken(&environments(connection, vault)?, name, None)?;
        let document = EnvironmentDocument {
            name: name.to_string(),
            ..original.document(vault)?
        };
        diesel::update(
            http_environments::table
                .filter(http_environments::kind.eq(ENVIRONMENT))
                .filter(http_environments::position.gt(original.position)),
        )
        .set(http_environments::position.eq(http_environments::position + 1))
        .execute(connection)?;
        let copy = insert(connection, vault, &document, original.position + 1, now)?;
        find(connection, &copy)?.open(vault)
    })
}

pub fn delete(connection: &mut Library, id: &str) -> Result<(), StorageError> {
    let (connection, _) = connection.split();
    let removed = diesel::delete(
        http_environments::table
            .find(id)
            .filter(http_environments::kind.eq(ENVIRONMENT)),
    )
    .execute(connection)?;
    if removed == 0 {
        return Err(StorageError::HttpEnvironmentNotFound(id.to_string()));
    }
    Ok(())
}
