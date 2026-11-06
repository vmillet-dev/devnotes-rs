//! The console, an answer's tests, and what leaves the area to be kept, over a real core.

use chrono::Utc;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::console::{ConsoleEvent, ConsoleLine, secret_segments};
use engine::http::scripts::model::LogLevel;

use super::tests::{answered, click_id, said_by, sending_to};
use crate::notes::bench::{Bench, settle, until};

const TALL: egui::Vec2 = egui::vec2(1_100.0, 1_200.0);

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

fn answer() -> Harness<'static, Bench> {
    let mut harness = sending_to("/json");
    harness.set_size(TALL);
    settle(&mut harness);
    click_id(&mut harness, "http-send");
    answered(&mut harness);
    harness
}

fn active_key(harness: &Harness<'_, Bench>) -> String {
    harness
        .state()
        .state
        .http
        .tabs()
        .active()
        .unwrap()
        .key
        .clone()
}

/// A line under the send the tab is showing.
fn say(harness: &mut Harness<'_, Bench>, line: ConsoleLine) {
    let key = active_key(harness);
    let id = {
        let http = &harness.state().state.http;
        http.sends().of(&key).unwrap().id.clone()
    };
    harness
        .state_mut()
        .state
        .http
        .receive_console(ConsoleEvent {
            id,
            at: Utc::now(),
            request: "Ping".to_owned(),
            iteration: None,
            line,
        });
    settle(harness);
}

fn log(text: &str) -> ConsoleLine {
    ConsoleLine::Log {
        level: LogLevel::default(),
        text: text.to_owned(),
        by: None,
    }
}

#[test]
fn folded_the_console_counts_and_unfolded_it_filters() {
    let mut harness = answer();
    say(&mut harness, log("hello {{token}}"));
    say(
        &mut harness,
        ConsoleLine::Missed {
            path: "body.id".to_owned(),
            variable: "id".to_owned(),
        },
    );
    assert!(said_by(&harness, "http-console-summary").is_some());

    click_id(&mut harness, "http-console-unfold");
    let lines = |harness: &Harness<'_, Bench>| {
        harness
            .query_all_by(|node| node.author_id() == Some("http-console-line"))
            .count()
    };
    assert_eq!(lines(&harness), 2);

    click_id(&mut harness, "http-console-filter-errors");
    assert_eq!(lines(&harness), 1);

    click_id(&mut harness, "http-console-clear");
    assert_eq!(lines(&harness), 0);

    click_id(&mut harness, "http-console-fold");
    assert!(has(&harness, "http-console-unfold"));
}

#[test]
fn a_secret_written_as_its_name_is_set_apart() {
    assert_eq!(
        secret_segments("Bearer {{token}} now"),
        [
            ("Bearer ".to_owned(), false),
            ("{{token}}".to_owned(), true),
            (" now".to_owned(), false),
        ]
    );
    let empty = secret_segments("{{}} x");
    assert!(
        empty.iter().all(|(_, secret)| !secret),
        "no name, no secret"
    );
}

#[test]
fn an_answer_is_kept_as_a_note_where_the_dialog_says() {
    let mut harness = answer();
    let bench = harness.state_mut();
    bench.state.spaces.create_space(&bench.backend, "Travail");
    until(&mut harness, "the space is read", |harness| {
        !harness.state().state.spaces.spaces().is_empty()
    });
    click_id(&mut harness, "http-response-save-note");
    until(&mut harness, "the dialog asks", |harness| {
        has(harness, "save-as-note-submit")
    });
    let last = std::cell::Cell::new(None);
    until(&mut harness, "the dialog rests", |harness| {
        let submit = harness.get_by(|node| node.author_id() == Some("save-as-note-submit"));
        let at = submit.accesskit_node().bounding_box();
        last.replace(at) == at && at.is_some()
    });

    click_id(&mut harness, "save-as-note-submit");
    until(&mut harness, "the note is written", |harness| {
        !has(harness, "save-as-note") && harness.state().state.status.latest().is_some()
    });
    let (_, said) = harness.state().state.status.latest().unwrap();
    assert!(said.text(i18n::Language::En).contains("Response from"));
}
