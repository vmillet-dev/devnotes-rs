//! The acceptance sheet's cases on security, the window and the preferences that no other scenario
//! holds. A step opens on the case it runs, so that a run can be read back into the sheet.

use std::path::{Path, PathBuf};

use e2e::{App, PASSPHRASE, Tree, step};
use egui::accesskit::{Node, Toggled};
use egui::{Key, Modifiers};

const NEXT: &str = "a second end-to-end passphrase";
const SECRET: &str = "zq-secret-recette";
const LATER: &str = "zq-apres-copie";
const THEME_LIGHT: &str = "setting-theme-2";
const THEME_DARK: &str = "setting-theme-1";
const COMPACT: &str = "setting-density-1";
const COMFORTABLE: &str = "setting-density-0";
const ENGLISH: &str = "setting-locale-2";

fn preferences(app: &mut App) {
    app.click("file-menu");
    app.click("file-preferences");
    app.wait_for("settings-dialog");
}

fn security(app: &mut App) {
    preferences(app);
    app.click("settings-page-security");
    app.wait_for("setting-change-passphrase");
}

fn close(app: &mut App) {
    app.click("settings-close");
    app.eventually("the panel closed", |tree| !tree.has("settings-dialog"));
}

fn unlock(app: &mut App, phrase: &str) {
    app.fill("vault-passphrase", phrase);
    app.click("vault-submit");
    app.wait_for("area-notes");
    app.eventually("the cards are drawn", |tree| tree.has("note-card"));
}

fn a_note(app: &mut App, title: &str, body: &str) {
    app.click("new-note");
    app.fill("editor-title", title);
    app.fill("editor-body", body);
    app.click("editor-close");
    app.eventually("the editor closed", |tree| !tree.has("editor-title"));
}

fn is_on(node: Option<&Node>) -> bool {
    node.and_then(Node::toggled) == Some(Toggled::True)
}

fn top_of(tree: &Tree) -> f64 {
    tree.find("area-option-notes")
        .and_then(Node::bounds)
        .map_or(f64::INFINITY, |bounds| bounds.y0)
}

fn switch_top(app: &mut App) -> f64 {
    top_of(&app.tree())
}

fn files_under(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files_under(&path, found);
        } else {
            found.push(path);
        }
    }
}

fn the_library(profile: &Path) -> PathBuf {
    let mut libraries = std::fs::read_dir(profile.join("libraries")).unwrap();
    libraries.next().unwrap().unwrap().path()
}

#[test]
fn recette_security() {
    let directory = tempfile::tempdir().unwrap();
    let profile = directory.path();
    let mut app = App::launch("37_recette_security", profile);
    app.create_library();
    app.eventually("the samples are drawn", |tree| tree.has("note-card"));
    let mut app = app.relaunch();
    unlock(&mut app, PASSPHRASE);

    step("SEC-01 the page holds the phrase and the copies", || {
        security(&mut app);
        assert!(is_on(app.tree().find("setting-automatic-backups")));
        app.capture("security");
    });

    step("SEC-02 a copy of the launch, offered back", || {
        app.scroll_to("settings-content", "backup-row");
        assert!(app.has("backup-restore"));
        close(&mut app);
    });

    let mut app = app.relaunch();
    unlock(&mut app, PASSPHRASE);

    step("SEC-03 a second launch of the day takes no copy", || {
        security(&mut app);
        app.scroll_to("settings-content", "backup-row");
        assert_eq!(app.tree().count("backup-row"), 1);
    });

    step("SEC-05 a new phrase too short changes nothing", || {
        app.scroll_top("settings-content");
        app.click("setting-change-passphrase");
        app.fill("change-passphrase-current", PASSPHRASE);
        app.fill("change-passphrase-next", "court-08");
        app.fill("change-passphrase-confirmation", "court-08");
        app.wait_for("change-passphrase-problem");
        assert!(
            app.tree()
                .find("change-passphrase-submit")
                .is_some_and(Node::is_disabled)
        );
        app.click("change-passphrase-cancel");
        app.eventually("the dialog closed", |tree| !tree.has("change-passphrase"));
    });

    step("SEC-06 the session carries on over a new phrase", || {
        app.click("setting-change-passphrase");
        app.wait_for("change-passphrase-copies");
        app.capture("change-passphrase");
        app.fill("change-passphrase-current", PASSPHRASE);
        app.fill("change-passphrase-next", NEXT);
        app.fill("change-passphrase-confirmation", NEXT);
        app.click("change-passphrase-submit");
        app.eventually("the dialog closed", |tree| !tree.has("change-passphrase"));
        app.wait_for("status-toast");
        close(&mut app);
        assert!(app.has("note-card"), "no phrase is asked again");
    });

    restoring(&mut app, profile);
    app.quit();
}

