//! Tab and Shift+Tab, and a whole text swapped for another as one edit (the reference's
//! `indentation.ts`). Offsets are `char`s, as everywhere in this crate.

/// One replacement, so undo takes it back in one step, and where the selection lands after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub from: usize,
    pub to: usize,
    pub insert: String,
    pub selection_start: usize,
    pub selection_end: usize,
}

/// How far Shift+Tab takes back a line indented with spaces, when a level is a tab.
const TAB_WIDTH: usize = 4;

/// At a caret, spaces up to the next level (or a tab); with a selection, one more level on every
/// line it touches — never in place of the selected text.
pub fn indent(text: &str, start: usize, end: usize, unit: &str) -> TextEdit {
    let chars: Vec<char> = text.chars().collect();
    if start == end {
        let column = start - line_start(&chars, start);
        let insert = if unit == "\t" {
            unit.to_owned()
        } else {
            let width = unit.chars().count().max(1);
            " ".repeat(width - column % width)
        };
        let caret = start + insert.chars().count();
        return TextEdit {
            from: start,
            to: end,
            insert,
            selection_start: caret,
            selection_end: caret,
        };
    }
    edit_lines(&chars, start, end, |line| LineChange {
        remove: 0,
        add: if line.is_empty() { "" } else { unit }.to_owned(),
    })
}

/// One level less on every line the caret or the selection touches, if it has one; `None` when
/// no line had any.
pub fn outdent(text: &str, start: usize, end: usize, unit: &str) -> Option<TextEdit> {
    let chars: Vec<char> = text.chars().collect();
    let width = if unit == "\t" {
        TAB_WIDTH
    } else {
        unit.chars().count()
    };
    let edit = edit_lines(&chars, start, end, |line| {
        if line.first() == Some(&'\t') {
            return LineChange {
                remove: 1,
                add: String::new(),
            };
        }
        let spaces = line.iter().take_while(|c| **c == ' ').count();
        LineChange {
            remove: spaces.min(width),
            add: String::new(),
        }
    });
    let before: String = chars[edit.from..edit.to].iter().collect();
    (edit.insert != before).then_some(edit)
}

/// The whole text for another, as one edit of only what differs: undo takes it back in one step,
/// and the lines either side keep their place.
pub fn rewrite(before: &str, after: &str, cursor: usize) -> TextEdit {
    let old: Vec<char> = before.chars().collect();
    let new: Vec<char> = after.chars().collect();
    let shortest = old.len().min(new.len());
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(shortest - prefix)
        .take_while(|(a, b)| a == b)
        .count();
    TextEdit {
        from: prefix,
        to: old.len() - suffix,
        insert: new[prefix..new.len() - suffix].iter().collect(),
        selection_start: cursor,
        selection_end: cursor,
    }
}

struct LineChange {
    /// Characters taken from the start of the line.
    remove: usize,
    /// Put at its start in their place.
    add: String,
}

fn line_start(chars: &[char], at: usize) -> usize {
    chars[..at]
        .iter()
        .rposition(|c| *c == '\n')
        .map_or(0, |newline| newline + 1)
}

