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
use devnotes_lib::http::send::{ImpliedSource, SHOWN_LIMIT, SendRequest, SentBody, USER_AGENT};
use devnotes_lib::http::transport::TransportSettings;
use devnotes_lib::http::{
    cancel_http_send, create_http_collection, forget_http_response, http_response_image,
    implied_http_headers, save_http_response, save_http_settings, send_http_request,
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
            // A client that gave up — its delay ran out, it was cancelled — is not waited for.
            let _ = stream.write_all(&answer(&request, port));
            let _ = stream.flush();
            let _ = seen.send(request);
        }
    });
    (port, received)
}

/// A header of what the server read, by its name whatever its case.
fn header_of(seen: &Seen, name: &str) -> Option<String> {
    seen.head.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name)
            .then(|| value.trim().to_string())
    })
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
        environment_id: None,
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

fn variable(name: &str, value: &str) -> devnotes_lib::http::environments::Variable {
    devnotes_lib::http::environments::Variable {
        name: name.to_string(),
        value: value.to_string(),
        ..Default::default()
    }
}

/// A request in a folder of a collection, each level setting variables, sent with Staging
/// over globals that would send it nowhere.
fn layered(session: &Session, port: u16) -> SendRequest {
    use devnotes_lib::http::environments::{EnvironmentPatch, VariableKind};
    use devnotes_lib::http::{
        create_http_environment, create_http_folder, save_http_environment, save_http_globals,
    };

    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let invoices = session
        .call(|app| create_http_folder(api.id.clone(), None, "Factures".to_string(), app))
        .unwrap();
    for (kind, id, variables) in [
        (
            HttpItemKind::Collection,
            &api.id,
            vec![variable("tenant", "acme"), variable("page", "9")],
        ),
        (
            HttpItemKind::Folder,
            &invoices.id,
            vec![variable("page", "3")],
        ),
    ] {
        let item = HttpItem {
            kind,
            id: id.clone(),
        };
        let settings = ContainerSettings {
            variables,
            ..ContainerSettings::default()
        };
        session
            .call(|app| save_http_settings(item, settings, app))
            .unwrap();
    }
    let staging = session
        .call(|app| create_http_environment("Staging".to_string(), app))
        .unwrap();
    let mut token = variable("token", "t0k3n");
    token.kind = VariableKind::Secret;
    let patch = EnvironmentPatch {
        variables: Some(vec![
            variable("baseUrl", &format!("http://127.0.0.1:{port}")),
            token,
            variable("tenant", "staging-tenant"),
        ]),
        ..EnvironmentPatch::default()
    };
    session
        .call(|app| save_http_environment(staging.id.clone(), patch, app))
        .unwrap();
    session
        .call(|app| save_http_globals(vec![variable("baseUrl", "http://nowhere.invalid")], app))
        .unwrap();

    let mut sent = request("{{baseUrl}}/invoices?page={{page}}&size={{size}}");
    sent.method = HttpMethod::Post;
    sent.collection_id = Some(api.id);
    sent.folder_id = Some(invoices.id);
    sent.environment_id = Some(staging.id);
    sent.document.variables = vec![variable("size", "50")];
    sent.document.headers = vec![KeyValue {
        key: "X-Tenant".to_string(),
        value: "{{tenant}}".to_string(),
        ..KeyValue::default()
    }];
    sent.document.auth = RequestAuth::Bearer {
        token: "{{token}}".to_string(),
    };
    sent.document.body = RequestBody::Json {
        text: r#"{"tenant":"{{tenant}}"}"#.to_string(),
    };
    sent
}

