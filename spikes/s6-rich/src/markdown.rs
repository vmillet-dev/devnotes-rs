//! Markdown ↔ model: read by pulldown-cmark (the options of `notes::markdown`), written the way
//! `@tiptap/markdown` + `escapeMarkdownText` write it today.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::model::{Block, Inline, Item, Marks};

const OPTIONS: Options = Options::ENABLE_TABLES.union(Options::ENABLE_TASKLISTS).union(Options::ENABLE_STRIKETHROUGH);

pub fn parse(markdown: &str) -> Vec<Block> {
    let events: Vec<Event> = Parser::new_ext(markdown, OPTIONS).collect();
    let mut at = 0;
    blocks(&events, &mut at, None)
}

fn blocks(events: &[Event], at: &mut usize, until: Option<TagEnd>) -> Vec<Block> {
    let mut out = Vec::new();
    while *at < events.len() {
        let event = &events[*at];
        *at += 1;
        match event {
            Event::End(end) if Some(*end) == until => return out,
            Event::Start(Tag::Paragraph) => out.push(Block::Paragraph(inlines(events, at, TagEnd::Paragraph))),
            Event::Start(Tag::Heading { level, .. }) => out.push(Block::Heading(*level as u8, inlines(events, at, TagEnd::Heading(*level)))),
            Event::Start(Tag::BlockQuote(kind)) => out.push(Block::Quote(blocks(events, at, Some(TagEnd::BlockQuote(*kind))))),
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => info.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let mut text = String::new();
                while let Some(Event::Text(t)) = events.get(*at) {
                    text.push_str(t);
                    *at += 1;
                }
                *at += 1; // End(CodeBlock)
                out.push(Block::Code { language, text: text.strip_suffix('\n').unwrap_or(&text).into() });
            }
            Event::Start(Tag::List(start)) => {
                let ordered = *start;
                let mut items = Vec::new();
                while let Some(Event::Start(Tag::Item)) = events.get(*at) {
                    *at += 1;
                    let task = if let Some(Event::TaskListMarker(done)) = events.get(*at) {
                        *at += 1;
                        Some(*done)
                    } else {
                        None
                    };
                    // A tight item holds bare inline events: wrap them in a paragraph.
                    let mut item_blocks = Vec::new();
                    loop {
                        match events.get(*at) {
                            Some(Event::End(TagEnd::Item)) => {
                                *at += 1;
                                break;
                            }
                            Some(Event::Start(Tag::Paragraph | Tag::List(_) | Tag::CodeBlock(_) | Tag::BlockQuote(_) | Tag::Heading { .. } | Tag::Table(_))) => {
                                let mut nested = blocks(events, at, Some(TagEnd::Item));
                                item_blocks.append(&mut nested);
                                break;
                            }
                            Some(_) => item_blocks.push(Block::Paragraph(inlines_until_block(events, at))),
                            None => break,
                        }
                    }
                    items.push(Item { task, blocks: item_blocks });
                }
                *at += 1; // End(List)
                out.push(Block::List { ordered, items });
            }
            Event::Start(Tag::Table(_)) => {
                let mut header = Vec::new();
                let mut rows = Vec::new();
                let mut row = Vec::new();
                while *at < events.len() {
                    let e = &events[*at];
                    *at += 1;
                    match e {
                        Event::Start(Tag::TableCell) => row.push(inlines(events, at, TagEnd::TableCell)),
                        Event::End(TagEnd::TableHead) => header = std::mem::take(&mut row),
                        Event::End(TagEnd::TableRow) => rows.push(std::mem::take(&mut row)),
                        Event::End(TagEnd::Table) => break,
                        _ => {}
                    }
                }
                out.push(Block::Table { header, rows });
            }
            Event::Rule => out.push(Block::Rule),
            _ => {}
        }
    }
    out
}

