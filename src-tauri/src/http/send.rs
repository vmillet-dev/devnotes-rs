//! A request sent by Rust: the `WebView` reaches no network, and will not. What it inherits is read
//! under the lock; the request is composed and sent without it, so a slow server stalls nothing.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::task::AbortHandle;
use url::Url;
use uuid::Uuid;

use super::cookies::{self, Change, JarCookie};
use super::graphql::{self, GraphqlDocument, GraphqlResult};
use super::model::{
    HeaderFault, HttpMethod, KeyPlace, KeyValue, RequestAuth, RequestBody, RequestDocument,
    RequestKind, header_fault,
};
use super::response::{self, Cookie};
use super::settings::{HttpOrigin, Inherited, implied_content_type, switches_off};
use super::transport::{Limit, Limits, SetBy, Transport};
use super::{variables, websocket};
use crate::notes::language::Language;

/// What a request says it comes from when it says nothing: many APIs refuse one that is silent.
pub const USER_AGENT: &str = concat!("DevNotes/", env!("CARGO_PKG_VERSION"));
const ACCEPT_ANYTHING: &str = "*/*";

/// Whether a header that left is one the application writes by itself, whatever its version:
/// what is rebuilt from the history leaves it out, and it is written again when sent.
pub fn is_default(name: &str, value: &str) -> bool {
    (name.eq_ignore_ascii_case("user-agent") && value.starts_with("DevNotes/"))
        || (name.eq_ignore_ascii_case("accept") && value == ACCEPT_ANYTHING)
}

/// What crosses to be shown: past it the text is cut, and the whole is offered as a file.
pub const SHOWN_LIMIT: usize = 1024 * 1024;
/// What is read at all: past it the reading stops.
pub const KEPT_LIMIT: usize = 64 * 1024 * 1024;

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
    /// The environment its `{{variables}}` resolve from, over the globals; `None` for the
    /// globals alone.
    pub environment_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Redirect {
    pub status: u16,
    /// Where it sends to.
    pub url: String,
    /// The URL that answered with it.
    #[serde(default)]
    pub from: String,
    /// What it set: a login's cookie is set here, not by the page it leads to.
    #[serde(default)]
    pub cookies: Vec<Cookie>,
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
    #[error("No answer within {} ms", .0.value)]
    Timeout(Limit),
    #[error("More than {} redirects", .0.value)]
    TooManyRedirects(Limit),
    #[error("The file {0} cannot be read")]
    FileUnreadable(String),
    #[error("The send was cancelled")]
    Cancelled,
    #[error("The GraphQL variables are not a JSON object: {0}")]
    GraphqlVariables(String),
    /// The server answered the WebSocket handshake with this status instead of 101.
    #[error("The server refused the WebSocket: {0}")]
    Handshake(u16),
    #[error("Network error: {0}")]
    Network(String),
    /// A socket's URL on a scheme that is not one a socket opens on; `twin` is the same URL on
    /// `ws://` or `wss://`, when it was typed on `http://` or `https://`.
    #[error("A WebSocket opens on ws:// or wss://")]
    SocketScheme { twin: Option<String> },
    /// A header that cannot be built: named, with who wrote it, rather than blamed on the URL.
    #[error("The header {name} cannot be sent: its {fault:?} is not valid")]
    Header {
        name: String,
        fault: HeaderFault,
        set_by: SetBy,
    },
}

/// Who put a header on a request that no row of its own wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ImpliedSource {
    Body,
    /// A folder or the collection, named in `from`.
    Above,
    /// The application's own: `User-Agent`, `Accept`.
    Default,
    /// `Host`, which the client writes from the URL.
    Url,
}

/// A header sent without a row of the request's own: the editor lists them under its table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImpliedHeader {
    pub name: String,
    pub value: String,
    pub source: ImpliedSource,
    pub from: Option<HttpOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Wrote {
    Request,
    Auth,
    Above(HttpOrigin),
    Body,
    Default,
}

struct Written {
    name: String,
    value: String,
    by: Wrote,
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
    /// The secret variables in force, by name and value: the exchange shows their names.
    pub secret_values: Vec<(String, String)>,
    /// Its answer is read as GraphQL.
    pub graphql: bool,
    /// Its timeout, redirects and TLS: the defaults until the command sets what is inherited.
    pub transport: Transport,
    pub limits: Limits,
    /// The jar as the command read it under the lock, for the hops to read and fill. `None`
    /// sends no cookie and keeps none, at any hop.
    pub jar: Option<Vec<JarCookie>>,
}

impl Outgoing {
    pub fn timeout(&self) -> Limit {
        Limit {
            value: self.transport.timeout_ms,
            set_by: self.limits.timeout.clone(),
        }
    }

    pub fn redirects(&self) -> Limit {
        Limit {
            value: self.transport.max_redirects,
            set_by: self.limits.redirects.clone(),
        }
    }
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

/// The first `{{variable}}` left in what will be sent, in reading order: one no level sets.
fn first_variable(
    kind: RequestKind,
    document: &RequestDocument,
    inherited: &Inherited,
) -> Option<String> {
    variables::texts(kind, document, inherited)
        .into_iter()
        .find_map(|text| variables::names_in(text).into_iter().next())
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
    // ⚠️ Why, never the URL itself: a detail is shown and logged, and a URL carries a key in
    // its query or a password before its host.
    if !matches!(url.scheme(), "http" | "https") {
        return Err(SendError::InvalidUrl(format!(
            "the scheme {}",
            url.scheme()
        )));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(SendError::InvalidUrl("no host".to_string()));
    }
    Ok(url)
}

/// The `Content-Type` its kind and its body imply: a socket has no body, nor a GraphQL `GET`.
fn content_type(kind: RequestKind, document: &RequestDocument) -> Option<&'static str> {
    match kind {
        RequestKind::Http => implied_content_type(&document.body),
        RequestKind::Graphql => (!document.graphql.as_get).then_some("application/json"),
        RequestKind::Websocket => None,
    }
}

