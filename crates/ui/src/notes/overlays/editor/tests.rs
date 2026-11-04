use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::kind::NoteKind;
use engine::notes::model::{DisplayNote, NotePatch};

use crate::notes::bench::{Bench, area, cards, click_id, settle};
use crate::testing;

fn stored(harness: &egui_kittest::Harness<'_, Bench>, id: &str) -> DisplayNote {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, id)
    })
    .unwrap()
}

fn written(harness: &egui_kittest::Harness<'_, Bench>) -> usize {
    harness
        .state()
        .state
        .notes_query
        .view()
        .unwrap()
        .cards()
        .count()
}

fn is_open(harness: &egui_kittest::Harness<'_, Bench>) -> bool {
    harness
        .query_by(|node| node.author_id() == Some("editor-title"))
        .is_some()
}

fn type_into(harness: &mut egui_kittest::Harness<'_, Bench>, id: &str, text: &str) {
    click_id(harness, id);
    harness
        .get_by(|node| node.author_id() == Some(id))
        .type_text(text);
    settle(harness);
}

#[test]
fn a_note_opened_edited_and_closed_shows_its_change_on_its_card() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql prod").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    type_into(&mut harness, "editor-body", "\nSELECT 2;");
    click_id(&mut harness, "editor-close");

    assert!(!is_open(&harness));
    assert_eq!(stored(&harness, &id).content, "SELECT 1;\nSELECT 2;");
    let snippet = harness
        .get_by(|node| node.author_id() == Some("note-card-snippet"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert!(snippet.contains("SELECT 2;"), "{snippet}");
}

#[test]
fn a_title_is_written_when_its_field_is_left_and_nothing_else_moves() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);
    let before = stored(&harness, &id).updated_at;

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-body");
    click_id(&mut harness, "editor-title");
    assert_eq!(stored(&harness, &id).updated_at, before, "nothing moved");
    harness
        .get_by(|node| node.author_id() == Some("editor-title"))
        .type_text(" prod");
    settle(&mut harness);
    click_id(&mut harness, "editor-body");

    assert_eq!(stored(&harness, &id).title, "psql prod");
    assert!(is_open(&harness));
}

#[test]
fn the_context_and_a_typed_tag_are_kept_on_close() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    type_into(&mut harness, "editor-source", "Ops / DB");
    type_into(&mut harness, "editor-tag-add", "postgres");
    click_id(&mut harness, "editor-close");

    let note = stored(&harness, &id);
    assert_eq!(note.source, "Ops / DB");
    assert_eq!(note.tags, ["postgres"]);
}

#[test]
fn a_list_is_edited_one_task_a_line() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "groceries").id.clone();
    let patch = NotePatch {
        kind: Some(NoteKind::Checklist),
        content: Some(String::new()),
        ..NotePatch::default()
    };
    let at = id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, at, patch)
    })
    .unwrap();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    type_into(&mut harness, "editor-body", "milk\n[x] bread");
    click_id(&mut harness, "editor-close");

    let items: Vec<_> = stored(&harness, &id)
        .items
        .iter()
        .map(|item| (item.text.clone(), item.done))
        .collect();
    assert_eq!(
        items,
        [("milk".to_string(), false), ("bread".to_string(), true)]
    );
}

#[test]
fn a_new_note_is_written_once_and_only_once_worth_keeping() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "new-note");
    assert_eq!(written(&harness), 1);
    click_id(&mut harness, "editor-close");
    assert_eq!(written(&harness), 1);

    click_id(&mut harness, "new-note");
    type_into(&mut harness, "editor-title", "Rotate the certificate");
    type_into(&mut harness, "editor-body", "openssl req -new");
    click_id(&mut harness, "editor-close");

    assert_eq!(written(&harness), 2);
    assert_eq!(cards(&harness), 2);
}

#[test]
fn pinning_a_draft_makes_it_worth_keeping() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "new-note");
    click_id(&mut harness, "editor-pin");

    assert_eq!(written(&harness), 2);
    let opened = harness.state().state.editor.opened().unwrap();
    assert!(!opened.is_draft());
    assert!(opened.note().unwrap().pinned);
    click_id(&mut harness, "editor-close");
    assert_eq!(written(&harness), 2);
}

#[test]
fn pinning_a_note_shows_at_once_and_offers_the_undo() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-pin");

    assert!(stored(&harness, &id).pinned);
    let opened = harness.state().state.editor.opened().unwrap();
    assert!(opened.note().unwrap().pinned);
    harness.get_by(|node| node.author_id() == Some("undo-restore"));
}

#[test]
fn deleting_asks_twice_then_closes_and_offers_the_undo() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-delete");
    assert!(is_open(&harness));
    assert_eq!(written(&harness), 1);
    click_id(&mut harness, "editor-delete");

    assert!(!is_open(&harness));
    assert_eq!(written(&harness), 0);
    harness.get_by(|node| node.author_id() == Some("undo-restore"));
}

#[test]
fn escape_leaves_the_field_first_then_closes() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    type_into(&mut harness, "editor-body", "\n-- done");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(is_open(&harness));
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);

    assert!(!is_open(&harness));
    assert_eq!(stored(&harness, &id).content, "SELECT 1;\n-- done");
}

#[test]
fn folding_the_header_leaves_its_summary_and_is_remembered() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-fold-header");

    assert!(
        harness
            .query_by(|node| node.author_id() == Some("editor-source"))
            .is_none()
    );
    harness.get_by(|node| node.author_id() == Some("editor-header-summary"));
    assert!(harness.state().state.settings.editor_header_folded);
    click_id(&mut harness, "editor-header-summary");
    harness.get_by(|node| node.author_id() == Some("editor-source"));
}

#[test]
fn converting_rewrites_the_body_and_the_undo_puts_it_back() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "steps").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    let session = harness.state().state.editor.opened().unwrap().session;
    click_id(&mut harness, "editor-kind");
    harness.get_by_label("Convert to a todo list").click();
    settle(&mut harness);

    assert_eq!(stored(&harness, &id).kind, NoteKind::Checklist);
    let opened = harness.state().state.editor.opened().unwrap();
    assert_ne!(opened.session, session, "the fields start again");
    click_id(&mut harness, "undo-restore");
    assert_eq!(stored(&harness, &id).kind, NoteKind::Snippet);
    assert_eq!(stored(&harness, &id).content, "SELECT 1;");
}
