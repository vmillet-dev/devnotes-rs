//! The runner's page over a real core; its report is fed by a fake source of events, the bench
//! having no channel from the core.

use std::sync::Arc;

use chrono::Utc;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::console::{ConsoleEvent, ConsoleLine};
use engine::http::model::{HttpMethod, HttpRequestDraft, RequestDocument, RequestKind};
use engine::http::runner::{RunEvent, RunResult, RunStep, RunSummary};
use engine::http::scripts::model::{LogLevel, TestResult};
use engine::http::send::SentResponse;

use super::tests::{choose, click_id, collection, said_by, sending_to};
use crate::notes::bench::{Bench, area_of, settle, until};
use crate::state::Area;
use crate::state::http::runner::Phase;
use crate::state::http::send::Phase as SendPhase;
use crate::testing;

const TALL: egui::Vec2 = egui::vec2(1_100.0, 900.0);

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

fn count(harness: &Harness<'_, Bench>, author_id: &str) -> usize {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .count()
}

fn labels(harness: &Harness<'_, Bench>, author_id: &str) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .map(|node| {
            let node = node.accesskit_node();
            node.label().or(node.value()).unwrap_or_default()
        })
        .collect()
}

/// Two requests to the local server at `path`, the runner open on their collection.
fn runner(path: &str) -> Harness<'static, Bench> {
    runner_on(testing::serve(), path)
}

fn runner_on(port: u16, path: &str) -> Harness<'static, Bench> {
    let (backend, profile) = testing::unlocked();
    let local = collection(&backend, "Local");
    for name in ["Login", "Orders"] {
        let draft = HttpRequestDraft {
            collection_id: local.id.clone(),
            folder_id: None,
            name: name.to_owned(),
            kind: RequestKind::Http,
            method: HttpMethod::Post,
            document: RequestDocument {
                url: format!("http://127.0.0.1:{port}{path}"),
                ..RequestDocument::default()
            },
        };
        engine::db::run_here(backend.engine(), move |engine| {
            engine::http::create_http_request(engine, draft)
        })
        .unwrap();
    }
    let mut harness = area_of(Area::Http, backend, profile);
    harness.set_size(TALL);
    settle(&mut harness);
    open(&mut harness);
    harness
}

fn open(harness: &mut Harness<'_, Bench>) {
    choose(harness, "Local", &["http-node-run"]);
    until(harness, "the order is read", |harness| {
        count(harness, "http-runner-row") == 2
    });
}

fn phase(harness: &Harness<'_, Bench>) -> Phase {
    harness.state().state.http.runner().phase()
}

fn result(iteration: u32, name: &str, tests: Vec<TestResult>) -> RunResult {
    RunResult {
        iteration,
        request_id: name.to_owned(),
        name: name.to_owned(),
        path: vec!["Auth".to_owned()],
        kind: RequestKind::Http,
        method: HttpMethod::Post,
        status: Some(200),
        reason: "OK".to_owned(),
        millis: 12,
        tests,
        error: None,
        stop: None,
        skipped: false,
        response: None,
    }
}

fn passed(name: &str) -> TestResult {
    TestResult {
        name: name.to_owned(),
        passed: true,
        ..TestResult::default()
    }
}

fn failed(name: &str) -> TestResult {
    TestResult {
        name: name.to_owned(),
        expected: Some("200".to_owned()),
        actual: Some("404".to_owned()),
        code: Some("expect(res.status).toBe(200);".to_owned()),
        ..TestResult::default()
    }
}

fn say(harness: &mut Harness<'_, Bench>, step: RunStep) {
    let event = RunEvent {
        id: "1".to_owned(),
        step,
    };
    harness.state_mut().state.http.receive_run(event);
    settle(harness);
}

fn ended(harness: &mut Harness<'_, Bench>, result: RunResult) {
    let result = Box::new(result);
    say(harness, RunStep::Ended { result });
}

