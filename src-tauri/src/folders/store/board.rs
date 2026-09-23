//! The geometry, which is the half of a folder that does not travel. Same split as
//! `notes::store::trash` beside `notes::trash`: the rules are in `folders::board`, this is
//! only the SQL that stores and reads them.

use std::collections::HashMap;

use diesel::prelude::*;

use crate::count::saturating_u32;
use crate::db::Library;
use crate::db::schema::{folders, note_positions, notes};
use crate::error::StorageError;
use crate::folders::board::{
    self, BoardArrangement, BoardFrame, BoardLayout, BoardPoint, BoardScope, CardPlacement,
    ZonePlacement,
};

/// What a board read resolves before anything can be drawn: where each zone sits, and
/// where each loose card does.
pub type Geometry = (HashMap<String, BoardFrame>, HashMap<String, BoardPoint>);

/// A folder never laid out answers `None` — all four columns move together.
pub fn frames(
    connection: &mut SqliteConnection,
    space_id: &str,
) -> Result<HashMap<String, BoardFrame>, StorageError> {
    let rows = folders::table
        .filter(folders::space_id.eq(space_id))
        .select((folders::id, folders::x, folders::y, folders::w, folders::h))
        .load::<(String, Option<i32>, Option<i32>, Option<i32>, Option<i32>)>(connection)?;

    Ok(rows
        .into_iter()
        .filter_map(|(id, x, y, width, height)| {
            Some((
                id,
                BoardFrame {
                    x: x?,
                    y: y?,
                    width: width?,
                    height: height?,
                },
            ))
        })
        .collect())
}

/// One zone's frame, or `None` for a folder that has never been laid out.
pub fn frame_of(
    connection: &mut SqliteConnection,
    folder_id: &str,
) -> Result<Option<BoardFrame>, StorageError> {
    let row = folders::table
        .find(folder_id)
        .select((folders::x, folders::y, folders::w, folders::h))
        .first::<(Option<i32>, Option<i32>, Option<i32>, Option<i32>)>(connection)
        .optional()?;

    // All four columns move together: a half-laid-out folder is not a state.
    Ok(row.and_then(|(x, y, width, height)| {
        Some(BoardFrame {
            x: x?,
            y: y?,
            width: width?,
            height: height?,
        })
    }))
}

/// Opens a zone far enough to show everything filed into it, and never closes it again.
///
/// ⚠️ Grow only. A zone somebody stretched keeps its size; one they made too small for
/// what is now in it is reopened by the drop — where shrinking would move the board under
/// the pointer every time a card is taken out.
///
/// ⚠️ Rows counted against the zone's **own** width, not the nominal two columns: a zone
/// widened by hand fits more across, and growing it by the default would leave a band of
/// nothing under the cards.
///
/// A folder with no frame yet is left alone: `geometry` computes its first one from the
/// same count, on the next read.
pub fn grow_to_fit(
    connection: &mut SqliteConnection,
    folder_id: &str,
    note_count: usize,
) -> Result<(), StorageError> {
    let Some(frame) = frame_of(connection, folder_id)? else {
        return Ok(());
    };

    let needed = board::zone_height(note_count, board::columns_in(frame.width));
    if frame.height >= needed {
        return Ok(());
    }

    set_frame(
        connection,
        folder_id,
        board::clamp(BoardFrame {
            height: needed,
            ..frame
        }),
    )
}

pub fn set_frame(
    connection: &mut SqliteConnection,
    folder_id: &str,
    frame: BoardFrame,
) -> Result<(), StorageError> {
    diesel::update(folders::table.find(folder_id))
        .set((
            folders::x.eq(frame.x),
            folders::y.eq(frame.y),
            folders::w.eq(frame.width),
            folders::h.eq(frame.height),
        ))
        .execute(connection)?;

    Ok(())
}

/// ⚠️ Narrowed by subquery, not by a list of bound ids — the rule every side table here
/// follows. Reading a superset is harmless: the caller only looks up the notes it holds.
pub fn positions(
    connection: &mut SqliteConnection,
    space_id: &str,
) -> Result<HashMap<String, BoardPoint>, StorageError> {
    let rows = note_positions::table
        .filter(
            note_positions::note_id.eq_any(
                notes::table
                    .select(notes::id)
                    .filter(notes::space_id.eq(space_id.to_string())),
            ),
        )
        .select((
            note_positions::note_id,
            note_positions::x,
            note_positions::y,
        ))
        .load::<(String, i32, i32)>(connection)?;

    Ok(rows
        .into_iter()
        .map(|(note_id, x, y)| (note_id, BoardPoint { x, y }))
        .collect())
}

