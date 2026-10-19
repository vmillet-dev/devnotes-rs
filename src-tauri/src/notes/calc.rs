//! A Calcul note: a sheet of lines, each computed as it is typed — variables, units, dates,
//! percentages, totals. The results are never stored: they are computed when the sheet is
//! read, here, and the front writes them in the language on screen.

mod eval;
mod lexer;
mod maths;
pub mod units;
pub mod words;

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;

use eval::{Context, Notation, Parser, Quantity, Shown, Value};
use lexer::{Tok, Token};
use units::{Css, Dim, Dimension, UnitId};
use words::CalcWord;

/// One unit, or one over another: `€/h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CalcUnit {
    pub unit: UnitId,
    pub per: Option<UnitId>,
}

/// Exact decimals as text: a float would round them before the front decides how to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CalcValue {
    Number {
        value: String,
        unit: Option<CalcUnit>,
    },
    /// `value` in percent: `25` for 25 %. `signed` for a change, written `+25 %`.
    Percent {
        value: String,
        signed: bool,
    },
    Date {
        date: NaiveDate,
    },
    /// A duration in a unit and the next one down: `6 sem 3 j`.
    Span {
        parts: Vec<SpanPart>,
    },
    /// `255 en hex`: the digits `FF` in base 16, the front writing `0x` before them.
    Base {
        digits: String,
        radix: u32,
        negative: bool,
    },
    /// `5300 en sci`: `5.3` times ten to the 3.
    Scientific {
        mantissa: String,
        exponent: i32,
    },
    /// `2/10 en fraction`: `1/5`, reduced.
    Fraction {
        numerator: String,
        denominator: String,
        negative: bool,
    },
    /// `20/5 en multiple`: `×4`.
    Multiplier {
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpanPart {
    pub value: String,
    pub unit: UnitId,
}

/// Why a line has no result. It stays on its line: the others go on computing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CalcError {
    /// In UTF-16 units from the start of the line.
    Unreadable {
        at: u32,
    },
    UnknownName {
        name: String,
    },
    /// `en = 5`: a word of the grammar is no name.
    Reserved {
        word: String,
    },
    /// `distance + 3 Go`: a data size is no length.
    Mismatch {
        expected: Dimension,
        found: Dimension,
        unit: Option<CalcUnit>,
    },
    /// ⚠️ No rate is fetched: the sheet works offline, and euros stay euros.
    Currency,
    NotConvertible {
        from: Dimension,
        to: Option<CalcUnit>,
    },
    DivisionByZero,
    /// `total` with nothing to add up above it in the block.
    NothingAbove,
    /// `préc` with no line above that gave a result.
    NoPrevious,
    /// A date moved by something other than whole days, months or years.
    DateShift,
    /// An even root of a negative number.
    NegativeRoot,
    /// A logarithm of zero or less, or in a base of one or less.
    LogDomain,
    /// A base or a bitwise operation on a fraction.
    NotWhole,
    /// `ppi = 0`: a setting is above zero.
    NotPositive,
    /// `à 7`: a yearly rate is a percentage.
    RateExpected,
    Overflow,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CalcLine {
    /// Ends a block, as a heading does.
    Blank,
    Heading {
        title: String,
    },
    Comment,
    /// Words with no number in them: a sentence, not a mistake.
    Text,
    Value {
        value: CalcValue,
        /// The variable the line names, `prix` in `prix = 120 €`.
        name: Option<String>,
        /// `total` or `moyenne`: drawn with a rule above, a ledger's own convention.
        aggregate: bool,
        dimension: Dimension,
    },
    Error {
        error: CalcError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TokenKind {
    Number,
    Unit,
    Operator,
    Keyword,
    Name,
    Date,
    Comment,
    Heading,
    Error,
}

/// What the editor colours, in UTF-16 units from the start of the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CalcSpan {
    pub start: u32,
    pub end: u32,
    pub kind: TokenKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CalcRow {
    pub line: CalcLine,
    pub spans: Vec<CalcSpan>,
    /// What `en` or `à` was asked to convert: the editor proposes the units of that kind.
    pub converting: Option<Dimension>,
}

/// A name, the line it was given on, its value there and the lines that read it before it
/// was given another. Zero-based lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CalcVariable {
    pub name: String,
    pub line: u32,
    pub value: CalcValue,
    pub uses: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CalcSheet {
    pub rows: Vec<CalcRow>,
    pub variables: Vec<CalcVariable>,
}

/// One of the last lines a card shows: what was written, and what it gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CalcCardLine {
    pub expression: String,
    pub value: CalcValue,
    pub aggregate: bool,
}

/// How many a card shows: often the conclusion of the sheet.
pub const CARD_LINES: usize = 4;

/// The last figures of a quotient carried through more operations are noise: `12 pt en px` is
/// 16, not 15,999…. Twenty-five figures stay, past what any display shows.
fn tidy(decimal: Decimal) -> Decimal {
    let whole = decimal.trunc().abs();
    let figures = if whole.is_zero() {
        0
    } else {
        u32::try_from(whole.to_string().len()).unwrap_or(28)
    };
    let places = 25_u32.saturating_sub(figures);
    if decimal.scale() > places {
        decimal.round_dp(places)
    } else {
        decimal
    }
}

fn text(decimal: Decimal) -> String {
    tidy(decimal).normalize().to_string()
}

fn utf16(text: &str) -> u32 {
    u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX)
}

struct Units {
    table: Vec<units::UnitDef>,
    symbols: HashMap<&'static str, usize>,
    factors: HashMap<UnitId, (Decimal, Dim)>,
}

impl Units {
    fn new() -> Self {
        let table = units::table();
        let symbols = table
            .iter()
            .enumerate()
            .flat_map(|(index, unit)| unit.symbols.iter().map(move |symbol| (*symbol, index)))
            .collect();
        let factors = table
            .iter()
            .map(|unit| (unit.id, (unit.factor, unit.dim)))
            .collect();
        Self {
            table,
            symbols,
            factors,
        }
    }

    /// A pixel's and an `em`'s size follow the sheet's settings.
    fn unit(&self, unit: UnitId, css: &Css) -> Option<(Decimal, Dim)> {
        let &(factor, dim) = self.factors.get(&unit)?;
        Some((css.factor(unit).unwrap_or(factor), dim))
    }

    /// Base units in one, as above over below, and what it measures.
    fn factor(&self, shown: Shown, css: &Css) -> Option<(Decimal, Decimal, Dim)> {
        match shown {
            Shown::One(unit) => self
                .unit(unit, css)
                .map(|(factor, dim)| (factor, Decimal::ONE, dim)),
            Shown::Per(above, below) => {
                let (top, top_dim) = self.unit(above, css)?;
                let (bottom, bottom_dim) = self.unit(below, css)?;
                Some((top, bottom, top_dim.over(bottom_dim)))
            }
        }
    }

    /// A duration nobody named is shown in the largest unit it fills.
    fn natural(&self, quantity: &Quantity) -> Option<Shown> {
        if quantity.dim != Dim::TIME {
            return None;
        }
        let amount = quantity.amount.abs();
        let unit = [UnitId::Day, UnitId::Hour, UnitId::Minute, UnitId::Second]
            .into_iter()
            .find(|unit| {
                self.factors
                    .get(unit)
                    .is_some_and(|(factor, _)| amount >= *factor)
            })
            .unwrap_or(UnitId::Millisecond);
        Some(Shown::One(unit))
    }

    fn show(&self, value: &Value, css: &Css) -> Result<CalcValue, CalcError> {
        let quantity = match value {
            Value::Percent { ratio, signed } => {
                return Ok(CalcValue::Percent {
                    value: text(ratio * Decimal::ONE_HUNDRED),
                    signed: *signed,
                });
            }
            Value::Date(date) => return Ok(CalcValue::Date { date: *date }),
            Value::Quantity(quantity) => quantity,
        };
        if quantity.dim.is_none() {
            return Ok(CalcValue::Number {
                value: text(quantity.amount),
                unit: None,
            });
        }
        let fits = |shown: &Shown| {
            self.factor(*shown, css)
                .is_some_and(|(_, _, dim)| dim == quantity.dim)
        };
        let shown = quantity
            .unit
            .filter(fits)
            .or_else(|| self.natural(quantity))
            .ok_or(CalcError::Unsupported)?;
        if quantity.dim.has_temperature() {
            return Ok(CalcValue::Number {
                value: text(quantity.amount),
                unit: Some(shown.calc_unit()),
            });
        }
        let (above, below, _) = self.factor(shown, css).ok_or(CalcError::Unsupported)?;
        let value = quantity
            .amount
            .checked_mul(below)
            .and_then(|product| product.checked_div(above))
            .ok_or(CalcError::Overflow)?;
        let factor = above;
        if quantity.spans
            && let Shown::One(unit) = shown
            && let Some(span) = self.span(quantity.amount, value, factor, unit)
        {
            return Ok(span);
        }
        Ok(CalcValue::Number {
            value: text(value),
            unit: Some(shown.calc_unit()),
        })
    }

    /// `45 jours en sem` is `6 sem 3 j`: the whole weeks, and what is left in whole days.
    fn span(
        &self,
        amount: Decimal,
        value: Decimal,
        factor: Decimal,
        unit: UnitId,
    ) -> Option<CalcValue> {
        let smaller = match unit {
            UnitId::Week => UnitId::Day,
            UnitId::Day => UnitId::Hour,
            UnitId::Hour => UnitId::Minute,
            UnitId::Minute => UnitId::Second,
            _ => return None,
        };
        if value.fract().is_zero() || value.is_sign_negative() {
            return None;
        }
        let whole = value.trunc();
        let (small, _) = self.factors.get(&smaller)?;
        let rest = (amount - whole * factor).checked_div(*small)?;
        rest.fract().is_zero().then(|| CalcValue::Span {
            parts: vec![
                SpanPart {
                    value: text(whole),
                    unit,
                },
                SpanPart {
                    value: text(rest),
                    unit: smaller,
                },
            ],
        })
    }
}

fn spans_of(tokens: &[Token], kinds: &[Option<TokenKind>]) -> Vec<CalcSpan> {
    tokens
        .iter()
        .zip(kinds)
        .filter_map(|(token, kind)| {
            kind.map(|kind| CalcSpan {
                start: token.start,
                end: token.end,
                kind,
            })
        })
        .collect()
}

/// The span of a comment written after an expression, from its `//` to the end.
fn trailing_comment(line: &str) -> Option<CalcSpan> {
    let at = line.find("//")?;
    Some(CalcSpan {
        start: utf16(&line[..at]),
        end: utf16(line),
        kind: TokenKind::Comment,
    })
}

fn row(line: CalcLine, spans: Vec<CalcSpan>) -> CalcRow {
    CalcRow {
        line,
        spans,
        converting: None,
    }
}

/// What the sheet knows so far, line after line.
struct Walk {
    today: NaiveDate,
    units: Units,
    variables: HashMap<String, Value>,
    definitions: Vec<CalcVariable>,
    /// The definition of each name in effect: the one its uses are counted on.
    current: HashMap<String, usize>,
    block: Vec<Quantity>,
    previous: Option<Value>,
    css: Css,
}

/// `255 en hex`, `5300 en sci`: a plain number written as the line asked.
fn notated(
    value: &Value,
    shown: CalcValue,
    notation: Option<Notation>,
) -> Result<CalcValue, CalcError> {
    let (Some(notation), Value::Quantity(quantity)) = (notation, value) else {
        return Ok(shown);
    };
    if !quantity.dim.is_none() {
        return Ok(shown);
    }
    Ok(match notation {
        Notation::Base(radix) => {
            let (negative, digits) = maths::digits(quantity.amount, radix)?;
            CalcValue::Base {
                digits,
                radix,
                negative,
            }
        }
        Notation::Scientific => {
            let (mantissa, exponent) = maths::scientific(tidy(quantity.amount));
            CalcValue::Scientific { mantissa, exponent }
        }
        Notation::Fraction => {
            let (negative, numerator, denominator) = maths::fraction(tidy(quantity.amount));
            CalcValue::Fraction {
                numerator: numerator.to_string(),
                denominator: denominator.to_string(),
                negative,
            }
        }
        Notation::Multiplier => CalcValue::Multiplier {
            value: text(quantity.amount),
        },
        Notation::Decimal => shown,
    })
}

/// `Prix : 11 € + 34,45 €`: the words before a colon name the line, and are read past.
fn label_end(tokens: &[Token]) -> Option<usize> {
    if !matches!(tokens.first()?.tok, Tok::Word(_)) {
        return None;
    }
    let colon = tokens
        .iter()
        .position(|token| token.tok == Tok::Other(':'))?;
    tokens[..colon]
        .iter()
        .all(|token| token.tok != Tok::Symbol('='))
        .then_some(colon + 1)
}

/// The line after its label, the name it gives and the expression.
struct Head<'t> {
    tokens: &'t [Token],
    name: Option<String>,
    body: &'t [Token],
}

