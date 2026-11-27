//! What a page of the documentation is: the conventions of `docs/content/README.md`, read from its
//! Markdown. ⚠️ The site reads the same files with its own reader (`website/scripts/docs.mjs`): a
//! convention is added to both, and what neither knows is refused rather than guessed.

use std::fmt;
use std::ops::Range;

use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};
use serde::Deserialize;

use crate::notes::view::fold;

/// What heads a table of shortcuts, per language.
const SHORTCUTS: [&str; 2] = ["Touches", "Keys"];

/// The pages in reading order, by group: `docs/content/index.json`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    pub groups: Vec<Group>,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    pub name: Names,
    pub pages: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub id: String,
    /// A folder's colour, by its name in the palette.
    pub colour: String,
    /// Its name in the list of pages.
    pub name: Names,
}

/// One text per language.
#[derive(Debug, Deserialize)]
pub struct Names {
    fr: String,
    en: String,
}

impl Names {
    pub fn of(&self, language: &str) -> &str {
        if language == "fr" { &self.fr } else { &self.en }
    }
}

impl Index {
    pub fn pages(&self) -> impl Iterator<Item = &Entry> {
        self.groups.iter().flat_map(|group| &group.pages)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub body: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        id: String,
        text: String,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    Code {
        title: String,
        language: String,
        text: String,
    },
    Note {
        tone: Tone,
        text: Vec<Inline>,
    },
    Shortcuts {
        title: String,
        rows: Vec<Shortcut>,
    },
    Table {
        title: String,
        head: Vec<String>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    /// A list of pages, as the overview's.
    Pages(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    /// Key by key.
    pub keys: Vec<String>,
    pub does: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Tip,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Code(String),
    /// A combination, key by key.
    Keys(Vec<String>),
    Strong(String),
    /// A label of the application, as it is on screen.
    Label(String),
    Link {
        text: String,
        to: Target,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Page { id: String, section: Option<String> },
    Address(String),
}

/// Why a page is not read, and where.
#[derive(Debug, PartialEq, Eq)]
pub struct Refusal {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.line, self.message)
    }
}

type Reading<T> = Result<T, Refusal>;

/// A section's address: its text folded, its punctuation dropped, its spaces dashes. The form
/// GitHub gives a heading leads there too.
pub fn slug(heading: &str) -> String {
    let kept: String = fold(heading)
        .chars()
        .filter(|each| each.is_alphanumeric() || each.is_whitespace() || matches!(each, '_' | '-'))
        .collect();
    kept.split(|each: char| each.is_whitespace() || each == '-')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn read(id: &str, markdown: &str) -> Reading<Page> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_GFM;
    let mut reader = Reader {
        id,
        markdown,
        events: Parser::new_ext(markdown, options)
            .into_offset_iter()
            .collect(),
        at: 0,
    };
    reader.page()
}

struct Reader<'a> {
    id: &'a str,
    markdown: &'a str,
    events: Vec<(Event<'a>, Range<usize>)>,
    at: usize,
}

impl<'a> Reader<'a> {
    fn page(&mut self) -> Reading<Page> {
        let title = match self.next() {
            Some(Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            })) => self.plain()?,
            _ => return self.refuse("a page begins with # and its title"),
        };
        let summary = match self.next() {
            Some(Event::Start(Tag::Paragraph)) => self.plain()?,
            _ => return self.refuse("the title is followed by one paragraph, the summary"),
        };

        let mut body = Vec::new();
        while self.at < self.events.len() {
            body.push(self.titled_block()?);
        }
        Ok(Page {
            id: self.id.to_owned(),
            title,
            summary,
            body,
        })
    }

    /// A block, and the title of a table or a code block: the bold line just above it.
    fn titled_block(&mut self) -> Reading<Block> {
        let first = self.block(None)?;
        let titled = matches!(
            self.peek(),
            Some(Event::Start(Tag::Table(_) | Tag::CodeBlock(_)))
        );
        match first {
            Block::Paragraph(text) if titled => match text.as_slice() {
                [Inline::Strong(title)] => self.block(Some(title)),
                _ => Ok(Block::Paragraph(text)),
            },
            other => Ok(other),
        }
    }

    fn block(&mut self, title: Option<&str>) -> Reading<Block> {
        let Some(event) = self.next() else {
            return self.refuse("a block is missing");
        };
        match event {
            Event::Start(Tag::Paragraph) => Ok(Block::Paragraph(self.inline()?)),
            Event::Start(Tag::Heading { level, .. }) => self.heading(level),
            Event::Start(Tag::List(start)) => self.list(start.is_some()),
            Event::Start(Tag::BlockQuote(kind)) => self.note(kind),
            Event::Start(Tag::CodeBlock(kind)) => match title {
                Some(title) => self.code(title, &kind),
                None => self.refuse("a code block has its title in bold on the line above"),
            },
            Event::Start(Tag::Table(_)) => match title {
                Some(title) => self.table(title),
                None => self.refuse("a table has its title in bold on the line above"),
            },
            other => self.refuse(format!("{other:?} is not read")),
        }
    }

