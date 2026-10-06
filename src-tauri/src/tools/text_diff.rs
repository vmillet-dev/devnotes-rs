//! Two texts compared line by line, the changed words or characters marked inside a modified
//! line, and the unified diff `diff -u` would write. What to ignore is ignored in the comparison
//! alone: every line is shown as it was typed.

use std::fmt::Write;

use serde::{Deserialize, Serialize};
use similar::{Algorithm, ChangeTag, DiffOp, TextDiff, capture_diff_slices, group_diff_ops};
use specta::Type;

use super::diff::MAX_ROWS;
use crate::count::saturating_u32;

/// The lines of context around each hunk of the unified diff, as `diff -u` keeps.
const CONTEXT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Granularity {
    Lines,
    Words,
    Characters,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct TextDiffRequest {
    pub a: String,
    pub b: String,
    pub granularity: Granularity,
    pub ignore_trailing_whitespace: bool,
    pub ignore_all_whitespace: bool,
    pub ignore_case: bool,
    /// CRLF against LF is the commonest difference nobody meant.
    pub ignore_line_endings: bool,
}

/// A piece of a line, marked when it is what changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextPiece {
    pub text: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextDiffLine {
    /// One-based, in that side's own text.
    pub number: u32,
    pub pieces: Vec<TextPiece>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TextRowKind {
    Same,
    Added,
    Removed,
    Modified,
}

/// One row of the side-by-side view: a side without a line is a gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextDiffRow {
    pub kind: TextRowKind,
    pub left: Option<TextDiffLine>,
    pub right: Option<TextDiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextDiffAnswer {
    /// In the unit asked: lines, words or characters.
    pub added: u32,
    pub removed: u32,
    pub rows: Vec<TextDiffRow>,
    pub rows_truncated: bool,
    /// From A to B, three lines of context, as `diff -u` writes it.
    pub unified: String,
    pub identical: bool,
    /// Both sides are JSON: they can be compared as values instead.
    pub both_json: bool,
}

/// The lines of a text, each with its line ending kept unless endings are ignored.
fn lines_of(text: &str, ignore_endings: bool) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = text.split_inclusive('\n').collect();
    for line in &mut lines {
        let bare = line.strip_suffix('\n').unwrap_or(line);
        *line = if ignore_endings {
            bare.strip_suffix('\r').unwrap_or(bare)
        } else {
            bare
        };
    }
    lines
}

/// What a line is compared as: the options take out what they ignore.
fn key(line: &str, request: &TextDiffRequest) -> String {
    let mut key: String = if request.ignore_all_whitespace {
        line.chars().filter(|c| !c.is_whitespace()).collect()
    } else if request.ignore_trailing_whitespace {
        line.trim_end().to_owned()
    } else {
        line.to_owned()
    };
    if request.ignore_case {
        key = key.to_lowercase();
    }
    key
}

/// A line as shown: a carriage return the comparison kept is not drawn.
fn shown(line: &str) -> String {
    line.strip_suffix('\r').unwrap_or(line).to_owned()
}

fn whole(number: usize, line: &str, changed: bool) -> TextDiffLine {
    TextDiffLine {
        number: saturating_u32(number + 1),
        pieces: vec![TextPiece {
            text: shown(line),
            changed,
        }],
    }
}

fn units(line: &str, granularity: Granularity) -> usize {
    match granularity {
        Granularity::Lines => 1,
        Granularity::Words => line.split_whitespace().count(),
        Granularity::Characters => line.chars().count(),
    }
}

fn push_piece(pieces: &mut Vec<TextPiece>, text: &str, changed: bool) {
    match pieces.last_mut() {
        Some(last) if last.changed == changed => last.text.push_str(text),
        _ => pieces.push(TextPiece {
            text: text.to_owned(),
            changed,
        }),
    }
}

/// The two sides of a modified line, the words or characters that differ marked, and how many
/// of them each side holds.
fn inline(
    old: &str,
    new: &str,
    granularity: Granularity,
) -> ((Vec<TextPiece>, usize), (Vec<TextPiece>, usize)) {
    let (old, new) = (shown(old), shown(new));
    let diff = match granularity {
        Granularity::Characters => TextDiff::from_chars(&old, &new),
        Granularity::Words | Granularity::Lines => TextDiff::from_words(&old, &new),
    };
    let (mut left, mut right) = ((Vec::new(), 0), (Vec::new(), 0));
    for change in diff.iter_all_changes() {
        let text = change.value();
        let counted = match granularity {
            Granularity::Characters => text.chars().count(),
            _ => usize::from(!text.trim().is_empty()),
        };
        match change.tag() {
            ChangeTag::Equal => {
                push_piece(&mut left.0, text, false);
                push_piece(&mut right.0, text, false);
            }
            ChangeTag::Delete => {
                push_piece(&mut left.0, text, true);
                left.1 += counted;
            }
            ChangeTag::Insert => {
                push_piece(&mut right.0, text, true);
                right.1 += counted;
            }
        }
    }
    (left, right)
}

fn unified(a: &[&str], b: &[&str], ops: &[DiffOp]) -> String {
    let mut out = String::new();
    for group in group_diff_ops(ops.to_vec(), CONTEXT) {
        let (Some(first), Some(last)) = (group.first(), group.last()) else {
            continue;
        };
        let old = first.old_range().start..last.old_range().end;
        let new = first.new_range().start..last.new_range().end;
        // `diff -u` numbers an empty range by the line before it.
        let start = |range: &std::ops::Range<usize>| {
            if range.is_empty() {
                range.start
            } else {
                range.start + 1
            }
        };
        let _ = writeln!(
            out,
            "@@ -{},{} +{},{} @@",
            start(&old),
            old.len(),
            start(&new),
            new.len()
        );
        for op in &group {
            for change in op.iter_changes(a, b) {
                let sign = match change.tag() {
                    ChangeTag::Equal => ' ',
                    ChangeTag::Delete => '-',
                    ChangeTag::Insert => '+',
                };
                let _ = writeln!(out, "{sign}{}", shown(change.value()));
            }
        }
    }
    if out.is_empty() {
        out
    } else {
        format!("--- a\n+++ b\n{out}")
    }
}

/// The side-by-side rows, and what they add and remove in the unit asked.
struct Aligned<'a> {
    a: &'a [&'a str],
    b: &'a [&'a str],
    granularity: Granularity,
    rows: Vec<TextDiffRow>,
    added: usize,
    removed: usize,
}

