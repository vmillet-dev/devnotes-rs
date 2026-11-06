//! A preference applies on a button; only the theme and the density show while chosen. The
//! language is established here, never assumed: `system` is the machine's.

use e2e::{App, step};
use egui::{Key, Modifiers};

const THEME_LIGHT: &str = "setting-theme-2";
const THEME_DARK: &str = "setting-theme-1";
const COMPACT: &str = "setting-density-1";
const COMFORTABLE: &str = "setting-density-0";
const FRENCH: &str = "setting-locale-1";
const ENGLISH: &str = "setting-locale-2";
const NOTE_KIND_NOTE: &str = "setting-note-kind-0";
const NOTE_KIND_SNIPPET: &str = "setting-note-kind-1";

fn open(app: &mut App) {
    app.click("file-menu");
    app.click("file-preferences");
    app.wait_for("settings-dialog");
}

fn closed(app: &mut App) {
    app.eventually("the panel is closed", |tree| !tree.has("settings-dialog"));
}

fn locale(app: &mut App, choice: &str) {
    app.click("setting-locale");
    app.click(choice);
}

/// What the theme control offers, which names the theme it would switch to: it reads the window's.
fn theme_word(app: &mut App) -> String {
    app.tree().text("theme-toggle").unwrap_or_default()
}

fn file_word(app: &mut App) -> String {
    app.tree().text("file-menu").unwrap_or_default()
}

#[test]
fn preferences() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("12_preferences", profile.path());
    app.create_library();

    step(
        "the theme shows as it is chosen, and Cancel takes it back",
        || {
            open(&mut app);
            app.capture("preferences");
            app.click(THEME_DARK);
            app.click("settings-apply");
            let dark = theme_word(&mut app);

            app.click(THEME_LIGHT);
            app.eventually("the window previews the light theme", |tree| {
                tree.text("theme-toggle").is_some_and(|word| word != dark)
            });
            app.click("settings-cancel");
            closed(&mut app);
            app.eventually("the theme is back", |tree| {
                tree.text("theme-toggle").as_deref() == Some(dark.as_str())
            });
        },
    );

    step("the language waits for the button", || {
        open(&mut app);
        locale(&mut app, FRENCH);
        app.click("settings-apply");
        app.eventually("the window speaks French", |tree| {
            tree.text("file-menu").as_deref() == Some("Fichier")
        });

        locale(&mut app, ENGLISH);
        assert_eq!(file_word(&mut app), "Fichier", "nothing before the button");
        app.click("settings-apply");
        app.eventually("the window speaks English", |tree| {
            tree.text("file-menu").as_deref() == Some("File")
        });
        app.click("settings-close");
        closed(&mut app);
    });

    step("Escape says what is waiting instead of closing", || {
        open(&mut app);
        app.click(COMPACT);
        app.press(Key::Escape, Modifiers::NONE);
        app.wait_for("settings-unapplied");
        assert!(app.has("settings-dialog"));
        app.click("settings-discard");
        closed(&mut app);

        open(&mut app);
        app.click(COMPACT);
        app.press(Key::Escape, Modifiers::NONE);
        app.click("settings-apply-close");
        closed(&mut app);
    });

    step("Escape with nothing waiting just closes", || {
        open(&mut app);
        app.press(Key::Escape, Modifiers::NONE);
        closed(&mut app);
    });

    step("« New note » opens the kind the preference says", || {
        open(&mut app);
        app.click(NOTE_KIND_NOTE);
        app.click("settings-close");
        closed(&mut app);
        app.click("new-note");
        app.wait_for("editor-rich");
        assert!(!app.has("editor-body"));
        app.click("editor-close");
        app.eventually("the editor closed", |tree| !tree.has("editor-rich"));

        open(&mut app);
        app.click(NOTE_KIND_SNIPPET);
        app.click("settings-close");
        closed(&mut app);
        app.click("new-note");
        app.wait_for("editor-body");
        app.click("editor-close");
        app.eventually("the editor closed", |tree| !tree.has("editor-body"));
    });

    let theme = theme_word(&mut app);
    let mut app = app.relaunch();
    app.fill("vault-passphrase", e2e::PASSPHRASE);
    app.click("vault-submit");
    app.wait_for("area-notes");

    step(
        "what was applied is what the next launch opens with",
        || {
            assert_eq!(file_word(&mut app), "File");
            assert_eq!(theme_word(&mut app), theme);
            open(&mut app);
            app.click(COMFORTABLE);
            app.click("settings-cancel");
            closed(&mut app);
        },
    );

    app.quit();
}
