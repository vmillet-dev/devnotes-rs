//! A snippet formatted by Prettier itself (plan D1, ADR S8): Prettier 3.9.10 and its plugins in
//! QuickJS, on a thread of their own, loaded by the first format and kept warm. It is handed
//! text, never code, and every `{{…}}` comes back as it went in (`shield`). Which parser a
//! language has is the caller's to say.

mod host;

use crate::{changed_lines, shield_fields, unshield_fields};

/// One per grammar Prettier reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parser {
    Babel,
    Typescript,
    Json,
    Css,
    Scss,
    Less,
    Html,
    Vue,
    Angular,
    Markdown,
    Yaml,
    Graphql,
}

impl Parser {
    fn name(self) -> &'static str {
        match self {
            Parser::Babel => "babel",
            Parser::Typescript => "typescript",
            Parser::Json => "json",
            Parser::Css => "css",
            Parser::Scss => "scss",
            Parser::Less => "less",
            Parser::Html => "html",
            Parser::Vue => "vue",
            Parser::Angular => "angular",
            Parser::Markdown => "markdown",
            Parser::Yaml => "yaml",
            Parser::Graphql => "graphql",
        }
    }
}

/// Prettier's own options, as the library's settings make them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Prettier's own switches, one for one"
)]
pub struct Options {
    pub print_width: u32,
    pub tab_width: u32,
    pub use_tabs: bool,
    pub single_quote: bool,
    pub semi: bool,
    /// `all`, or `none`.
    pub trailing_comma: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub text: String,
    /// A `char` offset, as the editor counts.
    pub cursor: usize,
    pub parser: Parser,
    pub options: Options,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Formatted {
        text: String,
        cursor: usize,
        /// Zero-based, in the formatted text.
        changed_lines: Vec<usize>,
    },
    Unchanged,
    /// One-based, as Prettier reports them.
    Syntax {
        line: u32,
        column: u32,
    },
    /// A `{{…}}` did not come back as it went in: nothing is applied.
    Fields,
    Failed,
}

/// Blocks until Prettier answers: called off the window's thread.
pub fn format(request: &Request) -> Answer {
    let Some(shielded) = shield_fields(&request.text, request.cursor) else {
        return Answer::Failed;
    };
    let cursor = utf16_of(&shielded.text, shielded.cursor);
    let printed = match host::print(&shielded.text, cursor, request.parser, request.options) {
        host::Printed::Text { text, cursor } => {
            let cursor = chars_of(&text, cursor);
            (text, cursor)
        }
        host::Printed::Syntax { line, column } => return Answer::Syntax { line, column },
        host::Printed::Failed => return Answer::Failed,
    };
    let Some((mut text, cursor)) = unshield_fields(&printed.0, printed.1, &shielded) else {
        return Answer::Fields;
    };
    // A snippet ends where it was written to end; Prettier's closing newline is a file's.
    if !request.text.ends_with('\n') {
        let kept = text
            .strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .map(str::len);
        if let Some(kept) = kept {
            text.truncate(kept);
        }
    }
    if text == request.text {
        return Answer::Unchanged;
    }
    Answer::Formatted {
        cursor: cursor.min(text.chars().count()),
        changed_lines: changed_lines(&request.text, &text),
        text,
    }
}

/// Prettier counts in UTF-16 units, the editor in `char`s.
fn utf16_of(text: &str, chars: usize) -> usize {
    text.chars().take(chars).map(char::len_utf16).sum()
}

fn chars_of(text: &str, units: usize) -> usize {
    let mut counted = 0;
    text.chars()
        .take_while(|glyph| {
            counted += glyph.len_utf16();
            counted <= units
        })
        .count()
}

#[cfg(test)]
mod tests;
