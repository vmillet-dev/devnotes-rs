//! Which lines of a text changed (the reference's `line-diff.ts`): what a format touched, for the
//! gutter and the count it announces, and what a revision's diff marks.

/// Past this many cells the table costs more than the answer is worth: every line counts.
const MAX_CELLS: usize = 4_000_000;

/// The lines of `after`, zero-based, outside the longest run of lines it shares with `before`.
pub fn changed_lines(before: &str, after: &str) -> Vec<usize> {
    let old: Vec<&str> = before.split('\n').collect();
    let next: Vec<&str> = after.split('\n').collect();
    let start = old.iter().zip(&next).take_while(|(a, b)| a == b).count();
    let (mut old_end, mut next_end) = (old.len(), next.len());
    while old_end > start && next_end > start && old[old_end - 1] == next[next_end - 1] {
        old_end -= 1;
        next_end -= 1;
    }
    let (rows, columns) = (old_end - start, next_end - start);
    if rows == 0 || rows * columns > MAX_CELLS {
        return (start..next_end).collect();
    }
    // Common-subsequence lengths from the end, so the walk below reads forwards.
    let width = columns + 1;
    let mut table = vec![0_u32; (rows + 1) * width];
    for i in (0..rows).rev() {
        for j in (0..columns).rev() {
            table[i * width + j] = if old[start + i] == next[start + j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let mut changed = Vec::new();
    let (mut i, mut j) = (0, 0);
    while j < columns {
        if i < rows && old[start + i] == next[start + j] {
            i += 1;
            j += 1;
        } else if i < rows && table[(i + 1) * width + j] >= table[i * width + j + 1] {
            i += 1;
        } else {
            changed.push(start + j);
            j += 1;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_texts_change_nothing() {
        assert!(changed_lines("a\nb", "a\nb").is_empty());
    }

    #[test]
    fn a_line_rewritten_in_place_is_marked() {
        assert_eq!(changed_lines("a\nb\nc", "a\nB\nc"), [1]);
        assert_eq!(changed_lines("x\nsame\ny", "X\nsame\nY"), [0, 2]);
    }

    #[test]
    fn a_split_statement_marks_its_lines_and_nothing_after() {
        assert_eq!(
            changed_lines("keep\nf(a, b)\nend", "keep\nf(\n  a,\n  b,\n)\nend"),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn lines_only_removed_mark_nothing() {
        assert!(changed_lines("a\n\n\nb", "a\nb").is_empty());
    }

    #[test]
    fn texts_too_long_to_compare_mark_every_new_line() {
        let before: Vec<String> = (0..2100).map(|n| format!("old {n}")).collect();
        let after: Vec<String> = (0..2100).map(|n| format!("new {n}")).collect();
        assert_eq!(
            changed_lines(&before.join("\n"), &after.join("\n")).len(),
            2100
        );
    }
}
