//! What the review against the reference put back, over a real core (plan § 5.3).

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::http::model::{HttpMethod, RequestKind};
use engine::notes::kind::NoteKind;
use engine::notes::model::NotePatch;

use crate::backend::Backend;
use crate::notes::bench::{Bench, area, cards, click, click_id, settle, until};
use crate::state::Area;
use crate::testing;

fn present(harness: &Harness<'_, Bench>, author_id: &str) -> bool {
    harness
        .query_by(|node| node.author_id() == Some(author_id))
        .is_some()
}

fn rect(harness: &Harness<'_, Bench>, author_id: &str) -> egui::Rect {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .rect()
}

fn hover_first_card(harness: &mut Harness<'_, Bench>) {
    harness
        .get_by(|node| node.author_id() == Some("note-card-open"))
        .hover();
    settle(harness);
}

/// `title`, of `kind`, holding `content`, through the core.
fn note(backend: &Backend, title: &str, kind: NoteKind, content: &str) -> String {
    let id = testing::note(backend, title).id.clone();
    let patch = NotePatch {
        kind: Some(kind),
        content: Some(content.to_owned()),
        ..NotePatch::default()
    };
    let at = id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, at, patch)
    })
    .unwrap();
    id
}

#[test]
fn the_controls_of_the_header_stand_on_one_line() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let harness = area(backend, profile);

    let middle = rect(&harness, "new-note").center().y;
    for author_id in ["sort-trigger", "facets-toggle", "quick-filter-0"] {
        let off = (rect(&harness, author_id).center().y - middle).abs();
        assert!(
            off <= 1.0,
            "{author_id} is {off} away from the row's middle"
        );
    }
}

#[test]
fn a_card_offers_its_box_under_the_pointer_and_its_title_stays_where_it_is() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    let left = rect(&harness, "note-card-title").left();
    assert!(!present(&harness, "note-card-check"));

    hover_first_card(&mut harness);

    assert!((rect(&harness, "note-card-title").left() - left).abs() < 0.5);
    click_id(&mut harness, "note-card-check");
    assert_eq!(harness.state().state.selection.checked_in(&[]).len(), 0);
    assert!(present(&harness, "selection-count"));
}

#[test]
fn a_ticked_card_shows_its_box_before_its_title() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    hover_first_card(&mut harness);
    click_id(&mut harness, "note-card-check");

    harness.event(egui::Event::PointerGone);
    settle(&mut harness);

    assert!(present(&harness, "note-card-check"));
}

#[test]
fn a_search_that_finds_nothing_offers_to_clear_it() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    click_id(&mut harness, "search-input");
    harness
        .get_by(|node| node.author_id() == Some("search-input"))
        .type_text("zzzz");
    until(&mut harness, "the search found nothing", |harness| {
        present(harness, "canvas-clear-filters")
    });

    click_id(&mut harness, "canvas-clear-filters");

    until(&mut harness, "the card is back", |harness| {
        cards(harness) == 1
    });
}

#[test]
fn a_selection_is_copied_from_its_bar() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    hover_first_card(&mut harness);
    click_id(&mut harness, "note-card-check");

    click_id(&mut harness, "selection-copy");
    settle(&mut harness);

    assert!(present(&harness, "selection-count"));
}

#[test]
fn the_new_note_menu_says_new_until_a_note_was_made() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    assert!(!harness.state().state.settings.note_kind_tried);

    click_id(&mut harness, "new-note-kind");
    click_id(&mut harness, "new-note-note");
    settle(&mut harness);

    assert!(harness.state().state.settings.note_kind_tried);
}

#[test]
fn the_sort_menu_marks_what_is_in_force_and_stays_narrow() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "sort-trigger");

    let widest = harness
        .query_all_by(|node| node.author_id() == Some("sort-key"))
        .map(|node| node.rect().width())
        .fold(0.0, f32::max);
    assert!(widest < 240.0, "the menu is {widest} wide");
}

#[test]
fn a_card_menu_draws_its_entries_and_deletes_in_two_steps() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    hover_first_card(&mut harness);

    click_id(&mut harness, "note-card-menu");

    for author_id in [
        "note-card-open-menu",
        "note-card-pin-toggle",
        "note-card-priority",
        "note-card-duplicate",
        "note-card-delete",
    ] {
        assert!(present(&harness, author_id), "{author_id} is not offered");
    }
}

fn open_palette(harness: &mut Harness<'_, Bench>) {
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::ALT, Key::P);
    settle(harness);
}

fn options(harness: &Harness<'_, Bench>) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some("palette-option"))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect()
}

fn type_in_palette(harness: &mut Harness<'_, Bench>, words: &str) {
    harness
        .get_by(|node| node.author_id() == Some("palette-input"))
        .type_text(words);
    for _ in 0..20 {
        harness.step();
    }
    settle(harness);
}

fn with_a_request() -> Harness<'static, Bench> {
    let (backend, profile) = testing::unlocked();
    let api = crate::http::tests::collection(&backend, "API");
    crate::http::tests::request(
        &backend,
        (&api, None),
        "Lister",
        (RequestKind::Http, HttpMethod::Get),
    );
    area(backend, profile)
}

