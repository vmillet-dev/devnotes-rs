//! Sending, against a server on the loopback: what left, what came back, and every way it fails.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use devnotes_lib::error::ErrorCode;
use devnotes_lib::http::model::{
    ContainerSettings, HttpItem, HttpItemKind, HttpMethod, KeyValue, RequestAuth, RequestBody,
    RequestDocument, RequestKind,
};
use devnotes_lib::http::send::{SHOWN_LIMIT, SendRequest, SentBody};
use devnotes_lib::http::{
    cancel_http_send, create_http_collection, forget_http_response, http_response_image,
    save_http_response, save_http_settings, send_http_request,
};
use devnotes_lib::notes::language::Language;

use super::{Session, code};

/// What the server read: the request line and headers, then the body.
struct Seen {
    head: String,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Seen {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let end = loop {
        let read = stream.read(&mut buffer).unwrap();
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break end + 4;
        }
        assert!(read > 0, "the request ended before its headers");
    };
    let head = String::from_utf8_lossy(&bytes[..end]).into_owned();
    let length = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    while bytes.len() < end + length {
        let read = stream.read(&mut buffer).unwrap();
        bytes.extend_from_slice(&buffer[..read]);
    }
    Seen {
        head,
        body: bytes[end..end + length].to_vec(),
    }
}

/// Answers each connection with what `answer` writes for it, and reports what it read.
fn serve(
    connections: usize,
    answer: impl Fn(&Seen, u16) -> Vec<u8> + Send + 'static,
) -> (u16, mpsc::Receiver<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (seen, received) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming().take(connections) {
            let mut stream = stream.unwrap();
            let request = read_request(&mut stream);
            stream.write_all(&answer(&request, port)).unwrap();
            stream.flush().unwrap();
            let _ = seen.send(request);
        }
    });
    (port, received)
}

