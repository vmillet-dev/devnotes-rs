//! The board over a real core, drawn as the window draws it.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::folders::model::Folder;
use engine::spaces::model::Space;

use crate::notes::bench::{
    Bench, area, cards, click, click_id, folder, names, remembered, space, until,
};
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
            engine::folders::file_notes(
                engine,
                vec![note.id.clone()],
                folder,
                engine::folders::board::PreviewLines::default(),
            )?;
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
    // Wide enough for both zones whole: two folders made in one millisecond come in either
    // order, and the second's ⋯ and corner must be on screen whichever it is.
    harness.set_size(egui::vec2(1_600.0, 900.0));
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
        harness.state().state.active_folder(),
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
    until(&mut harness, "the board dims", |harness| {
        !lit(harness, "Dump nocturne")
    });

    assert!(lit(&harness, "EXPLAIN lent sur join"));
    assert_eq!(titles_in(&harness, "Perf"), ["EXPLAIN lent sur join"]);
    assert_eq!(offset(&harness), before, "dimmed in its seat");
    click_id(&mut harness, "search-clear");
    until(&mut harness, "the board lights again", |harness| {
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
    let zone_x = |harness: &Harness<'_, Bench>| frame_of(harness, "Perf").x;
    // Waited for, not settled: under load the view is read again a frame or two later.
    until(&mut harness, "the zone is read far off", |harness| {
        zone_x(harness) == 1_600
    });

    click_id(&mut harness, "board-tidy");
    assert_eq!(zone_x(&harness), 1_600, "aligning moves no zone");

    click_id(&mut harness, "board-tidy-more");
    click_id(&mut harness, "board-tidy-everything");
    until(&mut harness, "the zone is back at its seat", |harness| {
        zone_x(harness) != 1_600
    });
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
    until(&mut harness, "the hand's layout is back", |harness| {
        zone_x(harness) == 1_600
    });
}

/// A press at `from`, the pointer moved in steps, a release at `to`: as a hand drags.
fn drag(
    harness: &mut Harness<'_, Bench>,
    from: egui::Pos2,
    to: egui::Pos2,
    button: egui::PointerButton,
) {
    let press = |pos, pressed| egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.event(egui::Event::PointerMoved(from));
    harness.step();
    harness.event(press(from, true));
    harness.step();
    for step in 1..=6 {
        #[expect(clippy::cast_precision_loss, reason = "six steps")]
        let at = from + (to - from) * (step as f32 / 6.0);
        harness.event(egui::Event::PointerMoved(at));
        harness.step();
    }
    harness.event(press(to, false));
    crate::notes::bench::settle(harness);
    harness.event(egui::Event::PointerGone);
}

/// The folder the core says the note is in, as the board last read it.
fn filed_in(harness: &Harness<'_, Bench>, title: &str) -> Option<String> {
    harness
        .state()
        .state
        .board
        .cards()
        .find(|entry| entry.note.title == title)
        .and_then(|entry| entry.note.folder_id.clone())
}

fn folder_named(harness: &Harness<'_, Bench>, space: &Space, name: &str) -> Folder {
    let space_id = space.id.clone();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::folders::list_folders(engine, Some(space_id))
    })
    .unwrap()
    .into_iter()
    .find(|folder| folder.name == name)
    .unwrap()
}

/// The frame of the zone named `name`, as the board last read it. By name: two folders made in
/// the same millisecond come back in either order.
fn frame_of(harness: &Harness<'_, Bench>, name: &str) -> engine::folders::board::BoardFrame {
    harness
        .state()
        .state
        .board
        .view()
        .unwrap()
        .zones
        .iter()
        .find(|zone| zone.folder.name == name)
        .unwrap()
        .frame
}

/// A zone's own control, by the folder its label names.
fn control_in(harness: &Harness<'_, Bench>, author_id: &str, zone: &str) -> egui::Rect {
    harness
        .get_by(|node| {
            node.author_id() == Some(author_id)
                && node.label().is_some_and(|label| label.contains(zone))
        })
        .rect()
}

