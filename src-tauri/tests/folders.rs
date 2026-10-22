//! The folder as data: a region of a space that notes are filed into, and that never
//! takes a note with it when it goes.

use std::collections::HashMap;

use devnotes_lib::db::{Library, iso8601, open_in_memory};
use devnotes_lib::error::StorageError;
use devnotes_lib::folders::model::{FolderColour, Lineage, NoteFiling};
use devnotes_lib::folders::store::{
    create, delete, file_many, list, recolour, rename, restore_filings,
};
use devnotes_lib::notes::kind::NoteKind;
use devnotes_lib::notes::language::Language;
use devnotes_lib::notes::model::{Note, NoteDraft, NotePatch};
use devnotes_lib::notes::priority::Priority;
use devnotes_lib::notes::store::{create as create_note, update};
use devnotes_lib::spaces::store as spaces;

mod common;
use common::{at, snippet as draft, space, t0, t1};

fn note_in(connection: &mut Library, space_id: &str) -> Note {
    create_note(connection, draft(space_id), t0()).unwrap()
}

fn folder_of(connection: &mut Library, note_id: &str) -> Option<String> {
    use devnotes_lib::db::schema::notes;
    use diesel::prelude::*;

    notes::table
        .find(note_id)
        .select(notes::folder_id)
        .first::<Option<String>>(connection.db())
        .unwrap()
}

fn updated_at_of(connection: &mut Library, note_id: &str) -> String {
    use devnotes_lib::db::schema::notes;
    use diesel::prelude::*;

    notes::table
        .find(note_id)
        .select(notes::updated_at)
        .first::<String>(connection.db())
        .unwrap()
}

fn names(connection: &mut Library, space_id: &str) -> Vec<String> {
    list(connection, Some(space_id))
        .unwrap()
        .into_iter()
        .map(|folder| folder.name)
        .collect()
}

#[test]
fn a_created_folder_is_listed_back_inside_its_space() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let other = space(&mut connection, "Veille");

    let created = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    create(&mut connection, &other, None, "Liens", t0()).unwrap();

    let listed = list(&mut connection, Some(&sql)).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
    assert_eq!(listed[0].name, "Perf");
    assert_eq!(listed[0].space_id, sql);
}

/// Reading order, which is the order the board lays its zones out in — and which a
/// rename must not reshuffle.
#[test]
fn folders_come_back_in_the_order_they_were_made() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");

    create(
        &mut connection,
        &sql,
        None,
        "Migrations",
        at("2026-07-25T09:00:00.000Z"),
    )
    .unwrap();
    create(
        &mut connection,
        &sql,
        None,
        "Perf",
        at("2026-07-25T09:30:00.000Z"),
    )
    .unwrap();
    let reporting = create(
        &mut connection,
        &sql,
        None,
        "Reporting",
        at("2026-07-25T10:00:00.000Z"),
    )
    .unwrap();

    rename(&mut connection, &reporting.id, "Analytics").unwrap();

    assert_eq!(
        names(&mut connection, &sql),
        ["Migrations", "Perf", "Analytics"]
    );
}

/// Assigned rather than chosen: drawing a folder stays one gesture, and two made back
/// to back must not come out the same colour.
#[test]
fn the_palette_rotates_so_neighbours_differ() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");

    let first = create(&mut connection, &sql, None, "Migrations", t0()).unwrap();
    let second = create(&mut connection, &sql, None, "Perf", t0()).unwrap();

    assert_ne!(first.colour, second.colour);
    assert_eq!(first.colour, FolderColour::nth(0));
    assert_eq!(second.colour, FolderColour::nth(1));
}

/// Each space starts the rotation over: its own board is what a colour tells apart.
#[test]
fn the_rotation_is_counted_inside_one_space() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let veille = space(&mut connection, "Veille");

    create(&mut connection, &sql, None, "Migrations", t0()).unwrap();
    create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let elsewhere = create(&mut connection, &veille, None, "Liens", t0()).unwrap();

    assert_eq!(elsewhere.colour, FolderColour::nth(0));
}

#[test]
fn a_folder_can_be_given_another_colour() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let folder = create(&mut connection, &sql, None, "Perf", t0()).unwrap();

    let recoloured = recolour(&mut connection, &folder.id, FolderColour::Red).unwrap();

    assert_eq!(recoloured.colour, FolderColour::Red);
    assert_eq!(
        list(&mut connection, Some(&sql)).unwrap()[0].colour,
        FolderColour::Red
    );
}

#[test]
fn two_folders_of_one_space_cannot_share_a_name_whatever_the_case() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    create(&mut connection, &sql, None, "Perf", t0()).unwrap();

    let refused = create(&mut connection, &sql, None, "perf", t0());

    assert!(matches!(refused, Err(StorageError::DuplicateFolderName(_))));
    assert_eq!(names(&mut connection, &sql).len(), 1);
}

/// Uniqueness is per space, not per library: two spaces both wanting a "Perf" is the
/// normal case, not a collision.
#[test]
fn the_same_name_is_free_again_in_another_space() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let veille = space(&mut connection, "Veille");
    create(&mut connection, &sql, None, "Perf", t0()).unwrap();

    assert!(create(&mut connection, &veille, None, "Perf", t0()).is_ok());
}

#[test]
fn correcting_the_case_of_a_name_is_not_a_duplicate_of_itself() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let folder = create(&mut connection, &sql, None, "perf", t0()).unwrap();

    let renamed = rename(&mut connection, &folder.id, "Perf").unwrap();

    assert_eq!(renamed.name, "Perf");
}

#[test]
fn a_folder_of_an_unknown_space_is_refused() {
    let mut connection = open_in_memory().unwrap();

    let refused = create(&mut connection, "nowhere", None, "Perf", t0());

    assert!(matches!(refused, Err(StorageError::SpaceNotFound(_))));
}

#[test]
fn filing_a_note_answers_where_it_came_from() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);

    let filed = file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    assert_eq!(
        filed,
        [NoteFiling {
            note_id: note.id.clone(),
            folder_id: None
        }]
    );
    assert_eq!(folder_of(&mut connection, &note.id), Some(perf.id));
}

#[test]
fn filing_with_no_folder_takes_a_note_back_out() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    let unfiled = file_many(&mut connection, std::slice::from_ref(&note.id), None, t1()).unwrap();

    assert_eq!(unfiled[0].folder_id, Some(perf.id));
    assert_eq!(folder_of(&mut connection, &note.id), None);
}

/// A batch reports what it actually changed, so the undo cannot unfile a note the batch
/// never touched.
#[test]
fn a_note_already_in_the_folder_is_not_reported_as_filed() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let already = note_in(&mut connection, &sql);
    let moving = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&already.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    let filed = file_many(
        &mut connection,
        &[already.id.clone(), moving.id.clone()],
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    assert_eq!(filed.len(), 1);
    assert_eq!(filed[0].note_id, moving.id);
}

