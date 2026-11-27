//! An update found while the passphrase is asked: said on the unlock card, never a dialog
//! taking the keys from the field.

use std::path::Path;
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::DevNotesApp;
use super::tests::{click, open_library, present, settle, startup};
use crate::services::updates::UpdateState;
use crate::services::updates::fake::FakeUpdater;

/// A library made, then the window launched again over it: the unlock card, the launch's
/// check answered by `fake` while it is shown.
fn at_the_gate(directory: &Path, fake: Arc<FakeUpdater>) -> Harness<'static, DevNotesApp> {
    let first = open_library(directory);
    crate::testing::close_library(first.state().backend());
    drop(first);
    let startup = startup(directory);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 720.0))
        .build_eframe(|cc| DevNotesApp::new(cc, startup).unwrap());
    harness
        .ctx
        .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
    settle(&mut harness);
    harness.state_mut().state.updates = UpdateState::with(fake);
    let app = harness.state_mut();
    app.state.updates.check(&app.backend, true);
    settle(&mut harness);
    harness
}

fn focused(harness: &Harness<'_, DevNotesApp>) -> Option<String> {
    let node = harness.query_by(|node| node.is_focused() && node.author_id().is_some())?;
    node.accesskit_node().author_id().map(str::to_owned)
}

#[test]
fn an_update_found_at_the_gate_is_said_on_the_card_and_the_field_keeps_the_keys() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = at_the_gate(directory.path(), FakeUpdater::offering("0.26.0"));

    assert!(!harness.state().state().gate.is_open());
    assert!(
        !present(&harness, "update-prompt"),
        "no dialog over the field"
    );
    assert!(present(&harness, "gate-update"));
    assert_eq!(focused(&harness).as_deref(), Some("vault-passphrase"));

    harness
        .get_by(|node| node.author_id() == Some("vault-passphrase"))
        .type_text(crate::testing::PASSPHRASE);
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);

    assert!(
        harness.state().state().gate.is_open(),
        "Enter still unlocks"
    );
    assert!(!present(&harness, "update-prompt"), "nor once unlocked");
    click(&mut harness, "about-open");
    let entry = harness.get_by(|node| node.author_id() == Some("about-check-updates"));
    let label = entry.accesskit_node().label().unwrap_or_default();
    assert!(label.contains("0.26.0"), "{label}");
}

#[test]
fn install_now_from_the_gate_downloads_the_update() {
    let directory = tempfile::tempdir().unwrap();
    let fake = FakeUpdater::offering("0.26.0");
    let mut harness = at_the_gate(directory.path(), Arc::clone(&fake));

    click(&mut harness, "gate-update-install");
    settle(&mut harness);

    assert!(
        harness.state().state().quitting,
        "downloaded, it leaves for the installer"
    );
    eframe::App::on_exit(harness.state_mut());
    assert!(*fake.relaunched.lock().unwrap());
}

#[test]
fn an_update_silenced_is_not_said_at_the_gate() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = at_the_gate(directory.path(), Arc::new(FakeUpdater::default()));
    harness.state_mut().state.settings.update_notifications = false;
    harness.state_mut().state.updates = UpdateState::with(FakeUpdater::offering("0.26.0"));
    let app = harness.state_mut();
    app.state.updates.check(&app.backend, true);
    settle(&mut harness);

    assert!(!present(&harness, "gate-update"));
    assert!(!present(&harness, "update-prompt"));
}
