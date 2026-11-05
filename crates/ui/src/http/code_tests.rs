//! The code dialog and a draft's first save, over a real core.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::codegen::CodeTarget;
use engine::http::model::RequestAuth;

use super::tests::{active, click_id, click_named, editing, paiements, said_by};
use crate::notes::bench::{Bench, area_of, names, settle, until};
use crate::state::Area;
use crate::state::http::code::Asked;
use crate::state::http::tabs::DraftEdit;
use crate::testing;

fn has(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .next()
        .is_some()
}

fn code_of(harness: &Harness<'_, Bench>) -> String {
    said_by(harness, "http-code-output").unwrap_or_default()
}

/// The dialog on `POST Login`, a token of the request's own.
fn dialog() -> Harness<'static, Bench> {
    let mut harness = editing();
    let tabs = harness.state_mut().state.http.tabs_mut();
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| {
        draft.apply(DraftEdit::Auth(RequestAuth::Bearer {
            token: "{{token}}".to_owned(),
        }));
    });
    settle(&mut harness);
    click_id(&mut harness, "http-code-open");
    until(&mut harness, "the code is written", |harness| {
        code_of(harness).contains("api.example.com")
    });
    rest(&mut harness);
    harness
}

/// A dialog centres itself again when what it shows changes its height.
fn rest(harness: &mut Harness<'_, Bench>) {
    let last = std::cell::Cell::new(None);
    until(harness, "the dialog rests", |harness| {
        let copy = harness.get_by(|node| node.author_id() == Some("http-code-copy"));
        let at = copy.accesskit_node().bounding_box();
        last.replace(at) == at && at.is_some()
    });
}

#[test]
fn the_code_is_rust_s_and_each_target_writes_its_own() {
    let mut harness = dialog();
    assert!(code_of(&harness).starts_with("curl"));

    click_named(&mut harness, "http-code-target", "JavaScript · fetch");
    until(&mut harness, "the fetch call", |harness| {
        code_of(harness).contains("await fetch(")
    });
    assert_eq!(
        harness.state().state.http.code().target(),
        CodeTarget::Fetch
    );

    let multiline = harness.get_by(|node| node.author_id() == Some("http-code-multiline"));
    assert!(
        multiline.accesskit_node().is_disabled(),
        "a program is laid out on its own"
    );

    click_id(&mut harness, "http-code-copy");
    click_id(&mut harness, "http-code-copy");
    assert!(has(&harness, "http-code-output"));
}

#[test]
fn the_list_is_filtered_by_a_name_or_a_library() {
    let mut harness = dialog();
    let field = harness.get_by(|node| node.author_id() == Some("http-code-filter"));
    field.focus();
    settle(&mut harness);
    harness
        .get_by(|node| node.author_id() == Some("http-code-filter"))
        .type_text("axios");
    settle(&mut harness);

    assert_eq!(names(&harness, "http-code-target"), ["JavaScript · axios"]);
}

#[test]
fn secrets_are_written_in_clear_only_after_a_yes() {
    let mut harness = dialog();
    click_id(&mut harness, "http-code-secrets");
    assert!(has(&harness, "http-code-secrets-ask"));
    assert!(!harness.state().state.http.code().options().include_secrets);

    rest(&mut harness);
    click_id(&mut harness, "http-code-secrets-cancel");
    assert!(!has(&harness, "http-code-secrets-ask"));

    click_id(&mut harness, "http-code-secrets");
    rest(&mut harness);
    click_id(&mut harness, "http-code-secrets-yes");
    assert!(harness.state().state.http.code().options().include_secrets);
    until(&mut harness, "the note says so", |harness| {
        has(harness, "http-code-note")
    });

    harness
        .state_mut()
        .state
        .http
        .ask_code(Asked::ToggleSecrets);
    settle(&mut harness);
    assert!(!harness.state().state.http.code().options().include_secrets);
    assert!(!has(&harness, "http-code-secrets-ask"), "off never asks");
}

/// A draft begun from the empty workspace, some typing in it.
fn draft(harness: &mut Harness<'_, Bench>) {
    click_id(harness, "http-tab-new");
    until(harness, "the draft is open", |harness| {
        has(harness, "http-url")
    });
    let tabs = harness.state_mut().state.http.tabs_mut();
    let key = tabs.active().unwrap().key.clone();
    tabs.edit(&key, |draft| {
        draft.apply(DraftEdit::Url("https://api.example.com/first".to_owned()));
    });
    settle(harness);
}

fn type_field(harness: &mut Harness<'_, Bench>, author_id: &str, words: &str) {
    let field = harness.get_by(|node| node.author_id() == Some(author_id));
    field.focus();
    settle(harness);
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .type_text(words);
    settle(harness);
}

#[test]
fn a_draft_s_first_save_makes_the_collection_it_lacks() {
    let (backend, profile) = testing::unlocked();
    let mut harness = area_of(Area::Http, backend, profile);
    draft(&mut harness);

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    settle(&mut harness);
    assert!(has(&harness, "http-save-dialog"));
    assert!(!has(&harness, "http-save-place"), "nothing to choose");

    click_id(&mut harness, "http-save-submit");
    assert!(has(&harness, "http-save-missing"));

    type_field(&mut harness, "http-save-collection", "API");
    click_id(&mut harness, "http-save-submit");
    until(&mut harness, "the request is in the rail", |harness| {
        names(harness, "http-node-name").len() == 2
    });
    assert!(!has(&harness, "http-save-dialog"));
    let tab = harness.state().state.http.tabs().active().unwrap();
    assert_eq!(tab.request_id.as_deref(), Some(tab.key.as_str()));
    assert!(!tab.is_dirty());
    assert!(tab.place.is_some());
}

#[test]
fn a_draft_is_saved_in_a_place_of_the_tree_and_in_a_folder_it_names() {
    let mut harness = paiements();
    draft(&mut harness);
    click_id(&mut harness, "http-save");
    assert!(has(&harness, "http-save-place"));

    click_id(&mut harness, "http-save-new-folder");
    click_id(&mut harness, "http-save-submit");
    assert!(has(&harness, "http-save-missing"));
    type_field(&mut harness, "http-save-folder", "Neuf");
    click_id(&mut harness, "http-save-submit");

    until(&mut harness, "the folder is in the rail", |harness| {
        names(harness, "http-node-name").contains(&"Neuf".to_owned())
    });
    assert!(
        harness
            .state()
            .state
            .http
            .tabs()
            .active()
            .unwrap()
            .place
            .is_some()
    );
}

#[test]
fn closing_a_draft_can_save_it_first_and_the_tab_goes_once_it_is_written() {
    let mut harness = paiements();
    draft(&mut harness);
    let key = active_key(&harness);

    harness.state_mut().state.http.tabs_mut().ask_close(&key);
    settle(&mut harness);
    click_id(&mut harness, "http-close-save");
    assert!(has(&harness, "http-save-dialog"));

    click_id(&mut harness, "http-save-submit");
    until(&mut harness, "the tab has gone", |harness| {
        !has(harness, "http-url") || active(harness).name != "Untitled"
    });
    assert!(!has(&harness, "http-save-dialog"));
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
