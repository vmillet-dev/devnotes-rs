//! The file itself, read from outside the application: one key per setting, each a string, and
//! nothing written before the draft is applied.

use std::path::Path;

use e2e::{App, step};
use serde_json::Value;

const THEME_DARK: &str = "setting-theme-1";
const THEME_LIGHT: &str = "setting-theme-2";

fn stored(profile: &Path) -> serde_json::Map<String, Value> {
    let Ok(text) = std::fs::read_to_string(profile.join("preferences.json")) else {
        return serde_json::Map::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn says(profile: &Path, key: &str) -> Option<String> {
    stored(profile)
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Read again through the window: a write answers frames later, on the core's queue.
fn settled(app: &mut App, profile: &Path, key: &str, want: &str) {
    app.eventually(&format!("{key} reaches the file"), |_| {
        says(profile, key).as_deref() == Some(want)
    });
}

fn open(app: &mut App) {
    app.click("file-menu");
    app.click("file-preferences");
    app.wait_for("settings-dialog");
}

#[test]
fn preferences_on_disk() {
    let directory = tempfile::tempdir().unwrap();
    let profile = directory.path();
    let mut app = App::launch("15_preferences_on_disk", profile);
    app.create_library();

    step("the file is written, one key per setting", || {
        open(&mut app);
        app.click(THEME_DARK);
        app.click("settings-close");
        settled(&mut app, profile, "devnotes.theme", "dark");
    });

    step("the file waits for the draft to be applied", || {
        open(&mut app);
        app.click(THEME_LIGHT);
        // The window has drawn the choice, and with it any write it would have asked for.
        app.wait_for("settings-apply");
        assert_eq!(
            says(profile, "devnotes.theme").as_deref(),
            Some("dark"),
            "a preview is not a write"
        );
        app.click("settings-close");
        settled(&mut app, profile, "devnotes.theme", "light");
    });

    step("the locale the panel applied is stored", || {
        open(&mut app);
        app.click("setting-locale");
        app.click("setting-locale-2");
        app.click("settings-close");
        settled(&mut app, profile, "devnotes.locale", "en");
    });

    step("every value is a string", || {
        let file = stored(profile);
        let others: Vec<&String> = file
            .iter()
            .filter(|(_, value)| !value.is_string())
            .map(|(key, _)| key)
            .collect();
        assert!(others.is_empty(), "{others:?}");
    });

    app.quit();
}
