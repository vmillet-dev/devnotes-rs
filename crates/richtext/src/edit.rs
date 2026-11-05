//! What typing and the toolbar do to the document: each takes the selection and answers the new
//! one. Behaviour follows the reference's editor (ProseMirror's base keymap and TipTap's list
//! commands): Enter in an empty item leaves the list, Backspace at an item's start lifts it, Tab
//! nests an item under the one above.

use crate::inlines::{self, Mark};
use crate::model::{Block, Document, Inline, Item, ListKind, Marks, Position, Selection};
use crate::tree::{self, Leaf, LeafMut};

fn at(path: Vec<usize>, offset: usize) -> Position {
    Position { path, offset }
}

fn caret(path: Vec<usize>, offset: usize) -> Selection {
    Selection::caret(at(path, offset))
}

/// The selection's two ends, in reading order.
pub fn ordered(selection: &Selection) -> (Position, Position) {
    if selection.anchor <= selection.head {
        (selection.anchor.clone(), selection.head.clone())
    } else {
        (selection.head.clone(), selection.anchor.clone())
    }
}

/// The text selected, a line between leaves.
pub fn selected_text(document: &Document, selection: &Selection) -> String {
    let (start, end) = ordered(selection);
    let mut out = Vec::new();
    for path in tree::leaves(document) {
        if path < start.path || path > end.path {
            continue;
        }
        let text = match tree::leaf(document, &path) {
            Some(Leaf::Inlines(content)) => inlines::plain(content),
            Some(Leaf::Code(text)) => text.clone(),
            None => continue,
        };
        let from = if path == start.path { start.offset } else { 0 };
        let to = if path == end.path {
            end.offset
        } else {
            text.chars().count()
        };
        out.push(
            text.chars()
                .skip(from)
                .take(to.saturating_sub(from))
                .collect::<String>(),
        );
    }
    out.join("\n")
}

/// Text typed at the caret, in place of what is selected.
pub fn insert_text(
    document: &mut Document,
    selection: &Selection,
    text: &str,
    marks: &Marks,
) -> Selection {
    let mut selection = delete_selection(document, selection);
    // A code block holds its lines: they go in whole, never as blocks.
    if let Some(LeafMut::Code(code)) = tree::leaf_mut(document, &selection.head.path) {
        let Position { path, offset } = selection.head.clone();
        let byte = code
            .char_indices()
            .nth(offset)
            .map_or(code.len(), |(at, _)| at);
        code.insert_str(byte, text);
        return caret(path, offset + text.chars().count());
    }
    for (line, piece) in text.split('\n').enumerate() {
        if line > 0 {
            selection = split(document, &selection);
        }
        if piece.is_empty() {
            continue;
        }
        let Position { path, offset } = selection.head.clone();
        let count = piece.chars().count();
        match tree::leaf_mut(document, &path) {
            Some(LeafMut::Inlines(content)) => inlines::insert(
                content,
                offset,
                Inline::Text {
                    text: piece.to_owned(),
                    marks: marks.clone(),
                },
            ),
            Some(LeafMut::Code(code)) => {
                let byte = code
                    .char_indices()
                    .nth(offset)
                    .map_or(code.len(), |(at, _)| at);
                code.insert_str(byte, piece);
            }
            None => return selection,
        }
        selection = caret(path, offset + count);
    }
    selection
}

/// Shift+Enter: a line break inside the block.
pub fn hard_break(document: &mut Document, selection: &Selection) -> Selection {
    let selection = delete_selection(document, selection);
    let Position { path, offset } = selection.head.clone();
    match tree::leaf_mut(document, &path) {
        Some(LeafMut::Inlines(content)) => {
            inlines::insert(content, offset, Inline::HardBreak);
            caret(path, offset + 1)
        }
        _ => insert_text(document, &selection, "\n", &Marks::default()),
    }
}

/// What is selected taken out, the two ends joined; a caret is left as it is.
pub fn delete_selection(document: &mut Document, selection: &Selection) -> Selection {
    if selection.is_empty() {
        return selection.clone();
    }
    let (start, end) = ordered(selection);
    if start.path == end.path {
        cut(document, &start.path, start.offset..end.offset);
        return Selection::caret(start);
    }
    let tail = take_from(document, &end.path, end.offset);
    let units = units(document);
    for (path, leaf) in units.iter().rev() {
        if *path <= start.path || *path > end.path {
            continue;
        }
        if *leaf && is_cell(document, path) {
            cut(document, path, 0..usize::MAX);
        } else if let Some((blocks, index)) = tree::siblings_mut(document, path) {
            blocks.remove(index);
        }
    }
    prune(&mut document.blocks);
    cut(document, &start.path, start.offset..usize::MAX);
    append(document, &start.path, tail);
    Selection::caret(start)
}