fn response(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    write!(
        out,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    )
    .unwrap();
    for (name, value) in headers {
        write!(out, "{name}: {value}\r\n").unwrap();
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
    out
}

fn request(url: &str) -> SendRequest {
    SendRequest {
        id: "tab-1".to_string(),
        kind: RequestKind::Http,
        method: HttpMethod::Get,
        document: RequestDocument {
            url: url.to_string(),
            ..RequestDocument::default()
        },
        collection_id: None,
        folder_id: None,
        name: "Ping".to_string(),
        request_id: None,
    }
}

#[test]
fn a_request_leaves_with_what_its_collection_hands_down_and_its_answer_comes_back() {
    let session = Session::open();
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let settings = ContainerSettings {
        auth: RequestAuth::Bearer {
            token: "s3cret".to_string(),
        },
        headers: vec![KeyValue {
            key: "Accept".to_string(),
            value: "application/json".to_string(),
            ..KeyValue::default()
        }],
        ..ContainerSettings::default()
    };
    session
        .call(|app| {
            save_http_settings(
                HttpItem {
                    kind: HttpItemKind::Collection,
                    id: api.id.clone(),
                },
                settings,
                app,
            )
        })
        .unwrap();
    let (port, seen) = serve(1, |_, _| {
        response(
            "201 Created",
            &[
                ("Content-Type", "application/json"),
                ("Set-Cookie", "session=abc; Path=/; HttpOnly"),
            ],
            b"{\"id\":7}",
        )
    });

    let mut sent = request(&format!("http://127.0.0.1:{port}/users?page=1"));
    sent.method = HttpMethod::Post;
    sent.collection_id = Some(api.id.clone());
    sent.document.body = RequestBody::Json {
        text: "{\"name\":\"Ada\"}".to_string(),
    };
    let answer = session.call(|app| send_http_request(sent, app)).unwrap();

    assert_eq!((answer.status, answer.reason.as_str()), (201, "Created"));
    assert_eq!((answer.body.as_str(), answer.size), ("{\"id\":7}", 8));
    assert!(!answer.binary && !answer.cut && !answer.incomplete);
    assert_eq!(answer.language, Language::Json);
    assert_eq!(answer.pretty.as_deref(), Some("{\n  \"id\": 7\n}"));
    assert_eq!(answer.cookies.len(), 1);
    assert!(answer.cookies[0].http_only);
    assert_eq!(answer.exchange.method, HttpMethod::Post);
    assert_eq!(
        answer.exchange.body,
        SentBody::Text {
            text: "{\"name\":\"Ada\"}".to_string()
        }
    );
    assert!(
        answer
            .exchange
            .headers
            .iter()
            .any(|header| header.key == "Authorization" && header.value == "Bearer s3cret")
    );
    assert!(
        answer
            .headers
            .iter()
            .any(|header| header.key == "content-type" && header.value == "application/json")
    );

    let seen = seen.recv().unwrap();
    assert!(seen.head.starts_with("POST /users?page=1 HTTP/1.1\r\n"));
    let head = seen.head.to_ascii_lowercase();
    assert!(head.contains("authorization: bearer s3cret\r\n"));
    assert!(head.contains("accept: application/json\r\n"));
    assert!(head.contains("content-type: application/json\r\n"));
    assert_eq!(seen.body, b"{\"name\":\"Ada\"}");
}

#[test]
fn the_redirects_are_followed_and_recorded() {
    let session = Session::open();
    let (port, _seen) = serve(2, |seen, port| {
        if seen.head.starts_with("GET /old ") {
            response(
                "301 Moved Permanently",
                &[("Location", &format!("http://127.0.0.1:{port}/new"))],
                b"",
            )
        } else {
            response("200 OK", &[("Content-Type", "text/plain")], b"here")
        }
    });

    let answer = session
        .call(|app| send_http_request(request(&format!("http://127.0.0.1:{port}/old")), app))
        .unwrap();

    assert_eq!(answer.body, "here");
    assert_eq!(answer.url, format!("http://127.0.0.1:{port}/new"));
    assert_eq!(answer.redirects.len(), 1);
    assert_eq!(answer.redirects[0].status, 301);
}

#[test]
fn a_body_past_what_is_shown_is_cut_and_saved_whole_to_a_file() {
    let session = Session::open();
    let large = vec![b'a'; SHOWN_LIMIT + 10];
    let served = large.clone();
    let (port, _seen) = serve(1, move |_, _| {
        response("200 OK", &[("Content-Type", "text/plain")], &served)
    });

    let answer = session
        .call(|app| send_http_request(request(&format!("http://127.0.0.1:{port}/")), app))
        .unwrap();
    assert!(answer.cut && !answer.incomplete);
    assert_eq!(answer.body.len(), SHOWN_LIMIT);
    assert_eq!(answer.size as usize, large.len());

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("body.txt");
    let target = path.to_string_lossy().into_owned();
    assert!(
        session
            .call(|app| save_http_response("tab-1".to_string(), target.clone(), app))
            .unwrap()
    );
    assert_eq!(std::fs::read(&path).unwrap(), large);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);

    session
        .call(|app| forget_http_response("tab-1".to_string(), app))
        .unwrap();
    assert!(
        !session
            .call(|app| save_http_response("tab-1".to_string(), target, app))
            .unwrap()
    );
}

#[test]
fn an_image_is_kept_as_bytes_not_shown_as_text() {
    let session = Session::open();
    let (port, _seen) = serve(1, |_, _| {
        response(
            "200 OK",
            &[("Content-Type", "image/png")],
            &[0x89, b'P', b'N', b'G'],
        )
    });

    let answer = session
        .call(|app| send_http_request(request(&format!("http://127.0.0.1:{port}/a.png")), app))
        .unwrap();

    assert!(answer.binary && answer.cut);
    assert_eq!((answer.body.as_str(), answer.size), ("", 4));
    assert_eq!(
        session
            .call(|app| http_response_image("tab-1".to_string(), app))
            .unwrap()
            .as_deref(),
        Some("data:image/png;base64,iVBORw==")
    );
}

#[test]
fn a_variable_a_bad_url_a_closed_port_and_a_plain_server_over_tls_each_fail_by_code() {
    let session = Session::open();
    let failed = |url: &str| code(session.call(|app| send_http_request(request(url), app)));

    assert_eq!(failed("{{baseUrl}}/users"), ErrorCode::HttpVariable);
    assert_eq!(failed("ftp://example.com"), ErrorCode::HttpInvalidUrl);
    // `.invalid` is reserved never to resolve.
    assert_eq!(
        failed("http://devnotes.invalid/"),
        ErrorCode::HttpUnresolved
    );

    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    assert_eq!(
        failed(&format!("http://127.0.0.1:{port}/")),
        ErrorCode::HttpRefused
    );

    // Reads the client's hello and answers it in plain text. ⚠️ Closed before the client reads,
    // the connection is reset on Windows, and the reset is all the client sees.
    let plain = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = plain.local_addr().unwrap().port();
    thread::spawn(move || {
        let (mut stream, _) = plain.accept().unwrap();
        let _ = stream.read(&mut [0; 4096]);
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
        thread::sleep(Duration::from_secs(2));
    });
    assert_eq!(
        failed(&format!("https://127.0.0.1:{port}/")),
        ErrorCode::HttpTls
    );
}

