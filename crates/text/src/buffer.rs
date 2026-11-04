//! The text being edited and its selection. Every edit goes through [`Buffer::replace`], which
//! records it for undo and says from which line the text changed: what the highlighter and the
//! view lay out again.

use std::ops::Range;

use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;

use crate::history::{Change, History, Kind, Step};
use crate::indent::{self, TextEdit};
use crate::search::{self, Count, Query};

/// The anchor stays where the selection began; the head moves with the caret.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    pub fn caret(at: usize) -> Self {
        Self {
            anchor: at,
            head: at,
        }
    }

    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    pub fn is_empty(self) -> bool {
        self.anchor == self.head
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Backward,
    Forward,
}

/// How much a deletion or a move takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Grapheme,
    Word,
    Line,
}

/// What an edit touched: everything from this line on may have moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edited {
    pub first_line: usize,
}

#[derive(Debug, Default)]
pub struct Buffer {
    rope: Rope,
    selection: Selection,
    history: History,
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            selection: Selection::default(),
            history: History::default(),
        }
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    pub fn len(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Lines as an editor counts them: a text ending with a newline has an empty last one.
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// A line without its line ending.
    pub fn line(&self, index: usize) -> String {
        let line = self.rope.line(index).to_string();
        line.trim_end_matches(['\n', '\r']).to_owned()
    }

    pub fn line_of(&self, at: usize) -> usize {
        self.rope.char_to_line(at.min(self.len()))
    }

    pub fn line_start(&self, line: usize) -> usize {
        self.rope.line_to_char(line.min(self.line_count()))
    }

    /// Where the line's text ends, before its line ending.
    pub fn line_end(&self, line: usize) -> usize {
        self.line_start(line) + self.line(line).chars().count()
    }

    pub fn selection(&self) -> Selection {
        self.selection
    }

    pub fn selected_text(&self) -> String {
        self.rope.slice(self.selection.range()).to_string()
    }

    /// The caret moved by itself: typing after this is a step of its own.
    pub fn select(&mut self, selection: Selection) {
        let len = self.len();
        self.selection = Selection {
            anchor: selection.anchor.min(len),
            head: selection.head.min(len),
        };
        self.history.break_group();
    }

    pub fn select_all(&mut self) {
        self.select(Selection {
            anchor: 0,
            head: self.len(),
        });
    }

    /// The caret to `at`, the anchor kept when `extend`.
    pub fn move_to(&mut self, at: usize, extend: bool) {
        let anchor = if extend { self.selection.anchor } else { at };
        self.select(Selection { anchor, head: at });
    }

    /// What is typed or pasted, in place of the selection.
    pub fn insert(&mut self, text: &str) -> Edited {
        let range = self.selection.range();
        let typing = range.is_empty() && text.chars().count() == 1;
        let kind = if typing { Kind::Typing } else { Kind::Other };
        let caret = range.start + text.chars().count();
        self.replace(range, text, Selection::caret(caret), kind)
    }

    /// The selection, or a grapheme, a word or a line from the caret; `None` when there is
    /// nothing that way.
    pub fn delete(&mut self, direction: Direction, unit: Unit) -> Option<Edited> {
        let range = if self.selection.is_empty() {
            let at = self.selection.head;
            if unit == Unit::Line {
                self.whole_lines(at..at)
            } else {
                let to = self.step(at, direction, unit);
                at.min(to)..at.max(to)
            }
        } else {
            self.selection.range()
        };
        if range.is_empty() {
            return None;
        }
        let start = range.start;
        Some(self.replace(range, "", Selection::caret(start), Kind::Erasing))
    }

    /// One replacement as one step of undo, the selection where the edit puts it.
    pub fn apply(&mut self, edit: &TextEdit) -> Edited {
        let selection = Selection {
            anchor: edit.selection_start,
            head: edit.selection_end,
        };
        self.replace(edit.from..edit.to, &edit.insert, selection, Kind::Other)
    }

    /// Tab: to the next level at a caret, a level more on every line of a selection.
    pub fn indent(&mut self, unit: &str) -> Edited {
        let range = self.selection.range();
        let edit = indent::indent(&self.text(), range.start, range.end, unit);
        self.apply(&edit)
    }

    /// Shift+Tab; `None` when no line had a level to lose.
    pub fn outdent(&mut self, unit: &str) -> Option<Edited> {
        let range = self.selection.range();
        let edit = indent::outdent(&self.text(), range.start, range.end, unit)?;
        Some(self.apply(&edit))
    }

    /// Alt+↑ / Alt+↓: the selected lines swap with the one above or below.
    pub fn move_lines(&mut self, direction: Direction) -> Option<Edited> {
        let range = self.selection.range();
        let first = self.line_of(range.start);
        let mut last = self.line_of(range.end);
        if range.end > range.start && range.end == self.line_start(last) && last > first {
            last -= 1;
        }
        let lines: Vec<String> = (first..=last).map(|line| self.line(line)).collect();
        let block = lines.join("\n");
        let shift = |at: usize, by: isize| at.saturating_add_signed(by);
        match direction {
            Direction::Backward if first > 0 => {
                let above = self.line(first - 1);
                let from = self.line_start(first - 1);
                let to = self.line_end(last);
                let moved = isize::try_from(above.chars().count() + 1).unwrap_or(0);
                let selection = Selection {
                    anchor: shift(self.selection.anchor, -moved),
                    head: shift(self.selection.head, -moved),
                };
                Some(self.replace(
                    from..to,
                    &format!("{block}\n{above}"),
                    selection,
                    Kind::Other,
                ))
            }
            Direction::Forward if last + 1 < self.line_count() => {
                let below = self.line(last + 1);
                let from = self.line_start(first);
                let to = self.line_end(last + 1);
                let moved = isize::try_from(below.chars().count() + 1).unwrap_or(0);
                let selection = Selection {
                    anchor: shift(self.selection.anchor, moved),
                    head: shift(self.selection.head, moved),
                };
                Some(self.replace(
                    from..to,
                    &format!("{below}\n{block}"),
                    selection,
                    Kind::Other,
                ))
            }
            _ => None,
        }
    }

    /// Ctrl+/: `token ` before every line of the selection, or off them all when each has it.
    pub fn toggle_comment(&mut self, token: &str) -> Edited {
        let range = self.selection.range();
        let span = self.whole_lines(range.clone());
        let first = self.line_of(span.start);
        let text = self.rope.slice(span.clone()).to_string();
        let body = text.strip_suffix('\n').unwrap_or(&text);
        let lines: Vec<&str> = body.split('\n').collect();
        let marked = format!("{token} ");
        let commented = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .all(|line| line.trim_start().starts_with(token));
        let changed: Vec<String> = lines
            .iter()
            .map(|line| {
                if line.trim().is_empty() {
                    return (*line).to_owned();
                }
                let indent = line.len() - line.trim_start().len();
                let (head, rest) = line.split_at(indent);
                if commented {
                    let uncommented = rest
                        .strip_prefix(&marked)
                        .or_else(|| rest.strip_prefix(token))
                        .unwrap_or(rest);
                    format!("{head}{uncommented}")
                } else {
                    format!("{head}{marked}{rest}")
                }
            })
            .collect();
        let mut insert = changed.join("\n");
        if body.len() < text.len() {
            insert.push('\n');
        }
        let grown = insert.chars().count();
        let end = span.start + grown - usize::from(body.len() < text.len());
        let selection = if range.is_empty() {
            let column = self.selection.head - self.line_start(first);
            let line_now = changed.first().map_or(0, |line| line.chars().count());
            Selection::caret(span.start + column.min(line_now))
        } else {
            Selection {
                anchor: span.start,
                head: end,
            }
        };
        self.replace(span, &insert, selection, Kind::Other)
    }

    pub fn undo(&mut self) -> Option<Edited> {
        let step = self.history.undo()?;
        let mut first = usize::MAX;
        for change in step.changes.iter().rev() {
            let end = change.at + change.inserted.chars().count();
            first = first.min(self.splice(change.at..end, &change.removed));
        }
        self.selection = step.before;
        Some(Edited { first_line: first })
    }

    pub fn redo(&mut self) -> Option<Edited> {
        let step = self.history.redo()?;
        let mut first = usize::MAX;
        for change in &step.changes {
            let end = change.at + change.removed.chars().count();
            first = first.min(self.splice(change.at..end, &change.inserted));
        }
        self.selection = step.after;
        Some(Edited { first_line: first })
    }

    /// Where the caret goes one grapheme or one word `direction` from `at`; a line end counts as
    /// one grapheme, `\r\n` included.
    pub fn step(&self, at: usize, direction: Direction, unit: Unit) -> usize {
        match unit {
            Unit::Grapheme => self.grapheme(at, direction),
            Unit::Word => self.word(at, direction),
            Unit::Line => match direction {
                Direction::Backward => self.line_start(self.line_of(at)),
                Direction::Forward => self.line_end(self.line_of(at)),
            },
        }
    }

    /// Every match of `query`, up to `cap`.
    pub fn find(&self, query: &Query, cap: usize) -> Vec<Range<usize>> {
        search::find(&self.text(), query, cap)
    }

    /// How many `query` finds, and which one is selected.
    pub fn count(&self, query: &Query) -> Count {
        let found = self.find(query, search::COUNT_CAP);
        let selected = self.selection.range();
        Count {
            total: found.len(),
            current: found
                .iter()
                .position(|hit| *hit == selected)
                .map(|at| at + 1),
            capped: found.len() >= search::COUNT_CAP,
        }
    }

    /// Selects the next match from `from` that way, wrapping round; `false` when there is none.
    pub fn seek(&mut self, query: &Query, from: usize, direction: Direction) -> bool {
        let found = self.find(query, usize::MAX);
        let hit = match direction {
            Direction::Forward => found
                .iter()
                .find(|hit| hit.start >= from)
                .or_else(|| found.first()),
            Direction::Backward => found
                .iter()
                .rev()
                .find(|hit| hit.end <= from)
                .or_else(|| found.last()),
        };
        let Some(hit) = hit.cloned() else {
            return false;
        };
        self.select(Selection {
            anchor: hit.start,
            head: hit.end,
        });
        true
    }

    /// The selected match replaced, then the next one selected; the next one selected only, when
    /// the selection is on no match.
    pub fn replace_one(&mut self, query: &Query) -> Option<Edited> {
        let selected = self.selection.range();
        let on_match = self.find(query, usize::MAX).contains(&selected);
        if !on_match {
            self.seek(query, selected.start, Direction::Forward);
            return None;
        }
        let replacement = query.replacement(&self.selected_text());
        let end = selected.start + replacement.chars().count();
        let edited = self.replace(
            selected.clone(),
            &replacement,
            Selection::caret(end),
            Kind::Other,
        );
        self.seek(query, end, Direction::Forward);
        Some(edited)
    }

    /// Every match, as one step of undo; how many were replaced.
    pub fn replace_all(&mut self, query: &Query) -> (usize, Option<Edited>) {
        let found = self.find(query, usize::MAX);
        let Some(first) = found.first() else {
            return (0, None);
        };
        let first_line = self.line_of(first.start);
        let before = self.selection;
        let mut changes = Vec::new();
        let mut shift = 0_isize;
        for hit in &found {
            let removed = self.rope.slice(hit.clone()).to_string();
            let inserted = query.replacement(&removed);
            let at = hit.start.saturating_add_signed(shift);
            shift += isize::try_from(inserted.chars().count()).unwrap_or(0)
                - isize::try_from(removed.chars().count()).unwrap_or(0);
            changes.push(Change {
                at,
                removed,
                inserted,
            });
        }
        for change in &changes {
            let end = change.at + change.removed.chars().count();
            self.splice(change.at..end, &change.inserted);
        }
        let caret = Selection::caret(before.head.min(self.len()));
        self.selection = caret;
        self.history.record(Step {
            changes,
            before,
            after: caret,
            kind: Kind::Other,
        });
        (found.len(), Some(Edited { first_line }))
    }

    /// The one way the text changes: recorded, the selection set after it.
    fn replace(&mut self, range: Range<usize>, text: &str, after: Selection, kind: Kind) -> Edited {
        let removed = self.rope.slice(range.clone()).to_string();
        let before = self.selection;
        let first_line = self.splice(range.clone(), text);
        self.selection = after;
        self.history.record(Step {
            changes: vec![Change {
                at: range.start,
                removed,
                inserted: text.to_owned(),
            }],
            before,
            after,
            kind,
        });
        Edited { first_line }
    }

    /// The rope changed, nothing recorded; the first line it touched.
    fn splice(&mut self, range: Range<usize>, text: &str) -> usize {
        let first_line = self.rope.char_to_line(range.start);
        self.rope.remove(range.clone());
        self.rope.insert(range.start, text);
        first_line
    }

    /// From the start of the first line `range` touches to past the line ending of its last.
    fn whole_lines(&self, range: Range<usize>) -> Range<usize> {
        let first = self.line_of(range.start);
        let mut last = self.line_of(range.end);
        if range.end > range.start && range.end == self.line_start(last) && last > first {
            last -= 1;
        }
        let end = if last + 1 < self.line_count() {
            self.line_start(last + 1)
        } else {
            self.len()
        };
        self.line_start(first)..end
    }

    fn grapheme(&self, at: usize, direction: Direction) -> usize {
        let line = self.line_of(at);
        let start = self.line_start(line);
        let text = self.rope.line(line).to_string();
        let offset = at - start;
        let bounds: Vec<usize> = text
            .grapheme_indices(true)
            .map(|(byte, _)| text[..byte].chars().count())
            .chain(std::iter::once(text.chars().count()))
            .collect();
        match direction {
            Direction::Forward => bounds
                .iter()
                .find(|bound| **bound > offset)
                .map_or(at, |bound| start + bound),
            Direction::Backward if offset == 0 => {
                if line == 0 {
                    return 0;
                }
                // Back over the line ending above, `\r\n` as one.
                let above = self.rope.line(line - 1).to_string();
                let ending = if above.ends_with("\r\n") { 2 } else { 1 };
                at - ending
            }
            Direction::Backward => bounds
                .iter()
                .rev()
                .find(|bound| **bound < offset)
                .map_or(at, |bound| start + bound),
        }
    }

    /// Over the spaces, then over one run of word characters or of punctuation; a line end
    /// alone is a step.
    fn word(&self, at: usize, direction: Direction) -> usize {
        // 0 a space, 1 punctuation, 2 a word character.
        let class = |c: char| {
            if c.is_alphanumeric() || c == '_' {
                2
            } else {
                u8::from(!c.is_whitespace())
            }
        };
        let char_at = |i: usize| self.rope.char(i);
        let len = self.len();
        let mut i = at;
        match direction {
            Direction::Forward => {
                if i < len && char_at(i) == '\n' {
                    return i + 1;
                }
                while i < len && char_at(i) != '\n' && class(char_at(i)) == 0 {
                    i += 1;
                }
                if i < len && char_at(i) != '\n' {
                    let run = class(char_at(i));
                    while i < len && char_at(i) != '\n' && class(char_at(i)) == run {
                        i += 1;
                    }
                }
                i
            }
            Direction::Backward => {
                if i > 0 && char_at(i - 1) == '\n' {
                    return self.grapheme(i, Direction::Backward);
                }
                while i > 0 && char_at(i - 1) != '\n' && class(char_at(i - 1)) == 0 {
                    i -= 1;
                }
                if i > 0 && char_at(i - 1) != '\n' {
                    let run = class(char_at(i - 1));
                    while i > 0 && char_at(i - 1) != '\n' && class(char_at(i - 1)) == run {
                        i -= 1;
                    }
                }
                i
            }
        }
    }
}

#[cfg(test)]
mod tests;
