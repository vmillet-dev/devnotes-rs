//! The words a sheet reads: each concept once, under a neutral name, with its spellings in
//! French and in English. Both languages are read whatever the language on screen, so a sheet
//! keeps its meaning for everyone; the front proposes the spelling of the one on screen.

use serde::Serialize;
use specta::Type;

use super::lexer::{Tok, Token};
use crate::closed_enum::closed_enum;

closed_enum! {
    pub enum CalcWord {
        #[default]
        Convert = "convert",
        At = "at",
        Of = "of",
        From = "from",
        /// The end of `de … à`.
        To = "to",
        Over = "over",
        Excluding = "excluding",
        Total = "total",
        Mean = "mean",
        Today = "today",
        Tomorrow = "tomorrow",
        Yesterday = "yesterday",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WordClass {
    /// Reserved: `en = 5` is an error on its line, never a name.
    Syntax,
    /// A name of the sheet: one defined above hides it from its line down.
    Value,
}

pub struct Spellings {
    pub class: WordClass,
    /// The first of each language is the one the completion proposes.
    pub fr: &'static [&'static str],
    pub en: &'static [&'static str],
}

impl CalcWord {
    pub const fn spellings(self) -> Spellings {
        use WordClass::{Syntax, Value};
        let (class, fr, en): (WordClass, &[&str], &[&str]) = match self {
            Self::Convert => (Syntax, &["en", "vers"], &["in", "to", "as"]),
            Self::At => (Syntax, &["à"], &["at"]),
            Self::Of => (Syntax, &["de", "du", "des"], &["of"]),
            Self::From => (Syntax, &["de"], &["from"]),
            Self::To => (Syntax, &["à"], &["to"]),
            Self::Over => (Syntax, &["sur"], &["over"]),
            Self::Excluding => (
                Syntax,
                &["hors", "ex"],
                &["excluding", "excl", "ex", "without"],
            ),
            Self::Total => (Value, &["total", "somme"], &["total", "sum"]),
            Self::Mean => (Value, &["moyenne"], &["average", "avg", "mean"]),
            Self::Today => (Value, &["aujourd'hui"], &["today"]),
            Self::Tomorrow => (Value, &["demain"], &["tomorrow"]),
            Self::Yesterday => (Value, &["hier"], &["yesterday"]),
        };
        Spellings { class, fr, en }
    }

    fn every_spelling(self) -> impl Iterator<Item = &'static str> {
        let spellings = self.spellings();
        spellings.fr.iter().chain(spellings.en).copied()
    }
}

/// Where the grammar needs one spelling twice; the position decides which concept it is.
pub const SHARED: &[(&str, &[CalcWord])] = &[
    ("de", &[CalcWord::Of, CalcWord::From]),
    ("à", &[CalcWord::At, CalcWord::To]),
    ("to", &[CalcWord::Convert, CalcWord::To]),
];

/// `’` is the apostrophe a keyboard often types: `aujourd’hui` is `aujourd'hui`.
fn fold(word: &str) -> String {
    word.to_lowercase().replace('’', "'")
}

fn part_matches(part: &str, token: &Token) -> bool {
    match &token.tok {
        Tok::Word(word) => fold(word) == part,
        Tok::Symbol(symbol) => part.chars().eq([*symbol]),
        _ => false,
    }
}

/// How many tokens from `at` spell `word`, its longest spelling first.
pub(super) fn spelled(word: CalcWord, tokens: &[Token], at: usize) -> Option<usize> {
    word.every_spelling()
        .filter_map(|spelling| {
            let parts: Vec<&str> = spelling.split(' ').collect();
            let found = tokens.get(at..at + parts.len())?;
            parts
                .iter()
                .zip(found)
                .all(|(part, token)| part_matches(part, token))
                .then_some(parts.len())
        })
        .max()
}

/// A syntax word, which no name can take.
pub(super) fn reserved(word: &str) -> bool {
    let folded = fold(word);
    CalcWord::ALL.iter().any(|concept| {
        concept.spellings().class == WordClass::Syntax
            && concept.every_spelling().any(|spelling| spelling == folded)
    })
}

/// A concept and its spellings, as the front's completion reads them.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WordEntry {
    pub word: CalcWord,
    pub class: WordClass,
    pub fr: Vec<&'static str>,
    pub en: Vec<&'static str>,
}

pub fn catalogue() -> Vec<WordEntry> {
    CalcWord::ALL
        .iter()
        .map(|&word| {
            let spellings = word.spellings();
            WordEntry {
                word,
                class: spellings.class,
                fr: spellings.fr.to_vec(),
                en: spellings.en.to_vec(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::super::lexer::tokens;
    use super::super::units;
    use super::*;

    #[test]
    fn every_spelling_of_every_concept_is_read_in_each_language() {
        for word in CalcWord::ALL {
            for spelling in word.every_spelling() {
                let line = tokens(spelling).unwrap();
                assert_eq!(
                    spelled(word, &line, 0),
                    Some(line.len()),
                    "{spelling} is not read as {word}"
                );
                let mut letters = spelling.chars();
                let capital: String = letters
                    .next()
                    .into_iter()
                    .flat_map(char::to_uppercase)
                    .chain(letters)
                    .collect();
                assert!(spelled(word, &tokens(&capital).unwrap(), 0).is_some());
            }
        }
        let typed = tokens("aujourd’hui").unwrap();
        assert_eq!(spelled(CalcWord::Today, &typed, 0), Some(1));
    }

    #[test]
    fn a_spelling_names_two_concepts_only_where_it_is_declared() {
        let mut concepts: BTreeMap<&str, BTreeSet<CalcWord>> = BTreeMap::new();
        for word in CalcWord::ALL {
            for spelling in word.every_spelling() {
                concepts.entry(spelling).or_default().insert(word);
            }
        }
        let shared: BTreeMap<&str, BTreeSet<CalcWord>> = concepts
            .into_iter()
            .filter(|(_, words)| words.len() > 1)
            .collect();
        let declared: BTreeMap<&str, BTreeSet<CalcWord>> = SHARED
            .iter()
            .map(|(spelling, words)| (*spelling, words.iter().copied().collect()))
            .collect();
        assert_eq!(shared, declared);
    }

    #[test]
    fn a_spelling_is_written_folded_and_one_space_apart() {
        for word in CalcWord::ALL {
            for spelling in word.every_spelling() {
                assert_eq!(fold(spelling), spelling, "{spelling}");
                assert!(!spelling.contains("  ") && spelling.trim() == spelling);
            }
        }
    }

    #[test]
    fn a_word_that_is_also_a_unit_is_told_apart_by_where_it_stands() {
        let symbols: BTreeSet<&str> = units::table()
            .iter()
            .flat_map(|unit| unit.symbols.iter().copied())
            .collect();
        let both: Vec<&str> = CalcWord::ALL
            .iter()
            .flat_map(|word| word.every_spelling())
            .filter(|spelling| symbols.contains(spelling))
            .collect();
        assert_eq!(both, ["in"]);
    }

    #[test]
    fn only_syntax_words_are_reserved() {
        assert!(reserved("en"));
        assert!(reserved("De"));
        assert!(!reserved("total"));
        assert!(!reserved("aujourd'hui"));
        assert!(!reserved("prix"));
    }
}
