//! The board over a real core, drawn as the window draws it.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::folders::model::Folder;
use engine::spaces::model::Space;

use crate::notes::bench::{Bench, area, cards, click, click_id, folder, names, remembered, space};
use crate::state::board::Mode;
use crate::testing;

/// In `space`, filed in `into` when one is given.
fn note_in(
    harness_backend: &crate::backend::Backend,
    space: &Space,
    title: &str,
    into: Option<&Folder>,
) -> String {
    let draft = testing::snippet(space.id.clone(), title.into());
    let folder = into.map(|folder| folder.id.clone());
    engine::db::run_here(harness_backend.engine(), move |engine| {
        let note = engine::notes::create_note(engine, draft)?;
        if folder.is_some() {
            engine::folders::file_notes(engine, vec![note.id.clone()], folder)?;
        }
        Ok::<_, engine::error::AppError>(note.id.clone())
    })
    .unwrap()
}

/// « Tableau »: two folders, a note filed in the first, a loose one.
fn tableau() -> (Harness<'static, Bench>, Space) {
    let (backend, profile) = testing::unlocked();
    space(&backend, "Accueil");
    let tableau = space(&backend, "Tableau");
    let perf = folder(&backend, &tableau, None, "Perf");
    folder(&backend, &tableau, None, "Migrations");
    note_in(&backend, &tableau, "EXPLAIN lent sur join", Some(&perf));
    note_in(&backend, &tableau, "Dump nocturne", None);
    let mut harness = area(backend, profile);
    click(&mut harness, "space-option", "Tableau");
    (harness, tableau)
}

fn rect_of(harness: &Harness<'_, Bench>, author_id: &str, name: &str) -> egui::Rect {
    harness
        .get_by(|node| node.author_id() == Some(author_id) && node.label().as_deref() == Some(name))
        .rect()
}

/// The titles of the cards drawn inside `zone`'s frame.
fn titles_in(harness: &Harness<'_, Bench>, zone: &str) -> Vec<String> {
    let frame = rect_of(harness, "board-zone", zone);
    harness
        .query_all_by(|node| node.author_id() == Some("note-card-title"))
        .filter(|node| frame.contains_rect(node.rect()))
        .map(|node| {
            let node = node.accesskit_node();
            node.label().or_else(|| node.value()).unwrap_or_default()
        })
        .collect()
}

#[test]
fn the_board_draws_each_folder_as_a_zone_holding_its_notes_the_loose_ones_beside() {
    let (mut harness, tableau) = tableau();
    assert!(
        harness.state().state.board_place().is_none(),
        "the list is the default"
    );

    click_id(&mut harness, "view-board");

    let mut zones = names(&harness, "board-zone-open");
    zones.sort();
    assert_eq!(zones, ["Migrations", "Perf"]);
    assert_eq!(titles_in(&harness, "Perf"), ["EXPLAIN lent sur join"]);
    assert!(titles_in(&harness, "Migrations").is_empty());
    assert_eq!(names(&harness, "board-loose-card"), ["Dump nocturne"]);
    let perf = rect_of(&harness, "board-zone", "Perf");
    let migrations = rect_of(&harness, "board-zone", "Migrations");
    assert!(!perf.intersects(migrations), "laid out beside, not on top");
    assert_eq!(
        remembered(&harness, &format!("devnotes.notes.view.{}", tableau.id)).as_deref(),
        Some("board")
    );
}

#[test]
fn the_view_is_the_space_s_own_and_the_list_is_left_as_it_was() {
    let (mut harness, tableau) = tableau();
    click_id(&mut harness, "view-board");
    click(&mut harness, "space-option", "Accueil");
    assert!(harness.state().state.board_place().is_none());
    click(&mut harness, "space-option", "Tableau");
    assert!(harness.state().state.board_place().is_some());

    click_id(&mut harness, "view-list");
    assert_eq!(cards(&harness), 2);
    let preferences = &harness.state().state.library_preferences;
    assert_eq!(Mode::read(preferences, &tableau.id), Mode::List);
}

#[test]
fn a_zone_title_and_a_sub_folder_chip_open_their_folder() {
    let (backend, profile) = testing::unlocked();
    let tableau = space(&backend, "Tableau");
    let base = folder(&backend, &tableau, None, "Base");
    let perf = folder(&backend, &tableau, Some(&base), "Perf");
    note_in(&backend, &tableau, "Index partiel", Some(&perf));
    let mut harness = area(backend, profile);
    click(&mut harness, "space-option", "Tableau");
    click_id(&mut harness, "view-board");
    assert_eq!(
        titles_in(&harness, "Base"),
        ["Index partiel"],
        "a zone gathers its sub-tree"
    );

    click(&mut harness, "board-zone-child", "Perf");
    assert_eq!(
        harness.state().state.spaces.active_folder(),
        Some(perf.id.as_str())
    );
    assert!(
        harness.state().state.board_place().is_none(),
        "a folder of none is a grid"
    );

    click(&mut harness, "folder-breadcrumb-back", "‹ Tableau");
    click(&mut harness, "board-zone-open", "Base");
    let place = harness.state().state.board_place().unwrap();
    assert_eq!(place.root_id.as_deref(), Some(base.id.as_str()));
    assert_eq!(names(&harness, "board-zone-open"), ["Perf"]);
}

