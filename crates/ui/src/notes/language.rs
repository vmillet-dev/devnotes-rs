//! What the interface knows of each language, one row each in `languages.json`: the badge's
//! short label, the name in full, the hue of its family, the grammar that colours it, its line
//! comment and its indentation. Names of languages, the same in every locale, as in the reference.

use std::collections::HashMap;
use std::sync::LazyLock;

use engine::notes::language::Language;
use serde::Deserialize;

/// Six hues by family: the label names the language, the colour only groups it at a glance.
/// Blue is typed and read before it runs, amber what a page runs, green what a machine runs,
/// purple data and what shapes it, red configuration, neutral prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Hue {
    Blue,
    Amber,
    Green,
    Purple,
    Red,
    Neutral,
}

#[derive(Deserialize)]
struct Row {
    language: Language,
    label: String,
    name: String,
    hue: Hue,
    /// The grammar `text::highlight` colours it with; none is plain text.
    syntax: Option<String>,
    /// What Ctrl+/ puts before a line; none where the language has no line comment.
    comment: Option<String>,
    /// One level by the language's habit (the reference's `indentUnit` at « language »): four
    /// spaces by their style guides, Go's tab, two spaces otherwise.
    #[serde(default = "two_spaces")]
    indent: String,
}

fn two_spaces() -> String {
    "  ".to_owned()
}

static ROWS: LazyLock<HashMap<Language, Row>> = LazyLock::new(|| {
    let rows: Vec<Row> = serde_json::from_str(include_str!("languages.json")).unwrap_or_default();
    rows.into_iter().map(|row| (row.language, row)).collect()
});

/// Plain text's, for a language without a row: `languages_json_has_a_row_for_every_language`
/// keeps that from shipping.
static PLAIN: LazyLock<Row> = LazyLock::new(|| Row {
    language: Language::Txt,
    label: "TXT".to_owned(),
    name: "Text".to_owned(),
    hue: Hue::Neutral,
    syntax: None,
    comment: None,
    indent: two_spaces(),
});

fn row(language: Language) -> &'static Row {
    ROWS.get(&language).unwrap_or(&PLAIN)
}

/// The badge's, and the menu's: « TS ».
pub fn label(language: Language) -> &'static str {
    &row(language).label
}

/// In full: « TypeScript ».
pub fn name(language: Language) -> &'static str {
    &row(language).name
}

/// The name, where the label does not already say it.
pub fn hint(language: Language) -> &'static str {
    let name = name(language);
    if name.eq_ignore_ascii_case(label(language)) {
        ""
    } else {
        name
    }
}

pub fn hue(language: Language) -> Hue {
    row(language).hue
}

/// The grammar `text::highlight` colours a language with; `None` is plain text.
pub fn syntax(language: Language) -> Option<&'static str> {
    row(language).syntax.as_deref()
}

/// What Ctrl+/ puts before a line; `None` where it has no line comment (HTML, CSS, JSON…).
pub fn line_comment(language: Language) -> Option<&'static str> {
    row(language).comment.as_deref()
}

/// The grammar Prettier reads the language with (the reference's `PRETTIER_PARSERS`); `None`
/// where it does not format it.
pub fn formatter(language: Language) -> Option<text::format::Parser> {
    use Language as L;
    use text::format::Parser;
    match language {
        L::Js | L::Jsx => Some(Parser::Babel),
        L::Ts | L::Tsx => Some(Parser::Typescript),
        L::Json | L::Jsonld => Some(Parser::Json),
        L::Css => Some(Parser::Css),
        L::Scss => Some(Parser::Scss),
        L::Less => Some(Parser::Less),
        L::Html => Some(Parser::Html),
        L::Vue => Some(Parser::Vue),
        L::Angular => Some(Parser::Angular),
        L::Md => Some(Parser::Markdown),
        L::Yml => Some(Parser::Yaml),
        L::Graphql => Some(Parser::Graphql),
        _ => None,
    }
}

/// One level of indentation by the language's habit.
pub fn indent_unit(language: Language) -> &'static str {
    &row(language).indent
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_language_has_a_label_and_a_name_of_its_own() {
        let mut names = HashSet::new();
        for language in Language::ALL {
            assert!(!label(language).is_empty(), "{language:?}");
            assert!(names.insert(name(language)), "{language:?}");
        }
    }

    #[test]
    fn a_language_is_named_and_coloured_by_its_family() {
        assert_eq!(label(Language::Ts), "TS");
        assert_eq!(name(Language::Ts), "TypeScript");
        assert_eq!(hue(Language::Ts), Hue::Blue);
        assert_eq!(hue(Language::Js), Hue::Amber);
    }

    #[test]
    fn every_grammar_named_exists_and_the_common_languages_have_one() {
        text::highlight::warm();
        for language in Language::ALL {
            if let Some(name) = syntax(language) {
                assert!(text::highlight::has_syntax(name), "{language:?}: {name}");
            }
        }
        let common = [
            Language::Json,
            Language::Ts,
            Language::Py,
            Language::Rs,
            Language::Sql,
        ];
        assert!(
            common
                .into_iter()
                .all(|language| syntax(language).is_some())
        );
        assert_eq!(syntax(Language::Txt), None);
    }

    #[test]
    fn languages_json_has_a_row_for_every_language() {
        for language in Language::ALL {
            assert!(ROWS.contains_key(&language), "{language:?}");
        }
        assert_eq!(ROWS.len(), Language::ALL.len());
    }
}
