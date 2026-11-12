//! What the keys of the whole machine do once the application hears them.

use std::sync::Arc;

use egui_kittest::Harness;
use engine::notes::kind::NoteKind;
use platform::shortcuts::Global;

use super::DevNotesApp;
use super::tests::{click, commands, open_library, present, settle};
use crate::state::Area;

struct Copied(&'static str);

impl platform::clipboard::Clipboard for Copied {
    fn html(&self) -> Option<String> {
        None
    }

    fn text(&self) -> Option<String> {
        Some(self.0.to_owned())
    }
}

fn press(harness: &mut Harness<'_, DevNotesApp>, key: Global) {
    let ctx = harness.ctx.clone();
    harness.state_mut().asked_by_a_key(key, &ctx);
    settle(harness);
}

fn capture(harness: &mut Harness<'_, DevNotesApp>, copied: &'static str) {
    harness.state_mut().state.clipboard = Arc::new(Copied(copied));
    press(harness, Global::Capture);
}

fn opened_kind(harness: &Harness<'_, DevNotesApp>) -> Option<NoteKind> {
    let opened = harness.state().state().editor.opened()?;
    Some(opened.note()?.note.kind)
}

#[test]
fn the_new_note_key_opens_a_note_from_any_area() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    click(&mut harness, "area-option-tools");

    press(&mut harness, Global::NewNote);

    assert_eq!(harness.state().state().settings.area, Area::Notes);
    assert!(harness.state().state().editor.is_open());
}

#[test]
fn the_palette_key_opens_it_and_brings_the_window_up() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());
    let ctx = harness.ctx.clone();

    harness.state_mut().asked_by_a_key(Global::Palette, &ctx);
    harness.step();

    assert!(commands(&harness).contains(&egui::ViewportCommand::Focus));
    assert!(harness.state().state().palette.is_open());
}

#[test]
fn code_captured_opens_as_a_snippet_and_is_kept() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    capture(&mut harness, "fn main() {\n    println!(\"hello\");\n}\n");

    assert_eq!(opened_kind(&harness), Some(NoteKind::Snippet));
    let id = harness.state().state().editor.opened().unwrap().id();
    assert!(id.is_some(), "written at once");
}

#[test]
fn prose_captured_opens_as_a_note() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    capture(&mut harness, "A sentence to keep for later.");

    assert_eq!(opened_kind(&harness), Some(NoteKind::Note));
}

#[test]
fn an_empty_clipboard_captures_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open_library(directory.path());

    capture(&mut harness, "   \n");

    assert!(!harness.state().state().editor.is_open());
}

#[test]
fn before_a_library_is_open_nothing_is_captured() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = Harness::new_eframe(|cc| {
        DevNotesApp::new(cc, super::tests::startup(directory.path())).unwrap()
    });

    capture(&mut harness, "A sentence.");

    assert!(!harness.state().state().editor.is_open());
    assert!(present(&harness, "vault-passphrase"));
}
