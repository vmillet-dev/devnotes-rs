//! Import, export and share from the File menu. Every operation reports, including when it
//! changed nothing; a sealed file opens only with its phrase; an export of nothing asks nothing
//! and leaves no file behind.
//!
//! Every dialog of the run answers the one path the scenario fixed. Nothing compares a word of
//! the reports: they are in the system's language, so what they name and the cards are read.

use std::path::Path;

use e2e::{App, step};
use egui::{Key, Modifiers};

const CARDS: usize = 6;
const TITLE: &str = "Worth exporting";
const FILE: &str = "library.devnotes";
const PHRASE: &str = "an export passphrase";
/// What an export before the archive looks like, from a version with a language unknown here.
const NEWER: &str = r#"{
  "version": 1,
  "exportedAt": "2026-11-06T09:00:00Z",
  "spaces": [{ "id": "elsewhere", "name": "Ailleurs" }],
  "notes": [{
    "id": "written-by-a-newer-devnotes",
    "spaceId": "elsewhere",
    "title": "Ahead of this build",
    "language": "from-the-future",
    "content": "echo later",
    "source": "",
    "tags": [],
    "pinned": false,
    "createdAt": "2026-11-06T09:00:00Z",
    "updatedAt": "2026-11-06T09:00:00Z",
    "lifecycle": { "kind": "permanent" }
  }]
}"#;

fn cards(app: &mut App, count: usize, what: &str) {
    app.eventually(what, |tree| tree.count("note-card") == count);
}

fn menu(app: &mut App, entry: &str) {
    app.click("file-menu");
    app.click(&format!("file-option-{entry}"));
}

/// The report of the last operation, left off the screen for the next one to be seen.
fn report(app: &mut App) -> String {
    app.wait_for("status-toast");
    let said = app.tree().text("status-toast").unwrap_or_default();
    app.click("status-dismiss");
    app.eventually("the report is dismissed", |tree| !tree.has("status-toast"));
    said
}

fn write(app: &mut App, title: &str) {
    app.click("new-note");
    app.fill("editor-title", title);
    app.fill("editor-body", "echo hello");
    app.click("editor-close");
    app.eventually("the note is a card", |tree| !tree.has("editor-title"));
}

/// The first card, purged: the second press of Delete writes, then the trash forgets it.
fn purge_first(app: &mut App) {
    app.press(Key::ArrowDown, Modifiers::NONE);
    app.press(Key::Delete, Modifiers::NONE);
    app.wait_for("note-card-arming");
    app.press(Key::Delete, Modifiers::NONE);
    app.click("undo-dismiss");
    app.click("trash-open");
    app.wait_for("trash-row");
    app.click("trash-purge");
    app.click("trash-purge");
    app.eventually("the trash is empty", |tree| tree.has("trash-empty-state"));
    app.press(Key::Escape, Modifiers::NONE);
    app.eventually("the panel is closed", |tree| !tree.has("trash-close"));
}

fn is_archive(file: &Path) -> bool {
    std::fs::read(file).is_ok_and(|bytes| bytes.starts_with(b"PK\x03\x04"))
}

/// The one file the library's key does not protect carries a key of its own.
fn sealed_file(app: &mut App, file: &Path) {
    step("a sealed file does not show what it holds", || {
        menu(app, "exportAll");
        app.fill("passphrase-prompt-field", PHRASE);
        app.fill("passphrase-prompt-confirmation", PHRASE);
        app.click("passphrase-prompt-submit");
        let said = report(app);
        assert!(said.contains(FILE), "{said}");
        let bytes = std::fs::read(file).unwrap();
        let readable = bytes.windows(TITLE.len()).any(|at| at == TITLE.as_bytes());
        assert!(!readable, "the title is readable in the file");
    });

    step("it opens with its phrase, and only with it", || {
        menu(app, "import");
        app.fill("passphrase-prompt-field", "not the phrase");
        app.click("passphrase-prompt-submit");
        app.eventually("the prompt says the phrase is wrong", |tree| {
            tree.text("passphrase-prompt-problem")
                .is_some_and(|said| !said.is_empty())
        });
        app.eventually("the prompt is open again", |tree| {
            tree.find("passphrase-prompt-field")
                .is_some_and(|field| !field.is_disabled())
        });
        app.fill("passphrase-prompt-field", PHRASE);
        app.click("passphrase-prompt-submit");
        report(app);
        app.eventually("the prompt closed", |tree| !tree.has("passphrase-prompt"));
    });
}

#[test]
fn library_transfer() {
    let profile = tempfile::tempdir().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join(FILE);
    let mut app = App::launch_answering("10_library_transfer", profile.path(), &file);
    app.create_library();
    cards(&mut app, CARDS, "the samples are drawn");
    write(&mut app, TITLE);
    cards(&mut app, CARDS + 1, "the note is drawn");

    step("the menu greys what has nothing to act on", || {
        app.click("file-menu");
        app.wait_for("file-option-exportSelection");
        let tree = app.tree();
        assert!(
            tree.find("file-option-exportSelection")
                .unwrap()
                .is_disabled()
        );
        assert!(tree.find("file-option-copyMarkdown").unwrap().is_disabled());
        assert!(!tree.find("file-option-exportAll").unwrap().is_disabled());
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the menu is closed", |tree| {
            !tree.has("file-option-exportAll")
        });
    });

    step(
        "an export in the clear writes an archive and names it",
        || {
            menu(&mut app, "exportAll");
            app.wait_for("passphrase-prompt");
            app.capture("protect-prompt");
            app.click("passphrase-prompt-plain");
            let said = report(&mut app);
            assert!(said.contains(FILE), "{said}");
            assert!(said.contains(&(CARDS + 1).to_string()), "{said}");
            assert!(is_archive(&file));
        },
    );

    step("importing what is there adds nothing, and says so", || {
        menu(&mut app, "import");
        assert!(!report(&mut app).is_empty());
        cards(&mut app, CARDS + 1, "nothing came in");
    });

    step("a note gone for good comes back from the file", || {
        purge_first(&mut app);
        cards(&mut app, CARDS, "the note is gone");
        menu(&mut app, "import");
        report(&mut app);
        cards(&mut app, CARDS + 1, "the note is back");
    });

    sealed_file(&mut app, &file);

    step(
        "a file from a newer version comes in rather than failing",
        || {
            std::fs::write(&file, NEWER).unwrap();
            menu(&mut app, "import");
            report(&mut app);
            cards(&mut app, CARDS + 2, "the note from the future is drawn");
        },
    );

    step("Escape gives a prompt up, and nothing is written", || {
        std::fs::remove_file(&file).unwrap();
        menu(&mut app, "exportAll");
        app.wait_for("passphrase-prompt");
        app.press(Key::Escape, Modifiers::NONE);
        app.eventually("the prompt closed", |tree| !tree.has("passphrase-prompt"));
        assert!(!file.exists());
    });

    step("an empty space says so before any place is asked", || {
        app.click("space-create-open");
        app.fill("space-create-input", "Rien dedans");
        app.press(Key::Enter, Modifiers::NONE);
        cards(&mut app, 0, "the empty space is shown");
        menu(&mut app, "exportSpace");
        report(&mut app);
        assert!(!app.has("passphrase-prompt"));
        assert!(!file.exists());
    });

    app.quit();
}
