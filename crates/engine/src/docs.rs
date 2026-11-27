//! The documentation, compiled in: the Markdown of `docs/content`, which devnotes.fr shows too.

pub mod model;
pub mod search;

use std::sync::{LazyLock, OnceLock};

use model::{Index, Page};

// `SOURCES`: each file of `docs/content/<language>`, by language and page, written by `build.rs`.
include!(concat!(env!("OUT_DIR"), "/docs.rs"));

const INDEX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/content/index.json"
));

static READ_INDEX: LazyLock<Index> =
    LazyLock::new(|| serde_json::from_str(INDEX).expect("compiled in, and held by a test"));

/// Each page of `SOURCES`, read the first time it is asked for: one at a time, it costs a frame
/// nothing.
static PAGES: [OnceLock<Option<Page>>; SOURCES.len()] = [const { OnceLock::new() }; SOURCES.len()];

/// The pages, in reading order.
pub fn docs_index() -> &'static Index {
    &READ_INDEX
}

/// A page in a language, by its name in the index.
pub fn docs_page(language: &str, id: &str) -> Option<&'static Page> {
    let at = SOURCES
        .iter()
        .position(|source| source.0 == language && source.1 == id)?;
    PAGES[at]
        .get_or_init(|| model::read(id, SOURCES[at].2).ok())
        .as_ref()
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use super::*;
    use model::{Block, Inline, Target};

    const LANGUAGES: [&str; 2] = ["fr", "en"];

    fn ids() -> Vec<&'static str> {
        docs_index()
            .pages()
            .map(|entry| entry.id.as_str())
            .collect()
    }

    fn runs(block: &Block) -> Vec<&Inline> {
        match block {
            Block::Paragraph(text) | Block::Note { text, .. } => text.iter().collect(),
            Block::List { items, .. } => items.iter().flatten().collect(),
            Block::Table { rows, .. } => rows.iter().flatten().flatten().collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn every_file_is_read() {
        for (language, id, markdown) in SOURCES {
            if let Err(refusal) = model::read(id, markdown) {
                panic!("docs/content/{language}/{id}.md:{refusal}");
            }
        }
    }

    #[test]
    fn the_files_are_the_pages_of_the_index_in_every_language() {
        let mut named = ids();
        named.sort_unstable();
        for language in LANGUAGES {
            let written: Vec<_> = SOURCES
                .iter()
                .filter(|source| source.0 == language)
                .map(|source| source.1)
                .collect();
            assert_eq!(written, named, "docs/content/{language} and index.json");
        }
        assert_eq!(SOURCES.len(), named.len() * LANGUAGES.len());
    }

    /// Both languages say the same thing in the same order: a section added to one is missed in
    /// the other, and the site finds the section to land on by its place.
    #[test]
    fn a_page_has_the_same_shape_in_every_language() {
        for id in ids() {
            let [french, english] = LANGUAGES.map(|language| docs_page(language, id).expect(id));
            let shape = |page: &Page| -> Vec<_> {
                page.body
                    .iter()
                    .filter(|block| !matches!(block, Block::Paragraph(_)))
                    .map(|block| match block {
                        Block::Heading { level, .. } => (discriminant(block), usize::from(*level)),
                        Block::List { items, .. } => (discriminant(block), items.len()),
                        Block::Shortcuts { rows, .. } => (discriminant(block), rows.len()),
                        Block::Table { rows, .. } => (discriminant(block), rows.len()),
                        _ => (discriminant(block), 0),
                    })
                    .collect()
            };
            let (ours, theirs) = (shape(french), shape(english));
            let at = ours.iter().zip(&theirs).position(|(a, b)| a != b);
            assert!(
                at.is_none() && ours.len() == theirs.len(),
                "{id}: fr has {} sections, lists, notes and tables, en has {}; the first to differ is number {}",
                ours.len(),
                theirs.len(),
                at.unwrap_or_else(|| ours.len().min(theirs.len())) + 1,
            );
        }
    }

    #[test]
    fn two_sections_of_a_page_never_share_an_address() {
        for (language, id, _) in SOURCES {
            let page = docs_page(language, id).expect(id);
            let mut sections = Vec::new();
            for block in &page.body {
                if let Block::Heading { id: section, .. } = block {
                    assert!(
                        !sections.contains(&section),
                        "{language}/{id}.md: {section}"
                    );
                    sections.push(section);
                }
            }
        }
    }

    #[test]
    fn every_link_leads_to_a_page_and_a_section_that_exist() {
        for (language, id, _) in SOURCES {
            for run in docs_page(language, id)
                .expect(id)
                .body
                .iter()
                .flat_map(runs)
            {
                let Inline::Link {
                    text,
                    to: Target::Page { id: to, section },
                } = run
                else {
                    continue;
                };
                let name = format!("docs/content/{language}/{id}.md: « {text} »");
                let page = docs_page(language, to).unwrap_or_else(|| panic!("{name} → {to}"));
                let exists = section.as_ref().is_none_or(|section| {
                    page.body
                        .iter()
                        .any(|block| matches!(block, Block::Heading { id, .. } if id == section))
                });
                assert!(exists, "{name} → {to} has no section {section:?}");
            }
        }
    }

    #[test]
    fn the_overview_lists_every_other_page() {
        for language in LANGUAGES {
            let overview = docs_page(language, "overview").expect("the overview");
            let listed = overview.body.iter().find_map(|block| match block {
                Block::Pages(pages) => Some(pages.clone()),
                _ => None,
            });
            assert_eq!(
                listed.unwrap_or_default(),
                ids()[1..],
                "{language}/overview.md"
            );
        }
    }

    #[test]
    fn the_index_names_every_page_in_every_language() {
        for entry in docs_index().pages() {
            for language in LANGUAGES {
                assert!(!entry.name.of(language).is_empty(), "{}", entry.id);
            }
        }
        assert!(!docs_index().checked_on.is_empty());
    }
}
