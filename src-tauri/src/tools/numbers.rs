//! A number as a person types it — a point or a comma before the decimals, spaces or underscores
//! between the thousands, an exponent — read exactly, as a `Decimal`, never as a float.

use std::str::FromStr;

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumberProblem {
    /// Zero-based, in characters of the text given.
    Unreadable(usize),
    /// Past the 28 digits a `Decimal` holds.
    TooLarge,
}

/// A space may sit between two digits, never elsewhere: `1 000` is a thousand, `1 x` is not.
fn is_group_space(character: char) -> bool {
    matches!(character, ' ' | '_' | '\u{a0}' | '\u{202f}' | '\'')
}

/// The number the text starts with (after any space), and how many characters it took.
pub(crate) fn leading_number(text: &str) -> Result<(Decimal, usize), NumberProblem> {
    let characters: Vec<char> = text.chars().collect();
    let mut at = characters.iter().take_while(|c| c.is_whitespace()).count();
    let start = at;
    let mut mantissa = String::new();
    if matches!(characters.get(at), Some('-' | '+')) {
        mantissa.push(characters[at]);
        at += 1;
    }
    let digits_start = at;
    let mut marks: Vec<(usize, char)> = Vec::new();
    while let Some(&character) = characters.get(at) {
        let between_digits = |c: char| {
            at > digits_start
                && characters[at - 1].is_ascii_digit()
                && characters.get(at + 1).is_some_and(char::is_ascii_digit)
                && (is_group_space(c) || c == '.' || c == ',')
        };
        if character.is_ascii_digit() {
            mantissa.push(character);
        } else if between_digits(character) {
            if character == '.' || character == ',' {
                marks.push((mantissa.len(), character));
            }
        } else {
            break;
        }
        at += 1;
    }
    if !mantissa.chars().any(|c| c.is_ascii_digit()) {
        return Err(NumberProblem::Unreadable(start));
    }

    // The last mark is the decimal one when the marks differ, or when it stands alone;
    // `1,234,567` repeats one mark, which then groups thousands.
    let decimal = match marks.as_slice() {
        [(index, _)] => Some(*index),
        [.., (index, last)] if marks.iter().any(|(_, mark)| mark != last) => Some(*index),
        _ => None,
    };
    if let Some(index) = decimal {
        mantissa.insert(index, '.');
    }

    let mut exponent = 0i64;
    if matches!(characters.get(at), Some('e' | 'E')) {
        let mut cursor = at + 1;
        let sign = match characters.get(cursor) {
            Some('-') => {
                cursor += 1;
                -1
            }
            Some('+') => {
                cursor += 1;
                1
            }
            _ => 1,
        };
        let digits: String = characters[cursor..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            exponent = sign * digits.parse::<i64>().map_err(|_| NumberProblem::TooLarge)?;
            at = cursor + digits.len();
        }
    }

    let mut value = Decimal::from_str(&mantissa).map_err(|_| NumberProblem::TooLarge)?;
    let ten = Decimal::TEN;
    for _ in 0..exponent.unsigned_abs() {
        if value.is_zero() {
            break;
        }
        value = if exponent > 0 {
            value.checked_mul(ten)
        } else {
            value.checked_div(ten)
        }
        .ok_or(NumberProblem::TooLarge)?;
    }
    Ok((value.normalize(), at))
}

/// A whole field that must be one number and nothing else.
pub(crate) fn number(text: &str) -> Result<Decimal, NumberProblem> {
    let (value, end) = leading_number(text)?;
    let rest = text.chars().skip(end);
    match rest.clone().position(|c| !c.is_whitespace()) {
        Some(offset) => Err(NumberProblem::Unreadable(end + offset)),
        None => Ok(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> String {
        number(text).map_or_else(|problem| format!("{problem:?}"), |value| value.to_string())
    }

    #[test]
    fn a_point_or_a_comma_before_the_decimals() {
        assert_eq!(read("1.5"), "1.5");
        assert_eq!(read("1,5"), "1.5");
        assert_eq!(read(" -0,25 "), "-0.25");
        assert_eq!(read("+3"), "3");
    }

    #[test]
    fn thousands_are_grouped_by_spaces_or_a_repeated_mark() {
        assert_eq!(read("1 000 000"), "1000000");
        assert_eq!(read("1\u{202f}000,5"), "1000.5");
        assert_eq!(read("1_000"), "1000");
        assert_eq!(read("1,234,567"), "1234567");
        assert_eq!(read("1.234.567"), "1234567");
        assert_eq!(read("1,234.5"), "1234.5");
        assert_eq!(read("1.234,5"), "1234.5");
        assert_eq!(read("1'000"), "1000");
    }

    #[test]
    fn an_exponent_is_exact() {
        assert_eq!(read("1e9"), "1000000000");
        assert_eq!(read("1.5E-3"), "0.0015");
        assert_eq!(read("2e+2"), "200");
    }

    #[test]
    fn what_floats_get_wrong_is_kept_exact() {
        let sum = number("0.1").unwrap() + number("0.2").unwrap();

        assert_eq!(sum.to_string(), "0.3");
    }

    #[test]
    fn what_is_no_number_says_where() {
        assert_eq!(read("abc"), "Unreadable(0)");
        assert_eq!(read("  -"), "Unreadable(2)");
        assert_eq!(read("12x"), "Unreadable(2)");
        assert_eq!(read("1 2 x"), "Unreadable(4)");
        assert_eq!(read("1..2"), "Unreadable(1)");
        assert_eq!(read(&"9".repeat(40)), "TooLarge");
        assert_eq!(read("1e400"), "TooLarge");
        assert_eq!(read("0e999999999999"), "0");
        assert_eq!(read("1e-999999999999"), "0");
    }

    #[test]
    fn the_leading_number_says_where_it_ends() {
        assert_eq!(
            leading_number("1,5 Go"),
            Ok((Decimal::from_str("1.5").unwrap(), 3))
        );
        assert_eq!(leading_number("750KiB"), Ok((Decimal::from(750), 3)));
    }
}
