//! The regular expressions: the syntax and its examples here, the rest in the reference's words.

use i18n::{Language, keys};

use super::reference::{Reference, Table, columns_of, groups_of, word};
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

/// `Note`: supported, with a condition the words spell out.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Support {
    Yes,
    No,
    Note,
}

use Support::{No, Note, Yes};

struct Entry {
    /// As it is written, and what the row copies. Unique: a flag is its letter.
    syntax: &'static str,
    group: &'static str,
    js: Support,
    pcre: Support,
    pattern: &'static str,
    /// `␊` stands for a line break.
    text: &'static str,
    matched: &'static [&'static str],
}

const ENTRIES: [Entry; 68] = [
    Entry {
        syntax: "^",
        group: "anchors",
        js: Yes,
        pcre: Yes,
        pattern: "^Bon",
        text: "Bonjour",
        matched: &["Bon"],
    },
    Entry {
        syntax: "$",
        group: "anchors",
        js: Yes,
        pcre: Yes,
        pattern: "\\d+$",
        text: "lot 42",
        matched: &["42"],
    },
    Entry {
        syntax: "\\b",
        group: "anchors",
        js: Yes,
        pcre: Yes,
        pattern: "\\bcat\\b",
        text: "cat scatter",
        matched: &["cat"],
    },
    Entry {
        syntax: "\\B",
        group: "anchors",
        js: Yes,
        pcre: Yes,
        pattern: "\\Bcat",
        text: "cat scatter",
        matched: &["cat"],
    },
    Entry {
        syntax: "\\A",
        group: "anchors",
        js: No,
        pcre: Yes,
        pattern: "\\Aab",
        text: "abab",
        matched: &["ab"],
    },
    Entry {
        syntax: "\\z",
        group: "anchors",
        js: No,
        pcre: Yes,
        pattern: "ab\\z",
        text: "abab",
        matched: &["ab"],
    },
    Entry {
        syntax: "\\Z",
        group: "anchors",
        js: No,
        pcre: Yes,
        pattern: "fin\\Z",
        text: "fin␊",
        matched: &["fin"],
    },
    Entry {
        syntax: "\\G",
        group: "anchors",
        js: No,
        pcre: Yes,
        pattern: "\\G\\d",
        text: "12a3",
        matched: &["1", "2"],
    },
    Entry {
        syntax: ".",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "a.c",
        text: "abc a-c",
        matched: &["abc", "a-c"],
    },
    Entry {
        syntax: "\\d",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "\\d+",
        text: "v12.3",
        matched: &["12", "3"],
    },
    Entry {
        syntax: "\\D",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "\\D+",
        text: "v12",
        matched: &["v"],
    },
    Entry {
        syntax: "\\w",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "\\w+",
        text: "user_id-2",
        matched: &["user_id", "2"],
    },
    Entry {
        syntax: "\\W",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "\\W",
        text: "a-b",
        matched: &["-"],
    },
    Entry {
        syntax: "\\s",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "a\\sb",
        text: "a b",
        matched: &["a b"],
    },
    Entry {
        syntax: "\\S",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "\\S+",
        text: "x = 1",
        matched: &["x", "=", "1"],
    },
    Entry {
        syntax: "[abc]",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "[aeiou]",
        text: "regex",
        matched: &["e", "e"],
    },
    Entry {
        syntax: "[^abc]",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "[^0-9]+",
        text: "a1b2",
        matched: &["a", "b"],
    },
    Entry {
        syntax: "[a-z]",
        group: "classes",
        js: Yes,
        pcre: Yes,
        pattern: "[a-f0-9]+",
        text: "0xff",
        matched: &["0", "ff"],
    },
    Entry {
        syntax: "\\p{L}",
        group: "classes",
        js: Note,
        pcre: Yes,
        pattern: "\\p{L}+",
        text: "été 2026",
        matched: &["été"],
    },
    Entry {
        syntax: "\\P{L}",
        group: "classes",
        js: Note,
        pcre: Yes,
        pattern: "\\P{L}+",
        text: "été 2026",
        matched: &[" 2026"],
    },
    Entry {
        syntax: "[[:alpha:]]",
        group: "classes",
        js: No,
        pcre: Yes,
        pattern: "[[:alpha:]]+",
        text: "abc123",
        matched: &["abc"],
    },
    Entry {
        syntax: "\\h",
        group: "classes",
        js: No,
        pcre: Yes,
        pattern: "a\\hb",
        text: "a b",
        matched: &["a b"],
    },
    Entry {
        syntax: "\\R",
        group: "classes",
        js: No,
        pcre: Yes,
        pattern: "a\\Rb",
        text: "a␍␊b",
        matched: &["a␍␊b"],
    },
    Entry {
        syntax: "\\X",
        group: "classes",
        js: No,
        pcre: Yes,
        pattern: "^\\X$",
        text: "é",
        matched: &["é"],
    },
    Entry {
        syntax: "*",
        group: "quantifiers",
        js: Yes,
        pcre: Yes,
        pattern: "ab*",
        text: "a ab abbb",
        matched: &["a", "ab", "abbb"],
    },
    Entry {
        syntax: "+",
        group: "quantifiers",
        js: Yes,
        pcre: Yes,
        pattern: "ab+",
        text: "a ab abbb",
        matched: &["ab", "abbb"],
    },
    Entry {
        syntax: "?",
        group: "quantifiers",
        js: Yes,
        pcre: Yes,
        pattern: "colou?r",
        text: "color colour",
        matched: &["color", "colour"],
    },
    Entry {
        syntax: "{n}",
        group: "quantifiers",
        js: Yes,
        pcre: Yes,
        pattern: "\\d{4}",
        text: "2026-10",
        matched: &["2026"],
    },
    Entry {
        syntax: "{n,}",
        group: "quantifiers",
        js: Yes,
        pcre: Yes,
        pattern: "\\d{2,}",
        text: "1 22 333",
        matched: &["22", "333"],
    },
    Entry {
        syntax: "{n,m}",
        group: "quantifiers",
        js: Yes,
        pcre: Yes,
        pattern: "\\d{2,3}",
        text: "1 22 4444",
        matched: &["22", "444"],
    },
    Entry {
        syntax: "*?",
        group: "quantifiers",
        js: Yes,
        pcre: Yes,
        pattern: "<.+?>",
        text: "<a><b>",
        matched: &["<a>", "<b>"],
    },
    Entry {
        syntax: "*+",
        group: "quantifiers",
        js: No,
        pcre: Yes,
        pattern: "\\d++\\d",
        text: "123",
        matched: &[],
    },
    Entry {
        syntax: "(…)",
        group: "groups",
        js: Yes,
        pcre: Yes,
        pattern: "(\\d+)-(\\d+)",
        text: "10-20",
        matched: &["10-20"],
    },
    Entry {
        syntax: "(?:…)",
        group: "groups",
        js: Yes,
        pcre: Yes,
        pattern: "(?:ab)+",
        text: "ababx",
        matched: &["abab"],
    },
    Entry {
        syntax: "(?<name>…)",
        group: "groups",
        js: Yes,
        pcre: Yes,
        pattern: "(?<year>\\d{4})",
        text: "en 2026",
        matched: &["2026"],
    },
    Entry {
        syntax: "(?P<name>…)",
        group: "groups",
        js: No,
        pcre: Yes,
        pattern: "(?P<year>\\d{4})",
        text: "en 2026",
        matched: &["2026"],
    },
    Entry {
        syntax: "\\1",
        group: "groups",
        js: Yes,
        pcre: Yes,
        pattern: "(\\w)\\1",
        text: "hello",
        matched: &["ll"],
    },
    Entry {
        syntax: "\\k<name>",
        group: "groups",
        js: Yes,
        pcre: Yes,
        pattern: "(?<c>\\w)\\k<c>",
        text: "hello",
        matched: &["ll"],
    },
    Entry {
        syntax: "a|b",
        group: "groups",
        js: Yes,
        pcre: Yes,
        pattern: "cat|dog",
        text: "a dog",
        matched: &["dog"],
    },
    Entry {
        syntax: "(?=…)",
        group: "groups",
        js: Yes,
        pcre: Yes,
        pattern: "\\d+(?=€)",
        text: "12€ 30$",
        matched: &["12"],
    },
    Entry {
        syntax: "(?!…)",
        group: "groups",
        js: Yes,
        pcre: Yes,
        pattern: "\\b\\d+\\b(?!€)",
        text: "12€ 30$",
        matched: &["30"],
    },
    Entry {
        syntax: "(?<=…)",
        group: "groups",
        js: Yes,
        pcre: Note,
        pattern: "(?<=\\$)\\d+",
        text: "€12 $30",
        matched: &["30"],
    },
    Entry {
        syntax: "(?<!…)",
        group: "groups",
        js: Yes,
        pcre: Note,
        pattern: "(?<!\\$)\\b\\d+",
        text: "$12 30",
        matched: &["30"],
    },
    Entry {
        syntax: "(?>…)",
        group: "groups",
        js: No,
        pcre: Yes,
        pattern: "(?>a+)b",
        text: "aaab",
        matched: &["aaab"],
    },
    Entry {
        syntax: "(?i:…)",
        group: "groups",
        js: Note,
        pcre: Yes,
        pattern: "(?i:abc)D",
        text: "ABCD abcd",
        matched: &["ABCD"],
    },
    Entry {
        syntax: "(?i)",
        group: "groups",
        js: No,
        pcre: Yes,
        pattern: "(?i)abc",
        text: "ABC",
        matched: &["ABC"],
    },
    Entry {
        syntax: "(?R)",
        group: "groups",
        js: No,
        pcre: Yes,
        pattern: "\\((?:[^()]|(?R))*\\)",
        text: "f((a)(b))",
        matched: &["((a)(b))"],
    },
    Entry {
        syntax: "(?#…)",
        group: "groups",
        js: No,
        pcre: Yes,
        pattern: "a(?#comment)b",
        text: "ab",
        matched: &["ab"],
    },
    Entry {
        syntax: "i",
        group: "flags",
        js: Yes,
        pcre: Yes,
        pattern: "/abc/i",
        text: "ABC",
        matched: &["ABC"],
    },
    Entry {
        syntax: "m",
        group: "flags",
        js: Yes,
        pcre: Yes,
        pattern: "/^\\d/m",
        text: "a␊1",
        matched: &["1"],
    },
    Entry {
        syntax: "s",
        group: "flags",
        js: Yes,
        pcre: Yes,
        pattern: "/a.b/s",
        text: "a␊b",
        matched: &["a␊b"],
    },
    Entry {
        syntax: "g",
        group: "flags",
        js: Yes,
        pcre: No,
        pattern: "/\\d/g",
        text: "a1b2",
        matched: &["1", "2"],
    },
    Entry {
        syntax: "u",
        group: "flags",
        js: Yes,
        pcre: Note,
        pattern: "/^.$/u",
        text: "😀",
        matched: &["😀"],
    },
    Entry {
        syntax: "y",
        group: "flags",
        js: Yes,
        pcre: No,
        pattern: "/\\d/y",
        text: "1a2",
        matched: &["1"],
    },
    Entry {
        syntax: "d",
        group: "flags",
        js: Yes,
        pcre: No,
        pattern: "/b(c)/d",
        text: "abc",
        matched: &["bc"],
    },
    Entry {
        syntax: "v",
        group: "flags",
        js: Yes,
        pcre: No,
        pattern: "/[\\p{L}--[a-z]]/v",
        text: "aÉb",
        matched: &["É"],
    },
    Entry {
        syntax: "x",
        group: "flags",
        js: No,
        pcre: Yes,
        pattern: "(?x) \\d+  # digits",
        text: "n° 42",
        matched: &["42"],
    },
    Entry {
        syntax: "n",
        group: "flags",
        js: No,
        pcre: Yes,
        pattern: "(?n)(a)(?<b>b)",
        text: "ab",
        matched: &["ab"],
    },
    Entry {
        syntax: "U",
        group: "flags",
        js: No,
        pcre: Yes,
        pattern: "(?U)a+",
        text: "aaa",
        matched: &["a", "a", "a"],
    },
    Entry {
        syntax: "\\.",
        group: "escapes",
        js: Yes,
        pcre: Yes,
        pattern: "\\d\\.\\d",
        text: "v3.5",
        matched: &["3.5"],
    },
    Entry {
        syntax: "\\t",
        group: "escapes",
        js: Yes,
        pcre: Yes,
        pattern: "a\\tb",
        text: "a⇥b",
        matched: &["a⇥b"],
    },
    Entry {
        syntax: "\\n",
        group: "escapes",
        js: Yes,
        pcre: Yes,
        pattern: "a\\nb",
        text: "a␊b",
        matched: &["a␊b"],
    },
    Entry {
        syntax: "\\xhh",
        group: "escapes",
        js: Yes,
        pcre: Yes,
        pattern: "\\x41",
        text: "ABC",
        matched: &["A"],
    },
    Entry {
        syntax: "\\uhhhh",
        group: "escapes",
        js: Yes,
        pcre: No,
        pattern: "\\u00e9",
        text: "café",
        matched: &["é"],
    },
    Entry {
        syntax: "\\u{h…}",
        group: "escapes",
        js: Note,
        pcre: No,
        pattern: "/\\u{1F600}/u",
        text: "😀",
        matched: &["😀"],
    },
    Entry {
        syntax: "\\x{h…}",
        group: "escapes",
        js: No,
        pcre: Yes,
        pattern: "\\x{1F600}",
        text: "😀",
        matched: &["😀"],
    },
    Entry {
        syntax: "\\Q…\\E",
        group: "escapes",
        js: No,
        pcre: Yes,
        pattern: "\\Q1+1=2\\E",
        text: "1+1=2",
        matched: &["1+1=2"],
    },
    Entry {
        syntax: "\\cX",
        group: "escapes",
        js: Yes,
        pcre: Yes,
        pattern: "\\cJ",
        text: "a␊b",
        matched: &["␊"],
    },
];

