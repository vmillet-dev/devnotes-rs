//! The global shortcuts let go while a shortcut field listens.

use std::str::FromStr;
use std::sync::Mutex;

use devnotes_lib::desktop::{GlobalAction, release_global_shortcuts};
use tauri::Manager;
use tauri::test::mock_app;
use tauri_plugin_global_shortcut::Shortcut;

/// What a key press is matched against is emptied with the registrations: a press arriving
/// meanwhile finds no action. The mock app has no plugin to release, and is not stopped by it.
#[test]
fn released_shortcuts_answer_no_action_until_they_are_taken_back() {
    let app = mock_app();
    app.manage(Mutex::new(vec![(
        Shortcut::from_str("Ctrl+Alt+N").unwrap(),
        GlobalAction::NewNote,
    )]));

    release_global_shortcuts(app.handle().clone());

    let active = app.state::<Mutex<Vec<(Shortcut, GlobalAction)>>>();
    assert!(active.lock().unwrap().is_empty());
}