/// A folder belongs to one space, so filing across spaces would show a chip the space
/// switcher can never reach.
#[test]
fn a_note_of_another_space_is_left_where_it_is() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let veille = space(&mut connection, "Veille");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let elsewhere = note_in(&mut connection, &veille);

    let filed = file_many(
        &mut connection,
        std::slice::from_ref(&elsewhere.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    assert!(filed.is_empty());
    assert_eq!(folder_of(&mut connection, &elsewhere.id), None);
}

#[test]
fn undoing_a_filing_puts_every_note_back_where_it_was() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let migrations = create(&mut connection, &sql, None, "Migrations", t0()).unwrap();
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let loose = note_in(&mut connection, &sql);
    let filed = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&filed.id),
        Some(&migrations.id),
        t1(),
    )
    .unwrap();

    let batch = file_many(
        &mut connection,
        &[loose.id.clone(), filed.id.clone()],
        Some(&perf.id),
        t1(),
    )
    .unwrap();
    restore_filings(&mut connection, &batch).unwrap();

    assert_eq!(folder_of(&mut connection, &loose.id), None);
    assert_eq!(folder_of(&mut connection, &filed.id), Some(migrations.id));
}

/// Undoing is not editing, and the canvas sorts on that column.
#[test]
fn undoing_a_filing_leaves_the_instant_alone() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);

    let batch = file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();
    restore_filings(&mut connection, &batch).unwrap();

    assert_eq!(
        updated_at_of(&mut connection, &note.id),
        iso8601::format(t1())
    );
}

/// The one thing a folder must never do.
#[test]
fn deleting_a_folder_leaves_its_notes_loose_rather_than_deleting_them() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    delete(&mut connection, &perf.id).unwrap();

    assert!(list(&mut connection, Some(&sql)).unwrap().is_empty());
    assert_eq!(folder_of(&mut connection, &note.id), None);
}

#[test]
fn deleting_an_unknown_folder_says_so_rather_than_answering_ok() {
    let mut connection = open_in_memory().unwrap();

    assert!(matches!(
        delete(&mut connection, "nowhere"),
        Err(StorageError::FolderNotFound(_))
    ));
}

/// The folders go with their space through the cascade; their notes went to the refuge
/// first, and come out of it loose.
#[test]
fn deleting_a_space_takes_its_folders_and_unfiles_the_notes_it_hands_over() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let refuge = space(&mut connection, "Veille");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    spaces::delete(&mut connection, &sql, &refuge).unwrap();

    assert!(list(&mut connection, Some(&sql)).unwrap().is_empty());
    assert_eq!(folder_of(&mut connection, &note.id), None);
}

/// The chip would otherwise name a folder the space switcher can never reach.
#[test]
fn moving_a_note_to_another_space_takes_it_out_of_its_folder() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let veille = space(&mut connection, "Veille");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    let patch = NotePatch {
        space_id: Some(veille.clone()),
        ..NotePatch::default()
    };
    update(&mut connection, &note.id, &patch, t1()).unwrap();

    assert_eq!(folder_of(&mut connection, &note.id), None);
}

/// Editing a note is not refiling it.
#[test]
fn a_patch_that_says_nothing_about_the_space_leaves_the_folder_alone() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
    let note = note_in(&mut connection, &sql);
    file_many(
        &mut connection,
        std::slice::from_ref(&note.id),
        Some(&perf.id),
        t1(),
    )
    .unwrap();

    let patch = NotePatch {
        title: Some("Autre".to_string()),
        ..NotePatch::default()
    };
    update(&mut connection, &note.id, &patch, t1()).unwrap();

    assert_eq!(folder_of(&mut connection, &note.id), Some(perf.id));
}

/// A note created inside a folder arrives already filed — one of the two places that do.
#[test]
fn a_draft_can_name_the_folder_it_is_born_in() {
    let mut connection = open_in_memory().unwrap();
    let sql = space(&mut connection, "SQL");
    let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();

    let born = create_note(
        &mut connection,
        NoteDraft {
            folder_id: Some(perf.id.clone()),
            ..draft(&sql)
        },
        t0(),
    )
    .unwrap();

    assert_eq!(born.folder_id, Some(perf.id));
}

mod board {
    use super::*;

    use devnotes_lib::folders::board::{self, BoardFrame, BoardPoint, BoardQuery, BoardView};
    use devnotes_lib::folders::store::board as geometry;
    use devnotes_lib::notes::store::fetch;
    use devnotes_lib::notes::view::{Grouping, NoteFilter, NoteOrder, NotesQuery};

    pub(super) fn request(space_id: &str) -> BoardQuery {
        BoardQuery {
            space_id: space_id.to_string(),
            root_id: None,
            search: String::new(),
            filter: NoteFilter::All,
            tags: Vec::new(),
            languages: Vec::new(),
            kinds: Vec::new(),
            priorities: Vec::new(),
            now: t1(),
        }
    }

    /// What `board_view` does, minus the decoration passes that need a connection.
    pub(super) fn view(connection: &mut Library, query: &BoardQuery) -> BoardView {
        let folders = list(connection, Some(&query.space_id)).unwrap();
        let (notes, facets) = fetch(
            connection,
            &NotesQuery {
                space_id: Some(query.space_id.clone()),
                folder_id: None,
                search: String::new(),
                filter: NoteFilter::All,
                tags: Vec::new(),
                languages: Vec::new(),
                kinds: Vec::new(),
                priorities: Vec::new(),
                order: NoteOrder::default(),
                grouping: Grouping::default(),
                ranked: false,
                now: query.now,
                tz_offset_minutes: 0,
                pinned_first: true,
            },
        )
        .unwrap();

        let lineage = Lineage::of(&folders);
        let root = query.root_id.as_deref();
        let zones: Vec<(String, bool)> = lineage
            .children(root)
            .iter()
            .map(|id| (id.clone(), !lineage.children(Some(id)).is_empty()))
            .collect();
        let (frames, positions) = geometry::geometry(
            connection,
            &query.space_id,
            &zones,
            &board::Occupancy::on(&notes, &lineage, root),
        )
        .unwrap();

        board::build(notes, &folders, &frames, &positions, facets, query)
    }

    pub(super) fn titled(connection: &mut Library, space_id: &str, title: &str) -> Note {
        create_note(
            connection,
            NoteDraft {
                title: title.to_string(),
                ..draft(space_id)
            },
            t0(),
        )
        .unwrap()
    }

    #[test]
    fn a_board_draws_every_folder_as_a_zone_holding_its_notes() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        // Distinct instants: `list` orders by `(created_at, id)`, so two folders made
        // in the same millisecond fall back to their UUIDs — stable across launches, which
        // is what the board needs, but not something a test can name.
        let migrations = create(&mut connection, &sql, None, "Migrations", t0()).unwrap();
        create(&mut connection, &sql, None, "Perf", t1()).unwrap();
        let filed = note_in(&mut connection, &sql);
        note_in(&mut connection, &sql);
        file_many(
            &mut connection,
            std::slice::from_ref(&filed.id),
            Some(&migrations.id),
            t1(),
        )
        .unwrap();

        let board = view(&mut connection, &request(&sql));

