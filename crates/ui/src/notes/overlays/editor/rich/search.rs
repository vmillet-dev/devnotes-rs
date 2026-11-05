//! The find bar over a Note (the reference's `rich-text-search`): the matches are in the text the
//! reader sees — each leaf's words with its marks unseen, so `**Ship** it` holds « Ship it » — and
//! none crosses from one block into the next.

use std::ops::Range;

use richtext::edit::ordered;
use richtext::{Marks, Position, Selection, inlines, tree};
use text::{COUNT_CAP, Count, Direction, Query};

use super::super::search_bar::{SearchBar, Searchable, SeekFrom};
use super::{RichEditor, Step};

/// A match: the leaf it is in and its `char` range there.
pub(super) type Found = (Vec<usize>, Range<usize>);

/// Every match, in reading order.
pub(super) fn matches(view: &RichEditor, query: &Query) -> Vec<Found> {
    let mut found = Vec::new();
    for path in tree::leaves(&view.document) {
        let text = match tree::leaf(&view.document, &path) {
            Some(tree::Leaf::Inlines(content)) => inlines::plain(content),
            Some(tree::Leaf::Code(text)) => text.clone(),
            None => continue,
        };
        for range in text::find(&text, query, COUNT_CAP) {
            found.push((path.clone(), range));
        }
    }
    found
}

/// What the bar draws over, when it is open: every match, for the view to light.
pub(super) fn lit(view: &RichEditor, bar: &SearchBar) -> Vec<Found> {
    if bar.open {
        matches(view, &bar.query)
    } else {
        Vec::new()
    }
}

fn start(found: &Found) -> Position {
    Position {
        path: found.0.clone(),
        offset: found.1.start,
    }
}

fn end(found: &Found) -> Position {
    Position {
        path: found.0.clone(),
        offset: found.1.end,
    }
}

fn select(found: &Found) -> Selection {
    Selection {
        anchor: start(found),
        head: end(found),
    }
}

impl RichEditor {
    /// The match the selection is on.
    fn selected_match(&self, found: &[Found]) -> Option<usize> {
        let (from, to) = ordered(&self.selection);
        found
            .iter()
            .position(|each| start(each) == from && end(each) == to)
    }
}

impl Searchable for RichEditor {
    fn selected_text(&self) -> String {
        richtext::edit::selected_text(&self.document, &self.selection)
    }

    fn count(&self, query: &Query) -> Count {
        let found = matches(self, query);
        Count {
            total: found.len(),
            current: self.selected_match(&found).map(|at| at + 1),
            capped: found.len() >= COUNT_CAP,
        }
    }

    fn seek(&mut self, query: &Query, from: SeekFrom, direction: Direction) {
        let found = matches(self, query);
        if found.is_empty() {
            return;
        }
        let (first, last) = ordered(&self.selection);
        let next = match (from, direction) {
            (SeekFrom::Start, _) => found.iter().position(|each| start(each) >= first),
            (SeekFrom::Past, Direction::Forward) => {
                found.iter().position(|each| start(each) >= last)
            }
            (SeekFrom::Past, Direction::Backward) => {
                found.iter().rposition(|each| end(each) <= first)
            }
        };
        let at = next.unwrap_or(match direction {
            Direction::Forward => 0,
            Direction::Backward => found.len() - 1,
        });
        self.selection = select(&found[at]);
        self.reveal = true;
    }

    fn replace_one(&mut self, query: &Query) -> Option<usize> {
        let found = matches(self, query);
        let Some(at) = self.selected_match(&found) else {
            self.seek(query, SeekFrom::Start, Direction::Forward);
            return None;
        };
        let replacement = query.replacement(&self.selected_text());
        let marks = self.marks_at_match(&found[at]);
        self.change(Step::Other, |document, selection| {
            richtext::edit::insert_text(document, selection, &replacement, &marks)
        });
        self.seek(query, SeekFrom::Start, Direction::Forward);
        Some(0)
    }

    fn replace_all(&mut self, query: &Query) -> Option<usize> {
        let found = matches(self, query);
        if found.is_empty() {
            return None;
        }
        self.record(Step::Other);
        // From the last: the offsets before it stay true.
        for each in found.iter().rev() {
            let replacement = query.replacement(&richtext::edit::selected_text(
                &self.document,
                &select(each),
            ));
            let marks = self.marks_at_match(each);
            richtext::edit::insert_text(&mut self.document, &select(each), &replacement, &marks);
        }
        self.selection = Selection::caret(tree::clamp(&self.document, &start(&found[0])));
        Some(0)
    }

    fn focus(&mut self) {
        self.refocus = true;
    }
}

impl RichEditor {
    /// The replacement wears what the match's first `char` wore.
    fn marks_at_match(&self, found: &Found) -> Marks {
        match tree::leaf(&self.document, &found.0) {
            Some(tree::Leaf::Inlines(content)) => inlines::marks_at(content, found.1.start + 1),
            _ => Marks::default(),
        }
    }
}
