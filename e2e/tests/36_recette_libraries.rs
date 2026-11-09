//! The acceptance sheet's cases on libraries that no other scenario holds. A step opens on the
//! case it runs, so that a run can be read back into the sheet.

use std::path::PathBuf;

use e2e::{App, PASSPHRASE, step};
use egui::{Key, Modifiers};

const OTHER: &str = "Recette 2";
const RENAMED: &str = "Recette Deux";
const ONLY_HERE: &str = "zq-isolation";
const TOO_SHORT: &str = "abc";

fn panel(app: &mut App) {
    app.click("file-menu");
    app.click("file-libraries");
    app.wait_for("library-row");
}

fn pass_the_gate(app: &mut App) {
    app.fill("vault-passphrase", PASSPHRASE);
    if app.has("vault-confirmation") {
        app.fill("vault-confirmation", PASSPHRASE);
    }
    app.click("vault-submit");
    app.wait_for("area-notes");
}

/// The key file of the only library of `profile`.
fn key_file(profile: &std::path::Path) -> PathBuf {
    let libraries = profile.join("libraries");
    let library = std::fs::read_dir(libraries)
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
    library.path().join("vault.json")
}

#[test]
fn recette_libraries() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("36_recette_libraries", profile.path());

    step(
        "LIB-01 the gate says the floor before anything is typed",
        || {
            app.wait_for("vault-minimum");
            assert!(app.has("vault-confirmation"));
        },
    );

    step("LIB-03 two different phrases are said to differ", || {
        app.fill("vault-passphrase", PASSPHRASE);
        app.fill("vault-confirmation", "correct horse battery stapler");
        app.wait_for("vault-problem");
        assert!(
            app.tree()
                .find("vault-submit")
                .is_some_and(egui::accesskit::Node::is_disabled)
        );
    });

    // A field that reveals keeps its role and no value: the picture is what says it.
    step("LIB-04 show and hide take both fields together", || {
        let hidden = std::fs::read(app.capture("phrase-hidden")).unwrap();
        app.click("vault-reveal");
        let shown = std::fs::read(app.capture("phrase-shown")).unwrap();
        assert_ne!(hidden, shown);
        app.click("vault-reveal");
    });

    step("LIB-05 the right phrase twice opens the canvas", || {
        app.click("vault-confirmation");
        app.press(Key::Backspace, Modifiers::NONE);
        app.click("vault-submit");
        app.wait_for("area-notes");
    });

    step("LIB-14 a note of one library is not in the other", || {
        app.click("new-note");
        app.fill("editor-title", ONLY_HERE);
        app.click("editor-close");
        app.eventually("the note is a card", |tree| {
            tree.texts("note-card").iter().any(|card| card == ONLY_HERE)
        });
        panel(&mut app);
        app.fill("library-new-name", OTHER);
        app.click("library-create");
        app.wait_for("vault-confirmation");
        pass_the_gate(&mut app);
        app.eventually("its samples are drawn", |tree| tree.has("note-card"));
        app.fill("search-input", ONLY_HERE);
        app.wait_for("search-matched");
        app.eventually("nothing is found", |tree| !tree.has("note-card"));
    });

    step("LIB-15 a library is renamed from the panel", || {
        app.click("search-clear");
        panel(&mut app);
        app.click("library-start-rename");
        app.click("library-rename");
        app.press(Key::A, Modifiers::COMMAND);
        app.type_text(RENAMED);
        app.press(Key::Enter, Modifiers::NONE);
        let registry = profile.path().join("libraries.json");
        app.eventually("the registry names it", |_| {
            std::fs::read_to_string(&registry).is_ok_and(|text| text.contains(RENAMED))
        });
        app.click("libraries-close");
    });

    let mut app = app.relaunch();

    step(
        "LIB-08 no length rule at the unlock, only a refusal",
        || {
            app.fill("vault-passphrase", TOO_SHORT);
            app.click("vault-submit");
            app.wait_for("vault-problem");
            assert!(!app.has("vault-minimum"));
        },
    );
    app.quit();
}

#[test]
fn recette_a_lost_key_file() {
    let profile = tempfile::tempdir().unwrap();
    let mut app = App::launch("36_recette_key_file", profile.path());
    app.create_library();
    app.quit();
    let key = key_file(profile.path());
    let kept = key.with_extension("json.bak");
    std::fs::rename(&key, &kept).unwrap();

    let mut app = App::launch("36_recette_key_file", profile.path());
    step("LIB-20 a library without its key file says so", || {
        app.wait_for("vault-key-missing");
        assert!(!app.has("vault-passphrase"));
    });
    app.quit();

    std::fs::rename(&kept, &key).unwrap();
    let mut app = App::launch("36_recette_key_file", profile.path());
    step(
        "LIB-20 the file put back, the phrase opens it again",
        || {
            pass_the_gate(&mut app);
        },
    );
    app.quit();
}
