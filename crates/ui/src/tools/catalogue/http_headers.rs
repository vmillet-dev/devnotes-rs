//! The HTTP headers: the facts here, what each does in the reference's words.

use i18n::{Language, keys};
use serde_json::Value;

use super::reference::{Reference, Table, columns_of, facts, groups_of, text, word};
use crate::tools::Tool;
use crate::tools::ui::{Cell, ReferenceRow};

const GROUPS: [&str; 10] = [
    "content",
    "caching",
    "range",
    "auth",
    "cookies",
    "security",
    "cors",
    "connection",
    "request",
    "response",
];
const WIDE_CARD: f32 = 760.0;

/// A header as it is written on the wire, a cookie attribute named alone; `by` is the RFC or the
/// living standard that defines it, or « de facto ».
const HEADERS: &str = include_str!("facts/http-headers.json");

fn row(header: &Value, language: Language) -> ReferenceRow {
    use keys::tools::http_headers as words;
    let tr = |key| i18n::translate(language, key, &[]);
    let (name, group) = (text(header, "name"), text(header, "group"));
    let (example, by) = (text(header, "example"), text(header, "by"));
    let lower = name.to_lowercase();
    let does = word(
        language,
        &format!("tools.http-headers.words.headers.{lower}"),
    );
    let heading = word(language, &format!("tools.http-headers.groups.{group}"));
    let direction = word(
        language,
        &format!(
            "tools.http-headers.directions.{}",
            text(header, "direction")
        ),
    );
    let tags: Vec<String> = [
        (header["security"] == true).then(|| tr(words::SECURITY)),
        (header["deprecated"] == true).then(|| tr(words::DEPRECATED)),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut named = Cell::mono(name);
    if !tags.is_empty() {
        named = named.below(tags.join(" · "));
    }
    let cells = vec![
        named,
        Cell::text(direction),
        Cell::text(does.clone()).below(example),
        Cell::text(by),
    ];
    ReferenceRow::new(name, group, cells)
        .searched_by([name, &does, example, by, &heading])
        .found_as(name, does)
}

pub(crate) fn table(language: Language) -> Table {
    Table {
        rows: facts(HEADERS).iter().map(|it| row(it, language)).collect(),
        groups: groups_of(language, "tools.http-headers.groups", &GROUPS),
        columns: columns_of(
            language,
            "tools.http-headers.columns",
            &[
                ("name", Some(0.31)),
                ("direction", Some(0.11)),
                ("does", None),
                ("by", Some(0.16)),
            ],
        ),
        caption: keys::tools::http_headers::NAME,
        placeholder: keys::tools::http_headers::PLACEHOLDER,
        card_width: WIDE_CARD,
        matches: None,
    }
}

pub(super) fn make() -> Box<dyn Tool> {
    Reference::of(table)
}
