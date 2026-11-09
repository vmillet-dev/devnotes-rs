//! A space exported from one machine and imported on another comes back arranged: the notes in
//! their folders, the folders cited by nobody left behind, and a second import adds nothing.
//!
//! « The other machine » is a second profile: a genuinely fresh library.

use e2e::{App, Tree, step};
use egui::accesskit::Node;
use egui::{Key, Modifiers};

const SPACE: &str = "Voyage";
const CITED: &str = "Perf";
const UNCITED: &str = "Jamais citée";
const FILED: &str = "EXPLAIN lent";
const LOOSE: &str = "Dump nocturne";

fn named(name: &str) -> impl Fn(&Node) -> bool + '_ {
    move |node| node.label() == Some(name)
}

fn folders_named(tree: &Tree, name: &str) -> usize {
    tree.texts("folder-option")
        .iter()
        .filter(|row| *row == name)
        .count()
}

fn write(app: &mut App, title: &str) {
    app.click("new-note");
    app.fill("editor-title", title);
    app.fill("editor-body", "SELECT 1;");
    app.click("editor-close");
    app.eventually("the note is a card", |tree| !tree.has("editor-title"));
}

fn make_folder(app: &mut App, name: &str) {
    app.click("folder-create-open");
    app.fill("folder-create-input", name);
    app.press(Key::Enter, Modifiers::NONE);
    app.eventually("the folder is listed", |tree| {
        folders_named(tree, name) == 1
    });
}

fn menu(app: &mut App, entry: &str) {
    app.click("file-menu");
    app.click(&format!("file-option-{entry}"));
}

fn report(app: &mut App) {
    app.wait_for("status-toast");
    app.click("status-dismiss");
    app.eventually("the report is dismissed", |tree| !tree.has("status-toast"));
}

#[test]
fn folders_travel() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("arranged.devnotes");
    let first = tempfile::tempdir().unwrap();
    let mut app = App::launch_answering("21_folders_travel", first.path(), &file);
    app.create_library();

    step("a space with a folder that holds a note", || {
        app.click("space-create-open");
        app.fill("space-create-input", SPACE);
        app.press(Key::Enter, Modifiers::NONE);
        app.eventually("the space is shown", |tree| tree.count("note-card") == 0);
        make_folder(&mut app, UNCITED);
        make_folder(&mut app, CITED);
        write(&mut app, LOOSE);
        app.click_where("folder-option", named(CITED));
        write(&mut app, FILED);
    });

    step("it leaves as a file", || {
        menu(&mut app, "exportSpace");
        app.click("passphrase-prompt-plain");
        report(&mut app);
        assert!(file.exists());
    });
    app.quit();

    let second = tempfile::tempdir().unwrap();
    let mut app = App::launch_answering("21_folders_travel", second.path(), &file);
    app.create_library();
    app.eventually("the samples are drawn", |tree| tree.has("note-card"));

    step("on the other side it arrives in its folder", || {
        menu(&mut app, "import");
        report(&mut app);
        app.click_where("space-option", named(SPACE));
        app.eventually("the two notes are drawn", |tree| {
            tree.count("note-card") == 2
        });
        let tree = app.tree();
        assert_eq!(folders_named(&tree, CITED), 1);
        assert_eq!(folders_named(&tree, UNCITED), 0, "cited by nobody");
        app.click_where("folder-option", named(CITED));
        app.eventually("the folder holds its note", |tree| {
            tree.count("note-card") == 1
                && tree.texts("note-card-title").contains(&FILED.to_owned())
        });
    });

    step("the same file a second time adds nothing", || {
        menu(&mut app, "import");
        report(&mut app);
        app.click_where("space-option", named(SPACE));
        app.eventually("still two notes", |tree| tree.count("note-card") == 2);
        assert_eq!(folders_named(&app.tree(), CITED), 1);
    });

    app.quit();
}