/// A selection ending at the start of a line leaves that line alone, as in any code editor.
fn edit_lines(
    chars: &[char],
    start: usize,
    end: usize,
    change: impl Fn(&[char]) -> LineChange,
) -> TextEdit {
    let from = line_start(chars, start);
    let last = if end > start && chars.get(end - 1) == Some(&'\n') {
        end - 1
    } else {
        end
    };
    let to = chars[last..]
        .iter()
        .position(|c| *c == '\n')
        .map_or(chars.len(), |newline| last + newline);

    let (mut old_at, mut new_at) = (from, from);
    let (mut selection_start, mut selection_end) = (start, end);
    let mut lines = Vec::new();
    for line in chars[from..to].split(|c| *c == '\n') {
        let LineChange { remove, add } = change(line);
        let added = add.chars().count();
        let place = |at: usize| new_at + added + at.saturating_sub(old_at + remove);
        if (old_at..=old_at + line.len()).contains(&start) {
            // A selection from a line's start keeps the level it just gained.
            selection_start = if start == old_at && start != end {
                new_at
            } else {
                place(start)
            };
        }
        if (old_at..=old_at + line.len()).contains(&end) {
            selection_end = place(end);
        }
        let changed: String = add.chars().chain(line[remove..].iter().copied()).collect();
        old_at += line.len() + 1;
        new_at += changed.chars().count() + 1;
        lines.push(changed);
    }
    let insert = lines.join("\n");
    if end > to {
        selection_end = end + insert.chars().count() - (to - from);
    }
    TextEdit {
        from,
        to,
        insert,
        selection_start,
        selection_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text after the edit, `[` and `]` where the selection lands.
    fn applied(text: &str, edit: Option<TextEdit>) -> Option<String> {
        let edit = edit?;
        let chars: Vec<char> = text.chars().collect();
        let after: Vec<char> = chars[..edit.from]
            .iter()
            .copied()
            .chain(edit.insert.chars())
            .chain(chars[edit.to..].iter().copied())
            .collect();
        let part = |a: usize, b: usize| after[a..b].iter().collect::<String>();
        Some(format!(
            "{}[{}]{}",
            part(0, edit.selection_start),
            part(edit.selection_start, edit.selection_end),
            part(edit.selection_end, after.len())
        ))
    }

    /// `[` and `]` mark the selection before the edit.
    fn selected(marked: &str) -> (String, usize, usize) {
        let start = marked.chars().position(|c| c == '[').unwrap();
        let end = marked.chars().position(|c| c == ']').unwrap() - 1;
        (marked.replace(['[', ']'], ""), start, end)
    }

    fn tab(marked: &str, unit: &str) -> Option<String> {
        let (text, start, end) = selected(marked);
        applied(&text, Some(indent(&text, start, end, unit)))
    }

    fn shift_tab(marked: &str, unit: &str) -> Option<String> {
        let (text, start, end) = selected(marked);
        applied(&text, outdent(&text, start, end, unit))
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "compared with what an edit answers"
    )]
    fn some(text: &str) -> Option<String> {
        Some(text.to_owned())
    }

    #[test]
    fn tab_puts_spaces_up_to_the_next_level_at_the_caret() {
        assert_eq!(tab("if x:\n[]return", "  "), some("if x:\n  []return"));
        assert_eq!(tab("a[]b", "    "), some("a   []b"));
        assert_eq!(tab("func[]", "\t"), some("func\t[]"));
    }

    #[test]
    fn tab_adds_a_level_to_every_line_a_selection_touches() {
        assert_eq!(tab("a\n[b\nc]\nd", "  "), some("a\n[  b\n  c]\nd"));
        assert_eq!(tab("let [x] = 1", "  "), some("  let [x] = 1"));
        assert_eq!(tab("[a\n\nb\n]c", "  "), some("[  a\n\n  b\n]c"));
    }

    #[test]
    fn shift_tab_takes_one_level_off() {
        assert_eq!(shift_tab("    ret[]urn", "  "), some("  ret[]urn"));
        assert_eq!(shift_tab("\t  x[]", "    "), some("  x[]"));
        assert_eq!(shift_tab(" x[]", "    "), some("x[]"));
        assert_eq!(shift_tab("      x[]", "\t"), some("  x[]"));
        assert_eq!(shift_tab("[  a\n    b\nc]", "  "), some("[a\n  b\nc]"));
        assert_eq!(shift_tab("  [] x", "  "), some("[] x"));
        assert_eq!(shift_tab("a\n[b]", "  "), None);
    }

    #[test]
    fn a_rewrite_replaces_only_what_differs() {
        let edit = rewrite("keep f(a,b) keep", "keep f(a, b) keep", 4);
        assert_eq!((edit.from, edit.to, edit.insert.as_str()), (9, 9, " "));
        assert_eq!(
            applied("keep f(a,b) keep", Some(edit)),
            some("keep[] f(a, b) keep")
        );
        assert_eq!(
            applied("a  b", Some(rewrite("a  b", "a b", 3))),
            some("a b[]")
        );
        assert_eq!(applied("aa", Some(rewrite("aa", "aaa", 3))), some("aaa[]"));
    }

    #[test]
    fn offsets_are_characters_not_bytes() {
        assert_eq!(tab("é[]", "  "), some("é []"));
    }
}