fn inlines_until_block(events: &[Event], at: &mut usize) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut marks = Marks::default();
    let mut link = None;
    while let Some(event) = events.get(*at) {
        if matches!(event, Event::End(TagEnd::Item) | Event::Start(Tag::List(_) | Tag::Paragraph | Tag::CodeBlock(_))) {
            break;
        }
        *at += 1;
        inline_event(event, events, at, &mut out, &mut marks, &mut link);
    }
    out
}

fn inlines(events: &[Event], at: &mut usize, end: TagEnd) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut marks = Marks::default();
    let mut link = None;
    while let Some(event) = events.get(*at) {
        *at += 1;
        if *event == Event::End(end) {
            break;
        }
        inline_event(event, events, at, &mut out, &mut marks, &mut link);
    }
    out
}

fn inline_event(event: &Event, events: &[Event], at: &mut usize, out: &mut Vec<Inline>, marks: &mut Marks, link: &mut Option<String>) {
    let push = |out: &mut Vec<Inline>, text: &str, marks: Marks, link: &Option<String>| {
        if let Some(Inline::Text { text: last, marks: m, link: l }) = out.last_mut() {
            if *m == marks && l == link {
                last.push_str(text);
                return;
            }
        }
        out.push(Inline::Text { text: text.into(), marks, link: link.clone() });
    };
    match event {
        Event::Text(t) => push(out, t, *marks, link),
        Event::Code(t) => push(out, t, Marks { code: true, ..*marks }, link),
        Event::Html(t) | Event::InlineHtml(t) => push(out, t, *marks, link),
        Event::SoftBreak => push(out, "\n", *marks, link),
        Event::HardBreak => out.push(Inline::HardBreak),
        Event::Start(Tag::Strong) => marks.bold = true,
        Event::End(TagEnd::Strong) => marks.bold = false,
        Event::Start(Tag::Emphasis) => marks.italic = true,
        Event::End(TagEnd::Emphasis) => marks.italic = false,
        Event::Start(Tag::Strikethrough) => marks.strike = true,
        Event::End(TagEnd::Strikethrough) => marks.strike = false,
        Event::Start(Tag::Link { dest_url, .. }) => *link = Some(dest_url.to_string()),
        Event::End(TagEnd::Link) => *link = None,
        Event::Start(Tag::Image { dest_url, .. }) => {
            let mut alt = String::new();
            while let Some(Event::Text(t)) = events.get(*at) {
                alt.push_str(t);
                *at += 1;
            }
            *at += 1; // End(Image)
            out.push(Inline::Image { alt, src: dest_url.to_string() });
        }
        _ => {}
    }
}

// --- writing -------------------------------------------------------------------------------

pub fn write(blocks: &[Block]) -> String {
    blocks.iter().map(|b| block(b, "")).collect::<Vec<_>>().join("\n\n")
}

