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

/// A `char` an event, as a keyboard types: the input rules read each one.
fn key_by_key(harness: &mut Harness<'_>, text: &str) {
    for glyph in text.chars() {
        type_in(harness, &glyph.to_string());
    }
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

#[test]
fn the_arrows_select_across_blocks_and_cut_then_paste_moves_the_text() {
    let (mut harness, id) = note("ab\n\ncd\n\nef");
    click_id(&mut harness, "editor-rich");
    press(&mut harness, Modifiers::COMMAND, Key::Home);
    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    press(&mut harness, Modifiers::SHIFT, Key::ArrowDown);
    press(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    harness.event(egui::Event::Cut);
    settle(&mut harness);
    assert_eq!(surface_text(&harness), "a\nef");
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    press(&mut harness, Modifiers::NONE, Key::End);
    press(&mut harness, Modifiers::SHIFT, Key::ArrowLeft);
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    harness.event(egui::Event::Paste("b\ncd".to_owned()));
    settle(&mut harness);
    click_id(&mut harness, "editor-close");
    assert_eq!(
        stored(&harness, &id),
        "ab\n\ncdef",
        "joined by Delete, then pasted"
    );
}

#[test]
fn a_link_is_written_from_its_form_and_undo_and_redo_step_through_it() {
    let (mut harness, id) = note("Voir");
    at_end(&mut harness);
    type_in(&mut harness, " ");
    click_id(&mut harness, "rich-link");
    harness
        .get_by(|node| node.author_id() == Some("rich-link-text"))
        .focus();
    harness.step();
    harness
        .get_by(|node| node.author_id() == Some("rich-link-text"))
        .type_text("le site");
    harness
        .get_by(|node| node.author_id() == Some("rich-link-address"))
        .focus();
    harness.step();
    harness
        .get_by(|node| node.author_id() == Some("rich-link-address"))
        .type_text("https://devnotes.fr");
    settle(&mut harness);
    click_id(&mut harness, "rich-link-apply");
    assert_eq!(surface_text(&harness), "Voir le site");
    click_id(&mut harness, "editor-rich");
    press(&mut harness, Modifiers::COMMAND, Key::Z);
    assert_eq!(surface_text(&harness), "Voir ");
    press(&mut harness, Modifiers::COMMAND, Key::Y);
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "Voir [le site](https://devnotes.fr)");
}

#[test]
fn markdown_typed_becomes_formatting_and_the_shortcuts_format() {
    let (mut harness, id) = note("");
    click_id(&mut harness, "editor-rich");
    key_by_key(&mut harness, "## Plan");
    press(&mut harness, Modifiers::NONE, Key::Enter);
    key_by_key(&mut harness, "- du **gras** ici");
    press(&mut harness, Modifiers::NONE, Key::Enter);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    type_in(&mut harness, "fin");
    press(&mut harness, Modifiers::COMMAND | Modifiers::ALT, Key::Num3);
    click_id(&mut harness, "editor-close");
    assert_eq!(
        stored(&harness, &id),
        "## Plan\n\n- du **gras** ici\n\n### fin"
    );
}

#[test]
fn a_code_block_takes_its_language_and_tab_indents_in_it() {
    let (mut harness, id) = note("");
    click_id(&mut harness, "editor-rich");
    click_id(&mut harness, "rich-codeBlock");
    click_id(&mut harness, "rich-code-language");
    harness.get_by_label("PY").click();
    settle(&mut harness);
    key_by_key(&mut harness, "if a:");
    press(&mut harness, Modifiers::NONE, Key::Enter);
    press(&mut harness, Modifiers::NONE, Key::Tab);
    key_by_key(&mut harness, "b");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "```py\nif a:\n    b\n```");
}

#[test]
fn a_table_is_inserted_walked_with_tab_and_edited_from_its_tools() {
    let (mut harness, id) = note("");
    click_id(&mut harness, "editor-rich");
    click_id(&mut harness, "rich-table");
    for word in ["a", "b", "c"] {
        type_in(&mut harness, word);
        press(&mut harness, Modifiers::NONE, Key::Tab);
    }
    type_in(&mut harness, "1");
    click_id(&mut harness, "rich-columnAfter");
    click_id(&mut harness, "rich-deleteColumn");
    click_id(&mut harness, "rich-rowAfter");
    click_id(&mut harness, "rich-deleteRow");
    click_id(&mut harness, "editor-close");
    assert_eq!(
        stored(&harness, &id),
        "| a   | b   | c   |\n| --- | --- | --- |\n| 1   |     |     |\n|     |     |     |"
    );

    let (mut harness, id) = note("| a |\n| --- |\n| 1 |\n\nfin");
    click_id(&mut harness, "editor-rich");
    press(&mut harness, Modifiers::COMMAND, Key::Home);
    click_id(&mut harness, "rich-deleteTable");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "fin");
}

/// What a page or an editor left on the clipboard besides the text.
struct Clipboard(Option<String>);

impl platform::clipboard::Clipboard for Clipboard {
    fn html(&self) -> Option<String> {
        self.0.clone()
    }
}

