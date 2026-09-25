//! What a Text note reads as once its Markdown is taken away: a card's preview, a search
//! excerpt. The rich editor writes the body; nobody reads `**` or `- [ ]` on a card.

use std::fmt::Write;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// One line per block, list items marked `•`, `1.` or `☐`/`☑`, table cells joined by ` · `.
pub(crate) fn plain(markdown: &str) -> String {
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
            Event::Start(Tag::Item) => {
                out.flush();
                let depth = lists.len().saturating_sub(1);
                out.line.push_str(&"  ".repeat(depth));
                match lists.last_mut() {
                    Some(Some(number)) => {
                        let _ = write!(out.line, "{number}. ");
                        *number += 1;
                    }
                    _ => out.line.push_str("• "),
                }
            }
            Event::TaskListMarker(done) => {
                if out.line.ends_with("• ") {
                    out.line.truncate(out.line.len() - "• ".len());
                }
                out.line.push_str(if done { "☑ " } else { "☐ " });
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

    out.text
}

#[derive(Default)]
struct Out {
    text: String,
    line: String,
}

impl Out {
    fn flush(&mut self) {
        let line = self.line.trim_end();
        if !line.trim().is_empty() {
            if !self.text.is_empty() {
                self.text.push('\n');
            }
            self.text.push_str(line);
        }
        self.line.clear();
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
