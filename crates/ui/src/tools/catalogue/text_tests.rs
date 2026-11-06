//! The text tools over a real core: what is typed in, what comes out.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use crate::notes::bench::{Bench, area_of, settle, until};
use crate::state::Area;
use crate::testing;

const WIDE: egui::Vec2 = egui::vec2(1_300.0, 900.0);

pub(super) fn opened(tool: &str) -> Harness<'static, Bench> {
    let (backend, profile) = testing::unlocked();
    let mut harness = area_of(Area::Tools, backend, profile);
    harness
        .ctx
        .set_missing_glyph_policy(egui::epaint::text::MissingGlyphPolicy::Tofu);
    harness.set_size(WIDE);
    settle(&mut harness);
    let bench = harness.state_mut();
    let asked = crate::action::Action::Tools(crate::tools::Action::Open(
        crate::tools::catalogue::find(tool).unwrap().id,
    ));
    let ctx = egui::Context::default();
    bench.state.apply(asked, &ctx, &bench.backend);
    settle(&mut harness);
    harness
}

pub(super) fn click(harness: &mut Harness<'_, Bench>, id: &str) {
    harness.get_by(|node| node.author_id() == Some(id)).click();
    settle(harness);
}

pub(super) fn type_in(harness: &mut Harness<'_, Bench>, id: &str, text: &str) {
    harness.get_by(|node| node.author_id() == Some(id)).focus();
    harness.step();
    harness
        .get_by(|node| node.author_id() == Some(id))
        .type_text(text);
    settle(harness);
}

pub(super) fn has(harness: &Harness<'_, Bench>, id: &str) -> bool {
    harness
        .query_all_by(|node| node.author_id() == Some(id))
        .next()
        .is_some()
}

pub(super) fn value_of(harness: &Harness<'_, Bench>, id: &str) -> String {
    let node = harness.get_by(|node| node.author_id() == Some(id));
    let node = node.accesskit_node();
    node.value().or_else(|| node.label()).unwrap_or_default()
}

pub(super) fn output_values(harness: &Harness<'_, Bench>) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some("output-value"))
        .map(|node| node.accesskit_node().value().unwrap_or_default())
        .collect()
}

/// Until the debounce has passed and the core answered.
pub(super) fn waits_for(
    harness: &mut Harness<'_, Bench>,
    what: &str,
    holds: impl Fn(&Harness<'_, Bench>) -> bool,
) {
    until(harness, what, holds);
}

#[test]
fn the_case_converter_writes_a_text_in_every_case_and_its_slug() {
    let mut harness = opened("case");

    type_in(&mut harness, "case-input", "parse URL");
    waits_for(&mut harness, "the cases are shown", |harness| {
        !output_values(harness).is_empty()
    });

    let values = output_values(&harness);
    assert!(values.contains(&"parseUrl".to_owned()), "{values:?}");
    assert!(values.contains(&"parse_url".to_owned()));
    assert!(values.contains(&"parse-url".to_owned()));
}

#[test]
fn the_case_converters_example_fills_its_input_and_clear_empties_it() {
    let mut harness = opened("case");

    click(&mut harness, "tool-sample");
    assert!(!value_of(&harness, "case-input").is_empty());

    click(&mut harness, "tool-clear");
    assert_eq!(value_of(&harness, "case-input"), "");
}

#[test]
fn line_breaks_count_the_endings_and_rewrite_them() {
    let mut harness = opened("line-breaks");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the endings are counted", |harness| {
        has(harness, "line-breaks-found")
    });
    assert!(value_of(&harness, "line-breaks-found").contains("CRLF 2"));

    assert!(value_of(&harness, "line-breaks-changes").contains('2'));
}

#[test]
fn lorem_opens_on_three_paragraphs_and_draws_another_text() {
    let mut harness = opened("lorem");
    waits_for(&mut harness, "a text is drawn", |harness| {
        has(harness, "lorem-output")
    });
    let first = value_of(&harness, "lorem-output");
    assert!(first.starts_with("Lorem ipsum"));

    click(&mut harness, "lorem-again");
    waits_for(&mut harness, "another text is drawn", |harness| {
        value_of(harness, "lorem-output") != first
    });
}

#[test]
fn text_statistics_count_the_characters_and_their_frequency() {
    let mut harness = opened("text-stats");

    type_in(&mut harness, "text-stats-input", "été a");
    waits_for(&mut harness, "the text is counted", |harness| {
        has(harness, "text-stats-count-words")
    });

    assert_eq!(value_of(&harness, "text-stats-count-words"), "2");
    assert_eq!(value_of(&harness, "text-stats-count-characters"), "5");
    assert!(has(&harness, "text-stats-frequency"));
}

#[test]
fn the_text_diff_counts_what_changed_and_swaps_its_sides() {
    let mut harness = opened("diff");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the texts are compared", |harness| {
        has(harness, "text-diff-rows")
    });
    assert_eq!(value_of(&harness, "text-diff-added"), "+ 4 lines");
    let added = value_of(&harness, "text-diff-added");

    click(&mut harness, "tool-action-swap");
    waits_for(&mut harness, "the sides have changed", |harness| {
        value_of(harness, "text-diff-added") != added
    });
}

#[test]
fn identical_texts_are_said_to_be() {
    let mut harness = opened("diff");

    type_in(&mut harness, "text-diff-a", "same");
    type_in(&mut harness, "text-diff-b", "same");

    waits_for(&mut harness, "the texts are compared", |harness| {
        has(harness, "text-diff-identical")
    });
}

#[test]
fn the_structured_diff_reads_two_formats_and_lists_the_value_that_changed() {
    let mut harness = opened("diff");
    click(&mut harness, "diff-mode-1");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the documents are compared", |harness| {
        has(harness, "structured-diff-change")
    });

    assert_eq!(value_of(&harness, "structured-diff-read-a"), "read as YAML");
    assert_eq!(value_of(&harness, "structured-diff-read-b"), "read as JSON");
    assert_eq!(value_of(&harness, "structured-diff-change"), "$.version");
}
