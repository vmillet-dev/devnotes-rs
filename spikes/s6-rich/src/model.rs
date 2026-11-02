//! The document model of `richtext`, reduced: blocks, inline runs with marks, positions.

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading(u8, Vec<Inline>),
    List { ordered: Option<u64>, items: Vec<Item> },
    Quote(Vec<Block>),
    Code { language: String, text: String },
    Table { header: Vec<Vec<Inline>>, rows: Vec<Vec<Vec<Inline>>> },
    Rule,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Marks {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text { text: String, marks: Marks, link: Option<String> },
    Image { alt: String, src: String },
    HardBreak,
}

impl Block {
    /// The plain text of a block's own line(s), as the caret walks it.
    pub fn text(&self) -> String {
        let inline = |runs: &[Inline]| {
            runs.iter()
                .map(|r| match r {
                    Inline::Text { text, .. } => text.clone(),
                    Inline::Image { .. } => "\u{fffc}".into(),
                    Inline::HardBreak => "\n".into(),
                })
                .collect::<String>()
        };
        match self {
            Block::Paragraph(runs) | Block::Heading(_, runs) => inline(runs),
            Block::Code { text, .. } => text.clone(),
            _ => String::new(),
        }
    }
}

/// Input rules of TipTap's StarterKit, applied when a paragraph's text starts with a trigger.
pub fn input_rule(block: &Block) -> Option<Block> {
    let Block::Paragraph(runs) = block else { return None };
    let [Inline::Text { text, .. }] = runs.as_slice() else { return None };
    let plain = |rest: &str| vec![Inline::Text { text: rest.into(), marks: Marks::default(), link: None }];
    for level in (1..=6u8).rev() {
        if let Some(rest) = text.strip_prefix(&format!("{} ", "#".repeat(level as usize))) {
            return Some(Block::Heading(level, plain(rest)));
        }
    }
    for (trigger, task) in [("[ ] ", Some(false)), ("[x] ", Some(true)), ("- ", None), ("* ", None), ("+ ", None)] {
        if let Some(rest) = text.strip_prefix(trigger) {
            return Some(Block::List { ordered: None, items: vec![Item { task, blocks: vec![Block::Paragraph(plain(rest))] }] });
        }
    }
    if let Some(rest) = text.strip_prefix("1. ") {
        return Some(Block::List { ordered: Some(1), items: vec![Item { task: None, blocks: vec![Block::Paragraph(plain(rest))] }] });
    }
    if let Some(rest) = text.strip_prefix("> ") {
        return Some(Block::Quote(vec![Block::Paragraph(plain(rest))]));
    }
    if text == "---" {
        return Some(Block::Rule);
    }
    if let Some(language) = text.strip_prefix("```") {
        return Some(Block::Code { language: language.trim().into(), text: String::new() });
    }
    // `**bold**` closing: the marked run replaces the delimiters.
    if let Some(inner) = text.strip_suffix("**").and_then(|t| t.rfind("**").map(|at| (at, t))) {
        let (at, t) = inner;
        let mut out = plain(&t[..at]);
        out.push(Inline::Text { text: t[at + 2..].into(), marks: Marks { bold: true, ..Marks::default() }, link: None });
        out.retain(|r| !matches!(r, Inline::Text { text, .. } if text.is_empty()));
        return Some(Block::Paragraph(out));
    }
    None
}