        assert_eq!(board.zones.len(), 2);
        assert_eq!(board.zones[0].folder.name, "Migrations");
        assert_eq!(board.zones[0].notes.len(), 1);
        assert!(board.zones[1].notes.is_empty());
        assert_eq!(board.loose.len(), 1);
    }

    /// A board is a list like the canvas: filed or loose, a card carries a preview.
    #[test]
    fn a_board_card_carries_the_head_of_a_long_body() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        // A snippet: a Note's preview is its words, not its lines.
        let long = || NoteDraft {
            language: Language::Sh,
            content: "1\n2\n3\n4\n5\n6\n7".to_string(),
            ..draft(&sql)
        };
        let filed = create_note(&mut connection, long(), t0()).unwrap();
        create_note(&mut connection, long(), t0()).unwrap();
        file_many(
            &mut connection,
            std::slice::from_ref(&filed.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        let mut board = view(&mut connection, &request(&sql));

        assert_eq!(board.notes_mut().count(), 2);
        assert!(
            board
                .notes_mut()
                .all(|note| note.truncated && note.content == "1\n2\n3\n4\n5")
        );
    }

    /// A filed card flows inside its zone; only a loose one carries a place of its own.
    #[test]
    fn a_filed_card_has_no_position_and_a_loose_one_does() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        let filed = note_in(&mut connection, &sql);
        note_in(&mut connection, &sql);
        file_many(
            &mut connection,
            std::slice::from_ref(&filed.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        let board = view(&mut connection, &request(&sql));

        assert!(board.zones[0].notes[0].position.is_none());
        assert!(board.loose[0].position.is_some());
    }

    /// The board would look shuffled at every launch otherwise.
    #[test]
    fn the_first_layout_is_written_down_and_read_back_unchanged() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        create(&mut connection, &sql, None, "Migrations", t0()).unwrap();
        create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        note_in(&mut connection, &sql);

        let first = view(&mut connection, &request(&sql));
        let second = view(&mut connection, &request(&sql));

        assert_eq!(
            first
                .zones
                .iter()
                .map(|zone| zone.frame)
                .collect::<Vec<_>>(),
            second
                .zones
                .iter()
                .map(|zone| zone.frame)
                .collect::<Vec<_>>(),
        );
        assert_eq!(first.loose[0].position, second.loose[0].position);
    }

    /// A frame is computed once: the gesture that files a card into a full zone is the one
    /// that makes room, or the card flows out of sight.
    #[test]
    fn filing_a_card_into_a_full_zone_makes_room_for_it() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        let filled: Vec<String> = ["a", "b"]
            .iter()
            .map(|title| titled(&mut connection, &sql, title).id)
            .collect();
        file_many(&mut connection, &filled, Some(&perf.id), t1()).unwrap();
        let before = view(&mut connection, &request(&sql)).zones[0].frame;

        let extra = titled(&mut connection, &sql, "c");
        file_many(
            &mut connection,
            std::slice::from_ref(&extra.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        let after = view(&mut connection, &request(&sql)).zones[0].frame;
        assert!(
            after.height > before.height,
            "the zone stayed {}px tall for three cards",
            before.height
        );
        assert_eq!(after.width, before.width, "the width must not move");
    }

    /// A zone slightly narrower than nominal still flows two across, so the next card filed
    /// in needs at most one more row.
    #[test]
    fn filing_into_a_zone_shaved_narrower_than_nominal_adds_one_row_and_not_two() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        view(&mut connection, &request(&sql));

        let shaved = BoardFrame {
            x: 0,
            y: 40,
            width: board::default_zone_width() - 14,
            height: board::zone_height(2, 2, false),
        };
        connection
            .transaction(|connection, _vault| geometry::set_frame(connection, &perf.id, shaved))
            .unwrap();

        let notes: Vec<String> = ["a", "b", "c"]
            .iter()
            .map(|title| titled(&mut connection, &sql, title).id)
            .collect();
        file_many(&mut connection, &notes, Some(&perf.id), t1()).unwrap();

        let after = view(&mut connection, &request(&sql)).zones[0].frame;
        assert_eq!(after.width, shaved.width, "the width must not move");
        assert_eq!(
            after.height,
            board::zone_height(3, 2, false),
            "three cards two across are two rows, not three"
        );
    }

    /// Grow only: shrinking would move the board under the pointer every time a card
    /// is taken out, and a zone somebody stretched is a zone they chose the size of.
    #[test]
    fn a_zone_made_bigger_by_hand_keeps_its_size() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        view(&mut connection, &request(&sql));

        let stretched = BoardFrame {
            x: 40,
            y: 40,
            width: 600,
            height: 900,
        };
        connection
            .transaction(|connection, _vault| geometry::set_frame(connection, &perf.id, stretched))
            .unwrap();

        let note = titled(&mut connection, &sql, "a");
        file_many(
            &mut connection,
            std::slice::from_ref(&note.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        assert_eq!(
            view(&mut connection, &request(&sql)).zones[0].frame,
            stretched
        );
    }

    /// Taking a card out leaves the room it needed: the board must not move on its own.
    #[test]
    fn unfiling_a_card_does_not_close_the_zone_again() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        let notes: Vec<String> = ["a", "b", "c"]
            .iter()
            .map(|title| titled(&mut connection, &sql, title).id)
            .collect();
        file_many(&mut connection, &notes, Some(&perf.id), t1()).unwrap();
        let grown = view(&mut connection, &request(&sql)).zones[0].frame;

        file_many(&mut connection, &notes[..1], None, t1()).unwrap();

        assert_eq!(view(&mut connection, &request(&sql)).zones[0].frame, grown);
    }

    /// Undoing puts notes back into a zone that may no longer be tall enough for them.
    #[test]
    fn undoing_a_filing_makes_room_for_what_comes_back() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        let notes: Vec<String> = ["a", "b", "c"]
            .iter()
            .map(|title| titled(&mut connection, &sql, title).id)
            .collect();
        file_many(&mut connection, &notes, Some(&perf.id), t1()).unwrap();
        let grown = view(&mut connection, &request(&sql)).zones[0].frame;

        // Squashed back down behind the zone's back, as a resize would.
        connection
            .transaction(|connection, _vault| {
                geometry::set_frame(
                    connection,
                    &perf.id,
                    BoardFrame {
                        height: 212,
                        ..grown
                    },
                )
            })
            .unwrap();
        let left = file_many(&mut connection, &notes[..1], None, t1()).unwrap();
        restore_filings(&mut connection, &left).unwrap();

        assert_eq!(
            view(&mut connection, &request(&sql)).zones[0].frame.height,
            grown.height
        );
    }

    #[test]
    fn a_stored_frame_survives_the_next_read() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        view(&mut connection, &request(&sql));

        let moved = BoardFrame {
            x: 700,
            y: 500,
            width: 300,
            height: 200,
        };
        connection
            .transaction(|connection, _vault| geometry::set_frame(connection, &perf.id, moved))
            .unwrap();

        assert_eq!(view(&mut connection, &request(&sql)).zones[0].frame, moved);
    }

    #[test]
    fn a_folder_added_later_lands_beside_the_others_rather_than_on_top() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        create(&mut connection, &sql, None, "Migrations", t0()).unwrap();
        let first = view(&mut connection, &request(&sql)).zones[0].frame;

        create(&mut connection, &sql, None, "Perf", t1()).unwrap();
        let board = view(&mut connection, &request(&sql));

        assert_eq!(board.zones[0].frame, first);
        assert_ne!(board.zones[1].frame, first);
    }

    /// Dimmed, never dropped: reflowing the survivors throws away the only thing the board
    /// has that the date view does not.
    #[test]
    fn a_search_dims_what_it_does_not_match_and_removes_nothing() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        let wanted = titled(&mut connection, &sql, "Index sur orders");
        titled(&mut connection, &sql, "Dump nocturne");
        file_many(
            &mut connection,
            std::slice::from_ref(&wanted.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        let board = view(
            &mut connection,
            &BoardQuery {
                search: "orders".to_string(),
                ..request(&sql)
            },
        );

        assert_eq!(board.zones[0].notes.len(), 1);
        assert_eq!(board.loose.len(), 1);
        assert!(board.zones[0].notes[0].matches);
        assert!(!board.loose[0].matches);
        assert_eq!(board.matched, 1);
        assert!(board.is_filtering);
    }

    /// Accents fold both ways here too: it is `notes::view`'s own matcher, not a second one.
    #[test]
    fn the_board_search_folds_accents_like_the_canvas_does() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        titled(&mut connection, &sql, "Étape suivante");

        let board = view(
            &mut connection,
            &BoardQuery {
                search: "etape".to_string(),
                ..request(&sql)
            },
        );

        assert!(board.loose[0].matches);
    }

    #[test]
    fn a_quick_filter_dims_rather_than_narrows_too() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        create_note(
            &mut connection,
            NoteDraft {
                pinned: true,
                ..draft(&sql)
            },
            t0(),
        )
        .unwrap();
        create_note(&mut connection, draft(&sql), t0()).unwrap();

        let board = view(
            &mut connection,
            &BoardQuery {
                filter: NoteFilter::Pinned,
                ..request(&sql)
            },
        );

        assert_eq!(board.loose.len(), 2);
        assert_eq!(board.matched, 1);
    }

    #[test]
    fn a_priority_dims_rather_than_narrows_too() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        create_note(&mut connection, draft(&sql), t0()).unwrap();
        create_note(
            &mut connection,
            NoteDraft {
                priority: Priority::High,
                ..draft(&sql)
            },
            t0(),
        )
        .unwrap();

        let board = view(
            &mut connection,
            &BoardQuery {
                priorities: vec![Priority::High],
                ..request(&sql)
            },
        );

        assert_eq!(board.loose.len(), 2);
        assert_eq!(board.matched, 1);
    }

    #[test]
    fn a_kind_dims_rather_than_narrows_too() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        create_note(&mut connection, draft(&sql), t0()).unwrap();
        create_note(
            &mut connection,
            NoteDraft {
                kind: NoteKind::Note,
                ..draft(&sql)
            },
            t0(),
        )
        .unwrap();

        let board = view(
            &mut connection,
            &BoardQuery {
                kinds: vec![NoteKind::Note],
                ..request(&sql)
            },
        );

        assert_eq!(board.loose.len(), 2);
        assert_eq!(board.matched, 1);
        assert!(board.is_filtering);
    }

    /// The two views answer the same question, and must not answer it differently. Tags
    /// are compared as the column compares them — ASCII case only, `#` stripped — and a
    /// tag that normalises to nothing asks for nothing.
    #[test]
    fn the_board_and_the_date_view_agree_on_what_matches() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        for (title, tags) in [
            ("accented", vec!["étape"]),
            ("capital", vec!["Étape"]),
            ("urgent", vec!["URGENT"]),
            ("plain", Vec::new()),
        ] {
            create_note(
                &mut connection,
                NoteDraft {
                    title: title.to_string(),
                    tags: tags.into_iter().map(ToString::to_string).collect(),
                    ..draft(&sql)
                },
                t0(),
            )
            .unwrap();
        }

        for asked in [vec!["étape"], vec!["#urgent"], vec![" # "], Vec::new()] {
            let tags: Vec<String> = asked.iter().map(ToString::to_string).collect();

            let query = NotesQuery {
                space_id: Some(sql.clone()),
                folder_id: None,
                search: String::new(),
                filter: NoteFilter::All,
                tags: tags.clone(),
                languages: Vec::new(),
                kinds: Vec::new(),
                priorities: Vec::new(),
                order: NoteOrder::default(),
                grouping: Grouping::default(),
                ranked: false,
                now: t1(),
                tz_offset_minutes: 0,
                pinned_first: true,
            };
            let (notes, facets) = fetch(&mut connection, &query).unwrap();
            let dated = devnotes_lib::notes::view::build(notes, facets, &query);
            let mut on_the_date_view: Vec<String> = dated
                .sections
                .iter()
                .flat_map(|section| section.notes.iter().map(|note| note.title.clone()))
                .collect();

            let board = view(
                &mut connection,
                &BoardQuery {
                    tags,
                    ..request(&sql)
                },
            );
            let mut lit_on_the_board: Vec<String> = board
                .loose
                .iter()
                .filter(|entry| entry.matches)
                .map(|entry| entry.note.title.clone())
                .collect();

            on_the_date_view.sort();
            lit_on_the_board.sort();
            assert_eq!(on_the_date_view, lit_on_the_board, "asked for {asked:?}");
            assert_eq!(
                dated.is_filtering, board.is_filtering,
                "asked for {asked:?}"
            );
        }
    }

    /// Filing a card into a zone drops the place it had on the background.
    #[test]
    fn filing_a_loose_note_forgets_where_it_sat() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        let note = note_in(&mut connection, &sql);
        view(&mut connection, &request(&sql));

        let placed = connection
            .transaction(|connection, _vault| geometry::positions(connection, &sql))
            .unwrap();
        assert!(placed.contains_key(&note.id));

        file_many(
            &mut connection,
            std::slice::from_ref(&note.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        let after = connection
            .transaction(|connection, _vault| geometry::positions(connection, &sql))
            .unwrap();
        assert!(!after.contains_key(&note.id));
    }

    #[test]
    fn a_stored_position_is_what_a_later_read_hands_back() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let note = note_in(&mut connection, &sql);
        view(&mut connection, &request(&sql));

        let chosen = BoardPoint { x: 640, y: 480 };
        connection
            .transaction(|connection, _vault| geometry::set_position(connection, &note.id, chosen))
            .unwrap();

        assert_eq!(
            view(&mut connection, &request(&sql)).loose[0].position,
            Some(chosen)
        );
    }

    /// A new note is the most recently updated, so it comes first in the list: it must not
    /// be handed seat zero, where another card already sits.
    #[test]
    fn a_note_created_later_is_placed_beside_the_cards_rather_than_on_one() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        titled(&mut connection, &sql, "Dump nocturne");
        let already_there = view(&mut connection, &request(&sql)).loose[0].position;

        create_note(
            &mut connection,
            NoteDraft {
                title: "Collé du presse-papiers".to_string(),
                ..draft(&sql)
            },
            t1(),
        )
        .unwrap();

        let board = view(&mut connection, &request(&sql));
        let places: Vec<_> = board.loose.iter().map(|entry| entry.position).collect();

        assert_eq!(places.len(), 2);
        assert!(
            places.contains(&already_there),
            "the card already there did not move"
        );
        assert_ne!(places[0], places[1], "two cards were written to one place");
    }

    /// Every unplaced card in the same pass, not just the first one.
    #[test]
    fn a_first_read_of_several_loose_notes_gives_each_its_own_place() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        for title in ["Un", "Deux", "Trois", "Quatre", "Cinq"] {
            titled(&mut connection, &sql, title);
        }

        let board = view(&mut connection, &request(&sql));
        let mut places: Vec<_> = board
            .loose
            .iter()
            .map(|entry| entry.position.expect("a loose card is always placed"))
            .map(|at| (at.x, at.y))
            .collect();
        places.sort_unstable();
        places.dedup();

        assert_eq!(places.len(), 5);
    }

    #[test]
    fn the_surface_grows_to_hold_whatever_is_furthest_out() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let note = note_in(&mut connection, &sql);
        view(&mut connection, &request(&sql));

        connection
            .transaction(|connection, _vault| {
                geometry::set_position(connection, &note.id, BoardPoint { x: 3000, y: 2000 })
            })
            .unwrap();

        let board = view(&mut connection, &request(&sql));
        assert!(board.width > 3000);
        assert!(board.height > 2000);
    }

    /// A board of one space says nothing about the next one.
    #[test]
    fn a_board_never_reaches_into_another_space() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let veille = space(&mut connection, "Veille");
        create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        create(&mut connection, &veille, None, "Liens", t0()).unwrap();
        note_in(&mut connection, &veille);

        let board = view(&mut connection, &request(&sql));

        assert_eq!(board.zones.len(), 1);
        assert!(board.loose.is_empty());
    }
}

