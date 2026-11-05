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
    // Loaded before the first frame, as the window has them a moment after: the captures show
    // the colours.
    text::highlight::warm();
    let startup = startup(profile);
    let mut harness = Harness::builder()
        .with_size(SIZE)
        .build_eframe(|cc| DevNotesApp::new(cc, startup).unwrap());
    harness.state_mut().state.clock = Box::new(FakeClock::at("2026-11-03T09:30:00Z"));
    settle(&mut harness);
    harness
}

fn settle(harness: &mut Harness<'_, DevNotesApp>) {
    super::tests::settle(harness);
}

fn click(harness: &mut Harness<'_, DevNotesApp>, author_id: &str) {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .click();
    settle(harness);
}

fn type_into(harness: &mut Harness<'_, DevNotesApp>, author_id: &str, words: &str) {
    click(harness, author_id);
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .type_text(words);
    settle(harness);
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
        settle(harness);
        let image = harness
            .render()
            .expect("a software adapter draws the window");
        if let Some(directory) = &directory {
            std::fs::create_dir_all(directory).unwrap();
            image
                .save(Path::new(directory).join(format!("{name}-{theme}.png")))
                .unwrap();
        }
        // By its action, not its button: a click would close what the capture shows.
        let other = match harness.ctx.theme() {
            egui::Theme::Dark => crate::theme::Appearance::Light,
            egui::Theme::Light => crate::theme::Appearance::Dark,
        };
        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        let action = crate::action::Action::App(crate::action::AppAction::SetAppearance(other));
        app.state.apply(action, &ctx, &app.backend);
        settle(harness);
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
    settle(&mut harness);
    capture(&mut harness, "banners-http");
}

#[test]
fn the_canvas_of_an_open_library() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    capture(&mut harness, "canvas-samples");

    kinds(&harness.state().backend);
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    capture(&mut harness, "canvas-notes");

    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open")
                && node.label().as_deref() == Some("psql prod")
        })
        .hover();
    settle(&mut harness);
    click(&mut harness, "note-card-menu");
    capture(&mut harness, "card-menu");
    click(&mut harness, "note-card-menu");
    click(&mut harness, "note-card-fill");
    capture(&mut harness, "placeholder-form");
    click(&mut harness, "placeholder-cancel");

    for _ in 0..2 {
        harness.key_press(egui::Key::ArrowRight);
        harness.key_press(egui::Key::X);
    }
    settle(&mut harness);
    capture(&mut harness, "selection-bar");
    click(&mut harness, "selection-pin");
    capture(&mut harness, "undo-bar");
    click(&mut harness, "undo-restore");
    click(&mut harness, "selection-delete");
    click(&mut harness, "selection-delete");
    click(&mut harness, "trash-open");
    capture(&mut harness, "trash-panel");
    click(&mut harness, "trash-restore-all");
    click(&mut harness, "trash-close");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);

    click(&mut harness, "sort-trigger");
    capture(&mut harness, "header-sort-menu");
    click(&mut harness, "sort-trigger");

    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open")
                && node.label().as_deref() == Some("psql prod")
        })
        .click();
    settle(&mut harness);
    capture(&mut harness, "editor-snippet");
    click(&mut harness, "editor-fold-header");
    capture(&mut harness, "editor-header-folded");
    click(&mut harness, "editor-header-summary");

    click(&mut harness, "editor-close");
    drawing_editor(&mut harness);
    harness.key_press_modifiers(
        egui::Modifiers::COMMAND | egui::Modifiers::ALT,
        egui::Key::P,
    );
    settle(&mut harness);
    capture(&mut harness, "palette-recent");
    harness
        .get_by(|node| node.author_id() == Some("palette-input"))
        .type_text("psql");
    // Past the search's pause, in egui's time.
    for _ in 0..20 {
        harness.step();
    }
    settle(&mut harness);
    capture(&mut harness, "palette-search");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    click(&mut harness, "new-note-kind");
    capture(&mut harness, "header-new-note-menu");
    click(&mut harness, "new-note-kind");
    click(&mut harness, "library-rail-toggle");
    click(&mut harness, "space-switcher");
    capture(&mut harness, "header-switchers");
    click(&mut harness, "space-switcher");
    click(&mut harness, "facets-toggle");
    capture(&mut harness, "facets-panel");
    click(&mut harness, "tag-manage");
    harness
        .get_all_by(|node| node.author_id() == Some("tag-item"))
        .next()
        .expect("the samples are tagged")
        .click();
    settle(&mut harness);
    capture(&mut harness, "tag-manager");
    click(&mut harness, "tag-manager-close");
    click(&mut harness, "facets-toggle");
    harness.set_size(egui::vec2(640.0, SIZE.y));
    settle(&mut harness);
    capture(&mut harness, "header-two-rows");
    harness.set_size(SIZE);
}

