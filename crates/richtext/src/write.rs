//! The document written back as the reference's editor wrote it, byte for byte: blocks a blank
//! line apart, tight lists, a table padded to its widest cell and set apart by one more blank
//! line on each side, a hard break as two spaces, and typed text escaped by `escape`.

use crate::escape::escape;
use crate::model::{Block, Cell, Document, Inline, Item, ListKind};

/// The narrowest column a table writes, as its separator's `---`.
const MIN_COLUMN: usize = 3;

pub fn write(document: &Document) -> String {
    blocks(&document.blocks, "\n\n")
        .trim_matches('\n')
        .to_owned()
}

fn blocks(blocks: &[Block], between: &str) -> String {
    blocks.iter().map(block).collect::<Vec<_>>().join(between)
}

fn block(block: &Block) -> String {
    match block {
        Block::Paragraph(content) => inlines(content),
        Block::Heading { level, content } => {
            format!("{} {}", "#".repeat(usize::from(*level)), inlines(content))
        }
        Block::List { kind, items } => list(*kind, items),
        Block::Quote(inner) => blocks(inner, "\n\n")
            .split('\n')
            .map(|line| {
                if line.is_empty() {
                    ">".to_owned()
                } else {
                    format!("> {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Block::CodeBlock { language, text } => {
            let newline = if text.ends_with('\n') || text.is_empty() {
                ""
            } else {
                "\n"
            };
            format!("```{language}\n{text}{newline}```")
        }
        Block::Table(rows) => format!("\n{}\n", table(rows)),
        Block::Image { alt, src } => format!("![{alt}]({src})"),
        Block::Rule => "---".to_owned(),
    }
}

fn list(kind: ListKind, items: &[Item]) -> String {
    items
        .iter()
        .enumerate()
        .map(|(at, item)| {
            let marker = match kind {
                ListKind::Bullet => "- ".to_owned(),
                ListKind::Ordered { start } => {
                    format!("{}. ", start + u64::try_from(at).unwrap_or(0))
                }
                ListKind::Task if item.checked => "- [x] ".to_owned(),
                ListKind::Task => "- [ ] ".to_owned(),
            };
            let indent = match kind {
                ListKind::Ordered { .. } => " ".repeat(marker.len()),
                ListKind::Bullet | ListKind::Task => "  ".to_owned(),
            };
            let body = blocks(&item.blocks, "\n");
            let mut lines = body.split('\n');
            let first = lines.next().unwrap_or_default();
            let rest: Vec<String> = lines
                .map(|line| {
                    if line.is_empty() {
                        String::new()
                    } else {
                        format!("{indent}{line}")
                    }
                })
                .collect();
            std::iter::once(format!("{marker}{first}"))
                .chain(rest)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn table(rows: &[Vec<Cell>]) -> String {
    let written: Vec<Vec<String>> = rows
        .iter()
        .map(|row| row.iter().map(|cell| inlines(cell)).collect())
        .collect();
    let columns = written.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|column| {
            written
                .iter()
                .filter_map(|row| row.get(column))
                .map(|cell| cell.chars().count())
                .max()
                .unwrap_or(0)
                .max(MIN_COLUMN)
        })
        .collect();
    let line = |cells: Vec<String>| format!("| {} |", cells.join(" | "));
    let row = |cells: &Vec<String>| {
        line(
            widths
                .iter()
                .enumerate()
                .map(|(column, width)| {
                    let cell = cells.get(column).map_or("", String::as_str);
                    let pad = width.saturating_sub(cell.chars().count());
                    format!("{cell}{}", " ".repeat(pad))
                })
                .collect(),
        )
    };
    let separator = line(widths.iter().map(|width| "-".repeat(*width)).collect());
    let mut out = Vec::new();
    for (at, cells) in written.iter().enumerate() {
        out.push(row(cells));
        if at == 0 {
            out.push(separator.clone());
        }
    }
    out.join("\n")
}

/// Marked runs: each mark's delimiters around the run that wears it.
fn inlines(content: &[Inline]) -> String {
    let mut out = String::new();
    for inline in content {
        match inline {
            Inline::HardBreak => out.push_str("  \n"),
            Inline::Text { text, marks } => {
                let starts_line = out.is_empty() || out.ends_with('\n');
                let mut run = if marks.code {
                    format!("`{text}`")
                } else {
                    escape(text, starts_line)
                };
                if marks.strike {
                    run = format!("~~{run}~~");
                }
                if marks.italic {
                    run = format!("*{run}*");
                }
                if marks.bold {
                    run = format!("**{run}**");
                }
                if let Some(href) = &marks.link {
                    run = format!("[{run}]({href})");
                }
                out.push_str(&run);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::{read, write};

    fn round(markdown: &str) -> String {
        write(&read(markdown))
    }

    #[test]
    fn nested_numbered_lists_indent_under_their_marker() {
        assert_eq!(
            round("1. un\n   1. un bis\n2. deux"),
            "1. un\n   1. un bis\n2. deux"
        );
    }

    #[test]
    fn an_image_in_a_paragraph_stands_on_its_own() {
        let written = round("avant ![x](devnotes:drawing/1)");
        assert!(
            written.ends_with("\n\n![x](devnotes:drawing/1)"),
            "{written}"
        );
    }

    #[test]
    fn a_code_block_keeps_its_text_and_a_bold_link_its_marks() {
        assert_eq!(round("```\na * b\n```"), "```\na * b\n```");
        assert_eq!(
            round("**[gras](https://a.fr)**"),
            "[**gras**](https://a.fr)"
        );
    }
}