impl Aligned<'_> {
    fn same(&mut self, old: usize, new: usize) {
        self.rows.push(TextDiffRow {
            kind: TextRowKind::Same,
            left: Some(whole(old, self.a[old], false)),
            right: Some(whole(new, self.b[new], false)),
        });
    }

    fn removed(&mut self, old: usize) {
        self.removed += units(self.a[old], self.granularity);
        self.rows.push(TextDiffRow {
            kind: TextRowKind::Removed,
            left: Some(whole(old, self.a[old], true)),
            right: None,
        });
    }

    fn added(&mut self, new: usize) {
        self.added += units(self.b[new], self.granularity);
        self.rows.push(TextDiffRow {
            kind: TextRowKind::Added,
            left: None,
            right: Some(whole(new, self.b[new], true)),
        });
    }

    fn modified(&mut self, old: usize, new: usize) {
        let (left, right) = if self.granularity == Granularity::Lines {
            self.removed += 1;
            self.added += 1;
            (whole(old, self.a[old], true), whole(new, self.b[new], true))
        } else {
            let ((left, gone), (right, came)) = inline(self.a[old], self.b[new], self.granularity);
            self.removed += gone;
            self.added += came;
            (
                TextDiffLine {
                    number: saturating_u32(old + 1),
                    pieces: left,
                },
                TextDiffLine {
                    number: saturating_u32(new + 1),
                    pieces: right,
                },
            )
        };
        self.rows.push(TextDiffRow {
            kind: TextRowKind::Modified,
            left: Some(left),
            right: Some(right),
        });
    }

    fn take(&mut self, op: &DiffOp) {
        match *op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => (0..len).for_each(|at| self.same(old_index + at, new_index + at)),
            DiffOp::Delete {
                old_index, old_len, ..
            } => (old_index..old_index + old_len).for_each(|at| self.removed(at)),
            DiffOp::Insert {
                new_index, new_len, ..
            } => (new_index..new_index + new_len).for_each(|at| self.added(at)),
            // Paired line for line; what is left over on one side is added or removed.
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                for at in 0..old_len.max(new_len) {
                    match (at < old_len, at < new_len) {
                        (true, true) => self.modified(old_index + at, new_index + at),
                        (true, false) => self.removed(old_index + at),
                        (false, true) => self.added(new_index + at),
                        (false, false) => {}
                    }
                }
            }
        }
    }
}

