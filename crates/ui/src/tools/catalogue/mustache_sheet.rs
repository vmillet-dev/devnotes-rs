//! Mustache: the syntax and its examples here, what each does in the reference's words.

use i18n::{Language, keys};
use serde_json::Value;

use super::reference::{Reference, Table, columns_of, facts, groups_of, text, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, ReferenceRow};

const GROUPS: [&str; 3] = ["values", "sections", "more"];
const WIDE_CARD: f32 = 760.0;

/// `view` is what the template is rendered with: its view as JSON, a partial.
const ENTRIES: &str = include_str!("facts/mustache-sheet.json");

fn row(entry: &Value, language: Language) -> ReferenceRow {
    let (syntax, group) = (text(entry, "syntax"), text(entry, "group"));
    let what = word(
        language,
        &format!("tools.mustache-sheet.words.entries.{syntax}"),
    );
    let heading = word(language, &format!("tools.mustache-sheet.groups.{group}"));
    let with = i18n::translate(language, keys::tools::mustache_sheet::WITH, &[]);
    let example = Cell::mono(text(entry, "template"))
        .below(format!("{with} {} →", text(entry, "view")))
        .result(text(entry, "output"));
    let cells = vec![Cell::mono(syntax), Cell::text(what.clone()), example];
    ReferenceRow::new(syntax, group, cells)
        .searched_by([syntax, &what, &heading])
        .found_as(syntax, what)
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: facts(ENTRIES).iter().map(|it| row(it, language)).collect(),
        groups: groups_of(language, "tools.mustache-sheet.groups", &GROUPS),
        columns: columns_of(
            language,
            "tools.mustache-sheet.columns",
            &[
                ("syntax", Some(0.25)),
                ("what", Some(0.32)),
                ("example", None),
            ],
        ),
        caption: keys::tools::mustache_sheet::NAME,
        placeholder: keys::tools::mustache_sheet::PLACEHOLDER,
        card_width: WIDE_CARD,
        matches: None,
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}
