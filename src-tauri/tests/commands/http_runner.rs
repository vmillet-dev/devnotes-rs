//! A collection run through its command, against a server on the loopback: its requests in
//! order over the rows of a data file, its steps said as events, nothing in the history.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use devnotes_lib::http::model::{
    ContainerSettings, HttpItem, HttpItemKind, HttpMethod, HttpRequestDraft, RequestDocument,
    RequestKind, RequestScripts, RunEntry,
};
use devnotes_lib::http::runner::{HTTP_RUN_EVENT, RunData, RunEvent, RunRequest, RunStep};
use devnotes_lib::http::scripts::process::host_from;
use devnotes_lib::http::{
    cancel_http_run, create_http_collection, create_http_environment, create_http_request,
    http_environments, http_history, http_run_order, read_http_run_data, run_http_collection,
    save_http_run_order, save_http_settings,
};
use tauri::Listener;

use super::Session;
use super::http_send::{response, serve};

fn hosted() -> Session {
    host_from(PathBuf::from(env!("CARGO_BIN_EXE_devnotes")));
    Session::open()
}

fn steps(session: &Session) -> Arc<Mutex<Vec<RunStep>>> {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&heard);
    session.0.listen_any(HTTP_RUN_EVENT, move |event| {
        if let Ok(event) = serde_json::from_str::<RunEvent>(event.payload()) {
            kept.lock().unwrap().push(event.step);
        }
    });
    heard
}

/// A collection of the requests given, `(name, path, scripts)`: its id and theirs.
fn collection(
    session: &Session,
    port: u16,
    requests: &[(&str, &str, RequestScripts)],
) -> (String, Vec<String>) {
    let api = session
        .call(|app| create_http_collection("Run".to_string(), app))
        .unwrap()
        .id;
    let ids = requests
        .iter()
        .map(|(name, path, scripts)| {
            let draft = HttpRequestDraft {
                collection_id: api.clone(),
                folder_id: None,
                name: (*name).to_string(),
                kind: RequestKind::Http,
                method: HttpMethod::Get,
                document: RequestDocument {
                    url: format!("http://127.0.0.1:{port}{path}"),
                    scripts: scripts.clone(),
                    ..RequestDocument::default()
                },
            };
            session
                .call(|app| create_http_request(draft, app))
                .unwrap()
                .id
        })
        .collect();
    (api, ids)
}

fn run(id: &str, api: &str, order: Vec<String>) -> RunRequest {
    RunRequest {
        id: id.to_string(),
        item: HttpItem {
            kind: HttpItemKind::Collection,
            id: api.to_string(),
        },
        order,
        iterations: 1,
        delay_ms: 0,
        data: None,
        stop_on_failure: false,
        keep_variables: true,
        keep_responses: false,
        environment_id: None,
        confirmed: false,
    }
}

fn ended(steps: &Arc<Mutex<Vec<RunStep>>>) -> Vec<(u32, String, bool)> {
    steps
        .lock()
        .unwrap()
        .iter()
        .filter_map(|step| match step {
            RunStep::Ended { result } => {
                Some((result.iteration, result.name.clone(), result.failed()))
            }
            _ => None,
        })
        .collect()
}

fn ok() -> Vec<u8> {
    response("200 OK", &[("Content-Type", "application/json")], b"{}")
}

#[test]
fn a_run_sends_its_requests_over_the_rows_of_its_data_and_writes_no_history() {
    let session = hosted();
    let staging = session
        .call(|app| create_http_environment("Staging".to_string(), app))
        .unwrap()
        .id;
    let (port, seen) = serve(4, |_, _| ok());
    let (api, ids) = collection(
        &session,
        port,
        &[
            (
                "Login",
                "/login",
                RequestScripts {
                    before: "env.set('seen', pm.iterationData.get('customer'));".to_string(),
                    tests: "test('ok', () => expect(res.status).toBe(200));".to_string(),
                    ..RequestScripts::default()
                },
            ),
            (
                "Client",
                "/c/{{customer}}?i={{seen}}",
                RequestScripts::default(),
            ),
        ],
    );
    let heard = steps(&session);
    let mut asked = run("run-1", &api, ids);
    asked.data = Some(RunData {
        columns: vec!["customer".to_string()],
        rows: vec![
            [("customer".to_string(), "cus_1".to_string())].into(),
            [("customer".to_string(), "cus_2".to_string())].into(),
        ],
    });
    asked.keep_variables = false;
    asked.environment_id = Some(staging.clone());

    session.call(|app| run_http_collection(asked, app)).unwrap();

    let paths: Vec<String> = (0..4)
        .map(|_| {
            let read = seen.recv_timeout(Duration::from_secs(5)).unwrap().head;
            read.lines().next().unwrap().to_string()
        })
        .collect();
    assert_eq!(
        paths,
        [
            "GET /login HTTP/1.1",
            "GET /c/cus_1?i=cus_1 HTTP/1.1",
            "GET /login HTTP/1.1",
            "GET /c/cus_2?i=cus_2 HTTP/1.1",
        ]
    );
    assert_eq!(
        ended(&heard),
        [
            (0, "Login".to_string(), false),
            (0, "Client".to_string(), false),
            (1, "Login".to_string(), false),
            (1, "Client".to_string(), false),
        ]
    );
    assert!(matches!(
        heard.lock().unwrap().last(),
        Some(RunStep::Finished {
            cancelled: false,
            ..
        })
    ));
    // A run keeps its report, not the history; and what it wrote was for it alone.
    assert!(session.call(|app| http_history(0, app)).unwrap().is_empty());
    let sets = session.call(http_environments).unwrap();
    assert!(sets.environments[0].variables.is_empty());
}

