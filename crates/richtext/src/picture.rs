//! The drawing a Note embeds: an image whose source is the drawing's id, and the part of the
//! text's width it takes, `devnotes:drawing/<id>?w=50`. Without one it is at its own size.

use crate::edit;
use crate::model::{Block, Document, Position, Selection};
use crate::tree;

/// How a Note names a drawing.
pub const DRAWING_SOURCE: &str = "devnotes:drawing/";

/// The narrowest and the widest a drawing is, in percent of the text's width.
pub const NARROWEST: u8 = 10;
pub const WIDEST: u8 = 100;

/// The widths a drag stops on, and how near it has to be.
pub const STOPS: [u8; 4] = [25, 50, 75, 100];
const NEAR: f32 = 3.0;

/// The drawing an image embeds, whatever its query says.
pub fn drawing_id(src: &str) -> Option<&str> {
    let id = src.strip_prefix(DRAWING_SOURCE)?;
    Some(id.split_once('?').map_or(id, |(id, _)| id))
}

/// A source as written: the image's own, and its width. Any other query is left in it.
pub(crate) fn read(src: String) -> (String, Option<u8>) {
    let width = drawing_id(&src).and_then(|_| {
        let (_, written) = src.split_once("?w=")?;
        let width: u8 = written.parse().ok()?;
        (NARROWEST..=WIDEST).contains(&width).then_some(width)
    });
    match (width, src.split_once('?')) {
        (Some(width), Some((source, _))) => (source.to_owned(), Some(width)),
        _ => (src, None),
    }
}

pub(crate) fn written(src: &str, width: Option<u8>) -> String {
    match width {
        Some(width) => format!("{src}?w={width}"),
        None => src.to_owned(),
    }
}

/// The width a drag to `percent` gives.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped between the narrowest and the widest"
)]
pub fn width_near(percent: f32) -> u8 {
    let stop = STOPS
        .into_iter()
        .find(|stop| (f32::from(*stop) - percent).abs() <= NEAR);
    let within = percent.clamp(f32::from(NARROWEST), f32::from(WIDEST));
    stop.unwrap_or_else(|| within.round() as u8)
}

/// A step wider or narrower, on a multiple of the step.
pub fn width_beside(width: u8, wider: bool) -> u8 {
    const STEP: u8 = 5;
    let stepped = if wider {
        (width / STEP + 1) * STEP
    } else {
        width.div_ceil(STEP).saturating_sub(1) * STEP
    };
    stepped.clamp(NARROWEST, WIDEST)
}

pub fn width(document: &Document, path: &[usize]) -> Option<u8> {
    match tree::block(document, path)? {
        Block::Image { width, .. } => *width,
        _ => None,
    }
}

/// Only a drawing has a width; whether it has another one now.
pub fn set_width(document: &mut Document, path: &[usize], to: Option<u8>) -> bool {
    let Some((blocks, index)) = tree::siblings_mut(document, path) else {
        return false;
    };
    match &mut blocks[index] {
        Block::Image { src, width, .. } if drawing_id(src).is_some() && *width != to => {
            *width = to;
            true
        }
        _ => false,
    }
}

/// What the caret meets past one end of a leaf or of an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Beside {
    Image(Vec<usize>),
    /// Its start, or its end when it is met going back.
    Leaf(Position),
}

/// A rule is stepped over.
pub fn beside(document: &Document, path: &[usize], forward: bool) -> Option<Beside> {
    let units = edit::units(document);
    let here = units.iter().position(|(unit, _)| unit == path)?;
    let mut back = units[..here].iter().rev();
    let mut ahead = units[here + 1..].iter();
    std::iter::from_fn(|| if forward { ahead.next() } else { back.next() }).find_map(
        |(unit, leaf)| {
            if *leaf {
                let length = tree::leaf(document, unit).map_or(0, |leaf| leaf.len());
                return Some(Beside::Leaf(Position {
                    path: unit.clone(),
                    offset: if forward { 0 } else { length },
                }));
            }
            let image = matches!(tree::block(document, unit), Some(Block::Image { .. }));
            image.then(|| Beside::Image(unit.clone()))
        },
    )
}

