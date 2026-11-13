//! What the error banner says: the last thing that failed, one at a time, so banners do not
//! stack.

use engine::error::{AppError, ErrorCode};
use i18n::{Key, Message, keys};

#[derive(Debug, Default)]
pub struct ErrorNotifier {
    notice: Option<Message>,
}

impl ErrorNotifier {
    /// `fallback` says what was attempted; a cause the core named says it better. The detail is
    /// the log's, never the screen's: it is the core's English, a path perhaps.
    pub fn report(&mut self, fallback: Key, error: &AppError) {
        log::warn!("{}: {}", fallback.path(), error.detail);
        self.notice = Some(message_of(error).unwrap_or_else(|| Message::new(fallback)));
    }

    /// The same, the fallback carrying what was typed: « Folder “Runbooks” could not be
    /// created ».
    pub fn report_message(&mut self, fallback: Message, error: &AppError) {
        log::warn!("{}: {}", fallback.key.path(), error.detail);
        self.notice = Some(message_of(error).unwrap_or(fallback));
    }

    /// What is refused before the core is asked: nothing to log.
    pub fn notify(&mut self, message: Message) {
        self.notice = Some(message);
    }

    pub fn notice(&self) -> Option<&Message> {
        self.notice.as_ref()
    }

    pub fn dismiss(&mut self) {
        self.notice = None;
    }
}

/// The sentence an error's code says, with its values; `None` when the action's own message
/// says it better.
pub fn message_of(error: &AppError) -> Option<Message> {
    let key = key_of(error)?;
    Some(
        error
            .params
            .iter()
            .fold(Message::new(key), |message, (name, value)| {
                message.with(name.as_str(), value.as_str())
            }),
    )
}

/// Exhaustive: a code added to the core does not compile until its sentence is decided.
fn key_of(error: &AppError) -> Option<Key> {
    use keys::errors as e;
    Some(match error.code {
        ErrorCode::NoteNotFound => e::NOTE_GONE,
        ErrorCode::SpaceNotFound => e::SPACE_GONE,
        ErrorCode::DuplicateSpaceName => e::SPACE_NAME_TAKEN,
        ErrorCode::FolderNotFound => e::FOLDER_GONE,
        ErrorCode::DuplicateFolderName => e::FOLDER_NAME_TAKEN,
        ErrorCode::AttachmentNotFound => e::ATTACHMENT_GONE,
        ErrorCode::HttpItemNotFound => e::HTTP_ITEM_GONE,
        ErrorCode::HttpEnvironmentNotFound => e::HTTP_ENVIRONMENT_GONE,
        ErrorCode::DuplicateEnvironmentName => e::ENVIRONMENT_NAME_TAKEN,
        ErrorCode::RevisionNotFound => e::REVISION_GONE,
        ErrorCode::LibraryNotFound => e::LIBRARY_GONE,
        ErrorCode::LibraryOpen => e::LIBRARY_OPEN,
        ErrorCode::LastLibrary => e::LAST_LIBRARY,
        ErrorCode::NothingToSetAside => e::NOTHING_TO_SET_ASIDE,
        ErrorCode::BackupNotFound => e::BACKUP_GONE,
        ErrorCode::BackupUnopenable => e::BACKUP_UNOPENABLE,
        ErrorCode::FileAccess => e::FILE_ACCESS,
        ErrorCode::FileTooLarge => e::FILE_TOO_LARGE,
        ErrorCode::ImportFormat => e::IMPORT_FORMAT,
        // A field of the code is no sentence: the action's own message says it better.
        ErrorCode::InvalidInput | ErrorCode::Storage => return None,
        ErrorCode::StorageUnavailable => e::STORAGE_UNAVAILABLE,
        ErrorCode::WrongPassphrase => e::WRONG_PASSPHRASE,
        ErrorCode::Locked => e::LOCKED,
        ErrorCode::PassphraseRequired => e::PASSPHRASE_REQUIRED,
        ErrorCode::LibraryDamaged => e::LIBRARY_DAMAGED,
        ErrorCode::HttpVariable => e::HTTP_VARIABLE,
        ErrorCode::HttpInvalidUrl => e::HTTP_INVALID_URL,
        ErrorCode::HttpUnresolved => e::HTTP_UNRESOLVED,
        ErrorCode::HttpRefused => e::HTTP_REFUSED,
        ErrorCode::HttpTls => e::HTTP_TLS,
        ErrorCode::HttpFileUnreadable => e::HTTP_FILE_UNREADABLE,
        ErrorCode::HttpFileTooLarge => e::HTTP_FILE_TOO_LARGE,
        ErrorCode::HttpCancelled => e::HTTP_CANCELLED,
        ErrorCode::HttpGraphqlVariables => e::HTTP_GRAPHQL_VARIABLES,
        ErrorCode::HttpWebsocketRefused => e::HTTP_WEBSOCKET_REFUSED,
        ErrorCode::HttpNetwork => e::HTTP_NETWORK,
        ErrorCode::HttpWebsocketScheme => e::HTTP_WEBSOCKET_SCHEME,
        ErrorCode::HttpScriptSkipped => e::HTTP_SCRIPT_SKIPPED,
        ErrorCode::HttpScriptMethod => e::HTTP_SCRIPT_METHOD,
        ErrorCode::HttpScriptProtected => e::HTTP_SCRIPT_PROTECTED,
        ErrorCode::HttpScriptFile => e::HTTP_SCRIPT_FILE,
        // A sentence per value of one parameter: whose delay ran out, which script failed how.
        ErrorCode::HttpTimeout => variant(error, "errors.httpTimeout", "source")?,
        ErrorCode::HttpTooManyRedirects => variant(error, "errors.httpTooManyRedirects", "source")?,
        ErrorCode::HttpHeaderName => variant(error, "errors.httpHeaderName", "source")?,
        ErrorCode::HttpHeaderValue => variant(error, "errors.httpHeaderValue", "source")?,
        ErrorCode::HttpScript => variant(error, "errors.httpScript", "failure")?,
        ErrorCode::HttpRunData => variant(error, "errors.httpRunData", "fault")?,
    })
}

