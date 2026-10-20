//! The geometry, the half of a folder that does not travel: the rules are in
//! `folders::board`, and this is the SQL that stores and reads them.

use std::collections::HashMap;

use diesel::prelude::*;

use crate::count::saturating_u32;
use crate::db::Library;
use crate::db::schema::{folders, note_positions, notes};
use crate::error::StorageError;
use crate::folders::board::{
    self, BoardArrangement, BoardFrame, BoardLayout, BoardPoint, BoardScope, CardPlacement,
    Occupancy, ZoneContents, ZonePlacement,
};

/// What a board read resolves before anything can be drawn: where each zone sits, and
/// where each loose card does.
pub type Geometry = (HashMap<String, BoardFrame>, HashMap<String, BoardPoint>);

type FrameColumns = (Option<i32>, Option<i32>, Option<i32>, Option<i32>);

/// All four columns move together: a half-laid-out folder is not a state, and a folder
/// never laid out answers `None`.
fn frame_from((x, y, width, height): FrameColumns) -> Option<BoardFrame> {
    Some(BoardFrame {
        x: x?,
        y: y?,
        width: width?,
        height: height?,
    })
}

pub fn frames(
    connection: &mut SqliteConnection,
    space_id: &str,
) -> Result<HashMap<String, BoardFrame>, StorageError> {
    let rows = folders::table
        .filter(folders::space_id.eq(space_id))
        .select((
            folders::id,
            (folders::x, folders::y, folders::w, folders::h),
        ))
        .load::<(String, FrameColumns)>(connection)?;

    Ok(rows
        .into_iter()
        .filter_map(|(id, columns)| Some((id, frame_from(columns)?)))
        .collect())
}

pub fn frame_of(
    connection: &mut SqliteConnection,
    folder_id: &str,
) -> Result<Option<BoardFrame>, StorageError> {
    let row = folders::table
        .find(folder_id)
        .select((folders::x, folders::y, folders::w, folders::h))
        .first::<FrameColumns>(connection)
        .optional()?;

    Ok(row.and_then(frame_from))
}

