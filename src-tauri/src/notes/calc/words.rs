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
        Plus = "plus",
        /// An addition between two numbers, a separator in a list of them.
        And = "and",
        Minus = "minus",
        Times = "times",
        Divide = "divide",
        Modulo = "modulo",
        Total = "total",
        Mean = "mean",
        /// The result of the last line above that gave one.
        Previous = "previous",
        Today = "today",
        Tomorrow = "tomorrow",
        Yesterday = "yesterday",
        Kilo = "kilo",
        Mega = "mega",
        Giga = "giga",
        Tera = "tera",
        Thousand = "thousand",
        Million = "million",
        Billion = "billion",
        Trillion = "trillion",
        Ten = "ten",
        Hundred = "hundred",
        Xor = "xor",
        /// `à 2 décimales`, `to 2 dp`.
        Decimals = "decimals",
        /// `à 5 chiffres`, `to 5 digits`: decimal places too.
        Digits = "digits",
        Rounded = "rounded",
        RoundedUp = "roundedUp",
        RoundedDown = "roundedDown",
        /// Opens `au 10 près`, `to nearest 10`.
        Nearest = "nearest",
        /// Closes `au 10 près`.
        Near = "near",
        Largest = "largest",
        Smallest = "smallest",
        Root = "root",
        Cbrt = "cbrt",
        Log = "log",
        Ln = "ln",
        Abs = "abs",
        Round = "round",
        Ceil = "ceil",
        Floor = "floor",
        Hex = "hex",
        Bin = "bin",
        Oct = "oct",
        Int = "int",
        Sci = "sci",
        Base = "base",
        Pi = "pi",
        E = "e",
        Tau = "tau",
        Phi = "phi",
        /// The screen's pixels to an inch, 96 unless a line sets it.
        Ppi = "ppi",
        /// A font size in pixels, 16 unless a line sets it.
        Em = "em",
        Rem = "rem",
        /// `5 % de plus sur 200`, `70 en % de plus que 20`, `5% on 200`.
        PercentOn = "percentOn",
        PercentOff = "percentOff",
        /// The unknown a line solves: `5 % de combien font 6`.
        What = "what",
        Is = "is",
        WhatPercent = "whatPercent",
        WhatTimes = "whatTimes",
        Fraction = "fraction",
        Multiple = "multiple",
        /// `20 % en décimal`: a percentage as the number it is.
        AsDecimal = "asDecimal",
        /// `50 as a % of 100`.
        Article = "article",
        /// `6 est à 60 ce que 8 est à combien`.
        IsTo = "isTo",
        Like = "like",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum WordClass {
    /// Reserved: `en = 5` is an error on its line, never a name.
    Syntax,
    /// A name of the sheet: one defined above hides it from its line down.
    Value,
    /// `ppi = 326`: the sheet's setting from its line down, no name.
    Setting,
}

pub struct Spellings {
    pub class: WordClass,
    /// The first of each language is the one the completion proposes.
    pub fr: &'static [&'static str],
    pub en: &'static [&'static str],
}

