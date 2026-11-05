//! Undo and redo by transaction: the document and the selection as they were before each one.
//! A Note is a few kilobytes, so a whole copy is the simplest step that cannot go wrong.

use crate::model::{Document, Selection};

/// Past this, the oldest steps are let go.
const STEPS: usize = 200;

#[derive(Debug, Default)]
pub struct History {
    done: Vec<(Document, Selection)>,
    undone: Vec<(Document, Selection)>,
}

impl History {
    /// Before a transaction changes `document`: what undo comes back to.
    pub fn record(&mut self, document: &Document, selection: &Selection) {
        if self.done.len() == STEPS {
            self.done.remove(0);
        }
        self.done.push((document.clone(), selection.clone()));
        self.undone.clear();
    }

    /// The state before the last transaction; `now` is kept for redo.
    pub fn undo(&mut self, now: (&Document, &Selection)) -> Option<(Document, Selection)> {
        let before = self.done.pop()?;
        self.undone.push((now.0.clone(), now.1.clone()));
        Some(before)
    }

    pub fn redo(&mut self, now: (&Document, &Selection)) -> Option<(Document, Selection)> {
        let after = self.undone.pop()?;
        self.done.push((now.0.clone(), now.1.clone()));
        Some(after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Block, Inline, Position};

    fn note(text: &str) -> Document {
        Document {
            blocks: vec![Block::Paragraph(vec![Inline::text(text)])],
        }
    }

    #[test]
    fn a_transaction_undone_comes_back_with_its_selection_and_is_redone() {
        let mut history = History::default();
        let at = Selection::caret(Position {
            path: vec![0],
            offset: 2,
        });
        history.record(&note("ab"), &at);
        let now = note("abc");
        let (before, selection) = history.undo((&now, &Selection::default())).unwrap();
        assert_eq!(before, note("ab"));
        assert_eq!(selection, at);
        let (after, _) = history.redo((&before, &selection)).unwrap();
        assert_eq!(after, now);
        assert!(history.redo((&after, &selection)).is_none());
    }
}
