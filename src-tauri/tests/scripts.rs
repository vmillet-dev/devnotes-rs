//! Scripts run where the application runs them: in the executable started again as a script
//! host, its sends made by this process against a server on the loopback.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use devnotes_lib::http::scripts::model::{
    FailureKind, Job, LayerScope, Phase, Script, ScriptKind, ScriptLayer, ScriptRequest,
    ScriptResponse,
};
use devnotes_lib::http::scripts::network::Network;
use devnotes_lib::http::scripts::process::ScriptProcess;
use devnotes_lib::http::transport::Transport;

fn host() -> ScriptProcess {
    ScriptProcess::start_at(Path::new(env!("CARGO_BIN_EXE_devnotes"))).unwrap()
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
            level: devnotes_lib::http::scripts::model::ScriptLevel::Request,
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

#[test]
fn a_host_that_cannot_start_is_said_as_such() {
    assert!(ScriptProcess::start_at(Path::new("this/does/not/exist")).is_err());
}
