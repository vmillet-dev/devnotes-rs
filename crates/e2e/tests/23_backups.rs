//! The copies, and the one gesture that puts one back. A copy is taken at unlock, so a real
//! relaunch comes first; the last step really replaces the library, and unlocks the copy.

use e2e::{App, PASSPHRASE, step};
use egui::accesskit::Node;

const LATER: &str = "Written after the copy";

fn security(app: &mut App) {
    app.click("file-menu");
    app.click("file-preferences");
    app.click("settings-page-security");
    app.scroll_to("settings-content", "backup-row");
}

/// The strip opens further down a panel that scrolls: its buttons are clicked once they are in view.
fn strip_shown(app: &mut App) {
    app.eventually("the strip is in view", |tree| {
        let place = |id| tree.find(id).and_then(Node::bounds);
        place("backup-confirm-restore")
            .zip(place("settings-cancel"))
            .is_some_and(|(button, footer)| button.y1 < footer.y0)
    });
}

fn close(app: &mut App) {
    app.click("settings-close");
    app.eventually("the panel closed", |tree| !tree.has("settings-dialog"));
}

fn unlock(app: &mut App) {
    app.fill("vault-passphrase", PASSPHRASE);
    app.click("vault-submit");
    app.wait_for("area-notes");
}

fn is_stamped(words: &str) -> bool {
    words
        .split_whitespace()
        .next()
        .is_some_and(|stamp| stamp.len() == 19 && stamp.as_bytes()[10] == b'_')
}

#[test]
fn backups() {
    let profile = tempfile::tempdir().unwrap();
    let app = {
        let mut app = App::launch("23_backups", profile.path());
        app.create_library();
        app.eventually("the samples are drawn", |tree| tree.has("note-card"));
        app
    };
    let mut app = app.relaunch();
    unlock(&mut app);

    step("the launch's copy is listed, with when and how big", || {
        security(&mut app);
        let detail = app.tree().text("backup-detail").unwrap_or_default();
        assert!(is_stamped(&detail), "{detail}");
        app.capture("backups");
        close(&mut app);
    });

    step(
        "the trigger names what a restore would do, and runs none",
        || {
            security(&mut app);
            app.click("backup-restore");
            app.wait_for("backup-confirm");
            strip_shown(&mut app);
            assert!(!app.has("backup-restore"), "the trigger gave way");
            assert!(app.has("settings-dialog"));
            app.click("backup-cancel");
            app.eventually("the strip is gone", |tree| !tree.has("backup-confirm"));
            close(&mut app);
        },
    );

    step("it replaces the library and goes back to the gate", || {
        app.click("new-note");
        app.fill("editor-title", LATER);
        app.fill("editor-body", "echo later");
        app.click("editor-close");
        app.eventually("the note is a card", |tree| !tree.has("editor-title"));

        security(&mut app);
        app.click("backup-restore");
        strip_shown(&mut app);
        app.click("backup-confirm-restore");
        app.wait_for("vault-passphrase");
        assert!(!app.has("file-menu"));
        assert!(!app.has("settings-dialog"));
        app.wait_for("status-toast");
    });

    step("the copy opens, without what was written after it", || {
        unlock(&mut app);
        app.eventually("the samples are drawn", |tree| tree.has("note-card"));
        let titles = app.tree().texts("note-card-title");
        assert!(!titles.contains(&LATER.to_owned()), "{titles:?}");
    });

    app.quit();
}