fn run(harness: &mut Harness<'_, Bench>) {
    click_id(harness, "http-runner-start-empty");
    until(harness, "the run is over", |harness| {
        phase(harness) == Phase::Done
    });
}

#[test]
fn a_collection_opens_on_its_requests_in_their_order() {
    let harness = runner("/json");
    assert_eq!(said_by(&harness, "http-runner").as_deref(), Some("Local"));
    assert_eq!(labels(&harness, "http-runner-row"), ["Login", "Orders"]);
    let ready = said_by(&harness, "http-runner-ready").unwrap();
    assert!(ready.starts_with("2 requests · 1 iteration"), "{ready}");
}

#[test]
fn a_row_moves_by_the_keys_and_the_order_is_kept_with_the_collection() {
    let mut harness = runner("/json");
    harness
        .get_by(|node| {
            node.author_id() == Some("http-runner-row") && node.label().as_deref() == Some("Login")
        })
        .focus();
    settle(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowDown);
    settle(&mut harness);
    assert_eq!(labels(&harness, "http-runner-row"), ["Orders", "Login"]);

    harness.key_press(egui::Key::Space);
    settle(&mut harness);
    assert_eq!(harness.state().state.http.runner().enabled_count(), 1);

    click_id(&mut harness, "http-history-open");
    open(&mut harness);
    assert_eq!(labels(&harness, "http-runner-row"), ["Orders", "Login"]);
    assert_eq!(harness.state().state.http.runner().enabled_count(), 1);
}

#[test]
fn a_request_left_out_is_not_counted_and_none_left_cannot_start() {
    let mut harness = runner("/json");
    let boxes = |harness: &Harness<'_, Bench>, at: usize| {
        harness
            .query_all_by(|node| node.author_id() == Some("http-runner-row-enabled"))
            .nth(at)
            .unwrap()
            .click();
    };
    boxes(&harness, 0);
    settle(&mut harness);
    boxes(&harness, 1);
    settle(&mut harness);
    assert_eq!(harness.state().state.http.runner().enabled_count(), 0);
    click_id(&mut harness, "http-runner-start");
    assert_eq!(phase(&harness), Phase::Idle);
}

