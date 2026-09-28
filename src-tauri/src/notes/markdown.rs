//! What a Note reads as once its Markdown is taken away: a card's preview, a search
//! excerpt. The rich editor writes the body; nobody reads `**` or `- [ ]` on a card.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::model::OutlineLine;
use crate::count::saturating_u32;

/// One line per block, list items marked `•`, `1.` or `☐`/`☑`, table cells joined by ` · `.
pub(crate) fn plain(markdown: &str) -> String {
    outline(markdown)
        .iter()
        .map(OutlineLine::plain)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The same lines, each saying what it is: what a card draws a Note from.
pub(crate) fn outline(markdown: &str) -> Vec<OutlineLine> {
    let mut out = Out::default();
    let mut lists: Vec<Option<u64>> = Vec::new();

    let options =
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Start(Tag::List(first)) => lists.push(first),
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Heading { .. }) => out.open(Block::Heading),
            Event::Start(Tag::CodeBlock(_)) => out.open(Block::Code),
            Event::Start(Tag::Item) => {
                let depth = saturating_u32(lists.len().saturating_sub(1));
                let marker = match lists.last_mut() {
                    Some(Some(number)) => {
                        let marker = format!("{number}.");
                        *number += 1;
                        marker
                    }
                    _ => "•".to_string(),
                };
                out.open(Block::Item { depth, marker });
            }
            Event::TaskListMarker(done) => {
                if let Block::Item { depth, .. } = out.block {
                    out.block = Block::Task { depth, done };
                }
            }
            Event::Text(text) | Event::Code(text) => out.line.push_str(&text),
            Event::SoftBreak => out.line.push(' '),
            Event::End(TagEnd::TableCell) => out.line.push_str(" · "),
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                if out.line.ends_with(" · ") {
                    out.line.truncate(out.line.len() - " · ".len());
                }
                out.flush();
            }
            Event::HardBreak
            | Event::End(
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::CodeBlock,
            ) => out.flush(),
            _ => {}
        }
    }
    out.flush();

    out.lines
}

/// What the line being written will be, decided when its block opens.
#[derive(Default)]
enum Block {
    #[default]
    Text,
    Heading,
    Code,
    Item {
        depth: u32,
        marker: String,
    },
    Task {
        depth: u32,
        done: bool,
    },
}

#[derive(Default)]
struct Out {
    lines: Vec<OutlineLine>,
    line: String,
    block: Block,
}

impl Out {
    fn open(&mut self, block: Block) {
        self.flush();
        self.block = block;
    }

    /// A line with nothing readable is dropped; what follows it is text until a block says.
    fn flush(&mut self) {
        let text = self.line.trim_end();
        if !text.trim().is_empty() {
            let text = text.to_string();
            self.lines.push(match std::mem::take(&mut self.block) {
                Block::Text => OutlineLine::Text { text },
                Block::Heading => OutlineLine::Heading { text },
                Block::Code => OutlineLine::Code { text },
                Block::Item { depth, marker } => OutlineLine::Item {
                    text,
                    depth,
                    marker,
                },
                Block::Task { depth, done } => OutlineLine::Task { text, depth, done },
            });
        }
        self.line.clear();
        self.block = Block::Text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_syntax_goes_and_the_words_stay() {
        assert_eq!(
            plain("# Release\n\nShip **today**, not *tomorrow* — ~~maybe~~ `v2`."),
            "Release\nShip today, not tomorrow — maybe v2."
        );
    }

    #[test]
    fn lists_are_marked_as_a_card_can_show_them() {
        assert_eq!(
            plain("- one\n- two\n  - nested\n\n1. first\n2. second"),
            "• one\n• two\n  • nested\n1. first\n2. second"
        );
    }

    #[test]
    fn a_task_says_whether_it_is_done() {
        assert_eq!(
            plain("- [ ] write the notes\n- [x] book the room"),
            "☐ write the notes\n☑ book the room"
        );
    }

    #[test]
    fn a_link_keeps_its_text_and_a_quote_its_words() {
        assert_eq!(
            plain("See [the runbook](https://example.com).\n\n> Measure first."),
            "See the runbook.\nMeasure first."
        );
    }

    #[test]
    fn a_table_reads_row_by_row() {
        assert_eq!(
            plain("| host | port |\n| --- | --- |\n| db | 5432 |"),
            "host · port\ndb · 5432"
        );
    }

    /// What the rich editor escapes, and what a field is, come back as typed.
    #[test]
    fn escapes_and_fields_read_as_typed() {
        assert_eq!(
            plain("2 \\* 3 and \\[x\\] for {{db_host}}"),
            "2 * 3 and [x] for {{db_host}}"
        );
    }

    /// How the rich editor stores a tab starting a line, which Markdown would read as code.
    #[test]
    fn a_tab_written_as_an_entity_reads_as_a_tab() {
        assert_eq!(plain("&#9;indented"), "\tindented");
    }

    #[test]
    fn plain_text_stays_plain() {
        assert_eq!(plain("just a line\nand another"), "just a line and another");
        assert_eq!(plain(""), "");
    }
}
