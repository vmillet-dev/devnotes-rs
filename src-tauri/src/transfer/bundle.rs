//! The commands in `transfer.rs` open the file and hold the lock; the rules live here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::Utc;
use diesel::SqliteConnection;
use uuid::Uuid;

use super::file::Payload;
use super::model::{self, Bundle, ExportScope, ImportReport, IncomingBundle};
use crate::attachments::model::Attachment;
use crate::attachments::sealed;
use crate::attachments::store as attachments;
use crate::count::saturating_u32;
use crate::db::Library;
use crate::error::StorageError;
use crate::folders::model::Folder;
use crate::folders::store as folders;
use crate::notes::model::Note;
use crate::notes::store as notes;
use crate::spaces::model::Space;
use crate::spaces::store as spaces;
use crate::vault::key::Vault;

/// The live notes a scope covers, then the bundle that carries them.
pub fn of(connection: &mut Library, scope: &ExportScope) -> Result<Bundle, StorageError> {
    let exported = match scope {
        ExportScope::Library => notes::all(connection, None)?,
        ExportScope::Space { space_id } => notes::all(connection, Some(space_id))?,
        ExportScope::Notes { ids } => notes::by_ids(connection, ids)?,
    };

    collect(connection, exported)
}

/// Only the spaces and folders cited travel: exporting one space must not recreate the tree.
pub fn collect(connection: &mut Library, exported: Vec<Note>) -> Result<Bundle, StorageError> {
    let spaces: Vec<Space> = spaces::list(connection)?
        .into_iter()
        .filter(|space| exported.iter().any(|note| note.space_id == space.id))
        .collect();

    let folders: Vec<Folder> = folders::list(connection, None)?
        .into_iter()
        .filter(|folder| {
            exported
                .iter()
                .any(|note| note.folder_id.as_deref() == Some(folder.id.as_str()))
        })
        .collect();

    let note_ids: Vec<String> = exported.iter().map(|note| note.id.clone()).collect();

    Ok(Bundle {
        version: model::FORMAT_VERSION,
        exported_at: Utc::now(),
        spaces,
        folders,
        attachments: attachments::for_notes(connection, &note_ids)?,
        notes: exported,
    })
}

/// Merge, never replace: spaces match by name, and a note whose id is taken is counted and
/// skipped, so importing a file twice duplicates nothing. One transaction for the whole
/// file, or a failure halfway leaves part of it in with the report lost.
pub fn merge(
    connection: &mut Library,
    incoming: IncomingBundle,
    payload: &mut Payload,
    directory: &Path,
) -> Result<ImportReport, StorageError> {
    let IncomingBundle { bundle, degraded } = incoming;

    connection.transaction(|connection, vault| {
        let mut report = ImportReport::default();

        let mut arrived: BTreeSet<String> = BTreeSet::new();
        let mut mapping: BTreeMap<String, String> = BTreeMap::new();
        let existing = spaces::list_in(connection, vault)?;

        for space in &bundle.spaces {
            let matched = existing
                .iter()
                .find(|candidate| candidate.name.to_lowercase() == space.name.to_lowercase());

            let local_id = if let Some(candidate) = matched {
                candidate.id.clone()
            } else {
                report.spaces_created += 1;
                spaces::create_in(connection, vault, &space.name)?.id
            };
            mapping.insert(space.id.clone(), local_id);
        }

        let folder_mapping =
            merge_folders(connection, vault, &bundle.folders, &mapping, &mut report)?;

        for mut note in bundle.notes {
            let Some(space_id) = mapping.get(&note.space_id) else {
                // A file truncated by hand: inventing a space would file the note out of sight.
                report.notes_skipped += 1;
                continue;
            };
            note.space_id.clone_from(space_id);
            // Remapped, and dropped when the file did not carry the folder: the id is the
            // sending library's, and a dangling one would fail the foreign key and the import.
            note.folder_id = note
                .folder_id
                .as_ref()
                .and_then(|id| folder_mapping.get(id))
                .cloned();

            let note = note.normalized();
            if notes::insert_imported_in(connection, vault, &note)? {
                report.notes_imported += 1;
                arrived.insert(note.id.clone());
                // Only what came in, or re-importing would keep reporting the same loss.
                if degraded.contains(&note.id) {
                    report.notes_degraded += 1;
                }
            } else {
                report.notes_skipped += 1;
            }
        }

        restore_attachments(
            connection,
            vault,
            bundle.attachments,
            &arrived,
            payload,
            directory,
            &mut report,
        )?;

        Ok(report)
    })
}

