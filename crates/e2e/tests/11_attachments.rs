//! Attachments: an empty strip that says so, a file chosen and kept under its name, an image shown
//! at once and grown in the lightbox, the card counting them, two files of one name, a removal
//! on the second click, and a file over the limit refused. The system's dialog is the one step no
//! driver reaches: the scenario answers it.

use e2e::{App, Tree, step};
use egui::{Key, Modifiers};

const TITLE: &str = "Note with a file";
const IMAGE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="120"><rect width="200" height="120" fill="#e5484d"/></svg>"##;

fn files(tree: &Tree) -> usize {
    tree.count("attachment-open")
}

fn open(app: &mut App) {
    app.click("search-input");
    app.press(Key::A, Modifiers::COMMAND);
    app.type_text(TITLE);
    app.eventually("the note alone", |tree| tree.count("note-card") == 1);
    app.click("note-card-open");
    app.wait_for("editor-title");
}

fn close(app: &mut App) {
    app.click("editor-close");
    app.eventually("the editor is closed", |tree| !tree.has("editor-title"));
}

#[test]
fn attachments() {
    let profile = tempfile::tempdir().unwrap();
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("diagram.svg");
    std::fs::write(&file, IMAGE).unwrap();
    let mut app = App::launch_answering("11_attachments", profile.path(), &file);
    app.create_library();
    app.wait_for("note-card");

    step("the strip says it is empty", || {
        app.click("new-note");
        app.fill("editor-title", TITLE);
        app.click("editor-body");
        app.type_text("step one");
        close(&mut app);
        open(&mut app);
        app.wait_for("attachment-empty");
        app.wait_for("attachment-add");
    });

    step("a file is kept, and an image shown at once", || {
        app.click("attachment-add");
        app.eventually("the file listed", |tree| files(tree) == 1);
        app.wait_for("attachment-zoom");
        assert!(!app.has("attachment-empty"));
        app.capture("attachment-strip");
    });

    step("the lightbox grows it and Escape takes it away", || {
        app.click("attachment-zoom");
        app.wait_for("lightbox-image");
        app.capture("lightbox");
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the lightbox gone", |tree| !tree.has("lightbox-image"));
        assert!(app.has("editor-title"), "only the lightbox went");
    });

    step("the card counts it and a reopen lists it", || {
        close(&mut app);
        app.wait_for("note-card-clip");
        open(&mut app);
        app.eventually("the file listed again", |tree| files(tree) == 1);
    });

    step("two files of one name are two records", || {
        app.click("attachment-add");
        app.eventually("both listed", |tree| files(tree) == 2);
    });

    step("a file goes on the second click, and only then", || {
        app.click_nth("attachment-remove", 0);
        assert_eq!(app.tree().count("attachment-open"), 2);
        app.click_nth("attachment-remove", 0);
        app.eventually("one left", |tree| files(tree) == 1);
    });

    step("a file over the limit is refused and says so", || {
        std::fs::write(&file, vec![b' '; 10 * 1024 * 1024 + 1]).unwrap();
        app.click("attachment-add");
        app.wait_for("error-banner");
        assert_eq!(app.tree().count("attachment-open"), 1);
    });

    app.quit();
}
