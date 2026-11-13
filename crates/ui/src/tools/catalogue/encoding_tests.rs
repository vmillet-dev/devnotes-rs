//! The encoding tools over a real core: what is typed in, what comes out.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::text_tests::{click, has, opened, output_values, type_in, value_of, waits_for};
use crate::notes::bench::{Bench, settle};

fn labels(harness: &Harness<'_, Bench>, id: &str) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some(id))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect()
}

#[test]
fn the_url_codec_decodes_what_is_typed_on_the_encoded_side() {
    let mut harness = opened("url-codec");

    type_in(&mut harness, "url-codec-encoded", "caf%C3%A9");

    waits_for(&mut harness, "the text is decoded", |harness| {
        value_of(harness, "url-codec-decoded") == "café"
    });
}

#[test]
fn a_broken_escape_is_said_where_it_is() {
    let mut harness = opened("url-codec");

    type_in(&mut harness, "url-codec-encoded", "100%zz");

    waits_for(&mut harness, "the escape is refused", |harness| {
        has(harness, "url-codec-problem")
    });
}

#[test]
fn the_url_parser_takes_a_url_apart_and_never_reads_its_password_back() {
    let mut harness = opened("url-codec");
    click(&mut harness, "tool-sample");
    click(&mut harness, "url-tab-1");

    waits_for(&mut harness, "the parts are shown", |harness| {
        has(harness, "url-parser-parts")
    });

    let values = output_values(&harness);
    assert!(values.contains(&"api.example.com".to_owned()), "{values:?}");
    assert!(values.contains(&"••••••".to_owned()));
    assert!(!values.iter().any(|value| value == "secret"));
}

#[test]
fn the_hash_tool_finds_the_algorithm_of_a_digest() {
    let mut harness = opened("hash");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the digest is matched", |harness| {
        has(harness, "hash-verdict")
    });

    assert!(value_of(&harness, "hash-verdict").contains("SHA-256"));
    assert!(has(&harness, "hash-results"));
}

#[test]
fn an_hmac_key_changes_the_digests_and_goes_with_the_tool() {
    let mut harness = opened("hash");
    type_in(&mut harness, "hash-input", "hello");
    waits_for(&mut harness, "it is hashed", |harness| {
        has(harness, "hash-results")
    });
    let plain = output_values(&harness);

    click(&mut harness, "hash-hmac");
    type_in(&mut harness, "hash-key", "key");
    waits_for(&mut harness, "it is keyed", |harness| {
        output_values(harness) != plain
    });

    click(&mut harness, "tool-back");
    let bench = harness.state_mut();
    let open = crate::action::Action::Tools(crate::tools::Action::Open("hash"));
    bench
        .state
        .apply(open, &egui::Context::default(), &bench.backend);
    settle(&mut harness);
    assert_eq!(value_of(&harness, "hash-key"), "");
}

#[test]
fn passwords_are_drawn_with_their_strength_and_kept_from_no_session() {
    let mut harness = opened("password");
    waits_for(&mut harness, "they are drawn", |harness| {
        has(harness, "password-strength")
    });
    let drawn = output_values(&harness);
    assert_eq!(drawn.len(), 5);
    assert_eq!(drawn[0].chars().count(), 20);

    click(&mut harness, "password-generate");
    waits_for(&mut harness, "others are drawn", |harness| {
        output_values(harness) != drawn
    });
}

#[test]
fn no_character_set_left_is_said() {
    let mut harness = opened("password");
    for set in ["lowercase", "uppercase", "digits", "symbols"] {
        click(&mut harness, &format!("password-set-{set}"));
    }

    waits_for(&mut harness, "nothing can be drawn", |harness| {
        has(harness, "password-problem")
    });
}

#[test]
fn a_token_is_decoded_verified_and_gone_with_the_tool() {
    let mut harness = opened("jwt");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the token is decoded", |harness| {
        has(harness, "jwt-state")
    });
    assert!(value_of(&harness, "jwt-header").contains("HS256"));
    assert!(value_of(&harness, "jwt-payload").contains("Ada Lovelace"));
    assert!(value_of(&harness, "jwt-verification").contains("Valid signature."));

    click(&mut harness, "tool-back");
    let bench = harness.state_mut();
    let open = crate::action::Action::Tools(crate::tools::Action::Open("jwt"));
    bench
        .state
        .apply(open, &egui::Context::default(), &bench.backend);
    settle(&mut harness);
    assert_eq!(value_of(&harness, "jwt-token"), "");
}

#[test]
fn a_token_that_is_none_says_why() {
    let mut harness = opened("jwt");

    type_in(&mut harness, "jwt-token", "not a token");

    waits_for(&mut harness, "the token is refused", |harness| {
        has(harness, "jwt-problem")
    });
}

#[test]
fn a_text_is_written_in_six_encodings() {
    let mut harness = opened("base64");

    click(&mut harness, "tool-sample");
    waits_for(&mut harness, "the text is encoded", |harness| {
        has(harness, "base64-encodings")
    });

    let values = output_values(&harness);
    assert_eq!(values.len(), 6);
    assert!(values.contains(&"Q2Fmw6k=".to_owned()), "{values:?}");
}

#[test]
fn an_encoding_is_recognised_and_decoded() {
    let mut harness = opened("base64");
    click(&mut harness, "base64-direction-1");

    type_in(&mut harness, "base64-input", "Q2Fmw6k=");

    waits_for(&mut harness, "the text is decoded", |harness| {
        has(harness, "base64-output")
    });
    assert_eq!(value_of(&harness, "base64-output"), "Café");
    assert!(has(&harness, "base64-reading"));
}

#[test]
fn the_ascii_table_lists_the_128_codes_and_finds_one_in_any_base() {
    let mut harness = opened("ascii");
    settle(&mut harness);
    assert_eq!(labels(&harness, "reference-row").len(), 128);

    type_in(&mut harness, "reference-search", "0x41");

    assert_eq!(labels(&harness, "reference-row"), ["65"]);
}

/// Left and opened again: the field's undo does not bring the key back.
#[test]
fn a_key_left_behind_is_gone_from_the_undo() {
    let mut harness = opened("hash");
    type_in(&mut harness, "hash-input", "hello");
    click(&mut harness, "hash-hmac");
    type_in(&mut harness, "hash-key", "key");
    settle(&mut harness);

    click(&mut harness, "tool-back");
    let bench = harness.state_mut();
    let open = crate::action::Action::Tools(crate::tools::Action::Open("hash"));
    bench
        .state
        .apply(open, &egui::Context::default(), &bench.backend);
    settle(&mut harness);

    click(&mut harness, "hash-key");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut harness);
    assert_eq!(value_of(&harness, "hash-key"), "");
}
