//! What it proves is the shape of the change: the key file is rewritten, the notes are not, and
//! the session carries on. Read from the file rather than from what the application says of it.

use std::path::{Path, PathBuf};

use e2e::{App, PASSPHRASE, step};

const NEXT: &str = "a second end-to-end passphrase";

/// The one key file of a profile, wherever its library lives.
fn key_file(directory: &Path) -> Option<PathBuf> {
    for entry in std::fs::read_dir(directory).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.ends_with("backups") {
                continue;
            }
            if let Some(found) = key_file(&path) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|name| name == "vault.json") {
            return Some(path);
        }
    }
    None
}

fn security(app: &mut App) {
    app.click("file-menu");
    app.click("file-preferences");
    app.click("settings-page-security");
    app.click("setting-change-passphrase");
    app.wait_for("change-passphrase");
}

fn fill_change(app: &mut App, current: &str) {
    app.fill("change-passphrase-current", current);
    app.fill("change-passphrase-next", NEXT);
    app.fill("change-passphrase-confirmation", NEXT);
    app.click("change-passphrase-submit");
}

#[test]
fn passphrase_change() {
    let directory = tempfile::tempdir().unwrap();
    let profile = directory.path();
    let mut app = App::launch("16_passphrase_change", profile);
    app.create_library();
    app.eventually("the samples are drawn", |tree| tree.has("note-card"));
    let file = key_file(profile).expect("the library has its key file");
    let first = std::fs::read(&file).unwrap();

    step(
        "a change that cannot name the phrase rewrites nothing",
        || {
            security(&mut app);
            fill_change(&mut app, "not the current one");
            app.eventually("the phrase is refused", |tree| {
                tree.text("change-passphrase-problem")
                    .is_some_and(|said| !said.is_empty())
            });
            assert_eq!(std::fs::read(&file).unwrap(), first);
            assert!(app.has("change-passphrase"), "the dialog stays");
        },
    );

    step(
        "the right phrase rewraps the key and leaves the notes",
        || {
            app.eventually("the fields are free again", |tree| {
                tree.find("change-passphrase-current")
                    .is_some_and(|field| !field.is_disabled())
            });
            fill_change(&mut app, PASSPHRASE);
            app.eventually("the dialog closed", |tree| !tree.has("change-passphrase"));
            app.wait_for("status-toast");
            assert_ne!(std::fs::read(&file).unwrap(), first);
            app.click("settings-close");
            app.eventually("the panel closed", |tree| !tree.has("settings-dialog"));
            assert!(app.has("note-card"), "the session carries on");
        },
    );

    let mut app = app.relaunch();
    step(
        "the new phrase opens the library, and the old one no longer",
        || {
            app.fill("vault-passphrase", PASSPHRASE);
            app.click("vault-submit");
            app.eventually("the old phrase is refused", |tree| {
                tree.text("vault-problem")
                    .is_some_and(|said| !said.is_empty())
            });
            app.fill("vault-passphrase", NEXT);
            app.click("vault-submit");
            app.wait_for("area-notes");
        },
    );

    app.quit();
}
