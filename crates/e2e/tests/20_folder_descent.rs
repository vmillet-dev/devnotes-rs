//! Opening a folder: from a zone's title on the board, a flat grid of its notes named after it,
//! the switchers put away for its breadcrumb, a note made inside it filed there, renamed from
//! beside the breadcrumb, and left again — Escape giving the search back first, then the folder
//! — down to the board it was opened from, or through the breadcrumb, or by deleting it.
//!
//! A space of its own, made here: the samples are in the system's language.

use e2e::{App, Tree, step};
use egui::accesskit::{Node, Toggled};
use egui::{Key, Modifiers};

const SPACE: &str = "Descente";
const INSIDE: &str = "EXPLAIN lent sur join";
const LOOSE: &str = "Dump nocturne";
const MADE: &str = "Cache hit ratio";

fn named(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label() == Some(name)
}

fn crumb(tree: &Tree) -> Option<String> {
    tree.text("folder-breadcrumb-name")
}

fn titles(tree: &Tree) -> Vec<String> {
    tree.all("note-card-title")
        .into_iter()
        .map(|node| node.label().or(node.value()).unwrap_or_default().to_owned())
        .collect()
}

fn heading(tree: &Tree) -> String {
    tree.texts("note-section").join(" ")
}

fn write(app: &mut App, title: &str) {
    app.click("new-note");
    app.fill("editor-title", title);
    app.fill("editor-body", "SELECT 1;");
    app.click("editor-close");
    app.eventually("the note is a card", |tree| {
        !tree.has("editor-title") && titles(tree).contains(&title.to_owned())
    });
}

fn search(app: &mut App, words: &str) {
    app.click("search-input");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(words);
}

fn clear(app: &mut App) {
    app.click("search-clear");
    app.eventually("the search is empty", |tree| !tree.has("search-clear"));
}

fn seed(app: &mut App) {
    step(
        "a space of its own, a folder holding one note, one loose",
        || {
            app.click("space-create-open");
            app.fill("space-create-input", SPACE);
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the new space is shown", |tree| {
                tree.texts("space-option").contains(&SPACE.to_owned()) && !tree.has("note-card")
            });
            app.click("folder-create-open");
            app.fill("folder-create-input", "Perf");
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the folder is listed", |tree| tree.has("folder-option"));
            app.click_where("folder-option", named("Perf"));
            app.wait_for("folder-breadcrumb-name");
            write(app, INSIDE);
            app.click_where("space-option", named(SPACE));
            app.eventually("the space is shown", |tree| crumb(tree).is_none());
            write(app, LOOSE);
            app.click("view-board");
            app.wait_for("board-zone");
        },
    );
}

fn inside(app: &mut App) {
    step(
        "a zone's title opens its folder, a flat grid of its notes",
        || {
            app.click_where("board-zone-open", named("Perf"));
            app.eventually("the breadcrumb names it, the grid shows its note", |tree| {
                crumb(tree).as_deref() == Some("Perf")
                    && !tree.has("board")
                    && titles(tree) == [INSIDE]
            });
            let tree = app.tree();
            assert!(
                heading(&tree).to_uppercase().starts_with("PERF"),
                "{}",
                heading(&tree)
            );
            for gone in ["space-switcher", "view-board", "folder-switcher"] {
                assert!(!tree.has(gone), "{gone} is put away");
            }
            assert!(tree.has("create-ghost"), "a note is made from here");
            app.capture("folder-open");
        },
    );

    step(
        "a search stays inside the folder, and drops the ghost",
        || {
            search(app, "EXPLAIN");
            // The heading follows the field at once, the ghost the view once the search rests.
            app.eventually("found inside, the list no longer named after it", |tree| {
                titles(tree) == [INSIDE]
                    && !heading(tree).to_uppercase().starts_with("PERF")
                    && !tree.has("create-ghost")
            });
            search(app, "Dump");
            app.eventually(
                "a note of the space outside it stays out of reach",
                |tree| tree.has("canvas-no-results") && crumb(tree).is_some(),
            );
            clear(app);
        },
    );

    step("a note made here arrives filed", || {
        write(app, MADE);
        app.eventually("both notes are in it", |tree| titles(tree).len() == 2);
    });

    step("renamed from beside the breadcrumb", || {
        app.click("folder-breadcrumb-menu");
        app.click("folder-rename-input");
        app.press(Key::A, Modifiers::COMMAND);
        app.type_text("Performance");
        app.press(Key::Enter, Modifiers::NONE);
        app.eventually("the breadcrumb carries the new name", |tree| {
            crumb(tree).as_deref() == Some("Performance")
        });
    });
}

fn leaving(app: &mut App) {
    step("Escape gives the search back before the folder", || {
        search(app, "EXPLAIN");
        app.eventually("searched", |tree| tree.has("search-clear"));
        // egui lets go of the field before the canvas reads the key: one press is one rung.
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the search is cleared, the folder kept", |tree| {
            !tree.has("search-clear") && crumb(tree).is_some()
        });
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("back on the board it was opened from", |tree| {
            crumb(tree).is_none() && tree.has("board")
        });
    });

    step("the breadcrumb leads back too", || {
        app.click_where("board-zone-open", named("Performance"));
        app.wait_for("folder-breadcrumb-name");
        app.click("folder-breadcrumb-back");
        app.eventually("back on the board", |tree| {
            crumb(tree).is_none() && tree.has("board")
        });
        assert!(
            app.tree()
                .all("space-option")
                .into_iter()
                .any(|node| node.label() == Some(SPACE) && node.toggled() == Some(Toggled::True))
        );
    });

    step(
        "deleted from inside it, its notes stay on the board",
        || {
            app.click_where("board-zone-open", named("Performance"));
            app.wait_for("folder-breadcrumb-name");
            app.click("folder-breadcrumb-menu");
            app.click("folder-delete");
            app.click("folder-delete");
            app.eventually("the folder is gone, its notes loose", |tree| {
                crumb(tree).is_none()
                    && !tree.has("board-zone")
                    && tree.count("board-loose-card") == 3
            });
        },
    );
}

#[test]
fn folder_descent() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("20_folder_descent", profile.path());
    app.create_library();
    app.wait_for("note-card");

    seed(&mut app);
    inside(&mut app);
    leaving(&mut app);
    app.quit();
}