mod gesture {
    use super::*;

    use devnotes_lib::folders::board::{
        BoardFrame, BoardPoint, CardPlacement, MIN_ZONE_HEIGHT, MIN_ZONE_WIDTH, ZonePlacement,
        clamp, clamp_point, zone_at,
    };
    use devnotes_lib::folders::store::board as geometry;

    fn frames(connection: &mut Library, space_id: &str) -> HashMap<String, BoardFrame> {
        connection
            .transaction(|connection, _vault| geometry::frames(connection, space_id))
            .unwrap()
    }

    fn positions(connection: &mut Library, space_id: &str) -> HashMap<String, BoardPoint> {
        connection
            .transaction(|connection, _vault| geometry::positions(connection, space_id))
            .unwrap()
    }

    /// Puts a zone and a card exactly where a drag would have left them.
    fn place(connection: &mut Library, zones: &[ZonePlacement], cards: &[CardPlacement]) {
        geometry::save_layout(connection, None, zones, cards).unwrap();
    }

    fn zone_at_frame(folder_id: &str, frame: BoardFrame) -> ZonePlacement {
        ZonePlacement {
            folder_id: folder_id.to_string(),
            frame,
        }
    }

    fn card_at(note_id: &str, position: BoardPoint) -> CardPlacement {
        CardPlacement {
            note_id: note_id.to_string(),
            position,
        }
    }