/// Every header of a request and who wrote it: the inherited ones, its own over them by name,
/// then, each only where no row sets it, the auth's, the body's type and the application's own.
fn written(
    document: &RequestDocument,
    inherited: &Inherited,
    content_type: Option<&str>,
) -> Vec<Written> {
    let mut headers: Vec<Written> = Vec::new();
    let mut set = |name: &str, value: &str, by: Wrote| {
        headers.retain(|known| !known.name.eq_ignore_ascii_case(name));
        headers.push(Written {
            name: name.to_string(),
            value: value.to_string(),
            by,
        });
    };
    for known in &inherited.headers {
        set(
            known.header.key.trim(),
            &known.header.value,
            Wrote::Above(known.from.clone()),
        );
    }
    for row in sent(&document.headers) {
        set(row.key.trim(), &row.value, Wrote::Request);
    }

    let mut unless_set = |name: &str, value: String, by: Wrote| {
        if !headers
            .iter()
            .any(|known| known.name.eq_ignore_ascii_case(name))
        {
            headers.push(Written {
                name: name.to_string(),
                value,
                by,
            });
        }
    };
    match auth_of(document, inherited) {
        RequestAuth::Basic { username, password } => {
            let pair =
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
            unless_set("Authorization", format!("Basic {pair}"), Wrote::Auth);
        }
        RequestAuth::Bearer { token } => {
            unless_set(
                "Authorization",
                format!("Bearer {}", token.trim()),
                Wrote::Auth,
            );
        }
        RequestAuth::ApiKey {
            name,
            value,
            place: KeyPlace::Header,
        } if !name.trim().is_empty() => unless_set(name.trim(), value.clone(), Wrote::Auth),
        _ => {}
    }
    if let Some(content_type) = content_type {
        unless_set("Content-Type", content_type.to_string(), Wrote::Body);
    }
    // Its row switched off, here or above, is how a request is sent without any.
    if !inherited.user_agent_off && !switches_off(&document.headers, "user-agent") {
        unless_set("User-Agent", USER_AGENT.to_string(), Wrote::Default);
    }
    // The client would add it by itself: written here, it is in what the history shows as sent.
    unless_set("Accept", ACCEPT_ANYTHING.to_string(), Wrote::Default);
    headers
}

/// The headers a generated program writes: what a send writes, but the application's own —
/// the program names itself —, a Basic auth, which each target writes its own way, and a
/// multipart form's type, which carries a boundary the tool alone chooses.
pub fn written_for_code(
    kind: RequestKind,
    document: &RequestDocument,
    inherited: &Inherited,
) -> Vec<(String, String)> {
    let basic = matches!(auth_of(document, inherited), RequestAuth::Basic { .. });
    let multipart =
        kind == RequestKind::Http && matches!(document.body, RequestBody::Multipart { .. });
    written(document, inherited, content_type(kind, document))
        .into_iter()
        .filter(|header| header.by != Wrote::Default)
        .filter(|header| !(basic && header.by == Wrote::Auth))
        .filter(|header| !(multipart && header.by == Wrote::Body))
        .map(|header| (header.name, header.value))
        .collect()
}

pub fn auth_of<'a>(document: &'a RequestDocument, inherited: &'a Inherited) -> &'a RequestAuth {
    match &document.auth {
        RequestAuth::Inherit => &inherited.auth,
        own => own,
    }
}

/// `api.example.com:8443`, the port only when it is not its scheme's own. A `{{variable}}`
/// reads as a host name, and is not one until it is filled.
fn host_of(url: &Url) -> Option<String> {
    let host = url.host_str()?;
    if !variables::names_in(host).is_empty() {
        return None;
    }
    Some(
        url.port()
            .map_or_else(|| host.to_string(), |port| format!("{host}:{port}")),
    )
}

