use devnotes_lib::db::Library;
use diesel::prelude::*;

use devnotes_lib::db::open_in_memory;
use devnotes_lib::db::schema::notes;
use devnotes_lib::error::StorageError;
use devnotes_lib::spaces::store::{create, delete, exists, list, rename, set_pinned};

const T0: &str = "2026-07-25T09:00:00.000Z";

/// A note written straight into the database: going through `notes::create` would
/// drag its own rules in.
fn note_in(connection: &mut Library, space_id: &str) {
    diesel::insert_into(notes::table)
        .values((
            notes::id.eq("n-1"),
            notes::space_id.eq(space_id),
            notes::title.eq("A"),
            notes::language.eq("txt"),
            notes::content.eq(""),
            notes::source.eq(""),
            notes::pinned.eq(false),
            notes::created_at.eq(T0),
            notes::updated_at.eq(T0),
            notes::lifecycle_kind.eq("permanent"),
        ))
        .execute(connection.db())
        .unwrap();
}

fn names(connection: &mut Library) -> Vec<String> {
    list(connection)
        .unwrap()
        .into_iter()
        .map(|space| space.name)
        .collect()
}

#[test]
fn a_created_space_is_listed_back() {
    let mut connection = open_in_memory().unwrap();

    let created = create(&mut connection, "Personal").unwrap();
    let listed = list(&mut connection).unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);
    assert_eq!(listed[0].name, "Personal");
}

#[test]
fn each_space_gets_its_own_identifier() {
    let mut connection = open_in_memory().unwrap();

    let first = create(&mut connection, "Personal").unwrap();
    let second = create(&mut connection, "Boulot").unwrap();

    assert_ne!(first.id, second.id);
}

#[test]
fn spaces_are_listed_in_name_order() {
    let mut connection = open_in_memory().unwrap();

    create(&mut connection, "Veille").unwrap();
    create(&mut connection, "Boulot").unwrap();
    create(&mut connection, "personal").unwrap();

    assert_eq!(names(&mut connection), ["Boulot", "personal", "Veille"]);
}

/// Name order alone put the space opened every morning wherever its initial fell,
/// under archives nobody touches. Pinning is the gesture the app already has for
/// "keep this within reach", and it hoists here the way it does on the canvas.
#[test]
fn a_pinned_space_comes_first_whatever_its_name() {
    let mut connection = open_in_memory().unwrap();
    create(&mut connection, "Boulot").unwrap();
    let veille = create(&mut connection, "Veille").unwrap();
    create(&mut connection, "Archive").unwrap();

    set_pinned(&mut connection, &veille.id, true).unwrap();

    assert_eq!(names(&mut connection), ["Veille", "Archive", "Boulot"]);
}

#[test]
fn pinned_spaces_are_still_sorted_among_themselves() {
    let mut connection = open_in_memory().unwrap();
    let veille = create(&mut connection, "Veille").unwrap();
    let boulot = create(&mut connection, "Boulot").unwrap();
    create(&mut connection, "Archive").unwrap();

    set_pinned(&mut connection, &veille.id, true).unwrap();
    set_pinned(&mut connection, &boulot.id, true).unwrap();

    assert_eq!(names(&mut connection), ["Boulot", "Veille", "Archive"]);
}

#[test]
fn unpinning_lets_a_space_fall_back_among_the_others() {
    let mut connection = open_in_memory().unwrap();
    let veille = create(&mut connection, "Veille").unwrap();
    create(&mut connection, "Archive").unwrap();

    set_pinned(&mut connection, &veille.id, true).unwrap();
    let unpinned = set_pinned(&mut connection, &veille.id, false).unwrap();

    assert!(!unpinned.pinned);
    assert_eq!(names(&mut connection), ["Archive", "Veille"]);
}

/// A rename answers with the row it read back, not with what it was sent — which
/// is what keeps it from quietly reporting a pinned space as unpinned.
#[test]
fn renaming_a_pinned_space_leaves_it_pinned() {
    let mut connection = open_in_memory().unwrap();
    let space = create(&mut connection, "Veille").unwrap();
    set_pinned(&mut connection, &space.id, true).unwrap();

    let renamed = rename(&mut connection, &space.id, "Veille technique").unwrap();

    assert!(renamed.pinned);
}

#[test]
fn pinning_a_space_that_is_gone_says_which_one() {
    let mut connection = open_in_memory().unwrap();

    let error = set_pinned(&mut connection, "missing", true).unwrap_err();

    assert!(matches!(error, StorageError::SpaceNotFound(id) if id == "missing"));
}

