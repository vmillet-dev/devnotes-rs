//! One line read and computed, with what the lines above it named. A recursive descent:
//! a conversion over sums over products over powers over a value.

use std::collections::HashMap;

use chrono::{Days, Months, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::lexer::{Tok, Token};
use super::maths::{self, Bits, Rounding};
use super::units::{Css, Dim, Dimension, UnitDef, UnitId, base_unit, from_celsius, to_celsius};
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
    /// What the last line above that gave a result gave, for `préc`.
    pub(super) previous: Option<&'a Value>,
    pub(super) css: Css,
}

#[derive(Clone, Copy)]
enum Product {
    Times,
    Over,
    Modulo,
}

/// How a plain number is written: `en hex`, `en sci`, `int(…)` back to decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Notation {
    Base(u32),
    Scientific,
    Decimal,
    /// `2/10 en fraction`: `1/5`.
    Fraction,
    /// `20/5 en multiple`: `×4`.
    Multiplier,
}

/// What a share is taken against: `de`/`of`, `de plus sur`/`on`, `de moins sur`/`off`.
#[derive(Clone, Copy)]
enum Share {
    Of,
    On,
    Off,
}

#[derive(Clone, Copy)]
enum Function {
    Root,
    Cbrt,
    Log,
    Ln,
    Abs,
    Round,
    Ceil,
    Floor,
    Written(Notation),
}

/// Longest spelling first: `racine cubique` before `racine`.
const FUNCTIONS: [(CalcWord, Function); 12] = [
    (CalcWord::Cbrt, Function::Cbrt),
    (CalcWord::Root, Function::Root),
    (CalcWord::Log, Function::Log),
    (CalcWord::Ln, Function::Ln),
    (CalcWord::Abs, Function::Abs),
    (CalcWord::Round, Function::Round),
    (CalcWord::Ceil, Function::Ceil),
    (CalcWord::Floor, Function::Floor),
    (CalcWord::Hex, Function::Written(Notation::Base(16))),
    (CalcWord::Bin, Function::Written(Notation::Base(2))),
    (CalcWord::Oct, Function::Written(Notation::Base(8))),
    (CalcWord::Int, Function::Written(Notation::Decimal)),
];

const NOTATIONS: [(CalcWord, Notation); 7] = [
    (CalcWord::Hex, Notation::Base(16)),
    (CalcWord::Bin, Notation::Base(2)),
    (CalcWord::Oct, Notation::Base(8)),
    (CalcWord::Sci, Notation::Scientific),
    (CalcWord::Int, Notation::Decimal),
    (CalcWord::AsDecimal, Notation::Decimal),
    (CalcWord::Fraction, Notation::Fraction),
];

fn plain(value: &Value) -> Result<Decimal, CalcError> {
    match value {
        Value::Quantity(quantity) if quantity.dim.is_none() => Ok(quantity.amount),
        other => Err(CalcError::Mismatch {
            expected: Dimension::Number,
            found: other.dimension(),
            unit: match other {
                Value::Quantity(quantity) => quantity.unit.map(Shown::calc_unit),
                _ => None,
            },
        }),
    }
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
    /// `en hex`, `en sci`: how the line's number is written.
    pub(super) notation: Option<Notation>,
    /// Inside `le plus grand de 3 et 5`, where `et` separates rather than adds.
    listing: bool,
    /// Reading a loan's amount, where `sur 6 ans` is its length, not a ratio.
    no_over: bool,
    /// Where the line ends, for a value missing there.
    end: u32,
}

type Answer = Result<Value, CalcError>;

fn overflow() -> CalcError {
    CalcError::Overflow
}

impl<'a> Parser<'a> {
    pub(super) fn new(context: &'a Context<'a>, tokens: &'a [Token], end: u32) -> Self {
        let mut parser = Self {
            context,
            tokens,
            at: 0,
            kinds: vec![None; tokens.len()],
            used: Vec::new(),
            converting: None,
            aggregate: false,
            notation: None,
            listing: false,
            no_over: false,
            end,
        };
        parser.skip_quoted();
        parser
    }

    pub(super) fn whole(&mut self) -> Answer {
        let value = self.conversion()?;
        self.skip_remarks();
        if self.at < self.tokens.len() {
            if !self.prose() {
                return Err(self.unreadable());
            }
            while self.at < self.tokens.len() {
                self.take(TokenKind::Comment);
            }
        }
        Ok(value)
    }

    /// `275 € pour le "Modèle 227"`: the words after a result, up to a quoted text, are left
    /// out. Words alone are not: `5 jours ouvrés` says it is not read rather than give 5 days.
    fn prose(&self) -> bool {
        let rest = &self.tokens[self.at..];
        rest.iter().any(|token| token.tok == Tok::Quoted)
            && rest
                .iter()
                .all(|token| matches!(token.tok, Tok::Word(_) | Tok::Other(_) | Tok::Quoted))
    }

