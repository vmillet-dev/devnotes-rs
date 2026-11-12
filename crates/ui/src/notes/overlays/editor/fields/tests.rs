use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::model::{DisplayNote, NotePatch};

use crate::notes::bench::{Bench, area, click_id, settle};
use crate::testing;

type Harness = egui_kittest::Harness<'static, Bench>;

const BODY: &str = "psql -h {{host}} -p {{port=5432}}";

/// A snippet holding `text`, open in the editor.
fn opened(text: &str) -> (Harness, String) {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let patch = NotePatch {
        content: Some(text.to_owned()),
        ..NotePatch::default()
    };
    let at = id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, at, patch)
    })
    .unwrap();
    let mut harness = area(backend, profile);
    harness.state_mut().state.editor.open(id.clone());
    settle(&mut harness);
    settle(&mut harness);
    (harness, id)
}

fn stored(harness: &Harness, id: &str) -> DisplayNote {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, id)
    })
    .unwrap()
}

fn value_of(note: &DisplayNote, name: &str) -> String {
    let field = note.placeholders.iter().find(|field| field.name == name);
    field.unwrap().value.clone()
}

fn has(harness: &Harness, id: &str) -> bool {
    harness
        .query_by(|node| node.author_id() == Some(id))
        .is_some()
}

fn counted(harness: &Harness) -> String {
    harness
        .get_by(|node| node.author_id() == Some("placeholder-panel-toggle"))
        .value()
        .unwrap_or_default()
}

/// Typed into the first field, which is then left for the title.
fn fill_first(harness: &mut Harness, words: &str) {
    type_first(harness, words);
    click_id(harness, "editor-title");
}

fn type_first(harness: &mut Harness, words: &str) {
    let first = |harness: &Harness| {
        let mut rows = harness.get_all_by(|node| node.author_id() == Some("placeholder-input"));
        rows.next().unwrap().accesskit_node().id()
    };
    let at = first(harness);
    harness.get_by(|node| node.id() == at).click();
    settle(harness);
    harness.get_by(|node| node.id() == at).type_text(words);
    settle(harness);
}

/// `author_id` clicked, then what reached the clipboard.
fn copied_by(harness: &mut Harness, author_id: &str) -> Option<String> {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .click();
    let mut copied = None;
    for _ in 0..60 {
        harness.step();
        harness.state().backend.settle();
        copied = copied.or_else(|| {
            let commands = &harness.output().platform_output.commands;
            commands.iter().find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
        });
    }
    copied
}

#[test]
fn a_note_with_no_field_has_no_panel() {
    let (harness, _) = opened("select 1");

    assert!(!has(&harness, "placeholder-panel-toggle"));
}

#[test]
fn the_panel_lists_the_fields_and_counts_what_is_filled() {
    let (mut harness, _) = opened(BODY);
    assert_eq!(counted(&harness), "0/2");
    let rows = harness.get_all_by(|node| node.author_id() == Some("placeholder-input"));
    assert_eq!(rows.count(), 2);

    fill_first(&mut harness, "db");

    assert_eq!(counted(&harness), "1/2");
}

/// A head that takes the height left pushes the rows under the footer, and no node says so.
#[test]
fn the_rows_stand_right_under_the_head_and_above_the_body() {
    let (harness, _) = opened(BODY);
    let toggle = harness.get_by(|node| node.author_id() == Some("placeholder-panel-toggle"));
    let row = harness
        .get_all_by(|node| node.author_id() == Some("placeholder-input"))
        .next()
        .unwrap();
    let body = harness.get_by(|node| node.author_id() == Some("editor-body"));

    let (toggle, row, body) = (toggle.rect(), row.rect(), body.rect());
    assert!(row.min.y - toggle.max.y < 40.0, "{toggle:?} {row:?}");
    assert!(body.min.y > row.max.y, "{row:?} {body:?}");
}

