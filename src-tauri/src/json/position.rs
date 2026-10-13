//! Where a reader's positions land in the front's terms: UTF-16 offsets, one-based lines and
//! columns. Each reader counts in its own unit — bytes, or a line and a column of characters —
//! and converts here.

use super::parse::{JsonError, JsonErrorReason, Span};

pub(crate) struct Positions<'a> {
    text: &'a str,
    /// The byte each line starts at.
    lines: Vec<usize>,
    /// The UTF-16 offset each line starts at.
    utf16: Vec<u32>,
}

impl<'a> Positions<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        let mut lines = vec![0];
        let mut utf16 = vec![0];
        let mut units = 0u32;
        for (at, character) in text.char_indices() {
            units += count(character.len_utf16());
            if character == '\n' {
                lines.push(at + 1);
                utf16.push(units);
            }
        }
        Self { text, lines, utf16 }
    }

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    /// A one-based line and a zero-based column counted in characters, as a byte.
    pub(crate) fn byte_at(&self, line: usize, column: usize) -> usize {
        let Some(&start) = self.lines.get(line.saturating_sub(1)) else {
            return self.text.len();
        };
        self.text[start..]
            .char_indices()
            .nth(column)
            .map_or(self.text.len(), |(at, _)| start + at)
    }

    pub(crate) fn utf16(&self, byte: usize) -> u32 {
        let byte = self.boundary(byte);
        let line = self.lines.partition_point(|&start| start <= byte) - 1;
        let start = self.lines[line];
        self.utf16[line] + count(self.text[start..byte].encode_utf16().count())
    }

    pub(crate) fn span(&self, start: usize, end: usize) -> Span {
        Span {
            start: self.utf16(start),
            end: self.utf16(end.max(start)),
        }
    }

    pub(crate) fn error(&self, reason: JsonErrorReason, byte: usize) -> JsonError {
        let byte = self.boundary(byte);
        let line = self.lines.partition_point(|&start| start <= byte) - 1;
        let start = self.lines[line];
        JsonError {
            reason,
            line: count(line) + 1,
            column: count(self.text[start..byte].encode_utf16().count()) + 1,
            offset: self.utf16(byte),
        }
    }

    /// Clamped to the text, and back to the start of the character it falls in.
    fn boundary(&self, byte: usize) -> usize {
        let mut byte = byte.min(self.text.len());
        while !self.text.is_char_boundary(byte) {
            byte -= 1;
        }
        byte
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_in_utf16_across_lines_and_wide_characters() {
        let positions = Positions::new("é: 1\n🦀: [2]\nz");

        assert_eq!(positions.byte_at(2, 3), "é: 1\n🦀: ".len());
        assert_eq!(positions.utf16(positions.byte_at(2, 3)), 9);
        assert_eq!(positions.utf16(positions.byte_at(3, 0)), 13);
        assert_eq!(positions.byte_at(9, 0), positions.text().len());
    }

    #[test]
    fn places_an_error_by_line_and_column_in_utf16() {
        let positions = Positions::new("a: 1\n🦀: :");
        let error = positions.error(JsonErrorReason::Malformed, "a: 1\n🦀: ".len());

        assert_eq!((error.line, error.column, error.offset), (2, 5, 9));
    }

    #[test]
    fn keeps_a_span_inside_the_text_and_on_characters() {
        let positions = Positions::new("é");

        assert_eq!(positions.span(1, 99), Span { start: 0, end: 1 });
        assert_eq!(positions.span(2, 0), Span { start: 1, end: 1 });
    }
}
