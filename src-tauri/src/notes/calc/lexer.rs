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
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Token {
    pub(super) tok: Tok,
    pub(super) start: u32,
    pub(super) end: u32,
    /// After a space: `€/h` is one unit, `€ / h` a division.
    pub(super) spaced: bool,
}

/// What breaks a line into tokens: `?`, `@` and their like are not part of the language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Unreadable {
    pub(super) at: u32,
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

/// Digits, then a point or a comma and more digits: `0,045` and `1.5` are both read.
fn number_at(characters: &[char], at: usize) -> Option<(Decimal, usize)> {
    let mut end = at;
    while characters.get(end).is_some_and(char::is_ascii_digit) {
        end += 1;
    }
    if end == at {
        return None;
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

pub(super) fn tokens(line: &str) -> Result<Vec<Token>, Unreadable> {
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
        } else {
            return Err(Unreadable { at: offsets[at] });
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
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(line: &str) -> Vec<Tok> {
        tokens(line)
            .unwrap()
            .into_iter()
            .map(|token| token.tok)
            .collect()
    }

    #[test]
    fn a_comma_or_a_point_marks_the_decimals() {
        assert_eq!(toks("0,045"), [Tok::Number("0.045".parse().unwrap())]);
        assert_eq!(toks("1.5"), [Tok::Number("1.5".parse().unwrap())]);
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
        let line = tokens("0,045 €/h").unwrap();
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
        let line = tokens("😀 = 2").unwrap_err();
        assert_eq!(line.at, 0);
        let line = tokens("é = 2").unwrap();
        assert_eq!((line[2].start, line[2].end), (4, 5));
        let line = tokens("°C").unwrap();
        assert_eq!(line[0].tok, Tok::Word("°C".into()));
    }
}