#[test]
fn a_card_dragged_into_a_zone_is_filed_there_and_back_out_on_the_ground() {
    let (mut harness, tableau) = tableau();
    click_id(&mut harness, "view-board");
    let card = rect_of(&harness, "board-loose-card", "Dump nocturne");
    let zone = rect_of(&harness, "board-zone", "Migrations");
    let migrations = folder_named(&harness, &tableau, "Migrations");

    drag(
        &mut harness,
        card.center(),
        zone.center(),
        egui::PointerButton::Primary,
    );
    assert_eq!(
        titles_in(&harness, "Migrations"),
        ["Dump nocturne"],
        "drawn there at once"
    );
    until(&mut harness, "the drop is written", |harness| {
        filed_in(harness, "Dump nocturne").as_deref() == Some(migrations.id.as_str())
    });

    let seat = harness
        .get_by(|node| {
            node.author_id() == Some("board-zone-card")
                && node.label().as_deref() == Some("Dump nocturne")
        })
        .rect();
    let ground = seat.center() + egui::vec2(0.0, 360.0);
    drag(
        &mut harness,
        seat.center(),
        ground,
        egui::PointerButton::Primary,
    );
    until(&mut harness, "the card is loose again", |harness| {
        filed_in(harness, "Dump nocturne").is_none()
            && names(harness, "board-loose-card") == ["Dump nocturne"]
    });
}

#[test]
fn a_zone_moves_by_its_grip_and_resizes_by_its_corner_on_the_grid_filing_nothing() {
    let (mut harness, tableau) = tableau();
    click_id(&mut harness, "view-board");
    let before = frame_of(&harness, "Perf");
    let grip = control_in(&harness, "board-zone-grip", "Perf");

    drag(
        &mut harness,
        grip.center(),
        grip.center() + egui::vec2(93.0, 141.0),
        egui::PointerButton::Primary,
    );
    until(&mut harness, "the move is written", |harness| {
        frame_of(harness, "Perf").x != before.x
    });
    let moved = frame_of(&harness, "Perf");
    assert_eq!((moved.width, moved.height), (before.width, before.height));
    assert_eq!((moved.x % 20, moved.y % 20), (0, 0), "on the grid");

    let corner = control_in(&harness, "board-zone-resize", "Perf");
    drag(
        &mut harness,
        corner.center(),
        corner.center() + egui::vec2(260.0, 200.0),
        egui::PointerButton::Primary,
    );
    until(&mut harness, "the resize is written", |harness| {
        frame_of(harness, "Perf").width > moved.width
    });
    let resized = frame_of(&harness, "Perf");
    assert_eq!((resized.x, resized.y), (moved.x, moved.y));
    let perf = folder_named(&harness, &tableau, "Perf");
    assert_eq!(filed_in(&harness, "EXPLAIN lent sur join"), Some(perf.id));
    assert_eq!(
        filed_in(&harness, "Dump nocturne"),
        None,
        "a stretched zone captures nothing"
    );
}

#[test]
fn the_grip_and_the_corner_answer_the_arrows_a_dot_at_a_time() {
    let (mut harness, _) = tableau();
    click_id(&mut harness, "view-board");
    let before = frame_of(&harness, "Perf");
    harness
        .get_by(|node| {
            node.author_id() == Some("board-zone-grip")
                && node.label().is_some_and(|label| label.contains("Perf"))
        })
        .focus();
    // egui locks the arrows to a widget from the frame after it took the focus.
    harness.step();
    harness.step();
    for key in [egui::Key::ArrowRight, egui::Key::ArrowDown] {
        harness.key_press(key);
        harness.step();
    }
    // A dot further each way, snapped onto the lattice.
    let wanted = (
        super::gesture::snap(before.x + 20),
        super::gesture::snap(before.y + 20),
    );
    until(&mut harness, "both nudges are written", |harness| {
        let after = frame_of(harness, "Perf");
        (after.x, after.y) == wanted
    });
    assert!(harness.state().state.board_place().is_some());
}