/// A query written like a pattern is looked for in the syntax, case and all: `\b` is not `\B`.
/// Words — « lookbehind », « paresseux » — are searched as everywhere else.
fn matches_regex(row: &ReferenceRow, query: &str) -> bool {
    let wanted = query.trim();
    if wanted.contains(|it| METACHARACTERS.contains(it)) {
        return row.key.contains(wanted);
    }
    row.matches(&crate::api::tools::fold(wanted))
}

fn support_cell(support: Support, note: &str) -> Cell {
    if support == No {
        return Cell::text("—");
    }
    if note.is_empty() {
        Cell::checked()
    } else {
        Cell::checked().below(note)
    }
}

fn example(entry: &Entry, language: Language) -> Cell {
    use keys::tools::regex_sheet as words;
    let on = i18n::translate(language, words::ON, &[]);
    let matched = if entry.matched.is_empty() {
        i18n::translate(language, words::NO_MATCH, &[])
    } else {
        entry.matched.join("  ")
    };
    Cell::mono(entry.pattern).below(format!("{on} {} → {matched}", entry.text))
}

fn row(entry: &Entry, language: Language) -> ReferenceRow {
    let syntax = entry.syntax;
    let said = |which: &str| {
        let path = format!("tools.regex-sheet.words.entries.{syntax}.{which}");
        word(language, &path)
    };
    let (what, js, pcre) = (said("what"), said("js"), said("pcre"));
    let heading = word(
        language,
        &format!("tools.regex-sheet.groups.{}", entry.group),
    );
    let cells = vec![
        Cell::mono(syntax),
        Cell::text(what.clone()),
        example(entry, language),
        support_cell(entry.js, &js),
        support_cell(entry.pcre, &pcre),
    ];
    ReferenceRow::new(syntax, entry.group, cells)
        .searched_by([syntax, &what, &js, &pcre, &heading])
        .found_as(syntax, what)
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: ENTRIES.iter().map(|it| row(it, language)).collect(),
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
