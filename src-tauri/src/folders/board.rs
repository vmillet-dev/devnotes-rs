//! The board: a second way to look at one space, where a folder is a region drawn on a
//! free canvas and its notes sit inside it.
//!
//! ⚠️ A query of its own rather than a bent [`crate::notes::view::NotesQuery`]: it answers
//! folders and positions, not sections, and `build_sections` must never learn about a
//! folder. What the two share is the coarse filtering, which happens in SQL either way.
//!
//! This module imports neither Diesel nor Tauri, so the layout rules are tested without
//! opening a database.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::model::Folder;
use crate::count::saturating_u32;
use crate::notes::language::Language;
use crate::notes::model::{self, DisplayNote, Note};
use crate::notes::view::{Criteria, Facets, NoteFilter};

/// The card is the same card as on the canvas — full size, with its language tag, its
/// snippet, its footer and its tags. The consequence is accepted: the board is large, and
/// panning arrives with it.
pub const CARD_WIDTH: i32 = 240;
/// Nominal: a real card grows with its content, and only the *first* frame is computed
/// from this. Once a zone has been resized the stored size is what counts.
pub const CARD_HEIGHT: i32 = 150;
pub const GAP: i32 = 12;
pub const ZONE_PADDING: i32 = 12;
/// The zone's own hairline. ⚠️ `box-sizing` is `border-box`, so it comes off the width
/// before anything flows inside — which is what [`columns_in`] got wrong.
pub const ZONE_BORDER: i32 = 1;
/// The title, the count and the ⋯ trigger.
pub const ZONE_HEADER: i32 = 38;
/// How many cards a freshly drawn zone is wide.
pub const ZONE_COLUMNS: i32 = 2;
/// A zone with no note is still a target to drop one into.
pub const MIN_ZONE_ROWS: i32 = 1;
/// How many zones sit side by side before the first layout wraps.
pub const BOARD_COLUMNS: i32 = 3;
pub const BOARD_MARGIN: i32 = 16;
/// How many loose cards sit side by side under the zones.
pub const LOOSE_COLUMNS: i32 = 4;
/// ⚠️ The zone body scrolls, so a vertical scrollbar can take a slice of the row. Without
/// this allowance two cards plus their gap come to *exactly* the inner width, the second
/// wraps, the wrap causes the scrollbar, and the scrollbar keeps it wrapped — a zone that
/// says "2" and shows one.
///
/// ⚠️ It is only ever added to [`default_zone_width`], never taken off a count: what fits
/// is what [`columns_in`] answers, and a zone [`zone_height`] sized correctly shows no
/// scrollbar at all. The measured slice in this `WebView` is 9px — this stays generous on
/// purpose, since it costs a few pixels of board and the alternative costs a column.
pub const SCROLLBAR: i32 = 18;

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardQuery {
    /// Required, unlike [`crate::notes::view::NotesQuery::space_id`]: a folder belongs to
    /// a space, so a board across all of them would have no zones to draw.
    pub space_id: String,
    pub search: String,
    pub filter: NoteFilter,
    pub tags: Vec<String>,
    pub languages: Vec<Language>,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardFrame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardPoint {
    pub x: i32,
    pub y: i32,
}

/// `flatten`: the front end draws this with the same card component the canvas uses.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardNote {
    #[serde(flatten)]
    pub note: DisplayNote,
    /// ⚠️ Dimmed in place rather than reflowed into a list: spatial memory is the only
    /// thing the board has that the date view does not, and a reflow throws it away.
    pub matches: bool,
    /// `None` inside a zone, where a card flows; `Some` only on the free background.
    pub position: Option<BoardPoint>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardZone {
    pub folder: Folder,
    pub frame: BoardFrame,
    pub notes: Vec<BoardNote>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardView {
    pub zones: Vec<BoardZone>,
    pub loose: Vec<BoardNote>,
    /// Attached to the space, like [`crate::notes::view::NotesView`]'s: facets drawn from
    /// already filtered notes would empty the rails on the first selection.
    pub available_tags: Vec<String>,
    pub available_languages: Vec<Language>,
    pub is_filtering: bool,
    pub matched: u32,
    /// The surface to pan over, so the front end sizes it from what is actually on it.
    pub width: i32,
    pub height: i32,
}

/// One zone that moved. A batch of these is what a gesture eventually writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ZonePlacement {
    pub folder_id: String,
    pub frame: BoardFrame,
}

/// One loose card that moved. ⚠️ Filing is not here: membership comes from
/// [`crate::folders::file_notes`], which answers what it changed so the undo can put it
/// back. A position is a local gesture and has no undo of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CardPlacement {
    pub note_id: String,
    pub position: BoardPoint,
}

