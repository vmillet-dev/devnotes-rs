//! What a script reaches through the host: `pm.sendRequest`, sent by Rust's client with the
//! transport of the request that carries the script, and `pm.cookies.jar()`, on the copy of the
//! jar that request's send works on. Nothing here holds the lock.

use std::fmt::Write as _;

use chrono::Utc;
use url::Url;

use super::model::{LogEntry, ScriptCookie, ScriptRequest, ScriptResponse, ScriptRow};
use super::process::{Refusal, Services};
use crate::http::cookies::{self, Change, JarCookie};
use crate::http::model::{HttpMethod, KeyValue, RequestKind};
use crate::http::send::{self, SentResponse};
use crate::http::settings;
use crate::http::transport::Transport;

/// What a protected environment refuses a script, by its code.
pub const PROTECTED: &str = "httpScriptProtected";

pub struct Network {
    runtime: tokio::runtime::Handle,
    transport: Transport,
    /// The jar's copy, `None` when the request sends no cookie; what the sends and the script
    /// did to it is in `changes`, for the command to merge under the lock.
    pub jar: Option<Vec<JarCookie>>,
    pub changes: Vec<Change>,
    /// `false` on a protected environment nobody confirmed: only what changes nothing leaves.
    may_change: bool,
    pub logs: Vec<LogEntry>,
    /// Each send a script made, and how it ended: what the console shows.
    pub sent: Vec<(ScriptRequest, Result<SentResponse, String>)>,
}

impl Network {
    #[must_use]
    pub fn new(
        runtime: tokio::runtime::Handle,
        transport: Transport,
        jar: Option<Vec<JarCookie>>,
        may_change: bool,
    ) -> Self {
        Self {
            runtime,
            transport,
            jar,
            changes: Vec::new(),
            may_change,
            logs: Vec::new(),
            sent: Vec::new(),
        }
    }

    fn send(&mut self, payload: &str) -> Result<serde_json::Value, Refusal> {
        let request: ScriptRequest =
            serde_json::from_str(payload).map_err(|error| Refusal::new(error.to_string(), None))?;
        let method = request
            .method
            .trim()
            .to_ascii_uppercase()
            .parse::<HttpMethod>()
            .map_err(|()| {
                Refusal::new(
                    format!(
                        "The method {} is not one a request is sent with",
                        request.method
                    ),
                    Some("httpScriptMethod"),
                )
            })?;
        if !self.may_change && !method.changes_nothing() {
            return Err(Refusal::new(
                format!(
                    "The environment is protected: a {method} sent by a script needs the send to be confirmed"
                ),
                Some(PROTECTED),
            ));
        }
        let mut outgoing = send::compose(
            RequestKind::Http,
            method,
            &request.document(),
            &settings::inherit(&[]),
        )
        .map_err(|error| Refusal::new(error.to_string(), Some("httpScriptSend")))?;
        outgoing.transport = self.transport;
        outgoing.jar = self.jar.clone().filter(|_| self.transport.use_cookies);
        let (answer, changes) = self.runtime.block_on(send::alone(outgoing));
        if let Some(jar) = self.jar.as_mut() {
            cookies::apply(jar, &changes);
        }
        self.changes.extend(changes);
        self.sent
            .push((request, answer.clone().map_err(|error| error.to_string())));
        let response =
            answer.map_err(|error| Refusal::new(error.to_string(), Some("httpScriptSend")))?;
        serde_json::to_value(response_of(&response))
            .map_err(|error| Refusal::new(error.to_string(), None))
    }

    fn jar(&mut self, kind: &str, payload: &str) -> Result<serde_json::Value, Refusal> {
        #[derive(serde::Deserialize)]
        struct Asked {
            url: String,
            #[serde(default)]
            name: String,
            #[serde(default)]
            cookie: ScriptCookie,
        }
        let asked: Asked =
            serde_json::from_str(payload).map_err(|error| Refusal::new(error.to_string(), None))?;
        let url = Url::parse(&asked.url)
            .map_err(|_| Refusal::new(format!("{} is not a URL", asked.url), None))?;
        let Some(jar) = self.jar.as_mut() else {
            return Err(Refusal::new(
                "This request sends no cookie: its jar is switched off",
                Some("httpScriptNoJar"),
            ));
        };
        let now = Utc::now();
        let found = |name: Option<&str>| -> Vec<JarCookie> {
            cookies::matching(jar, &url, now)
                .into_iter()
                .filter(|cookie| name.is_none_or(|name| cookie.name == name))
                .cloned()
                .collect()
        };
        let changes: Vec<Change> = match kind {
            "jarGet" => {
                let value = found(Some(&asked.name))
                    .first()
                    .map(|cookie| cookie.value.clone());
                return Ok(serde_json::json!(value));
            }
            "jarAll" => {
                let all: Vec<ScriptCookie> = found(None).iter().map(cookie_of).collect();
                return serde_json::to_value(all)
                    .map_err(|error| Refusal::new(error.to_string(), None));
            }
            "jarSet" => cookies::received(&[set_cookie(&asked.cookie)], &url, now),
            "jarUnset" => found(Some(&asked.name))
                .into_iter()
                .map(Change::Remove)
                .collect(),
            "jarClear" => found(None).into_iter().map(Change::Remove).collect(),
            _ => return Err(Refusal::new(format!("Nothing answers {kind}"), None)),
        };
        cookies::apply(jar, &changes);
        self.changes.extend(changes);
        Ok(serde_json::Value::Null)
    }
}

/// A cookie a script sets, as the line a server would have written for it.
fn set_cookie(cookie: &ScriptCookie) -> KeyValue {
    let mut line = format!("{}={}", cookie.name, cookie.value);
    for (attribute, value) in [
        ("Path", &cookie.path),
        ("Domain", &cookie.domain),
        ("Expires", &cookie.expires),
    ] {
        if let Some(value) = value {
            let _ = write!(line, "; {attribute}={value}");
        }
    }
    if cookie.secure {
        line.push_str("; Secure");
    }
    if cookie.http_only {
        line.push_str("; HttpOnly");
    }
    KeyValue {
        key: "Set-Cookie".to_string(),
        value: line,
        ..KeyValue::default()
    }
}

fn cookie_of(cookie: &JarCookie) -> ScriptCookie {
    ScriptCookie {
        name: cookie.name.clone(),
        value: cookie.value.clone(),
        domain: Some(cookie.domain.clone()),
        path: Some(cookie.path.clone()),
        expires: cookie.expires.map(|expires| expires.to_rfc2822()),
        http_only: cookie.http_only,
        secure: cookie.secure,
    }
}

/// An answer as a script reads it.
#[must_use]
pub fn response_of(response: &SentResponse) -> ScriptResponse {
    ScriptResponse {
        code: response.status,
        reason: response.reason.clone(),
        headers: response
            .headers
            .iter()
            .map(|header| ScriptRow {
                key: header.key.clone(),
                value: header.value.clone(),
                ..ScriptRow::default()
            })
            .collect(),
        body: response.body.clone(),
        millis: response.millis,
        size: response.size,
        cookies: response.cookies.iter().map(ScriptCookie::from).collect(),
    }
}

impl Services for Network {
    fn call(&mut self, kind: &str, payload: &str) -> Result<serde_json::Value, Refusal> {
        match kind {
            "send" => self.send(payload),
            _ if kind.starts_with("jar") => self.jar(kind, payload),
            _ => Err(Refusal::new(format!("Nothing answers {kind}"), None)),
        }
    }

    fn log(&mut self, entry: LogEntry) {
        self.logs.push(entry);
    }
}
