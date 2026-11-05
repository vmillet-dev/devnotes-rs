//! Where text lives in the document: a leaf is a block holding text — a paragraph, a heading, a
//! table's cell, a code block — reached by a path through lists' items, quotes and tables. A
//! caret is always in a leaf, and moves from one to the next in reading order.

use crate::model::{Block, Document, Inline, Position};

/// The text a leaf holds.
pub enum Leaf<'a> {
    Inlines(&'a Vec<Inline>),
    Code(&'a String),
}

pub enum LeafMut<'a> {
    Inlines(&'a mut Vec<Inline>),
    Code(&'a mut String),
}

impl Leaf<'_> {
    pub fn len(&self) -> usize {
        match self {
            Leaf::Inlines(content) => content.iter().map(Inline::len).sum(),
            Leaf::Code(text) => text.chars().count(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The leaf at `path`, if the path ends on one.
pub fn leaf<'a>(document: &'a Document, path: &[usize]) -> Option<Leaf<'a>> {
    leaf_in(&document.blocks, path)
}

fn leaf_in<'a>(blocks: &'a [Block], path: &[usize]) -> Option<Leaf<'a>> {
    let (first, rest) = path.split_first()?;
    match (blocks.get(*first)?, rest) {
        (Block::Paragraph(content) | Block::Heading { content, .. }, []) => {
            Some(Leaf::Inlines(content))
        }
        (Block::CodeBlock { text, .. }, []) => Some(Leaf::Code(text)),
        (Block::List { items, .. }, [item, rest @ ..]) => leaf_in(&items.get(*item)?.blocks, rest),
        (Block::Quote(inner), rest) => leaf_in(inner, rest),
        (Block::Table(rows), [row, cell]) => Some(Leaf::Inlines(rows.get(*row)?.get(*cell)?)),
        _ => None,
    }
}

pub fn leaf_mut<'a>(document: &'a mut Document, path: &[usize]) -> Option<LeafMut<'a>> {
    leaf_mut_in(&mut document.blocks, path)
}

fn leaf_mut_in<'a>(blocks: &'a mut [Block], path: &[usize]) -> Option<LeafMut<'a>> {
    let (first, rest) = path.split_first()?;
    match (blocks.get_mut(*first)?, rest) {
        (Block::Paragraph(content) | Block::Heading { content, .. }, []) => {
            Some(LeafMut::Inlines(content))
        }
        (Block::CodeBlock { text, .. }, []) => Some(LeafMut::Code(text)),
        (Block::List { items, .. }, [item, rest @ ..]) => {
            leaf_mut_in(&mut items.get_mut(*item)?.blocks, rest)
        }
        (Block::Quote(inner), rest) => leaf_mut_in(inner, rest),
        (Block::Table(rows), [row, cell]) => {
            Some(LeafMut::Inlines(rows.get_mut(*row)?.get_mut(*cell)?))
        }
        _ => None,
    }
}

/// Every leaf's path, in reading order.
pub fn leaves(document: &Document) -> Vec<Vec<usize>> {
    let mut found = Vec::new();
    collect(&document.blocks, &mut Vec::new(), &mut found);
    found
}

fn collect(blocks: &[Block], prefix: &mut Vec<usize>, found: &mut Vec<Vec<usize>>) {
    for (at, block) in blocks.iter().enumerate() {
        prefix.push(at);
        match block {
            Block::Paragraph(_) | Block::Heading { .. } | Block::CodeBlock { .. } => {
                found.push(prefix.clone());
            }
            Block::List { items, .. } => {
                for (index, item) in items.iter().enumerate() {
                    prefix.push(index);
                    collect(&item.blocks, prefix, found);
                    prefix.pop();
                }
            }
            Block::Quote(inner) => collect(inner, prefix, found),
            Block::Table(rows) => {
                for (row, cells) in rows.iter().enumerate() {
                    for cell in 0..cells.len() {
                        found.push([prefix.as_slice(), &[row, cell]].concat());
                    }
                }
            }
            Block::Image { .. } | Block::Rule => {}
        }
        prefix.pop();
    }
}

/// The list of blocks a path's block sits in, and its index there: the document's, a quote's
/// or an item's. `None` for a table's cell.
pub fn siblings_mut<'a>(
    document: &'a mut Document,
    path: &[usize],
) -> Option<(&'a mut Vec<Block>, usize)> {
    let (last, parent) = path.split_last()?;
    let blocks = container_mut(&mut document.blocks, parent)?;
    (*last < blocks.len()).then_some((blocks, *last))
}

fn container_mut<'a>(blocks: &'a mut Vec<Block>, path: &[usize]) -> Option<&'a mut Vec<Block>> {
    let Some((first, rest)) = path.split_first() else {
        return Some(blocks);
    };
    match (blocks.get_mut(*first)?, rest) {
        (Block::List { items, .. }, [item, rest @ ..]) => {
            container_mut(&mut items.get_mut(*item)?.blocks, rest)
        }
        (Block::Quote(inner), rest) => container_mut(inner, rest),
        _ => None,
    }
}

pub fn block<'a>(document: &'a Document, path: &[usize]) -> Option<&'a Block> {
    let (last, parent) = path.split_last()?;
    container(&document.blocks, parent)?.get(*last)
}

fn container<'a>(blocks: &'a [Block], path: &[usize]) -> Option<&'a [Block]> {
    let Some((first, rest)) = path.split_first() else {
        return Some(blocks);
    };
    match (blocks.get(*first)?, rest) {
        (Block::List { items, .. }, [item, rest @ ..]) => {
            container(&items.get(*item)?.blocks, rest)
        }
        (Block::Quote(inner), rest) => container(inner, rest),
        _ => None,
    }
}

/// Where a leaf's path enters a list: the list's path, the item's index, and what follows in
/// the item. `None` when the leaf is in no list.
pub fn innermost_item(
    document: &Document,
    path: &[usize],
) -> Option<(Vec<usize>, usize, Vec<usize>)> {
    let mut found = None;
    let mut blocks = document.blocks.as_slice();
    let mut at = 0;
    while at < path.len() {
        match blocks.get(path[at])? {
            Block::List { items, .. } if at + 1 < path.len() => {
                let item = path[at + 1];
                found = Some((path[..=at].to_vec(), item, path[at + 2..].to_vec()));
                blocks = &items.get(item)?.blocks;
                at += 2;
            }
            Block::Quote(inner) => {
                blocks = inner;
                at += 1;
            }
            _ => break,
        }
    }
    found
}

/// A position on a leaf: past a leaf's end, at its end; off every leaf, at the start of the
/// next one in reading order, or the end of the last.
pub fn clamp(document: &Document, position: &Position) -> Position {
    if let Some(found) = leaf(document, &position.path) {
        return Position {
            path: position.path.clone(),
            offset: position.offset.min(found.len()),
        };
    }
    let all = leaves(document);
    if let Some(next) = all.iter().find(|path| **path >= position.path) {
        return Position {
            path: next.clone(),
            offset: 0,
        };
    }
    all.last().map_or_else(Position::default, |path| Position {
        path: path.clone(),
        offset: leaf(document, path).map_or(0, |found| found.len()),
    })
}
