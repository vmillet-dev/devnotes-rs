//! What a sheet computes over plain numbers beyond the four operations: rounding, roots,
//! logarithms, bases and bits. Exact where the answer is: `sqrt 16` is 4, not 3.999….

use rust_decimal::prelude::{MathematicalOps, ToPrimitive};
use rust_decimal::{Decimal, RoundingStrategy};

use super::CalcError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rounding {
    /// A half away from zero: `5,5 arrondi` is 6, `-5,5` is -6.
    Nearest,
    Up,
    Down,
}

fn overflow() -> CalcError {
    CalcError::Overflow
}

/// To a multiple of `step`: `37 au 10 près` is 40.
pub(super) fn round_to(
    amount: Decimal,
    step: Decimal,
    rounding: Rounding,
) -> Result<Decimal, CalcError> {
    if step.is_zero() {
        return Err(CalcError::DivisionByZero);
    }
    let steps = amount.checked_div(step.abs()).ok_or_else(overflow)?;
    let whole = match rounding {
        Rounding::Nearest => {
            steps.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
        }
        Rounding::Up => steps.ceil(),
        Rounding::Down => steps.floor(),
    };
    whole.checked_mul(step.abs()).ok_or_else(overflow)
}

pub(super) fn round_places(amount: Decimal, places: u32) -> Decimal {
    amount.round_dp_with_strategy(places.min(28), RoundingStrategy::MidpointAwayFromZero)
}

/// Integer powers only, so `2 ^ 10` stays exact.
fn power(base: Decimal, exponent: u32) -> Option<Decimal> {
    (0..exponent).try_fold(Decimal::ONE, |product, _| product.checked_mul(base))
}

/// The root a decimal holds exactly, when one does: twelve places past the point is past what a
/// sheet writes and short of the last figure an approximation gets wrong.
fn exact_root(x: Decimal, approximate: Decimal, degree: u32) -> Decimal {
    let candidate = approximate.round_dp(12).normalize();
    if power(candidate, degree) == Some(x) {
        candidate
    } else {
        approximate
    }
}

/// `racine 2 (8)` is the square root of 8: an even root of a negative number is refused.
pub(super) fn root(x: Decimal, degree: u32) -> Result<Decimal, CalcError> {
    if degree == 0 {
        return Err(CalcError::Unsupported);
    }
    if x.is_sign_negative() && !x.is_zero() && degree.is_multiple_of(2) {
        return Err(CalcError::NegativeRoot);
    }
    if x.is_zero() || degree == 1 {
        return Ok(x);
    }
    let size = x.abs();
    let mut guess = if degree == 2 {
        size.sqrt().ok_or_else(overflow)?
    } else {
        let estimate = size
            .to_f64()
            .ok_or_else(overflow)?
            .powf(1.0 / f64::from(degree));
        Decimal::try_from(estimate).map_err(|_| overflow())?
    };
    // Newton's steps take the estimate to every figure a decimal holds.
    let n = Decimal::from(degree);
    for _ in 0..8 {
        let below = power(guess, degree - 1).ok_or_else(overflow)?;
        if below.is_zero() {
            break;
        }
        let quotient = size.checked_div(below).ok_or_else(overflow)?;
        guess = ((n - Decimal::ONE) * guess + quotient)
            .checked_div(n)
            .ok_or_else(overflow)?;
    }
    let found = exact_root(size, guess, degree);
    Ok(if x.is_sign_negative() { -found } else { found })
}

/// A whole result a decimal logarithm only comes near: `ln(e)` is 1.
fn whole_if_near(value: Decimal) -> Decimal {
    let whole = value.round();
    if (value - whole).abs() < Decimal::new(1, 20) {
        whole
    } else {
        value
    }
}

pub(super) fn ln(x: Decimal) -> Result<Decimal, CalcError> {
    if x <= Decimal::ZERO {
        return Err(CalcError::LogDomain);
    }
    x.checked_ln().map(whole_if_near).ok_or_else(overflow)
}

/// In base 10 unless one is given: `log 2 (8)` is 3, exactly.
pub(super) fn log(x: Decimal, base: Option<Decimal>) -> Result<Decimal, CalcError> {
    if x <= Decimal::ZERO {
        return Err(CalcError::LogDomain);
    }
    let Some(base) = base else {
        return x.checked_log10().map(whole_if_near).ok_or_else(overflow);
    };
    if base <= Decimal::ZERO || base == Decimal::ONE {
        return Err(CalcError::LogDomain);
    }
    let value = x
        .checked_ln()
        .zip(base.checked_ln())
        .and_then(|(above, below)| above.checked_div(below))
        .ok_or_else(overflow)?;
    let whole = value.round();
    let exact = whole
        .to_u32()
        .and_then(|exponent| power(base, exponent))
        .is_some_and(|reached| reached == x);
    Ok(if exact { whole } else { whole_if_near(value) })
}

/// A whole number, or the error that says one was expected.
pub(super) fn whole(value: Decimal) -> Result<i128, CalcError> {
    if !value.fract().is_zero() {
        return Err(CalcError::NotWhole);
    }
    value.to_i128().ok_or_else(overflow)
}