/// Refuses a frame nothing could be dropped into, and keeps a zone on the board.
///
/// ⚠️ Clamped rather than rejected: a resize that ends at a silly size is a slip, and
/// answering an error mid-gesture would leave the interface holding a frame the database
/// refused.
#[must_use]
pub fn clamp(frame: BoardFrame) -> BoardFrame {
    BoardFrame {
        x: frame.x.max(0),
        y: frame.y.max(0),
        width: frame.width.clamp(MIN_ZONE_WIDTH, MAX_SIDE),
        height: frame.height.clamp(MIN_ZONE_HEIGHT, MAX_SIDE),
    }
}

#[must_use]
pub fn clamp_point(point: BoardPoint) -> BoardPoint {
    BoardPoint {
        x: point.x.clamp(0, MAX_SIDE),
        y: point.y.clamp(0, MAX_SIDE),
    }
}

/// A zone must stay big enough to hold the header and one card, or it becomes a target
/// nothing can be dropped into.
pub const MIN_ZONE_WIDTH: i32 = ZONE_BORDER * 2 + ZONE_PADDING * 2 + CARD_WIDTH;
pub const MIN_ZONE_HEIGHT: i32 = ZONE_BORDER * 2 + ZONE_HEADER + ZONE_PADDING * 2 + CARD_HEIGHT;
/// Far enough for any board, near enough that a runaway drag cannot make the surface
/// unusable.
pub const MAX_SIDE: i32 = 100_000;

/// Which zone a point falls in, topmost first. `None` is the free background, which is a
/// legitimate answer: dropping there takes a note out of its folder.
///
/// ⚠️ Membership comes from the drop and from nothing else. Unreal's own rule — a comment
/// owns whatever it overlaps — was considered and refused: it silently refiles notes the
/// day a frame is stretched.
#[must_use]
pub fn zone_at(frames: &[(String, BoardFrame)], point: BoardPoint) -> Option<String> {
    frames
        .iter()
        .rev()
        .find(|(_, frame)| {
            point.x >= frame.x
                && point.x < frame.x + frame.width
                && point.y >= frame.y
                && point.y < frame.y + frame.height
        })
        .map(|(id, _)| id.clone())
}

/// The width every zone gets on its first layout: [`ZONE_COLUMNS`] cards, the gaps, and
/// room for the scrollbar that a zone too short for its cards will show.
#[must_use]
pub fn default_zone_width() -> i32 {
    ZONE_PADDING * 2 + ZONE_COLUMNS * CARD_WIDTH + (ZONE_COLUMNS - 1) * GAP + SCROLLBAR
}

/// How many cards fit across a zone of this width — one at the very least.
///
/// ⚠️ Exactly what `.zone-body` will do with that width, and nothing more cautious: the
/// hairline and the padding come off, the [`SCROLLBAR`] allowance does **not**. That
/// allowance belongs to [`default_zone_width`], which reserves it so the nominal zone still
/// shows two cards across when it is too short for them. Subtracted here it made the count
/// disagree with the flow: a zone shaved 14px narrower than nominal still laid two cards
/// across and was told it held one, so the next card filed into it bought a whole extra row
/// and left 162px of empty board under the cards (#284).
#[must_use]
pub fn columns_in(width: i32) -> i32 {
    let inner = width - ZONE_BORDER * 2 - ZONE_PADDING * 2;
    ((inner + GAP) / (CARD_WIDTH + GAP)).max(1)
}

