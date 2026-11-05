//! A server on this machine for the scenarios' sends, as the reference's suite had: `/json`
//! answers a small JSON and sets a cookie, `/slow` the same after two seconds, anything else is a
//! 404. Each connection is answered once, then closed.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::time::Duration;

/// Its port: the server lives as long as the scenario's process.
pub fn serve() -> u16 {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a local port is free");
    let port = listener.local_addr().expect("the port is known").port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || answer(stream));
        }
    });
    port
}

/// A socket server echoing each text, for the scenarios' connections; its port.
pub fn serve_socket() -> u16 {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a local port is free");
    let port = listener.local_addr().expect("the port is known").port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || echo(stream));
        }
    });
    port
}

fn echo(stream: TcpStream) {
    let Ok(mut socket) = tungstenite::accept(stream) else {
        return;
    };
    while let Ok(message) = socket.read() {
        if message.is_text() && socket.send(message).is_err() {
            return;
        }
    }
}

fn answer(mut stream: TcpStream) {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head);
    let path = head.split_whitespace().nth(1).unwrap_or("/").to_owned();
    if path.starts_with("/slow") {
        #[expect(
            clippy::disallowed_methods,
            reason = "the server's own latency, not a scenario waiting"
        )]
        std::thread::sleep(Duration::from_secs(2));
    }
    let (status, body) = if path.starts_with("/json") || path.starts_with("/slow") {
        (
            "200 OK",
            r#"{"name":"Ada","languages":["fr","en"],"admin":true}"#,
        )
    } else {
        ("404 Not Found", "missing")
    };
    let answer = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n\
         Set-Cookie: session=abc; Path=/\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(answer.as_bytes());
}
