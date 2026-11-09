//! A note expanded over the cards, with the layout of its kind: a snippet's fields over its code
//! and its text filled under it, the details in a column beside the body and put away, a task
//! list narrowed to the tasks done, and a title typed before J: the draft is written when the
//! next note opens. The scenario writes its own notes: the samples' names are translated.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

const SHIFT_COMMAND: Modifiers = Modifiers::COMMAND.plus(Modifiers::SHIFT);
const RENAMED: &str = "Renamed while expanded";

fn open_title(tree: &Tree) -> String {
    tree.find("editor-title")
        .and_then(|title| title.value())
        .unwrap_or_default()
        .to_owned()
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
            app.click("new-note-kind");
            app.click("new-note-snippet");
            app.fill("editor-title", "Connexion");
            app.click("editor-body");
            app.paste("psql -h {{host}} -p {{port=5432}}");
            app.click("editor-title");
            app.wait_for("placeholder-input");
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
        app.press(Key::L, SHIFT_COMMAND);
        app.fill("editor-title", "Tâches");
        app.click("checklist-add");
        app.type_text("Préparer");
        app.press(Key::Enter, Modifiers::NONE);
        app.type_text("Déployer");
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
