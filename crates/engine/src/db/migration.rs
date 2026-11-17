//! ⚠️ Append-only. Evolving the model means adding a
//! `migrations/YYYY-MM-DD-HHMMSS_name/` directory, never editing a shipped one.

use diesel::migration::MigrationSource;
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::error::StorageError;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

fn embedded_versions() -> Result<Vec<String>, StorageError> {
    let mut versions = MigrationSource::<Sqlite>::migrations(&MIGRATIONS)
        .map_err(|error| StorageError::Migration(error.to_string()))?
        .iter()
        .map(|migration| migration.name().version().to_string())
        .collect::<Vec<_>>();
    versions.sort();

    Ok(versions)
}

pub fn run(connection: &mut SqliteConnection) -> Result<(), StorageError> {
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
    use super::*;
    use crate::db::{open, open_in_memory};
    use crate::error::StorageError;
    use crate::layout::DATABASE;

    #[test]
    fn opening_twice_is_idempotent() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let path = directory.join(DATABASE);

        open(&path, crate::db::test_vault().unwrap()).unwrap();
        let mut connection = open(&path, crate::db::test_vault().unwrap()).unwrap();

        assert!(!connection.db().has_pending_migration(MIGRATIONS).unwrap());
    }

    #[test]
    fn a_fresh_database_applies_every_embedded_migration() {
        let mut connection = open_in_memory().unwrap();

        let applied = connection.db().applied_migrations().unwrap();
        assert_eq!(applied.len(), embedded_versions().unwrap().len());
    }

    /// A `down.sql` nobody runs is a `down.sql` nobody can trust: this is what would
    /// catch a `DROP COLUMN` blocked by an index the revert forgot to drop first.
    #[test]
    fn every_migration_can_be_reverted_and_replayed() {
        let mut connection = open_in_memory().unwrap();

        let connection = connection.db();
        connection.revert_all_migrations(MIGRATIONS).unwrap();
        assert!(connection.applied_migrations().unwrap().is_empty());

        connection.run_pending_migrations(MIGRATIONS).unwrap();

        assert!(!connection.has_pending_migration(MIGRATIONS).unwrap());
    }

    #[test]
    fn a_migration_this_binary_does_not_know_is_refused() {
        let mut connection = open_in_memory().unwrap();
        diesel::sql_query(
            "INSERT INTO __diesel_schema_migrations (version) VALUES ('2099-01-01-000000')",
        )
        .execute(connection.db())
        .unwrap();

        let error = run(connection.db()).unwrap_err();

        assert!(matches!(error, StorageError::SchemaTooRecent(_)));
    }
}