    fn heading(&mut self, level: HeadingLevel) -> Reading<Block> {
        let level = match level {
            HeadingLevel::H2 => 2,
            HeadingLevel::H3 => 3,
            _ => return self.refuse("a section is ## or ###"),
        };
        let text = self.plain()?;
        Ok(Block::Heading {
            level,
            id: slug(&text),
            text,
        })
    }

    fn list(&mut self, ordered: bool) -> Reading<Block> {
        let mut items = Vec::new();
        while let Some(Event::Start(Tag::Item)) = self.peek() {
            self.next();
            items.push(self.one_paragraph()?);
        }
        self.next();
        match items.iter().map(|item| only_page(item)).collect() {
            Some(pages) => Ok(Block::Pages(pages)),
            None => Ok(Block::List { ordered, items }),
        }
    }

    fn note(&mut self, kind: Option<BlockQuoteKind>) -> Reading<Block> {
        let tone = match kind {
            Some(BlockQuoteKind::Tip) => Tone::Tip,
            Some(BlockQuoteKind::Warning) => Tone::Warning,
            _ => return self.refuse("a quote begins with [!TIP] or [!WARNING], alone on its line"),
        };
        let text = self.one_paragraph()?;
        Ok(Block::Note { tone, text })
    }

    fn code(&mut self, title: &str, kind: &CodeBlockKind<'_>) -> Reading<Block> {
        let mut text = String::new();
        while let Some(Event::Text(line)) = self.peek() {
            text.push_str(line);
            self.next();
        }
        self.next();
        let language = match kind {
            CodeBlockKind::Fenced(language) => language.to_string(),
            CodeBlockKind::Indented => return self.refuse("a code block is between ```"),
        };
        Ok(Block::Code {
            title: title.to_owned(),
            language,
            text: text.trim_end_matches('\n').to_owned(),
        })
    }

    fn table(&mut self, title: &str) -> Reading<Block> {
        let mut rows = Vec::new();
        while let Some(Event::Start(Tag::TableHead | Tag::TableRow)) = self.peek() {
            self.next();
            let mut row = Vec::new();
            while let Some(Event::Start(Tag::TableCell)) = self.peek() {
                self.next();
                row.push(self.inline()?);
            }
            self.next();
            rows.push(row);
        }
        self.next();

        let mut rows = rows.into_iter();
        let head = rows
            .next()
            .unwrap_or_default()
            .iter()
            .map(|cell| text_of(cell))
            .collect::<Vec<_>>();
        let title = title.to_owned();
        if !SHORTCUTS.contains(&head[0].as_str()) {
            return Ok(Block::Table {
                title,
                head,
                rows: rows.collect(),
            });
        }
        let mut shortcuts = Vec::new();
        for row in rows {
            match row.as_slice() {
                [keys, does] => match keys.as_slice() {
                    [Inline::Keys(keys)] => shortcuts.push(Shortcut {
                        keys: keys.clone(),
                        does: text_of(does),
                    }),
                    _ => return self.refuse("a table of shortcuts holds one <kbd> per row"),
                },
                _ => return self.refuse("a table of shortcuts has two columns"),
            }
        }
        Ok(Block::Shortcuts {
            title,
            rows: shortcuts,
        })
    }

    /// What an item or a note holds: one paragraph, marked as such or not.
    fn one_paragraph(&mut self) -> Reading<Vec<Inline>> {
        let Some(Event::Start(Tag::Paragraph)) = self.peek() else {
            return self.inline();
        };
        self.next();
        let text = self.inline()?;
        match self.next() {
            Some(Event::End(_)) => Ok(text),
            _ => self.refuse("an item, or a note, holds one paragraph"),
        }
    }

    /// The text of what is open, with its marks, up to its end.
    fn inline(&mut self) -> Reading<Vec<Inline>> {
        let mut runs: Vec<Inline> = Vec::new();
        loop {
            let run = match self.next() {
                Some(Event::End(_)) | None => return Ok(runs),
                Some(Event::Text(text)) => Inline::Text(text.to_string()),
                Some(Event::SoftBreak) => Inline::Text(" ".to_owned()),
                Some(Event::Code(code)) => Inline::Code(code.to_string()),
                Some(Event::Start(Tag::Strong)) => Inline::Strong(self.plain()?),
                Some(Event::Start(Tag::Emphasis)) => Inline::Label(self.plain()?),
                Some(Event::Start(Tag::Link { dest_url, .. })) => Inline::Link {
                    text: self.plain()?,
                    to: self.target(&dest_url)?,
                },
                Some(Event::InlineHtml(tag)) if &*tag == "<kbd>" => Inline::Keys(self.keys()?),
                Some(other) => return self.refuse(format!("{other:?} is not read")),
            };
            match (runs.last_mut(), run) {
                (Some(Inline::Text(before)), Inline::Text(text)) => before.push_str(&text),
                (_, run) => runs.push(run),
            }
        }
    }

