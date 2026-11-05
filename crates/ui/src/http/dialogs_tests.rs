//! The cookie jar and a container's settings, over a real core.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::model::{HttpItem, HttpItemKind, RequestAuth};

use super::tests::{answered, choose, click_id, paiements, said_by, sending_to};
use crate::notes::bench::{Bench, settle, until};

const TALL: egui::Vec2 = egui::vec2(1_100.0, 1_200.0);

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

/// An answer that set `session=abc`, and the jar open on it.
fn jar() -> Harness<'static, Bench> {
    let mut harness = sending_to("/json");
    harness.set_size(TALL);
    settle(&mut harness);
    click_id(&mut harness, "http-send");
    answered(&mut harness);
    click_id(&mut harness, "http-cookies-open");
    until(&mut harness, "the jar is read", |harness| {
        has(harness, "http-cookie")
    });
    harness
}

#[test]
fn a_cookie_set_by_an_answer_is_listed_and_removed() {
    let mut harness = jar();
    assert_eq!(said_by(&harness, "http-cookie").as_deref(), Some("session"));
    assert!(has(&harness, "http-cookies-domain"));

    click_id(&mut harness, "http-cookie-remove");
    until(&mut harness, "the jar is empty", |harness| {
        has(harness, "http-cookies-empty")
    });
}

#[test]
fn emptying_the_jar_says_how_many_go_and_cancel_keeps_them() {
    let mut harness = jar();
    click_id(&mut harness, "http-cookies-clear");
    until(&mut harness, "the count is said", |harness| {
        has(harness, "http-cookies-clear-count")
    });
    click_id(&mut harness, "http-cookies-clear-cancel");
    assert!(has(&harness, "http-cookie"));

    click_id(&mut harness, "http-cookies-clear");
    until(&mut harness, "the count is said", |harness| {
        has(harness, "http-cookies-clear-confirm")
    });
    click_id(&mut harness, "http-cookies-clear-confirm");
    until(&mut harness, "the jar is empty", |harness| {
        has(harness, "http-cookies-empty")
    });

    click_id(&mut harness, "http-cookies-close");
    assert!(!has(&harness, "http-cookies-dialog"));
}

fn settings_of(harness: &Harness<'_, Bench>, kind: HttpItemKind, id: String) -> RequestAuth {
    let item = HttpItem { kind, id };
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::http::http_settings(engine, item)
    })
    .unwrap()
    .auth
}

fn collection_id(harness: &Harness<'_, Bench>) -> String {
    let rows = harness.state().state.http.rows();
    let row = rows.iter().find(|row| row.name == "Paiements").unwrap();
    row.id.clone()
}

/// The dialog on a row, at rest: it centres itself again when what it reads changes its height.
fn open_settings(harness: &mut Harness<'_, Bench>, row: &str) {
    choose(harness, row, &["http-node-settings"]);
    until(harness, "the settings are read", |harness| {
        has(harness, "http-auth-kind")
    });
    let last = std::cell::Cell::new(None);
    until(harness, "the dialog rests", |harness| {
        let cancel = harness.get_by(|node| node.author_id() == Some("http-settings-cancel"));
        let at = cancel.accesskit_node().bounding_box();
        last.replace(at) == at && at.is_some()
    });
}

#[test]
fn a_collection_s_settings_are_written_on_save_and_not_on_cancel() {
    let mut harness = paiements();
    open_settings(&mut harness, "Paiements");

    click_id(&mut harness, "http-auth-kind");
    click_id(&mut harness, "http-auth-kind-2");
    click_id(&mut harness, "http-settings-cancel");
    let id = collection_id(&harness);
    assert_eq!(
        settings_of(&harness, HttpItemKind::Collection, id.clone()),
        RequestAuth::Inherit
    );

    open_settings(&mut harness, "Paiements");
    click_id(&mut harness, "http-auth-kind");
    click_id(&mut harness, "http-auth-kind-2");
    click_id(&mut harness, "http-settings-save");
    until(&mut harness, "the dialog closes", |harness| {
        !has(harness, "http-settings-dialog")
    });
    assert_eq!(
        settings_of(&harness, HttpItemKind::Collection, id),
        RequestAuth::Bearer {
            token: String::new()
        }
    );
}

#[test]
fn a_folder_may_inherit_and_a_collection_may_not() {
    let mut harness = paiements();
    open_settings(&mut harness, "Auth");
    assert!(has(&harness, "http-auth-inherited"));
    click_id(&mut harness, "http-settings-cancel");

    open_settings(&mut harness, "Paiements");
    click_id(&mut harness, "http-auth-kind");
    assert!(!has(&harness, "http-auth-kind-4"), "four kinds, no inherit");
    assert!(!has(&harness, "http-auth-inherited"), "nothing is above it");
}
