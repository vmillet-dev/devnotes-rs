//! The history over a real core: what a send leaves, read again, deleted and reopened.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use engine::http::model::{HttpItem, HttpItemKind, HttpMethod};

use super::tests::{active, answered, click_id, said_by, sending_to, values};
use crate::notes::bench::{Bench, settle, until};
use crate::state::http::tabs::DraftEdit;
use crate::state::http::{Page, Write};

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

/// One send to `/json`, and the history open on it.
fn sent() -> Harness<'static, Bench> {
    let mut harness = sending_to("/json");
    click_id(&mut harness, "http-send");
    answered(&mut harness);
    click_id(&mut harness, "http-history-open");
    until(&mut harness, "the send is listed", |harness| {
        has(harness, "http-history-item")
    });
    harness
}

fn page(harness: &Harness<'_, Bench>) -> Page {
    harness.state().state.http.page()
}

fn read_it(harness: &mut Harness<'_, Bench>) {
    click_id(harness, "http-history-item");
    until(harness, "the send is read", |harness| {
        has(harness, "http-history-url")
    });
}

#[test]
fn a_send_is_listed_by_day_and_read_like_an_answer() {
    let mut harness = sent();
    assert_eq!(
        said_by(&harness, "http-history-day").as_deref(),
        Some("Today")
    );
    assert_eq!(
        said_by(&harness, "http-history-status").as_deref(),
        Some("200")
    );

    read_it(&mut harness);
    assert!(said_by(&harness, "http-history-url").is_some_and(|url| url.ends_with("/json")));
    assert!(has(&harness, "http-response-status"));
    assert!(
        !has(&harness, "http-response-clear"),
        "no tab to clear it from"
    );
}

#[test]
fn the_same_button_puts_the_page_away() {
    let mut harness = sent();
    assert_eq!(page(&harness), Page::History);
    click_id(&mut harness, "http-history-open");
    assert_eq!(page(&harness), Page::Requests);
}

#[test]
fn a_send_deleted_comes_back_from_its_bar() {
    let mut harness = sent();
    click_id(&mut harness, "http-history-delete");
    until(&mut harness, "the bar is shown", |harness| {
        has(harness, "http-history-undo") && !has(harness, "http-history-item")
    });

    click_id(&mut harness, "http-history-undo-restore");
    until(&mut harness, "the send is back", |harness| {
        has(harness, "http-history-item")
    });
    assert!(!has(&harness, "http-history-undo"));
}

#[test]
fn clearing_says_how_many_go() {
    let mut harness = sent();
    click_id(&mut harness, "http-history-clear");
    until(&mut harness, "the count is said", |harness| {
        has(harness, "http-history-clear-count")
    });
    click_id(&mut harness, "http-history-clear-cancel");
    assert!(has(&harness, "http-history-item"));

    click_id(&mut harness, "http-history-clear");
    until(&mut harness, "the count is said", |harness| {
        has(harness, "http-history-clear-confirm")
    });
    click_id(&mut harness, "http-history-clear-confirm");
    until(&mut harness, "nothing is left", |harness| {
        has(harness, "http-history-empty")
    });
}

fn change_method(harness: &mut Harness<'_, Bench>, method: HttpMethod) {
    let tabs = harness.state_mut().state.http.tabs_mut();
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| draft.apply(DraftEdit::Method(method)));
    settle(harness);
}

#[test]
fn a_send_goes_back_onto_its_request_asking_before_it_replaces_changes() {
    let mut harness = sent();
    change_method(&mut harness, HttpMethod::Delete);
    read_it(&mut harness);

    click_id(&mut harness, "http-history-reopen");
    assert!(has(&harness, "http-history-replace"));
    click_id(&mut harness, "http-history-replace-cancel");
    assert_eq!(active(&harness).method, HttpMethod::Delete);

    click_id(&mut harness, "http-history-reopen");
    click_id(&mut harness, "http-history-replace-confirm");
    until(&mut harness, "the requests are shown again", |harness| {
        page(harness) == Page::Requests
    });
    assert_eq!(active(&harness).method, HttpMethod::Get);
    assert!(values(&harness, "http-url")[0].ends_with("/json"));
}

#[test]
fn a_send_from_a_request_gone_opens_as_a_draft_named_after_its_time() {
    let mut harness = sent();
    read_it(&mut harness);
    let id = active_request(&harness);
    let bench = harness.state_mut();
    let item = HttpItem {
        kind: HttpItemKind::Request,
        id,
    };
    bench.state.http.write(&bench.backend, Write::Delete(item));
    until(&mut harness, "the tree is read again", |harness| {
        said_by(harness, "http-history-reopen").is_some_and(|said| !said.contains("in "))
    });

    click_id(&mut harness, "http-history-reopen");
    until(&mut harness, "the draft is open", |harness| {
        page(harness) == Page::Requests
    });
    let tab = harness.state().state.http.tabs().active().unwrap();
    assert!(tab.request_id.is_none());
    assert!(tab.draft.name.contains(" · "), "{}", tab.draft.name);
}

fn active_request(harness: &Harness<'_, Bench>) -> String {
    let tabs = harness.state().state.http.tabs();
    tabs.active().unwrap().request_id.clone().unwrap()
}
