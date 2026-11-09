//! The HTTP area: a collection begun from the empty rail, a folder and requests made from a row's
//! menu, a request moved with the keys, renamed in place, a fold kept across a relaunch, and the
//! collection deleted once the rail has said what goes with it. A request opens in its tab,
//! reopened at the next launch; a draft asks before it is closed. A body is read as it is typed,
//! and a GraphQL draft names the variables it lacks.
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

fn tabs(tree: &Tree) -> Vec<String> {
    tree.texts("http-tab-open")
}

/// The open tabs, in no order: a request clicked in the rail opens, the rows moved above too.
fn open(tree: &Tree) -> Vec<String> {
    let mut open = tabs(tree);
    open.sort();
    open
}

fn in_tabs(app: &mut App) {
    step(
        "a request opens in its tab, and a draft asks before it closes",
        || {
            app.click_where("http-node-name", named("GET Login"));
            app.eventually("the requests clicked have their tabs", |tree| {
                open(tree) == ["GET Login", "WS Flux live"]
            });
            app.click("http-tab-new");
            app.eventually("a draft opens beside them", |tree| tabs(tree).len() == 3);
            app.click_nth("http-tab-close", 2);
            app.wait_for("http-close-dialog");
            app.click("http-close-discard");
            app.eventually("the draft is gone", |tree| {
                open(tree) == ["GET Login", "WS Flux live"] && !tree.has("http-close-dialog")
            });
        },
    );
}

/// What the fields wearing `author_id` hold, top to bottom.
fn values(tree: &Tree, author_id: &str) -> Vec<String> {
    tree.all(author_id)
        .into_iter()
        .map(|node| node.value().unwrap_or_default().to_owned())
        .collect()
}

/// Typed at the end of what a field holds.
fn append(app: &mut App, author_id: &str, words: &str) {
    app.click(author_id);
    app.press(Key::End, Modifiers::NONE);
    app.type_text(words);
}

fn compose(app: &mut App) {
    step(
        "the query and its table kept in step, a header split, Ctrl+S",
        || {
            app.click_where("http-tab-open", named("GET Login"));
            app.wait_for("http-url");
            append(app, "http-url", "?page=2&q=x");
            app.eventually("the table follows the URL", |tree| {
                values(tree, "http-kv-key") == ["page", "q", ""]
            });
            app.click_nth("http-kv-enabled", 0);
            app.eventually("the URL follows the table", |tree| {
                tree.find("http-url")
                    .and_then(Node::value)
                    .is_some_and(|url| url == "?q=x")
            });
            app.click("http-section-headers");
            app.wait_for("http-implied-headers");
            app.click_nth("http-kv-key", 0);
            app.type_text("Accept: application/json");
            app.eventually("the header is split into its two cells", |tree| {
                values(tree, "http-kv-key").first().map(String::as_str) == Some("Accept")
                    && values(tree, "http-kv-value").first().map(String::as_str)
                        == Some("application/json")
            });
            app.press(Key::S, Modifiers::COMMAND);
            app.eventually("the tab is saved", |tree| {
                tabs(tree).contains(&"GET Login".to_owned())
            });
        },
    );
}

fn send(app: &mut App) {
    step(
        "sent from Rust on Ctrl+Enter: its status, time, size and body, then cleared",
        || {
            let port = e2e::serve();
            app.click("http-url");
            app.press(Key::A, Modifiers::COMMAND);
            app.type_text(&format!("http://127.0.0.1:{port}/json"));
            app.press(Key::Enter, Modifiers::COMMAND);
            app.eventually("the answer is shown", |tree| {
                tree.text("http-response-status").as_deref() == Some("200 OK")
                    && tree.has("http-response-time")
                    && tree.has("http-response-size")
                    && tree
                        .text("http-response-pretty")
                        .is_some_and(|body| body.contains("Ada"))
            });
            app.click("http-response-section-timeline");
            app.eventually("the exchange is shown", |tree| {
                tree.text("http-response-sent")
                    .is_some_and(|sent| sent.starts_with("GET http://127.0.0.1"))
            });
            app.click("http-response-clear");
            app.wait_for("http-response-idle");
        },
    );
    step("the send in the history, read again, then reopened", || {
        app.click("http-history-open");
        app.wait_for("http-history-item");
        app.click("http-history-item");
        app.wait_for("http-history-url");
        app.eventually("it reads like an answer", |tree| {
            tree.text("http-response-status").as_deref() == Some("200 OK")
        });
        app.click("http-history-reopen");
        app.wait_for("http-history-replace");
        app.click("http-history-replace-confirm");
        app.wait_for("http-url");
        app.eventually("the page has gone", |tree| !tree.has("http-history"));
    });
    step(
        "the cookie the answer set, in the jar, then thrown away",
        || {
            app.click("http-cookies-open");
            app.eventually("the cookie is listed", |tree| {
                tree.text("http-cookie").as_deref() == Some("session")
            });
            app.click("http-cookie-remove");
            app.wait_for("http-cookies-empty");
            app.click("http-cookies-close");
            app.eventually("the jar is closed", |tree| !tree.has("http-cookies-dialog"));
        },
    );
}

