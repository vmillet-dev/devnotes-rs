use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::language::Language;
use engine::notes::model::NotePatch;

use crate::notes::bench::{Bench, area, click_id, remembered, settle, until};
use crate::testing;

type Harness<'a> = egui_kittest::Harness<'a, Bench>;

const MESSY: &str = "import {inject} from '@angular/core'\nconst host = {{db_host}}\nexport class InvoicesApi{\nprivate readonly http=inject(HttpClient)\nlist(id:string){return this.http.get(`/api/customers/${id}/invoices`,{params:{page:1,limit:50}})}\n}";
/// The library style: single quotes, semicolons, 100 columns — and the field left as written.
const TIDY: &str = "import { inject } from '@angular/core';\nconst host = {{db_host}};\nexport class InvoicesApi {\n  private readonly http = inject(HttpClient);\n  list(id: string) {\n    return this.http.get(`/api/customers/${id}/invoices`, { params: { page: 1, limit: 50 } });\n  }\n}";

fn snippet(content: &str, language: Language) -> (Harness<'static>, String) {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "invoices").id.clone();
    let patch = NotePatch {
        content: Some(content.into()),
        language: Some(language),
        ..NotePatch::default()
    };
    let at = id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, &at, &patch)
    })
    .unwrap();
    let mut harness = area(backend, profile);
    click_id(&mut harness, "note-card-open");
    (harness, id)
}

fn code(harness: &Harness<'_>) -> String {
    let view = harness.state().state.notes_view.editor.as_ref().unwrap();
    view.code.as_ref().unwrap().text()
}

fn stored(harness: &Harness<'_>, id: &str) -> String {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, &id)
    })
    .unwrap()
    .content
    .clone()
}

fn notice(harness: &Harness<'_>) -> Option<String> {
    harness
        .query_by(|node| node.author_id() == Some("editor-format-notice"))
        .and_then(|node| node.accesskit_node().description())
}

fn formatted(harness: &mut Harness<'_>, expected: &str) {
    until(harness, "the formatted text in the field", |harness| {
        code(harness) == expected
    });
}

#[test]
fn the_button_formats_marks_the_lines_and_its_notice_undoes() {
    let (mut harness, _) = snippet(MESSY, Language::Ts);
    click_id(&mut harness, "editor-format");
    formatted(&mut harness, TIDY);
    assert_eq!(notice(&harness).as_deref(), Some("formatted"));
    let view = harness.state().state.notes_view.editor.as_ref().unwrap();
    assert!(
        !view.code.as_ref().unwrap().marked.is_empty(),
        "lines marked"
    );

    click_id(&mut harness, "editor-format-undo");
    assert_eq!(code(&harness), MESSY);
    assert!(notice(&harness).is_none());
}

fn rect(harness: &Harness<'_>, author_id: &str) -> egui::Rect {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .rect()
}

/// Where the frame painted `words`.
fn painted_at(harness: &Harness<'_>, words: &str) -> Option<egui::Rect> {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) if text.galley.text() == words => {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
}

#[test]
fn the_button_is_one_piece_with_its_chevron_and_says_its_keys_where_they_fit() {
    let (mut harness, _) = snippet(MESSY, Language::Ts);
    let (main, chevron) = (
        rect(&harness, "editor-format"),
        rect(&harness, "editor-format-settings"),
    );
    assert!(
        (main.right() - chevron.left()).abs() < 0.1,
        "{main:?} {chevron:?}"
    );
    assert!((main.height() - chevron.height()).abs() < 0.1);
    assert!(
        painted_at(&harness, "Shift Alt F").is_none(),
        "beside the cards"
    );

    harness.set_size(egui::vec2(1_700.0, 900.0));
    settle(&mut harness);
    click_id(&mut harness, "editor-fullscreen");
    let keys = painted_at(&harness, "Shift Alt F").expect("its keys, full screen");
    assert!(rect(&harness, "editor-format").contains_rect(keys));
}

