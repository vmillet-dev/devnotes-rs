//! A JSON parser that keeps where each value sits, which `serde_json` does not.

use serde::Serialize;
use specta::Type;

/// Offsets in UTF-16 code units: the front's strings count in them, and "Voir dans le code"
/// puts a caret with one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Object(Vec<(String, Node)>),
    Array(Vec<Node>),
    String(String),
    /// As written: `1e3` stays `1e3`, and a number past `f64` loses nothing.
    Number(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Node {
    pub(crate) value: Value,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum JsonErrorReason {
    UnexpectedEnd,
    UnexpectedCharacter,
    InvalidNumber,
    InvalidEscape,
    ControlCharacter,
    TrailingCharacters,
    TooDeep,
}

/// One-based line and column, the column in UTF-16 units like the offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct JsonError {
    pub reason: JsonErrorReason,
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}

/// Recursion is bounded here rather than by the stack: a text of ten thousand `[` would
/// otherwise take the process down.
const MAX_DEPTH: usize = 256;

pub(crate) fn parse(text: &str) -> Result<Node, JsonError> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
        utf16: 0,
    };

    let root = parser
        .value(0)
        .map_err(|reason| parser.error(text, reason))?;
    parser.whitespace();
    if parser.at < parser.bytes.len() {
        return Err(parser.error(text, JsonErrorReason::TrailingCharacters));
    }
    Ok(root)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    utf16: u32,
}

