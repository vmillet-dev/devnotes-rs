//! The offer over the whole window, with a faked updater.

use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::DevNotesApp;
use super::tests::{click, open_library, present, settle};
use crate::services::settings::Settings;
use crate::services::updates::UpdateState;
use crate::services::updates::fake::FakeUpdater;

fn offered(directory: &std::path::Path, fake: Arc<FakeUpdater>) -> Harness<'static, DevNotesApp> {
    let mut harness = open_library(directory);
    harness.state_mut().state.updates = UpdateState::with(fake);
    click(&mut harness, "about-open");
    click(&mut harness, "about-check-updates");
    settle(&mut harness);
    harness
}

fn stored(directory: &std::path::Path) -> Settings {
    let file = directory.join(engine::layout::PREFERENCES);
    Settings::read(&crate::services::preferences::Preferences::read(&file))
}

#[test]
fn a_copy_that_does_not_install_itself_lists_the_check_and_names_a_newer_version() {
    let directory = tempfile::tempdir().unwrap();
    let fake = FakeUpdater::offering("0.26.0");
    *fake.installs.lock().unwrap() = false;
    let harness = offered(directory.path(), fake);

    assert!(present(&harness, "about-whats-new"), "the menu stays open");
    assert!(
        !present(&harness, "update-prompt"),
        "nothing to install from here"
    );
    assert!(harness.query_by_label_contains("0.26.0").is_some());
}

#[test]
fn with_no_updater_the_check_is_still_listed() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    click(&mut harness, "about-open");

    assert!(present(&harness, "about-check-updates"));
}

#[test]
fn the_check_says_when_there_is_nothing_new() {
    let directory = tempfile::tempdir().unwrap();
    let harness = offered(directory.path(), Arc::new(FakeUpdater::default()));

    assert!(present(&harness, "about-check-status"));
    assert!(!present(&harness, "update-prompt"));
}

#[test]
fn a_new_version_shows_its_notes_and_later_leaves_the_dot() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = offered(directory.path(), FakeUpdater::offering("0.26.0"));

    assert!(present(&harness, "update-prompt"));
    assert!(harness.query_by_label_contains("Faster").is_some());
    assert!(!present(&harness, "about-whats-new"), "the menu went");

    click(&mut harness, "update-later");

    assert!(!present(&harness, "update-prompt"));
    assert!(present(&harness, "update-dot"));
    assert_eq!(stored(directory.path()).skipped_update, "");
}

#[test]
fn a_ticked_box_writes_the_version_down_whichever_way_out() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = offered(directory.path(), FakeUpdater::offering("0.26.0"));

    click(&mut harness, "update-skip-label");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);

    assert!(!present(&harness, "update-prompt"));
    assert_eq!(stored(directory.path()).skipped_update, "0.26.0");
}

#[test]
fn installing_leaves_first_and_cannot_be_dismissed() {
    let directory = tempfile::tempdir().unwrap();
    let fake = FakeUpdater::offering("0.26.0");
    let mut harness = offered(directory.path(), Arc::clone(&fake));

    click(&mut harness, "update-install");
    settle(&mut harness);

    assert!(harness.state().state().quitting);
    assert!(
        !*fake.relaunched.lock().unwrap(),
        "after the exit, not before"
    );
    assert!(present(&harness, "update-status"));
    assert!(!present(&harness, "update-later"));
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(present(&harness, "update-prompt"));

    eframe::App::on_exit(harness.state_mut());
    assert!(*fake.relaunched.lock().unwrap());
}

#[test]
fn an_unsaved_request_is_asked_about_before_the_install() {
    let directory = tempfile::tempdir().unwrap();
    let fake = FakeUpdater::offering("0.26.0");
    let mut harness = offered(directory.path(), Arc::clone(&fake));
    let kind = engine::http::model::RequestKind::Http;
    let tabs = harness.state_mut().state.http.tabs_mut();
    tabs.new_request(kind, "Sans titre".to_owned());
    settle(&mut harness);

    click(&mut harness, "update-install");

    assert!(present(&harness, "http-leave-dialog"));
    assert!(!*fake.relaunched.lock().unwrap());
    click(&mut harness, "http-leave-discard");
    settle(&mut harness);
    assert!(harness.state_mut().state.may_close(), "asked once");

    eframe::App::on_exit(harness.state_mut());
    assert!(*fake.relaunched.lock().unwrap());
}
