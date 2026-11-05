//! The HTTP area: a collection begun from the empty rail, a folder and requests made from a row's
//! menu, a request moved with the keys, renamed in place, a fold kept across a relaunch, and the
//! collection deleted once the rail has said what goes with it.
//!
//! Rows are read by their names, which the scenario types: the window speaks the system's
//! language.

use e2e::{App, Tree, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers};

const API: &str = "API Paiements";

/// The rail's rows, top to bottom: a request after its method.
fn rows(tree: &Tree) -> Vec<String> {
    tree.texts("http-node-name")
}

fn ends(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label().is_some_and(|label| label.ends_with(name))
}

fn named(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label() == Some(name)
}

fn create(app: &mut App, name: &str) {
    app.fill("http-create-input", name);
    app.press(Key::Enter, Modifiers::NONE);
}

/// A row's `⋯`, then an entry of its menu, a sub-menu's through its parent.
fn choose(app: &mut App, row: &str, entries: &[&str]) {
    app.click_where("http-node-menu", ends(row));
    for entry in entries {
        app.click(entry);
    }
}

fn organise(app: &mut App) {
    step("a collection begun from the empty rail", || {
        app.press(Key::Num3, Modifiers::COMMAND);
        app.wait_for("http-rail-empty");
        app.click("http-rail-empty-create");
        create(app, API);
        app.eventually("the collection is listed", |tree| rows(tree) == [API]);
    });
    step(
        "a folder and two requests made from the rows' menus",
        || {
            choose(app, API, &["http-node-new-folder"]);
            create(app, "Auth");
            app.eventually("the folder is listed", |tree| rows(tree) == [API, "Auth"]);
            choose(app, "Auth", &["http-node-new", "http-node-new-http"]);
            create(app, "Login");
            app.eventually("the request is listed", |tree| {
                rows(tree) == [API, "Auth", "GET Login"]
            });
            choose(app, "Auth", &["http-node-new", "http-node-new-websocket"]);
            create(app, "Flux");
            app.eventually("the socket is listed", |tree| {
                rows(tree) == [API, "Auth", "GET Login", "WS Flux"]
            });
        },
    );
    step(
        "a request moved with the keys, then out of its folder",
        || {
            app.click_where("http-node-name", named("WS Flux"));
            app.press(Key::ArrowUp, Modifiers::ALT);
            app.eventually("the socket is first", |tree| {
                rows(tree) == [API, "Auth", "WS Flux", "GET Login"]
            });
            app.eventually("the move is said", |tree| {
                tree.text("http-rail-moved")
                    .is_some_and(|said| said.contains("Login"))
            });
            app.press(Key::ArrowLeft, Modifiers::ALT);
            app.eventually("the socket is at the collection's root", |tree| {
                rows(tree) == [API, "Auth", "GET Login", "WS Flux"]
            });
            // Nothing is above a folder's first request but its folder: refused, and said.
            app.click_where("http-node-name", named("GET Login"));
            app.press(Key::ArrowRight, Modifiers::ALT);
            app.eventually("the refusal is said", |tree| {
                tree.text("http-rail-moved")
                    .is_some_and(|said| said.contains("Login"))
            });
        },
    );
    step("a request renamed in place, the field left", || {
        app.click_where("http-node-name", named("WS Flux"));
        app.press(Key::F2, Modifiers::NONE);
        app.wait_for("http-rename-input");
        app.press(Key::End, Modifiers::NONE);
        app.type_text(" live");
        app.click("http-rail-title");
        app.eventually("the new name is read back", |tree| {
            rows(tree).contains(&"WS Flux live".to_owned()) && !tree.has("http-rename-input")
        });
    });
}

fn fold_kept(app: App) -> App {
    let mut app = app;
    step("a folder folded, still folded after a relaunch", || {
        app.click_where("http-node-twisty", ends("Auth"));
        app.eventually("the folder is folded", |tree| {
            rows(tree) == [API, "Auth", "WS Flux live"]
        });
    });
    let mut app = app.relaunch();
    step("unlocked again, the HTTP area shows the fold", || {
        app.fill("vault-passphrase", e2e::PASSPHRASE);
        app.click("vault-submit");
        app.wait_for("area-http");
        app.eventually("the fold is kept", |tree| {
            rows(tree) == [API, "Auth", "WS Flux live"]
        });
    });
    app
}

fn delete(app: &mut App) {
    step(
        "the collection deleted after saying what goes with it",
        || {
            choose(app, API, &["http-node-delete"]);
            app.wait_for("http-delete-count");
            app.click("http-delete-cancel");
            app.eventually("cancel keeps it", |tree| {
                !tree.has("http-delete-confirm") && rows(tree).len() == 3
            });
            choose(app, API, &["http-node-delete"]);
            app.click("http-delete-submit");
            app.eventually("the rail is empty again", |tree| {
                rows(tree).is_empty() && tree.has("http-rail-empty")
            });
        },
    );
}

#[test]
fn http() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("30_http", profile.path());
    app.create_library();
    app.wait_for("note-card");

    organise(&mut app);
    let mut app = fold_kept(app);
    delete(&mut app);
    app.quit();
}