fn cut(document: &mut Document, path: &[usize], range: std::ops::Range<usize>) {
    match tree::leaf_mut(document, path) {
        Some(LeafMut::Inlines(content)) => {
            let end = range.end.min(inlines::len(content));
            inlines::delete(content, range.start.min(end)..end);
        }
        Some(LeafMut::Code(text)) => {
            let kept: String = text
                .chars()
                .enumerate()
                .filter(|(at, _)| !range.contains(at))
                .map(|(_, glyph)| glyph)
                .collect();
            *text = kept;
        }
        None => {}
    }
}

/// The leaf's text from `offset` on, as runs.
fn take_from(document: &Document, path: &[usize], offset: usize) -> Vec<Inline> {
    match tree::leaf(document, path) {
        Some(Leaf::Inlines(content)) => inlines::split(content, offset).1,
        Some(Leaf::Code(text)) => vec![Inline::text(text.chars().skip(offset).collect::<String>())],
        None => Vec::new(),
    }
}

fn append(document: &mut Document, path: &[usize], tail: Vec<Inline>) {
    match tree::leaf_mut(document, path) {
        Some(LeafMut::Inlines(content)) => {
            content.extend(tail);
            inlines::normalize(content);
        }
        Some(LeafMut::Code(text)) => text.push_str(&inlines::plain(&tail)),
        None => {}
    }
}

fn is_cell(document: &Document, path: &[usize]) -> bool {
    tree::leaf(document, path).is_some() && tree::block(document, path).is_none()
}

/// Every leaf, and every image and rule, in reading order: what a selection can cover.
fn units(document: &Document) -> Vec<(Vec<usize>, bool)> {
    let mut found = Vec::new();
    walk(&document.blocks, &mut Vec::new(), &mut found);
    found
}

fn walk(blocks: &[Block], prefix: &mut Vec<usize>, found: &mut Vec<(Vec<usize>, bool)>) {
    for (at, block) in blocks.iter().enumerate() {
        prefix.push(at);
        match block {
            Block::Paragraph(_) | Block::Heading { .. } | Block::CodeBlock { .. } => {
                found.push((prefix.clone(), true));
            }
            Block::Image { .. } | Block::Rule => found.push((prefix.clone(), false)),
            Block::List { items, .. } => {
                for (index, item) in items.iter().enumerate() {
                    prefix.push(index);
                    walk(&item.blocks, prefix, found);
                    prefix.pop();
                }
            }
            Block::Quote(inner) => walk(inner, prefix, found),
            Block::Table(rows) => {
                for (row, cells) in rows.iter().enumerate() {
                    for cell in 0..cells.len() {
                        found.push(([prefix.as_slice(), &[row, cell]].concat(), true));
                    }
                }
            }
        }
        prefix.pop();
    }
}

/// Items, lists and quotes left with nothing in them go; a document is never left without a
/// paragraph to type in.
fn prune(blocks: &mut Vec<Block>) {
    prune_in(blocks);
    if blocks.is_empty() {
        blocks.push(Block::Paragraph(Vec::new()));
    }
}

fn prune_in(blocks: &mut Vec<Block>) {
    blocks.retain_mut(|block| match block {
        Block::List { items, .. } => {
            for item in items.iter_mut() {
                prune_in(&mut item.blocks);
            }
            items.retain(|item| !item.blocks.is_empty());
            !items.is_empty()
        }
        Block::Quote(inner) => {
            prune_in(inner);
            !inner.is_empty()
        }
        _ => true,
    });
}

