//! Deleting a note does not delete it: it goes to the trash, and the undo bar, `Escape` while it
//! offers, and `Ctrl+Z` after it has gone take it back.

use e2e::{App, step};
use egui::{Key, Modifiers};

const CARDS: usize = 6;

fn cards(app: &mut App, count: usize, what: &str) {
    app.eventually(what, |tree| tree.count("note-card") == count);
}

/// The first card, sent to the trash from the keyboard: the second press is the one that writes.
fn delete_first(app: &mut App) {
    app.press(Key::ArrowDown, Modifiers::NONE);
    app.press(Key::Delete, Modifiers::NONE);
    app.wait_for("note-card-arming");
    app.press(Key::Delete, Modifiers::NONE);
    cards(app, CARDS - 1, "the note is in the trash");
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
        delete_first(&mut app);
        app.click("undo-dismiss");
        app.press(Key::Z, Modifiers::COMMAND);
        assert!(!app.has("undo-bar"));
        assert_eq!(app.tree().count("note-card"), CARDS - 1);
    });

    app.quit();
}
