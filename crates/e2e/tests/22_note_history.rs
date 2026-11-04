//! A note's history: nothing on a note nobody has edited, one body kept per edit, newest first,
//! a preview of what going back would change, and going back, which takes that version and every
//! newer one out of the list. Nothing on a todo list.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

const TITLE: &str = "History under test";
const LIST: &str = "List without history";

fn code(tree: &Tree) -> String {
    tree.find("editor-body")
        .and_then(|body| body.value())
        .unwrap_or_default()
        .to_owned()
}

/// The note titled `title`, found and opened from the keyboard: a list's card is mostly its
/// ticks.
fn open(app: &mut App, title: &str) {
    app.click("search-input");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(title);
    app.eventually("the note alone", |tree| tree.count("note-card") == 1);
    app.press(Key::ArrowDown, Modifiers::NONE);
    app.eventually("the card has the focus", |tree| {
        tree.focused().starts_with("note-card-open")
    });
    app.press(Key::Enter, Modifiers::NONE);
    app.wait_for("editor-title");
}

fn close(app: &mut App) {
    app.click("editor-close");
    app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
}

/// The code replaced by `text`, written on close.
fn rewrite(app: &mut App, text: &str) {
    open(app, TITLE);
    app.click("editor-body");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(text);
    close(app);
}

/// A row added to the list open, and typed in.
fn add(app: &mut App, words: &str) {
    let before = app.tree().count("checklist-text");
    app.click("checklist-add");
    app.eventually("a new row has the focus", |tree| {
        tree.count("checklist-text") == before + 1 && tree.focused().starts_with("checklist-text")
    });
    app.type_text(words);
}

fn todo_list_keeps_none(app: &mut App) {
    app.click("new-note-kind");
    app.click("new-note-checklist");
    app.fill("editor-title", LIST);
    add(app, "one");
    close(app);
    open(app, LIST);
    add(app, "two");
    close(app);
    open(app, LIST);
    assert!(!app.has("revisions-toggle"));
    close(app);
}

#[test]
fn note_history() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("22_note_history", profile.path());
    app.create_library();
    app.wait_for("note-card");

    step("a note nobody has edited shows no history", || {
        app.click("new-note");
        app.fill("editor-title", TITLE);
        app.click("editor-body");
        app.type_text("select 1");
        close(&mut app);
        open(&mut app, TITLE);
        assert!(!app.has("revisions-toggle"));
        close(&mut app);
    });

    step("each edit keeps the body before it, newest first", || {
        rewrite(&mut app, "select 2");
        rewrite(&mut app, "select 3");
        open(&mut app, TITLE);
        app.click("revisions-toggle");
        app.eventually("two versions kept", |tree| tree.count("revision-open") == 2);
    });

    step("a click previews what going back would change", || {
        app.click_nth("revision-open", 1);
        app.wait_for("revision-diff");
        app.eventually("« select 1 » comes back, « select 3 » goes", |tree| {
            let restored = tree.texts("diff-restored");
            let dropped = tree.texts("diff-dropped");
            restored.iter().any(|line| line.contains("select 1"))
                && dropped.iter().any(|line| line.contains("select 3"))
        });
        app.capture("revision-preview");
    });

    step(
        "going back puts the body in the editor, nothing newer kept",
        || {
            app.click("revision-restore");
            app.eventually("the oldest body back", |tree| code(tree) == "select 1");
            app.eventually("nothing newer kept", |tree| !tree.has("revisions-toggle"));
            close(&mut app);
            open(&mut app, TITLE);
            app.eventually("still the oldest body", |tree| code(tree) == "select 1");
            close(&mut app);
        },
    );

    step("a todo list keeps no history", || {
        todo_list_keeps_none(&mut app);
    });

    app.quit();
}
