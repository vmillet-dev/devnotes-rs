//! The environment a request is sent with, and what a request sets for itself: the picker with no
//! environment yet, then an auth and a setting saved and read back after a relaunch.

use e2e::{App, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers};

const API: &str = "API Paiements";

fn ends(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label().is_some_and(|label| label.ends_with(name))
}

fn create(app: &mut App, name: &str) {
    app.fill("http-create-input", name);
    app.press(Key::Enter, Modifiers::NONE);
}

fn a_request(app: &mut App) {
    step("a collection and a request to work on", || {
        app.press(Key::Num3, Modifiers::COMMAND);
        app.wait_for("http-rail-empty");
        app.click("http-rail-empty-create");
        create(app, API);
        app.wait_for("http-node-menu");
        app.click_where("http-node-menu", ends(API));
        app.click("http-node-new");
        app.click("http-node-new-http");
        create(app, "Login");
        app.eventually("the request is listed", |tree| {
            tree.texts("http-node-name").len() == 2
        });
        app.click_where("http-node-name", ends("Login"));
        app.wait_for("http-url");
    });
}

fn picker(app: &mut App) {
    step("the picker offers the globals alone", || {
        app.click("http-environment-picker");
        app.eventually("one entry, none", |tree| {
            tree.count("http-environment-option") == 1
        });
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("it closes", |tree| !tree.has("http-environment-option"));
    });
}

fn own_parts(app: &mut App) {
    step(
        "an inherited auth said, a token and a timeout set, Ctrl+S",
        || {
            app.click("http-section-auth");
            app.wait_for("http-auth-inherited");
            app.click("http-auth-kind");
            app.click("http-auth-kind-3");
            app.fill("http-auth-secret", "s3cret");
            app.click("http-section-settings");
            app.fill("http-transport-timeoutMs", "5000");
            app.press(Key::Enter, Modifiers::NONE);
            app.eventually("the field is settled", |tree| {
                tree.find("http-transport-timeoutMs")
                    .and_then(Node::value)
                    .is_some_and(|value| value == "5000")
            });
            app.press(Key::S, Modifiers::COMMAND);
        },
    );
}

fn read_back(app: App) -> App {
    let mut app = app.relaunch();
    step("unlocked again, the secret is masked, then shown", || {
        app.fill("vault-passphrase", e2e::PASSPHRASE);
        app.click("vault-submit");
        app.wait_for("area-http");
        app.wait_for("http-url");
        app.click("http-section-auth");
        app.wait_for("http-auth-secret");
        app.click("http-auth-reveal");
        app.eventually("the token came back", |tree| {
            tree.find("http-auth-secret")
                .and_then(Node::value)
                .is_some_and(|value| value == "s3cret")
        });
    });
    app
}

#[test]
fn http_environments() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("31_http_environments", profile.path());
    app.create_library();
    app.wait_for("note-card");

    a_request(&mut app);
    picker(&mut app);
    own_parts(&mut app);
    let app = read_back(app);
    app.quit();
}
