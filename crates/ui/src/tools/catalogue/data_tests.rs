//! The data and generator tools over a real core: what is typed in, what comes out.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::text_tests::{click, has, opened, output_values, type_in, value_of, waits_for};
use crate::notes::bench::{Bench, settle};

const SHIPPED_LISTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/resources/identity");

/// The lists the bench's core reads: its resources are its profile. ⚠️ The core keeps them for the
/// life of the process, so a test here can neither find them missing nor damage them.
fn lists(harness: &Harness<'_, Bench>) -> PathBuf {
    let paths = harness.state().backend.engine().paths();
    paths.resources.join("identity")
}

fn copy_folder(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_folder(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn identity_with_its_lists() -> Harness<'static, Bench> {
    let mut harness = opened("identity");
    let lists = lists(&harness);
    copy_folder(Path::new(SHIPPED_LISTS), &lists);
    click(&mut harness, "tool-action-draw");
    waits_for(&mut harness, "a person is drawn", |harness| {
        has(harness, "identity-card")
    });
    harness
}

#[test]
fn a_json_document_becomes_yaml_and_the_swap_brings_it_back() {
    let mut harness = opened("convert");

    type_in(&mut harness, "convert-input", r#"{"service":"billing"}"#);
    waits_for(&mut harness, "the document is converted", |harness| {
        has(harness, "convert-output")
    });
    assert!(value_of(&harness, "convert-output").contains("service: billing"));

    click(&mut harness, "convert-swap");
    waits_for(&mut harness, "the round trip is back", |harness| {
        has(harness, "convert-output") && value_of(harness, "convert-output").contains('{')
    });
    assert!(value_of(&harness, "convert-input").contains("service: billing"));
}

#[test]
fn a_broken_document_says_where_and_a_null_is_named_when_toml_drops_it() {
    let mut harness = opened("convert");

    type_in(&mut harness, "convert-input", "{");
    waits_for(&mut harness, "the place is said", |harness| {
        has(harness, "convert-unreadable")
    });

    click(&mut harness, "tool-clear");
    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the null is named", |harness| {
        has(harness, "convert-dropped")
    });
    assert!(value_of(&harness, "convert-dropped").contains("telephone"));
}

#[test]
fn a_pinned_seed_draws_the_same_documents_again() {
    let mut harness = opened("json-generator");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "documents are drawn", |harness| {
        has(harness, "json-generator-output")
    });
    assert!(has(&harness, "json-generator-seed-used"));

    click(&mut harness, "json-generator-keep-seed");
    waits_for(&mut harness, "the seed is pinned", |harness| {
        has(harness, "json-generator-seed-pinned")
    });
    let first = value_of(&harness, "json-generator-output");
    click(&mut harness, "tool-action-draw");
    settle(&mut harness);

    assert_eq!(value_of(&harness, "json-generator-output"), first);
}

#[test]
fn an_example_read_as_a_schema_offers_to_be_read_as_an_example() {
    let mut harness = opened("json-generator");

    click(&mut harness, "json-generator-source-0");
    type_in(&mut harness, "json-generator-input", r#"{"name":"Ada"}"#);
    waits_for(&mut harness, "the mistake is said", |harness| {
        has(harness, "json-generator-as-example")
    });

    click(&mut harness, "json-generator-as-example");
    waits_for(&mut harness, "documents are drawn", |harness| {
        has(harness, "json-generator-output")
    });
}

#[test]
fn identifiers_are_drawn_and_one_pasted_is_read_back() {
    let mut harness = opened("identifiers");

    waits_for(&mut harness, "identifiers are drawn", |harness| {
        has(harness, "identifiers-row")
    });

    let uuid = "0190a5f4-7e3b-7c1a-9f2e-3b4c5d6e7f80";
    type_in(&mut harness, "identifiers-checked", uuid);
    waits_for(&mut harness, "the identifier is read", |harness| {
        has(harness, "identifiers-facts")
    });
    assert!(
        output_values(&harness).iter().any(|it| it.contains("2024")),
        "a UUID v7 says its instant"
    );
}

#[test]
fn a_contrast_is_measured_and_its_fix_replaces_the_colour() {
    let mut harness = opened("colour");

    click(&mut harness, "tool-clear");
    type_in(&mut harness, "colour-input", "#777777");
    type_in(&mut harness, "colour-against", "#ffffff");
    waits_for(&mut harness, "the ratio is read", |harness| {
        has(harness, "colour-ratio")
    });
    assert!(has(&harness, "colour-sample"));

    click(&mut harness, "colour-fix");
    waits_for(&mut harness, "the fix is taken", |harness| {
        value_of(harness, "colour-input") != "#777777"
    });
}

#[test]
fn a_word_that_is_no_colour_is_said_and_the_swap_trades_the_two() {
    let mut harness = opened("colour");

    click(&mut harness, "colour-swap");
    assert_eq!(value_of(&harness, "colour-input"), "#ffffff");
    assert_eq!(value_of(&harness, "colour-against"), "#1e90ff");

    click(&mut harness, "tool-clear");
    type_in(&mut harness, "colour-input", "not a colour");
    waits_for(&mut harness, "the colour is refused", |harness| {
        has(harness, "colour-invalid")
    });
}

#[test]
fn a_code_is_drawn_and_a_url_without_its_scheme_is_said() {
    let mut harness = opened("qr");

    type_in(&mut harness, "qr-url", "exemple.fr");
    waits_for(&mut harness, "the code is drawn", |harness| {
        has(harness, "qr-code")
    });

    assert!(has(&harness, "qr-missing-scheme"));
    assert!(has(&harness, "qr-facts"));
}

#[test]
fn a_text_too_long_for_a_code_is_refused_with_its_size() {
    let mut harness = opened("qr");

    click(&mut harness, "qr-kind-0");
    type_in(&mut harness, "qr-text", &"a".repeat(3_000));
    waits_for(&mut harness, "the refusal is said", |harness| {
        has(harness, "qr-too-long")
    });

    assert!(!has(&harness, "qr-code"));
}

#[test]
fn a_wifi_password_warns_before_a_note_keeps_it() {
    let mut harness = opened("qr");

    click(&mut harness, "qr-kind-2");
    type_in(&mut harness, "qr-ssid", "Guests");
    type_in(&mut harness, "qr-password", "not-a-real-one");
    waits_for(&mut harness, "the code is drawn", |harness| {
        has(harness, "qr-code")
    });
    click(&mut harness, "tool-save-as-note");

    assert!(has(&harness, "save-as-note-warning"));
}

#[test]
fn the_code_is_saved_where_the_dialog_says() {
    let mut harness = opened("qr");
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("code.svg");
    harness.state_mut().state.dialog = Arc::new(platform::dialog::Fixed(path.clone()));

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the code is drawn", |harness| {
        has(harness, "qr-code")
    });
    click(&mut harness, "qr-svg");
    waits_for(&mut harness, "the file is written", |_| path.exists());

    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.starts_with("<svg") || written.starts_with("<?xml"));
}

