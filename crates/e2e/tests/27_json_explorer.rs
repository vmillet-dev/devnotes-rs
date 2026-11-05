//! A JSON snippet explored: the band says it parses, the graph draws its nodes and selects one
//! with its path, a row closes and opens its container, the arrows walk, everything folds and
//! opens again, a match is stepped to and shown in the code, the tree folds, the code and the
//! graph side by side select in each other. The sample used is found by its code, and every
//! entry by its path, which no translation touches.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

fn selected(tree: &Tree) -> String {
    tree.text("json-selected-path").unwrap_or_default()
}

fn nodes(tree: &Tree) -> usize {
    tree.count("json-node-head")
}

fn at(path: &'static str) -> impl Fn(&egui::accesskit::Node) -> bool {
    move |node| node.description() == Some(path)
}

fn invalid(tree: &Tree) -> bool {
    tree.find("editor-json-validity")
        .is_some_and(|node| node.invalid().is_some())
}

#[test]
fn json_explorer() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("27_json_explorer", profile.path());
    app.create_library();
    app.wait_for("note-card");
    app.fill("search-input", "p95Ms");
    app.eventually("the JSON sample, alone", |tree| {
        tree.count("note-card") == 1
    });

    step("the band says the draft parses, over its code", || {
        app.click("note-card-open");
        app.wait_for("editor-json-validity");
        app.eventually("the draft read as valid", |tree| !invalid(tree));
    });

    step("the graph draws the document and selects a node", || {
        app.click("editor-view-graph");
        app.wait_for("json-node-head");
        app.click("json-fit");
        app.eventually("every node", |tree| nodes(tree) == 6);
        app.click_where("json-node-head", at("$.owner"));
        app.eventually("its path said", |tree| selected(tree) == "$.owner");
        app.capture("graph");
    });

    step("a row closes and opens its container", || {
        app.click_where("json-row", at("$.endpoints"));
        app.eventually("the endpoints closed", |tree| nodes(tree) == 2);
        app.click_where("json-row", at("$.endpoints"));
        app.eventually("one level open again", |tree| nodes(tree) == 3);
    });

    step("the arrows walk the rows", || {
        app.click_where("json-node-head", at("$"));
        app.press(Key::ArrowDown, Modifiers::NONE);
        app.eventually("the first row", |tree| selected(tree) == "$.service");
    });

    step(
        "everything folds and opens, the selection falling back and coming back",
        || {
            app.click("json-open-all");
            app.eventually("every node", |tree| nodes(tree) == 6);
            app.click_where("json-row", at("$.owner.team"));
            app.click("json-collapse-all");
            app.eventually("the root alone", |tree| nodes(tree) == 1);
            app.eventually("the row still drawn", |tree| selected(tree) == "$.owner");
            app.click("json-open-all");
            app.eventually("back where it was", |tree| selected(tree) == "$.owner.team");
        },
    );

    step("a match is stepped to and shown in the code", || {
        app.fill("json-search", "ONCALL");
        app.eventually("one match", |tree| {
            tree.text("json-match-count").as_deref() == Some("1")
        });
        app.click("json-match-next");
        app.eventually("the match selected", |tree| {
            selected(tree) == "$.owner.onCall"
        });
        app.click("json-show-in-code");
        app.eventually("back to the code", |tree| {
            tree.has("editor-body") && !tree.has("json-node-head")
        });
    });

    step("the tree folds a container", || {
        app.click("editor-view-tree");
        app.click_where("json-line", at("$.owner"));
        app.eventually("the owner folded", |tree| {
            tree.all("json-line")
                .into_iter()
                .any(|line| at("$.owner")(line) && line.is_expanded() == Some(false))
        });
        app.capture("tree");
    });

    side_by_side(&mut app);
    broken_draft(&mut app);
    app.quit();
}

/// The code and the graph side by side, selecting in each other; the code hidden; too narrow.
fn side_by_side(app: &mut App) {
    step(
        "side by side the code and the graph select in each other",
        || {
            app.click("editor-view-graph");
            app.click("editor-side-by-side");
            app.eventually("the graph beside the code", |tree| {
                tree.has("side-by-side") && tree.has("side-by-side-code")
            });
            app.click("editor-body");
            app.press(Key::Home, Modifiers::COMMAND);
            app.press(Key::ArrowDown, Modifiers::NONE);
            app.press(Key::End, Modifiers::NONE);
            app.press(Key::ArrowLeft, Modifiers::NONE);
            app.press(Key::ArrowLeft, Modifiers::NONE);
            app.eventually("the value under the caret", |tree| {
                selected(tree) == "$.service"
            });
            app.capture("side-by-side");

            app.click("editor-hide-code");
            app.eventually("the graph alone", |tree| !tree.has("side-by-side"));
            app.click("editor-hide-code");
            app.wait_for("side-by-side");
            app.resize(720, 700);
            app.wait_for("editor-side-by-side-narrow");
            assert!(!app.has("side-by-side"), "too narrow, it is not squeezed");
            app.resize(1_100, 720);
            app.wait_for("side-by-side");
            app.click("editor-side-by-side");
            app.eventually("one at a time again", |tree| !tree.has("side-by-side"));
        },
    );
}

fn broken_draft(app: &mut App) {
    step("a broken draft says where it breaks", || {
        app.click("editor-view-code");
        app.click("editor-body");
        app.press(Key::End, Modifiers::COMMAND);
        app.type_text(",,");
        app.eventually("the draft refused", invalid);
        app.click("editor-view-graph");
        app.wait_for("json-invalid");
        app.click("editor-close");
        app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
    });
}
