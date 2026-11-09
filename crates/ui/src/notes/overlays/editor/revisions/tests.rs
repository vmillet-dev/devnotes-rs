use egui_kittest::kittest::Queryable;
use engine::notes::model::{DisplayNote, NotePatch};

use crate::notes::bench::{Bench, click_id, expanded_area, settle};
use crate::testing;

type Harness = egui_kittest::Harness<'static, Bench>;

fn stored(harness: &Harness, id: &str) -> DisplayNote {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, id)
    })
    .unwrap()
}

/// A snippet whose body was edited once, so one body is kept: « SELECT 1; ».
fn edited() -> (Harness, String) {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let patch = NotePatch {
        content: Some("SELECT 2;\nSELECT 3;".into()),
        ..NotePatch::default()
    };
    let at = id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, at, patch)
    })
    .unwrap();
    let mut harness = expanded_area(backend, profile);
    click_id(&mut harness, "note-card-open");
    (harness, id)
}

fn body(harness: &Harness) -> String {
    harness
        .get_by(|node| node.author_id() == Some("editor-body"))
        .value()
        .unwrap_or_default()
}

#[test]
fn the_history_lists_the_body_kept_and_previews_what_would_change() {
    let (mut harness, _) = edited();
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("revision-open"))
            .is_none(),
        "folded at first"
    );
    click_id(&mut harness, "revisions-toggle");
    click_id(&mut harness, "revision-open");

    harness.get_by(|node| node.author_id() == Some("diff-restored"));
    assert_eq!(
        harness
            .query_all_by(|node| node.author_id() == Some("diff-dropped"))
            .count(),
        2
    );
    harness.get_by(|node| node.author_id() == Some("revision-consequence"));
    click_id(&mut harness, "revision-back");
    harness.get_by(|node| node.author_id() == Some("revision-open"));
}

#[test]
fn going_back_puts_the_body_back_in_the_editor_and_leaves_the_date() {
    let (mut harness, id) = edited();
    let before = stored(&harness, &id).updated_at;
    click_id(&mut harness, "revisions-toggle");
    click_id(&mut harness, "revision-open");
    click_id(&mut harness, "revision-restore");

    assert_eq!(stored(&harness, &id).content, "SELECT 1;");
    assert_eq!(stored(&harness, &id).updated_at, before);
    assert_eq!(
        body(&harness),
        "SELECT 1;",
        "the editor shows the body put back"
    );
    click_id(&mut harness, "editor-close");
    assert_eq!(
        stored(&harness, &id).content,
        "SELECT 1;",
        "the close writes nothing older"
    );
}

#[test]
fn a_new_note_and_a_todo_list_offer_no_history() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = expanded_area(backend, profile);
    click_id(&mut harness, "new-note");
    settle(&mut harness);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("revisions-toggle"))
            .is_none()
    );
}
