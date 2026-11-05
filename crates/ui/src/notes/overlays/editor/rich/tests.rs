use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;

use crate::notes::bench::{Bench, area, click_id, settle};
use crate::testing;

type Harness<'a> = egui_kittest::Harness<'a, Bench>;

/// A Note holding `content`, open in the editor.
fn note(content: &str) -> (Harness<'static>, String) {
    let (backend, profile) = testing::unlocked();
    let content = content.to_owned();
    let space = testing::note(&backend, "seed").space_id.clone();
    let id = engine::db::run_here(backend.engine(), move |engine| {
        let draft = engine::notes::model::NoteDraft {
            kind: NoteKind::Note,
            language: Language::Txt,
            content,
            ..testing::snippet(space, "Standup".to_owned())
        };
        engine::notes::create_note(engine, draft)
    })
    .unwrap()
    .id
    .clone();
    let mut harness = area(backend, profile);
    harness
        .get_by(|node| node.author_id() == Some("search-input"))
        .focus();
    harness.step();
    harness
        .get_by(|node| node.author_id() == Some("search-input"))
        .type_text("Standup");
    crate::notes::bench::until(&mut harness, "the Note alone", |harness| {
        crate::notes::bench::cards(harness) == 1
    });
    click_id(&mut harness, "note-card-open");
    (harness, id)
}

fn stored(harness: &Harness<'_>, id: &str) -> String {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, id)
    })
    .unwrap()
    .content
    .clone()
}

fn surface_text(harness: &Harness<'_>) -> String {
    harness
        .get_by(|node| node.author_id() == Some("editor-rich"))
        .accesskit_node()
        .value()
        .unwrap_or_default()
}

fn press(harness: &mut Harness<'_>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    settle(harness);
}

fn type_in(harness: &mut Harness<'_>, text: &str) {
    harness
        .get_by(|node| node.author_id() == Some("editor-rich"))
        .type_text(text);
    settle(harness);
}

/// The caret at the end of the Note.
fn at_end(harness: &mut Harness<'_>) {
    click_id(harness, "editor-rich");
    press(harness, Modifiers::COMMAND, Key::End);
}

#[test]
fn a_note_opened_and_left_keeps_its_markdown_byte_for_byte() {
    let written = "Une note[^1].\n\n- un\n- deux";
    let (mut harness, id) = note(written);
    assert!(surface_text(&harness).contains("deux"));
    click_id(&mut harness, "editor-rich");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), written, "nothing rewritten");
}

#[test]
fn typing_enter_and_bold_are_written_as_markdown() {
    let (mut harness, id) = note("Ship it");
    at_end(&mut harness);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    press(&mut harness, Modifiers::COMMAND, Key::B);
    type_in(&mut harness, "now");
    press(&mut harness, Modifiers::COMMAND, Key::B);
    type_in(&mut harness, " please");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "Ship it\n\n**now** please");
}

#[test]
fn a_list_from_the_toolbar_nests_with_tab_and_a_box_is_ticked() {
    let (mut harness, id) = note("un");
    at_end(&mut harness);
    click_id(&mut harness, "rich-tasks");
    click_id(&mut harness, "editor-rich");
    press(&mut harness, Modifiers::COMMAND, Key::End);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    type_in(&mut harness, "deux");
    press(&mut harness, Modifiers::NONE, Key::Tab);
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "- [ ] un\n  - [ ] deux");
}

#[test]
fn undo_takes_a_typed_word_back_in_one_step_and_the_caret_crosses_blocks() {
    let (mut harness, id) = note("ab\n\ncd");
    click_id(&mut harness, "editor-rich");
    press(&mut harness, Modifiers::COMMAND, Key::Home);
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    press(&mut harness, Modifiers::NONE, Key::Home);
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(surface_text(&harness), "abcd", "joined");
    type_in(&mut harness, "XY");
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(surface_text(&harness), "abcd");
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(surface_text(&harness), "ab\ncd");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "ab\n\ncd");
}

#[test]
fn a_heading_and_a_quote_come_from_the_toolbar() {
    let (mut harness, id) = note("Titre\n\ncité");
    click_id(&mut harness, "editor-rich");
    press(&mut harness, Modifiers::COMMAND, Key::Home);
    click_id(&mut harness, "rich-h2");
    click_id(&mut harness, "editor-rich");
    press(&mut harness, Modifiers::COMMAND, Key::End);
    click_id(&mut harness, "rich-quote");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "## Titre\n\n> cité");
}

#[test]
fn a_task_s_box_is_ticked_and_written() {
    let (mut harness, id) = note("- [ ] write the notes");
    click_id(&mut harness, "rich-task");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "- [x] write the notes");
}

#[test]
fn an_empty_note_is_typed_into_from_its_first_line() {
    let (mut harness, id) = note("");
    click_id(&mut harness, "editor-rich");
    click_id(&mut harness, "rich-quote");
    type_in(&mut harness, "2 * 3 * 4");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "> 2 * 3 * 4");
}
