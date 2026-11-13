//! A socket against an echo server on the loopback: opened, written, read and closed, every
//! step an event on the one topic.

use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use engine::EngineEvent;
use engine::error::ErrorCode;
use engine::http::model::RequestDocument;
use engine::http::websocket::{SocketEvent, WebsocketDocument};
use engine::http::{WebsocketRequest, close_websocket, connect_websocket, send_websocket};
use tokio_tungstenite::tungstenite::{self, Message};

use super::{Session, code};

/// Answers a client asking for a subprotocol with `chat`.
#[allow(clippy::result_large_err, clippy::unnecessary_wraps)] // tungstenite's callback signature
fn take_chat(
    request: &tungstenite::handshake::server::Request,
    mut response: tungstenite::handshake::server::Response,
) -> Result<tungstenite::handshake::server::Response, tungstenite::handshake::server::ErrorResponse>
{
    if request.headers().contains_key("sec-websocket-protocol") {
        response
            .headers_mut()
            .insert("sec-websocket-protocol", "chat".parse().unwrap());
    }
    Ok(response)
}

/// Echoes each text as `echo: …`, says which subprotocol it took, and what the handshake
/// named as its agent.
#[allow(clippy::result_large_err)] // tungstenite's callback signature
fn echo() -> (u16, mpsc::Receiver<Option<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (named, agent) = mpsc::channel();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut socket = tungstenite::accept_hdr(
            stream,
            |request: &tungstenite::handshake::server::Request, response| {
                let agent = request.headers().get("user-agent");
                let _ = named.send(
                    agent
                        .and_then(|value| value.to_str().ok())
                        .map(str::to_string),
                );
                take_chat(request, response)
            },
        )
        .unwrap();
        while let Ok(message) = socket.read() {
            match message {
                Message::Text(text) => socket.send(Message::text(format!("echo: {text}"))).unwrap(),
                Message::Close(_) => break,
                _ => {}
            }
        }
    });
    (port, agent)
}

fn request(url: &str) -> WebsocketRequest {
    WebsocketRequest {
        environment_id: None,
        id: "socket-1".to_string(),
        document: RequestDocument {
            url: url.to_string(),
            websocket: WebsocketDocument {
                protocols: vec!["chat".to_string()],
                messages: Vec::new(),
            },
            ..RequestDocument::default()
        },
        collection_id: None,
        folder_id: None,
    }
}

#[test]
fn a_socket_opens_echoes_and_closes_each_step_an_event() {
    let session = Session::open();
    let seen = std::cell::Cell::new(0);
    let next = || {
        let waited = std::time::Instant::now();
        loop {
            let said: Vec<SocketEvent> = session
                .heard
                .events()
                .into_iter()
                .filter_map(|event| match event {
                    EngineEvent::Socket(event) => Some(event.event),
                    _ => None,
                })
                .collect();
            if let Some(event) = said.get(seen.get()) {
                seen.set(seen.get() + 1);
                return event.clone();
            }
            assert!(
                waited.elapsed() < Duration::from_secs(5),
                "no socket event came"
            );
            thread::sleep(Duration::from_millis(10));
        }
    };
    let (port, agent) = echo();

    session
        .wait(|e| connect_websocket(e, request(&format!("ws://127.0.0.1:{port}/"))))
        .unwrap();
    // The handshake is composed like a request: it names the application too.
    assert_eq!(
        agent
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .as_deref(),
        Some(engine::http::send::USER_AGENT)
    );
    assert_eq!(
        next(),
        SocketEvent::Opened {
            url: format!("ws://127.0.0.1:{port}/"),
            protocol: Some("chat".to_string())
        }
    );

    assert!(
        session
            .call(|e| send_websocket(e, "socket-1".to_string(), "hello".to_string()))
            .unwrap()
    );
    assert_eq!(
        next(),
        SocketEvent::Sent {
            text: "hello".to_string(),
            size: 5
        }
    );
    assert_eq!(
        next(),
        SocketEvent::Received {
            text: "echo: hello".to_string(),
            size: 11,
            binary: false,
            cut: false
        }
    );

    assert!(
        session
            .call(|e| close_websocket(e, "socket-1".to_string()))
            .unwrap()
    );
    assert!(matches!(next(), SocketEvent::Closed { .. }));
    assert!(
        !session
            .call(|e| send_websocket(e, "socket-1".to_string(), "late".to_string()))
            .unwrap()
    );
}

/// `wss://nexistepas.invalid` answered « Erreur réseau » where the same host over HTTP answered
/// « Le nom d'hôte ne se résout pas »; and `https://` in a socket was dialled without a word.
#[test]
fn a_socket_names_a_host_that_does_not_resolve_and_refuses_a_scheme_that_is_not_its_own() {
    let session = Session::open();
    // `.invalid` is reserved never to resolve.
    assert_eq!(
        code(session.wait(|e| connect_websocket(e, request("ws://devnotes.invalid/")))),
        ErrorCode::HttpUnresolved
    );

    let refused = session
        .wait(|e| connect_websocket(e, request("https://echo.exemple.fr/live?x=1")))
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::HttpWebsocketScheme);
    assert_eq!(
        refused.params.get("twin").map(String::as_str),
        Some("wss://echo.exemple.fr/live?x=1")
    );
    let refused = session
        .wait(|e| connect_websocket(e, request("ftp://exemple.fr")))
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::HttpWebsocketScheme);
    assert_eq!(refused.params.get("twin"), None);
}

#[test]
fn a_closed_port_and_a_refused_handshake_fail_by_code() {
    let session = Session::open();
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    assert_eq!(
        code(session.wait(|e| connect_websocket(e, request(&format!("ws://127.0.0.1:{port}/"))))),
        ErrorCode::HttpRefused
    );

    let plain = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = plain.local_addr().unwrap().port();
    thread::spawn(move || {
        use std::io::{Read, Write};
        let (mut stream, _) = plain.accept().unwrap();
        let _ = stream.read(&mut [0; 4096]);
        let _ = stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n");
        thread::sleep(Duration::from_secs(1));
    });
    let refused = session
        .wait(|e| connect_websocket(e, request(&format!("ws://127.0.0.1:{port}/"))))
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::HttpWebsocketRefused);
    assert_eq!(
        refused.params.get("status").map(String::as_str),
        Some("401")
    );
}

/// Opens sockets over TLS with a certificate nobody signed, for 127.0.0.1.
fn self_signed_echo() -> u16 {
    let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(certified.signing_key.serialize_der().into());
    let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![certified.cert.der().clone()], key)
        .unwrap();
    let config = std::sync::Arc::new(config);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let connection = rustls::ServerConnection::new(std::sync::Arc::clone(&config)).unwrap();
            let tls = rustls::StreamOwned::new(connection, stream);
            thread::spawn(move || {
                if let Ok(mut socket) = tungstenite::accept_hdr(tls, take_chat) {
                    while socket.read().is_ok() {}
                }
            });
        }
    });
    port
}

#[test]
fn a_self_signed_server_opens_only_with_the_check_off() {
    let session = Session::open();
    let port = self_signed_echo();
    let url = format!("wss://127.0.0.1:{port}/");

    let checked = session.wait(|e| connect_websocket(e, request(&url)));
    assert_eq!(code(checked), ErrorCode::HttpTls);

    let mut unchecked = request(&url);
    unchecked.id = "socket-2".to_string();
    unchecked.document.transport.verify_tls = Some(false);
    let opened = session.wait(|e| connect_websocket(e, unchecked));
    assert!(opened.is_ok(), "{opened:?}");
}