    fn skip_quoted(&mut self) {
        while self
            .tokens
            .get(self.at)
            .is_some_and(|token| token.tok == Tok::Quoted)
        {
            self.kinds[self.at] = Some(TokenKind::Comment);
            self.at += 1;
        }
    }

    /// `999 € (pour l'iPhone)`: parentheses after a value holding words and no name defined
    /// above are a remark, read past; holding an expression, they multiply.
    fn remark_end(&self) -> Option<usize> {
        self.peek_symbol(&['('])?;
        let inside = self.at + 1;
        let close = inside
            + self.tokens[inside..]
                .iter()
                .position(|token| token.tok == Tok::Symbol(')'))?;
        let words = self.tokens[inside..close]
            .iter()
            .all(|token| match &token.tok {
                Tok::Word(word) => !self.context.variables.contains_key(word),
                Tok::Other(_) | Tok::Quoted => true,
                _ => false,
            });
        (close > inside && words).then_some(close + 1)
    }

    fn skip_remarks(&mut self) {
        while let Some(end) = self.remark_end() {
            while self.at < end {
                self.take(TokenKind::Comment);
            }
        }
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
        self.skip_quoted();
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
        self.eat_as(word, TokenKind::Keyword)
    }

    fn eat_as(&mut self, word: CalcWord, kind: TokenKind) -> bool {
        let Some(count) = self.sees(word) else {
            return false;
        };
        for _ in 0..count {
            self.take(kind);
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

    fn notation_at(&self, index: usize) -> bool {
        self.tokens
            .get(index)
            .is_some_and(|token| token.tok == Tok::Symbol('%'))
            || NOTATIONS
                .iter()
                .map(|(word, _)| *word)
                .chain([CalcWord::Base])
                .any(|word| words::spelled(word, self.tokens, index).is_some())
    }

    /// A screen unit's size follows the sheet's `ppi`, `em` and `rem`.
    fn factor_of(&self, unit: &UnitDef) -> Decimal {
        self.context.css.factor(unit.id).unwrap_or(unit.factor)
    }

    fn unit_factor(&self, unit: UnitId) -> Decimal {
        self.context
            .units
            .iter()
            .find(|def| def.id == unit)
            .map_or(Decimal::ONE, |def| self.factor_of(def))
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
                above: self.factor_of(first),
                below: self.factor_of(second),
            };
            return Some((
                Shown::Per(first.id, second.id),
                first.dim.over(second.dim),
                factor,
            ));
        }
        let factor = Factor {
            above: self.factor_of(first),
            below: Decimal::ONE,
        };
        Some((Shown::One(first.id), first.dim, factor))
    }

