//! A line cut into tokens, each with where it stands in UTF-16 units: the front's editor
//! counts that way, and draws the colours at those offsets.

use chrono::NaiveDate;
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Tok {
    Number(Decimal),
    Date(NaiveDate),
    /// A name, a keyword or a unit: which one is decided by where it stands.
    Word(String),
    Symbol(char),
    /// Between double quotes, quotes included: text left out of the computation.
    Quoted,
    /// A character the grammar has no use for (`?`, `,`, `:`…): only a label, a remark or a
    /// sentence holds one.
    Other(char),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Token {
    pub(super) tok: Tok,
    pub(super) start: u32,
    pub(super) end: u32,
    /// After a space: `€/h` is one unit, `€ / h` a division.
    pub(super) spaced: bool,
}

const SYMBOLS: &[char] = &[
    '+', '-', '−', '×', '*', '/', '÷', '^', '(', ')', '=', '%', '€', '$',
];

fn is_word_start(character: char) -> bool {
    character.is_alphabetic() || character == '_' || character == '°'
}

fn is_word_part(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// `16/10/2026` or `2026-10-16`, read before a number would take its digits.
fn date_at(characters: &[char], at: usize) -> Option<(NaiveDate, usize)> {
    let text: String = characters[at..].iter().take(10).collect();
    let digits = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
    let ends = |length: usize| {
        characters
            .get(at + length)
            .is_none_or(|c| !c.is_ascii_digit() && *c != '/' && *c != '-')
    };

    if let [year, month, day] = text.split('-').collect::<Vec<_>>().as_slice()
        && year.len() == 4
        && month.len() == 2
        && day.len() >= 2
        && digits(year)
        && digits(month)
        && digits(&day[..2])
    {
        let day = &day[..2];
        let length = 10;
        if ends(length) {
            let date = NaiveDate::from_ymd_opt(
                year.parse().ok()?,
                month.parse().ok()?,
                day.parse().ok()?,
            )?;
            return Some((date, length));
        }
    }

    let parts: Vec<&str> = text.splitn(3, '/').collect();
    if let [day, month, rest] = parts.as_slice()
        && (1..=2).contains(&day.len())
        && (1..=2).contains(&month.len())
        && rest.len() >= 4
        && digits(day)
        && digits(month)
        && digits(&rest[..4])
    {
        let length = day.len() + month.len() + 2 + 4;
        if ends(length) {
            let date = NaiveDate::from_ymd_opt(
                rest[..4].parse().ok()?,
                month.parse().ok()?,
                day.parse().ok()?,
            )?;
            return Some((date, length));
        }
    }
    None
}

/// Three digits after a space, the next not a digit: `5 300`, and `1 578,13 €` as the front
/// writes it, its space a no-break one.
fn group_at(characters: &[char], at: usize) -> bool {
    characters
        .get(at)
        .is_some_and(|c| matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
        && (1..=3).all(|offset| {
            characters
                .get(at + offset)
                .is_some_and(char::is_ascii_digit)
        })
        && !characters.get(at + 4).is_some_and(char::is_ascii_digit)
}

/// Digits, grouped by thousands or not, then a point or a comma and more digits: `0,045` and
/// `1.5` are both read.
fn number_at(characters: &[char], at: usize) -> Option<(Decimal, usize)> {
    let mut end = at;
    while characters.get(end).is_some_and(char::is_ascii_digit) {
        end += 1;
    }
    if end == at {
        return None;
    }
    if end - at <= 3 {
        while group_at(characters, end) {
            end += 4;
        }
    }
    if matches!(characters.get(end), Some('.' | ','))
        && characters.get(end + 1).is_some_and(char::is_ascii_digit)
    {
        end += 1;
        while characters.get(end).is_some_and(char::is_ascii_digit) {
            end += 1;
        }
    }
    let text: String = characters[at..end]
        .iter()
        .filter(|c| !c.is_whitespace())
        .map(|c| if *c == ',' { '.' } else { *c })
        .collect();
    text.parse().ok().map(|value| (value, end - at))
}

/// A word with an apostrophe inside it is one word: `aujourd'hui`.
fn word_at(characters: &[char], at: usize) -> usize {
    let mut end = at + 1;
    while let Some(&character) = characters.get(end) {
        let inner_apostrophe = matches!(character, '\'' | '’')
            && characters
                .get(end + 1)
                .is_some_and(|next| next.is_alphabetic());
        if is_word_part(character) || inner_apostrophe {
            end += 1;
        } else {
            break;
        }
    }
    end - at
}

/// The line without what follows `//`, which a copied sheet carries its results in.
pub(super) fn without_comment(line: &str) -> &str {
    line.find("//").map_or(line, |at| &line[..at])
}

/// `"…"`, `“…”` or `«…»`, closed on the line: how far it runs, quotes included.
fn quoted_at(characters: &[char], at: usize) -> Option<usize> {
    let close = match characters[at] {
        '"' => '"',
        '“' => '”',
        '«' => '»',
        _ => return None,
    };
    let inside = characters[at + 1..].iter().position(|c| *c == close)?;
    Some(inside + 2)
}

pub(super) fn tokens(line: &str) -> Vec<Token> {
    let characters: Vec<char> = without_comment(line).chars().collect();
    let mut offsets = Vec::with_capacity(characters.len() + 1);
    let mut offset = 0u32;
    for character in &characters {
        offsets.push(offset);
        offset += u32::try_from(character.len_utf16()).unwrap_or(1);
    }
    offsets.push(offset);

    let mut tokens = Vec::new();
    let mut at = 0;
    let mut spaced = false;
    while let Some(&character) = characters.get(at) {
        if character.is_whitespace() {
            spaced = true;
            at += 1;
            continue;
        }
        let (tok, length) = if let Some((date, length)) = date_at(&characters, at) {
            (Tok::Date(date), length)
        } else if let Some((number, length)) = number_at(&characters, at) {
            (Tok::Number(number), length)
        } else if is_word_start(character) {
            let length = word_at(&characters, at);
            (
                Tok::Word(characters[at..at + length].iter().collect()),
                length,
            )
        } else if SYMBOLS.contains(&character) {
            (Tok::Symbol(character), 1)
        } else if let Some(length) = quoted_at(&characters, at) {
            (Tok::Quoted, length)
        } else {
            (Tok::Other(character), 1)
        };
        tokens.push(Token {
            tok,
            start: offsets[at],
            end: offsets[at + length],
            spaced,
        });
        spaced = false;
        at += length;
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(line: &str) -> Vec<Tok> {
        tokens(line).into_iter().map(|token| token.tok).collect()
    }

    #[test]
    fn a_comma_or_a_point_marks_the_decimals() {
        assert_eq!(toks("0,045"), [Tok::Number("0.045".parse().unwrap())]);
        assert_eq!(toks("1.5"), [Tok::Number("1.5".parse().unwrap())]);
    }

    #[test]
    fn thousands_grouped_by_a_space_are_one_number() {
        let number = |text: &str| Tok::Number(text.parse().unwrap());
        assert_eq!(toks("5 300"), [number("5300")]);
        assert_eq!(toks("1 000 000"), [number("1000000")]);
        assert_eq!(
            toks("1\u{202f}578,13 €"),
            [number("1578.13"), Tok::Symbol('€')]
        );
        assert_eq!(toks("1\u{a0}578,13"), [number("1578.13")]);
        assert_eq!(toks("1234 567"), [number("1234"), number("567")]);
        assert_eq!(toks("5 3000"), [number("5"), number("3000")]);
        assert_eq!(
            toks("3 h 20 min"),
            [
                number("3"),
                Tok::Word("h".into()),
                number("20"),
                Tok::Word("min".into())
            ]
        );
    }

    #[test]
    fn a_quoted_text_is_one_token_and_a_stray_character_another() {
        assert_eq!(
            toks("275 € « Modèle 227 » \"x\" “y” ?"),
            [
                Tok::Number(Decimal::from(275)),
                Tok::Symbol('€'),
                Tok::Quoted,
                Tok::Quoted,
                Tok::Quoted,
                Tok::Other('?')
            ]
        );
        assert_eq!(toks("\"open"), [Tok::Other('"'), Tok::Word("open".into())]);
    }

    #[test]
    fn a_date_is_read_before_its_digits_are_taken_for_a_division() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
        assert_eq!(toks("16/10/2026"), [Tok::Date(date)]);
        assert_eq!(toks("2026-10-16"), [Tok::Date(date)]);
        assert_eq!(
            toks("16/10"),
            [
                Tok::Number(Decimal::from(16)),
                Tok::Symbol('/'),
                Tok::Number(Decimal::from(10))
            ]
        );
    }

    #[test]
    fn a_unit_with_a_slash_and_no_space_stays_one_rate() {
        let line = tokens("0,045 €/h");
        assert_eq!(line[1].tok, Tok::Symbol('€'));
        assert!(!line[2].spaced);
        assert_eq!(line[3].tok, Tok::Word("h".into()));
    }

    #[test]
    fn an_apostrophe_inside_a_word_keeps_it_whole_and_a_comment_is_left_out() {
        assert_eq!(
            toks("aujourd'hui + 2 // ven."),
            [
                Tok::Word("aujourd'hui".into()),
                Tok::Symbol('+'),
                Tok::Number(Decimal::from(2)),
            ]
        );
    }

    #[test]
    fn offsets_are_counted_in_utf16_units() {
        let line = tokens("😀 = 2");
        assert_eq!(
            (line[0].tok.clone(), line[0].end, line[1].start),
            (Tok::Other('😀'), 2, 3)
        );
        let line = tokens("é = 2");
        assert_eq!((line[2].start, line[2].end), (4, 5));
        let line = tokens("°C");
        assert_eq!(line[0].tok, Tok::Word("°C".into()));
    }
}
