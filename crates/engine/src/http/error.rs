//! What the HTTP client's failures answer: a code and the words the front needs for its sentence.

use super::model::{HeaderFault, HttpItemKind};
use super::runner::DataFault;
use super::scripts::model::{FailureKind, ScriptError};
use super::send::SendError;
use super::transport::{SetBy, readable_delay};
use crate::error::{AppError, ErrorCode};

impl AppError {
    /// `source` picks the sentence — the request's own value, a folder's, the collection's or
    /// the default — and `name` is the folder or the collection that set it.
    pub(crate) fn set_by(self, set_by: &SetBy) -> Self {
        match set_by {
            SetBy::Request => self.and("source", "request"),
            SetBy::Default => self.and("source", "default"),
            SetBy::Above(origin) => {
                let source = match origin.kind {
                    HttpItemKind::Collection => "collection",
                    HttpItemKind::Folder | HttpItemKind::Request => "folder",
                };
                self.and("source", source).and("name", &origin.name)
            }
        }
    }
}

impl From<SendError> for AppError {
    fn from(error: SendError) -> Self {
        let detail = error.to_string();

        match error {
            SendError::Variable(name) => Self::with(ErrorCode::HttpVariable, detail, "name", &name),
            SendError::InvalidUrl(_) => Self::new(ErrorCode::HttpInvalidUrl, detail),
            SendError::Unresolved(_) => Self::new(ErrorCode::HttpUnresolved, detail),
            SendError::Refused(_) => Self::new(ErrorCode::HttpRefused, detail),
            SendError::Tls(_) => Self::new(ErrorCode::HttpTls, detail),
            SendError::Timeout(limit) => {
                let (amount, unit) = readable_delay(limit.value);
                Self::new(ErrorCode::HttpTimeout, detail)
                    .and("amount", &amount.to_string())
                    .and("unit", unit)
                    .set_by(&limit.set_by)
            }
            SendError::TooManyRedirects(limit) => {
                Self::new(ErrorCode::HttpTooManyRedirects, detail)
                    .and("count", &limit.value.to_string())
                    .set_by(&limit.set_by)
            }
            SendError::FileUnreadable(path) => {
                Self::with(ErrorCode::HttpFileUnreadable, detail, "path", &path)
            }
            SendError::FileTooLarge(path) => {
                Self::with(ErrorCode::HttpFileTooLarge, detail, "path", &path)
                    .and("limit", &crate::http::send::SENT_FILE_MEGABYTES.to_string())
            }
            SendError::Cancelled => Self::new(ErrorCode::HttpCancelled, detail),
            SendError::Fell => Self::new(ErrorCode::HttpFell, detail),
            SendError::GraphqlVariables(_) => Self::new(ErrorCode::HttpGraphqlVariables, detail),
            SendError::Handshake(status) => Self::with(
                ErrorCode::HttpWebsocketRefused,
                detail,
                "status",
                &status.to_string(),
            ),
            // The cause's own words: « Erreur réseau » alone tells nothing.
            SendError::Network(cause) => {
                Self::with(ErrorCode::HttpNetwork, detail, "cause", &cause)
            }
            SendError::SocketScheme { twin } => {
                let refused = Self::new(ErrorCode::HttpWebsocketScheme, detail);
                match twin {
                    Some(twin) => refused.and("twin", &twin),
                    None => refused,
                }
            }
            SendError::Header {
                name,
                fault,
                set_by,
            } => {
                let code = match fault {
                    HeaderFault::Name => ErrorCode::HttpHeaderName,
                    HeaderFault::Value => ErrorCode::HttpHeaderValue,
                };
                Self::with(code, detail, "header", &name).set_by(&set_by)
            }
        }
    }
}

impl From<DataFault> for AppError {
    fn from(fault: DataFault) -> Self {
        let detail = fault.to_string();
        let said = match fault {
            DataFault::Unreadable => "unreadable",
            DataFault::Empty => "empty",
            DataFault::TooLarge => "tooLarge",
        };
        Self::with(ErrorCode::HttpRunData, detail, "fault", said)
    }
}

impl From<ScriptError> for AppError {
    fn from(error: ScriptError) -> Self {
        let detail = error.to_string();

        match error {
            ScriptError::Stopped(stop) => {
                let mut refused = Self::with(
                    ErrorCode::HttpScript,
                    detail,
                    "failure",
                    &stop.kind.to_string(),
                );
                for (key, value) in [
                    ("level", stop.level.map(|level| level.to_string())),
                    ("name", stop.name),
                    ("owner", stop.owner),
                    ("line", stop.line.map(|line| line.to_string())),
                ] {
                    if let Some(value) = value {
                        refused = refused.and(key, &value);
                    }
                }
                // What the script threw is its own words; the rest is said by the front.
                if matches!(stop.kind, FailureKind::Error | FailureKind::Syntax) {
                    refused = refused.and("message", &stop.message);
                }
                refused
            }
            ScriptError::Skipped => Self::new(ErrorCode::HttpScriptSkipped, detail),
            ScriptError::File(path) => Self::with(ErrorCode::HttpScriptFile, detail, "path", &path),
            ScriptError::Method(method) => {
                Self::with(ErrorCode::HttpScriptMethod, detail, "method", &method)
            }
            ScriptError::Protected {
                method,
                environment,
            } => Self::with(
                ErrorCode::HttpScriptProtected,
                detail,
                "method",
                &method.to_string(),
            )
            .and("environment", &environment),
        }
    }
}
