//! Scripts run for real: the application relaunches itself as their host. One keeps a value of the
//! answer in a variable, a throw before the send stops it and leads back to its line.

use e2e::{App, Tree, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers};

fn ends(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label().is_some_and(|label| label.ends_with(name))
}

fn create(app: &mut App, name: &str) {
    app.fill("http-create-input", name);
    app.press(Key::Enter, Modifiers::NONE);
}

fn request(app: &mut App) {
    step("a request to the local server", || {
        let port = e2e::serve();
        app.press(Key::Num3, Modifiers::COMMAND);
        app.wait_for("http-rail-empty");
        app.click("http-rail-empty-create");
        create(app, "Boutique");
        app.wait_for("http-node-menu");
        app.click_where("http-node-menu", ends("Boutique"));
        app.click("http-node-new");
        app.click("http-node-new-http");
        create(app, "Commander");
        app.eventually("the request is listed", |tree| {
            tree.texts("http-node-name").len() == 2
        });
        app.click_where("http-node-name", ends("Commander"));
        app.fill("http-url", &format!("http://127.0.0.1:{port}/json"));
    });
}

fn counted(tree: &Tree) -> Vec<String> {
    tree.texts("http-section-count")
}

fn kept_value(app: &mut App) {
    step(
        "a script after the answer keeps a value in the globals",
        || {
            app.click("http-section-scripts");
            app.wait_for("http-script-code");
            app.click("http-script-phase-1");
            app.fill("http-script-code", "env.set('who', res.json().name);");
            app.eventually("the tab counts it", |tree| {
                counted(tree).contains(&"1".to_owned())
            });
            app.press(Key::Enter, Modifiers::COMMAND);
            app.wait_for("http-response-status");
            app.click("http-environments-open");
            app.eventually("the value is in the globals", |tree| {
                tree.all("http-variable-value")
                    .iter()
                    .any(|node| node.value() == Some("Ada"))
            });
            app.click("http-environments-open");
            app.wait_for("http-url");
        },
    );
}

fn stopped(app: &mut App) {
    step(
        "a throw before the send stops it, and leads to its line",
        || {
            app.click("http-script-phase-0");
            app.fill("http-script-code", "\nthrow new Error('refused');");
            // A key overtakes what is still being typed.
            app.eventually("the script is typed", |tree| {
                tree.find("http-script-code")
                    .and_then(Node::value)
                    .is_some_and(|text| text.contains("refused"))
            });
            app.press(Key::Enter, Modifiers::COMMAND);
            app.eventually("the stop is said", |tree| {
                tree.text("http-script-stop-message")
                    .is_some_and(|said| said.contains("refused"))
            });
            app.wait_for("http-script-stop-where");
            app.click("http-script-open");
            app.wait_for("http-script-code");
        },
    );
}

#[test]
fn http_scripts() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("34_http_scripts", profile.path());
    app.create_library();
    app.wait_for("note-card");

    request(&mut app);
    kept_value(&mut app);
    stopped(&mut app);
    app.quit();
}
