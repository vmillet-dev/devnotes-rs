//! Formatting a snippet with Prettier: the button formats the sample written on one line and its
//! notice undoes it, Shift+Alt+F formats from the title and the result is kept, a syntax error
//! is said, the library's settings change the style, and a language Prettier does not read
//! leaves the button disabled. The samples are found by their code, which no translation touches.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

fn code(tree: &Tree) -> String {
    tree.find("editor-body")
        .and_then(|body| body.value())
        .unwrap_or_default()
        .to_owned()
}

fn notice(tree: &Tree) -> Option<String> {
    tree.find("editor-format-notice")
        .and_then(|notice| notice.description())
        .map(str::to_owned)
}

fn open(app: &mut App, found_by: &str) {
    app.click("search-input");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(found_by);
    app.eventually("the sample alone", |tree| tree.count("note-card") == 1);
    app.click("note-card-open");
    app.wait_for("editor-body");
}

fn close(app: &mut App) {
    app.click("editor-close");
    app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
}

#[test]
fn formatting() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("26_formatting", profile.path());
    app.create_library();
    app.wait_for("note-card");
    open(&mut app, "withRetry");

    step("the button formats, marks the lines and says so", || {
        app.click("editor-format");
        app.eventually("the code on several lines", |tree| {
            code(tree).lines().count() > 5 && notice(tree).as_deref() == Some("formatted")
        });
    });

    step("its notice takes the format back", || {
        app.click("editor-format-undo");
        app.eventually("the code on one line again", |tree| {
            code(tree).lines().count() == 1
        });
    });

    step("Shift+Alt+F formats from the title, and it is kept", || {
        app.click("editor-title");
        app.press(Key::F, Modifiers::SHIFT | Modifiers::ALT);
        app.eventually("formatted", |tree| code(tree).lines().count() > 5);
        // Here and not before the undo: a slow capture outlasts the notice it would click.
        app.capture("formatted");
        close(&mut app);
        open(&mut app, "withRetry");
        assert!(
            app.tree()
                .find("editor-body")
                .is_some_and(|body| { body.value().unwrap_or_default().lines().count() > 5 })
        );
    });

    step("the library's indentation is the next format's", || {
        app.click("editor-format-settings");
        app.click("prettier-indentation-2");
        app.press(Key::Escape, Modifiers::NONE);
        app.click("editor-format");
        app.eventually("four spaces a level", |tree| {
            code(tree)
                .lines()
                .any(|line| line.starts_with("    ") && !line.starts_with("     "))
        });
    });

    step("a syntax error is said, and nothing changes", || {
        app.click("editor-body");
        app.press(Key::End, Modifiers::COMMAND);
        app.type_text(")");
        let typed = code(&app.tree());
        app.click("editor-format");
        app.eventually("the error said", |tree| {
            notice(tree).as_deref() == Some("syntax")
        });
        assert_eq!(code(&app.tree()), typed);
        close(&mut app);
    });

    step(
        "a language Prettier does not read leaves the button disabled",
        || {
            open(&mut app, "5432");
            assert!(
                app.tree()
                    .find("editor-format")
                    .is_some_and(egui::accesskit::Node::is_disabled)
            );
            close(&mut app);
        },
    );

    app.quit();
}
