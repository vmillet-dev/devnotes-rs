//! Nothing here knows the database.
//!
//! The export is a zip: the bundle at the root, one entry per attachment under
//! `attachments/`. ⚠️ Base64 inside the JSON was the obvious alternative and was refused:
//! it costs a third more bytes, and the import path holds the file as a `String`, then a
//! `serde_json::Value`, then a `Bundle` — three copies of every screenshot in memory.
//! Archive entries are pulled one at a time instead.
//!
//! ⚠️ An export leaves the library's key behind. The attachment files on disk are sealed
//! with a key that never leaves this machine, so they are opened on the way out and then
//! either written in the clear — an unprotected export is portable and readable, which is
//! what the exchange format exists for — or resealed under a key derived from the phrase
//! the user gave this one file.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::model::{Bundle, ExportReport, IncomingBundle};
use super::protect::{self, Recipe};
use crate::attachments::model::Attachment;
use crate::count::saturating_u32;
use crate::error::{FileContext, StorageError};
use crate::vault::key::Vault;

const BUNDLE_ENTRY: &str = "bundle.json";
const ATTACHMENTS_ENTRY: &str = "attachments";

/// ⚠️ Present only in a protected export, and it **replaces** `bundle.json` rather than
/// sitting beside it: a reader that finds this and cannot open it must not fall back on a
/// plaintext bundle that should not exist.
const SEALED_ENTRY: &str = "bundle.sealed";

/// Beside the sealed payload, in the clear: the salt and the cost are what a reader needs
/// to derive the same key, and neither is a secret.
const RECIPE_ENTRY: &str = "recipe.json";

/// What a zip opens with. An export written before the archive existed is plain JSON and
/// is still read: a new DevNotes reads an old file, an old DevNotes does not read a new one.
const ZIP_MAGIC: [u8; 4] = [b'P', b'K', 0x03, 0x04];

fn zip_error(error: &zip::result::ZipError) -> StorageError {
    StorageError::File(error.to_string())
}

fn deflated() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

/// A PNG is compressed already, and ciphertext does not compress at all.
fn stored_as_is() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}

/// ⚠️ Written beside the target then renamed: a truncating write destroys the previous
/// export the day the disk fills.
///
/// `passphrase` protects the archive. Without one the file is plaintext — every note,
/// every screenshot — which is what the interface has to say before it writes one.
pub fn write(
    path: &str,
    bundle: &Bundle,
    attachments: &Path,
    library: &Vault,
    passphrase: Option<&str>,
) -> Result<ExportReport, StorageError> {
    let json = serde_json::to_string_pretty(bundle)
        .map_err(|error| StorageError::File(error.to_string()))?;

    let protection = passphrase.map(protect::seal_with).transpose()?;

    let staged = staging_path(path);
    let stored = match archive(
        &staged,
        &json,
        &bundle.attachments,
        attachments,
        library,
        protection.as_ref(),
    ) {
        Ok(stored) => stored,
        Err(error) => {
            let _ = std::fs::remove_file(&staged);
            return Err(error);
        }
    };

    if let Err(error) = std::fs::rename(&staged, path) {
        let _ = std::fs::remove_file(&staged);
        return Err(StorageError::File(format!("{path}: {error}")));
    }

    Ok(ExportReport {
        notes: saturating_u32(bundle.notes.len()),
        spaces: saturating_u32(bundle.spaces.len()),
        folders: saturating_u32(bundle.folders.len()),
        attachments: stored,
        protected: passphrase.is_some(),
    })
}

