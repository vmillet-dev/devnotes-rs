//! Closing, minimising and quitting: what becomes of the window, tray or no tray.

use egui::{ViewportCommand as Command, ViewportEvent, ViewportId, ViewportInfo};
use egui_kittest::Harness;
use platform::tray;

use super::DevNotesApp;
use super::tests::{commands, startup, with_a_tray};
use crate::action::{Action, GlobalAction};
use crate::inputs::Input;

fn press_the_close_button(harness: &mut Harness<'_, DevNotesApp>) {
    viewport(harness).events.push(ViewportEvent::Close);
    harness.step();
}

fn viewport<'a>(harness: &'a mut Harness<'_, DevNotesApp>) -> &'a mut ViewportInfo {
    harness
        .input_mut()
        .viewports
        .entry(ViewportId::ROOT)
        .or_default()
}

fn hidden_and_held(commands: &[Command]) -> bool {
    commands.contains(&Command::Visible(false)) && commands.contains(&Command::CancelClose)
}

#[test]
fn with_a_tray_the_close_button_hides_the_window() {
    let directory = tempfile::tempdir().unwrap();
    let (mut harness, _, _) = with_a_tray(directory.path());

    press_the_close_button(&mut harness);

    assert!(hidden_and_held(&commands(&harness)));
}

#[test]
fn with_the_preference_off_the_close_button_ends_the_application() {
    let directory = tempfile::tempdir().unwrap();
    let (mut harness, _, _) = with_a_tray(directory.path());
    harness.state_mut().state.settings.close_to_tray = false;

    press_the_close_button(&mut harness);

    let commands = commands(&harness);
    assert!(commands.contains(&Command::Visible(false)));
    assert!(!commands.contains(&Command::CancelClose));
}

#[test]
fn without_a_tray_the_close_button_ends_the_application() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness =
        Harness::new_eframe(|cc| DevNotesApp::new(cc, startup(directory.path())).unwrap());

    press_the_close_button(&mut harness);

    assert!(!commands(&harness).contains(&Command::CancelClose));
}

#[test]
fn quit_ends_the_application_even_with_a_tray() {
    let directory = tempfile::tempdir().unwrap();
    let (mut harness, _, posted) = with_a_tray(directory.path());

    posted.post(Input::Tray(tray::Asked::Quit));
    harness.step();
    press_the_close_button(&mut harness);

    assert!(!commands(&harness).contains(&Command::CancelClose));
}

#[test]
fn a_quit_cancelled_at_the_question_leaves_the_close_button_hiding() {
    let directory = tempfile::tempdir().unwrap();
    let (mut harness, _, _) = with_a_tray(directory.path());
    let ctx = harness.ctx.clone();
    let app = harness.state_mut();
    let action = Action::Global(GlobalAction::Quit);
    app.state.apply(action, &ctx, &app.backend);

    let answer = crate::http::Action::Leave(crate::state::http::leave::Asked::Cancel);
    app.state.apply(Action::Http(answer), &ctx, &app.backend);
    press_the_close_button(&mut harness);

    assert!(hidden_and_held(&commands(&harness)));
}

#[test]
fn minimised_with_the_preference_on_the_window_goes_to_the_tray_once() {
    let directory = tempfile::tempdir().unwrap();
    let (mut harness, _, _) = with_a_tray(directory.path());
    harness.state_mut().state.settings.minimize_to_tray = true;

    viewport(&mut harness).minimized = Some(true);
    harness.step();
    assert!(commands(&harness).contains(&Command::Visible(false)));

    // Still minimised for a frame while it comes back: it must not go away again.
    viewport(&mut harness).minimized = Some(true);
    harness.step();
    assert!(!commands(&harness).contains(&Command::Visible(false)));
}

#[test]
fn minimised_with_the_preference_off_the_window_stays_in_the_taskbar() {
    let directory = tempfile::tempdir().unwrap();
    let (mut harness, _, _) = with_a_tray(directory.path());

    viewport(&mut harness).minimized = Some(true);
    harness.step();

    assert!(!commands(&harness).contains(&Command::Visible(false)));
}

#[test]
fn a_full_screen_is_not_the_size_to_reopen_at() {
    let directory = tempfile::tempdir().unwrap();
    let (mut harness, _, _) = with_a_tray(directory.path());
    let rect = |width| egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 600.0));
    let placed = ViewportInfo {
        outer_rect: Some(rect(800.0)),
        inner_rect: Some(rect(800.0)),
        ..Default::default()
    };
    let app = harness.state_mut();

    app.remember(&placed);
    app.remember(&ViewportInfo {
        fullscreen: Some(true),
        inner_rect: Some(rect(1_920.0)),
        ..placed
    });

    assert!((app.geometry.unwrap().width - 800.0).abs() < f32::EPSILON);
}

#[test]
fn a_window_that_names_no_place_keeps_its_size() {
    let directory = tempfile::tempdir().unwrap();
    let (mut harness, _, _) = with_a_tray(directory.path());
    let app = harness.state_mut();

    app.remember(&ViewportInfo {
        inner_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    });

    assert!((app.geometry.unwrap().width - 800.0).abs() < f32::EPSILON);
}

#[test]
fn a_whole_index_asks_the_lists_again() {
    let directory = tempfile::tempdir().unwrap();
    let (mut harness, _tray, inbox) = super::tests::with_a_tray(directory.path());
    let before = harness.state().state.revision;

    engine::EventSink::emit(&inbox, engine::EngineEvent::IndexReady);
    harness.step();

    assert_ne!(harness.state().state.revision, before);
}
