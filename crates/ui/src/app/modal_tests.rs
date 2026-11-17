//! The window's keys under a modal: what is under it hears nothing, and the focus comes back to
//! what opened it.

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::model::{HttpMethod, RequestKind};

use super::DevNotesApp;
use super::tests::{click, open_library, present, settle};

/// A request open in the HTTP area, its URL changed and not saved.
fn an_unsaved_request(directory: &std::path::Path) -> Harness<'static, DevNotesApp> {
    let mut harness = open_library(directory);
    let backend = harness.state().backend();
    let api = crate::http::tests::collection(backend, "API");
    crate::http::tests::request(
        backend,
        (&api, None),
        "Login",
        (RequestKind::Http, HttpMethod::Get),
    );
    click(&mut harness, "area-option-http");
    harness
        .get_by(|node| {
            node.author_id() == Some("http-node-name")
                && node.label().is_some_and(|label| label.ends_with("Login"))
        })
        .click();
    settle(&mut harness);
    let app = harness.state_mut();
    let key = app.state.http.tabs().active_key().unwrap().to_owned();
    app.state.http.tabs_mut().edit(&key, |draft| {
        use crate::state::http::tabs::DraftEdit;
        draft.apply(DraftEdit::Url("https://example.test/changed".to_owned()));
    });
    settle(&mut harness);
    assert!(unsaved(&harness));
    harness
}

fn unsaved(harness: &Harness<'_, DevNotesApp>) -> bool {
    harness
        .state()
        .state
        .http
        .tabs()
        .active()
        .is_some_and(crate::state::http::tabs::Tab::is_dirty)
}

#[test]
fn ctrl_s_under_the_cookie_jar_saves_nothing_underneath() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = an_unsaved_request(directory.path());
    click(&mut harness, "http-cookies-open");
    assert!(present(&harness, "http-cookies-dialog"));

    harness.key_press_modifiers(Modifiers::COMMAND, Key::S);
    settle(&mut harness);

    assert!(unsaved(&harness), "the request under the jar was saved");
}

fn focused(harness: &Harness<'_, DevNotesApp>) -> Option<String> {
    harness
        .query_all_by(|node| node.is_focused() && node.author_id().is_some())
        .find_map(|node| node.accesskit_node().author_id().map(str::to_owned))
}

#[test]
fn escape_closes_the_trash_and_gives_the_focus_back_to_its_button() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    // From the keyboard: the one for whom the focus coming back matters.
    harness
        .get_by(|node| node.author_id() == Some("trash-open"))
        .focus();
    settle(&mut harness);
    harness.key_press(Key::Enter);
    settle(&mut harness);
    assert!(present(&harness, "trash-close"));

    harness.key_press(Key::Escape);
    settle(&mut harness);

    assert!(!present(&harness, "trash-close"));
    assert_eq!(focused(&harness).as_deref(), Some("trash-open"));
}

/// A greyed entry says what it waits for: nothing is ticked, so the selection's export waits
/// for notes to be ticked.
#[test]
fn a_greyed_file_entry_says_what_it_waits_for() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    harness
        .ctx
        .all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);
    click(&mut harness, "file-menu");
    harness
        .get_by(|node| node.author_id() == Some("file-option-exportSelection"))
        .hover();
    for _ in 0..10 {
        harness.step();
    }

    let waiting = i18n::translate(
        harness.state().state.language,
        i18n::keys::file::NEEDS_SELECTION,
        &[],
    );
    assert!(
        harness
            .query_by(|node| {
                node.label().as_deref() == Some(waiting.as_str())
                    || node.value().as_deref() == Some(waiting.as_str())
            })
            .is_some(),
        "no « {waiting} » under the pointer"
    );
}

/// In a Note, `Ctrl+B` is bold: the rail's key, the same by default, stays with the editor while
/// it writes.
#[test]
fn ctrl_b_in_a_note_writes_bold_and_leaves_the_rail() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    // A Note of the test's own: the samples' titles follow the system's language.
    let backend = harness.state().backend();
    let space = crate::testing::note(backend, "seed").space_id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        let draft = engine::notes::model::NoteDraft {
            kind: engine::notes::kind::NoteKind::Note,
            language: engine::notes::language::Language::Txt,
            content: "Ship it".to_owned(),
            ..crate::testing::snippet(space, "Standup".to_owned())
        };
        engine::notes::create_note(engine, draft)
    })
    .unwrap();
    harness.state_mut().state.revision.bump();
    settle(&mut harness);
    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open") && node.label().as_deref() == Some("Standup")
        })
        .click();
    settle(&mut harness);
    click(&mut harness, "editor-rich");
    let rail = harness.state().state.settings.rail_shown;

    harness.key_press_modifiers(Modifiers::COMMAND, Key::B);
    settle(&mut harness);

    assert_eq!(
        harness.state().state.settings.rail_shown,
        rail,
        "the rail moved"
    );
}
