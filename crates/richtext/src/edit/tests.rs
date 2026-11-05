//! Each operation from Markdown to Markdown, the caret where the reference's editor leaves it.

use super::*;
use crate::{read, write};

fn caret_at(path: &[usize], offset: usize) -> Selection {
    caret(path.to_vec(), offset)
}

fn range(from: (&[usize], usize), to: (&[usize], usize)) -> Selection {
    Selection {
        anchor: at(from.0.to_vec(), from.1),
        head: at(to.0.to_vec(), to.1),
    }
}

#[test]
fn typing_writes_at_the_caret_with_its_marks_and_a_line_splits() {
    let mut document = read("ab");
    let bold = Marks {
        bold: true,
        ..Marks::default()
    };
    let after = insert_text(&mut document, &caret_at(&[0], 2), "c", &bold);
    assert_eq!(write(&document), "ab**c**");
    assert_eq!(after, caret_at(&[0], 3));
    let after = insert_text(&mut document, &after, "\nd", &Marks::default());
    assert_eq!(write(&document), "ab**c**\n\nd");
    assert_eq!(after, caret_at(&[1], 1));
}

#[test]
fn enter_splits_a_paragraph_and_a_heading_ends_into_a_paragraph() {
    let mut document = read("# Titre\n\nabcd");
    let after = split(&mut document, &caret_at(&[1], 2));
    assert_eq!(write(&document), "# Titre\n\nab\n\ncd");
    assert_eq!(after, caret_at(&[2], 0));
    let after = split(&mut document, &caret_at(&[0], 5));
    assert_eq!(after, caret_at(&[1], 0));
    assert_eq!(
        write(&document),
        "# Titre\n\n\n\nab\n\ncd",
        "an empty paragraph under it"
    );
}

#[test]
fn enter_in_an_item_adds_one_and_in_an_empty_one_leaves_the_list() {
    let mut document = read("- un\n- deux");
    let after = split(&mut document, &caret_at(&[0, 1, 0], 4));
    assert_eq!(after, caret_at(&[0, 2, 0], 0));
    let after = split(&mut document, &after);
    assert_eq!(after, caret_at(&[1], 0));
    assert!(matches!(&document.blocks[1], Block::Paragraph(content) if content.is_empty()));
    assert_eq!(write(&document), "- un\n- deux");
}

#[test]
fn backspace_lifts_an_item_and_joins_two_paragraphs() {
    let mut document = read("- un\n- deux\n- trois");
    let after = delete_backward(&mut document, &caret_at(&[0, 1, 0], 0));
    assert_eq!(write(&document), "- un\n\ndeux\n\n- trois");
    assert_eq!(after, caret_at(&[1], 0));
    let after = delete_backward(&mut document, &after);
    assert_eq!(write(&document), "- undeux\n\n- trois");
    assert_eq!(after, caret_at(&[0, 0, 0], 2));
}

#[test]
fn tab_nests_an_item_under_the_one_above_and_shift_tab_takes_it_back() {
    let mut document = read("- un\n- deux\n- trois");
    let after = sink_item(&mut document, &caret_at(&[0, 1, 0], 1));
    assert_eq!(write(&document), "- un\n  - deux\n- trois");
    assert_eq!(after, caret_at(&[0, 0, 1, 0, 0], 1));
    let after = lift_item(&mut document, &[0, 0, 1], 0);
    assert_eq!(write(&document), "- un\n- deux\n- trois");
    assert_eq!(after, caret_at(&[0, 1, 0], 0));
}

#[test]
fn a_selection_across_blocks_is_taken_out_and_its_ends_joined() {
    let mut document = read("abc\n\n- un\n- deux\n\n---\n\nfin");
    let after = delete_selection(&mut document, &range((&[0], 1), (&[3], 1)));
    assert_eq!(write(&document), "ain");
    assert_eq!(after, caret_at(&[0], 1));
}

