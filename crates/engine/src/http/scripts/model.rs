//! What crosses between a request, the script host and the prelude: the job a phase runs, and
//! what it leaves behind. Plain data, read and written as JSON on both sides.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::closed_enum::closed_enum;
use crate::http::model::{
    FormPart, HttpMethod, KeyValue, RequestBody, RequestDocument, RequestKind,
};
use crate::http::response::Cookie;

closed_enum! {
    /// Before the request leaves, or once it is answered: the tests run in the second.
    pub enum Phase {
        #[default]
        Before = "before",
        After = "after",
    }
}

closed_enum! {
    pub enum ScriptKind {
        #[default]
        Before = "before",
        After = "after",
        Tests = "tests",
    }
}

closed_enum! {
    /// Who holds a script.
    pub enum ScriptLevel {
        #[default]
        Request = "request",
        Folder = "folder",
        Collection = "collection",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Script {
    /// Chosen by whoever builds the job: what a failure and a test name it by.
    pub id: String,
    pub level: ScriptLevel,
    pub kind: ScriptKind,
    /// The request, folder or collection that holds it.
    pub name: String,
    /// The folder's or the collection's id; `None` for the request's own.
    pub owner: Option<String>,
    pub source: String,
}

/// A row as a script reads it: Postman's `{ key, value, disabled }`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptRow {
    pub key: String,
    pub value: String,
    pub disabled: bool,
    /// `file` for a multipart part sent from a path.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptFile {
    pub src: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptGraphql {
    pub query: String,
    pub variables: String,
}

/// A body by Postman's modes: `none`, `raw` (with its language), `urlencoded`, `formdata`,
/// `file` and `graphql`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptBody {
    pub mode: String,
    pub raw: String,
    pub language: String,
    pub urlencoded: Vec<ScriptRow>,
    pub formdata: Vec<ScriptRow>,
    pub file: ScriptFile,
    pub graphql: ScriptGraphql,
}

/// The request as a script reads and changes it: its own rows, its `{{variables}}` as typed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<ScriptRow>,
    pub body: ScriptBody,
}

/// What a script changed and Rust cannot send.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RequestFault {
    #[error("The method {0} is not one a request is sent with")]
    Method(String),
}

fn row(key: &str, value: &str, enabled: bool) -> ScriptRow {
    ScriptRow {
        key: key.to_string(),
        value: value.to_string(),
        disabled: !enabled,
        part: None,
    }
}

fn rows_of(rows: &[KeyValue]) -> Vec<ScriptRow> {
    rows.iter()
        .map(|kv| row(&kv.key, &kv.value, kv.enabled))
        .collect()
}

/// The script's rows back as the request's, each keeping the description of the row of its name.
fn key_values(rows: &[ScriptRow], before: &[KeyValue]) -> Vec<KeyValue> {
    rows.iter()
        .map(|row| KeyValue {
            enabled: !row.disabled,
            key: row.key.clone(),
            value: row.value.clone(),
            description: before
                .iter()
                .find(|known| known.key == row.key)
                .map(|known| known.description.clone())
                .unwrap_or_default(),
        })
        .collect()
}

impl ScriptBody {
    fn of(kind: RequestKind, document: &RequestDocument) -> Self {
        let mut body = Self {
            mode: "none".to_string(),
            language: "text".to_string(),
            ..Self::default()
        };
        if kind == RequestKind::Graphql {
            body.mode = "graphql".to_string();
            body.graphql = ScriptGraphql {
                query: document.graphql.query.clone(),
                variables: document.graphql.variables.clone(),
            };
            return body;
        }
        match &document.body {
            RequestBody::None => {}
            RequestBody::Json { text } => {
                body.mode = "raw".to_string();
                body.raw.clone_from(text);
                body.language = "json".to_string();
            }
            RequestBody::Text { text } => {
                body.mode = "raw".to_string();
                body.raw.clone_from(text);
            }
            RequestBody::Form { fields } => {
                body.mode = "urlencoded".to_string();
                body.urlencoded = rows_of(fields);
            }
            RequestBody::Multipart { parts } => {
                body.mode = "formdata".to_string();
                body.formdata = parts
                    .iter()
                    .map(|part| ScriptRow {
                        part: Some(if part.file { "file" } else { "text" }.to_string()),
                        ..row(&part.key, &part.value, part.enabled)
                    })
                    .collect();
            }
            RequestBody::Binary { path } => {
                body.mode = "file".to_string();
                body.file.src.clone_from(path);
            }
        }
        body
    }