fn decimal(value: i128) -> Result<Decimal, CalcError> {
    Decimal::try_from_i128_with_scale(value, 0).map_err(|_| overflow())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Bits {
    And,
    Or,
    Xor,
    Left,
    Right,
}

pub(super) fn bits(left: Decimal, right: Decimal, operation: Bits) -> Result<Decimal, CalcError> {
    let (a, b) = (whole(left)?, whole(right)?);
    let shift = || {
        u32::try_from(b)
            .ok()
            .filter(|count| *count < 127)
            .ok_or_else(overflow)
    };
    let value = match operation {
        Bits::And => a & b,
        Bits::Or => a | b,
        Bits::Xor => a ^ b,
        Bits::Left => a
            .checked_mul(1_i128.checked_shl(shift()?).ok_or_else(overflow)?)
            .ok_or_else(overflow)?,
        Bits::Right => a >> shift()?,
    };
    decimal(value)
}

/// The digits of a whole number in base 2, 8 or 16, and its sign.
pub(super) fn digits(value: Decimal, radix: u32) -> Result<(bool, String), CalcError> {
    let number = whole(value)?;
    let size = number.unsigned_abs();
    let digits = match radix {
        2 => format!("{size:b}"),
        8 => format!("{size:o}"),
        16 => format!("{size:X}"),
        _ => return Err(CalcError::Unsupported),
    };
    Ok((number < 0, digits))
}

/// `5300` is `5.3` times ten to the 3: the mantissa between 1 and 10, its exponent.
pub(super) fn scientific(value: Decimal) -> (String, i32) {
    let value = value.normalize();
    if value.is_zero() {
        return ("0".into(), 0);
    }
    let figures = value.mantissa().unsigned_abs().to_string();
    let exponent =
        i32::try_from(figures.len()).unwrap_or(0) - 1 - i32::try_from(value.scale()).unwrap_or(0);
    let rest = figures[1..].trim_end_matches('0');
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let mantissa = if rest.is_empty() {
        format!("{sign}{}", &figures[..1])
    } else {
        format!("{sign}{}.{rest}", &figures[..1])
    };
    (mantissa, exponent)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> Decimal {
        text.parse().unwrap()
    }

    #[test]
    fn a_root_is_exact_where_one_exists_and_refused_where_none_does() {
        assert_eq!(root(d("16"), 2), Ok(d("4")));
        assert_eq!(root(d("8"), 3), Ok(d("2")));
        assert_eq!(root(d("-8"), 3), Ok(d("-2")));
        assert_eq!(root(d("0.25"), 2), Ok(d("0.5")));
        assert_eq!(root(d("-4"), 2), Err(CalcError::NegativeRoot));
        let two = root(d("2"), 2).unwrap();
        assert!(two.to_string().starts_with("1.41421356237309504880168872"));
        let eight = root(d("8"), 2).unwrap();
        assert!(
            eight
                .to_string()
                .starts_with("2.828427124746190097603377448")
        );
    }

    #[test]
    fn a_logarithm_is_whole_where_it_is_and_refused_at_zero_or_below() {
        assert_eq!(log(d("100"), None), Ok(d("2")));
        assert_eq!(log(d("8"), Some(d("2"))), Ok(d("3")));
        assert_eq!(ln(Decimal::E), Ok(d("1")));
        assert_eq!(log(d("0"), None), Err(CalcError::LogDomain));
        assert_eq!(ln(d("-1")), Err(CalcError::LogDomain));
        assert_eq!(log(d("8"), Some(d("1"))), Err(CalcError::LogDomain));
    }

    #[test]
    fn rounding_takes_a_half_away_from_zero() {
        assert_eq!(round_to(d("5.5"), d("1"), Rounding::Nearest), Ok(d("6")));
        assert_eq!(round_to(d("-5.5"), d("1"), Rounding::Nearest), Ok(d("-6")));
        assert_eq!(round_to(d("5.5"), d("1"), Rounding::Down), Ok(d("5")));
        assert_eq!(round_to(d("21"), d("5"), Rounding::Up), Ok(d("25")));
        assert_eq!(round_to(d("37"), d("10"), Rounding::Nearest), Ok(d("40")));
        assert_eq!(round_places(d("3.14159265"), 5), d("3.14159"));
        assert_eq!(round_places(d("0.125"), 2), d("0.13"));
    }

    #[test]
    fn bits_and_bases_read_whole_numbers_only() {
        assert_eq!(bits(d("5"), d("3"), Bits::And), Ok(d("1")));
        assert_eq!(bits(d("5"), d("3"), Bits::Or), Ok(d("7")));
        assert_eq!(bits(d("5"), d("3"), Bits::Xor), Ok(d("6")));
        assert_eq!(bits(d("1"), d("4"), Bits::Left), Ok(d("16")));
        assert_eq!(bits(d("16"), d("2"), Bits::Right), Ok(d("4")));
        assert_eq!(bits(d("1.5"), d("1"), Bits::And), Err(CalcError::NotWhole));
        assert_eq!(digits(d("255"), 16), Ok((false, "FF".into())));
        assert_eq!(digits(d("-10"), 2), Ok((true, "1010".into())));
        assert_eq!(digits(d("255"), 8), Ok((false, "377".into())));
        assert_eq!(digits(d("2.5"), 16), Err(CalcError::NotWhole));
    }

    #[test]
    fn a_number_is_written_as_a_mantissa_and_its_exponent() {
        assert_eq!(scientific(d("5300")), ("5.3".into(), 3));
        assert_eq!(scientific(d("0.000123")), ("1.23".into(), -4));
        assert_eq!(scientific(d("-7")), ("-7".into(), 0));
        assert_eq!(scientific(Decimal::ZERO), ("0".into(), 0));
    }
}