/// The image gone, the caret in the text before it, or in what is left.
pub fn remove(document: &mut Document, path: &[usize]) -> Selection {
    let before = match beside(document, path, false) {
        Some(Beside::Leaf(position)) => position,
        _ => Position {
            path: path.to_vec(),
            offset: 0,
        },
    };
    if let Some((blocks, index)) = tree::siblings_mut(document, path) {
        blocks.remove(index);
    }
    edit::prune(&mut document.blocks);
    Selection::caret(tree::clamp(document, &before))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read as read_note, write};

    const SIZED: &str = "avant\n\n![schéma](devnotes:drawing/a1?w=50)\n\naprès";

    #[test]
    fn the_width_is_read_and_written_back() {
        let mut document = read_note(SIZED);
        assert_eq!(width(&document, &[1]), Some(50));
        assert!(matches!(
            &document.blocks[1],
            Block::Image { src, .. } if src == "devnotes:drawing/a1"
        ));
        assert_eq!(write(&document), SIZED);

        assert!(set_width(&mut document, &[1], Some(75)));
        assert!(!set_width(&mut document, &[1], Some(75)), "as it was");
        assert!(write(&document).contains("devnotes:drawing/a1?w=75)"));
        set_width(&mut document, &[1], None);
        assert!(write(&document).contains("devnotes:drawing/a1)"));
        assert!(!set_width(&mut document, &[0], Some(50)), "a paragraph");
    }

    #[test]
    fn what_is_not_a_width_is_left_as_written_and_the_drawing_still_found() {
        for query in ["w=5", "w=101", "w=large", "x=50", "w=50&x=1", "w="] {
            let written = format!("![a](devnotes:drawing/a1?{query})");
            let document = read_note(&written);
            assert_eq!(width(&document, &[0]), None, "{query}");
            assert_eq!(write(&document), written);
        }
        assert_eq!(drawing_id("devnotes:drawing/a1?w=large"), Some("a1"));
        assert_eq!(drawing_id("https://a.fr/a.png"), None);
        let remote = "![a](https://a.fr/a.png?w=50)";
        let mut document = read_note(remote);
        assert_eq!(width(&document, &[0]), None, "a query of its own");
        assert!(!set_width(&mut document, &[0], Some(50)));
        assert_eq!(write(&document), remote);
    }

    #[test]
    fn a_drag_stops_on_the_quarters_and_a_step_on_fives() {
        let stopped = [23.0, 28.0, 51.4, 98.0, 140.0, 2.0, 61.0].map(width_near);
        assert_eq!(stopped, [25, 25, 50, 100, 100, 10, 61]);
        assert_eq!(width_beside(50, true), 55);
        assert_eq!(width_beside(50, false), 45);
        assert_eq!(width_beside(52, true), 55);
        assert_eq!(width_beside(52, false), 50);
        assert_eq!(width_beside(100, true), 100);
        assert_eq!(width_beside(10, false), 10);
    }

    #[test]
    fn the_caret_meets_the_image_then_the_text_past_it() {
        let document = read_note(SIZED);
        assert_eq!(beside(&document, &[0], true), Some(Beside::Image(vec![1])));
        let leaf = |path: usize, offset| {
            Some(Beside::Leaf(Position {
                path: vec![path],
                offset,
            }))
        };
        assert_eq!(beside(&document, &[1], true), leaf(2, 0));
        assert_eq!(beside(&document, &[1], false), leaf(0, 5));
        assert_eq!(beside(&document, &[2], true), None);
        let ruled = read_note("a\n\n---\n\nb");
        assert_eq!(beside(&ruled, &[0], true), leaf(2, 0));
    }

    #[test]
    fn removed_it_leaves_the_text_and_never_an_empty_note() {
        let mut document = read_note(SIZED);
        let selection = remove(&mut document, &[1]);
        assert_eq!(write(&document), "avant\n\naprès");
        assert_eq!(selection.head.path, [0]);
        assert_eq!(selection.head.offset, 5);

        let mut alone = read_note("![a](devnotes:drawing/a1)");
        remove(&mut alone, &[0]);
        assert_eq!(alone.blocks, [Block::Paragraph(Vec::new())]);
    }
}