/// Enter: the block split at the caret. An empty item leaves its list, an empty paragraph its
/// quote; a code block takes a line.
pub fn split(document: &mut Document, selection: &Selection) -> Selection {
    let selection = delete_selection(document, selection);
    let Position { path, offset } = selection.head.clone();
    let empty = tree::leaf(document, &path).is_none_or(|leaf| leaf.is_empty());
    if let Some(Leaf::Code(_)) = tree::leaf(document, &path) {
        return insert_text(document, &selection, "\n", &Marks::default());
    }
    if is_cell(document, &path) {
        return hard_break(document, &selection);
    }
    if let Some((list, item, rest)) = tree::innermost_item(document, &path)
        && rest == [0]
    {
        if empty {
            return lift_item(document, &list, item);
        }
        let right = take_from(document, &path, offset);
        cut(document, &path, offset..usize::MAX);
        let Some((blocks, index)) = tree::siblings_mut(document, &list) else {
            return selection;
        };
        if let Block::List { items, .. } = &mut blocks[index] {
            items.insert(
                item + 1,
                Item {
                    checked: false,
                    blocks: vec![Block::Paragraph(right)],
                },
            );
        }
        return caret([list.as_slice(), &[item + 1, 0]].concat(), 0);
    }
    if empty && in_quote(document, &path) {
        return lift_from_quote(document, &path);
    }
    let right = take_from(document, &path, offset);
    cut(document, &path, offset..usize::MAX);
    let Some((blocks, index)) = tree::siblings_mut(document, &path) else {
        return selection;
    };
    let next = match &blocks[index] {
        Block::Heading { level, .. } if !right.is_empty() => Block::Heading {
            level: *level,
            content: right,
        },
        _ => Block::Paragraph(right),
    };
    blocks.insert(index + 1, next);
    let mut next_path = path;
    if let Some(last) = next_path.last_mut() {
        *last += 1;
    }
    caret(next_path, 0)
}

fn in_quote(document: &Document, path: &[usize]) -> bool {
    path.len() >= 2 && matches!(document.blocks.get(path[0]), Some(Block::Quote(_)))
}

/// An empty paragraph at the end of a top-level quote moves out under it.
fn lift_from_quote(document: &mut Document, path: &[usize]) -> Selection {
    let quote = path[0];
    if let Some(Block::Quote(inner)) = document.blocks.get_mut(quote)
        && path.len() == 2
        && path[1] < inner.len()
    {
        let block = inner.remove(path[1]);
        document.blocks.insert(quote + 1, block);
        prune(&mut document.blocks);
        let landed = if document
            .blocks
            .get(quote)
            .is_some_and(|block| matches!(block, Block::Quote(_)))
        {
            quote + 1
        } else {
            quote
        };
        return caret(vec![landed], 0);
    }
    caret(path.to_vec(), 0)
}

/// Shift+Tab, Backspace at its start, Enter in it empty: the item one level up. In a nested list it
/// becomes an item of the list above, the ones after it nested under it; at the top it leaves the
/// list, which is cut in two around it.
pub fn lift_item(document: &mut Document, list: &[usize], item: usize) -> Selection {
    let Some((blocks, index)) = tree::siblings_mut(document, list) else {
        return Selection::default();
    };
    let Block::List { kind, items } = &mut blocks[index] else {
        return Selection::default();
    };
    let kind = *kind;
    let after: Vec<Item> = items.drain(item + 1..).collect();
    let mut lifted = items.remove(item);
    let emptied = items.is_empty();
    let parent = tree::innermost_item(document, list).filter(|(_, _, rest)| rest.len() == 1);
    if let Some((outer, outer_item, _)) = parent {
        if !after.is_empty() {
            lifted.blocks.push(Block::List { kind, items: after });
        }
        let Some((blocks, index)) = tree::siblings_mut(document, &outer) else {
            return Selection::default();
        };
        if let Block::List { items, .. } = &mut blocks[index] {
            items.insert(outer_item + 1, lifted);
        }
        if emptied && let Some((blocks, index)) = tree::siblings_mut(document, list) {
            blocks.remove(index);
        }
        return caret([outer.as_slice(), &[outer_item + 1, 0]].concat(), 0);
    }
    let Some((blocks, index)) = tree::siblings_mut(document, list) else {
        return Selection::default();
    };
    let mut at = index + 1;
    let first = at;
    for block in lifted.blocks {
        blocks.insert(at, block);
        at += 1;
    }
    if !after.is_empty() {
        blocks.insert(at, Block::List { kind, items: after });
    }
    let mut landed = first;
    if emptied {
        blocks.remove(index);
        landed -= 1;
    }
    let mut path = list.to_vec();
    if let Some(last) = path.last_mut() {
        *last = landed;
    }
    caret(path, 0)
}

