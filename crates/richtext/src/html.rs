//! A page's HTML pasted into a Note, read as the reference's editor read it (`golden/html-paste`):
//! headings, paragraphs, lists, quotes, tables, emphasis, links and line breaks; a `pre` as a code
//! block whose marks are dropped; a span's weight and slant as bold and italic. Everything else is
//! its text. `html5ever` tokenizes; the tree is built here, as `read` builds Markdown's.

use std::cell::RefCell;

use html5ever::tendril::StrTendril;
use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};

use crate::inlines;
use crate::model::{Block, Cell, Document, Inline, Item, ListKind, Marks};

/// What the clipboard's page wrote that a Note keeps formatted (the reference's `formatted()`):
/// emphasis, a link, a heading, a list, a quote or a table — and no code, which pastes as text.
pub fn formatted(html: &str) -> bool {
    let names: Vec<String> = pieces(html)
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Open { name, href, .. } if name != "a" || href.is_some() => Some(name),
            _ => None,
        })
        .collect();
    let has = |wanted: &[&str]| names.iter().any(|name| wanted.contains(&name.as_str()));
    !has(&["pre", "code"])
        && has(&[
            "b",
            "strong",
            "i",
            "em",
            "a",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "ul",
            "ol",
            "blockquote",
            "table",
        ])
}

/// The HTML as a document; a Windows `CF_HTML` header before the markup is dropped.
pub fn from_html(html: &str) -> Document {
    let mut builder = Builder::default();
    for piece in pieces(html) {
        builder.piece(piece);
    }
    builder.finish()
}

#[derive(Debug)]
enum Piece {
    Open {
        name: String,
        href: Option<String>,
        style: String,
    },
    Close(String),
    Text(String),
}

#[derive(Default)]
struct Sink {
    pieces: RefCell<Vec<Piece>>,
}

impl TokenSink for Sink {
    type Handle = ();

    fn process_token(&self, token: Token, _line: u64) -> TokenSinkResult<()> {
        let piece = match token {
            Token::TagToken(Tag {
                kind, name, attrs, ..
            }) => {
                let name = name.to_string();
                let attribute = |wanted: &str| {
                    attrs
                        .iter()
                        .find(|attr| &*attr.name.local == wanted)
                        .map(|attr| attr.value.to_string())
                };
                match kind {
                    TagKind::StartTag => Piece::Open {
                        href: attribute("href"),
                        style: attribute("style").unwrap_or_default(),
                        name,
                    },
                    TagKind::EndTag => Piece::Close(name),
                }
            }
            Token::CharacterTokens(text) => Piece::Text(text.to_string()),
            _ => return TokenSinkResult::Continue,
        };
        self.pieces.borrow_mut().push(piece);
        TokenSinkResult::Continue
    }
}

fn pieces(html: &str) -> Vec<Piece> {
    let markup = html.find('<').map_or(html, |at| &html[at..]);
    let tokenizer = Tokenizer::new(Sink::default(), TokenizerOpts::default());
    let input = BufferQueue::default();
    input.push_back(StrTendril::from_slice(markup));
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    tokenizer.sink.pieces.take()
}

/// What is being read, innermost last.
enum Open {
    Blocks(Vec<Block>),
    Quote(Vec<Block>),
    List { kind: ListKind, items: Vec<Item> },
    Item(Vec<Block>),
    Text { level: u8, content: Vec<Inline> },
    Code(String),
    Table(Vec<Vec<Cell>>),
    Row(Vec<Cell>),
    Cell(Cell),
}

struct Builder {
    open: Vec<Open>,
    marks: Marks,
    /// Each open inline tag and what it turned on.
    worn: Vec<(String, Marks)>,
    /// Inside `head`, `style` or `script`: nothing shown.
    hidden: u32,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            open: vec![Open::Blocks(Vec::new())],
            marks: Marks::default(),
            worn: Vec::new(),
            hidden: 0,
        }
    }
}

impl Builder {
    fn piece(&mut self, piece: Piece) {
        match piece {
            Piece::Open { name, href, style } => self.open_tag(&name, href, &style),
            Piece::Close(name) => self.close_tag(&name),
            Piece::Text(text) if self.hidden == 0 => self.text(&text),
            Piece::Text(_) => {}
        }
    }

