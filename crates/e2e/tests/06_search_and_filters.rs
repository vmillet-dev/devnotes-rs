//! The search, the quick filter and the order, over the first launch's samples. Filtering,
//! grouping and folding run in Rust; what is proved here is the query the window sends.
//!
//! Nothing compares a seeded word: the samples are in the system's language. What is searched
//! is in their code, which no translation touches.

use e2e::{App, step};
use egui::{Key, Modifiers};

const CARDS: usize = 6;
/// Groupings in the menu's order: date, priority, format, none.
const NO_GROUPING: usize = 3;
/// Quick filters in the switch's order.
const ALL: &str = "quick-filter-0";
const PINNED: &str = "quick-filter-1";
const UNTRIAGED: &str = "quick-filter-2";

fn search(app: &mut App, words: &str) {
    app.click("search-input");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(words);
}

#[test]
fn search_and_filters() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("06_search_and_filters", profile.path());
    app.create_library();
    app.eventually("the samples are drawn", |tree| {
        tree.count("note-card") == CARDS
    });

    step(
        "a search finds a note by its body, past the debounce",
        || {
            search(&mut app, "withRetry");
            app.eventually("one card, and its count", |tree| {
                tree.count("note-card") == 1 && tree.has("search-matched")
            });
        },
    );

    step("down from the field to the card it found", || {
        app.press(Key::ArrowDown, Modifiers::NONE);
        app.eventually("the card has the focus", |tree| {
            tree.focused().starts_with("note-card-open")
        });
    });

    step("it folds case the way Rust does", || {
        search(&mut app, "WITHRETRY");
        app.eventually("the same card", |tree| tree.count("note-card") == 1);
    });

    step("nothing found says so, and still counts", || {
        search(&mut app, "nothing matches this");
        app.eventually("the canvas says nothing matches", |tree| {
            tree.has("canvas-no-results")
                && tree
                    .text("search-matched")
                    .is_some_and(|said| !said.is_empty())
        });
        app.capture("search-nothing");
    });

    step("the cross empties the field, and the count goes", || {
        app.click("search-clear");
        app.eventually("every card is back", |tree| {
            tree.count("note-card") == CARDS && !tree.has("search-matched")
        });
    });

    step(
        "the quick filter keeps one pinned note, then one with a deadline",
        || {
            app.click(PINNED);
            app.eventually("the pinned welcome", |tree| tree.count("note-card") == 1);
            app.click(UNTRIAGED);
            app.eventually("the list with a deadline", |tree| {
                tree.count("note-card") == 1
            });
            app.click(ALL);
            app.eventually("every card is back", |tree| {
                tree.count("note-card") == CARDS
            });
        },
    );

    step("Ctrl+K puts the caret in the search", || {
        app.press(Key::K, Modifiers::COMMAND);
        app.eventually("the search has the focus", |tree| {
            tree.focused().starts_with("search-input")
        });
    });

    step("no grouping lays the cards out as one list", || {
        app.click("sort-trigger");
        app.click_nth("sort-grouping", NO_GROUPING);
        app.eventually("the date sections are gone", |tree| {
            tree.count("note-section") <= 1 && tree.count("note-card") == CARDS
        });
        app.capture("sort-no-grouping");
        app.press(Key::Escape, Modifiers::NONE);
    });

    let mut app = app.relaunch();

    step("the arrangement is the library's, found again", || {
        app.fill("vault-passphrase", e2e::PASSPHRASE);
        app.click("vault-submit");
        app.wait_for("area-notes");
        app.eventually("still one list", |tree| {
            tree.count("note-card") == CARDS && tree.count("note-section") <= 1
        });
    });

    step("the board waits for its own phase", || {
        assert!(
            app.tree()
                .find("view-board")
                .is_some_and(egui::accesskit::Node::is_disabled)
        );
    });

    app.quit();
}