/// #486: the nearest level wins — the request's own, a folder's, the collection's, the
/// environment chosen, the globals — in the URL, a header, the body and the auth.
#[test]
fn a_variable_is_sent_as_the_nearest_level_sets_it() {
    use devnotes_lib::http::variables::VariableScope;
    use devnotes_lib::http::{VariablesRequest, http_variables};

    let session = Session::open();
    let (port, seen) = serve(1, |_, _| response("200 OK", &[], b"ok"));
    let sent = layered(&session, port);

    let asked = VariablesRequest {
        kind: RequestKind::Http,
        document: sent.document.clone(),
        collection_id: sent.collection_id.clone(),
        folder_id: sent.folder_id.clone(),
        environment_id: sent.environment_id.clone(),
    };
    let used = session.call(|app| http_variables(asked, app)).unwrap();
    let scopes: Vec<(&str, Option<VariableScope>)> = used
        .iter()
        .map(|known| {
            (
                known.name.as_str(),
                known.resolved.as_ref().map(|found| found.scope),
            )
        })
        .collect();
    assert_eq!(
        scopes,
        [
            ("baseUrl", Some(VariableScope::Environment)),
            ("page", Some(VariableScope::Folder)),
            ("size", Some(VariableScope::Request)),
            // The collection is nearer than the environment.
            ("tenant", Some(VariableScope::Collection)),
            ("token", Some(VariableScope::Environment)),
        ]
    );

    session.call(|app| send_http_request(sent, app)).unwrap();
    let seen = seen.recv().unwrap();
    assert!(
        seen.head
            .starts_with("POST /invoices?page=3&size=50 HTTP/1.1\r\n"),
        "{}",
        seen.head
    );
    assert_eq!(header_of(&seen, "x-tenant").as_deref(), Some("acme"));
    assert_eq!(
        header_of(&seen, "authorization").as_deref(),
        Some("Bearer t0k3n")
    );
    assert_eq!(seen.body, br#"{"tenant":"acme"}"#);
}

/// Fork C: a name no level sets is refused by name, and nothing leaves.
#[test]
fn a_variable_set_nowhere_stops_the_send_by_its_name() {
    let session = Session::open();
    let mut sent = request("http://127.0.0.1:9/tenants/{{tenantId}}");
    sent.document.variables = vec![variable("other", "1")];

    let answer = session.call(|app| send_http_request(sent, app));

    let name = answer
        .as_ref()
        .err()
        .and_then(|error| error.params.get("name").cloned());
    assert_eq!(code(answer), ErrorCode::HttpVariable);
    assert_eq!(name.as_deref(), Some("tenantId"));
}

/// `api.github.com` answers 403 to a request that names no agent: none was sent.
#[test]
fn a_request_names_the_application_unless_a_row_says_otherwise_or_switches_it_off() {
    let session = Session::open();
    let (port, seen) = serve(5, |_, _| response("200 OK", &[], b"ok"));
    let url = format!("http://127.0.0.1:{port}/events");
    let agent = |seen: &Seen| {
        seen.head.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("user-agent")
                .then(|| value.trim().to_string())
        })
    };
    let row = |value: &str, enabled: bool| KeyValue {
        enabled,
        key: "User-Agent".to_string(),
        value: value.to_string(),
        ..KeyValue::default()
    };
    let hand_down = |id: &str, headers: Vec<KeyValue>| {
        let item = HttpItem {
            kind: HttpItemKind::Collection,
            id: id.to_string(),
        };
        let settings = ContainerSettings {
            headers,
            ..ContainerSettings::default()
        };
        session
            .call(|app| save_http_settings(item, settings, app))
            .unwrap();
    };

    // Nothing said: the application and its version, as the editor lists them.
    let answer = session
        .call(|app| send_http_request(request(&url), app))
        .unwrap();
    let left = seen.recv().unwrap();
    assert_eq!(agent(&left).as_deref(), Some(USER_AGENT));
    assert!(left.head.to_ascii_lowercase().contains("accept: */*\r\n"));
    assert!(
        answer
            .exchange
            .headers
            .iter()
            .any(|header| header.key == "User-Agent" && header.value == USER_AGENT)
    );
    let listed = session
        .call(|app| {
            implied_http_headers(RequestKind::Http, request(&url).document, None, None, app)
        })
        .unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|header| (header.name.as_str(), header.value.clone(), header.source))
            .collect::<Vec<_>>(),
        vec![
            ("User-Agent", USER_AGENT.to_string(), ImpliedSource::Default),
            ("Accept", "*/*".to_string(), ImpliedSource::Default),
            ("Host", format!("127.0.0.1:{port}"), ImpliedSource::Url),
        ]
    );
    for header in &listed {
        let line = format!("{}: {}\r\n", header.name, header.value).to_ascii_lowercase();
        assert!(left.head.to_ascii_lowercase().contains(&line), "{line}");
    }

    // Its collection names another, then the request its own over it.
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    hand_down(&api.id, vec![row("Equipe/1", true)]);
    let mut placed = request(&url);
    placed.collection_id = Some(api.id.clone());
    session
        .call(|app| send_http_request(placed.clone(), app))
        .unwrap();
    assert_eq!(agent(&seen.recv().unwrap()).as_deref(), Some("Equipe/1"));
    let listed = session
        .call(|app| {
            implied_http_headers(
                RequestKind::Http,
                placed.document.clone(),
                Some(api.id.clone()),
                None,
                app,
            )
        })
        .unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|header| (header.value.as_str(), header.source))
            .next(),
        Some(("Equipe/1", ImpliedSource::Above))
    );
    assert_eq!(
        listed[0].from.as_ref().map(|from| from.name.as_str()),
        Some("API")
    );

    let mut own = placed.clone();
    own.document.headers = vec![row("curl/8.5.0", true)];
    session.call(|app| send_http_request(own, app)).unwrap();
    assert_eq!(agent(&seen.recv().unwrap()).as_deref(), Some("curl/8.5.0"));

    // Switched off, in the request or above it: none at all.
    let mut silent = request(&url);
    silent.document.headers = vec![row("", false)];
    session.call(|app| send_http_request(silent, app)).unwrap();
    assert_eq!(agent(&seen.recv().unwrap()), None);

    hand_down(&api.id, vec![row("Equipe/1", false)]);
    session.call(|app| send_http_request(placed, app)).unwrap();
    assert_eq!(agent(&seen.recv().unwrap()), None);
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