/// Tab in an item: it goes under the item above, into its nested list of the same kind.
pub fn sink_item(document: &mut Document, selection: &Selection) -> Selection {
    let path = selection.head.path.clone();
    let Some((list, item, rest)) = tree::innermost_item(document, &path) else {
        return selection.clone();
    };
    if item == 0 {
        return selection.clone();
    }
    let Some((blocks, index)) = tree::siblings_mut(document, &list) else {
        return selection.clone();
    };
    let Block::List { kind, items } = &mut blocks[index] else {
        return selection.clone();
    };
    let kind = *kind;
    let moved = items.remove(item);
    let above = &mut items[item - 1];
    let nested = match above.blocks.last_mut() {
        Some(Block::List { kind: same, items }) if *same == kind => items,
        _ => {
            above.blocks.push(Block::List {
                kind,
                items: Vec::new(),
            });
            match above.blocks.last_mut() {
                Some(Block::List { items, .. }) => items,
                _ => return selection.clone(),
            }
        }
    };
    nested.push(moved);
    let new_item = nested.len() - 1;
    let nested_at = above.blocks.len() - 1;
    let new_path = [
        list.as_slice(),
        &[item - 1, nested_at, new_item],
        rest.as_slice(),
    ]
    .concat();
    caret(new_path, selection.head.offset)
}

/// Backspace: the `char` before the caret, or at a block's start, what joins it to the one before.
pub fn delete_backward(document: &mut Document, selection: &Selection) -> Selection {
    if !selection.is_empty() {
        return delete_selection(document, selection);
    }
    let Position { path, offset } = selection.head.clone();
    if offset > 0 {
        cut(document, &path, offset - 1..offset);
        return caret(path, offset - 1);
    }
    if let Some((list, item, rest)) = tree::innermost_item(document, &path)
        && rest == [0]
    {
        return lift_item(document, &list, item);
    }
    if let Some(Block::Heading { content, .. }) = tree::block(document, &path) {
        let content = content.clone();
        if let Some((blocks, index)) = tree::siblings_mut(document, &path) {
            blocks[index] = Block::Paragraph(content);
        }
        return caret(path, 0);
    }
    if is_cell(document, &path) || matches!(tree::leaf(document, &path), Some(Leaf::Code(_))) {
        return selection.clone();
    }
    join_backward(document, &path)
}

/// The leaf at `path` onto the end of the one before it; an image or a rule between goes first.
fn join_backward(document: &mut Document, path: &[usize]) -> Selection {
    let units = units(document);
    let Some(here) = units.iter().position(|(unit, _)| unit == path) else {
        return caret(path.to_vec(), 0);
    };
    let Some((before, leaf)) = here.checked_sub(1).map(|at| units[at].clone()) else {
        return caret(path.to_vec(), 0);
    };
    if !leaf {
        if let Some((blocks, index)) = tree::siblings_mut(document, &before) {
            blocks.remove(index);
        }
        prune(&mut document.blocks);
        let moved = if same_parent(&before, path) {
            let mut moved = path.to_vec();
            if let Some(last) = moved.last_mut() {
                *last -= 1;
            }
            moved
        } else {
            path.to_vec()
        };
        return caret(moved, 0);
    }
    if is_cell(document, &before) || matches!(tree::leaf(document, &before), Some(Leaf::Code(_))) {
        return caret(path.to_vec(), 0);
    }
    let length = tree::leaf(document, &before).map_or(0, |leaf| leaf.len());
    let tail = take_from(document, path, 0);
    if let Some((blocks, index)) = tree::siblings_mut(document, path) {
        blocks.remove(index);
    }
    prune(&mut document.blocks);
    append(document, &before, tail);
    caret(before, length)
}

fn same_parent(a: &[usize], b: &[usize]) -> bool {
    a.len() == b.len() && a[..a.len() - 1] == b[..b.len() - 1]
}

/// Delete: the `char` after the caret, or at a block's end, the next block joined onto it.
pub fn delete_forward(document: &mut Document, selection: &Selection) -> Selection {
    if !selection.is_empty() {
        return delete_selection(document, selection);
    }
    let Position { path, offset } = selection.head.clone();
    let length = tree::leaf(document, &path).map_or(0, |leaf| leaf.len());
    if offset < length {
        cut(document, &path, offset..offset + 1);
        return selection.clone();
    }
    let leaves = tree::leaves(document);
    let next = leaves
        .iter()
        .position(|leaf| *leaf == path)
        .and_then(|at| leaves.get(at + 1));
    match next {
        Some(next) if !is_cell(document, next) && !is_cell(document, &path) => {
            let next = next.clone();
            join_backward(document, &next)
        }
        _ => selection.clone(),
    }
}

