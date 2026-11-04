//! `{{fields}}` in a snippet: copied, the note asks for them first, from the card as from the
//! keyboard; what is typed is kept, what is cancelled is not. The sample used is found by its
//! code, which no translation touches.

use e2e::{App, step};
use egui::{Key, Modifiers};

/// What the first field holds: its value, not its name.
fn first_value(tree: &e2e::Tree) -> String {
    tree.all("placeholder-input")
        .first()
        .and_then(|field| field.value())
        .unwrap_or_default()
        .to_owned()
}

/// The card's fill button, which a note with fields shows instead of a plain copy.
fn open_from_card(app: &mut App) {
    app.hover("note-card-open");
    app.click("note-card-fill");
    app.wait_for("placeholder-form-hint");
}

fn closed(app: &mut App) {
    app.eventually("the form is closed", |tree| {
        !tree.has("placeholder-form-hint")
    });
}

#[test]
fn placeholder_fields() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("08_placeholder_fields", profile.path());
    app.create_library();
    app.wait_for("note-card");
    app.fill("search-input", "5432");
    app.eventually("the snippet with fields, alone", |tree| {
        tree.count("note-card") == 1
    });

    step(
        "the keyboard asks for the fields too, in the first one",
        || {
            app.press(Key::ArrowDown, Modifiers::NONE);
            app.eventually("the card has the focus", |tree| {
                tree.focused().starts_with("note-card-open")
            });
            app.press(Key::C, Modifiers::NONE);
            app.eventually("the form, typing in its first field", |tree| {
                tree.focused().starts_with("placeholder-input")
            });
            app.capture("placeholder-form");
            app.press(Key::Escape, Modifiers::NONE);
            closed(&mut app);
        },
    );

    step("the card offers the fields instead of a plain copy", || {
        app.hover("note-card-open");
        app.wait_for("note-card-fill");
        assert!(!app.has("note-card-copy"));
    });

    step("what is typed is copied filled, and kept", || {
        open_from_card(&mut app);
        app.type_text("db.internal");
        app.press(Key::Enter, Modifiers::NONE);
        closed(&mut app);
        app.wait_for("status-toast");
        open_from_card(&mut app);
        app.eventually("the value typed comes back", |tree| {
            first_value(tree) == "db.internal"
        });
    });

    step("cancelled, the form writes nothing", || {
        app.click_nth("placeholder-input", 0);
        app.press(Key::A, Modifiers::COMMAND);
        app.type_text("db.discarded");
        app.click("placeholder-cancel");
        closed(&mut app);
        open_from_card(&mut app);
        app.eventually("the value kept is still there", |tree| {
            first_value(tree) == "db.internal"
        });
    });

    step("« copy as is » copies and closes", || {
        app.click("placeholder-copy-raw");
        closed(&mut app);
        app.wait_for("status-toast");
    });

    app.quit();
}
