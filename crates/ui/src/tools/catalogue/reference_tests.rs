//! The references: every row has its words in both languages, and each is searched as it reads.

use egui::Key;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use i18n::Language;

use super::REFERENCES;
use super::text_tests::{click, has, opened, type_in, value_of};
use crate::notes::bench::{Bench, settle};
use crate::tools::tests::tools;

const LANGUAGES: [Language; 2] = [Language::Fr, Language::En];

fn keys_found(id: &str, query: &str) -> Vec<String> {
    let (_, table) = REFERENCES.iter().find(|it| it.0 == id).unwrap();
    let table = table(Language::En);
    let found = table.found(query);
    found.iter().map(|row| row.key.clone()).collect()
}

fn hits(harness: &Harness<'_, Bench>) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some("tools-reference-hit"))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect()
}

fn searched(harness: &mut Harness<'_, Bench>, text: &str) {
    type_in(harness, "tools-search", text);
    settle(harness);
}

#[test]
fn every_row_has_its_words_and_every_word_its_row() {
    for language in LANGUAGES {
        for (id, table) in REFERENCES {
            let table = table(language);
            assert!(!table.rows.is_empty(), "{id}");
            for row in &table.rows {
                let (title, line) = &row.found_as;
                let key = &row.key;
                assert!(!title.is_empty() && !line.is_empty(), "{id} {key}");
                assert!(
                    table.groups.iter().any(|group| group.id == row.group),
                    "{id} {key}: its group"
                );
                assert_eq!(row.cells.len(), table.columns.len(), "{id} {key}");
            }
            for group in &table.groups {
                assert!(!group.label.is_empty(), "{id} {}", group.id);
            }
            for column in &table.columns {
                assert!(!column.label.is_empty(), "{id}");
            }
        }
    }
}

#[test]
fn a_word_that_holds_a_brace_keeps_it() {
    let (_, table) = REFERENCES
        .iter()
        .find(|it| it.0 == "mustache-sheet")
        .unwrap();
    let table = table(Language::En);
    let delimiters = table
        .rows
        .iter()
        .find(|row| row.key.contains("<%"))
        .unwrap();

    assert!(delimiters.found_as.1.contains("{{ }}"));
}

#[test]
fn each_reference_finds_what_it_is_asked() {
    assert_eq!(keys_found("http-status", "teapot"), ["418"]);
    assert!(keys_found("http-headers", "cors").len() > 3);
    assert_eq!(keys_found("mime-types", ".webp"), ["image/webp"]);
    assert!(keys_found("mime-types", "webp").contains(&"image/webp".to_owned()));
    assert!(keys_found("markdown-sheet", "bold").contains(&"**…**".to_owned()));
    assert!(!keys_found("mustache-sheet", "partial").is_empty());
}

#[test]
fn a_space_that_matters_is_copied_as_a_space() {
    let (_, table) = REFERENCES
        .iter()
        .find(|it| it.0 == "markdown-sheet")
        .unwrap();
    let table = table(Language::En);
    let spaced = table.rows.iter().find(|row| row.key.contains('␣')).unwrap();

    let (value, _) = spaced.copy.as_ref().unwrap();
    assert!(!value.contains('␣') && value.contains(' '));
}

#[test]
fn a_reference_opens_on_its_table_and_filters_it() {
    let mut harness = opened("http-status");

    assert!(has(&harness, "reference-row"));
    type_in(&mut harness, "reference-search", "zzz");
    settle(&mut harness);

    assert!(has(&harness, "reference-empty"));
}

#[test]
fn a_code_no_tool_answers_is_found_in_the_references_and_enter_opens_it_lit() {
    let mut harness = tools();
    harness.set_size(egui::vec2(1_300.0, 900.0));

    searched(&mut harness, "418");
    assert!(has(&harness, "tools-references"));
    assert!(!has(&harness, "tools-no-match"));
    assert!(has(&harness, "tools-first-result"));
    assert!(hits(&harness)[0].contains("418"));

    harness.key_press(Key::Enter);
    settle(&mut harness);

    assert!(has(&harness, "tool-title"));
    assert!(value_of(&harness, "reference-search").is_empty());
    let rows = harness.query_all_by(|node| node.author_id() == Some("reference-row"));
    assert!(rows.count() > 10, "its table whole");
}

#[test]
fn one_character_is_not_worth_reading_the_references() {
    let mut harness = tools();

    searched(&mut harness, "4");

    assert!(!has(&harness, "tools-references"));
}

#[test]
fn four_rows_a_reference_and_the_rest_opens_it_searched() {
    let mut harness = tools();
    harness.set_size(egui::vec2(1_300.0, 900.0));

    searched(&mut harness, "cors");
    assert!(has(&harness, "tools-reference-more"));

    click(&mut harness, "tools-reference-more");

    assert_eq!(value_of(&harness, "reference-search"), "cors");
}

#[test]
fn nothing_found_anywhere_says_so() {
    let mut harness = tools();

    searched(&mut harness, "zzzz");

    assert!(has(&harness, "tools-no-match"));
    assert!(!has(&harness, "tools-references"));
}