/// Tall enough for that many notes flowing that many across, and never shorter than one
/// row — an empty zone is still somewhere to drop a card.
///
/// ⚠️ The hairlines are in it, exactly as they are in [`columns_in`], and for a reason
/// that bites harder: `.zone-body` is the box that scrolls, so a zone one pixel short of
/// its own cards shows a **vertical** scrollbar, the scrollbar takes a slice of the row,
/// and the row it takes it from wraps — two cards across become one, two rows become three,
/// and nothing gets it back because the taller content keeps the scrollbar. Measured in the
/// assembled application: a 374px zone left its body 335px of client height where two rows
/// need 336, and the cards came out in a single column (#284).
#[must_use]
pub fn zone_height(note_count: usize, columns: i32) -> i32 {
    let columns = columns.max(1);
    let notes = i32::try_from(note_count).unwrap_or(i32::MAX);
    let rows = notes.div_euclid(columns) + i32::from(notes.rem_euclid(columns) != 0);

    let rows = rows.max(MIN_ZONE_ROWS);
    ZONE_BORDER * 2
        + ZONE_HEADER
        + ZONE_PADDING
        + rows * CARD_HEIGHT
        + (rows - 1) * GAP
        + ZONE_PADDING
}

/// The height a zone gets on its first layout, at [`ZONE_COLUMNS`] cards across.
#[must_use]
pub fn default_zone_height(note_count: usize) -> i32 {
    zone_height(note_count, ZONE_COLUMNS)
}

/// Lays zones out in reading order, wrapping every [`BOARD_COLUMNS`].
///
/// ⚠️ Deterministic and driven only by the order it is handed: the board would look
/// shuffled at every launch otherwise, and the caller reads folders in `created_at` order
/// for exactly that reason.
#[must_use]
pub fn arrange_zones(note_counts: &[usize]) -> Vec<BoardFrame> {
    let width = default_zone_width();
    let columns = usize::try_from(BOARD_COLUMNS).unwrap_or(1).max(1);

    let mut frames: Vec<BoardFrame> = Vec::with_capacity(note_counts.len());
    let mut row_top = BOARD_MARGIN;
    let mut row_height = 0;

    for (index, count) in note_counts.iter().enumerate() {
        let column = index % columns;
        if column == 0 && index > 0 {
            row_top += row_height + GAP;
            row_height = 0;
        }

        let height = default_zone_height(*count);
        row_height = row_height.max(height);

        frames.push(BoardFrame {
            x: BOARD_MARGIN + i32::try_from(column).unwrap_or(0) * (width + GAP),
            y: row_top,
            width,
            height,
        });
    }

    frames
}

/// Where the loose cards start: clear of the lowest zone.
///
/// ⚠️ It used to reserve a band above them for the "no folder · N" label. The label is
/// now a chip in the board's own corner — loose cards stopped being a band the day they
/// could be placed anywhere.
#[must_use]
pub fn loose_top(frames: &[BoardFrame]) -> i32 {
    frames
        .iter()
        .map(|frame| frame.y + frame.height)
        .max()
        .map_or(BOARD_MARGIN, |bottom| bottom + GAP * 2)
}

/// The `index`-th seat of the flow grid, left to right, wrapping every [`LOOSE_COLUMNS`].
fn slot(index: usize, top: i32) -> BoardPoint {
    let columns = usize::try_from(LOOSE_COLUMNS).unwrap_or(1).max(1);

    BoardPoint {
        x: BOARD_MARGIN + i32::try_from(index % columns).unwrap_or(0) * (CARD_WIDTH + GAP),
        y: top + i32::try_from(index / columns).unwrap_or(0) * (CARD_HEIGHT + GAP),
    }
}

/// Flows loose cards left to right under the zones, wrapping every [`LOOSE_COLUMNS`].
#[must_use]
pub fn arrange_loose(count: usize, top: i32) -> Vec<BoardPoint> {
    (0..count).map(|index| slot(index, top)).collect()
}

fn overlaps(at: BoardPoint, frame: BoardFrame) -> bool {
    at.x < frame.x + frame.width
        && frame.x < at.x + CARD_WIDTH
        && at.y < frame.y + frame.height
        && frame.y < at.y + CARD_HEIGHT
}

