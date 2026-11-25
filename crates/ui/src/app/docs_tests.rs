//! About › Documentation, over the whole window: its pages, its search, its links.

use std::path::PathBuf;
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use i18n::Language;

use super::DevNotesApp;
use super::about_link_tests::Opened;
use super::tests::{click, count, open_library, present, settle};
use crate::api::help;

fn open(directory: &std::path::Path, language: Language) -> Harness<'static, DevNotesApp> {
    let mut harness = open_library(directory);
    harness.state_mut().state.language = language;
    click(&mut harness, "about-open");
    click(&mut harness, "about-documentation");
    settle(&mut harness);
    harness
}

fn title(harness: &Harness<'_, DevNotesApp>) -> String {
    let node = harness.get_by(|node| node.author_id() == Some("docs-title"));
    node.accesskit_node().label().unwrap_or_default()
}

fn search(harness: &mut Harness<'_, DevNotesApp>, words: &str) {
    click(harness, "docs-search");
    harness
        .get_by(|node| node.author_id() == Some("docs-search"))
        .type_text(words);
    settle(harness);
}

/// Scrolled to, then clicked: what is out of sight is under the window's edge, or past it.
fn reach(harness: &mut Harness<'_, DevNotesApp>, author_id: &str) {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .scroll_to_me();
    settle(harness);
    click(harness, author_id);
}

/// The first of several nodes of one address.
fn click_first(harness: &mut Harness<'_, DevNotesApp>, author_id: &str) {
    let first = |harness: &Harness<'_, DevNotesApp>, scroll: bool| {
        let node = harness
            .get_all_by(|node| node.author_id() == Some(author_id))
            .next()
            .expect("at least one");
        if scroll {
            node.scroll_to_me();
        } else {
            node.click();
        }
    };
    first(harness, true);
    settle(harness);
    first(harness, false);
    settle(harness);
}

#[test]
fn every_page_is_drawn_in_every_language() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open(directory.path(), Language::Fr);

    for language in Language::ALL {
        harness.state_mut().state.language = language;
        for entry in help::docs_index().pages() {
            reach(&mut harness, &format!("docs-page-{}", entry.id));
            let page = help::docs_page(language, &entry.id).expect("a page");
            assert_eq!(title(&harness), page.title, "{language:?}");
        }
    }
}

#[test]
fn a_search_finds_a_section_without_its_accents_and_leads_to_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open(directory.path(), Language::Fr);

    search(&mut harness, "derivation cle");
    assert!(count(&harness, "docs-result") > 0);
    assert!(!present(&harness, "docs-page-start"), "the list gave way");
    click_first(&mut harness, "docs-result");

    assert_eq!(title(&harness), "Sécurité et sauvegardes");
    assert!(present(&harness, "docs-page-start"), "and came back");
}

#[test]
fn a_search_that_finds_nothing_says_so() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open(directory.path(), Language::Fr);

    search(&mut harness, "zzzz");
    assert!(present(&harness, "docs-none"));
    click(&mut harness, "docs-search-clear");
    assert!(present(&harness, "docs-page-start"));
}

#[test]
fn a_link_leads_to_its_page_and_the_page_goes_on_to_the_next() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open(directory.path(), Language::Fr);

    // « Commencez par la prise en main », in the overview.
    click_first(&mut harness, "docs-link");
    assert_eq!(title(&harness), "Prise en main");

    reach(&mut harness, "docs-next");
    assert_eq!(title(&harness), "Écrire");
    reach(&mut harness, "docs-previous");
    assert_eq!(title(&harness), "Prise en main");
}

#[test]
fn a_section_of_the_page_is_reached_from_the_list() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open(directory.path(), Language::Fr);
    reach(&mut harness, "docs-page-safety");
    let place = |harness: &Harness<'_, DevNotesApp>| {
        harness
            .get_all_by(|node| node.author_id() == Some("docs-section"))
            .find(|node| node.accesskit_node().label().as_deref() == Some("La corbeille et Ctrl Z"))
            .map(|node| node.rect().top())
    };
    let before = place(&harness).expect("the section is in the page");

    reach(&mut harness, "docs-section-la-corbeille-et-ctrl-z");

    let after = place(&harness).expect("the section is in the page");
    assert!(after < before - 100.0, "scrolled to: {before} then {after}");
}

#[test]
fn an_address_elsewhere_opens_the_browser() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open(directory.path(), Language::En);
    let opened = Arc::new(Opened::default());
    harness.state_mut().state.opener = Arc::clone(&opened) as _;

    click(&mut harness, "docs-site");
    reach(&mut harness, "docs-page-start");
    click_first(&mut harness, "docs-link");

    assert_eq!(
        *opened.0.lock().unwrap(),
        [
            PathBuf::from("https://devnotes.fr/en/docs"),
            PathBuf::from("https://devnotes.fr/en/download"),
        ]
    );
}

#[test]
fn an_example_is_copied_and_said() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open(directory.path(), Language::Fr);
    harness.state_mut().state.settings.copy_confirmation = true;
    reach(&mut harness, "docs-page-start");

    click_first(&mut harness, "docs-copy");

    assert!(harness.state().state().status.latest().is_some());
}

#[test]
fn escape_closes_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut harness = open(directory.path(), Language::Fr);
    assert!(present(&harness, "docs-dialog"));

    harness.key_press(egui::Key::Escape);
    settle(&mut harness);

    assert!(!present(&harness, "docs-dialog"));
}
