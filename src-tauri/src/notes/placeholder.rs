//! What counts as a `{{field}}`, and how one is filled.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

const OPEN: &str = "{{";
const CLOSE: &str = "}}";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Placeholder {
    pub name: String,
    pub default_value: String,
    /// A value gone orphan — its token renamed in the text — stays out of sight without
    /// being erased.
    pub value: String,
}

/// The rule of [`is_field_name`] as a pattern, which crosses as a constant: the front end
/// refuses a name in the same terms without keeping a copy. ⚠️ Rust checks characters rather
/// than matching it, so only `the_pattern_and_the_rule_agree` keeps the two in step.
pub(crate) const FIELD_NAME_PATTERN: &str = "^[A-Za-z0-9_-]+$";

/// Restricted to `[A-Za-z0-9_-]` on purpose: without it a note holding Angular template
/// code (`{{ user.name }}`) would demand a form on every copy.
fn is_field_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn parse_token(inner: &str) -> Option<Placeholder> {
    let (name, default_value) = match inner.split_once('=') {
        Some((name, value)) => (name.trim(), value.trim()),
        None => (inner.trim(), ""),
    };

    if !is_field_name(name) {
        return None;
    }

    Some(Placeholder {
        name: name.to_string(),
        default_value: default_value.to_string(),
        value: String::new(),
    })
}

enum Fragment<'a> {
    Literal(&'a str),
    Token {
        inner: &'a str,
        /// `None` when it is not a field: part of the text, not of the form.
        placeholder: Option<Placeholder>,
    },
}

