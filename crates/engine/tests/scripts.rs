//! Scripts run where the application runs them: in the executable started again as a script
//! host, its sends made by this process against a server on the loopback.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use engine::http::scripts::model::{
    FailureKind, Job, LayerScope, Phase, Script, ScriptKind, ScriptLayer, ScriptRequest,
    ScriptResponse,
};
use engine::http::scripts::network::Network;
use engine::http::scripts::process::ScriptProcess;
use engine::http::transport::Transport;

fn host() -> ScriptProcess {
    ScriptProcess::start_at(Path::new(env!("CARGO_BIN_EXE_script-host"))).unwrap()
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap()
}

/// Answers every connection with a login: a JSON token and a cookie.
fn serve() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buffer = [0; 8192];
            let _ = stream.read(&mut buffer);
            let body = "{\"access_token\":\"eyJ.secret.token\"}";
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nSet-Cookie: sid=42; Path=/\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    port
}

fn job(phase: Phase, source: &str) -> Job {
    Job {
        phase,
        scripts: vec![Script {
            id: "request:script".to_string(),
            level: engine::http::scripts::model::ScriptLevel::Request,
            kind: if phase == Phase::Before {
                ScriptKind::Before
            } else {
                ScriptKind::Tests
            },
            name: "Login".to_string(),
            owner: None,
            source: source.to_string(),
        }],
        request: ScriptRequest {
            method: "POST".to_string(),
            url: "{{baseUrl}}/login".to_string(),
            ..ScriptRequest::default()
        },
        response: (phase == Phase::After).then(|| ScriptResponse {
            code: 200,
            reason: "OK".to_string(),
            body: "{}".to_string(),
            ..ScriptResponse::default()
        }),
        layers: vec![ScriptLayer {
            scope: LayerScope::Environment,
            from: "Staging".to_string(),
            values: [("baseUrl".to_string(), "http://127.0.0.1".to_string())].into(),
        }],
        environment: Some("Staging".to_string()),
        ..Job::default()
    }
}

#[test]
fn a_script_runs_in_the_host_and_its_send_is_made_by_rust_with_the_jar() {
    let port = serve();
    let runtime = runtime();
    let mut network = Network::new(
        runtime.handle().clone(),
        Transport::default(),
        Some(Vec::new()),
        true,
    );
    let mut host = host();
    let source = format!(
        "const login = await pm.sendRequest('http://127.0.0.1:{port}/login');
         env.secret('accessToken', login.json().access_token);
         console.log('cookie', login.cookies.get('sid'));
         pm.cookies.jar().getAll('http://127.0.0.1:{port}/', (error, all) => env.set('cookies', all.length));
         pm.variables.set('started', 'yes');"
    );
    let outcome = host.run(
        &job(Phase::Before, &source),
        Duration::from_secs(10),
        &mut network,
    );

    assert_eq!(outcome.failure, None);
    let written: Vec<(&str, Option<&str>, bool)> = outcome
        .writes
        .iter()
        .map(|write| (write.name.as_str(), write.value.as_deref(), write.secret))
        .collect();
    assert_eq!(
        written,
        [
            ("accessToken", Some("eyJ.secret.token"), true),
            ("cookies", Some("1"), false)
        ]
    );
    assert_eq!(network.sent.len(), 1);
    assert_eq!(network.jar.as_ref().map(Vec::len), Some(1));
    assert_eq!(network.changes.len(), 1);
    assert_eq!(network.logs[0].text, "cookie 42");

    // The same host, once the request is answered: what the first phase set is still there.
    let outcome = host.run(
        &job(
            Phase::After,
            "test('kept', () => expect(pm.variables.get('started')).toBe('yes'));",
        ),
        Duration::from_secs(10),
        &mut network,
    );
    assert!(outcome.tests[0].passed, "{:?}", outcome.tests);
}

#[test]
fn a_protected_environment_lets_a_script_read_but_not_change_unless_confirmed() {
    let port = serve();
    let runtime = runtime();
    let mut network = Network::new(runtime.handle().clone(), Transport::default(), None, false);
    let source = format!(
        "await pm.sendRequest('http://127.0.0.1:{port}/read');
         await pm.sendRequest({{ url: 'http://127.0.0.1:{port}/pay', method: 'POST' }});"
    );
    let outcome = host().run(
        &job(Phase::Before, &source),
        Duration::from_secs(10),
        &mut network,
    );
    let failure = outcome.failure.unwrap();
    assert_eq!(failure.kind, FailureKind::Error);
    assert_eq!(failure.line, Some(2));
    assert!(failure.message.contains("protected"), "{}", failure.message);
    assert_eq!(network.sent.len(), 1, "the GET left, the POST did not");
}

