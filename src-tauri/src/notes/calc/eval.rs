//! One line read and computed, with what the lines above it named. A recursive descent:
//! a conversion over sums over products over powers over a value.

use std::collections::HashMap;

use chrono::{Days, Months, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::lexer::{Tok, Token};
use super::units::{Dim, Dimension, UnitDef, UnitId, base_unit, from_celsius, to_celsius};
use super::words::{self, CalcWord};
use super::{CalcError, CalcUnit, TokenKind};

/// How a quantity is shown: in one unit, one over another, or in a unit and the next one
/// down when it does not fall whole (`6 sem 3 j`), which only a conversion asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shown {
    One(UnitId),
    Per(UnitId, UnitId),
}

impl Shown {
    pub(super) fn calc_unit(self) -> CalcUnit {
        match self {
            Self::One(unit) => CalcUnit { unit, per: None },
            Self::Per(unit, per) => CalcUnit {
                unit,
                per: Some(per),
            },
        }
    }
}

/// Base units in one of a unit, as a fraction.
#[derive(Debug, Clone, Copy)]
pub(super) struct Factor {
    pub(super) above: Decimal,
    pub(super) below: Decimal,
}

impl Factor {
    fn apply(self, number: Decimal) -> Result<Decimal, CalcError> {
        number
            .checked_mul(self.above)
            .and_then(|product| product.checked_div(self.below))
            .ok_or(CalcError::Overflow)
    }
}

/// An amount in base units — but a temperature's in its own degrees, converted by formula.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Quantity {
    pub(super) amount: Decimal,
    pub(super) dim: Dim,
    pub(super) unit: Option<Shown>,
    /// Converted into weeks, days, hours or minutes: shown with the next unit down if needed.
    pub(super) spans: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Value {
    Quantity(Quantity),
    /// `0.15` for 15 %; `signed` for a change, written `+25 %`.
    Percent {
        ratio: Decimal,
        signed: bool,
    },
    Date(NaiveDate),
}

impl Value {
    pub(super) fn dimension(&self) -> Dimension {
        match self {
            Self::Quantity(quantity) => quantity.dim.dimension(),
            Self::Percent { .. } => Dimension::Number,
            Self::Date(_) => Dimension::Date,
        }
    }

    fn number(amount: Decimal) -> Self {
        Self::Quantity(Quantity {
            amount,
            dim: Dim::NONE,
            unit: None,
            spans: false,
        })
    }
}

/// What the lines above hand to this one.
pub(super) struct Context<'a> {
    pub(super) today: NaiveDate,
    pub(super) units: &'a [UnitDef],
    pub(super) symbols: &'a HashMap<&'static str, usize>,
    pub(super) variables: &'a HashMap<String, Value>,
    /// The quantities of the block so far, for `total` and `moyenne`.
    pub(super) block: &'a [Quantity],
}

pub(super) struct Parser<'a> {
    context: &'a Context<'a>,
    tokens: &'a [Token],
    at: usize,
    pub(super) kinds: Vec<Option<TokenKind>>,
    /// The names read, for the hover that says where a variable is used.
    pub(super) used: Vec<String>,
    /// What `en` or `à` was asked to convert: the units the editor proposes next.
    pub(super) converting: Option<Dimension>,
    /// `total` or `moyenne` read: the line is drawn with a rule above it.
    pub(super) aggregate: bool,
    /// Where the line ends, for a value missing there.
    end: u32,
}

type Answer = Result<Value, CalcError>;

fn overflow() -> CalcError {
    CalcError::Overflow
}

impl<'a> Parser<'a> {
    pub(super) fn new(context: &'a Context<'a>, tokens: &'a [Token], end: u32) -> Self {
        Self {
            context,
            tokens,
            at: 0,
            kinds: vec![None; tokens.len()],
            used: Vec::new(),
            converting: None,
            aggregate: false,
            end,
        }
    }

