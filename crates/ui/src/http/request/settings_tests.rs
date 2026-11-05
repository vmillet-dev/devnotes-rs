//! A request's auth, variables, settings and extractions, and the environment it is sent with,
//! over a real core.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::environments::{FilledBy, Variable, VariableKind};
use engine::http::model::{
    ContainerSettings, Extraction, HttpItem, HttpItemKind, HttpMethod, RequestAuth, RequestKind,
};

use crate::http::tests::{
    active, answered, click_id, click_named, collection, editing, request, said_by, sending_to,
    values,
};
use crate::notes::bench::{Bench, area_of, remembered, settle, until};
use crate::state::Area;
use crate::state::http::environments::ACTIVE_KEY;
use crate::state::http::tabs::DraftEdit;
use crate::testing;

const TALL: egui::Vec2 = egui::vec2(1_100.0, 1_200.0);

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

/// What the first label wearing `author_id` says.
fn first_said(harness: &Harness<'_, Bench>, author_id: &str) -> String {
    let node = harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .unwrap();
    let node = node.accesskit_node();
    node.label().or(node.value()).unwrap_or_default()
}

fn section(section: &str) -> Harness<'static, Bench> {
    let mut harness = editing();
    harness.set_size(TALL);
    click_id(&mut harness, &format!("http-section-{section}"));
    harness
}

fn type_in(harness: &mut Harness<'_, Bench>, author_id: &str, nth: usize, words: &str) {
    let node = harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .nth(nth)
        .unwrap();
    node.focus();
    settle(harness);
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .nth(nth)
        .unwrap()
        .type_text(words);
    settle(harness);
}

fn edit(harness: &mut Harness<'_, Bench>, edit: DraftEdit) {
    let tabs = harness.state_mut().state.http.tabs_mut();
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| draft.apply(edit));
    settle(harness);
}

fn pick(harness: &mut Harness<'_, Bench>, menu: &str, label: &str) {
    click_id(harness, menu);
    harness.get_by_label(label).click();
    settle(harness);
}

#[test]
fn an_auth_inherited_says_what_it_is_and_from_where_its_secret_masked() {
    let (backend, profile) = testing::unlocked();
    let api = collection(&backend, "Paiements");
    request(
        &backend,
        (&api, None),
        "Login",
        (RequestKind::Http, HttpMethod::Post),
    );
    let id = api.id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        let settings = ContainerSettings {
            auth: RequestAuth::Bearer {
                token: "s3cret".to_owned(),
            },
            ..ContainerSettings::default()
        };
        let item = HttpItem {
            kind: HttpItemKind::Collection,
            id,
        };
        engine::http::save_http_settings(engine, item, settings)
    })
    .unwrap();
    let mut harness = area_of(Area::Http, backend, profile);
    harness.set_size(TALL);
    click_named(&mut harness, "http-node-name", "POST Login");
    until(&mut harness, "the request is open", |harness| {
        has(harness, "http-section-auth")
    });
    click_id(&mut harness, "http-section-auth");

    until(&mut harness, "the origin is said", |harness| {
        said_by(harness, "http-auth-inherited").is_some_and(|said| said.contains("Paiements"))
    });
    let said = said_by(&harness, "http-auth-inherited").unwrap();
    assert!(said.starts_with("Bearer ••••••"), "{said}");
    assert!(!said.contains("s3cret"));
}

#[test]
fn an_auth_of_its_own_keeps_its_secret_masked_until_asked() {
    let mut harness = section("auth");
    pick(&mut harness, "http-auth-kind", "Bearer");
    assert_eq!(
        active(&harness).document.auth,
        RequestAuth::Bearer {
            token: String::new()
        }
    );

    type_in(&mut harness, "http-auth-secret", 0, "abc");
    assert_eq!(
        active(&harness).document.auth,
        RequestAuth::Bearer {
            token: "abc".to_owned()
        }
    );
    assert_ne!(values(&harness, "http-auth-secret"), ["abc"]);

    click_id(&mut harness, "http-auth-reveal");
    assert_eq!(values(&harness, "http-auth-secret"), ["abc"]);
}

