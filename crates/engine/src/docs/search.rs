//! The documentation searched: every page cut at its sections, so that an answer leads to the
//! place that holds the words.

use std::sync::OnceLock;

use super::model::{Block, Page, Shortcut, text_of};
use crate::notes::view::fold;

const AT_MOST: usize = 12;
/// What an excerpt shows around the word found, in words.
const BEFORE: usize = 6;
const AFTER: usize = 14;

/// A section that holds every word searched.
#[derive(Debug, Clone)]
pub struct Hit {
    pub page: &'static Page,
    /// The section's address; `None` is the top of the page.
    pub section: Option<&'static str>,
    pub heading: &'static str,
    pub excerpt: String,
}

#[derive(Debug)]
struct Section {
    page: &'static Page,
    id: Option<&'static str>,
    heading: &'static str,
    text: String,
    folded_heading: String,
    folded_text: String,
}

static SECTIONS: [OnceLock<Vec<Section>>; 2] = [const { OnceLock::new() }; 2];

/// The sections that hold every word of `query`, in any order, accents aside; those that hold
/// them in their heading first.
pub fn search_docs(language: &str, query: &str) -> Vec<Hit> {
    let folded = fold(query);
    let terms: Vec<_> = folded.split_whitespace().collect();
    let Some(first) = terms.first() else {
        return Vec::new();
    };
    let mut found: Vec<_> = sections(language)
        .iter()
        .filter(|section| {
            terms.iter().all(|term| {
                section.folded_heading.contains(term) || section.folded_text.contains(term)
            })
        })
        .collect();
    found.sort_by_key(|section| {
        let named = terms
            .iter()
            .filter(|term| section.folded_heading.contains(**term));
        std::cmp::Reverse(named.count())
    });
    found
        .into_iter()
        .take(AT_MOST)
        .map(|section| Hit {
            page: section.page,
            section: section.id,
            heading: section.heading,
            excerpt: excerpt(&section.text, first),
        })
        .collect()
}

fn sections(language: &str) -> &'static [Section] {
    let cell = &SECTIONS[usize::from(language == "fr")];
    cell.get_or_init(|| {
        super::docs_index()
            .pages()
            .filter_map(|entry| super::docs_page(language, &entry.id))
            .flat_map(cut)
            .collect()
    })
}

/// A page at each `##`; what comes before the first is its top.
fn cut(page: &'static Page) -> Vec<Section> {
    let mut sections = vec![(None, page.title.as_str(), vec![page.summary.clone()])];
    for block in &page.body {
        match block {
            Block::Heading { level: 2, id, text } => {
                sections.push((Some(id.as_str()), text.as_str(), Vec::new()));
            }
            other => {
                if let Some((_, _, words)) = sections.last_mut() {
                    words.push(said(other));
                }
            }
        }
    }
    sections
        .into_iter()
        .map(|(id, heading, words)| {
            let text = words.join(" ");
            Section {
                page,
                id,
                heading,
                folded_heading: fold(heading),
                folded_text: fold(&text),
                text,
            }
        })
        .collect()
}

fn said(block: &Block) -> String {
    match block {
        Block::Paragraph(text) | Block::Note { text, .. } => text_of(text),
        Block::Heading { text, .. } => text.clone(),
        Block::List { items, .. } => join(items.iter().map(|item| text_of(item))),
        Block::Code { title, text, .. } => format!("{title} {text}"),
        Block::Shortcuts { title, rows } => {
            let rows = rows
                .iter()
                .map(|Shortcut { keys, does }| format!("{} {does}", keys.join(" ")));
            format!("{title} {}", join(rows))
        }
        Block::Table { title, rows, .. } => {
            let cells = rows.iter().flatten().map(|cell| text_of(cell));
            format!("{title} {}", join(cells))
        }
        Block::Pages(_) => String::new(),
    }
}

fn join(texts: impl Iterator<Item = String>) -> String {
    texts.collect::<Vec<_>>().join(" ")
}

/// The words around the first one that holds `term`; the beginning when only the heading does.
fn excerpt(text: &str, term: &str) -> String {
    let words: Vec<_> = text.split_whitespace().collect();
    let at = words.iter().position(|word| fold(word).contains(term));
    let from = at.unwrap_or(0).saturating_sub(BEFORE);
    let to = words.len().min(at.unwrap_or(0) + AFTER);
    let before = if from > 0 { "…" } else { "" };
    let after = if to < words.len() { "…" } else { "" };
    format!("{before}{}{after}", words[from..to].join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_is_found_without_its_accents() {
        let hits = search_docs("fr", "derivation cle");
        assert_eq!(hits[0].page.id, "safety");
        assert_eq!(hits[0].section, Some("le-chiffrement"));
        assert!(hits[0].excerpt.contains("Dérivation"), "{:?}", hits[0]);
    }

    #[test]
    fn a_heading_comes_before_a_mention() {
        let hits = search_docs("en", "palette");
        assert_eq!(
            (hits[0].page.id.as_str(), hits[0].heading),
            ("paste", "The palette")
        );
    }

    #[test]
    fn every_word_is_asked_for_in_any_order() {
        let ids = |query| {
            search_docs("fr", query)
                .into_iter()
                .map(|hit| (hit.page.id.as_str(), hit.section))
                .collect::<Vec<_>>()
        };
        assert_eq!(ids("corbeille trente"), ids("trente corbeille"));
        assert!(search_docs("fr", "corbeille zzz").is_empty());
        assert!(search_docs("fr", "  ").is_empty());
    }

    #[test]
    fn a_language_answers_in_its_own_words() {
        assert!(!search_docs("en", "passphrase").is_empty());
        assert!(search_docs("fr", "passphrase").is_empty());
    }

    #[test]
    fn an_excerpt_says_where_it_was_cut() {
        let text = "un deux trois quatre cinq six sept huit neuf dix";
        assert_eq!(
            excerpt(text, "huit"),
            "…deux trois quatre cinq six sept huit neuf dix"
        );
        assert_eq!(excerpt(text, "absent"), text);
    }
}