#[test]
fn an_endless_loop_is_stopped_at_its_deadline_and_the_host_is_not_given_another_phase() {
    let runtime = runtime();
    let mut network = Network::new(runtime.handle().clone(), Transport::default(), None, true);
    let mut host = host();
    let started = Instant::now();
    let outcome = host.run(
        &job(Phase::Before, "while (true) {}"),
        Duration::from_millis(500),
        &mut network,
    );
    assert_eq!(outcome.failure.unwrap().kind, FailureKind::Timeout);
    assert!(started.elapsed() < Duration::from_secs(4));

    let outcome = host.run(&job(Phase::After, ""), Duration::from_secs(5), &mut network);
    assert_eq!(outcome.failure.unwrap().kind, FailureKind::Host);
}

/// Answers every connection with a 200, and keeps each request whole, headers and body.
fn serve_keeping() -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let kept = Arc::new(Mutex::new(Vec::new()));
    let into = Arc::clone(&kept);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 8192];
            while let Ok(read) = stream.read(&mut buffer) {
                request.extend_from_slice(&buffer[..read]);
                if read == 0 || whole(&request) {
                    break;
                }
            }
            into.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&request).into_owned());
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
            );
        }
    });
    (port, kept)
}

/// The headers ended, and as many bytes after them as they announced.
fn whole(request: &[u8]) -> bool {
    let text = String::from_utf8_lossy(request);
    let Some(end) = text.find("\r\n\r\n") else {
        return false;
    };
    let length = text[..end]
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    request.len() >= end + 4 + length
}

/// A file outside any profile, standing for a private key.
fn secret_file() -> (tempfile::TempDir, String) {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("id_ed25519");
    std::fs::write(&path, "PRIVATE KEY 7f3a9c").unwrap();
    (folder, path.to_string_lossy().into_owned())
}

#[test]
fn a_script_sending_a_file_by_itself_waits_for_a_yes() {
    let (port, kept) = serve_keeping();
    let (_folder, path) = secret_file();
    let runtime = runtime();
    let mut network = Network::new(runtime.handle().clone(), Transport::default(), None, true);
    let source = format!(
        "await pm.sendRequest({{ url: 'http://127.0.0.1:{port}/upload', method: 'POST',
           body: {{ mode: 'file', file: {{ src: {} }} }} }});",
        serde_json::to_string(&path).unwrap()
    );

    let refused = host().run(
        &job(Phase::Before, &source),
        Duration::from_secs(10),
        &mut network,
    );

    assert!(kept.lock().unwrap().is_empty());
    assert_eq!(refused.failure.unwrap().kind, FailureKind::Error);
    assert_eq!(network.refused_file.as_deref(), Some(path.as_str()));

    network.files.push(path);
    let sent = host().run(
        &job(Phase::Before, &source),
        Duration::from_secs(10),
        &mut network,
    );
    assert_eq!(sent.failure, None);
    assert!(kept.lock().unwrap()[0].ends_with("PRIVATE KEY 7f3a9c"));
}

#[test]
fn a_script_giving_the_request_a_file_body_waits_for_a_yes() {
    let (port, kept) = serve_keeping();
    let (_folder, path) = secret_file();
    let profile = tempfile::tempdir().unwrap();
    engine::http::scripts::process::host_from(env!("CARGO_BIN_EXE_script-host").into());
    let runtime = runtime();
    let paths = engine::Paths {
        profile: profile.path().to_path_buf(),
        resources: profile.path().to_path_buf(),
    };
    let sink = Arc::new(engine::RecordingSink::default());
    let core = Arc::new(engine::Engine::new(paths, sink, runtime.handle().clone()));
    engine::db::run_here(&core, |core| {
        let phrase = engine::vault::secret("correct horse battery staple".to_owned());
        engine::vault::create_vault(core, phrase)
    })
    .unwrap();
    let mut document = engine::http::model::RequestDocument {
        url: format!("http://127.0.0.1:{port}/upload"),
        ..Default::default()
    };
    document.scripts.before = format!(
        "pm.request.body.update({{ mode: 'file', file: {{ src: {} }} }});",
        serde_json::to_string(&path).unwrap()
    );
    let request = |files: Vec<String>| engine::http::send::SendRequest {
        id: "send-1".to_owned(),
        kind: engine::http::model::RequestKind::Http,
        method: engine::http::model::HttpMethod::Post,
        document: document.clone(),
        collection_id: None,
        folder_id: None,
        name: "Upload".to_owned(),
        request_id: None,
        environment_id: None,
        confirmed: false,
        files,
    };
    let send = |files| {
        runtime.block_on(engine::http::send_http_request(
            Arc::clone(&core),
            request(files),
        ))
    };

    let refused = send(Vec::new()).unwrap_err();
    assert!(kept.lock().unwrap().is_empty());
    assert_eq!(refused.code, engine::error::ErrorCode::HttpScriptFile);
    assert_eq!(refused.params["path"], path);

    send(vec![path]).unwrap();
    assert!(kept.lock().unwrap()[0].ends_with("PRIVATE KEY 7f3a9c"));
}

#[test]
fn a_host_that_cannot_start_is_said_as_such() {
    assert!(ScriptProcess::start_at(Path::new("this/does/not/exist")).is_err());
}
