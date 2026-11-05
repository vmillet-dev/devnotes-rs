//! The script editor over a real core. What runs them is the application relaunched: scenario 34.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use engine::http::model::{HttpItem, HttpItemKind, RequestScripts};
use engine::http::scripts::model::{FailureKind, ScriptKind, ScriptLevel, ScriptStop};

use super::tests::{active, click_id, click_named, editing, paiements};
use crate::notes::bench::{Bench, settle, until};
use crate::state::http::tabs::DraftEdit;

const AFTER: &str = "http-script-phase-1";

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

fn section(part: &str) -> Harness<'static, Bench> {
    let mut harness = editing();
    harness.set_size(egui::vec2(1_100.0, 1_000.0));
    settle(&mut harness);
    click_id(&mut harness, &format!("http-section-{part}"));
    harness
}

fn type_code(harness: &mut Harness<'_, Bench>, words: &str) {
    click_id(harness, "http-script-code");
    harness
        .get_by(|node| node.author_id() == Some("http-script-code"))
        .type_text(words);
    settle(harness);
}

fn scripts_of(harness: &Harness<'_, Bench>) -> RequestScripts {
    active(harness).document.scripts
}

fn said_count(harness: &Harness<'_, Bench>) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some("http-section-count"))
        .filter_map(|node| {
            use egui_kittest::kittest::NodeT;
            let node = node.accesskit_node();
            node.label().or(node.value())
        })
        .collect()
}

#[test]
fn each_phase_is_a_document_of_its_own_and_the_tab_counts_them() {
    let mut harness = section("scripts");
    type_code(&mut harness, "env.set('a', 1);");
    click_id(&mut harness, AFTER);
    type_code(&mut harness, "console.log(1);");

    let scripts = scripts_of(&harness);
    assert_eq!(scripts.before, "env.set('a', 1);");
    assert_eq!(scripts.after, "console.log(1);");
    assert_eq!(said_count(&harness), ["2"]);
}

#[test]
fn an_example_is_written_at_the_end_of_the_script() {
    let mut harness = section("scripts");
    type_code(&mut harness, "const a = 1;");
    click_id(&mut harness, "http-script-examples");
    click_id(&mut harness, "http-script-example-variable");

    assert_eq!(
        scripts_of(&harness).before,
        "const a = 1;\n\nenv.set('name', 'value');\n"
    );
}

#[test]
fn the_tests_are_counted_by_what_they_declare() {
    let mut harness = section("tests");
    assert!(!has(&harness, AFTER), "one document: nothing to switch");
    click_id(&mut harness, "http-script-examples");
    click_id(&mut harness, "http-script-example-test");

    assert!(
        scripts_of(&harness)
            .tests
            .starts_with("test(\"The status is 200\"")
    );
    assert_eq!(said_count(&harness), ["1"]);
}

#[test]
fn the_order_names_the_levels_and_one_opens_on_the_script_in_view() {
    let mut harness = section("scripts");
    until(&mut harness, "the levels are read", |harness| {
        harness
            .query_all_by(|node| node.author_id() == Some("http-script-level"))
            .count()
            == 3
    });

    click_named(
        &mut harness,
        "http-script-level",
        "collection Paiements · none",
    );
    until(&mut harness, "its settings are open", |harness| {
        has(harness, "http-settings-dialog")
    });
    let form = harness.state().state.http.container().unwrap();
    assert_eq!(form.focus, Some((ScriptKind::Before, None)));
}

#[test]
fn a_rule_stands_among_the_scripts_after_the_answer() {
    let mut harness = section("scripts");
    let tabs = harness.state_mut().state.http.tabs_mut();
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| {
        draft.apply(DraftEdit::Extractions(vec![
            engine::http::model::Extraction {
                path: "body.id".to_owned(),
                variable: "id".to_owned(),
                secret: false,
            },
        ]));
    });
    settle(&mut harness);
    assert!(!has(&harness, "http-extraction"));

    click_id(&mut harness, AFTER);
    assert!(has(&harness, "http-extraction"));
    click_id(&mut harness, "http-extraction-remove");
    assert!(active(&harness).document.extractions.is_empty());
}

fn stop(level: ScriptLevel, script: ScriptKind, owner: Option<String>) -> ScriptStop {
    ScriptStop {
        kind: FailureKind::Error,
        level: Some(level),
        script: Some(script),
        name: Some("Login".to_owned()),
        owner,
        line: Some(2),
        message: "boom".to_owned(),
    }
}

#[test]
fn open_the_script_leads_to_the_request_s_own_on_the_line_that_stopped() {
    let mut harness = editing();
    let key = harness
        .state()
        .state
        .http
        .tabs()
        .active()
        .unwrap()
        .key
        .clone();
    let stopped = stop(ScriptLevel::Request, ScriptKind::Tests, None);
    let bench = harness.state_mut();
    bench.state.http.open_script(&bench.backend, key, &stopped);
    settle(&mut harness);

    assert!(harness.state().state.http.script_focus().is_none(), "taken");
    assert!(has(&harness, "http-script-code"));
    assert!(!has(&harness, AFTER), "the tests' document");
}

#[test]
fn open_the_script_of_a_collection_opens_its_settings_on_the_phase() {
    let mut harness = paiements();
    let id = harness
        .state()
        .state
        .http
        .rows()
        .iter()
        .find(|row| row.name == "Paiements")
        .unwrap()
        .id
        .clone();
    let stopped = stop(ScriptLevel::Collection, ScriptKind::After, Some(id.clone()));
    let bench = harness.state_mut();
    let key = "any".to_owned();
    bench.state.http.open_script(&bench.backend, key, &stopped);
    until(&mut harness, "the settings are open", |harness| {
        harness.state().state.http.container().is_some()
    });

    let form = harness.state().state.http.container().unwrap();
    assert_eq!(form.focus, Some((ScriptKind::After, Some(2))));
    assert_eq!(
        form.container.item,
        HttpItem {
            kind: HttpItemKind::Collection,
            id
        }
    );
}
