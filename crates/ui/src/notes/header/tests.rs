//! The header over a real core: what it asks changes what the canvas shows.

use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::model::NotePatch;
use engine::notes::view::Grouping;

use crate::notes::bench::{area, cards, click, click_id, remembered, settle, until};
use crate::state::notes_query::{ARRANGEMENT_KEY, Arrangement};
use crate::testing;

fn headings(harness: &egui_kittest::Harness<'_, crate::notes::bench::Bench>) -> usize {
    harness
        .query_all_by(|node| node.author_id() == Some("note-section"))
        .count()
}

#[test]
fn a_search_narrows_the_canvas_once_the_keys_rest_and_its_cross_empties_it() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    testing::note(&backend, "curl health");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "search-input");
    harness
        .get_by(|node| node.author_id() == Some("search-input"))
        .type_text("PSQL");
    until(&mut harness, "the search narrowed the canvas", |harness| {
        cards(harness) == 1
    });

    let matched = harness
        .get_by(|node| node.author_id() == Some("search-matched"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert_eq!(matched, "1 result");
    click_id(&mut harness, "search-clear");
    assert_eq!(cards(&harness), 2);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("search-matched"))
            .is_none()
    );
}

#[test]
fn nothing_found_is_still_a_count() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "search-input");
    harness
        .get_by(|node| node.author_id() == Some("search-input"))
        .type_text("nothing matches this");
    until(&mut harness, "the canvas says nothing matches", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("canvas-no-results"))
            .is_some()
    });

    let matched = harness
        .get_by(|node| node.author_id() == Some("search-matched"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert_eq!(matched, "no results");
}

#[test]
fn the_quick_filter_keeps_the_pinned_notes() {
    let (backend, profile) = testing::unlocked();
    let pinned = testing::note(&backend, "pinned").id.clone();
    testing::note(&backend, "loose");
    let patch = NotePatch {
        pinned: Some(true),
        ..NotePatch::default()
    };
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, pinned, patch)
    })
    .unwrap();
    let mut harness = area(backend, profile);
    assert_eq!(cards(&harness), 2);

    click(&mut harness, "quick-filter-1", "Pinned");
    assert_eq!(cards(&harness), 1);
    click(&mut harness, "quick-filter-0", "All");
    assert_eq!(cards(&harness), 2);
}

#[test]
fn a_grouping_chosen_lays_the_canvas_out_and_is_the_librarys() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    assert_eq!(headings(&harness), 2);

    click_id(&mut harness, "sort-trigger");
    click(&mut harness, "sort-grouping", "No grouping");

    assert!(headings(&harness) < 2);
    // The menu stays open: order, direction and grouping are set together.
    harness.get_by(|node| node.author_id() == Some("sort-pinned-first"));
    let mut stored = crate::services::preferences::Preferences::default();
    stored.set(
        ARRANGEMENT_KEY,
        &remembered(&harness, ARRANGEMENT_KEY).unwrap(),
    );
    assert_eq!(Arrangement::read(&stored).grouping, Grouping::None);
}

#[test]
fn ctrl_k_puts_the_caret_in_the_search() {
    let (backend, profile) = testing::unlocked();
    let mut harness = area(backend, profile);

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    settle(&mut harness);

    assert!(
        harness
            .get_by(|node| node.author_id() == Some("search-input"))
            .is_focused()
    );
}

#[test]
fn the_board_is_not_offered_yet() {
    let (backend, profile) = testing::unlocked();
    let harness = area(backend, profile);

    let board = harness.get_by(|node| node.author_id() == Some("view-board"));
    assert!(board.accesskit_node().is_disabled());
}
