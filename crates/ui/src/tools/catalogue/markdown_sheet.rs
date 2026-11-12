//! Markdown: the syntax and its examples here, what each does in the reference's words.

use i18n::{Language, keys};
use serde_json::Value;

use super::reference::{Reference, Table, columns_of, facts, groups_of, text, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, ReferenceRow};

const GROUPS: [&str; 7] = [
    "headings", "emphasis", "lists", "links", "code", "blocks", "raw",
];
const WIDE_CARD: f32 = 760.0;
/// A space that matters is drawn so, and copied as a space.
const SPACE: char = '␣';

/// `github` is GitHub Flavored Markdown, and what github.com adds to it: footnotes, emoji.
const ENTRIES: &str = include_str!("facts/markdown-sheet.json");

fn works(yes: bool) -> Cell {
    if yes {
        Cell::checked()
    } else {
        Cell::text("—")
    }
}

fn row(entry: &Value, language: Language) -> ReferenceRow {
    let (syntax, group) = (text(entry, "syntax"), text(entry, "group"));
    let what = word(
        language,
        &format!("tools.markdown-sheet.words.entries.{syntax}"),
    );
    let heading = word(language, &format!("tools.markdown-sheet.groups.{group}"));
    let cells = vec![
        Cell::mono(syntax),
        Cell::text(what.clone()),
        Cell::mono(text(entry, "example")),
        works(entry["commonmark"] == true),
        works(entry["github"] == true),
    ];
    let mut row = ReferenceRow::new(syntax, group, cells)
        .searched_by([syntax, &what, &heading])
        .found_as(syntax, what);
    if syntax.contains(SPACE) {
        row.copy = Some((syntax.replace(SPACE, " "), syntax.to_owned()));
    }
    row
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: facts(ENTRIES).iter().map(|it| row(it, language)).collect(),
        groups: groups_of(language, "tools.markdown-sheet.groups", &GROUPS),
        columns: columns_of(
            language,
            "tools.markdown-sheet.columns",
            &[
                ("syntax", Some(0.15)),
                ("what", None),
                ("example", Some(0.3)),
                ("commonmark", Some(0.13)),
                ("github", Some(0.1)),
            ],
        ),
        caption: keys::tools::markdown_sheet::NAME,
        placeholder: keys::tools::markdown_sheet::PLACEHOLDER,
        card_width: WIDE_CARD,
        matches: None,
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}