#[test]
fn a_number_is_settled_once_left_and_a_switch_says_what_it_overrides() {
    let mut harness = section("settings");
    assert!(first_said(&harness, "http-transport-from").contains("30000 ms"));

    type_in(&mut harness, "http-transport-timeoutMs", 0, "5");
    assert_eq!(active(&harness).document.transport.timeout_ms, None);
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);
    assert_eq!(active(&harness).document.transport.timeout_ms, Some(100));

    assert!(!has(&harness, "http-transport-insecure"));
    pick(&mut harness, "http-transport-verifyTls", "No");
    assert_eq!(active(&harness).document.transport.verify_tls, Some(false));
    assert!(has(&harness, "http-transport-insecure"));
}

#[test]
fn a_variable_is_typed_a_fault_said_and_a_secret_masked() {
    let mut harness = section("variables");
    type_in(&mut harness, "http-variable-name", 0, "bad name");
    assert_eq!(active(&harness).document.variables[0].name, "bad name");
    assert!(has(&harness, "http-variable-fault"));

    edit(
        &mut harness,
        DraftEdit::Variables(vec![Variable {
            name: "token".to_owned(),
            value: "abc".to_owned(),
            kind: VariableKind::Secret,
            filled_by: Some(FilledBy {
                request_id: None,
                name: "Login".to_owned(),
            }),
            ..Variable::default()
        }]),
    );
    assert!(!has(&harness, "http-variable-fault"));
    assert_ne!(values(&harness, "http-variable-value")[0], "abc");
    assert!(has(&harness, "http-variable-filled"));

    click_id(&mut harness, "http-variable-reveal");
    assert_eq!(values(&harness, "http-variable-value")[0], "abc");

    click_id(&mut harness, "http-variable-remove");
    assert!(active(&harness).document.variables.is_empty());
}

#[test]
fn a_value_of_the_answer_becomes_a_rule_named_by_the_core() {
    let mut harness = sending_to("/json");
    click_id(&mut harness, "http-send");
    answered(&mut harness);
    click_id(&mut harness, "http-response-view-3");
    until(&mut harness, "the tree is shown", |harness| {
        has(harness, "json-line")
    });
    harness
        .query_all_by(|node| node.author_id() == Some("json-line"))
        .nth(1)
        .unwrap()
        .click();
    settle(&mut harness);
    click_id(&mut harness, "json-extract");
    assert!(has(&harness, "http-extract-path"));
    let path = said_by(&harness, "http-extract-path").unwrap();
    assert!(path.starts_with("body"), "{path}");

    click_id(&mut harness, "http-extract-submit");
    let rules = active(&harness).document.extractions;
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].path, path);
    assert!(!has(&harness, "http-extract-dialog"));
}

#[test]
fn a_rule_is_read_and_removed_under_the_scripts() {
    let mut harness = section("scripts");
    edit(
        &mut harness,
        DraftEdit::Extractions(vec![Extraction {
            path: "body.token".to_owned(),
            variable: "token".to_owned(),
            secret: true,
        }]),
    );
    assert_eq!(
        said_by(&harness, "http-extraction").as_deref(),
        Some("body.token")
    );

    click_id(&mut harness, "http-extraction-remove");
    assert!(active(&harness).document.extractions.is_empty());
}

#[test]
fn the_environment_chosen_is_the_library_s_and_a_deleted_one_is_none() {
    let (backend, profile) = testing::unlocked();
    let staging = engine::db::run_here(backend.engine(), |engine| {
        engine::http::create_http_environment(engine, "Staging".to_owned())
    })
    .unwrap();
    let api = collection(&backend, "Paiements");
    request(
        &backend,
        (&api, None),
        "Login",
        (RequestKind::Http, HttpMethod::Post),
    );
    let mut harness = area_of(Area::Http, backend, profile);
    click_named(&mut harness, "http-node-name", "POST Login");
    until(&mut harness, "the sets are read", |harness| {
        !harness.state().state.http.environments().all().is_empty()
    });

    click_id(&mut harness, "http-environment-picker");
    click_named(&mut harness, "http-environment-option", "Staging");

    let http = &harness.state().state.http;
    assert_eq!(http.environments().active_id(), Some(staging.id.as_str()));
    assert_eq!(remembered(&harness, ACTIVE_KEY), Some(staging.id.clone()));

    let http = &mut harness.state_mut().state.http;
    http.choose_environment(Some("deleted since".to_owned()));
    assert_eq!(http.environments().active_id(), None);
}

#[test]
fn a_request_offers_its_parts() {
    let harness = editing();
    for part in [
        "params",
        "headers",
        "auth",
        "body",
        "variables",
        "scripts",
        "settings",
    ] {
        assert!(has(&harness, &format!("http-section-{part}")), "{part}");
    }
}