#[test]
fn the_iterations_typed_are_what_the_run_says() {
    let mut harness = runner("/json");
    let field = harness.get_by(|node| node.author_id() == Some("http-runner-iterations"));
    field.focus();
    settle(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness
        .get_by(|node| node.author_id() == Some("http-runner-iterations"))
        .type_text("3");
    settle(&mut harness);
    let ready = said_by(&harness, "http-runner-ready").unwrap();
    assert!(ready.contains("3 iterations"), "{ready}");
}

#[test]
fn a_data_file_gives_an_iteration_a_row_until_it_is_taken_away() {
    let mut harness = runner("/json");
    let directory = tempfile::tempdir().unwrap();
    let csv = directory.path().join("clients.csv");
    std::fs::write(&csv, "customer\ncus_1\ncus_2\n").unwrap();
    harness.state_mut().state.dialog = Arc::new(platform::dialog::Fixed(csv));
    click_id(&mut harness, "http-runner-data-choose");
    until(&mut harness, "the file is read", |harness| {
        has(harness, "http-runner-data")
    });
    assert_eq!(harness.state().state.http.runner().runs(), 2);

    click_id(&mut harness, "http-runner-data-clear");
    assert!(has(&harness, "http-runner-data-choose"));
    assert_eq!(harness.state().state.http.runner().runs(), 1);
}

#[test]
fn a_file_that_is_no_data_is_said_and_nothing_is_chosen() {
    let mut harness = runner("/json");
    let directory = tempfile::tempdir().unwrap();
    let prose = directory.path().join("prose.txt");
    std::fs::write(&prose, "neither a CSV with a header nor a JSON array\n").unwrap();
    harness.state_mut().state.dialog = Arc::new(platform::dialog::Fixed(prose));
    click_id(&mut harness, "http-runner-data-choose");
    until(&mut harness, "the error is said", |harness| {
        harness.state().state.errors.notice().is_some()
    });
    assert!(harness.state().state.http.runner().data().is_none());
}

#[test]
fn the_report_sums_the_run_up_and_a_failure_opens_on_its_assertion() {
    let mut harness = runner("/json");
    run(&mut harness);
    ended(&mut harness, result(0, "Login", vec![passed("found")]));
    ended(&mut harness, result(0, "Orders", vec![failed("listed")]));
    say(
        &mut harness,
        RunStep::Finished {
            cancelled: false,
            millis: 1_200,
        },
    );

    assert!(has(&harness, "http-runner-failures"));
    let figures = said_by(&harness, "http-runner-figures").unwrap();
    assert!(figures.starts_with("2 requests · 1/2 tests"), "{figures}");
    assert_eq!(count(&harness, "http-runner-result-passed"), 1);
    assert_eq!(count(&harness, "http-runner-result-failed"), 1);
    let tests = labels(&harness, "http-runner-test");
    assert_eq!(tests.len(), 1);
    assert!(tests[0].starts_with("listed · "), "{tests:?}");

    click_id(&mut harness, "http-runner-filter-failed");
    assert_eq!(count(&harness, "http-runner-result-passed"), 0);
    click_id(&mut harness, "http-runner-filter-all");
    click_id(&mut harness, "http-runner-result-failed");
    assert_eq!(count(&harness, "http-runner-test"), 0, "folded");
    click_id(&mut harness, "http-runner-result-passed");
    assert_eq!(labels(&harness, "http-runner-test"), ["found"]);
}

#[test]
fn what_kept_a_request_from_its_answer_is_said_under_it() {
    let mut harness = runner("/json");
    run(&mut harness);
    let refused = RunResult {
        status: None,
        error: Some(engine::http::runner::RunError {
            code: engine::error::ErrorCode::HttpCancelled,
            params: std::collections::BTreeMap::new(),
        }),
        ..result(0, "Login", Vec::new())
    };
    ended(&mut harness, refused);
    assert!(has(&harness, "http-runner-why"));
    let skipped = RunResult {
        skipped: true,
        status: None,
        ..result(0, "Orders", Vec::new())
    };
    ended(&mut harness, skipped);
    assert!(has(&harness, "http-runner-result-skipped"));
}

#[test]
fn the_iterations_of_a_report_stand_apart() {
    let mut harness = runner("/json");
    let bench = harness.state_mut();
    let dialog = Arc::clone(&bench.state.dialog);
    let asked = crate::state::http::runner::Asked::Iterations(2);
    bench.state.http.ask_runner(&bench.backend, &dialog, asked);
    settle(&mut harness);
    run(&mut harness);
    ended(&mut harness, result(0, "Login", Vec::new()));
    ended(&mut harness, result(1, "Login", Vec::new()));
    assert_eq!(
        labels(&harness, "http-runner-iteration"),
        ["Iteration 1 / 2", "Iteration 2 / 2"]
    );
}

#[test]
fn a_run_cancelled_says_how_far_it_went() {
    let port = testing::serve();
    let mut harness = runner_on(port, "/slow");
    // Not settled: settling waits for the core, and the run with it.
    harness
        .get_by(|node| node.author_id() == Some("http-runner-start-empty"))
        .click();
    harness.step();
    harness.step();
    assert_eq!(phase(&harness), Phase::Running);
    harness.state_mut().state.http.receive_run(RunEvent {
        id: "1".to_owned(),
        step: RunStep::Started {
            iteration: 0,
            index: 0,
            name: "Login".to_owned(),
        },
    });
    harness.step();
    harness.step();
    assert!(has(&harness, "http-runner-progress"));
    for _ in 0..10_000 {
        if testing::slow_arrived(port) {
            break;
        }
        harness.step();
    }
    assert!(testing::slow_arrived(port), "the send reached the server");
    harness
        .get_by(|node| node.author_id() == Some("http-runner-cancel"))
        .click();
    harness.step();
    harness.step();
    until(&mut harness, "the run stops", |harness| {
        phase(harness) == Phase::Done
    });
    ended(&mut harness, result(0, "Login", Vec::new()));
    say(
        &mut harness,
        RunStep::Finished {
            cancelled: true,
            millis: 40,
        },
    );
    let said = said_by(&harness, "http-runner-cancelled").unwrap();
    assert_eq!(said, "Cancelled · 1 of 2 requests");
}

#[test]
fn a_kept_answer_reads_under_the_report() {
    let mut harness = runner("/json");
    run(&mut harness);
    let mut kept = sending_to("/json");
    click_id(&mut kept, "http-send");
    super::tests::answered(&mut kept);
    let http = &kept.state().state.http;
    let key = &http.tabs().active().unwrap().key;
    let SendPhase::Answered(response) = &http.sends().of(key).unwrap().phase else {
        panic!("the send is answered");
    };
    let response: SentResponse = response.as_ref().clone();
    let with_answer = RunResult {
        response: Some(response),
        ..result(0, "Login", Vec::new())
    };
    ended(&mut harness, with_answer);
    click_id(&mut harness, "http-runner-result-passed");
    assert!(has(&harness, "http-response-status"));
    click_id(&mut harness, "http-runner-result-passed");
    assert!(!has(&harness, "http-response-status"));
}

#[test]
fn a_protected_environment_asks_once_before_the_run() {
    let mut harness = runner("/json");
    click_id(&mut harness, "http-environments-open");
    super::environments_tests::create(&mut harness, "Production");
    click_id(&mut harness, "http-environment-use");
    click_id(&mut harness, "http-environment-protected");
    until(&mut harness, "it is protected", |harness| {
        let environments = harness.state().state.http.environments();
        environments.active().is_some_and(|active| active.protected)
    });
    open(&mut harness);

    click_id(&mut harness, "http-runner-start");
    assert_eq!(phase(&harness), Phase::Asking);
    let question = said_by(&harness, "http-runner-protected").unwrap();
    assert!(question.contains("POST Login, POST Orders"), "{question}");

    click_id(&mut harness, "http-runner-protected-cancel");
    assert_eq!(phase(&harness), Phase::Idle);

    click_id(&mut harness, "http-runner-start");
    click_id(&mut harness, "http-runner-protected-go");
    until(&mut harness, "the run is over", |harness| {
        phase(harness) == Phase::Done
    });
}

#[test]
fn the_console_of_a_run_hears_its_sends_and_the_next_run_forgets_it() {
    let mut harness = runner("/json");
    run(&mut harness);
    let line = ConsoleEvent {
        id: "run:1#0.1".to_owned(),
        at: Utc::now(),
        request: "Login".to_owned(),
        iteration: None,
        line: ConsoleLine::Log {
            level: LogLevel::default(),
            text: "heard".to_owned(),
            by: None,
        },
    };
    harness.state_mut().state.http.receive_console(line);
    settle(&mut harness);
    click_id(&mut harness, "http-console-unfold");
    assert_eq!(count(&harness, "http-console-line"), 1);

    click_id(&mut harness, "http-runner-start-empty");
    until(&mut harness, "the next run is over", |harness| {
        phase(harness) == Phase::Done
    });
    assert_eq!(count(&harness, "http-console-line"), 0);
}

#[test]
fn a_summary_counts_what_ran() {
    let results = [
        result(0, "Login", vec![passed("found"), failed("listed")]),
        result(0, "Orders", vec![passed("found")]),
    ];
    let said = RunSummary::of(&results);
    assert_eq!(
        said,
        RunSummary {
            failures: 1,
            requests: 2,
            passed: 2,
            tests: 3,
        }
    );
}
