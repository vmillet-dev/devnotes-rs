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

fn hide_the_rail(harness: &mut egui_kittest::Harness<'_, crate::notes::bench::Bench>) {
    click_id(harness, "library-rail-toggle");
    assert!(!harness.state().state.settings.rail_shown);
}

#[test]
fn the_switchers_take_the_rails_place_when_it_is_put_away() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "in home");
    let work = crate::notes::bench::space(&backend, "Work");
    let mut harness = area(backend, profile);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("space-switcher"))
            .is_none()
    );

    hide_the_rail(&mut harness);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("space-option"))
            .is_none()
    );
    click_id(&mut harness, "space-switcher");
    click(&mut harness, "space-option", "Work");

    assert_eq!(
        harness.state().state.spaces.active_space(),
        Some(work.id.as_str())
    );
    let name = harness
        .get_by(|node| node.author_id() == Some("space-switcher-name"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert_eq!(name, "Work");
    assert_eq!(cards(&harness), 0);
}

#[test]
fn a_space_is_made_from_the_switcher() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "in home");
    let mut harness = area(backend, profile);
    hide_the_rail(&mut harness);

    click_id(&mut harness, "space-switcher");
    click_id(&mut harness, "space-create-open");
    crate::notes::bench::type_and_enter(&mut harness, "space-create-input", "Reading");

    let spaces = crate::notes::bench::spaces_of(&harness);
    assert!(spaces.iter().any(|space| space.name == "Reading"));
}

#[test]
fn a_folder_open_is_a_breadcrumb_back_to_its_space() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "in home");
    let home = crate::notes::bench::spaces_of_backend(&backend)[0].clone();
    let runbooks = crate::notes::bench::folder(&backend, &home, None, "Runbooks");
    crate::notes::bench::folder(&backend, &home, Some(&runbooks), "api");
    let mut harness = area(backend, profile);
    hide_the_rail(&mut harness);

    click_id(&mut harness, "folder-switcher");
    click(&mut harness, "folder-option", "api");

    let here = harness
        .get_by(|node| node.author_id() == Some("folder-breadcrumb-name"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert_eq!(here, "api");
    click(&mut harness, "folder-breadcrumb-step", "Runbooks");
    assert_eq!(
        harness.state().state.spaces.active_folder(),
        Some(runbooks.id.as_str())
    );
    click_id(&mut harness, "folder-breadcrumb-back");
    assert_eq!(harness.state().state.spaces.active_folder(), None);
    harness.get_by(|node| node.author_id() == Some("folder-switcher"));
}

#[test]
fn the_kind_menu_opens_a_draft_of_its_kind() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "in home");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "new-note-kind");
    click_id(&mut harness, "new-note-checklist");

    let opened = harness.state().state.editor.opened().unwrap();
    assert_eq!(
        opened.note().unwrap().kind,
        engine::notes::kind::NoteKind::Checklist
    );
    harness.get_by(|node| node.author_id() == Some("editor-title"));
}
