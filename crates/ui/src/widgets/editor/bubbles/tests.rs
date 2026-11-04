use egui::Key;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

use super::super::{Editor, EditorView};
use super::Completion;

const WORDS: [&str; 3] = ["somme", "sommet", "moyenne"];

/// The editor, completing from `WORDS` from three characters, and saying the length of the
/// word under the pointer.
fn screen(text: &str) -> Harness<'static, Editor> {
    crate::testing::harness()
        .with_size(egui::vec2(600.0, 300.0))
        .build_ui_state(
            |ui, editor: &mut Editor| {
                let (from, word) = editor.word_at_caret();
                let items: Vec<String> = if word.chars().count() >= 3 {
                    WORDS
                        .iter()
                        .filter(|candidate| candidate.starts_with(&word) && **candidate != word)
                        .map(|candidate| (*candidate).to_owned())
                        .collect()
                } else {
                    Vec::new()
                };
                let hover = |at: usize| Some(format!("position {at}"));
                EditorView::new(editor, "Feuille", "sheet")
                    .completion(Some(Completion {
                        from,
                        items: &items,
                    }))
                    .hover(&hover)
                    .show(ui);
            },
            Editor::new(text),
        )
}

fn type_in(harness: &mut Harness<'_, Editor>, words: &str) {
    harness
        .get_by(|node| node.author_id() == Some("sheet"))
        .click();
    harness.run();
    let end = harness.state().buffer.len();
    harness.state_mut().buffer.move_to(end, false);
    harness
        .get_by(|node| node.author_id() == Some("sheet"))
        .type_text(words);
    harness.run();
    harness.run();
}

fn listed(harness: &Harness<'_, Editor>) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some("editor-completion-option"))
        .filter_map(|node| node.accesskit_node().label())
        .collect()
}

#[test]
fn the_list_opens_from_three_characters_and_tab_accepts_the_lit_entry() {
    let mut harness = screen("");
    type_in(&mut harness, "so");
    assert!(listed(&harness).is_empty());
    type_in(&mut harness, "m");
    assert_eq!(listed(&harness), ["somme", "sommet"]);

    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();

    assert_eq!(harness.state().text(), "sommet");
    assert!(listed(&harness).is_empty());
}

#[test]
fn escape_closes_the_list_and_the_text_keeps_the_focus() {
    let mut harness = screen("x = ");
    type_in(&mut harness, "moy");
    assert_eq!(listed(&harness), ["moyenne"]);

    harness.key_press(Key::Escape);
    harness.run();
    harness.run();

    assert!(listed(&harness).is_empty());
    assert!(
        harness
            .get_by(|node| node.author_id() == Some("sheet"))
            .is_focused()
    );
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(
        harness.state().text(),
        "x = moy\n",
        "Enter is the text's again"
    );
}

#[test]
fn an_entry_clicked_replaces_the_word() {
    let mut harness = screen("");
    type_in(&mut harness, "som");
    harness
        .get_by(|node| {
            node.author_id() == Some("editor-completion-option")
                && node.label().as_deref() == Some("somme")
        })
        .click();
    harness.run();
    assert_eq!(harness.state().text(), "somme");
}

#[test]
fn the_word_at_the_caret_stops_at_what_is_not_a_letter() {
    let mut editor = Editor::new("total = prix_ht");
    editor.buffer.move_to(15, false);
    assert_eq!(editor.word_at_caret(), (8, "prix_ht".to_owned()));
    editor.buffer.move_to(5, true);
    assert_eq!(editor.word_at_caret().1, "");
}

#[test]
fn the_tooltip_says_the_callers_words_for_the_position_hovered() {
    let mut harness = screen("abcdef");
    harness
        .ctx
        .all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);
    harness.run();
    let node = harness.get_by(|node| node.author_id() == Some("sheet"));
    let bounds = node.accesskit_node().bounding_box().unwrap();
    #[expect(clippy::cast_possible_truncation, reason = "points on a screen")]
    let at = egui::pos2(bounds.x0 as f32 + 60.0, bounds.y0 as f32 + 8.0);
    harness.hover_at(at);
    for _ in 0..10 {
        harness.step();
    }
    let tip = harness.query_by(|node| node.author_id() == Some("editor-hover"));
    assert!(tip.is_some_and(|tip| {
        tip.accesskit_node()
            .value()
            .unwrap_or_default()
            .starts_with("position")
    }));
}