/// One walk, so [`parse`] and [`fill`] share one idea of where a token starts and ends. An
/// unterminated `{{` ends it, and the rest comes back as one literal.
fn scan(content: &str, mut on_fragment: impl FnMut(Fragment<'_>)) {
    let mut rest = content;

    while let Some(start) = rest.find(OPEN) {
        let after_open = &rest[start + OPEN.len()..];
        let Some(end) = after_open.find(CLOSE) else {
            break;
        };

        on_fragment(Fragment::Literal(&rest[..start]));

        let inner = &after_open[..end];
        on_fragment(Fragment::Token {
            inner,
            placeholder: parse_token(inner),
        });

        rest = &after_open[end + CLOSE.len()..];
    }

    on_fragment(Fragment::Literal(rest));
}

/// The text says which fields exist, never the value map: a value whose token left the text
/// waits for it to come back.
pub fn parse(content: &str, values: &BTreeMap<String, String>) -> Vec<Placeholder> {
    let mut found: Vec<Placeholder> = Vec::new();

    scan(content, |fragment| {
        let Fragment::Token {
            placeholder: Some(mut placeholder),
            ..
        } = fragment
        else {
            return;
        };

        if found.iter().any(|seen| seen.name == placeholder.name) {
            return;
        }

        placeholder.value = values.get(&placeholder.name).cloned().unwrap_or_default();
        found.push(placeholder);
    });

    found
}

/// An empty value is dropped rather than stored: it means "keep what the snippet offers", and
/// a row would freeze that answer the day the default changes.
pub fn normalize_values(values: BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .into_iter()
        .filter(|(name, value)| is_field_name(name) && !value.is_empty())
        .collect()
}

/// Note value, then global variable, then the default written in the text.
pub fn resolve(
    globals: &BTreeMap<String, String>,
    values: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut resolved = globals.clone();

    for (name, value) in values {
        if !value.is_empty() {
            resolved.insert(name.clone(), value.clone());
        }
    }

    resolved
}

/// A token that is not a field is left as it is.
pub fn fill(content: &str, values: &BTreeMap<String, String>) -> String {
    let mut filled = String::with_capacity(content.len());

    scan(content, |fragment| match fragment {
        Fragment::Literal(text) => filled.push_str(text),
        Fragment::Token {
            placeholder: Some(placeholder),
            ..
        } => filled.push_str(
            values
                .get(&placeholder.name)
                .filter(|value| !value.is_empty())
                .unwrap_or(&placeholder.default_value),
        ),
        Fragment::Token {
            inner,
            placeholder: None,
        } => {
            filled.push_str(OPEN);
            filled.push_str(inner);
            filled.push_str(CLOSE);
        }
    });

    filled
}

#[cfg(test)]
mod tests {
    /// Every case that distinguishes the pattern from the character check is listed here:
    /// nothing else stops the two drifting.
    #[test]
    fn the_pattern_and_the_rule_agree() {
        let names = [
            "host",
            "HOST",
            "port_2",
            "a-b",
            "_",
            "-",
            "0",
            "",
            " host",
            "host ",
            "user.name",
            "user name",
            "café",
            "хост",
            "２",
            "host\n",
            "{{host}}",
        ];

        // A hand-rolled reader of the pattern: the crate carries no regex engine.
        let matches = |name: &str| {
            let class = super::FIELD_NAME_PATTERN
                .strip_prefix("^[")
                .and_then(|rest| rest.strip_suffix("]+$"))
                .expect("a character class pattern");

            !name.is_empty()
                && name.chars().all(|c| {
                    (class.contains("A-Z") && c.is_ascii_uppercase())
                        || (class.contains("a-z") && c.is_ascii_lowercase())
                        || (class.contains("0-9") && c.is_ascii_digit())
                        || (class.contains('_') && c == '_')
                        || (class.ends_with('-') && c == '-')
                })
        };

        for name in names {
            assert_eq!(
                super::is_field_name(name),
                matches(name),
                "{name:?} is read differently by the rule and by the pattern"
            );
        }
    }

    use super::*;

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    fn names(content: &str) -> Vec<String> {
        parse(content, &BTreeMap::new())
            .into_iter()
            .map(|placeholder| placeholder.name)
            .collect()
    }

    #[test]
    fn each_field_is_listed_once_in_order_of_appearance() {
        assert_eq!(
            names("psql -h {{host}} -p {{port}} -d {{db}} # {{host}}"),
            ["host", "port", "db"]
        );
    }

    #[test]
    fn a_default_value_is_read_after_the_equals_sign() {
        assert_eq!(
            parse("{{ port = 5432 }}", &BTreeMap::new()),
            [Placeholder {
                name: "port".to_string(),
                default_value: "5432".to_string(),
                value: String::new(),
            }]
        );
    }

    #[test]
    fn a_field_carries_what_was_already_typed_for_it() {
        assert_eq!(
            parse("{{host}}", &values(&[("host", "db.internal")])),
            [Placeholder {
                name: "host".to_string(),
                default_value: String::new(),
                value: "db.internal".to_string(),
            }]
        );
    }

    #[test]
    fn a_value_whose_token_left_the_text_is_not_a_field() {
        let fields = parse("{{hostname}}", &values(&[("host", "db")]));

        assert_eq!(names("{{hostname}}"), ["hostname"]);
        assert!(fields[0].value.is_empty());
    }

    #[test]
    fn only_what_can_designate_a_token_is_kept_for_writing() {
        assert_eq!(
            normalize_values(values(&[
                ("host", "db.internal"),
                ("port", ""),
                ("user.name", "x"),
            ])),
            values(&[("host", "db.internal")])
        );
    }

    #[test]
    fn a_template_expression_is_not_a_field() {
        assert!(names("<p>{{ user.name }}</p> {{ items[0] }}").is_empty());
    }

    #[test]
    fn an_unterminated_token_ends_the_scan_without_panicking() {
        assert_eq!(names("{{host}} puis {{oops"), ["host"]);
    }

    #[test]
    fn a_global_variable_fills_a_field_the_note_says_nothing_about() {
        let globals = values(&[("host", "db.internal")]);

        assert_eq!(
            fill("{{host}}", &resolve(&globals, &BTreeMap::new())),
            "db.internal"
        );
    }

    #[test]
    fn what_was_typed_on_the_note_wins_over_the_global_variable() {
        let globals = values(&[("host", "db.internal")]);

        assert_eq!(
            fill(
                "{{host}}",
                &resolve(&globals, &values(&[("host", "localhost")]))
            ),
            "localhost"
        );
    }

    #[test]
    fn an_empty_entry_leaves_the_global_variable_in_place() {
        let globals = values(&[("host", "db.internal")]);

        assert_eq!(
            fill("{{host}}", &resolve(&globals, &values(&[("host", "")]))),
            "db.internal"
        );
    }

    #[test]
    fn a_global_variable_wins_over_the_default_written_in_the_text() {
        let globals = values(&[("port", "6543")]);

        assert_eq!(
            fill("{{port=5432}}", &resolve(&globals, &BTreeMap::new())),
            "6543"
        );
    }

    #[test]
    fn filling_substitutes_every_occurrence() {
        assert_eq!(
            fill(
                "{{host}}:{{port}}/{{host}}",
                &values(&[("host", "db"), ("port", "5432")])
            ),
            "db:5432/db"
        );
    }

    #[test]
    fn a_missing_value_falls_back_to_the_default() {
        assert_eq!(fill("{{port=5432}}", &BTreeMap::new()), "5432");
    }

    #[test]
    fn an_empty_value_falls_back_to_the_default_too() {
        assert_eq!(fill("{{port=5432}}", &values(&[("port", "")])), "5432");
    }

    #[test]
    fn a_field_without_value_nor_default_disappears() {
        assert_eq!(fill("a{{x}}b", &BTreeMap::new()), "ab");
    }

    #[test]
    fn what_is_not_a_field_survives_untouched() {
        let template = "<p>{{ user.name }}</p>";

        assert_eq!(fill(template, &values(&[("user", "x")])), template);
    }
}