fn paste(harness: &mut Harness<'_>, text: &str, html: Option<&str>) {
    harness.state_mut().state.clipboard = std::sync::Arc::new(Clipboard(html.map(str::to_owned)));
    harness.event(egui::Event::Paste(text.to_owned()));
    settle(harness);
}

#[test]
fn a_page_pasted_keeps_its_bold_and_its_link() {
    let (mut harness, id) = note("");
    click_id(&mut harness, "editor-rich");
    paste(
        &mut harness,
        "Voir zqcopie et là",
        Some("<p>Voir <strong>zqcopie</strong> et <a href=\"https://devnotes.fr\">là</a></p>"),
    );
    click_id(&mut harness, "editor-close");
    assert_eq!(
        stored(&harness, &id),
        "Voir **zqcopie** et [là](https://devnotes.fr)"
    );
}

#[test]
fn code_pasted_into_an_empty_note_makes_it_a_snippet_and_text_pastes_as_text() {
    let (mut harness, id) = note("");
    click_id(&mut harness, "editor-rich");
    paste(
        &mut harness,
        "{\n  \"id\": 42\n}",
        Some("<pre><code>{}</code></pre>"),
    );
    crate::notes::bench::until(&mut harness, "a JSON snippet", |harness| {
        let id = id.clone();
        let note = engine::db::run_here(harness.state().backend.engine(), move |engine| {
            engine::notes::get_note(engine, id)
        })
        .unwrap();
        note.kind == NoteKind::Snippet && note.language == Language::Json
    });

    let (mut harness, id) = note("Voir");
    at_end(&mut harness);
    paste(&mut harness, " {\"a\": 1}", None);
    click_id(&mut harness, "editor-close");
    assert_eq!(
        stored(&harness, &id),
        "Voir {\"a\": 1}",
        "a Note with words stays a Note"
    );
}

fn search_count(harness: &Harness<'_>) -> String {
    harness
        .get_by(|node| node.author_id() == Some("search-count"))
        .accesskit_node()
        .label()
        .or_else(|| {
            harness
                .get_by(|node| node.author_id() == Some("search-count"))
                .accesskit_node()
                .value()
        })
        .unwrap_or_default()
}

#[test]
fn the_find_bar_counts_in_the_text_read_and_replaces_keeping_the_marks() {
    let (mut harness, id) = note("**Ship** it, ship it\n\nship");
    click_id(&mut harness, "editor-rich");
    press(&mut harness, Modifiers::COMMAND, Key::H);
    harness
        .get_by(|node| node.author_id() == Some("search-field"))
        .type_text("ship");
    settle(&mut harness);
    assert_eq!(search_count(&harness), "1 / 3");
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(search_count(&harness), "2 / 3");
    click_id(&mut harness, "search-replace-field");
    harness
        .get_by(|node| node.author_id() == Some("search-replace-field"))
        .type_text("go");
    settle(&mut harness);
    click_id(&mut harness, "search-replace-all");
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), "**go** it, go it\n\ngo");
}

/// A PNG of `side` pixels square, as the core keeps a picture.
fn png_uri(side: u32) -> String {
    use base64::Engine as _;
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, side, side);
    encoder.set_color(png::ColorType::Rgba);
    let mut writer = encoder.write_header().unwrap();
    let pixels = vec![200u8; usize::try_from(side * side * 4).unwrap()];
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

#[test]
fn an_embedded_drawing_shows_its_picture_and_a_lost_one_says_so() {
    let (backend, profile) = testing::unlocked();
    let seed = testing::note(&backend, "seed");
    let space = seed.space_id.clone();
    let drawing = engine::db::run_here(backend.engine(), move |engine| {
        let draft = engine::notes::model::NoteDraft {
            kind: NoteKind::Drawing,
            content: String::new(),
            ..testing::snippet(space, "Archi".to_owned())
        };
        let id = engine::notes::create_note(engine, draft)?.id.clone();
        let pictures = engine::notes::drawing::DrawingPictures {
            thumbnail: png_uri(4),
            picture: png_uri(40),
        };
        let patch = engine::notes::model::NotePatch {
            pictures: Some(pictures),
            ..engine::notes::model::NotePatch::default()
        };
        engine::notes::update_note(engine, id.clone(), patch)?;
        Ok::<_, engine::error::AppError>(id)
    })
    .unwrap();
    let space = seed.space_id.clone();
    let content =
        format!("![le schéma](devnotes:drawing/{drawing})\n\n![perdu](devnotes:drawing/nope)");
    engine::db::run_here(backend.engine(), move |engine| {
        let draft = engine::notes::model::NoteDraft {
            kind: NoteKind::Note,
            language: Language::Txt,
            content,
            ..testing::snippet(space, "Standup".to_owned())
        };
        engine::notes::create_note(engine, draft)
    })
    .unwrap();
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
    crate::notes::bench::until(&mut harness, "both read", |harness| {
        let said: Vec<String> = harness
            .query_all_by(|node| node.author_id() == Some("rich-embed"))
            .map(|node| node.accesskit_node().label().unwrap_or_default())
            .collect();
        said == ["le schéma", "Drawing not found"]
    });
}
