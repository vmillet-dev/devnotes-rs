//! The way out of a library whose phrase is forgotten: it archives the library and leaves the
//! profile with none, which is what turns the gate into the one that asks for a new phrase.

use e2e::{App, PASSPHRASE, step};

#[test]
fn forgotten_passphrase() {
    let profile = tempfile::tempdir().unwrap();
    let mut first = App::launch("24_forgotten_passphrase", profile.path());
    first.create_library();
    first.eventually("the samples are drawn", |tree| tree.has("note-card"));
    let mut app = first.relaunch();
    app.wait_for("vault-passphrase");

    step("a way out is offered from under the unlock button", || {
        assert!(app.has("vault-forgotten"));
    });

    step("the field gives way to what would be lost", || {
        app.click("vault-forgotten");
        app.wait_for("vault-archive");
        assert!(!app.has("vault-passphrase"));
    });

    step(
        "it goes back to the field without touching anything",
        || {
            app.click("vault-keep-trying");
            app.wait_for("vault-passphrase");
            assert!(!app.has("vault-archive"));
        },
    );

    step("it archives and comes back asking for a new phrase", || {
        app.click("vault-forgotten");
        app.click("vault-archive-confirm");
        app.wait_for("vault-confirmation");
        app.wait_for("status-toast");
    });

    step("a new phrase opens a new library, seeded again", || {
        app.fill("vault-passphrase", PASSPHRASE);
        app.fill("vault-confirmation", PASSPHRASE);
        app.click("vault-submit");
        app.wait_for("area-notes");
        app.eventually("the samples are drawn", |tree| tree.has("note-card"));
    });

    app.quit();
}
