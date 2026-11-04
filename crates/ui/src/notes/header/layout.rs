//! One row while every control holds in it, two on purpose past that — never one row that grows
//! because a control was squeezed. Decided from floors, not from the widths drawn: those differ
//! between one row and two, and an answer read off them would flip back and forth.

/// Between two controls, and between the groups.
pub(super) const GAP: f32 = 10.0;
/// The search stops growing there.
pub(super) const SEARCH_WIDEST: f32 = 520.0;

/// How narrow a control gets: in one row before the header takes two; in two, before it would
/// overflow at the window's floor with the rail open.
struct Floor {
    one_row: f32,
    two_rows: f32,
}

const SEARCH: Floor = Floor {
    one_row: 220.0,
    two_rows: 140.0,
};
const SWITCHER: Floor = Floor {
    one_row: 110.0,
    two_rows: 72.0,
};

/// What does not narrow, as last drawn, gaps inside each group included: the same in one row
/// and in two.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct Fixed {
    /// The rail toggle, the trash, the view, a breadcrumb.
    pub(super) place: f32,
    /// The quick filter, the order, the facets' toggle.
    pub(super) look: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Plan {
    pub(super) two_rows: bool,
    pub(super) search: f32,
    /// The widest each switcher is drawn.
    pub(super) switcher: f32,
}

/// `switchers`: each one's width with its whole name.
pub(super) fn plan(width: f32, fixed: Fixed, switchers: &[f32], new_note: f32) -> Plan {
    #[expect(clippy::cast_precision_loss, reason = "two switchers at most")]
    let count = switchers.len() as f32;
    let natural: f32 = switchers.iter().sum();
    let room = width - fixed.place - fixed.look - new_note - 2.0 * GAP;
    if room >= count * SWITCHER.one_row + SEARCH.one_row {
        // The search gives way first, then the switchers' names.
        let search = (room - natural).clamp(SEARCH.one_row, SEARCH_WIDEST);
        let switcher = share(room - search, count, SWITCHER.one_row);
        return Plan {
            two_rows: false,
            search,
            switcher,
        };
    }
    let first = width - fixed.place - new_note - GAP;
    Plan {
        two_rows: true,
        search: (width - fixed.look).clamp(SEARCH.two_rows, SEARCH_WIDEST),
        switcher: share(first, count, SWITCHER.two_rows),
    }
}

fn share(room: f32, count: f32, floor: f32) -> f32 {
    if count == 0.0 {
        return f32::INFINITY;
    }
    (room / count).max(floor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    const FIXED: Fixed = Fixed {
        place: 200.0,
        look: 300.0,
    };

    #[test]
    fn the_search_gives_way_before_the_switchers_then_two_rows() {
        let wide = plan(1_330.0, FIXED, &[150.0, 150.0], 140.0);
        assert!(!wide.two_rows);
        assert!(wide.switcher >= 150.0);

        let squeezed = plan(1_100.0, FIXED, &[150.0, 150.0], 140.0);
        assert!(!squeezed.two_rows);
        assert!(same(squeezed.search, SEARCH.one_row));
        assert!(squeezed.switcher < 150.0 && squeezed.switcher >= SWITCHER.one_row);

        let narrow = plan(640.0, FIXED, &[150.0, 150.0], 140.0);
        assert!(narrow.two_rows);
        assert!(narrow.search >= SEARCH.two_rows);
        assert!(narrow.switcher < 150.0);

        let tightest = plan(420.0, FIXED, &[150.0, 150.0], 140.0);
        assert!(same(tightest.switcher, SWITCHER.two_rows));
        assert!(same(tightest.search, SEARCH.two_rows));
    }

    #[test]
    fn without_switchers_only_the_search_narrows() {
        let plan = plan(1_100.0, FIXED, &[], 140.0);
        assert!(!plan.two_rows);
        assert!(same(
            plan.search,
            SEARCH_WIDEST.min(1_100.0 - 500.0 - 140.0 - 20.0)
        ));
    }
}
