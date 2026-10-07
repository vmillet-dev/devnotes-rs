//! A socket against an echo server on the loopback: opened, written, read and closed, every
//! step an event on the one topic.

use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use devnotes_lib::error::ErrorCode;
use devnotes_lib::http::model::RequestDocument;
use devnotes_lib::http::websocket::{
    SocketEvent, WEBSOCKET_EVENT, WebsocketDocument, WebsocketEvent,
};
use devnotes_lib::http::{WebsocketRequest, close_websocket, connect_websocket, send_websocket};
use tauri::Listener;
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

/// Echoes each text as `echo: …`, and says which subprotocol it took.
fn echo() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut socket = tungstenite::accept_hdr(stream, take_chat).unwrap();
        while let Ok(message) = socket.read() {
            match message {
                Message::Text(text) => socket.send(Message::text(format!("echo: {text}"))).unwrap(),
                Message::Close(_) => break,
                _ => {}
            }
        }
    });
    port
}

fn request(url: &str) -> WebsocketRequest {
    WebsocketRequest {
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
    let (heard, events) = mpsc::channel::<WebsocketEvent>();
    session.0.handle().listen(WEBSOCKET_EVENT, move |event| {
        let _ = heard.send(serde_json::from_str(event.payload()).unwrap());
    });
    let next = || events.recv_timeout(Duration::from_secs(5)).unwrap().event;
    let port = echo();

    session
        .call(|app| connect_websocket(request(&format!("ws://127.0.0.1:{port}/")), app))
        .unwrap();
    assert_eq!(
        next(),
        SocketEvent::Opened {
            url: format!("ws://127.0.0.1:{port}/"),
            protocol: Some("chat".to_string())
        }
    );

    assert!(
        session
            .call(|app| send_websocket("socket-1".to_string(), "hello".to_string(), app))
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
            .call(|app| close_websocket("socket-1".to_string(), app))
            .unwrap()
    );
    assert!(matches!(next(), SocketEvent::Closed { .. }));
    assert!(
        !session
            .call(|app| send_websocket("socket-1".to_string(), "late".to_string(), app))
            .unwrap()
    );
}

#[test]
fn a_closed_port_and_a_refused_handshake_fail_by_code() {
    let session = Session::open();
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    assert_eq!(
        code(
            session.call(|app| connect_websocket(request(&format!("ws://127.0.0.1:{port}/")), app))
        ),
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
        .call(|app| connect_websocket(request(&format!("ws://127.0.0.1:{port}/")), app))
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::HttpWebsocketRefused);
    assert_eq!(
        refused.params.get("status").map(String::as_str),
        Some("401")
    );
}
