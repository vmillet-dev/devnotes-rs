//! A request's body and its GraphQL document, over a real core.

use std::path::PathBuf;
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::graphql::GraphqlDocument;
use engine::http::model::{HttpMethod, RequestBody, RequestKind};

use crate::http::tests::{
    active, answered, click_id, click_named, collection, editing, request, said_by, sending_to,
    values,
};
use crate::notes::bench::{Bench, area_of, settle, until};
use crate::state::Area;
use crate::state::http::tabs::DraftEdit;
use crate::testing;

/// All of a box and what follows it in view: a click reaches only that.
const TALL: egui::Vec2 = egui::vec2(1_100.0, 1_200.0);

const JSON: &str = "http-body-kind-1";
const TEXT: &str = "http-body-kind-2";
const FORM: &str = "http-body-kind-3";
const MULTIPART: &str = "http-body-kind-4";
const FILE: &str = "http-body-kind-5";

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_by(|node| node.author_id() == Some(author_id))
        .is_some()
}

fn body_of(harness: &Harness<'_, Bench>) -> RequestBody {
    active(harness).document.body
}

fn graphql_of(harness: &Harness<'_, Bench>) -> GraphqlDocument {
    active(harness).document.graphql
}

fn type_in(harness: &mut Harness<'_, Bench>, author_id: &str, words: &str) {
    click_id(harness, author_id);
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .type_text(words);
    settle(harness);
}

/// The Body tab of `POST Login`, a kind chosen.
fn body(kind: &str) -> Harness<'static, Bench> {
    let mut harness = editing();
    harness.set_size(TALL);
    click_id(&mut harness, "http-section-body");
    click_id(&mut harness, kind);
    harness
}

fn edit(harness: &mut Harness<'_, Bench>, edit: DraftEdit) {
    let tabs = harness.state_mut().state.http.tabs_mut();
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| draft.apply(edit));
    settle(harness);
}

#[test]
fn a_kind_left_comes_back_as_it_was_and_json_and_text_share_their_text() {
    let mut harness = body(JSON);
    type_in(&mut harness, "http-body-text", "{\"amount\":4900}");
    let typed = "{\"amount\":4900}".to_owned();
    assert_eq!(
        body_of(&harness),
        RequestBody::Json {
            text: typed.clone()
        }
    );

    click_id(&mut harness, FORM);
    assert_eq!(body_of(&harness), RequestBody::Form { fields: Vec::new() });
    assert!(has(&harness, "http-form-table"));

    click_id(&mut harness, TEXT);
    assert_eq!(
        body_of(&harness),
        RequestBody::Text {
            text: typed.clone()
        }
    );
    click_id(&mut harness, JSON);
    assert_eq!(body_of(&harness), RequestBody::Json { text: typed });
}

#[test]
fn what_a_body_implies_is_said_and_json_that_does_not_read_is_not_formatted() {
    let mut harness = body(JSON);
    type_in(&mut harness, "http-body-text", "{\"a\":");
    until(&mut harness, "the fault is said", |harness| {
        has(harness, "http-body-problem")
    });
    assert!(!has(&harness, "http-body-valid"));
    assert!(
        said_by(&harness, "http-body-type").is_some_and(|said| said.contains("application/json"))
    );
    let format = harness.get_by(|node| node.author_id() == Some("http-body-format"));
    assert!(format.accesskit_node().is_disabled());

    edit(
        &mut harness,
        DraftEdit::Body(RequestBody::Json {
            text: "{\"a\":1}".to_owned(),
        }),
    );
    until(&mut harness, "the body reads", |harness| {
        has(harness, "http-body-valid")
    });
    click_id(&mut harness, "http-body-format");
    until(&mut harness, "the body is formatted", |harness| {
        body_of(harness)
            == RequestBody::Json {
                text: "{ \"a\": 1 }".to_owned(),
            }
    });
    assert_eq!(values(&harness, "http-body-text"), ["{ \"a\": 1 }"]);
}

#[test]
fn a_get_with_a_body_warns_on_its_tab_and_goes_back_to_none() {
    let mut harness = body(JSON);
    type_in(&mut harness, "http-body-text", "{}");
    assert!(!has(&harness, "http-body-unusual"));

    edit(&mut harness, DraftEdit::Method(HttpMethod::Get));
    until(&mut harness, "the warning is shown", |harness| {
        has(harness, "http-body-unusual")
    });
    assert!(has(&harness, "http-body-unusual-mark"));

    click_id(&mut harness, "http-section-params");
    assert!(!has(&harness, "http-body-unusual"));
    assert!(has(&harness, "http-body-unusual-mark"));

    click_id(&mut harness, "http-section-body");
    click_id(&mut harness, "http-body-to-none");
    assert_eq!(body_of(&harness), RequestBody::None);
    until(&mut harness, "the warning has gone", |harness| {
        !has(harness, "http-body-unusual") && !has(harness, "http-body-unusual-mark")
    });

    click_id(&mut harness, JSON);
    assert_eq!(
        body_of(&harness),
        RequestBody::Json {
            text: "{}".to_owned()
        }
    );
}