fn block(b: &Block, indent: &str) -> String {
    match b {
        Block::Paragraph(runs) => inline_runs(runs, true),
        Block::Heading(level, runs) => format!("{} {}", "#".repeat(*level as usize), inline_runs(runs, false)),
        Block::Quote(inner) => write(inner).lines().map(|l| if l.is_empty() { ">".to_string() } else { format!("> {l}") }).collect::<Vec<_>>().join("\n"),
        Block::Code { language, text } => format!("```{language}\n{text}\n```"),
        Block::Rule => "---".into(),
        Block::List { ordered, items } => items
            .iter()
            .enumerate()
            .map(|(n, item)| {
                let marker = match (ordered, item.task) {
                    (_, Some(done)) => format!("- [{}] ", if done { 'x' } else { ' ' }),
                    (Some(start), None) => format!("{}. ", start + n as u64),
                    (None, None) => "- ".into(),
                };
                let child_indent = format!("{indent}{}", " ".repeat(if ordered.is_some() && item.task.is_none() { marker.len() } else { 2 }));
                let body = item.blocks.iter().map(|b| block(b, &child_indent)).collect::<Vec<_>>().join("\n");
                let body = body.replace('\n', &format!("\n{}", " ".repeat(child_indent.len() - indent.len())));
                format!("{marker}{body}")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Table { header, rows } => {
            let row = |cells: &Vec<Vec<Inline>>| format!("| {} |", cells.iter().map(|c| inline_runs(c, false).replace('|', "\\|")).collect::<Vec<_>>().join(" | "));
            let mut out = vec![row(header), format!("| {} |", vec!["---"; header.len()].join(" | "))];
            out.extend(rows.iter().map(row));
            out.join("\n")
        }
    }
}

fn inline_runs(runs: &[Inline], starts_line: bool) -> String {
    let mut out = String::new();
    for (n, run) in runs.iter().enumerate() {
        match run {
            Inline::Text { text, marks, link } => {
                let mut piece = if marks.code { format!("`{text}`") } else { escape_markdown_text(text, starts_line && n == 0) };
                if marks.italic {
                    piece = format!("*{piece}*");
                }
                if marks.bold {
                    piece = format!("**{piece}**");
                }
                if marks.strike {
                    piece = format!("~~{piece}~~");
                }
                if let Some(url) = link {
                    piece = format!("[{piece}]({url})");
                }
                out.push_str(&piece);
            }
            Inline::Image { alt, src } => out.push_str(&format!("![{alt}]({src})")),
            Inline::HardBreak => out.push_str("\\\n"),
        }
    }
    out
}

/// `escapeMarkdownText` of `markdown-text.ts`, ported: escaped only where a character would read
/// as syntax; a `{{field}}` is left as typed; a tab starting a line is `&#9;`.
pub fn escape_markdown_text(text: &str, starts_line: bool) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        let Some(close) = rest[open..].find("}}").filter(|c| !rest[open + 2..open + c].contains(['{', '}', '\n'])) else { break };
        out.push_str(&escape_run(&rest[..open]));
        out.push_str(&rest[open..open + close + 2]);
        rest = &rest[open + close + 2..];
    }
    out.push_str(&escape_run(rest));
    if starts_line {
        let tabs = out.chars().take_while(|c| *c == '\t').count();
        return format!("{}{}", "&#9;".repeat(tabs), &out[tabs..]);
    }
    out
}

fn escape_run(run: &str) -> String {
    let chars: Vec<char> = run.chars().collect();
    let mut out = String::new();
    for (at, &c) in chars.iter().enumerate() {
        let before = at.checked_sub(1).map(|b| chars[b]);
        let after = chars.get(at + 1).copied();
        let space_before = before.is_none_or(char::is_whitespace);
        let space_after = after.is_none_or(char::is_whitespace);
        let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
        let escape = match c {
            '\\' => after.is_some_and(|a| a.is_ascii_punctuation()),
            '`' | '[' => true,
            '*' => !(space_before && space_after),
            '_' => !(space_before && space_after) && !(word(before) && word(after)),
            '~' => !space_after && after != Some('/'),
            '<' => after.is_some_and(|a| a.is_ascii_alphabetic() || "/!?".contains(a)),
            '&' => {
                let rest: String = chars[at..].iter().take(12).collect();
                is_entity(&rest)
            }
            _ => false,
        };
        if escape {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn is_entity(rest: &str) -> bool {
    let Some(end) = rest.find(';') else { return false };
    let body = &rest[1..end];
    !body.is_empty()
        && (body.strip_prefix('#').is_some_and(|d| d.chars().all(|c| c.is_ascii_digit()) && !d.is_empty())
            || body.strip_prefix("#x").or_else(|| body.strip_prefix("#X")).is_some_and(|h| h.chars().all(|c| c.is_ascii_hexdigit()) && !h.is_empty())
            || (body.starts_with(|c: char| c.is_ascii_alphabetic()) && body.chars().all(|c| c.is_ascii_alphanumeric())))
}
