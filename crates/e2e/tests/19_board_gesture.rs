//! Arranging the board by hand: a card dragged into a zone and back out, a zone moved and
//! resized, a folder drawn on the ground, a right-button sweep, a whole zone ticked, and the
//! tidy — each checked against what a relaunch reads back from the database. The keyboard twin
//! of a drop is the selection bar's « File into ».
//!
//! A space of its own, made here: the samples are in the system's language.

use e2e::{App, Tree, step};
use egui::accesskit::{Node, Rect};
use egui::{Key, Modifiers, PointerButton, Pos2};

const SPACE: &str = "Gestes";
const FILED: &str = "EXPLAIN lent sur join";
const LOOSE: &str = "Dump nocturne";
/// `folders::board::ZONE_HEADER`: every control of a zone's header holds in it.
const ZONE_HEADER: f64 = 38.0;

fn named(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label() == Some(name)
}

fn bounds_of(tree: &Tree, author_id: &str, name: &str) -> Option<Rect> {
    tree.all(author_id)
        .into_iter()
        .find(|node| node.label() == Some(name))
        .and_then(Node::bounds)
}

/// Points on the window, from the tree's `f64`.
fn point(x: f64, y: f64) -> Pos2 {
    #[expect(clippy::cast_possible_truncation, reason = "points on a screen")]
    Pos2::new(x as f32, y as f32)
}

fn centre(rect: Rect) -> Pos2 {
    point(
        f64::midpoint(rect.x0, rect.x1),
        f64::midpoint(rect.y0, rect.y1),
    )
}

/// The titles drawn inside `zone`.
fn titles_in(tree: &Tree, zone: &str) -> Vec<String> {
    let Some(frame) = bounds_of(tree, "board-zone", zone) else {
        return Vec::new();
    };
    tree.all("board-zone-card")
        .into_iter()
        .filter(|node| {
            node.bounds().is_some_and(|card| {
                card.x0 >= frame.x0 && card.x1 <= frame.x1 && card.y0 >= frame.y0
            })
        })
        .map(|node| node.label().unwrap_or_default().to_owned())
        .collect()
}

/// A zone's control, the one in that zone's header.
fn control_of(tree: &Tree, author_id: &str, zone: &str) -> Option<Rect> {
    let frame = bounds_of(tree, "board-zone", zone)?;
    tree.all(author_id)
        .into_iter()
        .filter_map(Node::bounds)
        .find(|control| {
            control.x0 >= frame.x0 - 1.0
                && control.x1 <= frame.x1 + 1.0
                && control.y0 >= frame.y0 - 1.0
                && control.y1 <= frame.y1 + 1.0
        })
}

fn write(app: &mut App, title: &str) {
    app.click("new-note");
    app.fill("editor-title", title);
    app.fill("editor-body", "SELECT 1;");
    app.click("editor-close");
    app.eventually("the note is a card", |tree| {
        !tree.has("editor-title") && tree.texts("note-card").contains(&title.to_owned())
    });
}

/// The selection bar's « File into », on the entry named `name`.
fn file_into(app: &mut App, name: &str) {
    app.click("selection-file");
    app.eventually("the folders are offered", |tree| {
        tree.has("selection-file-0")
    });
    let tree = app.tree();
    let entry = (0..12)
        .map(|at| format!("selection-file-{at}"))
        .find(|id| tree.text(id).as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no entry {name}"));
    app.click(&entry);
}

fn reopen(app: App) -> App {
    let mut app = app.relaunch();
    app.fill("vault-passphrase", e2e::PASSPHRASE);
    app.click("vault-submit");
    app.eventually("the board is back", |tree| tree.has("board-zone"));
    app
}

fn seed(app: &mut App) {
    step(
        "a space of its own: two folders, a note filed, one loose",
        || {
            app.click("space-create-open");
            app.fill("space-create-input", SPACE);
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the new space is shown", |tree| {
                tree.texts("space-option").contains(&SPACE.to_owned()) && !tree.has("note-card")
            });
            for name in ["Perf", "Migrations"] {
                app.click("folder-create-open");
                app.fill("folder-create-input", name);
                app.press(Key::Enter, Modifiers::NONE);
                app.eventually("the folder is listed", |tree| {
                    tree.texts("folder-option").contains(&name.to_owned())
                });
            }
            app.click_where("folder-option", named("Perf"));
            app.wait_for("folder-breadcrumb-name");
            write(app, FILED);
            app.click_where("space-option", named(SPACE));
            app.eventually("the space is shown", |tree| {
                !tree.has("folder-breadcrumb-name")
            });
            write(app, LOOSE);
            app.click("view-board");
            app.eventually("the board is drawn", |tree| tree.count("board-zone") == 2);
        },
    );
}