fn archive(
    staged: &Path,
    json: &str,
    records: &[Attachment],
    source: &Path,
    library: &Vault,
    protection: Option<&(Vault, Recipe)>,
) -> Result<u32, StorageError> {
    let target = File::create(staged).context(staged.display().to_string())?;
    let mut writer = ZipWriter::new(target);

    // The recipe goes in first and in the clear: a reader has to know how to derive the
    // key before it can be asked for a phrase.
    if let Some((_, recipe)) = protection {
        let written = serde_json::to_string_pretty(recipe)
            .map_err(|error| StorageError::File(error.to_string()))?;
        writer
            .start_file(RECIPE_ENTRY, deflated())
            .map_err(|error| zip_error(&error))?;
        writer.write_all(written.as_bytes()).context(RECIPE_ENTRY)?;
    }

    if let Some((vault, _)) = protection {
        writer
            .start_file(SEALED_ENTRY, stored_as_is())
            .map_err(|error| zip_error(&error))?;
        writer
            .write_all(&vault.seal_bytes(json.as_bytes())?)
            .context(SEALED_ENTRY)?;
    } else {
        writer
            .start_file(BUNDLE_ENTRY, deflated())
            .map_err(|error| zip_error(&error))?;
        writer.write_all(json.as_bytes()).context(BUNDLE_ENTRY)?;
    }

    let mut stored = 0;
    for record in records {
        let name = record.stored_name();

        // A record whose file has gone missing leaves the export rather than failing it:
        // the note still travels, and `attachments_missing` says so on the way back in.
        let Ok(sealed) = std::fs::read(source.join(&name)) else {
            continue;
        };

        // Opened under the library's key, then written the way this export travels.
        let plain = library.open_bytes(&sealed)?;
        let bytes = match protection {
            Some((vault, _)) => vault.seal_bytes(&plain)?,
            None => plain,
        };

        writer
            .start_file(format!("{ATTACHMENTS_ENTRY}/{name}"), stored_as_is())
            .map_err(|error| zip_error(&error))?;
        writer.write_all(&bytes).context(name)?;
        stored += 1;
    }

    writer.finish().map_err(|error| zip_error(&error))?;

    Ok(stored)
}

/// The bundle, and whatever carries the attachment bytes that belong with it.
///
/// ⚠️ Answers [`StorageError::PassphraseRequired`] on a protected file offered without
/// one: nothing can tell a protected archive from an ordinary one until it has looked
/// inside, so looking is this function's job rather than the interface's.
pub fn read(
    path: &str,
    passphrase: Option<&str>,
) -> Result<(IncomingBundle, Payload), StorageError> {
    let Some(mut archive) = open_archive(path)? else {
        let json = std::fs::read_to_string(path).context(path)?;
        return Ok((super::model::read_bundle(&json)?, Payload::Empty));
    };

    let Some(recipe) = read_recipe(&mut archive)? else {
        let raw = entry(&mut archive, BUNDLE_ENTRY)?;
        let json = String::from_utf8(raw)
            .map_err(|_| StorageError::ImportFormat("the bundle is not text".to_string()))?;

        return Ok((
            super::model::read_bundle(&json)?,
            Payload::Archive {
                archive: Box::new(archive),
                vault: None,
            },
        ));
    };

    let Some(passphrase) = passphrase else {
        return Err(StorageError::PassphraseRequired);
    };

    let vault = protect::open_with(passphrase, &recipe)?;
    let sealed = entry(&mut archive, SEALED_ENTRY)?;

    // ⚠️ The payload's own tag is the check: a phrase that does not open it is refused
    // here, so the file carries no separate verifier to work against.
    let raw = vault
        .open_bytes(&sealed)
        .map_err(|_| StorageError::WrongPassphrase)?;
    let json = String::from_utf8(raw)
        .map_err(|_| StorageError::ImportFormat("the bundle is not text".to_string()))?;

    Ok((
        super::model::read_bundle(&json)?,
        Payload::Archive {
            archive: Box::new(archive),
            vault: Some(vault),
        },
    ))
}

/// Whether a file will want a phrase, asked without one so an interface can prompt.
///
/// ⚠️ The recipe and nothing else. Answering this through [`read`] would parse the whole
/// bundle — a hundred megabytes on a large library — and then throw it away, for the import
/// to parse it again a moment later.
pub fn is_protected(path: &str) -> Result<bool, StorageError> {
    let Some(mut archive) = open_archive(path)? else {
        return Ok(false);
    };

    Ok(read_recipe(&mut archive)?.is_some())
}

