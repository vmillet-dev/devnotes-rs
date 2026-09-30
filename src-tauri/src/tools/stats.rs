//! A text counted as a reader counts it and as a program does, and the characters it is made of,
//! the invisible ones named.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::Type;
use unicode_segmentation::UnicodeSegmentation;

use crate::count::saturating_u32;

pub const MAX_FREQUENCIES: usize = 300;

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StatsRequest {
    pub text: String,
    /// `A` and `a` counted as one.
    pub fold_case: bool,
    pub count_whitespace: bool,
}

/// What a reader cannot see, or would take for something else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Invisible {
    Space,
    Tab,
    LineBreak,
    NoBreakSpace,
    NarrowNoBreakSpace,
    OtherSpace,
    ZeroWidthSpace,
    ZeroWidthJoiner,
    ZeroWidthNonJoiner,
    ByteOrderMark,
    SoftHyphen,
    Control,
}

impl Invisible {
    fn of(grapheme: &str) -> Option<Self> {
        let mut characters = grapheme.chars();
        let first = characters.next()?;
        if grapheme == "\r\n" {
            return Some(Self::LineBreak);
        }
        if characters.next().is_some() {
            return None;
        }
        Some(match first {
            ' ' => Self::Space,
            '\t' => Self::Tab,
            '\n' | '\r' | '\u{2028}' | '\u{2029}' => Self::LineBreak,
            '\u{a0}' => Self::NoBreakSpace,
            '\u{202f}' => Self::NarrowNoBreakSpace,
            '\u{200b}' => Self::ZeroWidthSpace,
            '\u{200d}' => Self::ZeroWidthJoiner,
            '\u{200c}' => Self::ZeroWidthNonJoiner,
            '\u{feff}' => Self::ByteOrderMark,
            '\u{ad}' => Self::SoftHyphen,
            other if other.is_whitespace() => Self::OtherSpace,
            other if other.is_control() => Self::Control,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Frequency {
    pub character: String,
    /// `U+0065 U+0301`: every code point of a character written with several.
    pub code_points: String,
    pub invisible: Option<Invisible>,
    pub count: u32,
    /// Of the characters counted, in percent.
    pub share: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextStats {
    /// As a reader counts them: `é` written as `e` and an accent is one, and so is a flag.
    pub characters: u32,
    pub code_points: u32,
    /// What JavaScript's `length` counts, and many a field limit.
    pub utf16_units: u32,
    pub utf8_bytes: u32,
    pub non_whitespace: u32,
    pub words: u32,
    pub lines: u32,
    pub non_empty_lines: u32,
    pub paragraphs: u32,
    pub distinct: u32,
    pub frequencies: Vec<Frequency>,
    pub frequencies_truncated: bool,
}

fn is_blank(grapheme: &str) -> bool {
    grapheme.chars().all(char::is_whitespace)
}

pub fn count(request: &StatsRequest) -> TextStats {
    let text = request.text.as_str();
    let graphemes: Vec<&str> = text.graphemes(true).collect();

    let lines: Vec<&str> = text.lines().collect();
    let filled = |line: &&str| !line.trim().is_empty();
    let paragraphs = lines
        .iter()
        .zip(std::iter::once(&"").chain(lines.iter()))
        .filter(|(line, before)| filled(line) && !filled(before))
        .count();

    let mut counts: HashMap<String, u32> = HashMap::new();
    let mut counted = 0u32;
    for grapheme in &graphemes {
        if !request.count_whitespace && is_blank(grapheme) {
            continue;
        }
        let key = if request.fold_case {
            grapheme.to_lowercase()
        } else {
            (*grapheme).to_owned()
        };
        *counts.entry(key).or_default() += 1;
        counted += 1;
    }
    let distinct = counts.len();
    let mut frequencies: Vec<(String, u32)> = counts.into_iter().collect();
    frequencies.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let truncated = frequencies.len() > MAX_FREQUENCIES;
    frequencies.truncate(MAX_FREQUENCIES);

    TextStats {
        characters: saturating_u32(graphemes.len()),
        code_points: saturating_u32(text.chars().count()),
        utf16_units: saturating_u32(text.encode_utf16().count()),
        utf8_bytes: saturating_u32(text.len()),
        non_whitespace: saturating_u32(graphemes.iter().filter(|g| !is_blank(g)).count()),
        words: saturating_u32(text.unicode_words().count()),
        lines: saturating_u32(lines.len()),
        non_empty_lines: saturating_u32(lines.iter().filter(|line| filled(line)).count()),
        paragraphs: saturating_u32(paragraphs),
        distinct: saturating_u32(distinct),
        frequencies: frequencies
            .into_iter()
            .map(|(character, count)| Frequency {
                code_points: character
                    .chars()
                    .map(|c| format!("U+{:04X}", u32::from(c)))
                    .collect::<Vec<_>>()
                    .join(" "),
                invisible: Invisible::of(&character),
                share: f64::from(count) * 100.0 / f64::from(counted.max(1)),
                character,
                count,
            })
            .collect(),
        frequencies_truncated: truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(text: &str) -> TextStats {
        count(&StatsRequest {
            text: text.to_owned(),
            fold_case: false,
            count_whitespace: false,
        })
    }

    #[test]
    fn a_character_is_what_a_reader_sees() {
        let composed = stats("été");
        let decomposed = stats("e\u{301}te\u{301}");
        let flag = stats("🇫🇷");

        assert_eq!(
            (
                composed.characters,
                composed.code_points,
                composed.utf16_units,
                composed.utf8_bytes
            ),
            (3, 3, 3, 5)
        );
        assert_eq!(
            (
                decomposed.characters,
                decomposed.code_points,
                decomposed.utf8_bytes
            ),
            (3, 5, 7)
        );
        assert_eq!(
            (
                flag.characters,
                flag.code_points,
                flag.utf16_units,
                flag.utf8_bytes
            ),
            (1, 2, 4, 8)
        );
        assert_eq!(stats("👍🏽").characters, 1);
    }

    #[test]
    fn words_follow_unicode_boundaries() {
        assert_eq!(stats("l’été, don't stop — 42 fois").words, 5);
        assert_eq!(stats("snake_case kebab-case").words, 3);
        assert_eq!(stats("  ").words, 0);
    }

    #[test]
    fn lines_non_empty_lines_and_paragraphs() {
        let text = stats("Premier.\r\nSuite.\r\n\r\n  \nSecond.\nFin");

        assert_eq!(
            (text.lines, text.non_empty_lines, text.paragraphs),
            (6, 4, 2)
        );
        assert_eq!(stats("sans saut final").lines, 1);
        assert_eq!(stats("avec saut final\n").lines, 1);
        let empty = stats("");
        assert_eq!(
            (empty.characters, empty.words, empty.lines, empty.paragraphs),
            (0, 0, 0, 0)
        );
        assert!(empty.frequencies.is_empty());
    }

    #[test]
    fn whitespace_is_left_out_of_the_characters_counted_apart() {
        let text = stats("a b\tc\n");

        assert_eq!((text.characters, text.non_whitespace), (6, 3));
    }

    #[test]
    fn frequencies_most_frequent_first_folded_or_not() {
        let plain = stats("Abba");
        let rows: Vec<(&str, u32)> = plain
            .frequencies
            .iter()
            .map(|f| (f.character.as_str(), f.count))
            .collect();
        assert_eq!(rows, [("b", 2), ("A", 1), ("a", 1)]);
        assert!((plain.frequencies[0].share - 50.0).abs() < 1e-9);

        let folded = count(&StatsRequest {
            text: "Abba".into(),
            fold_case: true,
            count_whitespace: false,
        });
        assert_eq!(folded.frequencies[0].count, 2);
        assert_eq!(folded.distinct, 2);
    }

    #[test]
    fn invisible_characters_are_named_with_their_code_points() {
        let text = count(&StatsRequest {
            text: "a\u{a0}b\u{200b}c d\te\r\n\u{feff}e\u{301}".into(),
            fold_case: false,
            count_whitespace: true,
        });
        let named = |invisible: Invisible| {
            text.frequencies
                .iter()
                .find(|f| f.invisible == Some(invisible))
                .map(|f| f.code_points.clone())
        };

        assert_eq!(named(Invisible::NoBreakSpace).as_deref(), Some("U+00A0"));
        assert_eq!(named(Invisible::ZeroWidthSpace).as_deref(), Some("U+200B"));
        assert_eq!(named(Invisible::ByteOrderMark).as_deref(), Some("U+FEFF"));
        assert_eq!(named(Invisible::Space).as_deref(), Some("U+0020"));
        assert_eq!(named(Invisible::Tab).as_deref(), Some("U+0009"));
        assert_eq!(
            named(Invisible::LineBreak).as_deref(),
            Some("U+000D U+000A")
        );
        assert!(
            text.frequencies
                .iter()
                .any(|f| f.code_points == "U+0065 U+0301" && f.invisible.is_none())
        );
        assert!(
            stats("a b")
                .frequencies
                .iter()
                .all(|f| f.invisible.is_none())
        );
    }

    #[test]
    fn a_long_list_of_characters_is_cut_and_says_so() {
        let text: String = (0..400u32)
            .filter_map(|n| char::from_u32(0x4E00 + n))
            .collect();
        let many = stats(&text);

        assert_eq!(many.distinct, 400);
        assert_eq!(many.frequencies.len(), MAX_FREQUENCIES);
        assert!(many.frequencies_truncated);
    }
}
