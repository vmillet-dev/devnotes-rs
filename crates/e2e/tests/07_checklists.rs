//! Todo lists: made from the kind menu, typed a row at a time, split by Enter, pasted line by
//! line, ticked from the editor and from the card, reordered by Alt+arrows — the twin of the
//! grip's drag — and a row removed. A list's position is all the identity an item has.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

const TITLE: &str = "Release checklist";

/// The rows' words, top to bottom: a row's value, not its name.
fn rows(tree: &Tree) -> Vec<String> {
    tree.all("checklist-text")
        .into_iter()
        .map(|row| row.value().unwrap_or_default().to_owned())
        .collect()
}

fn ticks(tree: &Tree) -> Vec<bool> {
    tree.all("checklist-check")
        .into_iter()
        .map(|tick| tick.toggled() == Some(egui::accesskit::Toggled::True))
        .collect()
}

/// The list found by its title and opened from the keyboard: a list's card is mostly its ticks.
fn open(app: &mut App) {
    app.click("search-input");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(TITLE);
    app.eventually("the list alone", |tree| tree.count("note-card") == 1);
    app.press(Key::ArrowDown, Modifiers::NONE);
    app.eventually("the card has the focus", |tree| {
        tree.focused().starts_with("note-card-open")
    });
    app.press(Key::Enter, Modifiers::NONE);
    app.wait_for("checklist-add");
}

fn close(app: &mut App) {
    app.click("editor-close");
    app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
}

/// A new row, typed in.
fn add(app: &mut App, words: &str) {
    let before = app.tree().count("checklist-text");
    app.click("checklist-add");
    app.eventually("a new row has the focus", |tree| {
        tree.count("checklist-text") == before + 1 && tree.focused().starts_with("checklist-text")
    });
    app.type_text(words);
}

#[test]
fn checklists() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("07_checklists", profile.path());
    app.create_library();
    app.wait_for("note-card");

    step("a list is made from the kind menu, its rows typed", || {
        app.click("new-note-kind");
        app.click("new-note-checklist");
        app.fill("editor-title", TITLE);
        add(&mut app, "Tag the release");
        add(&mut app, "Write the changelog");
        add(&mut app, "Publish the binaries");
        close(&mut app);
        open(&mut app);
        app.eventually("the rows in the order added", |tree| {
            rows(tree)
                == [
                    "Tag the release",
                    "Write the changelog",
                    "Publish the binaries",
                ]
        });
        app.capture("checklist-editor");
    });

    step(
        "Enter cuts a row at the caret, Backspace joins it back",
        || {
            app.click_nth("checklist-text", 2);
            app.press(Key::End, Modifiers::NONE);
            for _ in 0.."binaries".len() {
                app.press(Key::ArrowLeft, Modifiers::NONE);
            }
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the row cut in two", |tree| {
                rows(tree).get(2..) == Some(&["Publish the ".to_owned(), "binaries".to_owned()][..])
            });
            app.press(Key::Backspace, Modifiers::NONE);
            app.eventually("joined back", |tree| rows(tree).len() == 3);
        },
    );

    step(
        "lines pasted into a row are as many items, their boxes read",
        || {
            add(&mut app, "");
            app.paste("- Freeze the merges\n- [x] Back the database up");
            app.eventually("two rows from the paste", |tree| {
                rows(tree).get(3..)
                    == Some(
                        &[
                            "Freeze the merges".to_owned(),
                            "Back the database up".to_owned(),
                        ][..],
                    )
            });
            app.eventually("the box pasted ticked", |tree| {
                ticks(tree) == [false, false, false, false, true]
            });
        },
    );

    step("a row is ticked, moved up by Alt+↑ and removed", || {
        app.click_nth("checklist-check", 0);
        app.eventually("the first row ticked", |tree| {
            ticks(tree).first() == Some(&true)
        });
        app.click_nth("checklist-text", 1);
        app.press(Key::ArrowUp, Modifiers::ALT);
        app.eventually("the second row first", |tree| {
            rows(tree).first().map(String::as_str) == Some("Write the changelog")
        });
        app.click_nth("checklist-remove", 4);
        app.eventually("the last row gone", |tree| rows(tree).len() == 4);
        close(&mut app);
    });

    step("the order and the ticks are written", || {
        open(&mut app);
        app.eventually("as left", |tree| {
            rows(tree)
                == [
                    "Write the changelog",
                    "Tag the release",
                    "Publish the binaries",
                    "Freeze the merges",
                ]
                && ticks(tree) == [false, true, false, false]
        });
        close(&mut app);
    });

    step("a row is ticked from the card without opening it", || {
        app.click_nth("note-card-item", 0);
        app.eventually("the editor stayed closed", |tree| !tree.has("editor-title"));
        open(&mut app);
        app.eventually("the first open row ticked", |tree| {
            ticks(tree).first() == Some(&true)
        });
        close(&mut app);
    });

    app.quit();
}