/// `None` for a file that is not a zip: a `.json` export written before the archive, which
/// carries no attachments and cannot be protected.
fn open_archive(path: &str) -> Result<Option<ZipArchive<File>>, StorageError> {
    let mut file = File::open(path).context(path)?;

    let mut magic = [0u8; 4];
    let zipped = file.read_exact(&mut magic).is_ok() && magic == ZIP_MAGIC;
    file.seek(SeekFrom::Start(0)).context(path)?;

    if !zipped {
        return Ok(None);
    }

    Ok(Some(ZipArchive::new(file).map_err(|error| {
        StorageError::ImportFormat(error.to_string())
    })?))
}

fn read_recipe(archive: &mut ZipArchive<File>) -> Result<Option<Recipe>, StorageError> {
    if archive.by_name(RECIPE_ENTRY).is_err() {
        return Ok(None);
    }

    let raw = entry(archive, RECIPE_ENTRY)?;
    let recipe: Recipe = serde_json::from_slice(&raw)
        .map_err(|error| StorageError::ImportFormat(format!("unreadable recipe: {error}")))?;

    Ok(Some(recipe))
}

fn entry(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>, StorageError> {
    let mut entry = archive
        .by_name(name)
        .map_err(|_| StorageError::ImportFormat(format!("no {name} in the archive")))?;

    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).context(name)?;

    Ok(bytes)
}

/// The attachment bytes of an import, handed over one at a time.
///
/// ⚠️ `Debug` says nothing about the key it may hold, for the same reason `Vault`'s does.
pub enum Payload {
    /// A `.json` export: it carried no attachments.
    Empty,
    Archive {
        archive: Box<ZipArchive<File>>,
        /// `None` for a plaintext archive; the export's own key for a protected one.
        vault: Option<Vault>,
    },
}

impl std::fmt::Debug for Payload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("Payload::Empty"),
            Self::Archive { vault, .. } => formatter.write_str(if vault.is_some() {
                "Payload::Archive(protected)"
            } else {
                "Payload::Archive"
            }),
        }
    }
}

impl Payload {
    /// `None` when the archive names the record but does not carry its bytes, and `None`
    /// too when it carries them under a key that will not open them — a caller cannot act
    /// on the difference, and the import reports both as missing.
    pub fn take(&mut self, stored_name: &str) -> Option<Vec<u8>> {
        let Self::Archive { archive, vault } = self else {
            return None;
        };

        let mut entry = archive
            .by_name(&format!("{ATTACHMENTS_ENTRY}/{stored_name}"))
            .ok()?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).ok()?;

        match vault {
            Some(vault) => vault.open_bytes(&bytes).ok(),
            None => Some(bytes),
        }
    }
}

