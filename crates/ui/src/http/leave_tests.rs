//! The question before the window goes, over a real core.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::tests::{click_id, paiements, sending_to};
use crate::api;
use crate::notes::bench::{Bench, settle, until};
use crate::state::http::tabs::DraftEdit;
use crate::testing;

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

fn ask(harness: &mut Harness<'_, Bench>) -> bool {
    let may_go = harness.state_mut().state.http.ask_leave();
    settle(harness);
    may_go
}

fn released(harness: &Harness<'_, Bench>) -> bool {
    harness.state().state.http.leave().is_released()
}

/// A request of the bench changed, its tab open.
fn modified() -> Harness<'static, Bench> {
    let mut harness = sending_to("/json");
    let tabs = harness.state_mut().state.http.tabs_mut();
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| {
        draft.apply(DraftEdit::Url("https://api.test/changed".to_owned()));
    });
    settle(&mut harness);
    harness
}

fn stand_still(harness: &mut Harness<'_, Bench>) {
    let last = std::cell::Cell::new(None);
    until(harness, "the dialog rests", |harness| {
        let save = harness.get_by(|node| node.author_id() == Some("http-leave-save"));
        let at = save.accesskit_node().bounding_box();
        last.replace(at) == at && at.is_some()
    });
}

#[test]
fn nothing_is_asked_when_nothing_would_be_lost() {
    let mut harness = paiements();
    assert!(ask(&mut harness));
    assert!(!has(&harness, "http-leave-dialog"));
}

#[test]
fn a_request_with_changes_is_named_and_cancel_stays() {
    let mut harness = modified();
    assert!(!ask(&mut harness));
    assert!(has(&harness, "http-leave-dialog"));
    assert!(has(&harness, "http-leave-name"));
    stand_still(&mut harness);

    click_id(&mut harness, "http-leave-cancel");
    assert!(!has(&harness, "http-leave-dialog"));
    assert!(!released(&harness));
}

#[test]
fn discarding_lets_the_window_go_and_asks_no_more() {
    let mut harness = modified();
    ask(&mut harness);
    stand_still(&mut harness);
    click_id(&mut harness, "http-leave-discard");
    assert!(released(&harness));
    assert!(ask(&mut harness));
}

#[test]
fn saving_all_writes_each_request_then_lets_the_window_go() {
    let mut harness = modified();
    ask(&mut harness);
    stand_still(&mut harness);
    click_id(&mut harness, "http-leave-save");
    until(&mut harness, "the requests are written", released);
    assert_eq!(harness.state().state.http.modified().count(), 0);
}

#[test]
fn a_request_never_saved_is_asked_its_place_and_refusing_keeps_the_question() {
    let mut harness = paiements();
    let tabs = harness.state_mut().state.http.tabs_mut();
    tabs.new_request(engine::http::model::RequestKind::Http, "Draft".to_owned());
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| {
        draft.apply(DraftEdit::Url("https://api.test/new".to_owned()));
    });
    settle(&mut harness);
    ask(&mut harness);
    stand_still(&mut harness);
    click_id(&mut harness, "http-leave-save");
    until(&mut harness, "its place is asked", |harness| {
        has(harness, "http-save-dialog")
    });
    let last = std::cell::Cell::new(None);
    until(&mut harness, "the dialog rests", |harness| {
        let cancel = harness.get_by(|node| node.author_id() == Some("http-save-cancel"));
        let at = cancel.accesskit_node().bounding_box();
        last.replace(at) == at && at.is_some()
    });

    click_id(&mut harness, "http-save-cancel");
    until(&mut harness, "the question is back", |harness| {
        has(harness, "http-leave-dialog")
    });
    assert!(!released(&harness));
}

#[test]
fn a_write_queued_just_before_the_exit_is_written() {
    let (backend, _profile) = testing::unlocked();
    let queued = api::http::create_collection(&backend, "Queued".to_owned());
    assert!(backend.drain(std::time::Duration::from_secs(30)));
    drop(queued);
    let tree = engine::db::run_here(backend.engine(), engine::http::http_tree).unwrap();
    assert_eq!(tree.collections.len(), 1);
}