/// Bold, italic, strike, code or a link over what is selected; on if any of it lacks it.
pub fn toggle_mark(document: &mut Document, selection: &Selection, mark: &Mark) {
    let (start, end) = ordered(selection);
    let spans = spans(document, &start, &end);
    let on = !spans
        .iter()
        .all(|(path, range)| match tree::leaf(document, path) {
            Some(Leaf::Inlines(content)) => inlines::has(content, range.clone(), mark),
            _ => true,
        });
    set_mark(document, selection, mark, on);
}

pub fn set_mark(document: &mut Document, selection: &Selection, mark: &Mark, on: bool) {
    let (start, end) = ordered(selection);
    for (path, range) in spans(document, &start, &end) {
        if let Some(LeafMut::Inlines(content)) = tree::leaf_mut(document, &path) {
            inlines::set(content, range, mark, on);
        }
    }
}

/// Each leaf the selection covers, and the range of it covered.
fn spans(
    document: &Document,
    start: &Position,
    end: &Position,
) -> Vec<(Vec<usize>, std::ops::Range<usize>)> {
    tree::leaves(document)
        .into_iter()
        .filter(|path| *path >= start.path && *path <= end.path)
        .map(|path| {
            let length = tree::leaf(document, &path).map_or(0, |leaf| leaf.len());
            let from = if path == start.path { start.offset } else { 0 };
            let to = if path == end.path { end.offset } else { length };
            (path, from..to.max(from))
        })
        .collect()
}

/// A heading of `level`, or a paragraph for `None`, for each paragraph or heading selected.
pub fn set_heading(document: &mut Document, selection: &Selection, level: Option<u8>) {
    let (start, end) = ordered(selection);
    for (path, _) in spans(document, &start, &end) {
        let Some((blocks, index)) = tree::siblings_mut(document, &path) else {
            continue;
        };
        let content = match &blocks[index] {
            Block::Paragraph(content) | Block::Heading { content, .. } => content.clone(),
            _ => continue,
        };
        blocks[index] = match level {
            Some(level) => Block::Heading { level, content },
            None => Block::Paragraph(content),
        };
    }
}

/// The list the caret is in becomes `kind`, or, already of it, is undone; outside a list, the
/// caret's block becomes one.
pub fn toggle_list(document: &mut Document, selection: &Selection, kind: ListKind) -> Selection {
    let path = selection.head.path.clone();
    let same = |a: ListKind, b: ListKind| std::mem::discriminant(&a) == std::mem::discriminant(&b);
    if let Some((list, item, rest)) = tree::innermost_item(document, &path) {
        let Some((blocks, index)) = tree::siblings_mut(document, &list) else {
            return selection.clone();
        };
        let Block::List {
            kind: current,
            items,
        } = &mut blocks[index]
        else {
            return selection.clone();
        };
        if !same(*current, kind) {
            *current = kind;
            return selection.clone();
        }
        let before: usize = items[..item].iter().map(|each| each.blocks.len()).sum();
        let unwrapped: Vec<Block> = std::mem::take(items)
            .into_iter()
            .flat_map(|each| each.blocks)
            .collect();
        blocks.splice(index..=index, unwrapped);
        let mut landed = list;
        if let Some(last) = landed.last_mut() {
            *last += before + rest.first().copied().unwrap_or(0);
        }
        let landed = [landed.as_slice(), rest.get(1..).unwrap_or_default()].concat();
        return Selection::caret(at(landed, selection.head.offset));
    }
    let Some((blocks, index)) = tree::siblings_mut(document, &path) else {
        return selection.clone();
    };
    if !matches!(blocks[index], Block::Paragraph(_) | Block::Heading { .. }) {
        return selection.clone();
    }
    let block = blocks.remove(index);
    blocks.insert(
        index,
        Block::List {
            kind,
            items: vec![Item {
                checked: false,
                blocks: vec![block],
            }],
        },
    );
    let landed = [path.as_slice(), &[0, 0]].concat();
    caret(landed, selection.head.offset)
}

/// The quote the caret is in is undone; outside one, its block goes into a quote.
pub fn toggle_quote(document: &mut Document, selection: &Selection) -> Selection {
    let path = selection.head.path.clone();
    let Some(&top) = path.first() else {
        return selection.clone();
    };
    if let Some(Block::Quote(inner)) = document.blocks.get_mut(top) {
        let inner = std::mem::take(inner);
        document.blocks.splice(path[0]..=path[0], inner);
        let landed = [
            &[path[0] + path.get(1).copied().unwrap_or(0)],
            path.get(2..).unwrap_or_default(),
        ]
        .concat();
        return caret(landed, selection.head.offset);
    }
    let block = document.blocks.remove(top);
    document.blocks.insert(top, Block::Quote(vec![block]));
    let landed = [&[top, 0], path.get(1..).unwrap_or_default()].concat();
    caret(landed, selection.head.offset)
}