    #[test]
    fn a_moved_zone_and_a_moved_card_are_written_as_one_batch() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        let note = note_in(&mut connection, &sql);

        let frame = BoardFrame {
            x: 600,
            y: 320,
            width: 520,
            height: 400,
        };
        let point = BoardPoint { x: 48, y: 720 };
        geometry::save_layout(
            &mut connection,
            None,
            &[ZonePlacement {
                folder_id: perf.id.clone(),
                frame,
            }],
            &[CardPlacement {
                note_id: note.id.clone(),
                position: point,
            }],
        )
        .unwrap();

        assert_eq!(frames(&mut connection, &sql).get(&perf.id), Some(&frame));
        assert_eq!(positions(&mut connection, &sql).get(&note.id), Some(&point));
    }

    /// A zone nothing can be dropped into is not a zone. Clamped rather than refused:
    /// answering an error mid-gesture leaves the interface holding a frame nothing stored.
    #[test]
    fn a_zone_squashed_to_nothing_keeps_room_for_one_card() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();

        geometry::save_layout(
            &mut connection,
            None,
            &[ZonePlacement {
                folder_id: perf.id.clone(),
                frame: BoardFrame {
                    x: -50,
                    y: -50,
                    width: 4,
                    height: 4,
                },
            }],
            &[],
        )
        .unwrap();

        let stored = frames(&mut connection, &sql);
        let frame = stored.get(&perf.id).unwrap();
        assert_eq!(frame.x, 0);
        assert_eq!(frame.y, 0);
        assert_eq!(frame.width, MIN_ZONE_WIDTH);
        assert_eq!(frame.height, MIN_ZONE_HEIGHT);
    }

    /// A filed card flows inside its zone. Writing a position for one would put back the
    /// row `file_many` had just dropped, and the table would stop meaning "this is loose".
    #[test]
    fn a_card_filed_since_the_drag_is_not_given_a_position_back() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        let note = note_in(&mut connection, &sql);
        file_many(
            &mut connection,
            std::slice::from_ref(&note.id),
            Some(&perf.id),
            t1(),
        )
        .unwrap();

        geometry::save_layout(
            &mut connection,
            None,
            &[],
            &[CardPlacement {
                note_id: note.id.clone(),
                position: BoardPoint { x: 10, y: 10 },
            }],
        )
        .unwrap();

        assert!(!positions(&mut connection, &sql).contains_key(&note.id));
    }

    /// Every zone or none: half a board is a board nobody arranged.
    #[test]
    fn a_batch_naming_a_folder_that_is_gone_writes_nothing_at_all() {
        let mut connection = open_in_memory().unwrap();
        let sql = space(&mut connection, "SQL");
        let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
        let note = note_in(&mut connection, &sql);

        let refused = geometry::save_layout(
            &mut connection,
            None,
            &[
                ZonePlacement {
                    folder_id: perf.id.clone(),
                    frame: BoardFrame {
                        x: 600,
                        y: 320,
                        width: 520,
                        height: 400,
                    },
                },
                ZonePlacement {
                    folder_id: "gone".to_string(),
                    frame: BoardFrame {
                        x: 0,
                        y: 0,
                        width: 520,
                        height: 400,
                    },
                },
            ],
            &[CardPlacement {
                note_id: note.id.clone(),
                position: BoardPoint { x: 48, y: 720 },
            }],
        );

        assert!(matches!(refused, Err(StorageError::FolderNotFound(_))));
        assert!(frames(&mut connection, &sql).is_empty());
        assert!(positions(&mut connection, &sql).is_empty());
    }

    #[test]
    fn an_empty_batch_is_not_an_error() {
        let mut connection = open_in_memory().unwrap();

        assert!(geometry::save_layout(&mut connection, None, &[], &[]).is_ok());
    }

    /// Membership comes from the drop, and from nothing else.
    #[test]
    fn a_point_names_the_zone_it_falls_in() {
        let zones = vec![
            (
                "a".to_string(),
                BoardFrame {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                },
            ),
            (
                "b".to_string(),
                BoardFrame {
                    x: 200,
                    y: 0,
                    width: 100,
                    height: 100,
                },
            ),
        ];

        assert_eq!(
            zone_at(&zones, BoardPoint { x: 50, y: 50 }),
            Some("a".into())
        );
        assert_eq!(
            zone_at(&zones, BoardPoint { x: 250, y: 50 }),
            Some("b".into())
        );
    }

    /// The free background is a legitimate answer: dropping there unfiles the note.
    #[test]
    fn a_point_on_the_background_names_no_zone() {
        let zones = vec![(
            "a".to_string(),
            BoardFrame {
                x: 0,
                y: 0,
                width: 100,
                height: 100,
            },
        )];

        assert_eq!(zone_at(&zones, BoardPoint { x: 150, y: 50 }), None);
        // The far edges belong to the next zone along, not to this one.
        assert_eq!(zone_at(&zones, BoardPoint { x: 100, y: 50 }), None);
        assert_eq!(zone_at(&zones, BoardPoint { x: 0, y: 0 }), Some("a".into()));
    }

    /// Drawn later means drawn on top, so it is what a drop lands in.
    #[test]
    fn overlapping_zones_hand_the_drop_to_the_topmost() {
        let zones = vec![
            (
                "under".to_string(),
                BoardFrame {
                    x: 0,
                    y: 0,
                    width: 200,
                    height: 200,
                },
            ),
            (
                "over".to_string(),
                BoardFrame {
                    x: 50,
                    y: 50,
                    width: 100,
                    height: 100,
                },
            ),
        ];

        assert_eq!(
            zone_at(&zones, BoardPoint { x: 100, y: 100 }),
            Some("over".into())
        );
    }

    #[test]
    fn a_runaway_drag_cannot_put_anything_off_the_board() {
        assert_eq!(clamp_point(BoardPoint { x: -40, y: -40 }).x, 0);
        assert_eq!(
            clamp(BoardFrame {
                x: 10,
                y: 10,
                width: 1_000_000,
                height: 10
            })
            .width,
            devnotes_lib::folders::board::MAX_SIDE
        );
    }

    /// The one board gesture that cannot be walked back by dragging: it moves everything at
    /// once and overwrites sizes chosen by hand.
    mod tidy_up {
        use super::*;

        use devnotes_lib::folders::board::{BoardScope, default_zone_height, default_zone_width};

        const DRAGGED: BoardFrame = BoardFrame {
            x: 900,
            y: 640,
            width: 900,
            height: 700,
        };

        #[test]
        fn a_zone_dragged_away_comes_back_to_its_seat() {
            let mut connection = open_in_memory().unwrap();
            let sql = space(&mut connection, "SQL");
            let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
            place(&mut connection, &[zone_at_frame(&perf.id, DRAGGED)], &[]);

            geometry::arrange(&mut connection, &sql, None, BoardScope::Everything).unwrap();

            let frame = *frames(&mut connection, &sql).get(&perf.id).unwrap();
            assert_eq!(frame.x, 16);
            assert_eq!(frame.y, 16);
            assert_eq!(frame.width, default_zone_width());
            assert_eq!(frame.height, default_zone_height(0, false));
        }

        /// The half that makes it a tidy-up: a zone too small for what it holds is reopened.
        #[test]
        fn a_zone_is_resized_to_what_it_actually_holds() {
            let mut connection = open_in_memory().unwrap();
            let sql = space(&mut connection, "SQL");
            let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
            let notes: Vec<String> = (0..5).map(|_| note_in(&mut connection, &sql).id).collect();
            file_many(&mut connection, &notes, Some(&perf.id), t1()).unwrap();

            geometry::arrange(&mut connection, &sql, None, BoardScope::Everything).unwrap();

            let frame = *frames(&mut connection, &sql).get(&perf.id).unwrap();
            assert_eq!(frame.height, default_zone_height(5, false));
        }

        #[test]
        fn the_loose_cards_flow_under_the_zones() {
            let mut connection = open_in_memory().unwrap();
            let sql = space(&mut connection, "SQL");
            let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
            let filed = note_in(&mut connection, &sql).id;
            file_many(&mut connection, &[filed], Some(&perf.id), t1()).unwrap();
            let loose = note_in(&mut connection, &sql).id;

            geometry::arrange(&mut connection, &sql, None, BoardScope::Everything).unwrap();

            let zone = *frames(&mut connection, &sql).get(&perf.id).unwrap();
            let at = *positions(&mut connection, &sql).get(&loose).unwrap();
            assert!(at.y > zone.y + zone.height);
        }

        /// What the undo reads. Restoring the answer has to put the board back exactly as
        /// it was, or the tidy-up is a one-way gesture.
        #[test]
        fn it_answers_the_layout_it_replaced() {
            let mut connection = open_in_memory().unwrap();
            let sql = space(&mut connection, "SQL");
            let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
            let note = note_in(&mut connection, &sql);
            let seat = BoardPoint { x: 96, y: 900 };
            place(
                &mut connection,
                &[zone_at_frame(&perf.id, DRAGGED)],
                &[card_at(&note.id, seat)],
            );

            let before = geometry::arrange(&mut connection, &sql, None, BoardScope::Everything)
                .unwrap()
                .previous;
            place(&mut connection, &before.zones, &before.cards);

            assert_eq!(frames(&mut connection, &sql).get(&perf.id), Some(&DRAGGED));
            assert_eq!(positions(&mut connection, &sql).get(&note.id), Some(&seat));
        }

        /// A folder the board has never drawn had no place to go back to, and inventing
        /// one on the undo would leave it somewhere nobody chose.
        #[test]
        fn a_zone_that_never_had_a_place_is_not_offered_back() {
            let mut connection = open_in_memory().unwrap();
            let sql = space(&mut connection, "SQL");
            create(&mut connection, &sql, None, "Perf", t0()).unwrap();
            note_in(&mut connection, &sql);

            let before =
                geometry::arrange(&mut connection, &sql, None, BoardScope::Everything).unwrap();

            assert!(before.previous.zones.is_empty());
            assert!(before.previous.cards.is_empty());
        }

        /// The half worth a single click: what goes to pieces is the cards outside the
        /// zones, and a zone somebody sized by hand is the only manual work the board holds.
        #[test]
        fn aligning_the_loose_cards_leaves_every_zone_exactly_where_it_was() {
            let mut connection = open_in_memory().unwrap();
            let sql = space(&mut connection, "SQL");
            let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
            let note = note_in(&mut connection, &sql);
            place(
                &mut connection,
                &[zone_at_frame(&perf.id, DRAGGED)],
                &[card_at(&note.id, BoardPoint { x: 640, y: 1200 })],
            );

            let done =
                geometry::arrange(&mut connection, &sql, None, BoardScope::LooseCards).unwrap();

            assert_eq!(frames(&mut connection, &sql).get(&perf.id), Some(&DRAGGED));
            assert!(done.previous.zones.is_empty());
            assert_eq!(done.moved, 1);
        }

        /// The cards flow under the zones **as they stand**, not under where a full
        /// reorganisation would have put them.
        #[test]
        fn the_aligned_cards_clear_the_zones_where_they_actually_are() {
            let mut connection = open_in_memory().unwrap();
            let sql = space(&mut connection, "SQL");
            let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
            let note = note_in(&mut connection, &sql);
            place(&mut connection, &[zone_at_frame(&perf.id, DRAGGED)], &[]);

            geometry::arrange(&mut connection, &sql, None, BoardScope::LooseCards).unwrap();

            let at = *positions(&mut connection, &sql).get(&note.id).unwrap();
            assert!(at.y > DRAGGED.y + DRAGGED.height);
        }

        /// What the undo bar reads. A board nobody disturbed must open no undo window,
        /// or the bar offers to put back an arrangement that never changed.
        #[test]
        fn a_board_already_in_order_reports_nothing_moved() {
            let mut connection = open_in_memory().unwrap();
            let sql = space(&mut connection, "SQL");
            create(&mut connection, &sql, None, "Perf", t0()).unwrap();
            note_in(&mut connection, &sql);
            geometry::arrange(&mut connection, &sql, None, BoardScope::Everything).unwrap();

            let again =
                geometry::arrange(&mut connection, &sql, None, BoardScope::Everything).unwrap();

            assert_eq!(again.moved, 0);
        }

        #[test]
        fn a_trashed_note_takes_no_room_in_the_zone_it_was_in() {
            use devnotes_lib::notes::store::trash::trash_many;

            let mut connection = open_in_memory().unwrap();
            let sql = space(&mut connection, "SQL");
            let perf = create(&mut connection, &sql, None, "Perf", t0()).unwrap();
            let notes: Vec<String> = (0..4).map(|_| note_in(&mut connection, &sql).id).collect();
            file_many(&mut connection, &notes, Some(&perf.id), t1()).unwrap();
            trash_many(&mut connection, &notes[..3], t1()).unwrap();

            geometry::arrange(&mut connection, &sql, None, BoardScope::Everything).unwrap();

            let frame = *frames(&mut connection, &sql).get(&perf.id).unwrap();
            assert_eq!(frame.height, default_zone_height(1, false));
        }
    }
}

