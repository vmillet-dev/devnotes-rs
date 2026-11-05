//! Markdown typed becomes what it writes (the input rules of the reference's `StarterKit`, as
//! `golden/rich-keys` records them): `# ` to `###### `, `- `, `* `, `+ `, `1. `, `[ ] `, `[x] `,
//! `> `, a fence, `---`, and `**bold**`, `*italic*`, `~~strike~~`, `` `code` `` closed at the caret.
//! A spaced emphasis — `2 * 3 *` — is kept as typed (the reference's `SpacedEmphasis`). The
//! patterns use lookahead, which the `regex` crate does not: each is read here by hand.

use crate::edit;
use crate::inlines::{self, Mark};
use crate::model::{Block, Document, Inline, ListKind, Selection};
use crate::tree::{self, Leaf};

/// What a rule did: the mark it applied, which the next typed text does not wear.
pub struct Applied {
    pub selection: Selection,
    pub mark: Option<Mark>,
}

/// After a `char` typed at the caret: the rule its text now ends with, applied.
pub fn after_typing(document: &mut Document, selection: &Selection) -> Option<Applied> {
    if !selection.is_empty() {
        return None;
    }
    let head = selection.head.clone();
    let Some(Leaf::Inlines(content)) = tree::leaf(document, &head.path) else {
        return None;
    };
    let before = inlines::split(content, head.offset).0;
    let text = inlines::plain(&before);
    let in_code = matches!(before.last(), Some(Inline::Text { marks, .. }) if marks.code);
    if in_code {
        return None;
    }
    let paragraph = matches!(tree::block(document, &head.path), Some(Block::Paragraph(_)));
    if paragraph && let Some(block) = block_rule(&text) {
        return Some(Applied {
            selection: apply_block(document, selection, block, head.offset),
            mark: None,
        });
    }
    let (mark, open, inner, close) = mark_rule(&text)?;
    let path = head.path.clone();
    let Some(tree::LeafMut::Inlines(content)) = tree::leaf_mut(document, &path) else {
        return None;
    };
    // The closing delimiter, then the opening one: the offsets before them stay true.
    inlines::delete(content, inner.end..inner.end + close);
    inlines::delete(content, open..open + (inner.start - open));
    let marked = open..open + (inner.end - inner.start);
    inlines::set(content, marked.clone(), &mark, true);
    Some(Applied {
        selection: Selection::caret(crate::model::Position {
            path,
            offset: marked.end,
        }),
        mark: Some(mark),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockRule {
    Heading(u8),
    Quote,
    List(ListKind, bool),
    Code(String),
    Rule,
}

/// A rule over the whole paragraph typed so far, each pattern of the corpus read in turn.
fn block_rule(text: &str) -> Option<BlockRule> {
    // The horizontal rule: three dashes need no space after them, underscores and stars do.
    if ["---", "—-", "___ ", "*** "].contains(&text) {
        return Some(BlockRule::Rule);
    }
    let body = text.strip_suffix(char::is_whitespace)?;
    if (1..=6).contains(&body.len()) && body.chars().all(|glyph| glyph == '#') {
        return Some(BlockRule::Heading(u8::try_from(body.len()).unwrap_or(1)));
    }
    let lead = body.trim_start();
    match lead {
        ">" => return Some(BlockRule::Quote),
        "-" | "+" | "*" => return Some(BlockRule::List(ListKind::Bullet, false)),
        "[]" | "[ ]" | "[(]" => return Some(BlockRule::List(ListKind::Task, false)),
        "[x]" => return Some(BlockRule::List(ListKind::Task, true)),
        _ => {}
    }
    if let Some(number) = body.strip_suffix('.')
        && !number.is_empty()
        && number.chars().all(|glyph| glyph.is_ascii_digit())
    {
        let start = number.parse().unwrap_or(1);
        return Some(BlockRule::List(ListKind::Ordered { start }, false));
    }
    let fence = body
        .strip_prefix("```")
        .or_else(|| body.strip_prefix("~~~"))?;
    fence
        .chars()
        .all(|glyph| glyph.is_ascii_lowercase())
        .then(|| BlockRule::Code(fence.to_owned()))
}

fn apply_block(
    document: &mut Document,
    selection: &Selection,
    rule: BlockRule,
    typed: usize,
) -> Selection {
    let path = selection.head.path.clone();
    let cleared = edit::delete_selection(
        document,
        &Selection {
            anchor: crate::model::Position {
                path: path.clone(),
                offset: 0,
            },
            head: crate::model::Position {
                path,
                offset: typed,
            },
        },
    );
    match rule {
        BlockRule::Heading(level) => {
            edit::set_heading(document, &cleared, Some(level));
            cleared
        }
        BlockRule::Quote => edit::toggle_quote(document, &cleared),
        BlockRule::List(kind, checked) => {
            let listed = edit::toggle_list(document, &cleared, kind);
            if checked
                && let Some((list, item, _)) = tree::innermost_item(document, &listed.head.path)
            {
                edit::toggle_task(document, &list, item);
            }
            listed
        }
        BlockRule::Code(language) => edit::set_code_block(document, &cleared, language),
        BlockRule::Rule => edit::replace_with_rule(document, &cleared),
    }
}

/// A mark closed at the end of `text`: the mark, where its opening delimiter starts, the inner
/// range, and the closing delimiter's length — all in `char`s.
fn mark_rule(text: &str) -> Option<(Mark, usize, std::ops::Range<usize>, usize)> {
    let glyphs: Vec<char> = text.chars().collect();
    if let Some((open, inner)) = code_span(&glyphs) {
        return Some((Mark::Code, open, inner, 1));
    }
    for (delimiter, mark) in [
        ("**", Mark::Bold),
        ("__", Mark::Bold),
        ("~~", Mark::Strike),
        ("*", Mark::Italic),
        ("_", Mark::Italic),
    ] {
        if let Some((open, inner)) = emphasis(&glyphs, delimiter) {
            if mark == Mark::Italic && spaced(&glyphs[inner.clone()]) {
                return None;
            }
            return Some((mark, open, inner, delimiter.len()));
        }
    }
    None
}

/// `(?:^|\s)(D(?!\s+D)([^c]+)D(?!\s+D))$` for a delimiter `D` of the `char` `c`: the opening
/// at the start or after a space, the inner neither empty nor spaces alone, holding no `c`.
fn emphasis(glyphs: &[char], delimiter: &str) -> Option<(usize, std::ops::Range<usize>)> {
    let delimiter: Vec<char> = delimiter.chars().collect();
    let width = delimiter.len();
    let mark = delimiter[0];
    let end = glyphs.len();
    if end < 2 * width + 1 || glyphs[end - width..] != delimiter[..] {
        return None;
    }
    let close = end - width;
    let open = glyphs[..close].iter().rposition(|glyph| *glyph == mark)? + 1 - width;
    if glyphs.get(open..open + width)? != delimiter.as_slice() {
        return None;
    }
    let inner = open + width..close;
    let flanked = open == 0 || glyphs[open - 1].is_whitespace();
    let filled = !inner.is_empty()
        && !glyphs[inner.clone()]
            .iter()
            .all(|glyph| glyph.is_whitespace());
    let clean = !glyphs[inner.clone()].contains(&mark);
    (flanked && filled && clean).then_some((open, inner))
}

/// The reference's `SpacedEmphasis`: an inner that starts or ends with a space is arithmetic.
fn spaced(inner: &[char]) -> bool {
    inner.first().is_some_and(|glyph| glyph.is_whitespace())
        || inner.last().is_some_and(|glyph| glyph.is_whitespace())
}

/// `` `([^`]+)`(?!`)$ ``, not right after another backtick.
fn code_span(glyphs: &[char]) -> Option<(usize, std::ops::Range<usize>)> {
    let end = glyphs.len();
    if end < 3 || glyphs[end - 1] != '`' {
        return None;
    }
    let open = glyphs[..end - 1].iter().rposition(|glyph| *glyph == '`')?;
    if open + 1 == end - 1 || (open > 0 && glyphs[open - 1] == '`') {
        return None;
    }
    Some((open, open + 1..end - 1))
}

#[cfg(test)]
mod tests;
