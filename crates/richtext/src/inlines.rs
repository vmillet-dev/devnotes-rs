//! A leaf's marked runs, edited by `char` offset: a hard break counts one.

use std::ops::Range;

use crate::model::{Inline, Marks};

/// One mark, as a toggle names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mark {
    Bold,
    Italic,
    Strike,
    Code,
    /// `None` takes the link off.
    Link(Option<String>),
}

pub fn len(content: &[Inline]) -> usize {
    content.iter().map(Inline::len).sum()
}

/// The text, a break written `\n`.
pub fn plain(content: &[Inline]) -> String {
    content
        .iter()
        .map(|inline| match inline {
            Inline::Text { text, .. } => text.as_str(),
            Inline::HardBreak => "\n",
        })
        .collect()
}

/// The runs before `at`, and from it.
pub fn split(content: &[Inline], at: usize) -> (Vec<Inline>, Vec<Inline>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut seen = 0;
    for inline in content {
        let length = inline.len();
        if seen + length <= at {
            left.push(inline.clone());
        } else if seen >= at {
            right.push(inline.clone());
        } else if let Inline::Text { text, marks } = inline {
            let byte = byte_of(text, at - seen);
            left.push(Inline::Text {
                text: text[..byte].to_owned(),
                marks: marks.clone(),
            });
            right.push(Inline::Text {
                text: text[byte..].to_owned(),
                marks: marks.clone(),
            });
        }
        seen += length;
    }
    (left, right)
}

fn byte_of(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(at, _)| at)
}

pub fn insert(content: &mut Vec<Inline>, at: usize, inline: Inline) {
    let (mut left, right) = split(content, at);
    left.push(inline);
    left.extend(right);
    *content = left;
    normalize(content);
}

pub fn delete(content: &mut Vec<Inline>, range: Range<usize>) {
    let (left, rest) = split(content, range.start);
    let (_, right) = split(&rest, range.end - range.start);
    *content = left;
    content.extend(right);
    normalize(content);
}

/// Runs of equal marks joined, empty ones dropped.
pub fn normalize(content: &mut Vec<Inline>) {
    let mut joined: Vec<Inline> = Vec::with_capacity(content.len());
    for inline in content.drain(..) {
        if inline.is_empty() {
            continue;
        }
        if let (
            Some(Inline::Text { text, marks }),
            Inline::Text {
                text: more,
                marks: worn,
            },
        ) = (joined.last_mut(), &inline)
            && marks == worn
        {
            text.push_str(more);
            continue;
        }
        joined.push(inline);
    }
    *content = joined;
}

/// What typing at `at` writes with: the marks of the `char` before it, a link only inside one.
pub fn marks_at(content: &[Inline], at: usize) -> Marks {
    let mut seen = 0;
    let mut before = None;
    let mut after = None;
    for inline in content {
        let length = inline.len();
        if let Inline::Text { marks, .. } = inline {
            if seen < at && at <= seen + length {
                before = Some(marks);
            }
            if seen <= at && at < seen + length && after.is_none() {
                after = Some(marks);
            }
        }
        seen += length;
    }
    let Some(marks) = before.or(after.filter(|_| at == 0)) else {
        return Marks::default();
    };
    let mut marks = marks.clone();
    if after.is_none_or(|after| after.link != marks.link) {
        marks.link = None;
    }
    marks
}

fn wears(marks: &Marks, mark: &Mark) -> bool {
    match mark {
        Mark::Bold => marks.bold,
        Mark::Italic => marks.italic,
        Mark::Strike => marks.strike,
        Mark::Code => marks.code,
        Mark::Link(href) => marks.link == *href,
    }
}

pub fn put(marks: &mut Marks, mark: &Mark, on: bool) {
    match mark {
        Mark::Bold => marks.bold = on,
        Mark::Italic => marks.italic = on,
        Mark::Strike => marks.strike = on,
        Mark::Code => marks.code = on,
        Mark::Link(href) => marks.link = if on { href.clone() } else { None },
    }
}

/// Whether every `char` of `range` wears the mark.
pub fn has(content: &[Inline], range: Range<usize>, mark: &Mark) -> bool {
    let (_, rest) = split(content, range.start);
    let (inside, _) = split(&rest, range.end - range.start);
    inside.iter().all(|inline| match inline {
        Inline::Text { marks, .. } => wears(marks, mark),
        Inline::HardBreak => true,
    })
}

pub fn set(content: &mut Vec<Inline>, range: Range<usize>, mark: &Mark, on: bool) {
    let (left, rest) = split(content, range.start);
    let (mut inside, right) = split(&rest, range.end - range.start);
    for inline in &mut inside {
        if let Inline::Text { marks, .. } = inline {
            put(marks, mark, on);
        }
    }
    *content = left;
    content.extend(inside);
    content.extend(right);
    normalize(content);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold(text: &str) -> Inline {
        Inline::Text {
            text: text.to_owned(),
            marks: Marks {
                bold: true,
                ..Marks::default()
            },
        }
    }

    #[test]
    fn a_run_splits_inside_and_joins_back() {
        let mut content = vec![Inline::text("ab"), bold("cdé"), Inline::HardBreak];
        let (left, right) = split(&content, 3);
        assert_eq!(left, vec![Inline::text("ab"), bold("c")]);
        assert_eq!(right, vec![bold("dé"), Inline::HardBreak]);
        insert(&mut content, 5, bold("!"));
        assert_eq!(
            content,
            vec![Inline::text("ab"), bold("cdé!"), Inline::HardBreak]
        );
        delete(&mut content, 1..4);
        assert_eq!(
            content,
            vec![Inline::text("a"), bold("é!"), Inline::HardBreak]
        );
    }

    #[test]
    fn a_mark_toggles_over_a_range_and_typing_takes_the_one_before() {
        let mut content = vec![Inline::text("hello")];
        set(&mut content, 1..3, &Mark::Bold, true);
        assert!(has(&content, 1..3, &Mark::Bold));
        assert!(!has(&content, 0..3, &Mark::Bold));
        assert!(marks_at(&content, 3).bold);
        assert!(!marks_at(&content, 1).bold);
        set(
            &mut content,
            0..5,
            &Mark::Link(Some("https://a.fr".into())),
            true,
        );
        assert!(marks_at(&content, 2).link.is_some());
        assert!(
            marks_at(&content, 5).link.is_none(),
            "a link does not grow at its end"
        );
    }
}
