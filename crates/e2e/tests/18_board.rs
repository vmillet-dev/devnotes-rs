//! The board: a space's folders drawn as zones holding their notes, the loose ones beside, the
//! same card the list draws, a first layout that comes back unchanged after a relaunch, and the
//! view remembered space by space. Only a real database proves the first layout is written.
//!
//! A space of its own, made here: the samples are in the system's language.

use e2e::{App, Tree, step};
use egui::accesskit::{Node, Rect, Toggled};
use egui::{Key, Modifiers};

const SPACE: &str = "Tableau";
const FILED: &str = "EXPLAIN lent sur join";
const LOOSE: &str = "Dump nocturne";
const LIST: &str = "Avant la release";

fn named(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label() == Some(name)
}

fn is_on(tree: &Tree, author_id: &str) -> bool {
    tree.find(author_id)
        .is_some_and(|node| node.toggled() == Some(Toggled::True))
}

fn frame_of(tree: &Tree, zone: &str) -> Option<Rect> {
    tree.all("board-zone")
        .into_iter()
        .find(|node| node.label() == Some(zone))
        .and_then(Node::bounds)
}

/// The titles drawn inside `zone`, in reading order.
fn titles_in(tree: &Tree, zone: &str) -> Vec<String> {
    let Some(frame) = frame_of(tree, zone) else {
        return Vec::new();
    };
    tree.all("note-card-title")
        .into_iter()
        .filter(|node| {
            node.bounds().is_some_and(|card| {
                card.x0 >= frame.x0
                    && card.x1 <= frame.x1
                    && card.y0 >= frame.y0
                    && card.y1 <= frame.y1
            })
        })
        .map(|node| node.label().or(node.value()).unwrap_or_default().to_owned())
        .collect()
}

fn frames(tree: &Tree) -> Vec<Rect> {
    tree.all("board-zone")
        .into_iter()
        .filter_map(Node::bounds)
        .collect()
}

/// A note written from the editor in whatever space or folder is shown.
fn write(app: &mut App, title: &str) {
    app.click("new-note");
    app.fill("editor-title", title);
    app.fill("editor-body", "SELECT 1;");
    app.click("editor-close");
    app.eventually("the note is a card", |tree| {
        !tree.has("editor-title") && tree.texts("note-card").contains(&title.to_owned())
    });
}

fn choose_space(app: &mut App, name: &str) {
    app.click_where("space-option", named(name));
    app.eventually("the space is shown", |tree| {
        tree.all("space-option")
            .into_iter()
            .any(|node| node.label() == Some(name) && node.toggled() == Some(Toggled::True))
    });
}

/// A space of its own: two folders, a note filed in the first, one loose.
fn seed(app: &mut App) {
    step(
        "a space of its own: two folders, a note filed, one loose",
        || {
            app.click("space-create-open");
            app.fill("space-create-input", SPACE);
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the new space is shown, empty", |tree| {
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
            choose_space(app, SPACE);
            write(app, LOOSE);
        },
    );
}

/// What a card on the board still does: tick its items, take the selection, take the arrows.
fn on_the_board(app: &mut App) {
    step(
        "a todo list is ticked on the board, and shown ticked",
        || {
            app.click("new-note-kind");
            app.click("new-note-checklist");
            app.fill("editor-title", LIST);
            app.click("checklist-add");
            app.eventually("the new row has the focus", |tree| {
                tree.focused().starts_with("checklist-text")
            });
            app.type_text("Tag");
            app.click("editor-close");
            app.eventually("the list is a loose card", |tree| {
                tree.texts("board-loose-card").contains(&LIST.to_owned())
            });
            app.click_nth("note-card-item", 0);
            app.eventually("the item is ticked", |tree| {
                tree.find("note-card-item")
                    .is_some_and(|node| node.toggled() == Some(Toggled::True))
            });
        },
    );

    step("the corner is the selection tick's", || {
        let title = app.tree().texts("board-loose-card")[0].clone();
        let its_tick = |node: &Node| node.label().is_some_and(|label| label.contains(&title));
        app.hover("board-loose-card");
        app.click_where("note-card-check", its_tick);
        app.wait_for("selection-count");
        app.click_where("note-card-check", its_tick);
        app.eventually("the bar is gone", |tree| !tree.has("selection-count"));
    });

    step(
        "the arrows walk the cards as the board lays them out",
        || {
            app.click("search-input");
            app.press(Key::ArrowDown, Modifiers::NONE);
            for _ in 0..3 {
                app.press(Key::ArrowLeft, Modifiers::NONE);
            }
            app.eventually("the first card of the board has the focus", |tree| {
                tree.focused().contains(FILED)
            });
            app.press(Key::ArrowRight, Modifiers::NONE);
            app.eventually("then the next", |tree| !tree.focused().contains(FILED));
            app.press(Key::Escape, Modifiers::NONE);
        },
    );
}

