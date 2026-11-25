//! About › Documentation: its pages, read without the network, a search that leads to a section,
//! a link from one page to another, and the way out. Addressed by the pages' names, the same in
//! both languages; what is searched is a word of both.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

fn title(tree: &Tree) -> String {
    tree.text("docs-title").unwrap_or_default()
}

#[test]
fn documentation() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("42_documentation", profile.path());
    app.create_library();

    step("the About menu opens it on the overview", || {
        app.click("about-open");
        app.click("about-documentation");
        app.wait_for("docs-dialog");
        app.wait_for("docs-card-start");
    });

    step("a card leads to its page, and the page to the next", || {
        let overview = title(&app.tree());
        app.click("docs-card-start");
        app.eventually("another page is read", |tree| title(tree) != overview);
        let start = title(&app.tree());
        app.click("docs-page-writing");
        app.eventually("a page is chosen in the list", |tree| title(tree) != start);
    });

    step("a search answers sections, and one of them is read", || {
        app.fill("docs-search", "Argon2id");
        app.eventually("the list gave way to the answers", |tree| {
            tree.has("docs-result") && !tree.has("docs-page-start")
        });
        app.click("docs-result");
        app.eventually("the list is back", |tree| tree.has("docs-page-start"));
    });

    step("Escape closes it", || {
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the window is closed", |tree| !tree.has("docs-dialog"));
    });
}
