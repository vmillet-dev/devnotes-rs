//! A blank profile: the gate creates the library, then a real relaunch unlocks it.

use e2e::{App, PASSPHRASE, step};

#[test]
fn first_launch() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("01_first_launch", profile.path());
    // Typed in two goes: the start alone is under the floor of twelve characters.
    let (start, rest) = PASSPHRASE.split_at(7);

    step(
        "a blank profile opens on the gate, asking the phrase twice",
        || {
            app.wait_for("vault-confirmation");
            assert!(!app.has("area-switch"));
            app.capture("gate-create");
        },
    );

    step("a phrase under twelve characters cannot be chosen", || {
        app.fill("vault-passphrase", start);
        app.fill("vault-confirmation", start);
        app.eventually("the submit is disabled", |tree| {
            tree.find("vault-submit")
                .is_some_and(egui::accesskit::Node::is_disabled)
        });
    });

    step(
        "a long enough phrase, twice, creates the library and opens it",
        || {
            app.fill("vault-passphrase", rest);
            app.fill("vault-confirmation", rest);
            app.click("vault-submit");
            app.wait_for("area-notes");
            assert!(app.has("area-switch"));
        },
    );

    let mut app = app.relaunch();

    step("a relaunch asks for the phrase once", || {
        app.wait_for("vault-passphrase");
        assert!(!app.has("vault-confirmation"));
    });

    step("a wrong phrase is refused and says so", || {
        app.fill("vault-passphrase", "not the phrase");
        app.click("vault-submit");
        app.eventually("the problem line speaks", |tree| {
            tree.text("vault-problem")
                .is_some_and(|said| !said.is_empty())
        });
        assert!(!app.has("area-notes"));
        app.capture("gate-wrong");
    });

    step("the phrase chosen opens the library", || {
        app.fill("vault-passphrase", PASSPHRASE);
        app.click("vault-submit");
        app.wait_for("area-notes");
    });

    app.quit();
}
