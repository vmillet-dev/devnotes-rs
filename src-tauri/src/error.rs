//! No user-facing text leaves this module: a `String` would put French in the English UI and
//! make callers parse prose. The front maps `code` onto a translation key.

use std::collections::BTreeMap;

use serde::Serialize;
use specta::Type;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Invalid field \"{field}\": {detail}")]
pub struct ValidationError {
    /// The offending field, spelled the way the front names it.
    pub field: &'static str,
    pub detail: String,
}

impl ValidationError {
    pub fn new(field: &'static str, detail: impl Into<String>) -> Self {
        Self {
            field,
            detail: detail.into(),
        }
    }
}

/// Each variant becomes a code the front end translates; `Display` is only the technical
/// detail.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Never a silent `Ok`: the front would believe the write went through.
    #[error("Note not found: {0}")]
    NoteNotFound(String),
    #[error("Space not found: {0}")]
    SpaceNotFound(String),
    #[error("A space named \"{0}\" already exists")]
    DuplicateSpaceName(String),
    #[error("Folder not found: {0}")]
    FolderNotFound(String),
    #[error("A folder named \"{0}\" already exists in this space")]
    DuplicateFolderName(String),
    #[error("Attachment not found: {0}")]
    AttachmentNotFound(String),
    #[error("Revision not found: {0}")]
    RevisionNotFound(String),
    #[error("Library not found: {0}")]
    LibraryNotFound(String),
    /// Refused on a library that is open: deleting it or setting it aside would move files
    /// out from under a live connection.
    #[error("The library is open")]
    LibraryOpen,
    #[error("The last library cannot be deleted")]
    LastLibrary,
    #[error("Nothing to set aside at {0}")]
    NothingToSetAside(String),
    #[error("Backup not found: {0}")]
    BackupNotFound(String),
    /// A copy whose key file did not travel with it: it opens for nobody.
    #[error("Backup {0} cannot be opened: its key file is missing")]
    BackupUnopenable(String),
    #[error("File error: {0}")]
    File(String),
    #[error("Unreadable export file: {0}")]
    ImportFormat(String),
    #[error("Note \"{id}\" unreadable: field \"{field}\" is out of format")]
    CorruptRow { id: String, field: &'static str },
    #[error("Database carrying migration \"{0}\", unknown to this version of DevNotes")]
    SchemaTooRecent(String),
    #[error("Migration failed: {0}")]
    Migration(String),
    /// Its own variant because the answer is: this file will not get better by retrying.
    #[error("The library is damaged: {0}")]
    Damaged(String),
    /// Deriving a key, sealing a value, or opening one that will not open.
    #[error("Vault error: {0}")]
    Vault(String),
    /// Says only that: which of the passphrase and the file is wrong is not for a caller to
    /// narrow down.
    #[error("Wrong passphrase")]
    WrongPassphrase,
    /// A protected export offered without its phrase: not a failure, the interface asks.
    #[error("This file is protected by a passphrase")]
    PassphraseRequired,
    /// A command panicked while holding the connection.
    #[error("Storage unavailable: a previous operation failed")]
    Unavailable,
    /// A command reached the library before the passphrase: a caller that jumped the queue.
    #[error("The library is locked")]
    Locked,
    /// `#[from]`: required by `Connection::transaction`.
    #[error("Storage error: {0}")]
    Sqlite(#[from] diesel::result::Error),
}

/// Turns any failure into a [`StorageError::File`] named after what it was about.
pub(crate) trait FileContext<T> {
    fn context(self, what: impl std::fmt::Display) -> Result<T, StorageError>;
}

impl<T, E: std::fmt::Display> FileContext<T> for Result<T, E> {
    fn context(self, what: impl std::fmt::Display) -> Result<T, StorageError> {
        self.map_err(|error| StorageError::File(format!("{what}: {error}")))
    }
}

/// ⚠️ A new variant breaks the front-end build until `CODE_KEYS`
/// (`core/services/errors/error-notifier.service.ts`) and both locales have its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    NoteNotFound,
    SpaceNotFound,
    DuplicateSpaceName,
    FolderNotFound,
    DuplicateFolderName,
    AttachmentNotFound,
    RevisionNotFound,
    LibraryNotFound,
    LibraryOpen,
    LastLibrary,
    NothingToSetAside,
    BackupNotFound,
    BackupUnopenable,
    FileAccess,
    ImportFormat,
    /// The `field` parameter names the offending field.
    InvalidInput,
    /// Poisoned mutex: a command panicked while holding the connection.
    StorageUnavailable,
    /// The unlock screen clears the field on this one rather than showing a banner.
    WrongPassphrase,
    /// A command ran before the library was unlocked.
    Locked,
    /// The import needs the phrase the export was protected with.
    PassphraseRequired,
    /// SQLite says the file is corrupt: the one code the interface answers with an action.
    LibraryDamaged,
    Storage,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    /// Values to interpolate into the translated message, e.g. `{ "name": "Personal" }`.
    pub params: BTreeMap<String, String>,
    pub detail: String,
}

