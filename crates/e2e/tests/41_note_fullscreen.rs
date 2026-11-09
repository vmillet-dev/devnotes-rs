//! A note expanded over the cards, with the layout of its kind: a snippet's fields over its code
//! and its text filled under it, the details in a column beside the body and put away, a task
//! list narrowed to the tasks done, and a title typed before J: the draft is written when the
//! next note opens. The samples are found by their names, which no translation touches.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

const RENAMED: &str = "Renamed while expanded";

fn open_title(tree: &Tree) -> String {
    tree.find("editor-title")
        .and_then(|title| title.value())
        .unwrap_or_default()
        .to_owned()
}

fn search(app: &mut App, words: &str) {
    app.click("search-input");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(words);
    app.eventually("the sample alone", |tree| tree.count("note-card") == 1);
}

fn close(app: &mut App) {
    app.click("editor-close");
    app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
}

#[test]
fn note_fullscreen() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("41_note_fullscreen", profile.path());
    app.create_library();
    app.wait_for("note-card");
    // Room for the details beside the body.
    app.resize(1_440, 900);

    step(
        "a snippet expanded: its fields over the code, filled under it, details beside",
        || {
            search(&mut app, "psql connection");
            app.click("note-card-open");
            app.click("editor-fullscreen");
            app.eventually("the column and the filled text", |tree| {
                !tree.has("note-card") && tree.has("editor-space") && tree.has("editor-filled")
            });
            app.click_nth("placeholder-input", 0);
            app.type_text("db.internal");
            app.click("editor-title");
            app.eventually("one field of two filled", |tree| {
                tree.find("placeholder-panel-toggle")
                    .and_then(|toggle| toggle.value())
                    == Some("1/2")
            });
            app.capture("snippet");
        },
    );

    step("the details are put away, then shown again", || {
        app.click("editor-details");
        app.eventually("the column gone", |tree| !tree.has("editor-space"));
        app.click("editor-details");
        app.wait_for("editor-space");
        close(&mut app);
    });

    step("a task list expanded narrows to the tasks done", || {
        search(&mut app, "Things to try");
        app.click("note-card-open");
        app.wait_for("checklist-shown-0");
        let all = app.tree().count("checklist-text");
        app.click_nth("checklist-check", 0);
        app.click("checklist-shown-2");
        app.eventually("the task done, alone", |tree| {
            tree.count("checklist-text") == 1
        });
        app.capture("tasks");
        app.click("checklist-shown-0");
        app.eventually("every task again", |tree| {
            tree.count("checklist-text") == all
        });
        close(&mut app);
    });

    step(
        "a title typed is written when J opens the next note",
        || {
            app.click("search-input");
            app.press(Key::A, Modifiers::COMMAND);
            app.press(Key::Backspace, Modifiers::NONE);
            app.eventually("every card", |tree| tree.count("note-card") > 2);
            app.click_nth("note-card-open", 0);
            app.wait_for("editor-position");
            app.click("editor-title");
            app.press(Key::A, Modifiers::COMMAND);
            app.type_text(RENAMED);
            // The title takes the keys: Escape gives it up first.
            app.press(Key::Escape, Modifiers::NONE);
            app.press(Key::J, Modifiers::NONE);
            app.eventually("the next note open", |tree| {
                tree.has("editor-title") && open_title(tree) != RENAMED
            });
            // Stepped to, the note leaves the keys to J and K.
            app.press(Key::K, Modifiers::NONE);
            app.eventually("the draft written", |tree| open_title(tree) == RENAMED);
        },
    );

    app.quit();
}
