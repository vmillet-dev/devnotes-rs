//! Markdown read into the document by `pulldown-cmark`, with the core's options (`notes::markdown`,
//! held equal by a test there): what the schema does not hold is read as what it shows — raw HTML
//! as its text, a footnote as its brackets, a reference link as a link.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::model::{Block, Cell, Document, Inline, Item, ListKind, Marks};

pub const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_STRIKETHROUGH);

/// What is being read, innermost last.
enum Open {
    Blocks(Vec<Block>),
    Quote(Vec<Block>),
    List {
        kind: ListKind,
        items: Vec<Item>,
    },
    Item(Item),
    /// Text not in a paragraph of its own: a tight list's item, or an HTML block.
    Loose(Vec<Inline>),
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Code {
        language: String,
        text: String,
    },
    Table(Vec<Vec<Cell>>),
    Row(Vec<Cell>),
    Cell(Cell),
    Image {
        src: String,
        alt: String,
    },
}

struct Reader {
    open: Vec<Open>,
    marks: Marks,
    /// Emphasis can nest in itself: `*a *b* c*`.
    depth: [u32; 3],
}

pub fn read(markdown: &str) -> Document {
    let mut reader = Reader {
        open: vec![Open::Blocks(Vec::new())],
        marks: Marks::default(),
        depth: [0; 3],
    };
    for event in Parser::new_ext(markdown, OPTIONS) {
        reader.event(event);
    }
    while reader.open.len() > 1 {
        reader.close();
    }
    match reader.open.pop() {
        Some(Open::Blocks(blocks)) => Document { blocks },
        _ => Document::default(),
    }
}