#[test]
fn a_send_under_way_is_cancelled_by_its_id() {
    let session = Session::open();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    // Accepts, then answers nothing until the test is over.
    let silent = thread::spawn(move || listener.accept().map(|(stream, _)| stream));

    let app = session.0.handle().clone();
    let sending = tauri::async_runtime::spawn(send_http_request(
        request(&format!("http://127.0.0.1:{port}/slow")),
        app.clone(),
    ));
    // Polled from the test's own thread: the send is under way once there is one to cancel.
    while !tauri::async_runtime::block_on(cancel_http_send("tab-1".to_string(), app.clone()))
        .unwrap()
    {
        thread::sleep(Duration::from_millis(10));
    }
    let cancelled = tauri::async_runtime::block_on(sending).unwrap();

    assert_eq!(cancelled.unwrap_err().code, ErrorCode::HttpCancelled);
    assert!(
        !session
            .call(|app| cancel_http_send("tab-1".to_string(), app))
            .unwrap()
    );
    drop(silent);
}

#[test]
fn every_send_that_left_is_recorded_masked_and_the_history_clears() {
    use devnotes_lib::http::model::KeyPlace;
    use devnotes_lib::http::{
        clear_http_history, count_http_history, http_history, http_history_draft,
        http_history_entry,
    };

    let session = Session::open();
    let (port, _seen) = serve(1, |_, _| {
        response("200 OK", &[("Content-Type", "text/plain")], b"pong")
    });
    let mut keyed = request(&format!("http://127.0.0.1:{port}/ping?key=s3cret"));
    keyed.document.auth = RequestAuth::ApiKey {
        name: "key".to_string(),
        value: "s3cret".to_string(),
        place: KeyPlace::Query,
    };
    keyed.document.headers = vec![KeyValue {
        key: "Authorization".to_string(),
        value: "Bearer t0ken".to_string(),
        ..KeyValue::default()
    }];
    session.call(|app| send_http_request(keyed, app)).unwrap();

    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    let refused =
        session.call(|app| send_http_request(request(&format!("http://127.0.0.1:{port}/")), app));
    assert!(refused.is_err());
    let never_left = session.call(|app| send_http_request(request("{{baseUrl}}/x"), app));
    assert!(never_left.is_err());

    let days = session.call(|app| http_history(0, app)).unwrap();
    assert_eq!(days.len(), 1);
    let items = &days[0].items;
    assert_eq!(items.len(), 2);
    assert_eq!(
        (items[0].status, items[0].summary.failure),
        (None, Some(ErrorCode::HttpRefused))
    );
    assert_eq!(items[1].status, Some(200));
    assert_eq!(items[1].summary.name, "Ping");
    assert!(
        items[1]
            .summary
            .url
            .ends_with("/ping?key=••••••••&key=••••••••")
    );

    let entry = session
        .call(|app| http_history_entry(items[1].id.clone(), app))
        .unwrap();
    let response = entry.record.response.unwrap();
    assert_eq!(response.body, "pong");
    assert!(
        entry
            .record
            .exchange
            .headers
            .iter()
            .any(|header| header.key == "Authorization" && header.value == "Bearer ••••••••")
    );

    let draft = session
        .call(|app| http_history_draft(items[1].id.clone(), app))
        .unwrap();
    assert!(draft.document.url.ends_with("/ping"));
    assert!(draft.document.headers.is_empty());

    assert_eq!(session.call(count_http_history).unwrap(), 2);
    assert_eq!(session.call(clear_http_history).unwrap(), 2);
    assert!(session.call(|app| http_history(0, app)).unwrap().is_empty());
}

