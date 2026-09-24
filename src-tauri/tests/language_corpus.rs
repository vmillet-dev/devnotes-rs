//! What detection actually answers, against real snippets rather than one line each.
//!
//! A corpus and not examples: the inline tests are cases written from memory, and say nothing
//! about what a change to one marker broke elsewhere. This file does.
//!
//! One file per case, named `<language>[-<variant>].txt`, so adding a case is adding a file.
//! Read from disk rather than `include_str!`: the point is that nobody has to edit a
//! table to add a snippet.

use std::fs;
use std::path::PathBuf;

use devnotes_lib::notes::language::{Language, from_content};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

/// `rs-match-arm.txt` is Rust; `txt-prose.txt` is plain text. The prefix is the answer.
fn expected(file_name: &str) -> Language {
    let stem = file_name.trim_end_matches(".txt");
    let tag = stem.split_once('-').map_or(stem, |(head, _)| head);

    tag.parse()
        .unwrap_or_else(|()| panic!("{file_name} names a language that does not exist: {tag}"))
}

fn cases() -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = fs::read_dir(corpus())
        .expect("the corpus directory ships with the tests")
        .map(|entry| entry.expect("a readable corpus entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("a corpus file with a readable name")
                .to_string();
            (name, fs::read_to_string(&path).expect("a readable snippet"))
        })
        .collect();

    found.sort_by(|(a, _), (b, _)| a.cmp(b));
    found
}

/// Asserted in bulk and reported in bulk: one failing snippet must not hide the other
/// nineteen, because what a change to a marker breaks is exactly what this file is for.
#[test]
fn every_snippet_is_recognised_as_what_it_is() {
    let wrong: Vec<String> = cases()
        .into_iter()
        .filter_map(|(name, content)| {
            let want = expected(&name);
            let got = from_content(&content);
            (got != want).then(|| format!("{name}: wanted {want}, got {got}"))
        })
        .collect();

    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The corpus is only worth what it covers: a language with no snippet is a language whose
/// markers nothing is watching.
#[test]
fn every_detectable_language_has_at_least_one_snippet() {
    let covered: Vec<Language> = cases()
        .into_iter()
        .map(|(name, _)| expected(&name))
        .collect();

    let missing: Vec<String> = Language::ALL
        .iter()
        .filter(|language| !covered.contains(language))
        .map(ToString::to_string)
        .collect();

    assert!(missing.is_empty(), "no snippet for: {}", missing.join(", "));
}

/// A Rust snippet carrying a match arm is Rust, whatever `=>` suggests.
#[test]
fn a_lambda_arrow_no_longer_decides_the_language() {
    let rust = fs::read_to_string(corpus().join("rs-match-arm.txt")).unwrap();

    assert_eq!(from_content(&rust), Language::Rs);
}

/// `class Note { }` is written the same way in six of these languages, and a wrong answer
/// costs more than none.
#[test]
fn a_snippet_that_could_be_anything_is_left_alone() {
    for name in ["txt-ambiguous.txt", "txt-lambda.txt", "txt-prose.txt"] {
        let content = fs::read_to_string(corpus().join(name)).unwrap();
        assert_eq!(from_content(&content), Language::Txt, "{name}");
    }
}