impl CalcWord {
    #[allow(
        clippy::too_many_lines,
        reason = "one line per concept: the table is the point"
    )]
    pub const fn spellings(self) -> Spellings {
        use WordClass::{Setting, Syntax, Value};
        let (class, fr, en): (WordClass, &[&str], &[&str]) = match self {
            Self::Convert => (Syntax, &["en", "vers"], &["in", "to", "as"]),
            Self::At => (Syntax, &["à"], &["at"]),
            Self::Of => (Syntax, &["de", "du", "des"], &["of"]),
            Self::From => (Syntax, &["de"], &["from"]),
            Self::To => (Syntax, &["à"], &["to"]),
            Self::Over => (Syntax, &["sur"], &["over"]),
            Self::Excluding => (Syntax, &["hors", "ex"], &["excluding", "excl", "ex"]),
            Self::Plus => (Syntax, &["plus", "avec"], &["plus", "with"]),
            Self::And => (Syntax, &["et"], &["and"]),
            Self::Minus => (
                Syntax,
                &["moins", "sans"],
                &["minus", "without", "subtract"],
            ),
            Self::Times => (
                Syntax,
                &["fois", "multiplié par", "mul"],
                &["times", "multiplied by", "mul"],
            ),
            Self::Divide => (Syntax, &["divisé par"], &["divided by", "divide by"]),
            Self::Modulo => (Syntax, &["mod", "modulo"], &["mod", "modulo"]),
            Self::Total => (Value, &["total", "somme"], &["total", "sum"]),
            Self::Mean => (Value, &["moyenne"], &["average", "avg", "mean"]),
            Self::Previous => (Value, &["préc", "précédent"], &["prev", "previous"]),
            Self::Today => (Value, &["aujourd'hui"], &["today"]),
            Self::Tomorrow => (Value, &["demain"], &["tomorrow"]),
            Self::Yesterday => (Value, &["hier"], &["yesterday"]),
            Self::Kilo => (Value, &["k"], &["k"]),
            Self::Mega => (Value, &["M"], &["M"]),
            Self::Giga => (Value, &["Md"], &["bn"]),
            Self::Tera => (Value, &["T"], &["T", "tn"]),
            Self::Thousand => (
                Value,
                &["mille", "millier", "milliers"],
                &["thousand", "thousands"],
            ),
            Self::Million => (Value, &["million", "millions"], &["million", "millions"]),
            Self::Billion => (Value, &["milliard", "milliards"], &["billion", "billions"]),
            Self::Trillion => (Value, &["mille milliards"], &["trillion", "trillions"]),
            Self::Ten => (Value, &["dizaine", "dizaines"], &["ten", "tens"]),
            Self::Hundred => (Value, &["centaine", "centaines"], &["hundred", "hundreds"]),
            Self::Xor => (Syntax, &["xor"], &["xor"]),
            Self::Decimals => (
                Syntax,
                &["décimales", "décimale"],
                &["dp", "decimals", "decimal"],
            ),
            Self::Digits => (Syntax, &["chiffres", "chiffre"], &["digits", "digit"]),
            Self::Rounded => (Syntax, &["arrondi"], &["rounded"]),
            Self::RoundedUp => (Syntax, &["arrondi supérieur"], &["rounded up"]),
            Self::RoundedDown => (Syntax, &["arrondi inférieur"], &["rounded down"]),
            Self::Nearest => (Syntax, &["au", "à la"], &["to nearest", "to the nearest"]),
            Self::Near => (Syntax, &["près"], &[]),
            Self::Largest => (
                Syntax,
                &["le plus grand de", "plus grand de"],
                &["larger of", "the larger of", "largest of"],
            ),
            Self::Smallest => (
                Syntax,
                &["le plus petit de", "plus petit de"],
                &["smaller of", "the smaller of", "smallest of"],
            ),
            Self::Root => (Value, &["racine", "racine carrée"], &["sqrt", "root"]),
            Self::Cbrt => (Value, &["racine cubique", "cbrt"], &["cbrt"]),
            Self::Log => (Value, &["log"], &["log"]),
            Self::Ln => (Value, &["ln"], &["ln"]),
            Self::Abs => (Value, &["abs", "valeur absolue"], &["abs"]),
            Self::Round => (Value, &["arrondi"], &["round"]),
            Self::Ceil => (Value, &["plafond"], &["ceil"]),
            Self::Floor => (Value, &["plancher"], &["floor"]),
            Self::Hex => (Value, &["hex"], &["hex"]),
            Self::Bin => (Value, &["bin"], &["bin"]),
            Self::Oct => (Value, &["oct"], &["oct"]),
            Self::Int => (Value, &["int"], &["int"]),
            Self::Sci => (Value, &["sci"], &["sci"]),
            Self::Base => (Syntax, &["base"], &["base"]),
            Self::Pi => (Value, &["pi", "π"], &["pi", "π"]),
            Self::E => (Value, &["e"], &["e"]),
            Self::Tau => (Value, &["tau", "τ"], &["tau", "τ"]),
            Self::Phi => (Value, &["phi", "φ"], &["phi", "φ"]),
            Self::Ppi => (Setting, &["ppi"], &["ppi"]),
            Self::Em => (Setting, &["em"], &["em"]),
            Self::Rem => (Setting, &["rem"], &["rem"]),
            Self::PercentOn => (Syntax, &["de plus sur", "de plus que"], &["on"]),
            Self::PercentOff => (Syntax, &["de moins sur", "de moins que"], &["off"]),
            Self::What => (Syntax, &["combien"], &["what"]),
            Self::Is => (Syntax, &["font", "fait"], &["is"]),
            Self::WhatPercent => (
                Syntax,
                &["c'est combien de %", "combien de %"],
                &["is what %"],
            ),
            Self::WhatTimes => (
                Syntax,
                &["c'est combien de fois", "combien de fois"],
                &["is what x", "is what times"],
            ),
            // Values, not syntax: `x` and `a` are names a sheet gives often.
            Self::Fraction => (Value, &["fraction"], &["fraction"]),
            Self::Multiple => (
                Value,
                &["multiple", "multiplicateur"],
                &["multiplier", "multiple", "x"],
            ),
            Self::AsDecimal => (Value, &["décimal"], &["decimal", "dec"]),
            Self::Article => (Value, &[], &["a"]),
            Self::IsTo => (Syntax, &["est à"], &["is to"]),
            Self::Like => (Syntax, &["ce que"], &["as"]),
        };
        Spellings { class, fr, en }
    }

    /// `M` is a million and `m` a metre: a scale's letters are read in their case.
    pub const fn exact(self) -> bool {
        matches!(self, Self::Kilo | Self::Mega | Self::Giga | Self::Tera)
    }

    /// What a scale multiplies by.
    pub fn scale(self) -> Option<u64> {
        Some(match self {
            Self::Kilo | Self::Thousand => 1_000,
            Self::Mega | Self::Million => 1_000_000,
            Self::Giga | Self::Billion => 1_000_000_000,
            Self::Tera | Self::Trillion => 1_000_000_000_000,
            Self::Ten => 10,
            Self::Hundred => 100,
            _ => return None,
        })
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
    // A line ends `arrondi` and a function opens it.
    ("arrondi", &[CalcWord::Rounded, CalcWord::Round]),
    // `5 km as m`, but `6 is to 60 as 8 is to what`.
    ("as", &[CalcWord::Convert, CalcWord::Like]),
    // `to 2 decimal` places, `to decimal` a percentage.
    ("decimal", &[CalcWord::Decimals, CalcWord::AsDecimal]),
];

