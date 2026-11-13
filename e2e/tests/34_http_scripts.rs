//! Scripts run for real: the application relaunches itself as their host. One keeps a value of the
//! answer in a variable, a test reads the answer, the console hears them, the answer is kept as a
//! note, a file a script asks to send waits for a yes, and a throw before the send stops it and
//! leads back to its line.

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

fn tests_and_console(app: &mut App) {
    step("the tests of an answer, and what the console heard", || {
        app.click("http-section-tests");
        app.fill(
            "http-script-code",
            "test('the status is 200', () => { expect(res.status).toBe(200); });",
        );
        app.eventually("the test is typed", |tree| {
            tree.find("http-script-code")
                .and_then(Node::value)
                .is_some_and(|text| text.contains("toBe(200)"))
        });
        app.press(Key::Enter, Modifiers::COMMAND);
        app.wait_for("http-response-section-tests");
        app.click("http-response-section-tests");
        app.wait_for("http-tests-passed");
        app.click("http-console-unfold");
        app.wait_for("http-console-line");
        app.click("http-console-fold");
        app.wait_for("http-console-unfold");
        app.click("http-response-section-body");
    });
    step("the answer kept as a note", || {
        app.click("http-response-save-note");
        app.wait_for("save-as-note-submit");
        app.eventually("the dialog is filled", |tree| {
            tree.find("save-as-note-title")
                .and_then(Node::value)
                .is_some_and(|title| title.contains("Commander"))
        });
        app.click("save-as-note-submit");
        app.eventually("the dialog closes", |tree| !tree.has("save-as-note"));
    });
}

fn file_asked(app: &mut App, path: &str) {
    step("a file a script asks to send waits for a yes", || {
        app.click("http-section-scripts");
        app.click("http-script-phase-0");
        app.fill(
            "http-script-code",
            &format!("pm.request.body.update({{ mode: 'file', file: {{ src: '{path}' }} }});"),
        );
        app.eventually("the script is typed", |tree| {
            tree.find("http-script-code")
                .and_then(Node::value)
                .is_some_and(|text| text.contains("id_ed25519"))
        });
        app.press(Key::Enter, Modifiers::COMMAND);
        app.eventually("the file is named", |tree| {
            tree.text("http-script-file-confirm")
                .is_some_and(|said| said.contains("id_ed25519"))
        });
        app.capture("file-asked");
        app.click("http-script-file-cancel");
        app.eventually("nothing left", |tree| {
            !tree.has("http-script-file-confirm") && !tree.has("http-response-status")
        });
        app.press(Key::Enter, Modifiers::COMMAND);
        app.wait_for("http-script-file-confirm");
        app.click("http-script-file-send");
        app.wait_for("http-response-status");
    });
}

fn stopped(app: &mut App) {
    step(
        "a throw before the send stops it, and leads to its line",
        || {
            app.click("http-section-scripts");
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

    let outside = tempfile::tempdir().unwrap();
    let key = outside.path().join("id_ed25519");
    std::fs::write(&key, "not a key").unwrap();
    let path = key.display().to_string().replace('\\', "/");

    request(&mut app);
    kept_value(&mut app);
    tests_and_console(&mut app);
    file_asked(&mut app, &path);
    stopped(&mut app);
    app.quit_discarding();
}