/// A task's box ticked or not, by the path of its item's list and its index.
pub fn toggle_task(document: &mut Document, list: &[usize], item: usize) {
    if let Some((blocks, index)) = tree::siblings_mut(document, list)
        && let Block::List { items, .. } = &mut blocks[index]
        && let Some(item) = items.get_mut(item)
    {
        item.checked = !item.checked;
    }
}

/// The caret's paragraph becomes a code block in `language`, its text kept.
pub fn set_code_block(
    document: &mut Document,
    selection: &Selection,
    language: String,
) -> Selection {
    let path = selection.head.path.clone();
    if let Some((blocks, index)) = tree::siblings_mut(document, &path)
        && let Block::Paragraph(content) = &blocks[index]
    {
        let text = inlines::plain(content);
        blocks[index] = Block::CodeBlock { language, text };
    }
    selection.clone()
}

/// Ctrl+Alt+C: a paragraph becomes a code block, a code block a paragraph, its lines kept.
pub fn toggle_code_block(document: &mut Document, selection: &Selection) -> Selection {
    let path = selection.head.path.clone();
    let Some((blocks, index)) = tree::siblings_mut(document, &path) else {
        return selection.clone();
    };
    match &blocks[index] {
        Block::CodeBlock { text, .. } => {
            let mut content = Vec::new();
            for (at, line) in text.split('\n').enumerate() {
                if at > 0 {
                    content.push(Inline::HardBreak);
                }
                content.push(Inline::text(line));
            }
            inlines::normalize(&mut content);
            blocks[index] = Block::Paragraph(content);
            selection.clone()
        }
        _ => set_code_block(document, selection, String::new()),
    }
}

/// The fence of the code block the caret is in.
pub fn set_language(document: &mut Document, selection: &Selection, language: String) {
    if let Some((blocks, index)) = tree::siblings_mut(document, &selection.head.path)
        && let Block::CodeBlock {
            language: fence, ..
        } = &mut blocks[index]
    {
        *fence = language;
    }
}

/// Shift+Tab in a code block: one level of `unit` off the caret's line.
pub fn outdent_code(document: &mut Document, selection: &Selection, unit: &str) -> Selection {
    let Position { path, offset } = selection.head.clone();
    let Some(LeafMut::Code(text)) = tree::leaf_mut(document, &path) else {
        return selection.clone();
    };
    let chars: Vec<char> = text.chars().collect();
    let start = chars[..offset.min(chars.len())]
        .iter()
        .rposition(|glyph| *glyph == '\n')
        .map_or(0, |at| at + 1);
    let line: String = chars[start..].iter().collect();
    let taken = if line.starts_with(unit) {
        unit.chars().count()
    } else {
        line.chars()
            .take_while(|glyph| *glyph == ' ')
            .count()
            .min(unit.chars().count())
    };
    if taken == 0 {
        return selection.clone();
    }
    *text = chars[..start]
        .iter()
        .chain(&chars[start + taken..])
        .collect();
    caret(path, offset.saturating_sub(taken).max(start))
}

/// The caret's paragraph becomes a rule, what followed the caret a paragraph under it.
pub fn replace_with_rule(document: &mut Document, selection: &Selection) -> Selection {
    let path = selection.head.path.clone();
    let rest = take_from(document, &path, selection.head.offset);
    let Some((blocks, index)) = tree::siblings_mut(document, &path) else {
        return selection.clone();
    };
    blocks[index] = Block::Rule;
    blocks.insert(index + 1, Block::Paragraph(rest));
    let mut next = path;
    if let Some(last) = next.last_mut() {
        *last += 1;
    }
    caret(next, 0)
}

/// A rule after the caret's top block, the caret in a paragraph after it.
pub fn insert_rule(document: &mut Document, selection: &Selection) -> Selection {
    let top = selection.head.path.first().copied().unwrap_or(0);
    document.blocks.insert(top + 1, Block::Rule);
    document
        .blocks
        .insert(top + 2, Block::Paragraph(Vec::new()));
    caret(vec![top + 2], 0)
}

#[cfg(test)]
mod tests;
