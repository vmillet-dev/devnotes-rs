//! The libraries panel over the whole window: what it offers, what a switch asks first, and the
//! gate it leaves for.

use egui_kittest::Harness;

use super::DevNotesApp;
use super::tests::{click, count, open_library, present, settle, type_into};

fn libraries_panel(harness: &mut Harness<'_, DevNotesApp>) {
    click(harness, "file-menu");
    click(harness, "file-libraries");
}

fn first_launch_gate(harness: &mut Harness<'_, DevNotesApp>) {
    type_into(harness, "vault-passphrase", crate::testing::PASSPHRASE);
    type_into(harness, "vault-confirmation", crate::testing::PASSPHRASE);
    click(harness, "vault-submit");
    settle(harness);
}

/// A second library, made from the panel and passed through the gate.
fn with_two_libraries(directory: &std::path::Path) -> Harness<'static, DevNotesApp> {
    let mut harness = open_library(directory);
    libraries_panel(&mut harness);
    type_into(&mut harness, "library-new-name", "Boulot");
    click(&mut harness, "library-create");
    settle(&mut harness);
    first_launch_gate(&mut harness);
    libraries_panel(&mut harness);
    harness
}

#[test]
fn the_open_library_is_said_and_the_last_cannot_go() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    libraries_panel(&mut harness);

    assert!(present(&harness, "library-open"));
    assert!(!present(&harness, "library-delete"));
    assert!(!present(&harness, "library-switch"));
    click(&mut harness, "libraries-close");
    assert!(!present(&harness, "library-row"));
}

#[test]
fn creating_a_library_leaves_for_its_gate_and_takes_the_panel_with_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    libraries_panel(&mut harness);
    assert!(!present(&harness, "vault-passphrase"));

    type_into(&mut harness, "library-new-name", "Boulot");
    click(&mut harness, "library-create");
    settle(&mut harness);

    assert!(present(&harness, "vault-confirmation"), "asked twice");
    assert!(!present(&harness, "library-row"));
    assert!(!present(&harness, "file-menu"));
}

#[test]
fn a_library_is_renamed_where_it_is_listed() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    libraries_panel(&mut harness);

    click(&mut harness, "library-start-rename");
    assert!(present(&harness, "library-rename"));
    type_into(&mut harness, "library-rename", " Maison");
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);

    assert!(!present(&harness, "library-rename"));
    let libraries = harness.state().state().libraries.libraries();
    assert!(
        libraries[0].name.contains("Maison"),
        "{}",
        libraries[0].name
    );
}

#[test]
fn escape_takes_a_rename_back_before_it_closes_the_panel() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    libraries_panel(&mut harness);
    click(&mut harness, "library-start-rename");

    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(!present(&harness, "library-rename"));
    assert!(present(&harness, "library-row"));

    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(!present(&harness, "library-row"));
}

#[test]
fn a_deletion_is_named_can_be_taken_back_and_then_erases() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = with_two_libraries(directory.path());
    assert_eq!(count(&harness, "library-row"), 2);

    click(&mut harness, "library-delete");
    assert!(present(&harness, "library-confirm"));
    click(&mut harness, "library-cancel-delete");
    assert!(!present(&harness, "library-confirm"));

    click(&mut harness, "library-delete");
    click(&mut harness, "library-confirm-delete");
    settle(&mut harness);

    assert_eq!(count(&harness, "library-row"), 1);
    assert!(harness.state().state().status.latest().is_some());
}

#[test]
fn a_switch_asks_about_a_request_left_unsaved_and_cancelling_stays() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = with_two_libraries(directory.path());
    let kind = engine::http::model::RequestKind::Http;
    let tabs = harness.state_mut().state.http.tabs_mut();
    tabs.new_request(kind, "Sans titre".to_owned());
    settle(&mut harness);

    click(&mut harness, "library-switch");
    assert!(present(&harness, "http-leave-dialog"));
    click(&mut harness, "http-leave-cancel");
    assert!(!present(&harness, "http-leave-dialog"));
    assert!(!present(&harness, "vault-passphrase"), "nothing changed");

    click(&mut harness, "library-switch");
    click(&mut harness, "http-leave-discard");
    settle(&mut harness);
    assert!(present(&harness, "vault-passphrase"));
    assert!(
        !present(&harness, "vault-confirmation"),
        "it has its phrase"
    );
}
