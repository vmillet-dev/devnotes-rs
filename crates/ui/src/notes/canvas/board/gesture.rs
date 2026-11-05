//! The geometry a pointer drag needs, in the surface's pixels: no egui, no state. Everything a
//! gesture writes lands on the [`GRID_PX`] lattice the ground is dotted with, and nothing snaps
//! before the pointer has travelled, so a click never becomes a move.

use engine::folders::board::{BoardFrame, BoardPoint, MIN_ZONE_HEIGHT, MIN_ZONE_WIDTH};

/// The lattice the ground is dotted with.
pub(super) const GRID_PX: i32 = 20;

/// Below this, a pointer that moved is a click that wobbled.
const DRAG_THRESHOLD_PX: i32 = 4;

/// A zone drawn smaller than this was a click on the background, not a gesture.
const MIN_DRAWN_ZONE: i32 = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Kind {
    /// The whole card is its handle; `filed` when it flows in a zone rather than sitting free.
    Card {
        note_id: String,
        filed: bool,
    },
    MoveZone(String),
    ResizeZone(String),
    /// The left button on the empty ground: a folder, once named.
    DrawZone,
    /// The right button anywhere: the cards it sweeps are ticked.
    SelectBand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Gesture {
    pub(super) kind: Kind,
    /// Where the pointer went down.
    pub(super) origin: BoardPoint,
    /// Where the thing moved started, so a drag keeps its grab offset.
    pub(super) from: BoardFrame,
    pub(super) to: BoardFrame,
    /// False until the pointer has travelled: a click must not persist.
    pub(super) moved: bool,
}

impl Gesture {
    pub(super) fn new(kind: Kind, origin: BoardPoint, from: BoardFrame) -> Self {
        Self {
            kind,
            origin,
            from,
            to: from,
            moved: false,
        }
    }

    /// The pointer is at `at`: the frame follows once it has travelled.
    pub(super) fn follow(&mut self, at: BoardPoint) {
        self.moved |= travelled(self.origin, at);
        if !self.moved {
            return;
        }
        // Exhaustive: a gesture of another kind does not compile until it says how it follows.
        self.to = match self.kind {
            Kind::Card { .. } | Kind::MoveZone(_) => moved_to(self, at),
            Kind::ResizeZone(_) => resized_to(self, at),
            Kind::DrawZone | Kind::SelectBand => drawn_to(self.origin, at),
        };
    }
}

/// To the nearest dot, a half rounding up as the reference's `Math.round` did.
pub(super) fn snap(value: i32) -> i32 {
    (value + GRID_PX / 2).div_euclid(GRID_PX) * GRID_PX
}

fn travelled(origin: BoardPoint, at: BoardPoint) -> bool {
    (at.x - origin.x).abs() >= DRAG_THRESHOLD_PX || (at.y - origin.y).abs() >= DRAG_THRESHOLD_PX
}

/// Keeps the grab offset: a frame follows the pointer, it does not jump under it.
fn moved_to(gesture: &Gesture, at: BoardPoint) -> BoardFrame {
    BoardFrame {
        x: snap(gesture.from.x + at.x - gesture.origin.x).max(0),
        y: snap(gesture.from.y + at.y - gesture.origin.y).max(0),
        ..gesture.from
    }
}

/// Snapped before it is clamped: the minimum is `folders::board`'s and not on the grid.
fn resized_to(gesture: &Gesture, at: BoardPoint) -> BoardFrame {
    BoardFrame {
        width: snap(gesture.from.width + at.x - gesture.origin.x).max(MIN_ZONE_WIDTH),
        height: snap(gesture.from.height + at.y - gesture.origin.y).max(MIN_ZONE_HEIGHT),
        ..gesture.from
    }
}

/// The band, whichever corner it started from. ⚠️ Both corners snap, not the size, which would
/// leave the far edge between two dots.
pub(super) fn drawn_to(origin: BoardPoint, at: BoardPoint) -> BoardFrame {
    let left = snap(origin.x.min(at.x)).max(0);
    let top = snap(origin.y.min(at.y)).max(0);
    BoardFrame {
        x: left,
        y: top,
        width: (snap(origin.x.max(at.x)) - left).max(0),
        height: (snap(origin.y.max(at.y)) - top).max(0),
    }
}

/// A band barely dragged was a click on the background, and creates nothing.
pub(super) fn worth_drawing(band: BoardFrame) -> bool {
    band.width >= MIN_DRAWN_ZONE && band.height >= MIN_DRAWN_ZONE
}

/// What a drawn band becomes once it is a zone: never too small to drop a card into.
pub(super) fn as_zone(band: BoardFrame) -> BoardFrame {
    BoardFrame {
        width: band.width.max(MIN_ZONE_WIDTH),
        height: band.height.max(MIN_ZONE_HEIGHT),
        ..band
    }
}

/// Touching is enough, as with any rubber band; half-open on the far edges.
pub(super) fn overlaps(band: BoardFrame, card: BoardFrame) -> bool {
    band.x < card.x + card.width
        && card.x < band.x + band.width
        && band.y < card.y + card.height
        && card.y < band.y + band.height
}

/// Moved by the keyboard: one dot of the lattice per arrow.
pub(super) fn nudged(frame: BoardFrame, (dx, dy): (i32, i32), resizing: bool) -> BoardFrame {
    if resizing {
        return BoardFrame {
            width: (frame.width + dx * GRID_PX).max(MIN_ZONE_WIDTH),
            height: (frame.height + dy * GRID_PX).max(MIN_ZONE_HEIGHT),
            ..frame
        };
    }
    BoardFrame {
        x: snap(frame.x + dx * GRID_PX).max(0),
        y: snap(frame.y + dy * GRID_PX).max(0),
        ..frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZONE: BoardFrame = BoardFrame {
        x: 100,
        y: 100,
        width: 300,
        height: 240,
    };

    fn at(x: i32, y: i32) -> BoardPoint {
        BoardPoint { x, y }
    }

    #[test]
    fn a_wobble_is_a_click_and_a_move_lands_on_the_grid_keeping_its_grab() {
        let mut gesture = Gesture::new(Kind::MoveZone("perf".into()), at(150, 120), ZONE);

        gesture.follow(at(152, 121));
        assert!(!gesture.moved);
        assert_eq!(gesture.to, ZONE);

        gesture.follow(at(197, 168));
        assert!(gesture.moved);
        assert_eq!((gesture.to.x, gesture.to.y), (140, 140));
        assert_eq!((gesture.to.width, gesture.to.height), (300, 240));
    }

    #[test]
    fn a_resize_snaps_then_clamps_and_moves_nothing() {
        let mut gesture = Gesture::new(Kind::ResizeZone("perf".into()), at(400, 340), ZONE);

        gesture.follow(at(470, 350));
        assert_eq!(
            gesture.to,
            BoardFrame {
                width: 380,
                height: 260,
                ..ZONE
            }
        );
        gesture.follow(at(0, 0));
        assert_eq!(
            (gesture.to.width, gesture.to.height),
            (MIN_ZONE_WIDTH, MIN_ZONE_HEIGHT)
        );
        assert_eq!((gesture.to.x, gesture.to.y), (100, 100));
    }

    #[test]
    fn a_band_is_the_same_from_either_corner_and_snaps_both() {
        let forward = drawn_to(at(61, 899), at(483, 1218));
        let backward = drawn_to(at(483, 1218), at(61, 899));

        assert_eq!(forward, backward);
        assert_eq!(
            forward,
            BoardFrame {
                x: 60,
                y: 900,
                width: 420,
                height: 320
            }
        );
        assert!(!worth_drawing(drawn_to(at(10, 10), at(30, 30))));
        assert_eq!(
            as_zone(BoardFrame {
                width: 40,
                height: 40,
                ..forward
            })
            .width,
            MIN_ZONE_WIDTH
        );
    }

    #[test]
    fn touching_is_overlapping_and_the_far_edges_are_open() {
        let card = BoardFrame {
            x: 0,
            y: 0,
            width: 240,
            height: 150,
        };

        assert!(overlaps(
            BoardFrame {
                x: 239,
                y: 149,
                width: 10,
                height: 10
            },
            card
        ));
        assert!(!overlaps(
            BoardFrame {
                x: 240,
                y: 0,
                width: 10,
                height: 10
            },
            card
        ));
    }

    #[test]
    fn the_keyboard_moves_a_dot_at_a_time_and_never_off_the_board() {
        assert_eq!(nudged(ZONE, (1, 0), false).x, 120);
        assert_eq!(nudged(BoardFrame { x: 0, ..ZONE }, (-1, 0), false).x, 0);
        assert_eq!(nudged(ZONE, (0, 1), true).height, 260);
        assert_eq!(nudged(ZONE, (-9, 0), true).width, MIN_ZONE_WIDTH);
    }
}