fn restoring(app: &mut App, profile: &Path) {
    step(
        "SEC-08 the restore says what leaves, and Cancel keeps it",
        || {
            a_note(app, LATER, "echo after");
            security(app);
            app.scroll_to("settings-content", "backup-row");
            app.click("backup-restore");
            app.wait_for("backup-confirm");
            app.capture("restore-confirm");
            app.click("backup-cancel");
            app.eventually("the strip is gone", |tree| !tree.has("backup-confirm"));
            close(app);
            app.eventually("the note stays", |tree| {
                tree.texts("note-card-title").iter().any(|it| it == LATER)
            });
        },
    );

    step(
        "SEC-09 a restore sets the library aside in replaced",
        || {
            security(app);
            app.scroll_to("settings-content", "backup-row");
            app.click("backup-restore");
            app.scroll_above(
                "settings-content",
                "backup-confirm-restore",
                "settings-cancel",
            );
            app.click("backup-confirm-restore");
            app.wait_for("vault-passphrase");
            app.wait_for("status-toast");
            app.capture("restored");
            let replaced = the_library(profile).join("replaced");
            assert!(std::fs::read_dir(replaced).unwrap().next().is_some());
        },
    );

    step(
        "SEC-10 a copy older than the change opens with the new phrase",
        || {
            app.fill("vault-passphrase", PASSPHRASE);
            app.click("vault-submit");
            app.eventually("the old phrase is refused", |tree| {
                tree.text("vault-problem")
                    .is_some_and(|said| !said.is_empty())
            });
            unlock(app, NEXT);
        },
    );

    step("SEC-09 what was written after the copy is gone", || {
        let titles = app.tree().texts("note-card-title");
        assert!(!titles.contains(&LATER.to_owned()), "{titles:?}");
    });
}

#[test]
fn recette_nothing_readable_on_the_disk() {
    let directory = tempfile::tempdir().unwrap();
    let profile = directory.path();
    let mut app = App::launch("37_recette_disk", profile);
    app.create_library();
    a_note(&mut app, SECRET, SECRET);
    app.quit();

    step(
        "SEC-12 neither title nor body can be read on the disk",
        || {
            let mut files = Vec::new();
            files_under(&profile.join("libraries"), &mut files);
            assert!(!files.is_empty());
            for file in files {
                let bytes = std::fs::read(&file).unwrap();
                let found = bytes
                    .windows(SECRET.len())
                    .any(|window| window == SECRET.as_bytes());
                assert!(!found, "{} reads in the clear", file.display());
            }
        },
    );
}

#[test]
fn recette_window() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("37_recette_window", profile.path());
    app.create_library();
    app.eventually("the samples are drawn", |tree| tree.has("note-card"));

    step("APP-01 each area is found as it was left", || {
        app.click("note-card");
        app.wait_for("editor-title");
        app.press(Key::Num2, Modifiers::COMMAND);
        app.wait_for("area-tools");
        app.click("tools-entry-case");
        app.wait_for("tool-title");
        app.press(Key::Num3, Modifiers::COMMAND);
        app.wait_for("http-rail-title");
        app.click("http-tab-new");
        app.wait_for("http-tab-open");
        app.press(Key::Num1, Modifiers::COMMAND);
        app.wait_for("area-notes");
        app.wait_for("editor-title");
        app.press(Key::Num2, Modifiers::COMMAND);
        app.wait_for("tool-title");
        app.press(Key::Num3, Modifiers::COMMAND);
        app.wait_for("http-tab-open");
        app.press(Key::Num1, Modifiers::COMMAND);
        app.wait_for("editor-title");
        app.click("editor-close");
        app.eventually("the editor closed", |tree| !tree.has("editor-title"));
    });

    step("APP-02 one state of the rail for the three areas", || {
        let in_the_rail = switch_top(&mut app);
        app.press(Key::B, Modifiers::COMMAND);
        app.eventually("the bar took the switch", |tree| top_of(tree) < in_the_rail);
        app.press(Key::Num2, Modifiers::COMMAND);
        app.wait_for("area-tools");
        assert!(switch_top(&mut app) < in_the_rail);
        app.press(Key::Num3, Modifiers::COMMAND);
        app.wait_for("http-tab-open");
        assert!(switch_top(&mut app) < in_the_rail);
        app.press(Key::B, Modifiers::COMMAND);
        app.eventually("the rail took it back", |tree| top_of(tree) >= in_the_rail);
        app.press(Key::Num1, Modifiers::COMMAND);
        app.wait_for("area-notes");
        assert!(switch_top(&mut app) >= in_the_rail);
    });

    step("APP-07 the bar's theme switches in one click", || {
        let word = app.tree().text("theme-toggle").unwrap_or_default();
        app.click("theme-toggle");
        app.eventually("the word changed", |tree| {
            tree.text("theme-toggle").is_some_and(|now| now != word)
        });
    });

    let word = app.tree().text("theme-toggle").unwrap_or_default();
    let mut app = app.relaunch_discarding();
    unlock(&mut app, PASSPHRASE);

    step("APP-07 the theme survives a relaunch", || {
        assert_eq!(app.tree().text("theme-toggle").unwrap_or_default(), word);
    });

    step(
        "APP-13 quitting takes two clicks, a click elsewhere calls it off",
        || {
            app.click("file-menu");
            let word = app.tree().text("file-quit").unwrap_or_default();
            app.click("file-quit");
            app.eventually("the entry asks", |tree| {
                tree.text("file-quit").is_some_and(|now| now != word)
            });
            app.click("file-menu");
            app.eventually("the menu closed", |tree| !tree.has("file-quit"));
            app.click("file-menu");
            assert_eq!(app.tree().text("file-quit").unwrap_or_default(), word);
            app.click("file-quit");
            app.click("file-quit");
        },
    );

    app.ended();
}