impl AppError {
    fn new(code: ErrorCode, detail: String) -> Self {
        Self {
            code,
            params: BTreeMap::new(),
            detail,
        }
    }

    fn with(code: ErrorCode, detail: String, key: &str, value: &str) -> Self {
        let mut error = Self::new(code, detail);
        error.params.insert(key.to_string(), value.to_string());
        error
    }

    pub fn storage_unavailable() -> Self {
        Self::new(
            ErrorCode::StorageUnavailable,
            "Storage unavailable: a previous operation failed".to_string(),
        )
    }
}

impl From<ValidationError> for AppError {
    fn from(error: ValidationError) -> Self {
        Self::with(
            ErrorCode::InvalidInput,
            error.to_string(),
            "field",
            error.field,
        )
    }
}

impl From<StorageError> for AppError {
    fn from(error: StorageError) -> Self {
        let detail = error.to_string();

        match error {
            StorageError::NoteNotFound(id) => {
                Self::with(ErrorCode::NoteNotFound, detail, "id", &id)
            }
            StorageError::SpaceNotFound(id) => {
                Self::with(ErrorCode::SpaceNotFound, detail, "id", &id)
            }
            StorageError::DuplicateSpaceName(name) => {
                Self::with(ErrorCode::DuplicateSpaceName, detail, "name", &name)
            }
            StorageError::FolderNotFound(id) => {
                Self::with(ErrorCode::FolderNotFound, detail, "id", &id)
            }
            StorageError::DuplicateFolderName(name) => {
                Self::with(ErrorCode::DuplicateFolderName, detail, "name", &name)
            }
            StorageError::AttachmentNotFound(id) => {
                Self::with(ErrorCode::AttachmentNotFound, detail, "id", &id)
            }
            StorageError::RevisionNotFound(id) => {
                Self::with(ErrorCode::RevisionNotFound, detail, "id", &id)
            }
            StorageError::LibraryNotFound(id) => {
                Self::with(ErrorCode::LibraryNotFound, detail, "id", &id)
            }
            StorageError::LibraryOpen => Self::new(ErrorCode::LibraryOpen, detail),
            StorageError::LastLibrary => Self::new(ErrorCode::LastLibrary, detail),
            StorageError::NothingToSetAside(path) => {
                Self::with(ErrorCode::NothingToSetAside, detail, "path", &path)
            }
            StorageError::BackupNotFound(id) => {
                Self::with(ErrorCode::BackupNotFound, detail, "id", &id)
            }
            StorageError::BackupUnopenable(id) => {
                Self::with(ErrorCode::BackupUnopenable, detail, "id", &id)
            }
            StorageError::Unavailable => Self::new(ErrorCode::StorageUnavailable, detail),
            StorageError::WrongPassphrase => Self::new(ErrorCode::WrongPassphrase, detail),
            StorageError::PassphraseRequired => Self::new(ErrorCode::PassphraseRequired, detail),
            StorageError::Locked => Self::new(ErrorCode::Locked, detail),
            StorageError::Damaged(_) => Self::new(ErrorCode::LibraryDamaged, detail),
            StorageError::File(_) => Self::new(ErrorCode::FileAccess, detail),
            StorageError::ImportFormat(_) => Self::new(ErrorCode::ImportFormat, detail),
            // Nothing here gives the front anything to do beyond reporting the failure.
            StorageError::SchemaTooRecent(_)
            | StorageError::Migration(_)
            | StorageError::Vault(_)
            | StorageError::CorruptRow { .. }
            | StorageError::Sqlite(_) => Self::new(ErrorCode::Storage, detail),
        }
    }
}