/// Opens a zone far enough to show everything filed into it, and never shrinks it: a zone
/// made too small is reopened by the drop, while shrinking would move the board under the
/// pointer. Rows are counted against the zone's own width. A folder with no frame yet is left
/// to `geometry`, which computes its first one on the next read.
pub fn grow_to_fit(
    connection: &mut SqliteConnection,
    folder_id: &str,
    note_count: usize,
    nested: bool,
) -> Result<(), StorageError> {
    let Some(frame) = frame_of(connection, folder_id)? else {
        return Ok(());
    };

    let needed = board::zone_height(note_count, board::columns_in(frame.width), nested);
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

/// Narrowed by the space's subquery: reading a superset is harmless, since the caller only
/// looks up the notes it holds.
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

/// A row exists only for a loose note: filing one drops it, since a filed card flows.
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
/// `root` is the board the gesture was made on: a card's place is on its own folder's board.
pub fn save_layout(
    connection: &mut Library,
    root: Option<&str>,
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

        // ⚠️ Only a note filed in the root has a place on its board: one filed elsewhere between
        // the drag and the save would get back the row `file_many` just dropped.
        let moved: Vec<&String> = cards.iter().map(|placement| &placement.note_id).collect();
        let here: std::collections::HashSet<String> = notes::table
            .filter(notes::id.eq_any(moved))
            .select((notes::id, notes::folder_id))
            .load::<(String, Option<String>)>(connection)?
            .into_iter()
            .filter(|(_, folder_id)| folder_id.as_deref() == root)
            .map(|(id, _)| id)
            .collect();

        for placement in cards {
            if !here.contains(&placement.note_id) {
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
/// ⚠️ A read that writes: every folder and loose note gets a first position stored, or the
/// first drag lands among cards that have none. Idempotent — a second read writes nothing.
/// `zones` are the board's — the root's children, each saying whether it shows sub-folders —
/// and only their frames and the places of the cards free on it are read: another board's
/// geometry must not take a seat on this one.
pub fn geometry(
    connection: &mut Library,
    space_id: &str,
    zones: &[(String, bool)],
    occupancy: &Occupancy,
) -> Result<Geometry, StorageError> {
    connection.transaction(|connection, _vault| {
        let mut stored_frames = frames(connection, space_id)?;
        stored_frames.retain(|id, _| zones.iter().any(|(zone, _)| zone == id));
        let mut stored_positions = positions(connection, space_id)?;
        stored_positions.retain(|id, _| occupancy.loose.contains(id));

        let folder_ids: Vec<&String> = zones.iter().map(|(id, _)| id).collect();
        let missing: Vec<&&String> = folder_ids
            .iter()
            .filter(|id| !stored_frames.contains_key(**id))
            .collect();

        if !missing.is_empty() {
            // Against every zone, so one added later takes its reading-order slot rather
            // than the first zone's.
            let contents: Vec<ZoneContents> = zones
                .iter()
                .map(|(id, nested)| ZoneContents {
                    notes: occupancy.per_folder.get(id).copied().unwrap_or(0),
                    nested: *nested,
                })
                .collect();
            let arranged = board::arrange_zones(&contents);

            for (id, frame) in folder_ids.into_iter().zip(arranged) {
                if stored_frames.contains_key(id) {
                    continue;
                }
                set_frame(connection, id, frame)?;
                stored_frames.insert(id.clone(), frame);
            }
        }

        let unplaced: Vec<&String> = occupancy
            .loose
            .iter()
            .filter(|id| !stored_positions.contains_key(*id))
            .collect();

        if !unplaced.is_empty() {
            let zones: Vec<BoardFrame> = stored_frames.values().copied().collect();
            let top = board::loose_top(&zones);
            // The first seat nothing stands on, never the one of its index: a new note arrives
            // at index 0 and would land on whichever card sits there.
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

/// The board's zones in reading order, and what each one's sub-tree holds.
///
/// Ids and counts rather than `folders::store::list` and `notes::store::fetch`, which would
/// decrypt the whole space to answer a count.
fn folder_counts(
    connection: &mut SqliteConnection,
    space_id: &str,
    root: Option<&str>,
) -> Result<Vec<(String, ZoneContents)>, StorageError> {
    let lineage = crate::folders::store::lineage(connection, space_id)?;

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

    Ok(lineage
        .children(root)
        .iter()
        .map(|id| {
            let notes = lineage
                .subtree(id)
                .iter()
                .map(|below| counts.get(below).copied().unwrap_or(0))
                .sum();
            let nested = !lineage.children(Some(id)).is_empty();
            (id.clone(), ZoneContents { notes, nested })
        })
        .collect())
}

/// The notes filed in the root itself, free on its board, in its order: pinned first, then by
/// last change.
fn loose_ids(
    connection: &mut SqliteConnection,
    space_id: &str,
    root: Option<&str>,
) -> Result<Vec<String>, StorageError> {
    let mut query = notes::table
        .filter(notes::space_id.eq(space_id))
        .filter(notes::deleted_at.is_null())
        .into_boxed();
    query = match root {
        Some(root) => query.filter(notes::folder_id.eq(root.to_string())),
        None => query.filter(notes::folder_id.is_null()),
    };

    Ok(query
        .order((
            notes::pinned.desc(),
            notes::updated_at.desc(),
            notes::id.asc(),
        ))
        .select(notes::id)
        .load::<String>(connection)?)
}

/// Rewrites one board's geometry as far as `scope` allows, and answers what it was.
///
/// It answers the previous layout: the new one arrives with the front end's reload anyway.
/// Only what had a place is reported, or an undo would invent positions nobody chose, and
/// `moved` counts what came out somewhere else, so a board already in order opens no undo.
pub fn arrange(
    connection: &mut Library,
    space_id: &str,
    root: Option<&str>,
    scope: BoardScope,
) -> Result<BoardArrangement, StorageError> {
    connection.transaction(|connection, _vault| {
        let loose = loose_ids(connection, space_id, root)?;
        let zones = folder_counts(connection, space_id, root)?;
        let mut before = frames(connection, space_id)?;
        before.retain(|id, _| zones.iter().any(|(zone, _)| zone == id));
        let mut places = positions(connection, space_id)?;
        places.retain(|id, _| loose.contains(id));

        let next = match scope {
            BoardScope::Everything => board::arrange_everything(&zones, &loose),
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
