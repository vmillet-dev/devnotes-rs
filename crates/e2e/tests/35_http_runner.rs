//! A collection run: two requests over two iterations, each with its test, summed up, what failed
//! open on its assertion, nothing in the history; the rows of a data file; the order arranged kept
//! over a relaunch; and the quit that asks about a request left with changes.

use e2e::{App, Tree, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers};

const API: &str = "Execution";

fn ends(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label().is_some_and(|label| label.ends_with(name))
}

fn create(app: &mut App, name: &str) {
    app.fill("http-create-input", name);
    app.press(Key::Enter, Modifiers::NONE);
}

fn a_request(app: &mut App, name: &str, url: &str) {
    app.click_where("http-node-menu", ends(API));
    app.click("http-node-new");
    app.click("http-node-new-http");
    create(app, name);
    app.eventually("the request is listed", |tree| {
        tree.all("http-node-name")
            .iter()
            .any(|node| node.label().is_some_and(|label| label.ends_with(name)))
    });
    app.click_where("http-node-name", ends(name));
    app.fill("http-url", url);
    app.click("http-section-tests");
    app.wait_for("http-script-code");
    app.fill(
        "http-script-code",
        "test('found', () => { expect(res.status).toBe(200); });",
    );
    app.eventually("the test is typed", |tree| {
        tree.find("http-script-code")
            .and_then(Node::value)
            .is_some_and(|text| text.contains("toBe(200)"))
    });
    app.click("http-save");
    app.eventually("the request is saved", |tree| {
        tree.find("http-save").is_some_and(Node::is_disabled)
    });
}

fn collection(app: &mut App) {
    step("a collection of two requests with their tests", || {
        let port = e2e::serve();
        app.press(Key::Num3, Modifiers::COMMAND);
        app.wait_for("http-rail-empty");
        app.click("http-rail-empty-create");
        create(app, API);
        app.wait_for("http-node-menu");
        a_request(app, "Login", &format!("http://127.0.0.1:{port}/json"));
        a_request(app, "Client", &format!("http://127.0.0.1:{port}/missing"));
    });
}

fn open_runner(app: &mut App) {
    app.click_where("http-node-menu", ends(API));
    app.click("http-node-run");
    app.eventually("the order is read", |tree| {
        tree.count("http-runner-row") == 2
    });
}

fn figures(tree: &Tree) -> String {
    tree.text("http-runner-figures").unwrap_or_default()
}

fn over_iterations(app: &mut App) {
    step("a run over two iterations, summed up", || {
        open_runner(app);
        app.click("http-runner-iterations");
        app.press(Key::A, Modifiers::COMMAND);
        app.type_text("2");
        app.eventually("the field says two iterations", |tree| {
            tree.find("http-runner-iterations").and_then(Node::value) == Some("2")
        });
        app.click("http-runner-start");
        app.eventually("the run is summed up", |tree| {
            figures(tree).contains(" 2/4") && figures(tree).ends_with(" s")
        });
    });
    step("each iteration apart, the failed requests open", || {
        let tree = app.tree();
        assert_eq!(tree.count("http-runner-iteration"), 2);
        assert_eq!(tree.count("http-runner-result-passed"), 2);
        assert_eq!(tree.count("http-runner-result-failed"), 2);
        assert_eq!(tree.count("http-runner-test"), 2);
        app.click("http-runner-filter-failed");
        app.eventually("only the failed are shown", |tree| {
            tree.count("http-runner-result-passed") == 0
        });
        app.click("http-runner-filter-all");
    });
    step("a run writes no history", || {
        app.click("http-history-open");
        app.wait_for("http-history");
        app.eventually("nothing is listed", |tree| {
            tree.count("http-history-item") == 0
        });
    });
}

fn over_a_file(app: &mut App) {
    step("a data file gives an iteration a row", || {
        open_runner(app);
        app.click("http-runner-data-choose");
        app.wait_for("http-runner-data");
        app.click("http-runner-start");
        app.eventually("each row ran", |tree| {
            tree.count("http-runner-iteration") == 3
        });
    });
}

fn order_kept(app: &mut App) {
    step("the order moved by the keys", || {
        app.click_nth("http-runner-row", 0);
        // The arrows are the row's from the frame after it took the focus.
        app.eventually("the row has the focus", |tree| {
            tree.focused().contains("http-runner-row")
        });
        app.eventually("the row keeps it", |tree| {
            tree.focused().contains("http-runner-row")
        });
        app.press(Key::ArrowDown, Modifiers::ALT);
        app.eventually("the rows are swapped", |tree| {
            tree.all("http-runner-row")
                .first()
                .is_some_and(|row| row.label() == Some("Client"))
        });
    });
}

#[test]
fn http_runner() {
    let profile = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let csv = files.path().join("clients.csv");
    std::fs::write(&csv, "customer\ncus_1\ncus_2\ncus_3\n").unwrap();
    let mut app = App::launch_answering("35_http_runner", profile.path(), &csv);
    app.create_library();
    app.wait_for("note-card");

    collection(&mut app);
    over_iterations(&mut app);
    over_a_file(&mut app);
    order_kept(&mut app);

    let mut app = app.relaunch();
    step("the order is still the one arranged", || {
        app.fill("vault-passphrase", e2e::PASSPHRASE);
        app.click("vault-submit");
        app.wait_for("area-http");
        app.wait_for("http-node-menu");
        open_runner(&mut app);
        app.eventually("the order is the one kept", |tree| {
            tree.all("http-runner-row")
                .first()
                .is_some_and(|row| row.label() == Some("Client"))
        });
    });
    step("a quit asks about a request with changes", || {
        app.click_where("http-node-name", ends("Login"));
        app.wait_for("http-url");
        app.fill("http-url", "/changed");
        app.eventually("the request has changes", |tree| {
            tree.find("http-save")
                .is_some_and(|save| !save.is_disabled())
        });
        app.act("quit");
        app.wait_for("http-leave-dialog");
        app.click("http-leave-cancel");
        app.eventually("the question is gone", |tree| {
            !tree.has("http-leave-dialog")
        });
        app.act("quit");
        app.wait_for("http-leave-dialog");
        app.click("http-leave-save");
    });
    app.ended();
}
