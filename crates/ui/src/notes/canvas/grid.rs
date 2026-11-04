//! Where the arrows take the focus in the list view: through a section's cards as they are laid
//! out, `columns` a row, and on into the next section or back into the one before. Read from
//! the core's sections, not from what is drawn: a virtualised list draws only what is in view.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Previous,
    Next,
    Up,
    Down,
}

/// The card the focus moves to from `from`; the first card when nothing has the focus yet.
/// `sections` holds each section's note ids in order, empty ones included.
pub fn neighbour<'a>(
    sections: &[Vec<&'a str>],
    columns: usize,
    from: Option<&str>,
    direction: Direction,
) -> Option<&'a str> {
    let columns = columns.max(1);
    let filled: Vec<&Vec<&str>> = sections.iter().filter(|cards| !cards.is_empty()).collect();
    let Some(from) = from.and_then(|id| locate(&filled, id)) else {
        return filled.first().and_then(|cards| cards.first().copied());
    };
    let (section, at) = from;
    let cards = filled[section];
    let column = at % columns;
    let target = match direction {
        Direction::Next if at + 1 < cards.len() => Some((section, at + 1)),
        Direction::Next => (section + 1 < filled.len()).then_some((section + 1, 0)),
        Direction::Previous if at > 0 => Some((section, at - 1)),
        Direction::Previous => section
            .checked_sub(1)
            .map(|before| (before, filled[before].len() - 1)),
        Direction::Down if at + columns < cards.len() => Some((section, at + columns)),
        // A shorter last row below: its last card.
        Direction::Down if at / columns < (cards.len() - 1) / columns => {
            Some((section, cards.len() - 1))
        }
        Direction::Down => (section + 1 < filled.len())
            .then(|| (section + 1, column.min(filled[section + 1].len() - 1))),
        Direction::Up if at >= columns => Some((section, at - columns)),
        Direction::Up => section.checked_sub(1).map(|before| {
            let above = filled[before].len();
            let last_row = (above - 1) / columns * columns;
            (before, (last_row + column).min(above - 1))
        }),
    };
    target.map(|(section, at)| filled[section][at])
}

fn locate(sections: &[&Vec<&str>], id: &str) -> Option<(usize, usize)> {
    sections.iter().enumerate().find_map(|(section, cards)| {
        cards
            .iter()
            .position(|card| *card == id)
            .map(|at| (section, at))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two sections: « a » to « e » under one heading, an empty one, « f » and « g ».
    fn sections() -> Vec<Vec<&'static str>> {
        vec![vec!["a", "b", "c", "d", "e"], Vec::new(), vec!["f", "g"]]
    }

    fn moved(columns: usize, from: &str, direction: Direction) -> Option<&'static str> {
        neighbour(&sections(), columns, Some(from), direction)
    }

    #[test]
    fn the_first_arrow_lands_on_the_first_card() {
        assert_eq!(neighbour(&sections(), 3, None, Direction::Down), Some("a"));
        assert_eq!(
            neighbour(&sections(), 3, Some("gone"), Direction::Next),
            Some("a")
        );
    }

    #[test]
    fn left_and_right_follow_the_reading_order_across_sections() {
        assert_eq!(moved(3, "b", Direction::Next), Some("c"));
        assert_eq!(moved(3, "e", Direction::Next), Some("f"));
        assert_eq!(moved(3, "f", Direction::Previous), Some("e"));
        assert_eq!(moved(3, "a", Direction::Previous), None);
        assert_eq!(moved(3, "g", Direction::Next), None);
    }

    #[test]
    fn up_and_down_keep_the_column_whatever_the_width() {
        // Three columns: a b c / d e — then f g.
        assert_eq!(moved(3, "b", Direction::Down), Some("e"));
        assert_eq!(moved(3, "c", Direction::Down), Some("e"));
        assert_eq!(moved(3, "e", Direction::Down), Some("g"));
        assert_eq!(moved(3, "d", Direction::Down), Some("f"));
        assert_eq!(moved(3, "g", Direction::Up), Some("e"));
        assert_eq!(moved(3, "f", Direction::Up), Some("d"));
        // Two columns: a b / c d / e — then f g.
        assert_eq!(moved(2, "b", Direction::Down), Some("d"));
        assert_eq!(moved(2, "e", Direction::Up), Some("c"));
        assert_eq!(moved(2, "g", Direction::Up), Some("e"));
        // One: a column of cards.
        assert_eq!(moved(1, "e", Direction::Down), Some("f"));
    }
}
