//! Each rule typed a `char` at a time, as the editor runs it after every one.

use super::*;
use crate::model::{Marks, Position};
use crate::{read, write};

/// What a fresh Note holds after `typed`, each rule applied as it closes.
fn typed(typed: &str) -> String {
    let mut document = read("");
    document.blocks.push(Block::Paragraph(Vec::new()));
    let mut selection = Selection::caret(Position {
        path: vec![0],
        offset: 0,
    });
    for glyph in typed.chars() {
        selection = edit::insert_text(
            &mut document,
            &selection,
            &glyph.to_string(),
            &Marks::default(),
        );
        if let Some(applied) = after_typing(&mut document, &selection) {
            selection = applied.selection;
        }
    }
    write(&document)
}

#[test]
fn block_rules_turn_the_paragraph_into_what_its_start_writes() {
    assert_eq!(typed("## Titre"), "## Titre");
    assert_eq!(typed("> cité"), "> cité");
    assert_eq!(typed("- un"), "- un");
    assert_eq!(typed("+ un"), "- un");
    assert_eq!(typed("3. trois"), "3. trois");
    assert_eq!(typed("[x] fait"), "- [x] fait");
    assert_eq!(typed("[ ] à faire"), "- [ ] à faire");
    assert_eq!(typed("```sql code"), "```sql\ncode\n```");
    assert_eq!(typed("---"), "---");
    assert_eq!(typed("####### sept"), "####### sept", "seven is no heading");
}

#[test]
fn marks_close_at_the_caret_and_spaced_emphasis_stays_as_typed() {
    assert_eq!(typed("du **gras** ici"), "du **gras** ici");
    assert_eq!(typed("un *mot*"), "un *mot*");
    assert_eq!(typed("un _mot_"), "un *mot*");
    assert_eq!(typed("du ~~barré~~"), "du ~~barré~~");
    assert_eq!(typed("du `code`"), "du `code`");
    assert_eq!(typed("2 * 3 * 4"), "2 * 3 * 4");
    assert_eq!(
        typed("snake_case_name"),
        "snake_case_name",
        "no italic inside a word"
    );
}

#[test]
fn every_input_rule_of_the_reference_has_its_port() {
    let corpus: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../golden/rich-keys/cases.json")).unwrap();
    // `image` turns `![alt](src)` into a picture: a Note's pictures are its drawings, embedded
    // from the toolbar (#1215).
    let ported = [
        "heading",
        "blockquote",
        "bulletList",
        "orderedList",
        "taskItem",
        "codeBlock",
        "horizontalRule",
        "bold",
        "italic",
        "strike",
        "code",
        "spacedEmphasis",
        "image",
    ];
    for case in &corpus {
        let rules = case["output"]["inputRules"].as_array().unwrap();
        let name = case["input"].as_str().unwrap();
        assert!(
            rules.is_empty() || ported.contains(&name),
            "{name} has rules of its own"
        );
    }
}