#[test]
fn a_mark_toggles_over_a_selection_across_two_paragraphs() {
    let mut document = read("ab\n\ncd");
    let selection = range((&[0], 1), (&[1], 1));
    toggle_mark(&mut document, &selection, &Mark::Bold);
    assert_eq!(write(&document), "a**b**\n\n**c**d");
    toggle_mark(&mut document, &selection, &Mark::Bold);
    assert_eq!(write(&document), "ab\n\ncd");
}

#[test]
fn headings_lists_and_quotes_come_and_go() {
    let mut document = read("un");
    set_heading(&mut document, &caret_at(&[0], 0), Some(2));
    assert_eq!(write(&document), "## un");
    set_heading(&mut document, &caret_at(&[0], 0), None);
    let after = toggle_list(&mut document, &caret_at(&[0], 1), ListKind::Bullet);
    assert_eq!(write(&document), "- un");
    let after = toggle_list(&mut document, &after, ListKind::Ordered { start: 1 });
    assert_eq!(write(&document), "1. un");
    let after = toggle_list(&mut document, &after, ListKind::Ordered { start: 1 });
    assert_eq!(write(&document), "un");
    let after = toggle_quote(&mut document, &after);
    assert_eq!(write(&document), "> un");
    toggle_quote(&mut document, &after);
    assert_eq!(write(&document), "un");
}

#[test]
fn a_task_is_ticked_and_the_selection_read_as_text() {
    let mut document = read("- [ ] fait\n\nfin");
    toggle_task(&mut document, &[0], 0);
    assert_eq!(write(&document), "- [x] fait\n\nfin");
    let text = selected_text(&document, &range((&[0, 0, 0], 2), (&[1], 2)));
    assert_eq!(text, "it\nfi");
}

#[test]
fn delete_joins_the_next_block_and_backspace_takes_a_rule_out() {
    let mut document = read("ab\n\ncd");
    let after = delete_forward(&mut document, &caret_at(&[0], 2));
    assert_eq!(write(&document), "abcd");
    assert_eq!(after, caret_at(&[0], 2));
    let after = delete_forward(&mut document, &caret_at(&[0], 1));
    assert_eq!(write(&document), "acd");
    assert_eq!(after, caret_at(&[0], 1));

    let mut document = read("un\n\n---\n\ndeux");
    let after = delete_backward(&mut document, &caret_at(&[2], 0));
    assert_eq!(write(&document), "un\n\ndeux");
    assert_eq!(after, caret_at(&[1], 0));
}

#[test]
fn a_heading_at_its_start_becomes_a_paragraph_and_a_break_is_typed() {
    let mut document = read("## Titre");
    delete_backward(&mut document, &caret_at(&[0], 0));
    assert_eq!(write(&document), "Titre");
    let after = hard_break(&mut document, &caret_at(&[0], 2));
    assert_eq!(write(&document), "Ti  \ntre");
    assert_eq!(after, caret_at(&[0], 3));
}

#[test]
fn a_nested_item_lifted_takes_the_ones_after_it_along() {
    let mut document = read("- un\n  - a\n  - b\n- deux");
    let after = lift_item(&mut document, &[0, 0, 1], 0);
    assert_eq!(write(&document), "- un\n- a\n  - b\n- deux");
    assert_eq!(after, caret_at(&[0, 1, 0], 0));
}

#[test]
fn an_empty_line_at_a_quote_s_end_leaves_it_and_a_rule_is_inserted() {
    let mut document = read("> cité");
    let after = split(&mut document, &caret_at(&[0, 0], 4));
    let after = split(&mut document, &after);
    assert_eq!(after, caret_at(&[1], 0));
    assert_eq!(write(&document), "> cité");
    let after = insert_rule(&mut document, &after);
    assert_eq!(after, caret_at(&[3], 0));
    assert!(matches!(document.blocks[2], Block::Rule));
}

#[test]
fn a_selection_inside_one_block_and_text_typed_over_it() {
    let mut document = read("abcdef");
    let after = insert_text(
        &mut document,
        &range((&[0], 1), (&[0], 4)),
        "X",
        &Marks::default(),
    );
    assert_eq!(write(&document), "aXef");
    assert_eq!(after, caret_at(&[0], 2));
}