/// Matched by name inside the destination space, case-insensitively, and created when absent.
/// A folder whose space did not make it is dropped: its notes were skipped too.
fn merge_folders(
    connection: &mut SqliteConnection,
    vault: &Vault,
    incoming: &[Folder],
    spaces: &BTreeMap<String, String>,
    report: &mut ImportReport,
) -> Result<BTreeMap<String, String>, StorageError> {
    let mut mapping: BTreeMap<String, String> = BTreeMap::new();

    for folder in incoming {
        let Some(space_id) = spaces.get(&folder.space_id) else {
            continue;
        };

        let existing = folders::list_in(connection, vault, Some(space_id))?;
        let matched = existing
            .iter()
            .find(|candidate| candidate.name.to_lowercase() == folder.name.to_lowercase());

        let local_id = if let Some(candidate) = matched {
            candidate.id.clone()
        } else {
            report.folders_created += 1;
            folders::create_in(connection, vault, space_id, &folder.name, folder.created_at)?.id
        };
        mapping.insert(folder.id.clone(), local_id);
    }

    Ok(mapping)
}

/// The file before the record, as in `attachments.rs`; a rolled-back transaction leaves files
/// the startup sweep collects. Only attachments whose note arrived: re-importing adds nothing.
fn restore_attachments(
    connection: &mut SqliteConnection,
    vault: &Vault,
    records: Vec<Attachment>,
    arrived: &BTreeSet<String>,
    payload: &mut Payload,
    directory: &Path,
    report: &mut ImportReport,
) -> Result<(), StorageError> {
    for record in records {
        if !arrived.contains(&record.note_id) {
            continue;
        }

        // The archive is keyed by the sending library's stored name: bytes first, then the id.
        let Some(bytes) = payload.take(&record.stored_name()) else {
            report.attachments_missing += 1;
            continue;
        };

        // ⚠️ Remapped like a space or a folder: this id came from a file someone was sent, and it
        // is half of the name joined onto the attachments directory below. The size is what
        // arrived, never what the file claimed.
        let record = Attachment {
            id: Uuid::new_v4().to_string(),
            byte_size: saturating_u32(bytes.len()),
            ..record
        };

        sealed::write_sealed(vault, &directory.join(record.stored_name()), &bytes)?;
        attachments::create(connection, vault, &record)?;
        report.attachments_imported += 1;
    }

    Ok(())
}

/// The space names an export needs to render a note's breadcrumb.
pub fn space_names(connection: &mut Library) -> Result<BTreeMap<String, String>, StorageError> {
    Ok(spaces::list(connection)?
        .into_iter()
        .map(|space| (space.id, space.name))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::notes::fixtures::note as sample;
    use crate::transfer::file;
    use crate::vault::key::Cost;

    fn sending_vault() -> Vault {
        Vault::derive("the sending library", b"0123456789abcdef", Cost::FOR_TESTS).unwrap()
    }

    /// A record claiming `id = "../vault"` must not write the wrapped key beside the library.
    #[test]
    fn an_identifier_read_out_of_a_file_cannot_write_outside_the_attachments_directory() {
        let scratch = tempfile::tempdir().unwrap();
        let profile = scratch.path().to_path_buf();
        let sending_files = profile.join("sending");
        let receiving_files = profile.join("attachments");
        std::fs::create_dir_all(&sending_files).unwrap();
        std::fs::create_dir_all(&receiving_files).unwrap();
        std::fs::write(profile.join("vault.json"), b"the wrapped master key").unwrap();

        let note = sample();
        let hostile = Attachment {
            id: "../vault".to_string(),
            note_id: note.id.clone(),
            file_name: "note.json".to_string(),
            mime_type: "application/json".to_string(),
            byte_size: 4,
            created_at: note.created_at,
        };

        let vault = sending_vault();
        std::fs::write(
            sending_files.join(hostile.stored_name()),
            vault.seal_bytes(b"CLOBBERED").unwrap(),
        )
        .unwrap();

        let bundle = Bundle {
            version: model::FORMAT_VERSION,
            exported_at: note.created_at,
            spaces: vec![Space {
                id: note.space_id.clone(),
                name: "Personal".to_string(),
                pinned: false,
            }],
            folders: Vec::new(),
            notes: vec![note],
            attachments: vec![hostile],
        };

        let path = profile
            .join("crafted.devnotes")
            .to_string_lossy()
            .to_string();
        file::write(&path, &bundle, &sending_files, &vault, None).unwrap();

        let mut receiving = open_in_memory().unwrap();
        let (incoming, mut payload) = file::read(&path, None).unwrap();
        let report = merge(&mut receiving, incoming, &mut payload, &receiving_files).unwrap();

        assert_eq!(report.attachments_imported, 1);
        assert_eq!(
            std::fs::read(profile.join("vault.json")).unwrap(),
            b"the wrapped master key",
            "the key file was rewritten by an import"
        );

        // The bytes did arrive — contained, under an id this library generated.
        let landed: Vec<std::fs::DirEntry> = std::fs::read_dir(&receiving_files)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(landed.len(), 1);
        let landed = landed[0].path();
        let stem = landed.file_stem().unwrap().to_string_lossy();
        assert_eq!(landed.extension().unwrap(), "json");
        assert!(Uuid::parse_str(&stem).is_ok(), "{stem}");
    }
}