fn bodies(app: &mut App) {
    step(
        "a JSON body read as it is typed, said on a GET, taken back to none",
        || {
            app.click("http-section-body");
            app.click("http-body-kind-1");
            app.fill("http-body-text", "{\"amount\":4900}");
            app.eventually("the body reads and its type is said", |tree| {
                tree.has("http-body-valid")
                    && tree
                        .text("http-body-type")
                        .is_some_and(|said| said.contains("application/json"))
            });
            app.wait_for("http-body-unusual");
            app.wait_for("http-body-unusual-mark");
            app.click("http-body-to-none");
            app.eventually("no body, no warning", |tree| {
                !tree.has("http-body-unusual") && !tree.has("http-body-text")
            });
        },
    );
    step("a GraphQL draft: its variable named, then added", || {
        app.click("http-tab-new-kind");
        app.click("http-new-kind-graphql");
        app.fill(
            "http-graphql-query",
            "query One($code: ID!) { product(code: $code) { id } }",
        );
        app.wait_for("http-graphql-missing");
        app.click("http-graphql-add");
        app.eventually("the variable has its null", |tree| {
            !tree.has("http-graphql-missing")
                && values(tree, "http-graphql-variables")
                    .first()
                    .is_some_and(|text| text.contains("\"code\": null"))
        });
        app.click_nth("http-tab-close", 2);
        app.wait_for("http-close-dialog");
        app.click("http-close-discard");
        app.eventually("the draft is gone", |tree| tabs(tree).len() == 2);
    });
}

fn socket(app: &mut App) {
    step(
        "a socket connected, a message echoed, then disconnected",
        || {
            let port = e2e::serve_socket();
            app.click_where("http-tab-open", named("WS Flux live"));
            app.wait_for("http-socket-message");
            app.click("http-url");
            app.press(Key::A, Modifiers::COMMAND);
            app.type_text(&format!("ws://127.0.0.1:{port}/"));
            app.click("http-socket-connect");
            app.eventually("the socket is open", |tree| {
                tree.count("http-socket-entry") == 1
            });
            app.fill("http-socket-message", "ping");
            // A key overtakes what is still being typed: it would reach the area, not the message.
            app.eventually("the message is typed", |tree| {
                values(tree, "http-socket-message")
                    .first()
                    .map(String::as_str)
                    == Some("ping")
            });
            app.press(Key::Enter, Modifiers::COMMAND);
            app.eventually("the message came back", |tree| {
                tree.count("http-socket-entry") == 3
            });
            app.click("http-socket-connect");
            app.eventually("the socket is closed", |tree| {
                tree.count("http-socket-entry") == 4
            });
            app.press(Key::S, Modifiers::COMMAND);
        },
    );
}

fn fold_kept(app: App) -> App {
    let mut app = app;
    step("a folder folded, still folded after a relaunch", || {
        app.click_where("http-node-twisty", ends("Auth"));
        app.eventually("the folder is folded", |tree| {
            rows(tree) == [API, "Auth", "WS Flux live"]
        });
    });
    let mut app = app.relaunch_discarding();
    step("unlocked again, the HTTP area shows the fold", || {
        app.fill("vault-passphrase", e2e::PASSPHRASE);
        app.click("vault-submit");
        app.wait_for("area-http");
        app.eventually("the fold and the tab are kept", |tree| {
            rows(tree) == [API, "Auth", "WS Flux live"]
                && open(tree) == ["GET Login", "WS Flux live"]
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
                rows(tree).is_empty() && tree.has("http-rail-empty") && tabs(tree).is_empty()
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
    in_tabs(&mut app);
    compose(&mut app);
    send(&mut app);
    bodies(&mut app);
    socket(&mut app);
    let mut app = fold_kept(app);
    delete(&mut app);
    app.quit();
}
