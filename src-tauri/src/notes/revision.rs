//! The body as it was before an edit.
//!
//! ⚠️ The trash protects a deletion; nothing protected an edit. You adjust a command that
//! worked, it stops working, and the version that worked is gone. The point is not the
//! restoring — it is the **ease**: a text you know is recoverable is a text you edit
//! freely, and touching a snippet that works stops costing nerve.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Serialize;
use similar::{ChangeTag, DiffTag, TextDiff};
use specta::Type;

use crate::count::saturating_u32;

/// How many bodies are kept per note.
///
/// ⚠️ A **count**, not a time window. A body runs to tens of kilobytes, so a cap is the
/// only bound that is predictable for storage — the trash's thirty days is a precedent
/// for the shape of the retention, not for its unit.
pub const KEEP: usize = 20;

/// One kept body, as the panel lists it.
///
/// ⚠️ Metadata only. The bodies are what makes this table big, and a list that carried
/// twenty of them would send the whole history across to draw twenty dates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Revision {
    pub id: String,
    pub taken_at: DateTime<Utc>,
    /// So a row can say how much a version held without carrying it.
    pub characters: u32,
}

pub(crate) fn describe(id: String, taken_at: DateTime<Utc>, content: &str) -> Revision {
    Revision {
        id,
        taken_at,
        characters: saturating_u32(content.chars().count()),
    }
}

/// One line of a kept body, compared with the text restoring it would replace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DiffLine {
    /// In both: restoring leaves it where it is.
    Kept { text: String },
    /// Only in the kept body: restoring brings it back.
    Restored { text: String },
    /// Only in the current text: restoring takes it away.
    Dropped { text: String },
    /// Unchanged lines too far from any change to be worth reading.
    Skipped { count: u32 },
}

/// Lines shown either side of a change, as a patch shows them.
const CONTEXT: usize = 3;

/// ⚠️ A cap on time rather than on size: past it `similar` approximates instead of holding
/// the panel on a body of tens of kilobytes.
const DIFF_TIMEOUT: Duration = Duration::from_millis(300);

/// What restoring `version` over `current` would change, line by line. Empty when the two
/// are the same text.
#[must_use]
pub fn diff(current: &str, version: &str) -> Vec<DiffLine> {
    let diff = TextDiff::configure()
        .timeout(DIFF_TIMEOUT)
        .diff_lines(current, version);
    if diff.ops().iter().all(|op| op.tag() == DiffTag::Equal) {
        return Vec::new();
    }

    let total = diff.ops().last().map_or(0, |op| op.old_range().end);
    let mut lines = Vec::new();
    let mut shown = 0;

    for group in diff.grouped_ops(CONTEXT) {
        let (Some(first), Some(last)) = (group.first(), group.last()) else {
            continue;
        };
        skip(&mut lines, first.old_range().start - shown);

        for op in &group {
            for change in diff.iter_changes(op) {
                let text = change.value().trim_end_matches(['\n', '\r']).to_string();
                lines.push(match change.tag() {
                    ChangeTag::Equal => DiffLine::Kept { text },
                    ChangeTag::Insert => DiffLine::Restored { text },
                    ChangeTag::Delete => DiffLine::Dropped { text },
                });
            }
        }
        shown = last.old_range().end;
    }
    skip(&mut lines, total - shown);

    lines
}

fn skip(lines: &mut Vec<DiffLine>, count: usize) {
    if count > 0 {
        lines.push(DiffLine::Skipped {
            count: saturating_u32(count),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbered(count: usize) -> Vec<String> {
        (1..=count).map(|n| format!("ligne {n}")).collect()
    }

    fn text_of(line: &DiffLine) -> Option<&str> {
        match line {
            DiffLine::Kept { text } | DiffLine::Restored { text } | DiffLine::Dropped { text } => {
                Some(text)
            }
            DiffLine::Skipped { .. } => None,
        }
    }

    #[test]
    fn the_same_text_has_nothing_to_show() {
        assert!(diff("a\nb\n", "a\nb\n").is_empty());
    }

    /// ⚠️ The typo in the middle of two hundred lines is the case this exists for: the
    /// change and its context, and the rest folded to a count.
    #[test]
    fn a_change_is_shown_with_its_context_and_the_rest_is_folded() {
        let current = numbered(20);
        let mut version = current.clone();
        version[9] = "ligne dix".to_string();

        let lines = diff(&current.join("\n"), &version.join("\n"));

        assert_eq!(lines.first(), Some(&DiffLine::Skipped { count: 6 }));
        assert!(lines.contains(&DiffLine::Dropped {
            text: "ligne 10".to_string()
        }));
        assert!(lines.contains(&DiffLine::Restored {
            text: "ligne dix".to_string()
        }));
        assert_eq!(lines.last(), Some(&DiffLine::Skipped { count: 7 }));
        assert_eq!(
            lines
                .iter()
                .filter(|line| matches!(line, DiffLine::Kept { .. }))
                .count(),
            6
        );
    }

    /// Restoring is what is being previewed: a line only the kept body has comes **back**.
    #[test]
    fn a_line_only_the_version_holds_is_one_restoring_brings_back() {
        let lines = diff("select 1\n", "select 1\nwhere id = 2\n");

        assert_eq!(
            lines,
            vec![
                DiffLine::Kept {
                    text: "select 1".to_string()
                },
                DiffLine::Restored {
                    text: "where id = 2".to_string()
                },
            ]
        );
    }

    #[test]
    fn a_line_carries_no_line_ending() {
        let lines = diff("a\r\nb\r\n", "a\r\nc\r\n");

        assert!(!lines.is_empty());
        assert!(
            lines
                .iter()
                .filter_map(text_of)
                .all(|text| !text.ends_with(['\n', '\r']))
        );
    }
}
