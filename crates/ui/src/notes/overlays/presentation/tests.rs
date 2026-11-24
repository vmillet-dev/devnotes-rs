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
    presenting(BODY)
}

/// A Note holding `body`, open and presented.
fn presenting(body: &str) -> Harness<'static> {
    let body = body.to_owned();
    let (backend, profile) = testing::unlocked();
    let space = testing::note(&backend, "seed").space_id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        let draft = engine::notes::model::NoteDraft {
            kind: NoteKind::Note,
            language: Language::Txt,
            content: body,
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

/// Where the words `said` are drawn.
fn place(harness: &Harness<'_>, said: &str) -> egui::Rect {
    harness
        .query_all_by(|node| {
            node.label()
                .or_else(|| node.value())
                .is_some_and(|words| words == said)
        })
        .next()
        .unwrap_or_else(|| panic!("{said} is not on screen"))
        .rect()
}

#[test]
fn a_slide_is_where_it_stays_from_its_first_frame() {
    let mut harness = presenting(
        "## Courte\n\nUne ligne.\n\n## Longue\n\nUne.\n\nDeux.\n\nTrois.\n\nQuatre.\n\nCinq.\n",
    );
    for title in ["Courte", "Longue"] {
        // Not `key_press`, whose release takes a frame of its own.
        harness.input_mut().events.push(egui::Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        });
        harness.step();
        let first = place(&harness, title);
        settle(&mut harness);
        assert_eq!(place(&harness, title), first, "{title} moved");
    }
}

#[test]
fn titles_and_text_start_at_the_margin_on_every_slide_and_code_is_centred() {
    let mut harness = presenting(
        "# Un\n\nUne ligne.\n\n## Deux\n\n- Un point\n\n## Trois\n\n```rust\nlet x = 1;\n```\n",
    );
    harness.set_size(egui::vec2(1_900.0, 900.0));
    settle(&mut harness);
    let margin = 0.06 * 1_900.0;

    press(&mut harness, Key::ArrowRight);
    let first = place(&harness, "Un").min;
    assert!((first.x - margin).abs() < 1.0, "{first:?}");
    assert!((place(&harness, "Une ligne.").left() - margin).abs() < 1.0);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(place(&harness, "Deux").min, first);
    assert!(place(&harness, "Un point").left() < margin + 80.0);
    press(&mut harness, Key::ArrowRight);
    assert_eq!(place(&harness, "Trois").min, first);

    let code = place(&harness, "let x = 1;");
    assert!((code.center().x - 950.0).abs() < 1.0, "{code:?}");
    assert!(code.width() < 400.0);
}

/// A slide of one code block of `lines`, shown.
fn coded(lines: &[String]) -> Harness<'static> {
    let mut harness = presenting(&format!("## Code\n\n```rust\n{}\n```\n", lines.join("\n")));
    harness.set_size(egui::vec2(1_600.0, 900.0));
    settle(&mut harness);
    press(&mut harness, Key::ArrowRight);
    harness
}

fn numbered(count: usize) -> Vec<String> {
    (1..=count)
        .map(|at| format!("let line_{at} = {at};"))
        .collect()
}

/// The code painted: what it covers, what it is clipped to, and its first line's height.
fn code(harness: &Harness<'_>) -> (egui::Rect, egui::Rect, f32) {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) if text.galley.text().contains("let ") => Some((
                egui::Rect::from_min_size(text.pos, text.galley.size()),
                clipped.clip_rect,
                text.galley.rows[0].height(),
            )),
            _ => None,
        })
        // Over the editor, which paints it too.
        .next_back()
        .expect("the code is painted")
}

#[test]
fn a_line_of_code_wider_than_the_slide_is_wrapped() {
    let words = "client.get(url).header(token).timeout(thirty).send().await ";
    let harness = coded(&[format!("let {}END", words.repeat(6))]);

    let (covered, clip, _) = code(&harness);
    assert!(covered.right() <= clip.right(), "{covered:?} in {clip:?}");
    assert!(covered.left() >= clip.left());
}

#[test]
fn a_block_taller_than_the_slide_is_fitted_and_past_the_floor_it_scrolls() {
    let (_, _, full) = code(&coded(&numbered(3)));

    let title = place(&coded(&numbered(3)), "Code");
    assert_eq!(place(&coded(&numbered(60)), "Code"), title, "one title");
    let (covered, clip, fitted) = code(&coded(&numbered(30)));
    assert!(fitted < full, "{fitted} against {full}");
    assert!(covered.bottom() <= clip.bottom(), "{covered:?} in {clip:?}");

    let mut harness = coded(&numbered(60));
    let (covered, _, floor) = code(&harness);
    assert!(
        (floor / full - super::SMALLEST).abs() < 0.05,
        "{floor} against {full}"
    );
    let bottom = 900.0 * 0.9;
    assert!(covered.bottom() > bottom, "sixty lines do not hold");
    harness.event(egui::Event::PointerMoved(egui::pos2(800.0, 400.0)));
    for _ in 0..40 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -300.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    settle(&mut harness);
    assert!(
        code(&harness).0.bottom() <= bottom,
        "the last line is reached"
    );
}

#[test]
fn a_fitted_slide_is_where_it_stays_from_its_first_frame() {
    let mut harness = presenting(&format!(
        "## Titre\n\n```rust\n{}\n```\n",
        numbered(30).join("\n")
    ));
    harness.input_mut().events.push(egui::Event::Key {
        key: Key::ArrowRight,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    let first = (place(&harness, "Titre"), code(&harness).0);
    settle(&mut harness);
    assert_eq!((place(&harness, "Titre"), code(&harness).0), first);
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
