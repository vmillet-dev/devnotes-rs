//! What a Note holds: GitHub's Markdown and nothing more — no underline, no footnote.

/// The marks a run of text wears; a link is a mark, as the reference's schema had it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "marks are worn together, not one of a set"
)]
pub struct Marks {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text {
        text: String,
        marks: Marks,
    },
    /// Shift+Enter, or two spaces at a line's end.
    HardBreak,
}

impl Inline {
    pub fn text(text: impl Into<String>) -> Self {
        Inline::Text {
            text: text.into(),
            marks: Marks::default(),
        }
    }

    /// How many `char`s it takes: a break is one.
    pub fn len(&self) -> usize {
        match self {
            Inline::Text { text, .. } => text.chars().count(),
            Inline::HardBreak => 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Bullet,
    Ordered {
        start: u64,
    },
    /// Each item checked or not.
    Task,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Meaningful in a task list alone.
    pub checked: bool,
    pub blocks: Vec<Block>,
}

pub type Cell = Vec<Inline>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    List {
        kind: ListKind,
        items: Vec<Item>,
    },
    Quote(Vec<Block>),
    CodeBlock {
        language: String,
        text: String,
    },
    /// The first row is the header.
    Table(Vec<Vec<Cell>>),
    Image {
        alt: String,
        src: String,
    },
    Rule,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

/// A place in the document: the path of indices down to a block holding text — through lists'
/// items, quotes' blocks and tables' cells — and a `char` offset into its text.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub path: Vec<usize>,
    pub offset: usize,
}

/// The anchor stays where the selection began; the head moves with the caret.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn caret(at: Position) -> Self {
        Self {
            anchor: at.clone(),
            head: at,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}
