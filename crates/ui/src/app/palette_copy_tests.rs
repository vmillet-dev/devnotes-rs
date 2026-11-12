//! A copy from the palette: over another application the window goes back and a toast says it.

use std::sync::{Arc, Mutex};

use egui::ViewportCommand as Command;
use egui_kittest::Harness;

use super::DevNotesApp;
use super::tests::{commands, open_library, settle};

#[derive(Default)]
struct Toasts(Mutex<Vec<(String, String)>>);

impl platform::notify::Notifier for Toasts {
    fn notify(&self, title: &str, body: &str) {
        let said = (title.to_owned(), body.to_owned());
        self.0.lock().unwrap().push(said);
    }
}

fn copied_from_the_palette(
    harness: &mut Harness<'_, DevNotesApp>,
    from_elsewhere: bool,
) -> Arc<Toasts> {
    let toasts = Arc::new(Toasts::default());
    let ctx = harness.ctx.clone();
    let app = harness.state_mut();
    app.state.notifier = toasts.clone();
    app.state.palette.open(&app.backend, from_elsewhere);
    app.state.palette.copying();
    let said = i18n::Message::new(i18n::keys::settings::COPIED);
    app.state
        .copy_out(&ctx, &app.backend, "echo copied".to_owned(), said);
    harness.step();
    toasts
}

#[test]
fn over_another_application_the_window_goes_and_a_toast_says_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    let toasts = copied_from_the_palette(&mut harness, true);

    assert!(commands(&harness).contains(&Command::Visible(false)));
    settle(&mut harness);
    assert_eq!(toasts.0.lock().unwrap().len(), 1);
    let state = harness.state().state();
    assert!(!state.palette.is_open());
    assert!(state.status.latest().is_none(), "the window has gone");
}

#[test]
fn with_devnotes_in_front_it_only_closes_and_says_so_in_the_window() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    let toasts = copied_from_the_palette(&mut harness, false);

    assert!(!commands(&harness).contains(&Command::Visible(false)));
    settle(&mut harness);
    assert!(toasts.0.lock().unwrap().is_empty());
    assert!(harness.state().state().status.latest().is_some());
}

#[test]
fn with_the_confirmation_off_the_window_goes_without_a_word() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    harness.state_mut().state.settings.copy_confirmation = false;

    let toasts = copied_from_the_palette(&mut harness, true);

    assert!(commands(&harness).contains(&Command::Visible(false)));
    settle(&mut harness);
    assert!(toasts.0.lock().unwrap().is_empty());
}