/// What a line opens on: a label read past, then a name and its `=`; a reserved word as the
/// name is the line's error.
fn head<'t>(tokens: &'t [Token], spans: &mut Vec<CalcSpan>) -> Result<Head<'t>, CalcRow> {
    let tokens = match label_end(tokens) {
        Some(end) => {
            spans.push(CalcSpan {
                start: tokens[0].start,
                end: tokens[end - 1].end,
                kind: TokenKind::Comment,
            });
            &tokens[end..]
        }
        None => tokens,
    };
    match tokens {
        [
            Token {
                tok: Tok::Word(word),
                start,
                end,
                ..
            },
            Token {
                tok: Tok::Symbol('='),
                ..
            },
            ..,
        ] => {
            if words::reserved(word) {
                spans.push(CalcSpan {
                    start: *start,
                    end: *end,
                    kind: TokenKind::Error,
                });
                let error = CalcError::Reserved { word: word.clone() };
                return Err(row(CalcLine::Error { error }, std::mem::take(spans)));
            }
            Ok(Head {
                tokens,
                name: Some(word.clone()),
                body: &tokens[2..],
            })
        }
        _ => Ok(Head {
            tokens,
            name: None,
            body: tokens,
        }),
    }
}

impl Walk {
    fn line(&mut self, index: u32, line: &str) -> CalcRow {
        let trimmed = line.trim();
        let whole = |kind| {
            vec![CalcSpan {
                start: 0,
                end: utf16(line),
                kind,
            }]
        };
        if trimmed.is_empty() {
            self.block.clear();
            return row(CalcLine::Blank, Vec::new());
        }
        if let Some(title) = trimmed.strip_prefix('#') {
            self.block.clear();
            let title = title.trim().to_string();
            return row(CalcLine::Heading { title }, whole(TokenKind::Heading));
        }
        if trimmed.starts_with("//") {
            return row(CalcLine::Comment, whole(TokenKind::Comment));
        }
        self.expression(index, line, &lexer::tokens(line))
    }

