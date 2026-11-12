//! The regular expressions: the syntax and its examples here, the rest in the reference's words.

use i18n::{Language, keys};
use serde_json::Value;

use super::reference::{Reference, Table, columns_of, facts, groups_of, text, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, ReferenceRow};

const GROUPS: [&str; 6] = [
    "anchors",
    "classes",
    "quantifiers",
    "groups",
    "flags",
    "escapes",
];
const WIDE_CARD: f32 = 760.0;
const METACHARACTERS: &str = r"\^$.*+?()[]{}|";

/// A syntax as it is written, unique: a flag is its letter. `␊` in a text stands for a line break.
const ENTRIES: &str = include_str!("facts/regex-sheet.json");

/// A query written like a pattern is looked for in the syntax, case and all: `\b` is not `\B`.
/// Words — « lookbehind », « paresseux » — are searched as everywhere else.
fn matches_regex(row: &ReferenceRow, query: &str) -> bool {
    let wanted = query.trim();
    if wanted.contains(|it| METACHARACTERS.contains(it)) {
        return row.key.contains(wanted);
    }
    row.matches(&crate::api::tools::fold(wanted))
}

/// A note is the condition the words spell out: a flag it needs, a limit.
fn support_cell(supported: bool, note: &str) -> Cell {
    if !supported {
        return Cell::text("—");
    }
    if note.is_empty() {
        Cell::checked()
    } else {
        Cell::checked().below(note)
    }
}

fn example(entry: &Value, language: Language) -> Cell {
    use keys::tools::regex_sheet as words;
    let on = i18n::translate(language, words::ON, &[]);
    let found: Vec<&str> = entry["matched"]
        .as_array()
        .map_or_else(Vec::new, |it| it.iter().filter_map(Value::as_str).collect());
    let matched = if found.is_empty() {
        i18n::translate(language, words::NO_MATCH, &[])
    } else {
        found.join("  ")
    };
    Cell::mono(text(entry, "pattern"))
        .below(format!("{on} {} →", text(entry, "text")))
        .result(matched)
}

fn row(entry: &Value, language: Language) -> ReferenceRow {
    let (syntax, group) = (text(entry, "syntax"), text(entry, "group"));
    let said = |which: &str| {
        let path = format!("tools.regex-sheet.words.entries.{syntax}.{which}");
        word(language, &path)
    };
    let (what, js, pcre) = (said("what"), said("js"), said("pcre"));
    let heading = word(language, &format!("tools.regex-sheet.groups.{group}"));
    let cells = vec![
        Cell::mono(syntax),
        Cell::text(what.clone()),
        example(entry, language),
        support_cell(entry["js"] == true, &js),
        support_cell(entry["pcre"] == true, &pcre),
    ];
    ReferenceRow::new(syntax, group, cells)
        .searched_by([syntax, &what, &js, &pcre, &heading])
        .found_as(syntax, what)
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: facts(ENTRIES).iter().map(|it| row(it, language)).collect(),
        groups: groups_of(language, "tools.regex-sheet.groups", &GROUPS),
        columns: columns_of(
            language,
            "tools.regex-sheet.columns",
            &[
                ("syntax", Some(0.18)),
                ("what", None),
                ("example", Some(0.31)),
                ("js", Some(0.12)),
                ("pcre", Some(0.12)),
            ],
        ),
        caption: keys::tools::regex_sheet::NAME,
        placeholder: keys::tools::regex_sheet::PLACEHOLDER,
        card_width: WIDE_CARD,
        matches: Some(matches_regex),
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_written_like_a_pattern_is_case_sensitive_syntax() {
        let table = table(Language::En);
        let found = |query: &str| {
            let rows = table.rows.iter();
            rows.filter(|row| matches_regex(row, query))
                .map(|row| row.key.as_str())
                .collect::<Vec<_>>()
        };

        assert!(found(r"\b").contains(&r"\b"));
        assert!(!found(r"\b").contains(&r"\B"));
        assert!(found("lookbehind").len() >= 2);
    }
}
