//! The offer over the whole window, with a faked updater.

use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

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

/// The launch's check, silent, over the library open: then the menu.
fn launched(directory: &std::path::Path, fake: Arc<FakeUpdater>) -> Harness<'static, DevNotesApp> {
    let mut harness = open_library(directory);
    harness.state_mut().state.updates = UpdateState::with(fake);
    let app = harness.state_mut();
    app.state.updates.check(&app.backend, true);
    settle(&mut harness);
    harness
}

fn rect(harness: &Harness<'_, DevNotesApp>, author_id: &str) -> egui::Rect {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .rect()
}

#[test]
fn the_menu_says_nothing_of_a_check_it_did_not_ask_and_forgets_one_it_did() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = launched(directory.path(), Arc::new(FakeUpdater::default()));

    click(&mut harness, "about-open");
    assert!(
        !present(&harness, "about-check-status"),
        "nothing was asked"
    );

    click(&mut harness, "about-check-updates");
    settle(&mut harness);
    assert!(present(&harness, "about-check-status"));

    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    click(&mut harness, "about-open");
    assert!(
        !present(&harness, "about-check-status"),
        "what was found then is not said again"
    );
}

#[test]
fn an_update_known_is_named_by_its_entry_drawn_clear_of_the_next() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = launched(directory.path(), FakeUpdater::offering("0.26.0"));
    click(&mut harness, "update-later");

    click(&mut harness, "about-open");
    let entry = harness.get_by(|node| node.author_id() == Some("about-check-updates"));
    let label = entry.accesskit_node().label().unwrap_or_default();
    assert!(label.contains("0.26.0"), "{label}");
    let (entry, next) = (
        rect(&harness, "about-check-updates"),
        rect(&harness, "about-whats-new"),
    );
    // The separator between them is 7 points tall.
    assert!(next.top() >= entry.bottom() + 6.0, "{entry:?} {next:?}");

    click(&mut harness, "about-check-updates");
    assert!(present(&harness, "update-prompt"), "the offer reopened");
}

#[test]
fn an_update_found_with_the_library_open_still_opens_its_dialog() {
    let directory = tempfile::tempdir().unwrap();
    let harness = launched(directory.path(), FakeUpdater::offering("0.26.0"));
    assert!(present(&harness, "update-prompt"));
}