#[test]
fn the_board_of_the_samples() {
    let profile = tempfile::tempdir().unwrap();
    let mut harness = launch(profile.path());
    type_into(&mut harness, "vault-passphrase", PASSPHRASE);
    type_into(&mut harness, "vault-confirmation", PASSPHRASE);
    click(&mut harness, "vault-submit");
    settle(&mut harness);
    harness
        .get_all_by(|node| node.author_id() == Some("space-option"))
        .next()
        .expect("the samples have a space")
        .click();
    settle(&mut harness);
    click(&mut harness, "view-board");
    capture(&mut harness, "board");
    harness
        .get_all_by(|node| node.author_id() == Some("board-zone-menu"))
        .next()
        .expect("the samples are filed")
        .click();
    settle(&mut harness);
    capture(&mut harness, "board-zone-menu");
}

/// The Dessin of `kinds`, as it opens and with a shape selected.
fn drawing_editor(harness: &mut Harness<'_, DevNotesApp>) {
    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open") && node.label().as_deref() == Some("Sketch")
        })
        .click();
    settle(harness);
    capture(harness, "editor-drawing");
    let stage = harness
        .get_by(|node| node.author_id() == Some("drawing-stage"))
        .rect();
    let shape = stage.min + egui::vec2(170.0, 90.0);
    harness.event(egui::Event::PointerMoved(shape));
    harness.run_steps(1);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: shape,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run_steps(1);
    }
    settle(harness);
    capture(harness, "editor-drawing-selected");
    click(harness, "editor-close");
}

/// One note of each kind, with marks, tags and a search's worth of words.
fn kinds(backend: &crate::backend::Backend) {
    use engine::notes::checklist::ChecklistItem;
    use engine::notes::kind::NoteKind;
    use engine::notes::model::NotePatch;
    let scenes: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../golden/fabric/cases.json")).unwrap();
    let scene = scenes[0]["output"].as_str().unwrap().to_owned();
    let shapes = [
        (
            "psql prod",
            NoteKind::Snippet,
            "psql -h {{host}} -U admin\nSELECT count(*) FROM users;",
            vec!["db", "prod", "ops"],
            Vec::new(),
        ),
        (
            "Deploy checklist",
            NoteKind::Note,
            "# Release\n\nShip on Friday.\n\n- build\n- [x] tag\n- [ ] announce",
            vec!["release"],
            Vec::new(),
        ),
        (
            "Groceries",
            NoteKind::Checklist,
            "",
            Vec::new(),
            vec![("milk", true), ("bread", false), ("eggs", false)],
        ),
        (
            "Budget",
            NoteKind::Calc,
            "loyer = 800 €\ncharges = 120,5 €\ntotal",
            Vec::new(),
            Vec::new(),
        ),
        (
            "Sketch",
            NoteKind::Drawing,
            scene.as_str(),
            Vec::new(),
            Vec::new(),
        ),
        (
            "",
            NoteKind::Snippet,
            "curl -s https://example.test/health",
            Vec::new(),
            Vec::new(),
        ),
    ];
    for (title, kind, content, tags, items) in shapes {
        let id = crate::testing::note(backend, "seed").id.clone();
        let patch = NotePatch {
            title: Some(title.to_string()),
            kind: Some(kind),
            content: Some(content.to_string()),
            tags: Some(tags.into_iter().map(String::from).collect()),
            items: Some(
                items
                    .into_iter()
                    .map(|(text, done)| ChecklistItem {
                        text: text.to_string(),
                        done,
                    })
                    .collect(),
            ),
            pinned: Some(title == "psql prod"),
            ..NotePatch::default()
        };
        let mut written = backend.db(move |engine| engine::notes::update_note(engine, id, patch));
        backend.settle();
        written.poll().unwrap().unwrap();
    }
}