pub fn set_position(
    connection: &mut SqliteConnection,
    note_id: &str,
    point: BoardPoint,
) -> Result<(), StorageError> {
    diesel::replace_into(note_positions::table)
        .values((
            note_positions::note_id.eq(note_id),
            note_positions::x.eq(point.x),
            note_positions::y.eq(point.y),
        ))
        .execute(connection)?;

    Ok(())
}

/// ⚠️ A row exists only for a loose note, so filing one into a zone drops it: a filed card
/// flows inside its zone and has no position of its own to keep consistent.
pub fn forget_positions(
    connection: &mut SqliteConnection,
    note_ids: &[String],
) -> Result<(), StorageError> {
    if note_ids.is_empty() {
        return Ok(());
    }

    diesel::delete(note_positions::table.filter(note_positions::note_id.eq_any(note_ids)))
        .execute(connection)?;

    Ok(())
}

/// One transaction for the whole gesture: a board half written is a board nobody arranged.
pub fn save_layout(
    connection: &mut Library,
    zones: &[ZonePlacement],
    cards: &[CardPlacement],
) -> Result<(), StorageError> {
    if zones.is_empty() && cards.is_empty() {
        return Ok(());
    }

    connection.transaction(|connection, _vault| {
        for placement in zones {
            if !crate::folders::store::exists(connection, &placement.folder_id)? {
                return Err(StorageError::FolderNotFound(placement.folder_id.clone()));
            }
            set_frame(
                connection,
                &placement.folder_id,
                board::clamp(placement.frame),
            )?;
        }

        for placement in cards {
            // ⚠️ Only a loose note has a place of its own. A card filed between the drag
            // and the save flows inside its zone, and writing a position for it would put
            // a row back that `file_many` had just dropped.
            let filed: Option<String> = notes::table
                .find(&placement.note_id)
                .select(notes::folder_id)
                .first::<Option<String>>(connection)
                .optional()?
                .flatten();
            if filed.is_some() {
                continue;
            }

            set_position(
                connection,
                &placement.note_id,
                board::clamp_point(placement.position),
            )?;
        }

        Ok(())
    })
}

/// Reads the geometry, filling in whatever has never been laid out, in one transaction.
///
/// ⚠️ A read that writes, deliberately: every folder and every loose note needs a first
/// position, and computing one on the fly without storing it would let the very first drag
/// land next to cards that have no stored place of their own. It is idempotent — the
/// second board read of a space writes nothing.
pub fn geometry<S: std::hash::BuildHasher>(
    connection: &mut Library,
    space_id: &str,
    folder_ids: &[String],
    note_counts: &HashMap<String, usize, S>,
    loose_ids: &[String],
) -> Result<Geometry, StorageError> {
    connection.transaction(|connection, _vault| {
        let mut stored_frames = frames(connection, space_id)?;
        let mut stored_positions = positions(connection, space_id)?;

        let missing: Vec<&String> = folder_ids
            .iter()
            .filter(|id| !stored_frames.contains_key(*id))
            .collect();

        if !missing.is_empty() {
            // ⚠️ Arranged against *every* folder, not just the unplaced ones, so a folder
            // added later lands in the slot reading order gives it rather than on top of
            // the first zone.
            let counts: Vec<usize> = folder_ids
                .iter()
                .map(|id| note_counts.get(id).copied().unwrap_or(0))
                .collect();
            let arranged = board::arrange_zones(&counts);

            for (id, frame) in folder_ids.iter().zip(arranged) {
                if stored_frames.contains_key(id) {
                    continue;
                }
                set_frame(connection, id, frame)?;
                stored_frames.insert(id.clone(), frame);
            }
        }

        let unplaced: Vec<&String> = loose_ids
            .iter()
            .filter(|id| !stored_positions.contains_key(*id))
            .collect();

        if !unplaced.is_empty() {
            let zones: Vec<BoardFrame> = stored_frames.values().copied().collect();
            let top = board::loose_top(&zones);
            // ⚠️ The first seat nothing is standing on, and never the seat the note's
            // index in the list gives it: a note created now is the most recently updated,
            // so it arrives at index 0 and used to be written on top of whichever card was
            // laid out there on the board's very first read.
            let mut standing: Vec<BoardPoint> = stored_positions.values().copied().collect();

            for id in unplaced {
                let point = board::free_slot(top, &standing, &zones);
                set_position(connection, id, point)?;
                stored_positions.insert(id.clone(), point);
                standing.push(point);
            }
        }

        Ok((stored_frames, stored_positions))
    })
}