#[test]
fn a_value_is_written_on_leaving_its_field_and_no_date_moves() {
    let (mut harness, id) = opened(BODY);
    let before = stored(&harness, &id).updated_at;

    fill_first(&mut harness, "db");

    let note = stored(&harness, &id);
    assert_eq!(value_of(&note, "host"), "db");
    assert_eq!(note.updated_at, before);
}

#[test]
fn closing_writes_the_value_still_being_typed() {
    let (mut harness, id) = opened(BODY);

    type_first(&mut harness, "db");
    click_id(&mut harness, "editor-close");
    settle(&mut harness);

    assert_eq!(value_of(&stored(&harness, &id), "host"), "db");
}

#[test]
fn folded_the_panel_says_what_it_holds_and_the_choice_is_remembered() {
    let (mut harness, _) = opened(BODY);
    fill_first(&mut harness, "db");

    click_id(&mut harness, "placeholder-panel-toggle");

    assert!(!has(&harness, "placeholder-input"));
    assert!(!harness.state().state.settings.editor_fields_open);
    let toggle = harness.get_by(|node| node.author_id() == Some("placeholder-panel-toggle"));
    let label = toggle.accesskit_node().label().unwrap_or_default();
    assert!(label.ends_with("host = db"));
    assert_eq!(counted(&harness), "1/2");
}

#[test]
fn the_preview_shows_the_body_filled_and_follows_the_typing() {
    let (mut harness, _) = opened(BODY);
    click_id(&mut harness, "placeholder-panel-preview");
    settle(&mut harness);
    assert!(!has(&harness, "editor-body"));

    type_first(&mut harness, "db");
    settle(&mut harness);

    let session = harness.state().state.editor.opened().unwrap().session;
    let filled = harness.state().state.fill.preview(session);
    assert_eq!(filled, Some("psql -h db -p 5432"));
}

#[test]
fn folding_closes_the_preview() {
    let (mut harness, _) = opened(BODY);
    click_id(&mut harness, "placeholder-panel-preview");
    settle(&mut harness);

    click_id(&mut harness, "placeholder-panel-toggle");
    settle(&mut harness);

    assert!(has(&harness, "editor-body"));
}

#[test]
fn clearing_empties_the_values_and_writes_it() {
    let (mut harness, id) = opened(BODY);
    fill_first(&mut harness, "db");

    click_id(&mut harness, "placeholder-panel-reset");
    settle(&mut harness);

    assert_eq!(counted(&harness), "0/2");
    assert_eq!(value_of(&stored(&harness, &id), "host"), "");
    assert!(!has(&harness, "placeholder-panel-reset"));
}

#[test]
fn the_editor_copies_filled_with_what_the_panel_holds_without_the_form() {
    let (mut harness, _) = opened(BODY);
    fill_first(&mut harness, "db");

    let copied = copied_by(&mut harness, "editor-copy-filled");

    assert_eq!(copied.as_deref(), Some("psql -h db -p 5432"));
    assert!(!has(&harness, "placeholder-form-hint"));
}

#[test]
fn copy_as_is_keeps_the_tokens() {
    let (mut harness, _) = opened(BODY);
    fill_first(&mut harness, "db");

    let copied = copied_by(&mut harness, "placeholder-panel-copy-raw");

    assert_eq!(copied.as_deref(), Some(BODY));
}

/// Expanded, the code stays and its filled text stands under it, following the typing.
#[test]
fn expanded_the_filled_text_stands_under_the_code() {
    let (mut harness, _) = opened(BODY);
    harness.state_mut().state.settings.editor_fullscreen = true;
    settle(&mut harness);
    assert!(!has(&harness, "placeholder-panel-preview"));

    type_first(&mut harness, "db");
    settle(&mut harness);

    assert!(has(&harness, "editor-body"));
    assert!(has(&harness, "editor-filled"));
    let session = harness.state().state.editor.opened().unwrap().session;
    let filled = harness.state().state.fill.preview(session);
    assert_eq!(filled, Some("psql -h db -p 5432"));
}
