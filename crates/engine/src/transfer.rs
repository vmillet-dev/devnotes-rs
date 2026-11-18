pub mod bundle;
pub mod file;
pub mod model;
pub mod protect;

use std::sync::Arc;

use crate::Engine;
use crate::attachments;
use crate::db::{self, lock};
use crate::error::AppError;
use crate::index::Changed;
use crate::notes::store as notes;
use crate::tools::off_thread;
use crate::vault::secret;
use model::{ExportReport, ExportScope, ImportReport};

/// A `None` passphrase writes the file in the clear: the library's key protects what is
/// on this machine, never what leaves it. The rows are read on the db queue; the file — every
/// attachment opened, sealed and zipped — is written off it, with the key alone: a large export
/// holds back no other command.
pub async fn export_notes(
    engine: Arc<Engine>,
    path: String,
    scope: ExportScope,
    passphrase: Option<String>,
) -> Result<ExportReport, AppError> {
    let passphrase = secret(passphrase);
    model::validate_path(&path)?;
    crate::vault::validate_protection(passphrase.as_deref())?;

    let (exported, directory, vault) = db::run(engine, move |engine| {
        let mut connection = lock(&engine.db)?;
        let exported = bundle::of(&mut connection, &scope)?;
        let directory = attachments::files::directory(&connection);
        Ok((exported, directory, connection.shared_vault()))
    })
    .await?;
    // Nothing to export writes nothing: an empty file would read as an export of something.
    if exported.notes.is_empty() {
        return Ok(ExportReport::nothing(passphrase.is_some()));
    }

    let written = off_thread(move || {
        file::write(&path, &exported, &directory, &vault, passphrase.as_deref())
    })
    .await?;
    Ok(written?)
}

/// How many notes an export would carry, asked before the save dialog: none is said at once.
pub fn count_export(engine: &Engine, scope: &ExportScope) -> Result<u32, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;

    Ok(bundle::count(&mut connection, scope)?)
}

/// The file is read and opened off the db queue: parsing a large export there would hold back
/// every other command for the length of it.
pub async fn import_notes(
    engine: Arc<Engine>,
    path: String,
    passphrase: Option<String>,
) -> Result<ImportReport, AppError> {
    model::validate_path(&path)?;
    let passphrase = secret(passphrase);
    let read = off_thread(move || file::read(&path, passphrase.as_deref())).await?;
    let (imported, mut payload) = read?;

    db::run(engine, move |engine| {
        engine.write(|db| {
            let mut connection = lock(db)?;
            let directory = attachments::files::directory(&connection);

            let answer = bundle::merge(&mut connection, imported, &mut payload, &directory)?;
            let changed = Changed::everything();
            Ok((answer, changed))
        })
    })
    .await
}

/// Whether an import will want a phrase, so the interface can ask before it starts.
pub fn export_is_protected(path: &str) -> Result<bool, AppError> {
    model::validate_path(path)?;

    Ok(file::is_protected(path)?)
}

/// Nothing is sent anywhere: "share" stops at the clipboard.
pub fn share_notes(engine: &Engine, ids: &[String]) -> Result<String, AppError> {
    let db = &engine.db;
    let mut connection = lock(db)?;
    let selected = notes::by_ids(&mut connection, ids)?;
    let names = bundle::space_names(&mut connection)?;

    Ok(model::to_markdown(&selected, &names))
}