    fn open_tag(&mut self, name: &str, href: Option<String>, style: &str) {
        match name {
            "head" | "style" | "script" | "title" => self.hidden += 1,
            "p" | "div" => self.start(Open::Text {
                level: 0,
                content: Vec::new(),
            }),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = name[1..].parse().unwrap_or(1);
                self.start(Open::Text {
                    level,
                    content: Vec::new(),
                });
            }
            "ul" | "ol" => {
                let kind = if name == "ol" {
                    ListKind::Ordered { start: 1 }
                } else {
                    ListKind::Bullet
                };
                self.start(Open::List {
                    kind,
                    items: Vec::new(),
                });
            }
            "li" => self.start(Open::Item(Vec::new())),
            "blockquote" => self.start(Open::Quote(Vec::new())),
            "pre" => self.start(Open::Code(String::new())),
            "table" => self.start(Open::Table(Vec::new())),
            "tr" => self.start(Open::Row(Vec::new())),
            "td" | "th" => self.start(Open::Cell(Vec::new())),
            "br" => self.line_break(),
            _ => self.wear(name, href, style),
        }
    }

    /// An inline tag's marks, kept until it closes.
    fn wear(&mut self, name: &str, href: Option<String>, style: &str) {
        let before = self.marks.clone();
        let style = style.to_ascii_lowercase().replace(' ', "");
        match name {
            "b" | "strong" => self.marks.bold = true,
            "i" | "em" => self.marks.italic = true,
            "s" | "del" | "strike" => self.marks.strike = true,
            "code" => self.marks.code = true,
            "a" => self.marks.link = href,
            "span" => {
                let weight = style
                    .split(';')
                    .find_map(|rule| rule.strip_prefix("font-weight:"))
                    .unwrap_or_default();
                if weight == "bold" || weight.parse::<u32>().is_ok_and(|weight| weight >= 600) {
                    self.marks.bold = true;
                }
                if style.contains("font-style:italic") {
                    self.marks.italic = true;
                }
            }
            _ => return,
        }
        self.worn.push((name.to_owned(), before));
    }

    fn close_tag(&mut self, name: &str) {
        match name {
            "head" | "style" | "script" | "title" => self.hidden = self.hidden.saturating_sub(1),
            "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "li"
            | "blockquote" | "pre" | "table" | "tr" | "td" | "th" => self.close_until(name),
            _ => {
                if let Some(at) = self.worn.iter().rposition(|(worn, _)| worn == name) {
                    self.marks = self.worn[at].1.clone();
                    self.worn.truncate(at);
                }
            }
        }
    }

    /// A block opens: text read so far in a paragraph of its own is closed first.
    fn start(&mut self, open: Open) {
        if matches!(self.open.last(), Some(Open::Text { .. })) {
            self.close();
        }
        self.open.push(open);
    }

    /// Closes what is open down to the block `name` opened.
    fn close_until(&mut self, name: &str) {
        let matches = |open: &Open| match open {
            Open::Text { .. } => ["p", "div", "h1", "h2", "h3", "h4", "h5", "h6"].contains(&name),
            Open::List { .. } => name == "ul" || name == "ol",
            Open::Item(_) => name == "li",
            Open::Quote(_) => name == "blockquote",
            Open::Code(_) => name == "pre",
            Open::Table(_) => name == "table",
            Open::Row(_) => name == "tr",
            Open::Cell(_) => name == "td" || name == "th",
            Open::Blocks(_) => false,
        };
        if !self.open.iter().any(matches) {
            return;
        }
        while let Some(last) = self.open.last() {
            let done = matches(last);
            self.close();
            if done {
                return;
            }
        }
    }

    fn close(&mut self) {
        let Some(open) = self.open.pop() else {
            return;
        };
        match open {
            Open::Blocks(blocks) => self.open.push(Open::Blocks(blocks)),
            Open::Text { level, mut content } => {
                trim(&mut content);
                if !content.is_empty() {
                    let block = if level == 0 {
                        Block::Paragraph(content)
                    } else {
                        Block::Heading { level, content }
                    };
                    self.block(block);
                }
            }
            Open::Quote(blocks) => self.block(Block::Quote(blocks)),
            Open::List { kind, items } => self.block(Block::List { kind, items }),
            Open::Item(blocks) => {
                if let Some(Open::List { items, .. }) = self.open.last_mut() {
                    items.push(Item {
                        checked: false,
                        blocks,
                    });
                }
            }
            Open::Code(text) => self.block(Block::CodeBlock {
                language: String::new(),
                text: text.trim_end_matches('\n').to_owned(),
            }),
            Open::Table(rows) => self.block(Block::Table(rows)),
            Open::Row(cells) => {
                if let Some(Open::Table(rows)) = self.open.last_mut() {
                    rows.push(cells);
                }
            }
            Open::Cell(mut cell) => {
                trim(&mut cell);
                if let Some(Open::Row(cells)) = self.open.last_mut() {
                    cells.push(cell);
                }
            }
        }
    }

    fn block(&mut self, block: Block) {
        if let Some(Open::Blocks(blocks) | Open::Quote(blocks) | Open::Item(blocks)) =
            self.open.last_mut()
        {
            blocks.push(block);
        }
    }

    fn line_break(&mut self) {
        if let Some(Open::Code(text)) = self.open.last_mut() {
            text.push('\n');
            return;
        }
        if let Some(content) = self.content() {
            content.push(Inline::HardBreak);
        }
    }

    /// Text outside a `pre` has its spaces collapsed, as a page shows it.
    fn text(&mut self, text: &str) {
        if let Some(Open::Code(code)) = self.open.last_mut() {
            code.push_str(text);
            return;
        }
        let mut collapsed = String::with_capacity(text.len());
        for glyph in text.chars() {
            if glyph.is_whitespace() && glyph != '\u{a0}' {
                if !collapsed.ends_with(' ') {
                    collapsed.push(' ');
                }
            } else {
                collapsed.push(glyph);
            }
        }
        let marks = self.marks.clone();
        let Some(content) = self.content() else {
            return;
        };
        let at_start = content.is_empty() || matches!(content.last(), Some(Inline::HardBreak));
        let collapsed = if at_start {
            collapsed.trim_start().to_owned()
        } else {
            collapsed
        };
        if collapsed.is_empty() {
            return;
        }
        let at = inlines::len(content);
        inlines::insert(
            content,
            at,
            Inline::Text {
                text: collapsed,
                marks,
            },
        );
    }

    /// Where inline content goes: the open paragraph or cell, or one opened for loose text.
    fn content(&mut self) -> Option<&mut Vec<Inline>> {
        if matches!(
            self.open.last(),
            Some(Open::Blocks(_) | Open::Quote(_) | Open::Item(_))
        ) {
            self.open.push(Open::Text {
                level: 0,
                content: Vec::new(),
            });
        }
        match self.open.last_mut() {
            Some(Open::Text { content, .. } | Open::Cell(content)) => Some(content),
            _ => None,
        }
    }

    fn finish(mut self) -> Document {
        while self.open.len() > 1 {
            self.close();
        }
        match self.open.pop() {
            Some(Open::Blocks(blocks)) => Document { blocks },
            _ => Document::default(),
        }
    }
}