fn cards(app: &mut App) {
    step(
        "a card dragged into a zone is filed there, and its place forgotten",
        || {
            let tree = app.tree();
            let card = bounds_of(&tree, "board-loose-card", LOOSE).unwrap();
            let zone = bounds_of(&tree, "board-zone", "Migrations").unwrap();
            // The card lands where its corner does: the grab, at its centre, keeps the offset.
            let into = point(zone.x0 + 150.0, zone.y0 + 120.0);
            app.drag(centre(card), into, PointerButton::Primary);
            app.eventually("the card flows in the zone", |tree| {
                titles_in(tree, "Migrations") == [LOOSE]
                    && !tree.texts("board-loose-card").contains(&LOOSE.to_owned())
            });
            app.capture("board-filed");
        },
    );

    step("dropped on the ground, it comes back out", || {
        let card = bounds_of(&app.tree(), "board-zone-card", LOOSE).unwrap();
        let ground = Pos2::new(centre(card).x, centre(card).y + 360.0);
        app.drag(centre(card), ground, PointerButton::Primary);
        app.eventually("the card is loose again", |tree| {
            tree.texts("board-loose-card").contains(&LOOSE.to_owned())
                && titles_in(tree, "Migrations").is_empty()
        });
    });

    step(
        "the keyboard files with no pointer, from the selection bar",
        || {
            app.hover("board-zone-card");
            app.click("note-card-check");
            app.wait_for("selection-count");
            file_into(app, "Migrations");
            app.eventually("the note is in the other zone", |tree| {
                titles_in(tree, "Migrations") == [FILED]
            });
            // While the bar offers the filing back, Escape would take it back.
            app.click("undo-dismiss");
            app.press(Key::Escape, Modifiers::NONE);
            app.hover("board-zone-card");
            app.click("note-card-check");
            file_into(app, "Perf");
            app.eventually("and back", |tree| titles_in(tree, "Perf") == [FILED]);
            // While the bar offers the filing back, Escape would take it back.
            app.click("undo-dismiss");
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually("the selection is let go", |tree| {
                !tree.has("selection-count")
            });
        },
    );

    step("a zone's menu ticks every card it holds", || {
        app.click_where("board-zone-menu", |node| {
            node.label().is_some_and(|label| label.contains("Perf"))
        });
        app.click("folder-select-notes");
        app.eventually("its card is ticked", |tree| {
            tree.text("selection-count")
                .is_some_and(|said| said.contains('1'))
        });
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the selection is let go", |tree| {
            !tree.has("selection-count")
        });
    });
}