    fn conversion(&mut self) -> Answer {
        let mut value = self.bits_or()?;
        loop {
            if let Some(rounded) = self.rounding(&value)? {
                value = rounded;
            } else if self.eat(CalcWord::WhatPercent) {
                // `20 c'est combien de % de 200`, `180 is what % off 200`.
                let Some(share) = self.share() else {
                    return Err(self.unreadable());
                };
                let whole = self.additive()?;
                value = share_of(value, whole, share).map_err(|error| self.blame(error))?;
            } else if self.eat(CalcWord::IsTo) {
                value = self.proportion(value)?;
            } else if self.duration_after(CalcWord::After) || self.duration_after(CalcWord::During)
            {
                value = self.compound(value)?;
            } else if self.eat(CalcWord::Invested) {
                // `500 € investis 1 500 € récupérés` is ×3.
                let returned = self.principal()?;
                if !self.eat(CalcWord::Returned) {
                    return Err(self.unreadable());
                }
                let times = divide(returned, value).map_err(|error| self.blame(error))?;
                value = self.written(times, Notation::Multiplier)?;
            } else if self.eat(CalcWord::Convert) {
                self.converting = Some(value.dimension());
                value = self.convert(value)?;
            } else if self.eat(CalcWord::At) {
                self.converting = Some(value.dimension());
                // A unit alone is where to convert to; a quantity, the rate to divide by.
                if self.peek_symbol(&['%']).is_some() || self.unit_at(self.at).is_some() {
                    value = self.convert(value)?;
                } else {
                    let rate = self.additive()?;
                    if let Some(changed) = self.change_asked(&value, &rate)? {
                        value = changed;
                        continue;
                    }
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

    /// `arrondi`, `arrondi supérieur au 5 près`, `au millier près`, `à 2 décimales`: the
    /// value itself is rounded, so the lines below read it rounded.
    fn rounding(&mut self, value: &Value) -> Result<Option<Value>, CalcError> {
        let direction = if self.eat(CalcWord::RoundedUp) {
            Some(Rounding::Up)
        } else if self.eat(CalcWord::RoundedDown) {
            Some(Rounding::Down)
        } else if self.eat(CalcWord::Rounded) {
            Some(Rounding::Nearest)
        } else {
            None
        };
        let step = self.nearest()?;
        if direction.is_some() || step.is_some() {
            let (direction, step) = (
                direction.unwrap_or(Rounding::Nearest),
                step.unwrap_or(Decimal::ONE),
            );
            let rounded = self.in_its_unit(value.clone(), |amount| {
                maths::round_to(amount, step, direction)
            });
            return rounded.map(Some).map_err(|error| self.blame(error));
        }
        let Some(places) = self.places() else {
            return Ok(None);
        };
        Ok(Some(self.in_its_unit(value.clone(), |amount| {
            Ok(maths::round_places(amount, places))
        })?))
    }

    /// `au 10 près`, `au millier près`, `to nearest 10`: the step to round to.
    fn nearest(&mut self) -> Result<Option<Decimal>, CalcError> {
        if !self.eat(CalcWord::Nearest) {
            return Ok(None);
        }
        let mut step = None;
        if let Some(Tok::Number(number)) = self.peek().cloned() {
            self.take(TokenKind::Number);
            step = Some(number);
        }
        while let Some((scale, taken)) = words::scale_at(self.tokens, self.at) {
            for _ in 0..taken {
                self.take(TokenKind::Number);
            }
            step = Some(step.unwrap_or(Decimal::ONE) * Decimal::from(scale));
        }
        let Some(step) = step else {
            return Err(self.unreadable());
        };
        self.eat(CalcWord::Near);
        Ok(Some(step))
    }

    /// `à 2 décimales`, `to 2 dp`, `à 5 chiffres`: how many places, read only when the whole
    /// form is there — `à 100 Mo/s` stays a rate.
    fn places(&mut self) -> Option<u32> {
        let opener = [CalcWord::At, CalcWord::Convert, CalcWord::To]
            .into_iter()
            .find_map(|word| self.sees(word))?;
        let Some(Tok::Number(places)) = self.tokens.get(self.at + opener).map(|token| &token.tok)
        else {
            return None;
        };
        let places = places.to_u32().filter(|_| places.fract().is_zero())?;
        let after = self.at + opener + 1;
        let closer = words::spelled(CalcWord::Decimals, self.tokens, after)
            .or_else(|| words::spelled(CalcWord::Digits, self.tokens, after))?;
        for _ in 0..opener {
            self.take(TokenKind::Keyword);
        }
        self.take(TokenKind::Number);
        for _ in 0..closer {
            self.take(TokenKind::Keyword);
        }
        Some(places)
    }

    /// Rounded in the unit it is shown in: `1,236 km à 2 décimales` is 1,24 km, not 1 236 m.
    fn in_its_unit(
        &self,
        value: Value,
        round: impl Fn(Decimal) -> Result<Decimal, CalcError>,
    ) -> Answer {
        match value {
            Value::Quantity(quantity) => {
                let (above, below) = match quantity.unit {
                    _ if quantity.dim.has_temperature() => (Decimal::ONE, Decimal::ONE),
                    Some(Shown::One(unit)) => (self.unit_factor(unit), Decimal::ONE),
                    Some(Shown::Per(top, bottom)) => {
                        (self.unit_factor(top), self.unit_factor(bottom))
                    }
                    None => (Decimal::ONE, Decimal::ONE),
                };
                let shown = quantity
                    .amount
                    .checked_mul(below)
                    .and_then(|product| product.checked_div(above))
                    .ok_or_else(overflow)?;
                let amount = round(shown)?
                    .checked_mul(above)
                    .and_then(|product| product.checked_div(below))
                    .ok_or_else(overflow)?;
                Ok(Value::Quantity(Quantity { amount, ..quantity }))
            }
            Value::Percent { ratio, signed } => {
                let percent = round(ratio * Decimal::ONE_HUNDRED)?;
                Ok(Value::Percent {
                    ratio: percent / Decimal::ONE_HUNDRED,
                    signed,
                })
            }
            Value::Date(_) => Err(CalcError::Unsupported),
        }
    }

    /// `50 en % de 100`, `70 as a % on 20`: the `%` taken, then what the share is taken against.
    fn share_after_percent(&mut self) -> Option<Share> {
        let next = self.at + 1;
        let ahead = [CalcWord::PercentOn, CalcWord::PercentOff, CalcWord::Of]
            .into_iter()
            .any(|word| words::spelled(word, self.tokens, next).is_some());
        if !ahead {
            return None;
        }
        self.take(TokenKind::Unit);
        self.share()
    }

    /// `de plus sur`, `de moins sur`, `de` — the longest first, `de` being the start of the others.
    fn share(&mut self) -> Option<Share> {
        if self.eat(CalcWord::PercentOn) {
            Some(Share::On)
        } else if self.eat(CalcWord::PercentOff) {
            Some(Share::Off)
        } else if self.eat(CalcWord::Of) {
            Some(Share::Of)
        } else {
            None
        }
    }

    /// `50 à 75 c'est combien de %`, `40 to 90 as %`, `50 to 75 is what x`: a change asked
    /// after two values.
    fn change_asked(&mut self, from: &Value, to: &Value) -> Result<Option<Value>, CalcError> {
        let percent = self.eat(CalcWord::WhatPercent)
            || (self.sees(CalcWord::Convert).is_some_and(|taken| {
                self.tokens
                    .get(self.at + taken)
                    .is_some_and(|token| token.tok == Tok::Symbol('%'))
            }) && {
                self.eat(CalcWord::Convert);
                self.take(TokenKind::Unit);
                true
            });
        if percent {
            return change(from.clone(), to.clone())
                .map(Some)
                .map_err(|error| self.blame(error));
        }
        if self.eat(CalcWord::WhatTimes) {
            let times = divide(to.clone(), from.clone()).map_err(|error| self.blame(error))?;
            return self.written(times, Notation::Multiplier).map(Some);
        }
        Ok(None)
    }

    /// `6 est à 60 ce que 8 est à combien`, `5 is to 10 as what is to 80`: the unknown of a
    /// proportion, its third or its fourth.
    fn proportion(&mut self, first: Value) -> Answer {
        let second = self.additive()?;
        if !self.eat(CalcWord::Like) {
            return Err(self.unreadable());
        }
        let third = if self.eat(CalcWord::What) {
            None
        } else {
            Some(self.additive()?)
        };
        if !self.eat(CalcWord::IsTo) {
            return Err(self.unreadable());
        }
        let fourth = if self.eat(CalcWord::What) {
            None
        } else {
            Some(self.additive()?)
        };
        let found = match (third, fourth) {
            // The ratio first, so the unknown keeps the unit of the value it scales.
            (Some(third), None) => divide(third, first).and_then(|ratio| multiply(ratio, second)),
            (None, Some(fourth)) => divide(fourth, second).and_then(|ratio| multiply(ratio, first)),
            _ => return Err(self.unreadable()),
        };
        found.map_err(|error| self.blame(error))
    }

    /// `255 en hex`: a plain number, whole for a base, written another way.
    fn written(&mut self, value: Value, notation: Notation) -> Answer {
        // `50 % en fraction` is 1/2: a percentage written as the number it is.
        let value = match (value, notation) {
            (
                Value::Percent { ratio, .. },
                Notation::Fraction | Notation::Multiplier | Notation::Decimal,
            ) => Value::number(ratio),
            (value, _) => value,
        };
        let amount = plain(&value).map_err(|error| self.blame(error))?;
        if let Notation::Base(_) = notation {
            maths::whole(amount).map_err(|error| self.blame(error))?;
        }
        self.notation = Some(notation);
        Ok(value)
    }

    fn convert(&mut self, value: Value) -> Answer {
        // `50 to 75 is what %`: `to` there is the second value, not where to convert.
        if let Some(Tok::Number(_)) = self.peek() {
            let to = self.additive()?;
            return match self.change_asked(&value, &to)? {
                Some(changed) => Ok(changed),
                None => Err(self.unreadable()),
            };
        }
        if self.eat(CalcWord::Multiple) {
            let times = if self.eat(CalcWord::Of) {
                let unit = self.additive()?;
                divide(value, unit).map_err(|error| self.blame(error))?
            } else {
                value
            };
            return self.written(times, Notation::Multiplier);
        }
        let article = self.sees(CalcWord::Article).filter(|taken| {
            self.tokens
                .get(self.at + taken)
                .is_some_and(|token| token.tok == Tok::Symbol('%'))
        });
        if article.is_some() {
            self.eat(CalcWord::Article);
        }
        if self.peek_symbol(&['%']).is_some()
            && let Some(share) = self.share_after_percent()
        {
            let whole = self.additive()?;
            return share_of(value, whole, share).map_err(|error| self.blame(error));
        }
        for (word, notation) in NOTATIONS {
            if self.eat(word) {
                return self.written(value, notation);
            }
        }
        if self.eat(CalcWord::Base) {
            let Some(Tok::Number(radix)) = self.peek().cloned() else {
                return Err(self.unreadable());
            };
            self.take(TokenKind::Number);
            let notation = match radix.to_u32().filter(|_| radix.fract().is_zero()) {
                Some(radix @ (2 | 8 | 16)) => Notation::Base(radix),
                Some(10) => Notation::Decimal,
                _ => return Err(self.blame(CalcError::Unsupported)),
            };
            return self.written(value, notation);
        }
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

    fn bits(&mut self, left: Value, right: Value, operation: Bits) -> Answer {
        let both = plain(&left).and_then(|a| Ok((a, plain(&right)?)));
        both.and_then(|(a, b)| maths::bits(a, b, operation))
            .map(Value::number)
            .map_err(|error| self.blame(error))
    }

    /// `5 | 3`, below `xor`, below `&`, below the shifts, below the sums: as C orders them.
    fn bits_or(&mut self) -> Answer {
        let mut left = self.bits_xor()?;
        while self.peek_symbol(&['|']).is_some() {
            self.take(TokenKind::Operator);
            let right = self.bits_xor()?;
            left = self.bits(left, right, Bits::Or)?;
        }
        Ok(left)
    }

    fn bits_xor(&mut self) -> Answer {
        let mut left = self.bits_and()?;
        while self.eat_as(CalcWord::Xor, TokenKind::Operator) {
            let right = self.bits_and()?;
            left = self.bits(left, right, Bits::Xor)?;
        }
        Ok(left)
    }

    fn bits_and(&mut self) -> Answer {
        let mut left = self.shift()?;
        while self.peek_symbol(&['&']).is_some() {
            self.take(TokenKind::Operator);
            let right = self.shift()?;
            left = self.bits(left, right, Bits::And)?;
        }
        Ok(left)
    }

    /// `<<` or `>>`, its two signs side by side.
    fn shift_ahead(&self) -> Option<Bits> {
        let symbol = self.peek_symbol(&['<', '>'])?;
        let second = self.tokens.get(self.at + 1)?;
        (second.tok == Tok::Symbol(symbol) && !second.spaced).then_some(if symbol == '<' {
            Bits::Left
        } else {
            Bits::Right
        })
    }

    fn shift(&mut self) -> Answer {
        let mut left = self.additive()?;
        while let Some(operation) = self.shift_ahead() {
            self.take(TokenKind::Operator);
            self.take(TokenKind::Operator);
            let right = self.additive()?;
            left = self.bits(left, right, operation)?;
        }
        Ok(left)
    }

    fn additive(&mut self) -> Answer {
        let mut left = self.term()?;
        loop {
            let plus = if let Some(symbol) = self.peek_symbol(&['+', '-', '−']) {
                self.take(TokenKind::Operator);
                symbol == '+'
            } else if self.eat_as(CalcWord::Plus, TokenKind::Operator)
                || (!self.listing && self.eat_as(CalcWord::And, TokenKind::Operator))
            {
                true
            } else if self.eat_as(CalcWord::Minus, TokenKind::Operator) {
                false
            } else {
                return Ok(left);
            };
            let right = self.term()?;
            left = add(left, right, plus).map_err(|error| self.blame(error))?;
        }
    }

    fn product(&mut self) -> Option<Product> {
        if let Some(symbol) = self.peek_symbol(&['×', '*', '/', '÷']) {
            self.take(TokenKind::Operator);
            return Some(if matches!(symbol, '×' | '*') {
                Product::Times
            } else {
                Product::Over
            });
        }
        if self.eat_as(CalcWord::Times, TokenKind::Operator) {
            Some(Product::Times)
        } else if self.eat_as(CalcWord::Divide, TokenKind::Operator) {
            Some(Product::Over)
        } else if self.eat_as(CalcWord::Modulo, TokenKind::Operator) {
            Some(Product::Modulo)
        } else {
            // `6 (3)` is 18: a parenthesis right after a value multiplies it.
            self.peek_symbol(&['(']).map(|_| Product::Times)
        }
    }

    fn term(&mut self) -> Answer {
        let mut left = self.power()?;
        loop {
            self.skip_remarks();
            if let Some(product) = self.product() {
                let right = self.power()?;
                left = match product {
                    Product::Times => multiply(left, right),
                    Product::Over => divide(left, right),
                    Product::Modulo => modulo(left, right),
                }
                .map_err(|error| self.blame(error))?;
            } else if self.eat(CalcWord::Excluding) {
                let right = self.power()?;
                left = excluding(left, right).map_err(|error| self.blame(error))?;
            } else if let Value::Percent { ratio, .. } = left
                && let Some(share) = self.share()
            {
                // `5 % de combien font 6` solves for the whole; `5 % de plus sur 200` applies.
                let solving = self.eat(CalcWord::What);
                if solving && !self.eat(CalcWord::Is) {
                    return Err(self.unreadable());
                }
                let right = self.power()?;
                left = if solving {
                    solve(ratio, share, right)
                } else {
                    applied(ratio, share, right)
                }
                .map_err(|error| self.blame(error))?;
            } else if !self.no_over && self.eat(CalcWord::Over) {
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
            Tok::Symbol('$' | '€')
                if self
                    .tokens
                    .get(self.at + 1)
                    .is_some_and(|next| !next.spaced && matches!(next.tok, Tok::Number(_))) =>
            {
                self.prefixed()
            }
            Tok::Symbol(_) | Tok::Other(_) | Tok::Quoted => Err(self.unreadable()),
        }
    }

    /// A number, its unit, and the quantities of the same kind written after it: `3 h 20 min`.
    fn quantity(&mut self, number: Decimal) -> Answer {
        let mut number = number;
        while let Some((scale, taken)) = words::scale_at(self.tokens, self.at) {
            for _ in 0..taken {
                self.take(TokenKind::Number);
            }
            number = number
                .checked_mul(Decimal::from(scale))
                .ok_or_else(overflow)?;
        }
        if self.peek_symbol(&['%']).is_some() {
            self.take(TokenKind::Unit);
            return Ok(Value::Percent {
                ratio: number / Decimal::ONE_HUNDRED,
                signed: false,
            });
        }
        // `255 in hex`, `0,35 in %`: `in` before what a number is written as converts it, inches aside.
        if self.sees(CalcWord::Convert) == Some(1) && self.notation_at(self.at + 1) {
            return Ok(Value::number(number));
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

    /// `$1,000`, `€500`: a currency written before its amount, as English does.
    fn prefixed(&mut self) -> Answer {
        let Some(unit) = self.unit_at(self.at) else {
            return Err(self.unreadable());
        };
        self.take(TokenKind::Unit);
        let Some(Tok::Number(mut number)) = self.peek().cloned() else {
            return Err(self.unreadable());
        };
        self.take(TokenKind::Number);
        while let Some((scale, taken)) = words::scale_at(self.tokens, self.at) {
            for _ in 0..taken {
                self.take(TokenKind::Number);
            }
            number = number
                .checked_mul(Decimal::from(scale))
                .ok_or_else(overflow)?;
        }
        Ok(Value::Quantity(Quantity {
            amount: number
                .checked_mul(self.factor_of(unit))
                .ok_or_else(overflow)?,
            dim: unit.dim,
            unit: Some(Shown::One(unit.id)),
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
        // Before `total`, which `total repayment on` starts with.
        if let Some(found) = self.finance()? {
            return Ok(found);
        }
        if self.eat(CalcWord::Previous) {
            return match self.context.previous {
                Some(value) => Ok(value.clone()),
                None => Err(self.blame(CalcError::NoPrevious)),
            };
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
        let phi = "1.6180339887498948482045868344"
            .parse()
            .unwrap_or(Decimal::ONE);
        for (constant, value) in [
            (CalcWord::Pi, Decimal::PI),
            (CalcWord::E, Decimal::E),
            (CalcWord::Tau, Decimal::TWO_PI),
            (CalcWord::Phi, phi),
        ] {
            if self.eat(constant) {
                return Ok(Value::number(value));
            }
        }
        for (word, function) in FUNCTIONS {
            if self.eat(word) {
                return self.call(function);
            }
        }
        for (word, largest) in [(CalcWord::Largest, true), (CalcWord::Smallest, false)] {
            if self.eat(word) {
                let listing = std::mem::replace(&mut self.listing, true);
                let found = self.extreme(largest);
                self.listing = listing;
                return found;
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

impl Parser<'_> {
    /// `sqrt(16)`, `sqrt 16`, `racine 2 (8)`, `log 2 (8)`: the argument in parentheses, or the
    /// value after the word; a bare number before parentheses is a degree or a base.
    fn call(&mut self, function: Function) -> Answer {
        let degree = match self.peek().cloned() {
            Some(Tok::Number(number))
                if matches!(function, Function::Root | Function::Log)
                    && self
                        .tokens
                        .get(self.at + 1)
                        .is_some_and(|token| token.tok == Tok::Symbol('(')) =>
            {
                self.take(TokenKind::Number);
                Some(number)
            }
            _ => None,
        };
        let argument = if self.peek_symbol(&['(']).is_some() {
            self.primary()?
        } else {
            self.power()?
        };
        let number = |value: &Value| plain(value);
        let result = match function {
            Function::Root => {
                let degree = match degree {
                    None => 2,
                    Some(degree) => degree
                        .to_u32()
                        .filter(|_| degree.fract().is_zero())
                        .ok_or(CalcError::Unsupported)
                        .map_err(|error| self.blame(error))?,
                };
                number(&argument)
                    .and_then(|x| maths::root(x, degree))
                    .map(Value::number)
            }
            Function::Cbrt => number(&argument)
                .and_then(|x| maths::root(x, 3))
                .map(Value::number),
            Function::Log => number(&argument)
                .and_then(|x| maths::log(x, degree))
                .map(Value::number),
            Function::Ln => number(&argument).and_then(maths::ln).map(Value::number),
            Function::Abs => match argument {
                Value::Quantity(quantity) if !quantity.dim.has_temperature() => {
                    Ok(Value::Quantity(Quantity {
                        amount: quantity.amount.abs(),
                        ..quantity
                    }))
                }
                Value::Percent { ratio, signed } => Ok(Value::Percent {
                    ratio: ratio.abs(),
                    signed,
                }),
                _ => Err(CalcError::Unsupported),
            },
            Function::Round | Function::Ceil | Function::Floor => {
                let direction = match function {
                    Function::Ceil => Rounding::Up,
                    Function::Floor => Rounding::Down,
                    _ => Rounding::Nearest,
                };
                self.in_its_unit(argument, |amount| {
                    maths::round_to(amount, Decimal::ONE, direction)
                })
            }
            Function::Written(notation) => return self.written(argument, notation),
        };
        result.map_err(|error| self.blame(error))
    }

    /// `le plus grand de 100 et 200`, `smaller of 5 and 10`: of one kind, the unit kept.
    fn extreme(&mut self, largest: bool) -> Answer {
        let mut found = self.additive()?;
        loop {
            let comma = self.peek() == Some(&Tok::Other(','));
            if comma {
                self.take(TokenKind::Operator);
            } else if !self.eat_as(CalcWord::And, TokenKind::Operator) {
                return Ok(found);
            }
            let next = self.additive()?;
            let order = match (&found, &next) {
                (Value::Quantity(a), Value::Quantity(b)) if a.dim == b.dim => {
                    let b_amount = if a.dim.has_temperature() {
                        in_degrees_of(b, a)
                    } else {
                        b.amount
                    };
                    a.amount.cmp(&b_amount)
                }
                (Value::Percent { ratio: a, .. }, Value::Percent { ratio: b, .. }) => a.cmp(b),
                (Value::Date(a), Value::Date(b)) => a.cmp(b),
                _ => return Err(self.blame(mismatch(&found, &next))),
            };
            let keep_next = if largest {
                order.is_lt()
            } else {
                order.is_gt()
            };
            if keep_next {
                found = next;
            }
        }
    }
}

/// The calendar's mean year, as the duration units have it.
const YEAR_SECONDS: i64 = 31_556_952;

impl Parser<'_> {
    /// `après 3 ans`, `for 3 years`: the word taken only before a number, `for the` being prose.
    fn duration_after(&mut self, word: CalcWord) -> bool {
        let number = self.sees(word).is_some_and(|taken| {
            self.tokens
                .get(self.at + taken)
                .is_some_and(|token| matches!(token.tok, Tok::Number(_)))
        });
        number && self.eat(word)
    }

    /// An amount read up to the words of a form: `sur 6 ans` is a loan's length there.
    fn principal(&mut self) -> Answer {
        let over = std::mem::replace(&mut self.no_over, true);
        let found = self.additive();
        self.no_over = over;
        found
    }

    /// `3 ans`, `36 mois`: a duration in years.
    fn years(&mut self) -> Result<Decimal, CalcError> {
        match self.principal()? {
            Value::Quantity(quantity) if quantity.dim == Dim::TIME => quantity
                .amount
                .checked_div(Decimal::from(YEAR_SECONDS))
                .ok_or_else(overflow),
            other => Err(self.blame(CalcError::Mismatch {
                expected: Dimension::Time,
                found: other.dimension(),
                unit: None,
            })),
        }
    }

    /// `à 7 %`, `@ 7%`: the yearly rate, as a ratio.
    fn rate(&mut self) -> Result<Decimal, CalcError> {
        if !self.eat(CalcWord::At) {
            return Err(self.unreadable());
        }
        match self.additive()? {
            Value::Percent { ratio, .. } => Ok(ratio),
            _ => Err(self.blame(CalcError::RateExpected)),
        }
    }

    /// `composés mensuellement`: how many times a year the interest joins the amount, once
    /// unless a line says otherwise.
    fn periods(&mut self) -> Result<Decimal, CalcError> {
        if !self.eat(CalcWord::Compounded) {
            return Ok(Decimal::ONE);
        }
        for (word, count) in [
            (CalcWord::Monthly, 12),
            (CalcWord::Quarterly, 4),
            (CalcWord::Yearly, 1),
            (CalcWord::Daily, 365),
        ] {
            if self.eat(word) {
                return Ok(Decimal::from(count));
            }
        }
        Err(self.unreadable())
    }

    /// `1 000 € après 3 ans à 7 %` is 1 225,04 €: the amount and its compound interest.
    fn compound(&mut self, principal: Value) -> Answer {
        let years = self.years()?;
        let rate = self.rate()?;
        let periods = self.periods()?;
        let grown = maths::grown(rate, periods, years).map_err(|error| self.blame(error))?;
        multiply(principal, Value::number(grown)).map_err(|error| self.blame(error))
    }

    /// The forms that open on their own words: interest, return, present value and loans.
    fn finance(&mut self) -> Result<Option<Value>, CalcError> {
        if self.eat(CalcWord::InterestOn) {
            let principal = self.principal()?;
            if !(self.eat(CalcWord::After) || self.eat(CalcWord::During)) {
                return Err(self.unreadable());
            }
            let grown = self.compound(principal.clone())?;
            return add(grown, principal, false)
                .map(Some)
                .map_err(|error| self.blame(error));
        }
        if self.eat(CalcWord::AnnualReturn) {
            return self.annual_return().map(Some);
        }
        if self.eat(CalcWord::PresentValue) {
            let future = self.principal()?;
            if !(self.eat(CalcWord::Within)
                || self.eat(CalcWord::After)
                || self.eat(CalcWord::During))
            {
                return Err(self.unreadable());
            }
            let years = self.years()?;
            let rate = self.rate()?;
            let periods = self.periods()?;
            let grown = maths::grown(rate, periods, years).map_err(|error| self.blame(error))?;
            return divide(future, Value::number(grown))
                .map(Some)
                .map_err(|error| self.blame(error));
        }
        for (word, part) in [
            (CalcWord::Repayment, Loan::Monthly),
            (CalcWord::TotalRepayment, Loan::Total),
            (CalcWord::TotalInterest, Loan::Interest),
        ] {
            if self.eat(word) {
                return self.loan(part).map(Some);
            }
        }
        Ok(None)
    }

    /// `rendement annuel sur 1 000 € investis 2 500 € récupérés en 7 ans` is ≈ 13,99 %.
    fn annual_return(&mut self) -> Answer {
        let invested = self.principal()?;
        if !self.eat(CalcWord::Invested) {
            return Err(self.unreadable());
        }
        let returned = self.principal()?;
        if !self.eat(CalcWord::Returned) {
            return Err(self.unreadable());
        }
        if !(self.eat(CalcWord::Convert) || self.eat(CalcWord::After) || self.eat(CalcWord::During))
        {
            return Err(self.unreadable());
        }
        let years = self.years()?;
        let times = divide(returned, invested).and_then(|times| plain(&times));
        let yearly = times
            .and_then(|times| maths::yearly(times, years))
            .map_err(|error| self.blame(error))?;
        Ok(Value::Percent {
            ratio: yearly,
            signed: false,
        })
    }

    /// `mensualité de 10 000 € sur 6 ans à 6 %` is 165,73 €; its total, its interest.
    fn loan(&mut self, part: Loan) -> Answer {
        let principal = self.principal()?;
        if !(self.eat(CalcWord::Over) || self.eat(CalcWord::During)) {
            return Err(self.unreadable());
        }
        let years = self.years()?;
        let rate = self.rate()?;
        let months = years * Decimal::from(12);
        let payment =
            maths::payment(rate / Decimal::from(12), months).map_err(|error| self.blame(error))?;
        let factor = match part {
            Loan::Monthly => payment,
            Loan::Total => payment * months,
            Loan::Interest => payment * months - Decimal::ONE,
        };
        multiply(principal, Value::number(factor)).map_err(|error| self.blame(error))
    }
}

#[derive(Clone, Copy)]
enum Loan {
    Monthly,
    Total,
    Interest,
}

/// What a share multiplies: the ratio, one plus it, or one less it.
fn share_factor(ratio: Decimal, share: Share) -> Value {
    Value::number(match share {
        Share::Of => ratio,
        Share::On => Decimal::ONE + ratio,
        Share::Off => Decimal::ONE - ratio,
    })
}

/// `5 % de plus sur 200` is 210, `5% off 200` 190.
fn applied(ratio: Decimal, share: Share, whole: Value) -> Answer {
    multiply(whole, share_factor(ratio, share))
}

/// `5 % de combien font 6` is 120, `5% on what is 6` 5,71.
fn solve(ratio: Decimal, share: Share, result: Value) -> Answer {
    divide(result, share_factor(ratio, share))
}

/// `50 en % de 100` is 50 %; `70 en % de plus que 20` 250 %; `20 en % de moins que 70` 71,43 %.
fn share_of(part: Value, whole: Value, share: Share) -> Answer {
    match share {
        Share::Of => ratio(part, whole),
        Share::On => ratio(add(part, whole.clone(), false)?, whole),
        Share::Off => ratio(add(whole.clone(), part, false)?, whole),
    }
}

/// `17 mod 5` is 2, and `-7 mod 3` is 2: the remainder takes the sign of the divisor.
fn modulo(left: Value, right: Value) -> Answer {
    match (left, right) {
        (Value::Quantity(a), Value::Quantity(b)) if a.dim == b.dim && !a.dim.has_temperature() => {
            if b.amount.is_zero() {
                return Err(CalcError::DivisionByZero);
            }
            let mut rest = a.amount.checked_rem(b.amount).ok_or_else(overflow)?;
            if !rest.is_zero() && rest.is_sign_negative() != b.amount.is_sign_negative() {
                rest += b.amount;
            }
            Ok(Value::Quantity(Quantity {
                amount: rest,
                spans: false,
                ..a
            }))
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
