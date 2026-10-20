//! What a Note reads as once its Markdown is taken away: a card's preview, a search
//! excerpt. The rich editor writes the body; nobody reads `**` or `- [ ]` on a card.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::model::{MapNode, OutlineLine, Presentation};
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

    for event in Parser::new_ext(markdown, OPTIONS) {
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

/// A slide starts at each `#` or `##` heading and after each `---` rule, outside any list or
/// quote: a `##` in a code block is code. Each slide is a slice of the body, so the rich editor
/// draws it as it was written; a slide with nothing in it is dropped.
pub(crate) fn present(markdown: &str) -> Presentation {
    let mut slides = Vec::new();
    let mut headings: Vec<(HeadingLevel, String, u32)> = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut heading: Option<(HeadingLevel, String)> = None;

    let cut = |slides: &mut Vec<String>, from: usize, to: usize| {
        let slide = markdown[from..to].trim();
        if !slide.is_empty() {
            slides.push(slide.to_string());
        }
    };

    for (event, range) in Parser::new_ext(markdown, OPTIONS).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if let Tag::Heading { level, .. } = tag {
                    if depth == 0 && level <= HeadingLevel::H2 {
                        cut(&mut slides, start, range.start);
                        start = range.start;
                    }
                    heading = Some((level, String::new()));
                }
                depth += 1;
            }
            Event::End(tag) => {
                depth = depth.saturating_sub(1);
                if matches!(tag, TagEnd::Heading(_))
                    && let Some((level, text)) = heading.take()
                    && !text.trim().is_empty()
                {
                    headings.push((level, text.trim().to_string(), saturating_u32(slides.len())));
                }
            }
            Event::Rule if depth == 0 => {
                cut(&mut slides, start, range.start);
                start = range.end;
            }
            Event::Text(words) | Event::Code(words) => {
                if let Some((_, text)) = heading.as_mut() {
                    text.push_str(&words);
                }
            }
            _ => {}
        }
    }
    cut(&mut slides, start, markdown.len());

    Presentation {
        slides,
        map: tree(&headings),
    }
}

const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_STRIKETHROUGH);

/// Each heading takes the ones after it that are deeper, up to the next of its level or above.
fn tree(headings: &[(HeadingLevel, String, u32)]) -> Vec<MapNode> {
    let mut nodes = Vec::new();
    let mut at = 0;
    while let Some((level, text, slide)) = headings.get(at) {
        let end = headings[at + 1..]
            .iter()
            .position(|(other, ..)| other <= level)
            .map_or(headings.len(), |offset| at + 1 + offset);
        nodes.push(MapNode {
            text: text.clone(),
            slide: *slide,
            children: tree(&headings[at + 1..end]),
        });
        at = end;
    }
    nodes
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

    /// A card shows a code block's lines as written, and a search finds and quotes them.
    #[test]
    fn a_code_block_reads_as_its_lines() {
        assert_eq!(
            plain(
                "Check:

```sql
SELECT 1;
  -- *not* emphasis
```

Done."
            ),
            "Check:
SELECT 1;
  -- *not* emphasis
Done."
        );
    }

    #[test]
    fn plain_text_stays_plain() {
        assert_eq!(plain("just a line\nand another"), "just a line and another");
        assert_eq!(plain(""), "");
    }

    fn node(text: &str, slide: u32, children: Vec<MapNode>) -> MapNode {
        MapNode {
            text: text.to_string(),
            slide,
            children,
        }
    }

    #[test]
    fn a_note_is_cut_at_its_sections_and_its_rules_as_the_body_wrote_them() {
        let presentation = present(
            "Intro line.\n\n## Why\n\nBecause *so*.\n\n---\n\nA pause.\n\n# Next\n\n### Detail\n\nMore.",
        );

        assert_eq!(
            presentation.slides,
            [
                "Intro line.",
                "## Why\n\nBecause *so*.",
                "A pause.",
                "# Next\n\n### Detail\n\nMore."
            ]
        );
    }

    /// A `##` in a fence is code, and one in a quote or a list belongs to it.
    #[test]
    fn a_heading_or_a_rule_inside_a_block_cuts_nothing() {
        let presentation = present(
            "## One\n\n```md\n## not a slide\n---\n```\n\n> ## quoted\n\n- item\n\n  ---\n\n  more",
        );

        assert_eq!(presentation.slides.len(), 1);
    }

    #[test]
    fn nothing_to_cut_is_one_slide_and_nothing_at_all_none() {
        assert_eq!(present("Just words.").slides, ["Just words."]);
        assert!(present("  \n\n---\n\n---\n").slides.is_empty());
        assert!(present("").map.is_empty());
    }

    #[test]
    fn the_map_holds_each_heading_under_the_one_above_it_with_its_slide() {
        let presentation = present(
            "# Plan\n\n## Build\n\n### Rust\n\n### Front\n\n## Ship `v2` **now**\n\n#### Deep\n\n##\n",
        );

        assert_eq!(
            presentation.map,
            [node(
                "Plan",
                0,
                vec![
                    node(
                        "Build",
                        1,
                        vec![node("Rust", 1, vec![]), node("Front", 1, vec![])]
                    ),
                    node("Ship v2 now", 2, vec![node("Deep", 2, vec![])]),
                ],
            )]
        );
    }

    /// Two trees side by side when the Note opens on a `##` and a `#` comes later.
    #[test]
    fn a_heading_above_the_first_one_starts_a_tree_of_its_own() {
        let presentation = present("## Before\n\n# After\n\n## Under");

        assert_eq!(
            presentation.map,
            [
                node("Before", 0, vec![]),
                node("After", 1, vec![node("Under", 2, vec![])])
            ]
        );
    }
}
