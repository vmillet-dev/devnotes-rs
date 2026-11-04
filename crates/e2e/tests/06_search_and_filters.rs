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

/// The header's controls, left to right as they are drawn.
const HEADER: [&str; 13] = [
    "library-rail-toggle",
    "trash-open",
    "space-switcher",
    "view-list",
    "folder-switcher",
    "search-input",
    "quick-filter-0",
    "quick-filter-2",
    "sort-trigger",
    "facets-toggle",
    "new-note",
    "new-note-kind",
    "area-notes",
];

/// How many rows the header's controls sit on, each one's row, and whether any runs past the
/// notes area. Rows are told apart by their middles, as the controls differ in height.
struct Layout {
    rows: usize,
    row_of: Vec<(&'static str, usize)>,
    overflowing: bool,
}

fn layout(tree: &e2e::Tree) -> Layout {
    let area = tree
        .find("area-notes")
        .and_then(egui::accesskit::Node::bounds);
    let boxes: Vec<(&'static str, egui::accesskit::Rect)> = HEADER[..HEADER.len() - 1]
        .iter()
        .filter_map(|id| Some((*id, tree.find(id)?.bounds()?)))
        .collect();
    let mut middles: Vec<f64> = boxes
        .iter()
        .map(|(_, b)| f64::midpoint(b.y0, b.y1))
        .collect();
    middles.sort_by(f64::total_cmp);
    let starts: Vec<f64> = middles
        .iter()
        .enumerate()
        .filter(|(at, middle)| *at == 0 || **middle - middles[at - 1] > 12.0)
        .map(|(_, middle)| *middle)
        .collect();
    let row_of = boxes
        .iter()
        .map(|(id, b)| {
            let middle = f64::midpoint(b.y0, b.y1);
            (
                *id,
                starts
                    .iter()
                    .filter(|start| **start <= middle + 12.0)
                    .count(),
            )
        })
        .collect();
    let overflowing = area.is_none_or(|area| {
        boxes
            .iter()
            .any(|(_, b)| b.x0 < area.x0 - 1.0 || b.x1 > area.x1 + 1.0)
    });
    Layout {
        rows: starts.len(),
        row_of,
        overflowing,
    }
}

fn row(layout: &Layout, id: &str) -> usize {
    layout
        .row_of
        .iter()
        .find(|(each, _)| *each == id)
        .map_or(0, |(_, row)| *row)
}

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

    rails_and_header(&mut app);

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

/// The facets' rails over the samples, then the header at three widths.
fn rails_and_header(app: &mut App) {
    step(
        "a kind from the rail narrows the cards, and « All » gives them back",
        || {
            app.click("facets-toggle");
            app.click("kind-chip-note");
            app.eventually("the Notes alone, and the toggle counts one chip", |tree| {
                let cards = tree.count("note-card");
                cards > 0 && cards < CARDS && tree.text("facets-count").as_deref() == Some("1")
            });
            assert!(
                app.tree()
                    .find("facets-toggle")
                    .is_some_and(egui::accesskit::Node::is_disabled),
                "it refuses to fold while a chip is pressed"
            );
            app.capture("facets-kind");
            app.click("kind-chip-all");
            app.eventually("every card is back", |tree| {
                tree.count("note-card") == CARDS
            });
        },
    );

    step(
        "a tag and a format narrow too, and one click clears them",
        || {
            app.click("tag-pill");
            app.eventually("fewer cards", |tree| tree.count("note-card") < CARDS);
            app.click_nth("language-chip", 0);
            app.eventually("two chips pressed", |tree| {
                tree.text("facets-count").as_deref() == Some("2")
            });
            app.click("clear-filters");
            app.eventually("every card is back, and the way out goes", |tree| {
                tree.count("note-card") == CARDS
                    && !tree.has("clear-filters")
                    && !tree.has("facets-count")
            });
        },
    );

    step("Escape lets a chip go", || {
        app.click("kind-chip-note");
        app.eventually("fewer cards", |tree| tree.count("note-card") < CARDS);
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("every card is back", |tree| {
            tree.count("note-card") == CARDS
        });
        app.click("facets-toggle");
        app.eventually("the rails fold", |tree| !tree.has("kind-chip-all"));
    });

    step(
        "the header holds one row at 1,100 with the rail open",
        || {
            app.resize(1_100, 720);
            let shown = layout(&app.tree());
            assert!(!shown.overflowing);
            assert_eq!(shown.rows, 1);
        },
    );

    step("the rail put away, one row at 1,330, two at 640", || {
        app.click("library-rail-toggle");
        app.wait_for("space-switcher");
        app.resize(1_330, 720);
        app.eventually("one row, inside the window", |tree| {
            let shown = layout(tree);
            shown.rows == 1 && !shown.overflowing
        });
        app.resize(1_100, 720);
        app.eventually("nothing overflows", |tree| !layout(tree).overflowing);
        app.resize(640, 720);
        app.eventually("two rows, « New note » ending the first", |tree| {
            let shown = layout(tree);
            shown.rows == 2
                && !shown.overflowing
                && row(&shown, "new-note") == row(&shown, "library-rail-toggle")
                && row(&shown, "search-input") == 2
        });
        app.capture("header-two-rows");
        app.resize(1_100, 720);
        app.click("library-rail-toggle");
    });
}
