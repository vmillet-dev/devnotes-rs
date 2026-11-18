//! The three areas: reached from the bar and by `Ctrl+1/2/3` read by position, each keeping its
//! state, the window reopening on the last one, and a key moved in the preferences.

use e2e::{App, PASSPHRASE, step};
use egui::{Key, Modifiers};

fn tools(app: &mut App) {
    app.press(Key::Num2, Modifiers::COMMAND);
    app.wait_for("area-tools");
}

fn notes(app: &mut App) {
    app.press(Key::Num1, Modifiers::COMMAND);
    app.wait_for("area-notes");
}

fn shortcuts(app: &mut App) {
    app.click("file-menu");
    app.click("file-preferences");
    app.click("settings-page-shortcuts");
    app.wait_for("shortcut-field-palette");
}

fn close_preferences(app: &mut App) {
    app.click("settings-close");
    app.eventually("the panel closed", |tree| !tree.has("settings-dialog"));
}

fn reach(app: &mut App, author_id: &str) {
    app.scroll_above("settings-content", author_id, "settings-cancel");
}

fn value_of(app: &mut App, author_id: &str) -> String {
    let tree = app.tree();
    let node = tree.find(author_id);
    node.and_then(egui::accesskit::Node::value)
        .unwrap_or_default()
        .to_owned()
}

/// The sidebar toggle's box in the area shown, once it holds still.
fn toggle_box(app: &mut App, author_id: &str) -> (f64, f64, f64, f64) {
    app.at_rest(author_id);
    let tree = app.tree();
    let bounds = tree
        .find(author_id)
        .and_then(egui::accesskit::Node::bounds)
        .expect("the toggle is drawn");
    (bounds.x0, bounds.y0, bounds.x1, bounds.y1)
}

/// The toggle stands at one place in the three areas.
fn one_toggle_place(app: &mut App) {
    let same = |a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)| {
        [a.0 - b.0, a.1 - b.1, a.2 - b.2, a.3 - b.3]
            .iter()
            .all(|gap| gap.abs() < 0.5)
    };
    step(
        "the sidebar toggle stands at one place in every area",
        || {
            let notes = toggle_box(app, "library-rail-toggle");
            app.click("area-option-tools");
            let tools = toggle_box(app, "tools-rail-toggle");
            assert!(
                same(tools, notes),
                "{tools:?} in the tools, {notes:?} in the notes"
            );
            app.click("area-option-http");
            let http = toggle_box(app, "http-rail-toggle");
            assert!(
                same(http, notes),
                "{http:?} in HTTP, {notes:?} in the notes"
            );
            app.click("area-option-notes");
            app.wait_for("area-notes");
        },
    );
}

#[test]
fn areas() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("28_areas", profile.path());
    app.create_library();
    app.eventually("the samples are drawn", |tree| tree.has("note-card"));

    step("the bar goes to the tools, and back", || {
        app.click("area-option-tools");
        app.wait_for("area-tools");
        assert!(!app.has("canvas"));
        app.click("area-option-notes");
        app.wait_for("area-notes");
    });

    one_toggle_place(&mut app);

    step(
        "the keys go there too, and the notes keep their search",
        || {
            app.click("search-input");
            app.type_text("zzz-nothing-matches");
            app.eventually("nothing matches", |tree| !tree.has("note-card"));
            tools(&mut app);
            notes(&mut app);
            app.eventually("the search is as it was", |tree| !tree.has("note-card"));
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually("the search is off", |tree| tree.has("note-card"));
        },
    );

    step("what the native side asks comes back to the notes", || {
        tools(&mut app);
        app.act("new-note");
        app.wait_for("editor-title");
        app.click("editor-close");
        app.eventually("the editor closed", |tree| !tree.has("editor-title"));
        assert!(app.has("area-notes"));
    });

    step("a global key is refused without a modifier", || {
        shortcuts(&mut app);
        let before = value_of(&mut app, "shortcut-field-palette");
        app.click("shortcut-field-palette");
        app.press(Key::P, Modifiers::NONE);
        app.wait_for("shortcut-error-palette");
        assert_eq!(value_of(&mut app, "shortcut-field-palette"), before);
    });

    step("a key another action holds is refused by its name", || {
        app.press(Key::N, Modifiers::COMMAND | Modifiers::ALT);
        app.eventually("the refusal names the key", |tree| {
            tree.text("shortcut-error-palette")
                .is_some_and(|said| said.contains("Ctrl+Alt+N"))
        });
    });

    step("a free one is taken, and put back", || {
        app.press(Key::J, Modifiers::COMMAND | Modifiers::ALT);
        app.eventually("the field shows it", |tree| {
            tree.find("shortcut-field-palette")
                .and_then(egui::accesskit::Node::value)
                == Some("Ctrl+Alt+J")
        });
        app.click("shortcut-reset-palette");
        app.eventually("the field is back", |tree| {
            tree.find("shortcut-field-palette")
                .and_then(egui::accesskit::Node::value)
                == Some("Ctrl+Alt+P")
        });
    });

    step("Escape ends the listening, not the panel", || {
        reach(&mut app, "shortcut-field-canvas.pin");
        app.click("shortcut-field-canvas.pin");
        // The field keeps Escape from the frame after it took the focus: `has` settles.
        assert!(app.has("settings-dialog"));
        app.press(Key::Escape, Modifiers::NONE);
        assert!(app.has("settings-dialog"));
    });

    step(
        "a canvas key takes a bare letter, and the canvas answers",
        || {
            reach(&mut app, "shortcut-field-canvas.check");
            app.click("shortcut-field-canvas.check");
            app.press(Key::J, Modifiers::NONE);
            app.eventually("the field shows it", |tree| {
                tree.find("shortcut-field-canvas.check")
                    .and_then(egui::accesskit::Node::value)
                    == Some("J")
            });
            close_preferences(&mut app);
            app.press(Key::ArrowDown, Modifiers::NONE);
            app.press(Key::J, Modifiers::NONE);
            app.wait_for("selection-count");
        },
    );

    tools(&mut app);
    let mut app = app.relaunch();
    app.fill("vault-passphrase", PASSPHRASE);
    app.click("vault-submit");

    step("the window reopens on the area it was left in", || {
        app.wait_for("area-tools");
    });

    app.quit();
}
