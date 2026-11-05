//! The environments' page and what a protected one asks, over a real core.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::environments::VariableKind;
use engine::http::model::{HttpMethod, RequestKind};

use super::tests::{
    answered, click_id, click_named, collection, editing, request, said_by, sending_to, values,
};
use crate::notes::bench::{Bench, area_of, settle, until};
use crate::state::Area;
use crate::state::http::Page;
use crate::state::http::tabs::DraftEdit;
use crate::testing;

const TALL: egui::Vec2 = egui::vec2(1_100.0, 1_200.0);

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

fn page() -> Harness<'static, Bench> {
    let (backend, profile) = testing::unlocked();
    let api = collection(&backend, "Paiements");
    let kind = (RequestKind::Http, HttpMethod::Post);
    request(&backend, (&api, None), "Login", kind);
    let mut harness = area_of(Area::Http, backend, profile);
    harness.set_size(TALL);
    settle(&mut harness);
    click_id(&mut harness, "http-environments-open");
    until(&mut harness, "the page is read", |harness| {
        has(harness, "http-environments-empty")
    });
    harness
}

fn type_in(harness: &mut Harness<'_, Bench>, author_id: &str, words: &str) {
    let node = harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .unwrap();
    node.focus();
    settle(harness);
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .unwrap()
        .type_text(words);
    settle(harness);
}

fn create(harness: &mut Harness<'_, Bench>, name: &str) {
    click_id(harness, "http-environment-create-open");
    type_in(harness, "http-environment-create-input", name);
    click_id(harness, "http-environment-create-submit");
    until(harness, "the environment is shown", |harness| {
        has(harness, "http-environment-name")
    });
}

fn listed(harness: &Harness<'_, Bench>) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some("http-environment-entry"))
        .filter_map(|node| node.accesskit_node().label())
        .collect()
}

fn environments<'a>(
    harness: &'a Harness<'_, Bench>,
) -> &'a crate::state::http::environments::Environments {
    harness.state().state.http.environments()
}

#[test]
fn an_environment_is_created_shown_used_and_deleted_after_a_question() {
    let mut harness = page();
    create(&mut harness, "Staging");
    assert_eq!(listed(&harness), ["Globals", "Staging"]);
    assert_eq!(environments(&harness).active_id(), None);

    click_id(&mut harness, "http-environment-use");
    assert_eq!(
        environments(&harness).active().map(|it| it.name.as_str()),
        Some("Staging")
    );

    click_id(&mut harness, "http-environment-delete");
    assert!(has(&harness, "http-environment-delete-question"));
    click_id(&mut harness, "http-environment-delete-cancel");
    assert_eq!(listed(&harness), ["Globals", "Staging"]);

    click_id(&mut harness, "http-environment-delete");
    click_id(&mut harness, "http-environment-delete-confirm");
    until(&mut harness, "it has gone", |harness| {
        listed(harness) == ["Globals"]
    });
    assert_eq!(environments(&harness).active_id(), None);
}

#[test]
fn a_name_is_written_once_left_and_a_copy_is_shown() {
    let mut harness = page();
    create(&mut harness, "Staging");

    type_in(&mut harness, "http-environment-name", " EU");
    harness.key_press(egui::Key::Enter);
    until(&mut harness, "the name is written", |harness| {
        listed(harness) == ["Globals", "Staging EU"]
    });

    click_id(&mut harness, "http-environment-duplicate");
    until(&mut harness, "the copy is listed", |harness| {
        listed(harness).len() == 3
    });
    let shown = environments(&harness).shown().unwrap();
    assert_eq!(shown.name, "Staging EU (copy)");
}

#[test]
fn rows_typed_stay_as_typed_and_are_written_once_the_keys_rest() {
    let mut harness = page();
    type_in(&mut harness, "http-variable-name", "base");
    assert_eq!(values(&harness, "http-variable-name")[0], "base");

    until(&mut harness, "the globals hold it", |harness| {
        environments(harness)
            .globals()
            .first()
            .is_some_and(|row| row.name == "base")
    });
    assert_eq!(values(&harness, "http-variable-name")[0], "base");
}

fn click_first(harness: &mut Harness<'_, Bench>, author_id: &str) {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .unwrap()
        .click();
    settle(harness);
}

#[test]
fn a_secret_shown_in_one_set_is_masked_in_the_next() {
    let mut harness = page();
    type_in(&mut harness, "http-variable-name", "token");
    type_in(&mut harness, "http-variable-value", "abc");
    click_first(&mut harness, "http-variable-kind");
    click_id(&mut harness, "http-variable-kind-2");
    assert_eq!(environments(&harness).rows()[0].kind, VariableKind::Secret);
    assert_ne!(values(&harness, "http-variable-value")[0], "abc");

    click_id(&mut harness, "http-variable-reveal");
    assert_eq!(values(&harness, "http-variable-value")[0], "abc");

    create(&mut harness, "Staging");
    click_first(&mut harness, "http-environment-entry");
    until(&mut harness, "the globals are shown again", |harness| {
        has(harness, "http-variable-reveal")
    });
    assert_ne!(values(&harness, "http-variable-value")[0], "abc");
}

#[test]
fn a_protected_environment_asks_before_a_post_and_lets_a_get_through() {
    let mut harness = sending_to("/json");
    harness.set_size(TALL);
    settle(&mut harness);
    click_id(&mut harness, "http-environments-open");
    create(&mut harness, "Production");
    click_id(&mut harness, "http-environment-use");
    click_id(&mut harness, "http-environment-protected");
    until(&mut harness, "it is protected", |harness| {
        environments(harness)
            .active()
            .is_some_and(|active| active.protected)
    });
    assert!(has(&harness, "http-environment-protected-note"));
    click_id(&mut harness, "http-environments-open");
    let tabs = harness.state_mut().state.http.tabs_mut();
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| {
        draft.apply(DraftEdit::Method(HttpMethod::Post));
    });
    settle(&mut harness);

    click_id(&mut harness, "http-send");
    assert!(has(&harness, "http-protected-confirm"));
    assert!(!has(&harness, "http-response-status"));

    click_id(&mut harness, "http-protected-cancel");
    assert!(!has(&harness, "http-protected-confirm"));

    click_id(&mut harness, "http-send");
    click_id(&mut harness, "http-protected-send");
    answered(&mut harness);

    let tabs = harness.state_mut().state.http.tabs_mut();
    tabs.edit(&key, |draft| {
        draft.apply(DraftEdit::Method(HttpMethod::Get));
    });
    settle(&mut harness);
    click_id(&mut harness, "http-send");
    assert!(!has(&harness, "http-protected-confirm"));
}

#[test]
fn a_name_nothing_sets_is_given_a_row_on_the_page() {
    let mut harness = editing();
    harness.set_size(TALL);
    settle(&mut harness);
    let tabs = harness.state_mut().state.http.tabs_mut();
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| {
        draft.apply(DraftEdit::Url("{{base}}/login".to_owned()));
    });
    until(&mut harness, "the chip is shown", |harness| {
        has(harness, "http-used-variable")
    });

    click_id(&mut harness, "http-used-variable");
    assert_eq!(harness.state().state.http.page(), Page::Environments);
    assert_eq!(values(&harness, "http-variable-name")[0], "base");
    assert!(said_by(&harness, "http-environments").is_some());
}

#[test]
fn a_request_opened_from_the_rail_leaves_the_page() {
    let mut harness = page();
    click_named(&mut harness, "http-node-name", "POST Login");
    until(&mut harness, "the request is shown", |harness| {
        has(harness, "http-url")
    });
    assert_eq!(harness.state().state.http.page(), Page::Requests);
}
