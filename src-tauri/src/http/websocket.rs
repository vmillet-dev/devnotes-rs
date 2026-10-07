//! A WebSocket lives in Rust, a task a socket: the front composes, Rust connects, writes and
//! reads, and every event reaches the front on one topic. Closing a tab closes its socket;
//! switching library closes them all.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use rustls_platform_verifier::ConfigVerifierExt;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::mpsc;
use tokio::task::AbortHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{Connector, connect_async_tls_with_config};
use url::Url;

use super::send::{Outgoing, SendError, install_crypto};
use crate::error::{AppError, ErrorCode};

pub const WEBSOCKET_EVENT: &str = "devnotes://websocket";
/// A message longer than this crosses cut: the log shows, it does not archive.
pub const MESSAGE_SHOWN: usize = 64 * 1024;
const CLOSE_GRACE: Duration = Duration::from_secs(3);

/// A request's own WebSocket part: what it asks the server, and what it keeps to send again.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct WebsocketDocument {
    /// `Sec-WebSocket-Protocol`, in the order of preference.
    pub protocols: Vec<String>,
    pub messages: Vec<SavedMessage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedMessage {
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WebsocketEvent {
    /// The id the front connected under.
    pub id: String,
    pub at: DateTime<Utc>,
    pub event: SocketEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SocketEvent {
    Opened {
        url: String,
        /// The subprotocol the server chose.
        protocol: Option<String>,
    },
    Sent {
        text: String,
        size: u32,
    },
    Received {
        /// Empty for a binary message.
        text: String,
        size: u32,
        binary: bool,
        cut: bool,
    },
    Closed {
        code: Option<u16>,
        reason: String,
    },
    /// The connection broke after it opened.
    Failed {
        code: ErrorCode,
        detail: String,
    },
}

enum Outbound {
    Text(String),
    Close,
}

struct Socket {
    outbound: mpsc::UnboundedSender<Outbound>,
    task: AbortHandle,
}

/// The sockets open, by the id the front gave each.
#[derive(Default)]
pub struct Sockets {
    open: Mutex<HashMap<String, Socket>>,
}

/// `wss://` and `ws://` as typed, `http(s)://` too; no scheme is `ws://`.
pub fn socket_url(typed: &str) -> String {
    let typed = typed.trim();
    if let Some(rest) = typed.strip_prefix("wss://") {
        format!("https://{rest}")
    } else if let Some(rest) = typed.strip_prefix("ws://") {
        format!("http://{rest}")
    } else {
        typed.to_string()
    }
}

/// The URL `compose` checked, back on the socket's scheme.
fn as_socket(url: &Url) -> String {
    let text = url.as_str();
    text.strip_prefix("https://").map_or_else(
        || {
            text.strip_prefix("http://")
                .map_or_else(|| text.to_string(), |rest| format!("ws://{rest}"))
        },
        |rest| format!("wss://{rest}"),
    )
}

fn cut(text: &str) -> (String, bool) {
    if text.len() <= MESSAGE_SHOWN {
        return (text.to_string(), false);
    }
    let mut end = MESSAGE_SHOWN;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

fn failure(error: &tungstenite::Error) -> SendError {
    match error {
        tungstenite::Error::Http(response) => SendError::Handshake(response.status().as_u16()),
        tungstenite::Error::Url(error) => SendError::InvalidUrl(error.to_string()),
        tungstenite::Error::Tls(error) => SendError::Tls(error.to_string()),
        tungstenite::Error::Io(io) => match io.kind() {
            std::io::ErrorKind::ConnectionRefused => SendError::Refused(io.to_string()),
            std::io::ErrorKind::TimedOut => SendError::Timeout,
            _ => {
                let text = io.to_string().to_ascii_lowercase();
                if text.contains("lookup")
                    || text.contains("no such host")
                    || text.contains("resolve")
                {
                    SendError::Unresolved(io.to_string())
                } else if io
                    .get_ref()
                    .is_some_and(|inner| inner.downcast_ref::<rustls::Error>().is_some())
                {
                    SendError::Tls(io.to_string())
                } else {
                    SendError::Network(io.to_string())
                }
            }
        },
        other => SendError::Network(other.to_string()),
    }
}

fn emit<R: Runtime>(app: &AppHandle<R>, id: &str, event: SocketEvent) {
    let _ = app.emit(
        WEBSOCKET_EVENT,
        WebsocketEvent {
            id: id.to_string(),
            at: Utc::now(),
            event,
        },
    );
}

impl Sockets {
    /// Connects, then hands the socket to a task of its own: the command answers once the
    /// handshake did, its failure a code like a send's.
    pub async fn connect<R: Runtime>(
        &self,
        app: AppHandle<R>,
        id: String,
        outgoing: Outgoing,
        protocols: &[String],
    ) -> Result<(), SendError> {
        install_crypto();
        let url = as_socket(&outgoing.url);
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|error| failure(&error))?;
        for (key, value) in &outgoing.headers {
            let name = tungstenite::http::HeaderName::from_bytes(key.as_bytes())
                .map_err(|error| SendError::Network(error.to_string()))?;
            let value = HeaderValue::from_str(value)
                .map_err(|error| SendError::Network(error.to_string()))?;
            request.headers_mut().insert(name, value);
        }
        let protocols: Vec<&str> = protocols
            .iter()
            .map(|protocol| protocol.trim())
            .filter(|protocol| !protocol.is_empty())
            .collect();
        if !protocols.is_empty() {
            let value = HeaderValue::from_str(&protocols.join(", "))
                .map_err(|error| SendError::Network(error.to_string()))?;
            request
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", value);
        }
        let tls = rustls::ClientConfig::with_platform_verifier()
            .map_err(|error| SendError::Tls(error.to_string()))?;
        let (stream, response) = tokio::time::timeout(
            Duration::from_secs(30),
            connect_async_tls_with_config(
                request,
                None,
                false,
                Some(Connector::Rustls(Arc::new(tls))),
            ),
        )
        .await
        .map_err(|_| SendError::Timeout)?
        .map_err(|error| failure(&error))?;

        let protocol = response
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        emit(&app, &id, SocketEvent::Opened { url, protocol });

        let (outbound, inbound) = mpsc::unbounded_channel();
        let task = tauri::async_runtime::spawn(run(app, id.clone(), stream, inbound));
        if let Ok(mut open) = self.open.lock()
            && let Some(previous) = open.insert(
                id,
                Socket {
                    outbound,
                    task: task.inner().abort_handle(),
                },
            )
        {
            previous.task.abort();
        }
        Ok(())
    }

    /// `false` when no socket is open under that id.
    pub fn send(&self, id: &str, text: String) -> bool {
        self.open
            .lock()
            .ok()
            .and_then(|open| {
                open.get(id)
                    .map(|socket| socket.outbound.send(Outbound::Text(text)).is_ok())
            })
            .unwrap_or(false)
    }

    /// Asks the server to close; the task lets go after `CLOSE_GRACE` whatever it answers.
    pub fn close(&self, id: &str) -> bool {
        self.open
            .lock()
            .ok()
            .and_then(|mut open| open.remove(id))
            .is_some_and(|socket| {
                let asked = socket.outbound.send(Outbound::Close).is_ok();
                if !asked {
                    socket.task.abort();
                }
                true
            })
    }

    /// Another library opens: no socket of the one before outlives it.
    pub fn close_all(&self) {
        if let Ok(mut open) = self.open.lock() {
            for (_, socket) in open.drain() {
                socket.task.abort();
            }
        }
    }
}

type Stream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn run<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    stream: Stream,
    mut outbound: mpsc::UnboundedReceiver<Outbound>,
) {
    let (mut write, mut read) = stream.split();
    let mut closing: Option<tokio::time::Instant> = None;
    loop {
        let deadline = closing.map_or_else(
            || tokio::time::Instant::now() + Duration::from_hours(24),
            |since| since + CLOSE_GRACE,
        );
        tokio::select! {
            message = read.next() => match message {
                Some(Ok(Message::Text(text))) => {
                    let size = u32::try_from(text.len()).unwrap_or(u32::MAX);
                    let (text, cut) = cut(text.as_str());
                    emit(&app, &id, SocketEvent::Received { text, size, binary: false, cut });
                }
                Some(Ok(Message::Binary(bytes))) => {
                    let size = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
                    emit(&app, &id, SocketEvent::Received { text: String::new(), size, binary: true, cut: false });
                }
                Some(Ok(Message::Close(frame))) => {
                    emit(&app, &id, closed(frame.as_ref()));
                    break;
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                // A close we asked for that the server cut short is still a close.
                Some(Err(_)) if closing.is_some() => {
                    emit(&app, &id, SocketEvent::Closed { code: Some(1000), reason: String::new() });
                    break;
                }
                Some(Err(error)) => {
                    let failed = AppError::from(failure(&error));
                    emit(&app, &id, SocketEvent::Failed { code: failed.code, detail: failed.detail });
                    break;
                }
                None => {
                    emit(&app, &id, SocketEvent::Closed { code: None, reason: String::new() });
                    break;
                }
            },
            next = outbound.recv(), if closing.is_none() => match next {
                Some(Outbound::Text(text)) => {
                    let size = u32::try_from(text.len()).unwrap_or(u32::MAX);
                    let (shown, _) = cut(&text);
                    match write.send(Message::text(text)).await {
                        Ok(()) => emit(&app, &id, SocketEvent::Sent { text: shown, size }),
                        Err(error) => {
                            let failed = AppError::from(failure(&error));
                            emit(&app, &id, SocketEvent::Failed { code: failed.code, detail: failed.detail });
                            break;
                        }
                    }
                }
                Some(Outbound::Close) | None => {
                    let _ = write
                        .send(Message::Close(Some(CloseFrame { code: CloseCode::Normal, reason: "".into() })))
                        .await;
                    closing = Some(tokio::time::Instant::now());
                }
            },
            () = tokio::time::sleep_until(deadline), if closing.is_some() => {
                emit(&app, &id, SocketEvent::Closed { code: Some(1000), reason: String::new() });
                break;
            }
        }
    }
}

fn closed(frame: Option<&CloseFrame>) -> SocketEvent {
    SocketEvent::Closed {
        code: frame.map(|frame| u16::from(frame.code)),
        reason: frame
            .map(|frame| frame.reason.to_string())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_socket_url_is_checked_as_http_and_sent_back_on_its_scheme() {
        assert_eq!(
            socket_url(" wss://echo.exemple.fr/ws "),
            "https://echo.exemple.fr/ws"
        );
        assert_eq!(socket_url("ws://localhost:8080"), "http://localhost:8080");
        assert_eq!(socket_url("echo.exemple.fr"), "echo.exemple.fr");

        assert_eq!(
            as_socket(&Url::parse("https://a.fr/ws?x=1").unwrap()),
            "wss://a.fr/ws?x=1"
        );
        assert_eq!(
            as_socket(&Url::parse("http://a.fr/").unwrap()),
            "ws://a.fr/"
        );
    }

    #[test]
    fn a_long_message_is_cut_on_a_character() {
        let long = "é".repeat(MESSAGE_SHOWN);
        let (shown, was_cut) = cut(&long);
        assert!(was_cut && shown.len() <= MESSAGE_SHOWN && long.starts_with(&shown));
        assert_eq!(cut("hi"), ("hi".to_string(), false));
    }

    #[test]
    fn a_close_frame_says_its_code_and_reason() {
        let frame = CloseFrame {
            code: CloseCode::Away,
            reason: "bye".into(),
        };
        assert_eq!(
            closed(Some(&frame)),
            SocketEvent::Closed {
                code: Some(1001),
                reason: "bye".to_string()
            }
        );
        assert_eq!(
            closed(None),
            SocketEvent::Closed {
                code: None,
                reason: String::new()
            }
        );
    }
}
