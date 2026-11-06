//! Several libraries: created from the File menu and opened through the gate, each with its own
//! phrase and its own samples, a switch that asks before it loses an unsaved request, the way
//! between them from the gate itself, and a deletion named before it runs.

use e2e::{App, PASSPHRASE, Tree, step};
use egui::{Key, Modifiers};

const OTHER: &str = "Boulot";
/// Only the first library holds it, which is what tells the two rails apart.
const HOME: &str = "Maison";

fn panel(app: &mut App) {
    app.click("file-menu");
    app.click("file-libraries");
    app.wait_for("library-row");
}

fn close_panel(app: &mut App) {
    app.click("libraries-close");
    app.eventually("the panel is closed", |tree| !tree.has("library-row"));
}

/// The library the gate asks for, as its menu names it.
fn gate_library(tree: &Tree) -> String {
    tree.find("vault-library-switch")
        .and_then(egui::accesskit::Node::value)
        .unwrap_or_default()
        .to_owned()
}

fn pass_the_gate(app: &mut App) {
    app.fill("vault-passphrase", PASSPHRASE);
    if app.has("vault-confirmation") {
        app.fill("vault-confirmation", PASSPHRASE);
    }
    app.click("vault-submit");
    app.wait_for("area-notes");
}

fn has_space(tree: &Tree, name: &str) -> bool {
    tree.texts("space-option").iter().any(|row| row == name)
}

/// A request begun in the HTTP area and left there, then back to the notes.
fn leave_a_request_unsaved(app: &mut App) {
    app.press(Key::Num3, Modifiers::COMMAND);
    app.click("http-tab-new");
    app.wait_for("http-url");
    app.press(Key::Num1, Modifiers::COMMAND);
    app.wait_for("area-notes");
}

#[test]
fn libraries() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("25_libraries", profile.path());
    app.create_library();
    app.eventually("the samples are drawn", |tree| tree.has("note-card"));
    app.click("space-create-open");
    app.fill("space-create-input", HOME);
    app.press(Key::Enter, Modifiers::NONE);
    app.eventually("the space is listed", |tree| has_space(tree, HOME));

    step("the panel lists the one open, and says so", || {
        panel(&mut app);
        app.capture("libraries");
        let tree = app.tree();
        assert_eq!(tree.count("library-row"), 1);
        assert!(tree.has("library-open"));
        assert!(!tree.has("library-delete"), "the last cannot go");
    });

    step("creating one goes straight to it, through the gate", || {
        app.fill("library-new-name", OTHER);
        app.click("library-create");
        app.wait_for("vault-confirmation");
        assert!(!app.has("file-menu"));
        assert!(!app.has("library-row"), "the panel went with the library");
        pass_the_gate(&mut app);
    });

    step(
        "it is a library of its own, seeded as a first launch",
        || {
            app.eventually("its samples are drawn", |tree| tree.has("note-card"));
            assert!(!has_space(&app.tree(), HOME));
            panel(&mut app);
            assert_eq!(app.tree().count("library-row"), 2);
            close_panel(&mut app);
        },
    );

    step("a switch asks before it loses an unsaved request", || {
        leave_a_request_unsaved(&mut app);
        panel(&mut app);
        app.click("library-switch");
        app.wait_for("http-leave-dialog");
        assert_eq!(app.tree().count("http-leave-name"), 1);
        app.click("http-leave-cancel");
        app.eventually("the question is gone", |tree| {
            !tree.has("http-leave-dialog")
        });
        assert!(!app.has("vault-passphrase"), "nothing changed");

        app.click("library-switch");
        app.click("http-leave-discard");
        app.wait_for("vault-passphrase");
        assert!(!app.has("vault-confirmation"), "this one has its phrase");
    });

    step(
        "the gate goes from one library to the other, and back",
        || {
            app.click("vault-library-switch");
            app.click("vault-library-switch-1");
            app.eventually("the gate asks for the other", |tree| {
                gate_library(tree) == OTHER
            });
            app.click("vault-library-switch");
            app.click("vault-library-switch-0");
            app.eventually("the gate asks for the first", |tree| {
                tree.has("vault-library-switch") && gate_library(tree) != OTHER
            });
        },
    );

    step("what was left there is found", || {
        pass_the_gate(&mut app);
        app.eventually("its own spaces are listed", |tree| has_space(tree, HOME));
    });

    step("a deletion is named before it takes anything", || {
        panel(&mut app);
        app.click("library-delete");
        app.wait_for("library-confirm");
        let said = app.tree().text("library-confirm").unwrap_or_default();
        assert!(said.contains(OTHER), "{said}");
        assert_eq!(app.tree().count("library-row"), 2);
        app.click("library-confirm-delete");
        app.eventually("the library is erased", |tree| {
            tree.count("library-row") == 1
        });
    });

    step("the one that is left offers no deletion", || {
        assert!(!app.has("library-delete"));
        close_panel(&mut app);
    });

    app.quit();
}