    fn expression(&mut self, index: u32, line: &str, tokens: &[Token]) -> CalcRow {
        let mut spans = Vec::new();
        let Head { tokens, name, body } = match head(tokens, &mut spans) {
            Ok(head) => head,
            Err(row) => return row,
        };

        let context = Context {
            today: self.today,
            units: &self.units.table,
            symbols: &self.units.symbols,
            variables: &self.variables,
            block: &self.block,
            previous: self.previous.as_ref(),
            css: self.css,
        };
        let end = utf16(lexer::without_comment(line).trim_end());
        let mut parser = Parser::new(&context, body, end);
        let mut result = parser.whole();
        let setting = name.as_deref().and_then(words::setting);
        if name.is_some() {
            spans.push(CalcSpan {
                start: tokens[0].start,
                end: tokens[0].end,
                kind: if setting.is_some() {
                    TokenKind::Keyword
                } else {
                    TokenKind::Name
                },
            });
            spans.push(CalcSpan {
                start: tokens[1].start,
                end: tokens[1].end,
                kind: TokenKind::Operator,
            });
        }
        spans.extend(spans_of(body, &parser.kinds));
        spans.extend(trailing_comment(line));
        let (used, converting, aggregate, notation) = (
            parser.used,
            parser.converting,
            parser.aggregate,
            parser.notation,
        );
        if let (Some(setting), Ok(value)) = (setting, &result) {
            result = self.set(setting, value);
        }

        for used_name in &used {
            if let Some(&definition) = self.current.get(used_name)
                && self.definitions[definition].uses.last() != Some(&index)
            {
                self.definitions[definition].uses.push(index);
            }
        }

        let css = self.css;
        let line = match result.and_then(|value| {
            let shown = self.units.show(&value, &css)?;
            notated(&value, shown, notation).map(|shown| (value, shown))
        }) {
            Ok((value, shown)) => self.value(index, name, value, shown, aggregate),
            Err(error) => {
                let sentence = name.is_none()
                    && used.is_empty()
                    && !tokens
                        .iter()
                        .any(|token| matches!(token.tok, Tok::Number(_) | Tok::Date(_)))
                    && matches!(
                        error,
                        CalcError::Unreadable { .. } | CalcError::UnknownName { .. }
                    );
                if sentence {
                    spans.retain(|span| span.kind == TokenKind::Comment);
                    CalcLine::Text
                } else {
                    CalcLine::Error { error }
                }
            }
        };
        CalcRow {
            line,
            spans,
            converting,
        }
    }

    /// `ppi = 326`, `em = 20 px`: the sheet's setting from this line down. A size in pixels
    /// may be written bare.
    fn set(&mut self, setting: CalcWord, value: &Value) -> Result<Value, CalcError> {
        let pixel = self.css.factor(UnitId::Pixel).ok_or(CalcError::Overflow)?;
        let pixels = match value {
            Value::Quantity(quantity) if quantity.dim.is_none() => quantity.amount,
            Value::Quantity(quantity)
                if quantity.dim == Dim::LENGTH && setting != CalcWord::Ppi =>
            {
                quantity
                    .amount
                    .checked_div(pixel)
                    .ok_or(CalcError::Overflow)?
            }
            other => {
                return Err(CalcError::Mismatch {
                    expected: Dimension::Number,
                    found: other.dimension(),
                    unit: None,
                });
            }
        };
        if pixels <= Decimal::ZERO {
            return Err(CalcError::NotPositive);
        }
        let pixels = tidy(pixels);
        match setting {
            CalcWord::Ppi => {
                self.css.ppi = pixels;
                return Ok(Value::Quantity(Quantity {
                    amount: pixels,
                    dim: Dim::NONE,
                    unit: None,
                    spans: false,
                }));
            }
            CalcWord::Em => self.css.em = pixels,
            _ => self.css.rem = pixels,
        }
        Ok(Value::Quantity(Quantity {
            amount: pixels.checked_mul(pixel).ok_or(CalcError::Overflow)?,
            dim: Dim::LENGTH,
            unit: Some(Shown::One(UnitId::Pixel)),
            spans: false,
        }))
    }

    fn value(
        &mut self,
        index: u32,
        name: Option<String>,
        value: Value,
        shown: CalcValue,
        aggregate: bool,
    ) -> CalcLine {
        let dimension = value.dimension();
        // `ppi = 326` sets the sheet: no name, nothing for `total` or `préc` to read.
        if name
            .as_deref()
            .is_some_and(|name| words::setting(name).is_some())
        {
            return CalcLine::Value {
                value: shown,
                name,
                aggregate,
                dimension,
            };
        }
        if let Some(name) = &name {
            self.current.insert(name.clone(), self.definitions.len());
            self.definitions.push(CalcVariable {
                name: name.clone(),
                line: index,
                value: shown.clone(),
                uses: Vec::new(),
            });
            self.variables.insert(name.clone(), value.clone());
        }
        self.previous = Some(value.clone());
        if !aggregate && let Value::Quantity(quantity) = value {
            self.block.push(quantity);
        }
        CalcLine::Value {
            value: shown,
            name,
            aggregate,
            dimension,
        }
    }
}

/// The whole sheet, top to bottom: a variable takes the value it was last given above,
/// and `total` reads the block it is in. `today` is the local day, as the notes' sections use.
pub fn evaluate(sheet: &str, today: NaiveDate) -> CalcSheet {
    let mut walk = Walk {
        today,
        units: Units::new(),
        variables: HashMap::new(),
        definitions: Vec::new(),
        current: HashMap::new(),
        block: Vec::new(),
        previous: None,
        css: Css::default(),
    };
    let rows = sheet
        .split('\n')
        .enumerate()
        .map(|(index, raw)| {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            walk.line(u32::try_from(index).unwrap_or(u32::MAX), line)
        })
        .collect();

    CalcSheet {
        rows,
        variables: walk.definitions,
    }
}