#[test]
fn each_switch_of_the_settings_has_its_label_on_its_row() {
    let (mut harness, _) = snippet(MESSY, Language::Ts);
    click_id(&mut harness, "editor-format-settings");

    for (author_id, words) in [
        ("prettier-semicolons", "Semicolons"),
        ("prettier-trailingCommas", "Trailing commas"),
        ("prettier-formatOnSave", "Format on save"),
    ] {
        let switch = rect(&harness, author_id);
        let label = painted_at(&harness, words).unwrap_or_else(|| panic!("{words} is not painted"));
        assert!(label.right() < switch.left(), "{words}");
        assert!(
            (label.center().y - switch.center().y).abs() < 3.0,
            "{words}"
        );
    }
    click_id(&mut harness, "prettier-semicolons");
    assert!(!harness.state().state.prettier.semicolons);
}

#[test]
fn a_language_prettier_does_not_read_leaves_the_settings_in_reach() {
    let (mut harness, _) = snippet("SELECT 1;", Language::Sql);
    let main = harness.get_by(|node| node.author_id() == Some("editor-format"));
    assert!(main.accesskit_node().is_disabled());

    click_id(&mut harness, "editor-format-settings");

    rect(&harness, "prettier-semicolons");
}

#[test]
fn shift_alt_f_formats_from_the_title_and_the_result_is_saved() {
    let (mut harness, id) = snippet(MESSY, Language::Ts);
    click_id(&mut harness, "editor-title");
    harness.key_press_modifiers(Modifiers::SHIFT | Modifiers::ALT, Key::F);
    settle(&mut harness);
    formatted(&mut harness, TIDY);
    click_id(&mut harness, "editor-close");
    assert_eq!(stored(&harness, &id), TIDY);
}

#[test]
fn a_syntax_error_is_said_and_nothing_changes() {
    let broken = "const a = {\n  b: 1,,\n}";
    let (mut harness, _) = snippet(broken, Language::Ts);
    click_id(&mut harness, "editor-format");
    until(&mut harness, "the error said", |harness| {
        notice(harness).as_deref() == Some("syntax")
    });
    assert_eq!(code(&harness), broken);
}

#[test]
fn a_language_prettier_does_not_read_leaves_the_button_disabled_and_named() {
    let (harness, _) = snippet("select  *  from notes", Language::Sql);
    let button = harness.get_by(|node| node.author_id() == Some("editor-format"));
    assert!(button.accesskit_node().is_disabled());
}

#[test]
fn the_library_settings_apply_at_once_and_are_kept() {
    let (mut harness, _) = snippet(MESSY, Language::Ts);
    click_id(&mut harness, "editor-format-settings");
    click_id(&mut harness, "prettier-quotes-1");
    click_id(&mut harness, "prettier-indentation-2");
    let kept = remembered(&harness, crate::notes::prettier::KEY).unwrap_or_default();
    assert!(kept.contains("\"quotes\":\"double\""), "{kept}");

    harness.key_press(Key::Escape);
    settle(&mut harness);
    click_id(&mut harness, "editor-format");
    until(
        &mut harness,
        "formatted in the library's style",
        |harness| {
            code(harness).starts_with("import { inject } from \"@angular/core\";\n")
                && code(harness).contains("\n    private readonly")
        },
    );
}

#[test]
fn a_body_left_is_formatted_on_save_when_the_library_asks() {
    let (mut harness, id) = snippet("a", Language::Ts);
    click_id(&mut harness, "editor-format-settings");
    click_id(&mut harness, "prettier-formatOnSave");
    harness.key_press(Key::Escape);
    settle(&mut harness);

    click_id(&mut harness, "editor-body");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness
        .get_by(|node| node.author_id() == Some("editor-body"))
        .type_text("const  b=1");
    settle(&mut harness);
    click_id(&mut harness, "editor-close");
    until(&mut harness, "written formatted", |harness| {
        stored(harness, &id) == "const b = 1;"
    });
}