#[test]
fn the_palette_recalls_the_requests_and_opens_one_in_its_area() {
    let mut harness = with_a_request();
    open_palette(&mut harness);
    until(&mut harness, "the request is listed", |harness| {
        options(harness).iter().any(|name| name == "Lister")
    });

    type_in_palette(&mut harness, "Lister");
    until(&mut harness, "the request alone is found", |harness| {
        options(harness)
            .first()
            .is_some_and(|name| name == "Lister")
    });
    harness.key_press(Key::Enter);
    settle(&mut harness);

    let state = &harness.state().state;
    assert_eq!(state.settings.area, Area::Http);
    assert!(!state.palette.is_open());
}

#[test]
fn the_palette_finds_a_tool_by_its_name_first_and_opens_it() {
    let (backend, profile) = testing::unlocked();
    let mut harness = area(backend, profile);
    open_palette(&mut harness);

    type_in_palette(&mut harness, "json");
    until(&mut harness, "the tools are found", |harness| {
        !options(harness).is_empty()
    });
    let first = options(&harness).remove(0);
    assert!(first.to_lowercase().contains("json"), "{first} comes first");
    harness.key_press(Key::Enter);
    settle(&mut harness);

    let state = &harness.state().state;
    assert_eq!(state.settings.area, Area::Tools);
    assert!(state.tools.open_tool().is_some());
}

#[test]
fn the_palette_narrows_to_requests_and_to_tools_with_a_chip() {
    let mut harness = with_a_request();
    open_palette(&mut harness);

    type_in_palette(&mut harness, "@requests");
    harness.key_press(Key::Tab);
    settle(&mut harness);
    until(&mut harness, "only the request is left", |harness| {
        options(harness) == ["Lister"]
    });

    harness.key_press(Key::Backspace);
    settle(&mut harness);
    type_in_palette(&mut harness, "@tools");
    harness.key_press(Key::Tab);
    settle(&mut harness);
    until(&mut harness, "the tools are listed", |harness| {
        options(harness).len() >= 3
    });
}

#[test]
fn the_palette_runs_the_commands_of_the_other_areas() {
    let (backend, profile) = testing::unlocked();
    let mut harness = area(backend, profile);
    open_palette(&mut harness);

    type_in_palette(&mut harness, "> new request");
    until(&mut harness, "the command is offered", |harness| {
        options(harness).len() == 1
    });
    harness.key_press(Key::Enter);
    settle(&mut harness);

    assert_eq!(harness.state().state.settings.area, Area::Http);
}

#[test]
fn the_palette_says_how_many_fields_a_note_waits_for() {
    let (backend, profile) = testing::unlocked();
    note(&backend, "psql", NoteKind::Snippet, "psql -h {{host}}");
    let mut harness = area(backend, profile);
    open_palette(&mut harness);

    until(&mut harness, "the note is recalled", |harness| {
        options(harness).iter().any(|name| name == "psql")
    });
}

#[test]
fn the_palette_recalls_a_sheet_with_a_short_date_before_an_accent() {
    let (backend, profile) = testing::unlocked();
    note(&backend, "Agenda", NoteKind::Calc, "RDV le 5/11/24 à 10h");
    let mut harness = area(backend, profile);
    open_palette(&mut harness);

    until(&mut harness, "the sheet is recalled", |harness| {
        options(harness).iter().any(|name| name == "Agenda")
    });
}

#[test]
fn a_drawing_is_inserted_from_the_toolbar_of_a_note() {
    let (backend, profile) = testing::unlocked();
    let written = note(&backend, "Procédure", NoteKind::Note, "# Titre");
    let scene = testing::note(&backend, "Schéma").id.clone();
    let patch = NotePatch {
        kind: Some(NoteKind::Drawing),
        ..NotePatch::default()
    };
    let at = scene.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, at, patch)
    })
    .unwrap();
    let mut harness = crate::notes::bench::expanded_area(backend, profile);
    click(&mut harness, "note-card-open", "Procédure");
    until(&mut harness, "the toolbar offers the drawing", |harness| {
        present(harness, "rich-embed-drawing")
    });

    click_id(&mut harness, "rich-embed-drawing");
    click_id(&mut harness, "rich-embed-drawing-0");
    click_id(&mut harness, "editor-close");
    settle(&mut harness);

    let backend = &harness.state().backend;
    let stored = engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::get_note(engine, written)
    })
    .unwrap();
    assert!(
        stored
            .content
            .contains(&format!("devnotes:drawing/{scene}"))
    );
}

#[test]
fn a_sheet_sets_its_decimals_from_its_footer() {
    let (backend, profile) = testing::unlocked();
    note(&backend, "Budget", NoteKind::Calc, "total = 10 + 3");
    let mut harness = area(backend, profile);
    click(&mut harness, "note-card-open", "Budget");
    until(&mut harness, "the sheet is open", |harness| {
        present(harness, "calc-decimals")
    });

    click_id(&mut harness, "calc-decimals");
    click_id(&mut harness, "calc-decimals-3");

    assert_eq!(harness.state().state.settings.calc_decimals, Some(2));
}
