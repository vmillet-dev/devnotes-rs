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
        .query_all_by(|node| node.is_focused())
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