#[test]
fn only_what_crosses_the_viewport_is_drawn() {
    let (backend, profile) = testing::unlocked();
    let tableau = space(&backend, "Tableau");
    for at in 0..60 {
        note_in(&backend, &tableau, &format!("Note {at}"), None);
    }
    let mut harness = area(backend, profile);
    click(&mut harness, "space-option", "Tableau");
    click_id(&mut harness, "view-board");

    let drawn = harness
        .query_all_by(|node| node.author_id() == Some("board-loose-card"))
        .count();
    assert!(drawn > 0 && drawn < 60, "{drawn} cards drawn");
    assert_eq!(harness.state().state.board.cards().count(), 60);
}

/// The budget of § 6.2, measured by hand: `cargo test -p ui --release board_budget --
/// --ignored --nocapture`. A frame is egui's pass, without painting.
#[test]
#[ignore = "a measure, not a check"]
fn board_budget() {
    let (backend, profile) = testing::unlocked();
    testing::corpus(&backend, 3_000);
    let mut harness = area(backend, profile);
    click(&mut harness, "space-option", "Corpus");
    click_id(&mut harness, "view-board");
    let centre = rect_of(&harness, "board", "board").center();
    harness.event(egui::Event::PointerMoved(centre));
    let mut frames = Vec::new();
    for _ in 0..200 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -80.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
        });
        let started = std::time::Instant::now();
        harness.step();
        frames.push(started.elapsed());
    }
    frames.sort();
    println!(
        "3,000 cards: scrolling p50 {:?}, p95 {:?}, p99 {:?}",
        frames[100], frames[190], frames[198]
    );
}

/// Whether the seat of the card titled `title` is lit.
fn lit(harness: &Harness<'_, Bench>, title: &str) -> bool {
    !harness
        .get_by(|node| {
            matches!(
                node.author_id(),
                Some("board-loose-card" | "board-zone-card")
            ) && node.label().as_deref() == Some(title)
        })
        .accesskit_node()
        .is_disabled()
}

#[test]
fn a_search_dims_what_it_does_not_match_and_moves_nothing() {
    let (mut harness, _) = tableau();
    click_id(&mut harness, "view-board");
    let offset = |harness: &Harness<'_, Bench>| {
        rect_of(harness, "board-loose-card", "Dump nocturne").min
            - rect_of(harness, "board-zone", "Perf").min
    };
    let before = offset(&harness);

    click_id(&mut harness, "search-input");
    harness
        .get_by(|node| node.author_id() == Some("search-input"))
        .type_text("EXPLAIN");
    crate::notes::bench::until(&mut harness, "the board dims", |harness| {
        !lit(harness, "Dump nocturne")
    });

    assert!(lit(&harness, "EXPLAIN lent sur join"));
    assert_eq!(titles_in(&harness, "Perf"), ["EXPLAIN lent sur join"]);
    assert_eq!(offset(&harness), before, "dimmed in its seat");
    click_id(&mut harness, "search-clear");
    crate::notes::bench::until(&mut harness, "the board lights again", |harness| {
        lit(harness, "Dump nocturne")
    });
}

/// The zone of `name`, far off to the right, as a hand would have left it.
fn far_off(harness: &Harness<'_, Bench>, space: &Space, name: &str) {
    let space_id = space.id.clone();
    let name = name.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        let folder = engine::folders::list_folders(engine, Some(space_id))?
            .into_iter()
            .find(|folder| folder.name == name)
            .unwrap();
        let zone = engine::folders::board::ZonePlacement {
            folder_id: folder.id,
            frame: engine::folders::board::BoardFrame {
                x: 1_600,
                y: 40,
                width: 300,
                height: 220,
            },
        };
        engine::folders::save_board_layout(engine, None, vec![zone], Vec::new())
    })
    .unwrap();
}

#[test]
fn aligning_leaves_the_zones_and_reorganising_brings_them_back_undoably() {
    let (mut harness, tableau) = tableau();
    click_id(&mut harness, "view-board");
    let seat = rect_of(&harness, "board-zone", "Perf");
    far_off(&harness, &tableau, "Perf");
    harness.state_mut().state.revision.bump();
    crate::notes::bench::settle(&mut harness);
    let zone_x =
        |harness: &Harness<'_, Bench>| harness.state().state.board.view().unwrap().zones[0].frame.x;
    assert_eq!(zone_x(&harness), 1_600);

    click_id(&mut harness, "board-tidy");
    assert_eq!(zone_x(&harness), 1_600, "aligning moves no zone");

    click_id(&mut harness, "board-tidy-more");
    click_id(&mut harness, "board-tidy-everything");
    assert_ne!(zone_x(&harness), 1_600);
    assert_eq!(
        rect_of(&harness, "board-zone", "Perf"),
        seat,
        "and the ground pans home"
    );
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("undo-text"))
            .is_some()
    );

    click_id(&mut harness, "undo-restore");
    assert_eq!(zone_x(&harness), 1_600);
}
