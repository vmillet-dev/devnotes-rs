//! A Calcul note: a sheet of lines, each computed as it is typed — variables, units, dates,
//! percentages, totals. The results are never stored: they are computed when the sheet is
//! read, here, and the front writes them in the language on screen.

mod eval;
mod lexer;
pub mod units;

use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;
use specta::Type;

use eval::{Context, Parser, Quantity, Shown, Value, is_keyword};
use lexer::{Tok, Token};
use units::{Dim, Dimension, UnitId};

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
    /// A date moved by something other than whole days, months or years.
    DateShift,
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

fn text(decimal: Decimal) -> String {
    decimal.normalize().to_string()
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

    /// Base units in one, as above over below, and what it measures.
    fn factor(&self, shown: Shown) -> Option<(Decimal, Decimal, Dim)> {
        match shown {
            Shown::One(unit) => self
                .factors
                .get(&unit)
                .map(|&(factor, dim)| (factor, Decimal::ONE, dim)),
            Shown::Per(above, below) => {
                let (top, top_dim) = self.factors.get(&above)?;
                let (bottom, bottom_dim) = self.factors.get(&below)?;
                Some((*top, *bottom, top_dim.over(*bottom_dim)))
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

    fn show(&self, value: &Value) -> Result<CalcValue, CalcError> {
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
            self.factor(*shown)
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
        let (above, below, _) = self.factor(shown).ok_or(CalcError::Unsupported)?;
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
        match lexer::tokens(line) {
            Ok(tokens) => self.expression(index, line, &tokens),
            Err(unreadable) if line.chars().any(|c| c.is_ascii_digit()) => row(
                CalcLine::Error {
                    error: CalcError::Unreadable { at: unreadable.at },
                },
                Vec::new(),
            ),
            Err(_) => row(CalcLine::Text, Vec::new()),
        }
    }

    fn expression(&mut self, index: u32, line: &str, tokens: &[Token]) -> CalcRow {
        let (name, body) = match tokens {
            [
                Token {
                    tok: Tok::Word(word),
                    ..
                },
                Token {
                    tok: Tok::Symbol('='),
                    ..
                },
                ..,
            ] if !is_keyword(word) => (Some(word.clone()), &tokens[2..]),
            _ => (None, tokens),
        };

        let context = Context {
            today: self.today,
            units: &self.units.table,
            symbols: &self.units.symbols,
            variables: &self.variables,
            block: &self.block,
        };
        let end = utf16(lexer::without_comment(line).trim_end());
        let mut parser = Parser::new(&context, body, end);
        let result = parser.whole();
        let mut spans = Vec::new();
        if name.is_some() {
            spans.push(CalcSpan {
                start: tokens[0].start,
                end: tokens[0].end,
                kind: TokenKind::Name,
            });
            spans.push(CalcSpan {
                start: tokens[1].start,
                end: tokens[1].end,
                kind: TokenKind::Operator,
            });
        }
        spans.extend(spans_of(body, &parser.kinds));
        spans.extend(trailing_comment(line));
        let (used, converting, aggregate) = (parser.used, parser.converting, parser.aggregate);

        for used_name in &used {
            if let Some(&definition) = self.current.get(used_name)
                && self.definitions[definition].uses.last() != Some(&index)
            {
                self.definitions[definition].uses.push(index);
            }
        }

        let line =
            match result.and_then(|value| self.units.show(&value).map(|shown| (value, shown))) {
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

    fn value(
        &mut self,
        index: u32,
        name: Option<String>,
        value: Value,
        shown: CalcValue,
        aggregate: bool,
    ) -> CalcLine {
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
        let dimension = value.dimension();
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
    fn powers_parentheses_and_signs() {
        assert_eq!(value("2 ^ 10"), number("1024", None));
        assert_eq!(value("(1 + 2) × -3"), number("-9", None));
        assert_eq!(value("1 / 4"), number("0.25", None));
    }
}
