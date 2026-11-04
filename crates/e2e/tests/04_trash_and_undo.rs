//! Deleting a note does not delete it: it goes to the trash, and the undo bar, `Escape` while it
//! offers, and `Ctrl+Z` after it has gone take it back; the trash panel restores, purges and
//! empties. The samples' titles are translated: rows are counted, never read.

use e2e::{App, step};
use egui::{Key, Modifiers};

const CARDS: usize = 6;

fn cards(app: &mut App, count: usize, what: &str) {
    app.eventually(what, |tree| tree.count("note-card") == count);
}

/// The first card, sent to the trash from the keyboard: the second press is the one that writes.
fn delete_first(app: &mut App) {
    let before = app.tree().count("note-card");
    app.press(Key::ArrowDown, Modifiers::NONE);
    app.press(Key::Delete, Modifiers::NONE);
    app.wait_for("note-card-arming");
    app.press(Key::Delete, Modifiers::NONE);
    cards(app, before - 1, "the note is in the trash");
}

/// Deleted, and the offer to take it back turned down.
fn trash_first(app: &mut App) {
    delete_first(app);
    app.click("undo-dismiss");
}

fn rows(app: &mut App, count: usize, what: &str) {
    app.eventually(what, |tree| tree.count("trash-row") == count);
}

fn emptied(app: &mut App) {
    app.eventually("the trash says it is empty", |tree| {
        tree.has("trash-empty-state") && tree.count("trash-row") == 0
    });
}

#[test]
fn trash_and_undo() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("04_trash_and_undo", profile.path());
    app.create_library();
    cards(&mut app, CARDS, "the samples are drawn");

    step(
        "the card's menu needs the second click to delete anything",
        || {
            app.hover("note-card-open");
            app.click("note-card-menu");
            app.click("note-card-delete");
            assert_eq!(app.tree().count("note-card"), CARDS);
            app.press(Key::Escape, Modifiers::NONE);
        },
    );

    step("Delete arms the card, and Escape calls it off", || {
        app.press(Key::ArrowDown, Modifiers::NONE);
        app.press(Key::Delete, Modifiers::NONE);
        app.wait_for("note-card-arming");
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the card is disarmed", |tree| !tree.has("note-card-arming"));
        // Back to zero, not to one: the next Delete arms again rather than trashing.
        app.press(Key::Delete, Modifiers::NONE);
        app.wait_for("note-card-arming");
        assert_eq!(app.tree().count("note-card"), CARDS);
        app.press(Key::Escape, Modifiers::NONE);
    });

    step("Escape takes the note back while the bar offers", || {
        delete_first(&mut app);
        app.wait_for("undo-bar");
        app.capture("undo-bar");
        app.press(Key::Escape, Modifiers::NONE);
        cards(&mut app, CARDS, "the note is back");
    });

    step("the bar's Undo takes it back", || {
        delete_first(&mut app);
        app.click("undo-restore");
        cards(&mut app, CARDS, "the note is back");
        assert!(!app.has("undo-bar"));
    });

    step(
        "Ctrl+Z still takes it back once the banner has gone",
        || {
            delete_first(&mut app);
            app.wait_for("undo-bar");
            app.eventually("the banner's timer runs out", |tree| !tree.has("undo-bar"));
            app.press(Key::Z, Modifiers::COMMAND);
            cards(&mut app, CARDS, "the note is back");
        },
    );

    step("a dismissed offer is given up for good", || {
        trash_first(&mut app);
        app.press(Key::Z, Modifiers::COMMAND);
        assert!(!app.has("undo-bar"));
        assert_eq!(app.tree().count("note-card"), CARDS - 1);
    });

    trash_panel(&mut app);
    app.quit();
}

/// Starts with one note in the trash and the others on the canvas.
fn trash_panel(app: &mut App) {
    step("the trash panel restores what was deleted", || {
        app.click("trash-open");
        rows(app, 1, "the deleted note is listed");
        app.capture("trash-panel");
        app.click("trash-restore");
        emptied(app);
        app.click("trash-close");
        cards(app, CARDS, "the note is back");
    });

    step("a row is purged for good on the second click", || {
        trash_first(app);
        app.click("trash-open");
        rows(app, 1, "the deleted note is listed");
        app.click("trash-purge");
        assert_eq!(app.tree().count("trash-row"), 1);
        app.click("trash-purge");
        emptied(app);
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("Escape closes the panel", |tree| !tree.has("trash-close"));
        assert_eq!(app.tree().count("note-card"), CARDS - 1);
    });

    step(
        "a search narrows the trash, and restoring reaches what it found",
        || {
            for _ in 0..3 {
                trash_first(app);
            }
            app.click("trash-open");
            rows(app, 3, "the three deleted notes are listed");
            app.fill("trash-search", "zz-nothing-zz");
            app.eventually("the search finds nothing", |tree| {
                tree.has("trash-empty-state") && !tree.has("trash-empty")
            });
            app.press(Key::A, Modifiers::COMMAND);
            app.press(Key::Backspace, Modifiers::NONE);
            rows(app, 3, "the whole trash is back");
            app.click("trash-restore-all");
            emptied(app);
            app.click("trash-close");
            cards(app, CARDS - 1, "the three notes are back");
        },
    );

    step("emptying says what it erases, then erases it", || {
        trash_first(app);
        trash_first(app);
        app.click("trash-open");
        rows(app, 2, "the two deleted notes are listed");
        app.click("trash-empty");
        app.wait_for("trash-empty-warning");
        app.click("trash-empty-confirm");
        emptied(app);
        app.click("trash-close");
        cards(app, CARDS - 3, "the erased notes stay gone");
    });
}