/// `’` is the apostrophe a keyboard often types: `aujourd’hui` is `aujourd'hui`.
fn fold(word: &str) -> String {
    word.to_lowercase().replace('’', "'")
}

fn part_matches(part: &str, token: &Token, exact: bool) -> bool {
    match &token.tok {
        Tok::Word(word) if exact => word == part,
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
                .all(|(part, token)| part_matches(part, token, word.exact()))
                .then_some(parts.len())
        })
        .max()
}

/// The setting `ppi`, `em` or `rem` a word names, which a line `ppi = …` gives.
pub(super) fn setting(word: &str) -> Option<CalcWord> {
    let folded = fold(word);
    CalcWord::ALL.into_iter().find(|concept| {
        concept.spellings().class == WordClass::Setting
            && concept.every_spelling().any(|spelling| spelling == folded)
    })
}

/// A syntax word, which no name can take.
pub(super) fn reserved(word: &str) -> bool {
    let folded = fold(word);
    CalcWord::ALL.iter().any(|concept| {
        concept.spellings().class == WordClass::Syntax
            && concept.every_spelling().any(|spelling| spelling == folded)
    })
}

/// The scale written at `at`, `k` or `milliard`, and how many tokens it takes.
pub(super) fn scale_at(tokens: &[Token], at: usize) -> Option<(u64, usize)> {
    CalcWord::ALL
        .iter()
        .filter_map(|word| Some((word.scale()?, spelled(*word, tokens, at)?)))
        .max_by_key(|(_, taken)| *taken)
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
                let line = tokens(spelling);
                assert_eq!(
                    spelled(word, &line, 0),
                    Some(line.len()),
                    "{spelling} is not read as {word}"
                );
                if word.exact() {
                    continue;
                }
                let mut letters = spelling.chars();
                let capital: String = letters
                    .next()
                    .into_iter()
                    .flat_map(char::to_uppercase)
                    .chain(letters)
                    .collect();
                assert!(spelled(word, &tokens(&capital), 0).is_some());
            }
        }
        let typed = tokens("aujourd’hui");
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
    fn a_scale_is_read_in_its_case_and_a_word_in_any() {
        assert_eq!(spelled(CalcWord::Mega, &tokens("m"), 0), None);
        assert_eq!(spelled(CalcWord::Kilo, &tokens("K"), 0), None);
        assert_eq!(scale_at(&tokens("Md"), 0), Some((1_000_000_000, 1)));
        assert_eq!(
            scale_at(&tokens("Mille milliards"), 0),
            Some((1_000_000_000_000, 2))
        );
    }

    #[test]
    fn a_spelling_is_written_folded_and_one_space_apart() {
        for word in CalcWord::ALL {
            for spelling in word.every_spelling() {
                if !word.exact() {
                    assert_eq!(fold(spelling), spelling, "{spelling}");
                }
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
        let both: BTreeSet<&str> = CalcWord::ALL
            .iter()
            .flat_map(|word| word.every_spelling())
            .filter(|spelling| symbols.contains(spelling))
            .collect();
        assert_eq!(both, BTreeSet::from(["in", "em", "rem"]));
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