    /// The body a request sends for this one; what Rust has no kind for keeps the one it had.
    fn onto(&self, before: &RequestBody) -> RequestBody {
        match self.mode.as_str() {
            "none" => RequestBody::None,
            "raw" if self.language.eq_ignore_ascii_case("json") => RequestBody::Json {
                text: self.raw.clone(),
            },
            "raw" => RequestBody::Text {
                text: self.raw.clone(),
            },
            "urlencoded" => {
                let fields = match before {
                    RequestBody::Form { fields } => fields.as_slice(),
                    _ => &[],
                };
                RequestBody::Form {
                    fields: key_values(&self.urlencoded, fields),
                }
            }
            "formdata" => RequestBody::Multipart {
                parts: self
                    .formdata
                    .iter()
                    .map(|row| FormPart {
                        enabled: !row.disabled,
                        key: row.key.clone(),
                        value: row.value.clone(),
                        file: row.part.as_deref() == Some("file"),
                        description: String::new(),
                    })
                    .collect(),
            },
            "file" => RequestBody::Binary {
                path: self.file.src.clone(),
            },
            _ => before.clone(),
        }
    }
}

impl ScriptRequest {
    #[must_use]
    pub fn of(kind: RequestKind, method: HttpMethod, document: &RequestDocument) -> Self {
        Self {
            method: method.to_string(),
            url: document.url.clone(),
            headers: rows_of(&document.headers),
            body: ScriptBody::of(kind, document),
        }
    }

    /// What the script left, written over the request: its method, URL, headers and body. A
    /// GraphQL request keeps its kind, and takes the query and variables back.
    pub fn onto(
        &self,
        kind: RequestKind,
        document: &mut RequestDocument,
    ) -> Result<HttpMethod, RequestFault> {
        let method = self
            .method
            .trim()
            .to_ascii_uppercase()
            .parse::<HttpMethod>()
            .map_err(|()| RequestFault::Method(self.method.clone()))?;
        document.url.clone_from(&self.url);
        document.headers = key_values(&self.headers, &document.headers);
        if kind == RequestKind::Graphql {
            if self.body.mode == "graphql" {
                document.graphql.query.clone_from(&self.body.graphql.query);
                document
                    .graphql
                    .variables
                    .clone_from(&self.body.graphql.variables);
            }
        } else {
            document.body = self.body.onto(&document.body);
        }
        Ok(method)
    }

    /// A request a script sends by itself, its body by Postman's modes.
    #[must_use]
    pub fn document(&self) -> RequestDocument {
        let mut document = RequestDocument {
            url: self.url.clone(),
            ..RequestDocument::default()
        };
        document.headers = key_values(&self.headers, &[]);
        document.body = self.body.onto(&RequestBody::None);
        document
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptCookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<String>,
    pub http_only: bool,
    pub secure: bool,
}

impl From<&Cookie> for ScriptCookie {
    fn from(cookie: &Cookie) -> Self {
        Self {
            name: cookie.name.clone(),
            value: cookie.value.clone(),
            domain: cookie.domain.clone(),
            path: cookie.path.clone(),
            expires: cookie.expires.clone(),
            http_only: cookie.http_only,
            secure: cookie.secure,
        }
    }
}

/// The answer as a script reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptResponse {
    pub code: u16,
    pub reason: String,
    pub headers: Vec<ScriptRow>,
    /// As text, up to what the response pane shows.
    pub body: String,
    pub millis: u32,
    pub size: u32,
    pub cookies: Vec<ScriptCookie>,
}

closed_enum! {
    /// A level of variables, as the prelude names it; `data` is a row of a run's data file.
    pub enum LayerScope {
        #[default]
        Data = "data",
        Request = "request",
        Folder = "folder",
        Collection = "collection",
        Environment = "environment",
        Globals = "globals",
    }
}

/// One level's variables, raw: a script reads `{{other}}` in a value as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptLayer {
    pub scope: LayerScope,
    pub from: String,
    pub values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptInfo {
    /// From 0.
    pub iteration: u32,
    pub iteration_count: u32,
    pub request_name: String,
    pub request_id: Option<String>,
}

/// One phase of one request: its scripts in the order they run, and what they read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Job {
    pub phase: Phase,
    pub scripts: Vec<Script>,
    pub request: ScriptRequest,
    pub response: Option<ScriptResponse>,
    /// The nearest first: data, request, folders, collection, environment, globals.
    pub layers: Vec<ScriptLayer>,
    /// The active environment's name; `None` with « Aucun », where `env.set` writes the globals.
    pub environment: Option<String>,
    pub cookies: Vec<ScriptCookie>,
    pub info: ScriptInfo,
}

