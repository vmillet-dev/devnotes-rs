use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;
use engine::notes::language::Language;
use engine::notes::model::NotePatch;

use crate::notes::bench::{Bench, area, click_id, settle};
use crate::testing;

type Harness = egui_kittest::Harness<'static, Bench>;

/// A snippet holding `content` in `language`, open in the editor.
fn snippet(content: &str, language: Language) -> (Harness, String) {
    text::highlight::warm();
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "deploy").id.clone();
    let patch = NotePatch {
        content: Some(content.into()),
        language: Some(language),
        ..NotePatch::default()
    };
    let at = id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, at, patch)
    })
    .unwrap();
    let mut harness = area(backend, profile);
    click_id(&mut harness, "note-card-open");
    (harness, id)
}

fn stored(harness: &Harness, id: &str) -> String {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, id)
    })
    .unwrap()
    .content
    .clone()
}

fn press(harness: &mut Harness, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    settle(harness);
}

fn code(harness: &Harness) -> String {
    harness
        .get_by(|node| node.author_id() == Some("editor-body"))
        .value()
        .unwrap_or_default()
}

fn count(harness: &Harness) -> String {
    harness
        .get_by(|node| node.author_id() == Some("search-count"))
        .value()
        .unwrap_or_default()
}

#[test]
fn tab_and_the_comment_follow_the_language() {
    let (mut harness, id) = snippet("print(1)", Language::Py);
    click_id(&mut harness, "editor-body");
    press(&mut harness, Modifiers::COMMAND, Key::Home);
    press(&mut harness, Modifiers::NONE, Key::Tab);
    assert_eq!(code(&harness), "    print(1)", "four spaces in Python");
    press(&mut harness, Modifiers::COMMAND, Key::Slash);
    assert_eq!(code(&harness), "    # print(1)");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "    # print(1)");
}

#[test]
fn ctrl_f_finds_counts_and_moves_between_matches() {
    let (mut harness, _) = snippet("id = 1\nuser_id = id\nprint(id)", Language::Py);
    click_id(&mut harness, "editor-body");
    press(&mut harness, Modifiers::COMMAND, Key::F);
    harness.get_by(|node| node.author_id() == Some("search-field"));
    harness
        .get_by(|node| node.author_id() == Some("search-field"))
        .type_text("id");
    settle(&mut harness);
    assert_eq!(count(&harness), "1 / 4");

    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(count(&harness), "2 / 4");
    click_id(&mut harness, "search-word");
    assert_eq!(
        count(&harness),
        "3 matches",
        "the one inside user_id no longer counts"
    );
    press(&mut harness, Modifiers::SHIFT, Key::Enter);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("search-field"))
            .is_none()
    );
}

#[test]
fn ctrl_h_replaces_one_then_all_and_one_undo_takes_all_back() {
    let (mut harness, id) = snippet("a b a b a", Language::Txt);
    click_id(&mut harness, "editor-body");
    press(&mut harness, Modifiers::COMMAND, Key::H);
    harness
        .get_by(|node| node.author_id() == Some("search-field"))
        .type_text("a");
    settle(&mut harness);
    click_id(&mut harness, "search-replace-field");
    harness
        .get_by(|node| node.author_id() == Some("search-replace-field"))
        .type_text("x");
    settle(&mut harness);
    click_id(&mut harness, "search-replace-one");
    assert_eq!(code(&harness), "x b a b a");
    click_id(&mut harness, "search-replace-all");
    assert_eq!(code(&harness), "x b x b x");

    click_id(&mut harness, "editor-body");
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(code(&harness), "x b a b a");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "x b a b a");
}

#[test]
fn a_snippet_s_fields_are_marked_where_the_core_finds_them() {
    let ranges = crate::api::notes::field_ranges("é {{host}} {{ not a field }} {{port=5432}}");
    assert_eq!(ranges, [2..10, 29..42]);
}

#[test]
fn the_language_chosen_colours_the_code_and_the_footer_names_it() {
    let (mut harness, _) = snippet("SELECT 1;", Language::Txt);
    click_id(&mut harness, "editor-language");
    harness
        .get_by(|node| node.author_id() == Some("editor-language-filter"))
        .type_text("sql");
    harness.run();
    harness.get_by_label("SQL").click();
    settle(&mut harness);
    let stats = harness
        .get_by(|node| node.author_id() == Some("editor-stats"))
        .value()
        .unwrap_or_default();
    assert!(stats.starts_with("SQL"), "{stats}");
}