#[test]
fn a_run_stops_at_its_first_failure_when_asked() {
    let session = hosted();
    let (port, _seen) = serve(2, |_, _| ok());
    let failing = RequestScripts {
        tests: "test('never', () => expect(res.status).toBe(500));".to_string(),
        ..RequestScripts::default()
    };
    let (api, ids) = collection(
        &session,
        port,
        &[("A", "/a", failing), ("B", "/b", RequestScripts::default())],
    );
    let heard = steps(&session);
    let mut asked = run("run-2", &api, ids);
    asked.stop_on_failure = true;
    asked.keep_responses = true;

    session.call(|app| run_http_collection(asked, app)).unwrap();

    assert_eq!(ended(&heard), [(0, "A".to_string(), true)]);
    let kept = heard.lock().unwrap().iter().any(
        |step| matches!(step, RunStep::Ended { result } if result.response.as_ref().is_some_and(|response| response.status == 200)),
    );
    assert!(kept, "the answer kept with its result");
}

#[test]
fn a_script_names_the_request_a_run_goes_to_and_null_ends_it() {
    let session = hosted();
    let (port, _seen) = serve(3, |_, _| ok());
    let to_c = RequestScripts {
        after: "pm.execution.setNextRequest('C');".to_string(),
        ..RequestScripts::default()
    };
    let end = RequestScripts {
        after: "pm.execution.setNextRequest(null);".to_string(),
        ..RequestScripts::default()
    };
    let (api, ids) = collection(
        &session,
        port,
        &[
            ("A", "/a", to_c),
            ("B", "/b", RequestScripts::default()),
            ("C", "/c", end),
            ("D", "/d", RequestScripts::default()),
        ],
    );
    let heard = steps(&session);

    session
        .call(|app| run_http_collection(run("run-3", &api, ids), app))
        .unwrap();

    let names: Vec<String> = ended(&heard).into_iter().map(|(_, name, _)| name).collect();
    assert_eq!(names, ["A", "C"]);
}

#[test]
fn a_run_is_cancelled_between_two_requests() {
    let session = hosted();
    let (port, _seen) = serve(2, |_, _| ok());
    let (api, ids) = collection(
        &session,
        port,
        &[
            ("A", "/a", RequestScripts::default()),
            ("B", "/b", RequestScripts::default()),
        ],
    );
    let heard = steps(&session);
    let mut asked = run("run-4", &api, ids);
    asked.delay_ms = 20_000;
    let app = session.0.handle().clone();
    let running =
        std::thread::spawn(move || tauri::async_runtime::block_on(run_http_collection(asked, app)));
    let waited = std::time::Instant::now();
    while ended(&heard).is_empty() {
        assert!(
            waited.elapsed() < Duration::from_secs(10),
            "the first request never ended"
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(
        session
            .call(|app| cancel_http_run("run-4".to_string(), app))
            .unwrap()
    );
    running.join().unwrap().unwrap();

    assert_eq!(ended(&heard).len(), 1);
    assert!(matches!(
        heard.lock().unwrap().last(),
        Some(RunStep::Finished {
            cancelled: true,
            ..
        })
    ));
    assert!(
        !session
            .call(|app| cancel_http_run("run-4".to_string(), app))
            .unwrap()
    );
}

#[test]
fn a_data_file_is_read_as_a_csv_or_says_why_not() {
    let directory = tempfile::tempdir().unwrap();
    let csv = directory.path().join("clients.csv");
    std::fs::write(&csv, "customer,amount\ncus_1,12\ncus_2,8\n").unwrap();
    let data =
        tauri::async_runtime::block_on(read_http_run_data(csv.display().to_string())).unwrap();
    assert_eq!(data.columns, ["customer", "amount"]);
    assert_eq!(data.rows.len(), 2);

    let empty = directory.path().join("empty.csv");
    std::fs::write(&empty, "customer\n").unwrap();
    let error = tauri::async_runtime::block_on(read_http_run_data(empty.display().to_string()))
        .unwrap_err();
    assert_eq!(error.params["fault"], "empty");
    assert!(
        tauri::async_runtime::block_on(read_http_run_data(
            directory.path().join("nowhere.csv").display().to_string()
        ))
        .is_err()
    );
}

#[test]
fn the_order_arranged_is_kept_and_a_request_added_goes_at_its_end() {
    let session = Session::open();
    let (api, ids) = collection(
        &session,
        9,
        &[
            ("A", "/a", RequestScripts::default()),
            ("B", "/b", RequestScripts::default()),
        ],
    );
    let item = || HttpItem {
        kind: HttpItemKind::Collection,
        id: api.clone(),
    };
    let entry = |id: &str, enabled: bool| RunEntry {
        id: id.to_string(),
        enabled,
    };
    session
        .call(|app| {
            save_http_run_order(
                item(),
                vec![entry(&ids[1], true), entry(&ids[0], false)],
                app,
            )
        })
        .unwrap();
    let added = session
        .call(|app| {
            create_http_request(
                HttpRequestDraft {
                    collection_id: api.clone(),
                    folder_id: None,
                    name: "C".to_string(),
                    kind: RequestKind::Http,
                    method: HttpMethod::Get,
                    document: RequestDocument::default(),
                },
                app,
            )
        })
        .unwrap()
        .id;
    // The settings dialog saves what it holds, which has no run order.
    session
        .call(|app| save_http_settings(item(), ContainerSettings::default(), app))
        .unwrap();

    assert_eq!(
        session.call(|app| http_run_order(item(), app)).unwrap(),
        [
            entry(&ids[1], true),
            entry(&ids[0], false),
            entry(&added, true)
        ]
    );
}