/// The sentence states the value in force and who set it: it used to state the defaults,
/// « 30 secondes » for a request cut after one.
#[test]
fn a_timeout_and_a_redirect_too_many_say_the_value_in_force_and_who_set_it() {
    use devnotes_lib::http::http_history;

    let session = Session::open();
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    session
        .call(|app| {
            save_http_settings(
                HttpItem {
                    kind: HttpItemKind::Collection,
                    id: api.id.clone(),
                },
                ContainerSettings {
                    transport: TransportSettings {
                        timeout_ms: Some(300),
                        ..TransportSettings::default()
                    },
                    ..ContainerSettings::default()
                },
                app,
            )
        })
        .unwrap();
    let params = |failed: &devnotes_lib::error::AppError| {
        failed
            .params
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" ")
    };

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    // Accepts each connection and answers none of them until the test is over.
    let silent = thread::spawn(move || listener.incoming().take(2).collect::<Result<Vec<_>, _>>());
    let slow = format!("http://127.0.0.1:{port}/slow");

    let mut inherited = request(&slow);
    inherited.collection_id = Some(api.id.clone());
    let failed = session
        .call(|app| send_http_request(inherited, app))
        .unwrap_err();
    assert_eq!(failed.code, ErrorCode::HttpTimeout);
    assert_eq!(
        params(&failed),
        "amount=300 name=API source=collection unit=ms"
    );

    let mut own = request(&slow);
    own.collection_id = Some(api.id.clone());
    own.document.transport.timeout_ms = Some(1_000);
    let failed = session.call(|app| send_http_request(own, app)).unwrap_err();
    assert_eq!(failed.code, ErrorCode::HttpTimeout);
    assert_eq!(params(&failed), "amount=1 source=request unit=s");
    drop(silent);

    let (port, _seen) = serve(13, |_, port| {
        response(
            "302 Found",
            &[("Location", &format!("http://127.0.0.1:{port}/again"))],
            b"",
        )
    });
    let around = format!("http://127.0.0.1:{port}/around");
    let mut once = request(&around);
    once.document.transport.max_redirects = Some(1);
    let failed = session
        .call(|app| send_http_request(once, app))
        .unwrap_err();
    assert_eq!(failed.code, ErrorCode::HttpTooManyRedirects);
    assert_eq!(params(&failed), "count=1 source=request");

    let failed = session
        .call(|app| send_http_request(request(&around), app))
        .unwrap_err();
    assert_eq!(failed.code, ErrorCode::HttpTooManyRedirects);
    assert_eq!(params(&failed), "count=10 source=default");

    // The history says the same sentence: it keeps the values with the code.
    let days = session.call(|app| http_history(0, app)).unwrap();
    let last = &days[0].items[0].summary;
    assert_eq!(last.failure, Some(ErrorCode::HttpTooManyRedirects));
    assert_eq!(
        last.failure_params.get("count").map(String::as_str),
        Some("10")
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

/// « Vider l'historique » was the only way to remove the one send made with the wrong token,
/// and « Rouvrir » opened a stranger with no tie to the request the send came from.
#[test]
fn one_send_is_removed_and_put_back_and_goes_back_onto_the_request_it_came_from() {
    use devnotes_lib::http::model::HttpRequestDraft;
    use devnotes_lib::http::{
        create_http_request, delete_http_history_entry, delete_http_item, http_history,
        http_history_onto_request, restore_http_history_entry,
    };

    let session = Session::open();
    let (port, _seen) = serve(2, |_, _| response("200 OK", &[], b"ok"));
    let api = session
        .call(|app| create_http_collection("API".to_string(), app))
        .unwrap();
    let token = KeyValue {
        key: "Authorization".to_string(),
        value: "Bearer t0ken".to_string(),
        ..KeyValue::default()
    };
    let held = session
        .call(|app| {
            create_http_request(
                HttpRequestDraft {
                    collection_id: api.id.clone(),
                    folder_id: None,
                    name: "ma req".to_string(),
                    kind: RequestKind::Http,
                    method: HttpMethod::Get,
                    document: RequestDocument {
                        url: format!("http://127.0.0.1:{port}/now"),
                        headers: vec![token.clone()],
                        ..RequestDocument::default()
                    },
                },
                app,
            )
        })
        .unwrap();
    for path in ["first", "second?page=2"] {
        let mut sent = request(&format!("http://127.0.0.1:{port}/{path}"));
        sent.collection_id = Some(api.id.clone());
        sent.request_id = Some(held.id.clone());
        sent.document.headers = vec![token.clone()];
        session.call(|app| send_http_request(sent, app)).unwrap();
    }
    let listed = || {
        let days = session.call(|app| http_history(0, app)).unwrap();
        days.into_iter()
            .flat_map(|day| day.items)
            .collect::<Vec<_>>()
    };

    // Newest first, each URL in its two parts: the host, and what tells the two apart.
    let before = listed();
    assert_eq!(
        before
            .iter()
            .map(|item| (item.url_head.clone(), item.url_tail.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (format!("127.0.0.1:{port}"), "/second?page=2"),
            (format!("127.0.0.1:{port}"), "/first"),
        ]
    );

    let removed = session
        .call(|app| delete_http_history_entry(before[0].id.clone(), app))
        .unwrap();
    assert_eq!(listed(), before[1..]);
    session
        .call(|app| restore_http_history_entry(removed, app))
        .unwrap();
    assert_eq!(listed(), before);

    // Back onto its request: the send's URL, and the request's own secret where the record
    // holds a mask.
    let onto = session
        .call(|app| http_history_onto_request(before[1].id.clone(), app))
        .unwrap()
        .unwrap();
    assert_eq!(onto.name, "ma req");
    assert_eq!(onto.document.url, format!("http://127.0.0.1:{port}/first"));
    assert_eq!(onto.document.headers, vec![token]);

    // The request gone, there is nothing to go back onto: a draft it is.
    session
        .call(|app| {
            delete_http_item(
                HttpItem {
                    kind: HttpItemKind::Request,
                    id: held.id.clone(),
                },
                app,
            )
        })
        .unwrap();
    assert_eq!(
        session
            .call(|app| http_history_onto_request(before[1].id.clone(), app))
            .unwrap(),
        None
    );
}

#[test]
fn a_graphql_request_is_posted_as_json_and_its_errors_read_apart_from_its_data() {
    use devnotes_lib::http::graphql::GraphqlDocument;
    use devnotes_lib::http::graphql::QueryFault;
    use devnotes_lib::http::{add_graphql_variables, describe_graphql};

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
    assert_eq!(described.query_problem, None);

    // What the editor says before a send: a brace left open, a variable without a value.
    let open = GraphqlDocument {
        query: "query Country($code: ID!) { country(code: $code) { name }".to_string(),
        ..GraphqlDocument::default()
    };
    let described = session.call(|_| describe_graphql(open.clone())).unwrap();
    let problem = described.query_problem.unwrap();
    assert_eq!(
        (problem.fault, problem.line, problem.column),
        (QueryFault::UnclosedBrace, 1, 27)
    );
    assert_eq!(described.missing, ["code"]);
    assert_eq!(
        session.call(|_| add_graphql_variables(open)).unwrap(),
        "{
  \"code\": null
}"
    );
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
    // Two of one path length go back in the order they were set (#705).
    assert!(
        carried.contains("cookie: session=abc; theme=dark\r\n"),
        "{carried}"
    );
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

/// `/cookies/set?session=abc123` answers a 302 that sets the cookie and leads to a page that
/// echoes what it received: it received none, and the jar stayed empty. Nearly every login.
#[test]
fn a_cookie_set_on_a_redirect_is_sent_to_the_hop_that_follows_and_kept_in_the_jar() {
    use devnotes_lib::http::{clear_http_cookies, count_http_cookies, http_cookies};

    let session = Session::open();
    let (port, seen) = serve(4, |seen, _| {
        if seen.head.starts_with("GET /cookies/set ") {
            response(
                "302 Found",
                &[
                    ("Location", "/cookies"),
                    ("Set-Cookie", "session=abc123; Path=/"),
                ],
                b"",
            )
        } else {
            let sent = header_of(seen, "cookie").unwrap_or_default();
            response(
                "200 OK",
                &[
                    ("Content-Type", "text/plain"),
                    ("Set-Cookie", "seen=1; Path=/"),
                ],
                sent.as_bytes(),
            )
        }
    });
    let set = format!("http://127.0.0.1:{port}/cookies/set");
    let landing = format!("http://127.0.0.1:{port}/cookies");

    let answer = session
        .call(|app| send_http_request(request(&set), app))
        .unwrap();

    assert_eq!(answer.body, "session=abc123");
    assert_eq!(answer.url, landing);
    assert_eq!(header_of(&seen.recv().unwrap(), "cookie"), None);
    assert_eq!(
        header_of(&seen.recv().unwrap(), "cookie").as_deref(),
        Some("session=abc123")
    );
    // The answer lists what was set along the chain, the redirect's first, with who set it.
    assert_eq!(
        answer
            .cookies
            .iter()
            .map(|cookie| (cookie.name.as_str(), cookie.from.clone()))
            .collect::<Vec<_>>(),
        vec![("session", Some(set.clone())), ("seen", None)]
    );
    assert_eq!(answer.redirects.len(), 1);
    let redirect = &answer.redirects[0];
    assert_eq!(
        (
            redirect.status,
            redirect.from.as_str(),
            redirect.url.as_str()
        ),
        (302, set.as_str(), landing.as_str())
    );
    assert_eq!(redirect.cookies.len(), 1);
    assert_eq!(redirect.cookies[0].name, "session");

    let domains = session.call(http_cookies).unwrap();
    let mut kept: Vec<&str> = domains[0]
        .cookies
        .iter()
        .map(|stored| stored.cookie.name.as_str())
        .collect();
    kept.sort_unstable();
    assert_eq!(kept, ["seen", "session"]);

    // « Envoyer et garder les cookies : Non »: none sent at any hop, none kept.
    assert_eq!(session.call(clear_http_cookies).unwrap(), 2);
    let mut alone = request(&set);
    alone.document.transport.use_cookies = Some(false);
    let answer = session.call(|app| send_http_request(alone, app)).unwrap();
    assert_eq!(answer.body, "");
    assert_eq!(session.call(count_http_cookies).unwrap(), 0);
    // What the answers set is still read, as it always was.
    assert_eq!(answer.cookies.len(), 2);
}

#[test]
fn a_redirect_to_another_host_carries_neither_a_cookie_nor_the_authorization() {
    let row = |key: &str, value: &str| KeyValue {
        key: key.to_string(),
        value: value.to_string(),
        ..KeyValue::default()
    };
    let session = Session::open();
    let (elsewhere, landed) = serve(2, |_, _| response("200 OK", &[], b"there"));
    let (port, seen) = serve(2, move |_, _| {
        response(
            "302 Found",
            &[
                ("Location", &format!("http://localhost:{elsewhere}/landing")),
                ("Set-Cookie", "session=abc; Path=/"),
            ],
            b"",
        )
    });
    let login = format!("http://127.0.0.1:{port}/login");

    let mut sent = request(&login);
    sent.document.headers = vec![row("Authorization", "Bearer t0ken"), row("X-Trace", "1")];
    let answer = session.call(|app| send_http_request(sent, app)).unwrap();

    assert_eq!(answer.body, "there");
    assert_eq!(
        header_of(&seen.recv().unwrap(), "authorization").as_deref(),
        Some("Bearer t0ken")
    );
    let there = landed.recv().unwrap();
    assert_eq!(header_of(&there, "authorization"), None);
    assert_eq!(header_of(&there, "cookie"), None);
    // What is not a secret travels, and the page says where it was sent from.
    assert_eq!(header_of(&there, "x-trace").as_deref(), Some("1"));
    assert_eq!(header_of(&there, "referer"), Some(login.clone()));

    // A `Cookie` typed wins over the jar's for the host it is typed for, and stays there too.
    let mut typed = request(&login);
    typed.document.headers = vec![row("Cookie", "typed=1")];
    session.call(|app| send_http_request(typed, app)).unwrap();
    assert_eq!(
        header_of(&seen.recv().unwrap(), "cookie").as_deref(),
        Some("typed=1")
    );
    assert_eq!(header_of(&landed.recv().unwrap(), "cookie"), None);
}

#[test]
fn a_303_after_a_post_becomes_a_get_without_its_body_and_a_307_sends_it_again() {
    let session = Session::open();
    let (port, seen) = serve(4, |seen, _| {
        if seen.head.starts_with("POST /see-other ") {
            response("303 See Other", &[("Location", "done")], b"")
        } else if seen.head.starts_with("POST /temporary ") {
            response("307 Temporary Redirect", &[("Location", "/kept")], b"")
        } else {
            response("200 OK", &[], b"ok")
        }
    });
    let posting = |path: &str| {
        let mut sent = request(&format!("http://127.0.0.1:{port}/{path}"));
        sent.method = HttpMethod::Post;
        sent.document.body = RequestBody::Json {
            text: "{\"a\":1}".to_string(),
        };
        session.call(|app| send_http_request(sent, app)).unwrap()
    };

    let answer = posting("see-other");
    assert_eq!(answer.url, format!("http://127.0.0.1:{port}/done"));
    assert_eq!(seen.recv().unwrap().body, b"{\"a\":1}");
    let after = seen.recv().unwrap();
    assert!(after.head.starts_with("GET /done "), "{}", after.head);
    assert!(after.body.is_empty());
    assert_eq!(header_of(&after, "content-type"), None);

    posting("temporary");
    seen.recv().unwrap();
    let after = seen.recv().unwrap();
    assert!(after.head.starts_with("POST /kept "), "{}", after.head);
    assert_eq!(after.body, b"{\"a\":1}");
    assert_eq!(
        header_of(&after, "content-type").as_deref(),
        Some("application/json")
    );
}

#[test]
fn the_delay_is_the_whole_chain_s_and_a_cancel_ends_it_on_the_way() {
    let session = Session::open();
    // Each hop answers after 400 ms: within 600 ms a hop, not within 600 ms in all.
    let (port, _seen) = serve(2, |seen, _| {
        thread::sleep(Duration::from_millis(400));
        if seen.head.starts_with("GET /first ") {
            response("302 Found", &[("Location", "/second")], b"")
        } else {
            response("200 OK", &[], b"late")
        }
    });
    let mut slow = request(&format!("http://127.0.0.1:{port}/first"));
    slow.document.transport.timeout_ms = Some(600);
    assert_eq!(
        code(session.call(|app| send_http_request(slow, app))),
        ErrorCode::HttpTimeout
    );

    // Redirected at once, then never answered: cancelled once the first hop is behind.
    let (port, seen) = serve(2, |seen, _| {
        if seen.head.starts_with("GET /first ") {
            response("302 Found", &[("Location", "/second")], b"")
        } else {
            thread::sleep(Duration::from_secs(5));
            Vec::new()
        }
    });
    let app = session.0.handle().clone();
    let sending = tauri::async_runtime::spawn(send_http_request(
        request(&format!("http://127.0.0.1:{port}/first")),
        app.clone(),
    ));
    seen.recv().unwrap();
    while !tauri::async_runtime::block_on(cancel_http_send("tab-1".to_string(), app.clone()))
        .unwrap()
    {
        thread::sleep(Duration::from_millis(10));
    }
    let cancelled = tauri::async_runtime::block_on(sending).unwrap();
    assert_eq!(cancelled.unwrap_err().code, ErrorCode::HttpCancelled);
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