#[test]
fn a_graphql_request_is_posted_as_json_and_its_errors_read_apart_from_its_data() {
    use devnotes_lib::http::describe_graphql;
    use devnotes_lib::http::graphql::GraphqlDocument;

    let session = Session::open();
    let (port, seen) = serve(1, |_, _| {
        response(
            "200 OK",
            &[("Content-Type", "application/json")],
            br#"{"data":{"me":null},"errors":[{"message":"Unauthorised","path":["me"]}]}"#,
        )
    });
    let mut sent = request(&format!("http://127.0.0.1:{port}/graphql"));
    sent.kind = RequestKind::Graphql;
    sent.document.graphql = GraphqlDocument {
        query: "query Me { me { id } } query Other { x }".to_string(),
        operation_name: Some("Me".to_string()),
        ..GraphqlDocument::default()
    };
    let document = sent.document.graphql.clone();

    let answer = session.call(|app| send_http_request(sent, app)).unwrap();

    let read = answer.graphql.unwrap();
    assert_eq!(read.data.as_deref(), Some("{\n  \"me\": null\n}"));
    assert_eq!(read.errors[0].message, "Unauthorised");
    assert_eq!(read.errors[0].path.as_deref(), Some("me"));
    let seen = seen.recv().unwrap();
    assert!(seen.head.starts_with("POST /graphql HTTP/1.1\r\n"));
    assert!(
        String::from_utf8(seen.body)
            .unwrap()
            .contains("\"operationName\":\"Me\"")
    );

    let described = session.call(|_| describe_graphql(document)).unwrap();
    assert_eq!(described.operations, ["Me", "Other"]);
}

#[test]
fn an_answer_fills_the_jar_the_next_request_carries_it_and_the_manager_empties_it() {
    use devnotes_lib::http::{
        clear_http_cookies, count_http_cookies, delete_http_cookie, delete_http_cookie_domain,
        http_cookies,
    };

    let session = Session::open();
    let (port, seen) = serve(3, |seen, _| {
        if seen.head.starts_with("GET /login ") {
            response(
                "200 OK",
                &[
                    ("Set-Cookie", "session=abc; Path=/; HttpOnly"),
                    ("Set-Cookie", "theme=dark; Path=/"),
                ],
                b"",
            )
        } else {
            response("200 OK", &[], b"")
        }
    });
    let base = format!("http://127.0.0.1:{port}");

    session
        .call(|app| send_http_request(request(&format!("{base}/login")), app))
        .unwrap();
    session
        .call(|app| send_http_request(request(&format!("{base}/me")), app))
        .unwrap();
    let mut alone = request(&format!("{base}/me"));
    alone.document.transport.use_cookies = Some(false);
    session.call(|app| send_http_request(alone, app)).unwrap();

    let heads: Vec<String> = seen.iter().take(3).map(|seen| seen.head).collect();
    assert!(!heads[0].to_ascii_lowercase().contains("cookie:"));
    let carried = heads[1].to_ascii_lowercase();
    assert!(carried.contains("cookie: "), "{carried}");
    assert!(carried.contains("session=abc") && carried.contains("theme=dark"));
    assert!(!heads[2].to_ascii_lowercase().contains("cookie:"));

    let domains = session.call(http_cookies).unwrap();
    assert_eq!(domains.len(), 1);
    assert_eq!(domains[0].domain, "127.0.0.1");
    assert_eq!(domains[0].cookies.len(), 2);
    let id = domains[0].cookies[0].id.clone();
    assert_eq!(session.call(|app| delete_http_cookie(id, app)).unwrap(), 1);
    assert_eq!(session.call(count_http_cookies).unwrap(), 1);
    assert_eq!(
        session
            .call(|app| delete_http_cookie_domain("127.0.0.1".to_string(), app))
            .unwrap(),
        1
    );
    assert_eq!(session.call(clear_http_cookies).unwrap(), 0);
}

#[test]
fn a_request_set_not_to_follow_redirects_answers_the_redirect_itself() {
    let session = Session::open();
    let (port, _seen) = serve(1, |_, port| {
        response(
            "302 Found",
            &[("Location", &format!("http://127.0.0.1:{port}/new"))],
            b"",
        )
    });
    let mut sent = request(&format!("http://127.0.0.1:{port}/old"));
    sent.document.transport.follow_redirects = Some(false);

    let answer = session.call(|app| send_http_request(sent, app)).unwrap();

    assert_eq!(answer.status, 302);
    assert!(answer.redirects.is_empty());
}