    fn plain(&mut self) -> Reading<String> {
        let runs = self.inline()?;
        match runs.as_slice() {
            [] => Ok(String::new()),
            [Inline::Text(text)] => Ok(text.clone()),
            _ => self.refuse("plain text is expected, without a mark"),
        }
    }

    fn keys(&mut self) -> Reading<Vec<String>> {
        let mut keys = String::new();
        loop {
            match self.next() {
                Some(Event::Text(text)) => keys.push_str(&text),
                Some(Event::InlineHtml(tag)) if &*tag == "</kbd>" => {
                    return Ok(keys.split(' ').map(str::to_owned).collect());
                }
                _ => return self.refuse("a <kbd> holds keys, and is closed"),
            }
        }
    }

    fn target(&self, href: &str) -> Reading<Target> {
        if href.starts_with("https://") || href.starts_with("http://") {
            return Ok(Target::Address(href.to_owned()));
        }
        let (file, section) = match href.split_once('#') {
            Some((file, section)) => (file, Some(slug(section))),
            None => (href, None),
        };
        let id = match file.strip_suffix(".md") {
            Some(id) if !id.contains(['/', '.']) => id,
            None if file.is_empty() && section.is_some() => self.id,
            _ => return self.refuse(format!("{href} leads nowhere")),
        };
        Ok(Target::Page {
            id: id.to_owned(),
            section,
        })
    }

    fn peek(&self) -> Option<&Event<'a>> {
        self.events.get(self.at).map(|(event, _)| event)
    }

    fn next(&mut self) -> Option<Event<'a>> {
        let (event, _) = self.events.get(self.at)?;
        self.at += 1;
        Some(event.clone())
    }

    /// At the event just read.
    fn refuse<T>(&self, message: impl Into<String>) -> Reading<T> {
        let at = self
            .events
            .get(self.at.saturating_sub(1))
            .map_or(0, |(_, range)| range.start);
        Err(Refusal {
            line: self.markdown[..at].matches('\n').count() + 1,
            message: message.into(),
        })
    }
}

/// The page an item leads to, when it is a link to it and nothing more.
fn only_page(item: &[Inline]) -> Option<String> {
    match item {
        [
            Inline::Link {
                to: Target::Page { id, section: None },
                ..
            },
        ] => Some(id.clone()),
        _ => None,
    }
}

