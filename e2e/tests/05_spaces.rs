//! Spaces from the library rail: made, renamed, pinned, chosen, remembered, and deleted with
//! their notes moved out first, or put in the trash. Only a real database proves the cascade
//! never fires.
//!
//! Nothing compares a seeded word: the samples are in the system's language. The space made
//! here is named by the scenario, and found by that name.

use e2e::{App, step};
use egui::accesskit::{Node, Toggled};
use egui::{Key, Modifiers};

const CARDS: usize = 6;
const DOOMED: &str = "Brouillon à jeter";

fn named(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label() == Some(name)
}

/// A row's ⋯, by the name its label carries in any language.
fn options_of(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label().is_some_and(|label| label.contains(name))
}

fn is_shown(tree: &e2e::Tree, name: &str) -> bool {
    tree.all("space-option")
        .into_iter()
        .any(|node| node.label() == Some(name) && node.toggled() == Some(Toggled::True))
}

fn deleted_with_its_notes(app: &mut App) {
    step(
        "a space deleted with its notes puts them in the trash",
        || {
            app.click("space-create-open");
            app.fill("space-create-input", "Brouillons");
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the new space is shown", |tree| {
                is_shown(tree, "Brouillons") && tree.count("note-card") == 0
            });
            app.click("new-note");
            app.fill("editor-title", DOOMED);
            app.fill("editor-body", "à jeter");
            app.click("editor-close");
            app.eventually("its note is a card", |tree| tree.count("note-card") == 1);
            app.click_where("space-edit", options_of("Brouillons"));
            app.click("space-delete-held");
            app.click("space-delete-held-1");
            app.click("space-delete");
            app.click("space-delete");
            app.eventually("the space is gone, its note not in the refuge", |tree| {
                tree.count("space-option") == 1 && tree.count("note-card") == CARDS
            });
            app.click("trash-open");
            app.eventually("the note waits in the trash", |tree| {
                tree.texts("trash-row-title").contains(&DOOMED.to_owned())
            });
            app.press(Key::Escape, Modifiers::NONE);
        },
    );
}

#[test]
fn spaces() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("05_spaces", profile.path());
    app.create_library();

    step(
        "a new library has one space, and it cannot be deleted",
        || {
            app.eventually("the samples are drawn", |tree| {
                tree.count("note-card") == CARDS
            });
            assert_eq!(app.tree().count("space-option"), 1);
            app.click("space-edit");
            app.wait_for("space-delete-blocked");
            assert!(!app.has("space-delete"));
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually("the panel is closed", |tree| {
                !tree.has("space-delete-blocked")
            });
        },
    );

    step("a space made from the rail is the one shown", || {
        app.click("space-create-open");
        app.fill("space-create-input", "Veille");
        app.press(Key::Enter, Modifiers::NONE);
        app.eventually("the new space is listed and shown", |tree| {
            is_shown(tree, "Veille") && tree.count("note-card") == 0
        });
        app.capture("rail-new-space");
    });

    step("a space renamed in its panel", || {
        app.click_where("space-edit", options_of("Veille"));
        app.wait_for("space-rename-input");
        app.press(Key::A, Modifiers::COMMAND);
        app.type_text("Lectures");
        app.press(Key::Enter, Modifiers::NONE);
        app.eventually("the space comes back under its new name", |tree| {
            let names = tree.texts("space-option");
            names.contains(&"Lectures".to_owned()) && !names.contains(&"Veille".to_owned())
        });
    });

    step(
        "pinned, it heads the list whatever its name, and falls back",
        || {
            app.click_where("space-edit", options_of("Lectures"));
            app.click("space-pin");
            app.eventually("the pinned space is first", |tree| {
                tree.texts("space-option").first().map(String::as_str) == Some("Lectures")
            });
            app.click("space-pin");
            app.eventually("the unpinned space is back in its name's order", |tree| {
                let names = tree.texts("space-option");
                names.len() == 2 && names.contains(&"Lectures".to_owned())
            });
            app.click_where("space-edit", options_of("Lectures"));
            app.eventually("the panel is closed", |tree| !tree.has("space-pin"));
        },
    );

    step("« all spaces » shows every note, a space its own", || {
        app.click("space-option-all");
        app.eventually("every card is back", |tree| {
            tree.count("note-card") == CARDS
        });
        app.click_where("space-option", named("Lectures"));
        app.eventually("the canvas narrowed to the space", |tree| {
            tree.count("note-card") == 0
        });
    });

    let mut app = app.relaunch();

    step("a relaunch opens on the space it was left on", || {
        app.fill("vault-passphrase", e2e::PASSPHRASE);
        app.click("vault-submit");
        app.wait_for("area-notes");
        app.eventually("the space chosen before is shown", |tree| {
            is_shown(tree, "Lectures") && tree.count("note-card") == 0
        });
    });

    step(
        "deleting a space says where its notes go, then moves them",
        || {
            app.click_where("space-edit", options_of("Lectures"));
            app.wait_for("space-delete-consequence");
            app.capture("rail-space-panel");
            app.click("space-delete");
            app.click("space-delete");
            app.eventually("the space is gone, its refuge shown", |tree| {
                tree.count("space-option") == 1 && tree.count("note-card") == CARDS
            });
        },
    );

    deleted_with_its_notes(&mut app);

    app.quit();
}
