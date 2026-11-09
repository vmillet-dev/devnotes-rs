//! The palette, opened from a second launch as the global shortcut will open it, and from its
//! key in the window: it narrows to what is typed, opens on Enter, offers to create a note named
//! after a query that finds nothing, and lists the commands after `>`. What is searched is in the
//! samples' code, which no translation touches.

use e2e::{App, step};
use egui::{Key, Modifiers};

const CARDS: usize = 6;
const NOTHING: &str = "zq query matching nothing at all";

fn opened(app: &mut App) {
    app.eventually("the palette is open, typing in its field", |tree| {
        tree.focused().starts_with("palette-input")
    });
}

/// What the field holds, then the palette's answer, past its pause.
fn search(app: &mut App, words: &str) {
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(words);
}

fn editor_title(tree: &e2e::Tree) -> Option<String> {
    tree.find("editor-title")
        .and_then(|field| field.value())
        .map(str::to_owned)
}

#[test]
fn quick_palette() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("09_quick_palette", profile.path());
    app.create_library();
    app.eventually("the samples are drawn", |tree| {
        tree.count("note-card") == CARDS
    });

    step("a second launch opens it, on the recent notes", || {
        app.act("palette");
        opened(&mut app);
        app.eventually("recent notes, and nothing said empty", |tree| {
            tree.count("palette-option") > 0 && !tree.has("palette-empty")
        });
        app.capture("palette-recent");
    });

    step(
        "it narrows to what is typed, and Enter opens the note",
        || {
            search(&mut app, "withRetry");
            app.eventually("one note answers", |tree| tree.count("palette-option") == 1);
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the editor is open on it, the palette gone", |tree| {
                tree.has("editor-title") && !tree.has("palette-input")
            });
            app.click("editor-close");
            app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
        },
    );

    step(
        "a query that finds nothing offers a note named after it",
        || {
            app.press(Key::P, Modifiers::COMMAND | Modifiers::ALT);
            opened(&mut app);
            search(&mut app, NOTHING);
            app.eventually(
                "the offer replaces the list, and says nothing matched",
                |tree| {
                    tree.has("palette-create")
                        && tree.has("palette-empty")
                        && tree.count("palette-option") == 0
                },
            );
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the note opens titled, already written", |tree| {
                editor_title(tree).as_deref() == Some(NOTHING)
            });
            app.click("editor-close");
            app.eventually("one card more", |tree| tree.count("note-card") == CARDS + 1);
        },
    );

    step("`>` lists the commands, and Escape closes", || {
        app.press(Key::P, Modifiers::COMMAND | Modifiers::ALT);
        opened(&mut app);
        app.type_text(">");
        app.eventually("the commands", |tree| {
            tree.count("palette-option") >= 5 && tree.has("palette-group")
        });
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the palette is closed", |tree| !tree.has("palette-input"));
    });

    app.quit();
}