/// Folders inside folders: one tree per space, a board per level.
mod nesting {
    use super::board::{request, titled, view};
    use super::*;

    use devnotes_lib::folders::board::{BoardFrame, BoardPoint, CardPlacement, ZonePlacement};
    use devnotes_lib::folders::model::Folder;
    use devnotes_lib::folders::store::board as geometry;
    use devnotes_lib::folders::store::{move_back, move_to, subtree};
    use devnotes_lib::notes::store::fetch;
    use devnotes_lib::notes::view::{Grouping, NoteFilter, NoteOrder, NotesQuery};

    /// `SQL › Perf › Index` and `SQL › Migrations`, and `Angular` beside them.
    struct Tree {
        space: String,
        sql: Folder,
        perf: Folder,
        index: Folder,
        migrations: Folder,
        angular: Folder,
    }

    fn tree(connection: &mut Library) -> Tree {
        // A millisecond apart: folders made in one instant would come back in the order of
        // their ids.
        let after = |ms: i64| t0() + chrono::TimeDelta::milliseconds(ms);
        let space = space(connection, "Découverte");
        let sql = create(connection, &space, None, "SQL", after(0)).unwrap();
        let perf = create(connection, &space, Some(&sql.id), "Perf", after(1)).unwrap();
        let index = create(connection, &space, Some(&perf.id), "Index", after(2)).unwrap();
        let migrations = create(connection, &space, Some(&sql.id), "Migrations", after(3)).unwrap();
        let angular = create(connection, &space, None, "Angular", after(4)).unwrap();
        Tree {
            space,
            sql,
            perf,
            index,
            migrations,
            angular,
        }
    }