#[test]
fn a_band_drawn_on_the_ground_becomes_the_folder_named_there() {
    let (mut harness, tableau) = tableau();
    click_id(&mut harness, "view-board");
    let ground = rect_of(&harness, "board", "board");
    let from = egui::pos2(ground.left() + 520.0, ground.top() + 420.0);

    drag(
        &mut harness,
        from,
        from + egui::vec2(300.0, 230.0),
        egui::PointerButton::Primary,
    );
    assert!(harness.state().state.board.drawn_zone().is_some());
    crate::notes::bench::type_and_enter(&mut harness, "folder-name-input", "Reporting");
    until(&mut harness, "the zone is drawn", |harness| {
        names(harness, "board-zone-open").contains(&"Reporting".to_owned())
    });

    let made = folder_named(&harness, &tableau, "Reporting");
    let view = harness.state().state.board.view().unwrap();
    let zone = view
        .zones
        .iter()
        .find(|zone| zone.folder.id == made.id)
        .unwrap();
    assert_eq!((zone.frame.x % 20, zone.frame.y % 20), (0, 0));
    assert!(zone.frame.width >= 300 - 20);
}

#[test]
fn a_right_button_sweep_ticks_the_cards_it_crosses_zones_included() {
    let (mut harness, _) = tableau();
    click_id(&mut harness, "view-board");
    let ground = rect_of(&harness, "board", "board");

    drag(
        &mut harness,
        ground.min + egui::vec2(4.0, 4.0),
        ground.max - egui::vec2(4.0, 4.0),
        egui::PointerButton::Secondary,
    );

    let ticked = harness.state().state.on_screen();
    let ticked: Vec<&str> = ticked.iter().map(String::as_str).collect();
    assert_eq!(harness.state().state.selection.checked_in(&ticked).len(), 2);
    assert!(
        harness.state().state.board.drawn_zone().is_none(),
        "the right button draws no folder"
    );
}

#[test]
fn a_zone_menu_ticks_every_card_it_holds() {
    let (mut harness, _) = tableau();
    click_id(&mut harness, "view-board");
    harness
        .get_by(|node| {
            node.author_id() == Some("board-zone-menu")
                && node.label().is_some_and(|label| label.contains("Perf"))
        })
        .click();
    until(&mut harness, "the menu is open", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("folder-select-notes"))
            .is_some()
    });
    click_id(&mut harness, "folder-select-notes");

    let ticked = harness.state().state.on_screen();
    let ticked: Vec<&str> = ticked.iter().map(String::as_str).collect();
    assert_eq!(harness.state().state.selection.checked_in(&ticked).len(), 1);
}

#[test]
fn a_card_let_go_of_over_a_rail_row_is_filed_there_and_offered_back() {
    let (mut harness, tableau) = tableau();
    click_id(&mut harness, "view-board");
    let perf = folder_named(&harness, &tableau, "Perf");
    let card = rect_of(&harness, "board-loose-card", "Dump nocturne");
    let row = harness
        .get_by(|node| {
            node.author_id() == Some("folder-option") && node.label().as_deref() == Some("Perf")
        })
        .rect();

    drag(
        &mut harness,
        card.center(),
        row.center(),
        egui::PointerButton::Primary,
    );
    until(&mut harness, "the card is filed from the rail", |harness| {
        filed_in(harness, "Dump nocturne").as_deref() == Some(perf.id.as_str())
    });
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("undo-restore"))
            .is_some()
    );
}

#[test]
fn an_open_folder_s_list_is_named_after_it_until_something_is_searched() {
    let (mut harness, _) = tableau();
    click(&mut harness, "folder-option", "Perf");
    let heading = |harness: &Harness<'_, Bench>| names(harness, "note-section").join(" ");
    assert!(
        heading(&harness).starts_with("PERF"),
        "{}",
        heading(&harness)
    );

    click_id(&mut harness, "search-input");
    harness
        .get_by(|node| node.author_id() == Some("search-input"))
        .type_text("EXPLAIN");
    until(&mut harness, "the search is asked", |harness| {
        !heading(harness).starts_with("PERF")
    });
}
