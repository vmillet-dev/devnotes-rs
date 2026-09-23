//! ⚠️ Append-only. Evolving the model means adding a
//! `migrations/YYYY-MM-DD-HHMMSS_name/` directory, never editing a shipped one.

use diesel::migration::MigrationSource;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::sqlite::Sqlite;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::error::StorageError;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[derive(QueryableByName)]
struct UserVersion {
    #[diesel(sql_type = Integer)]
    user_version: i32,
}

fn embedded_versions() -> Result<Vec<String>, StorageError> {
    let mut versions = MigrationSource::<Sqlite>::migrations(&MIGRATIONS)
        .map_err(|error| StorageError::Migration(error.to_string()))?
        .iter()
        .map(|migration| migration.name().version().to_string())
        .collect::<Vec<_>>();
    versions.sort();

    Ok(versions)
}

/// ⚠️ 0.1.0 versioned its schema with `PRAGMA user_version` rather than Diesel's table, and
/// replaying the migrations over it would fail on the first `CREATE TABLE`. It is refused by
/// name instead: 0.4 still adopts that history, and a library opened there once opens here.
fn refuse_a_pre_diesel_schema(connection: &mut SqliteConnection) -> Result<(), StorageError> {
    let legacy = diesel::sql_query("PRAGMA user_version")
        .get_result::<UserVersion>(connection)?
        .user_version;

    if legacy > 0 {
        return Err(StorageError::Migration(format!(
            "schema version {legacy} was written by DevNotes 0.1.0: open this library with \
             DevNotes 0.4 first"
        )));
    }

    Ok(())
}

pub fn run(connection: &mut SqliteConnection) -> Result<(), StorageError> {
    refuse_a_pre_diesel_schema(connection)?;

    let embedded = embedded_versions()?;

    // An applied migration we do not know signals a database written by a newer version.
    let applied = connection
        .applied_migrations()
        .map_err(|error| StorageError::Migration(error.to_string()))?;
    if let Some(unknown) = applied
        .iter()
        .map(ToString::to_string)
        .find(|version| !embedded.contains(version))
    {
        return Err(StorageError::SchemaTooRecent(unknown));
    }

    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|error| StorageError::Migration(error.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use diesel::connection::SimpleConnection;

    use super::*;
    use crate::db::{configure, open, open_in_memory};
    use crate::error::StorageError;
    use crate::layout::DATABASE;

    #[test]
    fn opening_twice_is_idempotent() {
        let directory = std::env::temp_dir().join(format!("devnotes-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(DATABASE);

        open(&path, crate::db::test_vault().unwrap()).unwrap();
        let mut connection = open(&path, crate::db::test_vault().unwrap()).unwrap();

        assert!(!connection.has_pending_migration(MIGRATIONS).unwrap());

        std::fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_fresh_database_applies_every_embedded_migration() {
        let mut connection = open_in_memory().unwrap();

        let applied = connection.applied_migrations().unwrap();
        assert_eq!(applied.len(), embedded_versions().unwrap().len());
    }

    /// A `down.sql` nobody runs is a `down.sql` nobody can trust: this is what would
    /// catch a `DROP COLUMN` blocked by an index the revert forgot to drop first.
    #[test]
    fn every_migration_can_be_reverted_and_replayed() {
        let mut connection = open_in_memory().unwrap();

        connection.revert_all_migrations(MIGRATIONS).unwrap();
        assert!(connection.applied_migrations().unwrap().is_empty());

        connection.run_pending_migrations(MIGRATIONS).unwrap();

        assert!(!connection.has_pending_migration(MIGRATIONS).unwrap());
    }

    /// The schema 0.1.0 shipped, versioned the way it versioned it.
    #[test]
    fn a_schema_versioned_by_the_old_pragma_is_refused_rather_than_replayed() {
        let mut connection = SqliteConnection::establish(":memory:").unwrap();
        configure(&mut connection).unwrap();
        connection
            .batch_execute(include_str!(
                "../../migrations/2026-07-25-000001_initial/up.sql"
            ))
            .unwrap();
        connection.batch_execute("PRAGMA user_version = 1").unwrap();

        let Err(error) = run(&mut connection) else {
            panic!("a 0.1.0 schema was migrated as if it were empty");
        };

        assert!(matches!(error, StorageError::Migration(_)));
        assert!(error.to_string().contains("0.4"), "{error}");
    }

    #[test]
    fn a_migration_this_binary_does_not_know_is_refused() {
        let mut connection = open_in_memory().unwrap();
        diesel::sql_query(
            "INSERT INTO __diesel_schema_migrations (version) VALUES ('2099-01-01-000000')",
        )
        .execute(connection.db())
        .unwrap();

        let error = run(&mut connection).unwrap_err();

        assert!(matches!(error, StorageError::SchemaTooRecent(_)));
    }
}
