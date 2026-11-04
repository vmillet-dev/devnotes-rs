use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;
use engine::notes::kind::NoteKind;
use engine::notes::model::NotePatch;

use super::{char_at, used_on};
use crate::notes::bench::{Bench, area, click_id, settle};
use crate::testing;

type Harness = egui_kittest::Harness<'static, Bench>;

/// No result here is rounded: `≈` is in no embedded font, and the harness refuses a system's.
const SHEET: &str =
    "# Budget\nhébergement = 100,52 €\ndomaines = 4,99 €\nmensuel = total\ndistance + 3 Go";

/// A sheet holding `text`, open in the editor and computed.
fn opened(text: &str) -> (Harness, String) {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "budget").id.clone();
    let patch = NotePatch {
        kind: Some(NoteKind::Calc),
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

fn values(harness: &Harness, id: &str) -> Vec<String> {
    harness
        .get_all_by(|node| node.author_id() == Some(id))
        .map(|node| node.value().unwrap_or_default())
        .collect()
}

fn value(harness: &Harness, id: &str) -> String {
    harness
        .get_by(|node| node.author_id() == Some(id))
        .value()
        .unwrap_or_default()
}

fn click_result(harness: &mut Harness, at: usize, modifiers: Modifiers) {
    harness
        .get_all_by(|node| node.author_id() == Some("calc-result"))
        .nth(at)
        .unwrap()
        .click_modifiers(modifiers);
    settle(harness);
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

#[test]
fn each_line_shows_its_result_and_an_error_stays_on_its_line() {
    let (harness, _) = opened(SHEET);

    let results = values(&harness, "calc-result");
    assert_eq!(results.len(), 3);
    assert!(results[0].contains("100.52"), "{results:?}");
    assert_eq!(values(&harness, "calc-failure").len(), 1);
}

#[test]
fn the_footer_counts_lines_and_names() {
    let (harness, _) = opened(SHEET);

    let counts = value(&harness, "calc-counts");
    assert!(counts.starts_with("5 lines"), "{counts}");
    assert!(counts.contains("3 variables"), "{counts}");
}

#[test]
fn a_result_clicked_is_summed_and_clicked_again_is_let_go() {
    let (mut harness, _) = opened(SHEET);

    click_result(&mut harness, 0, Modifiers::NONE);
    let said = value(&harness, "calc-summary");
    assert!(said.starts_with("1 line selected"), "{said}");
    assert!(said.contains("sum"), "{said}");

    click_result(&mut harness, 0, Modifiers::NONE);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("calc-summary"))
            .is_none()
    );
}

#[test]
fn ctrl_adds_a_result_and_shift_takes_a_range() {
    let (mut harness, _) = opened("1\n2\n3");

    click_result(&mut harness, 0, Modifiers::NONE);
    click_result(&mut harness, 2, Modifiers::COMMAND);
    let said = value(&harness, "calc-summary");
    assert!(said.starts_with("2 lines selected"), "{said}");
    assert!(said.contains("sum 4"), "{said}");

    click_result(&mut harness, 0, Modifiers::NONE);
    click_result(&mut harness, 0, Modifiers::NONE);
    click_result(&mut harness, 0, Modifiers::NONE);
    click_result(&mut harness, 2, Modifiers::SHIFT);
    let said = value(&harness, "calc-summary");
    assert!(said.contains("sum 6"), "{said}");
    assert!(said.contains("mean 2"), "{said}");
}

#[test]
fn different_units_do_not_add_up() {
    let (mut harness, _) = opened("3 €\n4 Go");

    click_result(&mut harness, 0, Modifiers::NONE);
    click_result(&mut harness, 1, Modifiers::COMMAND);

    assert!(value(&harness, "calc-summary").contains("different units"));
}

#[test]
fn what_is_typed_is_computed_and_written_on_close() {
    let (mut harness, id) = opened("1 + 1");
    click_id(&mut harness, "editor-body");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::End);
    harness.key_press(Key::Enter);
    harness
        .get_by(|node| node.author_id() == Some("editor-body"))
        .type_text("40 + 2");
    settle(&mut harness);
    // Past the pause, on the harness's clock.
    for _ in 0..3 {
        harness.step();
    }
    settle(&mut harness);

    assert!(values(&harness, "calc-result").contains(&"42".to_owned()));
    click_id(&mut harness, "editor-close");
    settle(&mut harness);
    assert_eq!(stored(&harness, &id), "1 + 1\n40 + 2");
}

#[test]
fn a_result_is_copied_whole_and_the_footer_says_so() {
    let (mut harness, _) = opened("100,5 €");

    harness
        .get_by(|node| node.author_id() == Some("calc-result"))
        .hover();
    settle(&mut harness);
    harness
        .get_by(|node| node.author_id() == Some("calc-copy"))
        .click();
    let mut copied = false;
    for _ in 0..4 {
        harness.step();
        copied |= harness.output().platform_output.commands.iter().any(|command| {
            matches!(command, egui::OutputCommand::CopyText(text) if text.starts_with("100.5"))
        });
    }
    assert!(copied);
    settle(&mut harness);
    assert!(value(&harness, "calc-summary").starts_with("Copied"));
}

#[test]
fn the_copy_icon_waits_for_the_pointer() {
    let (mut harness, _) = opened("1\n2");
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("calc-copy"))
            .is_none()
    );

    harness
        .get_all_by(|node| node.author_id() == Some("calc-result"))
        .next()
        .unwrap()
        .hover();
    settle(&mut harness);

    assert_eq!(
        harness
            .get_all_by(|node| node.author_id() == Some("calc-copy"))
            .count(),
        1
    );
}

/// The note's own copy, not a result's: its results follow as comments.
#[test]
fn copying_the_sheet_writes_its_results_as_comments() {
    let (mut harness, _) = opened("2 + 2\nune phrase");

    harness
        .get_by(|node| node.author_id() == Some("editor-copy"))
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

    assert_eq!(copied.as_deref(), Some("2 + 2 // 4\nune phrase"));
}

#[test]
fn an_offset_is_read_in_utf16_units() {
    assert_eq!(char_at("a😀b", 3), Some(2));
    assert_eq!(char_at("abc", 3), Some(3));
    assert_eq!(char_at("abc", 9), None);
}

#[test]
fn where_a_name_is_read_is_said() {
    use i18n::Language;

    assert_eq!(used_on(&[], Language::En), "never used");
    assert_eq!(used_on(&[2], Language::En), "used on line 3");
    assert_eq!(used_on(&[2, 4], Language::En), "used on lines 3, 5");
}