#[test]
fn recette_preferences() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("37_recette_preferences", profile.path());
    app.create_library();

    step("PRF-01 the panel: its pages and its footer", || {
        preferences(&mut app);
        for id in [
            "settings-page-general",
            "settings-page-variables",
            "settings-page-shortcuts",
            "settings-page-security",
            "settings-cancel",
            "settings-apply",
            "settings-close",
        ] {
            assert!(app.has(id), "{id}");
        }
        app.capture("preferences");
    });

    step(
        "PRF-02 theme and density preview, Cancel takes both back",
        || {
            app.click(THEME_DARK);
            app.click(COMFORTABLE);
            app.click("settings-apply");
            let word = app.tree().text("theme-toggle").unwrap_or_default();
            let before = std::fs::read(app.capture("density-before")).unwrap();
            app.click(THEME_LIGHT);
            app.click(COMPACT);
            app.eventually("the theme previews", |tree| {
                tree.text("theme-toggle").is_some_and(|now| now != word)
            });
            assert_ne!(before, std::fs::read(app.capture("density-after")).unwrap());
            app.click("settings-cancel");
            app.eventually("the panel closed", |tree| !tree.has("settings-dialog"));
            assert_eq!(app.tree().text("theme-toggle").unwrap_or_default(), word);
            preferences(&mut app);
            assert!(is_on(app.tree().find(THEME_DARK)));
            assert!(is_on(app.tree().find(COMFORTABLE)));
        },
    );

    step(
        "PRF-05 three changes waiting are counted, each way out does as it says",
        || {
            app.click(THEME_LIGHT);
            app.click(COMPACT);
            app.click("setting-locale");
            app.click(ENGLISH);
            app.press(Key::Escape, Modifiers::NONE);
            app.eventually("the footer counts them", |tree| {
                tree.text("settings-unapplied")
                    .is_some_and(|said| said.contains('3'))
            });
            app.capture("unapplied");
            app.click("settings-discard");
            app.eventually("the panel closed", |tree| !tree.has("settings-dialog"));
            preferences(&mut app);
            assert!(is_on(app.tree().find(THEME_DARK)));

            app.click(THEME_LIGHT);
            app.press(Key::Escape, Modifiers::NONE);
            app.click("settings-apply-close");
            app.eventually("the panel closed", |tree| !tree.has("settings-dialog"));
            preferences(&mut app);
            assert!(is_on(app.tree().find(THEME_LIGHT)));
            app.click(THEME_DARK);
            close(&mut app);
        },
    );

    step("PRF-15 a canvas key refuses the navigation's keys", || {
        preferences(&mut app);
        app.click("settings-page-shortcuts");
        app.scroll_above(
            "settings-content",
            "shortcut-field-canvas.pin",
            "settings-cancel",
        );
        app.click("shortcut-field-canvas.pin");
        assert!(app.has("settings-dialog"));
        app.press(Key::ArrowUp, Modifiers::NONE);
        app.wait_for("shortcut-error-canvas.pin");
        app.capture("reserved");
        app.press(Key::Escape, Modifiers::NONE);
        assert!(app.has("settings-dialog"));
    });

    app.quit();
}