/// The last lines that gave something, the way a card shows the end of a sheet.
pub fn card_lines(sheet: &str, today: NaiveDate) -> Vec<CalcCardLine> {
    let lines: Vec<&str> = sheet.split('\n').collect();
    let mut found: Vec<CalcCardLine> = evaluate(sheet, today)
        .rows
        .into_iter()
        .zip(&lines)
        .filter_map(|(row, line)| match row.line {
            CalcLine::Value {
                value, aggregate, ..
            } => Some(CalcCardLine {
                expression: lexer::without_comment(line).trim().to_string(),
                value,
                aggregate,
            }),
            _ => None,
        })
        .collect();
    let skip = found.len().saturating_sub(CARD_LINES);
    found.drain(..skip);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 16).unwrap()
    }

    fn lines(sheet: &str) -> Vec<CalcLine> {
        evaluate(sheet, today())
            .rows
            .into_iter()
            .map(|row| row.line)
            .collect()
    }

    fn value(line: &str) -> CalcValue {
        match lines(line).pop() {
            Some(CalcLine::Value { value, .. }) => value,
            other => panic!("{line}: {other:?}"),
        }
    }

    fn number(value: &str, unit: Option<UnitId>) -> CalcValue {
        CalcValue::Number {
            value: value.to_string(),
            unit: unit.map(|unit| CalcUnit { unit, per: None }),
        }
    }

    fn percent(value: &str, signed: bool) -> CalcValue {
        CalcValue::Percent {
            value: value.to_string(),
            signed,
        }
    }

    fn error(line: &str) -> CalcError {
        match lines(line).pop() {
            Some(CalcLine::Error { error }) => error,
            other => panic!("{line}: {other:?}"),
        }
    }

    #[test]
    fn the_five_questions_of_the_percentages_tool() {
        assert_eq!(value("20 % de 340"), number("68", None));
        assert_eq!(value("30 sur 120 en %"), percent("25", false));
        assert_eq!(value("de 80 à 100"), percent("25", true));
        assert_eq!(value("250 + 20 %"), number("300", None));
        assert_eq!(value("300 hors 20 %"), number("250", None));
    }

    #[test]
    fn english_words_read_the_same_whatever_the_language_on_screen() {
        assert_eq!(value("20 % of 340"), number("68", None));
        assert_eq!(value("from 80 to 100"), percent("25", true));
        assert_eq!(value("5 km in m"), number("5000", Some(UnitId::Metre)));
    }

    #[test]
    fn units_convert_within_their_kind() {
        assert_eq!(
            value("38 °C en °F"),
            number("100.4", Some(UnitId::Fahrenheit))
        );
        assert_eq!(value("2,5 kg en g"), number("2500", Some(UnitId::Gram)));
        assert_eq!(
            value("3 h 20 min en min"),
            number("200", Some(UnitId::Minute))
        );
        assert_eq!(value("1 Go en Mo"), number("1000", Some(UnitId::Megabyte)));
        let CalcValue::Number { value: miles, .. } = value("5 km en mi") else {
            panic!()
        };
        assert!(miles.starts_with("3.10685"));
    }

    #[test]
    fn a_size_at_a_rate_is_a_duration() {
        assert_eq!(value("1,44 To à 100 Mo/s"), number("4", Some(UnitId::Hour)));
    }

    #[test]
    fn a_price_per_hour_times_hours_is_euros() {
        let sheet = "prix = 0,045 €/h\nserveurs = 3\ncoût = prix × 730 h × serveurs";
        assert_eq!(
            lines(sheet).pop(),
            Some(CalcLine::Value {
                value: number("98.55", Some(UnitId::Euro)),
                name: Some("coût".into()),
                aggregate: false,
                dimension: Dimension::Euro,
            })
        );
        assert_eq!(
            value("0,045 €/h"),
            CalcValue::Number {
                value: "0.045".into(),
                unit: Some(CalcUnit {
                    unit: UnitId::Euro,
                    per: Some(UnitId::Hour)
                })
            }
        );
    }

    #[test]
    fn a_percentage_taken_off_then_added_is_a_share_each_time() {
        let sheet = "coût = 98,55 €\nremise = 15 %\nttc = coût − remise + 20 %";
        assert_eq!(
            match lines(sheet).pop() {
                Some(CalcLine::Value { value, .. }) => value,
                _ => panic!(),
            },
            number("100.521", Some(UnitId::Euro))
        );
    }

    #[test]
    fn dates_move_by_days_weeks_and_months_and_subtract_into_days() {
        assert_eq!(
            value("aujourd'hui + 45 jours"),
            CalcValue::Date {
                date: NaiveDate::from_ymd_opt(2026, 11, 30).unwrap()
            }
        );
        assert_eq!(
            value("today + 2 wk"),
            CalcValue::Date {
                date: NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
            }
        );
        assert_eq!(
            value("31/01/2026 + 1 mois"),
            CalcValue::Date {
                date: NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
            }
        );
        assert_eq!(
            value("16/10/2026 − 01/01/2026"),
            number("288", Some(UnitId::Day))
        );
        assert_eq!(error("aujourd'hui + 3 h"), CalcError::DateShift);
    }

    #[test]
    fn a_duration_into_weeks_keeps_the_days_left() {
        assert_eq!(
            value("45 jours en sem"),
            CalcValue::Span {
                parts: vec![
                    SpanPart {
                        value: "6".into(),
                        unit: UnitId::Week
                    },
                    SpanPart {
                        value: "3".into(),
                        unit: UnitId::Day
                    },
                ]
            }
        );
        let sheet = "livraison = aujourd'hui + 45 jours\nlivraison − aujourd'hui en sem";
        assert!(matches!(
            lines(sheet).pop(),
            Some(CalcLine::Value {
                value: CalcValue::Span { .. },
                ..
            })
        ));
    }

    #[test]
    fn total_and_mean_read_the_block_above_and_a_heading_starts_another() {
        let sheet =
            "# Budget\na = 100 €\nb = 26 €\nmensuel = total\nmensuel × 12\n\nc = 3 €\nmoyenne";
        let found = lines(sheet);
        assert_eq!(
            found[3],
            CalcLine::Value {
                value: number("126", Some(UnitId::Euro)),
                name: Some("mensuel".into()),
                aggregate: true,
                dimension: Dimension::Euro,
            }
        );
        assert!(
            matches!(&found[4], CalcLine::Value { value, .. } if *value == number("1512", Some(UnitId::Euro)))
        );
        assert!(
            matches!(&found[7], CalcLine::Value { value, .. } if *value == number("3", Some(UnitId::Euro)))
        );
        assert_eq!(error("total"), CalcError::NothingAbove);
    }

    #[test]
    fn an_error_stays_on_its_line_and_says_why() {
        let sheet = "distance = 5 km\ndistance + 3 Go\ndistance en m";
        let found = lines(sheet);
        assert_eq!(
            found[1],
            CalcLine::Error {
                error: CalcError::Mismatch {
                    expected: Dimension::Length,
                    found: Dimension::Data,
                    unit: Some(CalcUnit {
                        unit: UnitId::Gigabyte,
                        per: None
                    }),
                }
            }
        );
        assert!(matches!(found[2], CalcLine::Value { .. }));
        assert_eq!(error("x + 1"), CalcError::UnknownName { name: "x".into() });
        assert_eq!(error("4 / 0"), CalcError::DivisionByZero);
    }

    #[test]
    fn a_conversion_between_currencies_is_refused_offline() {
        assert_eq!(error("100 € en $"), CalcError::Currency);
        assert!(matches!(error("100 € + 3 $"), CalcError::Mismatch { .. }));
        assert_eq!(value("100 € × 3"), number("300", Some(UnitId::Euro)));
    }

    #[test]
    fn headings_comments_and_sentences_have_no_result_and_no_error() {
        assert_eq!(
            lines("# Planning\n// une remarque\nLe serveur de recette\n"),
            [
                CalcLine::Heading {
                    title: "Planning".into()
                },
                CalcLine::Comment,
                CalcLine::Text,
                CalcLine::Blank,
            ]
        );
    }

    #[test]
    fn a_copied_sheet_reads_back_with_its_results_as_comments() {
        assert_eq!(
            value("prix = 120 € // 120,00 €"),
            number("120", Some(UnitId::Euro))
        );
    }

    #[test]
    fn a_variable_says_where_it_is_given_and_read() {
        let sheet = evaluate("a = 2\nb = a × 3\na = 5\na + b", today());
        let found: Vec<_> = sheet
            .variables
            .iter()
            .map(|variable| (variable.name.as_str(), variable.line, variable.uses.clone()))
            .collect();
        assert_eq!(
            found,
            [("a", 0, vec![1]), ("b", 1, vec![3]), ("a", 2, vec![3])]
        );
    }

    #[test]
    fn the_editor_is_told_what_kind_is_being_converted_and_where_the_words_are() {
        let sheet = evaluate("3 h 20 min en ", today());
        let row = &sheet.rows[0];
        assert_eq!(row.converting, Some(Dimension::Time));
        assert!(matches!(
            row.line,
            CalcLine::Error {
                error: CalcError::Unreadable { .. }
            }
        ));
        let kinds: Vec<_> = row.spans.iter().map(|span| span.kind).collect();
        assert_eq!(
            kinds,
            [
                TokenKind::Number,
                TokenKind::Unit,
                TokenKind::Number,
                TokenKind::Unit,
                TokenKind::Keyword
            ]
        );
    }

    #[test]
    fn a_card_shows_the_last_four_lines_that_gave_something() {
        let sheet = "a = 1\nb = 2\n// c\nc = 3\nd = 4\ne = 5 // 5\nnope +";
        let card = card_lines(sheet, today());
        let expressions: Vec<_> = card.iter().map(|line| line.expression.as_str()).collect();
        assert_eq!(expressions, ["b = 2", "c = 3", "d = 4", "e = 5"]);
    }

    #[test]
    fn a_syntax_word_is_no_name_and_says_so_on_its_line_alone() {
        let found = lines("a = 2\nen = 5\nDe = 1\na × 3");
        assert_eq!(
            found[1],
            CalcLine::Error {
                error: CalcError::Reserved { word: "en".into() }
            }
        );
        assert_eq!(
            found[2],
            CalcLine::Error {
                error: CalcError::Reserved { word: "De".into() }
            }
        );
        assert!(matches!(&found[3], CalcLine::Value { value, .. } if *value == number("6", None)));
        let row = &evaluate("en = 5", today()).rows[0];
        assert_eq!(
            row.spans,
            [CalcSpan {
                start: 0,
                end: 2,
                kind: TokenKind::Error
            }]
        );
    }

    #[test]
    fn a_name_hides_a_value_word_from_its_line_down() {
        let found =
            lines("a = 2\nb = 3\ntotal\ntotal = 10\ntotal + 1\naujourd'hui = 4\naujourd'hui × 2");
        assert!(
            matches!(&found[2], CalcLine::Value { value, aggregate: true, .. } if *value == number("5", None))
        );
        assert!(matches!(
            &found[3],
            CalcLine::Value { name: Some(name), aggregate: false, .. } if name == "total"
        ));
        assert!(
            matches!(&found[4], CalcLine::Value { value, aggregate: false, .. } if *value == number("11", None))
        );
        assert!(matches!(&found[6], CalcLine::Value { value, .. } if *value == number("8", None)));

        let found = lines("a = 2\ntotal = total\ntotal");
        assert!(matches!(
            &found[1],
            CalcLine::Value {
                aggregate: true,
                ..
            }
        ));
        assert!(
            matches!(&found[2], CalcLine::Value { value, aggregate: false, .. } if *value == number("2", None))
        );
    }

    #[test]
    fn a_form_of_several_words_wins_over_a_name_of_one() {
        assert_eq!(
            cents("mensualité = 100 €\nmensualité de 10 000 € sur 6 ans à 6 %"),
            number("165.73", Some(UnitId::Euro))
        );
        assert_eq!(
            value("mensualité = 100 €\nmensualité + 1 €"),
            number("101", Some(UnitId::Euro))
        );
        assert_eq!(
            cents("total = $2\ntotal cost of $10,000 over 6 years at 6%"),
            number("11932.48", Some(UnitId::Dollar))
        );
        assert_eq!(
            value("racine = 5\nracine carrée(16) + racine"),
            number("9", None)
        );
    }

    #[test]
    fn each_concept_reads_in_french_and_in_english() {
        let date = |days: u64| CalcValue::Date {
            date: today().checked_add_days(chrono::Days::new(days)).unwrap(),
        };
        let pairs = [
            ("5 km en m", "5 km in m"),
            ("5 km vers m", "5 km as m"),
            ("1,44 To à 100 Mo/s", "1,44 To at 100 Mo/s"),
            ("20 % de 340", "20 % of 340"),
            ("de 80 à 100", "from 80 to 100"),
            ("30 sur 120 en %", "30 over 120 as %"),
            ("300 hors 20 %", "300 excluding 20 %"),
            ("300 ex 20 %", "300 excl 20 %"),
            ("a = 1\nb = 3\nsomme", "a = 1\nb = 3\nsum"),
            ("a = 1\nb = 3\nmoyenne", "a = 1\nb = 3\navg"),
            ("demain", "tomorrow"),
        ];
        for (french, english) in pairs {
            assert_eq!(value(french), value(english), "{french} / {english}");
        }
        assert_eq!(value("demain"), date(1));
        assert_eq!(value("Aujourd'hui"), date(0));
    }

    #[test]
    fn a_sheet_written_before_the_table_reads_the_same() {
        let sheet = "# Budget\nhébergement = 100,521 €\ndomaines = 4,99 €\nmensuel = total\nmensuel × 12\n3 h 20 min en min\n20 % of 340\n100 € en $";
        let found = lines(sheet);
        assert_eq!(found.len(), 8);
        assert_eq!(
            found[0],
            CalcLine::Heading {
                title: "Budget".into()
            }
        );
        assert!(
            matches!(&found[3], CalcLine::Value { value, aggregate: true, .. } if *value == number("105.511", Some(UnitId::Euro)))
        );
        assert!(
            matches!(&found[4], CalcLine::Value { value, .. } if *value == number("1266.132", Some(UnitId::Euro)))
        );
        assert!(
            matches!(&found[5], CalcLine::Value { value, .. } if *value == number("200", Some(UnitId::Minute)))
        );
        assert!(matches!(&found[6], CalcLine::Value { value, .. } if *value == number("68", None)));
        assert_eq!(
            found[7],
            CalcLine::Error {
                error: CalcError::Currency
            }
        );
    }

    #[test]
    fn operators_are_written_in_words_in_both_languages() {
        let cases = [
            ("10 plus 5", "15"),
            ("10 et 5", "15"),
            ("10 and 5", "15"),
            ("10 avec 5", "15"),
            ("10 with 5", "15"),
            ("10 moins 4", "6"),
            ("10 minus 4", "6"),
            ("10 subtract 4", "6"),
            ("200 sans 30", "170"),
            ("200 without 30", "170"),
            ("8 fois 9", "72"),
            ("8 times 9", "72"),
            ("6 multiplié par 7", "42"),
            ("6 multiplied by 7", "42"),
            ("6 mul 7", "42"),
            ("42 divisé par 6", "7"),
            ("42 divided by 6", "7"),
            ("42 divide by 6", "7"),
            ("17 mod 5", "2"),
            ("17 modulo 5", "2"),
            ("-7 mod 3", "2"),
            ("2 + 3 fois 4", "14"),
        ];
        for (line, result) in cases {
            assert_eq!(value(line), number(result, None), "{line}");
        }
        assert_eq!(
            value("17 km mod 5 km"),
            number("2", Some(UnitId::Kilometre))
        );
        assert_eq!(error("5 mod 0"), CalcError::DivisionByZero);
        assert!(matches!(error("17 km mod 5"), CalcError::Mismatch { .. }));
        assert_eq!(
            error("fois = 3"),
            CalcError::Reserved {
                word: "fois".into()
            }
        );
    }

    #[test]
    fn without_subtracts_and_hors_still_takes_a_share_back_off() {
        assert_eq!(value("300 sans 20 %"), number("240", None));
        assert_eq!(value("300 without 20 %"), number("240", None));
        assert_eq!(value("300 hors 20 %"), number("250", None));
        assert_eq!(value("300 excluding 20 %"), number("250", None));
    }

    #[test]
    fn a_parenthesis_after_a_value_multiplies_it() {
        assert_eq!(value("6 (3)"), number("18", None));
        assert_eq!(value("2 (3 + 4)"), number("14", None));
        assert_eq!(value("(1 + 2)(3 + 1)"), number("12", None));
        assert_eq!(value("a = 4\n3 (a)"), number("12", None));
    }

    #[test]
    fn the_previous_result_is_the_last_line_above_that_gave_one() {
        assert_eq!(value("10 + 5\npréc × 2"), number("30", None));
        assert_eq!(value("10 + 5\n// double\n\nprev × 2"), number("30", None));
        assert_eq!(
            value("a = 3 €\nx +\nprécédent + 1 €"),
            number("4", Some(UnitId::Euro))
        );
        assert_eq!(error("préc + 1"), CalcError::NoPrevious);
        assert_eq!(value("préc = 2\npréc × 3"), number("6", None));
    }

    #[test]
    fn a_label_a_quote_and_a_remark_are_left_out_of_the_computation() {
        assert_eq!(
            value("Prix : 11 € + 34,45 €"),
            number("45.45", Some(UnitId::Euro))
        );
        assert_eq!(value("Mensuel : 1200 / 12"), number("100", None));
        assert_eq!(value("Price HT : ht = 100\nht × 2"), number("200", None));
        assert_eq!(
            value("275 € pour le \"Modèle 227\""),
            number("275", Some(UnitId::Euro))
        );
        assert_eq!(value("275 for the « Model 227 »"), number("275", None));
        assert_eq!(
            value("\"Loyer\" 800 € + 50 €"),
            number("850", Some(UnitId::Euro))
        );
        assert_eq!(
            value("999 € (pour l'iPhone, en promo)"),
            number("999", Some(UnitId::Euro))
        );
        assert_eq!(
            value("999 € (for iPhone) + 1 €"),
            number("1000", Some(UnitId::Euro))
        );
        assert_eq!(value("2 × (3 + 4)"), number("14", None));
        assert!(matches!(
            error("5 jours ouvrés"),
            CalcError::Unreadable { .. }
        ));

        let row = &evaluate("Prix : 11 € \"x\"", today()).rows[0];
        let kinds: Vec<_> = row.spans.iter().map(|span| span.kind).collect();
        assert_eq!(
            kinds,
            [
                TokenKind::Comment,
                TokenKind::Number,
                TokenKind::Unit,
                TokenKind::Comment
            ]
        );
        assert_eq!((row.spans[0].start, row.spans[0].end), (0, 6));
        assert_eq!(lines("Prix :"), [CalcLine::Text]);
    }

    #[test]
    fn large_numbers_are_written_with_their_scale() {
        let cases = [
            ("2k", "2000"),
            ("3M", "3000000"),
            ("1,5 milliard", "1500000000"),
            ("2 Md", "2000000000"),
            ("2 bn", "2000000000"),
            ("1 T", "1000000000000"),
            ("1 tn", "1000000000000"),
            ("10 mille", "10000"),
            ("10 thousand", "10000"),
            ("2 millions", "2000000"),
            ("1 billion", "1000000000"),
            ("1 trillion", "1000000000000"),
            ("1 mille milliards", "1000000000000"),
        ];
        for (line, result) in cases {
            assert_eq!(value(line), number(result, None), "{line}");
        }
        assert_eq!(value("2k €"), number("2000", Some(UnitId::Euro)));
        assert_eq!(value("3 m"), number("3", Some(UnitId::Metre)));
        assert_eq!(value("2 K"), number("2", Some(UnitId::Kelvin)));
        assert_eq!(value("k = 5\n2k"), number("2000", None));
    }

    #[test]
    fn thousands_grouped_by_a_space_read_back_as_the_front_writes_them() {
        assert_eq!(value("5 300"), number("5300", None));
        assert_eq!(
            value("1\u{202f}578,13 € + 1 €"),
            number("1579.13", Some(UnitId::Euro))
        );
        assert_eq!(
            value("1\u{a0}578,13 €"),
            number("1578.13", Some(UnitId::Euro))
        );
    }

    fn starts(line: &str, figures: &str) {
        match value(line) {
            CalcValue::Number { value, .. } => {
                assert!(value.starts_with(figures), "{line}: {value}");
            }
            other => panic!("{line}: {other:?}"),
        }
    }

    fn based(digits: &str, radix: u32) -> CalcValue {
        CalcValue::Base {
            digits: digits.into(),
            radix,
            negative: false,
        }
    }

    #[test]
    fn a_line_rounds_its_own_value_in_both_languages() {
        let cases = [
            ("1/3 à 2 décimales", "0.33"),
            ("1/3 to 2 dp", "0.33"),
            ("2/3 à 1 décimale", "0.7"),
            ("pi à 5 chiffres", "3.14159"),
            ("pi to 5 digits", "3.14159"),
            ("5,5 arrondi", "6"),
            ("5,5 rounded", "6"),
            ("-5,5 arrondi", "-6"),
            ("5,5 arrondi inférieur", "5"),
            ("5,5 rounded down", "5"),
            ("5,2 arrondi supérieur", "6"),
            ("5,2 rounded up", "6"),
            ("37 au 10 près", "40"),
            ("37 to nearest 10", "40"),
            ("2100 au millier près", "2000"),
            ("2100 to nearest thousand", "2000"),
            ("490 à la centaine près", "500"),
            ("490 to the nearest hundred", "500"),
            ("21 arrondi supérieur au 5 près", "25"),
            ("21 rounded up to nearest 5", "25"),
        ];
        for (line, result) in cases {
            assert_eq!(value(line), number(result, None), "{line}");
        }
        assert_eq!(value("x = 1/3 à 2 décimales\nx × 3"), number("0.99", None));
        assert_eq!(
            value("1,236 km à 2 décimales"),
            number("1.24", Some(UnitId::Kilometre))
        );
        assert_eq!(value("33,333 % à 1 décimale"), percent("33.3", false));
        assert_eq!(value("1,44 To à 100 Mo/s"), number("4", Some(UnitId::Hour)));
    }

    #[test]
    fn functions_read_with_or_without_parentheses() {
        let cases = [
            ("sqrt(16)", "4"),
            ("sqrt 16", "4"),
            ("racine 16", "4"),
            ("racine carrée(16) + 1", "5"),
            ("cbrt 8", "2"),
            ("racine cubique(27)", "3"),
            ("log(100)", "2"),
            ("log 2 (8)", "3"),
            ("ln(e)", "1"),
            ("abs(-42)", "42"),
            ("valeur absolue(-42)", "42"),
            ("round(3.7)", "4"),
            ("round 3.45", "3"),
            ("arrondi(3,7)", "4"),
            ("ceil(3.2)", "4"),
            ("plafond(3,2)", "4"),
            ("floor(3.9)", "3"),
            ("plancher(3,9)", "3"),
        ];
        for (line, result) in cases {
            assert_eq!(value(line), number(result, None), "{line}");
        }
        starts("racine 2 (8)", "2.82842712474619");
        starts("root 2 (8)", "2.82842712474619");
        starts("sqrt 2", "1.41421356237309504880168");
        starts("pi", "3.14159265358979");
        starts("π × 2", "6.28318530717958");
        starts("tau", "6.28318530717958");
        starts("τ", "6.28318530717958");
        starts("e", "2.71828182845904");
        starts("phi", "1.61803398874989");
        assert_eq!(value("e = 5\ne × 2"), number("10", None));
        assert_eq!(value("abs(-3 km)"), number("3", Some(UnitId::Kilometre)));
        assert_eq!(error("sqrt(-4)"), CalcError::NegativeRoot);
        assert_eq!(error("log(0)"), CalcError::LogDomain);
        assert_eq!(error("ln(-1)"), CalcError::LogDomain);
        assert_eq!(value("cbrt -8"), number("-2", None));
        let found = lines("a = 2\nsqrt(-4)\na × 2");
        assert!(matches!(&found[2], CalcLine::Value { value, .. } if *value == number("4", None)));
    }

    #[test]
    fn a_root_of_any_degree_is_computed_within_a_moment() {
        let started = std::time::Instant::now();
        starts("racine 4000000000 (2)", "1.000000000173286795");
        starts("racine 10000000000 (2)", "1.000000000069314");
        assert_eq!(value("racine 4000000000 (1)"), number("1", None));
        assert_eq!(value("racine 64 (18446744073709551616)"), number("2", None));
        assert_eq!(error("racine 10000000000 (-2)"), CalcError::NegativeRoot);
        assert_eq!(value("racine 10000000001 (-1)"), number("-1", None));
        assert_eq!(value("racine 10000000000 (0)"), number("0", None));
        assert_eq!(error("racine 2,5 (8)"), CalcError::Unsupported);
        starts("log 1,0000001 (2)", "6931472.15");
        match value("rendement annuel sur 1 € investis 2 € récupérés en 4000000000 ans") {
            CalcValue::Percent { value, .. } => {
                assert!(value.starts_with("0.0000000173286795"), "{value}");
            }
            other => panic!("{other:?}"),
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn numbers_are_read_and_written_in_other_bases() {
        assert_eq!(value("255 en hex"), based("FF", 16));
        assert_eq!(value("255 in hex"), based("FF", 16));
        assert_eq!(value("10 en bin"), based("1010", 2));
        assert_eq!(value("255 en oct"), based("377", 8));
        assert_eq!(value("0xFF"), number("255", None));
        assert_eq!(value("0b1010"), number("10", None));
        assert_eq!(value("0o377"), number("255", None));
        assert_eq!(value("0b101101 en base 8"), based("55", 8));
        assert_eq!(value("0xFF as base 2"), based("11111111", 2));
        assert_eq!(value("0xFF en base 10"), number("255", None));
        assert_eq!(value("hex(99)"), based("63", 16));
        assert_eq!(value("bin(0x73)"), based("1110011", 2));
        assert_eq!(value("int(0o55)"), number("45", None));
        assert_eq!(
            value("-10 en bin"),
            CalcValue::Base {
                digits: "1010".into(),
                radix: 2,
                negative: true
            }
        );
        assert_eq!(
            value("5300 en sci"),
            CalcValue::Scientific {
                mantissa: "5.3".into(),
                exponent: 3
            }
        );
        assert_eq!(value("5,3e+3"), number("5300", None));
        assert_eq!(error("2,5 en hex"), CalcError::NotWhole);
        assert!(matches!(error("5 km en hex"), CalcError::Mismatch { .. }));
        assert_eq!(error("255 en base 3"), CalcError::Unsupported);
    }

    #[test]
    fn bits_combine_whole_numbers_only() {
        let cases = [
            ("5 & 3", "1"),
            ("5 | 3", "7"),
            ("5 xor 3", "6"),
            ("1 << 4", "16"),
            ("16 >> 2", "4"),
            ("2 + 3 << 1", "10"),
            ("0xF0 | 0x0F", "255"),
        ];
        for (line, result) in cases {
            assert_eq!(value(line), number(result, None), "{line}");
        }
        assert_eq!(error("1,5 & 1"), CalcError::NotWhole);
        assert!(matches!(error("1 < 4"), CalcError::Unreadable { .. }));
    }

    #[test]
    fn the_larger_or_the_smaller_of_numbers_named_on_a_line() {
        assert_eq!(value("le plus grand de 100 et 200"), number("200", None));
        assert_eq!(value("larger of 100 and 200"), number("200", None));
        assert_eq!(value("le plus petit de 5 et 10"), number("5", None));
        assert_eq!(value("smaller of 5 and 10"), number("5", None));
        assert_eq!(value("le plus grand de 1, 5 et 3"), number("5", None));
        assert_eq!(
            value("le plus grand de 3 km et 500 m"),
            number("3", Some(UnitId::Kilometre))
        );
        assert!(matches!(
            error("larger of 3 km and 2 kg"),
            CalcError::Mismatch { .. }
        ));
        assert_eq!(value("le plus grand de 1 et 2 + 3"), number("5", None));
    }

    #[test]
    fn screen_units_follow_the_sheet_settings_from_their_line_down() {
        assert_eq!(value("12 pt en px"), number("16", Some(UnitId::Pixel)));
        assert_eq!(value("1 rem en px"), number("16", Some(UnitId::Pixel)));
        assert_eq!(
            value("em = 20 px\n1,2 em en px"),
            number("24", Some(UnitId::Pixel))
        );
        assert_eq!(
            value("em = 20\n1,2 em en px"),
            number("24", Some(UnitId::Pixel))
        );
        assert_eq!(
            value("rem = 18 px\n2 rem en px"),
            number("36", Some(UnitId::Pixel))
        );
        let found = lines("1 cm en px\nppi = 326\n1 cm en px");
        let figures = |line: &CalcLine| match line {
            CalcLine::Value {
                value: CalcValue::Number { value, .. },
                ..
            } => value.clone(),
            other => panic!("{other:?}"),
        };
        assert!(figures(&found[0]).starts_with("37.795275590551181"));
        assert!(figures(&found[2]).starts_with("128.346456692913385"));
        assert!(matches!(
            &found[1],
            CalcLine::Value { name: Some(name), value, .. } if name == "ppi" && *value == number("326", None)
        ));
        assert!(evaluate("ppi = 326\nem = 20", today()).variables.is_empty());
        assert_eq!(error("ppi = 0"), CalcError::NotPositive);
        assert!(matches!(error("ppi = 5 kg"), CalcError::Mismatch { .. }));
    }

    #[test]
    fn a_setting_line_is_no_figure_for_total_or_previous() {
        starts("ppi = 326\n1 cm en px\ntotal", "128.346456692913385");
        assert_eq!(
            value("10 €\n20 €\nem = 20 px\ntotal"),
            number("30", Some(UnitId::Euro))
        );
        assert_eq!(
            value("10 €\nppi = 326\npréc + 5 €"),
            number("15", Some(UnitId::Euro))
        );
    }

    #[test]
    fn frequencies_and_the_prefixes_a_developer_reads() {
        assert_eq!(value("2 MHz en Hz"), number("2000000", Some(UnitId::Hertz)));
        assert_eq!(
            value("1 GHz en MHz"),
            number("1000", Some(UnitId::Megahertz))
        );
        assert_eq!(value("1 / 2 s"), number("0.5", Some(UnitId::Hertz)));
        assert_eq!(value("60 Hz × 2 s"), number("120", None));
        assert_eq!(
            value("1 µs en ns"),
            number("1000", Some(UnitId::Nanosecond))
        );
        assert_eq!(
            value("1 us en ns"),
            number("1000", Some(UnitId::Nanosecond))
        );
        assert_eq!(
            value("5 nm en µm"),
            number("0.005", Some(UnitId::Micrometre))
        );
        assert_eq!(value("1 EB en PB"), number("1000", Some(UnitId::Petabyte)));
        assert_eq!(
            value("1 YiB en ZiB"),
            number("1024", Some(UnitId::Zebibyte))
        );
        assert_eq!(
            value("1 Pbit en Tbit"),
            number("1000", Some(UnitId::Terabit))
        );
        assert!(matches!(error("2 MHz + 3 s"), CalcError::Mismatch { .. }));
    }

    fn rounded(line: &str, places: u32) -> CalcValue {
        match value(line) {
            CalcValue::Number { value, unit } => CalcValue::Number {
                value: value
                    .parse::<Decimal>()
                    .unwrap()
                    .round_dp(places)
                    .normalize()
                    .to_string(),
                unit,
            },
            CalcValue::Percent { value, signed } => CalcValue::Percent {
                value: value
                    .parse::<Decimal>()
                    .unwrap()
                    .round_dp(places)
                    .normalize()
                    .to_string(),
                signed,
            },
            other => other,
        }
    }

    #[test]
    fn a_share_is_added_or_taken_off_and_the_whole_solved_for() {
        assert_eq!(value("5 % de plus sur 200"), number("210", None));
        assert_eq!(value("5% on 200"), number("210", None));
        assert_eq!(value("5 % de moins sur 200"), number("190", None));
        assert_eq!(value("5% off 200"), number("190", None));
        assert_eq!(value("5 % de combien font 6"), number("120", None));
        assert_eq!(value("5% of what is 6"), number("120", None));
        assert_eq!(
            rounded("5 % de plus sur combien font 6", 2),
            number("5.71", None)
        );
        assert_eq!(rounded("5% on what is 6", 2), number("5.71", None));
        assert_eq!(
            rounded("5 % de moins sur combien font 6", 2),
            number("6.32", None)
        );
        assert_eq!(rounded("5% off what is 6", 2), number("6.32", None));
        assert_eq!(
            value("5 % de combien font 6 €"),
            number("120", Some(UnitId::Euro))
        );
        assert_eq!(error("0 % de combien font 6"), CalcError::DivisionByZero);
    }

    #[test]
    fn a_value_as_a_share_of_another_and_a_change_between_two() {
        assert_eq!(value("50 en % de 100"), percent("50", false));
        assert_eq!(value("50 as a % of 100"), percent("50", false));
        assert_eq!(value("70 en % de plus que 20"), percent("250", false));
        assert_eq!(value("70 as a % on 20"), percent("250", false));
        assert_eq!(
            rounded("20 en % de moins que 70", 2),
            percent("71.43", false)
        );
        assert_eq!(rounded("20 as a % off 70", 2), percent("71.43", false));
        assert_eq!(value("50 à 75 c'est combien de %"), percent("50", true));
        assert_eq!(value("50 to 75 is what %"), percent("50", true));
        assert_eq!(value("40 à 90 en %"), percent("125", true));
        assert_eq!(value("40 to 90 as %"), percent("125", true));
        assert_eq!(
            value("180 c'est combien de % de moins que 200"),
            percent("10", false)
        );
        assert_eq!(value("180 is what % off 200"), percent("10", false));
        assert_eq!(
            value("180 c'est combien de % de plus que 150"),
            percent("20", false)
        );
        assert_eq!(value("180 is what % on 150"), percent("20", false));
        assert_eq!(value("20 c'est combien de % de 200"), percent("10", false));
        assert_eq!(value("20 is what % of 200"), percent("10", false));
        assert_eq!(value("1,44 To à 100 Mo/s"), number("4", Some(UnitId::Hour)));
    }

    #[test]
    fn fractions_multipliers_and_decimals() {
        let fraction = |numerator: &str, denominator: &str| CalcValue::Fraction {
            numerator: numerator.into(),
            denominator: denominator.into(),
            negative: false,
        };
        let times = |value: &str| CalcValue::Multiplier {
            value: value.into(),
        };
        assert_eq!(value("2/10 en fraction"), fraction("1", "5"));
        assert_eq!(value("2/10 as fraction"), fraction("1", "5"));
        assert_eq!(value("50 % en fraction"), fraction("1", "2"));
        assert_eq!(value("0,25 en fraction"), fraction("1", "4"));
        assert_eq!(value("1/3 en fraction"), fraction("1", "3"));
        assert_eq!(value("20/5 en multiple"), times("4"));
        assert_eq!(value("20/5 as multiplier"), times("4"));
        assert_eq!(value("50 en multiple de 5"), times("10"));
        assert_eq!(value("50 as x of 5"), times("10"));
        assert_eq!(value("50 à 75 c'est combien de fois"), times("1.5"));
        assert_eq!(value("50 to 75 is what x"), times("1.5"));
        assert_eq!(value("0,35 en %"), percent("35", false));
        assert_eq!(value("20/200 en %"), percent("10", false));
        assert_eq!(value("20 % en décimal"), number("0.2", None));
        assert_eq!(value("20 % as dec"), number("0.2", None));
        assert_eq!(value("50 % to decimal"), number("0.5", None));
        assert_eq!(value("1/3 to 2 decimal"), number("0.33", None));
        assert_eq!(value("x = 4\nx × 2"), number("8", None));
        assert_eq!(value("a = 50\na as a % of 100"), percent("50", false));
    }

    #[test]
    fn a_proportion_solves_for_its_unknown() {
        assert_eq!(
            value("6 est à 60 ce que 8 est à combien"),
            number("80", None)
        );
        assert_eq!(value("6 is to 60 as 8 is to what"), number("80", None));
        assert_eq!(
            value("5 est à 10 ce que combien est à 80"),
            number("40", None)
        );
        assert_eq!(value("5 is to 10 as what is to 80"), number("40", None));
        assert_eq!(
            value("2 h est à 120 km ce que 3 h est à combien"),
            number("180", Some(UnitId::Kilometre))
        );
        assert!(matches!(
            error("6 est à 60 ce que 8"),
            CalcError::Unreadable { .. }
        ));
    }

    #[test]
    fn the_five_questions_of_the_percentages_tool_read_as_before() {
        assert_eq!(value("20 % de 340"), number("68", None));
        assert_eq!(value("30 sur 120 en %"), percent("25", false));
        assert_eq!(value("de 80 à 100"), percent("25", true));
        assert_eq!(value("250 + 20 %"), number("300", None));
        assert_eq!(value("300 hors 20 %"), number("250", None));
        assert_eq!(value("20 % of 340"), number("68", None));
    }

    fn cents(line: &str) -> CalcValue {
        rounded(line, 2)
    }

    #[test]
    fn compound_interest_keeps_the_amount_s_currency() {
        let euros = |value: &str| number(value, Some(UnitId::Euro));
        let dollars = |value: &str| number(value, Some(UnitId::Dollar));
        assert_eq!(cents("1 000 € après 3 ans à 7 %"), euros("1225.04"));
        assert_eq!(cents("$1,000 after 3 years at 7%"), dollars("1225.04"));
        assert_eq!(cents("$1,000 after 3 years @ 7%"), dollars("1225.04"));
        assert_eq!(
            cents("1 000 € pendant 3 ans à 7 % composés mensuellement"),
            euros("1232.93")
        );
        assert_eq!(
            cents("$1,000 for 3 years at 7% compounding monthly"),
            dollars("1232.93")
        );
        assert_eq!(
            cents("1 000 € pendant 3 ans à 7 % composés trimestriellement"),
            euros("1231.44")
        );
        assert_eq!(
            cents("$1,000 after 3 years at 7% compounded quarterly"),
            dollars("1231.44")
        );
        assert_eq!(
            cents("intérêts sur 1 000 € après 3 ans à 7 %"),
            euros("225.04")
        );
        assert_eq!(
            cents("interest on $1,000 after 3 years @ 7%"),
            dollars("225.04")
        );
        assert_eq!(cents("1 000 € après 36 mois à 7 %"), euros("1225.04"));
        assert_eq!(error("1 000 € après 3 ans à 7"), CalcError::RateExpected);
        assert!(matches!(
            error("1 000 € après 3 km à 7 %"),
            CalcError::Mismatch { .. }
        ));
    }

    #[test]
    fn a_length_named_above_is_read_after_apres_as_a_number_is() {
        let euros = |value: &str| number(value, Some(UnitId::Euro));
        let named = "capital = 1 000 €\ndurée = 3 ans\ntaux = 7 %\n";
        assert_eq!(
            cents(&format!("{named}capital après durée à taux")),
            euros("1225.04")
        );
        assert_eq!(
            cents(&format!("{named}capital pendant durée à taux")),
            euros("1225.04")
        );
        assert_eq!(
            cents("principal = $1,000\nterm = 3 years\nprincipal after term @ 7%"),
            number("1225.04", Some(UnitId::Dollar))
        );
        assert_eq!(
            value("prix = 275 €\nprix for the \"Model 227\""),
            number("275", Some(UnitId::Euro))
        );
    }

    #[test]
    fn a_return_a_present_value_and_a_loan() {
        let euros = |value: &str| number(value, Some(UnitId::Euro));
        let dollars = |value: &str| number(value, Some(UnitId::Dollar));
        let times = CalcValue::Multiplier { value: "3".into() };
        assert_eq!(value("500 € investis 1 500 € récupérés"), times);
        assert_eq!(value("$500 invested $1,500 returned"), times);
        assert_eq!(
            cents("rendement annuel sur 1 000 € investis 2 500 € récupérés en 7 ans"),
            percent("13.99", false)
        );
        assert_eq!(
            cents("annual return on $1,000 invested $2,500 returned after 7 years"),
            percent("13.99", false)
        );
        assert_eq!(
            cents("valeur actuelle de 1 000 € dans 20 ans à 10 %"),
            euros("148.64")
        );
        assert_eq!(
            cents("present value of $1,000 after 20 years at 10%"),
            dollars("148.64")
        );
        assert_eq!(
            cents("mensualité de 10 000 € sur 6 ans à 6 %"),
            euros("165.73")
        );
        assert_eq!(
            cents("monthly repayment on $10,000 over 6 years at 6%"),
            dollars("165.73")
        );
        assert_eq!(
            cents("coût total de 10 000 € sur 6 ans à 6 %"),
            euros("11932.48")
        );
        assert_eq!(
            cents("total repayment on $10,000 over 6 years at 6%"),
            dollars("11932.48")
        );
        assert_eq!(
            cents("intérêts totaux de 10 000 € sur 6 ans à 6 %"),
            euros("1932.48")
        );
        assert_eq!(
            cents("total interest on $10,000 over 6 years at 6%"),
            dollars("1932.48")
        );
        assert_eq!(cents("mensualité de 1 200 € sur 1 an à 0 %"), euros("100"));
        assert_eq!(value("30 sur 120 en %"), percent("25", false));
        assert_eq!(value("a = 1\nb = 3\ntotal"), number("4", None));
    }

    #[test]
    fn powers_parentheses_and_signs() {
        assert_eq!(value("2 ^ 10"), number("1024", None));
        assert_eq!(value("(1 + 2) × -3"), number("-9", None));
        assert_eq!(value("1 / 4"), number("0.25", None));
    }
}