    pub(super) fn whole(&mut self) -> Answer {
        let value = self.conversion()?;
        if self.at < self.tokens.len() {
            return Err(self.unreadable());
        }
        Ok(value)
    }

    fn unreadable(&mut self) -> CalcError {
        match self.tokens.get(self.at) {
            Some(token) => {
                self.kinds[self.at] = Some(TokenKind::Error);
                CalcError::Unreadable { at: token.start }
            }
            None => CalcError::Unreadable { at: self.end },
        }
    }

    /// The last token taken is the one at fault: the unit added to the wrong quantity.
    fn blame(&mut self, error: CalcError) -> CalcError {
        if let Some(last) = self.at.checked_sub(1) {
            self.kinds[last] = Some(TokenKind::Error);
        }
        error
    }

    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.at).map(|token| &token.tok)
    }

    fn take(&mut self, kind: TokenKind) {
        self.kinds[self.at] = Some(kind);
        self.at += 1;
    }

    fn peek_symbol(&self, symbols: &[char]) -> Option<char> {
        match self.peek() {
            Some(Tok::Symbol(symbol)) if symbols.contains(symbol) => Some(*symbol),
            _ => None,
        }
    }

    fn sees(&self, word: CalcWord) -> Option<usize> {
        words::spelled(word, self.tokens, self.at)
    }

    fn eat(&mut self, word: CalcWord) -> bool {
        let Some(count) = self.sees(word) else {
            return false;
        };
        for _ in 0..count {
            self.take(TokenKind::Keyword);
        }
        true
    }

    fn text_at(&self, index: usize) -> Option<String> {
        match self.tokens.get(index).map(|token| &token.tok) {
            Some(Tok::Word(word)) => Some(word.clone()),
            Some(Tok::Symbol(symbol @ ('€' | '$' | '%'))) => Some(symbol.to_string()),
            _ => None,
        }
    }

    fn unit_at(&self, index: usize) -> Option<&'a UnitDef> {
        let text = self.text_at(index)?;
        self.context
            .symbols
            .get(text.as_str())
            .map(|&found| &self.context.units[found])
    }

    /// A unit, or one over another written without a space: `€/h`, `Mo/s`. Its factor comes
    /// in two, multiplied by the first and divided by the second: `1/3600` is not exact.
    fn read_unit(&mut self) -> Option<(Shown, Dim, Factor)> {
        let first = self.unit_at(self.at)?;
        self.take(TokenKind::Unit);
        let joined = |token: Option<&Token>| token.is_some_and(|token| !token.spaced);
        if self.peek_symbol(&['/']).is_some()
            && joined(self.tokens.get(self.at))
            && joined(self.tokens.get(self.at + 1))
            && let Some(second) = self.unit_at(self.at + 1)
            && !first.dim.has_temperature()
            && !second.dim.has_temperature()
        {
            self.take(TokenKind::Unit);
            self.take(TokenKind::Unit);
            let factor = Factor {
                above: first.factor,
                below: second.factor,
            };
            return Some((
                Shown::Per(first.id, second.id),
                first.dim.over(second.dim),
                factor,
            ));
        }
        let factor = Factor {
            above: first.factor,
            below: Decimal::ONE,
        };
        Some((Shown::One(first.id), first.dim, factor))
    }

    fn conversion(&mut self) -> Answer {
        let mut value = self.additive()?;
        loop {
            if self.eat(CalcWord::Convert) {
                self.converting = Some(value.dimension());
                value = self.convert(value)?;
            } else if self.eat(CalcWord::At) {
                self.converting = Some(value.dimension());
                // A unit alone is where to convert to; a quantity, the rate to divide by.
                if self.peek_symbol(&['%']).is_some() || self.unit_at(self.at).is_some() {
                    value = self.convert(value)?;
                } else {
                    let rate = self.additive()?;
                    value = divide(value, rate).map_err(|error| self.blame(error))?;
                    if value.dimension() != Dimension::Time {
                        return Err(self.blame(CalcError::Mismatch {
                            expected: Dimension::Time,
                            found: value.dimension(),
                            unit: None,
                        }));
                    }
                }
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn convert(&mut self, value: Value) -> Answer {
        if self.peek_symbol(&['%']).is_some() {
            self.take(TokenKind::Unit);
            return match value {
                Value::Percent { .. } => Ok(value),
                Value::Quantity(quantity) if quantity.dim.is_none() => Ok(Value::Percent {
                    ratio: quantity.amount,
                    signed: false,
                }),
                other => Err(self.blame(CalcError::NotConvertible {
                    from: other.dimension(),
                    to: None,
                })),
            };
        }
        let Some((shown, dim, _)) = self.read_unit() else {
            return Err(self.unreadable());
        };
        convert(value, shown, dim).map_err(|error| self.blame(error))
    }

    fn additive(&mut self) -> Answer {
        let mut left = self.term()?;
        while let Some(symbol) = self.peek_symbol(&['+', '-', '−']) {
            self.take(TokenKind::Operator);
            let right = self.term()?;
            left = add(left, right, symbol == '+').map_err(|error| self.blame(error))?;
        }
        Ok(left)
    }

    fn term(&mut self) -> Answer {
        let mut left = self.power()?;
        loop {
            if let Some(symbol) = self.peek_symbol(&['×', '*', '/', '÷']) {
                self.take(TokenKind::Operator);
                let right = self.power()?;
                left = if matches!(symbol, '×' | '*') {
                    multiply(left, right)
                } else {
                    divide(left, right)
                }
                .map_err(|error| self.blame(error))?;
            } else if self.eat(CalcWord::Excluding) {
                let right = self.power()?;
                left = excluding(left, right).map_err(|error| self.blame(error))?;
            } else if matches!(left, Value::Percent { .. }) && self.eat(CalcWord::Of) {
                let right = self.power()?;
                left = multiply(left, right).map_err(|error| self.blame(error))?;
            } else if self.eat(CalcWord::Over) {
                let right = self.power()?;
                left = ratio(left, right).map_err(|error| self.blame(error))?;
            } else {
                return Ok(left);
            }
        }
    }

    fn power(&mut self) -> Answer {
        let base = self.unary()?;
        if self.peek_symbol(&['^']).is_none() {
            return Ok(base);
        }
        self.take(TokenKind::Operator);
        let exponent = self.unary()?;
        power(base, exponent).map_err(|error| self.blame(error))
    }

    fn unary(&mut self) -> Answer {
        if let Some(symbol) = self.peek_symbol(&['-', '−', '+']) {
            self.take(TokenKind::Operator);
            let value = self.unary()?;
            return if symbol == '+' {
                Ok(value)
            } else {
                negate(value)
            };
        }
        self.primary()
    }

    fn primary(&mut self) -> Answer {
        let Some(token) = self.peek().cloned() else {
            return Err(self.unreadable());
        };
        match token {
            Tok::Number(number) => {
                self.take(TokenKind::Number);
                self.quantity(number)
            }
            Tok::Date(date) => {
                self.take(TokenKind::Date);
                Ok(Value::Date(date))
            }
            Tok::Symbol('(') => {
                self.take(TokenKind::Operator);
                let value = self.conversion()?;
                if self.peek_symbol(&[')']).is_none() {
                    return Err(self.unreadable());
                }
                self.take(TokenKind::Operator);
                Ok(value)
            }
            Tok::Word(word) => self.word(&word),
            Tok::Symbol(_) => Err(self.unreadable()),
        }
    }

    /// A number, its unit, and the quantities of the same kind written after it: `3 h 20 min`.
    fn quantity(&mut self, number: Decimal) -> Answer {
        if self.peek_symbol(&['%']).is_some() {
            self.take(TokenKind::Unit);
            return Ok(Value::Percent {
                ratio: number / Decimal::ONE_HUNDRED,
                signed: false,
            });
        }
        let Some((shown, dim, factor)) = self.read_unit() else {
            return Ok(Value::number(number));
        };
        let mut amount = if dim.has_temperature() {
            number
        } else {
            factor.apply(number)?
        };
        while let Some(Tok::Number(next)) = self.peek().cloned()
            && !dim.has_temperature()
            && self
                .unit_at(self.at + 1)
                .is_some_and(|unit| unit.dim == dim)
        {
            self.take(TokenKind::Number);
            let Some((_, _, factor)) = self.read_unit() else {
                break;
            };
            amount = amount
                .checked_add(factor.apply(next)?)
                .ok_or_else(overflow)?;
        }
        Ok(Value::Quantity(Quantity {
            amount,
            dim,
            unit: Some(shown),
            spans: false,
        }))
    }

    /// A name defined above first, then a value the sheet knows, then a word opening a form.
    fn word(&mut self, word: &str) -> Answer {
        if let Some(value) = self.context.variables.get(word) {
            self.used.push(word.to_string());
            self.take(TokenKind::Name);
            return Ok(value.clone());
        }
        let shifted = |days: i64, today: NaiveDate| {
            let shift = Days::new(days.unsigned_abs());
            if days < 0 {
                today.checked_sub_days(shift)
            } else {
                today.checked_add_days(shift)
            }
        };
        for (day, days) in [
            (CalcWord::Today, 0),
            (CalcWord::Tomorrow, 1),
            (CalcWord::Yesterday, -1),
        ] {
            if self.eat(day) {
                return shifted(days, self.context.today)
                    .map(Value::Date)
                    .ok_or_else(overflow);
            }
        }
        for (block, mean) in [(CalcWord::Total, false), (CalcWord::Mean, true)] {
            if self.eat(block) {
                self.aggregate = true;
                return aggregate(self.context.block, mean);
            }
        }
        if self.eat(CalcWord::From) {
            let from = self.additive()?;
            if !self.eat(CalcWord::To) {
                return Err(self.unreadable());
            }
            let to = self.additive()?;
            return change(from, to).map_err(|error| self.blame(error));
        }
        let at = self.tokens[self.at].start;
        self.kinds[self.at] = Some(TokenKind::Error);
        if words::reserved(word) {
            return Err(CalcError::Unreadable { at });
        }
        Err(CalcError::UnknownName {
            name: word.to_string(),
        })
    }
}

fn mismatch(expected: &Value, found: &Value) -> CalcError {
    CalcError::Mismatch {
        expected: expected.dimension(),
        found: found.dimension(),
        unit: match found {
            Value::Quantity(quantity) => quantity.unit.map(Shown::calc_unit),
            _ => None,
        },
    }
}

/// A temperature added to another is read in the first one's degrees.
fn in_degrees_of(quantity: &Quantity, of: &Quantity) -> Decimal {
    match (quantity.unit, of.unit) {
        (Some(Shown::One(from)), Some(Shown::One(to))) if from != to => {
            from_celsius(to_celsius(quantity.amount, from), to)
        }
        _ => quantity.amount,
    }
}

fn add(left: Value, right: Value, plus: bool) -> Answer {
    let signed = |amount: Decimal| if plus { amount } else { -amount };
    match (left, right) {
        (Value::Quantity(a), Value::Quantity(b)) => {
            if a.dim != b.dim {
                return Err(mismatch(&Value::Quantity(a), &Value::Quantity(b)));
            }
            let right = if a.dim.has_temperature() {
                in_degrees_of(&b, &a)
            } else {
                b.amount
            };
            Ok(Value::Quantity(Quantity {
                amount: a.amount.checked_add(signed(right)).ok_or_else(overflow)?,
                unit: a.unit.or(b.unit),
                dim: a.dim,
                spans: false,
            }))
        }
        // `250 + 20 %` is 300: a percentage added to a quantity is a share of it.
        (Value::Quantity(a), Value::Percent { ratio, .. }) => {
            let factor = Decimal::ONE + signed(ratio);
            Ok(Value::Quantity(Quantity {
                amount: a.amount.checked_mul(factor).ok_or_else(overflow)?,
                ..a
            }))
        }
        (Value::Percent { ratio: a, .. }, Value::Percent { ratio: b, .. }) => Ok(Value::Percent {
            ratio: a + signed(b),
            signed: false,
        }),
        (Value::Date(date), Value::Quantity(shift)) if shift.dim == Dim::TIME => {
            shift_date(date, &shift, plus)
        }
        (Value::Date(a), Value::Date(b)) if !plus => {
            let days = Decimal::from((a - b).num_days());
            Ok(Value::Quantity(Quantity {
                amount: days * Decimal::from(86_400),
                dim: Dim::TIME,
                unit: Some(Shown::One(UnitId::Day)),
                spans: false,
            }))
        }
        (left, right) => Err(mismatch(&left, &right)),
    }
}

/// By calendar months for months and years, by whole days otherwise.
fn shift_date(date: NaiveDate, shift: &Quantity, plus: bool) -> Answer {
    let (unit, count) = match shift.unit {
        Some(Shown::One(UnitId::Month)) => (Decimal::from(2_629_746), true),
        Some(Shown::One(UnitId::Year)) => (Decimal::from(2_629_746 * 12), true),
        _ => (Decimal::from(86_400), false),
    };
    let steps = shift.amount.checked_div(unit).ok_or_else(overflow)?;
    if !steps.fract().is_zero() {
        return Err(CalcError::DateShift);
    }
    let steps = steps.to_i64().ok_or_else(overflow)?;
    let steps = if plus { steps } else { -steps };
    let size = steps.unsigned_abs();
    let moved = match (count, steps >= 0) {
        (true, ahead) => {
            let months = Months::new(u32::try_from(size).map_err(|_| overflow())?);
            if ahead {
                date.checked_add_months(months)
            } else {
                date.checked_sub_months(months)
            }
        }
        (false, true) => date.checked_add_days(Days::new(size)),
        (false, false) => date.checked_sub_days(Days::new(size)),
    };
    moved.map(Value::Date).ok_or_else(overflow)
}

/// What a product is shown in: the unit that already says its kind, or one built from the
/// base units — `€/h × h` is euros.
fn product_unit(a: &Quantity, b: &Quantity, dim: Dim) -> Option<Shown> {
    if dim.is_none() {
        return None;
    }
    if a.dim.is_none() {
        return b.unit;
    }
    if b.dim.is_none() {
        return a.unit;
    }
    shown_for(dim, &[a, b])
}

/// A unit of the quantities at hand that measures `dim`, or one part of a rate that does.
fn shown_for(dim: Dim, among: &[&Quantity]) -> Option<Shown> {
    for quantity in among {
        if quantity.dim == dim {
            return quantity.unit;
        }
    }
    for quantity in among {
        if let Some(Shown::Per(above, below)) = quantity.unit
            && let Some((top, bottom)) = quantity.dim.as_rate()
        {
            if top == dim {
                return Some(Shown::One(above));
            }
            if bottom == dim {
                return Some(Shown::One(below));
            }
        }
    }
    if let Some((top, bottom)) = dim.as_rate() {
        let unit_of = |part: Dim| {
            among
                .iter()
                .find_map(|quantity| match quantity.unit {
                    Some(Shown::One(unit)) if quantity.dim == part => Some(unit),
                    _ => None,
                })
                .or_else(|| base_unit(part))
        };
        return Some(Shown::Per(unit_of(top)?, unit_of(bottom)?));
    }
    base_unit(dim).map(Shown::One)
}

fn multiply(left: Value, right: Value) -> Answer {
    match (left, right) {
        (Value::Quantity(a), Value::Quantity(b)) => {
            if (a.dim.has_temperature() && !b.dim.is_none())
                || (b.dim.has_temperature() && !a.dim.is_none())
            {
                return Err(CalcError::Unsupported);
            }
            let dim = a.dim.times(b.dim);
            Ok(Value::Quantity(Quantity {
                amount: a.amount.checked_mul(b.amount).ok_or_else(overflow)?,
                unit: product_unit(&a, &b, dim),
                dim,
                spans: false,
            }))
        }
        (Value::Quantity(a), Value::Percent { ratio, .. })
        | (Value::Percent { ratio, .. }, Value::Quantity(a)) => Ok(Value::Quantity(Quantity {
            amount: a.amount.checked_mul(ratio).ok_or_else(overflow)?,
            ..a
        })),
        (Value::Percent { ratio: a, .. }, Value::Percent { ratio: b, .. }) => Ok(Value::Percent {
            ratio: a.checked_mul(b).ok_or_else(overflow)?,
            signed: false,
        }),
        (left, right) => Err(mismatch(&left, &right)),
    }
}

fn divide(left: Value, right: Value) -> Answer {
    let by = |amount: Decimal, divisor: Decimal| {
        if divisor.is_zero() {
            Err(CalcError::DivisionByZero)
        } else {
            amount.checked_div(divisor).ok_or_else(overflow)
        }
    };
    match (left, right) {
        (Value::Quantity(a), Value::Quantity(b)) => {
            if (a.dim.has_temperature() && !b.dim.is_none()) || b.dim.has_temperature() {
                return Err(CalcError::Unsupported);
            }
            let dim = a.dim.over(b.dim);
            let unit = if dim.is_none() {
                None
            } else if b.dim.is_none() {
                a.unit
            } else if dim == Dim::TIME {
                // Data at a rate, money at an hourly price: a duration, shown in its own size.
                None
            } else {
                shown_for(dim, &[&a, &b])
            };
            Ok(Value::Quantity(Quantity {
                amount: by(a.amount, b.amount)?,
                dim,
                unit,
                spans: false,
            }))
        }
        (Value::Quantity(a), Value::Percent { ratio, .. }) => Ok(Value::Quantity(Quantity {
            amount: by(a.amount, ratio)?,
            ..a
        })),
        (Value::Percent { ratio: a, .. }, Value::Percent { ratio: b, .. }) => {
            Ok(Value::number(by(a, b)?))
        }
        (left, right) => Err(mismatch(&left, &right)),
    }
}

/// `300 hors 20 %` is 250: the amount before the share was added.
fn excluding(left: Value, right: Value) -> Answer {
    match (left, right) {
        (Value::Quantity(a), Value::Percent { ratio, .. }) => {
            let divisor = Decimal::ONE + ratio;
            if divisor.is_zero() {
                return Err(CalcError::DivisionByZero);
            }
            Ok(Value::Quantity(Quantity {
                amount: a.amount.checked_div(divisor).ok_or_else(overflow)?,
                ..a
            }))
        }
        (left, right) => Err(mismatch(&left, &right)),
    }
}

/// `30 sur 120` is 25 %.
fn ratio(left: Value, right: Value) -> Answer {
    match (&left, &right) {
        (Value::Quantity(a), Value::Quantity(b)) if a.dim == b.dim => {
            if b.amount.is_zero() {
                return Err(CalcError::DivisionByZero);
            }
            let (part, whole) = if a.dim.has_temperature() {
                (a.amount, in_degrees_of(b, a))
            } else {
                (a.amount, b.amount)
            };
            Ok(Value::Percent {
                ratio: part.checked_div(whole).ok_or_else(overflow)?,
                signed: false,
            })
        }
        _ => Err(mismatch(&left, &right)),
    }
}

/// `de 80 à 100` is +25 %.
fn change(from: Value, to: Value) -> Answer {
    match (&from, &to) {
        (Value::Quantity(a), Value::Quantity(b)) if a.dim == b.dim => {
            if a.amount.is_zero() {
                return Err(CalcError::DivisionByZero);
            }
            let ratio = (b.amount - a.amount)
                .checked_div(a.amount)
                .ok_or_else(overflow)?;
            Ok(Value::Percent {
                ratio,
                signed: true,
            })
        }
        _ => Err(mismatch(&from, &to)),
    }
}

fn power(base: Value, exponent: Value) -> Answer {
    let Value::Quantity(exponent) = exponent else {
        return Err(CalcError::Unsupported);
    };
    if !exponent.dim.is_none()
        || !exponent.amount.fract().is_zero()
        || exponent.amount.abs() > Decimal::from(64)
    {
        return Err(CalcError::Unsupported);
    }
    let Value::Quantity(base) = base else {
        return Err(CalcError::Unsupported);
    };
    if !base.dim.is_none() {
        return Err(CalcError::Unsupported);
    }
    let count = exponent
        .amount
        .to_i64()
        .ok_or_else(overflow)?
        .unsigned_abs();
    let mut result = Decimal::ONE;
    for _ in 0..count {
        result = result.checked_mul(base.amount).ok_or_else(overflow)?;
    }
    if exponent.amount.is_sign_negative() {
        if result.is_zero() {
            return Err(CalcError::DivisionByZero);
        }
        result = Decimal::ONE.checked_div(result).ok_or_else(overflow)?;
    }
    Ok(Value::number(result))
}

fn negate(value: Value) -> Answer {
    match value {
        Value::Quantity(quantity) => Ok(Value::Quantity(Quantity {
            amount: -quantity.amount,
            ..quantity
        })),
        Value::Percent { ratio, signed } => Ok(Value::Percent {
            ratio: -ratio,
            signed,
        }),
        Value::Date(_) => Err(CalcError::Unsupported),
    }
}

/// ⚠️ Euros are never turned into dollars: no rate is fetched, the sheet works offline.
fn convert(value: Value, shown: Shown, dim: Dim) -> Answer {
    let Value::Quantity(quantity) = value else {
        return Err(CalcError::NotConvertible {
            from: value.dimension(),
            to: Some(shown.calc_unit()),
        });
    };
    if quantity.dim != dim {
        return Err(if quantity.dim.is_currency() && dim.is_currency() {
            CalcError::Currency
        } else {
            CalcError::NotConvertible {
                from: quantity.dim.dimension(),
                to: Some(shown.calc_unit()),
            }
        });
    }
    let amount = match (quantity.dim.has_temperature(), quantity.unit, shown) {
        (true, Some(Shown::One(from)), Shown::One(to)) => {
            from_celsius(to_celsius(quantity.amount, from), to)
        }
        _ => quantity.amount,
    };
    Ok(Value::Quantity(Quantity {
        amount,
        dim,
        unit: Some(shown),
        spans: matches!(
            shown,
            Shown::One(UnitId::Week | UnitId::Day | UnitId::Hour | UnitId::Minute)
        ),
    }))
}

/// The quantities above in the block, all of one kind: euros are not added to gigabytes.
fn aggregate(block: &[Quantity], mean: bool) -> Answer {
    let Some(first) = block.first() else {
        return Err(CalcError::NothingAbove);
    };
    let mut sum = Decimal::ZERO;
    for quantity in block {
        if quantity.dim != first.dim {
            return Err(mismatch(
                &Value::Quantity(first.clone()),
                &Value::Quantity(quantity.clone()),
            ));
        }
        let amount = if first.dim.has_temperature() {
            in_degrees_of(quantity, first)
        } else {
            quantity.amount
        };
        sum = sum.checked_add(amount).ok_or_else(overflow)?;
    }
    let amount = if mean {
        sum.checked_div(Decimal::from(block.len()))
            .ok_or_else(overflow)?
    } else {
        sum
    };
    Ok(Value::Quantity(Quantity {
        amount,
        dim: first.dim,
        unit: first.unit,
        spans: false,
    }))
}
