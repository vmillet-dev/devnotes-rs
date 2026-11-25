//! The documentation quotes the application: a label or a shortcut renamed here is found stale in
//! its pages. The words are the pages' own, as they are typed, so each failure names its line.

use std::collections::BTreeSet;

use engine::docs::model::{Block, Inline, Page};
use engine::docs::{docs_index, docs_page};
use serde_json::Value;

use super::shortcut_sheet;
use super::shortcuts::Bindings;

const LANGUAGES: [(&str, &str); 2] = [
    ("fr", include_str!("../../../i18n/translations/fr.json")),
    ("en", include_str!("../../../i18n/translations/en.json")),
];

/// A key as the pages write it, and as the bindings do.
const KEYS: [(&str, &str); 8] = [
    ("Échap", "Escape"),
    ("Esc", "Escape"),
    ("Entrée", "Enter"),
    ("Suppr", "Delete"),
    ("Retour\u{a0}arrière", "Backspace"),
    ("Espace", "Space"),
    ("Maj", "Shift"),
    ("Super", "Meta"),
];

fn pages(language: &str) -> impl Iterator<Item = &'static Page> {
    docs_index()
        .pages()
        .filter_map(move |entry| docs_page(language, &entry.id))
}

fn runs(block: &Block) -> Vec<Inline> {
    match block {
        Block::Paragraph(text) | Block::Note { text, .. } => text.clone(),
        Block::List { items, .. } => items.concat(),
        Block::Table { rows, .. } => rows.concat().concat(),
        Block::Shortcuts { rows, .. } => rows
            .iter()
            .map(|row| Inline::Keys(row.keys.clone()))
            .collect(),
        Block::Heading { .. } | Block::Code { .. } | Block::Pages(_) => Vec::new(),
    }
}

/// The line of a page where something is quoted.
fn place(language: &str, page: &Page, quoted: &str) -> String {
    let path = format!("docs/content/{language}/{}.md", page.id);
    let full = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(full).unwrap_or_default();
    let line = text.lines().position(|line| line.contains(quoted));
    format!("{path}:{}: {quoted}", line.map_or(0, |line| line + 1))
}

fn straight(text: &str) -> String {
    text.replace('’', "'").trim().to_owned()
}

fn texts(node: &Value, out: &mut BTreeSet<String>) {
    match node {
        Value::String(text) => {
            out.insert(straight(text));
        }
        Value::Object(entries) => entries.values().for_each(|value| texts(value, out)),
        _ => {}
    }
}

/// Whether a label is a text of the application, whatever its placeholders hold.
fn fits(text: &str, label: &str) -> bool {
    let Some((before, rest)) = text.split_once("{{") else {
        return text == label;
    };
    let Some((_, after)) = rest.split_once("}}") else {
        return false;
    };
    let letters = format!("{before}{after}");
    letters.chars().filter(|each| each.is_alphabetic()).count() > 3
        && label.len() > before.len() + after.len()
        && label.starts_with(before)
        && label.ends_with(after)
}

#[test]
fn a_label_fits_its_text_and_what_its_placeholder_holds() {
    assert!(fits("Tableau", "Tableau"));
    assert!(fits("Quitter {{app}}", "Quitter DevNotes"));
    assert!(fits(
        "+ Créer une note « {{query}} »",
        "+ Créer une note « … »"
    ));
    assert!(!fits("Tableau", "Le tableau"));
    assert!(!fits("Quitter {{app}}", "Quitter"));
    assert!(!fits("{{count}} / {{all}}", "Ranger dans"));
}

#[test]
fn every_label_the_pages_quote_is_the_application_s() {
    let mut stale = Vec::new();
    for (language, json) in LANGUAGES {
        let mut known = BTreeSet::new();
        texts(
            &serde_json::from_str(json).expect("the translations"),
            &mut known,
        );
        for page in pages(language) {
            for run in page.body.iter().flat_map(runs) {
                let Inline::Label(label) = run else { continue };
                let parts = label.split('→').map(straight);
                for part in parts.filter(|part| !known.iter().any(|text| fits(text, part))) {
                    stale.push(format!("{} (« {part} »)", place(language, page, &label)));
                }
            }
        }
    }
    assert!(
        stale.is_empty(),
        "these labels are no text of the application:\n{}",
        stale.join("\n")
    );
}

/// Every combination the application answers to, by default, as `Ctrl Alt P`.
fn combinations() -> BTreeSet<String> {
    let bindings = Bindings::default();
    let mut known = BTreeSet::new();
    for row in shortcut_sheet::sheet().iter().flat_map(|group| &group.rows) {
        let caps = row.caps(&bindings);
        known.insert(caps.join(" "));
        // `Alt ↑ ↓` is two combinations, quoted together or apart.
        if let Some((arrows, held)) = caps.split_last() {
            for arrow in arrows.split(' ') {
                known.insert([held, &[arrow.to_owned()]].concat().join(" "));
            }
        }
    }
    known
}

/// Only what holds a modifier: a letter alone, an arrow or Escape mean something nearly everywhere.
#[test]
fn every_shortcut_the_pages_quote_is_the_application_s() {
    let known = combinations();
    let mut stale = Vec::new();
    for (language, _) in LANGUAGES {
        for page in pages(language) {
            for run in page.body.iter().flat_map(runs) {
                let Inline::Keys(keys) = run else { continue };
                let named: Vec<_> = keys
                    .iter()
                    .map(|key| {
                        KEYS.iter()
                            .find(|(typed, _)| typed == key)
                            .map_or(key.as_str(), |(_, name)| name)
                    })
                    .collect();
                let held = matches!(named[0], "Ctrl" | "Alt" | "Shift" | "Meta");
                if held && named.len() > 1 && !known.contains(&named.join(" ")) {
                    stale.push(place(
                        language,
                        page,
                        &format!("<kbd>{}</kbd>", keys.join(" ")),
                    ));
                }
            }
        }
    }
    assert!(
        stale.is_empty(),
        "these shortcuts are none of the application's:\n{}",
        stale.join("\n")
    );
}
