use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::kind::NoteKind;
use engine::notes::language::Language;

use crate::notes::bench::{Bench, click_id, expanded_area, settle, until};
use crate::testing;

type Harness<'a> = egui_kittest::Harness<'a, Bench>;

const BODY: &str = "## Préparer\n\nLa sauvegarde.\n\n## Basculer\n\n### DNS\n\nLe TTL.\n";

/// A Note of two sections, open and presented.
fn presented() -> Harness<'static> {
    let (backend, profile) = testing::unlocked();
    let space = testing::note(&backend, "seed").space_id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        let draft = engine::notes::model::NoteDraft {
            kind: NoteKind::Note,
            language: Language::Txt,
            content: BODY.to_owned(),
            ..testing::snippet(space, "Bascule".to_owned())
        };
        engine::notes::create_note(engine, draft)
    })
    .unwrap();
    let mut harness = expanded_area(backend, profile);
    harness
        .get_by(|node| node.author_id() == Some("search-input"))
        .focus();
    harness.step();
    harness
        .get_by(|node| node.author_id() == Some("search-input"))
        .type_text("Bascule");
    until(&mut harness, "the Note alone", |harness| {
        crate::notes::bench::cards(harness) == 1
    });
    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-more");
    click_id(&mut harness, "editor-present");
    until(&mut harness, "the cover", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("presentation-cover"))
            .is_some()
    });
    harness
}

fn count(harness: &Harness<'_>) -> String {
    let node = harness.get_by(|node| node.author_id() == Some("presentation-count"));
    let node = node.accesskit_node();
    node.label().or_else(|| node.value()).unwrap_or_default()
}

fn press(harness: &mut Harness<'_>, key: Key) {
    harness.key_press_modifiers(Modifiers::NONE, key);
    settle(harness);
}

fn says(harness: &Harness<'_>, words: &str) -> bool {
    harness
        .query_all_by(|node| {
            node.label()
                .or_else(|| node.value())
                .is_some_and(|said| said.contains(words))
        })
        .next()
        .is_some()
}

#[test]
fn the_title_comes_first_then_each_section_a_slide_moved_by_the_keys_and_the_buttons() {
    let mut harness = presented();
    assert_eq!(count(&harness), "1 / 3");

    press(&mut harness, Key::ArrowRight);
    assert_eq!(count(&harness), "2 / 3");
    assert!(says(&harness, "La sauvegarde."));

    press(&mut harness, Key::End);
    assert_eq!(count(&harness), "3 / 3");
    assert!(says(&harness, "Le TTL."));
    press(&mut harness, Key::PageDown);
    assert_eq!(count(&harness), "3 / 3", "nothing past the last");

    click_id(&mut harness, "presentation-previous");
    assert_eq!(count(&harness), "2 / 3");
    press(&mut harness, Key::Home);
    assert_eq!(count(&harness), "1 / 3");
    click_id(&mut harness, "presentation-next");
    assert_eq!(count(&harness), "2 / 3");
}

#[test]
fn the_map_opens_the_slide_of_the_heading_pressed() {
    let mut harness = presented();

    press(&mut harness, Key::M);
    let headings: Vec<String> = harness
        .query_all_by(|node| node.author_id() == Some("presentation-map-node"))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect();
    assert_eq!(headings, ["Bascule", "Préparer", "Basculer", "DNS"]);

    harness
        .get_by(|node| {
            node.author_id() == Some("presentation-map-node")
                && node.label().as_deref() == Some("DNS")
        })
        .click();
    settle(&mut harness);

    assert_eq!(count(&harness), "3 / 3");
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("presentation-map-node"))
            .is_none()
    );
}

#[test]
fn escape_leaves_and_the_editor_stays() {
    let mut harness = presented();

    press(&mut harness, Key::Escape);
    until(&mut harness, "left", |harness| {
        harness
            .query_by(|node| node.author_id() == Some("presentation"))
            .is_none()
    });

    assert!(
        harness
            .query_by(|node| node.author_id() == Some("editor-rich"))
            .is_some()
    );
}
