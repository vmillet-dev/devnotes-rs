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
fn with_no_updater_the_menu_offers_no_check() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    click(&mut harness, "about-open");

    assert!(present(&harness, "about-whats-new"));
    assert!(!present(&harness, "about-check-updates"));
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

    click(&mut harness, "update-later");

    assert!(!present(&harness, "update-prompt"));
    assert!(present(&harness, "update-dot"));
    assert_eq!(stored(directory.path()).skipped_update, "");
}

#[test]
fn a_ticked_box_writes_the_version_down_whichever_way_out() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = offered(directory.path(), FakeUpdater::offering("0.26.0"));

    click(&mut harness, "update-skip");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);

    assert!(!present(&harness, "update-prompt"));
    assert_eq!(stored(directory.path()).skipped_update, "0.26.0");
}

#[test]
fn installing_relaunches_and_cannot_be_dismissed() {
    let directory = tempfile::tempdir().unwrap();
    let fake = FakeUpdater::offering("0.26.0");
    let mut harness = offered(directory.path(), Arc::clone(&fake));

    click(&mut harness, "update-install");
    settle(&mut harness);

    assert!(*fake.relaunched.lock().unwrap());
    assert!(present(&harness, "update-status"));
    assert!(!present(&harness, "update-later"));
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(present(&harness, "update-prompt"));
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
    assert!(*fake.relaunched.lock().unwrap());
}
