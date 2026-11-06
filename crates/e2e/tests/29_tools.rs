//! The tools' home: reached from the notes on its shortcut, its search finding a tool by a keyword,
//! the tool opened from the keyboard, its star and its place among the recent ones over a relaunch.

use e2e::{App, Tree, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers};

fn search_is_focused(tree: &Tree) -> bool {
    tree.focused().contains("tools-search")
}

fn home(app: &mut App) {
    step("the home opens on its search from the notes", || {
        app.press(Key::T, Modifiers::COMMAND | Modifiers::SHIFT);
        app.wait_for("area-tools");
        app.eventually("the search takes the focus", search_is_focused);
    });
    step("a keyword narrows the catalogue", || {
        app.fill("tools-search", "crlf");
        app.wait_for("tools-entry-line-breaks");
        app.eventually("the others are gone", |tree| {
            tree.find("tools-entry-hash").is_none()
        });
        app.press(Key::Escape, Modifiers::NONE);
        app.wait_for("tools-entry-hash");
    });
}

fn a_tool(app: &mut App) {
    step("Enter opens the first result, in its frame", || {
        app.fill("tools-search", "cron");
        app.wait_for("tools-first-result");
        app.press(Key::Enter, Modifiers::NONE);
        app.wait_for("tool-title");
        app.wait_for("tools-rail-tool-zones");
    });
    step("its star files it under the favourites", || {
        app.click("tool-favourite");
        app.wait_for("tools-rail-favourites");
    });
    step("Ctrl+K goes back to the search", || {
        app.press(Key::K, Modifiers::COMMAND);
        app.eventually("the search takes the focus", search_is_focused);
        app.press(Key::Escape, Modifiers::NONE);
        app.wait_for("tools-favourite-cron");
    });
}

fn text_tools(app: &mut App) {
    step("the case converter answers what is typed", || {
        app.click("tools-entry-case");
        app.wait_for("tool-title");
        app.fill("case-input", "parse URL");
        app.eventually("the cases are shown", |tree| {
            tree.texts("output-value")
                .iter()
                .any(|it| it == "parse_url")
        });
    });
    step("the diff compares two texts", || {
        app.click("tools-rail-tool-diff");
        app.fill("text-diff-a", "one");
        app.fill("text-diff-b", "two");
        app.wait_for("text-diff-rows");
        app.click("tool-clear");
        app.click("tool-back");
    });
}

/// By the search: the catalogue is longer than the window.
fn open_tool(app: &mut App, tool: &str) {
    app.fill("tools-search", tool);
    app.eventually("the catalogue is narrowed", |tree| {
        !tree.has("tools-entry-hash")
    });
    app.click(&format!("tools-entry-{tool}"));
    app.wait_for("tool-title");
}

/// With the search let go: what it holds stays on the home.
fn back_to_the_home(app: &mut App) {
    app.click("tool-back");
    app.click("tools-search");
    app.press(Key::Escape, Modifiers::NONE);
    app.wait_for("tools-entry-hash");
}

fn data_tools(app: &mut App) {
    step(
        "a document is converted, and the swap brings it back",
        || {
            open_tool(app, "convert");
            app.fill("convert-input", r#"{"service":"billing"}"#);
            app.eventually("the YAML is shown", |tree| {
                tree.text("convert-output")
                    .is_some_and(|it| it.contains("service: billing"))
            });
            app.click("convert-swap");
            app.eventually("the JSON is back", |tree| {
                tree.text("convert-output")
                    .is_some_and(|it| it.contains('{'))
            });
            back_to_the_home(app);
        },
    );
    step("a seed brings the same person back", || {
        open_tool(app, "identity");
        app.wait_for("identity-card");
        let drawn = app.tree().text("identity-surname");
        app.fill("identity-seed", "devnotes");
        app.wait_for("identity-seed-pinned");
        app.eventually("the seed's person is on the card", |tree| {
            tree.text("identity-surname") != drawn
        });
        let surname = app.tree().text("identity-surname");
        app.fill("identity-seed", "another");
        app.eventually("another person is drawn", |tree| {
            tree.text("identity-surname") != surname
        });
        app.fill("identity-seed", "devnotes");
        app.eventually("the first person is back", |tree| {
            tree.text("identity-surname") == surname
        });
        back_to_the_home(app);
    });
    step("a code is drawn from a URL", || {
        open_tool(app, "qr");
        app.fill("qr-url", "https://exemple.fr/doc");
        app.wait_for("qr-code");
        back_to_the_home(app);
    });
}

fn references(app: &mut App) {
    step("a code no tool answers is found in the references", || {
        app.fill("tools-search", "418");
        app.wait_for("tools-references");
        app.wait_for("tools-first-result");
        assert!(!app.has("tools-no-match"));
        app.press(Key::Enter, Modifiers::NONE);
        app.wait_for("reference-search");
        app.eventually("the table is whole, its own search empty", |tree| {
            tree.count("reference-row") > 3
                && tree.find("reference-search").and_then(Node::value) != Some("418")
        });
        back_to_the_home(app);
    });
    step("the rest of a reference opens it searched", || {
        app.fill("tools-search", "cors");
        app.click("tools-reference-more");
        app.eventually("the reference is searched for it", |tree| {
            tree.find("reference-search").and_then(Node::value) == Some("cors")
        });
        back_to_the_home(app);
    });
}

fn remembered(mut app: App) -> App {
    step("the star and the recent tools come back", || {
        app.click("tools-entry-case");
        app.wait_for("tool-title");
    });
    let mut app = app.relaunch();
    step("over a relaunch", || {
        app.fill("vault-passphrase", e2e::PASSPHRASE);
        app.click("vault-submit");
        app.wait_for("tools-favourite-cron");
        app.wait_for("tools-recent-case");
        let tree = app.tree();
        assert!(
            tree.text("tools-count")
                .is_some_and(|words| words.contains("32"))
        );
    });
    app
}

#[test]
fn tools_home() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("29_tools", profile.path());
    app.create_library();
    app.wait_for("note-card");

    home(&mut app);
    a_tool(&mut app);
    text_tools(&mut app);
    data_tools(&mut app);
    references(&mut app);
    remembered(app).quit();
}