fn zones(app: &mut App) {
    step("a zone's header holds in ZONE_HEADER", || {
        let tree = app.tree();
        let frame = bounds_of(&tree, "board-zone", "Perf").unwrap();
        for control in ["board-zone-grip", "board-zone-open", "board-zone-menu"] {
            let drawn = control_of(&tree, control, "Perf").unwrap();
            assert!(
                drawn.y1 <= frame.y0 + ZONE_HEADER,
                "{control} overflows the header"
            );
        }
    });

    step(
        "a zone moved by its grip lands on the grid and keeps its notes",
        || {
            let tree = app.tree();
            let before = bounds_of(&tree, "board-zone", "Perf").unwrap();
            let grip = control_of(&tree, "board-zone-grip", "Perf").unwrap();
            let to = centre(grip) + egui::vec2(93.0, 141.0);
            app.drag(centre(grip), to, PointerButton::Primary);
            app.eventually("the zone moved, its size kept", |tree| {
                bounds_of(tree, "board-zone", "Perf").is_some_and(|moved| {
                    (moved.x0 - before.x0).abs() > 0.5
                        && (moved.width() - before.width()).abs() < 0.5
                })
            });
            assert_eq!(titles_in(&app.tree(), "Perf"), [FILED]);
        },
    );

    step(
        "resized by its corner, it captures and releases nothing",
        || {
            let tree = app.tree();
            let before = bounds_of(&tree, "board-zone", "Perf").unwrap();
            let corner = control_of(&tree, "board-zone-resize", "Perf").unwrap();
            let to = centre(corner) + egui::vec2(260.0, 120.0);
            app.drag(centre(corner), to, PointerButton::Primary);
            app.eventually("the zone is wider, where it was", |tree| {
                bounds_of(tree, "board-zone", "Perf").is_some_and(|resized| {
                    resized.width() > before.width() + 100.0 && (resized.x0 - before.x0).abs() < 0.5
                })
            });
            let tree = app.tree();
            assert_eq!(titles_in(&tree, "Perf"), [FILED]);
            assert!(tree.texts("board-loose-card").contains(&LOOSE.to_owned()));
        },
    );

    step(
        "a band drawn on the ground becomes the folder named there",
        || {
            let ground = app.tree().find("board").and_then(Node::bounds).unwrap();
            let from = point(ground.x0 + 40.0, ground.y1 - 260.0);
            app.drag(
                from,
                from + egui::vec2(300.0, 220.0),
                PointerButton::Primary,
            );
            app.fill("folder-name-input", "Reporting");
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the zone is drawn where it was drawn", |tree| {
                tree.texts("board-zone-open")
                    .contains(&"Reporting".to_owned())
            });
            app.capture("board-drawn-zone");
        },
    );
}

fn selecting(app: &mut App) {
    step(
        "a right-button sweep ticks the cards it crosses, zones included",
        || {
            let ground = app.tree().find("board").and_then(Node::bounds).unwrap();
            let from = point(ground.x0 + 4.0, ground.y0 + 4.0);
            let to = point(ground.x1 - 40.0, ground.y1 - 60.0);
            app.drag(from, to, PointerButton::Secondary);
            app.eventually("both cards are ticked", |tree| {
                tree.text("selection-count")
                    .is_some_and(|said| said.contains('2'))
            });
            assert!(
                !app.has("folder-name-input"),
                "the right button draws no folder"
            );
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually("the selection is let go", |tree| {
                !tree.has("selection-count")
            });
        },
    );
}

/// One step: the core writes in order, so the undo of the reorganisation puts back the layout
/// aligning left — the hand's, if aligning moved no zone.
fn tidying(app: &mut App) {
    step(
        "aligning leaves the zones; reorganising moves them, and its undo puts the hand's back",
        || {
            let by_hand = bounds_of(&app.tree(), "board-zone", "Perf").unwrap();
            app.click("board-tidy");
            app.click("board-tidy-more");
            app.click("board-tidy-everything");
            app.eventually("the zone is back at its seat", |tree| {
                bounds_of(tree, "board-zone", "Perf")
                    .is_some_and(|seat| (seat.x0 - by_hand.x0).abs() > 0.5)
                    && tree.has("undo-restore")
            });
            app.capture("board-reorganised");
            app.click("undo-restore");
            app.eventually("the hand's layout is back", |tree| {
                bounds_of(tree, "board-zone", "Perf").is_some_and(|back| {
                    (back.x0 - by_hand.x0).abs() < 0.5 && (back.y0 - by_hand.y0).abs() < 0.5
                })
            });
        },
    );
}

#[test]
fn board_gesture() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("19_board_gesture", profile.path());
    app.create_library();
    app.wait_for("note-card");

    seed(&mut app);
    cards(&mut app);
    zones(&mut app);
    let before = (
        bounds_of(&app.tree(), "board-zone", "Perf"),
        bounds_of(&app.tree(), "board-loose-card", LOOSE),
        bounds_of(&app.tree(), "board-zone", "Reporting"),
    );
    let mut app = reopen(app);

    step("a relaunch reads back every place the hand chose", || {
        let tree = app.tree();
        let after = (
            bounds_of(&tree, "board-zone", "Perf"),
            bounds_of(&tree, "board-loose-card", LOOSE),
            bounds_of(&tree, "board-zone", "Reporting"),
        );
        assert_eq!(after, before);
    });

    selecting(&mut app);
    tidying(&mut app);
    app.quit();
}
