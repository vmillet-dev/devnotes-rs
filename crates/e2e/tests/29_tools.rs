//! The tools' home: reached from the notes on its shortcut, its search finding a tool by a keyword,
//! the tool opened from the keyboard, its star and its place among the recent ones over a relaunch.

use e2e::{App, Tree, step};
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
    remembered(app).quit();
}