    fn filed(connection: &mut Library, tree: &Tree, title: &str, folder: &Folder) -> Note {
        let note = titled(connection, &tree.space, title);
        file_many(
            connection,
            std::slice::from_ref(&note.id),
            Some(&folder.id),
            t1(),
        )
        .unwrap();
        note
    }

    fn parent_of(connection: &mut Library, id: &str) -> Option<String> {
        list(connection, None)
            .unwrap()
            .into_iter()
            .find(|folder| folder.id == id)
            .and_then(|folder| folder.parent_id)
    }

    fn frame_of(connection: &mut Library, id: &str) -> Option<BoardFrame> {
        connection
            .transaction(|connection, _vault| geometry::frame_of(connection, id))
            .unwrap()
    }

    #[test]
    fn a_sub_tree_holds_the_folder_and_everything_under_it() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);

        let mut held = subtree(connection.db(), &tree.sql.id).unwrap();
        held.sort();
        let mut expected = vec![
            tree.sql.id.clone(),
            tree.perf.id.clone(),
            tree.index.id.clone(),
            tree.migrations.id.clone(),
        ];
        expected.sort();

        assert_eq!(held, expected);
        assert_eq!(
            parent_of(&mut connection, &tree.index.id),
            Some(tree.perf.id)
        );
    }

    /// Two folders may each hold a « Perf », as two spaces may; two siblings may not.
    #[test]
    fn a_name_is_unique_among_siblings_only() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);

        assert!(
            create(
                &mut connection,
                &tree.space,
                Some(&tree.angular.id),
                "Perf",
                t0()
            )
            .is_ok()
        );
        assert!(matches!(
            create(
                &mut connection,
                &tree.space,
                Some(&tree.sql.id),
                "perf",
                t0()
            ),
            Err(StorageError::DuplicateFolderName(_))
        ));
        let at_top = create(&mut connection, &tree.space, None, "Perf", t0()).unwrap();
        assert!(matches!(
            move_to(&mut connection, &at_top.id, Some(&tree.sql.id)),
            Err(StorageError::DuplicateFolderName(_))
        ));
    }

    #[test]
    fn a_folder_never_moves_into_itself_nor_across_spaces() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        let other = space(&mut connection, "Ailleurs");
        let elsewhere = create(&mut connection, &other, None, "Ops", t0()).unwrap();

        assert!(matches!(
            move_to(&mut connection, &tree.sql.id, Some(&tree.index.id)),
            Err(StorageError::Invalid(_))
        ));
        assert!(matches!(
            move_to(&mut connection, &tree.sql.id, Some(&tree.sql.id)),
            Err(StorageError::Invalid(_))
        ));
        assert!(matches!(
            move_to(&mut connection, &tree.perf.id, Some(&elsewhere.id)),
            Err(StorageError::Invalid(_))
        ));
        assert!(matches!(
            create(
                &mut connection,
                &tree.space,
                Some(&elsewhere.id),
                "Ici",
                t0()
            ),
            Err(StorageError::Invalid(_))
        ));
    }

    /// Its frame belongs to the board it left: it takes a free slot on the new one, and the
    /// undo gives it both back.
    #[test]
    fn a_moved_folder_takes_what_it_holds_and_leaves_its_frame_behind() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        let note = filed(&mut connection, &tree, "EXPLAIN", &tree.index);
        view(&mut connection, &request(&tree.space));
        let mut inside = request(&tree.space);
        inside.root_id = Some(tree.sql.id.clone());
        view(&mut connection, &inside);
        let before = frame_of(&mut connection, &tree.perf.id).unwrap();

        let was = move_to(&mut connection, &tree.perf.id, Some(&tree.angular.id)).unwrap();

        assert_eq!(was.parent_id.as_deref(), Some(tree.sql.id.as_str()));
        assert_eq!(was.frame, Some(before));
        assert_eq!(
            parent_of(&mut connection, &tree.perf.id),
            Some(tree.angular.id.clone())
        );
        assert_eq!(frame_of(&mut connection, &tree.perf.id), None);
        assert_eq!(
            parent_of(&mut connection, &tree.index.id),
            Some(tree.perf.id.clone())
        );
        assert_eq!(
            folder_of(&mut connection, &note.id),
            Some(tree.index.id.clone())
        );

        assert!(move_back(&mut connection, &was).unwrap());
        assert_eq!(
            parent_of(&mut connection, &tree.perf.id),
            Some(tree.sql.id.clone())
        );
        assert_eq!(frame_of(&mut connection, &tree.perf.id), Some(before));
    }

    #[test]
    fn an_undo_whose_parent_is_gone_leaves_the_folder_where_it_is() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        let was = move_to(&mut connection, &tree.migrations.id, None).unwrap();
        delete(&mut connection, &tree.sql.id).unwrap();

        assert!(!move_back(&mut connection, &was).unwrap());
        assert_eq!(parent_of(&mut connection, &tree.migrations.id), None);
    }

    /// Nothing is lost, and nothing lands out of reach: the parent takes what it held.
    #[test]
    fn deleting_a_folder_hands_its_notes_and_sub_folders_to_its_parent() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        let held = filed(&mut connection, &tree, "EXPLAIN", &tree.perf);
        let below = filed(&mut connection, &tree, "Index partiel", &tree.index);

        delete(&mut connection, &tree.perf.id).unwrap();

        assert_eq!(
            folder_of(&mut connection, &held.id),
            Some(tree.sql.id.clone())
        );
        assert_eq!(
            folder_of(&mut connection, &below.id),
            Some(tree.index.id.clone())
        );
        assert_eq!(
            parent_of(&mut connection, &tree.index.id),
            Some(tree.sql.id.clone())
        );
    }

    #[test]
    fn deleting_a_folder_at_the_top_leaves_its_notes_loose_and_its_children_at_the_top() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        let held = filed(&mut connection, &tree, "Requête", &tree.sql);

        delete(&mut connection, &tree.sql.id).unwrap();

        assert_eq!(folder_of(&mut connection, &held.id), None);
        assert_eq!(parent_of(&mut connection, &tree.perf.id), None);
        assert_eq!(parent_of(&mut connection, &tree.migrations.id), None);
    }

    /// The space's board shows its top folders; each gathers its whole sub-tree.
    #[test]
    fn a_zone_gathers_the_notes_of_everything_under_it() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        filed(&mut connection, &tree, "Requête", &tree.sql);
        filed(&mut connection, &tree, "EXPLAIN", &tree.perf);
        filed(&mut connection, &tree, "Index partiel", &tree.index);
        titled(&mut connection, &tree.space, "Loose");

        let board = view(&mut connection, &request(&tree.space));

        let zones: Vec<&str> = board
            .zones
            .iter()
            .map(|zone| zone.folder.name.as_str())
            .collect();
        assert_eq!(zones, ["SQL", "Angular"]);
        assert_eq!(board.zones[0].notes.len(), 3);
        let children: Vec<(&str, u32)> = board.zones[0]
            .children
            .iter()
            .map(|child| (child.name.as_str(), child.count))
            .collect();
        assert_eq!(children, [("Migrations", 0), ("Perf", 2)]);
        assert!(board.zones[1].children.is_empty());
        assert_eq!(board.loose.len(), 1);
    }

    /// A folder's board: its sub-folders as zones, what is filed in it free on its ground, and
    /// nothing from outside it.
    #[test]
    fn a_folders_board_draws_its_sub_folders_and_its_own_notes_free() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        let own = filed(&mut connection, &tree, "Requête", &tree.sql);
        filed(&mut connection, &tree, "Index partiel", &tree.index);
        filed(&mut connection, &tree, "Composant", &tree.angular);
        titled(&mut connection, &tree.space, "Loose");
        let mut inside = request(&tree.space);
        inside.root_id = Some(tree.sql.id.clone());

        let board = view(&mut connection, &inside);

        let zones: Vec<(&str, usize)> = board
            .zones
            .iter()
            .map(|zone| (zone.folder.name.as_str(), zone.notes.len()))
            .collect();
        assert_eq!(zones, [("Perf", 1), ("Migrations", 0)]);
        assert_eq!(board.loose.len(), 1);
        assert_eq!(board.loose[0].note.id, own.id);
        assert!(board.loose[0].position.is_some());
        assert_eq!(board.matched, 2);
    }

    /// A row of chips under the title is height the cards must not pay for.
    #[test]
    fn a_zone_with_sub_folders_is_taller_by_their_row() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);

        let board = view(&mut connection, &request(&tree.space));

        assert_eq!(
            board.zones[0].frame.height - board.zones[1].frame.height,
            devnotes_lib::folders::board::ZONE_SUBFOLDERS
        );
    }

    /// Filed three levels down, it shows in its zone on every board above it.
    #[test]
    fn filing_deep_makes_room_in_every_zone_above_it() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        view(&mut connection, &request(&tree.space));
        let top = frame_of(&mut connection, &tree.sql.id).unwrap();

        let notes: Vec<String> = (0..5)
            .map(|index| titled(&mut connection, &tree.space, &format!("n{index}")).id)
            .collect();
        file_many(&mut connection, &notes, Some(&tree.index.id), t1()).unwrap();

        assert!(frame_of(&mut connection, &tree.sql.id).unwrap().height > top.height);
    }

    /// A card's place is on its own folder's board: saved from another board, it is refused.
    #[test]
    fn a_place_is_kept_only_on_the_board_of_the_cards_own_folder() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        let own = filed(&mut connection, &tree, "Requête", &tree.sql);
        let deeper = filed(&mut connection, &tree, "EXPLAIN", &tree.perf);
        let place = BoardPoint { x: 640, y: 480 };
        let cards = [own.id.clone(), deeper.id.clone()].map(|note_id| CardPlacement {
            note_id,
            position: place,
        });

        geometry::save_layout(&mut connection, Some(&tree.sql.id), &[], &cards).unwrap();

        let placed = connection
            .transaction(|connection, _vault| geometry::positions(connection, &tree.space))
            .unwrap();
        assert_eq!(placed.get(&own.id), Some(&place));
        assert!(!placed.contains_key(&deeper.id));
    }

    /// Filed elsewhere, a card leaves its place behind: it means nothing on another board.
    #[test]
    fn filing_a_note_forgets_its_place() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        let note = titled(&mut connection, &tree.space, "Loose");
        view(&mut connection, &request(&tree.space));

        file_many(
            &mut connection,
            std::slice::from_ref(&note.id),
            Some(&tree.perf.id),
            t1(),
        )
        .unwrap();

        let placed = connection
            .transaction(|connection, _vault| geometry::positions(connection, &tree.space))
            .unwrap();
        assert!(!placed.contains_key(&note.id));
    }

    #[test]
    fn a_folders_board_is_tidied_on_its_own() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        let frame = BoardFrame {
            x: 2000,
            y: 2000,
            width: 600,
            height: 400,
        };
        geometry::save_layout(
            &mut connection,
            None,
            &[ZonePlacement {
                folder_id: tree.angular.id.clone(),
                frame,
            }],
            &[],
        )
        .unwrap();

        geometry::arrange(
            &mut connection,
            &tree.space,
            Some(&tree.sql.id),
            devnotes_lib::folders::board::BoardScope::Everything,
        )
        .unwrap();

        assert_eq!(frame_of(&mut connection, &tree.angular.id), Some(frame));
        assert!(frame_of(&mut connection, &tree.perf.id).is_some());
    }

    /// The list view's opened folder is the zone's rule: everything under it.
    #[test]
    fn an_opened_folder_lists_its_whole_sub_tree() {
        let mut connection = open_in_memory().unwrap();
        let tree = tree(&mut connection);
        filed(&mut connection, &tree, "Requête", &tree.sql);
        filed(&mut connection, &tree, "Index partiel", &tree.index);
        filed(&mut connection, &tree, "Composant", &tree.angular);

        let (notes, _) = fetch(
            &mut connection,
            &NotesQuery {
                space_id: Some(tree.space.clone()),
                folder_id: Some(tree.sql.id.clone()),
                search: String::new(),
                filter: NoteFilter::All,
                tags: Vec::new(),
                languages: Vec::new(),
                kinds: Vec::new(),
                priorities: Vec::new(),
                order: NoteOrder::default(),
                grouping: Grouping::default(),
                ranked: false,
                now: t1(),
                tz_offset_minutes: 0,
                pinned_first: true,
            },
        )
        .unwrap();

        let mut titles: Vec<String> = notes.into_iter().map(|note| note.title).collect();
        titles.sort();
        assert_eq!(titles, ["Index partiel", "Requête"]);
    }
}