type Parsed<T> = Result<T, JsonErrorReason>;

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// A continuation byte adds nothing, a four-byte sequence's lead a surrogate pair.
    fn bump(&mut self) {
        let byte = self.bytes[self.at];
        self.at += 1;
        self.utf16 += match byte {
            0x80..=0xBF => 0,
            0xF0..=0xFF => 2,
            _ => 1,
        };
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.bump();
        }
    }

    fn expect(&mut self, byte: u8) -> Parsed<()> {
        match self.peek() {
            Some(found) if found == byte => {
                self.bump();
                Ok(())
            }
            Some(_) => Err(JsonErrorReason::UnexpectedCharacter),
            None => Err(JsonErrorReason::UnexpectedEnd),
        }
    }

    fn value(&mut self, depth: usize) -> Parsed<Node> {
        if depth > MAX_DEPTH {
            return Err(JsonErrorReason::TooDeep);
        }
        self.whitespace();
        let start = self.utf16;
        let value = match self.peek() {
            None => return Err(JsonErrorReason::UnexpectedEnd),
            Some(b'{') => self.object(depth)?,
            Some(b'[') => self.array(depth)?,
            Some(b'"') => Value::String(self.string()?),
            Some(b't') => self.literal("true", Value::Bool(true))?,
            Some(b'f') => self.literal("false", Value::Bool(false))?,
            Some(b'n') => self.literal("null", Value::Null)?,
            Some(b'-' | b'0'..=b'9') => Value::Number(self.number()?),
            Some(_) => return Err(JsonErrorReason::UnexpectedCharacter),
        };
        Ok(Node {
            value,
            span: Span {
                start,
                end: self.utf16,
            },
        })
    }

    fn object(&mut self, depth: usize) -> Parsed<Value> {
        self.bump();
        let mut members = Vec::new();
        self.whitespace();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(Value::Object(members));
        }
        loop {
            self.whitespace();
            match self.peek() {
                Some(b'"') => {}
                Some(_) => return Err(JsonErrorReason::UnexpectedCharacter),
                None => return Err(JsonErrorReason::UnexpectedEnd),
            }
            let key = self.string()?;
            self.whitespace();
            self.expect(b':')?;
            members.push((key, self.value(depth + 1)?));
            self.whitespace();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b'}') => {
                    self.bump();
                    return Ok(Value::Object(members));
                }
                Some(_) => return Err(JsonErrorReason::UnexpectedCharacter),
                None => return Err(JsonErrorReason::UnexpectedEnd),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Parsed<Value> {
        self.bump();
        let mut items = Vec::new();
        self.whitespace();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.whitespace();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b']') => {
                    self.bump();
                    return Ok(Value::Array(items));
                }
                Some(_) => return Err(JsonErrorReason::UnexpectedCharacter),
                None => return Err(JsonErrorReason::UnexpectedEnd),
            }
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Parsed<Value> {
        for expected in word.bytes() {
            self.expect(expected)?;
        }
        Ok(value)
    }

    fn digits(&mut self) -> usize {
        let from = self.at;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump();
        }
        self.at - from
    }

    fn number(&mut self) -> Parsed<String> {
        let from = self.at;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        match self.peek() {
            Some(b'0') => self.bump(),
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(JsonErrorReason::InvalidNumber),
        }
        if self.peek() == Some(b'.') {
            self.bump();
            if self.digits() == 0 {
                return Err(JsonErrorReason::InvalidNumber);
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.bump();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump();
            }
            if self.digits() == 0 {
                return Err(JsonErrorReason::InvalidNumber);
            }
        }
        Ok(String::from_utf8_lossy(&self.bytes[from..self.at]).into_owned())
    }

    fn string(&mut self) -> Parsed<String> {
        self.bump();
        let mut text: Vec<u8> = Vec::new();
        loop {
            match self.peek() {
                None => return Err(JsonErrorReason::UnexpectedEnd),
                Some(b'"') => {
                    self.bump();
                    return Ok(String::from_utf8_lossy(&text).into_owned());
                }
                Some(b'\\') => {
                    self.bump();
                    let decoded = self.escape()?;
                    let mut buffer = [0; 4];
                    text.extend_from_slice(decoded.encode_utf8(&mut buffer).as_bytes());
                }
                Some(0x00..=0x1F) => return Err(JsonErrorReason::ControlCharacter),
                Some(byte) => {
                    text.push(byte);
                    self.bump();
                }
            }
        }
    }

    fn escape(&mut self) -> Parsed<char> {
        let Some(byte) = self.peek() else {
            return Err(JsonErrorReason::UnexpectedEnd);
        };
        self.bump();
        Ok(match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => return self.unicode(),
            _ => return Err(JsonErrorReason::InvalidEscape),
        })
    }

    /// A lone surrogate is written as U+FFFD: valid JSON, but no character.
    fn unicode(&mut self) -> Parsed<char> {
        let first = self.hex4()?;
        if !(0xD800..=0xDBFF).contains(&first) {
            return Ok(char::from_u32(first).unwrap_or(char::REPLACEMENT_CHARACTER));
        }
        if self.bytes.get(self.at..self.at + 2) != Some(b"\\u") {
            return Ok(char::REPLACEMENT_CHARACTER);
        }
        self.bump();
        self.bump();
        let second = self.hex4()?;
        if !(0xDC00..=0xDFFF).contains(&second) {
            return Ok(char::REPLACEMENT_CHARACTER);
        }
        let code = 0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00);
        Ok(char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER))
    }

    fn hex4(&mut self) -> Parsed<u32> {
        let mut code = 0;
        for _ in 0..4 {
            let digit = match self.peek() {
                None => return Err(JsonErrorReason::UnexpectedEnd),
                Some(byte) => char::from(byte)
                    .to_digit(16)
                    .ok_or(JsonErrorReason::InvalidEscape)?,
            };
            self.bump();
            code = code * 16 + digit;
        }
        Ok(code)
    }

    fn error(&self, text: &str, reason: JsonErrorReason) -> JsonError {
        let before = &text[..self.at.min(text.len())];
        let line_start = before.rfind('\n').map_or(0, |at| at + 1);
        let column = before[line_start..].encode_utf16().count();

        JsonError {
            reason,
            line: count(before.matches('\n').count()) + 1,
            column: count(column) + 1,
            offset: self.utf16,
        }
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error(text: &str) -> JsonError {
        parse(text).expect_err("an invalid document")
    }

    #[test]
    fn keeps_every_value_and_where_it_sits() {
        let root = parse(r#"{"a": [1, "é", true, null], "b": {}}"#).unwrap();
        let Value::Object(members) = &root.value else {
            panic!("an object");
        };

        assert_eq!(members[0].0, "a");
        let Value::Array(items) = &members[0].1.value else {
            panic!("an array");
        };
        assert_eq!(items[0].value, Value::Number("1".into()));
        assert_eq!(items[1].value, Value::String("é".into()));
        assert_eq!(items[2].value, Value::Bool(true));
        assert_eq!(items[3].value, Value::Null);
        assert_eq!(members[0].1.span, Span { start: 6, end: 26 });
        assert_eq!(root.span, Span { start: 0, end: 36 });
    }

    #[test]
    fn counts_offsets_in_utf16_units() {
        let root = parse(r#"["😀", 1]"#).unwrap();
        let Value::Array(items) = root.value else {
            panic!("an array");
        };

        // The emoji is one character, two UTF-16 units and four bytes.
        assert_eq!(items[0].span, Span { start: 1, end: 5 });
        assert_eq!(items[1].span, Span { start: 7, end: 8 });
    }

    #[test]
    fn decodes_escapes_and_surrogate_pairs() {
        let root = parse(r#""a\"b\\c\/\né😀\ud800""#).unwrap();

        assert_eq!(root.value, Value::String("a\"b\\c/\né😀\u{fffd}".into()));
    }

    #[test]
    fn keeps_a_number_as_written() {
        assert_eq!(
            parse("-1.5e+10").unwrap().value,
            Value::Number("-1.5e+10".into())
        );
    }

    #[test]
    fn says_where_a_document_breaks() {
        let broken = error("{\n  \"a\": 1,,\n}");

        assert_eq!(broken.reason, JsonErrorReason::UnexpectedCharacter);
        assert_eq!((broken.line, broken.column, broken.offset), (2, 10, 11));
    }

    #[test]
    fn names_what_went_wrong() {
        assert_eq!(error("").reason, JsonErrorReason::UnexpectedEnd);
        assert_eq!(error("[1,").reason, JsonErrorReason::UnexpectedEnd);
        assert_eq!(error("01").reason, JsonErrorReason::TrailingCharacters);
        assert_eq!(error("1.").reason, JsonErrorReason::InvalidNumber);
        assert_eq!(error("-").reason, JsonErrorReason::InvalidNumber);
        assert_eq!(error("1e").reason, JsonErrorReason::InvalidNumber);
        assert_eq!(error(r#""\x""#).reason, JsonErrorReason::InvalidEscape);
        assert_eq!(error(r#""\u12G4""#).reason, JsonErrorReason::InvalidEscape);
        assert_eq!(error("\"a\nb\"").reason, JsonErrorReason::ControlCharacter);
        assert_eq!(error("{} {}").reason, JsonErrorReason::TrailingCharacters);
        assert_eq!(
            error("{\"a\" 1}").reason,
            JsonErrorReason::UnexpectedCharacter
        );
        assert_eq!(error("{1: 2}").reason, JsonErrorReason::UnexpectedCharacter);
        assert_eq!(error("[1 2]").reason, JsonErrorReason::UnexpectedCharacter);
        assert_eq!(error("tru").reason, JsonErrorReason::UnexpectedEnd);
        assert_eq!(error("nul!").reason, JsonErrorReason::UnexpectedCharacter);
        assert_eq!(error("{\"a\":1").reason, JsonErrorReason::UnexpectedEnd);
        assert_eq!(error("{\"a").reason, JsonErrorReason::UnexpectedEnd);
        assert_eq!(error("{").reason, JsonErrorReason::UnexpectedEnd);
        assert_eq!(error("\"\\").reason, JsonErrorReason::UnexpectedEnd);
        assert_eq!(error("\"\\u12").reason, JsonErrorReason::UnexpectedEnd);
        assert_eq!(error("?").reason, JsonErrorReason::UnexpectedCharacter);
    }

    #[test]
    fn refuses_a_nesting_deeper_than_it_can_walk() {
        let deep = "[".repeat(10_000);

        assert_eq!(error(&deep).reason, JsonErrorReason::TooDeep);
    }

    #[test]
    fn reads_a_scalar_document_and_blank_around_it() {
        assert_eq!(
            parse(" \n 42 \t").unwrap().value,
            Value::Number("42".into())
        );
        assert_eq!(parse("[]").unwrap().value, Value::Array(Vec::new()));
    }
}