fn card_frame(at: BoardPoint) -> BoardFrame {
    BoardFrame {
        x: at.x,
        y: at.y,
        width: CARD_WIDTH,
        height: CARD_HEIGHT,
    }
}

/// Below everything already on the board — where a seat goes when the grid has none free.
fn under_everything(taken: &[BoardPoint], zones: &[BoardFrame]) -> BoardPoint {
    let bottom = taken
        .iter()
        .map(|at| at.y + CARD_HEIGHT)
        .chain(zones.iter().map(|frame| frame.y + frame.height))
        .max()
        .unwrap_or(BOARD_MARGIN);

    BoardPoint {
        x: BOARD_MARGIN,
        y: bottom + GAP,
    }
}

/// Where a card that has never been placed goes: the first seat of the flow grid nothing
/// is standing on, neither a card that already has a place nor a zone.
///
/// ⚠️ Not the seat its index in the list gives it. A new note is the most recently
/// updated, so it arrives at index 0 and was handed seat 0 — which whichever card was laid
/// out there on the very first read of the board is still sitting on. Two cards, one place.
///
/// ⚠️ A rectangle test and not an equality one: a card dragged by hand almost never sits
/// exactly on a seat, and a card half over one still hides what lands there.
#[must_use]
pub fn free_slot(top: i32, taken: &[BoardPoint], zones: &[BoardFrame]) -> BoardPoint {
    // Each occupant blocks at most the four seats its box can straddle; past that the
    // board is arranged in a way the grid cannot answer, and the card goes under it all.
    let limit = (taken.len() + zones.len()) * 4;

    (0..=limit)
        .map(|index| slot(index, top))
        .find(|at| {
            !taken.iter().any(|other| overlaps(*at, card_frame(*other)))
                && !zones.iter().any(|frame| overlaps(*at, *frame))
        })
        .unwrap_or_else(|| under_everything(taken, zones))
}

/// The surface to pan over: whatever the furthest zone or card reaches, plus a margin, and
/// never smaller than one screen's worth — a board holding one zone should still feel like
/// a canvas.
#[must_use]
pub fn surface(zones: &[BoardZone], loose: &[BoardNote]) -> (i32, i32) {
    const MIN_WIDTH: i32 = 960;
    const MIN_HEIGHT: i32 = 540;

    let mut right = MIN_WIDTH;
    let mut bottom = MIN_HEIGHT;

    for zone in zones {
        right = right.max(zone.frame.x + zone.frame.width);
        bottom = bottom.max(zone.frame.y + zone.frame.height);
    }
    for note in loose {
        if let Some(position) = note.position {
            right = right.max(position.x + CARD_WIDTH);
            bottom = bottom.max(position.y + CARD_HEIGHT);
        }
    }

    (right + BOARD_MARGIN, bottom + BOARD_MARGIN)
}

/// Pinned first, then by when they last moved — the order the canvas gives them, kept so a
/// note does not sit in one place on one view and another on the other.
fn in_zone_order(notes: &mut [BoardNote]) {
    notes.sort_by_key(|entry| !entry.note.pinned);
}