/// What a request sends without a row of its own, read from what `compose` writes: the two
/// cannot disagree. Nothing is refused here: a URL that does not read yet has no `Host`.
pub fn implied(
    kind: RequestKind,
    document: &RequestDocument,
    inherited: &Inherited,
) -> Vec<ImpliedHeader> {
    let headers = written(document, inherited, content_type(kind, document));
    let typed_host = headers
        .iter()
        .any(|known| known.name.eq_ignore_ascii_case("host"));
    let mut listed: Vec<ImpliedHeader> = headers
        .into_iter()
        .filter_map(|header| {
            let (source, from) = match header.by {
                Wrote::Request | Wrote::Auth => return None,
                Wrote::Above(origin) => (ImpliedSource::Above, Some(origin)),
                Wrote::Body => (ImpliedSource::Body, None),
                Wrote::Default => (ImpliedSource::Default, None),
            };
            Some(ImpliedHeader {
                name: header.name,
                value: header.value,
                source,
                from,
            })
        })
        .collect();
    let url = match kind {
        RequestKind::Websocket => websocket::socket_url(&document.url).unwrap_or_default(),
        RequestKind::Http | RequestKind::Graphql => document.url.clone(),
    };
    if !typed_host && let Some(host) = parse_url(&url).ok().as_ref().and_then(host_of) {
        listed.push(ImpliedHeader {
            name: "Host".to_string(),
            value: host,
            source: ImpliedSource::Url,
            from: None,
        });
    }
    listed.sort_by_key(|header| header.source);
    listed
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

    let auth = auth_of(document, inherited);
    if let RequestAuth::ApiKey {
        name,
        value,
        place: KeyPlace::Query,
    } = auth
        && !name.trim().is_empty()
    {
        url.query_pairs_mut().append_pair(name.trim(), value);
    }

    let body = match graphql {
        Some(graphql) => graphql_body(graphql, &mut url)?,
        None => body_of(&document.body)?,
    };
    // A multipart type carries its boundary, which reqwest alone knows.
    let implied =
        content_type(kind, document).filter(|_| !matches!(body, OutgoingBody::Multipart(_)));
    let headers = written(document, inherited, implied);
    // ⚠️ Before the client builds it: its builder says « invalid URL » for a header it refuses.
    if let Some((header, fault)) = headers
        .iter()
        .find_map(|header| Some((header, header_fault(&header.name, &header.value)?)))
    {
        return Err(SendError::Header {
            name: header.name.clone(),
            fault,
            set_by: match &header.by {
                Wrote::Above(origin) => SetBy::Above(origin.clone()),
                _ => SetBy::Request,
            },
        });
    }
    let headers = headers
        .into_iter()
        .map(|header| (header.name, header.value))
        .collect();

    let secrets = match auth {
        RequestAuth::ApiKey { name, .. } if !name.trim().is_empty() => {
            vec![name.trim().to_ascii_lowercase()]
        }
        _ => Vec::new(),
    };

    let set_by = |own: bool, above: &SetBy| {
        if own { SetBy::Request } else { above.clone() }
    };

    Ok(Outgoing {
        method,
        url,
        headers,
        body,
        secrets,
        secret_values: Vec::new(),
        graphql: graphql.is_some(),
        jar: None,
        transport: inherited.transport.over(&document.transport),
        limits: Limits {
            timeout: set_by(
                document.transport.timeout_ms.is_some(),
                &inherited.limits.timeout,
            ),
            redirects: set_by(
                document.transport.max_redirects.is_some(),
                &inherited.limits.redirects,
            ),
        },
    })
}

