//! Creating a note writes nothing until it is worth saving, and the editor writes on the way
//! out: two rules about a row in the database, proved on the assembled window.

use e2e::{App, step};
use egui::{Key, Modifiers};

const CARDS: usize = 6;
const TITLE: &str = "Rotate the staging certificate";
const BODY: &str = "openssl req -new -key staging.key -out staging.csr";

fn cards(app: &mut App, count: usize, what: &str) {
    app.eventually(what, |tree| tree.count("note-card") == count);
}

#[test]
fn note_lifecycle() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("02_note_lifecycle", profile.path());
    app.create_library();
    cards(&mut app, CARDS, "the samples are drawn");

    step(
        "a card names its kind by an icon, and only a snippet its format",
        || {
            let tree = app.tree();
            assert_eq!(tree.count("kind-icon"), CARDS);
            let formats = tree.count("note-format");
            assert!(formats > 0 && formats < CARDS, "{formats} formats");
            // Title bar over status line, on every card.
            for (icon, when) in tree.all("kind-icon").iter().zip(tree.all("note-card-when")) {
                let (icon, when) = (icon.bounds().unwrap(), when.bounds().unwrap());
                assert!(icon.y1 <= when.y0, "{icon:?} over {when:?}");
            }
        },
    );

    step("a new note opens as a draft that is not yet a note", || {
        app.click("new-note");
        app.wait_for("editor-title");
        // Beside the editor fewer cards fit on screen, none in a narrow window: none is added.
        let drawn = app.tree().count("note-card");
        assert!(drawn <= CARDS, "{drawn} cards drawn");
    });

    step(
        "it is written once it carries something worth keeping",
        || {
            app.fill("editor-title", TITLE);
            app.fill("editor-body", BODY);
            app.capture("editor-draft");
            app.click("editor-close");
            app.eventually("the editor is closed and the note is a card", |tree| {
                !tree.has("editor-title") && tree.texts("note-card").contains(&TITLE.to_owned())
            });
            cards(&mut app, CARDS + 1, "one card more, and only one");
        },
    );

    step(
        "the ghost opens a draft, and closed untouched it writes nothing",
        || {
            app.scroll_to("canvas", "create-ghost");
            app.scroll_above("canvas", "create-ghost", "canvas-keyboard-hint");
            app.at_rest("create-ghost");
            app.click("create-ghost");
            app.wait_for("editor-title");
            app.click("editor-close");
            app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
            // Under a header of two rows the top ones went out of view, and were not counted.
            app.scroll_top("canvas");
            cards(&mut app, CARDS + 1, "no card more");
        },
    );

    step("the kind menu opens the same draft as the button", || {
        app.click("new-note-kind");
        app.click("new-note-snippet");
        app.wait_for("editor-title");
        app.click("editor-close");
        app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
        cards(&mut app, CARDS + 1, "no card more");
    });

    step(
        "each kind opens from its own key, and writes nothing untouched",
        || {
            for (key, shift) in [(Key::N, false), (Key::N, true), (Key::L, true)] {
                let modifiers = if shift {
                    Modifiers::COMMAND | Modifiers::SHIFT
                } else {
                    Modifiers::COMMAND
                };
                app.press(key, modifiers);
                app.wait_for("editor-title");
                app.click("editor-close");
                app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
            }
            cards(&mut app, CARDS + 1, "no card more");
            assert!(app.has("canvas-keyboard-hint"));
        },
    );

    step("a note opened from its card shows what was written", || {
        app.click_where("note-card-open", |node| node.label() == Some(TITLE));
        app.eventually("the note is read back whole", |tree| {
            tree.find("editor-body")
                .and_then(egui::accesskit::Node::value)
                == Some(BODY)
        });
        app.click("editor-close");
    });

    let mut app = app.relaunch();

    step(
        "the note is read back from the library on a relaunch",
        || {
            app.fill("vault-passphrase", e2e::PASSPHRASE);
            app.click("vault-submit");
            app.wait_for("area-notes");
            app.eventually("its card is there", |tree| {
                tree.texts("note-card").contains(&TITLE.to_owned())
            });
        },
    );

    app.quit();
}
