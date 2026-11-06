//! The widgets the tools share, one by one, and what the frame does with them over a real core.

use egui::{Key, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::language::Language as NoteLanguage;
use i18n::Language;

use super::note_picker::{self, PickedNote};
use super::*;
use crate::notes::bench::{Bench, area_of, names, settle, until};
use crate::state::Area;
use crate::testing;
use crate::tools::{Drawn, Tool, ToolResult};

fn drawn_alone(draw: impl FnMut(&mut Ui) + 'static) -> Harness<'static> {
    let mut draw = draw;
    testing::harness()
        .with_size(Vec2::new(700.0, 300.0))
        .build_ui(move |ui| draw(ui))
}

fn values(harness: &Harness<'_>) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some("output-value"))
        .map(|node| node.accesskit_node().value().unwrap_or_default())
        .collect()
}

#[test]
fn a_result_row_says_its_name_and_value_and_copies_the_exact_one() {
    let mut harness = drawn_alone(|ui| {
        ResultRow::new("Size", "1.5 GiB")
            .copied("1610612736")
            .show(ui, Language::En);
    });
    harness.run();
    assert_eq!(values(&harness), ["1.5 GiB"]);

    harness.get_by_label("Copy Size").click();
    harness.step();

    let copied = harness.output().platform_output.commands.iter().any(
        |command| matches!(command, egui::OutputCommand::CopyText(text) if text == "1610612736"),
    );
    assert!(copied);
}

#[test]
fn a_reading_offers_no_copy_and_an_empty_value_cannot_be_copied() {
    let mut harness = drawn_alone(|ui| {
        ResultRow::new("Reading", "ok")
            .reading()
            .show(ui, Language::En);
        CopyValue::new("", "nothing", Language::En).show(ui);
    });
    harness.run();

    assert!(harness.query_by_label("Copy Reading").is_none());
    assert!(
        harness
            .get_by_label("Copy nothing")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn a_long_value_goes_under_its_name_held_to_a_height() {
    let long = "a".repeat(5_000);
    let mut harness = drawn_alone(move |ui| {
        ResultRow::new("Long", &long).show(ui, Language::En);
    });
    harness.run();

    let row = harness.get_by(|node| node.author_id() == Some("output-row"));
    let height = row.rect().height();
    assert!(height < 240.0, "the row is {height} high");
}

#[test]
fn an_output_row_shows_its_value_and_what_it_measures() {
    let mut harness = drawn_alone(|ui| {
        OutputRow::new("SHA-256", "abc")
            .meta("256 bits")
            .show(ui, Language::En);
    });
    harness.run();

    assert_eq!(values(&harness), ["abc"]);
    assert!(harness.query_by_label("Copy SHA-256").is_some());
}

fn table_rows() -> Vec<ReferenceRow> {
    [
        ("404", "4xx", "Not Found"),
        ("500", "5xx", "Internal Server Error"),
    ]
    .into_iter()
    .map(|(key, group, name)| {
        ReferenceRow::new(key, group, vec![Cell::mono(key), Cell::text(name)])
            .searched_by([key, name])
    })
    .collect()
}

fn table(query: &'static str) -> Harness<'static, String> {
    let rows = table_rows();
    let groups = ["4xx", "5xx"].map(|id| Group {
        id: id.to_owned(),
        label: id.to_owned(),
    });
    let columns = ["Code", "Name"].map(|label| Column {
        label: label.to_owned(),
        share: None,
    });
    testing::harness()
        .with_size(Vec2::new(900.0, 500.0))
        .build_ui_state(
            move |ui, query: &mut String| {
                let table = ReferenceTable {
                    rows: &rows,
                    groups: &groups,
                    columns: &columns,
                    caption: "Statuses",
                    placeholder: "A code",
                    card_width: 560.0,
                    lit: None,
                };
                table.show(ui, query, Language::En);
            },
            query.to_owned(),
        )
}

fn row_keys(harness: &Harness<'_, String>) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some("reference-row"))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect()
}

#[test]
fn a_reference_lists_its_rows_under_their_groups_and_counts_them() {
    let mut harness = table("");
    harness.run();

    assert_eq!(row_keys(&harness), ["404", "500"]);
    let count = harness.get_by(|node| node.author_id() == Some("reference-count"));
    assert_eq!(count.accesskit_node().value().as_deref(), Some("2 entries"));
}

#[test]
fn a_search_narrows_the_rows_folding_case_and_says_when_nothing_is_left() {
    let mut harness = table("NOT");
    harness.run();
    assert_eq!(row_keys(&harness), ["404"]);

    harness.state_mut().clear();
    harness.state_mut().push_str("zzz");
    harness.run();
    assert!(row_keys(&harness).is_empty());
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("reference-empty"))
            .is_some()
    );
}

/// A tool that keeps what it is given: the frame's side of the contract.
#[derive(Default)]
struct Echo {
    text: String,
    warned: bool,
}

impl Tool for Echo {
    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        note_import(ui, drawn, "text");
        text_area(ui, &mut self.text, ("Text", "echo-input"), "", 3);
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, _: Language) -> Option<ToolResult> {
        (!self.text.is_empty()).then(|| ToolResult {
            title: "Echoed".to_owned(),
            language: NoteLanguage::Txt,
            content: self.text.to_uppercase(),
            warning: self
                .warned
                .then_some(i18n::keys::tools::password::SAVE_WARNING),
        })
    }

    fn imported(&mut self, _: &str, note: &PickedNote) {
        self.text.clone_from(&note.content);
    }

    fn clear(&mut self) {
        self.text.clear();
    }
}

fn echo(text: &str, warned: bool) -> Harness<'static, Bench> {
    let (backend, profile) = testing::unlocked();
    let mut harness = area_of(Area::Tools, backend, profile);
    harness
        .ctx
        .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
    let bench = harness.state_mut();
    bench.state.spaces.create_space(&bench.backend, "Travail");
    until(&mut harness, "the space is listed", |harness| {
        !harness.state().state.spaces.spaces().is_empty()
    });
    click(&mut harness, "tools-entry-case");
    let tool = Echo {
        text: text.to_owned(),
        warned,
    };
    harness
        .state_mut()
        .state
        .tools_view
        .put("case", Box::new(tool));
    settle(&mut harness);
    harness
}

fn click(harness: &mut Harness<'_, Bench>, id: &str) {
    harness.get_by(|node| node.author_id() == Some(id)).click();
    settle(harness);
}

fn a_note(harness: &mut Harness<'_, Bench>, title: &str, content: &str) {
    let bench = harness.state();
    let space = bench.state.spaces.spaces()[0].id.clone();
    let mut draft = testing::snippet(space, title.to_owned());
    draft.content = content.to_owned();
    engine::db::run_here(bench.backend.engine(), move |engine| {
        engine::notes::create_note(engine, draft)
    })
    .unwrap();
}

#[test]
fn a_note_is_imported_whole_into_the_input_it_was_asked_for() {
    let mut harness = echo("", false);
    a_note(&mut harness, "Journal", "the whole body");

    click(&mut harness, "note-import-text");
    assert_eq!(names(&harness, "note-picker-option"), ["Journal"]);
    harness
        .get_by(|node| {
            node.author_id() == Some("note-picker-option")
                && node.label().as_deref() == Some("Journal")
        })
        .click();
    settle(&mut harness);

    assert!(names(&harness, "note-picker").is_empty());
    let input = harness.get_by(|node| node.author_id() == Some("echo-input"));
    assert_eq!(
        input.accesskit_node().value().as_deref(),
        Some("the whole body")
    );
}

#[test]
fn enter_takes_the_first_note_found_and_escape_leaves_the_tool_as_it_was() {
    let mut harness = echo("kept", false);
    a_note(&mut harness, "Journal", "the whole body");

    click(&mut harness, "note-import-text");
    harness.key_press(Key::Escape);
    settle(&mut harness);
    assert!(names(&harness, "note-picker").is_empty());
    let input = harness.get_by(|node| node.author_id() == Some("echo-input"));
    assert_eq!(input.accesskit_node().value().as_deref(), Some("kept"));

    click(&mut harness, "note-import-text");
    harness
        .get_by(|node| node.author_id() == Some("note-picker-search"))
        .type_text("Journal");
    settle(&mut harness);
    harness.key_press(Key::Enter);
    settle(&mut harness);
    let input = harness.get_by(|node| node.author_id() == Some("echo-input"));
    assert_eq!(
        input.accesskit_node().value().as_deref(),
        Some("the whole body")
    );
}

#[test]
fn a_tool_with_nothing_computed_offers_nothing_to_keep() {
    let harness = echo("", false);

    let save = harness.get_by(|node| node.author_id() == Some("tool-save-as-note"));
    assert!(save.accesskit_node().is_disabled());
}

#[test]
fn a_result_is_written_as_a_note_with_where_it_came_from() {
    let mut harness = echo("computed", false);

    click(&mut harness, "tool-save-as-note");
    click(&mut harness, "save-as-note-submit");

    let any = crate::tools::catalogue::find("case").unwrap().from_note[0].1;
    let asked = note_picker::query("Echoed".to_owned(), any, chrono::Utc::now());
    let note = engine::db::run_here(harness.state().backend.engine(), move |engine| {
        let view = engine::notes::query_notes(engine, asked)?;
        let id = view.sections[0].cards[0].note.note.id.clone();
        engine::notes::get_note(engine, id)
    })
    .unwrap();
    assert_eq!(note.content, "COMPUTED");
    assert!(note.source.starts_with("Tools / "));
}

#[test]
fn a_secret_kept_says_so_and_the_button_says_it_goes_all_the_same() {
    let mut harness = echo("computed", true);

    click(&mut harness, "tool-save-as-note");

    assert!(
        harness
            .query_by(|node| node.author_id() == Some("save-as-note-warning"))
            .is_some()
    );
    let submit = harness.get_by(|node| node.author_id() == Some("save-as-note-submit"));
    assert_eq!(
        submit.accesskit_node().label().as_deref(),
        Some("Save anyway")
    );
}