#[test]
fn a_pinned_seed_brings_the_same_person_back() {
    let mut harness = identity_with_its_lists();

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the sample is drawn", |harness| {
        has(harness, "identity-seed-pinned")
    });
    settle(&mut harness);
    let surname = value_of(&harness, "identity-surname");

    click(&mut harness, "tool-action-draw");
    settle(&mut harness);

    assert_eq!(value_of(&harness, "identity-surname"), surname);
    assert!(has(&harness, "identity-document"));
    assert!(has(&harness, "identity-block-contact"));
    assert!(has(&harness, "identity-block-bank"));
    assert!(has(&harness, "identity-block-company"));
}

#[test]
fn the_pin_writes_the_seed_just_used_and_lets_it_go() {
    let mut harness = identity_with_its_lists();

    assert!(has(&harness, "identity-seed-used"));
    click(&mut harness, "identity-pin-seed");
    assert!(has(&harness, "identity-seed-pinned"));
    assert!(!value_of(&harness, "identity-seed").is_empty());

    click(&mut harness, "identity-pin-seed");
    assert!(!has(&harness, "identity-seed-pinned"));
}

#[test]
fn the_card_country_belongs_to_the_batch_and_cannot_leave_it() {
    let mut harness = identity_with_its_lists();
    let own = "identity-batch-country-FR";
    let is_lit = |harness: &Harness<'_, Bench>, id: &str| {
        let chip = harness.get_by(|node| node.author_id() == Some(id));
        chip.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
    };

    click(&mut harness, own);
    assert!(is_lit(&harness, own));

    click(&mut harness, "identity-batch-country-DE");
    assert!(is_lit(&harness, "identity-batch-country-DE"));
    click(&mut harness, "identity-batch-country-DE");
    assert!(!is_lit(&harness, "identity-batch-country-DE"));
}

#[test]
fn a_batch_is_written_as_a_file_of_as_many_people_as_asked() {
    let mut harness = identity_with_its_lists();
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("people.json");
    harness.state_mut().state.dialog = Arc::new(platform::dialog::Fixed(path.clone()));

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the sample is drawn", |harness| {
        has(harness, "identity-seed-pinned")
    });
    settle(&mut harness);
    let surname = value_of(&harness, "identity-surname");
    click(&mut harness, "identity-export");
    waits_for(&mut harness, "the file is written", |_| path.exists());

    let written = std::fs::read_to_string(&path).unwrap();
    let people: serde_json::Value = serde_json::from_str(&written).unwrap();
    assert_eq!(people.as_array().unwrap().len(), 10);
    assert_eq!(
        people[0]["surname"].as_str().unwrap().to_uppercase(),
        surname
    );
}

#[test]
fn the_separator_chosen_is_kept_with_the_application() {
    let mut harness = identity_with_its_lists();

    click(&mut harness, "identity-format-1");
    click(&mut harness, "identity-separator-1");
    settle(&mut harness);

    assert_eq!(
        harness.state().state.tools.csv_separator(),
        engine::tools::identity::CsvSeparator::Semicolon
    );
}
