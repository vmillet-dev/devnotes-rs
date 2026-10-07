//! A request sent by Rust: the `WebView` reaches no network, and will not. What it inherits is read
//! under the lock; the request is composed and sent without it, so a slow server stalls nothing.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

use base64::Engine;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::task::AbortHandle;
use url::Url;
use uuid::Uuid;

use super::graphql::{self, GraphqlResult};
use super::model::{
    HttpMethod, KeyPlace, KeyValue, RequestAuth, RequestBody, RequestDocument, RequestKind,
};
use super::response::{self, Cookie};
use super::settings::{Inherited, implied_content_type};
use crate::notes::language::Language;
use crate::notes::placeholder;

/// What crosses to be shown: past it the text is cut, and the whole is offered as a file.
pub const SHOWN_LIMIT: usize = 1024 * 1024;
/// What is read at all: past it the reading stops.
pub const KEPT_LIMIT: usize = 64 * 1024 * 1024;
const DEFAULT_TIMEOUT_MS: u32 = 30_000;
const MAX_REDIRECTS: usize = 10;

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    /// Chosen by the front, so it can cancel the send it started and save what came back.
    pub id: String,
    pub kind: RequestKind,
    pub method: HttpMethod,
    pub document: RequestDocument,
    /// Where the request sits, for what it inherits; `None` for a draft not placed yet.
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    /// For the history: what the request is called, and which one it is once saved.
    pub name: String,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Redirect {
    pub status: u16,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SentResponse {
    pub status: u16,
    /// The status's canonical words, « OK »; empty for a code with none.
    pub reason: String,
    pub millis: u32,
    /// Bytes received.
    pub size: u32,
    /// Where the answer came from, after the redirects.
    pub url: String,
    pub headers: Vec<KeyValue>,
    /// The body as text, up to `SHOWN_LIMIT`; empty when it is not text.
    pub body: String,
    pub binary: bool,
    /// The text shown is not the whole body.
    pub cut: bool,
    /// The reading stopped at `KEPT_LIMIT`: even the file is not the whole body.
    pub incomplete: bool,
    pub redirects: Vec<Redirect>,
    /// What the body is coloured and saved as.
    pub language: Language,
    /// A JSON body laid out; `None` for another, or one cut short.
    pub pretty: Option<String>,
    pub cookies: Vec<Cookie>,
    /// The request as it left, for the timeline.
    pub exchange: Exchange,
    /// A GraphQL answer read apart: its `data` and its `errors`.
    pub graphql: Option<GraphqlResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Exchange {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub body: SentBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SentBody {
    None,
    /// Up to `SHOWN_LIMIT`.
    Text {
        text: String,
    },
    Bytes {
        size: u32,
    },
    Multipart {
        fields: Vec<String>,
    },
}

/// The image an answer's bytes make, inline: past this, it is saved rather than shown.
const IMAGE_LIMIT: usize = 10 * 1024 * 1024;

/// Why nothing came back, each a code the front translates.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SendError {
    /// Environments resolve them (v0.11); until then a request carrying one cannot leave.
    #[error("The variable {0} has no value")]
    Variable(String),
    #[error("The URL cannot be sent: {0}")]
    InvalidUrl(String),
    #[error("The host name does not resolve: {0}")]
    Unresolved(String),
    #[error("The connection was refused: {0}")]
    Refused(String),
    #[error("The secure connection failed: {0}")]
    Tls(String),
    #[error("No answer before the timeout")]
    Timeout,
    #[error("Too many redirects")]
    TooManyRedirects,
    #[error("The file {0} cannot be read")]
    FileUnreadable(String),
    #[error("The send was cancelled")]
    Cancelled,
    #[error("The GraphQL variables are not a JSON object: {0}")]
    GraphqlVariables(String),
    #[error("Network error: {0}")]
    Network(String),
}

/// What will go on the wire, composed before anything is sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outgoing {
    pub method: HttpMethod,
    pub url: Url,
    pub headers: Vec<(String, String)>,
    pub body: OutgoingBody,
    /// The API key's name, lowercased: a header or a query pair the history masks.
    pub secrets: Vec<String>,
    /// Its answer is read as GraphQL.
    pub graphql: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutgoingBody {
    None,
    Bytes(Vec<u8>),
    /// Text fields, and files already read.
    Multipart(Vec<(String, Part)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Text(String),
    File { name: String, bytes: Vec<u8> },
}

fn sent(rows: &[KeyValue]) -> impl Iterator<Item = &KeyValue> {
    rows.iter()
        .filter(|row| row.enabled && !row.key.trim().is_empty())
}

fn read(path: &str) -> Result<Vec<u8>, SendError> {
    std::fs::read(path).map_err(|_| SendError::FileUnreadable(path.to_string()))
}

/// The first `{{variable}}` of what will be sent, in reading order.
fn first_variable(
    kind: RequestKind,
    document: &RequestDocument,
    inherited: &Inherited,
) -> Option<String> {
    let none = BTreeMap::new();
    let mut texts: Vec<&str> = vec![&document.url];
    for row in inherited
        .headers
        .iter()
        .map(|known| &known.header)
        .chain(sent(&document.headers))
    {
        texts.extend([row.key.as_str(), row.value.as_str()]);
    }
    let auth = match &document.auth {
        RequestAuth::Inherit => &inherited.auth,
        own => own,
    };
    match auth {
        RequestAuth::Basic { username, password } => {
            texts.extend([username.as_str(), password.as_str()]);
        }
        RequestAuth::Bearer { token } => texts.push(token),
        RequestAuth::ApiKey { name, value, .. } => texts.extend([name.as_str(), value.as_str()]),
        RequestAuth::Inherit | RequestAuth::None => {}
    }
    if kind == RequestKind::Graphql {
        texts.extend([
            document.graphql.query.as_str(),
            document.graphql.variables.as_str(),
        ]);
        return texts
            .into_iter()
            .find_map(|text| placeholder::parse(text, &none).into_iter().next())
            .map(|found| found.name);
    }
    match &document.body {
        RequestBody::Json { text } | RequestBody::Text { text } => texts.push(text),
        RequestBody::Form { fields } => {
            for row in sent(fields) {
                texts.extend([row.key.as_str(), row.value.as_str()]);
            }
        }
        RequestBody::Multipart { parts } => {
            for part in parts.iter().filter(|part| part.enabled) {
                texts.extend([part.key.as_str(), part.value.as_str()]);
            }
        }
        RequestBody::None | RequestBody::Binary { .. } => {}
    }
    texts
        .into_iter()
        .find_map(|text| placeholder::parse(text, &none).into_iter().next())
        .map(|found| found.name)
}

/// `api.example.com/users` is sent as `http://api.example.com/users`, as a browser would.
fn parse_url(typed: &str) -> Result<Url, SendError> {
    let typed = typed.trim();
    let url = if typed.contains("://") {
        Url::parse(typed)
    } else {
        Url::parse(&format!("http://{typed}"))
    };
    let url = url.map_err(|error| SendError::InvalidUrl(error.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none_or(str::is_empty) {
        return Err(SendError::InvalidUrl(url.to_string()));
    }
    Ok(url)
}

/// The request as it will be sent: the headers inherited, then its own over them by name, the
/// auth resolved, the body built and its files read.
pub fn compose(
    kind: RequestKind,
    method: HttpMethod,
    document: &RequestDocument,
    inherited: &Inherited,
) -> Result<Outgoing, SendError> {
    if let Some(name) = first_variable(kind, document, inherited) {
        return Err(SendError::Variable(name));
    }
    let mut url = parse_url(&document.url)?;
    let graphql = (kind == RequestKind::Graphql).then_some(&document.graphql);
    let method = match graphql {
        Some(graphql) if graphql.as_get => HttpMethod::Get,
        Some(_) => HttpMethod::Post,
        None => method,
    };

    let mut headers: Vec<(String, String)> = Vec::new();
    let mut set = |key: &str, value: &str| {
        headers.retain(|(known, _)| !known.eq_ignore_ascii_case(key));
        headers.push((key.to_string(), value.to_string()));
    };
    for known in &inherited.headers {
        set(known.header.key.trim(), &known.header.value);
    }
    for row in sent(&document.headers) {
        set(row.key.trim(), &row.value);
    }
    let typed = |headers: &[(String, String)], name: &str| {
        headers
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case(name))
    };

    let auth = match &document.auth {
        RequestAuth::Inherit => &inherited.auth,
        own => own,
    };
    match auth {
        RequestAuth::Basic { username, password } if !typed(&headers, "authorization") => {
            let pair =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            headers.push(("Authorization".to_string(), format!("Basic {pair}")));
        }
        RequestAuth::Bearer { token } if !typed(&headers, "authorization") => {
            headers.push((
                "Authorization".to_string(),
                format!("Bearer {}", token.trim()),
            ));
        }
        RequestAuth::ApiKey { name, value, place } if !name.trim().is_empty() => match place {
            KeyPlace::Header if !typed(&headers, name.trim()) => {
                headers.push((name.trim().to_string(), value.clone()));
            }
            KeyPlace::Header => {}
            KeyPlace::Query => {
                url.query_pairs_mut().append_pair(name.trim(), value);
            }
        },
        _ => {}
    }

    let (body, implied) = match graphql {
        Some(graphql) => {
            let variables = graphql::variables(graphql).map_err(SendError::GraphqlVariables)?;
            if graphql.as_get {
                let mut pairs = url.query_pairs_mut();
                pairs.append_pair("query", &graphql.query);
                if let Some(variables) = variables {
                    pairs.append_pair("variables", variables);
                }
                if let Some(name) = graphql
                    .operation_name
                    .as_deref()
                    .filter(|name| !name.is_empty())
                {
                    pairs.append_pair("operationName", name);
                }
                (OutgoingBody::None, None)
            } else {
                let body = graphql::body(graphql, variables).into_bytes();
                (OutgoingBody::Bytes(body), Some("application/json"))
            }
        }
        // A multipart type carries its boundary, which reqwest alone knows.
        None => (
            body_of(&document.body)?,
            implied_content_type(&document.body)
                .filter(|_| !matches!(document.body, RequestBody::Multipart { .. })),
        ),
    };
    if let Some(implied) = implied
        && !typed(&headers, "content-type")
    {
        headers.push(("Content-Type".to_string(), implied.to_string()));
    }

    let secrets = match auth {
        RequestAuth::ApiKey { name, .. } if !name.trim().is_empty() => {
            vec![name.trim().to_ascii_lowercase()]
        }
        _ => Vec::new(),
    };

    Ok(Outgoing {
        method,
        url,
        headers,
        body,
        secrets,
        graphql: graphql.is_some(),
    })
}

fn body_of(body: &RequestBody) -> Result<OutgoingBody, SendError> {
    Ok(match body {
        RequestBody::None => OutgoingBody::None,
        RequestBody::Json { text } | RequestBody::Text { text } => {
            OutgoingBody::Bytes(text.clone().into_bytes())
        }
        RequestBody::Form { fields } => {
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(sent(fields).map(|row| (row.key.trim(), row.value.as_str())))
                .finish();
            OutgoingBody::Bytes(encoded.into_bytes())
        }
        RequestBody::Multipart { parts } => {
            let mut fields = Vec::new();
            for part in parts
                .iter()
                .filter(|part| part.enabled && !part.key.trim().is_empty())
            {
                let value = if part.file {
                    Part::File {
                        name: Path::new(&part.value)
                            .file_name()
                            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
                        bytes: read(&part.value)?,
                    }
                } else {
                    Part::Text(part.value.clone())
                };
                fields.push((part.key.trim().to_string(), value));
            }
            OutgoingBody::Multipart(fields)
        }
        RequestBody::Binary { path } => OutgoingBody::Bytes(read(path)?),
    })
}

/// Its type says so, or failing one, its bytes read as UTF-8.
fn is_text(content_type: Option<&str>, bytes: &[u8]) -> bool {
    let Some(kind) = content_type.map(str::to_ascii_lowercase) else {
        return std::str::from_utf8(bytes).is_ok();
    };
    let essence = kind.split(';').next().unwrap_or_default().trim();
    if essence.starts_with("text/")
        || [
            "json",
            "xml",
            "javascript",
            "x-www-form-urlencoded",
            "graphql",
            "yaml",
        ]
        .iter()
        .any(|word| essence.contains(word))
    {
        return true;
    }
    if ["image/", "audio/", "video/", "font/"]
        .iter()
        .any(|family| essence.starts_with(family))
        || essence == "application/octet-stream"
        || essence == "application/pdf"
        || essence == "application/zip"
    {
        return false;
    }
    std::str::from_utf8(bytes).is_ok()
}

/// What failed, read from the error and the chain under it: reqwest names neither DNS nor TLS.
fn failure(error: &reqwest::Error) -> SendError {
    if error.is_timeout() {
        return SendError::Timeout;
    }
    if error.is_redirect() {
        return SendError::TooManyRedirects;
    }
    if error.is_builder() {
        return SendError::InvalidUrl(error.to_string());
    }
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(current) = cause {
        if current.downcast_ref::<rustls::Error>().is_some() {
            return SendError::Tls(current.to_string());
        }
        if let Some(io) = current.downcast_ref::<std::io::Error>() {
            match io.kind() {
                std::io::ErrorKind::ConnectionRefused => return SendError::Refused(io.to_string()),
                std::io::ErrorKind::TimedOut => return SendError::Timeout,
                _ => {}
            }
            // ⚠️ `io::Error::source` skips the error it wraps, where rustls puts its own.
            if let Some(inner) = io.get_ref() {
                cause = Some(inner);
                continue;
            }
        }
        let text = current.to_string().to_ascii_lowercase();
        if text.contains("dns error")
            || text.contains("failed to lookup")
            || text.contains("no such host")
        {
            return SendError::Unresolved(current.to_string());
        }
        if text.contains("certificate") || text.contains("invalid peer") {
            return SendError::Tls(current.to_string());
        }
        cause = current.source();
    }
    SendError::Network(error.to_string())
}

fn wire_method(method: HttpMethod) -> reqwest::Method {
    match method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
    }
}

fn install_crypto() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // `Err` is another provider already installed: one is all a process needs.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// The answer, and every byte of its body that was read.
struct Received {
    response: SentResponse,
    kept: Kept,
}

struct Kept {
    content_type: Option<String>,
    bytes: Vec<u8>,
}

pub fn exchange(outgoing: &Outgoing) -> Exchange {
    let body = match &outgoing.body {
        OutgoingBody::None => SentBody::None,
        OutgoingBody::Bytes(bytes) => match std::str::from_utf8(bytes) {
            Ok(text) => SentBody::Text {
                text: text.chars().take(SHOWN_LIMIT).collect(),
            },
            Err(_) => SentBody::Bytes {
                size: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
            },
        },
        OutgoingBody::Multipart(parts) => SentBody::Multipart {
            fields: parts.iter().map(|(key, _)| key.clone()).collect(),
        },
    };
    Exchange {
        method: outgoing.method,
        url: outgoing.url.to_string(),
        headers: outgoing
            .headers
            .iter()
            .map(|(key, value)| KeyValue {
                key: key.clone(),
                value: value.clone(),
                ..KeyValue::default()
            })
            .collect(),
        body,
    }
}

fn request(client: &reqwest::Client, outgoing: Outgoing) -> reqwest::RequestBuilder {
    let mut request = client.request(wire_method(outgoing.method), outgoing.url);
    for (key, value) in &outgoing.headers {
        request = request.header(key, value);
    }
    match outgoing.body {
        OutgoingBody::None => request,
        OutgoingBody::Bytes(bytes) => request.body(bytes),
        OutgoingBody::Multipart(parts) => {
            let mut form = reqwest::multipart::Form::new();
            for (key, part) in parts {
                form = match part {
                    Part::Text(text) => form.text(key, text),
                    Part::File { name, bytes } => {
                        form.part(key, reqwest::multipart::Part::bytes(bytes).file_name(name))
                    }
                };
            }
            request.multipart(form)
        }
    }
}

async fn send(outgoing: Outgoing) -> Result<Received, SendError> {
    install_crypto();
    let redirects: Arc<Mutex<Vec<Redirect>>> = Arc::default();
    let seen = Arc::clone(&redirects);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(u64::from(DEFAULT_TIMEOUT_MS)))
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() > MAX_REDIRECTS {
                return attempt.error("too many redirects");
            }
            if let Ok(mut seen) = seen.lock() {
                seen.push(Redirect {
                    status: attempt.status().as_u16(),
                    url: attempt.url().to_string(),
                });
            }
            attempt.follow()
        }))
        .build()
        .map_err(|error| SendError::Network(error.to_string()))?;

    let exchange = exchange(&outgoing);
    let is_graphql = outgoing.graphql;
    let request = request(&client, outgoing);

    let started = Instant::now();
    let mut response = request.send().await.map_err(|error| failure(&error))?;
    let status = response.status();
    let url = response.url().to_string();
    let headers: Vec<KeyValue> = response
        .headers()
        .iter()
        .map(|(key, value)| KeyValue {
            key: key.to_string(),
            value: String::from_utf8_lossy(value.as_bytes()).into_owned(),
            ..KeyValue::default()
        })
        .collect();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let mut bytes = Vec::new();
    let mut incomplete = false;
    while let Some(chunk) = response.chunk().await.map_err(|error| failure(&error))? {
        let room = KEPT_LIMIT - bytes.len();
        if chunk.len() > room {
            bytes.extend_from_slice(&chunk[..room]);
            incomplete = true;
            break;
        }
        bytes.extend_from_slice(&chunk);
    }
    let millis = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);
    let binary = !is_text(content_type.as_deref(), &bytes);
    let shown = if binary {
        String::new()
    } else {
        // Cut on a character, never inside one.
        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(SHOWN_LIMIT)]).into_owned();
        text.trim_end_matches('\u{FFFD}').to_string()
    };
    let cut = binary || bytes.len() > SHOWN_LIMIT || incomplete;
    let language = response::language(content_type.as_deref(), &shown);
    let pretty = (language == Language::Json && !cut)
        .then(|| response::pretty_json(&shown))
        .flatten();
    let graphql = (is_graphql && !cut)
        .then(|| graphql::result(&shown))
        .flatten();

    Ok(Received {
        response: SentResponse {
            status: status.as_u16(),
            reason: status.canonical_reason().unwrap_or_default().to_string(),
            millis,
            size: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
            url,
            cookies: response::cookies(&headers),
            headers,
            cut,
            body: shown,
            binary,
            incomplete,
            redirects: redirects
                .lock()
                .map(|seen| seen.clone())
                .unwrap_or_default(),
            language,
            pretty,
            exchange,
            graphql,
        },
        kept: Kept {
            content_type,
            bytes,
        },
    })
}