/// Assembles what the command read. Reads no database: the geometry arrives already
/// resolved, and so does the decoration that needs a connection.
#[must_use]
pub fn build<S: std::hash::BuildHasher>(
    notes: Vec<Note>,
    folders: Vec<Folder>,
    frames: &HashMap<String, BoardFrame, S>,
    positions: &HashMap<String, BoardPoint, S>,
    facets: Facets,
    request: &BoardQuery,
) -> BoardView {
    // ⚠️ Nothing is dropped: every criterion decides what is dimmed, never what is drawn.
    // Reflowing the survivors into a list would throw away the spatial memory the board
    // exists for.
    let criteria = Criteria::new(
        &request.search,
        request.filter,
        &request.tags,
        &request.languages,
    );

    let mut by_folder: HashMap<String, Vec<BoardNote>> = HashMap::new();
    let mut loose: Vec<BoardNote> = Vec::new();
    let mut matched = 0usize;

    for note in notes {
        let hit = criteria.passes(&note);
        matched += usize::from(hit);

        let folder_id = note.folder_id.clone();
        let entry = BoardNote {
            note: model::decorate(note, request.now),
            matches: hit,
            position: None,
        };

        match folder_id {
            Some(id) => by_folder.entry(id).or_default().push(entry),
            None => loose.push(entry),
        }
    }

    for entry in &mut loose {
        entry.position = positions.get(&entry.note.id).copied();
    }

    let placed = frames.values().copied().collect::<Vec<_>>();
    let fallback_top = loose_top(&placed);

    let zones: Vec<BoardZone> = folders
        .into_iter()
        .map(|folder| {
            let mut notes = by_folder.remove(&folder.id).unwrap_or_default();
            in_zone_order(&mut notes);

            let frame = frames.get(&folder.id).copied().unwrap_or(BoardFrame {
                x: BOARD_MARGIN,
                y: BOARD_MARGIN,
                width: default_zone_width(),
                height: default_zone_height(notes.len()),
            });

            BoardZone {
                folder,
                frame,
                notes,
            }
        })
        .collect();

    // A loose note with no stored place is one the geometry pass has not seen yet — it is
    // put on free ground rather than on top of a card that has one. Same rule as
    // `store::board::geometry`, which is what will write the place down.
    let mut standing: Vec<BoardPoint> = loose.iter().filter_map(|entry| entry.position).collect();
    for entry in &mut loose {
        if entry.position.is_none() {
            let at = free_slot(fallback_top, &standing, &placed);
            entry.position = Some(at);
            standing.push(at);
        }
    }

    // The quick filter counts here and not on the date view, deliberately: the board dims on
    // it, where the date view keeps its sections.
    let is_filtering = criteria.narrows() || request.filter != NoteFilter::All;

    let (width, height) = surface(&zones, &loose);

    BoardView {
        zones,
        loose,
        available_tags: facets.tags,
        available_languages: facets.languages,
        is_filtering,
        matched: saturating_u32(matched),
        width,
        height,
    }
}

/// A whole board's geometry in one value: every zone's frame and every loose card's
/// place. It says what a tidy-up is about to write, and — read back before the write —
/// what it has to be able to put back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardLayout {
    pub zones: Vec<ZonePlacement>,
    pub cards: Vec<CardPlacement>,
}

/// How much of a board a tidy-up is allowed to move.
///
/// ⚠️ Two, and not one with a warning on it. What goes to pieces on a board is the cards
/// **outside** the zones; a zone somebody positioned and sized by hand is the only manual
/// work the board holds. One button did both, so the click that repaired the cheap half
/// destroyed the expensive one — which is what stops anyone pressing it twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BoardScope {
    /// The loose cards alone, flowed under the zones **as they stand**. Often, and nothing
    /// anybody chose is lost.
    LooseCards,
    /// The zones as well: back in reading order, at the size their contents need. Rarely,
    /// and it overwrites every frame that was set by hand.
    Everything,
}

/// What a tidy-up did, and what it takes to walk it back.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BoardArrangement {
    /// ⚠️ What actually **moved**, not what was placed. A board already in order moves
    /// nothing, and an undo bar offering to put back a board nobody disturbed is noise.
    pub moved: u32,
    pub previous: BoardLayout,
}

/// Puts a whole space back in order: zones in reading order three across, each at the
/// height its contents need, and the loose cards flowing underneath.
///
/// ⚠️ It **resizes** as well as repositions, which is the whole point — an arrangement
/// that leaves a zone too small for what is in it has not arranged anything — and it is
/// therefore the one board gesture that overwrites a size chosen by hand. That is what
/// makes its undo non-optional, and what keeps it a notch further away than
/// [`arrange_loose_cards`].
///
/// `folder_counts` arrives in `created_at` order, as [`arrange_zones`] needs it: the same
/// board tidied twice has to be the same board.
#[must_use]
pub fn arrange_everything(folder_counts: &[(String, usize)], loose_ids: &[String]) -> BoardLayout {
    let counts: Vec<usize> = folder_counts.iter().map(|(_, count)| *count).collect();
    let frames = arrange_zones(&counts);

    BoardLayout {
        zones: folder_counts
            .iter()
            .zip(frames.iter())
            .map(|((folder_id, _), frame)| ZonePlacement {
                folder_id: folder_id.clone(),
                frame: *frame,
            })
            .collect(),
        cards: flowed(loose_ids, loose_top(&frames)),
    }
}

