//! Undo and redo, by steps: what one keystroke, one paste, one indent or one « replace all »
//! changed. Typing joins the step before it while it goes on where it left off, so one undo takes
//! back a word typed, not a letter.

use crate::buffer::Selection;

/// The most steps kept; the oldest go first.
const DEPTH: usize = 500;

/// One range replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    pub(crate) at: usize,
    pub(crate) removed: String,
    pub(crate) inserted: String,
}

/// What joins: letters typed one after the other, or erased one before the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Typing,
    Erasing,
    Other,
}

#[derive(Debug, Clone)]
pub(crate) struct Step {
    pub(crate) changes: Vec<Change>,
    pub(crate) before: Selection,
    pub(crate) after: Selection,
    pub(crate) kind: Kind,
}

#[derive(Debug, Default)]
pub(crate) struct History {
    done: Vec<Step>,
    undone: Vec<Step>,
    /// Off once the caret moved by itself: typing elsewhere starts a step of its own.
    joinable: bool,
}

impl History {
    pub(crate) fn record(&mut self, step: Step) {
        self.undone.clear();
        if self.joinable
            && let Some(last) = self.done.last_mut()
            && joins(last, &step)
        {
            last.changes.extend(step.changes);
            last.after = step.after;
        } else {
            self.done.push(step);
            if self.done.len() > DEPTH {
                self.done.remove(0);
            }
        }
        self.joinable = true;
    }

    pub(crate) fn break_group(&mut self) {
        self.joinable = false;
    }

    pub(crate) fn undo(&mut self) -> Option<Step> {
        let step = self.done.pop()?;
        self.undone.push(step.clone());
        self.joinable = false;
        Some(step)
    }

    pub(crate) fn redo(&mut self) -> Option<Step> {
        let step = self.undone.pop()?;
        self.done.push(step.clone());
        self.joinable = false;
        Some(step)
    }
}

fn joins(last: &Step, next: &Step) -> bool {
    let (Some(previous), [change]) = (last.changes.last(), next.changes.as_slice()) else {
        return false;
    };
    match (last.kind, next.kind) {
        (Kind::Typing, Kind::Typing) => {
            change.at == previous.at + previous.inserted.chars().count()
                && !change.inserted.contains('\n')
                // A word ends a step: the space after it starts the next.
                && (!change.inserted.starts_with(char::is_whitespace)
                    || previous.inserted.ends_with(char::is_whitespace))
        }
        (Kind::Erasing, Kind::Erasing) => {
            change.at + change.removed.chars().count() == previous.at || change.at == previous.at
        }
        _ => false,
    }
}