#[test]
fn a_duplicate_name_is_refused_regardless_of_case() {
    let mut connection = open_in_memory().unwrap();
    create(&mut connection, "Personal").unwrap();

    let error = create(&mut connection, "PERSONAL").unwrap_err();

    assert!(matches!(error, StorageError::DuplicateSpaceName(_)));
    assert_eq!(list(&mut connection).unwrap().len(), 1);
}

#[test]
fn exists_distinguishes_known_from_unknown_identifiers() {
    let mut connection = open_in_memory().unwrap();
    let space = create(&mut connection, "Personal").unwrap();

    assert!(exists(connection.db(), &space.id).unwrap());
    assert!(!exists(connection.db(), "unknown").unwrap());
}

#[test]
fn a_renamed_space_keeps_its_identifier() {
    let mut connection = open_in_memory().unwrap();
    let space = create(&mut connection, "Personal").unwrap();

    let renamed = rename(&mut connection, &space.id, "Personnel").unwrap();

    assert_eq!(renamed.id, space.id);
    assert_eq!(renamed.name, "Personnel");
    assert_eq!(list(&mut connection).unwrap()[0].name, "Personnel");
}

#[test]
fn a_space_can_be_renamed_to_a_different_case_of_its_own_name() {
    let mut connection = open_in_memory().unwrap();
    let space = create(&mut connection, "personal").unwrap();

    let renamed = rename(&mut connection, &space.id, "Personal").unwrap();

    assert_eq!(renamed.name, "Personal");
}

#[test]
fn renaming_onto_another_space_name_is_refused() {
    let mut connection = open_in_memory().unwrap();
    create(&mut connection, "Boulot").unwrap();
    let space = create(&mut connection, "Personal").unwrap();

    let error = rename(&mut connection, &space.id, "BOULOT").unwrap_err();

    assert!(matches!(error, StorageError::DuplicateSpaceName(_)));
    assert_eq!(list(&mut connection).unwrap()[1].name, "Personal");
}

#[test]
fn renaming_an_unknown_space_reports_an_error() {
    let mut connection = open_in_memory().unwrap();

    let error = rename(&mut connection, "unknown", "Personal").unwrap_err();

    assert!(matches!(error, StorageError::SpaceNotFound(_)));
}

#[test]
fn deleting_a_space_moves_its_notes_to_the_target() {
    let mut connection = open_in_memory().unwrap();
    let doomed = create(&mut connection, "Personal").unwrap();
    let refuge = create(&mut connection, "Boulot").unwrap();
    note_in(&mut connection, &doomed.id);

    delete(&mut connection, &doomed.id, &refuge.id).unwrap();

    let space_id = notes::table
        .find("n-1")
        .select(notes::space_id)
        .first::<String>(connection.db())
        .unwrap();
    assert_eq!(space_id, refuge.id);
    assert_eq!(list(&mut connection).unwrap().len(), 1);
}

#[test]
fn moving_notes_out_of_a_deleted_space_does_not_touch_their_timestamps() {
    let mut connection = open_in_memory().unwrap();
    let doomed = create(&mut connection, "Personal").unwrap();
    let refuge = create(&mut connection, "Boulot").unwrap();
    note_in(&mut connection, &doomed.id);

    delete(&mut connection, &doomed.id, &refuge.id).unwrap();

    let updated_at = notes::table
        .find("n-1")
        .select(notes::updated_at)
        .first::<String>(connection.db())
        .unwrap();
    assert_eq!(updated_at, T0);
}

#[test]
fn deleting_an_empty_space_leaves_the_others_alone() {
    let mut connection = open_in_memory().unwrap();
    let doomed = create(&mut connection, "Personal").unwrap();
    let refuge = create(&mut connection, "Boulot").unwrap();

    delete(&mut connection, &doomed.id, &refuge.id).unwrap();

    assert_eq!(names(&mut connection), ["Boulot"]);
}

#[test]
fn deleting_an_unknown_space_reports_an_error() {
    let mut connection = open_in_memory().unwrap();
    let refuge = create(&mut connection, "Boulot").unwrap();

    let error = delete(&mut connection, "unknown", &refuge.id).unwrap_err();

    assert!(matches!(error, StorageError::SpaceNotFound(_)));
}

#[test]
fn deleting_into_an_unknown_space_changes_nothing() {
    let mut connection = open_in_memory().unwrap();
    let doomed = create(&mut connection, "Personal").unwrap();
    note_in(&mut connection, &doomed.id);

    let error = delete(&mut connection, &doomed.id, "unknown").unwrap_err();

    assert!(matches!(error, StorageError::SpaceNotFound(_)));
    assert_eq!(list(&mut connection).unwrap().len(), 1);
    assert_eq!(
        notes::table
            .count()
            .get_result::<i64>(connection.db())
            .unwrap(),
        1
    );
}