pub fn diff(request: &TextDiffRequest) -> TextDiffAnswer {
    let a = lines_of(&request.a, request.ignore_line_endings);
    let b = lines_of(&request.b, request.ignore_line_endings);
    let (keys_a, keys_b): (Vec<String>, Vec<String>) = (
        a.iter().map(|line| key(line, request)).collect(),
        b.iter().map(|line| key(line, request)).collect(),
    );
    let ops = capture_diff_slices(Algorithm::Myers, &keys_a, &keys_b);

    let mut rows = Aligned {
        a: &a,
        b: &b,
        granularity: request.granularity,
        rows: Vec::new(),
        added: 0,
        removed: 0,
    };
    for op in &ops {
        rows.take(op);
    }

    let rows_truncated = rows.rows.len() > MAX_ROWS;
    rows.rows.truncate(MAX_ROWS);
    let json = |text: &str| {
        !text.trim().is_empty() && serde_json::from_str::<serde_json::Value>(text).is_ok()
    };
    TextDiffAnswer {
        added: saturating_u32(rows.added),
        removed: saturating_u32(rows.removed),
        rows: rows.rows,
        rows_truncated,
        unified: unified(&a, &b, &ops),
        identical: ops.iter().all(|op| matches!(op, DiffOp::Equal { .. })),
        both_json: json(&request.a) && json(&request.b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(a: &str, b: &str, granularity: Granularity) -> TextDiffRequest {
        TextDiffRequest {
            a: a.to_owned(),
            b: b.to_owned(),
            granularity,
            ignore_trailing_whitespace: false,
            ignore_all_whitespace: false,
            ignore_case: false,
            ignore_line_endings: false,
        }
    }

    fn kinds(answer: &TextDiffAnswer) -> Vec<TextRowKind> {
        answer.rows.iter().map(|row| row.kind).collect()
    }

    fn marked(line: &TextDiffLine) -> Vec<&str> {
        line.pieces
            .iter()
            .filter(|piece| piece.changed)
            .map(|piece| piece.text.as_str())
            .collect()
    }

    const BEFORE: &str = "host = db\nport = 5432\npool = 10\nuser = app\n";
    const AFTER: &str = "host = db\nport = 6432\npool = 10\nuser = app\nssl = on\n";

    #[test]
    fn lines_are_paired_side_by_side_and_counted() {
        let answer = diff(&ask(BEFORE, AFTER, Granularity::Lines));

        assert_eq!(
            kinds(&answer),
            [
                TextRowKind::Same,
                TextRowKind::Modified,
                TextRowKind::Same,
                TextRowKind::Same,
                TextRowKind::Added
            ]
        );
        assert_eq!((answer.added, answer.removed), (2, 1));
        assert!(!answer.identical);
        let modified = &answer.rows[1];
        assert_eq!(modified.left.as_ref().unwrap().number, 2);
        assert_eq!(marked(modified.right.as_ref().unwrap()), ["port = 6432"]);
    }

    #[test]
    fn words_and_characters_are_marked_inside_a_modified_line() {
        let words = diff(&ask(
            "the quick brown fox",
            "the slow brown fox",
            Granularity::Words,
        ));
        let row = &words.rows[0];
        assert_eq!(marked(row.left.as_ref().unwrap()), ["quick"]);
        assert_eq!(marked(row.right.as_ref().unwrap()), ["slow"]);
        assert_eq!((words.added, words.removed), (1, 1));

        let characters = diff(&ask("colour", "color", Granularity::Characters));
        let row = &characters.rows[0];
        assert_eq!(marked(row.left.as_ref().unwrap()), ["u"]);
        assert!(marked(row.right.as_ref().unwrap()).is_empty());
        assert_eq!((characters.added, characters.removed), (0, 1));
    }

    #[test]
    fn each_option_ignores_what_it_names() {
        let options = |set: fn(&mut TextDiffRequest), a: &str, b: &str| {
            let mut request = ask(a, b, Granularity::Lines);
            set(&mut request);
            diff(&request).identical
        };
        assert!(!options(|_| {}, "a  \nb", "a\nb"));
        assert!(options(
            |r| r.ignore_trailing_whitespace = true,
            "a  \nb",
            "a\nb"
        ));
        assert!(options(
            |r| r.ignore_all_whitespace = true,
            "a b\nc",
            "ab\n c"
        ));
        assert!(options(|r| r.ignore_case = true, "Hello", "hELLO"));
        assert!(!options(
            |r| r.ignore_trailing_whitespace = true,
            "a b",
            "ab"
        ));
    }

    #[test]
    fn crlf_against_lf_is_a_difference_until_line_endings_are_ignored() {
        let mut request = ask("a\r\nb\r\n", "a\nb\n", Granularity::Lines);
        let answer = diff(&request);
        assert!(!answer.identical);
        assert_eq!(
            answer.rows[0].left.as_ref().unwrap().pieces[0].text,
            "a",
            "a carriage return is not drawn"
        );

        request.ignore_line_endings = true;
        assert!(diff(&request).identical);
    }

    #[test]
    fn identical_texts_and_an_empty_side() {
        let same = diff(&ask(BEFORE, BEFORE, Granularity::Words));
        assert!(same.identical);
        assert_eq!(same.unified, "");
        assert_eq!((same.added, same.removed), (0, 0));

        let empty = diff(&ask("", "one\ntwo", Granularity::Lines));
        assert_eq!(kinds(&empty), [TextRowKind::Added, TextRowKind::Added]);
        assert_eq!(empty.added, 2);
    }

    #[test]
    fn the_unified_diff_is_what_diff_u_writes() {
        let answer = diff(&ask(BEFORE, AFTER, Granularity::Lines));

        assert_eq!(
            answer.unified,
            "--- a\n+++ b\n@@ -1,4 +1,5 @@\n host = db\n-port = 5432\n+port = 6432\n pool = 10\n user = app\n+ssl = on\n"
        );
    }

    #[test]
    fn the_rows_are_cut_past_their_cap_and_say_so() {
        let many = "line\n".repeat(MAX_ROWS + 10);
        let answer = diff(&ask(&many, "", Granularity::Lines));

        assert!(answer.rows_truncated);
        assert_eq!(answer.rows.len(), MAX_ROWS);
        assert_eq!(answer.removed, saturating_u32(MAX_ROWS + 10));
    }

    #[test]
    fn two_json_documents_are_said_to_be_comparable_as_values() {
        assert!(diff(&ask(r#"{"a":1}"#, r#"{"a":2}"#, Granularity::Lines)).both_json);
        assert!(!diff(&ask(r#"{"a":1}"#, "a: 2", Granularity::Lines)).both_json);
    }
}
