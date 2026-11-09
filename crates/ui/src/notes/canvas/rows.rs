//! The view flattened into rows of one height each — a section's heading, then its cards a
//! row at a time — so that only the rows in the viewport are drawn (plan § 3.6). Cards are a
//! fixed height in a grid of `repeat(auto-fill, minmax(220px, 1fr))`: the rows are uniform.

use std::ops::Range;

use engine::notes::view::NotesView;

const CARD_MIN_WIDTH: f32 = 220.0;
/// A heading's line, 11 px uppercase, and the 12 px under it.
pub const HEADING_HEIGHT: f32 = 15.0;
const HEADING_GAP: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Heading {
        section: usize,
    },
    /// Up to `columns` cells from `first`; the cell past a section's last card is its ghost.
    Cards {
        section: usize,
        first: usize,
    },
}

/// Where each row starts, worked out once for a view at a width.
#[derive(Debug, Default)]
pub struct Rows {
    all: Vec<Row>,
    tops: Vec<f32>,
    height: f32,
    columns: usize,
    card_width: f32,
}

impl Rows {
    /// `card` is a card's height, `grid` the gap between cards, `section` the one after a
    /// section.
    pub fn of(view: &NotesView, width: f32, card: f32, (grid, section): (f32, f32)) -> Self {
        let columns = columns(width, grid);
        #[expect(clippy::cast_precision_loss, reason = "a handful of columns")]
        let card_width = ((width - grid * (columns - 1) as f32) / columns as f32).max(0.0);
        let mut rows = Self {
            columns,
            card_width,
            ..Self::default()
        };
        for (index, shown) in view.sections.iter().enumerate() {
            let cells = shown.cards.len() + usize::from(shown.show_create_ghost);
            let after = if cells == 0 { section } else { HEADING_GAP };
            rows.push(Row::Heading { section: index }, HEADING_HEIGHT + after);
            let mut first = 0;
            while first < cells {
                let last = first + columns >= cells;
                let gap = if last { section } else { grid };
                rows.push(
                    Row::Cards {
                        section: index,
                        first,
                    },
                    card + gap,
                );
                first += columns;
            }
        }
        rows
    }

    fn push(&mut self, row: Row, height: f32) {
        self.all.push(row);
        self.tops.push(self.height);
        self.height += height;
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn card_width(&self) -> f32 {
        self.card_width
    }

    /// The rows that cross `top..bottom`, with where each starts.
    /// Where the row holding note `id` starts, if the view holds it.
    pub fn top_of(&self, view: &NotesView, id: &str) -> Option<f32> {
        let (section, at) = view
            .sections
            .iter()
            .enumerate()
            .find_map(|(section, shown)| {
                shown
                    .cards
                    .iter()
                    .position(|card| card.id == id)
                    .map(|at| (section, at))
            })?;
        self.all
            .iter()
            .zip(&self.tops)
            .find_map(|(row, top)| match row {
                Row::Cards { section: s, first }
                    if *s == section && (*first..*first + self.columns).contains(&at) =>
                {
                    Some(*top)
                }
                _ => None,
            })
    }

    pub fn visible(&self, top: f32, bottom: f32) -> impl Iterator<Item = (f32, Row)> + '_ {
        self.range(top, bottom)
            .map(|index| (self.tops[index], self.all[index]))
    }

    fn range(&self, top: f32, bottom: f32) -> Range<usize> {
        let first = self
            .tops
            .partition_point(|&start| start <= top)
            .saturating_sub(1);
        let end = self.tops.partition_point(|&start| start < bottom);
        first..end.max(first)
    }
}

/// As many 220 px columns as fit with their gaps, one at least.
fn columns(width: f32, gap: f32) -> usize {
    let fitting = ((width + gap) / (CARD_MIN_WIDTH + gap)).floor();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "floored, and clamped to one at least"
    )]
    let fitting = fitting.max(1.0) as usize;
    fitting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_follow_the_width_and_never_fall_below_one() {
        assert_eq!(columns(100.0, 12.0), 1);
        assert_eq!(columns(220.0, 12.0), 1);
        assert_eq!(columns(452.0, 12.0), 2);
        assert_eq!(columns(451.0, 12.0), 1);
        assert_eq!(columns(1_100.0, 12.0), 4);
    }

    fn rows(tops: &[f32]) -> Rows {
        Rows {
            all: tops
                .iter()
                .enumerate()
                .map(|(index, _)| Row::Heading { section: index })
                .collect(),
            tops: tops.to_vec(),
            height: tops.last().unwrap() + 100.0,
            columns: 1,
            card_width: 0.0,
        }
    }

    #[test]
    fn only_the_rows_crossing_the_viewport_are_visible() {
        let rows = rows(&[0.0, 100.0, 200.0, 300.0, 400.0]);

        let seen: Vec<f32> = rows.visible(150.0, 250.0).map(|(top, _)| top).collect();

        assert_eq!(seen, [100.0, 200.0]);
        assert_eq!(rows.visible(0.0, 50.0).count(), 1);
        assert_eq!(rows.visible(450.0, 900.0).count(), 1);
    }
}
