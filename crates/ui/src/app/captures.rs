//! The window in each state the title bar, the gate and the banners can be in, both themes, drawn
//! on a software adapter (WARP, lavapipe) and written to `DEVNOTES_CAPTURES` when it is set, for
//! a look by eye beside the reference's captures. Nothing here compares pixels.

use std::path::Path;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::DevNotesApp;
use super::tests::startup;
use crate::services::time::fake::FakeClock;
use crate::testing::PASSPHRASE;

const SIZE: egui::Vec2 = egui::vec2(1100.0, 720.0);

fn launch(profile: &Path) -> Harness<'static, DevNotesApp> {
    let startup = startup(profile);
    let mut harness = Harness::builder()
        .with_size(SIZE)
        .build_eframe(|cc| DevNotesApp::new(cc, startup).unwrap());
    harness.state_mut().state.clock = Box::new(FakeClock::at("2026-11-03T09:30:00Z"));
    settle(&mut harness);
    harness
}

fn settle(harness: &mut Harness<'_, DevNotesApp>) {
    harness.state().backend.settle();
    harness.run();
}

fn click(harness: &mut Harness<'_, DevNotesApp>, author_id: &str) {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .click();
    harness.run();
}

fn type_into(harness: &mut Harness<'_, DevNotesApp>, author_id: &str, words: &str) {
    click(harness, author_id);
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .type_text(words);
    harness.run();
}

/// Draws the window as it stands, then again in the other theme, and gives the first one back.
fn capture(harness: &mut Harness<'_, DevNotesApp>, name: &str) {
    let directory = std::env::var_os("DEVNOTES_CAPTURES");
    for _ in 0..2 {
        let theme = match harness.ctx.theme() {
            egui::Theme::Dark => "dark",
            egui::Theme::Light => "light",
        };
        // kittest draws its pointer, and where a click left it.
        harness.event(egui::Event::PointerGone);
        harness.run();
        let image = harness
            .render()
            .expect("a software adapter draws the window");
        if let Some(directory) = &directory {
            std::fs::create_dir_all(directory).unwrap();
            image
                .save(Path::new(directory).join(format!("{name}-{theme}.png")))
                .unwrap();
        }
        click(harness, "theme-toggle");
    }
}

#[test]
fn the_gate_in_each_of_its_states() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    capture(&mut harness, "gate-create");

    type_into(&mut harness, "vault-passphrase", "too short");
    type_into(&mut harness, "vault-confirmation", "too shorts");
    capture(&mut harness, "gate-create-refused");
    drop(harness);

    // A later launch over the library the first one created.
    let mut creating = launch(profile.path());
    type_into(&mut creating, "vault-passphrase", PASSPHRASE);
    type_into(&mut creating, "vault-confirmation", PASSPHRASE);
    click(&mut creating, "vault-submit");
    settle(&mut creating);
    crate::testing::close_library(&creating.state().backend);
    drop(creating);

    let mut harness = launch(profile.path());
    capture(&mut harness, "gate-unlock");

    type_into(&mut harness, "vault-passphrase", "not the phrase");
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    capture(&mut harness, "gate-unlock-wrong");

    click(&mut harness, "vault-forgotten");
    capture(&mut harness, "gate-forgotten");
}

#[test]
fn the_title_bar_and_the_banners_over_an_open_library() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    capture(&mut harness, "titlebar-notes");

    click(&mut harness, "area-option-http");
    let state = &mut harness.state_mut().state;
    state.errors.report(
        i18n::keys::errors::ATTACHMENTS_LOAD_FAILED,
        &engine::error::AppError::from(engine::error::StorageError::Locked),
    );
    state
        .status
        .report(i18n::Message::new(i18n::keys::settings::COPIED));
    harness.run();
    capture(&mut harness, "banners-http");
}