#[test]
fn a_body_is_a_file_by_its_path() {
    let mut harness = body(FILE);
    assert!(said_by(&harness, "http-body-path").is_some());

    let path = PathBuf::from("facture.pdf");
    harness.state_mut().state.dialog = Arc::new(platform::dialog::Fixed(path));
    click_id(&mut harness, "http-body-choose");
    until(&mut harness, "the path is written", |harness| {
        body_of(harness)
            == RequestBody::Binary {
                path: "facture.pdf".to_owned(),
            }
    });
    assert_eq!(
        said_by(&harness, "http-body-path").as_deref(),
        Some("facture.pdf")
    );
}

fn parts_of(harness: &Harness<'_, Bench>) -> Vec<(String, String, bool)> {
    match body_of(harness) {
        RequestBody::Multipart { parts } => parts
            .into_iter()
            .map(|part| (part.key, part.value, part.file))
            .collect(),
        other => panic!("no parts in {other:?}"),
    }
}

#[test]
fn a_multipart_row_is_a_text_or_a_file_and_the_blank_row_becomes_a_part() {
    let mut harness = body(MULTIPART);
    type_in(&mut harness, "http-part-key", "note");
    assert_eq!(
        parts_of(&harness),
        [("note".to_owned(), String::new(), false)]
    );

    let values_at = harness
        .query_all_by(|node| node.author_id() == Some("http-part-value"))
        .count();
    assert_eq!(values_at, 2, "a value for the part and the blank row");

    let path = PathBuf::from("photo.png");
    harness.state_mut().state.dialog = Arc::new(platform::dialog::Fixed(path));
    harness
        .query_all_by(|node| node.author_id() == Some("http-part-kind"))
        .next()
        .unwrap()
        .click();
    settle(&mut harness);
    click_id(&mut harness, "http-part-choose");
    until(&mut harness, "the file is written", |harness| {
        parts_of(harness) == [("note".to_owned(), "photo.png".to_owned(), true)]
    });

    click_id(&mut harness, "http-part-remove");
    assert_eq!(parts_of(&harness), []);
}

#[test]
fn ctrl_enter_in_a_body_sends_and_writes_no_line() {
    let mut harness = sending_to("/json");
    harness.set_size(TALL);
    click_id(&mut harness, "http-section-body");
    click_id(&mut harness, JSON);
    type_in(&mut harness, "http-body-text", "{}");

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Enter);
    answered(&mut harness);

    assert_eq!(
        body_of(&harness),
        RequestBody::Json {
            text: "{}".to_owned()
        }
    );
}

/// A GraphQL request, its Query tab in front.
fn graphql() -> Harness<'static, Bench> {
    let (backend, profile) = testing::unlocked();
    let api = collection(&backend, "Produits");
    let kind = (RequestKind::Graphql, HttpMethod::Post);
    request(&backend, (&api, None), "Catalogue", kind);
    let mut harness = area_of(Area::Http, backend, profile);
    harness.set_size(TALL);
    click_named(&mut harness, "http-node-name", "GQL Catalogue");
    until(&mut harness, "the query is open", |harness| {
        has(harness, "http-graphql-query")
    });
    harness
}

#[test]
fn a_query_left_open_is_said_and_stops_nothing() {
    let mut harness = graphql();
    assert!(has(&harness, "http-graphql-variables-hint"));

    type_in(&mut harness, "http-graphql-query", "{ products");
    until(&mut harness, "the brace is said", |harness| {
        has(harness, "http-graphql-query-problem")
    });
    let format = harness.get_by(|node| node.author_id() == Some("http-graphql-format-query"));
    assert!(format.accesskit_node().is_disabled());
    assert_eq!(graphql_of(&harness).query, "{ products");
}

#[test]
fn a_variable_without_a_value_is_named_and_added() {
    let mut harness = graphql();
    let query = "query One($code: ID!) { product(code: $code) { id } }".to_owned();
    edit(
        &mut harness,
        DraftEdit::Graphql(GraphqlDocument {
            query,
            ..GraphqlDocument::default()
        }),
    );
    until(&mut harness, "the variable is named", |harness| {
        has(harness, "http-graphql-missing")
    });
    assert_eq!(
        said_by(&harness, "http-graphql-variable").as_deref(),
        Some("$code")
    );

    click_id(&mut harness, "http-graphql-add");
    until(&mut harness, "the variable is added", |harness| {
        graphql_of(harness).variables.contains("\"code\": null")
    });
    until(&mut harness, "nothing lacks", |harness| {
        !has(harness, "http-graphql-missing")
    });
}

#[test]
fn the_operation_sent_is_chosen_when_there_are_several_and_a_get_is_ticked() {
    let mut harness = graphql();
    assert!(!has(&harness, "http-graphql-operation"));
    let query = "query One { a } query Two { b }".to_owned();
    edit(
        &mut harness,
        DraftEdit::Graphql(GraphqlDocument {
            query,
            ..GraphqlDocument::default()
        }),
    );
    until(&mut harness, "the operations are offered", |harness| {
        has(harness, "http-graphql-operation")
    });

    click_id(&mut harness, "http-graphql-get");
    assert!(graphql_of(&harness).as_get);
}