/// Whether the card titled `title` is drawn lit: a dimmed one keeps its seat, disabled.
fn lit(tree: &Tree, title: &str) -> Option<bool> {
    tree.all("board-loose-card")
        .into_iter()
        .chain(tree.all("board-zone-card"))
        .find(|node| node.label() == Some(title))
        .map(|node| !node.is_disabled())
}

/// A search dims on the board, and the tidy names what it would touch.
fn dimmed(app: &mut App) {
    step(
        "a search dims what it does not match, and drops nothing",
        || {
            app.click("search-input");
            app.type_text("EXPLAIN");
            app.eventually("the loose card is dimmed", |tree| {
                lit(tree, LOOSE) == Some(false)
            });
            let tree = app.tree();
            assert_eq!(lit(&tree, FILED), Some(true));
            assert_eq!(titles_in(&tree, "Perf"), [FILED]);
            app.capture("board-dimmed");
            app.click("search-clear");
            app.eventually("the card is lit again", |tree| {
                lit(tree, LOOSE) == Some(true)
            });
        },
    );

    step(
        "reorganising says how many folders and cards it moves",
        || {
            app.click("board-tidy-more");
            app.eventually("the entry counts two of each", |tree| {
                tree.text("board-tidy-everything")
                    .is_some_and(|said| said.matches('2').count() == 2)
            });
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually("the menu is closed", |tree| {
                !tree.has("board-tidy-everything")
            });
        },
    );
}

#[test]
fn board() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("18_board", profile.path());
    app.create_library();
    app.wait_for("note-card");
    let samples = app.tree().texts("space-option")[0].clone();

    seed(&mut app);

    step(
        "the list is the default, and there is no board across every space",
        || {
            assert!(is_on(&app.tree(), "view-list"));
            assert!(!app.has("board"));
            app.click("space-option-all");
            app.eventually("the board is offered disabled", |tree| {
                tree.find("view-board").is_some_and(Node::is_disabled)
            });
            choose_space(&mut app, SPACE);
        },
    );

    step(
        "each folder is a zone holding its notes, the loose ones beside",
        || {
            app.click("view-board");
            app.eventually("the zones are drawn", |tree| tree.count("board-zone") == 2);
            let tree = app.tree();
            let mut zones = tree.texts("board-zone-open");
            zones.sort();
            assert_eq!(zones, ["Migrations", "Perf"]);
            assert_eq!(titles_in(&tree, "Perf"), [FILED]);
            assert!(titles_in(&tree, "Migrations").is_empty());
            assert_eq!(tree.texts("board-loose-card"), [LOOSE]);
            // The same card as the list's: its title, and the layer that opens it.
            assert_eq!(tree.count("note-card-open"), 2);
            let [first, second] = frames(&tree)[..] else {
                panic!("two zones");
            };
            assert!(
                first.x1 <= second.x0 || second.x1 <= first.x0,
                "side by side"
            );
            app.capture("board");
        },
    );

    on_the_board(&mut app);
    dimmed(&mut app);

    let before = frames(&app.tree());
    let mut app = app.relaunch();

    step(
        "a relaunch comes back on the board, its layout unchanged",
        || {
            app.fill("vault-passphrase", e2e::PASSPHRASE);
            app.click("vault-submit");
            app.eventually("the board is back", |tree| tree.count("board-zone") == 2);
            assert_eq!(frames(&app.tree()), before);
        },
    );

    step("the view is remembered space by space", || {
        choose_space(&mut app, &samples);
        app.eventually("the samples are listed", |tree| {
            !tree.has("board") && tree.has("note-card")
        });
        choose_space(&mut app, SPACE);
        app.eventually("the board is back", |tree| tree.has("board"));
    });

    step("the list is left exactly as it was", || {
        app.click("view-list");
        app.eventually("the three notes are listed", |tree| {
            !tree.has("board") && tree.count("note-card") == 3
        });
        app.click("view-board");
        app.wait_for("board");
    });

    step(
        "a window larger than the board is pointer ground all over",
        || {
            app.resize(1600, 1000);
            app.eventually("the surface covers the ground", |tree| {
                let (Some(ground), Some(surface)) = (
                    tree.find("board").and_then(Node::bounds),
                    tree.find("board-surface").and_then(Node::bounds),
                ) else {
                    return false;
                };
                surface.x1 >= ground.x1 && surface.y1 >= ground.y1
            });
        },
    );

    app.quit();
}