closed_enum! {
    /// Where a script wrote.
    pub enum WriteScope {
        #[default]
        Environment = "environment",
        Globals = "globals",
        Collection = "collection",
        Data = "data",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VariableWrite {
    pub scope: WriteScope,
    pub name: String,
    /// `None` unsets it.
    pub value: Option<String>,
    pub secret: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub skipped: bool,
    pub message: Option<String>,
    /// As the prelude writes a value: a text quoted, the rest as JSON.
    pub expected: Option<String>,
    pub actual: Option<String>,
    /// The script it was written in, and the line of the assertion that failed.
    pub script: Option<String>,
    pub line: Option<u32>,
    /// Filled by Rust from the script: whose it is, and the line of code that failed as written.
    pub kind: Option<ScriptKind>,
    pub level: Option<ScriptLevel>,
    pub holder: Option<String>,
    pub owner: Option<String>,
    pub code: Option<String>,
}

impl TestResult {
    /// Placed in the script that wrote it, `scripts` the phase's.
    #[must_use]
    pub fn placed(self, scripts: &[Script]) -> Self {
        let Some(script) = self
            .script
            .as_deref()
            .and_then(|id| scripts.iter().find(|script| script.id == id))
        else {
            return self;
        };
        let code = self.line.and_then(|line| {
            let index = usize::try_from(line).ok()?.checked_sub(1)?;
            script
                .source
                .lines()
                .nth(index)
                .map(|code| code.trim().to_string())
        });
        Self {
            kind: Some(script.kind),
            level: Some(script.level),
            holder: Some(script.name.clone()),
            owner: script.owner.clone(),
            code,
            ..self
        }
    }
}

closed_enum! {
    pub enum FailureKind {
        /// Thrown by the script.
        #[default]
        Error = "error",
        /// It does not read as JavaScript.
        Syntax = "syntax",
        /// Past its time: an endless loop, or a wait that never ends.
        Timeout = "timeout",
        Memory = "memory",
        /// The script host could not be started, or stopped.
        Host = "host",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptFailure {
    pub kind: FailureKind,
    pub script: Option<String>,
    pub line: Option<u32>,
    pub message: String,
}

/// `pm.execution.setNextRequest`: `set` once called, `name` `None` to stop a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NextRequest {
    pub set: bool,
    pub name: Option<String>,
}

/// What a phase left: the request as changed (before it is sent), the variables written, the
/// tests, why it stopped.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Outcome {
    pub request: Option<ScriptRequest>,
    pub writes: Vec<VariableWrite>,
    pub tests: Vec<TestResult>,
    pub failure: Option<ScriptFailure>,
    pub skip: bool,
    pub next: NextRequest,
    /// `pm.variables` as the phase left them: the nearest level the request resolves from.
    pub locals: BTreeMap<String, String>,
}

impl Outcome {
    #[must_use]
    pub fn failed(failure: ScriptFailure) -> Self {
        Self {
            failure: Some(failure),
            ..Self::default()
        }
    }
}

closed_enum! {
    pub enum LogLevel {
        #[default]
        Log = "log",
        Info = "info",
        Debug = "debug",
        Warn = "warn",
        Error = "error",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LogEntry {
    pub level: LogLevel,
    pub text: String,
    pub script: Option<String>,
    /// When the host said it: set on this side, the console's time.
    #[serde(skip)]
    pub at: Option<chrono::DateTime<chrono::Utc>>,
}

/// A script that stopped, as the front names it: what failed, where, and whose script it was.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptStop {
    pub kind: FailureKind,
    pub level: Option<ScriptLevel>,
    pub script: Option<ScriptKind>,
    /// The request, folder or collection that holds it.
    pub name: Option<String>,
    /// The folder's or the collection's id, for « Ouvrir le script ».
    pub owner: Option<String>,
    pub line: Option<u32>,
    pub message: String,
}

impl ScriptStop {
    /// `scripts` the phase's, which the failure names by id.
    #[must_use]
    pub fn of(failure: &ScriptFailure, scripts: &[Script]) -> Self {
        let script = failure
            .script
            .as_deref()
            .and_then(|id| scripts.iter().find(|script| script.id == id))
            .or_else(|| (scripts.len() == 1).then(|| &scripts[0]));
        Self {
            kind: failure.kind,
            level: script.map(|script| script.level),
            script: script.map(|script| script.kind),
            name: script.map(|script| script.name.clone()),
            owner: script.and_then(|script| script.owner.clone()),
            line: failure.line,
            message: failure.message.clone(),
        }
    }
}

/// Why the scripts kept a request from leaving.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScriptError {
    #[error("A script before the send stopped: {}", .0.message)]
    Stopped(ScriptStop),
    #[error("A script skipped the request")]
    Skipped,
    #[error("A script set the method {0}, which no request is sent with")]
    Method(String),
    #[error("The scripts made the request a {method} on {environment}, which is protected")]
    Protected {
        method: HttpMethod,
        environment: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Extracted {
    pub path: String,
    pub variable: String,
    /// `false`: the path led nowhere, the variable was left as it was.
    pub found: bool,
}

/// What the scripts after the answer left beside it; what they logged is the console's.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptsAnswer {
    pub failure: Option<ScriptStop>,
    /// The tests, in the order they ran: kept with the history entry.
    pub tests: Vec<TestResult>,
    /// Each extraction of the request, and whether its path still read.
    pub extracted: Vec<Extracted>,
    /// A script wrote a variable: the environments and the settings are read again.
    pub wrote: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::graphql::GraphqlDocument;

    fn header(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.to_string(),
            value: value.to_string(),
            description: format!("about {key}"),
            ..KeyValue::default()
        }
    }

    #[test]
    fn a_request_goes_to_a_script_and_comes_back_with_what_it_changed() {
        let mut document = RequestDocument {
            url: "{{baseUrl}}/pay".to_string(),
            headers: vec![header("Accept", "application/json")],
            body: RequestBody::Json {
                text: "{\"amount\":1}".to_string(),
            },
            ..RequestDocument::default()
        };
        let mut request = ScriptRequest::of(RequestKind::Http, HttpMethod::Post, &document);
        assert_eq!(request.body.mode, "raw");
        assert_eq!(request.body.language, "json");

        request.method = "put".to_string();
        request.url.push_str("?retry=1");
        request.headers.push(row("X-Signature", "3f9a", true));
        request.body.raw = "{\"amount\":2}".to_string();
        let method = request.onto(RequestKind::Http, &mut document).unwrap();

        assert_eq!(method, HttpMethod::Put);
        assert_eq!(document.url, "{{baseUrl}}/pay?retry=1");
        assert_eq!(document.headers[0].description, "about Accept");
        assert_eq!(document.headers[1].key, "X-Signature");
        assert_eq!(
            document.body,
            RequestBody::Json {
                text: "{\"amount\":2}".to_string()
            }
        );
    }

    #[test]
    fn a_method_rust_does_not_send_is_refused_by_its_name() {
        let mut document = RequestDocument::default();
        let mut request = ScriptRequest::of(RequestKind::Http, HttpMethod::Get, &document);
        request.method = "PURGE".to_string();
        assert_eq!(
            request.onto(RequestKind::Http, &mut document),
            Err(RequestFault::Method("PURGE".to_string()))
        );
    }

    #[test]
    fn every_body_reads_by_its_postman_mode_and_back() {
        let bodies = [
            RequestBody::None,
            RequestBody::Text {
                text: "hello".to_string(),
            },
            RequestBody::Form {
                fields: vec![header("q", "1")],
            },
            RequestBody::Multipart {
                parts: vec![FormPart {
                    key: "file".to_string(),
                    value: "/tmp/a.png".to_string(),
                    file: true,
                    ..FormPart::default()
                }],
            },
            RequestBody::Binary {
                path: "/tmp/a.bin".to_string(),
            },
        ];
        for body in bodies {
            let mut document = RequestDocument {
                body: body.clone(),
                ..RequestDocument::default()
            };
            let request = ScriptRequest::of(RequestKind::Http, HttpMethod::Post, &document);
            request.onto(RequestKind::Http, &mut document).unwrap();
            assert_eq!(document.body, body);
        }
    }

    #[test]
    fn a_graphql_request_hands_its_query_and_keeps_its_kind() {
        let mut document = RequestDocument {
            graphql: GraphqlDocument {
                query: "{ me { id } }".to_string(),
                ..GraphqlDocument::default()
            },
            ..RequestDocument::default()
        };
        let mut request = ScriptRequest::of(RequestKind::Graphql, HttpMethod::Post, &document);
        assert_eq!(request.body.mode, "graphql");
        request.body.graphql.query = "{ me { name } }".to_string();
        request.onto(RequestKind::Graphql, &mut document).unwrap();
        assert_eq!(document.graphql.query, "{ me { name } }");
        assert_eq!(document.body, RequestBody::None);
    }
}
