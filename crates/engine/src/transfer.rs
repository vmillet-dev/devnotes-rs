#![allow(clippy::needless_pass_by_value)]

pub mod bundle;
pub mod file;
pub mod model;
pub mod protect;

use crate::Engine;
use crate::attachments;
use crate::db::lock;
use crate::error::AppError;
use crate::notes::store as notes;
use crate::vault::secret;
use model::{ExportReport, ExportScope, ImportReport};

/// A `None` passphrase writes the file in the clear: the library's key protects what is
/// on this machine, never what leaves it.
pub fn export_notes(
    engine: &Engine,
    path: String,
    scope: ExportScope,
    passphrase: Option<String>,
) -> Result<ExportReport, AppError> {
    let db = &engine.db;
    let passphrase = secret(passphrase);
    model::validate_path(&path)?;
    crate::vault::validate_protection(passphrase.as_deref())?;

    let mut connection = lock(db)?;
    let exported = bundle::of(&mut connection, &scope)?;
    // Nothing to export writes nothing: an empty file would read as an export of something.
    if exported.notes.is_empty() {
        return Ok(ExportReport::nothing(passphrase.is_some()));
    }

    Ok(file::write(
        &path,
        &exported,
        &attachments::files::directory(&connection),
        connection.vault(),
        passphrase.as_deref(),
    )?)
}

/// How many notes an export would carry, asked before the save dialog: none is said at once.
pub fn count_export(engine: &Engine, scope: ExportScope) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(bundle::count(&mut connection, &scope)?)
}

/// The file is read before the lock is taken: parsing a large export while holding the
/// connection would block every other command for the length of it.
pub fn import_notes(
    engine: &Engine,
    path: String,
    passphrase: Option<String>,
) -> Result<ImportReport, AppError> {
    let db = &engine.db;
    model::validate_path(&path)?;

    let (imported, mut payload) = file::read(&path, secret(passphrase).as_deref())?;

    let mut connection = lock(db)?;
    let directory = attachments::files::directory(&connection);

    Ok(bundle::merge(
        &mut connection,
        imported,
        &mut payload,
        &directory,
    )?)
}

/// Whether an import will want a phrase, so the interface can ask before it starts.
pub fn export_is_protected(path: String) -> Result<bool, AppError> {
    model::validate_path(&path)?;

    Ok(file::is_protected(&path)?)
}

/// Nothing is sent anywhere: "share" stops at the clipboard.
pub fn share_notes(engine: &Engine, ids: Vec<String>) -> Result<String, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let selected = notes::by_ids(&mut connection, &ids)?;
    let names = bundle::space_names(&mut connection)?;

    Ok(model::to_markdown(&selected, &names))
}