/// The sends under way and the last body of each, by the id the front gave: what « Annuler »
/// aborts, and what « Enregistrer dans un fichier » writes.
#[derive(Default)]
pub struct Sending {
    /// Numbered, so a send replaced under the same id does not clear its successor's entry.
    running: Mutex<HashMap<String, (u64, AbortHandle)>>,
    started: AtomicU64,
    bodies: Mutex<HashMap<String, Arc<Kept>>>,
}

impl Sending {
    /// A task of its own, so cancelling aborts it wherever it waits.
    pub async fn run(&self, id: &str, outgoing: Outgoing) -> Result<SentResponse, SendError> {
        let task = tauri::async_runtime::spawn(send(outgoing));
        let number = self.started.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut running) = self.running.lock()
            && let Some((_, previous)) =
                running.insert(id.to_string(), (number, task.inner().abort_handle()))
        {
            previous.abort();
        }
        let answer = task.await;
        if let Ok(mut running) = self.running.lock()
            && running
                .get(id)
                .is_some_and(|(current, _)| *current == number)
        {
            running.remove(id);
        }
        let received = answer.map_err(|_| SendError::Cancelled)??;
        if let Ok(mut bodies) = self.bodies.lock() {
            bodies.insert(id.to_string(), Arc::new(received.kept));
        }
        Ok(received.response)
    }

    /// `false` when nothing was under way under that id.
    pub fn cancel(&self, id: &str) -> bool {
        self.running
            .lock()
            .ok()
            .and_then(|running| running.get(id).map(|(_, task)| task.abort()))
            .is_some()
    }

    /// Staged then renamed: a half-written file never takes the name.
    pub fn save(&self, id: &str, path: &Path) -> Result<bool, std::io::Error> {
        let Some(kept) = self.kept(id) else {
            return Ok(false);
        };
        let mut staged = path.as_os_str().to_owned();
        staged.push(format!(".{}.tmp", Uuid::new_v4()));
        std::fs::write(&staged, &kept.bytes)?;
        std::fs::rename(&staged, path).inspect_err(|_| {
            let _ = std::fs::remove_file(&staged);
        })?;
        Ok(true)
    }

    /// An image answer as a `data:` URI, which the page shows; `None` for anything else.
    pub fn image(&self, id: &str) -> Option<String> {
        let kept = self.kept(id)?;
        let kind = kept.content_type.as_deref()?.split(';').next()?.trim();
        (kind.to_ascii_lowercase().starts_with("image/") && kept.bytes.len() <= IMAGE_LIMIT).then(
            || {
                let encoded = base64::engine::general_purpose::STANDARD.encode(&kept.bytes);
                format!("data:{kind};base64,{encoded}")
            },
        )
    }

    fn kept(&self, id: &str) -> Option<Arc<Kept>> {
        self.bodies.lock().ok()?.get(id).cloned()
    }

    pub fn forget(&self, id: &str) {
        if let Ok(mut bodies) = self.bodies.lock() {
            bodies.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::graphql::GraphqlDocument;
    use crate::http::model::{FormPart, HttpItemKind};
    use crate::http::settings::{HttpOrigin, InheritedHeader};

    fn row(key: &str, value: &str) -> KeyValue {
        KeyValue {
            key: key.to_string(),
            value: value.to_string(),
            ..KeyValue::default()
        }
    }

    fn document(url: &str) -> RequestDocument {
        RequestDocument {
            url: url.to_string(),
            ..RequestDocument::default()
        }
    }

    fn nothing() -> Inherited {
        Inherited {
            auth: RequestAuth::None,
            auth_from: None,
            headers: Vec::new(),
        }
    }

    fn header<'a>(outgoing: &'a Outgoing, name: &str) -> Option<&'a str> {
        outgoing
            .headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn a_url_without_a_scheme_is_sent_over_http_and_any_other_scheme_is_refused() {
        let outgoing = compose(
            RequestKind::Http,
            HttpMethod::Get,
            &document("api.example.com/users?page=1"),
            &nothing(),
        )
        .unwrap();
        assert_eq!(outgoing.url.as_str(), "http://api.example.com/users?page=1");

        for refused in ["ftp://example.com", "http://", "  "] {
            assert!(matches!(
                compose(
                    RequestKind::Http,
                    HttpMethod::Get,
                    &document(refused),
                    &nothing()
                ),
                Err(SendError::InvalidUrl(_))
            ));
        }
    }

    #[test]
    fn a_variable_left_anywhere_stops_the_request_by_its_name() {
        assert_eq!(
            compose(
                RequestKind::Http,
                HttpMethod::Get,
                &document("{{baseUrl}}/users"),
                &nothing()
            ),
            Err(SendError::Variable("baseUrl".to_string()))
        );

        let mut bearer = nothing();
        bearer.auth = RequestAuth::Bearer {
            token: "{{accessToken}}".to_string(),
        };
        assert_eq!(
            compose(
                RequestKind::Http,
                HttpMethod::Get,
                &document("https://example.com"),
                &bearer
            ),
            Err(SendError::Variable("accessToken".to_string()))
        );

        // Angular's `{{ user.name }}` is text, not a variable.
        let mut templated = document("https://example.com");
        templated.body = RequestBody::Text {
            text: "{{ user.name }}".to_string(),
        };
        assert!(compose(RequestKind::Http, HttpMethod::Post, &templated, &nothing()).is_ok());
    }

    #[test]
    fn its_own_headers_override_the_inherited_ones_by_name_and_a_disabled_one_is_not_sent() {
        let mut inherited = nothing();
        inherited.headers = vec![InheritedHeader {
            header: row("Accept", "text/plain"),
            from: HttpOrigin {
                kind: HttpItemKind::Collection,
                id: "c".to_string(),
                name: "API".to_string(),
            },
        }];
        let mut own = document("https://example.com");
        own.headers = vec![
            row("accept", "application/json"),
            KeyValue {
                enabled: false,
                ..row("X-Debug", "1")
            },
        ];

        let outgoing = compose(RequestKind::Http, HttpMethod::Get, &own, &inherited).unwrap();
        assert_eq!(
            outgoing.headers,
            vec![("accept".to_string(), "application/json".to_string())]
        );
    }

    #[test]
    fn the_auth_inherited_or_its_own_becomes_a_header_or_a_query_pair() {
        let mut inherited = nothing();
        inherited.auth = RequestAuth::Basic {
            username: "ada".to_string(),
            password: "lovelace".to_string(),
        };
        let outgoing = compose(
            RequestKind::Http,
            HttpMethod::Get,
            &document("https://example.com"),
            &inherited,
        )
        .unwrap();
        assert_eq!(
            header(&outgoing, "authorization"),
            Some("Basic YWRhOmxvdmVsYWNl")
        );

        let mut keyed = document("https://example.com/?a=1");
        keyed.auth = RequestAuth::ApiKey {
            name: "key".to_string(),
            value: "s e".to_string(),
            place: KeyPlace::Query,
        };
        let outgoing = compose(RequestKind::Http, HttpMethod::Get, &keyed, &inherited).unwrap();
        assert_eq!(outgoing.url.as_str(), "https://example.com/?a=1&key=s+e");
        assert_eq!(header(&outgoing, "authorization"), None);

        let mut typed = document("https://example.com");
        typed.auth = RequestAuth::Bearer {
            token: "t".to_string(),
        };
        typed.headers = vec![row("Authorization", "Token mine")];
        let outgoing = compose(RequestKind::Http, HttpMethod::Get, &typed, &nothing()).unwrap();
        assert_eq!(header(&outgoing, "authorization"), Some("Token mine"));
    }

    #[test]
    fn the_body_carries_its_type_unless_one_is_typed_and_a_form_is_encoded() {
        let mut json = document("https://example.com");
        json.body = RequestBody::Json {
            text: "{\"a\":1}".to_string(),
        };
        let outgoing = compose(RequestKind::Http, HttpMethod::Post, &json, &nothing()).unwrap();
        assert_eq!(header(&outgoing, "content-type"), Some("application/json"));
        assert_eq!(outgoing.body, OutgoingBody::Bytes(b"{\"a\":1}".to_vec()));

        json.headers = vec![row("Content-Type", "application/vnd.api+json")];
        let outgoing = compose(RequestKind::Http, HttpMethod::Post, &json, &nothing()).unwrap();
        assert_eq!(
            header(&outgoing, "content-type"),
            Some("application/vnd.api+json")
        );

        let mut form = document("https://example.com");
        form.body = RequestBody::Form {
            fields: vec![row("q", "a b&c"), row("lang", "fr")],
        };
        let outgoing = compose(RequestKind::Http, HttpMethod::Post, &form, &nothing()).unwrap();
        assert_eq!(
            outgoing.body,
            OutgoingBody::Bytes(b"q=a+b%26c&lang=fr".to_vec())
        );
    }

    #[test]
    fn a_file_is_read_when_sent_and_one_missing_is_named() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("avatar.png");
        std::fs::write(&path, [1, 2, 3]).unwrap();
        let path = path.to_string_lossy().into_owned();

        let mut multipart = document("https://example.com");
        multipart.body = RequestBody::Multipart {
            parts: vec![
                FormPart {
                    key: "name".to_string(),
                    value: "Ada".to_string(),
                    ..FormPart::default()
                },
                FormPart {
                    key: "avatar".to_string(),
                    value: path.clone(),
                    file: true,
                    ..FormPart::default()
                },
            ],
        };
        let outgoing =
            compose(RequestKind::Http, HttpMethod::Post, &multipart, &nothing()).unwrap();
        assert_eq!(
            outgoing.body,
            OutgoingBody::Multipart(vec![
                ("name".to_string(), Part::Text("Ada".to_string())),
                (
                    "avatar".to_string(),
                    Part::File {
                        name: "avatar.png".to_string(),
                        bytes: vec![1, 2, 3]
                    }
                ),
            ])
        );
        assert_eq!(header(&outgoing, "content-type"), None);

        let mut binary = document("https://example.com");
        let missing = directory
            .path()
            .join("gone.bin")
            .to_string_lossy()
            .into_owned();
        binary.body = RequestBody::Binary {
            path: missing.clone(),
        };
        assert_eq!(
            compose(RequestKind::Http, HttpMethod::Put, &binary, &nothing()),
            Err(SendError::FileUnreadable(missing))
        );
    }

    #[test]
    fn a_graphql_request_is_a_json_post_or_its_parts_in_the_query_of_a_get() {
        let mut query = document("https://api.exemple.fr/graphql");
        query.body = RequestBody::Text {
            text: "ignored".to_string(),
        };
        query.graphql = GraphqlDocument {
            query: "query A { a }".to_string(),
            variables: "{\"n\": 1}".to_string(),
            operation_name: Some("A".to_string()),
            as_get: false,
        };

        let post = compose(RequestKind::Graphql, HttpMethod::Delete, &query, &nothing()).unwrap();
        assert_eq!(post.method, HttpMethod::Post);
        assert_eq!(header(&post, "content-type"), Some("application/json"));
        assert_eq!(
            post.body,
            OutgoingBody::Bytes(
                br#"{"query":"query A { a }","variables":{"n": 1},"operationName":"A"}"#.to_vec()
            )
        );
        assert!(post.graphql);

        query.graphql.as_get = true;
        let get = compose(RequestKind::Graphql, HttpMethod::Post, &query, &nothing()).unwrap();
        assert_eq!(
            (get.method, &get.body),
            (HttpMethod::Get, &OutgoingBody::None)
        );
        assert_eq!(
            get.url.as_str(),
            "https://api.exemple.fr/graphql?query=query+A+%7B+a+%7D&variables=%7B%22n%22%3A+1%7D&operationName=A"
        );

        query.graphql.variables = "[1]".to_string();
        assert!(matches!(
            compose(RequestKind::Graphql, HttpMethod::Post, &query, &nothing()),
            Err(SendError::GraphqlVariables(_))
        ));
    }

    #[test]
    fn a_body_is_text_by_its_type_or_failing_one_by_its_bytes() {
        assert!(is_text(Some("application/json; charset=utf-8"), &[0xff]));
        assert!(is_text(Some("application/problem+xml"), b""));
        assert!(!is_text(Some("image/png"), b"PNG"));
        assert!(is_text(None, "héllo".as_bytes()));
        assert!(!is_text(None, &[0xff, 0xfe, 0x00]));
    }
}