/// What a run of text says, its marks aside.
pub fn text_of(runs: &[Inline]) -> String {
    runs.iter()
        .map(|run| match run {
            Inline::Text(text)
            | Inline::Code(text)
            | Inline::Strong(text)
            | Inline::Label(text)
            | Inline::Link { text, .. } => text.clone(),
            Inline::Keys(keys) => keys.join(" "),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "# Titre\n\nRésumé.\n\n";

    fn body(markdown: &str) -> Vec<Block> {
        match read("page", &format!("{HEAD}{markdown}")) {
            Ok(page) => page.body,
            Err(refusal) => panic!("{refusal}"),
        }
    }

    fn refused(markdown: &str) -> String {
        read("page", &format!("{HEAD}{markdown}"))
            .expect_err("the page is refused")
            .to_string()
    }

    fn text(text: &str) -> Inline {
        Inline::Text(text.to_owned())
    }

    #[test]
    fn a_page_opens_on_its_title_and_its_summary() {
        let page = read("page", HEAD).expect("a page");
        assert_eq!(
            (page.title.as_str(), page.summary.as_str()),
            ("Titre", "Résumé.")
        );
        assert!(page.body.is_empty());

        let refusal = read("page", "Sans titre.\n").expect_err("no title");
        assert!(refusal.message.contains("begins with #"), "{refusal}");
        let refusal = read("page", "# Titre\n\n- liste\n").expect_err("no summary");
        assert!(refusal.message.contains("one paragraph"), "{refusal}");
    }

    #[test]
    fn a_sentence_holds_labels_shortcuts_code_bold_and_links() {
        let blocks = body(
            "Faites *Fichier → Préférences…* (<kbd>Ctrl Alt P</kbd>), `{{host}}`, **tout** ou [là](start.md#Clé-maître).",
        );
        assert_eq!(
            blocks,
            [Block::Paragraph(vec![
                text("Faites "),
                Inline::Label("Fichier → Préférences…".to_owned()),
                text(" ("),
                Inline::Keys(vec!["Ctrl".to_owned(), "Alt".to_owned(), "P".to_owned()]),
                text("), "),
                Inline::Code("{{host}}".to_owned()),
                text(", "),
                Inline::Strong("tout".to_owned()),
                text(" ou "),
                Inline::Link {
                    text: "là".to_owned(),
                    to: Target::Page {
                        id: "start".to_owned(),
                        section: Some("cle-maitre".to_owned()),
                    },
                },
                text("."),
            ])]
        );
    }

    #[test]
    fn a_link_leads_to_a_page_a_section_of_this_one_or_an_address() {
        let to = |markdown: &str| match body(markdown).remove(0) {
            Block::Paragraph(runs) => match runs.into_iter().next() {
                Some(Inline::Link { to, .. }) => to,
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        };
        assert_eq!(
            to("[a](#ici)"),
            Target::Page {
                id: "page".to_owned(),
                section: Some("ici".to_owned()),
            }
        );
        assert_eq!(
            to("[a](https://devnotes.fr/telecharger)"),
            Target::Address("https://devnotes.fr/telecharger".to_owned())
        );
        assert!(refused("[a](../autre.txt)").contains("leads nowhere"));
    }

    #[test]
    fn a_list_of_nothing_but_pages_is_the_list_of_them() {
        assert_eq!(
            body("- [Écrire](writing.md)\n- [Ranger](organising.md)\n"),
            [Block::Pages(vec![
                "writing".to_owned(),
                "organising".to_owned()
            ])]
        );
        assert_eq!(
            body("1. un\n2. *deux*\n"),
            [Block::List {
                ordered: true,
                items: vec![vec![text("un")], vec![Inline::Label("deux".to_owned())]],
            }]
        );
    }

    #[test]
    fn a_tip_and_a_warning_are_notes() {
        assert_eq!(
            body("> [!WARNING]\n> **Attention.** Voilà.\n"),
            [Block::Note {
                tone: Tone::Warning,
                text: vec![Inline::Strong("Attention.".to_owned()), text(" Voilà.")],
            }]
        );
        assert!(refused("> Une citation.\n").contains("[!TIP]"));
    }

    #[test]
    fn a_table_and_a_code_block_are_titled_by_the_bold_line_above() {
        assert_eq!(
            body("**Fichiers**\n\n| Système | Fichier |\n| --- | --- |\n| Linux | `a\\|b` |\n"),
            [Block::Table {
                title: "Fichiers".to_owned(),
                head: vec!["Système".to_owned(), "Fichier".to_owned()],
                rows: vec![vec![
                    vec![text("Linux")],
                    vec![Inline::Code("a|b".to_owned())]
                ]],
            }]
        );
        assert_eq!(
            body("**Installer**\n\n```sh\nsudo apt\n\nls\n```\n"),
            [Block::Code {
                title: "Installer".to_owned(),
                language: "sh".to_owned(),
                text: "sudo apt\n\nls".to_owned(),
            }]
        );
        assert!(refused("| A |\n| --- |\n| b |\n").contains("a table has its title"));
        assert!(refused("```sh\nls\n```\n").contains("a code block has its title"));
    }

    #[test]
    fn a_table_headed_by_the_keys_is_one_of_shortcuts() {
        for head in SHORTCUTS {
            let table = format!(
                "**Palette**\n\n| {head} | Action |\n| --- | --- |\n| <kbd>↑ ↓</kbd> | Choisir |\n"
            );
            assert_eq!(
                body(&table),
                [Block::Shortcuts {
                    title: "Palette".to_owned(),
                    rows: vec![Shortcut {
                        keys: vec!["↑".to_owned(), "↓".to_owned()],
                        does: "Choisir".to_owned(),
                    }],
                }]
            );
        }
        let table = "**Palette**\n\n| Touches | Action |\n| --- | --- |\n| Ctrl | Choisir |\n";
        assert!(refused(table).contains("one <kbd> per row"));
    }

    #[test]
    fn what_is_not_a_convention_is_refused_at_its_line() {
        assert!(refused("#### Trop bas\n").starts_with("5: a section is ## or ###"));
        for markdown in [
            "![image](a.png)\n",
            "<b>gras</b>\n",
            "---\n",
            "- un\n  - dedans\n",
            "> [!TIP]\n> un\n>\n> deux\n",
        ] {
            assert!(
                refused(markdown).starts_with('5') || refused(markdown).starts_with(['6', '8'])
            );
        }
    }

    #[test]
    fn a_section_is_addressed_by_its_text() {
        assert_eq!(slug("Où vont mes notes ?"), "ou-vont-mes-notes");
        assert_eq!(slug("où-vont-mes-notes-"), "ou-vont-mes-notes");
        assert_eq!(slug("L’API : `GET`, « v1 »"), "lapi-get-v1");
    }
}