/// `errors.httpTimeout.folder`: the sub-key the parameter names, `request` when it names none.
fn variant(error: &AppError, group: &str, parameter: &str) -> Option<Key> {
    let value = error
        .params
        .get(parameter)
        .map_or("request", String::as_str);
    Key::find(&format!("{group}.{value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_the_core_named_is_its_sentence_with_its_values() {
        let mut errors = ErrorNotifier::default();
        let error = AppError::from(engine::error::StorageError::WrongPassphrase);

        errors.report(keys::errors::UNLOCK_FAILED, &error);

        assert_eq!(errors.notice().unwrap().key, keys::errors::WRONG_PASSPHRASE);
    }

    #[test]
    fn an_error_without_a_sentence_says_what_was_attempted() {
        let mut errors = ErrorNotifier::default();
        let error = AppError::from(engine::error::ValidationError::new("title", "too long"));

        errors.report(keys::errors::NOTE_SAVE_FAILED, &error);

        assert_eq!(errors.notice().unwrap().key, keys::errors::NOTE_SAVE_FAILED);
        errors.dismiss();
        assert!(errors.notice().is_none());
    }

    #[test]
    fn a_variant_is_chosen_by_its_parameter() {
        let mut error = AppError::from(engine::error::StorageError::WrongPassphrase);
        error.code = ErrorCode::HttpTimeout;
        error
            .params
            .insert("source".to_string(), "folder".to_string());
        error.params.insert("name".to_string(), "API".to_string());

        let message = message_of(&error).unwrap();

        assert_eq!(message.key.path(), "errors.httpTimeout.folder");
        assert!(message.text(i18n::Language::En).contains("API"));
        error.params.remove("source");
        assert_eq!(
            message_of(&error).unwrap().key.path(),
            "errors.httpTimeout.request"
        );
    }
}