/// A GraphQL operation as a JSON body, or in the query of a `GET`.
fn graphql_body(graphql: &GraphqlDocument, url: &mut Url) -> Result<OutgoingBody, SendError> {
    let variables = graphql::variables(graphql).map_err(SendError::GraphqlVariables)?;
    if !graphql.as_get {
        let body = graphql::body(graphql, variables).into_bytes();
        return Ok(OutgoingBody::Bytes(body));
    }
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
    Ok(OutgoingBody::None)
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
/// Windows' own codes for a name that does not resolve: `WSAHOST_NOT_FOUND`, `WSATRY_AGAIN`,
/// `WSANO_RECOVERY`, `WSANO_DATA`.
const NAME_NOT_FOUND: std::ops::RangeInclusive<i32> = 11001..=11004;

/// What a connection that failed ran into, read off the chain of its causes: one reading for
/// a request and for a socket, so the same failure has the same name on both. `None` when the
/// chain names nothing known. `timeout` is the delay that was waited, when one was.
///
/// ⚠️ By its code before its words: Windows words its errors in the system's language, and
/// « Hôte inconnu » matches no wording — a socket said « Erreur réseau » where a request,
/// whose client prefixes the same failure with `dns error`, named it.
pub(crate) fn cause_of(
    error: &(dyn std::error::Error + 'static),
    timeout: Option<&Limit>,
) -> Option<SendError> {
    let mut cause = Some(error);
    while let Some(current) = cause {
        if current.downcast_ref::<rustls::Error>().is_some() {
            return Some(SendError::Tls(current.to_string()));
        }
        if let Some(io) = current.downcast_ref::<std::io::Error>() {
            match io.kind() {
                std::io::ErrorKind::ConnectionRefused => {
                    return Some(SendError::Refused(io.to_string()));
                }
                std::io::ErrorKind::TimedOut => {
                    if let Some(limit) = timeout {
                        return Some(SendError::Timeout(limit.clone()));
                    }
                }
                _ => {}
            }
            if io
                .raw_os_error()
                .is_some_and(|code| cfg!(windows) && NAME_NOT_FOUND.contains(&code))
            {
                return Some(SendError::Unresolved(io.to_string()));
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
            return Some(SendError::Unresolved(current.to_string()));
        }
        if text.contains("certificate") || text.contains("invalid peer") {
            return Some(SendError::Tls(current.to_string()));
        }
        cause = current.source();
    }
    None
}

fn failure(error: &reqwest::Error, timeout: &Limit) -> SendError {
    if error.is_timeout() {
        return SendError::Timeout(timeout.clone());
    }
    if error.is_builder() {
        // `reqwest` names the URL in its own message: the cause under it does not.
        return SendError::InvalidUrl(innermost(error));
    }
    cause_of(error, Some(timeout)).unwrap_or_else(|| SendError::Network(innermost(error)))
}

/// The last cause of a chain, in its own words: what « Erreur réseau » is followed by, when it
/// is all that can be said.
pub(crate) fn innermost(error: &(dyn std::error::Error + 'static)) -> String {
    let mut last = error;
    while let Some(next) = last.source() {
        last = next;
    }
    last.to_string()
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

pub(crate) fn install_crypto() {
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

/// The request as it left, a secret variable's value written as its `{{name}}`.
pub fn exchange(outgoing: &Outgoing) -> Exchange {
    let hide = |text: &str| variables::hide(text, &outgoing.secret_values);
    let body = match &outgoing.body {
        OutgoingBody::None => SentBody::None,
        OutgoingBody::Bytes(bytes) => match std::str::from_utf8(bytes) {
            Ok(text) => SentBody::Text {
                text: hide(&text.chars().take(SHOWN_LIMIT).collect::<String>()),
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
        url: hide(outgoing.url.as_str()),
        headers: outgoing
            .headers
            .iter()
            .map(|(key, value)| KeyValue {
                key: key.clone(),
                value: hide(value),
                ..KeyValue::default()
            })
            .collect(),
        body,
    }
}

/// One request of a send: the first as composed, then one a redirect followed.
struct Hop {
    method: reqwest::Method,
    url: Url,
    /// Without the jar's `Cookie`, read again for each hop's own URL.
    headers: Vec<(String, String)>,
    body: OutgoingBody,
}

/// What a redirect to another host does not carry over, as a browser drops them.
const NOT_ACROSS_HOSTS: [&str; 5] = [
    "authorization",
    "cookie",
    "cookie2",
    "proxy-authorization",
    "www-authenticate",
];
/// What describes a body a 301, 302 or 303 leaves behind.
const OF_THE_BODY: [&str; 4] = [
    "content-type",
    "content-length",
    "content-encoding",
    "transfer-encoding",
];

fn without(headers: &mut Vec<(String, String)>, names: &[&str]) {
    headers.retain(|(name, _)| !names.iter().any(|gone| name.eq_ignore_ascii_case(gone)));
}

/// The page a redirect comes from, as the client said it when it followed them itself: never
/// from `https` to `http`, and without its credentials or its fragment.
fn referer(from: &Url, to: &Url) -> Option<String> {
    if from.scheme() == "https" && to.scheme() == "http" {
        return None;
    }
    let mut referer = from.clone();
    let _ = referer.set_username("");
    let _ = referer.set_password(None);
    referer.set_fragment(None);
    Some(referer.to_string())
}

impl Hop {
    /// The request a redirect asks for, or `None` when the answer is not one to follow. The
    /// client's own rules, kept: a 301, 302 or 303 becomes a `GET` without its body, a 307 or
    /// 308 keeps both; another host gets neither the `Authorization` nor a typed `Cookie`; a
    /// `Location` is read against the URL that answered.
    fn after(&self, status: u16, headers: &[KeyValue]) -> Option<Self> {
        let keeps_body = match status {
            301..=303 => false,
            307 | 308 => true,
            _ => return None,
        };
        let location = headers
            .iter()
            .find(|header| header.key.eq_ignore_ascii_case("location"))?;
        let url = self.url.join(location.value.trim()).ok()?;
        if !matches!(url.scheme(), "http" | "https") {
            return None;
        }
        let mut next = Self {
            method: self.method.clone(),
            url,
            headers: self.headers.clone(),
            body: self.body.clone(),
        };
        if !keeps_body {
            next.body = OutgoingBody::None;
            without(&mut next.headers, &OF_THE_BODY);
            if !matches!(next.method, reqwest::Method::GET | reqwest::Method::HEAD) {
                next.method = reqwest::Method::GET;
            }
        }
        if next.url.host_str() != self.url.host_str()
            || next.url.port_or_known_default() != self.url.port_or_known_default()
        {
            without(&mut next.headers, &NOT_ACROSS_HOSTS);
        }
        without(&mut next.headers, &["referer"]);
        if let Some(referer) = referer(&self.url, &next.url) {
            next.headers.push(("Referer".to_string(), referer));
        }
        Some(next)
    }

    /// The jar's `Cookie` for this hop's URL, unless one is typed: what is typed wins, as for
    /// `Authorization`.
    fn cookie(&self, jar: Option<&[JarCookie]>) -> Option<String> {
        if self
            .headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        {
            return None;
        }
        cookies::header_for(jar?, &self.url, Utc::now())
    }
}

fn request(client: &reqwest::Client, hop: &Hop, cookie: Option<&str>) -> reqwest::RequestBuilder {
    let mut request = client.request(hop.method.clone(), hop.url.clone());
    for (key, value) in &hop.headers {
        request = request.header(key, value);
    }
    if let Some(cookie) = cookie {
        request = request.header("Cookie", cookie);
    }
    // Copied: a 307 sends the same body again.
    match &hop.body {
        OutgoingBody::None => request,
        OutgoingBody::Bytes(bytes) => request.body(bytes.clone()),
        OutgoingBody::Multipart(parts) => {
            let mut form = reqwest::multipart::Form::new();
            for (key, part) in parts {
                form = match part {
                    Part::Text(text) => form.text(key.clone(), text.clone()),
                    Part::File { name, bytes } => form.part(
                        key.clone(),
                        reqwest::multipart::Part::bytes(bytes.clone()).file_name(name.clone()),
                    ),
                };
            }
            request.multipart(form)
        }
    }
}

/// A client per send, for its TLS. It follows no redirect: `send` does, a hop at a time.
fn client(transport: Transport) -> Result<reqwest::Client, SendError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .tls_danger_accept_invalid_certs(!transport.verify_tls)
        .build()
        .map_err(|error| SendError::Network(error.to_string()))
}

fn headers_of(response: &reqwest::Response) -> Vec<KeyValue> {
    response
        .headers()
        .iter()
        .map(|(key, value)| KeyValue {
            key: key.to_string(),
            value: String::from_utf8_lossy(value.as_bytes()).into_owned(),
            ..KeyValue::default()
        })
        .collect()
}

/// Where a chain of redirects ended, and what was met on the way.
struct Arrival {
    response: reqwest::Response,
    headers: Vec<KeyValue>,
    redirects: Vec<Redirect>,
    /// What the redirects set, each with the hop that set it.
    cookies: Vec<Cookie>,
}

/// What a chain is followed under: the request's own limits, and the jar its hops share.
struct Chain {
    client: reqwest::Client,
    follows: bool,
    timeout: Limit,
    most: Limit,
    deadline: Instant,
    jar: Option<Vec<JarCookie>>,
    changes: Arc<Mutex<Vec<Change>>>,
}

impl Chain {
    /// ⚠️ The redirects are followed here, one request a hop, and not by the client: it has no
    /// jar, so a cookie set on a 302 — nearly every login — was neither kept nor sent to the
    /// page it leads to. Each hop's `Set-Cookie` goes through the jar's rules as it arrives,
    /// and each hop is sent the line the jar gives for its own URL. The jar is a copy:
    /// `changes` is what the command merges under the lock once the send is over, however it
    /// ended. The delay is the whole chain's, and a cancel aborts it between two hops as
    /// anywhere else.
    async fn follow(mut self, mut hop: Hop) -> Result<Arrival, SendError> {
        let mut redirects: Vec<Redirect> = Vec::new();
        let mut cookies: Vec<Cookie> = Vec::new();
        loop {
            let left = self.deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(SendError::Timeout(self.timeout));
            }
            let cookie = hop.cookie(self.jar.as_deref());
            let response = request(&self.client, &hop, cookie.as_deref())
                .timeout(left)
                .send()
                .await
                .map_err(|error| failure(&error, &self.timeout))?;
            let headers = headers_of(&response);
            if let Some(jar) = self.jar.as_mut() {
                let set = cookies::received(&headers, &hop.url, Utc::now());
                cookies::apply(jar, &set);
                if let Ok(mut changes) = self.changes.lock() {
                    changes.extend(set);
                }
            }
            let status = response.status().as_u16();
            let Some(next) = self.follows.then(|| hop.after(status, &headers)).flatten() else {
                return Ok(Arrival {
                    response,
                    headers,
                    redirects,
                    cookies,
                });
            };
            if redirects.len() >= usize::try_from(self.most.value).unwrap_or(usize::MAX) {
                return Err(SendError::TooManyRedirects(self.most));
            }
            let set: Vec<Cookie> = response::cookies(&headers)
                .into_iter()
                .map(|cookie| Cookie {
                    from: Some(hop.url.to_string()),
                    ..cookie
                })
                .collect();
            redirects.push(Redirect {
                status,
                url: next.url.to_string(),
                from: hop.url.to_string(),
                cookies: set.clone(),
            });
            cookies.extend(set);
            hop = next;
        }
    }
}

async fn send(
    mut outgoing: Outgoing,
    changes: Arc<Mutex<Vec<Change>>>,
) -> Result<Received, SendError> {
    install_crypto();
    let jar = outgoing.jar.take();
    let mut exchange = exchange(&outgoing);
    let is_graphql = outgoing.graphql;
    let timeout = outgoing.timeout();
    let started = Instant::now();
    let chain = Chain {
        client: client(outgoing.transport)?,
        follows: outgoing.transport.follow_redirects,
        most: outgoing.redirects(),
        deadline: started + Duration::from_millis(u64::from(timeout.value)),
        timeout: timeout.clone(),
        jar,
        changes,
    };
    let hop = Hop {
        method: wire_method(outgoing.method),
        url: outgoing.url,
        headers: outgoing.headers,
        body: outgoing.body,
    };
    // The request as it left: with the line its first hop carries.
    if let Some(line) = hop.cookie(chain.jar.as_deref()) {
        exchange.headers.push(KeyValue {
            key: "Cookie".to_string(),
            value: line,
            ..KeyValue::default()
        });
    }

    let Arrival {
        mut response,
        headers,
        redirects,
        cookies: set_on_the_way,
    } = chain.follow(hop).await?;
    let failure = |error: reqwest::Error| failure(&error, &timeout);
    let status = response.status();
    let url = response.url().to_string();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let mut bytes = Vec::new();
    let mut incomplete = false;
    while let Some(chunk) = response.chunk().await.map_err(failure)? {
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
            cookies: set_on_the_way
                .into_iter()
                .chain(response::cookies(&headers))
                .collect(),
            headers,
            cut,
            body: shown,
            binary,
            incomplete,
            redirects,
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
    /// A task of its own, so cancelling aborts it wherever it waits. Beside the answer, what
    /// the hops did to the jar: kept even when the chain ended on a failure, as a browser keeps
    /// the cookie of a login whose landing page did not load.
    pub async fn run(
        &self,
        id: &str,
        outgoing: Outgoing,
    ) -> (Result<SentResponse, SendError>, Vec<Change>) {
        let changes: Arc<Mutex<Vec<Change>>> = Arc::default();
        let task = tauri::async_runtime::spawn(send(outgoing, Arc::clone(&changes)));
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
        let answer = answer
            .map_err(|_| SendError::Cancelled)
            .flatten()
            .map(|received| {
                if let Ok(mut bodies) = self.bodies.lock() {
                    bodies.insert(id.to_string(), Arc::new(received.kept));
                }
                received.response
            });
        let changes = changes
            .lock()
            .map(|mut set| std::mem::take(&mut *set))
            .unwrap_or_default();
        (answer, changes)
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
        // Removed on either failure: a body cut short by a full disk is a response left in
        // the clear under a name nobody chose.
        std::fs::write(&staged, &kept.bytes)
            .and_then(|()| std::fs::rename(&staged, path))
            .inspect_err(|_| {
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
            transport: crate::http::transport::Transport::default(),
            limits: Limits::default(),
            user_agent_off: false,
            variables: Vec::new(),
        }
    }

    fn from(kind: HttpItemKind, name: &str) -> HttpOrigin {
        HttpOrigin {
            kind,
            id: name.to_lowercase(),
            name: name.to_string(),
        }
    }

    fn off(key: &str) -> KeyValue {
        KeyValue {
            enabled: false,
            ..row(key, "")
        }
    }

    fn get(document: &RequestDocument, inherited: &Inherited) -> Outgoing {
        compose(RequestKind::Http, HttpMethod::Get, document, inherited).unwrap()
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
            vec![
                ("accept".to_string(), "application/json".to_string()),
                ("User-Agent".to_string(), USER_AGENT.to_string()),
            ]
        );
    }

    /// « Erreur réseau » for a socket, « Le nom d'hôte ne se résout pas » for a request: the
    /// same failure, read by two functions. One reads a chain now, whoever built it.
    #[test]
    fn a_cause_is_read_the_same_whatever_wraps_it() {
        use std::io::{Error, ErrorKind};

        #[derive(Debug)]
        struct Wrapped(Box<dyn std::error::Error + Send + Sync>);
        impl std::fmt::Display for Wrapped {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "error sending request")
            }
        }
        impl std::error::Error for Wrapped {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(self.0.as_ref())
            }
        }
        let limit = Limit {
            value: 5_000,
            set_by: SetBy::Request,
        };
        let read = |error: Error| {
            let plain = cause_of(&error, Some(&limit));
            let kind = error.kind();
            let wrapped = Wrapped(Box::new(Error::new(kind, Wrapped(Box::new(error)))));
            assert_eq!(
                cause_of(&wrapped, Some(&limit)).map(|found| std::mem::discriminant(&found)),
                plain.as_ref().map(std::mem::discriminant),
            );
            plain
        };

        // The three wordings of a name that does not resolve.
        for wording in [
            "dns error: failed to lookup address information",
            "failed to lookup address information: Name or service not known",
            "No such host is known. (os error 11001)",
        ] {
            assert!(
                matches!(read(Error::other(wording)), Some(SendError::Unresolved(_))),
                "{wording}"
            );
        }
        assert!(matches!(
            read(Error::from(ErrorKind::ConnectionRefused)),
            Some(SendError::Refused(_))
        ));
        assert_eq!(
            read(Error::from(ErrorKind::TimedOut)),
            Some(SendError::Timeout(limit.clone()))
        );
        assert!(matches!(
            read(Error::other(rustls::Error::HandshakeNotComplete)),
            Some(SendError::Tls(_))
        ));
        assert!(matches!(
            read(Error::other("invalid peer certificate: UnknownIssuer")),
            Some(SendError::Tls(_))
        ));
        // Nothing known: the caller says « network », with the last cause's own words.
        assert_eq!(read(Error::other("connection reset by peer")), None);
        assert_eq!(
            innermost(&Wrapped(Box::new(Error::other("connection reset by peer")))),
            "connection reset by peer"
        );
        // Once open, a socket has no delay of its own to name.
        assert_eq!(cause_of(&Error::from(ErrorKind::TimedOut), None), None);
    }

    /// Windows says it in the system's language: « Hôte inconnu » matches no wording.
    #[cfg(windows)]
    #[test]
    fn a_name_that_does_not_resolve_is_read_by_its_code_on_windows() {
        for code in [11001, 11002, 11004] {
            let error = std::io::Error::from_raw_os_error(code);
            assert!(
                matches!(cause_of(&error, None), Some(SendError::Unresolved(_))),
                "{code}"
            );
        }
        assert_eq!(
            cause_of(&std::io::Error::from_raw_os_error(10054), None),
            None
        );
    }

    /// `api.github.com` answers 403 to a request that names no agent, and none was sent.
    #[test]
    fn a_request_names_the_application_unless_a_row_says_otherwise_at_any_level() {
        let plain = document("https://api.github.com/events");
        let outgoing = get(&plain, &nothing());
        assert!(USER_AGENT.starts_with("DevNotes/"));
        assert_eq!(
            USER_AGENT.strip_prefix("DevNotes/"),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(header(&outgoing, "user-agent"), Some(USER_AGENT));
        assert_eq!(header(&outgoing, "accept"), Some("*/*"));
        assert!(
            outgoing
                .headers
                .iter()
                .all(|(name, value)| is_default(name, value))
        );
        assert!(is_default("user-agent", "DevNotes/0.9.0"));
        assert!(!is_default("User-Agent", "curl/8.5.0") && !is_default("Accept", "text/csv"));

        let mut collection = nothing();
        collection.headers = vec![InheritedHeader {
            header: row("user-agent", "Équipe/1"),
            from: from(HttpItemKind::Collection, "API"),
        }];
        assert_eq!(
            header(&get(&plain, &collection), "user-agent"),
            Some("Équipe/1")
        );

        let mut own = plain.clone();
        own.headers = vec![row(" User-Agent ", "curl/8.5.0"), row("Accept", "text/csv")];
        let outgoing = get(&own, &collection);
        assert_eq!(header(&outgoing, "user-agent"), Some("curl/8.5.0"));
        assert_eq!(header(&outgoing, "accept"), Some("text/csv"));
        assert_eq!(outgoing.headers.len(), 2);
    }

    /// « Cette URL ne peut pas être envoyée », on a good URL, for a header its folder set.
    #[test]
    fn a_header_that_cannot_be_built_stops_the_request_by_its_name_and_who_wrote_it() {
        let auth = from(HttpItemKind::Folder, "Auth");
        let mut inherited = nothing();
        inherited.headers = vec![InheritedHeader {
            header: row("X-Recette: 1", "rsgfdregr"),
            from: auth.clone(),
        }];
        let plain = document("https://api.exemple.fr");
        assert_eq!(
            compose(RequestKind::Http, HttpMethod::Get, &plain, &inherited),
            Err(SendError::Header {
                name: "X-Recette: 1".to_string(),
                fault: HeaderFault::Name,
                set_by: SetBy::Above(auth),
            })
        );

        let mut own = plain.clone();
        own.headers = vec![row("X-Trace", "a\nb")];
        assert_eq!(
            compose(RequestKind::Http, HttpMethod::Get, &own, &nothing()),
            Err(SendError::Header {
                name: "X-Trace".to_string(),
                fault: HeaderFault::Value,
                set_by: SetBy::Request,
            })
        );

        // Its own row of the same name replaces the inherited one, fault and all.
        own.headers = vec![row("x-recette: 1", "ok"), off("X Off")];
        assert!(matches!(
            compose(RequestKind::Http, HttpMethod::Get, &own, &inherited),
            Err(SendError::Header {
                set_by: SetBy::Request,
                ..
            })
        ));
        own.headers = vec![off("X Off")];
        assert!(compose(RequestKind::Http, HttpMethod::Get, &own, &nothing()).is_ok());

        // The key's own name, when the auth writes the header.
        let mut keyed = plain;
        keyed.auth = RequestAuth::ApiKey {
            name: "X Key".to_string(),
            value: "s3cret".to_string(),
            place: KeyPlace::Header,
        };
        assert!(matches!(
            compose(RequestKind::Http, HttpMethod::Get, &keyed, &nothing()),
            Err(SendError::Header {
                fault: HeaderFault::Name,
                ..
            })
        ));
    }

    fn hop(method: reqwest::Method, url: &str, headers: &[(&str, &str)]) -> Hop {
        Hop {
            method,
            url: Url::parse(url).unwrap(),
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                .collect(),
            body: OutgoingBody::Bytes(b"{}".to_vec()),
        }
    }

    fn to(location: &str) -> Vec<KeyValue> {
        vec![row("location", location)]
    }

    /// What the client did when it followed the redirects itself, now that a hop is ours.
    #[test]
    fn a_redirect_is_followed_by_its_status_its_location_read_against_what_answered() {
        let posted = hop(
            reqwest::Method::POST,
            "https://api.exemple.fr/v1/login?next=1#top",
            &[("Content-Type", "application/json"), ("X-Trace", "1")],
        );

        for status in [301, 302, 303] {
            let next = posted.after(status, &to("../me")).unwrap();
            assert_eq!(next.url.as_str(), "https://api.exemple.fr/me");
            assert_eq!(next.method, reqwest::Method::GET);
            assert_eq!(next.body, OutgoingBody::None);
            assert_eq!(
                next.headers,
                vec![
                    ("X-Trace".to_string(), "1".to_string()),
                    (
                        "Referer".to_string(),
                        "https://api.exemple.fr/v1/login?next=1".to_string()
                    ),
                ]
            );
        }
        for status in [307, 308] {
            let next = posted.after(status, &to("/v2/login")).unwrap();
            assert_eq!(next.method, reqwest::Method::POST);
            assert_eq!(next.body, posted.body);
            assert!(next.headers.iter().any(|(name, _)| name == "Content-Type"));
        }
        // A `HEAD` stays one; what is no redirect, or leads nowhere a request goes, is the answer.
        let head = hop(reqwest::Method::HEAD, "https://exemple.fr/", &[]);
        assert_eq!(
            head.after(302, &to("/a")).unwrap().method,
            reqwest::Method::HEAD
        );
        for status in [200, 300, 304, 305, 404] {
            assert!(posted.after(status, &to("/a")).is_none(), "{status}");
        }
        assert!(posted.after(302, &[]).is_none());
        assert!(posted.after(302, &to("ftp://exemple.fr/a")).is_none());
        assert!(posted.after(302, &to("http://")).is_none());
    }

    #[test]
    fn another_host_or_port_gets_no_secret_and_a_plain_page_no_referer_from_a_secure_one() {
        let secret = hop(
            reqwest::Method::GET,
            "https://ada:pw@api.exemple.fr/login",
            &[
                ("authorization", "Bearer t"),
                ("Cookie", "typed=1"),
                ("Proxy-Authorization", "Basic x"),
                ("Accept", "*/*"),
                ("Referer", "https://ailleurs.fr/"),
            ],
        );
        let names = |next: &Hop| -> Vec<String> {
            next.headers.iter().map(|(name, _)| name.clone()).collect()
        };

        let same = secret.after(302, &to("/me")).unwrap();
        assert_eq!(
            names(&same),
            [
                "authorization",
                "Cookie",
                "Proxy-Authorization",
                "Accept",
                "Referer"
            ]
        );
        // Its credentials are not in what it says it comes from.
        assert_eq!(
            same.headers.last().map(|(_, value)| value.as_str()),
            Some("https://api.exemple.fr/login")
        );

        for elsewhere in [
            "https://autre.exemple.fr/me",
            "https://api.exemple.fr:8443/me",
        ] {
            assert_eq!(
                names(&secret.after(302, &to(elsewhere)).unwrap()),
                ["Accept", "Referer"],
                "{elsewhere}"
            );
        }
        assert_eq!(
            names(&secret.after(302, &to("http://api.exemple.fr/me")).unwrap()),
            ["Accept"]
        );
    }

    #[test]
    fn a_hop_carries_the_jar_s_line_for_its_own_url_unless_a_cookie_is_typed() {
        let jar = vec![JarCookie {
            name: "session".to_string(),
            value: "abc".to_string(),
            domain: "api.exemple.fr".to_string(),
            host_only: true,
            path: "/".to_string(),
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
            created: None,
        }];
        let get = |url: &str, headers: &[(&str, &str)]| hop(reqwest::Method::GET, url, headers);

        assert_eq!(
            get("https://api.exemple.fr/me", &[]).cookie(Some(&jar)),
            Some("session=abc".to_string())
        );
        assert_eq!(get("https://autre.fr/", &[]).cookie(Some(&jar)), None);
        assert_eq!(
            get("https://api.exemple.fr/me", &[("cookie", "typed=1")]).cookie(Some(&jar)),
            None
        );
        // No jar: the request neither sends nor keeps a cookie.
        assert_eq!(get("https://api.exemple.fr/me", &[]).cookie(None), None);
    }

    #[test]
    fn a_user_agent_row_switched_off_sends_none_and_never_removes_one_set_above() {
        let mut own = document("https://example.com");
        own.headers = vec![off("user-agent")];
        assert_eq!(header(&get(&own, &nothing()), "user-agent"), None);

        let silent = Inherited {
            user_agent_off: true,
            ..nothing()
        };
        let plain = document("https://example.com");
        assert_eq!(header(&get(&plain, &silent), "user-agent"), None);
        assert_eq!(header(&get(&plain, &silent), "accept"), Some("*/*"));

        // Off here, set above: the inherited one still leaves, as any other header would.
        let mut collection = nothing();
        collection.headers = vec![InheritedHeader {
            header: row("User-Agent", "Équipe/1"),
            from: from(HttpItemKind::Collection, "API"),
        }];
        assert_eq!(
            header(&get(&own, &collection), "user-agent"),
            Some("Équipe/1")
        );
    }

    /// The editor's list is read from what `compose` writes: the same headers, less its own rows.
    #[test]
    fn what_leaves_without_a_row_is_listed_with_who_put_it_there() {
        let api = from(HttpItemKind::Collection, "API");
        let mut inherited = nothing();
        inherited.auth = RequestAuth::Bearer {
            token: "t".to_string(),
        };
        inherited.headers = vec![
            InheritedHeader {
                header: row("X-Client", "web"),
                from: api.clone(),
            },
            InheritedHeader {
                header: row("X-Trace", "1"),
                from: api.clone(),
            },
        ];
        let mut posted = document("https://api.exemple.fr:8443/users");
        posted.headers = vec![row("x-trace", "2")];
        posted.body = RequestBody::Json {
            text: "{}".to_string(),
        };

        let listed = implied(RequestKind::Http, &posted, &inherited);

        let seen: Vec<(&str, &str, ImpliedSource)> = listed
            .iter()
            .map(|header| (header.name.as_str(), header.value.as_str(), header.source))
            .collect();
        assert_eq!(
            seen,
            vec![
                ("Content-Type", "application/json", ImpliedSource::Body),
                ("X-Client", "web", ImpliedSource::Above),
                ("User-Agent", USER_AGENT, ImpliedSource::Default),
                ("Accept", "*/*", ImpliedSource::Default),
                ("Host", "api.exemple.fr:8443", ImpliedSource::Url),
            ]
        );
        assert_eq!(listed[1].from, Some(api));

        // Each one written by the request leaves the list; so does the default once switched off.
        posted.headers = vec![
            row("Content-Type", "application/vnd.api+json"),
            row("Accept", "text/csv"),
            row("Host", "autre.exemple.fr"),
            off("User-Agent"),
        ];
        let names: Vec<String> = implied(RequestKind::Http, &posted, &inherited)
            .into_iter()
            .map(|header| header.name)
            .collect();
        assert_eq!(names, vec!["X-Client", "X-Trace"]);
    }

    #[test]
    fn a_url_that_does_not_read_yet_has_no_host_and_a_socket_or_a_graphql_get_no_type() {
        let names = |kind, document: &RequestDocument| -> Vec<String> {
            implied(kind, document, &nothing())
                .into_iter()
                .map(|header| header.name)
                .collect()
        };

        for unread in [
            "{{baseUrl}}/users",
            "https://{{host}}.exemple.fr",
            "",
            "ftp://x",
        ] {
            assert_eq!(
                names(RequestKind::Http, &document(unread)),
                vec!["User-Agent", "Accept"]
            );
        }
        assert_eq!(
            names(RequestKind::Http, &document("https://example.com")),
            vec!["User-Agent", "Accept", "Host"]
        );

        let mut socket = document("wss://echo.exemple.fr/live");
        socket.body = RequestBody::Json {
            text: "{}".to_string(),
        };
        let listed = implied(RequestKind::Websocket, &socket, &nothing());
        assert_eq!(
            listed.last().map(|header| header.value.as_str()),
            Some("echo.exemple.fr")
        );
        assert!(listed.iter().all(|header| header.name != "Content-Type"));

        let mut query = document("https://example.com/graphql");
        query.graphql.query = "{ viewer { login } }".to_string();
        assert_eq!(
            names(RequestKind::Graphql, &query),
            vec!["Content-Type", "User-Agent", "Accept", "Host"]
        );
        query.graphql.as_get = true;
        assert_eq!(
            names(RequestKind::Graphql, &query),
            vec!["User-Agent", "Accept", "Host"]
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

    /// Its own value is the request's; one left unset is whoever set it above, or nobody.
    #[test]
    fn a_request_knows_who_set_its_timeout_and_its_redirects() {
        use crate::http::model::HttpItemKind;
        use crate::http::settings::HttpOrigin;

        let folder = SetBy::Above(HttpOrigin {
            kind: HttpItemKind::Folder,
            id: "f".to_string(),
            name: "Auth".to_string(),
        });
        let above = Inherited {
            transport: Transport {
                timeout_ms: 2_000,
                ..Transport::default()
            },
            limits: Limits {
                timeout: folder.clone(),
                redirects: SetBy::Default,
            },
            ..nothing()
        };
        let mut own = document("https://api.exemple.fr");
        own.transport.max_redirects = Some(1);

        let outgoing = compose(RequestKind::Http, HttpMethod::Get, &own, &above).unwrap();

        assert_eq!(
            outgoing.timeout(),
            Limit {
                value: 2_000,
                set_by: folder
            }
        );
        assert_eq!(
            outgoing.redirects(),
            Limit {
                value: 1,
                set_by: SetBy::Request
            }
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