/// A block's text without the spaces its markup left at its ends.
fn trim(content: &mut Vec<Inline>) {
    while matches!(content.last(), Some(Inline::HardBreak)) {
        content.pop();
    }
    if let Some(Inline::Text { text, .. }) = content.last_mut() {
        let kept = text.trim_end().len();
        text.truncate(kept);
    }
    inlines::normalize(content);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_s_html_reads_as_the_reference_read_it() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../../golden/html-paste/cases.json")).unwrap();
        let differing: Vec<String> = cases
            .iter()
            .filter_map(|case| {
                let html = case["input"].as_str().unwrap();
                let written = crate::write(&from_html(html));
                let expected = case["output"].as_str().unwrap();
                (written != expected)
                    .then(|| format!("{html}\n  gave {written:?}\n  want {expected:?}"))
            })
            .collect();
        assert!(differing.is_empty(), "{}", differing.join("\n"));
    }

    #[test]
    fn only_a_page_s_formatting_is_kept_and_code_pastes_as_text() {
        assert!(formatted("<p>Du <b>gras</b></p>"));
        assert!(formatted("<a href=\"https://a.fr\">lien</a>"));
        assert!(!formatted("<a>sans adresse</a>"));
        assert!(!formatted("<pre><code><b>npm</b></code></pre>"));
        assert!(!formatted("<div>du texte</div>"));
        assert!(!formatted("   "));
    }

    #[test]
    fn a_windows_header_before_the_markup_is_dropped() {
        let html = "Version:0.9\r\nStartHTML:0000000105\r\n<html><body><!--StartFragment--><b>gras</b><!--EndFragment--></body></html>";
        assert_eq!(crate::write(&from_html(html)), "**gras**");
    }
}
