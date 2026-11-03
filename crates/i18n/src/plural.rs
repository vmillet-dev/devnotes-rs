//! Counting, in the one ICU shape the translations use:
//! `{name, plural, =0 {…} one {# thing} other {# things}}` — `plural`, `=N`, the CLDR categories
//! of French and English, and `#`; no `select`, no nesting. The reference's transpiler, ported.

use crate::{Language, Value};

/// Every `{name, plural, …}` block of `source` replaced by the branch its count chooses. An
/// unbalanced block is left as written: half a sentence would be worse than its source, which
/// is at least visibly wrong.
pub(crate) fn expand(source: &str, params: &[(&str, Value)], language: Language) -> String {
    let mut out = String::new();
    let mut from = 0;
    while let Some((start, name, branches_from)) = next_head(source, from) {
        let Some(closing) = matching_brace(source, start) else {
            break;
        };
        let count = params
            .iter()
            .find(|(param, _)| *param == name)
            .and_then(|(_, value)| value.as_number());
        out.push_str(&source[from..start]);
        out.push_str(&choose(&source[branches_from..closing], count, language));
        from = closing + 1;
    }
    out.push_str(&source[from..]);
    out
}

/// The next `{name, plural,` at or after `from`: where it opens, the name, where its branches
/// start.
fn next_head(source: &str, from: usize) -> Option<(usize, &str, usize)> {
    let mut at = from;
    while let Some(offset) = source[at..].find('{') {
        let start = at + offset;
        if let Some((name, branches_from)) = head_at(source, start) {
            return Some((start, name, branches_from));
        }
        at = start + 1;
    }
    None
}

fn head_at(source: &str, start: usize) -> Option<(&str, usize)> {
    let rest = &source[start + 1..];
    let first = rest.find(',')?;
    let name = rest[..first].trim();
    let named = !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    let after = &rest[first + 1..];
    let second = after.find(',')?;
    (named && after[..second].trim() == "plural").then_some((name, start + first + second + 3))
}

fn matching_brace(source: &str, start: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (at, character) in source[start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + at);
                }
            }
            _ => {}
        }
    }
    None
}

/// An exact `=N` wins over a category: French calls zero `one`, so « 0 note » would read as a
/// singular where the sentence wants « aucune note ».
fn choose(branches: &str, count: Option<f64>, language: Language) -> String {
    let cases = parse_branches(branches);
    let pick = |selector: &str| {
        cases
            .iter()
            .find(|(case, _)| *case == selector)
            .map(|(_, text)| *text)
    };
    let exact = count.map(|count| format!("={}", number(count)));
    let category = count.map_or("other", |count| category(count, language));
    let chosen = exact
        .as_deref()
        .and_then(pick)
        .or_else(|| pick(category))
        .or_else(|| pick("other"))
        .unwrap_or_default();
    chosen.replace('#', &count.map(number).unwrap_or_default())
}

/// `selector {text}` pairs, in order, the braces inside each text left alone.
fn parse_branches(branches: &str) -> Vec<(&str, &str)> {
    let mut cases = Vec::new();
    let mut at = 0;
    while let Some(offset) = branches[at..].find('{') {
        let open = at + offset;
        let Some(closing) = matching_brace(branches, open) else {
            break;
        };
        let selector = branches[at..open].trim();
        if !selector.is_empty() {
            cases.push((selector, &branches[open + 1..closing]));
        }
        at = closing + 1;
    }
    cases
}

/// The CLDR plural category, as `Intl.PluralRules` gives it for the two languages.
pub(crate) fn category(count: f64, language: Language) -> &'static str {
    let integer = count.abs().trunc();
    let whole = count.fract().abs() < f64::EPSILON;
    match language {
        Language::En if whole && (integer - 1.0).abs() < f64::EPSILON => "one",
        // `i = 0,1`: the integer part decides, so 1.5 is `one` too.
        Language::Fr if integer <= 1.0 => "one",
        Language::Fr if whole && (integer % 1_000_000.0).abs() < f64::EPSILON => "many",
        Language::En | Language::Fr => "other",
    }
}

/// As JavaScript's `String(count)` writes it: `2`, `1.5`, `1000000`.
pub(crate) fn number(count: f64) -> String {
    format!("{count}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTES: &str = "{count, plural, =0 {aucune note} one {# note} other {# notes}}";

    fn of(source: &str, count: f64, language: Language) -> String {
        expand(source, &[("count", Value::Number(count))], language)
    }

    #[test]
    fn an_exact_case_wins_over_a_category() {
        assert_eq!(of(NOTES, 0.0, Language::Fr), "aucune note");
        assert_eq!(of(NOTES, 1.0, Language::Fr), "1 note");
        assert_eq!(of(NOTES, 1.5, Language::Fr), "1.5 note");
        assert_eq!(of(NOTES, 21.0, Language::Fr), "21 notes");
    }

    #[test]
    fn english_and_french_count_apart() {
        let source = "{n, plural, one {# day} many {# of days} other {# days}}";
        let read = |count, language| expand(source, &[("n", Value::Number(count))], language);

        assert_eq!(read(1.5, Language::En), "1.5 days");
        assert_eq!(read(1_000_000.0, Language::En), "1000000 days");
        assert_eq!(read(1_000_000.0, Language::Fr), "1000000 of days");
        assert_eq!(read(0.0, Language::En), "0 days");
    }

    #[test]
    fn text_around_and_between_blocks_is_kept() {
        let source =
            "Il reste {a, plural, one {# fichier} other {# fichiers}} sur {b, plural, other {#}}.";
        let params = [("a", Value::Number(2.0)), ("b", Value::Number(9.0))];

        assert_eq!(
            expand(source, &params, Language::Fr),
            "Il reste 2 fichiers sur 9."
        );
    }

    #[test]
    fn a_missing_count_takes_other_and_leaves_hash_empty() {
        assert_eq!(expand(NOTES, &[], Language::En), " notes");
    }

    #[test]
    fn an_unbalanced_block_is_left_as_written() {
        let source = "{count, plural, one {# note}";

        assert_eq!(of(source, 1.0, Language::Fr), source);
    }

    #[test]
    fn a_plain_brace_is_not_a_block() {
        assert_eq!(
            of("{ not, a block }", 1.0, Language::En),
            "{ not, a block }"
        );
    }
}
