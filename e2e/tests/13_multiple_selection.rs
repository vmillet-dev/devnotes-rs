//! Several notes at once: ticked from the keyboard, tagged and deleted together, and only them.
//! What is searched is a tag the scenario gives, which no translation touches.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

const CARDS: usize = 6;

fn tick_two(app: &mut App) {
    for key in [Key::X, Key::ArrowRight, Key::X] {
        app.press(key, Modifiers::NONE);
    }
    app.eventually("two cards are ticked", |tree| {
        tree.text("selection-count")
            .is_some_and(|said| said.contains('2'))
    });
}

/// Where each card's box is: one per card, in the tree's order.
fn boxes(tree: &Tree) -> Vec<(f64, f64)> {
    tree.all("note-card-check")
        .iter()
        .filter_map(|node| node.bounds())
        .map(|bounds| (bounds.x0, bounds.y0))
        .collect()
}

/// A box ticked by the pointer, and every card's box where it was before.
fn ticked_by_its_box(app: &mut App) {
    let before = boxes(&app.tree());
    assert_eq!(before.len(), CARDS, "every card shows its box");
    app.click_nth("note-card-check", 1);
    app.wait_for("selection-count");
    assert_eq!(boxes(&app.tree()), before, "no box moved");
    app.click_nth("note-card-check", 1);
    app.eventually("the bar is gone", |tree| !tree.has("selection-count"));
}

#[test]
fn multiple_selection() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("13_multiple_selection", profile.path());
    app.create_library();
    app.eventually("the samples are drawn", |tree| {
        tree.count("note-card") == CARDS
    });

    step("a card is ticked by its box, which stays put", || {
        ticked_by_its_box(&mut app);
    });

    step("no bar until a card is ticked, then it counts them", || {
        assert!(!app.has("selection-count"));
        app.press(Key::ArrowDown, Modifiers::NONE);
        tick_two(&mut app);
        app.capture("selection-bar");
    });

    step("a tag reaches every ticked note, and only those", || {
        app.fill("selection-tag", "batchtag");
        app.press(Key::Enter, Modifiers::NONE);
        // While the bar offers the tagging back, Escape would take it back.
        app.click("undo-dismiss");
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the selection is let go", |tree| {
            !tree.has("selection-count")
        });
        app.click("search-input");
        app.type_text("batchtag");
        app.eventually("the two tagged notes are found", |tree| {
            tree.count("note-card") == 2
        });
    });

    step("deleting the selection takes two clicks", || {
        app.press(Key::ArrowDown, Modifiers::NONE);
        tick_two(&mut app);
        app.click("selection-delete");
        assert_eq!(app.tree().count("note-card"), 2);
        app.click("selection-delete");
        app.eventually("both are in the trash, and the bar is gone", |tree| {
            tree.has("canvas-no-results") && !tree.has("selection-count")
        });
        app.click("search-clear");
        app.eventually("the others are untouched", |tree| {
            tree.count("note-card") == CARDS - 2
        });
    });

    app.quit();
}
