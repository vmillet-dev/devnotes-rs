//! Editing a snippet: title, code and context written on the way out, the footer counting what
//! the code holds, full screen and the folded header keeping the draft, Tab staying in the code
//! and the find bar finding in it. The Note part comes with the rich editor.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

const TITLE: &str = "Rollout under edit";

/// The code as the editor holds it.
fn code(tree: &Tree) -> String {
    tree.find("editor-body")
        .and_then(|body| body.value())
        .unwrap_or_default()
        .to_owned()
}

fn open(app: &mut App) {
    app.click("search-input");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(TITLE);
    app.eventually("the snippet alone", |tree| tree.count("note-card") == 1);
    app.click("note-card-open");
    app.wait_for("editor-body");
}

fn close(app: &mut App) {
    app.click("editor-close");
    app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
}

#[test]
fn editing_a_note() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("03_editing_a_note", profile.path());
    app.create_library();
    app.wait_for("note-card");

    step("a snippet written from a new note", || {
        app.click("new-note");
        app.fill("editor-title", TITLE);
        app.click("editor-body");
        app.type_text("kubectl rollout status");
        close(&mut app);
    });

    step("title, code and context are written on the way out", || {
        open(&mut app);
        app.click("editor-body");
        app.press(Key::A, Modifiers::COMMAND);
        app.type_text("kubectl rollout restart deployment/api");
        app.fill("editor-source", "runbooks/api.md");
        close(&mut app);
        open(&mut app);
        app.eventually("the code as typed", |tree| {
            code(tree) == "kubectl rollout restart deployment/api"
        });
        app.eventually("the context as typed", |tree| {
            tree.find("editor-source").and_then(|field| field.value()) == Some("runbooks/api.md")
        });
    });

    step("the footer counts one line in the singular", || {
        app.eventually("« 1 line »", |tree| {
            tree.text("editor-stats")
                .is_some_and(|stats| stats.contains("· 1 ligne ·") || stats.contains("· 1 line ·"))
        });
        app.capture("snippet-editor");
    });

    step(
        "Tab stays in the code, and the find bar finds in it",
        || {
            app.click("editor-body");
            app.press(Key::Home, Modifiers::COMMAND);
            app.press(Key::Tab, Modifiers::NONE);
            app.eventually("a level before the code", |tree| {
                code(tree).starts_with("  kubectl")
            });
            app.press(Key::F, Modifiers::COMMAND);
            app.wait_for("search-field");
            app.type_text("rollout");
            app.eventually("one match, selected", |tree| {
                tree.text("search-count").as_deref() == Some("1 / 1")
            });
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually("the bar is closed", |tree| !tree.has("search-field"));
        },
    );

    step("full screen and back keep the draft", || {
        app.click("editor-body");
        app.press(Key::End, Modifiers::COMMAND);
        app.type_text(" --watch");
        app.click("editor-fullscreen");
        app.click("editor-fullscreen");
        close(&mut app);
        open(&mut app);
        app.eventually("the draft written", |tree| code(tree).ends_with("--watch"));
    });

    step(
        "the header folds, and stays folded from note to note",
        || {
            app.click("editor-fold-header");
            app.wait_for("editor-header-summary");
            close(&mut app);
            open(&mut app);
            app.wait_for("editor-header-summary");
            assert!(!app.has("editor-source"));
            app.click("editor-header-summary");
            app.wait_for("editor-source");
            close(&mut app);
        },
    );

    app.quit();
}
