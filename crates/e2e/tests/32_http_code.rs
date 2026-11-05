//! A request written for another tool: a draft saved into the collection its first save makes,
//! an environment's secret left as a variable until a yes, and the targets of the list.

use e2e::{App, Tree, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers};

fn code(tree: &Tree) -> String {
    tree.find("http-code-output")
        .and_then(Node::label)
        .unwrap_or_default()
        .to_owned()
}

fn named(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label() == Some(name)
}

fn saved_draft(app: &mut App) {
    step("a draft saved into the collection it lacks", || {
        app.press(Key::Num3, Modifiers::COMMAND);
        app.wait_for("http-rail-empty");
        app.click("http-tab-new");
        app.fill("http-url", "https://api.exemple.fr/{{path}}");
        app.press(Key::S, Modifiers::COMMAND);
        app.wait_for("http-save-dialog");
        app.fill("http-save-collection", "Facturation");
        app.click("http-save-submit");
        app.eventually("the request is in the rail", |tree| {
            tree.texts("http-node-name").len() == 2 && !tree.has("http-save-dialog")
        });
    });
}

fn secret(app: &mut App) {
    step("an environment that sets a secret and a path", || {
        app.click("http-environment-picker");
        app.click("http-environment-manage");
        app.click("http-environment-create-open");
        app.fill("http-environment-create-input", "Staging");
        app.click("http-environment-create-submit");
        app.wait_for("http-environment-name");
        app.click_nth("http-variable-name", 0);
        app.type_text("path");
        app.click_nth("http-variable-value", 0);
        app.type_text("invoices");
        app.click("http-environment-use");
        app.eventually("it is the active one", |tree| {
            !tree.has("http-environment-use")
        });
        app.click("http-environments-open");
        app.wait_for("http-url");
    });
}

fn generated(app: &mut App) {
    step("cURL from the active environment, then as written", || {
        app.click("http-code-open");
        app.wait_for("http-code-dialog");
        app.eventually("the URL is resolved", |tree| {
            code(tree).contains("https://api.exemple.fr/invoices")
        });
        app.click("http-code-resolve");
        app.eventually("the URL is as written", |tree| {
            code(tree).contains("https://api.exemple.fr/{{path}}")
        });
        app.click("http-code-resolve");
        app.eventually("resolved again", |tree| code(tree).contains("/invoices"));
    });
    step("other targets, one of them found by the filter", || {
        app.click_where("http-code-target", named("JavaScript · fetch"));
        app.eventually("the fetch call", |tree| code(tree).contains("await fetch("));
        app.fill("http-code-filter", "elixir");
        app.click_where("http-code-target", named("Elixir · Req"));
        app.eventually("the Req call", |tree| code(tree).contains("Req.request!("));
        app.click("http-code-copy");
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the dialog closes", |tree| !tree.has("http-code-dialog"));
    });
}

#[test]
fn http_code() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("32_http_code", profile.path());
    app.create_library();
    app.wait_for("note-card");

    saved_draft(&mut app);
    secret(&mut app);
    generated(&mut app);
    app.quit();
}
