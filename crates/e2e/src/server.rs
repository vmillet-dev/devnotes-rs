//! A server on this machine for the scenarios' sends, as the reference's suite had: `/json`
//! answers a small JSON and sets a cookie, `/slow` the same after two seconds, `/echo` what it
//! was sent, `/redirect/N` N redirects then `/json`, `/bytes` what is no text, `/png` a picture,
//! anything else is a 404. Each connection is answered once, then closed.

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
    let head = String::from_utf8_lossy(&head).into_owned();
    let path = head.split_whitespace().nth(1).unwrap_or("/").to_owned();
    if path.starts_with("/slow") {
        #[expect(
            clippy::disallowed_methods,
            reason = "the server's own latency, not a scenario waiting"
        )]
        std::thread::sleep(Duration::from_secs(2));
    }
    let sent = body_of(&mut stream, &head);
    let reply = reply(&path, &head, &sent);
    let answer = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\n{}\
         Set-Cookie: session=abc; Path=/\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n",
        reply.status,
        reply.kind,
        reply.more,
        reply.body.len()
    );
    let _ = stream.write_all(answer.as_bytes());
    let _ = stream.write_all(&reply.body);
}

struct Reply {
    status: &'static str,
    kind: &'static str,
    more: String,
    body: Vec<u8>,
}

fn body_of(stream: &mut TcpStream, head: &str) -> Vec<u8> {
    let length = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    let mut body = vec![0_u8; length];
    let _ = stream.read_exact(&mut body);
    body
}

fn reply(path: &str, head: &str, sent: &[u8]) -> Reply {
    let plain = |status, kind, body: &[u8]| Reply {
        status,
        kind,
        more: String::new(),
        body: body.to_vec(),
    };
    if let Some(left) = path.strip_prefix("/redirect/") {
        let left: u32 = left.parse().unwrap_or(0);
        let to = if left > 1 {
            format!("/redirect/{}", left - 1)
        } else {
            "/json".to_owned()
        };
        return Reply {
            more: format!("Location: {to}\r\n"),
            ..plain("302 Found", "text/plain", b"")
        };
    }
    if path.starts_with("/json") || path.starts_with("/slow") {
        let body = br#"{"name":"Ada","languages":["fr","en"],"admin":true}"#;
        return plain("200 OK", "application/json", body);
    }
    if path.starts_with("/echo") {
        let body = [head.as_bytes(), sent].concat();
        return plain("200 OK", "text/plain", &body);
    }
    if path.starts_with("/bytes") {
        return plain("200 OK", "application/octet-stream", &[0xff; 2048]);
    }
    if path.starts_with("/png") {
        return plain("200 OK", "image/png", PIXEL);
    }
    plain("404 Not Found", "application/json", b"missing")
}

/// One white pixel.
const PIXEL: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xff, 0xff, 0xff,
    0x7f, 0x00, 0x09, 0xfb, 0x03, 0xfd, 0x2a, 0x86, 0xe3, 0x8a, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];