/// Every folder of the space in reading order, and how many live notes each one holds.
///
/// ⚠️ Ids and counts rather than [`crate::folders::store::list`] and
/// [`crate::notes::store::fetch`]: a tidy-up needs no name and no body, and those two
/// would decrypt the whole space to answer a count.
fn folder_counts(
    connection: &mut SqliteConnection,
    space_id: &str,
) -> Result<Vec<(String, usize)>, StorageError> {
    let ids = folders::table
        .filter(folders::space_id.eq(space_id))
        .order((folders::created_at.asc(), folders::id.asc()))
        .select(folders::id)
        .load::<String>(connection)?;

    let filed = notes::table
        .filter(notes::space_id.eq(space_id))
        .filter(notes::deleted_at.is_null())
        .filter(notes::folder_id.is_not_null())
        .select(notes::folder_id)
        .load::<Option<String>>(connection)?;

    let mut counts: HashMap<String, usize> = HashMap::new();
    for folder_id in filed.into_iter().flatten() {
        *counts.entry(folder_id).or_default() += 1;
    }

    Ok(ids
        .into_iter()
        .map(|id| {
            let count = counts.get(&id).copied().unwrap_or(0);
            (id, count)
        })
        .collect())
}

/// The unfiled notes, in the order the board draws them — pinned first, then by when they
/// last moved, exactly as `notes::store::fetch` hands them over.
fn loose_ids(
    connection: &mut SqliteConnection,
    space_id: &str,
) -> Result<Vec<String>, StorageError> {
    Ok(notes::table
        .filter(notes::space_id.eq(space_id))
        .filter(notes::deleted_at.is_null())
        .filter(notes::folder_id.is_null())
        .order((
            notes::pinned.desc(),
            notes::updated_at.desc(),
            notes::id.asc(),
        ))
        .select(notes::id)
        .load::<String>(connection)?)
}

/// Rewrites one space's geometry as far as `scope` allows, and answers what it was.
///
/// ⚠️ The **previous** layout and not the new one: the new one arrives with the reload the
/// front end does anyway, where this is the only moment the old one still exists.
///
/// ⚠️ Only what actually had a place is reported back. A zone the board had never laid out
/// had nothing to restore, and writing a frame for it on the undo would invent a position
/// the user never chose. `moved` is narrower still — it counts what came out somewhere
/// other than where it went in, so a board already in order opens no undo window at all.
pub fn arrange(
    connection: &mut Library,
    space_id: &str,
    scope: BoardScope,
) -> Result<BoardArrangement, StorageError> {
    connection.transaction(|connection, _vault| {
        let loose = loose_ids(connection, space_id)?;
        let before = frames(connection, space_id)?;
        let places = positions(connection, space_id)?;

        let next = match scope {
            BoardScope::Everything => {
                board::arrange_everything(&folder_counts(connection, space_id)?, &loose)
            }
            BoardScope::LooseCards => {
                let standing: Vec<BoardFrame> = before.values().copied().collect();
                board::arrange_loose_cards(&standing, &loose)
            }
        };

        for placement in &next.zones {
            set_frame(
                connection,
                &placement.folder_id,
                board::clamp(placement.frame),
            )?;
        }
        for placement in &next.cards {
            set_position(
                connection,
                &placement.note_id,
                board::clamp_point(placement.position),
            )?;
        }

        let zones: Vec<ZonePlacement> = next
            .zones
            .iter()
            .filter_map(|placement| {
                before.get(&placement.folder_id).map(|frame| ZonePlacement {
                    folder_id: placement.folder_id.clone(),
                    frame: *frame,
                })
            })
            .collect();
        let cards: Vec<CardPlacement> = next
            .cards
            .iter()
            .filter_map(|placement| {
                places
                    .get(&placement.note_id)
                    .map(|position| CardPlacement {
                        note_id: placement.note_id.clone(),
                        position: *position,
                    })
            })
            .collect();

        let moved = zones
            .iter()
            .filter(|was| {
                next.zones.iter().any(|now| {
                    now.folder_id == was.folder_id && board::clamp(now.frame) != was.frame
                })
            })
            .count()
            + cards
                .iter()
                .filter(|was| {
                    next.cards.iter().any(|now| {
                        now.note_id == was.note_id
                            && board::clamp_point(now.position) != was.position
                    })
                })
                .count();

        Ok(BoardArrangement {
            moved: saturating_u32(moved),
            previous: BoardLayout { zones, cards },
        })
    })
}
