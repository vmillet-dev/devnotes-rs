//! The passphrase prompt of the File menu over the whole window.

use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::DevNotesApp;
use super::tests::{click, open_library, present, settle, type_into};

const PHRASE: &str = "an export passphrase";

fn export(harness: &mut Harness<'_, DevNotesApp>) {
    click(harness, "file-menu");
    click(harness, "file-option-exportAll");
    settle(harness);
}

fn can_submit(harness: &Harness<'_, DevNotesApp>) -> bool {
    let submit = harness.get_by(|node| node.author_id() == Some("passphrase-prompt-submit"));
    !submit.accesskit_node().is_disabled()
}

fn asking(directory: &std::path::Path) -> Harness<'static, DevNotesApp> {
    let mut harness = open_library(directory);
    let file = directory.join("library.devnotes");
    harness.state_mut().state.dialog = Arc::new(platform::dialog::Fixed(file));
    export(&mut harness);
    type_into(&mut harness, "passphrase-prompt-field", PHRASE);
    type_into(&mut harness, "passphrase-prompt-confirmation", PHRASE);
    assert!(can_submit(&harness));
    harness
}

#[test]
fn a_phrase_typed_does_not_outlive_a_cancelled_prompt() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = asking(directory.path());

    click(&mut harness, "passphrase-prompt-cancel");
    settle(&mut harness);
    assert!(!present(&harness, "passphrase-prompt"));

    export(&mut harness);
    assert!(!can_submit(&harness), "the fields start empty");
}

#[test]
fn a_phrase_typed_does_not_outlive_an_export_in_the_clear() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = asking(directory.path());

    click(&mut harness, "passphrase-prompt-plain");
    settle(&mut harness);
    assert!(!present(&harness, "passphrase-prompt"));

    export(&mut harness);
    assert!(!can_submit(&harness), "the fields start empty");
}
