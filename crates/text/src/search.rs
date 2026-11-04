//! What the search bar looks for (the reference's `SearchSpec` over `@codemirror/search`): a
//! text or a regular expression, matching case or not, whole words or not. Matches are `char`
//! ranges.

use std::ops::Range;

use regex::{Regex, RegexBuilder};

/// Past this many, the count stops: typing in the field stays smooth on a 2 MB snippet.
pub const COUNT_CAP: usize = 1000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub search: String,
    pub replace: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regexp: bool,
}

/// How many matches, and which one the selection is on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Count {
    pub total: usize,
    /// One-based; `None` while the selection is on none of them.
    pub current: Option<usize>,
    pub capped: bool,
}

impl Query {
    /// `None` for nothing to look for, or a pattern that does not read.
    fn matcher(&self) -> Option<Regex> {
        if self.search.is_empty() {
            return None;
        }
        let pattern = if self.regexp {
            self.search.clone()
        } else {
            regex::escape(&self.search)
        };
        RegexBuilder::new(&pattern)
            .case_insensitive(!self.case_sensitive)
            .multi_line(true)
            .build()
            .ok()
    }

    /// Whether the pattern reads; an empty search is valid and finds nothing.
    pub fn is_valid(&self) -> bool {
        self.search.is_empty() || self.matcher().is_some()
    }

    /// What replaces `found`: `$&` and `$1` name the match and its groups in a regular
    /// expression, as they did in the reference; a text replaces as it is written.
    pub(crate) fn replacement(&self, found: &str) -> String {
        if !self.regexp {
            return self.replace.clone();
        }
        let Some(matcher) = self.matcher() else {
            return self.replace.clone();
        };
        let Some(captures) = matcher.captures(found) else {
            return self.replace.clone();
        };
        let template = self.replace.replace("$&", "${0}");
        let mut out = String::new();
        captures.expand(&template, &mut out);
        out
    }
}

/// Every match of `query` in `text`, up to `cap`.
pub fn find(text: &str, query: &Query, cap: usize) -> Vec<Range<usize>> {
    let Some(matcher) = query.matcher() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    // Byte offsets walked once into char offsets, in order.
    let (mut byte, mut chars) = (0, 0);
    let mut to_char = |at: usize| {
        chars += text[byte..at].chars().count();
        byte = at;
        chars
    };
    for hit in matcher.find_iter(text) {
        if hit.is_empty() {
            continue;
        }
        if query.whole_word && !whole_word(text, hit.start(), hit.end()) {
            continue;
        }
        let start = to_char(hit.start());
        let end = to_char(hit.end());
        found.push(start..end);
        if found.len() >= cap {
            break;
        }
    }
    found
}

fn whole_word(text: &str, start: usize, end: usize) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let before = text[..start].chars().next_back().is_none_or(|c| !word(c));
    let after = text[end..].chars().next().is_none_or(|c| !word(c));
    before && after
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(search: &str) -> Query {
        Query {
            search: search.into(),
            ..Query::default()
        }
    }

    #[test]
    fn a_text_is_found_ignoring_case_unless_asked() {
        assert_eq!(find("Host host", &query("host"), COUNT_CAP), [0..4, 5..9]);
        let exact = Query {
            case_sensitive: true,
            ..query("host")
        };
        assert_eq!(find("Host host", &exact, COUNT_CAP), vec![5..9]);
    }

    #[test]
    fn whole_words_skip_a_match_inside_a_word() {
        let whole = Query {
            whole_word: true,
            ..query("id")
        };
        assert_eq!(
            find("id user_id idx (id)", &whole, COUNT_CAP),
            [0..2, 16..18]
        );
    }

    #[test]
    fn a_regular_expression_reads_its_metacharacters_and_a_text_does_not() {
        let pattern = Query {
            regexp: true,
            ..query(r"\d+")
        };
        assert_eq!(find("a 12 b 345", &pattern, COUNT_CAP), [2..4, 7..10]);
        assert_eq!(find("a.b axb", &query("a.b"), COUNT_CAP), vec![0..3]);
        assert!(
            !Query {
                regexp: true,
                ..query("(")
            }
            .is_valid()
        );
    }

    #[test]
    fn ranges_are_characters_and_the_cap_stops_the_count() {
        assert_eq!(find("éé x", &query("x"), COUNT_CAP), vec![3..4]);
        assert_eq!(find("aaaa", &query("a"), 2).len(), 2);
    }

    #[test]
    fn a_replacement_names_the_groups_in_a_regular_expression_only() {
        let swap = Query {
            regexp: true,
            replace: "$2=$1 ($&)".into(),
            ..query(r"(\w+):(\w+)")
        };
        assert_eq!(swap.replacement("port:5432"), "5432=port (port:5432)");
        let plain = Query {
            replace: "$1".into(),
            ..query("a")
        };
        assert_eq!(plain.replacement("a"), "$1");
    }
}