impl Reader {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(TagEnd::Emphasis) => self.mark(0, false),
            Event::End(TagEnd::Strong) => self.mark(1, false),
            Event::End(TagEnd::Strikethrough) => self.mark(2, false),
            Event::End(TagEnd::Link) => self.marks.link = None,
            Event::End(TagEnd::HtmlBlock) => self.close(),
            Event::End(_) => {
                self.end_loose();
                self.close();
            }
            Event::Text(text) => self.text(&text),
            Event::Code(code) => {
                let marks = Marks {
                    code: true,
                    ..self.marks.clone()
                };
                self.inline(Inline::Text {
                    text: code.into_string(),
                    marks,
                });
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                let text = without_tags(&html);
                self.text(text.trim_end_matches('\n'));
            }
            Event::SoftBreak => self.text("\n"),
            Event::HardBreak => self.inline(Inline::HardBreak),
            Event::Rule => self.block(Block::Rule),
            Event::TaskListMarker(checked) => {
                if let Some(Open::Item(item)) = self.open.last_mut() {
                    item.checked = checked;
                }
                if let Some(Open::List { kind, .. }) = self.open.iter_mut().rev().nth(1) {
                    *kind = ListKind::Task;
                }
            }
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        if is_block(&tag) {
            self.end_loose();
        }
        let open = match tag {
            Tag::Paragraph => Open::Paragraph(Vec::new()),
            Tag::Heading { level, .. } => Open::Heading {
                level: heading_level(level),
                content: Vec::new(),
            },
            Tag::BlockQuote(_) => Open::Quote(Vec::new()),
            Tag::CodeBlock(kind) => Open::Code {
                language: match kind {
                    CodeBlockKind::Fenced(language) => language.into_string(),
                    CodeBlockKind::Indented => String::new(),
                },
                text: String::new(),
            },
            Tag::HtmlBlock => Open::Loose(Vec::new()),
            Tag::List(start) => Open::List {
                kind: start.map_or(ListKind::Bullet, |start| ListKind::Ordered { start }),
                items: Vec::new(),
            },
            Tag::Item => Open::Item(Item {
                checked: false,
                blocks: Vec::new(),
            }),
            Tag::Table(_) => Open::Table(Vec::new()),
            Tag::TableHead | Tag::TableRow => Open::Row(Vec::new()),
            Tag::TableCell => Open::Cell(Vec::new()),
            Tag::Emphasis => return self.mark(0, true),
            Tag::Strong => return self.mark(1, true),
            Tag::Strikethrough => return self.mark(2, true),
            Tag::Link { dest_url, .. } => {
                self.marks.link = Some(dest_url.into_string());
                return;
            }
            Tag::Image { dest_url, .. } => Open::Image {
                src: dest_url.into_string(),
                alt: String::new(),
            },
            _ => return,
        };
        self.open.push(open);
    }

    fn mark(&mut self, which: usize, on: bool) {
        let depth = &mut self.depth[which];
        *depth = if on {
            *depth + 1
        } else {
            depth.saturating_sub(1)
        };
        let worn = *depth > 0;
        match which {
            0 => self.marks.italic = worn,
            1 => self.marks.bold = worn,
            _ => self.marks.strike = worn,
        }
    }

    fn close(&mut self) {
        let Some(open) = self.open.pop() else {
            return;
        };
        match open {
            Open::Blocks(blocks) => self.open.push(Open::Blocks(blocks)),
            Open::Quote(blocks) => self.block(Block::Quote(blocks)),
            Open::List { kind, items } => self.block(Block::List { kind, items }),
            Open::Item(item) => {
                if let Some(Open::List { items, .. }) = self.open.last_mut() {
                    items.push(item);
                }
            }
            Open::Loose(content) | Open::Paragraph(content) => {
                if !content.is_empty() {
                    self.block(Block::Paragraph(content));
                }
            }
            Open::Heading { level, content } => self.block(Block::Heading { level, content }),
            Open::Code { language, text } => self.block(Block::CodeBlock { language, text }),
            Open::Table(rows) => self.block(Block::Table(rows)),
            Open::Row(cells) => {
                if let Some(Open::Table(rows)) = self.open.last_mut() {
                    rows.push(cells);
                }
            }
            Open::Cell(cell) => {
                if let Some(Open::Row(cells)) = self.open.last_mut() {
                    cells.push(cell);
                }
            }
            Open::Image { src, alt } => self.block(Block::Image { alt, src }),
        }
    }

    /// A block where it belongs; a paragraph read so far is closed first, so it comes before.
    fn block(&mut self, block: Block) {
        if let Some(Open::Paragraph(content)) = self.open.last_mut() {
            let content = std::mem::take(content);
            self.open.pop();
            if !content.is_empty() {
                self.block(Block::Paragraph(content));
            }
            self.block(block);
            self.open.push(Open::Paragraph(Vec::new()));
            return;
        }
        self.end_loose();
        match self.open.last_mut() {
            Some(Open::Blocks(blocks) | Open::Quote(blocks)) => blocks.push(block),
            Some(Open::Item(item)) => item.blocks.push(block),
            _ => {}
        }
    }

    /// A tight item's text becomes its first paragraph once a block follows it.
    fn end_loose(&mut self) {
        if matches!(self.open.last(), Some(Open::Loose(_))) {
            self.close();
        }
    }

    fn text(&mut self, text: &str) {
        if let Some(Open::Code { text: code, .. }) = self.open.last_mut() {
            code.push_str(text);
            return;
        }
        if let Some(Open::Image { alt, .. }) = self.open.last_mut() {
            alt.push_str(text);
            return;
        }
        if self.marks.link.is_some() {
            let marks = self.marks.clone();
            self.inline(Inline::Text {
                text: text.to_owned(),
                marks,
            });
            return;
        }
        for (piece, link) in autolinks(text) {
            let marks = Marks {
                link,
                ..self.marks.clone()
            };
            self.inline(Inline::Text {
                text: piece.to_owned(),
                marks,
            });
        }
    }

    /// Into the paragraph, heading or cell being read; a list's tight item opens one of its own.
    fn inline(&mut self, inline: Inline) {
        if matches!(
            self.open.last(),
            Some(Open::Item(_) | Open::Blocks(_) | Open::Quote(_))
        ) {
            self.open.push(Open::Loose(Vec::new()));
        }
        let Some(
            Open::Paragraph(content)
            | Open::Loose(content)
            | Open::Heading { content, .. }
            | Open::Cell(content),
        ) = self.open.last_mut()
        else {
            return;
        };
        if let (
            Some(Inline::Text { text, marks }),
            Inline::Text {
                text: more,
                marks: worn,
            },
        ) = (content.last_mut(), &inline)
            && marks == worn
        {
            text.push_str(more);
            return;
        }
        content.push(inline);
    }
}

fn is_block(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::List(_)
            | Tag::Item
            | Tag::Table(_)
    )
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// HTML read as the text it shows.
fn without_tags(html: &str) -> String {
    let mut text = String::new();
    let mut inside = false;
    for glyph in html.chars() {
        match glyph {
            '<' => inside = true,
            '>' if inside => inside = false,
            _ if !inside => text.push(glyph),
            _ => {}
        }
    }
    text
}

/// A bare `http://` or `https://` address is a link to itself, as the reference's editor read it.
fn autolinks(text: &str) -> Vec<(&str, Option<String>)> {
    let mut pieces = Vec::new();
    let mut rest = text;
    while let Some(at) = ["https://", "http://"]
        .iter()
        .filter_map(|scheme| rest.find(scheme))
        .min()
    {
        let end = rest[at..]
            .find(char::is_whitespace)
            .map_or(rest.len(), |end| at + end);
        let url = rest[at..end].trim_end_matches(['.', ',', ';', ':', '!', '?', ')']);
        let end = at + url.len();
        if !rest[..at].is_empty() {
            pieces.push((&rest[..at], None));
        }
        pieces.push((&rest[at..end], Some(url.to_owned())));
        rest = &rest[end..];
    }
    if !rest.is_empty() {
        pieces.push((rest, None));
    }
    pieces
}