/// ⚠️ Same directory as the target, or the rename crosses volumes and stops being atomic.
fn staging_path(path: &str) -> PathBuf {
    let target = Path::new(path);
    let name = target
        .file_name()
        .map_or_else(|| OsString::from("export"), OsStr::to_os_string);

    let mut staged = OsString::from(".");
    staged.push(name);
    staged.push(format!(".{}.tmp", Uuid::new_v4()));

    target.with_file_name(staged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::fixtures::note as sample;
    use crate::spaces::model::Space;
    use crate::vault::key::Cost;

    fn library() -> Vault {
        Vault::derive("the library", b"0123456789abcdef", Cost::FOR_TESTS).unwrap()
    }

    fn bundle() -> Bundle {
        Bundle {
            version: super::super::model::FORMAT_VERSION,
            exported_at: sample().created_at,
            spaces: vec![Space {
                id: "s-1".to_string(),
                name: "Personal".to_string(),
                pinned: false,
            }],
            folders: Vec::new(),
            notes: vec![sample()],
            attachments: Vec::new(),
        }
    }

    fn record() -> Attachment {
        Attachment {
            id: "a-1".to_string(),
            note_id: sample().id,
            file_name: "capture.png".to_string(),
            mime_type: "image/png".to_string(),
            byte_size: 4,
            created_at: sample().created_at,
        }
    }

    /// The attachment as it really sits beside the database: sealed under the library.
    fn seal_beside(directory: &Path, vault: &Vault, bytes: &[u8]) {
        let sealed = vault.seal_bytes(bytes).unwrap();
        std::fs::write(directory.join(record().stored_name()), sealed).unwrap();
    }

    /// A staging file one directory away would make the rename cross volumes.
    #[test]
    fn the_staging_file_sits_next_to_its_target() {
        let target = std::env::temp_dir()
            .join("documents")
            .join("library.devnotes");

        let staged = staging_path(&target.to_string_lossy());

        assert_eq!(staged.parent(), target.parent());
        assert_ne!(staged.file_name(), target.file_name());
    }

    #[test]
    fn two_exports_of_the_same_target_never_stage_the_same_file() {
        let scratch = tempfile::tempdir().unwrap();
        let target = scratch.path().join("library.devnotes");

        let first = staging_path(&target.to_string_lossy());
        let second = staging_path(&target.to_string_lossy());

        assert_ne!(first, second);
    }

    #[test]
    fn an_export_leaves_no_staging_file_behind() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");

        write(
            &target.to_string_lossy(),
            &bundle(),
            &directory,
            &library(),
            None,
        )
        .unwrap();

        let left: Vec<_> = std::fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();

        assert_eq!(left, ["library.devnotes"]);
    }

    #[test]
    fn exporting_over_an_existing_file_replaces_it_whole() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");
        std::fs::write(&target, "previous export, longer than what replaces it").unwrap();

        write(
            &target.to_string_lossy(),
            &bundle(),
            &directory,
            &library(),
            None,
        )
        .unwrap();

        let written = std::fs::read(&target).unwrap();
        assert_eq!(written[..4], ZIP_MAGIC);
    }

    #[test]
    fn an_export_to_an_unreachable_directory_reports_rather_than_panicking() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();

        let error = write(
            "/no/such/directory/library.devnotes",
            &bundle(),
            &directory,
            &library(),
            None,
        )
        .unwrap_err();

        assert!(matches!(error, StorageError::File(_)), "{error}");
    }

    #[test]
    fn a_written_bundle_reads_back_as_itself() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");

        write(
            &target.to_string_lossy(),
            &bundle(),
            &directory,
            &library(),
            None,
        )
        .unwrap();
        let (read_back, _) = read(&target.to_string_lossy(), None).unwrap();

        assert_eq!(read_back.bundle.notes.len(), 1);
        assert_eq!(read_back.bundle.spaces[0].name, "Personal");
    }

    /// The point of the archive: the bytes travel with the record.
    #[test]
    fn an_attachment_travels_with_its_note() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");
        let vault = library();
        seal_beside(&directory, &vault, b"\x89PNG");

        let mut exported = bundle();
        exported.attachments = vec![record()];
        let report = write(
            &target.to_string_lossy(),
            &exported,
            &directory,
            &vault,
            None,
        )
        .unwrap();

        assert_eq!(report.attachments, 1);
        assert!(!report.protected);

        let (read_back, mut payload) = read(&target.to_string_lossy(), None).unwrap();
        assert_eq!(read_back.bundle.attachments.len(), 1);
        assert_eq!(
            payload.take(&record().stored_name()),
            Some(b"\x89PNG".to_vec())
        );
    }

    /// ⚠️ A record whose file has gone missing must not fail the export.
    #[test]
    fn a_record_whose_file_is_gone_leaves_the_export_rather_than_failing_it() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");

        let mut exported = bundle();
        exported.attachments = vec![record()];
        let report = write(
            &target.to_string_lossy(),
            &exported,
            &directory,
            &library(),
            None,
        )
        .unwrap();

        assert_eq!(report.attachments, 0);
        assert_eq!(report.notes, 1);
    }

    /// ⚠️ New DevNotes reads what old DevNotes wrote: a `.json` export predates the archive.
    #[test]
    fn a_json_export_from_before_the_archive_still_imports() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.json");
        let json = serde_json::to_string_pretty(&bundle()).unwrap();
        std::fs::write(&target, json).unwrap();

        let (read_back, payload) = read(&target.to_string_lossy(), None).unwrap();

        assert_eq!(read_back.bundle.notes.len(), 1);
        assert!(matches!(payload, Payload::Empty));
    }

    /// ⚠️ The whole point of protecting an export: the file most likely to leave the
    /// machine was the one carrying everything in the clear.
    #[test]
    fn a_protected_export_carries_none_of_the_notes_in_the_clear() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");
        let vault = library();
        seal_beside(&directory, &vault, b"a screenshot of something");

        let mut exported = bundle();
        exported.notes[0].content = "psql -h prod -W hunter2".to_string();
        exported.attachments = vec![record()];

        let report = write(
            &target.to_string_lossy(),
            &exported,
            &directory,
            &vault,
            Some("a shared phrase"),
        )
        .unwrap();
        assert!(report.protected);

        let raw = std::fs::read(&target).unwrap();
        let haystack = String::from_utf8_lossy(&raw);
        assert!(!haystack.contains("hunter2"));
        assert!(!haystack.contains("Personal"));
        assert!(!haystack.contains("a screenshot of"));
    }

    #[test]
    fn a_protected_export_reads_back_whole_with_its_phrase() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");
        let vault = library();
        seal_beside(&directory, &vault, b"\x89PNG");

        let mut exported = bundle();
        exported.attachments = vec![record()];
        write(
            &target.to_string_lossy(),
            &exported,
            &directory,
            &vault,
            Some("a shared phrase"),
        )
        .unwrap();

        let (read_back, mut payload) =
            read(&target.to_string_lossy(), Some("a shared phrase")).unwrap();

        assert_eq!(read_back.bundle.notes.len(), 1);
        assert_eq!(
            payload.take(&record().stored_name()),
            Some(b"\x89PNG".to_vec())
        );
    }

    /// ⚠️ Offered without one, it asks rather than failing: nothing can know a file is
    /// protected until something has looked inside it.
    #[test]
    fn a_protected_export_asks_for_a_phrase_rather_than_failing() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");

        write(
            &target.to_string_lossy(),
            &bundle(),
            &directory,
            &library(),
            Some("a shared phrase"),
        )
        .unwrap();

        let error = read(&target.to_string_lossy(), None).unwrap_err();
        assert!(matches!(error, StorageError::PassphraseRequired), "{error}");
        assert!(is_protected(&target.to_string_lossy()).unwrap());
    }

    #[test]
    fn the_wrong_phrase_is_refused_rather_than_read_as_nonsense() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");

        write(
            &target.to_string_lossy(),
            &bundle(),
            &directory,
            &library(),
            Some("a shared phrase"),
        )
        .unwrap();

        let error = read(&target.to_string_lossy(), Some("the wrong one")).unwrap_err();

        assert!(matches!(error, StorageError::WrongPassphrase), "{error}");
    }

    /// ⚠️ Asked before an import starts, so it must not pay for the bundle: a file whose
    /// payload could not be parsed at all still answers the question.
    #[test]
    fn whether_a_file_is_protected_is_answered_without_reading_the_bundle() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");
        write(
            &target.to_string_lossy(),
            &bundle(),
            &directory,
            &library(),
            None,
        )
        .unwrap();

        // The entry is there and is nonsense; only the recipe decides the answer.
        let mut rewritten = ZipWriter::new(std::fs::File::create(&target).unwrap());
        rewritten.start_file(BUNDLE_ENTRY, deflated()).unwrap();
        rewritten.write_all(b"not json at all").unwrap();
        rewritten.finish().unwrap();

        assert!(!is_protected(&target.to_string_lossy()).unwrap());
        // And the import that follows is what says the file is unreadable.
        assert!(read(&target.to_string_lossy(), None).is_err());
    }

    /// An ordinary export needs no phrase, and must not be made to ask for one.
    #[test]
    fn an_unprotected_export_is_not_reported_as_protected() {
        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().to_path_buf();
        let target = directory.join("library.devnotes");

        write(
            &target.to_string_lossy(),
            &bundle(),
            &directory,
            &library(),
            None,
        )
        .unwrap();

        assert!(!is_protected(&target.to_string_lossy()).unwrap());
    }
}