/// The loose cards alone, flowed under the zones exactly where they already are.
///
/// ⚠️ `zones` comes back empty, which is the whole point: nothing sized or placed by hand
/// is touched, so this one is worth a single click in the corner where
/// [`arrange_everything`] is not.
#[must_use]
pub fn arrange_loose_cards(frames: &[BoardFrame], loose_ids: &[String]) -> BoardLayout {
    BoardLayout {
        zones: Vec::new(),
        cards: flowed(loose_ids, loose_top(frames)),
    }
}

fn flowed(loose_ids: &[String], top: i32) -> Vec<CardPlacement> {
    loose_ids
        .iter()
        .zip(arrange_loose(loose_ids.len(), top))
        .map(|(note_id, position)| CardPlacement {
            note_id: note_id.clone(),
            position,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⚠️ Two cards *and* room for the scrollbar: at exactly the inner width the second
    /// card wraps, which is what causes the scrollbar that keeps it wrapped.
    #[test]
    fn a_zone_is_two_cards_wide_with_room_for_the_scrollbar() {
        assert_eq!(default_zone_width(), 12 + 240 + 12 + 240 + 12 + SCROLLBAR);
        assert!(default_zone_width() - ZONE_PADDING * 2 > ZONE_COLUMNS * CARD_WIDTH + GAP);
    }

    #[test]
    fn a_zone_grows_a_row_at_a_time() {
        let one_row = default_zone_height(2);
        let two_rows = default_zone_height(3);

        assert_eq!(two_rows - one_row, CARD_HEIGHT + GAP);
        assert_eq!(default_zone_height(4), two_rows);
    }

    /// ⚠️ Measured in the assembled application, and the reason the fix above was not
    /// enough on its own: `.zone-body` is the box that scrolls, and the two hairlines came
    /// off its height as well. A 374px zone gave it 335px of client height where two rows of
    /// cards need 336 — one pixel — so a vertical scrollbar appeared, took 9px off the row,
    /// and the second card wrapped. Three cards in one column, in a zone sized for two.
    #[test]
    fn a_zone_is_tall_enough_for_its_rows_once_its_own_hairlines_are_paid_for() {
        let body = zone_height(3, 2) - ZONE_BORDER * 2 - ZONE_HEADER;

        assert!(
            body >= ZONE_PADDING * 2 + CARD_HEIGHT * 2 + GAP,
            "{body}px of body for two rows that need {}",
            ZONE_PADDING * 2 + CARD_HEIGHT * 2 + GAP
        );
    }

    /// ⚠️ The report: a zone dragged 14px narrower than nominal still flows two cards
    /// across — `box-sizing` is `border-box`, and no scrollbar is showing once the zone is
    /// tall enough — and the count said one, so a third note bought a second empty row.
    #[test]
    fn a_zone_shaved_narrower_than_nominal_still_fits_two_across() {
        let shaved = default_zone_width() - 14;

        assert_eq!(columns_in(shaved), 2);
        assert_eq!(
            zone_height(3, columns_in(shaved)),
            zone_height(3, ZONE_COLUMNS)
        );
    }

    /// The narrowest width that still flows two, and the widest that does not.
    #[test]
    fn the_count_turns_over_where_the_flow_does() {
        let two = ZONE_BORDER * 2 + ZONE_PADDING * 2 + CARD_WIDTH * 2 + GAP;

        assert_eq!(columns_in(two), 2);
        assert_eq!(columns_in(two - 1), 1);
    }

    /// ⚠️ A zone widened by hand fits more across, and growing it by the nominal two
    /// columns would leave a band of nothing under the cards.
    #[test]
    fn a_wider_zone_needs_fewer_rows_for_the_same_notes() {
        let nominal = default_zone_width();
        let wider = nominal + CARD_WIDTH + GAP;

        assert_eq!(columns_in(nominal), ZONE_COLUMNS);
        assert_eq!(columns_in(wider), ZONE_COLUMNS + 1);
        assert!(zone_height(6, columns_in(wider)) < zone_height(6, columns_in(nominal)));
    }

    /// Below one card across, the flow still has to have a column.
    #[test]
    fn a_zone_squashed_narrow_still_counts_one_column() {
        assert_eq!(columns_in(0), 1);
        assert_eq!(columns_in(MIN_ZONE_WIDTH), 1);
    }

    /// A zone with nothing in it is still a target to drop a card into.
    #[test]
    fn an_empty_zone_is_still_a_row_tall() {
        assert_eq!(default_zone_height(0), default_zone_height(1));
    }

    #[test]
    fn zones_are_laid_out_in_reading_order() {
        let frames = arrange_zones(&[1, 1, 1]);

        assert_eq!(frames[0].y, frames[1].y);
        assert!(frames[0].x < frames[1].x);
        assert!(frames[1].x < frames[2].x);
    }

    #[test]
    fn the_layout_wraps_after_three_zones() {
        let frames = arrange_zones(&[1, 1, 1, 1]);

        assert_eq!(frames[3].x, frames[0].x);
        assert!(frames[3].y > frames[0].y);
    }

    /// ⚠️ A row advances by its tallest zone, or the next row lands on top of it.
    #[test]
    fn a_wrapped_row_clears_the_tallest_zone_above_it() {
        let frames = arrange_zones(&[1, 9, 1, 1]);

        let tallest = frames[..3].iter().map(|f| f.y + f.height).max().unwrap();
        assert!(frames[3].y >= tallest);
    }

    /// The same board twice is the same board: it must not look shuffled at every launch.
    #[test]
    fn the_layout_is_the_same_every_time() {
        assert_eq!(arrange_zones(&[3, 1, 7]), arrange_zones(&[3, 1, 7]));
    }

    #[test]
    fn loose_cards_flow_under_the_zones_with_room_for_their_label() {
        let frames = arrange_zones(&[1]);
        let top = loose_top(&frames);

        assert!(top > frames[0].y + frames[0].height);

        let points = arrange_loose(5, top);
        assert_eq!(
            points[0],
            BoardPoint {
                x: BOARD_MARGIN,
                y: top
            }
        );
        assert_eq!(points[3].y, top);
        assert_eq!(points[4].x, BOARD_MARGIN);
        assert_eq!(points[4].y, top + CARD_HEIGHT + GAP);
    }

    #[test]
    fn an_empty_board_still_puts_its_loose_cards_somewhere() {
        assert_eq!(loose_top(&[]), BOARD_MARGIN);
    }

    /// ⚠️ The report: a note captured from the clipboard was written under a card that was
    /// already there, and had to be dragged off to be found.
    #[test]
    fn a_card_with_no_place_never_takes_one_that_is_occupied() {
        let top = loose_top(&[]);
        let first = slot(0, top);

        assert_eq!(free_slot(top, &[], &[]), first);
        assert_eq!(free_slot(top, &[first], &[]), slot(1, top));
    }

    /// A card dragged by hand almost never sits exactly on a seat, and it hides the two it
    /// straddles just as well as the one it would have sat on.
    #[test]
    fn a_seat_half_covered_is_a_seat_taken() {
        let top = loose_top(&[]);
        let nudged = BoardPoint {
            x: BOARD_MARGIN + 20,
            y: top + 20,
        };

        let at = free_slot(top, &[nudged], &[]);

        assert!(!overlaps(at, card_frame(nudged)));
        assert_eq!(at, slot(2, top));
    }

    #[test]
    fn a_zone_standing_on_a_seat_takes_it_too() {
        let top = loose_top(&[]);
        let over = BoardFrame {
            x: BOARD_MARGIN,
            y: top,
            width: CARD_WIDTH,
            height: CARD_HEIGHT,
        };

        assert_eq!(
            free_slot(top, &[], std::slice::from_ref(&over)),
            slot(1, top)
        );
    }

    /// Holes are filled rather than skipped: the board stays as tight as it was arranged.
    #[test]
    fn a_seat_freed_in_the_middle_is_the_next_one_given() {
        let top = loose_top(&[]);
        let taken = [slot(0, top), slot(2, top)];

        assert_eq!(free_slot(top, &taken, &[]), slot(1, top));
    }

    /// With every seat the scan can reach standing on something, the card goes under it all
    /// rather than on top of one.
    #[test]
    fn a_card_with_nowhere_to_sit_goes_below_everything() {
        let top = loose_top(&[]);
        let wall = BoardFrame {
            x: 0,
            y: top,
            width: MAX_SIDE,
            height: MAX_SIDE,
        };

        let at = free_slot(top, &[], std::slice::from_ref(&wall));

        assert_eq!(at.x, BOARD_MARGIN);
        assert!(at.y >= wall.y + wall.height);
    }

    #[test]
    fn the_surface_covers_the_furthest_zone() {
        let zone = BoardZone {
            folder: crate::folders::model::Folder {
                id: "f-1".to_string(),
                space_id: "s-1".to_string(),
                name: "Perf".to_string(),
                colour: crate::folders::model::FolderColour::Blue,
                created_at: Utc::now(),
            },
            frame: BoardFrame {
                x: 2000,
                y: 1500,
                width: 300,
                height: 200,
            },
            notes: Vec::new(),
        };

        let (width, height) = surface(std::slice::from_ref(&zone), &[]);

        assert_eq!(width, 2300 + BOARD_MARGIN);
        assert_eq!(height, 1700 + BOARD_MARGIN);
    }

    /// A board holding one small zone should still feel like a canvas.
    #[test]
    fn the_surface_is_never_smaller_than_a_screen() {
        let (width, height) = surface(&[], &[]);

        assert!(width >= 960);
        assert!(height >= 540);
    }

    #[test]
    fn tidying_up_puts_every_zone_back_in_reading_order() {
        let counts = vec![("a".into(), 1), ("b".into(), 2), ("c".into(), 3)];

        let layout = arrange_everything(&counts, &[]);

        let xs: Vec<i32> = layout.zones.iter().map(|zone| zone.frame.x).collect();
        assert_eq!(xs[0], BOARD_MARGIN);
        assert!(xs[1] > xs[0] && xs[2] > xs[1]);
        assert!(layout.zones.iter().all(|zone| zone.frame.y == BOARD_MARGIN));
    }

    /// ⚠️ The half that makes it a tidy-up rather than a reshuffle: a zone stretched or
    /// squashed by hand comes back at the height its contents need.
    #[test]
    fn tidying_up_resizes_a_zone_to_what_it_holds() {
        let layout = arrange_everything(&[("a".into(), 5)], &[]);

        assert_eq!(layout.zones[0].frame.height, default_zone_height(5));
        assert_eq!(layout.zones[0].frame.width, default_zone_width());
    }

    #[test]
    fn tidied_loose_cards_flow_under_the_zones() {
        let layout = arrange_everything(&[("a".into(), 2)], &["n1".into(), "n2".into()]);

        let zone = layout.zones[0].frame;
        assert!(
            layout
                .cards
                .iter()
                .all(|card| card.position.y > zone.y + zone.height)
        );
        assert_eq!(layout.cards[0].position.x, BOARD_MARGIN);
        assert!(layout.cards[1].position.x > layout.cards[0].position.x);
    }

    #[test]
    fn the_same_board_tidied_twice_is_the_same_board() {
        let counts = vec![("a".into(), 4), ("b".into(), 1)];
        let loose = vec!["n1".to_string(), "n2".to_string()];

        assert_eq!(
            arrange_everything(&counts, &loose),
            arrange_everything(&counts, &loose)
        );
    }

    #[test]
    fn tidying_an_empty_space_writes_nothing() {
        let layout = arrange_everything(&[], &[]);

        assert!(layout.zones.is_empty() && layout.cards.is_empty());
    }
}
